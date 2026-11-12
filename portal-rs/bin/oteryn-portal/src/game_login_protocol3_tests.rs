#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gateway login contract §19.1-§19.3 through the composed issuer: protocol 3
//! channel listing and selection, the channel refusal, Decision M1, the play
//! generation fence and recovery handle issuance, with protocol 2 unchanged.
use super::*;
use oteryn_internal_api::canonical::Value;
use sqlx::Row;

const OBSCURA: &str = "01934f10-7c03-7001-805b-3b1122334402";
const PYTHERA: &str = "01934f10-7c03-7001-805b-3b1122334403";
const MIRROR: &str = "01934f10-7c03-7001-805b-3b1122334404";

fn login3(ticket: &str, attempt: Uuid, channel: Option<&str>) -> String {
    let channel = channel.map_or("null".to_owned(), |c| format!("\"{c}\""));
    format!(
        r#"{{"protocol_version":3,"game_login_ticket":"{ticket}","attempt_ref":"{attempt}","character_id":"{CHARACTER}","channel_id":{channel},"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#
    )
}
fn entry(channel: &str, key: &str, label: &str) -> String {
    format!(r#"{{"channel_id":"{channel}","key":"{key}","label":"{label}"}}"#)
}
fn refusal3(code: &str, class: &str, attempt: Uuid, channels: &[String]) -> String {
    let mut body = format!(
        r#"{{"protocol_version":3,"error":{{"code":"{code}","public_class":"{class}"}},"attempt_ref":"{attempt}""#
    );
    if !channels.is_empty() {
        body.push_str(&format!(r#","channels":[{}]"#, channels.join(",")));
    }
    body.push('}');
    body
}
fn success3(
    body: &[u8],
    raw: &str,
) -> Result<oteryn_gateway::v3::Success, Box<dyn std::error::Error>> {
    let request: oteryn_gateway::wire::Request = serde_json::from_str(raw)?;
    Ok(oteryn_gateway::v3::parse_success(body, &request)
        .ok_or_else(|| format!("not a §19.2.4 success: {}", String::from_utf8_lossy(body)))?)
}
fn keys(success: &oteryn_gateway::v3::Success) -> Vec<&str> {
    success.channels.iter().map(|c| c.key.as_str()).collect()
}

impl Database {
    /// The three U19 channels of one World, each with its own route record,
    /// scope assignment and fresh runtime report (§19.2.1, §19.2.2).
    async fn three_channels(&self, grants: &mut Grants) -> TestResult {
        self.execute("UPDATE game_channels SET channel_key='antica'")
            .await?;
        for (source, channel, key) in [(2, OBSCURA, "obscura"), (3, PYTHERA, "pythera")] {
            let host = format!("{key}.example.test");
            let revision =
                admission::route_revision(1, WORLD.parse()?, channel.parse()?, &host, 7172, &host)?;
            sqlx::query("INSERT INTO game_channels (world_id,channel_id,channel_key,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision,native_login_enabled) VALUES ($1,$2,$3,$4,7172,$4,1,$5,true)")
                .bind(WORLD.parse::<Uuid>()?).bind(channel.parse::<Uuid>()?).bind(key).bind(&host).bind(&revision)
                .execute(&self.pool).await?;
            sqlx::query("INSERT INTO native_scope_assignments (world_id,channel_id,assignment_epoch,ownership_generation,node_identity,assigned_at) VALUES ($1,$2,1,1,'node',0)")
                .bind(WORLD.parse::<Uuid>()?).bind(channel.parse::<Uuid>()?)
                .execute(&self.pool).await?;
            self.report_with(
                &revision,
                source,
                &[("channel_id", Value::String(channel.into()))],
            )
            .await?;
        }
        grants.issuance.runtime.peers[0].scopes = vec![
            (WORLD.parse()?, CHANNEL.parse()?),
            (WORLD.parse()?, OBSCURA.parse()?),
            (WORLD.parse()?, PYTHERA.parse()?),
        ];
        Ok(())
    }
    fn protocol3_app(
        &self,
        grants: &Grants,
        handles: admission::Handles,
    ) -> Result<Router, Box<dyn std::error::Error>> {
        self.admission_v3(
            grants.issuance.clone(),
            grants.issuance.signer.clone(),
            Some(admission::Protocol3 {
                runtime: grants.issuance.runtime.clone(),
                handles,
            }),
        )
    }
    async fn handle_row(
        &self,
        handle: &str,
    ) -> Result<(Option<i64>, i64, Option<Uuid>, i64), Box<dyn std::error::Error>> {
        let hash = admission::handle_hash(handle).ok_or("handle shape")?;
        let row = sqlx::query("SELECT revoked_at, play_generation, device_family_id, expires_at - issued_at AS lifetime FROM native_recovery_handles WHERE handle_hash=$1")
            .bind(&hash[..]).fetch_one(&self.pool).await?;
        Ok((
            row.try_get("revoked_at")?,
            row.try_get("play_generation")?,
            row.try_get("device_family_id")?,
            row.try_get("lifetime")?,
        ))
    }
}

#[tokio::test]
async fn pg_protocol3_lists_selects_and_refuses_channels() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut grants = db.grants().await?;
    db.three_channels(&mut grants).await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let app = db.protocol3_app(&grants, admission::Handles::Off)?;

    // Automatic: the lowest canonical candidate, every candidate listed.
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let raw = login3(&native, attempt, None);
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let success = success3(&body, &raw)?;
    assert_eq!(
        (
            success.channel_id.as_str(),
            success.channel_selection.as_str()
        ),
        (CHANNEL, "automatic")
    );
    assert_eq!(keys(&success), ["antica", "obscura", "pythera"]);
    assert_eq!(success.endpoint.host, ROUTE_HOST);
    assert!(success.recovery.is_none());
    assert!(String::from_utf8(body)?.ends_with(r#","recovery":null}"#));
    // §8: the grant is the protocol 2 grant, with the selected channel.
    let (input, _) = success.grant.token.rsplit_once('.').expect("token");
    let payload = URL_SAFE_NO_PAD.decode(input.split_once('.').expect("token").1)?;
    let claims: serde_json::Value = serde_json::from_slice(&payload)?;
    assert_eq!(claims["channel_id"], CHANNEL);
    assert_eq!(claims["route_revision"], grants.revision.as_str());
    // A retry replays the same grant; a protocol 2 body under the same
    // attempt_ref is a conflict (the version is bound into the digest).
    let (status, again) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(success3(&again, &raw)?.grant.token, success.grant.token);
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login_body(&native, attempt, CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        refusal("NATIVE_LOGIN_ATTEMPT_CONFLICT", "RETRY_LOGIN", attempt)
    );

    // Requested.
    let native = db.native_ticket(account).await?;
    let raw = login3(&native, Uuid::now_v7(), Some(OBSCURA));
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let success = success3(&body, &raw)?;
    assert_eq!(
        (
            success.channel_id.as_str(),
            success.channel_selection.as_str(),
            success.endpoint.host.as_str()
        ),
        (OBSCURA, "requested", "obscura.example.test")
    );

    // §19.2.5: a requested non-candidate while others are candidates. The
    // transaction rolls back, so the same ticket works with a listed channel.
    db.execute("UPDATE game_channels SET native_login_enabled=false WHERE channel_key='pythera'")
        .await?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login3(&native, attempt, Some(PYTHERA)),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        String::from_utf8(body)?,
        refusal3(
            "NATIVE_LOGIN_CHANNEL_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt,
            &[
                entry(CHANNEL, "antica", "Antica"),
                entry(OBSCURA, "obscura", "Obscura")
            ]
        )
    );
    assert!(!db.ticket_used(&native).await?);
    let raw = login3(&native, Uuid::now_v7(), Some(OBSCURA));
    let (status, _) = admit(app.clone(), SERVICE, raw).await?;
    assert_eq!(status, StatusCode::OK);

    // Decision M1: a second valid record on antica's (host, port) routes
    // nowhere for both; protocol 2 selection is unchanged by it.
    let mirror = admission::route_revision(
        1,
        WORLD.parse()?,
        MIRROR.parse()?,
        ROUTE_HOST,
        7172,
        ROUTE_HOST,
    )?;
    sqlx::query("INSERT INTO game_channels (world_id,channel_id,channel_key,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision,native_login_enabled) VALUES ($1,$2,'mirror',$3,7172,$3,1,$4,true)")
        .bind(WORLD.parse::<Uuid>()?).bind(MIRROR.parse::<Uuid>()?).bind(ROUTE_HOST).bind(&mirror)
        .execute(&db.pool).await?;
    let native = db.native_ticket(account).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let success = success3(&body, &raw)?;
    assert_eq!(success.channel_id, OBSCURA);
    assert_eq!(keys(&success), ["obscura"]);
    let native = db.native_ticket(account).await?;
    let raw = login_body(&native, Uuid::now_v7(), CHARACTER);
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let request: oteryn_gateway::wire::Request = serde_json::from_str(&raw)?;
    let v2 = oteryn_gateway::wire::parse_success(&body, &request).expect("protocol 2 success");
    assert_eq!(v2.channel_id, CHANNEL);
    assert!(!String::from_utf8(body)?.contains("channels"));

    // A keyless channel is never a protocol 3 candidate.
    db.execute("UPDATE game_channels SET channel_key=NULL WHERE channel_key='obscura'")
        .await?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login3(&native, attempt, Some(OBSCURA)),
    )
    .await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        String::from_utf8(body)?,
        refusal3(
            "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt,
            &[]
        )
    );

    // §19.1: with the switch off every protocol 3 request is unsupported.
    let attempt = Uuid::now_v7();
    let (status, body) = admit(
        db.admission_with(grants.issuance.clone())?,
        SERVICE,
        login3(&native, attempt, None),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_UNSUPPORTED_VERSION",
            "CLIENT_UPDATE_REQUIRED",
            attempt
        )
    );
    assert!(!db.ticket_used(&native).await?);
    db.close().await
}

#[tokio::test]
async fn pg_protocol3_recovery_handles_bind_fence_and_revoke() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut grants = db.grants().await?;
    db.three_channels(&mut grants).await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let unbound = db.protocol3_app(
        &grants,
        admission::Handles::On {
            lifetime_seconds: 43200,
            require_device_binding: false,
        },
    )?;
    let generation = |db: &Database| {
        let pool = db.pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT generation FROM native_play_generations WHERE account_id=$1",
            )
            .bind(account)
            .fetch_one(&pool)
            .await
        }
    };

    // Issued in the fresh transaction; only the hash is stored.
    let native = db.native_ticket(account).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(unbound.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let first = success3(&body, &raw)?.recovery.expect("handle");
    assert!((43190..=43200).contains(&first.expires_in));
    assert_eq!(db.handle_row(&first.handle).await?, (None, 1, None, 43200));
    let stored: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM native_admissions WHERE position(convert_to($1,'UTF8') IN response_body) > 0",
    )
    .bind(&first.handle)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(stored, 0, "the plaintext handle is never stored");

    // A retry mints a new handle with the same bindings and revokes the old.
    let (status, again) = admit(unbound.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let second = success3(&again, &raw)?.recovery.expect("replacement");
    assert_ne!(second.handle, first.handle);
    assert!(db.handle_row(&first.handle).await?.0.is_some());
    assert_eq!(db.handle_row(&second.handle).await?, (None, 1, None, 43200));

    // Every committed fresh attempt advances the play generation and revokes
    // older handles, under protocol 3 and protocol 2 alike.
    let native = db.native_ticket(account).await?;
    let raw3 = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(unbound.clone(), SERVICE, raw3.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let third = success3(&body, &raw3)?.recovery.expect("handle");
    assert_eq!(generation(&db).await?, 2);
    assert!(db.handle_row(&second.handle).await?.0.is_some());
    assert_eq!(db.handle_row(&third.handle).await?.1, 2);
    let native = db.native_ticket(account).await?;
    let (status, _) = admit(
        unbound.clone(),
        SERVICE,
        login_body(&native, Uuid::now_v7(), CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(generation(&db).await?, 3);
    assert!(db.handle_row(&third.handle).await?.0.is_some());
    // A revoked handle is never revived by a retry.
    let (status, body) = admit(unbound.clone(), SERVICE, raw3.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(success3(&body, &raw3)?.recovery.is_none());

    // Binding required and no device family on the ticket: "recovery": null.
    let bound = db.protocol3_app(
        &grants,
        admission::Handles::On {
            lifetime_seconds: 3600,
            require_device_binding: true,
        },
    )?;
    let native = db.native_ticket(account).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(bound.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(success3(&body, &raw)?.recovery.is_none());

    // A live keyed device family binds the handle and caps its expiry at the
    // family's absolute deadline; revoking the family revokes the handle (DB-6).
    let family = Uuid::now_v7();
    let now = db.now().await?;
    sqlx::query("INSERT INTO device_sessions (id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at,persistence_class,device_public_key,device_key_thumbprint) SELECT $1,$2,$3,$4,game_auth_generation,native_security_generation,$5,$5,'session',$6,$7 FROM accounts WHERE id=$2")
        .bind(family).bind(account).bind(Uuid::now_v7()).bind(family.to_string())
        .bind(now + 600).bind(&[7u8; 32][..]).bind(URL_SAFE_NO_PAD.encode(Sha256::digest([7u8; 32])))
        .execute(&db.pool).await?;
    let native = db.native_ticket(account).await?;
    sqlx::query("UPDATE game_login_tickets SET device_family_id=$1 WHERE ticket_hash=$2")
        .bind(family)
        .bind(&Sha256::digest(native.as_bytes())[..])
        .execute(&db.pool)
        .await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(bound.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let handle = success3(&body, &raw)?.recovery.expect("bound handle");
    assert!(
        (590..=600).contains(&handle.expires_in),
        "{}",
        handle.expires_in
    );
    let (revoked, _, bound_family, _) = db.handle_row(&handle.handle).await?;
    assert_eq!((revoked, bound_family), (None, Some(family)));
    sqlx::query("UPDATE device_sessions SET revoked_at=$2 WHERE id=$1")
        .bind(family)
        .bind(now)
        .execute(&db.pool)
        .await?;
    assert!(db.handle_row(&handle.handle).await?.0.is_some());
    db.close().await
}
