use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[tokio::test]
async fn db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("code_dispatch_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(12)
        .after_connect(move |c, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'code-dispatch@example.test',now(),'test-credential-hash','active','en')").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
    // Persist a valid current MFA grant and browser session. Dispatch still runs
    // the real host, session, CSRF, enrollment, generation and permission checks.
    let generation = Uuid::now_v7();
    let grant = oteryn_identity::mfa::random_token()?;
    let cookie = oteryn_identity::mfa::random_token()?;
    let csrf = oteryn_identity::mfa::random_token()?;
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed,generation) VALUES($1,$2,true,$3)").bind(actor).bind(Sha256::digest(grant.as_bytes()).to_vec()).bind(generation).execute(&pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '10 minutes')").bind(oteryn_identity::mfa::digest(&grant)).bind(actor).bind(generation).bind(oteryn_identity::mfa::digest("test-credential-hash")).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,data,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,$4,'code-dispatch-test',now()+interval '10 minutes',now()+interval '1 hour')").bind(Sha256::digest(cookie.as_bytes()).to_vec()).bind(&csrf).bind(actor).bind(serde_json::json!({"mfa_grant":grant})).execute(&pool).await?;
    let config = |key: &str| match key {
        "OTERYN_STORE_CODES_ENABLED" | "OTERYN_ADMIN_PANEL" | "OTERYN_WALLET_ENABLED" => {
            Some("true".into())
        }
        "OTERYN_ADMIN_HOST" => Some("admin.example.test".into()),
        "OTERYN_MFA_KEY" => Some("19".repeat(32)),
        _ => None,
    };
    let app = configured_from(pool.clone(), &config)?;
    let send = |path: &str, body: serde_json::Value| {
        let req = Request::post(path)
            .header("host", "admin.example.test")
            .header("cookie", format!("__Host-oteryn_session={cookie}"))
            .header("x-csrf-token", &csrf)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?;
        Ok::<_, Box<dyn std::error::Error>>(app.clone().oneshot(req))
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let batch = serde_json::json!({"kind":"coins","amount":7,"count":1,"max_per_code":1,"max_per_account":1,"valid_from":now-60,"valid_until":now+3600,"note":"Dispatch regression"});
    let response = send("/en/admin/store/codes", batch.clone())?.await?;
    assert_eq!(response.status(), StatusCode::OK);
    let result: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await?)?;
    let redemption = serde_json::json!({"code":result["codes"][0],"operation":Uuid::now_v7()});
    for _ in 0..2 {
        assert_eq!(
            send("/en/shop/codes/redeem", redemption.clone())?
                .await?
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1"
        )
        .bind(actor)
        .fetch_one(&pool)
        .await?,
        7
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM code_redemptions")
            .fetch_one(&pool)
            .await?,
        1
    );
    let mut item = batch.clone();
    item["kind"] = "item_grant".into();
    assert_eq!(
        send("/en/admin/store/codes", item)?.await?.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM code_game_commands")
            .fetch_one(&pool)
            .await?,
        0
    );
    sqlx::query("DELETE FROM role_permissions WHERE permission='store.codes.generate'")
        .execute(&pool)
        .await?;
    assert_eq!(
        send("/en/admin/store/codes", batch)?.await?.status(),
        StatusCode::FORBIDDEN
    );
    for expected in [422, 422, 422, 429] {
        assert_eq!(
            send("/en/shop/codes/redeem", serde_json::json!({"unknown":true}))?
                .await?
                .status()
                .as_u16(),
            expected
        );
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    eprintln!(
        "PG db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
