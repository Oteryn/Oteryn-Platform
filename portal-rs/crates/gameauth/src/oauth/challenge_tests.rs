#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! OTERYN_NATIVE_LOGIN_CHALLENGE_V1, Platform side: counters, 403 envelope,
//! the browser check, signed one-use proofs and the flag-off baseline.
use super::challenge::*;
use super::*;
use axum::{
    Extension,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use std::net::{IpAddr, SocketAddr};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const REDIRECT: &str = "http://127.0.0.1:43210/callback";
const RETURN: &str = "http://127.0.0.1:50123/callback";
const STATE: &str = "Zm9yLXRoZS1uYXRpdmUtbG9naW4tY2hhbGxlbmdlLXRlc3Q";
// SOURCE: RFC 7636 Appendix B, public PKCE S256 test vector.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!(
                "SKIPPED native login challenge PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("challenge_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(6)
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
    async fn account(&self) -> Result<(Uuid, String), sqlx::Error> {
        let email = format!("{}@example.test", Uuid::now_v7().simple());
        let id = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-hash','active','en',now()) RETURNING id").bind(&email).fetch_one(&self.pool).await?;
        Ok((id, email))
    }
    async fn scalar(&self, sql: &'static str, id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(sql).bind(id).fetch_one(&self.pool).await
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
fn oauth() -> Config {
    Config {
        enabled: true,
        clients: vec![Client {
            client_id: "native-test".into(),
            redirect_uris: vec![REDIRECT.into()],
        }],
    }
}
fn key(byte: u8) -> String {
    URL_SAFE_NO_PAD.encode([byte; 32])
}
fn policy() -> ChallengeConfig {
    let mut config = ChallengeConfig {
        enabled: true,
        ..ChallengeConfig::default()
    };
    assert!(config.set_keys(&format!("k1={}", key(7))));
    config
}
fn source(last: u8) -> IpAddr {
    IpAddr::from([198, 51, 100, last])
}
fn authorization() -> Authorization {
    Authorization {
        response_type: "code".into(),
        client_id: "native-test".into(),
        redirect_uri: REDIRECT.into(),
        scope: "game:ticket".into(),
        state: "random-state-for-test".into(),
        code_challenge: CHALLENGE.into(),
        code_challenge_method: "S256".into(),
    }
}
async fn code(db: &Database, account: Uuid) -> Result<TokenRequest, Error> {
    Ok(TokenRequest {
        grant_type: "authorization_code".into(),
        client_id: "native-test".into(),
        redirect_uri: REDIRECT.into(),
        code_verifier: VERIFIER.into(),
        code: authorize(&db.pool, &oauth(), account, &authorization()).await?,
        ..Default::default()
    })
}
fn form(input: &TokenRequest) -> String {
    let mut f = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in [
        ("grant_type", &input.grant_type),
        ("client_id", &input.client_id),
        ("redirect_uri", &input.redirect_uri),
        ("code_verifier", &input.code_verifier),
        ("code", &input.code),
    ] {
        f.append_pair(k, v);
    }
    if let Some(id) = &input.challenge_id {
        f.append_pair("challenge_id", id);
    }
    if let Some(token) = &input.challenge_token {
        f.append_pair("challenge_token", token);
    }
    f.finish()
}
async fn post_token(
    app: &axum::Router,
    body: String,
) -> Result<(StatusCode, Vec<u8>, String), Box<dyn std::error::Error>> {
    let r = app
        .clone()
        .oneshot(
            Request::post("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))?,
        )
        .await?;
    let status = r.status();
    let cache = r
        .headers()
        .get("cache-control")
        .map(|v| v.to_str().unwrap_or_default().to_owned())
        .unwrap_or_default();
    Ok((
        status,
        to_bytes(r.into_body(), 16384).await?.to_vec(),
        cache,
    ))
}
fn token_app(db: &Database, challenge: ChallengeConfig, from: IpAddr) -> axum::Router {
    token_routes_with(db.pool.clone(), oauth(), challenge)
        .layer(Extension(ConnectInfo(SocketAddr::new(from, 40000))))
}
fn browser_app(db: &Database, account: Option<Uuid>, challenge: ChallengeConfig) -> axum::Router {
    browser_routes_with(db.pool.clone(), oauth(), challenge).layer(Extension(Browser {
        account,
        csrf: "a".repeat(64),
        language: "en".into(),
    }))
}
fn attempt_query(id: &str, redirect: &str, state: &str) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("client_id", "native-test")
        .append_pair("challenge_id", id)
        .append_pair("redirect_uri", redirect)
        .append_pair("state", state);
    q.finish()
}
async fn get_page(
    app: &axum::Router,
    query: &str,
) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let r = app
        .clone()
        .oneshot(Request::get(format!("/oauth/challenge?{query}")).body(Body::empty())?)
        .await?;
    let status = r.status();
    Ok((
        status,
        String::from_utf8(to_bytes(r.into_body(), 65536).await?.to_vec())?,
    ))
}
async fn post_page(
    app: &axum::Router,
    body: String,
) -> Result<axum::response::Response, Box<dyn std::error::Error>> {
    Ok(app
        .clone()
        .oneshot(
            Request::post("/oauth/challenge")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))?,
        )
        .await?)
}
fn shown_code(html: &str) -> String {
    let start = html
        .find("aria-describedby=\"challenge-alternative\">")
        .expect("code shown");
    html[start..]
        .split("</strong>")
        .next()
        .expect("code end")
        .split('>')
        .nth(1)
        .expect("code text")
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}
fn challenge_of(body: &[u8]) -> (String, i64) {
    let v: serde_json::Value = serde_json::from_slice(body).expect("json");
    (
        v["challenge_id"].as_str().expect("id").to_owned(),
        v["expires_in"].as_i64().expect("expires_in"),
    )
}
/// Drives page and form for a challenge and returns the loopback redirect.
async fn complete_in_browser(
    app: &axum::Router,
    id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let query = attempt_query(id, RETURN, STATE);
    let (status, html) = get_page(app, &query).await?;
    assert_eq!(status, StatusCode::OK, "{html}");
    let r = post_page(
        app,
        format!(
            "{query}&_csrf={}&answer={}",
            "a".repeat(64),
            shown_code(&html)
        ),
    )
    .await?;
    assert_eq!(r.status(), StatusCode::FOUND);
    Ok(r.headers()["location"].to_str()?.to_owned())
}
fn proof_of(location: &str) -> String {
    let url = url::Url::parse(location).expect("location");
    let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, "code");
    assert_eq!(pairs[1], ("state".into(), STATE.into()));
    pairs[0].1.clone()
}
async fn challenged(
    db: &Database,
    config: &ChallengeConfig,
    account: Uuid,
    email: &str,
) -> Result<(TokenRequest, Uuid), Box<dyn std::error::Error>> {
    for _ in 0..config.identity_failures {
        record_failure(&db.pool, config, email, source(200)).await?;
    }
    let input = code(db, account).await?;
    match super::challenge::exchange(&db.pool, &oauth(), config, &input, source(1)).await? {
        Outcome::Challenge { id, .. } => Ok((input, id)),
        Outcome::Tokens(_) => panic!("expected a challenge"),
    }
}

// ---------- no database ----------

#[test]
fn configuration_defaults_follow_the_owner_decision_and_fail_closed() {
    let c = ChallengeConfig::default();
    assert!(!c.enabled);
    assert_eq!(
        (
            c.identity_failures,
            c.identity_window_seconds,
            c.source_attempts,
            c.source_window_seconds,
            c.retention_seconds
        ),
        (5, 600, 20, 60, 86_400)
    );
    // Enabled without a signing key is not usable.
    let mut c = ChallengeConfig {
        enabled: true,
        ..ChallengeConfig::default()
    };
    assert!(!c.usable());
    for bad in [
        "",
        "k1",
        "k1=short",
        &format!("k1={}=", key(1)),
        &format!("bad kid={}", key(1)),
        &format!("k1={},k1={}", key(1), key(2)),
    ] {
        assert!(!c.set_keys(bad), "{bad}");
        assert!(!c.usable());
    }
    assert!(c.set_keys(&format!("k2={},k1={}", key(2), key(1))));
    assert!(c.usable());
    c.challenge_ttl_seconds = 301;
    assert!(!c.usable());
    c.challenge_ttl_seconds = 120;
    c.retention_seconds = 59;
    assert!(!c.usable());
    // Keys never reach Debug output.
    let debug = format!("{c:?}");
    assert!(!debug.contains(&key(2)) && !debug.contains("k2"));
    let unknown: Result<ChallengeConfig, _> = serde_json::from_str(r#"{"enabled":true,"x":1}"#);
    assert!(unknown.is_err());
}

#[test]
fn retry_fields_are_redacted_and_absent_by_default() {
    let input = TokenRequest {
        challenge_token: Some("secret-proof".into()),
        ..Default::default()
    };
    assert!(!format!("{input:?}").contains("secret-proof"));
    assert!(!TokenRequest::default().has_challenge_fields());
}

#[tokio::test]
async fn flag_off_challenge_page_is_not_routed_and_touches_no_database() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let browser = Browser {
        account: None,
        csrf: String::new(),
        language: "en".into(),
    };
    let off = browser_routes_with(pool.clone(), oauth(), ChallengeConfig::default())
        .layer(Extension(browser.clone()));
    for request in [
        Request::get("/oauth/challenge").body(Body::empty())?,
        Request::post("/oauth/challenge").body(Body::empty())?,
    ] {
        let r = off.clone().oneshot(request).await?;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert!(to_bytes(r.into_body(), 4096).await?.is_empty());
    }
    // Routed but refusing when the policy is on without a signing key.
    let keyless = ChallengeConfig {
        enabled: true,
        ..ChallengeConfig::default()
    };
    let r = browser_routes_with(pool, oauth(), keyless)
        .layer(Extension(browser))
        .oneshot(Request::get("/oauth/challenge").body(Body::empty())?)
        .await?;
    assert_ne!(r.status(), StatusCode::NOT_FOUND);
    assert_ne!(r.status(), StatusCode::OK);
    Ok(())
}

// ---------- PostgreSQL ----------

#[tokio::test]
async fn postgres_identity_threshold_n_minus_one_and_n() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    // Four failures (the email as typed differently) do not challenge.
    for _ in 0..4 {
        record_failure(
            &db.pool,
            &config,
            &format!("  {}  ", email.to_uppercase()),
            source(9),
        )
        .await?;
    }
    let input = code(&db, account).await?;
    let outcome =
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(1)).await?;
    assert!(matches!(outcome, Outcome::Tokens(_)));
    // The fifth does, before the code is consumed and without a credential.
    record_failure(&db.pool, &config, &email, source(10)).await?;
    let input = code(&db, account).await?;
    let families_before = db
        .scalar(
            "SELECT count(*) FROM oauth_token_families WHERE account_id=$1",
            account,
        )
        .await?;
    let expires_before: String = sqlx::query_scalar(
        "SELECT expires_at::text FROM oauth_authorization_codes WHERE code_hash=$1",
    )
    .bind(hash(&input.code))
    .fetch_one(&db.pool)
    .await?;
    let Outcome::Challenge { id, expires_in } =
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(1)).await?
    else {
        panic!("expected a challenge");
    };
    assert_eq!(id.get_version_num(), 7);
    assert!((1..=120).contains(&expires_in));
    let (used, expires_after): (bool, String) = sqlx::query_as(
        "SELECT used_at IS NOT NULL, expires_at::text FROM oauth_authorization_codes WHERE code_hash=$1",
    )
    .bind(hash(&input.code))
    .fetch_one(&db.pool)
    .await?;
    assert!(!used);
    assert_eq!(
        expires_before, expires_after,
        "code expiry is never extended"
    );
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM oauth_token_families WHERE account_id=$1",
            account
        )
        .await?,
        families_before
    );
    // Counters store digests only.
    let raw: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM native_login_challenge_counters WHERE position(convert_to($1,'UTF8') in key_hash)>0",
    )
    .bind(&email)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(raw, 0);
    // The legacy API never redeems a challenged code.
    assert_eq!(
        super::exchange(&db.pool, &oauth(), &input).await.err(),
        Some(Error::InvalidGrant)
    );
    // Nor does a second plain request: there is no challenge loop.
    assert_eq!(
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(1))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    db.close().await
}

#[tokio::test]
async fn postgres_source_rate_and_shared_counters() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = ChallengeConfig {
        source_attempts: 3,
        ..policy()
    };
    let (account, _) = db.account().await?;
    // A failed sign-in from the source counts as an attempt too.
    record_failure(&db.pool, &config, "nobody@example.test", source(5)).await?;
    let input = code(&db, account).await?;
    assert!(matches!(
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(5)).await?,
        Outcome::Tokens(_)
    ));
    // A second pool stands in for another replica: same counters.
    let replica = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(
            (*db.pool.connect_options())
                .clone()
                .options([("search_path", db.schema.as_str())]),
        )
        .await?;
    let input = code(&db, account).await?;
    assert!(matches!(
        super::challenge::exchange(&replica, &oauth(), &config, &input, source(5)).await?,
        Outcome::Challenge { .. }
    ));
    // Another source is unaffected.
    let input = code(&db, account).await?;
    assert!(matches!(
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(6)).await?,
        Outcome::Tokens(_)
    ));
    let reason: String =
        sqlx::query_scalar("SELECT reason FROM native_login_challenges WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(reason, "source");
    replica.close().await;
    db.close().await
}

#[tokio::test]
async fn postgres_failures_for_unknown_and_known_emails_are_identical() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (_, email) = db.account().await?;
    record_failure(&db.pool, &config, &email, source(1)).await?;
    record_failure(&db.pool, &config, "missing@example.test", source(2)).await?;
    let rows: Vec<(String, i32)> = sqlx::query_as(
        "SELECT kind, hits FROM native_login_challenge_counters ORDER BY kind, hits",
    )
    .fetch_all(&db.pool)
    .await?;
    assert_eq!(
        rows,
        vec![
            ("identity_failure".into(), 1),
            ("identity_failure".into(), 1),
            ("source_attempt".into(), 1),
            ("source_attempt".into(), 1)
        ]
    );
    // Retention: an old bucket is swept on the next write.
    sqlx::query("INSERT INTO native_login_challenge_counters(kind,key_hash,bucket_start,hits) VALUES('source_attempt',$1,now()-interval '25 hours',3)")
        .bind(hash("old")).execute(&db.pool).await?;
    record_failure(&db.pool, &config, &email, source(1)).await?;
    let old: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM native_login_challenge_counters WHERE bucket_start<now()-interval '24 hours'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(old, 0);
    db.close().await
}

#[tokio::test]
async fn postgres_http_challenge_success_and_replay() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    for _ in 0..5 {
        record_failure(&db.pool, &config, &email, source(9)).await?;
    }
    let mut input = code(&db, account).await?;
    let tokens = token_app(&db, config.clone(), source(1));
    let (status, body, cache) = post_token(&tokens, form(&input)).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(cache, "no-store");
    let (id, expires_in) = challenge_of(&body);
    assert_eq!(
        String::from_utf8(body)?,
        format!(
            r#"{{"error":"oteryn_challenge_required","challenge_id":"{id}","expires_in":{expires_in}}}"#
        )
    );
    assert!((1..=120).contains(&expires_in));
    // The browser check: a wrong answer shows a new code, the right one returns.
    let browser = browser_app(&db, Some(account), config.clone());
    let query = attempt_query(&id, RETURN, STATE);
    let (status, html) = get_page(&browser, &query).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Oteryn sign-in check"));
    assert!(html.contains("The same code in words"));
    // Only same-origin stylesheets; no third-party resource or script.
    assert!(!html.contains("src=") && !html.contains("href=\"http") && !html.contains("<script"));
    let first = shown_code(&html);
    assert_eq!(first.len(), 6);
    let wrong = if first == "000000" {
        "111111"
    } else {
        "000000"
    };
    let r = post_page(
        &browser,
        format!("{query}&_csrf={}&answer={wrong}", "a".repeat(64)),
    )
    .await?;
    assert_eq!(r.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(r.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("role=\"alert\""));
    let second = shown_code(&html);
    // The replaced code no longer works... unless it happens to repeat.
    if second != first {
        let r = post_page(
            &browser,
            format!("{query}&_csrf={}&answer={first}", "a".repeat(64)),
        )
        .await?;
        assert_eq!(r.status(), StatusCode::OK);
    }
    let (_, html) = get_page(&browser, &query).await?;
    let r = post_page(
        &browser,
        format!(
            "{query}&_csrf={}&answer={}",
            "a".repeat(64),
            shown_code(&html)
        ),
    )
    .await?;
    assert_eq!(r.status(), StatusCode::FOUND);
    let location = r.headers()["location"].to_str()?.to_owned();
    assert!(location.starts_with(&format!("{RETURN}?code=v1.k1.")));
    let proof = proof_of(&location);
    assert!(proof.len() <= 2048);
    assert!(
        proof
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    );
    // A completed challenge cannot be shown or completed again.
    let (status, _) = get_page(&browser, &query).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // The retry issues exactly one credential.
    input.challenge_id = Some(id.clone());
    input.challenge_token = Some(proof);
    let (status, body, cache) = post_token(&tokens, form(&input)).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(cache, "no-store");
    let v: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(
        verify_access(
            &db.pool,
            &oauth(),
            v["access_token"].as_str().unwrap_or_default()
        )
        .await?,
        account
    );
    let (status, body, _) = post_token(&tokens, form(&input)).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        &body[..],
        br#"{"error":"invalid_grant","error_description":"The provided authorization grant is invalid."}"#
    );
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM oauth_token_families WHERE account_id=$1",
            account
        )
        .await?,
        1
    );
    db.close().await
}

#[tokio::test]
async fn postgres_invalid_proofs_get_one_generic_refusal() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    let (input, id) = challenged(&db, &config, account, &email).await?;
    let browser = browser_app(&db, Some(account), config.clone());
    let proof = proof_of(&complete_in_browser(&browser, &id.to_string()).await?);
    let retry = |token: &str, challenge: &str| TokenRequest {
        challenge_id: Some(challenge.into()),
        challenge_token: Some(token.into()),
        ..input.clone_request()
    };
    let parts: Vec<&str> = proof.split('.').collect();
    // Signature of another key, with the same claims.
    let mut other = ChallengeConfig {
        enabled: true,
        ..ChallengeConfig::default()
    };
    assert!(other.set_keys(&format!("k1={}", key(8))));
    let mut flipped = parts[3].to_owned();
    let last = if flipped.ends_with('A') { "B" } else { "A" };
    flipped.replace_range(flipped.len() - 1.., last);
    let tampered_payload = {
        let mut claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[2])?)?;
        claims["exp"] = serde_json::json!(claims["exp"].as_i64().unwrap_or_default() + 600);
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?)
    };
    let other_id = Uuid::now_v7().to_string();
    for (token, challenge) in [
        (
            format!("{}.{}.{}.{flipped}", parts[0], parts[1], parts[2]),
            id.to_string(),
        ),
        (
            format!("{}.{}.{tampered_payload}.{}", parts[0], parts[1], parts[3]),
            id.to_string(),
        ),
        (
            format!("{}.k9.{}.{}", parts[0], parts[2], parts[3]),
            id.to_string(),
        ),
        (
            format!("{}.{}.{}", parts[0], parts[1], parts[2]),
            id.to_string(),
        ),
        (
            format!("v2.{}.{}.{}", parts[1], parts[2], parts[3]),
            id.to_string(),
        ),
        (proof.clone(), other_id.clone()),
        ("public-test-token".into(), id.to_string()),
        (proof.clone(), id.to_string().to_uppercase()),
    ] {
        assert_eq!(
            super::challenge::exchange(
                &db.pool,
                &oauth(),
                &config,
                &retry(&token, &challenge),
                source(1)
            )
            .await
            .err(),
            Some(Error::InvalidGrant),
            "{token}"
        );
    }
    // The genuine proof under a verifier that does not hold its key.
    assert_eq!(
        super::challenge::exchange(
            &db.pool,
            &oauth(),
            &other,
            &retry(&proof, &id.to_string()),
            source(1)
        )
        .await
        .err(),
        Some(Error::InvalidGrant)
    );
    // A different policy version is refused.
    let newer = ChallengeConfig {
        policy_version: 2,
        ..config.clone()
    };
    assert_eq!(
        super::challenge::exchange(
            &db.pool,
            &oauth(),
            &newer,
            &retry(&proof, &id.to_string()),
            source(1)
        )
        .await
        .err(),
        Some(Error::InvalidGrant)
    );
    // Half a retry is malformed.
    let half = TokenRequest {
        challenge_id: Some(id.to_string()),
        ..input.clone_request()
    };
    assert_eq!(
        super::challenge::exchange(&db.pool, &oauth(), &config, &half, source(1))
            .await
            .err(),
        Some(Error::InvalidRequest)
    );
    // A wrong PKCE verifier on the retry fails before the proof is spent.
    let mut wrong = retry(&proof, &id.to_string());
    wrong.code_verifier = "x".repeat(43);
    assert_eq!(
        super::challenge::exchange(&db.pool, &oauth(), &config, &wrong, source(1))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    // None of these spent the proof: the real retry still works once.
    assert!(matches!(
        super::challenge::exchange(
            &db.pool,
            &oauth(),
            &config,
            &retry(&proof, &id.to_string()),
            source(1)
        )
        .await?,
        Outcome::Tokens(_)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_expiry_of_challenge_or_code_refuses_the_retry() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    let browser = browser_app(&db, Some(account), config.clone());
    for expire in [
        "UPDATE native_login_challenges SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1",
        "UPDATE oauth_authorization_codes SET expires_at=clock_timestamp()-interval '1 second' WHERE code_hash=(SELECT code_hash FROM native_login_challenges WHERE id=$1)",
    ] {
        let (input, id) = challenged(&db, &config, account, &email).await?;
        let proof = proof_of(&complete_in_browser(&browser, &id.to_string()).await?);
        sqlx::query(expire).bind(id).execute(&db.pool).await?;
        let retry = TokenRequest {
            challenge_id: Some(id.to_string()),
            challenge_token: Some(proof),
            ..input.clone_request()
        };
        assert_eq!(
            super::challenge::exchange(&db.pool, &oauth(), &config, &retry, source(1))
                .await
                .err(),
            Some(Error::InvalidGrant)
        );
    }
    // A challenge never outlives its code.
    let (_, _) = (account, &email);
    let input = code(&db, account).await?;
    sqlx::query("UPDATE oauth_authorization_codes SET expires_at=clock_timestamp()+interval '30 seconds' WHERE code_hash=$1")
        .bind(hash(&input.code)).execute(&db.pool).await?;
    let Outcome::Challenge { expires_in, .. } =
        super::challenge::exchange(&db.pool, &oauth(), &config, &input, source(1)).await?
    else {
        panic!("expected a challenge");
    };
    assert!((1..=30).contains(&expires_in));
    // An expired challenge page is refused like an unknown one.
    let (_, id) = challenged(&db, &config, account, &email).await?;
    sqlx::query("UPDATE native_login_challenges SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(id).execute(&db.pool).await?;
    let (status, expired) =
        get_page(&browser, &attempt_query(&id.to_string(), RETURN, STATE)).await?;
    let (_, unknown) = get_page(
        &browser,
        &attempt_query(&Uuid::now_v7().to_string(), RETURN, STATE),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(expired, unknown);
    db.close().await
}

#[tokio::test]
async fn postgres_concurrent_retries_issue_one_credential() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    let (input, id) = challenged(&db, &config, account, &email).await?;
    let browser = browser_app(&db, Some(account), config.clone());
    let proof = proof_of(&complete_in_browser(&browser, &id.to_string()).await?);
    let retry = TokenRequest {
        challenge_id: Some(id.to_string()),
        challenge_token: Some(proof),
        ..input.clone_request()
    };
    let oauth = oauth();
    let results = futures_join(
        super::challenge::exchange(&db.pool, &oauth, &config, &retry, source(1)),
        super::challenge::exchange(&db.pool, &oauth, &config, &retry, source(2)),
    )
    .await;
    let issued = results
        .iter()
        .filter(|r| matches!(r, Ok(Outcome::Tokens(_))))
        .count();
    assert_eq!(issued, 1);
    assert!(
        results
            .iter()
            .any(|r| matches!(r, Err(Error::InvalidGrant)))
    );
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM oauth_token_families WHERE account_id=$1",
            account
        )
        .await?,
        1
    );
    // A rolled-back retry (code revoked under it) leaves the proof unspent.
    let (input, id) = challenged(&db, &config, account, &email).await?;
    let proof = proof_of(&complete_in_browser(&browser, &id.to_string()).await?);
    sqlx::query("UPDATE oauth_authorization_codes SET revoked_at=now() WHERE code_hash=$1")
        .bind(hash(&input.code))
        .execute(&db.pool)
        .await?;
    let retry = TokenRequest {
        challenge_id: Some(id.to_string()),
        challenge_token: Some(proof),
        ..input.clone_request()
    };
    assert_eq!(
        super::challenge::exchange(&db.pool, &oauth, &config, &retry, source(1))
            .await
            .err(),
        Some(Error::InvalidGrant)
    );
    let consumed: bool = sqlx::query_scalar(
        "SELECT consumed_at IS NOT NULL FROM native_login_challenges WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    assert!(!consumed);
    db.close().await
}
async fn futures_join<A, B, T>(a: A, b: B) -> [T; 2]
where
    A: std::future::Future<Output = T>,
    B: std::future::Future<Output = T>,
{
    let (a, b) = tokio::join!(a, b);
    [a, b]
}

#[tokio::test]
async fn postgres_challenge_page_validates_binding_identity_and_csrf() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let config = policy();
    let (account, email) = db.account().await?;
    let (other, _) = db.account().await?;
    let (_, id) = challenged(&db, &config, account, &email).await?;
    let id = id.to_string();
    let mine = browser_app(&db, Some(account), config.clone());
    let (_, unknown) = get_page(
        &mine,
        &attempt_query(&Uuid::now_v7().to_string(), RETURN, STATE),
    )
    .await?;
    // Another account's browser gets the same refusal as an unknown id.
    let (status, foreign) = get_page(
        &browser_app(&db, Some(other), config.clone()),
        &attempt_query(&id, RETURN, STATE),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(foreign, unknown);
    // Signed out: the sign-in page, nothing bound.
    let r = browser_app(&db, None, config.clone())
        .oneshot(
            Request::get(format!(
                "/oauth/challenge?{}",
                attempt_query(&id, RETURN, STATE)
            ))
            .body(Body::empty())?,
        )
        .await?;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert_eq!(r.headers()["location"], "/en/login");
    // Only an IPv4 loopback URI registered for the client is a return target.
    for redirect in [
        "http://[::1]:50123/callback",
        "https://example.test/callback",
        "http://127.0.0.1:50123/other",
        "http://127.0.0.1:50123/callback?x=1",
        "http://localhost:50123/callback",
    ] {
        let (status, body) = get_page(&mine, &attempt_query(&id, redirect, STATE)).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{redirect}");
        assert_eq!(body, unknown);
    }
    for state in ["short", &"x".repeat(1025), "has space in it ......"] {
        let (status, _) = get_page(&mine, &attempt_query(&id, RETURN, state)).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    // A POST before the page was shown is refused, and binds nothing.
    let query = attempt_query(&id, RETURN, STATE);
    let r = post_page(
        &mine,
        format!("{query}&_csrf={}&answer=123456", "a".repeat(64)),
    )
    .await?;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    // The first view binds return URI and state; another port or state is refused.
    let (status, html) = get_page(&mine, &query).await?;
    assert_eq!(status, StatusCode::OK);
    for (redirect, state) in [
        ("http://127.0.0.1:50124/callback", STATE),
        (RETURN, "another-state-of-enough-length"),
    ] {
        let (status, _) = get_page(&mine, &attempt_query(&id, redirect, state)).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let answer = shown_code(&html);
    // CSRF, duplicate and unknown fields are refused without spending attempts.
    for body in [
        format!("{query}&answer={answer}"),
        format!("{query}&_csrf=wrong&answer={answer}"),
        format!(
            "{query}&_csrf={0}&_csrf={0}&answer={answer}",
            "a".repeat(64)
        ),
        format!("{query}&_csrf={}&answer={answer}&extra=1", "a".repeat(64)),
    ] {
        let r = post_page(&mine, body).await?;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }
    let attempts = db
        .scalar(
            "SELECT answer_attempts::bigint FROM native_login_challenges WHERE id=$1",
            Uuid::parse_str(&id)?,
        )
        .await?;
    assert_eq!(attempts, 0);
    // Attempts are capped: the fifth wrong answer ends the challenge.
    let wrong = if answer == "000000" {
        "111111"
    } else {
        "000000"
    };
    for n in 1..=5 {
        let r = post_page(
            &mine,
            format!("{query}&_csrf={}&answer={wrong}", "a".repeat(64)),
        )
        .await?;
        let expected = if n < 5 {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        assert_eq!(r.status(), expected, "attempt {n}");
    }
    let (status, _) = get_page(&mine, &query).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    db.close().await
}

#[tokio::test]
async fn postgres_flag_off_keeps_the_token_route_unchanged() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (account, email) = db.account().await?;
    // Counters past every threshold change nothing while the flag is off.
    for _ in 0..10 {
        record_failure(&db.pool, &policy(), &email, source(1)).await?;
    }
    let off = token_app(&db, ChallengeConfig::default(), source(1));
    let input = code(&db, account).await?;
    let (status, body, cache) = post_token(&off, form(&input)).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache, "no-store");
    let v: serde_json::Value = serde_json::from_slice(&body)?;
    let keys: Vec<&String> = v
        .as_object()
        .map(|o| o.keys().collect())
        .unwrap_or_default();
    assert_eq!(
        keys,
        [
            "access_token",
            "expires_in",
            "refresh_token",
            "scope",
            "token_type"
        ]
    );
    // The retry fields are refused byte for byte like any unknown field was.
    let input = code(&db, account).await?;
    let base = form(&input);
    let (_, unknown, _) = post_token(&off, format!("{base}&unexpected=1")).await?;
    assert_eq!(
        &unknown[..],
        br#"{"error":"invalid_request","error_description":"The request is missing a required parameter or is malformed."}"#
    );
    for extra in [
        "challenge_id=01890a5d-ac96-774b-bcce-b302099a8057",
        "challenge_token=v1.a.b.c",
        "challenge_id=",
        "challenge_id=a&challenge_id=b",
    ] {
        let (status, body, _) = post_token(&off, format!("{base}&{extra}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}");
        assert_eq!(body, unknown, "{extra}");
    }
    let refresh = "grant_type=refresh_token&client_id=native-test&refresh_token=x&challenge_id=a";
    let (_, body, _) = post_token(&off, refresh.into()).await?;
    assert_eq!(body, unknown);
    // That code was not consumed by the refusals and still redeems normally.
    let (status, _, _) = post_token(&off, base).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        db.scalar(
            "SELECT count(*) FROM native_login_challenges WHERE account_id=$1",
            account
        )
        .await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn postgres_enabled_without_keys_or_source_fails_closed() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (account, _) = db.account().await?;
    let keyless = ChallengeConfig {
        enabled: true,
        ..ChallengeConfig::default()
    };
    let input = code(&db, account).await?;
    let (status, _, _) = post_token(&token_app(&db, keyless, source(1)), form(&input)).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    // No socket peer, no source: refused rather than counted as nobody.
    let app = token_routes_with(db.pool.clone(), oauth(), policy());
    let (status, _, _) = post_token(&app, form(&input)).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let used: bool = sqlx::query_scalar(
        "SELECT used_at IS NOT NULL FROM oauth_authorization_codes WHERE code_hash=$1",
    )
    .bind(hash(&input.code))
    .fetch_one(&db.pool)
    .await?;
    assert!(!used);
    db.close().await
}

impl TokenRequest {
    fn clone_request(&self) -> Self {
        Self {
            grant_type: self.grant_type.clone(),
            client_id: self.client_id.clone(),
            redirect_uri: self.redirect_uri.clone(),
            code: self.code.clone(),
            code_verifier: self.code_verifier.clone(),
            refresh_token: self.refresh_token.clone(),
            scope: self.scope.clone(),
            challenge_id: self.challenge_id.clone(),
            challenge_token: self.challenge_token.clone(),
        }
    }
}
