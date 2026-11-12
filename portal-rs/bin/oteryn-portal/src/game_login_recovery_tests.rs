#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gateway login contract §19.4, §19.5, §19.7 and §19.8 through the composed
//! issuer: recovery redemption with its own signing scope, replay, the shared
//! attempt namespace, the play generation fence, every refusal, concurrency
//! and handle revocation.
use super::*;
use oteryn_gameauth::keys;

const RECOVER: &str = "/internal/v1/game-auth/native-recoveries";
const REVOKE: &str = "/internal/v1/game-auth/native-recoveries/revoke";
const RECOVERY_KID: &str = "recovery-s19-1";
/// Well-formed, never issued.
const UNKNOWN: &str = "otr1.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn recover_body(handle: &str, attempt: Uuid, character: &str) -> String {
    format!(
        r#"{{"protocol_version":3,"recovery_handle":"{handle}","attempt_ref":"{attempt}","character_id":"{character}","offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#
    )
}
struct Reply {
    status: StatusCode,
    body: Vec<u8>,
    retry_after: Option<String>,
}
async fn post(
    app: Router,
    path: &str,
    token: &str,
    body: &str,
) -> Result<Reply, Box<dyn std::error::Error>> {
    let response = app.oneshot(bearer("POST", path, token, body)).await?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(header::RETRY_AFTER)
        .map(|v| v.to_str().map(str::to_owned))
        .transpose()?;
    Ok(Reply {
        status,
        body: to_bytes(response.into_body(), 65536).await?.to_vec(),
        retry_after,
    })
}
#[track_caller]
fn assert_refused(reply: &Reply, status: StatusCode, code: &str, class: &str, attempt: Uuid) {
    assert_eq!(
        (
            reply.status,
            String::from_utf8_lossy(&reply.body).into_owned()
        ),
        (status, refusal3(code, class, attempt, &[])),
        "{code}"
    );
    assert_eq!(reply.retry_after, None);
}
#[track_caller]
fn assert_authentication(reply: &Reply, attempt: Uuid) {
    assert_refused(
        reply,
        StatusCode::UNAUTHORIZED,
        "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
        "AUTHENTICATION_REQUIRED",
        attempt,
    );
}
fn token(body: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(body)?;
    Ok(value["grant"]["token"]
        .as_str()
        .ok_or("no token")?
        .to_owned())
}

impl Database {
    /// A §19.5 recovery custody beside the fresh one: its own files and kid,
    /// published in the recovery trust scope, composed through the binary's
    /// `recovery_issuer` exactly as from `PORTAL_*` variables.
    async fn recovery_custody(
        &self,
        grants: &Grants,
        protocol3: &admission::Protocol3,
    ) -> Result<(tempfile::TempDir, admission::RecoveryIssuer), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
        let wrapping = dir.path().join("wrapping");
        std::fs::write(&wrapping, Sha256::digest(Uuid::now_v7().as_bytes()))?;
        std::fs::set_permissions(&wrapping, std::fs::Permissions::from_mode(0o600))?;
        let custody = dir.path().join("custody");
        let witness = grants.lookup(&[])(admission::WITNESS_DIRECTORY).ok_or("witness")?;
        let settings = keys::Settings {
            enabled: true,
            key_purpose: keys::RECOVERY_KEY_PURPOSE.into(),
            clock_uncertainty_seconds: 1,
        };
        let registry = keys::PublicTrustRegistry::new_scoped(
            self.pool.clone(),
            settings.clone(),
            keys::Scope::Recovery,
            witness.into(),
        )?;
        {
            // The fresh admission kid is never published in the recovery scope.
            let mut stray = keys::Keyring::create_scoped(
                settings.clone(),
                keys::Scope::Recovery,
                keys::EncryptedFileStorage::from_key_file(dir.path().join("stray"), &wrapping)?,
            )?;
            let public = stray.generate_next(KID)?;
            assert!(registry.publish(&public).await.is_err());
        }
        {
            let mut ring = keys::Keyring::create_scoped(
                settings,
                keys::Scope::Recovery,
                keys::EncryptedFileStorage::from_key_file(&custody, &wrapping)?,
            )?;
            let public = ring.generate_next(RECOVERY_KID)?;
            registry.publish(&public).await?;
            sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()-interval '10 seconds' WHERE key_id=$1")
                .bind(RECOVERY_KID).execute(&self.pool).await?;
            let snapshot = registry.snapshot().await?;
            ring.activate(RECOVERY_KID, u64::try_from(self.now().await?)?, &snapshot)?;
        }
        let path = |p: &std::path::Path| p.to_string_lossy().into_owned();
        let (custody, wrapping) = (path(&custody), path(&wrapping));
        // The recovery custody must differ from the fresh one.
        let fresh = grants.lookup(&[])(admission::KEY_CUSTODY_FILE).ok_or("custody")?;
        let same = recovery_issuer(
            self.pool.clone(),
            &grants.issuance,
            Some(protocol3),
            &Self::projection_settings(),
            grants.lookup(&[
                (admission::RECOVERY_KEY_CUSTODY_FILE, &fresh),
                (admission::RECOVERY_KEY_WRAPPING_FILE, &wrapping),
            ]),
        )
        .ok_or("issuer")?;
        assert!(same.signer.is_none());
        let lookup = grants.lookup(&[
            (admission::RECOVERY_KEY_CUSTODY_FILE, &custody),
            (admission::RECOVERY_KEY_WRAPPING_FILE, &wrapping),
        ]);
        // Never composed while recovery handles are off.
        let off = admission::Protocol3 {
            handles: admission::Handles::Off,
            ..protocol3.clone()
        };
        assert!(
            recovery_issuer(
                self.pool.clone(),
                &grants.issuance,
                Some(&off),
                &Self::projection_settings(),
                &lookup,
            )
            .is_none()
        );
        let issuer = recovery_issuer(
            self.pool.clone(),
            &grants.issuance,
            Some(protocol3),
            &Self::projection_settings(),
            lookup,
        )
        .ok_or("issuer")?;
        assert!(issuer.signer.is_some(), "recovery custody must open");
        Ok((dir, issuer))
    }
}

/// One account with a live protocol 3 handle and the composed recovery issuer.
struct Recovery {
    db: Database,
    grants: Grants,
    _dir: tempfile::TempDir,
    issuer: admission::RecoveryIssuer,
    protocol3: admission::Protocol3,
    account: Uuid,
    handle: String,
    fresh_attempt: Uuid,
}
impl Recovery {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Some(db) = Database::open().await? else {
            return Ok(None);
        };
        let grants = db.grants().await?;
        db.execute("UPDATE game_channels SET channel_key='antica'")
            .await?;
        let protocol3 = admission::Protocol3 {
            runtime: grants.issuance.runtime.clone(),
            handles: admission::Handles::On {
                lifetime_seconds: 43200,
                require_device_binding: false,
                devices_enabled: true,
            },
        };
        let (dir, issuer) = db.recovery_custody(&grants, &protocol3).await?;
        let account = db.account().await?;
        db.publish(account).await?;
        let mut this = Self {
            db,
            grants,
            _dir: dir,
            issuer,
            protocol3,
            account,
            handle: String::new(),
            fresh_attempt: Uuid::nil(),
        };
        (this.handle, this.fresh_attempt) = this.login().await?;
        Ok(Some(this))
    }
    fn app(&self) -> Result<Router, Box<dyn std::error::Error>> {
        self.app_with(Some(self.protocol3.clone()), Some(self.issuer.clone()))
    }
    fn app_with(
        &self,
        protocol3: Option<admission::Protocol3>,
        recovery: Option<admission::RecoveryIssuer>,
    ) -> Result<Router, Box<dyn std::error::Error>> {
        self.db.admission_full(
            self.grants.issuance.clone(),
            self.grants.issuance.signer.clone(),
            protocol3,
            recovery,
        )
    }
    /// A protocol 3 fresh login: its handle and attempt.
    async fn login(&self) -> Result<(String, Uuid), Box<dyn std::error::Error>> {
        let native = self.db.native_ticket(self.account).await?;
        let attempt = Uuid::now_v7();
        let raw = login3(&native, attempt, None);
        let (status, body) = admit(self.app()?, SERVICE, raw.clone()).await?;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let handle = success3(&body, &raw)?.recovery.expect("handle").handle;
        Ok((handle, attempt))
    }
    async fn recover(
        &self,
        app: Router,
        attempt: Uuid,
    ) -> Result<Reply, Box<dyn std::error::Error>> {
        self.send(app, recover_body(&self.handle, attempt, CHARACTER))
            .await
    }
    async fn send(&self, app: Router, body: String) -> Result<Reply, Box<dyn std::error::Error>> {
        post(app, RECOVER, SERVICE, &body).await
    }
    /// Retires every recovery attempt, ending the §19.4 step 3 window.
    async fn settle(&self) -> TestResult {
        self.db
            .execute("UPDATE native_recoveries SET expires_at_ms=0, retires_at_ms=0")
            .await
    }
    async fn attempts(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM native_recoveries")
            .fetch_one(&self.db.pool)
            .await
    }
    async fn generation(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT generation FROM native_play_generations WHERE account_id=$1")
            .bind(self.account)
            .fetch_one(&self.db.pool)
            .await
    }
    async fn locator(
        &self,
        account: Uuid,
        world: &str,
        channel: &str,
        present: bool,
        conflicted: bool,
    ) -> TestResult {
        sqlx::query("INSERT INTO native_recovery_locators (character_id,account_id,world_id,channel_id,present,assignment_epoch,scope_ownership_generation,locator_revision,committed_at,source_authority,conflicted) VALUES ($1,$2,$3,$4,$5,1,1,1,0,'game',$6) ON CONFLICT (character_id) DO UPDATE SET account_id=EXCLUDED.account_id, world_id=EXCLUDED.world_id, channel_id=EXCLUDED.channel_id, present=EXCLUDED.present, conflicted=EXCLUDED.conflicted")
            .bind(CHARACTER.parse::<Uuid>()?).bind(account).bind(world.parse::<Uuid>()?).bind(channel.parse::<Uuid>()?).bind(present).bind(conflicted)
            .execute(&self.db.pool).await?;
        Ok(())
    }
    async fn close(self) -> TestResult {
        self.db.close().await
    }
}

#[tokio::test]
async fn pg_recovery_redeems_a_handle_and_replays_the_attempt() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    let attempt = Uuid::now_v7();
    let reply = r.recover(app.clone(), attempt).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert!(reply.body.len() <= admission::MAX_RECOVERY_RESPONSE);
    let text = String::from_utf8(reply.body.clone())?;
    let prefix = format!(
        r#"{{"protocol_version":3,"attempt_ref":"{attempt}","world_id":"{WORLD}","channel_id":"{CHANNEL}","endpoint":{{"host":"{ROUTE_HOST}","port":7172,"tls_server_name":"{ROUTE_HOST}","alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}},"grant":{{"profile":"oteryn-reauth-recovery-v1","token":""#
    );
    assert!(text.starts_with(&prefix), "{text}");
    let token = token(&reply.body)?;
    let rest = &text[prefix.len() + token.len()..];
    let valid: i64 = rest
        .strip_prefix(r#"","valid_for_seconds":"#)
        .and_then(|r| r.strip_suffix("}}"))
        .ok_or("member order")?
        .parse()?;
    assert!((18..=20).contains(&valid), "{valid}");

    // §19.4 step 6: the recovery header, signed by the recovery key the trust
    // registry publishes, with exactly the recovery claims in order.
    let (input, signature) = token.rsplit_once('.').ok_or("token")?;
    let (header64, payload64) = input.split_once('.').ok_or("token")?;
    assert_eq!(
        URL_SAFE_NO_PAD.decode(header64)?,
        format!(r#"{{"alg":"Ed25519","kid":"{RECOVERY_KID}","typ":"oteryn-recovery+jwt"}}"#)
            .into_bytes()
    );
    keys::verify(
        db_key(&r.db, RECOVERY_KID).await?,
        input.as_bytes(),
        &URL_SAFE_NO_PAD.decode(signature)?,
    )?;
    let payload = String::from_utf8(URL_SAFE_NO_PAD.decode(payload64)?)?;
    let claims: serde_json::Value = serde_json::from_str(&payload)?;
    let names = [
        "iss",
        "aud",
        "iat",
        "nbf",
        "exp",
        "jti",
        "profile",
        "purpose",
        "attempt_ref",
        "account_id",
        "character_id",
        "world_id",
        "account_security_generation",
        "protocol_major",
        "transport_profile",
        "ruleset_revision",
        "content_revision",
        "map_revision",
        "world_policy_revision",
    ];
    assert_eq!(claims.as_object().ok_or("claims")?.len(), names.len());
    let positions: Vec<usize> = names
        .iter()
        .map(|n| payload.find(&format!("\"{n}\":")).expect("claim"))
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{payload}");
    let generation: String =
        sqlx::query_scalar("SELECT native_security_generation::text FROM accounts WHERE id=$1")
            .bind(r.account)
            .fetch_one(&r.db.pool)
            .await?;
    for (name, value) in [
        ("iss", "urn:oteryn:platform:game-recovery"),
        ("aud", "urn:oteryn:game:recovery"),
        ("profile", "oteryn-reauth-recovery-v1"),
        ("purpose", "existing_actor_recovery"),
        ("attempt_ref", &attempt.to_string()),
        ("account_id", &r.account.to_string()),
        ("character_id", CHARACTER),
        ("world_id", WORLD),
        ("account_security_generation", &generation),
        ("ruleset_revision", "rules.1"),
        ("content_revision", "content.1"),
        ("map_revision", "map.1"),
        ("world_policy_revision", "policy.1"),
    ] {
        assert_eq!(claims[name], value, "{name}");
    }
    assert_eq!(
        (
            claims["protocol_major"].as_u64(),
            claims["transport_profile"].as_u64()
        ),
        (Some(1), Some(1))
    );
    assert_eq!(claims["nbf"], claims["iat"]);
    assert_eq!(
        claims["exp"].as_u64().ok_or("exp")? - claims["iat"].as_u64().ok_or("iat")?,
        20
    );
    // Stored as the signing input only (§6.1).
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT response_body FROM native_recoveries WHERE attempt_ref=$1")
            .bind(attempt)
            .fetch_one(&r.db.pool)
            .await?;
    let stored = String::from_utf8(stored)?;
    assert!(stored.contains(input) && !stored.contains(&token));

    // A retry re-signs the same grant; a changed payload or another handle is
    // a conflict.
    let again = r.recover(app.clone(), attempt).await?;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(token_of(&again)?, token);
    let changed = recover_body(&r.handle, attempt, CHARACTER).replace("0.1.0+test", "0.1.1+test");
    let conflict = |reply: &Reply| {
        assert_refused(
            reply,
            StatusCode::CONFLICT,
            "NATIVE_RECOVERY_ATTEMPT_CONFLICT",
            "RETRY_LOGIN",
            attempt,
        )
    };
    conflict(&r.send(app.clone(), changed).await?);
    conflict(
        &r.send(app.clone(), recover_body(UNKNOWN, attempt, CHARACTER))
            .await?,
    );

    // §19.4 step 3: one unretired attempt per account, with Retry-After.
    let pending = Uuid::now_v7();
    let reply = r.recover(app.clone(), pending).await?;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(
        String::from_utf8(reply.body)?,
        refusal3(
            "NATIVE_RECOVERY_ATTEMPT_PENDING",
            "TEMPORARILY_UNAVAILABLE",
            pending,
            &[]
        )
    );
    let after: u64 = reply.retry_after.ok_or("Retry-After")?.parse()?;
    assert!((1..=27).contains(&after), "{after}");

    // One attempt_ref namespace with fresh admissions, in both directions.
    let native = r.db.native_ticket(r.account).await?;
    let (status, body) = admit(app.clone(), SERVICE, login3(&native, attempt, None)).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        String::from_utf8(body)?,
        refusal3("NATIVE_LOGIN_ATTEMPT_CONFLICT", "RETRY_LOGIN", attempt, &[])
    );
    assert!(!r.db.ticket_used(&native).await?);
    let reply = r.recover(app.clone(), r.fresh_attempt).await?;
    assert_refused(
        &reply,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_ATTEMPT_CONFLICT",
        "RETRY_LOGIN",
        r.fresh_attempt,
    );

    // A retry with under one second left is expired; an unreadable record
    // needs reconciliation.
    sqlx::query("UPDATE native_recoveries SET expires_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+500 WHERE attempt_ref=$1")
        .bind(attempt).execute(&r.db.pool).await?;
    assert_refused(
        &r.recover(app.clone(), attempt).await?,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_GRANT_EXPIRED",
        "RETRY_LOGIN",
        attempt,
    );
    sqlx::query("UPDATE native_recoveries SET response_body=convert_to('{}','UTF8'), expires_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+20000, retires_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+26000 WHERE attempt_ref=$1")
        .bind(attempt).execute(&r.db.pool).await?;
    assert_refused(
        &r.recover(app.clone(), attempt).await?,
        StatusCode::SERVICE_UNAVAILABLE,
        "RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED",
        "TEMPORARILY_UNAVAILABLE",
        attempt,
    );

    // Coordinator decision 2026-11-12: a fresh login with protocol 3 OFF
    // advances the play generation, so an earlier handle is unusable even if
    // its revocation were lost.
    r.settle().await?;
    assert_eq!(r.generation().await?, 1);
    let native = r.db.native_ticket(r.account).await?;
    let (status, _) = admit(
        r.db.admission_with(r.grants.issuance.clone())?,
        SERVICE,
        login_body(&native, Uuid::now_v7(), CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(r.generation().await?, 2);
    let late = Uuid::now_v7();
    assert_authentication(&r.recover(app.clone(), late).await?, late);
    r.db.execute("UPDATE native_recovery_handles SET revoked_at=NULL")
        .await?;
    let late = Uuid::now_v7();
    assert_authentication(&r.recover(app.clone(), late).await?, late);
    r.close().await
}
fn token_of(reply: &Reply) -> Result<String, Box<dyn std::error::Error>> {
    token(&reply.body)
}
async fn db_key(db: &Database, kid: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    db.registry_key(kid).await
}

#[tokio::test]
async fn pg_recovery_refusals_fail_closed_without_recording_an_attempt() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    let attempt = Uuid::now_v7();
    let body = recover_body(&r.handle, attempt, CHARACTER);

    // Service authentication and the switches.
    let reply = post(app.clone(), RECOVER, "wrong", &body).await?;
    assert_eq!(
        (reply.status, reply.body),
        (
            StatusCode::UNAUTHORIZED,
            br#"{"error":"unauthorized_service"}"#.to_vec()
        )
    );
    let reply = r
        .send(
            r.db.admission_with(r.grants.issuance.clone())?,
            body.clone(),
        )
        .await?;
    assert_eq!(
        (reply.status, reply.body),
        (
            StatusCode::BAD_REQUEST,
            refusal(
                "NATIVE_LOGIN_UNSUPPORTED_VERSION",
                "CLIENT_UPDATE_REQUIRED",
                attempt
            )
        )
    );
    let unavailable = |reply: &Reply| {
        assert_refused(
            reply,
            StatusCode::SERVICE_UNAVAILABLE,
            "NATIVE_LOGIN_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt,
        )
    };
    unavailable(
        &r.send(r.app_with(Some(r.protocol3.clone()), None)?, body.clone())
            .await?,
    );
    let bound = admission::Protocol3 {
        handles: admission::Handles::On {
            lifetime_seconds: 43200,
            require_device_binding: true,
            devices_enabled: true,
        },
        ..r.protocol3.clone()
    };
    // §19.15: with binding on, a request without `device_proof` is malformed.
    assert_refused(
        &r.send(
            r.app_with(Some(bound), Some(r.issuer.clone()))?,
            body.clone(),
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "NATIVE_RECOVERY_REQUEST_MALFORMED",
        "RETRY_LOGIN",
        attempt,
    );
    let unsigned = admission::RecoveryIssuer {
        signer: None,
        ..r.issuer.clone()
    };
    unavailable(
        &r.send(
            r.app_with(Some(r.protocol3.clone()), Some(unsigned))?,
            body.clone(),
        )
        .await?,
    );

    // Bad input.
    let malformed = |reply: &Reply| {
        assert_refused(
            reply,
            StatusCode::BAD_REQUEST,
            "NATIVE_RECOVERY_REQUEST_MALFORMED",
            "RETRY_LOGIN",
            attempt,
        )
    };
    malformed(
        &r.send(app.clone(), recover_body("otr1.short", attempt, CHARACTER))
            .await?,
    );
    malformed(
        &r.send(
            app.clone(),
            body.replacen('{', r#"{"device_proof":"synthetic","#, 1),
        )
        .await?,
    );
    malformed(
        &r.send(app.clone(), body.replace(r#""offer":"#, r#""offers":"#))
            .await?,
    );
    assert_refused(
        &r.send(
            app.clone(),
            body.replace(r#""protocol_version":3"#, r#""protocol_version":2"#),
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "NATIVE_LOGIN_UNSUPPORTED_VERSION",
        "CLIENT_UPDATE_REQUIRED",
        attempt,
    );

    // §19.4 step 2: an unknown handle, or one bound to another Character.
    assert_authentication(
        &r.send(app.clone(), recover_body(UNKNOWN, attempt, CHARACTER))
            .await?,
        attempt,
    );
    assert_authentication(
        &r.send(
            app.clone(),
            recover_body(&r.handle, attempt, "01934f10-7c04-7001-805b-3b1122334499"),
        )
        .await?,
        attempt,
    );

    // §19.4 steps 4 and 5.
    let route = |reply: &Reply| {
        assert_refused(
            reply,
            StatusCode::SERVICE_UNAVAILABLE,
            "NATIVE_RECOVERY_ROUTE_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt,
        )
    };
    let dark = admission::RecoveryIssuer {
        projection_authority: String::new(),
        ..r.issuer.clone()
    };
    route(
        &r.send(
            r.app_with(Some(r.protocol3.clone()), Some(dark))?,
            body.clone(),
        )
        .await?,
    );
    const ELSEWHERE: &str = "01934f10-7c02-7001-805b-3b1122334499";
    r.locator(r.account, ELSEWHERE, CHANNEL, true, false)
        .await?;
    assert_refused(
        &r.send(app.clone(), body.clone()).await?,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_WORLD_STALE",
        "RETRY_LOGIN",
        attempt,
    );
    r.locator(r.account, WORLD, CHANNEL, true, true).await?;
    route(&r.send(app.clone(), body.clone()).await?);
    r.locator(r.account, WORLD, CHANNEL, false, false).await?;
    assert_refused(
        &r.send(app.clone(), body.clone()).await?,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_CHARACTER_CONFLICT",
        "SESSION_UNAVAILABLE",
        attempt,
    );
    // The locator's channel is used as is: never a substitute channel.
    r.locator(r.account, WORLD, OBSCURA, true, false).await?;
    route(&r.send(app.clone(), body.clone()).await?);
    r.locator(r.account, WORLD, CHANNEL, true, false).await?;
    r.db.execute("UPDATE game_channels SET native_login_enabled=false")
        .await?;
    route(&r.send(app.clone(), body.clone()).await?);
    r.db.execute("UPDATE game_channels SET native_login_enabled=true")
        .await?;
    assert_eq!(r.attempts().await?, 0, "refusals record no attempt");

    // A security generation other than the handle's is SECURITY_TERMINAL.
    r.db.execute("UPDATE native_recovery_handles SET security_generation=security_generation+1")
        .await?;
    assert_authentication(&r.send(app.clone(), body.clone()).await?, attempt);
    r.db.execute("UPDATE native_recovery_handles SET security_generation=security_generation-1")
        .await?;
    assert_eq!(r.attempts().await?, 0, "refusals record no attempt");

    // A Character listed as unavailable (in play) is recoverable; another
    // account's locator entry is ignored and the bound channel is used.
    r.db.execute("UPDATE account_characters SET availability='UNAVAILABLE'")
        .await?;
    r.locator(Uuid::now_v7(), ELSEWHERE, OBSCURA, false, true)
        .await?;
    let reply = r.send(app.clone(), body.clone()).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert!(String::from_utf8(reply.body)?.contains(&format!(r#""channel_id":"{CHANNEL}""#)));
    r.settle().await?;

    // Ownership: a Character no longer listed for the account.
    r.db.execute("DELETE FROM account_characters").await?;
    let other = Uuid::now_v7();
    assert_refused(
        &r.recover(app.clone(), other).await?,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_CHARACTER_CONFLICT",
        "SESSION_UNAVAILABLE",
        other,
    );

    // A suspended account is SECURITY_TERMINAL before anything else.
    r.db.suspend(r.account).await?;
    let other = Uuid::now_v7();
    assert_authentication(&r.recover(app.clone(), other).await?, other);
    assert_eq!(r.attempts().await?, 1);
    r.close().await
}

#[tokio::test]
async fn pg_recovery_double_redeem_issues_one_grant() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    // Two attempts with one handle: one grant, the other is pending.
    let (a, b) = (Uuid::now_v7(), Uuid::now_v7());
    let (first, second) = tokio::join!(r.recover(app.clone(), a), r.recover(app.clone(), b));
    let mut statuses = [first?.status, second?.status];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(r.attempts().await?, 1);
    // One attempt sent twice at once: the same grant, recorded once.
    r.settle().await?;
    let c = Uuid::now_v7();
    let (first, second) = tokio::join!(r.recover(app.clone(), c), r.recover(app.clone(), c));
    let (first, second) = (first?, second?);
    assert_eq!(
        (first.status, second.status),
        (StatusCode::OK, StatusCode::OK)
    );
    assert_eq!(token_of(&first)?, token_of(&second)?);
    assert_eq!(r.attempts().await?, 2);
    r.close().await
}

#[tokio::test]
async fn pg_recovery_revoke_is_idempotent_and_scoped_to_one_handle() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    let revoke = |handle: &str| format!(r#"{{"protocol_version":3,"recovery_handle":"{handle}"}}"#);
    let revoked = |reply: Reply| {
        assert_eq!(
            (reply.status, reply.body),
            (
                StatusCode::OK,
                br#"{"protocol_version":3,"revoked":true}"#.to_vec()
            )
        );
    };
    // Unknown, live and already revoked handles get the same answer, also
    // without the recovery issuer composed.
    revoked(
        post(
            r.app_with(Some(r.protocol3.clone()), None)?,
            REVOKE,
            SERVICE,
            &revoke(UNKNOWN),
        )
        .await?,
    );
    revoked(post(app.clone(), REVOKE, SERVICE, &revoke(&r.handle)).await?);
    let hash = admission::handle_hash(&r.handle).ok_or("handle")?;
    let at: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM native_recovery_handles WHERE handle_hash=$1")
            .bind(&hash[..])
            .fetch_one(&r.db.pool)
            .await?;
    revoked(post(app.clone(), REVOKE, SERVICE, &revoke(&r.handle)).await?);
    let again: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM native_recovery_handles WHERE handle_hash=$1")
            .bind(&hash[..])
            .fetch_one(&r.db.pool)
            .await?;
    assert!(at.is_some());
    assert_eq!(again, at, "a repeated revoke changes nothing");
    assert_eq!(r.generation().await?, 1, "revoke advances no generation");
    let attempt = Uuid::now_v7();
    assert_authentication(&r.recover(app.clone(), attempt).await?, attempt);

    // Bad input and the protocol 3 switch.
    let reply = post(app.clone(), REVOKE, SERVICE, &revoke("otr1.short")).await?;
    assert_eq!(
        (reply.status, String::from_utf8(reply.body)?),
        (
            StatusCode::BAD_REQUEST,
            r#"{"protocol_version":3,"error":{"code":"NATIVE_RECOVERY_REQUEST_MALFORMED","public_class":"RETRY_LOGIN"},"attempt_ref":null}"#.to_owned()
        )
    );
    let reply = post(
        app.clone(),
        REVOKE,
        SERVICE,
        &format!(r#"{{"protocol_version":3,"recovery_handle":"{UNKNOWN}","attempt_ref":null}}"#),
    )
    .await?;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let reply = post(
        r.db.admission_with(r.grants.issuance.clone())?,
        REVOKE,
        SERVICE,
        &revoke(UNKNOWN),
    )
    .await?;
    assert_eq!(
        (reply.status, String::from_utf8(reply.body)?),
        (
            StatusCode::BAD_REQUEST,
            r#"{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_UNSUPPORTED_VERSION","public_class":"CLIENT_UPDATE_REQUIRED"},"attempt_ref":null}"#.to_owned()
        )
    );
    r.close().await
}

#[tokio::test]
async fn pg_recovery_account_limit_is_twenty_committed_attempts_per_hour() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    // §19.9: 20 committed attempts per AccountId per hour. The rows are
    // retired, so the §19.4 step 3 window is closed; an older row is outside.
    let record = |seconds: f64| {
        let mut hash = [0u8; 32];
        hash[..16].copy_from_slice(Uuid::now_v7().as_bytes());
        hash[16..].copy_from_slice(Uuid::now_v7().as_bytes());
        sqlx::query("INSERT INTO native_recoveries (attempt_ref, handle_hash, request_digest, account_id, response_body, expires_at_ms, retires_at_ms, created_at) VALUES ($1, $2, $2, $3, '\\x7b7d', 0, 0, clock_timestamp() - make_interval(secs => $4))")
            .bind(Uuid::now_v7()).bind(hash.to_vec()).bind(r.account).bind(seconds)
            .execute(&r.db.pool)
    };
    record(3660.0).await?;
    for _ in 0..19 {
        record(3540.0).await?;
    }
    // 19 in the window leave room for one more.
    let reply = r.recover(app.clone(), Uuid::now_v7()).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    r.settle().await?;
    let before = r.attempts().await?;
    let attempt = Uuid::now_v7();
    let reply = r.recover(app.clone(), attempt).await?;
    assert_eq!(
        (
            reply.status,
            String::from_utf8_lossy(&reply.body).into_owned(),
            reply.retry_after.as_deref()
        ),
        (
            StatusCode::TOO_MANY_REQUESTS,
            refusal3(
                "NATIVE_LOGIN_RATE_LIMITED",
                "TEMPORARILY_UNAVAILABLE",
                attempt,
                &[]
            ),
            Some("60")
        )
    );
    // Exhaustion changes no state (§10).
    assert_eq!(r.attempts().await?, before);
    r.close().await
}

#[tokio::test]
async fn pg_recovery_ignores_locator_entries_of_a_lower_epoch() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let app = r.app()?;
    let attempt = Uuid::now_v7();
    // §19.6.3 restore reset: an entry of the current epoch steers recovery.
    r.locator(r.account, WORLD, CHANNEL, false, false).await?;
    assert_refused(
        &r.recover(app.clone(), attempt).await?,
        StatusCode::CONFLICT,
        "NATIVE_RECOVERY_CHARACTER_CONFLICT",
        "SESSION_UNAVAILABLE",
        attempt,
    );
    // A higher assignment epoch invalidates every entry of lower epochs: the
    // handle's bound channel is the target again.
    r.db.execute("UPDATE native_scope_assignments SET assignment_epoch=2")
        .await?;
    r.db.report_with(
        &r.grants.revision,
        2,
        &[(
            "assignment_epoch",
            oteryn_internal_api::canonical::Value::String("2".into()),
        )],
    )
    .await?;
    let reply = r.recover(app, attempt).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert!(String::from_utf8(reply.body)?.contains(&format!(r#""channel_id":"{CHANNEL}""#)));
    r.close().await
}

/// The §19.15.2 recovery proof: the JCS input, signed by `key`, under `kid`.
fn device_proof(key: &ed25519_dalek::SigningKey, kid: &str, handle: &str, attempt: Uuid) -> String {
    use ed25519_dalek::Signer;
    let thumbprint = URL_SAFE_NO_PAD.encode(Sha256::digest(key.verifying_key().as_bytes()));
    let handle: String = Sha256::digest(handle.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let input = format!(
        r#"{{"attempt_ref":"{attempt}","character_id":"{CHARACTER}","device_key_thumbprint":"{thumbprint}","purpose":"oteryn-native-recovery-pop-v1","recovery_handle_sha256":"{handle}"}}"#
    );
    let sig = URL_SAFE_NO_PAD.encode(key.sign(input.as_bytes()).to_bytes());
    format!(r#","device_proof":{{"kid":"{kid}","sig":"{sig}"}}}}"#)
}
/// A live session-class family with the synthetic device key (seed [7; 32]):
/// the key, its thumbprint, the family and the database time.
async fn keyed_family(
    r: &Recovery,
) -> Result<(ed25519_dalek::SigningKey, String, Uuid, i64), Box<dyn std::error::Error>> {
    let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let public = key.verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let family = Uuid::now_v7();
    let now = r.db.now().await?;
    sqlx::query("INSERT INTO device_sessions (id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at,persistence_class,device_public_key,device_key_thumbprint) SELECT $1,$2,$3,$4,game_auth_generation,native_security_generation,$5,$5,'session',$6,$7 FROM accounts WHERE id=$2")
        .bind(family).bind(r.account).bind(Uuid::now_v7()).bind(family.to_string())
        .bind(now + 600).bind(&public[..]).bind(&kid)
        .execute(&r.db.pool).await?;
    Ok((key, kid, family, now))
}
/// A native ticket issued by `family`.
async fn bound_ticket(r: &Recovery, family: Uuid) -> Result<String, Box<dyn std::error::Error>> {
    let native = r.db.native_ticket(r.account).await?;
    sqlx::query("UPDATE game_login_tickets SET device_family_id=$1 WHERE ticket_hash=$2")
        .bind(family)
        .bind(&Sha256::digest(native.as_bytes())[..])
        .execute(&r.db.pool)
        .await?;
    Ok(native)
}
fn with_proof(body: &str, proof: &str) -> String {
    let body = body.strip_suffix('}').expect("object");
    format!("{body}{proof}")
}

#[tokio::test]
async fn pg_recovery_with_device_binding_requires_a_proof_of_the_bound_family() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let bound = admission::Protocol3 {
        handles: admission::Handles::On {
            lifetime_seconds: 43200,
            require_device_binding: true,
            devices_enabled: true,
        },
        ..r.protocol3.clone()
    };
    let app = r.app_with(Some(bound), Some(r.issuer.clone()))?;
    let (key, kid, family, now) = keyed_family(&r).await?;
    let native = bound_ticket(&r, family).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let handle = success3(&body, &raw)?
        .recovery
        .expect("bound handle")
        .handle;
    let family_state = || async {
        sqlx::query_as::<_, (i64, Option<i64>)>(
            "SELECT idle_expires_at, revoked_at FROM device_sessions WHERE id=$1",
        )
        .bind(family)
        .fetch_one(&r.db.pool)
        .await
    };
    let before = family_state().await?;

    let attempt = Uuid::now_v7();
    let plain = recover_body(&handle, attempt, CHARACTER);
    let good = with_proof(&plain, &device_proof(&key, &kid, &handle, attempt));
    // Missing proof: malformed. Wrong kid, another key, another attempt's or
    // handle's signature: refused, and no attempt is recorded.
    assert_refused(
        &r.send(app.clone(), plain.clone()).await?,
        StatusCode::BAD_REQUEST,
        "NATIVE_RECOVERY_REQUEST_MALFORMED",
        "RETRY_LOGIN",
        attempt,
    );
    let other = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let other_kid = URL_SAFE_NO_PAD.encode(Sha256::digest(other.verifying_key().as_bytes()));
    for proof in [
        device_proof(&key, &other_kid, &handle, attempt),
        device_proof(&other, &kid, &handle, attempt),
        device_proof(&other, &other_kid, &handle, attempt),
        device_proof(&key, &kid, &handle, Uuid::now_v7()),
        device_proof(&key, &kid, UNKNOWN, attempt),
    ] {
        assert_authentication(
            &r.send(app.clone(), with_proof(&plain, &proof)).await?,
            attempt,
        );
    }
    // A handle bound to no family never recovers with binding on.
    r.db.execute("UPDATE native_recovery_handles SET device_family_id=NULL, device_key_thumbprint=NULL WHERE device_family_id IS NOT NULL")
        .await?;
    assert_authentication(&r.send(app.clone(), good.clone()).await?, attempt);
    sqlx::query("UPDATE native_recovery_handles SET device_family_id=$1, device_key_thumbprint=$2 WHERE revoked_at IS NULL")
        .bind(family)
        .bind(&kid)
        .execute(&r.db.pool)
        .await?;
    // An idle-expired family refuses even before any cleanup write.
    sqlx::query("UPDATE device_sessions SET idle_expires_at=$2 WHERE id=$1")
        .bind(family)
        .bind(now)
        .execute(&r.db.pool)
        .await?;
    assert_authentication(&r.send(app.clone(), good.clone()).await?, attempt);
    sqlx::query("UPDATE device_sessions SET idle_expires_at=$2 WHERE id=$1")
        .bind(family)
        .bind(before.0)
        .execute(&r.db.pool)
        .await?;
    assert_eq!(r.attempts().await?, 0, "refusals record no attempt");

    // The valid proof recovers, and reading the family never rotates or
    // touches it.
    let reply = r.send(app.clone(), good.clone()).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(family_state().await?, before);
    // A retry authenticates like a first request: a bad or missing proof
    // never returns the stored grant; the valid one does.
    assert_authentication(
        &r.send(
            app.clone(),
            with_proof(&plain, &device_proof(&other, &kid, &handle, attempt)),
        )
        .await?,
        attempt,
    );
    assert_refused(
        &r.send(app.clone(), plain.clone()).await?,
        StatusCode::BAD_REQUEST,
        "NATIVE_RECOVERY_REQUEST_MALFORMED",
        "RETRY_LOGIN",
        attempt,
    );
    let retry = r.send(app.clone(), good.clone()).await?;
    assert_eq!(retry.status, StatusCode::OK);
    assert_eq!(token(&retry.body)?.len(), token(&reply.body)?.len());
    assert_eq!(r.attempts().await?, 1);
    // Revoking the family revokes the handle (DB-6): the retry is refused.
    sqlx::query("UPDATE device_sessions SET revoked_at=$2 WHERE id=$1")
        .bind(family)
        .bind(now)
        .execute(&r.db.pool)
        .await?;
    assert_authentication(&r.send(app.clone(), good).await?, attempt);
    r.close().await
}

/// §19.15.1 with binding required and the device-session switch off: protocol
/// 3 logins work with `"recovery": null` and no handle, and recovery refuses
/// even a valid proof of a live family exactly as for an unknown handle.
#[tokio::test]
async fn pg_recovery_binding_without_devices_keeps_logins_and_refuses_recovery() -> TestResult {
    let Some(r) = Recovery::open().await? else {
        return Ok(());
    };
    let handles = |devices_enabled| admission::Handles::On {
        lifetime_seconds: 43200,
        require_device_binding: true,
        devices_enabled,
    };
    let on = r.app_with(
        Some(admission::Protocol3 {
            handles: handles(true),
            ..r.protocol3.clone()
        }),
        Some(r.issuer.clone()),
    )?;
    let off = r.app_with(
        Some(admission::Protocol3 {
            handles: handles(false),
            ..r.protocol3.clone()
        }),
        Some(r.issuer.clone()),
    )?;
    let (key, kid, family, _) = keyed_family(&r).await?;
    let native = bound_ticket(&r, family).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(on.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let handle = success3(&body, &raw)?
        .recovery
        .expect("bound handle")
        .handle;
    let attempt = Uuid::now_v7();
    let good = with_proof(
        &recover_body(&handle, attempt, CHARACTER),
        &device_proof(&key, &kid, &handle, attempt),
    );
    // Devices off: the valid proof is refused as an unknown handle is.
    assert_authentication(&r.send(off.clone(), good.clone()).await?, attempt);
    let unknown = with_proof(
        &recover_body(UNKNOWN, attempt, CHARACTER),
        &device_proof(&key, &kid, UNKNOWN, attempt),
    );
    assert_authentication(&r.send(on.clone(), unknown).await?, attempt);
    assert_eq!(r.attempts().await?, 0, "refusals record no attempt");
    // The same request recovers with devices on.
    let reply = r.send(on, good).await?;
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    // A protocol 3 login through the same family still succeeds, with
    // "recovery": null and no new handle.
    let handles_of = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM native_recovery_handles WHERE device_family_id=$1",
        )
        .bind(family)
        .fetch_one(&r.db.pool)
        .await
    };
    let before = handles_of().await?;
    let native = bound_ticket(&r, family).await?;
    let raw = login3(&native, Uuid::now_v7(), None);
    let (status, body) = admit(off, SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(success3(&body, &raw)?.recovery.is_none());
    assert_eq!(handles_of().await?, before);
    r.close().await
}
