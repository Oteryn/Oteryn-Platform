use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ACCOUNT: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70";
const WORLD: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71";
const CHANNEL: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a72";
const CHARACTER: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a73";
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a74";
const TICKET: &str = "synthetic-native-ticket";
const CURRENT: &str = "synthetic-current-service-credential";
const PREVIOUS: &str = "synthetic-previous-service-credential";

fn body() -> Vec<u8> {
    format!(r#"{{"protocol_version":2,"game_login_ticket":"{TICKET}","attempt_ref":"{ATTEMPT}","character_id":"{CHARACTER}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#).into_bytes()
}
fn tokens() -> Result<ServiceTokens, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("native-admission-{}", Uuid::now_v7()));
    std::fs::create_dir(&dir)?;
    let current = dir.join("current");
    let previous = dir.join("previous");
    std::fs::write(
        &current,
        format!("{:x}\n", Sha256::digest(CURRENT.as_bytes())),
    )?;
    std::fs::write(
        &previous,
        format!("{:X}", Sha256::digest(PREVIOUS.as_bytes())),
    )?;
    let result = ServiceTokens::from_files(&current, Some(&previous));
    std::fs::remove_dir_all(dir)?;
    Ok(result?)
}
#[derive(Debug)]
struct TestAccounts;
impl AccountStateLookup for TestAccounts {
    fn state<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        account: Uuid,
    ) -> Future<'a, AccountState> {
        Box::pin(async move {
            let state: Option<String> =
                sqlx::query_scalar("SELECT state FROM fixture_accounts WHERE id = $1 FOR UPDATE")
                    .bind(account)
                    .fetch_optional(connection)
                    .await
                    .map_err(|_| Error::Unavailable)?;
            Ok(match state.as_deref() {
                Some("active") => AccountState::Active,
                Some("suspended") => AccountState::Suspended,
                _ => AccountState::Unknown,
            })
        })
    }
}
#[derive(Debug)]
struct TestAuthority;
impl AdmissionAuthority for TestAuthority {
    fn redeem<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
    ) -> Future<'a, AuthorizedAccount> {
        Box::pin(async move {
            let hash = Sha256::digest(request.ticket().as_bytes());
            let row = sqlx::query("UPDATE fixture_tickets SET used = true WHERE hash = $1 AND NOT used RETURNING account_id")
                .bind(&hash[..]).fetch_optional(connection).await.map_err(|_| Error::Unavailable)?.ok_or(Error::TicketRejected)?;
            Ok(AuthorizedAccount {
                id: row.try_get("account_id").map_err(|_| Error::Unavailable)?,
                security_generation: 1,
            })
        })
    }
    fn validate_replay<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a AdmissionRequest,
        _: &'a [u8],
    ) -> Future<'a, ()> {
        Box::pin(async { Ok(()) })
    }
    fn issue<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        _: AuthorizedAccount,
    ) -> Future<'a, Material> {
        Box::pin(async move {
            if request.character_id.to_string() != CHARACTER {
                return Err(Error::CharacterConflict);
            }
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
            )
            .fetch_one(connection)
            .await
            .map_err(|_| Error::Unavailable)?;
            Ok(Material {
                world_id: Uuid::parse_str(WORLD).map_err(|_| Error::Unavailable)?,
                channel_id: Uuid::parse_str(CHANNEL).map_err(|_| Error::Unavailable)?,
                endpoint: Endpoint {
                    host: "game.example.invalid".into(),
                    port: 7172,
                    tls_server_name: "game.example.invalid".into(),
                    alpn: "oteryn-game/1".into(),
                    protocol_major: 1,
                    transport_profile: 1,
                },
                token: "synthetic.header.signature".into(),
                expires_at_ms: now + 20_000,
            })
        })
    }
}
struct Database {
    tokens: ServiceTokens,
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP native-admission database test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("admission_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        // The schema identifier is generated solely from a UUID, never from input.
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |connection, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(sql)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105190100_t19_native_admissions.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql("CREATE TABLE fixture_accounts (id uuid PRIMARY KEY, state text NOT NULL); CREATE TABLE fixture_tickets (hash bytea PRIMARY KEY, account_id uuid NOT NULL, used boolean NOT NULL DEFAULT false)").execute(&pool).await?;
        sqlx::query("INSERT INTO fixture_accounts VALUES ($1, 'active')")
            .bind(Uuid::parse_str(ACCOUNT)?)
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO fixture_tickets (hash, account_id) VALUES ($1,$2)")
            .bind(&Sha256::digest(TICKET.as_bytes())[..])
            .bind(Uuid::parse_str(ACCOUNT)?)
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            tokens: tokens()?,
            pool,
            admin,
            schema,
        }))
    }
    fn router(&self) -> axum::Router {
        router(
            self.pool.clone(),
            AdmissionConfig {
                enabled: true,
                tokens: Some(self.tokens.clone()),
                ..Default::default()
            },
            Arc::new(TestAccounts),
            Arc::new(TestAuthority),
        )
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
    async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM native_admissions")
            .fetch_one(&self.pool)
            .await
    }
}
async fn send(
    app: axum::Router,
    credential: &str,
    raw: Vec<u8>,
) -> Result<(StatusCode, Vec<u8>), Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::post("/internal/v1/game-auth/native-admissions")
                .header("Authorization", format!("Bearer {credential}"))
                .header("Content-Type", "application/json")
                .body(Body::from(raw))?,
        )
        .await?;
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    let status = response.status();
    Ok((status, to_bytes(response.into_body(), 8192).await?.to_vec()))
}

#[test]
fn wire_accepts_gateway_client_bytes_and_rejects_bad_input() -> TestResult {
    let gateway =
        oteryn_gateway::wire::parse_request(&body()).map_err(|_| "gateway request rejected")?;
    let raw =
        oteryn_gateway::wire::encode_request(&gateway).map_err(|_| "gateway encoding failed")?;
    let decoded = AdmissionRequest::decode(&raw)?;
    assert_eq!(decoded.attempt_ref.to_string(), ATTEMPT);
    let debug = format!("{decoded:?}");
    assert!(!debug.contains(TICKET));
    for raw in [
        b"[]".to_vec(),
        b"{".to_vec(),
        body().replace_bytes(b"\"channel_id\":null,", b""),
        body().replace_bytes(ATTEMPT.as_bytes(), b"bad-id"),
        body().replace_bytes(
            b"\"protocol_version\":2",
            b"\"protocol_version\":2,\"protocol_version\":2",
        ),
        vec![b' '; 2049],
    ] {
        assert!(AdmissionRequest::decode(&raw).is_err());
    }
    assert_eq!(
        AdmissionRequest::decode(
            &body().replace_bytes(b"\"protocol_version\":2", b"\"protocol_version\":3")
        )
        .err(),
        Some(Error::UnsupportedVersion)
    );
    assert_eq!(
        AdmissionRequest::decode(&body().replace_bytes(b"oteryn-game/1", b"other/1")).err(),
        Some(Error::OfferUnsupported)
    );
    // Member order and insignificant whitespace are accepted by the Laravel issuer.
    let value: serde_json::Value = serde_json::from_slice(&body())?;
    assert!(AdmissionRequest::decode(&serde_json::to_vec_pretty(&value)?).is_ok());
    Ok(())
}
trait ReplaceBytes {
    fn replace_bytes(&self, from: &[u8], to: &[u8]) -> Vec<u8>;
}
impl ReplaceBytes for Vec<u8> {
    fn replace_bytes(&self, from: &[u8], to: &[u8]) -> Vec<u8> {
        String::from_utf8_lossy(self)
            .replace(
                &*String::from_utf8_lossy(from),
                &String::from_utf8_lossy(to),
            )
            .into_bytes()
    }
}
#[test]
fn shared_golden_harness_reads_native_fixtures() -> TestResult {
    let fixtures = oteryn_internal_api::fixtures::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/oteryn-native-v1"
    ))?;
    assert!(!fixtures.is_empty());
    Ok(())
}
#[tokio::test]
async fn wrong_service_token_has_exact_laravel_body_without_database() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    let app = router(
        pool,
        AdmissionConfig {
            enabled: true,
            tokens: Some(tokens()?),
            ..Default::default()
        },
        Arc::new(Unavailable),
        Arc::new(Unavailable),
    );
    let (status, body) = send(app, "synthetic-wrong-token", body()).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, br#"{"error":"unauthorized_service"}"#);
    Ok(())
}
#[tokio::test]
async fn default_off_and_missing_configuration_fail_closed() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    let app = router(
        pool.clone(),
        AdmissionConfig::default(),
        Arc::new(Unavailable),
        Arc::new(Unavailable),
    );
    assert_eq!(
        send(app, CURRENT, body()).await?,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"error":"service_unavailable"}"#.to_vec()
        )
    );
    let app = router(
        pool,
        AdmissionConfig {
            tokens: Some(tokens()?),
            ..Default::default()
        },
        Arc::new(Unavailable),
        Arc::new(Unavailable),
    );
    let (status, raw) = send(app, CURRENT, body()).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(raw, Error::Unavailable.body(Some(ATTEMPT)));
    Ok(())
}
#[test]
fn invalid_hash_files_and_redacted_debug() -> TestResult {
    let path = std::env::temp_dir().join(format!("admission-hash-{}", Uuid::now_v7()));
    std::fs::write(&path, "invalid")?;
    assert!(ServiceTokens::from_files(&path, None).is_err());
    std::fs::remove_file(path)?;
    assert_eq!(format!("{:?}", tokens()?), "ServiceTokens{redacted}");
    Ok(())
}
#[tokio::test]
async fn db_valid_admission_matches_gateway_success_parser() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (status, raw) = send(db.router(), CURRENT, body()).await?;
    assert_eq!(status, StatusCode::OK);
    let request = oteryn_gateway::wire::parse_request(&body()).map_err(|_| "invalid fixture")?;
    assert!(oteryn_gateway::wire::parse_success(&raw, &request).is_some());
    assert_eq!(db.count().await?, 1);
    let stored: Vec<u8> = sqlx::query_scalar("SELECT ticket_hash FROM native_admissions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(stored, &Sha256::digest(TICKET.as_bytes())[..]);
    db.close().await
}
#[tokio::test]
async fn db_previous_service_token_is_accepted() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(send(db.router(), PREVIOUS, body()).await?.0, StatusCode::OK);
    db.close().await
}
#[tokio::test]
async fn db_suspended_unknown_and_ambiguous_accounts_are_refused_and_rolled_back() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for state in ["suspended", "unknown", "ambiguous"] {
        sqlx::query("UPDATE fixture_accounts SET state = $1")
            .bind(state)
            .execute(&db.pool)
            .await?;
        let (status, raw) = send(db.router(), CURRENT, body()).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(raw, format!(r#"{{"protocol_version":2,"error":{{"code":"NATIVE_LOGIN_AUTHENTICATION_REQUIRED","public_class":"AUTHENTICATION_REQUIRED"}},"attempt_ref":"{ATTEMPT}"}}"#).as_bytes());
        assert_eq!(db.count().await?, 0);
        assert!(
            !sqlx::query_scalar::<_, bool>("SELECT used FROM fixture_tickets")
                .fetch_one(&db.pool)
                .await?
        );
    }
    sqlx::query("DELETE FROM fixture_accounts")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        send(db.router(), CURRENT, body()).await?.0,
        StatusCode::UNAUTHORIZED
    );
    db.close().await
}
#[tokio::test]
async fn db_same_attempt_is_idempotent_and_changed_attempt_conflicts() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = send(db.router(), CURRENT, body()).await?;
    let second = send(db.router(), PREVIOUS, body()).await?;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(second.0, StatusCode::OK);
    let a: serde_json::Value = serde_json::from_slice(&first.1)?;
    let b: serde_json::Value = serde_json::from_slice(&second.1)?;
    assert_eq!(a["grant"]["token"], b["grant"]["token"]);
    assert!(a["grant"]["valid_for_seconds"].as_i64() >= b["grant"]["valid_for_seconds"].as_i64());
    let changed = body().replace_bytes(TICKET.as_bytes(), b"another-synthetic-ticket");
    assert_eq!(
        send(db.router(), CURRENT, changed).await?,
        (
            StatusCode::CONFLICT,
            Error::AttemptConflict.body(Some(ATTEMPT))
        )
    );
    assert_eq!(db.count().await?, 1);
    db.close().await
}
#[tokio::test]
async fn db_concurrent_same_attempt_creates_one_row() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (a, b) = tokio::join!(
        send(db.router(), CURRENT, body()),
        send(db.router(), CURRENT, body())
    );
    assert_eq!(a?.0, StatusCode::OK);
    assert_eq!(b?.0, StatusCode::OK);
    assert_eq!(db.count().await?, 1);
    db.close().await
}
#[tokio::test]
async fn db_consumed_ticket_cannot_authorize_another_attempt() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(send(db.router(), CURRENT, body()).await?.0, StatusCode::OK);
    let changed = body().replace_bytes(ATTEMPT.as_bytes(), Uuid::now_v7().to_string().as_bytes());
    assert_eq!(
        send(db.router(), CURRENT, changed).await?.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(db.count().await?, 1);
    db.close().await
}
#[tokio::test]
async fn db_malformed_body_rejected_with_contract_error() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for raw in [
        b"{".to_vec(),
        body().replace_bytes(b"\"channel_id\":null,", b""),
        vec![b'x'; 2049],
    ] {
        let attempt = super::wire::attempt(&raw);
        assert_eq!(
            send(db.router(), CURRENT, raw).await?,
            (
                StatusCode::BAD_REQUEST,
                Error::Malformed.body(attempt.as_deref())
            )
        );
    }
    assert_eq!(db.count().await?, 0);
    db.close().await
}
#[tokio::test]
async fn db_character_refusal_rolls_back_ticket_and_expired_replay_refuses() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let changed = body().replace_bytes(CHARACTER.as_bytes(), Uuid::now_v7().to_string().as_bytes());
    assert_eq!(
        send(db.router(), CURRENT, changed).await?.0,
        StatusCode::CONFLICT
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT used FROM fixture_tickets")
            .fetch_one(&db.pool)
            .await?
    );
    assert_eq!(send(db.router(), CURRENT, body()).await?.0, StatusCode::OK);
    sqlx::query("UPDATE native_admissions SET expires_at_ms = 0")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        send(db.router(), CURRENT, body()).await?,
        (
            StatusCode::CONFLICT,
            Error::GrantExpired.body(Some(ATTEMPT))
        )
    );
    db.close().await
}
#[tokio::test]
async fn db_service_and_account_budgets_refuse() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for _ in 0..600 {
        super::store::budget(&db.pool, &Sha256::digest(CURRENT.as_bytes()), 600).await?;
    }
    assert_eq!(
        send(db.router(), CURRENT, body()).await?.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("DELETE FROM native_admission_budgets")
        .execute(&db.pool)
        .await?;
    for i in 0..10_u8 {
        let mut hash = [0; 32];
        hash[0] = i;
        sqlx::query("INSERT INTO native_admissions (attempt_ref,ticket_hash,request_digest,account_id,response_body,expires_at_ms) VALUES ($1,$2,$2,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(&hash[..]).bind(Uuid::parse_str(ACCOUNT)?).bind(b"{}".as_slice()).execute(&db.pool).await?;
    }
    assert_eq!(
        send(db.router(), CURRENT, body()).await?.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT used FROM fixture_tickets")
            .fetch_one(&db.pool)
            .await?
    );
    db.close().await
}

#[tokio::test]
async fn db_missing_replay_authority_requires_reconciliation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(send(db.router(), CURRENT, body()).await?.0, StatusCode::OK);
    let app = router(
        db.pool.clone(),
        AdmissionConfig {
            enabled: true,
            tokens: Some(db.tokens.clone()),
            ..Default::default()
        },
        Arc::new(TestAccounts),
        Arc::new(Unavailable),
    );
    assert_eq!(
        send(app, CURRENT, body()).await?,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Error::ReconciliationRequired.body(Some(ATTEMPT))
        )
    );
    db.close().await
}

#[tokio::test]
async fn db_gateway_client_calls_real_admission_listener() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = db.router();
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let client = oteryn_gateway::issuer::IssuerClient::new(
        &format!("http://{address}"),
        CURRENT.into(),
        std::time::Duration::from_secs(5),
        std::time::Duration::from_secs(1),
    )
    .map_err(|_| "issuer configuration failed")?;
    let request = oteryn_gateway::wire::parse_request(&body()).map_err(|_| "fixture rejected")?;
    let success = client
        .admit(&request)
        .await
        .map_err(|_| "gateway admission failed")?;
    assert_eq!(success.attempt_ref, ATTEMPT);
    assert_eq!(success.world_id, WORLD);
    assert_eq!(success.channel_id, CHANNEL);
    assert_eq!(db.count().await?, 1);
    server.abort();
    db.close().await
}
