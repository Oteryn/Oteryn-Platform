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
