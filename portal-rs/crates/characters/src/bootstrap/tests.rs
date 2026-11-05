use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use oteryn_internal_api::{
    canonical,
    guard::{Binding, Identity, IdentityMap, VerifiedClient},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{future::Future, pin::Pin};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const OP: &str = "01890f4e-7c00-7000-8000-000000000001";
// SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
// crates/gameplay/durability/src/character_bootstrap_intent/tests.rs,
// read_request_is_the_exact_bounded_producer_request (exact request vector).
const REQUEST: &str =
    r#"{"contract_version":3,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#;
// SOURCE: Platform CharacterBootstrapIntentIssuer.php v3 field order and producer
// contract; synthetic identifiers and times, no private production data.
const GOLDEN: &str = r#"{"contract_version":3,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"01890f4e-7c00-7000-8000-000000000002","source_revision":"42","operation_id":"01890f4e-7c00-7000-8000-000000000001","operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":"01890f4e-7c00-7000-8000-000000000003","target_world_id":"01890f4e-7c00-7000-8000-000000000004","requested_name":"Aldric Vale","requested_sex":"Male","interpretation_context":{"profile_revision":"profile:1","ruleset_revision":"ruleset:1","content_revision":"content:1","starter_template_revision":"starter:1"},"issued_at_source":"1790000000","expires_at_source":"1790000060","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}"#;

#[derive(Debug)]
struct Accounts(AccountState);
impl AccountStateLookup for Accounts {
    fn state(&self, _: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async { self.0 })
    }
}
fn reader(pool: PgPool, state: AccountState) -> Reader {
    Reader::new(pool, Settings { enabled: true }, Arc::new(Accounts(state)))
}
fn app(reader: Reader) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(router(
        reader,
        Arc::new(IdentityMap::new(vec![Binding {
            identity: Identity::san("bootstrap-peer")?,
            group: GROUP.into(),
        }])?),
    ))
}
async fn call(
    app: Router,
    raw: &str,
    peer: Option<&str>,
    status: StatusCode,
    expected: &[u8],
) -> TestResult {
    let mut request = Request::post(PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(raw.to_owned()))?;
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                peer,
            )?));
    }
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), status);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    assert_eq!(
        to_bytes(response.into_body(), 4096).await?.as_ref(),
        expected
    );
    Ok(())
}
#[test]
fn request_vectors_and_bad_input() -> TestResult {
    for raw in [
        REQUEST.to_owned(),
        format!(r#"{{ "operation_id":"{OP}", "contract_version":3 }}"#),
    ] {
        assert_eq!(
            wire::request(&canonical::decode(raw.as_bytes(), 256, 2)?)?,
            OP
        );
    }
    for raw in [
        r#"{}"#,
        r#"{"contract_version":2,"operation_id":"x"}"#,
        r#"{"contract_version":3,"operation_id":null}"#,
        r#"{"contract_version":3,"operation_id":{}}"#,
        r#"{"contract_version":3,"contract_version":3}"#,
    ] {
        assert!(
            canonical::decode(raw.as_bytes(), 256, 2)
                .map(|v| wire::request(&v).is_err())
                .unwrap_or(true)
        );
    }
    assert!(wire::uuid(&OP.to_uppercase()).is_err());
    for number in ["01", "-1", "+1", "9223372036854775808"] {
        assert!(wire::decimal(number, 0).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn identity_bad_request_and_default_off() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?;
    let active = reader(pool.clone(), AccountState::Active);
    for peer in [None, Some("wrong-peer")] {
        call(
            app(active.clone())?,
            REQUEST,
            peer,
            StatusCode::UNAUTHORIZED,
            b"",
        )
        .await?;
    }
    for raw in [
        "{}",
        "{",
        &" ".repeat(257),
        r#"{"contract_version":3,"operation_id":"x","extra":1}"#,
    ] {
        call(
            app(active.clone())?,
            raw,
            Some("bootstrap-peer"),
            StatusCode::BAD_REQUEST,
            b"",
        )
        .await?;
    }
    let off = Reader::new(
        pool,
        Settings::default(),
        Arc::new(Accounts(AccountState::Active)),
    );
    call(
        app(off)?,
        REQUEST,
        Some("bootstrap-peer"),
        StatusCode::SERVICE_UNAVAILABLE,
        b"",
    )
    .await
}
struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED bootstrap PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("bootstrap_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(2)
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
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105220103_t22b_bootstrap_intents.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            admin,
            pool,
            schema,
        }))
    }
    async fn seed(&self, raw: &str) -> TestResult {
        sqlx::query("DELETE FROM character_bootstrap_intents")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM character_bootstrap_intent_authority")
            .execute(&self.pool)
            .await?;
        sqlx::query("INSERT INTO character_bootstrap_intent_authority VALUES (1,'42','01890f4e-7c00-7000-8000-000000000002')").execute(&self.pool).await?;
        sqlx::query("INSERT INTO character_bootstrap_intents (operation_id,issuer_decision_id,source_revision,account_id,target_world_id,requested_name,requested_sex,profile_revision,ruleset_revision,content_revision,starter_template_revision,issued_at_source,expires_at_source,intent_json,intent_sha256) VALUES ($1,'01890f4e-7c00-7000-8000-000000000002','42','01890f4e-7c00-7000-8000-000000000003','01890f4e-7c00-7000-8000-000000000004','Aldric Vale','Male','profile:1','ruleset:1','content:1','starter:1','1790000000','1790000060',$2,$3)").bind(OP).bind(raw).bind(format!("{:x}", Sha256::digest(raw))).execute(&self.pool).await?;
        Ok(())
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
async fn db_golden_both_sexes_and_http_success() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    for sex in ["Male", "Female"] {
        let golden = GOLDEN.replace("Male", sex);
        db.seed(&golden).await?;
        sqlx::query("UPDATE character_bootstrap_intents SET requested_sex = $1")
            .bind(sex)
            .execute(&db.pool)
            .await?;
        let active = reader(db.pool.clone(), AccountState::Active);
        assert_eq!(active.current(OP, 1790000001).await?, golden.as_bytes());
        // Move the fixture clock to wall time for the real HTTP path.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let live = golden
            .replace("1790000000", &now.to_string())
            .replace("1790000060", &(now + 60).to_string());
        sqlx::query("UPDATE character_bootstrap_intents SET issued_at_source=$1, expires_at_source=$2, intent_json=$3, intent_sha256=$4").bind(now.to_string()).bind((now+60).to_string()).bind(&live).bind(format!("{:x}", Sha256::digest(&live))).execute(&db.pool).await?;
        call(
            app(active)?,
            REQUEST,
            Some("bootstrap-peer"),
            StatusCode::OK,
            live.as_bytes(),
        )
        .await?;
    }
    db.close().await
}
#[tokio::test]
async fn db_unknown_accounts_expiry_and_missing_operation() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    db.seed(GOLDEN).await?;
    for state in [
        AccountState::Denied,
        AccountState::Unknown,
        AccountState::Ambiguous,
        AccountState::Unavailable,
    ] {
        assert_eq!(
            reader(db.pool.clone(), state).current(OP, 1790000001).await,
            Err(Error::Unavailable)
        );
    }
    let active = reader(db.pool.clone(), AccountState::Active);
    assert_eq!(active.current(OP, 1790000060).await, Err(Error::NotFound));
    assert_eq!(
        active.current(OP, 1789999999).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        active
            .current("01890f4e-7c00-7000-8000-000000000099", 1790000001)
            .await,
        Err(Error::NotFound)
    );
    call(
        app(active.clone())?,
        REQUEST,
        Some("bootstrap-peer"),
        StatusCode::NOT_FOUND,
        b"",
    )
    .await?;
    db.close().await
}
#[tokio::test]
async fn db_corruption_rollback_and_retired_versions_deny() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let active = reader(db.pool.clone(), AccountState::Active);
    for sql in [
        "DELETE FROM character_bootstrap_intent_authority",
        "UPDATE character_bootstrap_intent_authority SET last_source_revision='41'",
        "UPDATE character_bootstrap_intent_authority SET last_issuer_decision_id='conflict'",
        "UPDATE character_bootstrap_intents SET requested_sex=NULL",
        "UPDATE character_bootstrap_intents SET requested_sex='Female'",
        "UPDATE character_bootstrap_intents SET intent_sha256=repeat('0',64)",
        "UPDATE character_bootstrap_intents SET profile_revision='changed'",
    ] {
        db.seed(GOLDEN).await?;
        sqlx::raw_sql(sql).execute(&db.pool).await?;
        assert_eq!(
            active.current(OP, 1790000001).await,
            Err(Error::Unavailable),
            "{sql}"
        );
    }
    call(
        app(active.clone())?,
        REQUEST,
        Some("bootstrap-peer"),
        StatusCode::SERVICE_UNAVAILABLE,
        b"",
    )
    .await?;
    for invalid in [
        GOLDEN.replace("Aldric Vale", "Aldric  Vale"),
        GOLDEN.replace("\"requested_sex\":\"Male\"", "\"requested_sex\":0"),
        GOLDEN.replace("\"source_revision\":\"42\"", "\"source_revision\":\"042\""),
        GOLDEN.replace("profile:1", "profile/1"),
        GOLDEN.replace("1790000060", "1790000301"),
        GOLDEN.replace(
            "\"contract_version\":3",
            "\"contract_version\":3,\"extra\":true",
        ),
        GOLDEN.replace(
            "\"contract_version\":3",
            "\"contract_version\":3,\"contract_version\":3",
        ),
    ] {
        db.seed(&invalid).await?;
        assert_eq!(
            active.current(OP, 1790000001).await,
            Err(Error::Unavailable)
        );
    }
    for version in [1, 2] {
        db.seed(&GOLDEN.replace(
            "\"contract_version\":3",
            &format!("\"contract_version\":{version}"),
        ))
        .await?;
        assert_eq!(
            active.current(OP, 1790000001).await,
            Err(Error::Unavailable)
        );
    }
    db.seed(GOLDEN).await?;
    sqlx::query("UPDATE character_bootstrap_intent_authority SET last_source_revision='43', last_issuer_decision_id='01890f4e-7c00-7000-8000-000000000005'").execute(&db.pool).await?;
    assert_eq!(active.current(OP, 1790000001).await?, GOLDEN.as_bytes());
    db.close().await
}
