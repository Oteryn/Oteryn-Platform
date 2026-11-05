use super::*;
use crate::runtime::tests::{self as support, ACCOUNT, Database};
use axum::{
    body::to_bytes,
    http::{StatusCode, header},
};
use tower::ServiceExt;

use support::TestResult;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Result<Self, std::io::Error> {
        let path = std::env::temp_dir().join(format!(
            "portal-evidence-{}",
            sqlx::types::Uuid::now_v7().simple()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn settings(directory: &Directory) -> Settings {
    Settings {
        enabled: true,
        peer_identity: oteryn_internal_api::guard::Identity::san("evidence").ok(),
        activated: true,
        witness_directory: directory.0.clone(),
        source_authority: "platform".into(),
        fresh_account_purpose: "security".into(),
        fresh_account_scope: "admission".into(),
        fresh_key_purpose: "admission".into(),
        ..Settings::default()
    }
}
fn account(operation: &str) -> Value {
    let recovery = operation == RECOVERY_ACCOUNT;
    object([
        ("version", Value::Unsigned(if recovery { 2 } else { 1 })),
        ("operation", string(operation)),
        ("account_id", string(ACCOUNT)),
        (
            "purpose",
            string(if recovery {
                "platform_security"
            } else {
                "security"
            }),
        ),
        (
            "scope",
            string(if recovery {
                "existing_actor_recovery"
            } else {
                "admission"
            }),
        ),
    ])
}
fn trust(recovery: bool) -> Value {
    object([
        ("version", Value::Unsigned(if recovery { 2 } else { 1 })),
        (
            "operation",
            string(if recovery {
                RECOVERY_TRUST
            } else {
                FRESH_TRUST
            }),
        ),
        (
            "issuer",
            string(if recovery {
                "urn:oteryn:platform:game-recovery"
            } else {
                "urn:oteryn:platform:game-admission"
            }),
        ),
        (
            "profile",
            string(if recovery {
                "oteryn-reauth-recovery-v1"
            } else {
                "oteryn-pre-admission-v1"
            }),
        ),
        (
            "key_purpose",
            string(if recovery {
                "existing_actor_recovery"
            } else {
                "admission"
            }),
        ),
        ("key_id", string("key-1")),
    ])
}
async fn sources(db: &Database) -> TestResult {
    // Fixture-only schemas for the separate account and public-trust owners.
    sqlx::raw_sql("CREATE TABLE accounts (id uuid PRIMARY KEY, status text, native_security_generation bigint); CREATE TABLE native_game_signing_trust_profiles (id uuid PRIMARY KEY,issuer text,profile text,key_purpose text,profile_version bigint,issuer_revision bigint,revoked_at timestamptz); CREATE TABLE native_game_signing_trust_key_versions (profile_id uuid,key_id text,key_revision bigint,public_key text,trusted boolean);").execute(&db.pool).await?;
    sqlx::query("INSERT INTO accounts VALUES ($1,'active',1)")
        .bind(ACCOUNT.parse::<sqlx::types::Uuid>()?)
        .execute(&db.pool)
        .await?;
    for (suffix, recovery) in [(1, false), (2, true)] {
        let id = format!("019c0000-0000-7000-8000-{suffix:012}").parse::<sqlx::types::Uuid>()?;
        let value = trust(recovery);
        sqlx::query("INSERT INTO native_game_signing_trust_profiles VALUES ($1,$2,$3,$4,1,1,NULL)")
            .bind(id)
            .bind(text(&value, "issuer")?)
            .bind(text(&value, "profile")?)
            .bind(text(&value, "key_purpose")?)
            .execute(&db.pool)
            .await?;
        sqlx::query(
            "INSERT INTO native_game_signing_trust_key_versions VALUES ($1,'key-1',1,$2,true)",
        )
        .bind(id)
        .bind(URL_SAFE_NO_PAD.encode([7; 32]))
        .execute(&db.pool)
        .await?;
    }
    Ok(())
}
fn trust_floor(options: &Settings, recovery: bool, revision: i64) -> TestResult {
    let req = EvidenceRequest::decode(&trust(recovery))?;
    let witness = Witness::lock(&options.witness_directory, &req.state_namespace)?;
    witness.advance(&req.state_namespace, revision)?;
    Ok(())
}
#[test]
fn evidence_wire_all_operations_and_invalid_members() -> TestResult {
    for value in [
        account(FRESH_ACCOUNT),
        account(RECOVERY_ACCOUNT),
        trust(false),
        trust(true),
    ] {
        EvidenceRequest::decode(&value)?;
    }
    for (key, val) in [
        ("version", Value::Unsigned(2)),
        ("account_id", string("00000000-0000-0000-0000-000000000000")),
        ("purpose", string("")),
        ("scope", Value::Null),
        ("operation", string("Unknown")),
    ] {
        let mut value = account(FRESH_ACCOUNT);
        if let Value::Object(members) = &mut value {
            members.iter_mut().find(|(k, _)| k == key).ok_or("field")?.1 = val;
        }
        assert!(EvidenceRequest::decode(&value).is_err());
    }
    let mut extra = account(FRESH_ACCOUNT);
    if let Value::Object(members) = &mut extra {
        members.push(("unexpected".into(), Value::Bool(true)));
    }
    assert!(EvidenceRequest::decode(&extra).is_err());
    Ok(())
}
#[test]
fn witness_locks_bounds_monotonicity_and_durability() -> TestResult {
    let directory = Directory::new()?;
    let namespace = hash("account");
    let witness = Witness::lock(&directory.0, &namespace)?;
    assert!(Witness::lock(&directory.0, &namespace).is_err());
    assert_eq!(witness.floor(&namespace)?, None);
    witness.advance(&namespace, 2)?;
    assert_eq!(witness.floor(&namespace)?, Some(2));
    assert!(witness.advance(&namespace, 1).is_err());
    assert!(witness.floor("../escape").is_err());
    drop(witness);
    let witness = Witness::lock(&directory.0, &namespace)?;
    assert_eq!(witness.floor(&namespace)?, Some(2));
    fs::write(witness.path(&namespace), "999999999999999999999999999999")?;
    assert!(witness.floor(&namespace).is_err());
    Ok(())
}
#[tokio::test]
async fn evidence_guard_size_identity_and_throttle() -> TestResult {
    let directory = Directory::new()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let path = "/internal/v1/game-auth/native-evidence";
    let app: Router = router(pool.clone(), Settings::default(), support::identities()?);
    assert_eq!(
        app.oneshot(support::request(path, Some("evidence"), b"{}".to_vec())?)
            .await?
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut options = settings(&directory);
    options.requests_per_minute = 2;
    let app: Router = router(pool, options, support::identities()?);
    let response = app
        .clone()
        .oneshot(support::request(path, Some("node"), b"{}".to_vec())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    assert_eq!(
        app.clone()
            .oneshot(support::request(path, Some("evidence"), vec![b' '; 1025])?)
            .await?
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        app.clone()
            .oneshot(support::request(path, Some("evidence"), b"{}".to_vec())?)
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let response = app
        .oneshot(support::request(path, Some("evidence"), b"{}".to_vec())?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    Ok(())
}
#[tokio::test]
async fn db_evidence_fresh_recovery_share_revision_and_persist_exact_bytes() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let options = settings(&directory);
    for (i, operation) in [FRESH_ACCOUNT, RECOVERY_ACCOUNT, FRESH_ACCOUNT]
        .into_iter()
        .enumerate()
    {
        let req = EvidenceRequest::decode(&account(operation))?;
        let value = observe(&db.pool, &options, &req).await;
        assert_eq!(text(&value, "result")?, "observed");
        assert_eq!(text(&value, "source_revision")?, (i + 1).to_string());
        let wire:String=sqlx::query_scalar("SELECT response_json FROM native_evidence WHERE namespace_hash=$1 AND source_revision=$2").bind(&req.namespace).bind((i+1) as i64).fetch_one(&db.pool).await?;
        assert_eq!(wire.as_bytes(), canonical::encode(&value)?);
    }
    sqlx::query("UPDATE accounts SET status='suspended'")
        .execute(&db.pool)
        .await?;
    let value = observe(
        &db.pool,
        &options,
        &EvidenceRequest::decode(&account(FRESH_ACCOUNT))?,
    )
    .await;
    assert_eq!(value.get("allowed"), Some(&Value::Bool(false)));
    assert_eq!(text(&value, "source_revision")?, "4");
    db.close().await
}
#[tokio::test]
async fn db_evidence_trust_fresh_recovery_revocation_and_bad_public_key() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let options = settings(&directory);
    for recovery in [false, true] {
        let req = EvidenceRequest::decode(&trust(recovery))?;
        assert_eq!(
            text(&observe(&db.pool, &options, &req).await, "result")?,
            "unavailable"
        );
        trust_floor(&options, recovery, 1)?;
        let value = observe(&db.pool, &options, &req).await;
        assert_eq!(text(&value, "result")?, "observed");
        assert_eq!(value.get("trusted"), Some(&Value::Bool(true)));
    }
    sqlx::query(
        "UPDATE native_game_signing_trust_key_versions SET trusted=false WHERE profile_id='019c0000-0000-7000-8000-000000000001'",
    )
    .execute(&db.pool)
    .await?;
    let req = EvidenceRequest::decode(&trust(false))?;
    assert_eq!(
        observe(&db.pool, &options, &req).await.get("trusted"),
        Some(&Value::Bool(false))
    );
    sqlx::query(
        "UPDATE native_game_signing_trust_key_versions SET public_key='invalid' WHERE profile_id='019c0000-0000-7000-8000-000000000001'",
    )
    .execute(&db.pool)
    .await?;
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "result")?,
        "unavailable"
    );
    db.close().await
}
#[tokio::test]
async fn db_evidence_failures_are_bounded_and_do_not_create_observations() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let directory = Directory::new()?;
    let options = settings(&directory);
    let req = EvidenceRequest::decode(&account(FRESH_ACCOUNT))?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    sources(&db).await?;
    let mut unsupported = account(FRESH_ACCOUNT);
    if let Value::Object(m) = &mut unsupported {
        m.iter_mut().find(|(k, _)| k == "scope").ok_or("scope")?.1 = string("wrong");
    }
    let unsupported = EvidenceRequest::decode(&unsupported)?;
    assert_eq!(
        observe(&db.pool, &options, &unsupported).await,
        unsupported.failure("unsupported")
    );
    sqlx::query("DELETE FROM accounts")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("not_found")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM native_evidence")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[tokio::test]
async fn db_evidence_detects_database_restore_and_replaced_witness_store() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let mut options = settings(&directory);
    let req = EvidenceRequest::decode(&account(FRESH_ACCOUNT))?;
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "result")?,
        "observed"
    );
    sqlx::query("DELETE FROM native_evidence")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    let replacement = Directory::new()?;
    options.witness_directory = replacement.0.clone();
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    db.close().await
}
#[tokio::test]
async fn db_evidence_generation_mismatch_missing_floor_and_history_fail_closed() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let options = settings(&directory);
    let req = EvidenceRequest::decode(&account(FRESH_ACCOUNT))?;
    observe(&db.pool, &options, &req).await;
    sqlx::query("UPDATE accounts SET native_security_generation=2")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    let state = Witness::lock(&directory.0, &req.state_namespace)?;
    state.advance(&req.state_namespace, 2)?;
    drop(state);
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "result")?,
        "observed"
    );
    fs::remove_file(directory.0.join(format!("{}.floor", req.namespace)))?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    db.close().await
}
#[tokio::test]
async fn db_evidence_http_success_exact_failure_and_restart() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let options = settings(&directory);
    let path = "/internal/v1/game-auth/native-evidence";
    for revision in [1, 2] {
        let app: Router = router(db.pool.clone(), options.clone(), support::identities()?);
        let response = app
            .oneshot(support::request(
                path,
                Some("evidence"),
                canonical::encode(&account(FRESH_ACCOUNT))?,
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 8192).await?;
        let value = canonical::decode(&bytes, 8192, 2)?;
        assert_eq!(text(&value, "source_revision")?, revision.to_string());
    }
    db.close().await
}
#[test]
fn native_golden_vectors_use_shared_fixture_harness() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/oteryn-native-v1");
    let fixtures = oteryn_internal_api::fixtures::load(root)?;
    assert!(!fixtures.is_empty());
    // This directory currently contains gateway vectors only. Any future evidence
    // or runtime request vector is automatically checked by its domain decoder.
    for fixture in fixtures {
        match text(&fixture.value, "operation").ok() {
            Some(FRESH_ACCOUNT | RECOVERY_ACCOUNT | FRESH_TRUST | RECOVERY_TRUST) => {
                EvidenceRequest::decode(&fixture.value)?;
            }
            Some("ReportRuntimeStatusV1") => {
                crate::runtime::Report::decode(&fixture.value)?;
            }
            _ => {}
        }
        assert_eq!(canonical::encode(&fixture.value)?, fixture.canonical);
    }
    Ok(())
}

#[tokio::test]
async fn db_evidence_reconciles_committed_revision_and_rebinds_retained_store_after_restore()
-> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sources(&db).await?;
    let directory = Directory::new()?;
    let options = settings(&directory);
    let req = EvidenceRequest::decode(&account(FRESH_ACCOUNT))?;
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "source_revision")?,
        "1"
    );
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "source_revision")?,
        "2"
    );
    // Simulate a committed relational decision whose witness installation failed.
    fs::write(directory.0.join(format!("{}.floor", req.namespace)), "1\n")?;
    sqlx::query("DELETE FROM native_evidence_witness_stores")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        text(&observe(&db.pool, &options, &req).await, "source_revision")?,
        "3"
    );
    let witness = Witness::lock(&directory.0, &req.namespace)?;
    assert_eq!(witness.floor(&req.namespace)?, Some(3));
    drop(witness);
    // An active source cannot lose its account and become an ordinary not_found.
    sqlx::query("DELETE FROM accounts")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        observe(&db.pool, &options, &req).await,
        req.failure("unavailable")
    );
    db.close().await
}
#[tokio::test]
async fn evidence_capacity_refusal_uses_exact_unavailable_wire_without_queueing() -> TestResult {
    let directory = Directory::new()?;
    let state = Evidence {
        pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?,
        settings: settings(&directory),
        throttle: Arc::default(),
        capacity: Arc::new(Semaphore::new(2)),
        peer_guard: Router::new(),
        group_guard: Router::new(),
    };
    let _first = state.capacity.clone().try_acquire_owned()?;
    let _second = state.capacity.clone().try_acquire_owned()?;
    let response = handle(Extension(state), Extension(account(FRESH_ACCOUNT))).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 8192).await?.as_ref(),
        b"{\"version\":1,\"operation\":\"ReadAccountSecurityV1\",\"result\":\"unavailable\"}"
    );
    Ok(())
}
