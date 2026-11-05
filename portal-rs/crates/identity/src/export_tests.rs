use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn ambiguous_cookies_deny() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        axum::http::HeaderValue::from_static("__Host-oteryn_session=a; __Host-oteryn_session=b"),
    );
    assert_eq!(session_cookie(&headers), None);
}
#[tokio::test]
async fn disabled_and_bad_input() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        download(&pool, &ExportConfig::default(), "").await,
        Err(StatusCode::NOT_FOUND)
    );
    assert_eq!(
        download(&pool, &ExportConfig { enabled: true }, "bad").await,
        Err(StatusCode::UNAUTHORIZED)
    );
    Ok(())
}
#[tokio::test]
async fn postgres_owner_export_and_refusals() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED export PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("export_test_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES ('owner@example.invalid','private-password-hash','active','en',now()) RETURNING id").fetch_one(&pool).await?;
    sqlx::query("INSERT INTO accounts(email,password_hash,status,locale) VALUES ('other@example.invalid','other-private-hash','active','pl')").execute(&pool).await?;
    let token = "a".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until,data) VALUES ($1,$2,$3,'test browser',now()+interval '1 hour',now()+interval '1 day',now()+interval '5 minutes','{\"secret\":\"private-session-state\"}')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(id).bind("c".repeat(64)).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) SELECT $1,id,$2,'other-private-device',now()+interval '1 hour',now()+interval '1 day' FROM accounts WHERE email='other@example.invalid'")
        .bind(Sha256::digest("b".repeat(64).as_bytes()).to_vec()).bind("d".repeat(64)).execute(&pool).await?;
    sqlx::query("INSERT INTO account_consents(account_id,document_kind,document_version) VALUES ($1,'terms','v1')").bind(id).execute(&pool).await?;
    let config = ExportConfig { enabled: true };
    let data = download(&pool, &config, &token)
        .await
        .map_err(|s| s.to_string())?;
    assert_eq!(data["account"]["id"], id.to_string());
    assert_eq!(data["sessions"].as_array().map(Vec::len), Some(1));
    use tower::ServiceExt;
    for locale in ["pl", "en"] {
        let response = router(pool.clone(), config.clone())
            .oneshot(
                axum::http::Request::get(format!("/{locale}/account/privacy/export"))
                    .header(header::COOKIE, format!("__Host-oteryn_session={token}"))
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, private"
        );
        assert_eq!(
            response.headers()[header::CONTENT_DISPOSITION],
            "attachment; filename=\"account-data.json\""
        );
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
        let exported: Value = serde_json::from_slice(&body)?;
        assert_eq!(exported, data);
    }
    assert_eq!(data["consents"][0]["document_version"], "v1");
    let encoded = data.to_string();
    for forbidden in [
        "private-password-hash",
        "other@example.invalid",
        "private-session-state",
        "other-private-device",
        "csrf_token",
        "id_hash",
        &token,
    ] {
        assert!(!encoded.contains(forbidden));
    }
    for query in [
        "UPDATE web_sessions SET sudo_until=NULL",
        "UPDATE web_sessions SET sudo_until=now()+interval '1 minute',idle_expires_at=now()-interval '1 second'",
        "UPDATE web_sessions SET idle_expires_at=now()+interval '1 hour'; UPDATE accounts SET status='suspended'",
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(query))
            .execute(&pool)
            .await?;
        assert_eq!(
            download(&pool, &config, &token).await,
            Err(StatusCode::FORBIDDEN)
        );
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
