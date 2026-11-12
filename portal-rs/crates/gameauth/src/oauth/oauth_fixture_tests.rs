#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Byte-exact proof of `docs/contracts/fixtures/native-oauth-ticket-v1` (token
//! cases) through the real `/oauth/token` router, plus the manifest digests of
//! both native client fixture directories.
use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::FromRequest,
    http::Request,
};
use std::{collections::BTreeSet, path::PathBuf};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ACCOUNT: &str = "019a5f4a-2000-7000-8000-000000000003";

fn root(dir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures")
        .join(dir)
}
fn manifest(dir: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root(dir).join("manifest.json")).unwrap()).unwrap()
}
fn file(dir: &str, name: &str) -> Vec<u8> {
    std::fs::read(root(dir).join(name)).unwrap()
}
fn case<'a>(manifest: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["case"] == name)
        .unwrap_or_else(|| panic!("case {name} missing from manifest"))
}
fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}

#[test]
fn native_client_fixture_manifests_match_their_files() {
    for dir in ["native-oauth-ticket-v1", "native-characters-v1"] {
        let manifest = manifest(dir);
        let mut listed = BTreeSet::new();
        for entry in manifest["files"].as_array().unwrap() {
            let name = text(entry, "file");
            let bytes = file(dir, name);
            assert_eq!(
                bytes.len() as u64,
                entry["bytes"].as_u64().unwrap(),
                "{name}"
            );
            let digest: String = hash_bytes(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(digest, text(entry, "sha256"), "{name}");
            assert!(!bytes.ends_with(b"\n"), "{name} has a trailing newline");
            listed.insert(name.to_owned());
        }
        let mut on_disk = BTreeSet::new();
        for entry in std::fs::read_dir(root(dir)).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if name != "manifest.json" && name != "README.md" {
                on_disk.insert(name);
            }
        }
        assert_eq!(listed, on_disk, "{dir}: every vector is listed once");
        for c in manifest["cases"].as_array().unwrap() {
            for key in ["request", "response"] {
                if let Some(name) = c[key].as_str() {
                    assert!(listed.contains(name), "{dir}: {name}");
                }
            }
        }
    }
}
fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// The handler's own extractor and response type read and write these bytes.
#[tokio::test]
async fn token_fixtures_parse_with_the_handler_types() -> TestResult {
    const DIR: &str = "native-oauth-ticket-v1";
    let m = manifest(DIR);
    let form = |name: &str| {
        Request::post("/oauth/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(file(DIR, name)))
            .unwrap()
    };
    let axum::Form(code) =
        axum::Form::<TokenRequest>::from_request(form("token-request-code.form"), &())
            .await
            .map_err(|e| e.to_string())?;
    assert_eq!(code.grant_type, "authorization_code");
    assert_eq!(code.client_id, text(&m, "client_id"));
    assert_eq!(code.redirect_uri, text(&m, "redirect_uri"));
    assert_eq!(code.code, text(&m, "synthetic_authorization_code"));
    assert_eq!(code.code_verifier, text(&m, "code_verifier"));
    assert!(opaque(&code.code) && verifier(&code.code_verifier));
    assert_eq!(
        URL_SAFE_NO_PAD.encode(hash(&code.code_verifier)),
        text(&m, "code_challenge")
    );
    let axum::Form(refresh) =
        axum::Form::<TokenRequest>::from_request(form("token-request-refresh.form"), &())
            .await
            .map_err(|e| e.to_string())?;
    assert_eq!(refresh.grant_type, "refresh_token");
    assert_eq!(refresh.refresh_token, text(&m, "synthetic_refresh_token"));
    assert!(
        axum::Form::<TokenRequest>::from_request(form("token-request-unknown-field.form"), &())
            .await
            .is_err()
    );
    let placeholders = &m["placeholders"];
    let tokens = Tokens {
        token_type: "bearer",
        expires_in: 1800,
        access_token: text(placeholders, "access_token").into(),
        refresh_token: text(placeholders, "refresh_token").into(),
        scope: "game:ticket",
    };
    assert!(opaque(&tokens.access_token) && opaque(&tokens.refresh_token));
    assert_eq!(
        serde_json::to_vec(&tokens)?,
        file(DIR, "token-response.json")
    );
    Ok(())
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED OAuth fixture PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        // Generated from a UUID only, never from input.
        let schema = format!("oauth_fixture_{}", Uuid::now_v7().simple());
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

/// Replaces the one random opaque value of `field` with its fixture placeholder.
fn normalize(body: &[u8], field: &str, placeholder: &str) -> (Vec<u8>, String) {
    let value: serde_json::Value = serde_json::from_slice(body).unwrap();
    let actual = value[field].as_str().unwrap().to_owned();
    assert!(opaque(&actual), "{field} is not a 32-byte base64url value");
    let text = std::str::from_utf8(body).unwrap();
    let from = format!(r#""{field}":"{actual}""#);
    assert_eq!(text.matches(&from).count(), 1, "{field}");
    let to = format!(r#""{field}":"{placeholder}""#);
    (text.replace(&from, &to).into_bytes(), actual)
}

/// Sends one manifest case and checks status, listed headers and body bytes.
/// Returns the raw body, before normalization.
async fn drive(app: &Router, m: &serde_json::Value, name: &str) -> Vec<u8> {
    const DIR: &str = "native-oauth-ticket-v1";
    let c = case(m, name);
    let mut request = Request::builder()
        .method(text(c, "method"))
        .uri(text(c, "path"));
    for (key, value) in c["headers"].as_object().unwrap() {
        request = request.header(key.as_str(), value.as_str().unwrap());
    }
    let body = c["request"]
        .as_str()
        .map_or_else(Vec::new, |f| file(DIR, f));
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(
        u64::from(response.status().as_u16()),
        c["status"].as_u64().unwrap(),
        "{name}"
    );
    for (key, value) in c["response_headers"].as_object().unwrap() {
        assert_eq!(
            response
                .headers()
                .get(key.as_str())
                .map(|v| v.to_str().unwrap()),
            value.as_str(),
            "{name}: {key}"
        );
    }
    let raw = to_bytes(response.into_body(), 65536)
        .await
        .unwrap()
        .to_vec();
    let mut actual = raw.clone();
    for field in c["normalized"].as_array().unwrap() {
        let field = field.as_str().unwrap();
        actual = normalize(&actual, field, text(&m["placeholders"], field)).0;
    }
    let expected = c["response"]
        .as_str()
        .map_or_else(Vec::new, |f| file(DIR, f));
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "{name}"
    );
    raw
}

#[tokio::test]
async fn pg_token_endpoint_matches_native_oauth_ticket_fixture() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let m = manifest("native-oauth-ticket-v1");
    let client = text(&m, "client_id");
    let redirect = text(&m, "redirect_uri");
    let config = Config {
        enabled: true,
        clients: vec![Client {
            client_id: client.into(),
            redirect_uris: vec![redirect.into()],
        }],
    };
    let account = Uuid::parse_str(ACCOUNT)?;
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'fixture-player@example.invalid','test-hash','active','en',now())")
        .bind(account).execute(&db.pool).await?;
    let code = authorize(
        &db.pool,
        &config,
        account,
        &Authorization {
            response_type: "code".into(),
            client_id: client.into(),
            redirect_uri: redirect.into(),
            scope: "game:ticket".into(),
            state: "fixture-state-0001".into(),
            code_challenge: text(&m, "code_challenge").into(),
            code_challenge_method: "S256".into(),
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    // Pin the server-chosen code to the fixture's synthetic value.
    sqlx::query("UPDATE oauth_authorization_codes SET code_hash=$1 WHERE code_hash=$2")
        .bind(hash(text(&m, "synthetic_authorization_code")))
        .bind(hash(&code))
        .execute(&db.pool)
        .await?;
    let app = token_routes(db.pool.clone(), config);
    let mut covered = BTreeSet::new();

    let raw = drive(&app, &m, "token-code-success").await;
    let issued: serde_json::Value = serde_json::from_slice(&raw)?;
    let refresh = issued["refresh_token"].as_str().unwrap();
    assert_ne!(issued["access_token"], issued["refresh_token"]);
    sqlx::query("UPDATE oauth_tokens SET refresh_hash=$1 WHERE refresh_hash=$2")
        .bind(hash(text(&m, "synthetic_refresh_token")))
        .bind(hash(refresh))
        .execute(&db.pool)
        .await?;
    for name in [
        "token-code-success",
        "token-code-replay",
        "token-refresh-success",
        "token-unknown-field",
        "token-unsupported-grant",
        "token-unknown-client",
    ] {
        if name != "token-code-success" {
            drive(&app, &m, name).await;
        }
        covered.insert(name.to_owned());
    }
    let listed: BTreeSet<String> = m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| text(c, "case").to_owned())
        .filter(|c| c.starts_with("token-"))
        .collect();
    assert_eq!(covered, listed);
    db.close().await
}
