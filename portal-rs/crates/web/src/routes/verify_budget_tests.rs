use super::*;
use crate::auth_test_database as database;
use axum::{body::Body, extract::ConnectInfo, http::Request};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[tokio::test]
async fn audit02_resend_locales_share_source_budget_and_cooldown()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id:oteryn_identity::AccountId=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('resend-budget@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    let raw = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,'audit',$3,now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(&db.pool).await?;
    let app = crate::router(
        AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .layer(Extension(ConnectInfo(
        "192.0.2.80:1234".parse::<std::net::SocketAddr>()?,
    )));
    for n in 0..7 {
        let response = app
            .clone()
            .oneshot(
                Request::post(if n % 2 == 0 {
                    "/en/verify-email/resend"
                } else {
                    "/pl/verify-email/resend"
                })
                .header("cookie", format!("{}={raw}", crate::session::COOKIE_NAME))
                .header("x-csrf-token", &csrf)
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(
            response.status(),
            if n < 5 {
                StatusCode::SEE_OTHER
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
        if n >= 5 {
            assert!(response.headers().contains_key("retry-after"));
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM account_email_tokens WHERE account_id=$1"
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM outbox WHERE kind='email.verify'")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.close().await
}

#[tokio::test]
async fn verification_preserves_unrelated_session_and_logs_out_own_account()
-> Result<(), Box<dyn std::error::Error>> {
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let a: oteryn_identity::AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('verify-a@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    let b: oteryn_identity::AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('verify-b@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    let raw = "c".repeat(64);
    let hash = Sha256::digest(raw.as_bytes()).to_vec();
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,'regression',repeat('d',64),now()+interval '1 hour',now()+interval '1 day')").bind(&hash).bind(a).execute(&db.pool).await?;
    let app = crate::router(
        AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .layer(Extension(ConnectInfo(
        "192.0.2.81:1234".parse::<std::net::SocketAddr>()?,
    )));
    for (id, token) in [(b, "e".repeat(64)), (a, "f".repeat(64))] {
        sqlx::query("INSERT INTO account_email_tokens(account_id,token_hash,purpose,expires_at) VALUES($1,$2,'verify_email',now()+interval '1 hour')").bind(id).bind(verify::token_hash(&token)).execute(&db.pool).await?;
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/en/verify-email?token={token}"))
                    .header("cookie", format!("{}={raw}", crate::session::COOKIE_NAME))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let verified: bool = sqlx::query_scalar(
            "SELECT email_verified_at IS NOT NULL AND status='active' FROM accounts WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
        assert!(verified);
        let survives: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2)",
        )
        .bind(&hash)
        .bind(a)
        .fetch_one(&db.pool)
        .await?;
        if id == b {
            assert!(
                survives,
                "unrelated account A session must survive verification of B"
            );
            assert!(!response.headers().contains_key("set-cookie"));
        } else {
            assert!(!survives, "own verification must revoke the old session");
            assert!(response.headers().contains_key("set-cookie"));
        }
    }
    db.close().await
}
