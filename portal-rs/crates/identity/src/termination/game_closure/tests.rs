#![allow(clippy::unwrap_used, clippy::panic)]
use super::super::tests::Database;
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
const FIXTURE: &[u8] = include_bytes!("../fixtures/closure.json");
fn vectors() -> Vec<(String, Vec<u8>, String)> {
    let value: Value = serde_json::from_slice(FIXTURE).unwrap();
    value["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["name"].as_str().unwrap().into(),
                v["wire"].as_str().unwrap().as_bytes().to_vec(),
                v["sha256"].as_str().unwrap().into(),
            )
        })
        .collect()
}
fn seed() -> SigningKey {
    SigningKey::from_bytes(&[0x42; 32])
}
fn settings() -> Settings {
    Settings {
        enabled: true,
        v2_qualified: true,
        reversible_quarantine_accepted: true,
        disposition_and_retention_accepted: true,
        conflict_operator_configured: true,
    }
}
#[derive(Debug)]
struct Recovery(AtomicU64);
impl RecoveryFence for Recovery {
    fn current<'a>(&'a self, _: &'a mut Transaction<'_, Postgres>) -> RecoveryFuture<'a> {
        Box::pin(async { Ok(self.0.load(Ordering::SeqCst)) })
    }
}
#[derive(Debug)]
struct Local;
impl Boundary for Local {
    fn authorize<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        proof: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            if proof.0 == account.to_string() {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        })
    }
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM account_characters WHERE account_id=$1)",
            )
            .bind(account)
            .fetch_one(&mut **tx)
            .await?;
            if blocked {
                Err(Error::UnknownState)
            } else {
                Ok(())
            }
        })
    }
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        _: bool,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                .bind(account)
                .execute(&mut **tx)
                .await?;
            Ok(())
        })
    }
}
fn boundary(settings: Settings, recovery: Arc<Recovery>) -> Arc<GameBoundary> {
    Arc::new(
        GameBoundary::new(
            settings,
            Arc::new(Local),
            recovery,
            seed().verifying_key(),
            "oteryn:character-authority:primary".into(),
        )
        .unwrap(),
    )
}
fn service(db: &Database, boundary: Arc<GameBoundary>) -> Terminations {
    Terminations::new(
        db.pool.clone(),
        Config {
            enabled: true,
            ..Config::default()
        },
        boundary,
    )
    .unwrap()
}
async fn request(s: &Terminations, account: Uuid) -> Termination {
    s.request(
        account,
        &ActorProof(account.to_string()),
        "termination test password",
        "TERMINATE",
    )
    .await
    .unwrap()
}
#[test]
fn every_pinned_fixture_hash_wire_signature_and_payload() {
    assert_eq!(
        wire::hex(&digest(FIXTURE)),
        "22ea70a219875c97d988cd5bf1a74303eb772ef2027dd89b7e22a345033c2f19"
    );
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    for (name, raw, hash) in vectors() {
        assert_eq!(wire::hex(&digest(&raw)), hash);
        let sig = seed().sign(&raw).to_bytes();
        b.verify(&raw, &sig, 4096).unwrap();
        let wrong = SigningKey::from_bytes(&[0x43; 32]).sign(&raw).to_bytes();
        assert!(b.verify(&raw, &wrong, 4096).is_err());
        let mut changed = raw.clone();
        changed[20] ^= 1;
        assert!(b.verify(&changed, &sig, 4096).is_err());
        assert!(b.verify(&raw, &[], 4096).is_err());
        match name.as_str() {
            "close" | "cancel_resolution" => {
                assert_eq!(Intent::parse(&raw).unwrap().bytes().unwrap(), raw);
            }
            "closed_receipt" => {
                let receipt = Receipt::parse(&raw).unwrap();
                let intent = vectors().remove(0).1;
                receipt.bind(&Intent::parse(&intent).unwrap(), 3).unwrap();
                assert!(receipt.bind(&Intent::parse(&intent).unwrap(), 4).is_err());
                let payload = AccountClosureDecisionV1 {
                    intent_sha256: digest(&intent),
                    receipt: raw.clone(),
                    receipt_sha256: digest(&raw),
                    recovery_generation: 3,
                };
                let encoded = payload.encode_to_vec();
                let decoded = AccountClosureDecisionV1::decode(encoded.as_slice()).unwrap();
                decoded.validate(&intent).unwrap();
                let mut bad = decoded;
                bad.receipt_sha256[0] ^= 1;
                assert!(bad.validate(&intent).is_err());
            }
            "closure_projection" => {
                Projection::parse(&raw).unwrap();
            }
            _ => panic!("uncovered fixture {name}"),
        }
    }
}
#[test]
fn strict_wire_refuses_ambiguity_and_bad_proofs() {
    for (name, raw, _) in vectors() {
        let text = String::from_utf8(raw).unwrap();
        for changed in [
            format!("{text}\n"),
            text.replace(
                "\"contract_version\":2",
                "\"contract_version\":2,\"contract_version\":2",
            ),
            text.replace(
                "\"contract_version\":2",
                "\"contract_version\":2,\"unknown\":0",
            ),
            text.replace("\"contract_version\":2", "\"contract_version\":1"),
            text.replace("\"contract_version\":2", "\"contract_version\":2.0"),
            text.replace("0192a000", "0192A000"),
            text.replace("\"3\"", "\"03\""),
            text.replace("\"operation_id\":\"", "\"operation_id\":\"\\u0030"),
        ] {
            let valid = match name.as_str() {
                "close" | "cancel_resolution" => Intent::parse(changed.as_bytes()).is_ok(),
                "closed_receipt" => Receipt::parse(changed.as_bytes()).is_ok(),
                _ => Projection::parse(changed.as_bytes()).is_ok(),
            };
            assert!(!valid, "{name}: {changed}");
        }
    }
    let raw = vectors().remove(1).1;
    let text = String::from_utf8(raw).unwrap().replace(
        "\"detached_characters\":\"0\"",
        "\"detached_characters\":\"1\"",
    );
    assert!(Receipt::parse(text.as_bytes()).is_err());
    let mut payload = AccountClosureDecisionV1 {
        intent_sha256: vec![],
        receipt: vec![],
        receipt_sha256: vec![],
        recovery_generation: 0,
    };
    assert!(payload.validate(b"{}").is_err());
    payload.recovery_generation = 3;
    assert!(payload.validate(b"{}").is_err());
}
#[test]
fn owner_gates_default_off_and_watermark_staleness() {
    assert!(Settings::default().ready().is_err());
    for i in 0..5 {
        let mut s = settings();
        match i {
            0 => s.enabled = false,
            1 => s.v2_qualified = false,
            2 => s.reversible_quarantine_accepted = false,
            3 => s.disposition_and_retention_accepted = false,
            _ => s.conflict_operator_configured = false,
        }
        assert!(s.ready().is_err());
    }
    let wm = Watermark {
        contract_version: 2,
        operation: "PublishPublicProjectionWatermarkV1".into(),
        source_authority: "oteryn:character-authority:primary".into(),
        family: "account_closure".into(),
        projection_epoch: "3".into(),
        complete_through: "100".into(),
        observed_at: "101".into(),
    };
    assert!(wm.fresh(129).unwrap());
    assert!(!wm.fresh(130).unwrap());
    assert!(!wm.fresh(98).unwrap());
    Watermark::parse(&serde_json::to_vec(&wm).unwrap()).unwrap();
}
async fn issue(db: &Database, s: &Terminations, account: Uuid) -> (Termination, Intent) {
    let t = request(s, account).await;
    db.due(t.id).await.unwrap();
    assert_eq!(s.run_worker(100).await.unwrap().blocked, 1);
    let raw: Vec<u8> =
        sqlx::query_scalar("SELECT intent FROM plat3_closure_commands WHERE termination_id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    (t, Intent::parse(&raw).unwrap())
}
async fn publish(
    db: &Database,
    b: &GameBoundary,
    intent: &Intent,
    lcfa: bool,
) -> (Vec<u8>, Projection) {
    let mut receipt = Receipt::parse(&vectors().remove(1).1).unwrap();
    receipt.operation_id = intent.operation_id.clone();
    receipt.decided_at = intent.issued_at_source.clone();
    receipt.recovery_generation = intent.expected_recovery_generation.clone();
    let result = receipt.result_data.as_mut().unwrap();
    result.account_id = intent.account_id.clone();
    result.termination_id = intent.termination_id.clone();
    result.lifecycle_generation = intent.lifecycle_generation.clone();
    let bytes = serde_json::to_vec(result).unwrap();
    let marker = b",\"proof_sha256\":";
    let pos = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    let mut proof = bytes[..pos].to_vec();
    proof.push(b'}');
    result.proof_sha256 = wire::hex(&digest(&proof));
    let proof_hash = result.proof_sha256.clone();
    let raw = serde_json::to_vec(&receipt).unwrap();
    b.ingest_receipt(&db.pool, &raw, &seed().sign(&raw).to_bytes())
        .await
        .unwrap();
    b.ingest_receipt(&db.pool, &raw, &seed().sign(&raw).to_bytes())
        .await
        .unwrap();
    let mut projection = Projection::parse(&vectors().remove(2).1).unwrap();
    projection.account_id = intent.account_id.clone();
    projection.source_observed_at = intent.issued_at_source.clone();
    let record = &mut projection.closures[0];
    record.termination_id = intent.termination_id.clone();
    record.operation_id = intent.operation_id.clone();
    record.lifecycle_generation = intent.lifecycle_generation.clone();
    record.receipt_digest = wire::hex(&digest(&raw));
    record.proof_sha256 = proof_hash;
    let bytes = serde_json::to_vec(&projection).unwrap();
    b.ingest_projection(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
        .await
        .unwrap();
    let now = sqlx::query_scalar::<_, i64>("SELECT extract(epoch FROM clock_timestamp())::bigint")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let wm = Watermark {
        contract_version: 2,
        operation: "PublishPublicProjectionWatermarkV1".into(),
        source_authority: b.authority.clone(),
        family: "account_closure".into(),
        projection_epoch: "3".into(),
        complete_through: now.to_string(),
        observed_at: now.to_string(),
    };
    let bytes = serde_json::to_vec(&wm).unwrap();
    b.ingest_watermark(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
        .await
        .unwrap();
    if lcfa {
        sqlx::query("UPDATE account_character_feed SET highest_epoch='3',watermark_epoch='3',source_authority=$1,complete_through=$2,observed_at=$2 WHERE id=1").bind(&b.authority).bind(now).execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'3','12',$3,$4)").bind(wire::id(&intent.account_id).unwrap()).bind(&b.authority).bind(now).bind(digest(b"empty test portfolio")).execute(&db.pool).await.unwrap();
    }
    (raw, projection)
}
#[tokio::test]
async fn postgres_close_proof_requery_finalize_and_atomic_audit() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let recovery = Arc::new(Recovery(AtomicU64::new(3)));
    let b = boundary(settings(), recovery);
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (t, intent) = issue(&db, &s, account).await;
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    publish(&db, &b, &intent, true).await;
    assert_eq!(s.run_worker(100).await?.finalized, 1);
    assert_eq!(s.run_worker(100).await?, WorkerResult::default());
    let finalized: bool =
        sqlx::query_scalar("SELECT finalized_at IS NOT NULL FROM account_terminations WHERE id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await?;
    assert!(finalized);
    let decision: String =
        sqlx::query_scalar("SELECT resolution FROM plat3_closure_work WHERE termination_id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(decision, "FINALIZE");
    let payload: Vec<u8> = sqlx::query_scalar("SELECT payload FROM plat3_closure_audit_outbox")
        .fetch_one(&db.pool)
        .await?;
    let raw: Vec<u8> = sqlx::query_scalar(
        "SELECT intent FROM plat3_closure_commands WHERE command='CloseAccount'",
    )
    .fetch_one(&db.pool)
    .await?;
    AccountClosureDecisionV1::decode(payload.as_slice())?.validate(&raw)?;
    assert!(
        sqlx::query("UPDATE plat3_closure_commands SET intent=$1")
            .bind(b"{}".as_slice())
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await
}
#[tokio::test]
async fn postgres_missing_lcfa_stale_recovery_withdrawal_and_conflicting_receipt_stay_pending()
-> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let recovery = Arc::new(Recovery(AtomicU64::new(3)));
    let b = boundary(settings(), recovery.clone());
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (t, intent) = issue(&db, &s, account).await;
    let (raw, mut projection) = publish(&db, &b, &intent, false).await;
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    let changed =
        String::from_utf8(raw.clone())?.replace("\"decided_at\":\"", "\"decided_at\":\"1");
    assert!(
        b.ingest_receipt(
            &db.pool,
            changed.as_bytes(),
            &seed().sign(changed.as_bytes()).to_bytes()
        )
        .await
        .is_err()
    );
    recovery.0.store(4, Ordering::SeqCst);
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    recovery.0.store(3, Ordering::SeqCst);
    projection.projection_revision = "2".into();
    projection.closures.clear();
    let bytes = serde_json::to_vec(&projection)?;
    b.ingest_projection(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
        .await?;
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    let finalized: bool =
        sqlx::query_scalar("SELECT finalized_at IS NOT NULL FROM account_terminations WHERE id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await?;
    assert!(!finalized);
    db.close().await
}
#[tokio::test]
async fn postgres_cancel_before_delivery_and_after_proof_never_issues_finalize() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b.clone());
    for proof in [false, true] {
        let account = db.account().await?;
        let (t, intent) = issue(&db, &s, account).await;
        if proof {
            publish(&db, &b, &intent, true).await;
        }
        s.cancel(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
        )
        .await?;
        assert_eq!(s.run_worker(100).await?, WorkerResult::default());
        let raw:Vec<u8>=sqlx::query_scalar("SELECT intent FROM plat3_closure_commands WHERE termination_id=$1 AND command='ResolveAccountClosure'").bind(t.id).fetch_one(&db.pool).await?;
        let resolution = Intent::parse(&raw)?;
        assert_eq!(resolution.decision.as_deref(), Some("CANCEL"));
        assert_eq!(
            wire::number(&resolution.lifecycle_generation, true)?,
            wire::number(&intent.lifecycle_generation, true)? + 1
        );
        assert_eq!(resolution.close_operation_id, Some(intent.operation_id));
        let next = request(&s, account).await;
        let generation: i64 =
            sqlx::query_scalar("SELECT generation FROM plat3_closure_work WHERE termination_id=$1")
                .bind(next.id)
                .fetch_one(&db.pool)
                .await?;
        assert!(generation > wire::number(&resolution.lifecycle_generation, true)? as i64);
        s.cancel(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
        )
        .await?;
    }
    db.close().await
}
#[tokio::test]
async fn postgres_cancel_worker_race_has_one_resolution_and_generation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (t, intent) = issue(&db, &s, account).await;
    publish(&db, &b, &intent, true).await;
    let proof = ActorProof(account.to_string());
    let (cancel, worker) = tokio::join!(
        s.cancel(account, &proof, "termination test password"),
        s.run_worker(100)
    );
    let worker = worker?;
    assert!(cancel.is_ok() || worker.finalized == 1);
    let (cancelled,finalized):(bool,bool)=sqlx::query_as("SELECT cancelled_at IS NOT NULL,finalized_at IS NOT NULL FROM account_terminations WHERE id=$1").bind(t.id).fetch_one(&db.pool).await?;
    assert_ne!(cancelled, finalized);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM plat3_closure_commands WHERE termination_id=$1 AND command='ResolveAccountClosure'").bind(t.id).fetch_one(&db.pool).await?;
    assert_eq!(count, 1);
    db.close().await
}
#[tokio::test]
async fn postgres_projection_conflict_quarantines_and_higher_epoch_withdraws_proof() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (_, intent) = issue(&db, &s, account).await;
    let (_, mut projection) = publish(&db, &b, &intent, true).await;
    projection.closures.clear();
    let bytes = serde_json::to_vec(&projection)?;
    assert!(
        b.ingest_projection(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
            .await
            .is_err()
    );
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    projection.projection_epoch = "4".into();
    let bytes = serde_json::to_vec(&projection)?;
    assert!(
        b.ingest_projection(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
            .await
            .is_err()
    );
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    db.close().await
}

#[tokio::test]
async fn postgres_terminal_stale_generation_retries_fresh_bytes_and_rescan_keeps_unacknowledged_intents()
-> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let recovery = Arc::new(Recovery(AtomicU64::new(3)));
    let b = boundary(settings(), recovery.clone());
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (t, intent) = issue(&db, &s, account).await;
    let pending = b.pending_intents(&db.pool, 0, 40).await?;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        b.read_intent(&db.pool, wire::id(&intent.operation_id)?)
            .await?,
        Some(pending[0].clone())
    );
    let mut receipt = Receipt::parse(&vectors().remove(1).1)?;
    receipt.operation_id = intent.operation_id.clone();
    receipt.outcome = "REJECTED".into();
    receipt.result_code = "CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION".into();
    receipt.recovery_generation = "4".into();
    recovery.0.store(4, Ordering::SeqCst);
    receipt.result_data = None;
    receipt.decided_at = intent.issued_at_source.clone();
    let bytes = serde_json::to_vec(&receipt)?;
    b.ingest_receipt(&db.pool, &bytes, &seed().sign(&bytes).to_bytes())
        .await?;
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    let pending = b.pending_intents(&db.pool, 0, 40).await?;
    assert_eq!(pending.len(), 1);
    let fresh = Intent::parse(&pending[0])?;
    assert_ne!(fresh.operation_id, intent.operation_id);
    assert_eq!(fresh.lifecycle_generation, intent.lifecycle_generation);
    assert_eq!(fresh.scheduled_for, intent.scheduled_for);
    assert!(
        wire::number(&fresh.source_revision, true)? > wire::number(&intent.source_revision, true)?
    );
    s.cancel(
        account,
        &ActorProof(account.to_string()),
        "termination test password",
    )
    .await?;
    let pending = b.pending_intents(&db.pool, 0, 40).await?;
    assert_eq!(pending.len(), 2);
    let mut cancel = Intent::parse(pending.last().ok_or("missing CANCEL")?)?;
    assert_eq!(cancel.decision.as_deref(), Some("CANCEL"));
    assert_eq!(cancel.close_operation_id, Some(fresh.operation_id));
    let bytes = cancel.bytes()?;
    assert_eq!(
        b.read_intent(&db.pool, wire::id(&cancel.operation_id)?)
            .await?,
        Some(bytes.clone())
    );
    cancel.decision = Some("FINALIZE".into());
    assert_ne!(cancel.bytes()?, bytes);
    let resolution: String =
        sqlx::query_scalar("SELECT resolution FROM plat3_closure_work WHERE termination_id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(resolution, "CANCEL");
    db.close().await
}
#[tokio::test]
async fn postgres_flag_off_and_request_time_characters_are_still_blockers() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let b = boundary(Settings::default(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b.clone());
    assert!(
        s.request(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
            "TERMINATE"
        )
        .await
        .is_err()
    );
    assert!(b.pending_intents(&db.pool, 0, 1).await.is_err());
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Fixture','AVAILABLE')").bind(account).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&db.pool).await?;
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b);
    assert!(
        s.request(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
            "TERMINATE"
        )
        .await
        .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM plat3_closure_work")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}

#[tokio::test]
async fn postgres_conflicting_terminal_receipt_quarantines_retry_issuance() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let recovery = Arc::new(Recovery(AtomicU64::new(3)));
    let b = boundary(settings(), recovery.clone());
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (t, intent) = issue(&db, &s, account).await;
    let mut receipt = Receipt::parse(&vectors().remove(1).1)?;
    receipt.operation_id = intent.operation_id.clone();
    receipt.outcome = "REJECTED".into();
    receipt.result_code = "CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION".into();
    receipt.recovery_generation = "4".into();
    recovery.0.store(4, Ordering::SeqCst);
    receipt.result_data = None;
    receipt.decided_at = intent.issued_at_source.clone();
    let raw = serde_json::to_vec(&receipt)?;
    b.ingest_receipt(&db.pool, &raw, &seed().sign(&raw).to_bytes())
        .await?;
    receipt.result_code = "CHAR_CMD_ACCOUNT_CLOSE_OBLIGATION".into();
    let raw = serde_json::to_vec(&receipt)?;
    assert!(
        b.ingest_receipt(&db.pool, &raw, &seed().sign(&raw).to_bytes())
            .await
            .is_err()
    );
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM plat3_closure_commands WHERE termination_id=$1")
            .bind(t.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    db.close().await
}

#[tokio::test]
async fn postgres_higher_watermark_epoch_withdraws_previously_usable_proof() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let b = boundary(settings(), Arc::new(Recovery(AtomicU64::new(3))));
    let s = service(&db, b.clone());
    let account = db.account().await?;
    let (_, intent) = issue(&db, &s, account).await;
    publish(&db, &b, &intent, true).await;
    let now: i64 = sqlx::query_scalar("SELECT extract(epoch FROM clock_timestamp())::bigint")
        .fetch_one(&db.pool)
        .await?;
    let watermark = Watermark {
        contract_version: 2,
        operation: "PublishPublicProjectionWatermarkV1".into(),
        source_authority: b.authority.clone(),
        family: "account_closure".into(),
        projection_epoch: "4".into(),
        complete_through: now.to_string(),
        observed_at: now.to_string(),
    };
    let raw = serde_json::to_vec(&watermark)?;
    assert!(
        b.ingest_watermark(&db.pool, &raw, &seed().sign(&raw).to_bytes())
            .await
            .is_err()
    );
    let (epoch, withdrawn): (String, bool) =
        sqlx::query_as("SELECT highest_epoch,watermark IS NULL FROM plat3_closure_feed WHERE id=1")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(epoch, "4");
    assert!(withdrawn);
    assert_eq!(s.run_worker(100).await?.blocked, 1);
    db.close().await
}
