use super::*;
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED OAuth PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("oauth_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
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
    async fn account(&self) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-hash','active','en',now()) RETURNING id").bind(format!("{}@example.test",Uuid::now_v7())).fetch_one(&self.pool).await
    }
    async fn close(self) -> TestResult {
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
fn config() -> Config {
    Config {
        enabled: true,
        clients: vec![Client {
            client_id: "native-test".into(),
            redirect_uris: vec!["http://127.0.0.1:43210/callback".into()],
        }],
    }
}
// SOURCE: RFC 7636 Appendix B, public PKCE S256 test vector.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
fn authorization() -> Authorization {
    Authorization {
        response_type: "code".into(),
        client_id: "native-test".into(),
        redirect_uri: "http://127.0.0.1:43210/callback".into(),
        scope: "game:ticket".into(),
        state: "random-state-for-test".into(),
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
        code_challenge_method: "S256".into(),
    }
}
async fn code(db: &Database, account: Uuid) -> Result<TokenRequest, Error> {
    Ok(TokenRequest {
        grant_type: "authorization_code".into(),
        client_id: "native-test".into(),
        redirect_uri: authorization().redirect_uri,
        code: authorize(&db.pool, &config(), account, &authorization()).await?,
        code_verifier: VERIFIER.into(),
        ..Default::default()
    })
}
fn refresh_input(token: &str) -> TokenRequest {
    TokenRequest {
        grant_type: "refresh_token".into(),
        client_id: "native-test".into(),
        refresh_token: token.into(),
        ..Default::default()
    }
}
#[test]
fn validation_denies_every_unsupported_authorization_parameter() -> TestResult {
    assert!(authorization().valid(&config()));
    for key in [
        "response_type",
        "client_id",
        "redirect_uri",
        "scope",
        "state",
        "code_challenge",
        "code_challenge_method",
    ] {
        let a = authorization();
        let mut value = serde_json::json!({"response_type":a.response_type,"client_id":a.client_id,"redirect_uri":a.redirect_uri,"scope":a.scope,"state":a.state,"code_challenge":a.code_challenge,"code_challenge_method":a.code_challenge_method});
        value[key] = serde_json::json!("invalid");
        assert!(
            !serde_json::from_value::<Authorization>(value)?.valid(&config()),
            "{key}"
        );
    }
    for uri in [
        "http://localhost/callback",
        "http://127.0.0.1:43211/callback",
        "http://127.0.0.1:43210/other",
        "http://127.0.0.1:43210/callback?x=y",
        "http://127.0.0.1:43210/callback#fragment",
        "https://user@host/callback",
    ] {
        assert!(!config().redirect("native-test", uri));
    }
    let mut c = config();
    c.clients.push(c.clients[0].clone());
    assert!(!authorization().valid(&c));
    assert!(!authorization().valid(&Config::default()));
    assert!(!verifier("short"));
    assert!(!verifier(&"!".repeat(43)));
    assert!(!verifier(&"a".repeat(129)));
    Ok(())
}
#[tokio::test]
async fn postgres_success_rotation_and_family_reuse_revocation() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let input = code(&db, account).await?;
    let first = exchange(&db.pool, &config(), &input).await?;
    assert_eq!(
        verify_access(&db.pool, &config(), &first.access_token).await?,
        account
    );
    // SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87 crates/platform-client/src/native_login.rs token decoder.
    let value = serde_json::to_value(&first)?;
    assert_eq!(value["token_type"], "bearer");
    assert!(
        value["access_token"]
            .as_str()
            .ok_or("missing access token")?
            .len()
            <= 8192
    );
    let second = exchange(&db.pool, &config(), &refresh_input(&first.refresh_token)).await?;
    assert_ne!(first.refresh_token, second.refresh_token);
    assert_eq!(
        exchange(&db.pool, &config(), &refresh_input(&first.refresh_token))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        verify_access(&db.pool, &config(), &first.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        verify_access(&db.pool, &config(), &second.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        exchange(&db.pool, &config(), &refresh_input(&second.refresh_token))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM oauth_security_events")
        .fetch_one(&db.pool)
        .await?;
    assert!(count >= 4);
    db.close().await
}
#[tokio::test]
async fn postgres_code_refusals_and_single_use() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let mut input = code(&db, account).await?;
    input.code_verifier = "a".repeat(43);
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    input.code_verifier = VERIFIER.into();
    input.redirect_uri = "http://127.0.0.1:43211/callback".into();
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidRequest)
    );
    input.redirect_uri = authorization().redirect_uri;
    let mut other = config();
    other.clients.push(Client {
        client_id: "other".into(),
        redirect_uris: vec![input.redirect_uri.clone()],
    });
    input.client_id = "other".into();
    assert_eq!(
        exchange(&db.pool, &other, &input).await.err(),
        Some(Error::InvalidGrant)
    );
    input.client_id = "native-test".into();
    assert!(exchange(&db.pool, &config(), &input).await.is_ok());
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    let input = code(&db, account).await?;
    sqlx::query("UPDATE oauth_authorization_codes SET expires_at=now()-interval '1 second' WHERE code_hash=$1").bind(hash(&input.code)).execute(&db.pool).await?;
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    let mut input = code(&db, account).await?;
    input.code = secret()?;
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    db.close().await
}
#[tokio::test]
async fn postgres_refresh_refusals_and_expiry() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let tokens = exchange(&db.pool, &config(), &code(&db, account).await?).await?;
    let mut input = refresh_input(&tokens.refresh_token);
    input.scope = "admin".into();
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidRequest)
    );
    input.scope.clear();
    input.client_id = "unknown".into();
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidClient)
    );
    let mut other = config();
    other.clients.push(Client {
        client_id: "other".into(),
        redirect_uris: vec![],
    });
    input.client_id = "other".into();
    assert_eq!(
        exchange(&db.pool, &other, &input).await.err(),
        Some(Error::InvalidGrant)
    );
    input.client_id = "native-test".into();
    input.refresh_token = "bad".into();
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidRequest)
    );
    input.refresh_token = secret()?;
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    sqlx::query("UPDATE oauth_token_families SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        exchange(&db.pool, &config(), &refresh_input(&tokens.refresh_token))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        verify_access(&db.pool, &config(), &tokens.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    db.close().await
}
#[tokio::test]
async fn postgres_concurrent_code_and_refresh_are_serialized() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let input = code(&db, account).await?;
    let c = config();
    let (a, b) = tokio::join!(
        exchange(&db.pool, &c, &input),
        exchange(&db.pool, &c, &input)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let tokens = a.or(b)?;
    let input = refresh_input(&tokens.refresh_token);
    let (a, b) = tokio::join!(
        exchange(&db.pool, &c, &input),
        exchange(&db.pool, &c, &input)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let tokens = a.or(b)?;
    assert_eq!(
        verify_access(&db.pool, &c, &tokens.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    db.close().await
}
#[tokio::test]
async fn postgres_account_guards_credentials_and_explicit_revocation() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let old_code = code(&db, account).await?;
    let tokens = exchange(&db.pool, &config(), &code(&db, account).await?).await?;
    sqlx::query("UPDATE accounts SET password_hash='changed' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        exchange(&db.pool, &config(), &old_code).await.err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        verify_access(&db.pool, &config(), &tokens.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        exchange(&db.pool, &config(), &refresh_input(&tokens.refresh_token))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    let input = code(&db, account).await?;
    let tokens = exchange(&db.pool, &config(), &code(&db, account).await?).await?;
    let mut tx = db.pool.begin().await?;
    revoke_account(&mut tx, account).await?;
    tx.commit().await?;
    assert_eq!(
        exchange(&db.pool, &config(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    assert_eq!(
        verify_access(&db.pool, &config(), &tokens.access_token)
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    for change in [
        "status='suspended'",
        "status='closed'",
        "status='pending_verification'",
        "email_verified_at=NULL",
        "login_mfa_required=true",
    ] {
        sqlx::query("UPDATE accounts SET status='active',email_verified_at=now(),login_mfa_required=false WHERE id=$1").bind(account).execute(&db.pool).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE accounts SET {change} WHERE id=$1"
        )))
        .bind(account)
        .execute(&db.pool)
        .await?;
        assert_eq!(
            authorize(&db.pool, &config(), account, &authorization())
                .await
                .err(),
            Some(Error::InvalidGrant)
        );
    }
    assert_eq!(
        authorize(&db.pool, &config(), Uuid::now_v7(), &authorization())
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    db.close().await
}
#[tokio::test]
async fn postgres_http_consent_and_oauth_envelopes() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let browser = Browser {
        account: Some(account),
        csrf: "a".repeat(64),
        language: "en".into(),
    };
    let app = browser_routes(db.pool.clone(), config()).layer(Extension(browser.clone()));
    let a = authorization();
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in [
        ("response_type", a.response_type),
        ("client_id", a.client_id),
        ("redirect_uri", a.redirect_uri),
        ("scope", a.scope),
        ("state", a.state),
        ("code_challenge", a.code_challenge),
        ("code_challenge_method", a.code_challenge_method),
    ] {
        query.append_pair(k, &v);
    }
    let fields = query.finish();
    let r = app
        .clone()
        .oneshot(Request::get(format!("/oauth/authorize?{fields}")).body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(r.into_body(), 16384).await?.to_vec())?;
    assert!(html.contains("Authorize game sign-in"));
    assert!(html.contains("name=\"_csrf\""));
    for (decision, expected) in [("deny", "error=access_denied"), ("approve", "code=")] {
        let r = app
            .clone()
            .oneshot(
                Request::post("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "{fields}&_csrf={}&decision={decision}",
                        browser.csrf
                    )))?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::FOUND);
        let location = r.headers()["location"].to_str()?;
        assert!(location.contains(expected));
        assert!(location.contains("state=random-state-for-test"));
    }
    for suffix in [
        "decision=approve",
        "_csrf=wrong&decision=approve",
        "_csrf=wrong&decision=unexpected",
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::post("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("{fields}&{suffix}")))?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }
    for suffix in [
        format!("_csrf={}&decision=unexpected", browser.csrf),
        format!("_csrf={}&decision=approve&client_id=other", browser.csrf),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::post("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("{fields}&{suffix}")))?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }
    let anonymous = browser_routes(db.pool.clone(), config()).layer(Extension(Browser {
        account: None,
        ..browser.clone()
    }));
    let r = anonymous
        .clone()
        .oneshot(Request::get(format!("/oauth/authorize?{fields}")).body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert_eq!(r.headers()["location"], "/en/login");
    let r = anonymous
        .oneshot(
            Request::post("/oauth/authorize")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "{fields}&_csrf={}&decision=approve",
                    browser.csrf
                )))?,
        )
        .await?;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let r = app
        .oneshot(
            Request::get("/oauth/authorize?response_type=code&unexpected=true")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let app = token_routes(db.pool.clone(), config());
    for (body, expected) in [
        (
            "grant_type=password&client_id=native-test",
            "unsupported_grant_type",
        ),
        (
            "grant_type=authorization_code&client_id=native-test",
            "invalid_request",
        ),
        (
            "grant_type=authorization_code&client_id=unknown",
            "invalid_client",
        ),
        (
            "grant_type=refresh_token&client_id=native-test&unexpected=true",
            "invalid_request",
        ),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::post("/oauth/token")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(r.headers()["cache-control"], "no-store");
        let v: serde_json::Value = serde_json::from_slice(&to_bytes(r.into_body(), 8192).await?)?;
        assert_eq!(v["error"], expected);
    }
    db.close().await
}

#[tokio::test]
async fn disabled_oauth_denies_without_database_access() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        exchange(&pool, &Config::default(), &TokenRequest::default())
            .await
            .err(),
        Some(Error::Disabled)
    );
    assert_eq!(
        verify_access(&pool, &Config::default(), "bad").await.err(),
        Some(Error::Disabled)
    );
    assert_eq!(
        verify_access(&pool, &config(), "bad").await.err(),
        Some(Error::InvalidGrant)
    );
    let r = token_routes(pool.clone(), Config::default())
        .oneshot(Request::post("/oauth/token").body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let r = browser_routes(pool, Config::default())
        .layer(Extension(Browser {
            account: None,
            csrf: String::new(),
            language: "pl".into(),
        }))
        .oneshot(Request::get("/oauth/authorize").body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn consent_policy_keeps_form_origin_and_does_not_send_query_referrers() -> TestResult {
    use axum::{Router, middleware, response::Html, routing::get};
    let app = Router::new()
        .route("/consent", get(|| async { Html("<main>Consent</main>") }))
        .route(
            "/protocol",
            get(|| async { axum::Json(serde_json::json!({"error":"invalid_request"})) }),
        )
        .layer(middleware::from_fn(oauth_http::secure_response));
    for (path, expected) in [("/consent", "strict-origin"), ("/protocol", "no-referrer")] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.headers()["referrer-policy"], expected);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(
            response.headers()["content-security-policy"]
                .to_str()?
                .contains("form-action 'self'")
        );
    }
    Ok(())
}
