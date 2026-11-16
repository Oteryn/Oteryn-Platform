use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use time::{Duration, format_description::well_known::Rfc3339};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn pg_signed_completion_before_expiry_reconciles_after_local_expiry() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("late_result_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
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
    let store = Store::new(pool.clone(), Config { enabled: true });
    store
        .register_agent(&AgentHello {
            agent_id: "agent-a".into(),
            build: "test-build".into(),
            catalog_digest: vec![1; 32],
            advertised_operations: vec![AdvertisedOperation {
                operation: "world.save_now".into(),
                operation_version: 1,
                kind: Kind::Command,
            }],
            served_worlds: vec!["world-a".into()],
            served_nodes: vec![],
            owner_epoch: 1,
        })
        .await?;
    let now = OffsetDateTime::now_utc();
    let request = NewRequest {
        request_id: Uuid::now_v7(),
        admin_operation_id: Uuid::now_v7(),
        operation: "world.save_now".into(),
        operation_version: 1,
        kind: Kind::Command,
        target: Target {
            world_id: Some("world-a".into()),
            node_id: None,
            character_id: None,
            account_ref: None,
        },
        params: json!({}),
        params_digest: vec![2; 32],
        dry_run: false,
        risk_class: RiskClass::R2,
        actor: Uuid::now_v7(),
        reason: json!({"code":"TEST","text":"Test save"}),
        approvals: vec![],
        issued_at: now - Duration::seconds(1),
        expires_at: now + Duration::minutes(1),
        nonce: vec![3; 16],
        signer_key_id: "test-key".into(),
        canonical_envelope: b"signed test request".to_vec(),
        signature: vec![4; 64],
    };
    store.enqueue(&request).await?;
    assert_eq!(store.claim_pending("agent-a", 1, 1).await?.len(), 1);
    // Move the execution window into the past after delivery. Sign authentic
    // completion inside that window, then deliver it after local expiry.
    sqlx::query("UPDATE admin_control_requests SET issued_at=clock_timestamp()-interval '2 minutes',expires_at=clock_timestamp()-interval '1 minute' WHERE request_id=$1")
        .bind(request.request_id).execute(&pool).await?;
    let stored = store
        .get(request.request_id)
        .await?
        .ok_or("request missing")?;
    let completed = stored.expires_at - Duration::seconds(1);
    let key = SigningKey::from_bytes(&[9; 32]);
    let mut envelope: super::super::signer::ResultEnvelope = serde_json::from_value(json!({
        "request_id":request.request_id,"admin_operation_id":request.admin_operation_id,
        "status":"succeeded","error":null,"result":{},"dry_run_preview":null,
        "game_ledger_id":"ledger-original","audit_event_ids":[],"effective_revision":null,
        "progress":null,"completed_at":completed.format(&Rfc3339)?,"agent_id":"agent-a",
        "agent_build":"test-build","signature":""
    }))?;
    envelope.signature = URL_SAFE_NO_PAD.encode(
        key.sign(&super::super::signer::adm_x1_signing_bytes(&envelope)?)
            .to_bytes(),
    );
    super::super::signer::verify_result(
        &envelope,
        &key.verifying_key(),
        super::super::signer::Validity {
            not_before: now.unix_timestamp() - 300,
            not_after: now.unix_timestamp() + 300,
        },
        now.unix_timestamp(),
    )?;
    assert_eq!(store.expire_due().await?, 1);
    assert!(store.claim_pending("agent-a", 1, 1).await?.is_empty());
    let mut evidence = NewResult {
        request_id: request.request_id,
        agent_id: "agent-a".into(),
        status: Status::Succeeded,
        game_ledger_id: Some("ledger-original".into()),
        error_code: None,
        payload: serde_json::to_value(&envelope)?,
    };
    // Completion after the execution window cannot override expiry.
    evidence.payload["completed_at"] = json!(now.format(&Rfc3339)?);
    assert!(matches!(
        store.record_result(&evidence, 1).await,
        Err(Error::IllegalTransition)
    ));
    let original = serde_json::to_value(&envelope)?;
    for (field, invalid) in [
        ("completed_at", Value::Null),
        (
            "completed_at",
            json!((stored.issued_at - Duration::seconds(1)).format(&Rfc3339)?),
        ),
        ("request_id", json!(Uuid::now_v7())),
        ("admin_operation_id", json!(Uuid::now_v7())),
        ("agent_id", json!("another-agent")),
        ("signature", json!("")),
        ("game_ledger_id", json!("another-ledger")),
        ("status", json!("failed")),
    ] {
        evidence.payload = original.clone();
        evidence.payload[field] = invalid;
        assert!(
            matches!(
                store.record_result(&evidence, 1).await,
                Err(Error::IllegalTransition)
            ),
            "accepted mismatched {field}"
        );
    }
    evidence.payload = original;
    let row = store.record_result(&evidence, 1).await?;
    assert_eq!(row.payload, evidence.payload);
    assert_eq!(
        store
            .get(request.request_id)
            .await?
            .ok_or("request missing")?
            .status,
        Status::Succeeded
    );
    assert!(store.claim_pending("agent-a", 1, 1).await?.is_empty());
    assert!(matches!(
        store.record_result(&evidence, 1).await,
        Err(Error::IllegalTransition)
    ));
    // A terminal expiry reported by the agent is not a local ambiguity.
    let mut another = request.clone();
    another.request_id = Uuid::now_v7();
    another.admin_operation_id = Uuid::now_v7();
    store.enqueue(&another).await?;
    assert_eq!(store.claim_pending("agent-a", 1, 1).await?.len(), 1);
    let expiry = NewResult {
        request_id: another.request_id,
        status: Status::Expired,
        payload: json!({"status":"expired"}),
        ..evidence.clone()
    };
    store.record_result(&expiry, 1).await?;
    envelope.request_id = another.request_id;
    envelope.admin_operation_id = another.admin_operation_id;
    envelope.completed_at = Some(now.format(&Rfc3339)?);
    envelope.signature = URL_SAFE_NO_PAD.encode(
        key.sign(&super::super::signer::adm_x1_signing_bytes(&envelope)?)
            .to_bytes(),
    );
    evidence.request_id = another.request_id;
    evidence.payload = serde_json::to_value(&envelope)?;
    assert!(matches!(
        store.record_result(&evidence, 1).await,
        Err(Error::IllegalTransition)
    ));
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!(
        "PG pg_signed_completion_before_expiry_reconciles_after_local_expiry duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
