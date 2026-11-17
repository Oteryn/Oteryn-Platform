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
// SOURCE: producer contract v4 (agreed with the Game consumer lane 2026-11-17):
// the exact bounded read request, version 4.
const REQUEST: &str =
    r#"{"contract_version":4,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#;
// The retired version 3 request, otherwise identical.
const REQUEST_V3: &str =
    r#"{"contract_version":3,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#;
// SOURCE: producer contract v4 field order (`requested_vocation` after
// `requested_sex`); synthetic identifiers and times, no private production data.
const GOLDEN: &str = r#"{"contract_version":4,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"01890f4e-7c00-7000-8000-000000000002","source_revision":"42","operation_id":"01890f4e-7c00-7000-8000-000000000001","operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":"01890f4e-7c00-7000-8000-000000000003","target_world_id":"01890f4e-7c00-7000-8000-000000000004","requested_name":"Aldric Vale","requested_sex":"Male","requested_vocation":"Knight","interpretation_context":{"profile_revision":"profile:1","ruleset_revision":"ruleset:1","content_revision":"content:1","starter_template_revision":"starter:1"},"issued_at_source":"1790000000","expires_at_source":"1790000060","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}"#;

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
        format!(r#"{{ "operation_id":"{OP}", "contract_version":4 }}"#),
    ] {
        assert_eq!(
            wire::request(&canonical::decode(raw.as_bytes(), 256, 2)?)?,
            OP
        );
    }
    for raw in [
        r#"{}"#,
        r#"{"contract_version":2,"operation_id":"x"}"#,
        r#"{"contract_version":4,"operation_id":null}"#,
        r#"{"contract_version":4,"operation_id":{}}"#,
        r#"{"contract_version":4,"contract_version":4}"#,
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
#[test]
fn v3_read_request_is_rejected() -> TestResult {
    for version in [1, 2, 3, 5] {
        let raw = REQUEST.replace(
            "\"contract_version\":4",
            &format!("\"contract_version\":{version}"),
        );
        let value = canonical::decode(raw.as_bytes(), 256, 2)?;
        assert_eq!(wire::request(&value), Err(Error::BadInput), "{raw}");
    }
    Ok(())
}
#[tokio::test]
async fn identity_bad_request_and_default_off() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
        r#"{"contract_version":4,"operation_id":"x","extra":1}"#,
        REQUEST_V3,
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
        for migration in [
            include_str!("../../../../migrations/20261105220103_t22b_bootstrap_intents.sql"),
            include_str!("../../../../migrations/20261117160500_bootstrap_intent_v4_vocation.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
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
        sqlx::query("INSERT INTO character_bootstrap_intents (operation_id,issuer_decision_id,source_revision,account_id,target_world_id,requested_name,requested_sex,requested_vocation,profile_revision,ruleset_revision,content_revision,starter_template_revision,issued_at_source,expires_at_source,intent_json,intent_sha256) VALUES ($1,'01890f4e-7c00-7000-8000-000000000002','42','01890f4e-7c00-7000-8000-000000000003','01890f4e-7c00-7000-8000-000000000004','Aldric Vale','Male','Knight','profile:1','ruleset:1','content:1','starter:1','1790000000','1790000060',$2,$3)").bind(OP).bind(raw).bind(format!("{:x}", Sha256::digest(raw))).execute(&self.pool).await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
async fn db_every_vocation_is_served_from_the_wire() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let active = reader(db.pool.clone(), AccountState::Active);
    for vocation in ["Knight", "Paladin", "Sorcerer", "Druid", "Monk"] {
        let golden = GOLDEN.replace(
            "\"requested_vocation\":\"Knight\"",
            &format!("\"requested_vocation\":\"{vocation}\""),
        );
        db.seed(&golden).await?;
        sqlx::query("UPDATE character_bootstrap_intents SET requested_vocation = $1")
            .bind(vocation)
            .execute(&db.pool)
            .await?;
        let served = active.current(OP, 1790000001).await?;
        assert_eq!(served, golden.as_bytes());
        assert_eq!(
            canonical::decode(&served, 4096, 3)?.get("requested_vocation"),
            Some(&canonical::Value::String(vocation.into()))
        );
    }
    db.close().await
}
#[tokio::test]
async fn db_stored_v3_intent_read_fails_closed() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let active = reader(db.pool.clone(), AccountState::Active);
    // A retained v3 intent: version 3, no vocation member, no vocation column.
    let v3 = GOLDEN
        .replace("\"contract_version\":4", "\"contract_version\":3")
        .replace(",\"requested_vocation\":\"Knight\"", "");
    for column in [None, Some("Knight")] {
        db.seed(&v3).await?;
        sqlx::query("UPDATE character_bootstrap_intents SET requested_vocation = $1")
            .bind(column)
            .execute(&db.pool)
            .await?;
        assert_eq!(
            active.current(OP, 1790000001).await,
            Err(Error::Unavailable),
            "{column:?}"
        );
    }
    // A v3 body that carries a vocation member is not current either.
    db.seed(&GOLDEN.replace("\"contract_version\":4", "\"contract_version\":3"))
        .await?;
    assert_eq!(
        active.current(OP, 1790000001).await,
        Err(Error::Unavailable)
    );
    db.close().await
}
#[tokio::test]
async fn db_unknown_accounts_expiry_and_missing_operation() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
        "UPDATE character_bootstrap_intents SET requested_vocation=NULL",
        "UPDATE character_bootstrap_intents SET requested_vocation='Paladin'",
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
        GOLDEN.replace("\"Knight\"", "\"knight\""),
        GOLDEN.replace("\"Knight\"", "\"None\""),
        GOLDEN.replace("\"Knight\"", "\"\""),
        GOLDEN.replace("\"Knight\"", "0"),
        GOLDEN.replace(",\"requested_vocation\":\"Knight\"", ""),
        GOLDEN.replace("\"source_revision\":\"42\"", "\"source_revision\":\"042\""),
        GOLDEN.replace("profile:1", "profile/1"),
        GOLDEN.replace("1790000060", "1790000301"),
        GOLDEN.replace(
            "\"contract_version\":4",
            "\"contract_version\":4,\"extra\":true",
        ),
        GOLDEN.replace(
            "\"contract_version\":4",
            "\"contract_version\":4,\"contract_version\":4",
        ),
    ] {
        db.seed(&invalid).await?;
        assert_eq!(
            active.current(OP, 1790000001).await,
            Err(Error::Unavailable)
        );
    }
    for version in [1, 2, 3] {
        db.seed(&GOLDEN.replace(
            "\"contract_version\":4",
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

/// `docs/contracts/fixtures/character-bootstrap-intent-v4`: every file is listed with
/// its exact length and SHA-256; each valid intent is the issuer's own serializer
/// output for the manifest's fixed values and passes the reader's validator; each
/// rejected intent differs from the Knight vector only in its vocation member and
/// fails that validator. A fixture or serializer change is caught here.
#[test]
fn v4_fixtures_match_the_serializer_and_validator() -> TestResult {
    use std::collections::BTreeSet;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/character-bootstrap-intent-v4");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    let text =
        |value: &serde_json::Value, key: &str| -> Result<String, Box<dyn std::error::Error>> {
            Ok(value[key]
                .as_str()
                .ok_or(format!("{key} is not a string"))?
                .to_owned())
        };
    let mut listed = BTreeSet::new();
    for entry in manifest["files"].as_array().ok_or("files")? {
        let name = text(entry, "file")?;
        let bytes = std::fs::read(root.join(&name))?;
        assert_eq!(Some(bytes.len() as u64), entry["bytes"].as_u64(), "{name}");
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            text(entry, "sha256")?,
            "{name}"
        );
        assert!(!bytes.ends_with(b"\n"), "{name} has a trailing newline");
        listed.insert(name);
    }
    let mut on_disk = BTreeSet::new();
    for entry in std::fs::read_dir(&root)? {
        let name = entry?.file_name().into_string().map_err(|_| "file name")?;
        if name != "manifest.json" && name != "README.md" {
            on_disk.insert(name);
        }
    }
    assert_eq!(listed, on_disk, "every vector is listed once");

    let fixed = &manifest["fixed"];
    let context = issue::Context {
        profile_revision: text(fixed, "profile_revision")?,
        ruleset_revision: text(fixed, "ruleset_revision")?,
        content_revision: text(fixed, "content_revision")?,
        starter_template_revision: text(fixed, "starter_template_revision")?,
    };
    assert_eq!(context.starter_template_revision, "newhaven-starter-r1");
    let knight = std::fs::read(root.join("intent.knight.json"))?;
    let mut accepted = Vec::new();
    for case in manifest["cases"].as_array().ok_or("cases")? {
        let name = text(case, "file")?;
        assert!(listed.contains(&name), "{name}");
        let bytes = std::fs::read(root.join(&name))?;
        match text(case, "outcome")?.as_str() {
            "accept" => {
                let vocation = text(case, "requested_vocation")?;
                let expected = issue::Intent {
                    issuer_decision_id: &text(case, "issuer_decision_id")?,
                    source_revision: &text(case, "source_revision")?,
                    operation_id: &text(case, "operation_id")?,
                    account_id: &text(fixed, "account_id")?,
                    target_world_id: &text(fixed, "target_world_id")?,
                    requested_name: &text(fixed, "requested_name")?,
                    requested_sex: &text(case, "requested_sex")?,
                    requested_vocation: &vocation,
                    context: &context,
                    issued_at_source: &text(fixed, "issued_at_source")?,
                    expires_at_source: &text(fixed, "expires_at_source")?,
                }
                .encode()?;
                assert_eq!(bytes, expected, "{name} is the serializer's output");
                let value = wire::intent(&bytes)?;
                assert_eq!(
                    value.get("requested_vocation"),
                    Some(&canonical::Value::String(vocation.clone())),
                    "{name}"
                );
                accepted.push(vocation);
            }
            "reject" => {
                let member = "\"requested_vocation\":\"Knight\"";
                let derived = match case["requested_vocation"].as_str() {
                    Some(value) => String::from_utf8(knight.clone())?
                        .replace(member, &format!("\"requested_vocation\":\"{value}\"")),
                    None => String::from_utf8(knight.clone())?.replace(&format!(",{member}"), ""),
                };
                assert_eq!(bytes, derived.as_bytes(), "{name} differs only in vocation");
                assert_eq!(wire::intent(&bytes), Err(Error::Unavailable), "{name}");
            }
            other => return Err(format!("{name}: unknown outcome {other}").into()),
        }
    }
    assert_eq!(accepted, wire::VOCATIONS, "one valid intent per vocation");
    Ok(())
}

#[tokio::test]
async fn r7_read_below_retained_revision() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    db.seed(GOLDEN).await?;
    let older = GOLDEN
        .replace(OP, "01890f4e-7c00-7000-8000-000000000005")
        .replace("000000000002", "000000000006")
        .replace("\"42\"", "\"41\"");
    let op = "01890f4e-7c00-7000-8000-000000000005";
    sqlx::query("INSERT INTO character_bootstrap_intents(operation_id,issuer_decision_id,source_revision,account_id,target_world_id,requested_name,requested_sex,requested_vocation,profile_revision,ruleset_revision,content_revision,starter_template_revision,issued_at_source,expires_at_source,intent_json,intent_sha256) SELECT $1,'01890f4e-7c00-7000-8000-000000000006','41',account_id,target_world_id,requested_name,requested_sex,requested_vocation,profile_revision,ruleset_revision,content_revision,starter_template_revision,issued_at_source,expires_at_source,$2,$3 FROM character_bootstrap_intents").bind(op).bind(&older).bind(format!("{:x}",Sha256::digest(&older))).execute(&db.pool).await?;
    sqlx::query("UPDATE character_bootstrap_intent_authority SET last_source_revision='41', last_issuer_decision_id='01890f4e-7c00-7000-8000-000000000006'").execute(&db.pool).await?;
    assert!(
        matches!(
            reader(db.pool.clone(), AccountState::Active)
                .current(op, 1790000001)
                .await,
            Err(Error::Unavailable)
        ),
        "rolled-back authority served revision 41"
    );
    db.close().await
}
