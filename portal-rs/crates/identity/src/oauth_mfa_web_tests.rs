//! OAuth authorize through the full `oteryn_web::router` for an MFA-enrolled account.
//!
//! The router reads `OTERYN_MFA_KEY` and `PORTAL_OAUTH_CONFIG` from the environment and the
//! workspace forbids `unsafe` (`std::env::set_var`), so the parent test runs `router_flow` in a
//! child test process that carries those variables; nothing else in this run sees them.
use crate::{AccountId, login::Credentials, mfa::Mfa, verify::token_hash};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use sqlx::PgPool;
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const CHILD: &str = "PORTAL_OAUTH_MFA_WEB_CHILD";
const KEY: [u8; 32] = [7; 32];
const PASSWORD: &str = "test password";
const CSRF: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const REDIRECT: &str = "http://127.0.0.1:43210/callback";
// SOURCE: RFC 7636 Appendix B, public PKCE S256 test vector.
const AUTHORIZATION: [(&str, &str); 7] = [
    ("response_type", "code"),
    ("client_id", "native-test"),
    ("redirect_uri", REDIRECT),
    ("scope", "game:ticket"),
    ("state", "random-state-for-test"),
    (
        "code_challenge",
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
    ),
    ("code_challenge_method", "S256"),
];

#[test]
fn postgres_web_oauth_authorize_requires_account_bound_mfa() -> Result {
    if std::env::var_os("PORTAL_TEST_DATABASE_URL").is_none() {
        eprintln!("SKIPPED OAuth MFA web PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    }
    let key: String = KEY.iter().map(|b| format!("{b:02x}")).collect();
    let config = serde_json::json!({"enabled": true, "clients": [{"client_id": "native-test", "redirect_uris": [REDIRECT]}]});
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "oauth_mfa_web_tests::router_flow",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .env("OTERYN_MFA_KEY", key)
        .env("PORTAL_OAUTH_CONFIG", config.to_string())
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("1 passed"), "{stdout}\n{stderr}");
    assert!(!stderr.contains("SKIPPED"), "{stderr}");
    Ok(())
}

/// Runs only inside the child process started above.
#[tokio::test]
async fn router_flow() -> Result {
    if std::env::var_os(CHILD).is_none() {
        return Ok(());
    }
    tokio::time::timeout(std::time::Duration::from_secs(60), flow()).await?
}

async fn flow() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), KEY);
    let (owner, grant) = db.mfa_login(&mfa, "owner@example.test").await?;
    let (other, other_grant) = db.mfa_login(&mfa, "other@example.test").await?;
    assert!(mfa.satisfied(other, &other_grant).await?);
    assert!(!mfa.satisfied(owner, &other_grant).await?);
    let app = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .layer(axum::Extension(axum::extract::ConnectInfo(
        std::net::SocketAddr::from(([127, 0, 0, 1], 4321)),
    )));
    // A password-only session and a session holding another account's grant are refused.
    let password_only = db.session(owner, None).await?;
    let foreign = db.session(owner, Some(&other_grant)).await?;
    for cookie in [&password_only, &foreign] {
        for response in [
            app.clone().oneshot(show(cookie)?).await?,
            app.clone().oneshot(approve(cookie)?).await?,
        ] {
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers().get(header::LOCATION).ok_or("location")?,
                "/en/login"
            );
        }
    }
    assert_eq!(db.codes().await?, 0);
    // The owner's session with the grant from the real MFA completion gets consent and a code.
    let proven = db.session(owner, Some(&grant)).await?;
    let response = app.clone().oneshot(show(&proven)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(body.contains("name=\"decision\" value=\"approve\""));
    assert!(body.contains(CSRF));
    let response = app.clone().oneshot(approve(&proven)?).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(header::LOCATION)
        .ok_or("location")?
        .to_str()?;
    assert!(
        location.starts_with(&format!("{REDIRECT}?code=")),
        "{location}"
    );
    assert!(
        location.ends_with("&state=random-state-for-test"),
        "{location}"
    );
    assert_eq!(db.codes().await?, 1);
    db.close().await
}

fn show(cookie: &str) -> Result<Request<Body>> {
    Ok(Request::get(format!(
        "/oauth/authorize?{}",
        serde_urlencoded::to_string(AUTHORIZATION)?
    ))
    .header(header::COOKIE, cookie)
    .header(header::ACCEPT_LANGUAGE, "en")
    .body(Body::empty())?)
}

fn approve(cookie: &str) -> Result<Request<Body>> {
    let mut form = AUTHORIZATION.to_vec();
    form.extend([("_csrf", CSRF), ("decision", "approve")]);
    Ok(Request::post("/oauth/authorize")
        .header(header::COOKIE, cookie)
        .header(header::ACCEPT_LANGUAGE, "en")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(serde_urlencoded::to_string(form)?))?)
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED OAuth MFA web PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("oauth_mfa_web_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
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
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    /// Enrolls TOTP, then signs in with password and a recovery code; returns the web grant.
    async fn mfa_login(&self, mfa: &Mfa, email: &str) -> Result<(AccountId, String)> {
        let id: AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id")
            .bind(email).bind(crate::password::hash(PASSWORD)?).fetch_one(&self.pool).await?;
        let enrollment = mfa.enroll(id, PASSWORD).await?.ok_or("enrollment")?;
        let secret = totp_rs::Secret::Encoded(enrollment.secret).to_bytes()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let code = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, secret)?.generate(now);
        let codes = mfa.confirm(id, PASSWORD, &code).await?.ok_or("confirm")?;
        let required: bool =
            sqlx::query_scalar("SELECT login_mfa_required FROM accounts WHERE id=$1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        assert!(required);
        let challenge = mfa
            .begin_login(&Credentials {
                email: email.to_owned(),
                password: PASSWORD.to_owned(),
            })
            .await?
            .ok_or("challenge")?;
        let auth = mfa
            .complete(&challenge.token, codes.first().ok_or("code")?, false)
            .await?
            .ok_or("complete")?;
        assert_eq!(auth.account, id);
        Ok((id, auth.grant))
    }
    /// A signed-in web session row, holding `grant` under the session key the MFA flow uses.
    async fn session(&self, id: AccountId, grant: Option<&str>) -> Result<String> {
        let raw = format!(
            "{}{}",
            uuid::Uuid::now_v7().simple(),
            uuid::Uuid::now_v7().simple()
        );
        let data = grant.map_or_else(
            || serde_json::json!({}),
            |g| serde_json::json!({ crate::mfa::SESSION_GRANT_KEY: g }),
        );
        sqlx::query("INSERT INTO web_sessions (id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,data) VALUES ($1,$2,$3,'test',now()+interval '1 hour',now()+interval '2 hours',$4::text::jsonb)").bind(token_hash(&raw)).bind(id).bind(CSRF).bind(data.to_string()).execute(&self.pool).await?;
        Ok(format!("__Host-oteryn_session={raw}"))
    }
    async fn codes(&self) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM oauth_authorization_codes")
                .fetch_one(&self.pool)
                .await?,
        )
    }
    async fn close(self) -> Result {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
