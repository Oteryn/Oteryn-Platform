#[path = "email_change_test_db.rs"]
mod database;
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request as HttpRequest, header},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn req(
    method: &str,
    path: &str,
    cookie: &str,
    body: &str,
) -> Result<HttpRequest<Body>, Box<dyn std::error::Error>> {
    let mut request = HttpRequest::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("192.0.2.44:1234".parse::<SocketAddr>()?));
    Ok(request)
}
#[tokio::test]
async fn disabled_routes_are_absent() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let service = Service {
        pool: pool.clone(),
        enabled: false,
        origin: "https://portal.example.invalid".into(),
        security: Arc::new(DenySecurity),
    };
    assert_eq!(
        router(service, SessionLayer::new(pool))
            .oneshot(req("GET", "/en/account/security/email", "", "")?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}
#[tokio::test]
async fn postgres_browser_token_form_csrf_refusal_and_bad_input() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let service = Service {
        pool: db.pool.clone(),
        enabled: true,
        origin: "https://portal.example.invalid".into(),
        security: Arc::new(DenySecurity),
    };
    let app = router(service, SessionLayer::new(db.pool.clone()));
    let path = format!("/en/email-change/revert/{}", "a".repeat(64));
    let response = app.clone().oneshot(req("GET", &path, "", "")?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("empty cookie")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    assert!(html.contains("This was not me"));
    assert!(html.contains("viewport"));
    assert!(html.contains("op-button"));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("missing CSRF")?;
    assert_eq!(
        app.clone()
            .oneshot(req("POST", &path, &cookie, "")?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(req("POST", &path, &cookie, &format!("_csrf={csrf}"))?)
            .await?
            .status(),
        StatusCode::FOUND
    );
    assert_eq!(
        app.clone()
            .oneshot(req(
                "POST",
                "/en/email-change/revert/bad",
                &cookie,
                &format!("_csrf={csrf}")
            )?)
            .await?
            .status(),
        StatusCode::FOUND
    );
    assert_eq!(
        app.oneshot(req("GET", "/en/account/security/email", "", "")?)
            .await?
            .status(),
        StatusCode::FOUND
    );
    db.close().await?;
    Ok(())
}
struct Authority;
type SecurityFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, sqlx::Error>> + Send + 'a>>;
impl Security for Authority {
    fn proof<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        _: Uuid,
        _: bool,
        proof: &'a str,
    ) -> SecurityFuture<'a> {
        Box::pin(async move { Ok(proof == "verified-test-proof") })
    }
    fn revoke<'a>(&'a self, _: &'a mut sqlx::PgConnection, _: Uuid) -> SecurityFuture<'a> {
        Box::pin(async { Ok(true) })
    }
}
#[tokio::test]
async fn postgres_browser_confirmation_changes_address_and_rejects_replay() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'old@example.invalid',$2,'active','en',now())").bind(id).bind(oteryn_identity::password::hash("test-password")?).execute(&db.pool).await?;
    let service = Service {
        pool: db.pool.clone(),
        enabled: true,
        origin: "https://portal.example.invalid".into(),
        security: Arc::new(Authority),
    };
    assert_eq!(
        service
            .request(
                id,
                Request {
                    email: "new@example.invalid".into(),
                    email_confirmation: "new@example.invalid".into(),
                    current_password: "test-password".into(),
                    proof: "verified-test-proof".into(),
                    recovery: true,
                    backup: false
                }
            )
            .await?,
        Outcome::Requested
    );
    let mail: String = sqlx::query_scalar(
        "SELECT body FROM account_email_change_mail WHERE recipient='new@example.invalid'",
    )
    .fetch_one(&db.pool)
    .await?;
    let token = mail
        .rsplit('/')
        .next()
        .ok_or("missing confirmation token")?;
    let path = format!("/en/email-change/confirm/{token}");
    let app = router(service, SessionLayer::new(db.pool.clone()));
    let response = app.clone().oneshot(req("GET", &path, "", "")?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("empty cookie")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("missing CSRF")?;
    let response = app
        .clone()
        .oneshot(req("POST", &path, &cookie, &format!("_csrf={csrf}"))?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(email, "new@example.invalid");
    let response = app
        .oneshot(req("POST", &path, &cookie, &format!("_csrf={csrf}"))?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], path);
    db.close().await?;
    Ok(())
}
