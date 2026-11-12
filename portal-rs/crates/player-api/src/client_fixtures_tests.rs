#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Byte-exact proof of `docs/contracts/fixtures/native-oauth-ticket-v1` (ticket
//! cases) and `docs/contracts/fixtures/native-characters-v1` through the real
//! player API router, with the real OAuth verifier and LCFA projection.
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_characters::{lcfa, overview};
use oteryn_gameauth::{devices::PlatformAuthority, oauth};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{collections::BTreeSet, path::PathBuf};
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const TICKET_DIR: &str = "native-oauth-ticket-v1";
const CHARACTER_DIR: &str = "native-characters-v1";
const ACCOUNT: &str = "019a5f4a-2000-7000-8000-000000000003";
const AUTHORITY: &str = "oteryn:character-authority:primary";
const ALDRIC: &str = "019a5f4a-4000-7000-8000-000000000005";
const BRENNA: &str = "019a5f4a-4000-7000-8000-000000000006";

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
fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}
fn cases(m: &serde_json::Value, prefix: &str) -> BTreeSet<String> {
    m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| text(c, "case").to_owned())
        .filter(|c| c.starts_with(prefix))
        .collect()
}
/// A ticket has the shape of a 32-byte unpadded base64url value.
fn opaque(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Sends one manifest case and checks status, listed headers and body bytes.
/// Returns the raw body, before normalization.
async fn drive(app: &Router, dir: &str, m: &serde_json::Value, name: &str) -> Vec<u8> {
    let c = m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["case"] == name)
        .unwrap_or_else(|| panic!("case {name} missing from {dir}"));
    let mut request = Request::builder()
        .method(text(c, "method"))
        .uri(text(c, "path"));
    for (key, value) in c["headers"].as_object().unwrap() {
        request = request.header(key.as_str(), value.as_str().unwrap());
    }
    let body = c["request"]
        .as_str()
        .map_or_else(Vec::new, |f| file(dir, f));
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
        let value: serde_json::Value = serde_json::from_slice(&actual).unwrap();
        let random = text(&value, field).to_owned();
        assert!(opaque(&random), "{name}: {field}");
        let from = format!(r#""{field}":"{random}""#);
        let body = String::from_utf8(actual).unwrap();
        assert_eq!(body.matches(&from).count(), 1, "{name}: {field}");
        let to = format!(r#""{field}":"{}""#, text(&m["placeholders"], field));
        actual = body.replace(&from, &to).into_bytes();
    }
    let expected = c["response"]
        .as_str()
        .map_or_else(Vec::new, |f| file(dir, f));
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "{name}"
    );
    raw
}

/// The handler's own request and response types read and write these bytes.
#[test]
fn fixtures_parse_with_the_handler_types() -> TestResult {
    let request: TicketRequest = serde_json::from_slice(&file(TICKET_DIR, "ticket-request.json"))?;
    assert_eq!(request.protocol_version, 1);
    let unsupported: TicketRequest =
        serde_json::from_slice(&file(TICKET_DIR, "ticket-request-unsupported.json"))?;
    assert_ne!(unsupported.protocol_version, 1);
    let m = manifest(TICKET_DIR);
    let issued = oteryn_gameauth::tickets::Issued {
        protocol_version: 1,
        ticket: text(&m["placeholders"], "ticket").into(),
        expires_in: 60,
    };
    assert_eq!(
        serde_json::to_vec(&issued)?,
        file(TICKET_DIR, "ticket-response.json")
    );
    let m = manifest(CHARACTER_DIR);
    let world = text(&m, "world_id");
    let list = |characters: Vec<Character>| CharacterList {
        protocol_version: 2,
        characters,
    };
    assert_eq!(
        serde_json::to_vec(&list(vec![
            Character {
                character_id: ALDRIC.into(),
                world_id: world.into(),
                name: "Aldric".into(),
                availability: "AVAILABLE",
            },
            Character {
                character_id: BRENNA.into(),
                world_id: world.into(),
                name: "Brenna".into(),
                availability: "UNAVAILABLE",
            },
        ]))?,
        file(CHARACTER_DIR, "characters-response.json")
    );
    assert_eq!(
        serde_json::to_vec(&list(Vec::new()))?,
        file(CHARACTER_DIR, "characters-empty.json")
    );
    Ok(())
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    config: oauth::Config,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED client fixture PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        // Generated from a UUID only, never from input.
        let schema = format!("client_fixture_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let m = manifest(TICKET_DIR);
        let client = text(&m, "client_id");
        sqlx::query("INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket) VALUES($1,true,true,true,true)")
            .bind(Uuid::parse_str(client)?).execute(&pool).await?;
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'fixture-player@example.invalid','test-hash','active','en',now())")
            .bind(Uuid::parse_str(ACCOUNT)?).execute(&pool).await?;
        let config = oauth::Config {
            enabled: true,
            clients: vec![oauth::Client {
                client_id: client.into(),
                redirect_uris: vec![text(&m, "redirect_uri").into()],
            }],
        };
        Ok(Some(Self {
            pool,
            admin,
            schema,
            config,
        }))
    }
    /// A live `game:ticket` access token from the real code exchange, pinned to
    /// the fixture's synthetic bearer. A previous holder of that bearer is retired.
    async fn bearer(&self) -> TestResult {
        let m = manifest(TICKET_DIR);
        let client = text(&m, "client_id");
        let redirect = text(&m, "redirect_uri");
        let code = oauth::authorize(
            &self.pool,
            &self.config,
            Uuid::parse_str(ACCOUNT)?,
            &oauth::Authorization {
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
        let tokens = oauth::exchange(
            &self.pool,
            &self.config,
            &oauth::TokenRequest {
                grant_type: "authorization_code".into(),
                client_id: client.into(),
                redirect_uri: redirect.into(),
                code,
                code_verifier: text(&m, "code_verifier").into(),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
        let synthetic = text(&m, "synthetic_access_token");
        sqlx::query("UPDATE oauth_tokens SET access_hash=sha256(convert_to('retired:' || id::text,'UTF8')) WHERE access_hash=sha256(convert_to($1,'UTF8'))")
            .bind(synthetic).execute(&self.pool).await?;
        sqlx::query("UPDATE oauth_tokens SET access_hash=sha256(convert_to($1,'UTF8')) WHERE access_hash=sha256(convert_to($2,'UTF8'))")
            .bind(synthetic).bind(&tokens.access_token).execute(&self.pool).await?;
        Ok(())
    }
    fn api(&self, enabled: bool) -> Router {
        router(Api {
            tickets: Tickets::new(
                self.pool.clone(),
                Settings {
                    enabled,
                    nonproduction: true,
                },
                Arc::new(PlatformAuthority {
                    oauth: self.config.clone(),
                    projection_authority: String::new(),
                }),
            ),
            projection: Some(Projection::new(
                self.pool.clone(),
                lcfa::Settings {
                    enabled: true,
                    environment: oteryn_core::Environment::Testing,
                    source_authority: AUTHORITY.into(),
                    ..Default::default()
                },
                Arc::new(overview::Accounts(self.pool.clone())),
            )),
        })
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

#[tokio::test]
async fn pg_ticket_issuance_matches_native_oauth_ticket_fixture() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let m = manifest(TICKET_DIR);
    let synthetic = text(&m, "synthetic_access_token");
    db.bearer().await?;
    let app = db.api(true);
    let mut covered = BTreeSet::new();
    let mut run = async |app: &Router, name: &str| {
        covered.insert(name.to_owned());
        drive(app, TICKET_DIR, &m, name).await
    };
    run(&app, "ticket-missing-bearer").await;
    run(&app, "ticket-unknown-bearer").await;
    run(&app, "ticket-unsupported-version").await;
    run(&db.api(false), "ticket-flag-off").await;
    let raw = run(&app, "ticket-success").await;
    let issued: serde_json::Value = serde_json::from_slice(&raw)?;
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM game_login_tickets WHERE ticket_hash=sha256(convert_to($1,'UTF8')) AND account_id=$2 AND audience='oteryn-native-game-gateway'")
        .bind(text(&issued, "ticket")).bind(Uuid::parse_str(ACCOUNT)?).fetch_one(&db.pool).await?;
    assert_eq!(stored, 1);
    run(&app, "ticket-replay").await;
    // A fresh bearer that already spent this window's five issuance requests.
    db.bearer().await?;
    sqlx::query("INSERT INTO game_ticket_budgets(credential_hash,window_start,requests) VALUES(sha256(convert_to('ticket:' || $1,'UTF8')),clock_timestamp(),5) ON CONFLICT (credential_hash) DO UPDATE SET window_start=EXCLUDED.window_start, requests=EXCLUDED.requests")
        .bind(synthetic).execute(&db.pool).await?;
    run(&app, "ticket-rate-limited").await;
    db.pool.close().await;
    run(&app, "ticket-database-unavailable").await;
    assert_eq!(covered, cases(&m, "ticket-"));
    db.close().await
}

#[tokio::test]
async fn pg_native_characters_match_native_characters_fixture() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let m = manifest(CHARACTER_DIR);
    db.bearer().await?;
    let app = db.api(true);
    let mut covered = BTreeSet::new();
    let mut run = async |app: &Router, name: &str| {
        covered.insert(name.to_owned());
        drive(app, CHARACTER_DIR, &m, name).await
    };
    run(&app, "characters-missing-bearer").await;
    run(&app, "characters-unknown-bearer").await;
    run(&db.api(false), "characters-flag-off").await;
    run(&app, "characters-feed-stale").await;
    // A live watermark for epoch 1 from the configured Character Authority.
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1', watermark_epoch='1', source_authority=$1, complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint - 1, observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1")
        .bind(AUTHORITY).execute(&db.pool).await?;
    run(&app, "characters-no-snapshot").await;
    let account = Uuid::parse_str(ACCOUNT)?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'1','1',floor(extract(epoch FROM clock_timestamp()))::bigint - 1,sha256('fixture'::bytea))")
        .bind(account).bind(AUTHORITY).execute(&db.pool).await?;
    let world = Uuid::parse_str(text(&m, "world_id"))?;
    for (character, name, availability) in [
        (BRENNA, "Brenna", "UNAVAILABLE"),
        (ALDRIC, "Aldric", "AVAILABLE"),
    ] {
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,$5)")
            .bind(account).bind(Uuid::parse_str(character)?).bind(world).bind(name).bind(availability).execute(&db.pool).await?;
    }
    run(&app, "characters-success").await;
    // Reads 4 to 10 of this bearer's window still succeed; the 11th is refused.
    for _ in 4..=10 {
        run(&app, "characters-success").await;
    }
    run(&app, "characters-rate-limited").await;
    assert_eq!(covered, cases(&m, "characters-"));
    db.close().await
}
