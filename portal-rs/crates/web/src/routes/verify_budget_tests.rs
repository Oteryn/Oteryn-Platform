use super::*;
use axum::{body::Body, extract::ConnectInfo, http::Request};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
#[path = "verify_budget_db.rs"]
mod database;

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
