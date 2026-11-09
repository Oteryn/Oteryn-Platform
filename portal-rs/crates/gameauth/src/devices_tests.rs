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
    assert!(matches!(s.enroll("bad", true).await, Err(Failure::Denied)));
    assert!(matches!(
        s.enroll("valid", false).await,
        Err(Failure::Invalid)
    ));
    let c = s.enroll("valid", true).await.unwrap();
    assert!(valid_secret(c.secret()));
    assert!(matches!(
        s.enroll("valid", true).await,
        Err(Failure::Denied)
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
async fn database_concurrent_replay_revokes_successor()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, a)) = setup().await? else {
        return Ok(());
    };
    let c = s.enroll("valid", true).await.unwrap();
    let (first, second) = tokio::join!(
        s.rotate(c.secret(), a.client_id, Purpose::Ticket),
        s.rotate(c.secret(), a.client_id, Purpose::Ticket)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let success = first.or(second).unwrap();
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
        let c = s.enroll("valid", true).await.unwrap();
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
    let c = s.enroll("valid", true).await.unwrap();
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
    use axum::extract::ConnectInfo;
    use std::net::SocketAddr;
    let Some((s, _)) = setup().await? else {
        return Ok(());
    };
    let app: axum::Router = router(s);
    let source = SocketAddr::from(([127, 1, (Uuid::now_v7().as_u128() % 255) as u8, 1], 45000));
    let request = |body: &'static str| {
        let mut r = Request::post("https://example.test/api/v1/game-auth/device-sessions")
            .header("content-type", "application/json")
            .header("authorization", "Bearer valid")
            .body(Body::from(body))
            .unwrap();
        r.extensions_mut().insert(ConnectInfo(source));
        r.extensions_mut()
            .insert(VerifiedHttps::from_verified_tls());
        r
    };
    let bad = app
        .clone()
        .oneshot(request(r#"{"protocol_version":1,"remember_device":false}"#))
        .await?;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    let success = app
        .clone()
        .oneshot(request(r#"{"protocol_version":1,"remember_device":true}"#))
        .await?;
    assert_eq!(success.status(), StatusCode::OK);
    assert_eq!(success.headers()["cache-control"], "no-store, private");
    for _ in 0..28 {
        let _ = app.clone().oneshot(request("{}")).await?;
    }
    let limited = app.oneshot(request("{}")).await?;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()["retry-after"], "60");
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
    let c = s.enroll("valid", true).await.unwrap();
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
    let c = s.enroll("valid", true).await.unwrap();
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
    let Some((mut sessions, auth)) = setup().await? else {
        return Ok(None);
    };
    let client = auth.client_id;
    sqlx::query("UPDATE accounts SET email_verified_at=now() WHERE id=$1")
        .bind(auth.account_id)
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
    let code = crate::oauth::authorize(&sessions.pool, &config, auth.account_id, &request).await?;
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
        s.enroll("invalid", true).await,
        Err(Failure::Denied)
    ));
    let enrolled = s.enroll(&bearer, true).await.unwrap();
    assert!(matches!(
        s.enroll(&bearer, true).await,
        Err(Failure::Denied)
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
        Err(Failure::Denied)
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
        s.enroll(&bearer, true).await,
        Err(Failure::Denied)
    ));
    Ok(())
}

#[tokio::test]
async fn database_native_password_mfa_suspension_and_client_revocation()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for change in [
        "password",
        "mfa",
        "mfa-key",
        "suspension",
        "client",
        "generation",
    ] {
        let Some((s, account, client, bearer)) = native_setup().await? else {
            return Ok(());
        };
        let enrolled = s.enroll(&bearer, true).await.unwrap();
        match change {
            "password" => {
                sqlx::query("UPDATE accounts SET password_hash='changed-test-hash' WHERE id=$1")
                    .bind(account)
                    .execute(&s.pool)
                    .await?;
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
async fn database_native_character_projection_and_failure_rollback()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some((s, account, client, bearer)) = native_setup().await? else {
        return Ok(());
    };
    let enrolled = s.enroll(&bearer, true).await.unwrap();
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
    let enrollment = s.enroll(&bearer, true).await.unwrap();
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
            s.enroll(&bearer, true).await,
            Err(Failure::Denied)
        ));
    }
    Ok(())
}
