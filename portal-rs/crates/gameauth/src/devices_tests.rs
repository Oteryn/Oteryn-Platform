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
        s.rotate_test(c.secret(), Uuid::now_v7(), Purpose::Ticket)
            .await,
        Err(Failure::Denied)
    );
    let response = s
        .rotate_test(c.secret(), a.client_id, Purpose::Characters)
        .await
        .unwrap();
    assert_eq!(response["characters"][0]["availability"], "AVAILABLE");
    assert_eq!(response["absolute_expires_at"], c.absolute_expires_at);
    let ticket = s
        .rotate_test(
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
        s.rotate_test(
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
async fn database_concurrent_proof_replay_preserves_successor()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let proof = proof_for(&s, c.secret(), 7, 0).await;
    let (first, second) = tokio::join!(
        s.rotate_with_proof(c.secret(), a.client_id, Purpose::Ticket, &proof),
        s.rotate_with_proof(c.secret(), a.client_id, Purpose::Ticket, &proof)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let success = first.or(second).unwrap();
    assert!(
        s.rotate_test(
            success["device_credential"].as_str().unwrap(),
            a.client_id,
            Purpose::Ticket
        )
        .await
        .is_ok()
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
            s.rotate_test(c.secret(), a.client_id, Purpose::Characters)
                .await,
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
        s.rotate_test(c.secret(), a.client_id, Purpose::Ticket)
            .await,
        Err(Failure::Unavailable)
    );
    s.authority = original;
    assert!(
        s.rotate_test(c.secret(), a.client_id, Purpose::Characters)
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
        s.rotate_test(c.secret(), a.client_id, Purpose::Ticket)
            .await,
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
        s.rotate_test(c.secret(), a.client_id, Purpose::Ticket)
            .await,
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
        .rotate_test(enrolled.secret(), client, Purpose::Ticket)
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
        s.rotate_test(enrolled.secret(), client, Purpose::Ticket)
            .await,
        Err(Failure::Denied)
    );
    assert_eq!(
        s.rotate_test(
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
        .rotate_test(enrolled.secret(), client, Purpose::Ticket)
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
            s.rotate_test(enrolled.secret(), client, Purpose::Ticket)
                .await,
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
        .rotate_test(enrolled.secret(), client, Purpose::Ticket)
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
        s.rotate_test(enrolled.secret(), client, Purpose::Characters)
            .await,
        Err(Failure::Unavailable)
    );
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&s.pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)").bind(account).bind(vec![0u8;32]).execute(&s.pool).await?;
    let character = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','AVAILABLE')").bind(account).bind(character).bind(Uuid::now_v7()).execute(&s.pool).await?;
    let response = s
        .rotate_test(enrolled.secret(), client, Purpose::Characters)
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
        s.rotate_test(
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
        s.rotate_test(enrollment.secret(), client, Purpose::Ticket)
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
        s.rotate_test(credential.secret(), client, Purpose::Ticket)
            .await,
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
    async fn rotate_test(
        &self,
        secret: &str,
        client: Uuid,
        purpose: Purpose,
    ) -> Result<serde_json::Value> {
        let proof = proof_for(self, secret, 7, 0).await;
        self.rotate_with_proof(secret, client, purpose, &proof)
            .await
    }
}
fn enrollment_body(bearer: &str, client: Uuid, remembered: bool) -> serde_json::Value {
    let key = signed_key(bearer, client, remembered);
    serde_json::to_value(wire::EnrollmentRequest {
        protocol_version: 1,
        remember_device: remembered,
        device_public_key: URL_SAFE_NO_PAD.encode(key.public_key),
        enrollment_proof: URL_SAFE_NO_PAD.encode(key.signature),
    })
    .unwrap()
}
async fn proof_for(s: &Sessions, secret: &str, key_byte: u8, offset: i64) -> RotationProof {
    use ed25519_dalek::{Signer, SigningKey};
    let row: (Uuid,String,i64) = sqlx::query_as("SELECT f.id,f.rotation_nonce,floor(extract(epoch FROM clock_timestamp()))::bigint FROM device_sessions f JOIN device_session_credentials c ON c.family_id=f.id WHERE c.secret_hash=$1")
        .bind(hash(secret)).fetch_one(&s.pool).await.unwrap();
    let signing = SigningKey::from_bytes(&[key_byte; 32]);
    let mut proof = RotationProof {
        nonce: row.1,
        session_id: row.0,
        timestamp: row.2 + offset,
        device_public_key: URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes()),
        signature: String::new(),
    };
    proof.signature = URL_SAFE_NO_PAD.encode(signing.sign(proof.message().as_bytes()).to_bytes());
    proof
}

async fn unrelated_family(s: &Sessions, auth: &Authorization) -> Credential {
    use ed25519_dalek::{Signer, SigningKey};
    let other = Sessions::new(
        s.pool.clone(),
        s.settings.clone(),
        Arc::new(Authority {
            auth: auth.clone(),
            token: Uuid::now_v7().to_string(),
            fail: false,
        }),
    );
    let signing = SigningKey::from_bytes(&[8; 32]);
    let public = signing.verifying_key().to_bytes();
    let digest: String = hash("valid").iter().map(|b| format!("{b:02x}")).collect();
    let message = format!(
        "{{\"access_token_sha256\":\"{digest}\",\"client_id\":\"{}\",\"persistence\":\"remembered\",\"public_key\":\"{}\",\"purpose\":\"oteryn-native-device-enroll-v1\"}}",
        auth.client_id,
        URL_SAFE_NO_PAD.encode(public)
    );
    other
        .enroll_class(
            "valid",
            true,
            Some(DeviceKey {
                public_key: public.to_vec(),
                signature: signing.sign(message.as_bytes()).to_bytes().to_vec(),
            }),
        )
        .await
        .unwrap()
}
async fn assert_live(s: &Sessions, family: Uuid) {
    let revoked: Option<i64> =
        sqlx::query_scalar("SELECT revoked_at FROM device_sessions WHERE id=$1")
            .bind(family)
            .fetch_one(&s.pool)
            .await
            .unwrap();
    assert!(revoked.is_none());
}
#[tokio::test]
async fn devices_l3_rotation_requires_proof() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    assert!(matches!(
        s.enroll("valid", true).await,
        Err(Failure::Invalid)
    ));
    let c = s.enroll_test("valid", true).await.unwrap();
    let other = unrelated_family(&s, &a).await;
    assert_eq!(
        s.rotate(c.secret(), a.client_id, Purpose::Characters).await,
        Err(Failure::Denied)
    );
    let mut proof = proof_for(&s, c.secret(), 7, 0).await;
    proof.signature = URL_SAFE_NO_PAD.encode([0_u8; 64]);
    assert_eq!(
        s.rotate_with_proof(c.secret(), a.client_id, Purpose::Characters, &proof)
            .await,
        Err(Failure::Denied)
    );
    assert_live(&s, c.family_id).await;
    assert_live(&s, other.family_id).await;
    assert!(
        s.rotate_test(c.secret(), a.client_id, Purpose::Characters)
            .await
            .is_ok()
    );
    Ok(())
}
#[tokio::test]
async fn devices_l3_replayed_proof_refused() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let other = unrelated_family(&s, &a).await;
    let proof = proof_for(&s, c.secret(), 7, 0).await;
    let next = s
        .rotate_with_proof(c.secret(), a.client_id, Purpose::Characters, &proof)
        .await
        .unwrap();
    for secret in [c.secret(), next["device_credential"].as_str().unwrap()] {
        assert_eq!(
            s.rotate_with_proof(secret, a.client_id, Purpose::Ticket, &proof)
                .await,
            Err(Failure::Denied)
        );
    }
    assert_live(&s, c.family_id).await;
    assert_live(&s, other.family_id).await;
    assert!(
        s.rotate_test(
            next["device_credential"].as_str().unwrap(),
            a.client_id,
            Purpose::Ticket
        )
        .await
        .is_ok()
    );
    Ok(())
}
#[tokio::test]
async fn devices_l3_wrong_key_revokes_family() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let other = unrelated_family(&s, &a).await;
    let proof = proof_for(&s, c.secret(), 8, 0).await;
    assert_eq!(
        s.rotate_with_proof(c.secret(), a.client_id, Purpose::Ticket, &proof)
            .await,
        Err(Failure::Denied)
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM device_sessions WHERE id=$1")
            .bind(c.family_id)
            .fetch_one(&s.pool)
            .await?;
    assert!(revoked);
    assert_live(&s, other.family_id).await;
    assert_eq!(
        s.rotate_test(c.secret(), a.client_id, Purpose::Ticket)
            .await,
        Err(Failure::Denied)
    );
    Ok(())
}
#[tokio::test]
async fn devices_l3_stale_proof_refused() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll_test("valid", true).await.unwrap();
    let other = unrelated_family(&s, &a).await;
    for offset in [-61, 120, i64::MIN / 2] {
        let proof = proof_for(&s, c.secret(), 7, offset).await;
        assert_eq!(
            s.rotate_with_proof(c.secret(), a.client_id, Purpose::Characters, &proof)
                .await,
            Err(Failure::Denied)
        );
    }
    assert_live(&s, c.family_id).await;
    assert_live(&s, other.family_id).await;
    assert!(
        s.rotate_test(c.secret(), a.client_id, Purpose::Characters)
            .await
            .is_ok()
    );
    Ok(())
}
#[test]
fn devices_rotation_bad_input_refused() {
    for body in [
        r#"{"protocol_version":1,"client_id":"invalid"}"#,
        r#"{"protocol_version":1,"client_id":"018f1234-1234-7123-8123-123456789abc","rotation_proof":null}"#,
        r#"{"protocol_version":1,"client_id":"018f1234-1234-7123-8123-123456789abc","owner":"x"}"#,
        r#"{"protocol_version":1,"protocol_vers\u0069on":1,"client_id":"018f1234-1234-7123-8123-123456789abc"}"#,
    ] {
        assert!(matches!(
            wire::decode_operation(body.as_bytes(), false),
            Err(Failure::Invalid)
        ));
    }
    assert!(
        http::decode_enrollment(br#"{"protocol_version":1,"remember_device":true}"#, false)
            .is_err()
    );
}
fn client_proof(value: &serde_json::Value) -> RotationProof {
    use ed25519_dalek::{Signer, SigningKey};
    let signing = SigningKey::from_bytes(&[7; 32]);
    let mut proof = RotationProof {
        nonce: value["rotation_nonce"].as_str().unwrap().into(),
        session_id: serde_json::from_value(value["family_id"].clone()).unwrap(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        device_public_key: URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes()),
        signature: String::new(),
    };
    proof.signature = URL_SAFE_NO_PAD.encode(signing.sign(proof.message().as_bytes()).to_bytes());
    proof
}
#[tokio::test]
async fn devices_e2e_full_lifecycle() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&s.pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)").bind(account).bind(vec![0u8;32]).execute(&s.pool).await?;
    let character = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','AVAILABLE')").bind(account).bind(character).bind(Uuid::now_v7()).execute(&s.pool).await?;
    let server = tls::tests::Harness::new(router(s.clone())).await;
    let (status, enrolled) = server
        .post(
            "",
            &format!("Bearer {bearer}"),
            &enrollment_body(&bearer, client, true),
        )
        .await;
    assert_eq!(status, 200, "{enrolled}");
    let mut current = enrolled;
    for suffix in ["/native-characters", "/tickets", "/native-characters"] {
        let body = serde_json::to_value(wire::OperationRequest {
            protocol_version: 1,
            client_id: client,
            rotation_proof: Some(client_proof(&current)),
        })
        .unwrap();
        let (status, next) = server
            .post(
                suffix,
                &format!(
                    "OterynDevice {}",
                    current["device_credential"].as_str().unwrap()
                ),
                &body,
            )
            .await;
        assert_eq!(status, 200, "{next}");
        assert_ne!(current["device_credential"], next["device_credential"]);
        assert_ne!(current["rotation_nonce"], next["rotation_nonce"]);
        assert_eq!(current["family_id"], next["family_id"]);
        if suffix == "/tickets" {
            let row: (Uuid, Uuid) = sqlx::query_as(
                "SELECT account_id,device_family_id FROM game_login_tickets WHERE ticket_hash=$1",
            )
            .bind(hash(next["ticket"].as_str().unwrap()))
            .fetch_one(&s.pool)
            .await?;
            assert_eq!(row.0, account);
            assert_eq!(row.1.to_string(), current["family_id"]);
        } else {
            assert_eq!(next["characters"][0]["character_id"], character.to_string());
        }
        current = next;
    }
    let authorization = format!(
        "OterynDevice {}",
        current["device_credential"].as_str().unwrap()
    );
    let revoke = serde_json::json!({"protocol_version":1,"client_id":client});
    for _ in 0..2 {
        let (status, value) = server.post("/revoke", &authorization, &revoke).await;
        assert_eq!(status, 200);
        assert_eq!(
            value,
            serde_json::json!({"protocol_version":1,"revoked":true})
        );
    }
    let proof = serde_json::to_value(wire::OperationRequest {
        protocol_version: 1,
        client_id: client,
        rotation_proof: Some(client_proof(&current)),
    })
    .unwrap();
    assert_eq!(server.post("/tickets", &authorization, &proof).await.0, 401);
    Ok(())
}

#[test]
fn devices_contract_examples_match_types() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use serde::{Serialize, de::DeserializeOwned};
    fn roundtrip<T: Serialize + DeserializeOwned>(json: &str) {
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let typed: T = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), value);
    }
    let doc = include_str!(
        "../../../../docs/contracts/OTERYN_V2_NATIVE_REMEMBERED_DEVICE_CLIENT_CONTRACT.md"
    );
    let mut count = 0;
    let mut proof = None;
    let mut message = None;
    for section in doc.split("<!-- example:").skip(1) {
        let (kind, rest) = section.split_once(" -->").unwrap();
        let json = rest
            .split_once("```json\n")
            .unwrap()
            .1
            .split_once("\n```")
            .unwrap()
            .0;
        count += 1;
        match kind {
            "EnrollmentRequest" => {
                roundtrip::<wire::EnrollmentRequest>(json);
                let (remembered, key) = http::decode_enrollment(json.as_bytes(), false).unwrap();
                key.unwrap()
                    .verify(
                        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                        Uuid::parse_str("019a5f4a-1000-7000-8000-000000000001").unwrap(),
                        remembered,
                    )
                    .unwrap();
            }
            "OperationRequest" => {
                roundtrip::<wire::OperationRequest>(json);
                let operation = wire::decode_operation(json.as_bytes(), false).unwrap();
                if let Some(p) = operation.rotation_proof {
                    proof = Some(p);
                }
            }
            "CredentialResponse" => roundtrip::<wire::CredentialResponse>(json),
            "CharactersResponse" => roundtrip::<wire::CharactersResponse>(json),
            "TicketResponse" => roundtrip::<wire::TicketResponse>(json),
            "RevokeResponse" => roundtrip::<wire::RevokeResponse>(json),
            "ErrorResponse" => roundtrip::<wire::ErrorResponse>(json),
            "RotationMessage" => {
                roundtrip::<wire::RotationMessage>(json);
                message = Some(serde_json::from_str::<wire::RotationMessage>(json).unwrap());
            }
            _ => return Err(format!("undocumented example type: {kind}").into()),
        }
    }
    assert_eq!(count, doc.matches("```json\n").count());
    assert_eq!(count, 11);
    let proof = proof.unwrap();
    assert_eq!(proof.message(), message.unwrap().canonical());
    proof
        .check_fresh(proof.session_id, Some(&proof.nonce), proof.timestamp)
        .unwrap();
    proof
        .verify(&key::decode_bytes(&proof.device_public_key, 32).unwrap())
        .unwrap();
    Ok(())
}

#[test]
fn devices_l3_clock_skew_boundary_and_session_binding() {
    let family = Uuid::now_v7();
    let mut proof = RotationProof {
        nonce: URL_SAFE_NO_PAD.encode([9_u8; 32]),
        session_id: family,
        timestamp: 1000,
        device_public_key: String::new(),
        signature: String::new(),
    };
    for now in [940, 1000, 1060] {
        assert!(proof.check_fresh(family, Some(&proof.nonce), now).is_ok());
    }
    for now in [939, 1061, i64::MIN, i64::MAX] {
        assert_eq!(
            proof.check_fresh(family, Some(&proof.nonce), now),
            Err(Failure::Denied)
        );
    }
    proof.session_id = Uuid::now_v7();
    assert_eq!(
        proof.check_fresh(family, Some(&proof.nonce), 1000),
        Err(Failure::Denied)
    );
}
