#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
struct Authority {
    auth: Authorization,
    token: String,
    fail: bool,
}
impl NativeAuthority for Authority {
    fn enroll<'a>(
        &'a self,
        _: &'a mut PgConnection,
        bearer: &'a str,
    ) -> NativeFuture<'a, Enrollment> {
        Box::pin(async move {
            if bearer == "valid" {
                Ok(Enrollment {
                    authorization: self.auth.clone(),
                    token_id: self.token.clone(),
                })
            } else {
                Err(Failure::Denied)
            }
        })
    }
    fn verify<'a>(
        &'a self,
        _: &'a mut PgConnection,
        auth: &'a Authorization,
    ) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            if auth == &self.auth {
                Ok(())
            } else {
                Err(Failure::Denied)
            }
        })
    }
    fn use_native<'a>(
        &'a self,
        db: &'a mut PgConnection,
        _: &'a Authorization,
        _: Uuid,
        purpose: Purpose,
    ) -> NativeFuture<'a, NativeResult> {
        Box::pin(async move {
            if self.fail {
                return Err(Failure::Unavailable);
            }
            Ok(match purpose {
                Purpose::Characters => NativeResult::Characters(vec![Character {
                    character_id: Uuid::now_v7(),
                    world_id: Uuid::now_v7(),
                    name: "Player".into(),
                    availability: "AVAILABLE".into(),
                }]),
                Purpose::Ticket => NativeResult::Ticket {
                    ticket: "test-native-ticket".into(),
                    expires_in: 30,
                    expires_at: clock(db).await? + 30,
                },
            })
        })
    }
}
async fn setup()
-> std::result::Result<Option<(Sessions, Authorization)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL device tests skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t21_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    admin.close().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(schema)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let account_id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-only','active','en')").bind(account_id).bind(format!("{account_id}@example.test")).execute(&pool).await?;
    let auth = Authorization {
        account_id,
        client_id: Uuid::now_v7(),
        game_generation: 1,
        native_generation: 1,
    };
    let authority = Authority {
        auth: auth.clone(),
        token: Uuid::now_v7().to_string(),
        fail: false,
    };
    Ok(Some((
        Sessions::new(
            pool,
            Settings {
                enabled: true,
                environment: "testing".into(),
                ..Settings::default()
            },
            Arc::new(authority),
        ),
        auth,
    )))
}
#[tokio::test]
async fn database_enroll_rotate_ticket_revoke()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    assert!(matches!(
        s.enroll_test("bad", true).await,
        Err(Failure::Denied)
    ));
    assert!(matches!(
        s.enroll_test("valid", false).await,
        Err(Failure::Invalid)
    ));
    let c = s.enroll_test("valid", true).await.unwrap();
    assert!(valid_secret(c.secret()));
    assert!(matches!(
        s.enroll_test("valid", true).await,
        Err(Failure::Conflict)
    ));
    assert_eq!(
        s.rotate(c.secret(), Uuid::now_v7(), Purpose::Ticket).await,
        Err(Failure::Denied)
    );
    let response = s
        .rotate(c.secret(), a.client_id, Purpose::Characters)
        .await
        .unwrap();
    assert_eq!(response["characters"][0]["availability"], "AVAILABLE");
    assert_eq!(response["absolute_expires_at"], c.absolute_expires_at);
    let ticket = s
        .rotate(
            response["device_credential"].as_str().unwrap(),
            a.client_id,
            Purpose::Ticket,
        )
        .await
        .unwrap();
    assert_eq!(ticket["expires_in"], 30);
    s.revoke(c.secret(), a.client_id).await.unwrap();
    s.revoke(c.secret(), a.client_id).await.unwrap();
    assert_eq!(
        s.rotate(
            ticket["device_credential"].as_str().unwrap(),
            a.client_id,
            Purpose::Ticket
        )
        .await,
        Err(Failure::Denied)
    );
    let stored: Vec<u8> = sqlx::query_scalar(
        "SELECT secret_hash FROM device_session_credentials WHERE family_id=$1 AND sequence=0",
    )
    .bind(c.family_id)
    .fetch_one(&s.pool)
    .await?;
    assert_eq!(stored, hash(c.secret()));
    Ok(())
}
#[tokio::test]
async fn database_concurrent_reuse_allows_one_success_and_revokes_family()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let (first, second) = tokio::join!(
        s.rotate(c.secret(), a.client_id, Purpose::Ticket),
        s.rotate(c.secret(), a.client_id, Purpose::Ticket)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let success = first.or(second).unwrap();
    // The second use of the consumed predecessor revokes the whole family,
    // including the newest successor (accepted contract, rotation).
    assert_eq!(
        s.rotate(
            success["device_credential"].as_str().unwrap(),
            a.client_id,
            Purpose::Ticket
        )
        .await,
        Err(Failure::Denied)
    );
    Ok(())
}
#[tokio::test]
async fn database_expiry_and_generation_denials_commit()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for generation in [false, true] {
        let Some((s, a)) = setup().await? else {
            return Ok(());
        };
        let c = s.enroll_test("valid", true).await.unwrap();
        if generation {
            sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1").bind(a.account_id).execute(&s.pool).await?;
        } else {
            sqlx::query("UPDATE device_sessions SET idle_expires_at=0 WHERE id=$1")
                .bind(c.family_id)
                .execute(&s.pool)
                .await?;
        }
        assert_eq!(
            s.rotate(c.secret(), a.client_id, Purpose::Characters).await,
            Err(Failure::Denied)
        );
        let revoked: Option<i64> =
            sqlx::query_scalar("SELECT revoked_at FROM device_sessions WHERE id=$1")
                .bind(c.family_id)
                .fetch_one(&s.pool)
                .await?;
        assert!(revoked.is_some());
    }
    Ok(())
}
#[tokio::test]
async fn database_callback_failure_rolls_back()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((mut s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let original = s.authority.clone();
    s.authority = Arc::new(Authority {
        auth: a.clone(),
        token: String::new(),
        fail: true,
    });
    assert_eq!(
        s.rotate(c.secret(), a.client_id, Purpose::Ticket).await,
        Err(Failure::Unavailable)
    );
    s.authority = original;
    assert!(
        s.rotate(c.secret(), a.client_id, Purpose::Characters)
            .await
            .is_ok()
    );
    Ok(())
}
#[tokio::test]
async fn database_http_success_bad_input_and_source_limit()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let server = tls::tests::Harness::new(router(s)).await;
    let (status, _) = server
        .post_body(
            "",
            "Bearer valid",
            r#"{"protocol_version":1,"remember_device":false}"#,
        )
        .await;
    assert_eq!(status, 400);
    let body = enrollment_body("valid", a.client_id, true);
    let mut wrong = body.clone();
    wrong["enrollment_proof"] = URL_SAFE_NO_PAD.encode([0_u8; 64]).into();
    assert_eq!(server.post("", "Bearer valid", &wrong).await.0, 403);
    let (status, _) = server.post("", "Bearer valid", &body).await;
    assert_eq!(status, 200);
    assert_eq!(server.post("", "Bearer valid", &body).await.0, 409);
    for _ in 0..26 {
        server.post_body("", "Bearer valid", "{}").await;
    }
    let (status, body) = server.post_body("", "Bearer valid", "{}").await;
    assert_eq!(status, 429);
    assert_eq!(body["error"], "too_many_requests");
    Ok(())
}
#[tokio::test]
async fn default_and_production_are_unavailable()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    for settings in [
        Settings::default(),
        Settings {
            enabled: true,
            environment: "production".into(),
            ..Settings::default()
        },
    ] {
        let app: axum::Router = router(Sessions::new(
            pool.clone(),
            settings,
            Arc::new(UnavailableAuthority),
        ));
        let response = app
            .oneshot(Request::post("/api/v1/game-auth/device-sessions").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "no-store, private");
    }
    Ok(())
}

#[tokio::test]
async fn database_owner_revocation_and_suspension()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    assert_eq!(
        s.revoke_for_owner(Uuid::now_v7(), c.family_id).await,
        Err(Failure::Denied)
    );
    s.revoke_for_owner(a.account_id, c.family_id).await.unwrap();
    assert_eq!(
        s.rotate(c.secret(), a.client_id, Purpose::Ticket).await,
        Err(Failure::Denied)
    );
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(a.account_id)
        .execute(&s.pool)
        .await?;
    assert_eq!(
        s.rotate(c.secret(), a.client_id, Purpose::Ticket).await,
        Err(Failure::Denied)
    );
    s.revoke(c.secret(), a.client_id).await.unwrap();
    Ok(())
}

#[test]
fn protocol_one_fixture_bytes_and_proofs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/remembered-device-v1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for entry in manifest["files"].as_array().unwrap() {
        let bytes = std::fs::read(root.join(entry["file"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().unwrap());
        let digest: String = hash_bytes(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, entry["sha256"].as_str().unwrap());
    }
    let client = Uuid::parse_str(manifest["client_id"].as_str().unwrap()).unwrap();
    let bearer = manifest["synthetic_bearer"].as_str().unwrap();
    let unbound = std::fs::read(root.join("enroll.json")).unwrap();
    assert!(matches!(
        http::decode_enrollment(&unbound, false),
        Ok((true, None))
    ));
    let rotate = std::fs::read(root.join("rotate.json")).unwrap();
    assert_eq!(
        wire::decode_operation(&rotate, false).unwrap().client_id,
        client
    );
    for class in ["session", "remembered"] {
        let bytes = std::fs::read(root.join(format!("{class}-enroll.json"))).unwrap();
        let (remembered, key) = http::decode_enrollment(&bytes, false).unwrap();
        assert_eq!(remembered, class == "remembered");
        let key = key.unwrap();
        assert!(key.verify(bearer, client, remembered).is_ok());
        assert_eq!(
            key.verify("wrong", client, remembered),
            Err(Failure::Denied)
        );
        assert_eq!(
            key.verify(bearer, Uuid::now_v7(), remembered),
            Err(Failure::Denied)
        );
        assert_eq!(
            key.verify(bearer, client, !remembered),
            Err(Failure::Denied)
        );
    }
}

async fn native_setup()
-> std::result::Result<Option<(Sessions, Uuid, Uuid, String)>, Box<dyn std::error::Error>> {
    native_setup_with(false).await
}
/// `mfa` marks the account MFA-protected and approves through an MFA-satisfied browser session.
async fn native_setup_with(
    mfa: bool,
) -> std::result::Result<Option<(Sessions, Uuid, Uuid, String)>, Box<dyn std::error::Error>> {
    let Some((mut sessions, auth)) = setup().await? else {
        return Ok(None);
    };
    let client = auth.client_id;
    sqlx::query("UPDATE accounts SET email_verified_at=now(),login_mfa_required=$2 WHERE id=$1")
        .bind(auth.account_id)
        .bind(mfa)
        .execute(&sessions.pool)
        .await?;
    sqlx::query("INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket) VALUES($1,true,true,true,true)").bind(client).execute(&sessions.pool).await?;
    let config = crate::oauth::Config {
        enabled: true,
        clients: vec![crate::oauth::Client {
            client_id: client.to_string(),
            redirect_uris: vec!["http://127.0.0.1:41000/callback".into()],
        }],
    };
    let verifier = "v".repeat(43);
    let request = crate::oauth::Authorization {
        response_type: "code".into(),
        client_id: client.to_string(),
        redirect_uri: "http://127.0.0.1:41000/callback".into(),
        scope: "game:ticket".into(),
        state: "test-state-for-browser".into(),
        code_challenge: URL_SAFE_NO_PAD.encode(hash(&verifier)),
        code_challenge_method: "S256".into(),
    };
    let code = if mfa {
        crate::oauth::approve(&sessions.pool, &config, auth.account_id, true, &request)
            .await?
            .ok_or("MFA session was refused")?
    } else {
        crate::oauth::authorize(&sessions.pool, &config, auth.account_id, &request).await?
    };
    let tokens = crate::oauth::exchange(
        &sessions.pool,
        &config,
        &crate::oauth::TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: client.to_string(),
            redirect_uri: request.redirect_uri,
            code,
            code_verifier: verifier,
            ..Default::default()
        },
    )
    .await?;
    sessions.authority = Arc::new(PlatformAuthority {
        oauth: config,
        projection_authority: "test-source".into(),
    });
    Ok(Some((
        sessions,
        auth.account_id,
        client,
        tokens.access_token,
    )))
}

fn signed_key(bearer: &str, client: Uuid, remembered: bool) -> DeviceKey {
    use ed25519_dalek::{Signer, SigningKey};
    let signing = SigningKey::from_bytes(&[7; 32]);
    let public = signing.verifying_key().to_bytes();
    let digest: String = hash(bearer).iter().map(|b| format!("{b:02x}")).collect();
    let input = format!(
        "{{\"access_token_sha256\":\"{digest}\",\"client_id\":\"{client}\",\"persistence\":\"{}\",\"public_key\":\"{}\",\"purpose\":\"oteryn-native-device-enroll-v1\"}}",
        if remembered { "remembered" } else { "session" },
        URL_SAFE_NO_PAD.encode(public)
    );
    DeviceKey {
        public_key: public.to_vec(),
        signature: signing.sign(input.as_bytes()).to_bytes().to_vec(),
    }
}

#[tokio::test]
async fn database_native_oauth_enrollment_and_existing_ticket_issuer()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    assert!(matches!(
        s.enroll_test("invalid", true).await,
        Err(Failure::Denied)
    ));
    let enrolled = s.enroll_test(&bearer, true).await.unwrap();
    assert!(matches!(
        s.enroll_test(&bearer, true).await,
        Err(Failure::Conflict)
    ));
    let result = s
        .rotate(enrolled.secret(), client, Purpose::Ticket)
        .await
        .unwrap();
    let ticket = result["ticket"].as_str().unwrap();
    let row: (Uuid,Uuid,String,String) = sqlx::query_as("SELECT account_id,device_family_id,audience,native_generation FROM game_login_tickets WHERE ticket_hash=$1 AND used_at IS NULL AND expires_at>clock_timestamp()")
        .bind(hash(ticket)).fetch_one(&s.pool).await?;
    assert_eq!(row.0, account);
    assert_eq!(row.1, enrolled.family_id);
    assert_eq!(row.2, "oteryn-native-game-gateway");
    assert_eq!(row.3, "2");
    assert!(
        crate::oauth::verify_access(
            &s.pool,
            &crate::oauth::Config {
                enabled: true,
                clients: vec![crate::oauth::Client {
                    client_id: client.to_string(),
                    redirect_uris: vec![]
                }]
            },
            &bearer
        )
        .await
        .is_ok()
    );
    assert_eq!(
        s.rotate(enrolled.secret(), client, Purpose::Ticket).await,
        Err(Failure::Denied)
    );
    assert_eq!(
        s.rotate(
            result["device_credential"].as_str().unwrap(),
            client,
            Purpose::Ticket
        )
        .await,
        Err(Failure::Denied)
    );
    Ok(())
}

#[tokio::test]
async fn database_native_session_class_proof_and_fixed_deadline()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, _, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let wrong = signed_key("wrong", client, false);
    assert!(matches!(
        s.enroll_class(&bearer, false, Some(wrong)).await,
        Err(Failure::Forbidden)
    ));
    let key = signed_key(&bearer, client, false);
    let enrolled = s.enroll_class(&bearer, false, Some(key)).await.unwrap();
    assert_eq!(enrolled.absolute_expires_at, enrolled.idle_expires_at);
    let now = {
        let mut db = s.pool.acquire().await?;
        clock(&mut db).await.unwrap()
    };
    assert!((43195..=43200).contains(&(enrolled.absolute_expires_at - now)));
    let row: (String,i64) = sqlx::query_as("SELECT persistence_class,octet_length(device_public_key)::bigint FROM device_sessions WHERE id=$1").bind(enrolled.family_id).fetch_one(&s.pool).await?;
    assert_eq!(row, ("session".into(), 32));
    let response = s
        .rotate(enrolled.secret(), client, Purpose::Ticket)
        .await
        .unwrap();
    assert_eq!(response["idle_expires_at"], enrolled.idle_expires_at);
    assert!(matches!(
        s.enroll_test(&bearer, true).await,
        Err(Failure::Conflict)
    ));
    Ok(())
}

#[tokio::test]
async fn database_native_password_mfa_suspension_and_client_revocation()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for change in [
        "password",
        "prefenced-password",
        "mfa",
        "mfa-key",
        "suspension",
        "client",
        "generation",
    ] {
        let Some((s, account, client, bearer)) = native_setup().await? else {
            return Ok(());
        };
        let enrolled = s.enroll_test(&bearer, true).await.unwrap();
        match change {
            "password" => {
                sqlx::query("UPDATE accounts SET password_hash='changed-test-hash' WHERE id=$1")
                    .bind(account)
                    .execute(&s.pool)
                    .await?;
            }
            "prefenced-password" => {
                let mut tx = s.pool.begin().await?;
                sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1").bind(account).execute(&mut *tx).await?;
                sqlx::query("UPDATE accounts SET password_hash='changed-test-hash' WHERE id=$1")
                    .bind(account)
                    .execute(&mut *tx)
                    .await?;
                let generation: i64 = sqlx::query_scalar(
                    "SELECT native_security_generation FROM accounts WHERE id=$1",
                )
                .bind(account)
                .fetch_one(&mut *tx)
                .await?;
                assert_eq!(generation, 3);
                tx.commit().await?;
            }
            "mfa" => {
                sqlx::query("UPDATE accounts SET login_mfa_required=true WHERE id=$1")
                    .bind(account)
                    .execute(&s.pool)
                    .await?;
            }
            "mfa-key" => {
                sqlx::query(
                    "INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,false)",
                )
                .bind(account)
                .bind(vec![1u8; 32])
                .execute(&s.pool)
                .await?;
                sqlx::query("UPDATE account_mfa_totp SET confirmed=true WHERE account_id=$1")
                    .bind(account)
                    .execute(&s.pool)
                    .await?;
            }
            "suspension" => {
                sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
                    .bind(account)
                    .execute(&s.pool)
                    .await?;
            }
            "generation" => {
                sqlx::query(
                    "UPDATE accounts SET game_auth_generation=game_auth_generation+1 WHERE id=$1",
                )
                .bind(account)
                .execute(&s.pool)
                .await?;
            }
            _ => {
                sqlx::query(
                    "UPDATE oauth_native_clients SET native_ticket=false WHERE client_id=$1",
                )
                .bind(client)
                .execute(&s.pool)
                .await?;
            }
        }
        assert_eq!(
            s.rotate(enrolled.secret(), client, Purpose::Ticket).await,
            Err(Failure::Denied),
            "{change}"
        );
        let revoked: Option<i64> =
            sqlx::query_scalar("SELECT revoked_at FROM device_sessions WHERE id=$1")
                .bind(enrolled.family_id)
                .fetch_one(&s.pool)
                .await?;
        assert!(revoked.is_some(), "{change}");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_login_tickets")
            .fetch_one(&s.pool)
            .await?;
        assert_eq!(count, 0);
    }
    Ok(())
}

#[tokio::test]
async fn database_native_mfa_account_enrolls_and_issues_ticket()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup_with(true).await? else {
        return Ok(());
    };
    let required: bool = sqlx::query_scalar("SELECT login_mfa_required FROM accounts WHERE id=$1")
        .bind(account)
        .fetch_one(&s.pool)
        .await?;
    assert!(required);
    let enrolled = s.enroll_test(&bearer, true).await.unwrap();
    let ticket = s
        .rotate(enrolled.secret(), client, Purpose::Ticket)
        .await
        .unwrap();
    assert!(ticket["ticket"].is_string(), "{ticket}");
    Ok(())
}

#[tokio::test]
async fn database_native_character_projection_and_failure_rollback()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let enrolled = s.enroll_test(&bearer, true).await.unwrap();
    assert_eq!(
        s.rotate(enrolled.secret(), client, Purpose::Characters)
            .await,
        Err(Failure::Unavailable)
    );
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&s.pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)").bind(account).bind(vec![0u8;32]).execute(&s.pool).await?;
    let character = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','AVAILABLE')").bind(account).bind(character).bind(Uuid::now_v7()).execute(&s.pool).await?;
    let response = s
        .rotate(enrolled.secret(), client, Purpose::Characters)
        .await
        .unwrap();
    assert_eq!(
        response["characters"][0]["character_id"],
        character.to_string()
    );
    sqlx::query("UPDATE account_character_snapshots SET invalid=true WHERE account_id=$1")
        .bind(account)
        .execute(&s.pool)
        .await?;
    assert_eq!(
        s.rotate(
            response["device_credential"].as_str().unwrap(),
            client,
            Purpose::Characters
        )
        .await,
        Err(Failure::Unavailable)
    );
    Ok(())
}

#[tokio::test]
async fn database_native_normal_ticket_consumes_oauth_but_device_continues()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, _, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let enrollment = s.enroll_test(&bearer, true).await.unwrap();
    let authority = PlatformAuthority {
        oauth: crate::oauth::Config {
            enabled: true,
            clients: vec![crate::oauth::Client {
                client_id: client.to_string(),
                redirect_uris: vec![],
            }],
        },
        projection_authority: "test-source".into(),
    };
    let tickets = crate::tickets::Tickets::new(
        s.pool.clone(),
        crate::tickets::Settings {
            enabled: true,
            nonproduction: true,
        },
        Arc::new(authority),
    );
    let issued = tickets.issue(&bearer).await.unwrap();
    let family: Uuid =
        sqlx::query_scalar("SELECT device_family_id FROM game_login_tickets WHERE ticket_hash=$1")
            .bind(hash(&issued.ticket))
            .fetch_one(&s.pool)
            .await?;
    assert_eq!(family, enrollment.family_id);
    assert!(matches!(
        tickets.issue(&bearer).await,
        Err(crate::tickets::Error::Unauthorized)
    ));
    let revoked: bool = sqlx::query_scalar("SELECT f.revoked_at IS NOT NULL FROM oauth_token_families f JOIN oauth_tokens t ON t.family_id=f.id WHERE t.access_hash=$1").bind(hash(&bearer)).fetch_one(&s.pool).await?;
    assert!(revoked);
    assert!(
        s.rotate(enrollment.secret(), client, Purpose::Ticket)
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn database_native_original_oauth_generations_are_required()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for native in [false, true] {
        let Some((s, account, _, bearer)) = native_setup().await? else {
            return Ok(());
        };
        if native {
            sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1").bind(account).execute(&s.pool).await?;
        } else {
            sqlx::query(
                "UPDATE accounts SET game_auth_generation=game_auth_generation+1 WHERE id=$1",
            )
            .bind(account)
            .execute(&s.pool)
            .await?;
        }
        assert!(matches!(
            s.enroll_test(&bearer, true).await,
            Err(Failure::Denied)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn database_client_configuration_and_shared_lock()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((mut s, _, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let credential = s.enroll_test(&bearer, true).await.unwrap();
    s.authority = Arc::new(PlatformAuthority {
        oauth: crate::oauth::Config::default(),
        projection_authority: String::new(),
    });
    assert_eq!(
        s.rotate(credential.secret(), client, Purpose::Ticket).await,
        Err(Failure::Unavailable)
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM device_sessions WHERE id=$1")
            .bind(credential.family_id)
            .fetch_one(&s.pool)
            .await?;
    assert!(!revoked);
    let mut first = s.pool.begin().await?;
    assert_eq!(
        native::lock_client(&mut first, &crate::oauth::Config::default(), client).await,
        Err(Failure::Unavailable)
    );
    let config = crate::oauth::Config {
        enabled: true,
        clients: vec![crate::oauth::Client {
            client_id: client.to_string(),
            redirect_uris: vec![],
        }],
    };
    let mut ambiguous = config.clone();
    ambiguous.clients.extend(config.clients.clone());
    assert_eq!(
        native::lock_client(&mut first, &ambiguous, client).await,
        Err(Failure::Unavailable)
    );
    assert_eq!(
        native::lock_client(&mut first, &config, Uuid::now_v7()).await,
        Err(Failure::Denied)
    );
    native::lock_client(&mut first, &config, client)
        .await
        .unwrap();
    let mut second = s.pool.begin().await?;
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        native::lock_client(&mut second, &config, client),
    )
    .await?
    .unwrap();
    Ok(())
}

#[tokio::test]
async fn database_device_key_reuse_refuses_without_family()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let key = signed_key(&bearer, client, false);
    let thumbprint = URL_SAFE_NO_PAD.encode(hash_bytes(&key.public_key));
    sqlx::query("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at,device_public_key,device_key_thumbprint) VALUES($1,$2,$3,$4,1,1,9999999999,9999999999,$5,$6)")
        .bind(Uuid::now_v7()).bind(account).bind(client).bind(Uuid::now_v7().to_string()).bind(&key.public_key).bind(thumbprint).execute(&s.pool).await?;
    assert!(matches!(
        s.enroll_class(&bearer, false, Some(key)).await,
        Err(Failure::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM device_sessions")
        .fetch_one(&s.pool)
        .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[tokio::test]
async fn database_source_limit_purges_stale_windows()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, _)) = setup().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO device_session_source_limits(source_hash,window_start,attempts) VALUES($1,0,1)")
        .bind(hash("expired-source")).execute(&s.pool).await?;
    http::throttle(&s, "192.0.2.1").await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM device_session_source_limits WHERE window_start=0",
    )
    .fetch_one(&s.pool)
    .await?;
    assert_eq!(count, 0);
    Ok(())
}

// Existing service tests use real signatures too; no production bypass is added.
impl Sessions {
    async fn enroll_test(&self, bearer: &str, consent: bool) -> Result<Credential> {
        if !consent {
            return self.enroll(bearer, consent).await;
        }
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let auth = self.authority.enroll(&mut tx, bearer).await?.authorization;
        tx.rollback().await.map_err(unavailable)?;
        self.enroll_class(
            bearer,
            consent,
            Some(signed_key(bearer, auth.client_id, consent)),
        )
        .await
    }
}
fn enrollment_body(bearer: &str, client: Uuid, remembered: bool) -> serde_json::Value {
    let key = signed_key(bearer, client, remembered);
    serde_json::to_value(wire::EnrollmentRequest {
        protocol_version: 1,
        remember_device: remembered,
        device_public_key: Some(URL_SAFE_NO_PAD.encode(key.public_key)),
        enrollment_proof: Some(URL_SAFE_NO_PAD.encode(key.signature)),
    })
    .unwrap()
}
/// Oteryn-Game `crates/platform-client/src/native_login/owner_characters/device_transport.rs`
/// at this commit. Its serde_json has no `preserve_order`, so `json!` bodies are
/// sent with members in lexical order.
const GAME_MAIN_SHA: &str = "29eb08d392182a65da0304d8d29019588e1e7186";
/// `enroll_device(remembered = true)`: `json!({"protocol_version":1,"remember_device":true})`.
const GAME_UNBOUND_ENROLLMENT: &str = r#"{"protocol_version":1,"remember_device":true}"#;
/// `rotate_device` and `revoke_device`:
/// `json!({"protocol_version":1,"client_id":self.oauth_client_id})`.
fn game_operation(client: Uuid) -> String {
    format!(r#"{{"client_id":"{client}","protocol_version":1}}"#)
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/contracts/fixtures/remembered-device-v1")
            .join(name),
    )
    .unwrap()
}
fn fixture_value(name: &str) -> serde_json::Value {
    serde_json::from_slice(&fixture(name)).unwrap()
}
/// The Game client parser: exactly five credential members plus the
/// operation's own members (`ResponseBody::parse`, `deny_unknown_fields`).
fn assert_game_members(value: &serde_json::Value, extra: &[&str]) {
    let object = value.as_object().unwrap();
    let mut expected = vec![
        "protocol_version",
        "device_credential",
        "family_id",
        "absolute_expires_at",
        "idle_expires_at",
    ];
    expected.extend_from_slice(extra);
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    expected.sort_unstable();
    assert_eq!(keys, expected, "{value}");
}

#[test]
fn devices_rotation_bad_input_refused() {
    let client = "018f1234-1234-7123-8123-123456789abc";
    for body in [
        r#"{"protocol_version":1,"client_id":"invalid"}"#.to_owned(),
        format!(r#"{{"protocol_version":1,"client_id":"{client}","rotation_proof":null}}"#),
        format!(
            r#"{{"protocol_version":1,"client_id":"{client}","rotation_proof":{{"nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8"}}}}"#
        ),
        format!(r#"{{"protocol_version":1,"client_id":"{client}","owner":"x"}}"#),
        format!(r#"{{"protocol_version":1,"protocol_version":1,"client_id":"{client}"}}"#),
        format!(r#"{{"protocol_version":1,"client_id":"{client}","client_id":"{client}"}}"#),
        format!(r#"{{"protocol_version":"1","client_id":"{client}"}}"#),
        format!(r#"{{"protocol_version":2,"client_id":"{client}"}}"#),
        format!(
            r#"{{"protocol_version":1,"client_id":"{}"}}"#,
            client.to_uppercase()
        ),
        format!(
            r#"{{"protocol_version":1,"client_id":"{}"}}"#,
            client.replace('-', "")
        ),
        r#"{"protocol_version":1}"#.to_owned(),
        r#"[]"#.to_owned(),
    ] {
        assert!(
            matches!(
                wire::decode_operation(body.as_bytes(), false),
                Err(Failure::Invalid)
            ),
            "{body}"
        );
    }
    let valid = fixture("rotate.json");
    assert!(wire::decode_operation(&valid, false).is_ok());
    assert!(matches!(
        wire::decode_operation(&valid, true),
        Err(Failure::Invalid)
    ));
}

#[test]
fn devices_accepted_contract_fixtures_match_wire_types() {
    fn same_bytes<T: serde::Serialize + serde::de::DeserializeOwned>(name: &str) {
        let bytes = fixture(name);
        let typed: T = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&typed).unwrap(), bytes, "{name}");
    }
    same_bytes::<wire::EnrollmentRequest>("enroll.json");
    same_bytes::<wire::EnrollmentRequest>("remembered-enroll.json");
    same_bytes::<wire::EnrollmentRequest>("session-enroll.json");
    same_bytes::<wire::OperationRequest>("rotate.json");
    same_bytes::<wire::CredentialResponse>("enroll-response.json");
    same_bytes::<wire::TicketResponse>("ticket-response.json");
    same_bytes::<wire::RevokeResponse>("revoke-response.json");
    same_bytes::<wire::ErrorResponse>("invalid.json");
    same_bytes::<wire::ErrorResponse>("denied.json");
    same_bytes::<wire::ErrorResponse>("throttled.json");
    assert_game_members(&fixture_value("enroll-response.json"), &[]);
    assert_game_members(
        &fixture_value("ticket-response.json"),
        &["ticket", "expires_in", "expires_at"],
    );
}

#[test]
fn devices_game_client_exact_bytes_decode() {
    assert_eq!(GAME_MAIN_SHA.len(), 40);
    let manifest = fixture_value("manifest.json");
    let client = Uuid::parse_str(manifest["client_id"].as_str().unwrap()).unwrap();
    assert_eq!(
        fixture("enroll.json"),
        GAME_UNBOUND_ENROLLMENT.as_bytes(),
        "Game's unbound enrollment is the accepted contract's original request"
    );
    assert!(matches!(
        http::decode_enrollment(GAME_UNBOUND_ENROLLMENT.as_bytes(), false),
        Ok((true, None))
    ));
    let operation = wire::decode_operation(game_operation(client).as_bytes(), false).unwrap();
    assert_eq!(operation.client_id, client);
    // `SessionKey::enrollment`: the same members as the fixture, lexical order.
    let session = fixture_value("session-enroll.json");
    let game_session = format!(
        r#"{{"device_public_key":{},"enrollment_proof":{},"protocol_version":1,"remember_device":false}}"#,
        session["device_public_key"], session["enrollment_proof"]
    );
    let (remembered, key) = http::decode_enrollment(game_session.as_bytes(), false).unwrap();
    assert!(!remembered);
    key.unwrap()
        .verify(
            manifest["synthetic_bearer"].as_str().unwrap(),
            client,
            false,
        )
        .unwrap();
}

#[tokio::test]
async fn devices_e2e_unbound_lifecycle_with_game_client_bytes()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&s.pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)").bind(account).bind(vec![0u8;32]).execute(&s.pool).await?;
    let character = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','AVAILABLE')").bind(account).bind(character).bind(Uuid::now_v7()).execute(&s.pool).await?;
    let server = tls::tests::Harness::new(router(s.clone())).await;
    let bearer_header = format!("Bearer {bearer}");
    let (status, enrolled) = server
        .post_body("", &bearer_header, GAME_UNBOUND_ENROLLMENT)
        .await;
    assert_eq!(status, 200, "{enrolled}");
    assert_game_members(&enrolled, &[]);
    let family: Uuid = serde_json::from_value(enrolled["family_id"].clone())?;
    // The original request creates an unbound remembered family.
    let row: (String, Option<Vec<u8>>, Option<String>) = sqlx::query_as("SELECT persistence_class,device_public_key,device_key_thumbprint FROM device_sessions WHERE id=$1").bind(family).fetch_one(&s.pool).await?;
    assert_eq!(row, ("remembered".into(), None, None));
    let (status, conflict) = server
        .post_body("", &bearer_header, GAME_UNBOUND_ENROLLMENT)
        .await;
    assert_eq!(status, 409, "{conflict}");
    let operation = game_operation(client);
    let mut current = enrolled;
    for suffix in ["/native-characters", "/tickets", "/native-characters"] {
        let authorization = format!(
            "OterynDevice {}",
            current["device_credential"].as_str().unwrap()
        );
        let (status, next) = server.post_body(suffix, &authorization, &operation).await;
        assert_eq!(status, 200, "{next}");
        assert_ne!(current["device_credential"], next["device_credential"]);
        assert_eq!(current["family_id"], next["family_id"]);
        assert_eq!(current["absolute_expires_at"], next["absolute_expires_at"]);
        if suffix == "/tickets" {
            assert_game_members(&next, &["ticket", "expires_in", "expires_at"]);
            // The ticket links the unbound family; the §19 issuer reads it and
            // finds no device key, so required binding yields no handle.
            let row: (Uuid, Uuid) = sqlx::query_as(
                "SELECT account_id,device_family_id FROM game_login_tickets WHERE ticket_hash=$1",
            )
            .bind(hash(next["ticket"].as_str().unwrap()))
            .fetch_one(&s.pool)
            .await?;
            assert_eq!(row, (account, family));
        } else {
            assert_game_members(&next, &["characters"]);
            assert_eq!(next["characters"][0]["character_id"], character.to_string());
        }
        current = next;
    }
    let authorization = format!(
        "OterynDevice {}",
        current["device_credential"].as_str().unwrap()
    );
    for _ in 0..2 {
        let (status, value) = server
            .post_body("/revoke", &authorization, &operation)
            .await;
        assert_eq!(status, 200);
        assert_eq!(value, fixture_value("revoke-response.json"));
    }
    let (status, value) = server
        .post_body("/tickets", &authorization, &operation)
        .await;
    assert_eq!((status, value), (401, fixture_value("denied.json")));
    Ok(())
}

#[tokio::test]
async fn database_bound_enrollment_rotates_without_proof()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for remembered in [false, true] {
        let Some((s, _, client, bearer)) = native_setup().await? else {
            return Ok(());
        };
        let server = tls::tests::Harness::new(router(s.clone())).await;
        let (status, enrolled) = server
            .post(
                "",
                &format!("Bearer {bearer}"),
                &enrollment_body(&bearer, client, remembered),
            )
            .await;
        assert_eq!(status, 200, "{enrolled}");
        assert_game_members(&enrolled, &[]);
        let family: Uuid = serde_json::from_value(enrolled["family_id"].clone())?;
        let row: (String, bool) = sqlx::query_as("SELECT persistence_class,device_key_thumbprint IS NOT NULL FROM device_sessions WHERE id=$1").bind(family).fetch_one(&s.pool).await?;
        let class = if remembered { "remembered" } else { "session" };
        assert_eq!(row, (class.into(), true));
        let (status, next) = server
            .post_body(
                "/tickets",
                &format!(
                    "OterynDevice {}",
                    enrolled["device_credential"].as_str().unwrap()
                ),
                &game_operation(client),
            )
            .await;
        assert_eq!(status, 200, "{next}");
        assert_game_members(&next, &["ticket", "expires_in", "expires_at"]);
    }
    Ok(())
}

#[tokio::test]
async fn database_http_refuses_proposed_and_malformed_bodies()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let server = tls::tests::Harness::new(router(s.clone())).await;
    let invalid = (400, fixture_value("invalid.json"));
    let too_large = (413, fixture_value("invalid.json"));
    let oversized = format!(
        r#"{{"protocol_version":1,"remember_device":true{}}}"#,
        " ".repeat(1024)
    );
    let key = fixture_value("remembered-enroll.json")["device_public_key"].clone();
    for (suffix, body, expected) in [
        (
            "",
            format!(r#"{{"protocol_version":1,"remember_device":true,"device_public_key":{key}}}"#),
            &invalid,
        ),
        (
            "",
            r#"{"protocol_version":1,"remember_device":false}"#.to_owned(),
            &invalid,
        ),
        ("?a=1", GAME_UNBOUND_ENROLLMENT.to_owned(), &invalid),
        ("", oversized, &too_large),
    ] {
        assert_eq!(
            &server.post_body(suffix, "Bearer valid", &body).await,
            expected,
            "{suffix} {body}"
        );
    }
    let (status, enrolled) = server
        .post_body("", "Bearer valid", GAME_UNBOUND_ENROLLMENT)
        .await;
    assert_eq!(status, 200, "{enrolled}");
    let authorization = format!(
        "OterynDevice {}",
        enrolled["device_credential"].as_str().unwrap()
    );
    let client = a.client_id;
    // The PROPOSED client contract's `rotation_proof` is now an unknown member.
    let proposed = format!(
        r#"{{"protocol_version":1,"client_id":"{client}","rotation_proof":{{"nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8","session_id":"{}","timestamp":1794405600,"device_public_key":{key},"signature":"wCFa-qSVXu2_OTha678f8AHjtd2-eJt-S6Vbf3Osiv8aVnItvXBPmZS3-lv6khs0002zAGppqbQj_vi9VhSRBw"}}}}"#,
        enrolled["family_id"].as_str().unwrap()
    );
    let padded = format!("{}{}", game_operation(client), " ".repeat(1024));
    for (suffix, body, expected) in [
        ("/tickets", proposed.clone(), &invalid),
        ("/native-characters", proposed.clone(), &invalid),
        ("/revoke", proposed, &invalid),
        (
            "/tickets",
            format!(r#"{{"protocol_version":1,"client_id":"{client}","owner":"x"}}"#),
            &invalid,
        ),
        ("/tickets?a=1", game_operation(client), &invalid),
        ("/revoke?a=1", game_operation(client), &invalid),
        ("/tickets", padded, &too_large),
    ] {
        assert_eq!(
            &server.post_body(suffix, &authorization, &body).await,
            expected,
            "{suffix} {body}"
        );
    }
    // Refused bodies consumed nothing: the credential still rotates once.
    let (status, next) = server
        .post_body("/tickets", &authorization, &game_operation(client))
        .await;
    assert_eq!(status, 200, "{next}");
    assert_game_members(&next, &["ticket", "expires_in", "expires_at"]);
    let family: Uuid = serde_json::from_value(enrolled["family_id"].clone())?;
    let revoked: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM device_sessions WHERE id=$1")
            .bind(family)
            .fetch_one(&s.pool)
            .await?;
    assert!(revoked.is_none());
    Ok(())
}
