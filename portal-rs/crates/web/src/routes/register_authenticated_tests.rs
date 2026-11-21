use super::*;
use crate::auth_test_database as database;
use axum::{body::Body, http::Request};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[tokio::test]
async fn authenticated_registration_redirects_without_creating_account()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id: oteryn_identity::AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('existing@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    let raw = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,'registration-test',$3,now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(&db.pool).await?;
    let app = routes()
        .with_state(AppState {
            db: db.pool.clone(),
        })
        .layer(axum::middleware::from_fn_with_state(
            crate::session::SessionLayer::new(db.pool.clone()),
            crate::session::middleware,
        ))
        .layer(Extension(Language::En))
        .layer(Extension(crate::ratelimit::RateLimiter::new(
            db.pool.clone(),
        )))
        .layer(Extension(axum::extract::ConnectInfo(
            "192.0.2.82:1234".parse::<std::net::SocketAddr>()?,
        )));
    for method in [axum::http::Method::GET, axum::http::Method::POST] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/en/register")
                    .header("cookie", format!("{}={raw}", crate::session::COOKIE_NAME))
                    .header("x-csrf-token", &csrf)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()["location"], "/account");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.close().await
}

#[tokio::test]
async fn switching_account_with_previous_authorization_generation_is_refused()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let old: oteryn_identity::AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('old@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    sqlx::query("UPDATE accounts SET email_verified_at=now() WHERE id=$1")
        .bind(old)
        .execute(&db.pool)
        .await?;
    let new: oteryn_identity::AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('new@example.test','unused','en') RETURNING id").fetch_one(&db.pool).await?;
    let hash = Sha256::digest("a".repeat(64).as_bytes()).to_vec();
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,authorization_generation,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) SELECT $1,id,native_security_generation,'registration-test',$2,now()+interval '1 hour',now()+interval '1 day' FROM accounts WHERE id=$3").bind(&hash).bind("b".repeat(64)).bind(old).execute(&db.pool).await?;
    // This is the previous register -> rotate(Login(new)) UPDATE: it retains the
    // old account's login generation and the security trigger correctly rejects it.
    let error = sqlx::query("UPDATE web_sessions SET account_id=$2 WHERE id_hash=$1")
        .bind(&hash)
        .bind(new)
        .execute(&db.pool)
        .await
        .err()
        .ok_or("old authorization must not authenticate a different account")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("42501")
    );
    db.close().await
}

#[tokio::test]
async fn r7_registration_leaves_both_sessions_anonymous() -> Result<(), Box<dyn std::error::Error>>
{
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    // Empty breach-cache range for the form password below.
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES('AE9F0',ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id())").execute(&db.pool).await?;
    let app = routes()
        .with_state(AppState {
            db: db.pool.clone(),
        })
        .layer(axum::middleware::from_fn_with_state(
            crate::session::SessionLayer::new(db.pool.clone()),
            crate::session::middleware,
        ))
        .layer(Extension(Language::En))
        .layer(Extension(crate::ratelimit::RateLimiter::new(
            db.pool.clone(),
        )))
        .layer(Extension(axum::extract::ConnectInfo(
            "192.0.2.82:1234".parse::<std::net::SocketAddr>()?,
        )));
    for raw in ["a".repeat(64), "c".repeat(64)] {
        let csrf = "b".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,NULL,'registration-test',$2,now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(&csrf).execute(&db.pool).await?;

        let response = app.clone().oneshot(Request::post("/en/register")
            .header("cookie", format!("{}={raw}", crate::session::COOKIE_NAME))
            .header("x-csrf-token", &csrf)
            .header("content-type","application/x-www-form-urlencoded")
            .body(Body::from("email=new%40example.test&password=long-test-password99&password_confirmation=long-test-password99&terms=2026-11-05&privacy=2026-11-05"))?).await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let signed_in: i64 =
            sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id IS NOT NULL")
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(signed_in, 0, "registration must leave an anonymous session");
    }
    db.close().await
}
