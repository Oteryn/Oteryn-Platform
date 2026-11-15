use super::*;
use axum::{
    body::Body,
    http::{Request, header},
};
use tower::ServiceExt;
#[path = "credentials_test_db.rs"]
mod database;
fn request(
    method: &str,
    path: &str,
    cookie: &str,
    csrf: &str,
    body: &str,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-csrf-token", csrf)
        .body(Body::from(body.to_owned()))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("192.0.2.4:1234".parse::<SocketAddr>()?));
    Ok(request)
}
#[tokio::test]
async fn security_landing_preserves_the_language() {
    for (lang, target) in [
        (Language::Pl, "/pl/password/change"),
        (Language::En, "/en/password/change"),
    ] {
        let response = security_landing(Extension(lang)).await.into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], target);
    }
}

#[tokio::test]
async fn postgres_sudo_rotation_csrf_refusal_and_expiry() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = db.pool.clone();
    let password = oteryn_identity::password::hash("test password")?;
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(format!("{}@example.test",uuid::Uuid::now_v7())).bind(password).fetch_one(&pool).await?;
    let raw = "a".repeat(64);
    let csrf = "b".repeat(64);
    let hash = Sha256::digest(raw.as_bytes()).to_vec();
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')").bind(&hash).bind(id).bind(&csrf).execute(&pool).await?;
    let mut service = Credentials {
        pool: pool.clone(),
        enabled: true,
        revoker: Arc::new(credentials::UnavailableRevoker),
        notifications: Arc::new(oteryn_notify::Notifications {
            pool: pool.clone(),
            accounts: Arc::new(AccountState(Accounts { pool: pool.clone() })),
            unsubscribe: oteryn_notify::Unsubscribe::new(
                vec![7; 32],
                "https://example.test/unsubscribe".into(),
            )
            .map_err(|e| e.to_string())?,
        }),
    };
    let app = router(service.clone());
    let cookie = format!("{}={raw}", session::COOKIE_NAME);
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/en/account/sudo",
                &cookie,
                "",
                "current_password=test+password"
            )?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/en/account/sudo",
                &cookie,
                &csrf,
                "current_password=wrong"
            )?)
            .await?
            .status(),
        StatusCode::FOUND
    );
    let response = app
        .clone()
        .oneshot(request("GET", "/en/password/change", &cookie, "", "")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 20000)
            .await?
            .to_vec(),
    )?;
    assert!(html.contains("Change password"));
    assert!(html.contains("name=\"_csrf\""));
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/account/sudo",
            &cookie,
            &csrf,
            "current_password=test+password",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let rotated = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    assert_ne!(rotated, cookie);
    assert_eq!(service.require_sudo(&hash).await?, None);
    let raw = rotated.split_once('=').ok_or("invalid cookie")?.1;
    let hash = Sha256::digest(raw.as_bytes()).to_vec();
    assert_eq!(service.require_sudo(&hash).await?, Some(id));
    let next_csrf: String =
        sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE id_hash=$1")
            .bind(&hash)
            .fetch_one(&pool)
            .await?;
    assert_ne!(csrf, next_csrf);
    assert_eq!(app.clone().oneshot(request("POST","/en/password/change",&rotated,&next_csrf,"current_password=test+password&password=next+password&password_confirmation=next+password")?).await?.status(),StatusCode::SERVICE_UNAVAILABLE);
    sqlx::query("UPDATE web_sessions SET sudo_until=now() WHERE id_hash=$1")
        .bind(&hash)
        .execute(&pool)
        .await?;
    assert_eq!(service.require_sudo(&hash).await?, None);
    // The limiter uses fixed database-clock windows. Start the 11 charged
    // requests in the first half of a window so a boundary cannot reset the count.
    let window = NamedPolicy::IdentitySecurityMutation.window_seconds();
    while sqlx::query_scalar::<_, bool>(
        "SELECT mod(extract(epoch FROM statement_timestamp()), $1::bigint) * 2 > $1::bigint",
    )
    .bind(window)
    .fetch_one(&pool)
    .await?
    {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    for _ in 0..10 {
        let _ = app
            .clone()
            .oneshot(request(
                "POST",
                "/en/account/sudo",
                &rotated,
                &next_csrf,
                "current_password=wrong",
            )?)
            .await?;
    }
    assert_eq!(
        app.oneshot(request(
            "POST",
            "/en/account/sudo",
            &rotated,
            &next_csrf,
            "current_password=test+password"
        )?)
        .await?
        .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    service.revoker = Arc::new(TestRevoker);
    let success = router(service);
    let response = success.oneshot(request("POST","/en/password/change",&rotated,&next_csrf,"current_password=test+password&password=next+password&password_confirmation=next+password")?).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .ok_or("missing redirect")?,
        "/login"
    );
    assert!(
        response
            .headers()
            .get(header::SET_COOKIE)
            .ok_or("missing cleared cookie")?
            .to_str()?
            .contains("Max-Age=0")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}

// Test-only seam: no OAuth or device modules are registered in this HTTP harness.
struct TestRevoker;
impl credentials::AuthorizationRevoker for TestRevoker {
    fn revoke<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: uuid::Uuid,
    ) -> credentials::EffectFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn audit02_sudo_uses_one_connection_and_preserves_retry_after()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let path: String = sqlx::query_scalar("SHOW search_path")
        .fetch_one(&db.pool)
        .await?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(2))
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
        .connect_with((*db.pool.connect_options()).clone())
        .await?;
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('sudo-audit@example.test',$1,'active','en',now()) RETURNING id").bind(oteryn_identity::password::hash("test password")?).fetch_one(&pool).await?;
    let raw = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'audit',now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(&pool).await?;
    let service = Credentials {
        pool: pool.clone(),
        enabled: true,
        revoker: Arc::new(credentials::TransactionalRevoker),
        notifications: Arc::new(
            oteryn_identity::sessions::security_notifications(pool.clone())
                .map_err(|e| e.to_string())?,
        ),
    };
    let app = router(service);
    let cookie = format!("{}={raw}", session::COOKIE_NAME);
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        app.clone().oneshot(request(
            "POST",
            "/en/account/sudo",
            &cookie,
            &csrf,
            "current_password=test+password",
        )?),
    )
    .await??;
    assert_eq!(response.status(), StatusCode::FOUND);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let hash = Sha256::digest(cookie.split_once('=').ok_or("cookie")?.1.as_bytes()).to_vec();
    let csrf: String = sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE id_hash=$1")
        .bind(hash)
        .fetch_one(&pool)
        .await?;
    // Charge the same source/account policy to its boundary, then assert the real route.
    let mut keys = Keys::source("192.0.2.4".parse()?);
    keys.account = Some(id.to_string());
    let limiter = RateLimiter::new(pool.clone());
    for _ in 1..NamedPolicy::IdentitySecurityMutation.limit() {
        limiter
            .check(NamedPolicy::IdentitySecurityMutation, &[keys.clone()])
            .await
            .map_err(|_| "limiter")?;
    }
    let response = app
        .oneshot(request(
            "POST",
            "/en/account/sudo",
            &cookie,
            &csrf,
            "current_password=test+password",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .ok_or("retry")?
            .to_str()?
            .parse::<u64>()?
            > 0
    );
    pool.close().await;
    db.close().await
}

#[tokio::test]
async fn audit02_sudo_succeeds_when_sixteen_requests_fill_pool()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let path: String = sqlx::query_scalar("SHOW search_path")
        .fetch_one(&db.pool)
        .await?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(std::time::Duration::from_secs(3))
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
        .connect_with((*db.pool.connect_options()).clone())
        .await?;
    let service = Credentials {
        pool: pool.clone(),
        enabled: true,
        revoker: Arc::new(credentials::TransactionalRevoker),
        notifications: Arc::new(
            oteryn_identity::sessions::security_notifications(pool.clone())
                .map_err(|e| e.to_string())?,
        ),
    };
    let app = router(service);
    let password = oteryn_identity::password::hash("test password")?;
    let mut requests = Vec::new();
    for n in 1..=16 {
        let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(format!("sudo-{n}@example.test")).bind(&password).fetch_one(&pool).await?;
        let raw = format!("{n:064x}");
        let csrf = "c".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'audit',now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(&pool).await?;
        requests.push(request(
            "POST",
            "/en/account/sudo",
            &format!("{}={raw}", session::COOKIE_NAME),
            &csrf,
            "current_password=test+password",
        )?);
    }
    let barrier = Arc::new(tokio::sync::Barrier::new(16));
    let mut tasks = Vec::new();
    for request in requests {
        let app = app.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            app.oneshot(request).await
        }));
    }
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        for task in tasks {
            assert_eq!(task.await??.status(), StatusCode::FOUND);
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    pool.close().await;
    db.close().await
}
