#![allow(clippy::unwrap_used, clippy::panic)]
use super::authority_wire::{decode, number};
use super::game_wire::{digest, id, text};
use super::*;
use crate::{Evidence, Listing};
use oteryn_internal_api::canonical::{self, Value};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
const FIXTURES: &str = include_str!("../../tests/fixtures/plat3/decisions.json");
fn vectors() -> serde_json::Value {
    serde_json::from_str(FIXTURES).unwrap()
}
fn vector(name: &str) -> String {
    vectors()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()["wire"]
        .as_str()
        .unwrap()
        .into()
}
pub(super) fn uid(s: &str) -> Uuid {
    Uuid::parse_str(&format!("0192a000-0000-7000-8000-{s:0>12}")).unwrap()
}
fn key() -> ed25519_dalek::VerifyingKey {
    let hex = vectors()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|v| v["fixture_public_key_hex"].as_str())
        .unwrap()
        .to_string();
    let mut bytes = [0; 32];
    for (n, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[n * 2..n * 2 + 2], 16).unwrap();
    }
    ed25519_dalek::VerifyingKey::from_bytes(&bytes).unwrap()
}
fn authority() -> Authority {
    Authority {
        config: AuthorityConfig {
            enabled: true,
            contract_sha256: Some(PINNED_CONTRACT_SHA256),
            v2_receipts_accepted: true,
            eligibility_and_escrow_accepted: true,
            inbox_and_item_classes_accepted: true,
            gold_fee_and_headroom_accepted: true,
            common_recovery_scope_qualified: true,
            signing_and_retention_accepted: true,
            receipt_authority: "oteryn:character-authority:primary".into(),
            projection_authority: "oteryn:game:primary".into(),
            snapshot_keys: vec![("fixture-example-1".into(), key())],
            worlds: vec![uid("12")],
            escrow_account_id: None,
            character_cooldown_seconds: None,
            character_history_complete_since: None,
        },
        fence: RecoveryFence {
            projection_epoch: 3,
            recovery_generation: 3,
        },
    }
}
#[test]
fn every_pinned_wire_hash_and_closed_schema() {
    assert_eq!(
        digest(FIXTURES.as_bytes()),
        "88772654feb01b1a16bee13d046777aa161dcc0705601e9be1edc2e20954cd56"
    );
    let vectors = vectors();
    let all = vectors["vectors"].as_array().unwrap();
    assert_eq!(all.len(), 10);
    for v in all {
        let raw = v["wire"].as_str().unwrap();
        assert_eq!(digest(raw.as_bytes()), v["sha256"].as_str().unwrap());
        let data = decode(raw.as_bytes(), 270336).unwrap();
        match v["name"].as_str().unwrap() {
            "escrow_intent" | "delivery_intent" | "changed_retry" => {
                PreparedIntent::parse(raw).unwrap();
            }
            "escrow_receipt" | "delivery_receipt" | "stale_fence_rejection" => {
                Receipt::parse(raw.as_bytes()).unwrap();
            }
            "delivered_status" => {
                assert_eq!(text(&data, "state").unwrap(), "DELIVERED");
            }
            "locked_character_snapshot" => {
                let binding = SnapshotBinding {
                    source_authority: "oteryn:game:primary",
                    listing: uid("30"),
                    seller: uid("11"),
                    character: uid("31"),
                    world: uid("12"),
                    lock: uid("32"),
                    lock_revision: 1,
                    character_revision: 7,
                    eligibility_revision: 4,
                    projection_epoch: 3,
                    now: 1790000003,
                };
                assert_eq!(
                    verify_snapshot(raw.as_bytes(), &binding, "fixture-example-1", &key()).unwrap(),
                    "26d9a91adcd5a5b430194f1c876ca7d6e5d7e308a3e6cf0eed5f9926123c38c2"
                );
                let bad = raw.replace("\"level\":\"100\"", "\"level\":\"101\"");
                assert!(
                    verify_snapshot(bad.as_bytes(), &binding, "fixture-example-1", &key()).is_err()
                );
                // Rehashing the tampered payload must still fail the signature.
                let bad_value = decode(bad.as_bytes(), 262144).unwrap();
                let changed =
                    digest(&canonical::encode(bad_value.get("snapshot").unwrap()).unwrap());
                let bad = bad.replace(text(&bad_value, "snapshot_hash").unwrap(), &changed);
                assert!(
                    verify_snapshot(bad.as_bytes(), &binding, "fixture-example-1", &key()).is_err()
                );
                let mut wrong = binding.clone();
                wrong.seller = uid("13");
                assert!(
                    verify_snapshot(raw.as_bytes(), &wrong, "fixture-example-1", &key()).is_err()
                );
                assert!(verify_snapshot(raw.as_bytes(), &binding, "revoked", &key()).is_err());
            }
            "character_settlement_intent" | "character_settlement_receipt" => {
                assert_eq!(data.get("contract_version"), Some(&Value::Unsigned(1)));
                assert_eq!(
                    text(&data, "command").unwrap(),
                    "TransferCharacterOwnership"
                );
                assert!(Receipt::parse(raw.as_bytes()).is_err());
            }
            _ => panic!("unqualified pinned fixture"),
        }
    }
}
#[test]
fn receipt_rejects_noncanonical_unknown_null_numeric_and_wrong_bindings() {
    let raw = vector("escrow_receipt");
    let receipt = Receipt::parse(raw.as_bytes()).unwrap();
    let intent = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
    receipt.bind(&intent, 1790000003).unwrap();
    for bad in [
        format!("{raw}\n"),
        raw.replace(
            "\"contract_version\":2",
            "\"contract_version\":2,\"extra\":0",
        ),
        raw.replace("\"escrow_revision\":\"1\"", "\"escrow_revision\":1"),
        raw.replace("\"escrow_revision\":\"1\"", "\"escrow_revision\":\"01\""),
        raw.replace("\"quantity\":1", "\"quantity\":null"),
        raw.replace("COMMITTED", "ALREADY_APPLIED"),
        raw.replace("CHAR_CMD_OK", "CHAR_CMD_UNKNOWN"),
        raw.replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"0\"",
        ),
        raw.replace("\"escrow_id\"", "\"escrow_id\":null,\"escrow_id\""),
        raw.replace("fixture-sword", "fixture\\u002dsword"),
    ] {
        assert!(Receipt::parse(bad.as_bytes()).is_err());
    }
    let bad = PreparedIntent::parse(&vector("changed_retry")).unwrap();
    assert!(receipt.bind(&bad, 1790000003).is_err());
    assert!(receipt.bind(&intent, 1790000000).is_err());
    assert!(
        Receipt::parse(
            vector("stale_fence_rejection")
                .replace("CHAR_CMD_BAZAAR_STALE_REVISION", "CHAR_CMD_UNKNOWN")
                .as_bytes()
        )
        .is_err()
    );
}
#[test]
fn decision_hashes_bind_exact_bytes_and_nonzero_generation() {
    let i = vector("escrow_intent");
    let r = vector("escrow_receipt");
    let p = BazaarAuthorityDecisionV1 {
        intent_sha256: Sha256::digest(i.as_bytes()).to_vec(),
        receipt_sha256: Sha256::digest(r.as_bytes()).to_vec(),
        recovery_generation: 3,
    };
    let bytes = p.encode_to_vec();
    let decoded = BazaarAuthorityDecisionV1::decode(bytes.as_slice()).unwrap();
    decoded.verify(i.as_bytes(), r.as_bytes(), 3).unwrap();
    assert!(
        decoded
            .verify(format!("{i}\n").as_bytes(), r.as_bytes(), 3)
            .is_err()
    );
    assert!(decoded.verify(i.as_bytes(), r.as_bytes(), 4).is_err());
    assert!(decoded.verify(i.as_bytes(), r.as_bytes(), 0).is_err());
}
#[test]
fn candidate_owner_gates_and_missing_trust_never_enable() {
    assert!(!AuthorityConfig::default().available());
    let mut a = authority();
    assert!(a.config.available());
    a.config.enabled = false;
    assert!(!a.config.available());
    a.config.enabled = true;
    a.config.v2_receipts_accepted = false;
    assert!(!a.config.available());
    a.config.v2_receipts_accepted = true;
    a.config.snapshot_keys.clear();
    assert!(!a.config.available());
}
#[tokio::test]
async fn postgres_exact_receipt_replay_conflict_audit_rollback_and_nonrollback_floor() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL PLAT3: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let a = authority();
    sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&mut *tx)
        .await
        .unwrap();
    let intent = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
    let op = id(text(&intent.value, "operation_id").unwrap()).unwrap();
    sqlx::query("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,18,$6,$7,3,3)").bind(op).bind(format!("fixture:{}",Uuid::now_v7())).bind(id(text(&intent.value,"listing_id").unwrap()).unwrap()).bind(uid("11")).bind(intent.command().wire()).bind(std::str::from_utf8(intent.bytes()).unwrap()).bind(intent.binding_digest()).execute(&mut *tx).await.unwrap();
    let raw = vector("escrow_receipt");
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            raw.as_bytes(),
            1790000003
        )
        .await
        .unwrap(),
        ReceiptResult::Stored
    );
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            raw.as_bytes(),
            u64::MAX
        )
        .await
        .unwrap(),
        ReceiptResult::Duplicate
    );
    assert!(
        a.receive(&mut tx, "wrong", raw.as_bytes(), 1790000003)
            .await
            .is_err()
    );
    let p = BazaarAuthorityDecisionV1 {
        intent_sha256: Sha256::digest(intent.bytes()).to_vec(),
        receipt_sha256: Sha256::digest(raw.as_bytes()).to_vec(),
        recovery_generation: 3,
    }
    .encode_to_vec();
    let identity = DecisionIdentity {
        event_id: Uuid::now_v7(),
        operation_id: op,
        transaction_id: Uuid::now_v7(),
        ordinal: 1,
        count: 1,
    };
    assert_eq!(
        a.decision(
            &mut tx,
            &a.config.projection_authority,
            &identity,
            &p,
            b"synthetic-test-envelope"
        )
        .await
        .unwrap(),
        ReceiptResult::Stored
    );
    assert_eq!(
        a.decision(
            &mut tx,
            &a.config.projection_authority,
            &identity,
            &p,
            b"synthetic-test-envelope"
        )
        .await
        .unwrap(),
        ReceiptResult::Duplicate
    );
    let collision = DecisionIdentity {
        operation_id: Uuid::now_v7(),
        ..identity.clone()
    };
    assert_eq!(
        a.decision(
            &mut tx,
            &a.config.projection_authority,
            &collision,
            &p,
            b"colliding-envelope"
        )
        .await
        .unwrap(),
        ReceiptResult::Quarantined
    );
    assert!(a.retained(&mut tx, op).await.unwrap().is_none());
    assert!(
        a.retained(&mut tx, collision.operation_id)
            .await
            .unwrap()
            .is_none()
    );
    let changed = raw.replace(
        "\"decided_at\":\"1790000002\"",
        "\"decided_at\":\"1790000003\"",
    );
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            changed.as_bytes(),
            1790000003
        )
        .await
        .unwrap(),
        ReceiptResult::Quarantined
    );
    assert!(a.retained(&mut tx, op).await.unwrap().is_none());
    let original: String =
        sqlx::query_scalar("SELECT receipt FROM bazaar_authority_receipts WHERE operation_id=$1")
            .bind(op)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(original, raw);
    Outbox::reopen(&mut tx, 4, 4).await.unwrap();
    assert!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            raw.as_bytes(),
            1790000003
        )
        .await
        .is_err()
    );
    assert!(Outbox::reopen(&mut tx, 3, 3).await.is_err());
    tx.rollback().await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_authority_receipts WHERE operation_id=$1")
            .bind(op)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    pool.close().await;
}
#[tokio::test]
async fn postgres_suspended_delivered_settlement_captures_once() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(false, true, ItemScenario::Wallet).await;
}
#[tokio::test]
async fn postgres_suspended_compensation_refunds_once() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(true, true, ItemScenario::Wallet).await;
}
#[test]
fn all_nine_exact_result_schemas_reject_unknown_and_reordered_fields() {
    let item = decode(vector("escrow_receipt").as_bytes(), 270336)
        .unwrap()
        .get("result_data")
        .unwrap()
        .get("item")
        .unwrap()
        .clone();
    let snapshot = decode(vector("locked_character_snapshot").as_bytes(), 262144).unwrap();
    let uid = uid("32").to_string();
    let hash = "a".repeat(64);
    let cases = [
        (
            Command::Lock,
            format!(
                "{{\"character_id\":\"{uid}\",\"lock_id\":\"{uid}\",\"lock_revision\":\"1\",\"eligibility_revision\":\"4\",\"signed_snapshot\":{}}}",
                String::from_utf8(canonical::encode(&snapshot).unwrap()).unwrap()
            ),
        ),
        (
            Command::Bind,
            format!(
                "{{\"character_id\":\"{uid}\",\"lock_id\":\"{uid}\",\"lock_revision\":\"2\",\"transfer_operation_id\":\"{uid}\",\"transfer_binding_digest\":\"{hash}\"}}"
            ),
        ),
        (
            Command::Unlock,
            format!(
                "{{\"character_id\":\"{uid}\",\"lock_id\":\"{uid}\",\"lock_revision\":\"2\",\"current_account_id\":\"{uid}\"}}"
            ),
        ),
        (
            Command::Escrow,
            format!(
                "{{\"escrow_id\":\"{uid}\",\"escrow_revision\":\"1\",\"seller_character_id\":\"{uid}\",\"item\":{}}}",
                String::from_utf8(canonical::encode(&item).unwrap()).unwrap()
            ),
        ),
        (
            Command::Deliver,
            format!(
                "{{\"escrow_id\":\"{uid}\",\"escrow_revision\":\"2\",\"destination_account_id\":\"{uid}\",\"destination_character_id\":\"{uid}\",\"delivery_id\":\"{uid}\",\"item\":{}}}",
                String::from_utf8(canonical::encode(&item).unwrap()).unwrap()
            ),
        ),
        (
            Command::Return,
            format!(
                "{{\"escrow_id\":\"{uid}\",\"escrow_revision\":\"2\",\"destination_account_id\":\"{uid}\",\"destination_character_id\":\"{uid}\",\"delivery_id\":\"{uid}\",\"item\":{}}}",
                String::from_utf8(canonical::encode(&item).unwrap()).unwrap()
            ),
        ),
        (
            Command::Hold,
            format!(
                "{{\"hold_id\":\"{uid}\",\"hold_revision\":\"1\",\"buyer_account_id\":\"{uid}\",\"offer_id\":\"{uid}\",\"hold_purpose\":\"OFFER\",\"held_amount\":\"10000\"}}"
            ),
        ),
        (
            Command::Release,
            format!(
                "{{\"hold_id\":\"{uid}\",\"hold_revision\":\"2\",\"released_amount\":\"10000\",\"remaining_held\":\"0\",\"hold_state\":\"RELEASED\"}}"
            ),
        ),
        (
            Command::Payout,
            format!(
                "{{\"hold_id\":\"{uid}\",\"hold_revision\":\"2\",\"sale_id\":\"{uid}\",\"gross_amount\":\"10000\",\"fee_amount\":\"250\",\"seller_amount\":\"9750\",\"remaining_held\":\"0\",\"buyer_ledger_entry_id\":\"{uid}\",\"seller_ledger_entry_id\":\"{uid}\",\"burn_entry_id\":\"{uid}\",\"delivery_operation_id\":\"{uid}\",\"delivery_receipt_digest\":\"{hash}\"}}"
            ),
        ),
    ];
    for (command, data) in cases {
        let raw = format!(
            "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"{}\",\"operation_id\":\"{uid}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"result_data\":{data},\"recovery_generation\":\"3\",\"decided_at\":\"1790000002\"}}",
            command.wire()
        );
        assert_eq!(Receipt::parse(raw.as_bytes()).unwrap().command, command);
        let mut value = decode(raw.as_bytes(), 270336).unwrap();
        if let Value::Object(m) = &mut value
            && let Some((_, Value::Object(d))) = m.iter_mut().find(|(k, _)| k == "result_data")
        {
            d.swap(0, 1);
        }
        assert!(
            Receipt::parse(&canonical::encode(&value).unwrap()).is_err(),
            "{} reordered",
            command.wire()
        );
        assert!(
            Receipt::parse(
                raw.replace("\"result_data\":{", "\"result_data\":{\"unknown\":null,")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(Receipt::parse(raw.replace(&data, "null").as_bytes()).is_err());
    }
}
#[tokio::test]
async fn private_routes_disabled_no_store_and_receipt_v1_preservation() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use oteryn_internal_api::guard::IdentityMap;
    use std::sync::Arc;
    use tower::ServiceExt;
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let mut a = authority();
    a.config.enabled = false;
    let routes = authority_http::router(a.clone(), pool.clone(), Arc::new(IdentityMap::default()));
    for path in [
        "commands/pending",
        "commands/read",
        "commands/reconcile",
        "transferability",
        "status",
        "watermark",
    ] {
        let response = routes
            .clone()
            .oneshot(
                Request::post(format!("/internal/v1/bazaar/{path}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            response.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
    }
    let legacy = Router::new().route(
        "/internal/v1/game-auth/character-commands/receipt",
        post(|| async { StatusCode::IM_A_TEAPOT }),
    );
    let routes =
        authority_http::extend_receipt_facility(legacy, a, pool, Arc::new(IdentityMap::default()));
    for (body, status) in [
        (
            vector("character_settlement_receipt"),
            StatusCode::IM_A_TEAPOT,
        ),
        (vector("escrow_receipt"), StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let response = routes
            .clone()
            .oneshot(
                Request::post("/internal/v1/game-auth/character-commands/receipt")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
}
#[derive(Debug)]
struct QualifiedAccounts;
impl crate::Accounts for QualifiedAccounts {
    async fn state(
        &self,
        _: &mut sqlx::PgConnection,
        _: Uuid,
    ) -> Result<Option<crate::AccountState>, crate::Error> {
        Ok(Some(crate::AccountState {
            active: true,
            email_verified: true,
            mfa: true,
            sudo: true,
            bazaar_blocked: false,
        }))
    }
}
async fn audit(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    intent: &PreparedIntent,
    receipt: &str,
    now: u64,
) -> ReceiptResult {
    audit_result(a, tx, intent, receipt, now).await.unwrap()
}

async fn audit_result(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    intent: &PreparedIntent,
    receipt: &str,
    now: u64,
) -> Result<ReceiptResult, Error> {
    let payload = BazaarAuthorityDecisionV1 {
        intent_sha256: Sha256::digest(intent.bytes()).to_vec(),
        receipt_sha256: Sha256::digest(receipt.as_bytes()).to_vec(),
        recovery_generation: a.fence.recovery_generation,
    }
    .encode_to_vec();
    let envelope = event_wire::EventEnvelope {
        envelope_revision: 1,
        event_id: Uuid::now_v7().as_bytes().to_vec(),
        event_type_id: 6,
        event_schema_revision: 1,
        durability_class: 2,
        privacy_class: 3,
        retention_profile_id: "PLAT3_AUTHORITY_AUDIT_RETENTION_V1".into(),
        occurred_at_unix_ms: now as i64 * 1000,
        operation_id: Some(
            id(text(&intent.value, "operation_id").unwrap())
                .unwrap()
                .as_bytes()
                .to_vec(),
        ),
        world_id: Some(
            id(text(&intent.value, "world_id").unwrap())
                .unwrap()
                .as_bytes()
                .to_vec(),
        ),
        transaction_event: Some(event_wire::TransactionEventRef {
            transaction_id: Uuid::now_v7().as_bytes().to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: "fixture-game-build".into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..Default::default()
    };
    a.ingest_decision(
        tx,
        &a.config.projection_authority,
        &envelope.encode_to_vec(),
        now,
    )
    .await
}
/// A held purchase reversal of `coins`: coin debt as payments records it.
async fn owe(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, account: Uuid, coins: i64) -> Uuid {
    let (quote, order, reversal) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    sqlx::query("INSERT INTO coin_purchase_quotes (id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES ($1,$2,$3,$4,100,'EUR',2,'testing',now(),now())")
        .bind(quote).bind(account).bind(Uuid::now_v7()).bind(coins).execute(&mut **tx).await.unwrap();
    sqlx::query("INSERT INTO coin_purchase_orders (id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code,review_required) VALUES ($1,$2,$3,$4,$5,$6,100,'EUR',2,'testing',$7,'processing','review',true)")
        .bind(order).bind(account).bind(Uuid::now_v7()).bind(quote).bind(Uuid::now_v7()).bind(coins).bind(Uuid::now_v7())
        .execute(&mut **tx).await.unwrap();
    sqlx::query("INSERT INTO coin_purchase_reversals (id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES ($1,$2,$3,'chargeback','test',$4,$5,0,'held')")
        .bind(reversal).bind(order).bind(account).bind(reversal.to_string()).bind(coins)
        .execute(&mut **tx).await.unwrap();
    reversal
}
async fn owed(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, account: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COALESCE(SUM(owed),0)::bigint FROM coin_outstanding_debts WHERE account_id=$1 AND owed>0")
        .bind(account)
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}
async fn savepoint(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, sql: &'static str) {
    sqlx::query(sql).execute(&mut **tx).await.unwrap();
}
async fn store_intent(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, intent: &PreparedIntent) {
    let key = if intent.command() == Command::Deliver {
        super::game_store::issuance_key(
            id(text(&intent.value, "listing_id").unwrap()).unwrap(),
            Command::Deliver,
            id(text(&intent.value, "sale_id").unwrap()).unwrap(),
        )
    } else {
        format!("fixture:{}", Uuid::now_v7())
    };
    store_intent_with_key(tx, intent, &key).await;
}
async fn store_intent_with_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    intent: &PreparedIntent,
    key: &str,
) {
    store_intent_at(tx, intent, key, None).await;
}
async fn store_intent_at(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    intent: &PreparedIntent,
    key: &str,
    created_at: Option<time::OffsetDateTime>,
) {
    sqlx::query("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation,created_at) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,3,3,coalesce($9,now()))").bind(id(text(&intent.value,"operation_id").unwrap()).unwrap()).bind(key).bind(id(text(&intent.value,"listing_id").unwrap()).unwrap()).bind(uid("11")).bind(intent.command().wire()).bind(text(&intent.value,"source_revision").unwrap()).bind(std::str::from_utf8(intent.bytes()).unwrap()).bind(intent.binding_digest()).bind(created_at).execute(&mut **tx).await.unwrap();
}
#[tokio::test]
async fn postgres_item_receipt_to_verified_delivery_to_wallet_completion() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(false, false, ItemScenario::Wallet).await;
}
#[tokio::test]
async fn postgres_delivery_refusal_allows_return_and_refunds_fenced_coins() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    item_wallet_scenario(true, false, ItemScenario::Wallet).await;
    eprintln!(
        "postgres_delivery_refusal_allows_return_and_refunds_fenced_coins: {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn initial_null_operation_resync() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(false, false, ItemScenario::NullResync).await;
}
#[tokio::test]
async fn initial_null_operation_escrow_ready() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(false, false, ItemScenario::NullEscrow).await;
}
#[derive(PartialEq)]
enum ItemScenario {
    Wallet,
    HeldSale,
    NullResync,
    NullEscrow,
}
async fn item_wallet_scenario(compensate: bool, suspended: bool, scenario: ItemScenario) {
    let started = std::time::Instant::now();
    use crate::{Currency, Evidence, Listing, Reservation, Reservations, Settlement};
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL PLAT3: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&mut *tx)
        .await
        .unwrap();
    let a = authority();
    let funds = CoinReservations {
        authority: a.clone(),
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let now = now as u64;
    for (who, email) in [
        (uid("11"), "plat3-seller@invalid.test"),
        (uid("13"), "plat3-buyer@invalid.test"),
    ] {
        sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'synthetic','active','en')").bind(who).bind(email).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',10000,false,now()+interval '2 days')").bind(uid("20")).bind(uid("11")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_item_details(id,definition_key,definition_revision,world_id,seller_character_id,item_instance_id,quantity) VALUES($1,'fixture-sword','v1',$2,$3,$4,1)").bind(uid("20")).bind(uid("12")).bind(uid("21")).bind(uid("22")).execute(&mut *tx).await.unwrap();
    let listing: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(!a.escrow_ready(&mut tx, &listing).await.unwrap());
    let escrow = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
    store_intent(&mut tx, &escrow).await;
    let receipt = vector("escrow_receipt");
    a.receive(
        &mut tx,
        &a.config.receipt_authority,
        receipt.as_bytes(),
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        audit(&a, &mut tx, &escrow, &receipt, now).await,
        ReceiptResult::Stored
    );
    let outbox = Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    };
    let status = vector("delivered_status")
        .replace(
            "\"projection_revision\":\"2\"",
            "\"projection_revision\":\"1\"",
        )
        .replace("\"1790000002\"", &format!("\"{now}\""))
        .replace("DELIVERED", "ESCROWED")
        .replace("\"custody_revision\":\"2\"", "\"custody_revision\":\"1\"")
        .replace("000000000004", "000000000002");
    let status = if matches!(
        scenario,
        ItemScenario::NullResync | ItemScenario::NullEscrow
    ) {
        status.replace(&format!("\"{}\"", uid("2")), "null")
    } else {
        status
    };
    if scenario == ItemScenario::NullResync {
        sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$4,7,$5,1,true,'resync-test')")
            .bind(listing.id).bind(uid("12")).bind(uid("21")).bind(uid("22")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    }
    outbox
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            status.as_bytes(),
            now,
        )
        .await
        .unwrap();
    let policy = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishBazaarTransferabilityV1\",\"source_authority\":\"oteryn:game:primary\",\"world_id\":\"{}\",\"projection_epoch\":\"3\",\"projection_revision\":\"4\",\"source_observed_at\":\"{now}\",\"eligibility_revision\":\"4\",\"definitions\":[{{\"definition\":{{\"family\":\"item\",\"production_key\":\"fixture-sword\",\"revision_ref\":\"v1\"}},\"tradeable\":true,\"max_quantity\":1,\"allowed_attribute_keys\":[\"tier\"]}}]}}",
        uid("12")
    );
    outbox
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            policy.as_bytes(),
            now,
        )
        .await
        .unwrap();
    let watermark = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishBazaarWatermarkV1\",\"source_authority\":\"oteryn:game:primary\",\"projection_epoch\":\"3\",\"complete_through\":\"{now}\",\"observed_at\":\"{now}\"}}"
    );
    outbox
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            watermark.as_bytes(),
            now,
        )
        .await
        .unwrap();
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    if scenario == ItemScenario::NullResync {
        assert!(a.fresh(&mut tx).await.unwrap());
        return;
    }
    assert!(a.escrow_ready(&mut tx, &listing).await.unwrap());
    if matches!(
        scenario,
        ItemScenario::NullResync | ItemScenario::NullEscrow
    ) {
        return;
    }
    sqlx::query("SAVEPOINT moderation_hold")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,1,$2,'fixture moderation hold')").bind(listing.id).bind(listing.owner).execute(&mut *tx).await.unwrap();
    assert!(!a.escrow_ready(&mut tx, &listing).await.unwrap());
    sqlx::query("ROLLBACK TO SAVEPOINT moderation_hold")
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut accounts = [uid("11"), uid("13")];
    accounts.sort();
    funds.lock_accounts(&mut tx, &accounts).await.unwrap();
    let seed = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,20000)",
    )
    .bind(seed)
    .bind(uid("13"))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,20000,0)").bind(Uuid::now_v7()).bind(seed).bind(uid("13")).execute(&mut *tx).await.unwrap();
    let hold = Reservation {
        id: uid("25"),
        account: uid("13"),
        currency: Currency::Coins,
        amount: 12000,
    };
    // A debtor's outgoing coin reservation is refused (owner decision
    // 2026-11-14) and leaves no hold or wallet entry behind.
    savepoint(&mut tx, "SAVEPOINT buyer_debt").await;
    owe(&mut tx, uid("13"), 1).await;
    assert!(matches!(
        funds.reserve(&mut tx, hold).await,
        Err(crate::Error::InsufficientFunds)
    ));
    let holds: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_coin_holds WHERE id=$1")
        .bind(hold.id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(holds, 0);
    savepoint(&mut tx, "ROLLBACK TO SAVEPOINT buyer_debt").await;
    funds.reserve(&mut tx, hold).await.unwrap();
    funds.reserve(&mut tx, hold).await.unwrap();
    let sale = Settlement {
        id: uid("24"),
        listing_id: uid("20"),
        buyer: uid("13"),
        seller: uid("11"),
        currency: Currency::Coins,
        amount: 10000,
        commission: 1000,
        seller_net: 9000,
        reservation: uid("25"),
    };
    sqlx::query("INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key) VALUES($1,$2,$3,$4,'coins',10000,1000,9000,$5,'fixture')").bind(sale.id).bind(sale.listing_id).bind(sale.buyer).bind(sale.seller).bind(sale.reservation).execute(&mut *tx).await.unwrap();
    funds.settle(&mut tx, &sale).await.unwrap();
    funds.settle(&mut tx, &sale).await.unwrap();
    assert_eq!(
        oteryn_wallet::incoming_capacity_in(&mut tx, sale.seller)
            .await
            .unwrap(),
        sale.seller_net
    );
    assert!(funds.release(&mut tx, hold.id, hold.account).await.is_err());
    assert!(funds.complete(&mut tx, &sale).await.is_err());
    let delivery = PreparedIntent::parse(
        &vector("delivery_intent")
            .replace("1790000000", &(now - 1).to_string())
            .replace("1790000300", &(now + 299).to_string()),
    )
    .unwrap();
    store_intent(&mut tx, &delivery).await;
    a.publish_intent(&mut tx, &QualifiedAccounts, uid("4"))
        .await
        .unwrap();
    if compensate {
        sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
            .bind(listing.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery' WHERE id=$1")
            .bind(listing.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        if scenario == ItemScenario::HeldSale {
            crate::moderation::execute_staff(
                &mut tx,
                uid("11"),
                &crate::moderation::StaffCommand::Suspend {
                    listing: listing.id,
                    reason: "staff suspended sale".into(),
                },
            )
            .await
            .unwrap();
        }
        let return_wire = include_str!("fixtures/return.json")
            .replace(
                &"1".repeat(64),
                &digest(vector("escrow_receipt").as_bytes()),
            )
            .replace("1790000020", &(now - 1).to_string())
            .replace("1790000320", &(now + 299).to_string());
        let returning = PreparedIntent::parse(&return_wire).unwrap();
        store_intent(&mut tx, &returning).await;
        assert!(matches!(
            a.publish_intent(&mut tx, &QualifiedAccounts, uid("5"))
                .await,
            Err(Error::Conflict)
        ));
        let refused = format!(
            "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"DeliverBazaarItem\",\"operation_id\":\"{}\",\"outcome\":\"REJECTED\",\"result_code\":\"CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE\",\"result_data\":null,\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}",
            uid("4")
        );
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            refused.as_bytes(),
            now,
        )
        .await
        .unwrap();
        assert!(
            a.publish_intent(&mut tx, &QualifiedAccounts, uid("5"))
                .await
                .is_err()
        );
        audit(&a, &mut tx, &delivery, &refused, now).await;
        a.publish_intent(&mut tx, &QualifiedAccounts, uid("5"))
            .await
            .unwrap();
        assert!(funds.release(&mut tx, hold.id, hold.account).await.is_err());
        let returned = vector("delivery_receipt")
            .replace("DeliverBazaarItem", "ReturnBazaarItem")
            .replace("000000000004", "000000000005")
            .replace("000000000013", "000000000011")
            .replace("000000000014", "000000000021")
            .replace("1790000002", &now.to_string());
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            returned.as_bytes(),
            now,
        )
        .await
        .unwrap();
        audit(&a, &mut tx, &returning, &returned, now).await;
        let returned_status = vector("delivered_status")
            .replace("DELIVERED", "RETURNED")
            .replace("000000000004", "000000000005")
            .replace("1790000002", &now.to_string());
        outbox
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                returned_status.as_bytes(),
                now,
            )
            .await
            .unwrap();
        assert!(a.finish_compensation(&mut tx, &listing).await.is_err());
        if suspended {
            sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
                .bind(sale.buyer)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        funds.release(&mut tx, hold.id, hold.account).await.unwrap();
        funds.release(&mut tx, hold.id, hold.account).await.unwrap();
        assert_eq!(
            oteryn_wallet::incoming_capacity_in(&mut tx, sale.seller)
                .await
                .unwrap(),
            0
        );
        a.finish_compensation(&mut tx, &listing).await.unwrap();
        a.finish_compensation(&mut tx, &listing).await.unwrap();
        let disposition: (String,i64) = sqlx::query_as("SELECT l.state::text,(SELECT count(*) FROM bazaar_authority_compensations c WHERE c.listing_id=l.id) FROM bazaar_listings l WHERE l.id=$1")
            .bind(listing.id).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(disposition, ("cancelled".into(), 1));
        assert!(funds.complete(&mut tx, &sale).await.is_err());
        let balances: (i64,i64) = sqlx::query_as("SELECT sum(available_delta)::bigint,sum(reserved_delta)::bigint FROM wallet_entries WHERE account_id=$1").bind(sale.buyer).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(balances, (20000, 0));
        let captures: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM wallet_operations WHERE kind='capture' AND account_id=$1",
        )
        .bind(sale.buyer)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(captures, 0);
        // The terminal sale's evidence must be rechecked before a new action can
        // use the returned-custody proof. The compensation replay above is a no-op.
        assert!(!a.resynced(&mut tx).await.unwrap());
        a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
        assert!(a.resynced(&mut tx).await.unwrap());
        // The same returned-custody proof releases a Gold settlement fence,
        // while ordinary releases remain denied until payout.
        let gold_hold = Uuid::now_v7();
        gold_fact(
            &mut tx,
            Command::Hold,
            30,
            Uuid::now_v7(),
            gold_hold,
            (0, 1, 12000),
        )
        .await;
        sqlx::query("INSERT INTO bazaar_authority_gold_sales(id,listing_id,buyer,seller,gross_amount,fee_amount,seller_amount,hold_id,hold_revision) VALUES($1,$2,$3,$4,10000,250,9750,$5,1)")
            .bind(Uuid::now_v7()).bind(sale.listing_id).bind(sale.buyer).bind(sale.seller).bind(gold_hold).execute(&mut *tx).await.unwrap();
        let Value::Object(common) = decode(delivery.bytes(), 4096).unwrap() else {
            panic!("intent fixture");
        };
        let mut fields = common[..10].to_vec();
        let release_operation = Uuid::now_v7();
        for (name, value) in &mut fields {
            *value = match name.as_str() {
                "command" => Value::String(Command::Release.wire().into()),
                "operation_id" | "issuer_decision_id" => {
                    Value::String(release_operation.to_string())
                }
                "source_revision" => Value::String("31".into()),
                _ => value.clone(),
            };
        }
        for (key, value) in [
            ("buyer_account_id", sale.buyer.to_string()),
            ("hold_id", gold_hold.to_string()),
            ("expected_hold_revision", "1".into()),
            ("release_reason", "TERMINAL_REFUSAL".into()),
            ("amount", "12000".into()),
        ] {
            fields.push((key.into(), Value::String(value)));
        }
        fields.extend_from_slice(&common[common.len() - 3..]);
        let raw = String::from_utf8(canonical::encode(&Value::Object(fields)).unwrap()).unwrap();
        let releasing = PreparedIntent::parse(&raw).unwrap();
        assert!(
            runtime::gold::validate_release(&a, &mut tx, &releasing, None)
                .await
                .is_ok()
        );
        let ordinary = PreparedIntent::parse(&raw.replace("TERMINAL_REFUSAL", "OUTBID")).unwrap();
        assert!(
            runtime::gold::validate_release(&a, &mut tx, &ordinary, None)
                .await
                .is_err()
        );
        // A late contradictory/ambiguous delivery decision reinstates the fence.
        savepoint(&mut tx, "SAVEPOINT ambiguous_delivery").await;
        sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'RECEIPT')").bind(uid("4")).bind("0".repeat(64)).execute(&mut *tx).await.unwrap();
        assert!(
            runtime::gold::validate_release(&a, &mut tx, &releasing, None)
                .await
                .is_err()
        );
        savepoint(&mut tx, "ROLLBACK TO SAVEPOINT ambiguous_delivery").await;
        store_intent(&mut tx, &releasing).await;
        a.publish_intent(&mut tx, &QualifiedAccounts, release_operation)
            .await
            .unwrap();
        let release_receipt = format!(
            "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"ReleaseBazaarGold\",\"operation_id\":\"{release_operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"result_data\":{{\"hold_id\":\"{gold_hold}\",\"hold_revision\":\"2\",\"released_amount\":\"12000\",\"remaining_held\":\"0\",\"hold_state\":\"RELEASED\"}},\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}"
        );
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            release_receipt.as_bytes(),
            now,
        )
        .await
        .unwrap();
        audit(&a, &mut tx, &releasing, &release_receipt, now).await;
        assert_eq!(
            runtime::gold::current_hold(&a, &mut tx, listing.id, sale.buyer, None)
                .await
                .unwrap()
                .unwrap()
                .amount,
            0
        );
        assert_eq!(
            a.retained(&mut tx, uid("4")).await.unwrap().unwrap().1.raw,
            refused
        );
        tx.rollback().await.unwrap();
        eprintln!(
            "postgres_suspended_compensation_refunds_once: {:?}",
            started.elapsed()
        );
        return;
    }
    let delivered = vector("delivered_status").replace("1790000002", &now.to_string());
    outbox
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            delivered.as_bytes(),
            now,
        )
        .await
        .unwrap();
    assert!(!a.delivered(&mut tx, &sale).await.unwrap());
    let receipt = vector("delivery_receipt").replace("1790000002", &now.to_string());
    a.receive(
        &mut tx,
        &a.config.receipt_authority,
        receipt.as_bytes(),
        now,
    )
    .await
    .unwrap();
    assert!(!a.delivered(&mut tx, &sale).await.unwrap());
    sqlx::query("SAVEPOINT contradictory_move")
        .execute(&mut *tx)
        .await
        .unwrap();
    let return_wire = include_str!("fixtures/return.json")
        .replace(
            &"1".repeat(64),
            &digest(vector("escrow_receipt").as_bytes()),
        )
        .replace("1790000020", &(now - 1).to_string())
        .replace("1790000320", &(now + 299).to_string());
    let return_intent = PreparedIntent::parse(&return_wire).unwrap();
    store_intent(&mut tx, &return_intent).await;
    let conflicting = receipt
        .replace("DeliverBazaarItem", "ReturnBazaarItem")
        .replace("000000000004", "000000000005")
        .replace("000000000013", "000000000011")
        .replace("000000000014", "000000000021");
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            conflicting.as_bytes(),
            now
        )
        .await
        .unwrap(),
        ReceiptResult::Quarantined
    );
    let conflicts: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_authority_conflicts")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(conflicts, 2);
    assert!(!a.delivered(&mut tx, &sale).await.unwrap());
    sqlx::query("ROLLBACK TO SAVEPOINT contradictory_move")
        .execute(&mut *tx)
        .await
        .unwrap();
    audit(&a, &mut tx, &delivery, &receipt, now).await;
    assert!(a.delivered(&mut tx, &sale).await.unwrap());
    if suspended {
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id IN ($1,$2)")
            .bind(sale.buyer)
            .bind(sale.seller)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    // The seller's proceeds repay the seller's debt first. A buyer debt that
    // arose after the reservation does not stop the fill from completing.
    savepoint(&mut tx, "SAVEPOINT seller_debt").await;
    let seller_debt = owe(&mut tx, sale.seller, 4000).await;
    owe(&mut tx, sale.buyer, 500).await;
    funds.complete(&mut tx, &sale).await.unwrap();
    funds.complete(&mut tx, &sale).await.unwrap();
    assert_eq!(
        oteryn_wallet::incoming_capacity_in(&mut tx, sale.seller)
            .await
            .unwrap(),
        0
    );
    let seller: i64 = sqlx::query_scalar(
        "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(sale.seller)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(seller, 5000);
    assert_eq!(owed(&mut tx, sale.seller).await, 0);
    assert_eq!(owed(&mut tx, sale.buyer).await, 500);
    let buyer:(i64,i64)=sqlx::query_as("SELECT sum(available_delta)::bigint,sum(reserved_delta)::bigint FROM wallet_entries WHERE account_id=$1").bind(sale.buyer).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(buyer, (10000, 0));
    let (paid, credit): (i64, Uuid) = sqlx::query_as(
        "SELECT coins,credit_operation_id FROM coin_debt_settlements WHERE reversal_id=$1",
    )
    .bind(seller_debt)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(paid, 4000);
    let capture: Uuid = sqlx::query_scalar(
        "SELECT id FROM wallet_operations WHERE kind='capture' AND reference_id=$1",
    )
    .bind(sale.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(credit, capture);
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='wallet.coin_debt_settled' AND target=$1",
    )
    .bind(seller_debt.to_string())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(audited, 1);
    savepoint(&mut tx, "ROLLBACK TO SAVEPOINT seller_debt").await;
    // Without debt the fill completes exactly as before.
    funds.complete(&mut tx, &sale).await.unwrap();
    funds.complete(&mut tx, &sale).await.unwrap();
    assert_eq!(
        oteryn_wallet::incoming_capacity_in(&mut tx, sale.seller)
            .await
            .unwrap(),
        0
    );
    let settled: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM coin_debt_settlements WHERE account_id IN ($1,$2)",
    )
    .bind(sale.buyer)
    .bind(sale.seller)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(settled, 0);
    let seller: i64 = sqlx::query_scalar(
        "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(sale.seller)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(seller, 9000);
    let buyer:(i64,i64)=sqlx::query_as("SELECT sum(available_delta)::bigint,sum(reserved_delta)::bigint FROM wallet_entries WHERE account_id=$1").bind(sale.buyer).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(buyer, (10000, 0));
    let mut restored = a.clone();
    restored.fence.recovery_generation = 4;
    assert!(!restored.delivered(&mut tx, &sale).await.unwrap());
    tx.rollback().await.unwrap();
    pool.close().await;
    eprintln!(
        "postgres_suspended_delivered_settlement_captures_once: {:?}",
        started.elapsed()
    );
}
#[test]
fn all_nine_intent_schemas_and_exact_source_files() {
    let base = decode(vector("escrow_intent").as_bytes(), 4096).unwrap();
    let Value::Object(common) = base else {
        panic!("object fixture");
    };
    let u = uid("32");
    let buyer = uid("13");
    let h = "a".repeat(64);
    let cases = [
        (
            Command::Lock,
            format!(
                "{{\"character_id\":\"{u}\",\"expected_character_revision\":\"7\",\"eligibility_revision\":\"4\",\"snapshot_schema_version\":1}}"
            ),
        ),
        (
            Command::Bind,
            format!(
                "{{\"character_id\":\"{u}\",\"lock_id\":\"{u}\",\"expected_lock_revision\":\"1\",\"transfer_operation_id\":\"{u}\",\"transfer_binding_digest\":\"{h}\",\"purpose\":\"BAZAAR_LISTING_ESCROW\",\"sale_id\":null,\"funding\":null}}"
            ),
        ),
        (
            Command::Unlock,
            format!(
                "{{\"character_id\":\"{u}\",\"lock_id\":\"{u}\",\"expected_lock_revision\":\"1\"}}"
            ),
        ),
        (
            Command::Escrow,
            format!(
                "{{\"seller_character_id\":\"{u}\",\"item_instance_id\":\"{u}\",\"expected_state_revision\":\"7\",\"quantity\":1,\"transferability_revision\":\"4\"}}"
            ),
        ),
        (
            Command::Deliver,
            format!(
                "{{\"escrow_id\":\"{u}\",\"expected_escrow_revision\":\"1\",\"buyer_account_id\":\"{buyer}\",\"buyer_character_id\":\"{u}\",\"escrow_receipt_digest\":\"{h}\",\"sale_id\":\"{u}\",\"funding\":{{\"currency\":\"COINS\",\"reservation_id\":\"{u}\",\"hold_revision\":null,\"gross_amount\":\"10000\"}}}}"
            ),
        ),
        (
            Command::Return,
            format!(
                "{{\"escrow_id\":\"{u}\",\"expected_escrow_revision\":\"1\",\"seller_character_id\":\"{u}\",\"escrow_receipt_digest\":\"{h}\"}}"
            ),
        ),
        (
            Command::Hold,
            format!(
                "{{\"buyer_account_id\":\"{buyer}\",\"buyer_character_id\":\"{u}\",\"offer_id\":\"{u}\",\"hold_purpose\":\"OFFER\",\"amount\":\"10000\",\"expected_hold_revision\":\"0\"}}"
            ),
        ),
        (
            Command::Release,
            format!(
                "{{\"buyer_account_id\":\"{buyer}\",\"hold_id\":\"{u}\",\"expected_hold_revision\":\"1\",\"release_reason\":\"EXCESS\",\"amount\":\"10000\"}}"
            ),
        ),
        (
            Command::Payout,
            format!(
                "{{\"buyer_account_id\":\"{buyer}\",\"hold_id\":\"{u}\",\"expected_hold_revision\":\"1\",\"sale_id\":\"{u}\",\"gross_amount\":\"10000\",\"fee_bps\":250,\"fee_amount\":\"250\",\"seller_amount\":\"9750\",\"delivery_operation_id\":\"{u}\",\"delivery_receipt_digest\":\"{h}\"}}"
            ),
        ),
    ];
    for (command, extra) in cases {
        let mut fields = common[..10].to_vec();
        fields[1].1 = Value::String(command.wire().into());
        let Value::Object(extra) = decode(extra.as_bytes(), 4096).unwrap() else {
            panic!("command fields");
        };
        fields.extend(extra);
        fields.extend_from_slice(&common[common.len() - 3..]);
        let raw = String::from_utf8(canonical::encode(&Value::Object(fields)).unwrap()).unwrap();
        assert_eq!(PreparedIntent::parse(&raw).unwrap().command(), command);
        assert!(
            PreparedIntent::parse(&raw.replace(
                "\"contract_version\":2",
                "\"contract_version\":2,\"unknown\":null"
            ))
            .is_err()
        );
    }
    for (bytes, want) in [
        (
            include_bytes!("../../tests/fixtures/plat3/README.md").as_slice(),
            "01c7da512b09b43ae214f1dcb689939e5855eaf588a8baca94e5630c501b172d",
        ),
        (
            include_bytes!("../../tests/fixtures/plat3/plat3_authority.proto").as_slice(),
            "d8a306474f5c01275e5d87c4fd056d8f6ccad2ec577e7640c6fd1c66b72dd642",
        ),
        (
            include_bytes!("../../tests/fixtures/plat3/foundation.proto").as_slice(),
            "e55e1b634f18145f2b5a4622c648db31de7b93fc911bafb76d1acdf9af48cce5",
        ),
    ] {
        assert_eq!(digest(bytes), want);
    }
}
#[tokio::test]
async fn postgres_concurrent_receipt_publication_commits_one_immutable_decision() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use sqlx::AssertSqlSafe;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL PLAT3: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("plat3_concurrent_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |db, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(format!("SET search_path TO {path}")))
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut setup = pool.begin().await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&mut *setup)
        .await
        .unwrap();
    store_intent(
        &mut setup,
        &PreparedIntent::parse(&vector("escrow_intent")).unwrap(),
    )
    .await;
    setup.commit().await.unwrap();
    let a = authority();
    let raw = vector("escrow_receipt");
    let receive = || async {
        let mut tx = pool.begin().await.unwrap();
        let result = a
            .receive(
                &mut tx,
                &a.config.receipt_authority,
                raw.as_bytes(),
                1790000003,
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();
        result
    };
    let (first, second) = tokio::join!(receive(), receive());
    assert!(matches!(
        (first, second),
        (ReceiptResult::Stored, ReceiptResult::Duplicate)
            | (ReceiptResult::Duplicate, ReceiptResult::Stored)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_authority_receipts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    // CC section 4.3 acknowledgement bytes stay version 1, even for its
    // candidate v2 result-data extension. Verify the actual protected facility.
    {
        use axum::{
            Router,
            body::{Body, to_bytes},
            http::{Request, StatusCode},
        };
        use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
        use std::sync::Arc;
        use tower::ServiceExt;
        let identity = Identity::san("plat3-receipt-fixture").unwrap();
        let identities = IdentityMap::new(vec![Binding {
            identity: identity.clone(),
            group: authority_http::RECEIPT_GROUP.into(),
        }])
        .unwrap();
        let routes = authority_http::extend_receipt_facility(
            Router::new(),
            a.clone(),
            pool.clone(),
            Arc::new(identities),
        );
        let response = routes
            .oneshot(
                Request::post("/internal/v1/game-auth/character-commands/receipt")
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(identity))
                    .body(Body::from(raw.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 256).await.unwrap().as_ref(),
            b"{\"contract_version\":1,\"result\":\"accepted\"}"
        );
    }
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn gold_fact(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: Command,
    revision: u64,
    offer: Uuid,
    hold: Uuid,
    result: (u64, u64, u64),
) -> Uuid {
    let (expected, result_revision, amount) = result;
    let operation = Uuid::now_v7();
    let Value::Object(common) = decode(vector("escrow_intent").as_bytes(), 4096).unwrap() else {
        panic!("fixture object");
    };
    let mut fields = common[..10].to_vec();
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "command" => Value::String(command.wire().into()),
            "operation_id" | "issuer_decision_id" => Value::String(operation.to_string()),
            "source_revision" => Value::String(revision.to_string()),
            _ => value.clone(),
        };
    }
    let buyer = uid("13");
    let extra = if command == Command::Hold {
        format!(
            "{{\"buyer_account_id\":\"{buyer}\",\"buyer_character_id\":\"{}\",\"offer_id\":\"{offer}\",\"hold_purpose\":\"RESERVE\",\"amount\":\"{amount}\",\"expected_hold_revision\":\"{expected}\"}}",
            uid("21")
        )
    } else {
        format!(
            "{{\"buyer_account_id\":\"{buyer}\",\"hold_id\":\"{hold}\",\"expected_hold_revision\":\"{expected}\",\"release_reason\":\"OUTBID\",\"amount\":\"{amount}\"}}"
        )
    };
    let Value::Object(extra) = decode(extra.as_bytes(), 4096).unwrap() else {
        panic!("fields");
    };
    fields.extend(extra);
    fields.extend_from_slice(&common[common.len() - 3..]);
    let raw = String::from_utf8(canonical::encode(&Value::Object(fields)).unwrap()).unwrap();
    let intent = PreparedIntent::parse(&raw).unwrap();
    store_intent(tx, &intent).await;
    let data = if command == Command::Hold {
        format!(
            "{{\"hold_id\":\"{hold}\",\"hold_revision\":\"{result_revision}\",\"buyer_account_id\":\"{buyer}\",\"offer_id\":\"{offer}\",\"hold_purpose\":\"RESERVE\",\"held_amount\":\"{amount}\"}}"
        )
    } else {
        format!(
            "{{\"hold_id\":\"{hold}\",\"hold_revision\":\"{result_revision}\",\"released_amount\":\"{amount}\",\"remaining_held\":\"0\",\"hold_state\":\"RELEASED\"}}"
        )
    };
    let receipt = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"{}\",\"operation_id\":\"{operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"result_data\":{data},\"recovery_generation\":\"3\",\"decided_at\":\"1790000002\"}}",
        command.wire()
    );
    let r = Receipt::parse(receipt.as_bytes()).unwrap();
    sqlx::query("INSERT INTO bazaar_authority_receipts(operation_id,recovery_generation,source_authority,intent_digest,receipt_digest,receipt,outcome) VALUES($1,3,$2,$3,$4,$5,'COMMITTED')").bind(operation).bind(&authority().config.receipt_authority).bind(digest(intent.bytes())).bind(r.digest()).bind(&receipt).execute(&mut **tx).await.unwrap();
    let p = BazaarAuthorityDecisionV1 {
        intent_sha256: Sha256::digest(intent.bytes()).to_vec(),
        receipt_sha256: Sha256::digest(r.bytes()).to_vec(),
        recovery_generation: 3,
    }
    .encode_to_vec();
    sqlx::query("INSERT INTO bazaar_authority_decisions(event_id,envelope_semantics,transaction_id,ordinal,event_count,operation_id,recovery_generation,source_authority,payload,payload_digest) VALUES($1,$2,$1,1,1,$3,3,$4,$5,$6)").bind(Uuid::now_v7()).bind(b"gold lifecycle regression".as_slice()).bind(operation).bind(&authority().config.projection_authority).bind(&p).bind(Sha256::digest(&p).to_vec()).execute(&mut **tx).await.unwrap();
    operation
}
#[tokio::test]
async fn postgres_released_gold_hold_starts_new_lifecycle_and_reconstruction() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let a = authority();
    let offer = uid("20");
    let first = Uuid::now_v7();
    gold_fact(&mut tx, Command::Hold, 8001, offer, first, (0, 1, 10000)).await;
    let hold = runtime::gold::current_hold(&a, &mut tx, offer, uid("13"), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        runtime::gold::hold_binding(offer, Some(&hold), Uuid::now_v7()),
        (offer, 1)
    );
    gold_fact(&mut tx, Command::Release, 8002, offer, first, (1, 2, 10000)).await;
    let released = runtime::gold::current_hold(&a, &mut tx, offer, uid("13"), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(released.amount, 0);
    let request = Uuid::now_v7();
    assert_eq!(
        runtime::gold::hold_binding(offer, Some(&released), request),
        (request, 0)
    );
    let second = Uuid::now_v7();
    let op = gold_fact(&mut tx, Command::Hold, 8003, request, second, (0, 1, 20000)).await;
    let hold = runtime::gold::current_hold(&a, &mut tx, offer, uid("13"), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((hold.id, hold.revision, hold.amount), (second, 1, 20000));
    let (intent, receipt) = a.retained(&mut tx, op).await.unwrap().unwrap();
    runtime::gold::validate_hold(&a, &mut tx, &intent, receipt.value.get("result_data"))
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    eprintln!(
        "postgres_released_gold_hold_starts_new_lifecycle_and_reconstruction: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_completed_resync_does_not_rescan_terminal_history() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_resync(projection_epoch,recovery_generation) VALUES(3,3) ON CONFLICT DO NOTHING").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_resync_progress(projection_epoch,recovery_generation,worlds) VALUES(3,3,$1) ON CONFLICT DO NOTHING")
        .bind(format!("delivery-sales-v5:{}", uid("12")))
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) SELECT uuidv7(),$1,'item','auction','coins',100,false,'completed',now() FROM generate_series(1,2000)").bind(Uuid::now_v7()).execute(&mut *tx).await.unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        authority().finish_resync(&mut tx, &[uid("12")]),
    )
    .await
    .unwrap();
    assert!(result.is_ok());
    tx.rollback().await.unwrap();
    eprintln!(
        "postgres_completed_resync_does_not_rescan_terminal_history: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_receipt_evidence_and_worker_listing_lock_do_not_deadlock() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use crate::Evidence;
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&pool)
        .await
        .unwrap();
    let listing = Uuid::now_v7();
    let seller = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','gold',100,false,now()+interval '1 day')").bind(listing).bind(seller).execute(&pool).await.unwrap();
    let mut worker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE")
        .bind(listing)
        .execute(&mut *worker)
        .await
        .unwrap();
    let (signal, ready) = tokio::sync::oneshot::channel();
    let other = pool.clone();
    let receipt = tokio::spawn(async move {
        let mut tx = other.begin().await?;
        let a = authority();
        a.lock(&mut tx).await?;
        let _ = signal.send(());
        let sale = crate::Settlement {
            id: Uuid::now_v7(),
            listing_id: listing,
            buyer: Uuid::now_v7(),
            seller,
            currency: crate::Currency::Gold,
            amount: 100,
            commission: 3,
            seller_net: 97,
            reservation: Uuid::now_v7(),
        };
        let result = a.delivered(&mut tx, &sale).await;
        tx.rollback().await?;
        Ok::<_, Error>(result)
    });
    ready.await.unwrap();
    let worker_result = authority().lock(&mut worker).await;
    worker.rollback().await.unwrap();
    assert!(worker_result.is_ok());
    assert!(receipt.await.unwrap().unwrap().is_ok());
    eprintln!(
        "postgres_receipt_evidence_and_worker_listing_lock_do_not_deadlock: {:?}",
        started.elapsed()
    );
}

fn transfer_fixture(operation: Uuid, revision: i64, issued: u64) -> Value {
    let Value::Object(mut fields) =
        decode(vector("character_settlement_intent").as_bytes(), 4096).unwrap()
    else {
        panic!("transfer fixture");
    };
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "operation_id" => Value::String(operation.to_string()),
            "source_revision" => Value::String(revision.to_string()),
            "issued_at_source" => Value::String(issued.to_string()),
            "expires_at_source" => Value::String((issued + 300).to_string()),
            _ => value.clone(),
        };
    }
    Value::Object(fields)
}
async fn stage_transfer_fixture(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    listing: Uuid,
    transfer: &Value,
) {
    let binding = Value::Object(
        [
            "character_id",
            "expected_from_account_id",
            "to_account_id",
            "purpose",
        ]
        .into_iter()
        .map(|name| (name.into(), transfer.get(name).unwrap().clone()))
        .collect(),
    );
    let raw = String::from_utf8(canonical::encode(transfer).unwrap()).unwrap();
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_transfers WHERE listing_id=$1 AND purpose=$2)").bind(listing).bind(text(transfer,"purpose").unwrap()).fetch_one(&mut **tx).await.unwrap();
    let query = if exists {
        "INSERT INTO bazaar_authority_transfer_retries(operation_id,listing_id,purpose,decision_id,source_revision,intent,intent_digest,binding,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,3,3)"
    } else {
        "INSERT INTO bazaar_authority_transfers(operation_id,listing_id,purpose,decision_id,source_revision,intent,intent_digest,binding,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,3,3)"
    };
    sqlx::query(query)
        .bind(id(text(transfer, "operation_id").unwrap()).unwrap())
        .bind(listing)
        .bind(text(transfer, "purpose").unwrap())
        .bind(id(text(transfer, "issuer_decision_id").unwrap()).unwrap())
        .bind(number(transfer, "source_revision").unwrap() as i64)
        .bind(&raw)
        .bind(digest(raw.as_bytes()))
        .bind(String::from_utf8(canonical::encode(&binding).unwrap()).unwrap())
        .execute(&mut **tx)
        .await
        .unwrap();
}
async fn transfer_listing(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Uuid {
    let listing = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'character','auction','coins',100,false,now()+interval '1 day')").bind(listing).bind(uid("11")).execute(&mut **tx).await.unwrap();
    listing
}
#[tokio::test]
async fn postgres_legacy_staged_transfer_publishes_exact_bytes_after_newer_revision() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let listing = transfer_listing(&mut tx).await;
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let issued = runtime::now(&mut tx).await.unwrap();
    let operation = Uuid::now_v7();
    let transfer = transfer_fixture(operation, revision, issued);
    let raw = canonical::encode(&transfer).unwrap();
    issue_create_fixture(&mut tx).await.unwrap();
    // This ordering represents a pre-upgrade staged record whose revision was overtaken.
    stage_transfer_fixture(&mut tx, listing, &transfer).await;
    runtime::worker::publish_transfer(
        &mut tx,
        listing,
        text(&transfer, "purpose").unwrap(),
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    let stored: String =
        sqlx::query_scalar("SELECT intent_json FROM character_commands WHERE id=$1")
            .bind(operation)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(stored.as_bytes(), raw);
    runtime::worker::publish_transfer(
        &mut tx,
        listing,
        text(&transfer, "purpose").unwrap(),
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    eprintln!(
        "postgres_legacy_staged_transfer_publishes_exact_bytes_after_newer_revision: {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn postgres_transfer_expiry_requires_terminal_no_commit_before_replacement() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    transfer_rejection_scenario("CHAR_CMD_INTENT_EXPIRED").await;
    eprintln!(
        "postgres_transfer_expiry_requires_terminal_no_commit_before_replacement: {:?}",
        started.elapsed()
    );
}
async fn transfer_rejection_scenario(code: &str) {
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let listing = transfer_listing(&mut tx).await;
    sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
        .bind(listing)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery' WHERE id=$1")
        .bind(listing)
        .execute(&mut *tx)
        .await
        .unwrap();
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let operation = Uuid::now_v7();
    let now = runtime::now(&mut tx).await.unwrap();
    let transfer = transfer_fixture(operation, revision, now - 301);
    let purpose = text(&transfer, "purpose").unwrap();
    let raw = canonical::encode(&transfer).unwrap();
    stage_transfer_fixture(&mut tx, listing, &transfer).await;
    assert_eq!(
        runtime::characters::reconcile_transfer(&authority(), &mut tx, listing, purpose)
            .await
            .unwrap(),
        Some(operation)
    );
    issue_create_fixture(&mut tx).await.unwrap();
    runtime::worker::publish_transfer(&mut tx, listing, purpose, operation, &transfer, &raw)
        .await
        .unwrap();
    let receipt = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"TransferCharacterOwnership\",\"operation_id\":\"{operation}\",\"outcome\":\"REJECTED\",\"result_code\":\"{code}\",\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}"
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(operation)
        .bind(&receipt)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE character_commands SET state='PENDING',receipt_json=NULL WHERE id=$1")
        .bind(operation)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        runtime::characters::reconcile_transfer(&authority(), &mut tx, listing, purpose)
            .await
            .unwrap(),
        Some(operation)
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(operation)
        .bind(&receipt)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        runtime::characters::reconcile_transfer(&authority(), &mut tx, listing, purpose)
            .await
            .unwrap(),
        None
    );
    let old: String =
        sqlx::query_scalar("SELECT intent FROM bazaar_authority_transfers WHERE operation_id=$1")
            .bind(operation)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(old.as_bytes(), raw);
    let new_revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let new_operation = Uuid::now_v7();
    let retry = transfer_fixture(new_operation, new_revision, now);
    stage_transfer_fixture(&mut tx, listing, &retry).await;
    assert_eq!(
        runtime::characters::reconcile_transfer(&authority(), &mut tx, listing, purpose)
            .await
            .unwrap(),
        Some(new_operation)
    );
    let retry_raw = canonical::encode(&retry).unwrap();
    runtime::worker::publish_transfer(&mut tx, listing, purpose, new_operation, &retry, &retry_raw)
        .await
        .unwrap();
    assert!(new_revision > revision);
    assert_ne!(new_operation, operation);
    let old_receipt: String =
        sqlx::query_scalar("SELECT receipt_json FROM character_commands WHERE id=$1")
            .bind(operation)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(old_receipt, receipt);
    tx.rollback().await.unwrap();
    eprintln!(
        "transfer_rejection_scenario({code}): {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_transfer_session_rejection_requires_terminal_no_commit_before_replacement() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for code in [
        "CHAR_CMD_CHARACTER_IN_SESSION",
        "CHAR_CMD_OWNER_MISMATCH",
        "CHAR_CMD_CHARACTER_NOT_FOUND",
        "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE",
        "CHAR_CMD_HOUSING_NOT_PREPARED",
        "CHAR_CMD_SAME_ACCOUNT",
        "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT",
        "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE",
        "CHAR_CMD_INTENT_INVALID",
        "CHAR_CMD_OPERATION_CONFLICT",
        "CHAR_CMD_UNSUPPORTED",
    ] {
        transfer_rejection_scenario(code).await;
    }
    eprintln!(
        "postgres_transfer_session_rejection_requires_terminal_no_commit_before_replacement: {:?}",
        started.elapsed()
    );
}

async fn issue_create_fixture(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut **tx)
    .await?;
    let operation = Uuid::now_v7();
    let issued = runtime::now(tx).await.unwrap();
    let Value::Object(mut fields) = decode(
        include_str!("../../../characters/src/commands/fixtures/create-intent.json").as_bytes(),
        4096,
    )
    .unwrap() else {
        panic!("create fixture");
    };
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "operation_id" => Value::String(operation.to_string()),
            "source_revision" => Value::String(revision.to_string()),
            "issued_at_source" => Value::String(issued.to_string()),
            "expires_at_source" => Value::String((issued + 300).to_string()),
            _ => value.clone(),
        };
    }
    let intent = Value::Object(fields);
    let binding = Value::Object(
        ["account_id", "target_world_id", "requested_name"]
            .into_iter()
            .map(|name| (name.into(), intent.get(name).unwrap().clone()))
            .collect(),
    );
    sqlx::query("INSERT INTO character_commands(id,source_revision,issuer_decision_id,request_key,command,account_id,binding_json,intent_json) OVERRIDING SYSTEM VALUE VALUES($1,$2,$3,$1::text,'CreateCharacter',$4,$5,$6)")
        .bind(operation).bind(revision).bind(id(text(&intent,"issuer_decision_id").unwrap()).unwrap()).bind(id(text(&intent,"account_id").unwrap()).unwrap()).bind(String::from_utf8(canonical::encode(&binding).unwrap()).unwrap()).bind(String::from_utf8(canonical::encode(&intent).unwrap()).unwrap()).execute(&mut **tx).await?;
    Ok(operation)
}
#[tokio::test]
async fn postgres_private_transfer_barrier_fences_all_character_issuers() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let listing = transfer_listing(&mut tx).await;
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let operation = Uuid::now_v7();
    let now = runtime::now(&mut tx).await.unwrap();
    let transfer = transfer_fixture(operation, revision, now);
    stage_transfer_fixture(&mut tx, listing, &transfer).await;
    sqlx::query("SAVEPOINT before_competing_issue")
        .execute(&mut *tx)
        .await
        .unwrap();
    let blocked = issue_create_fixture(&mut tx).await;
    assert!(
        matches!(blocked,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("55P03"))
    );
    sqlx::query("ROLLBACK TO SAVEPOINT before_competing_issue")
        .execute(&mut *tx)
        .await
        .unwrap();
    let raw = canonical::encode(&transfer).unwrap();
    runtime::worker::publish_transfer(
        &mut tx,
        listing,
        text(&transfer, "purpose").unwrap(),
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    issue_create_fixture(&mut tx).await.unwrap();
    tx.rollback().await.unwrap();
    eprintln!(
        "postgres_private_transfer_barrier_fences_all_character_issuers: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_resync_advances_only_a_bounded_fence_scoped_operation_batch() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    for table in [
        "bazaar_game_issuer",
        "bazaar_game_outbox",
        "bazaar_authority_receipts",
        "bazaar_authority_decisions",
        "bazaar_authority_conflicts",
        "bazaar_authority_resync",
        "bazaar_resync_progress",
        "bazaar_game_projections",
        "bazaar_game_quarantine",
        "bazaar_listings",
        "bazaar_authority_assets",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE TEMP TABLE {table} (LIKE public.{table} INCLUDING ALL)"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO bazaar_game_issuer(singleton,projection_epoch,recovery_generation) VALUES(true,3,3)").execute(&mut *tx).await.unwrap();
    let a = authority();
    let now = runtime::now(&mut tx).await.unwrap();
    let policy = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishBazaarTransferabilityV1\",\"source_authority\":\"oteryn:game:primary\",\"world_id\":\"{}\",\"projection_epoch\":\"3\",\"projection_revision\":\"4\",\"source_observed_at\":\"{now}\",\"eligibility_revision\":\"4\",\"definitions\":[]}}",
        uid("12")
    );
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        &mut tx,
        &a.config.projection_authority,
        policy.as_bytes(),
        now,
    )
    .await
    .unwrap();
    let hold = Uuid::now_v7();
    let offer = Uuid::now_v7();
    for revision in 1..=49_u64 {
        gold_fact(
            &mut tx,
            Command::Hold,
            revision,
            offer,
            hold,
            (revision - 1, revision, 10000 + revision),
        )
        .await;
    }
    for expected in [24_u64, 48, 49] {
        a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
        let cursor:String=sqlx::query_scalar("SELECT operation_cursor::text FROM bazaar_resync_progress WHERE projection_epoch=3 AND recovery_generation=3").fetch_one(&mut *tx).await.unwrap();
        assert_eq!(cursor, expected.to_string());
        let complete:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_resync WHERE projection_epoch=3 AND recovery_generation=3)").fetch_one(&mut *tx).await.unwrap();
        assert_eq!(complete, expected == 49);
    }
    tx.rollback().await.unwrap();
    eprintln!(
        "postgres_resync_advances_only_a_bounded_fence_scoped_operation_batch: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn bzb_receipt_admission_precedes_oversized_version_selection() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use oteryn_internal_api::guard::{
        self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy, VerifiedClient,
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let identities = Arc::new(
        IdentityMap::new(vec![Binding {
            identity: Identity::spki_sha256([1; 32]),
            group: authority_http::RECEIPT_GROUP.into(),
        }])
        .unwrap(),
    );
    for enabled in [true, false] {
        let policy = RoutePolicy {
            enabled,
            group: authority_http::RECEIPT_GROUP.into(),
            max_body_bytes: 8192,
            max_json_depth: 12,
            json: JsonRule::Object,
        };
        let legacy = guard::protect(
            Router::new().route(
                "/internal/v1/game-auth/character-commands/receipt",
                post(|| async { StatusCode::OK }),
            ),
            identities.clone(),
            policy,
        );
        let mut a = authority();
        a.config.enabled = enabled;
        let app =
            authority_http::extend_receipt_facility(legacy, a, pool.clone(), identities.clone());
        let request = Request::post("/internal/v1/game-auth/character-commands/receipt")
            .header("content-type", "application/json")
            .extension(VerifiedClient::from_verified_tls13_http11(
                Identity::spki_sha256([2; 32]),
            ))
            .body(Body::from("x".repeat(270337)))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            if enabled {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        );
        if enabled {
            for _ in 0..120 {
                let request = Request::post("/internal/v1/game-auth/character-commands/receipt")
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(
                        Identity::spki_sha256([1; 32]),
                    ))
                    .body(Body::from("{"))
                    .unwrap();
                assert_eq!(
                    app.clone().oneshot(request).await.unwrap().status(),
                    StatusCode::BAD_REQUEST
                );
            }
            for body in ["{".to_owned(), "x".repeat(270337)] {
                let request = Request::post("/internal/v1/game-auth/character-commands/receipt")
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(
                        Identity::spki_sha256([1; 32]),
                    ))
                    .body(Body::from(body))
                    .unwrap();
                assert_eq!(
                    app.clone().oneshot(request).await.unwrap().status(),
                    StatusCode::TOO_MANY_REQUESTS
                );
            }
        }
    }
}

pub(super) struct BrowserFixture {
    pub(super) pool: sqlx::PgPool,
    admin: sqlx::PgPool,
    schema: String,
    directory: std::path::PathBuf,
    pub(super) runtime: runtime::Runtime,
}
impl BrowserFixture {
    pub(super) async fn new() -> Self {
        Self::deposited_item(true).await
    }
    pub(super) async fn deposited_item(active: bool) -> Self {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("bazaar_browser_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |db, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(db)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
            .execute(&mut *tx)
            .await
            .unwrap();
        let a = authority();
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        let now = now as u64;
        for (who, email) in [
            (uid("11"), "plat3-seller@invalid.test"),
            (uid("13"), "plat3-buyer@invalid.test"),
        ] {
            sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'synthetic','active','en')").bind(who).bind(email).execute(&mut *tx).await.unwrap();
        }
        sqlx::query("UPDATE account_character_feed SET highest_epoch='3', watermark_epoch='3',source_authority=$1,complete_through=$2,observed_at=$2 WHERE id=1")
            .bind(&a.config.receipt_authority).bind(now as i64).execute(&mut *tx).await.unwrap();
        for (account, character) in [(uid("11"), uid("21")), (uid("13"), uid("14"))] {
            sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'3','1',$3,$4)")
                .bind(account).bind(&a.config.receipt_authority).bind(now as i64).bind([0_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
            sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Fixture','AVAILABLE')")
                .bind(account).bind(character).bind(uid("12")).execute(&mut *tx).await.unwrap();
        }
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',10000,false,now()+interval '2 days')").bind(uid("20")).bind(uid("11")).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO bazaar_item_details(id,definition_key,definition_revision,world_id,seller_character_id,item_instance_id,quantity) VALUES($1,'fixture-sword','v1',$2,$3,$4,1)").bind(uid("20")).bind(uid("12")).bind(uid("21")).bind(uid("22")).execute(&mut *tx).await.unwrap();
        let listing: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(uid("20"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert!(!a.escrow_ready(&mut tx, &listing).await.unwrap());
        let escrow = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
        store_intent(&mut tx, &escrow).await;
        let receipt = vector("escrow_receipt");
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            receipt.as_bytes(),
            now,
        )
        .await
        .unwrap();
        assert_eq!(
            audit(&a, &mut tx, &escrow, &receipt, now).await,
            ReceiptResult::Stored
        );
        let outbox = Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: false,
            },
        };
        let status = vector("delivered_status")
            .replace(
                "\"projection_revision\":\"2\"",
                "\"projection_revision\":\"1\"",
            )
            .replace("\"1790000002\"", &format!("\"{now}\""))
            .replace("DELIVERED", "ESCROWED")
            .replace("\"custody_revision\":\"2\"", "\"custody_revision\":\"1\"")
            .replace("000000000004", "000000000002");
        outbox
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                status.as_bytes(),
                now,
            )
            .await
            .unwrap();
        let policy = format!(
            "{{\"contract_version\":2,\"operation\":\"PublishBazaarTransferabilityV1\",\"source_authority\":\"oteryn:game:primary\",\"world_id\":\"{}\",\"projection_epoch\":\"3\",\"projection_revision\":\"4\",\"source_observed_at\":\"{now}\",\"eligibility_revision\":\"4\",\"definitions\":[{{\"definition\":{{\"family\":\"item\",\"production_key\":\"fixture-sword\",\"revision_ref\":\"v1\"}},\"tradeable\":true,\"max_quantity\":1,\"allowed_attribute_keys\":[\"tier\"]}}]}}",
            uid("12")
        );
        outbox
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                policy.as_bytes(),
                now,
            )
            .await
            .unwrap();
        let watermark = format!(
            "{{\"contract_version\":2,\"operation\":\"PublishBazaarWatermarkV1\",\"source_authority\":\"oteryn:game:primary\",\"projection_epoch\":\"3\",\"complete_through\":\"{now}\",\"observed_at\":\"{now}\"}}"
        );
        outbox
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                watermark.as_bytes(),
                now,
            )
            .await
            .unwrap();
        a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
        assert!(a.escrow_ready(&mut tx, &listing).await.unwrap());

        sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$4,7,$5,1,true,'browser-fixture')")
            .bind(listing.id).bind(uid("12")).bind(uid("21")).bind(uid("22")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1 AND $2")
            .bind(listing.id)
            .bind(active)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let directory = std::env::temp_dir().join(format!("bazaar-fixture-{}", Uuid::now_v7()));
        std::fs::create_dir(&directory).unwrap();
        let runtime = runtime::Runtime {
            pool: pool.clone(),
            config: std::sync::Arc::new(runtime::Config::fixture(a, &directory)),
        };
        Self {
            pool,
            admin,
            schema,
            directory,
            runtime,
        }
    }
    pub(super) async fn end(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
        std::fs::remove_dir_all(self.directory).unwrap();
    }
}

impl BrowserFixture {
    async fn get(&self, path: &str) -> (axum::http::StatusCode, String) {
        use tower::ServiceExt;
        let response = self
            .runtime
            .browser_router()
            .oneshot(
                axum::http::Request::builder()
                    .uri(path)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }
    async fn bid(&self, maximum: i64) -> axum::http::StatusCode {
        use tower::ServiceExt;
        let token = "b".repeat(64);
        let csrf = "c".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day') ON CONFLICT(id_hash) DO NOTHING")
            .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(uid("13")).bind(&csrf).execute(&self.pool).await.unwrap();
        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/en/account/bazaar/{}/bids", uid("20")))
            .header(
                "cookie",
                format!("{}={token}", oteryn_web::session::COOKIE_NAME),
            )
            .header("origin", "https://portal.test")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(format!(
                "_csrf={csrf}&request_id={}&maximum={maximum}&buyer_character_id={}",
                Uuid::now_v7(),
                uid("14")
            )))
            .unwrap();
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                12345,
            ))));
        self.runtime
            .browser_router()
            .oneshot(request)
            .await
            .unwrap()
            .status()
    }
}
#[tokio::test]
async fn postgres_browser_typed_bans_unknown_states_and_lifted_expired_bids() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    let seed = Uuid::now_v7();
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
        .bind(uid("13"))
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,20000)",
    )
    .bind(seed)
    .bind(uid("13"))
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,20000,0)").bind(Uuid::now_v7()).bind(seed).bind(uid("13")).execute(&db.pool).await.unwrap();
    let ban = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES($1,$2,'ban')",
    )
    .bind(ban)
    .bind(uid("13"))
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at) VALUES($1,1,$2,'{}',true,now())").bind(uid("13")).bind(ban).execute(&db.pool).await.unwrap();
    assert_eq!(db.bid(10000).await, axum::http::StatusCode::FORBIDDEN);
    let reserved: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(uid("13"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(reserved, 0);
    sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at) VALUES($1,2,$2,'{}',false,now())").bind(uid("13")).bind(ban).execute(&db.pool).await.unwrap();
    assert_eq!(db.bid(10000).await, axum::http::StatusCode::SEE_OTHER);
    sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) VALUES($1,3,$2,'{}',true,now()-interval '2 days',now()-interval '1 day')").bind(uid("13")).bind(ban).execute(&db.pool).await.unwrap();
    assert_eq!(db.bid(11000).await, axum::http::StatusCode::SEE_OTHER);
    // Future legacy states must fail closed even before the application understands them.
    sqlx::raw_sql("ALTER TABLE sanctions DROP CONSTRAINT sanctions_category_check; ALTER TABLE sanctions DROP CONSTRAINT sanctions_status_check").execute(&db.pool).await.unwrap();
    for (category, status) in [("future", "pending"), ("warning", "future")] {
        let sanction = Uuid::now_v7();
        sqlx::query("INSERT INTO sanctions(id,account_id,category,status,public_reason,effective_at,created_by) VALUES($1,$2,$3,$4,'test future state',now(),$5)").bind(sanction).bind(uid("13")).bind(category).bind(status).bind(uid("11")).execute(&db.pool).await.unwrap();
        assert_eq!(db.bid(12000).await, axum::http::StatusCode::FORBIDDEN);
        sqlx::query("DELETE FROM sanctions WHERE id=$1")
            .bind(sanction)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    db.end().await;
    eprintln!(
        "postgres_browser_typed_bans_unknown_states_and_lifted_expired_bids: {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn postgres_browser_approved_item_attributes_quantity_and_deadline() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    for path in [
        format!("/en/bazaar/{}", uid("20")),
        format!("/en/bazaar/items/{}", uid("20")),
    ] {
        let (status, body) = db.get(&path).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(body.contains("fixture-sword"));
        assert!(body.contains("<dt>Tier</dt><dd>2</dd>"));
        assert!(body.contains("<dt>Quantity</dt><dd>1</dd>"));
        assert!(body.contains("Deadline:"));
    }
    db.end().await;
    eprintln!(
        "postgres_browser_approved_item_attributes_quantity_and_deadline: {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn postgres_browser_unavailable_catalogue_has_bounded_work_and_continuation() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    for _ in 0..110 {
        let listing = Uuid::now_v7();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'active',now()+interval '1 day')").bind(listing).bind(uid("11")).execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$1,7,$4,1,true,'unavailable')").bind(listing).bind(uid("12")).bind(uid("21")).bind([1_u8;32].as_slice()).execute(&db.pool).await.unwrap();
    }
    let issuer = async {
        let mut tx = db.pool.begin().await.unwrap();
        tokio::time::timeout(
            std::time::Duration::from_millis(500),
            db.runtime.authority().lock(&mut tx),
        )
        .await
        .unwrap()
        .unwrap();
        tx.rollback().await.unwrap();
    };
    let ((status, body), ()) = tokio::join!(db.get("/en/bazaar"), issuer);
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(!body.contains("fixture-sword"));
    assert!(body.contains("rel=\"next\""));
    let before = body
        .split("?before=")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let (_, second) = db.get(&format!("/en/bazaar?before={before}")).await;
    assert!(!second.contains("fixture-sword"));
    let before = second
        .split("?before=")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let (_, third) = db.get(&format!("/en/bazaar?before={before}")).await;
    assert!(third.contains("fixture-sword"));
    let mut tx = db.pool.begin().await.unwrap();
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        db.runtime.authority().lock(&mut tx),
    )
    .await
    .unwrap()
    .unwrap();
    tx.rollback().await.unwrap();
    db.end().await;
    eprintln!(
        "postgres_browser_unavailable_catalogue_has_bounded_work_and_continuation: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_expired_gold_release_restarts_with_audited_immutable_attempt() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    gold_release_retry_scenario(false).await;
}
#[tokio::test]
async fn postgres_accepted_bid_cleanup_retains_admission_after_release_retry() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    gold_release_retry_scenario(true).await;
}
async fn gold_release_retry_scenario(accepted: bool) {
    use crate::Listing;
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    let request = Uuid::now_v7();
    let hold = Uuid::now_v7();
    let mut tx = db.pool.begin().await.unwrap();
    let hold_operation = gold_fact(&mut tx, Command::Hold, 10, request, hold, (0, 1, 12000)).await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=10")
        .execute(&mut *tx)
        .await
        .unwrap();
    let listing: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    // A historical dispatch expires before Game executes it; replacement uses
    // today's clock throughout, including the eventual committed receipt.
    let game_now = runtime::now(&mut tx).await.unwrap();
    let original = Uuid::now_v7();
    let Value::Object(common) = decode(vector("escrow_intent").as_bytes(), 4096).unwrap() else {
        panic!("intent")
    };
    let mut fields = common[..10].to_vec();
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "command" => Value::String(Command::Release.wire().into()),
            "operation_id" | "issuer_decision_id" => Value::String(original.to_string()),
            "source_revision" => Value::String("11".into()),
            _ => value.clone(),
        };
    }
    for (key, value) in [
        ("buyer_account_id", uid("13").to_string()),
        ("hold_id", hold.to_string()),
        ("expected_hold_revision", "1".into()),
        ("release_reason", "EXCESS".into()),
        ("amount", "12000".into()),
        ("issued_at_source", (game_now - 301).to_string()),
        ("expires_at_source", (game_now - 1).to_string()),
    ] {
        fields.push((key.into(), Value::String(value)));
    }
    fields.push(common.last().unwrap().clone());
    let historical = PreparedIntent::parse(
        &String::from_utf8(canonical::encode(&Value::Object(fields)).unwrap()).unwrap(),
    )
    .unwrap();
    store_intent_with_key(
        &mut tx,
        &historical,
        &format!("bazaar:{}:ReleaseBazaarGold:{request}:1", listing.id),
    )
    .await;
    sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,3,3)").bind(original).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=11")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id,phase,release_operation_id,release_accepted) VALUES($1,$2,$3,$4,12000,$5,'releasing',$6,$7)")
        .bind(request).bind(listing.id).bind(uid("13")).bind(uid("14")).bind(hold_operation).bind(original).bind(accepted).execute(&mut *tx).await.unwrap();
    let raw: String =
        sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(original)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let intent = PreparedIntent::parse(&raw).unwrap();
    // Game executes after the original 300-second deadline.
    assert!(number(&intent.value, "expires_at_source").unwrap() < game_now);
    let rejection = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"ReleaseBazaarGold\",\"operation_id\":\"{original}\",\"outcome\":\"REJECTED\",\"result_code\":\"CHAR_CMD_INTENT_EXPIRED\",\"result_data\":null,\"recovery_generation\":\"3\",\"decided_at\":\"{game_now}\"}}"
    );
    db.runtime
        .authority()
        .receive(
            &mut tx,
            &authority().config.receipt_authority,
            rejection.as_bytes(),
            game_now,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    // A receipt without the matching audit cannot authorize replacement.
    db.runtime.tick_gold_bids().await.unwrap();
    let current: Uuid = sqlx::query_scalar(
        "SELECT release_operation_id FROM bazaar_authority_gold_bids WHERE request_id=$1",
    )
    .bind(request)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(current, original);
    let mut tx = db.pool.begin().await.unwrap();
    audit(
        db.runtime.authority(),
        &mut tx,
        &intent,
        &rejection,
        game_now,
    )
    .await;
    tx.commit().await.unwrap();
    let restarted = db.runtime.clone();
    restarted.tick_gold_bids().await.unwrap();
    let replacement: Uuid = sqlx::query_scalar(
        "SELECT release_operation_id FROM bazaar_authority_gold_bids WHERE request_id=$1",
    )
    .bind(request)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_ne!(replacement, original);
    let mut tx = db.pool.begin().await.unwrap();
    let original_retained = db
        .runtime
        .authority()
        .retained(&mut tx, original)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(original_retained.0.raw, raw);
    assert_eq!(original_retained.1.raw, rejection);
    let raw: String =
        sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(replacement)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let replacement_intent = PreparedIntent::parse(&raw).unwrap();
    assert!(
        number(&replacement_intent.value, "source_revision").unwrap()
            > number(&intent.value, "source_revision").unwrap()
    );
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"ReleaseBazaarGold\",\"operation_id\":\"{replacement}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"result_data\":{{\"hold_id\":\"{hold}\",\"hold_revision\":\"2\",\"released_amount\":\"12000\",\"remaining_held\":\"0\",\"hold_state\":\"RELEASED\"}},\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}"
    );
    db.runtime
        .authority()
        .receive(
            &mut tx,
            &authority().config.receipt_authority,
            receipt.as_bytes(),
            now,
        )
        .await
        .unwrap();
    audit(
        db.runtime.authority(),
        &mut tx,
        &replacement_intent,
        &receipt,
        now,
    )
    .await;
    tx.commit().await.unwrap();
    restarted.tick_gold_bids().await.unwrap();
    restarted.tick_gold_bids().await.unwrap();
    let phase: String =
        sqlx::query_scalar("SELECT phase FROM bazaar_authority_gold_bids WHERE request_id=$1")
            .bind(request)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(phase, if accepted { "accepted" } else { "refused" });
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        runtime::gold::current_hold(db.runtime.authority(), &mut tx, listing.id, uid("13"), None)
            .await
            .unwrap()
            .unwrap()
            .amount,
        0
    );
    tx.rollback().await.unwrap();
    db.end().await;
    eprintln!(
        "{}: {:?}",
        if accepted {
            "postgres_accepted_bid_cleanup_retains_admission_after_release_retry"
        } else {
            "postgres_expired_gold_release_restarts_with_audited_immutable_attempt"
        },
        started.elapsed()
    );
}

fn character_intent(
    command: Command,
    revision: u64,
    now: u64,
    extra: Vec<(&str, Value)>,
) -> PreparedIntent {
    let Value::Object(common) = decode(vector("escrow_intent").as_bytes(), 4096).unwrap() else {
        panic!("object")
    };
    let mut fields = common[..10].to_vec();
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "command" => Value::String(command.wire().into()),
            "listing_id" => Value::String(uid("30").to_string()),
            "operation_id" | "issuer_decision_id" => Value::String(Uuid::now_v7().to_string()),
            "source_revision" => Value::String(revision.to_string()),
            _ => value.clone(),
        };
    }
    fields.extend(extra.into_iter().map(|(k, v)| (k.into(), v)));
    fields.extend([
        ("issued_at_source".into(), Value::String(now.to_string())),
        (
            "expires_at_source".into(),
            Value::String((now + 300).to_string()),
        ),
        common.last().unwrap().clone(),
    ]);
    PreparedIntent::parse(
        &String::from_utf8(canonical::encode(&Value::Object(fields)).unwrap()).unwrap(),
    )
    .unwrap()
}
async fn committed_fact(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    intent: &PreparedIntent,
    data: &str,
    now: u64,
) {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(id(text(&intent.value, "operation_id").unwrap()).unwrap())
            .fetch_optional(&mut **tx)
            .await
            .unwrap();
    if let Some(existing) = existing {
        assert_eq!(existing.as_bytes(), intent.bytes());
    } else {
        if intent.command() == Command::Bind {
            store_intent_with_key(
                tx,
                intent,
                &format!(
                    "bazaar:{}:{}:{}:1",
                    text(&intent.value, "listing_id").unwrap(),
                    intent.command().wire(),
                    text(&intent.value, "transfer_operation_id").unwrap()
                ),
            )
            .await;
        } else {
            store_intent(tx, intent).await;
        }
    }
    let operation = text(&intent.value, "operation_id").unwrap();
    let receipt = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"{}\",\"operation_id\":\"{operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"result_data\":{data},\"recovery_generation\":\"{}\",\"decided_at\":\"{now}\"}}",
        intent.command().wire(),
        a.fence.recovery_generation
    );
    assert_eq!(
        a.receive(tx, &a.config.receipt_authority, receipt.as_bytes(), now)
            .await
            .unwrap_or_else(|error| panic!("{:?} receipt: {error}", intent.command())),
        ReceiptResult::Stored
    );
    assert_eq!(
        audit(a, tx, intent, &receipt, now).await,
        ReceiptResult::Stored
    );
}
#[tokio::test]
async fn postgres_browser_verified_character_snapshot_facts() {
    character_snapshot_browser_scenario(false, &[], false).await;
}

#[tokio::test]
async fn postgres_browser_zero_magic_skill_snapshot() {
    character_snapshot_browser_scenario(true, &[], false).await;
}

#[tokio::test]
async fn postgres_browser_preserves_signed_character_blessings() {
    character_snapshot_browser_scenario(false, &["blessing:earth", "blessing:fire"], false).await;
}

#[tokio::test]
async fn recovered_character_does_not_hide_catalogue() {
    character_snapshot_browser_scenario(false, &[], true).await;
}

async fn character_snapshot_browser_scenario(zero_magic: bool, blessings: &[&str], recover: bool) {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let mut db = BrowserFixture::new().await;
    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .escrow_account_id = Some(uid("40"));
    let mut signed_snapshot = vector("locked_character_snapshot");
    if zero_magic || !blessings.is_empty() {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        use ed25519_dalek::Signer;
        let signing = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        std::sync::Arc::make_mut(&mut db.runtime.config)
            .authority
            .config
            .snapshot_keys = vec![("fixture-example-1".into(), signing.verifying_key())];
        let original = decode(signed_snapshot.as_bytes(), 262144).unwrap();
        let payload =
            String::from_utf8(canonical::encode(original.get("snapshot").unwrap()).unwrap())
                .unwrap();
        let payload = if zero_magic {
            payload.replace(
                "\"progress_bps\":5000}]",
                "\"progress_bps\":5000},{\"key\":\"magic\",\"level\":\"0\",\"progress_bps\":0}]",
            )
        } else {
            let blessings = serde_json::to_string(blessings).unwrap();
            payload.replace("\"blessings\":[]", &format!("\"blessings\":{blessings}"))
        };
        let message = format!(
            "oteryn-v2-bazaar-character-snapshot-v1.{}",
            URL_SAFE_NO_PAD.encode(payload.as_bytes())
        );
        let signature = URL_SAFE_NO_PAD.encode(signing.sign(message.as_bytes()).to_bytes());
        signed_snapshot = format!(
            "{{\"snapshot\":{payload},\"snapshot_hash\":\"{}\",\"alg\":\"Ed25519\",\"kid\":\"fixture-example-1\",\"signature\":\"{signature}\"}}",
            digest(payload.as_bytes())
        );
    }
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let verified = verify_snapshot(
        signed_snapshot.as_bytes(),
        &SnapshotBinding {
            source_authority: &a.config.projection_authority,
            listing: uid("30"),
            seller: uid("11"),
            character: uid("31"),
            world: uid("12"),
            lock: uid("32"),
            lock_revision: 1,
            character_revision: 7,
            eligibility_revision: 4,
            projection_epoch: 3,
            now,
        },
        "fixture-example-1",
        &a.config.snapshot_keys[0].1,
    );
    assert!(
        verified.is_ok(),
        "signed character snapshot must be accepted: {verified:?}"
    );
    let listing = uid("30");
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'character','auction',$3::text::bazaar_currency,100,false,'active',now()+interval '1 day')").bind(listing).bind(uid("11")).bind(if recover {"gold"} else {"coins"}).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$3,7,$4,1,true,'character-browser')").bind(listing).bind(uid("12")).bind(uid("31")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    let strval = |value: Uuid| Value::String(value.to_string());
    let lock = character_intent(
        Command::Lock,
        101,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("expected_character_revision", Value::String("7".into())),
            ("eligibility_revision", Value::String("4".into())),
            ("snapshot_schema_version", Value::Unsigned(1)),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"1\",\"eligibility_revision\":\"4\",\"signed_snapshot\":{}}}",
        uid("31"),
        uid("32"),
        signed_snapshot
    );
    committed_fact(a, &mut tx, &lock, &data, now).await;
    let operation = Uuid::now_v7();
    let Value::Object(mut fields) = transfer_fixture(operation, 102, now) else {
        panic!("transfer")
    };
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "expected_from_account_id" => strval(uid("11")),
            "to_account_id" => strval(uid("40")),
            "purpose" => Value::String("BAZAAR_LISTING_ESCROW".into()),
            _ => value.clone(),
        };
    }
    let transfer = Value::Object(fields);
    stage_transfer_fixture(&mut tx, listing, &transfer).await;
    let raw = canonical::encode(&transfer).unwrap();
    runtime::worker::publish_transfer(
        &mut tx,
        listing,
        "BAZAAR_LISTING_ESCROW",
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    let receipt = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"TransferCharacterOwnership\",\"operation_id\":\"{operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"character_id\":\"{}\",\"from_account_id\":\"{}\",\"to_account_id\":\"{}\",\"character_revision\":\"8\",\"projection_epoch\":\"3\",\"from_projection_revision\":\"5\",\"to_projection_revision\":\"6\",\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}",
        uid("31"),
        uid("11"),
        uid("40")
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(operation)
        .bind(receipt)
        .execute(&mut *tx)
        .await
        .unwrap();
    let bind = character_intent(
        Command::Bind,
        103,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("lock_id", strval(uid("32"))),
            ("expected_lock_revision", Value::String("1".into())),
            ("transfer_operation_id", strval(operation)),
            ("transfer_binding_digest", Value::String(digest(&raw))),
            ("purpose", Value::String("BAZAAR_LISTING_ESCROW".into())),
            ("sale_id", Value::Null),
            ("funding", Value::Null),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"2\",\"transfer_operation_id\":\"{operation}\",\"transfer_binding_digest\":\"{}\"}}",
        uid("31"),
        uid("32"),
        digest(&raw)
    );
    committed_fact(a, &mut tx, &bind, &data, now).await;
    let snapshot = decode(signed_snapshot.as_bytes(), 262144).unwrap();
    let hash = text(&snapshot, "snapshot_hash").unwrap();
    let Value::Object(mut status) = decode(vector("delivered_status").as_bytes(), 4096).unwrap()
    else {
        panic!("status")
    };
    for (name, value) in &mut status {
        *value = match name.as_str() {
            "listing_id" => strval(listing),
            "asset_kind" => Value::String("CHARACTER".into()),
            "asset_id" => strval(uid("31")),
            "state" => Value::String("locked".into()),
            "custody_id" => strval(uid("32")),
            "snapshot_hash" => Value::String(hash.into()),
            "last_operation_id" => strval(operation),
            "source_observed_at" => Value::String(now.to_string()),
            _ => value.clone(),
        };
    }
    let status = String::from_utf8(canonical::encode(&Value::Object(status)).unwrap()).unwrap();
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        &mut tx,
        &a.config.projection_authority,
        status.as_bytes(),
        now,
    )
    .await
    .unwrap();
    if zero_magic || !blessings.is_empty() {
        use crate::characters::CharacterEvidence;
        let row: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(listing)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        let approved = a.approved(&mut tx, &row).await;
        assert!(
            approved.is_ok(),
            "signed character snapshot must be approved: {approved:?}"
        );
        let approved = approved.unwrap().unwrap();
        if zero_magic {
            assert_eq!(
                approved
                    .skills
                    .iter()
                    .find(|s| s.key == "magic")
                    .unwrap()
                    .level,
                0
            );
        } else {
            assert_eq!(approved.blessings, blessings);
        }
    }
    tx.commit().await.unwrap();
    let (status, catalogue) = db.get("/en/bazaar").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(catalogue.contains("Aldric"));
    let (status, body) = db.get(&format!("/en/bazaar/{listing}")).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    for fact in [
        "Aldric",
        "<dt>Level</dt><dd>100</dd>",
        "distance",
        "distance: 75",
        hash,
        "1790000002",
        "Deadline:",
    ] {
        assert!(body.contains(fact), "missing {fact}");
    }
    if zero_magic {
        assert!(body.contains("magic: 0"));
    }
    if recover {
        recovery_tests::recover_character(&mut db).await;
        let (status, catalogue) = db.get("/en/bazaar").await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(catalogue.contains("fixture-sword"));
        assert!(!catalogue.contains("Aldric"));
    }
    db.end().await;
    eprintln!(
        "postgres_browser_verified_character_snapshot_facts: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn l6_04_suspended_leader_can_be_outbid_and_exact_hold_released() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return;
    };
    let admin = sqlx::PgPool::connect(&url).await.unwrap();
    let schema = format!("l6_outbid_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |db, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&mut *tx)
        .await
        .unwrap();
    let [seller, first, second, listing] = [
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    ];
    for account in [seller, first, second] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$1::text||'@example.invalid','test','active',now(),'en')").bind(account).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
            .bind(account)
            .execute(&mut *tx)
            .await
            .unwrap();
        let key = Uuid::now_v7();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,1000)").bind(key).bind(account).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,1000,0)").bind(Uuid::now_v7()).bind(key).bind(account).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,now()+interval '2 days')").bind(listing).bind(seller).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
        .bind(listing)
        .execute(&mut *tx)
        .await
        .unwrap();
    let bazaar = crate::Bazaar::new(
        pool.clone(),
        PlatformAccounts {
            enabled: true,
            session_hash: None,
        },
        CoinReservations {
            authority: authority(),
        },
        authority(),
        crate::Flags {
            enabled: true,
            gold_enabled: false,
        },
    );
    bazaar
        .bid_in(&mut tx, listing, first, 200, "first")
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(first)
        .execute(&mut *tx)
        .await
        .unwrap();
    let outcome = bazaar
        .bid_in(&mut tx, listing, second, 300, "second")
        .await
        .unwrap();
    assert_eq!(outcome.leader, second);
    let balances:(i64,i64)=sqlx::query_as("SELECT sum(available_delta)::bigint,sum(reserved_delta)::bigint FROM wallet_entries WHERE account_id=$1").bind(first).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(balances, (1000, 0));
    let hold: i64 =
        sqlx::query_scalar("SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2")
            .bind(listing)
            .bind(first)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(hold, 0);
    bazaar
        .bid_in(&mut tx, listing, second, 300, "second")
        .await
        .unwrap();
    let releases:i64=sqlx::query_scalar("SELECT count(*) FROM wallet_operations WHERE kind='release' AND account_id=$1 AND reference_id=$2").bind(first).bind(listing).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(releases, 1);
    tx.rollback().await.unwrap();
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    eprintln!(
        "PG l6_04_suspended_leader_can_be_outbid_and_exact_hold_released {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

async fn returned_character_fixture(currency: &str) -> BrowserFixture {
    returned_character_in_state(currency, "active").await
}
async fn returned_character_in_state(currency: &str, state: &str) -> BrowserFixture {
    let mut db = BrowserFixture::new().await;
    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .escrow_account_id = Some(uid("40"));
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let listing = uid("30");
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'character','auction',$3::text::bazaar_currency,100,false,$4::text::bazaar_state,now()-interval '1 second')").bind(listing).bind(uid("11")).bind(currency).bind(state).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$3,7,$4,1,true,'character-browser')").bind(listing).bind(uid("12")).bind(uid("31")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    let strval = |value: Uuid| Value::String(value.to_string());
    let lock = character_intent(
        Command::Lock,
        101,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("expected_character_revision", Value::String("7".into())),
            ("eligibility_revision", Value::String("4".into())),
            ("snapshot_schema_version", Value::Unsigned(1)),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"1\",\"eligibility_revision\":\"4\",\"signed_snapshot\":{}}}",
        uid("31"),
        uid("32"),
        vector("locked_character_snapshot")
    );
    committed_fact(a, &mut tx, &lock, &data, now).await;
    let operation = Uuid::now_v7();
    let Value::Object(mut fields) = transfer_fixture(operation, 102, now) else {
        panic!("transfer")
    };
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "expected_from_account_id" => strval(uid("40")),
            "to_account_id" => strval(uid("11")),
            "purpose" => Value::String("BAZAAR_ESCROW_RETURN".into()),
            _ => value.clone(),
        };
    }
    let transfer = Value::Object(fields);
    stage_transfer_fixture(&mut tx, listing, &transfer).await;
    let raw = canonical::encode(&transfer).unwrap();
    runtime::worker::publish_transfer(
        &mut tx,
        listing,
        "BAZAAR_ESCROW_RETURN",
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    let receipt = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"TransferCharacterOwnership\",\"operation_id\":\"{operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"character_id\":\"{}\",\"from_account_id\":\"{}\",\"to_account_id\":\"{}\",\"character_revision\":\"8\",\"projection_epoch\":\"3\",\"from_projection_revision\":\"5\",\"to_projection_revision\":\"6\",\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}",
        uid("31"),
        uid("40"),
        uid("11")
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(operation)
        .bind(receipt)
        .execute(&mut *tx)
        .await
        .unwrap();
    let bind = character_intent(
        Command::Bind,
        103,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("lock_id", strval(uid("32"))),
            ("expected_lock_revision", Value::String("1".into())),
            ("transfer_operation_id", strval(operation)),
            ("transfer_binding_digest", Value::String(digest(&raw))),
            ("purpose", Value::String("BAZAAR_ESCROW_RETURN".into())),
            ("sale_id", Value::Null),
            ("funding", Value::Null),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"2\",\"transfer_operation_id\":\"{operation}\",\"transfer_binding_digest\":\"{}\"}}",
        uid("31"),
        uid("32"),
        digest(&raw)
    );
    committed_fact(a, &mut tx, &bind, &data, now).await;
    let snapshot = decode(vector("locked_character_snapshot").as_bytes(), 262144).unwrap();
    let hash = text(&snapshot, "snapshot_hash").unwrap();
    let Value::Object(mut status) = decode(vector("delivered_status").as_bytes(), 4096).unwrap()
    else {
        panic!("status")
    };
    for (name, value) in &mut status {
        *value = match name.as_str() {
            "listing_id" => strval(listing),
            "asset_kind" => Value::String("CHARACTER".into()),
            "asset_id" => strval(uid("31")),
            "state" => Value::String("RETURNED".into()),
            "custody_id" => strval(uid("32")),
            "snapshot_hash" => Value::String(hash.into()),
            "last_operation_id" => strval(operation),
            "source_observed_at" => Value::String(now.to_string()),
            _ => value.clone(),
        };
    }
    let status = String::from_utf8(canonical::encode(&Value::Object(status)).unwrap()).unwrap();
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        &mut tx,
        &a.config.projection_authority,
        status.as_bytes(),
        now,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=103")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    db
}

async fn reject_expired(db: &BrowserFixture, intent: &PreparedIntent) {
    reject(db, intent, "CHAR_CMD_INTENT_EXPIRED").await;
}
async fn reject(db: &BrowserFixture, intent: &PreparedIntent, code: &str) {
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let operation = text(&intent.value, "operation_id").unwrap();
    let receipt = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"{}","operation_id":"{operation}","outcome":"REJECTED","result_code":"{code}","result_data":null,"recovery_generation":"3","decided_at":"{now}"}}"#,
        intent.command().wire()
    );
    assert_eq!(
        db.runtime
            .authority()
            .receive(
                &mut tx,
                &authority().config.receipt_authority,
                receipt.as_bytes(),
                now
            )
            .await
            .unwrap(),
        ReceiptResult::Stored
    );
    assert_eq!(
        audit(db.runtime.authority(), &mut tx, intent, &receipt, now).await,
        ReceiptResult::Stored
    );
    tx.commit().await.unwrap();
}
async fn newest_intent(db: &BrowserFixture, listing: Uuid, command: Command) -> PreparedIntent {
    let raw: String = sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE listing_id=$1 AND command=$2 ORDER BY source_revision DESC LIMIT 1")
        .bind(listing).bind(command.wire()).fetch_one(&db.pool).await.unwrap();
    PreparedIntent::parse(&raw).unwrap()
}
#[tokio::test]
async fn postgres_cancelled_character_unlock_retries_both_currencies() {
    let started = std::time::Instant::now();
    for currency in ["coins", "gold"] {
        let db = returned_character_fixture(currency).await;
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let historical = character_intent(
            Command::Unlock,
            104,
            now - 301,
            vec![
                ("character_id", Value::String(uid("31").to_string())),
                ("lock_id", Value::String(uid("32").to_string())),
                ("expected_lock_revision", Value::String("2".into())),
            ],
        );
        store_intent_with_key(
            &mut tx,
            &historical,
            &format!("bazaar:{}:UnlockBazaarCharacter:{}:1", uid("30"), uid("30")),
        )
        .await;
        sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,3,3)")
            .bind(id(text(&historical.value,"operation_id").unwrap()).unwrap()).execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE bazaar_game_issuer SET source_revision=104")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
        let state: crate::State =
            sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
                .bind(uid("30"))
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(state, crate::State::Cancelled);
        let first = newest_intent(&db, uid("30"), Command::Unlock).await;
        reject_expired(&db, &first).await;
        db.runtime.tick().await.unwrap();
        let replacement = newest_intent(&db, uid("30"), Command::Unlock).await;
        assert_ne!(first.bytes(), replacement.bytes());
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let data = format!(
            r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"3","current_account_id":"{}"}}"#,
            uid("31"),
            uid("32"),
            uid("11")
        );
        committed_fact(db.runtime.authority(), &mut tx, &replacement, &data, now).await;
        tx.commit().await.unwrap();
        assert!(
            !runtime::worker::listing_batch(&db.pool, 3)
                .await
                .unwrap()
                .contains(&uid("30"))
        );
        // An audit conflict revokes completion and puts the listing back in cleanup.
        sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'RECEIPT')")
            .bind(id(text(&replacement.value,"operation_id").unwrap()).unwrap()).bind("a".repeat(64)).execute(&db.pool).await.unwrap();
        assert!(
            runtime::worker::listing_batch(&db.pool, 3)
                .await
                .unwrap()
                .contains(&uid("30"))
        );
        db.end().await;
    }
    eprintln!(
        "postgres_cancelled_character_unlock_retries_both_currencies: {:?}",
        started.elapsed()
    );
}

async fn character_status(
    db: &BrowserFixture,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &str,
    last_operation: Uuid,
    revisions: (&str, &str),
    now: u64,
) {
    let a = db.runtime.authority();
    let snapshot = decode(vector("locked_character_snapshot").as_bytes(), 262144).unwrap();
    let hash = text(&snapshot, "snapshot_hash").unwrap();
    let Value::Object(mut status) = decode(vector("delivered_status").as_bytes(), 4096).unwrap()
    else {
        panic!("status")
    };
    for (name, value) in &mut status {
        *value = match name.as_str() {
            "listing_id" => Value::String(uid("30").to_string()),
            "projection_epoch" => Value::String(a.fence.projection_epoch.to_string()),
            "projection_revision" => Value::String(revisions.0.into()),
            "asset_kind" => Value::String("CHARACTER".into()),
            "asset_id" => Value::String(uid("31").to_string()),
            "state" => Value::String(state.into()),
            "custody_id" => Value::String(uid("32").to_string()),
            "custody_revision" => Value::String(revisions.1.into()),
            "snapshot_hash" => Value::String(hash.into()),
            "last_operation_id" if last_operation.is_nil() => Value::Null,
            "last_operation_id" => Value::String(last_operation.to_string()),
            "source_observed_at" => Value::String(now.to_string()),
            _ => value.clone(),
        };
    }
    let status = String::from_utf8(canonical::encode(&Value::Object(status)).unwrap()).unwrap();
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(tx, &a.config.projection_authority, status.as_bytes(), now)
    .await
    .unwrap();
}
async fn character_transfer_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: Uuid,
    (from, to): (Uuid, Uuid),
    now: u64,
) {
    let (epoch, generation): (String, String) = sqlx::query_as(
        "SELECT projection_epoch::text,recovery_generation::text FROM bazaar_game_issuer",
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();
    let receipt = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"TransferCharacterOwnership\",\"operation_id\":\"{operation}\",\"outcome\":\"COMMITTED\",\"result_code\":\"CHAR_CMD_OK\",\"character_id\":\"{}\",\"from_account_id\":\"{from}\",\"to_account_id\":\"{to}\",\"character_revision\":\"8\",\"projection_epoch\":\"{epoch}\",\"from_projection_revision\":\"5\",\"to_projection_revision\":\"6\",\"recovery_generation\":\"{generation}\",\"decided_at\":\"{now}\"}}",
        uid("31")
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(operation)
        .bind(receipt)
        .execute(&mut **tx)
        .await
        .unwrap();
}
/// A sold Character in escrow (uid 40) awaiting its settlement leg, funded in
/// `currency` by buyer uid 13 for 100 under sale uid 33.
async fn escrow_character(
    db: &BrowserFixture,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    listing: Uuid,
    now: u64,
) {
    let a = db.runtime.authority();
    let strval = |value: Uuid| Value::String(value.to_string());
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();
    let operation = Uuid::now_v7();
    let Value::Object(mut fields) = transfer_fixture(operation, revision, now) else {
        panic!("transfer")
    };
    for (name, value) in &mut fields {
        *value = match name.as_str() {
            "expected_from_account_id" => strval(uid("11")),
            "to_account_id" => strval(uid("40")),
            "purpose" => Value::String("BAZAAR_LISTING_ESCROW".into()),
            _ => value.clone(),
        };
    }
    let transfer = Value::Object(fields);
    stage_transfer_fixture(tx, listing, &transfer).await;
    let raw = canonical::encode(&transfer).unwrap();
    runtime::worker::publish_transfer(
        tx,
        listing,
        "BAZAAR_LISTING_ESCROW",
        operation,
        &transfer,
        &raw,
    )
    .await
    .unwrap();
    character_transfer_receipt(tx, operation, (uid("11"), uid("40")), now).await;
    let bind = character_intent(
        Command::Bind,
        103,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("lock_id", strval(uid("32"))),
            ("expected_lock_revision", Value::String("1".into())),
            ("transfer_operation_id", strval(operation)),
            ("transfer_binding_digest", Value::String(digest(&raw))),
            ("purpose", Value::String("BAZAAR_LISTING_ESCROW".into())),
            ("sale_id", Value::Null),
            ("funding", Value::Null),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"2\",\"transfer_operation_id\":\"{operation}\",\"transfer_binding_digest\":\"{}\"}}",
        uid("31"),
        uid("32"),
        digest(&raw)
    );
    committed_fact(a, tx, &bind, &data, now).await;
    character_status(db, tx, "locked", operation, ("2", "2"), now).await;
}
async fn commit_character_lock(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    now: u64,
) {
    let strval = |value: Uuid| Value::String(value.to_string());
    let lock = character_intent(
        Command::Lock,
        101,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("expected_character_revision", Value::String("7".into())),
            ("eligibility_revision", Value::String("4".into())),
            ("snapshot_schema_version", Value::Unsigned(1)),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"1\",\"eligibility_revision\":\"4\",\"signed_snapshot\":{}}}",
        uid("31"),
        uid("32"),
        vector("locked_character_snapshot")
    );
    committed_fact(a, tx, &lock, &data, now).await;
}
async fn settling_character_fixture(currency: &str) -> BrowserFixture {
    use crate::{Currency, Reservation, Reservations, Settlement};
    let mut db = BrowserFixture::new().await;
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.authority.config.escrow_account_id = Some(uid("40"));
    config.escrow_account = Some(uid("40"));
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let listing = uid("30");
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at,leader,leader_max,reservation) VALUES($1,$2,'character','auction',$3::text::bazaar_currency,100,false,'active',now()-interval '1 second',$4,100,$1)").bind(listing).bind(uid("11")).bind(currency).bind(uid("13")).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery' WHERE id=$1")
        .bind(listing)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$3,7,$4,1,true,'character-settlement')").bind(listing).bind(uid("12")).bind(uid("31")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    commit_character_lock(a, &mut tx, now).await;
    let strval = |value: Uuid| Value::String(value.to_string());
    escrow_character(&db, &mut tx, listing, now).await;
    let funds = CoinReservations {
        authority: a.clone(),
    };
    if currency == "coins" {
        funds.lock_accounts(&mut tx, &[uid("13")]).await.unwrap();
        let op = Uuid::now_v7();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,100)").bind(op).bind(uid("13")).execute(&mut *tx).await.unwrap();
        oteryn_wallet::credit_in(&mut tx, op, uid("13"), 100)
            .await
            .unwrap();
        funds
            .reserve(
                &mut tx,
                Reservation {
                    id: listing,
                    account: uid("13"),
                    currency: Currency::Coins,
                    amount: 100,
                },
            )
            .await
            .unwrap();
        sqlx::query("INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key) VALUES($1,$2,$3,$4,'coins',100,10,90,$2,'bind-refusal')").bind(uid("33")).bind(listing).bind(uid("13")).bind(uid("11")).execute(&mut *tx).await.unwrap();
        let sale = Settlement {
            id: uid("33"),
            listing_id: listing,
            buyer: uid("13"),
            seller: uid("11"),
            currency: Currency::Coins,
            amount: 100,
            commission: 10,
            seller_net: 90,
            reservation: listing,
        };
        funds.settle(&mut tx, &sale).await.unwrap();
    } else {
        let hold = character_intent(
            Command::Hold,
            104,
            now,
            vec![
                ("buyer_account_id", strval(uid("13"))),
                ("buyer_character_id", strval(uid("14"))),
                ("offer_id", strval(uid("35"))),
                ("hold_purpose", Value::String("RESERVE".into())),
                ("amount", Value::String("100".into())),
                ("expected_hold_revision", Value::String("0".into())),
            ],
        );
        let data = format!(
            "{{\"hold_id\":\"{}\",\"hold_revision\":\"1\",\"buyer_account_id\":\"{}\",\"offer_id\":\"{}\",\"hold_purpose\":\"RESERVE\",\"held_amount\":\"100\"}}",
            uid("34"),
            uid("13"),
            uid("35")
        );
        committed_fact(a, &mut tx, &hold, &data, now).await;
        sqlx::query("INSERT INTO bazaar_authority_gold_sales(id,listing_id,buyer,seller,gross_amount,fee_amount,seller_amount,hold_id,hold_revision) VALUES($1,$2,$3,$4,100,3,97,$5,1)").bind(uid("33")).bind(listing).bind(uid("13")).bind(uid("11")).bind(uid("34")).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=110")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    db
}
async fn listing_state(db: &BrowserFixture) -> crate::State {
    sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
async fn outbox_count(db: &BrowserFixture, command: Command) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox WHERE listing_id=$1 AND command=$2")
        .bind(uid("30"))
        .bind(command.wire())
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
/// Coin releases to the buyer for sale uid 33, and the buyer's spendable coins.
async fn coin_refunds(db: &BrowserFixture) -> (i64, i64) {
    sqlx::query_as("SELECT (SELECT count(*) FROM wallet_operations WHERE kind='release' AND account_id=$1 AND reference_id=$2),(SELECT COALESCE(sum(available_delta),0)::bigint FROM wallet_entries WHERE account_id=$1)")
        .bind(uid("13")).bind(uid("33")).fetch_one(&db.pool).await.unwrap()
}
async fn bind_refusal_scenario(currency: &str) {
    let db = settling_character_fixture(currency).await;
    db.runtime.tick().await.unwrap();
    let settlement = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_eq!(
        text(&settlement.value, "purpose").unwrap(),
        "BAZAAR_SETTLEMENT"
    );
    reject(&db, &settlement, "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
    // The refusal retires the never-published settlement leg and issues the
    // escrow return; funds stay held until that return is proven.
    db.runtime.tick().await.unwrap();
    let returning = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_eq!(
        text(&returning.value, "purpose").unwrap(),
        "BAZAAR_ESCROW_RETURN"
    );
    // The return Bind is a fresh operation over a fresh transfer, so Game can
    // never answer it with the refused settlement's OPERATION_CONFLICT.
    assert_ne!(
        text(&returning.value, "operation_id").unwrap(),
        text(&settlement.value, "operation_id").unwrap()
    );
    assert_ne!(
        text(&returning.value, "transfer_operation_id").unwrap(),
        text(&settlement.value, "transfer_operation_id").unwrap()
    );
    // The retirement cites the refused Bind in the refusal's own recovery
    // generation, the evidence Game requires before it replaces the lock.
    let retired: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT evidence_operation_id,recovery_generation::text FROM bazaar_authority_transfer_retirements WHERE operation_id=$1",
    )
    .bind(id(text(&settlement.value, "transfer_operation_id").unwrap()).unwrap())
    .fetch_optional(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        retired,
        Some((
            id(text(&settlement.value, "operation_id").unwrap()).unwrap(),
            "3".to_owned()
        ))
    );
    assert_eq!(listing_state(&db).await, crate::State::AwaitingDelivery);
    assert_eq!(outbox_count(&db, Command::Release).await, 0);
    if currency == "coins" {
        assert_eq!(coin_refunds(&db).await, (0, 0));
    }
    // While Game has not decided the return Bind (Pending), the worker retries
    // the same bytes: no new leg, no release, no failure.
    db.runtime.tick().await.unwrap();
    assert_eq!(
        newest_intent(&db, uid("30"), Command::Bind).await.bytes(),
        returning.bytes()
    );
    assert_eq!(outbox_count(&db, Command::Bind).await, 3);
    assert_eq!(outbox_count(&db, Command::Release).await, 0);
    assert_eq!(listing_state(&db).await, crate::State::AwaitingDelivery);
    let transfer = id(text(&returning.value, "transfer_operation_id").unwrap()).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"3\",\"transfer_operation_id\":\"{transfer}\",\"transfer_binding_digest\":\"{}\"}}",
        uid("31"),
        uid("32"),
        text(&returning.value, "transfer_binding_digest").unwrap()
    );
    committed_fact(db.runtime.authority(), &mut tx, &returning, &data, now).await;
    tx.commit().await.unwrap();
    // A committed return Bind publishes the return leg, never a new settlement.
    db.runtime.tick().await.unwrap();
    let published: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM character_commands WHERE id=$1 AND state='PENDING')",
    )
    .bind(transfer)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(published);
    let settlements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_SETTLEMENT'",
    )
    .bind(uid("30"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(settlements, 1);
    assert_eq!(listing_state(&db).await, crate::State::AwaitingDelivery);
    assert_eq!(outbox_count(&db, Command::Unlock).await, 0);
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    character_transfer_receipt(&mut tx, transfer, (uid("40"), uid("11")), now).await;
    character_status(&db, &mut tx, "RETURNED", transfer, ("3", "3"), now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    if currency == "gold" {
        assert_eq!(listing_state(&db).await, crate::State::AwaitingDelivery);
        assert_eq!(outbox_count(&db, Command::Unlock).await, 0);
        let release = newest_intent(&db, uid("30"), Command::Release).await;
        assert_eq!(
            text(&release.value, "release_reason").unwrap(),
            "TERMINAL_REFUSAL"
        );
        assert_eq!(text(&release.value, "amount").unwrap(), "100");
        assert_eq!(
            text(&release.value, "hold_id").unwrap(),
            uid("34").to_string()
        );
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let data = format!(
            "{{\"hold_id\":\"{}\",\"hold_revision\":\"2\",\"released_amount\":\"100\",\"remaining_held\":\"0\",\"hold_state\":\"RELEASED\"}}",
            uid("34")
        );
        committed_fact(db.runtime.authority(), &mut tx, &release, &data, now).await;
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
    }
    assert_eq!(listing_state(&db).await, crate::State::Cancelled);
    let compensation: Uuid = sqlx::query_scalar(
        "SELECT return_operation_id FROM bazaar_authority_compensations WHERE listing_id=$1",
    )
    .bind(uid("30"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        compensation,
        id(text(&returning.value, "operation_id").unwrap()).unwrap()
    );
    assert_eq!(outbox_count(&db, Command::Unlock).await, 1);
    // Running the worker again neither releases twice nor issues a new leg.
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(outbox_count(&db, Command::Unlock).await, 1);
    assert_eq!(outbox_count(&db, Command::Bind).await, 3);
    if currency == "coins" {
        assert_eq!(coin_refunds(&db).await, (1, 100));
        let hold: (i64, bool) = sqlx::query_as(
            "SELECT amount,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2",
        )
        .bind(uid("30"))
        .bind(uid("13"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(hold, (0, true));
    } else {
        assert_eq!(outbox_count(&db, Command::Release).await, 1);
    }
    db.end().await;
}
#[tokio::test]
async fn postgres_refused_character_settlement_bind_returns_and_refunds_coins_once() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    bind_refusal_scenario("coins").await;
    eprintln!(
        "postgres_refused_character_settlement_bind_returns_and_refunds_coins_once: {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn postgres_refused_character_settlement_bind_returns_and_releases_gold_once() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    bind_refusal_scenario("gold").await;
    eprintln!(
        "postgres_refused_character_settlement_bind_returns_and_releases_gold_once: {:?}",
        started.elapsed()
    );
}
/// Game replaces the escrow lock only on a refusal recorded in the current
/// recovery generation; after recovery moves the fence on, a refusal from the
/// older generation is no evidence and must never drive the return leg.
#[tokio::test]
async fn postgres_older_generation_settlement_bind_refusal_never_returns_the_character() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("coins").await;
    db.runtime.tick().await.unwrap();
    let settlement = newest_intent(&db, uid("30"), Command::Bind).await;
    reject(&db, &settlement, "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
    let mut tx = db.pool.begin().await.unwrap();
    let current = db.runtime.authority();
    assert!(
        current
            .settlement_bind_refused(&mut tx, uid("30"))
            .await
            .unwrap()
            .is_some()
    );
    let mut recovered = current.clone();
    recovered.fence.recovery_generation = 4;
    assert!(
        recovered
            .settlement_bind_refused(&mut tx, uid("30"))
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
    db.end().await;
}
#[tokio::test]
async fn postgres_expired_character_settlement_bind_still_retries_both_currencies() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for currency in ["coins", "gold"] {
        let db = settling_character_fixture(currency).await;
        db.runtime.tick().await.unwrap();
        let first = newest_intent(&db, uid("30"), Command::Bind).await;
        assert_eq!(text(&first.value, "purpose").unwrap(), "BAZAAR_SETTLEMENT");
        reject_expired(&db, &first).await;
        db.runtime.tick().await.unwrap();
        let retry = newest_intent(&db, uid("30"), Command::Bind).await;
        assert_eq!(text(&retry.value, "purpose").unwrap(), "BAZAAR_SETTLEMENT");
        assert_ne!(
            text(&retry.value, "transfer_operation_id").unwrap(),
            text(&first.value, "transfer_operation_id").unwrap()
        );
        let refusal: Option<String> = sqlx::query_scalar(
            "SELECT refusal_code FROM bazaar_authority_assets WHERE listing_id=$1",
        )
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(refusal, None);
        let returns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_ESCROW_RETURN'",
        )
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(returns, 0);
        assert_eq!(listing_state(&db).await, crate::State::AwaitingDelivery);
        assert_eq!(outbox_count(&db, Command::Release).await, 0);
        if currency == "coins" {
            assert_eq!(coin_refunds(&db).await, (0, 0));
        }
        db.end().await;
    }
    eprintln!(
        "postgres_expired_character_settlement_bind_still_retries_both_currencies: {:?}",
        started.elapsed()
    );
}
#[test]
fn character_fixture_uses_one_source_clock_across_second_boundary() {
    let now = 1790000000;
    let intent = character_intent(
        Command::Unlock,
        104,
        now,
        vec![
            ("character_id", Value::String(uid("31").to_string())),
            ("lock_id", Value::String(uid("32").to_string())),
            ("expected_lock_revision", Value::String("2".into())),
        ],
    );
    assert_eq!(number(&intent.value, "issued_at_source").unwrap(), now);
    assert_eq!(
        number(&intent.value, "expires_at_source").unwrap(),
        now + 300
    );
}

async fn clone_verified_item(db: &BrowserFixture, listing: Uuid, world: Uuid, price: i64) {
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let item_instance = Uuid::now_v7();
    let escrow_id = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) SELECT $1,owner,kind,form,currency,$2,accept_offers,'draft',ends_at FROM bazaar_listings WHERE id=$3")
        .bind(listing).bind(price).bind(uid("20")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_item_details(id,definition_key,definition_revision,world_id,seller_character_id,item_instance_id,quantity) SELECT $1,definition_key,definition_revision,$2,seller_character_id,$4,quantity FROM bazaar_item_details WHERE id=$3")
        .bind(listing).bind(world).bind(uid("20")).bind(item_instance).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) SELECT $1,$1,$2,seller_character_id,$4,expected_revision,session_hash,quantity,publication_consent,input_binding FROM bazaar_authority_assets WHERE listing_id=$3")
        .bind(listing).bind(world).bind(uid("20")).bind(item_instance).execute(&mut *tx).await.unwrap();
    let operation = Uuid::now_v7();
    let revision: String =
        sqlx::query_scalar("SELECT (max(source_revision)+1)::text FROM bazaar_game_outbox")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let raw = vector("escrow_intent")
        .replace(
            "\"source_revision\":\"18\"",
            &format!("\"source_revision\":\"{revision}\""),
        )
        .replace(&uid("20").to_string(), &listing.to_string())
        .replace(&uid("12").to_string(), &world.to_string())
        .replace(&uid("02").to_string(), &operation.to_string())
        .replace(&uid("22").to_string(), &item_instance.to_string())
        .replace(&uid("23").to_string(), &escrow_id.to_string());
    let intent = PreparedIntent::parse(&raw).unwrap();
    store_intent(&mut tx, &intent).await;
    let receipt = vector("escrow_receipt")
        .replace(&uid("12").to_string(), &world.to_string())
        .replace(&uid("02").to_string(), &operation.to_string())
        .replace(&uid("22").to_string(), &item_instance.to_string())
        .replace(&uid("23").to_string(), &escrow_id.to_string());
    assert_eq!(
        db.runtime
            .authority()
            .receive(
                &mut tx,
                &authority().config.receipt_authority,
                receipt.as_bytes(),
                now
            )
            .await
            .unwrap(),
        ReceiptResult::Stored
    );
    assert_eq!(
        audit(db.runtime.authority(), &mut tx, &intent, &receipt, now).await,
        ReceiptResult::Stored
    );
    for kind in ["STATUS", "POLICY"] {
        let raw: String = sqlx::query_scalar(
            "SELECT canonical_body FROM bazaar_game_projections WHERE kind=$1 AND subject=$2",
        )
        .bind(kind)
        .bind(if kind == "STATUS" {
            uid("20")
        } else {
            uid("12")
        })
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let raw = raw
            .replace(&uid("20").to_string(), &listing.to_string())
            .replace(&uid("12").to_string(), &world.to_string())
            .replace(&uid("02").to_string(), &operation.to_string())
            .replace(&uid("22").to_string(), &item_instance.to_string())
            .replace(&uid("23").to_string(), &escrow_id.to_string());
        Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: false,
            },
        }
        .ingest(
            &mut tx,
            &authority().config.projection_authority,
            raw.as_bytes(),
            now,
        )
        .await
        .unwrap();
    }
    sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
        .bind(listing)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}
#[tokio::test]
async fn postgres_item_restrictions_scope_verified_world_definition_and_value() {
    use crate::moderation::{StaffCommand, execute_staff};
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    let seed = Uuid::now_v7();
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
        .bind(uid("13"))
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,20000)",
    )
    .bind(seed)
    .bind(uid("13"))
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,20000,0)").bind(Uuid::now_v7()).bind(seed).bind(uid("13")).execute(&db.pool).await.unwrap();
    clone_verified_item(&db, uid("50"), uid("42"), 10000).await;
    clone_verified_item(&db, uid("51"), uid("42"), 20000).await;
    clone_verified_item(&db, uid("52"), uid("12"), 10000).await;
    let mut tx = db.pool.begin().await.unwrap();
    execute_staff(
        &mut tx,
        uid("11"),
        &StaffCommand::RestrictItem {
            id: Uuid::now_v7(),
            world: uid("42"),
            family: "item".into(),
            production_key: "fixture-sword".into(),
            revision_ref: "v1".into(),
            minimum_value: 15000,
            denied: false,
            reason: "verified minimum restriction".into(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    for (listing, allowed) in [(uid("20"), true), (uid("50"), false), (uid("51"), true)] {
        let mut tx = db.pool.begin().await.unwrap();
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(listing)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(
            db.runtime
                .authority()
                .escrow_ready(&mut tx, &l)
                .await
                .unwrap(),
            allowed
        );
        let fenced: bool = sqlx::query_scalar("SELECT bazaar_item_restricted($1,$2)")
            .bind(listing)
            .bind(l.price)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(fenced, !allowed);
        tx.rollback().await.unwrap();
        let (status, _) = db.get(&format!("/en/bazaar/{listing}")).await;
        assert_eq!(
            status == axum::http::StatusCode::OK,
            allowed,
            "listing {listing}: {status}"
        );
    }
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=(SELECT max(source_revision) FROM bazaar_game_outbox)").execute(&db.pool).await.unwrap();
    assert_eq!(db.bid(12000).await, axum::http::StatusCode::SEE_OTHER);
    sqlx::query("UPDATE bazaar_listings SET ends_at=now()-interval '1 second' WHERE id=$1")
        .bind(uid("20"))
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    let settlements: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_settlements WHERE listing_id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(settlements, 1);
    // Restriction must not prevent the final transition after a safe return.
    sqlx::query("UPDATE bazaar_listings SET state='cancelled' WHERE id=$1")
        .bind(uid("50"))
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_listings SET state='cancelled' WHERE id=$1")
        .bind(uid("51"))
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_authority_assets SET cancel_requested=true WHERE listing_id=$1")
        .bind(uid("52"))
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    let returned = newest_intent(&db, uid("52"), Command::Return).await;
    let escrow = newest_intent(&db, uid("52"), Command::Escrow).await;
    let mut tx = db.pool.begin().await.unwrap();
    let (_, receipt) = db
        .runtime
        .authority()
        .retained(
            &mut tx,
            id(text(&escrow.value, "operation_id").unwrap()).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
    let d = receipt.value.get("result_data").unwrap();
    let Value::Object(mut item) = d.get("item").unwrap().clone() else {
        panic!("item");
    };
    for (key, value) in &mut item {
        if key == "state_revision" {
            *value = Value::String("9".into());
        }
    }
    let data = Value::Object(vec![
        ("escrow_id".into(), d.get("escrow_id").unwrap().clone()),
        ("escrow_revision".into(), Value::String("2".into())),
        (
            "destination_account_id".into(),
            Value::String(uid("11").to_string()),
        ),
        (
            "destination_character_id".into(),
            Value::String(uid("21").to_string()),
        ),
        (
            "delivery_id".into(),
            Value::String(Uuid::now_v7().to_string()),
        ),
        ("item".into(), Value::Object(item)),
    ]);
    let now = runtime::now(&mut tx).await.unwrap();
    committed_fact(
        db.runtime.authority(),
        &mut tx,
        &returned,
        &String::from_utf8(canonical::encode(&data).unwrap()).unwrap(),
        now,
    )
    .await;
    let raw: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1",
    )
    .bind(uid("52"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let raw = raw
        .replace("ESCROWED", "RETURNED")
        .replace(
            "\"projection_revision\":\"1\"",
            "\"projection_revision\":\"2\"",
        )
        .replace("\"custody_revision\":\"1\"", "\"custody_revision\":\"2\"")
        .replace(
            text(&escrow.value, "operation_id").unwrap(),
            text(&returned.value, "operation_id").unwrap(),
        );
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        &mut tx,
        &authority().config.projection_authority,
        raw.as_bytes(),
        now,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state: crate::State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("52"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, crate::State::Cancelled);
    db.end().await;
    eprintln!(
        "postgres_item_restrictions_scope_verified_world_definition_and_value: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait() {
    let started = std::time::Instant::now();
    let db = returned_character_fixture("coins").await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    let mut chains = Vec::new();
    for (command_index, command) in [
        Command::Return,
        Command::Unlock,
        Command::Release,
        Command::Payout,
    ]
    .into_iter()
    .enumerate()
    {
        let extra = match command {
            Command::Return => vec![
                ("escrow_id", Value::String(uid("23").to_string())),
                ("expected_escrow_revision", Value::String("1".into())),
                ("seller_character_id", Value::String(uid("21").to_string())),
                (
                    "escrow_receipt_digest",
                    Value::String(digest(vector("escrow_receipt").as_bytes())),
                ),
            ],
            Command::Unlock => vec![
                ("character_id", Value::String(uid("31").to_string())),
                ("lock_id", Value::String(uid("32").to_string())),
                ("expected_lock_revision", Value::String("2".into())),
            ],
            Command::Release => vec![
                ("buyer_account_id", Value::String(uid("13").to_string())),
                ("hold_id", Value::String(uid("60").to_string())),
                ("expected_hold_revision", Value::String("1".into())),
                ("release_reason", Value::String("EXCESS".into())),
                ("amount", Value::String("10000".into())),
            ],
            Command::Payout => vec![
                ("buyer_account_id", Value::String(uid("13").to_string())),
                ("hold_id", Value::String(uid("60").to_string())),
                ("expected_hold_revision", Value::String("1".into())),
                ("sale_id", Value::String(uid("70").to_string())),
                ("gross_amount", Value::String("10000".into())),
                ("fee_bps", Value::Unsigned(250)),
                ("fee_amount", Value::String("250".into())),
                ("seller_amount", Value::String("9750".into())),
                (
                    "delivery_operation_id",
                    Value::String(uid("03").to_string()),
                ),
                (
                    "delivery_receipt_digest",
                    Value::String(digest(vector("delivery_receipt").as_bytes())),
                ),
            ],
            _ => panic!("cleanup"),
        };
        let root = Uuid::now_v7();
        let mut step = root;
        let mut previous = root;
        let mut tail = root;
        // Setup mirrors production: authority_http.rs receives one receipt per
        // request transaction, so every step commits on its own and each receive
        // gets its own three-second mutation budget.
        for n in 0..128 {
            let mut tx = db.pool.begin().await.unwrap();
            let intent = character_intent(
                command,
                104 + command_index as u64 * 129 + n,
                now - 301,
                extra.clone(),
            );
            tail = id(text(&intent.value, "operation_id").unwrap()).unwrap();
            store_intent_with_key(
                &mut tx,
                &intent,
                &format!("bazaar:{}:{}:{step}:1", uid("30"), command.wire()),
            )
            .await;
            sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,3,3)").bind(tail).execute(&mut *tx).await.unwrap();
            let receipt = format!(
                r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"{}","operation_id":"{tail}","outcome":"REJECTED","result_code":"CHAR_CMD_INTENT_EXPIRED","result_data":null,"recovery_generation":"3","decided_at":"{now}"}}"#,
                command.wire()
            );
            db.runtime
                .authority()
                .receive(
                    &mut tx,
                    &authority().config.receipt_authority,
                    receipt.as_bytes(),
                    now,
                )
                .await
                .unwrap();
            assert_eq!(
                audit(db.runtime.authority(), &mut tx, &intent, &receipt, now).await,
                ReceiptResult::Stored
            );
            tx.commit().await.unwrap();
            previous = step;
            step = tail;
        }
        let mut tx = db.pool.begin().await.unwrap();
        // A persisted pointer from 127 successful retries must skip their history.
        sqlx::query("INSERT INTO bazaar_cleanup_attempts(listing_id,command,root_step,current_step) VALUES($1,$2,$3,$4)")
            .bind(uid("30")).bind(command.wire()).bind(root).bind(previous).execute(&mut *tx).await.unwrap();
        let pending = character_intent(
            command,
            232 + command_index as u64 * 129,
            now,
            extra.clone(),
        );
        let pending_id = id(text(&pending.value, "operation_id").unwrap()).unwrap();
        store_intent_with_key(
            &mut tx,
            &pending,
            &format!("bazaar:{}:{}:{tail}:1", uid("30"), command.wire()),
        )
        .await;
        tx.commit().await.unwrap();
        chains.push((command, root, tail, pending_id, extra));
    }
    sqlx::query("ANALYZE bazaar_game_outbox, bazaar_authority_receipts, bazaar_authority_decisions, bazaar_disposition_fences, bazaar_command_receipt_heads, bazaar_cleanup_attempts").execute(&db.pool).await.unwrap();
    for (command, root, tail, pending, extra) in chains {
        let mut tx = db.pool.begin().await.unwrap();
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(uid("30"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        // The bound counts tuples on the plan PostgreSQL uses at scale. On these
        // few-page fixture tables the planner may prefer reading the whole heap
        // over one primary-key probe (issue #1919: 3 pages, 256 tuples). Seq scans
        // stay possible where no index path exists, so an unindexed lookup still
        // fails the bound.
        sqlx::query("SET LOCAL enable_seqscan=off")
            .execute(&mut *tx)
            .await
            .unwrap();
        let before:i64=sqlx::query_scalar("SELECT coalesce(sum(seq_tup_read+idx_tup_fetch),0)::bigint FROM pg_stat_xact_user_tables").fetch_one(&mut *tx).await.unwrap();
        assert_eq!(
            db.runtime
                .issue(&mut tx, &l, l.owner, None, command, root, extra)
                .await
                .unwrap(),
            pending
        );
        let after:i64=sqlx::query_scalar("SELECT coalesce(sum(seq_tup_read+idx_tup_fetch),0)::bigint FROM pg_stat_xact_user_tables").fetch_one(&mut *tx).await.unwrap();
        assert!(
            after - before < 100,
            "{command:?}: {} tuples read",
            after - before
        );
        let pointer:Uuid=sqlx::query_scalar("SELECT current_step FROM bazaar_cleanup_attempts WHERE listing_id=$1 AND command=$2 AND root_step=$3").bind(l.id).bind(command.wire()).bind(root).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(pointer, tail);
        let runtime = db.runtime.clone();
        let issuer = tokio::spawn(async move {
            let mut tx = runtime.pool.begin().await.unwrap();
            runtime.authority().lock(&mut tx).await.unwrap();
            tx.commit().await.unwrap();
        });
        tx.commit().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), issuer)
            .await
            .unwrap()
            .unwrap();
    }
    db.end().await;
    eprintln!(
        "postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_return_publication_uses_live_dispositions_and_revokes_conflicts() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    let escrow = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
    let receipt = Receipt::parse(vector("escrow_receipt").as_bytes()).unwrap();
    let extra = vec![
        ("escrow_id", Value::String(uid("23").to_string())),
        ("expected_escrow_revision", Value::String("1".into())),
        ("seller_character_id", Value::String(uid("21").to_string())),
        ("escrow_receipt_digest", Value::String(receipt.digest())),
    ];
    let mut first = Uuid::nil();
    // One receipt per transaction, as the receipt route does: each mutation
    // transaction has its own three-second lock and statement budget.
    for n in 0..1024 {
        let mut tx = db.pool.begin().await.unwrap();
        let intent = character_intent(Command::Return, 19 + n, now - 301, extra.clone());
        let raw = std::str::from_utf8(intent.bytes())
            .unwrap()
            .replace(&uid("30").to_string(), &uid("20").to_string());
        let intent = PreparedIntent::parse(&raw).unwrap();
        let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
        if n == 0 {
            first = operation;
        }
        store_intent_at(
            &mut tx,
            &intent,
            &format!("fixture:{operation}"),
            Some(time::OffsetDateTime::from_unix_timestamp((now - 301) as i64).unwrap()),
        )
        .await;
        sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,3,3)").bind(operation).execute(&mut *tx).await.unwrap();
        let raw = format!(
            r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"ReturnBazaarItem","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_INTENT_EXPIRED","result_data":null,"recovery_generation":"3","decided_at":"{now}"}}"#
        );
        db.runtime
            .authority()
            .receive(
                &mut tx,
                &authority().config.receipt_authority,
                raw.as_bytes(),
                now,
            )
            .await
            .unwrap();
        assert_eq!(
            audit(db.runtime.authority(), &mut tx, &intent, &raw, now).await,
            ReceiptResult::Stored
        );
        tx.commit().await.unwrap();
    }
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=1042")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("ANALYZE bazaar_game_outbox, bazaar_authority_receipts, bazaar_authority_decisions, bazaar_disposition_fences, bazaar_command_receipt_heads, bazaar_cleanup_attempts").execute(&db.pool).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(seq_tup_read+idx_tup_fetch),0)::bigint FROM pg_stat_xact_user_tables",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let operation = db
        .runtime
        .issue(
            &mut tx,
            &l,
            l.owner,
            None,
            Command::Return,
            l.id,
            extra.clone(),
        )
        .await
        .unwrap();
    let after: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(seq_tup_read+idx_tup_fetch),0)::bigint FROM pg_stat_xact_user_tables",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(
        after - before < 100,
        "publication read {} tuples",
        after - before
    );
    let dispatched: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=$1)",
    )
    .bind(operation)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(dispatched);
    // Roll back this candidate, then poison a historical refusal. It must once
    // again fence compensation even though it was previously reconciled.
    tx.rollback().await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'DECISION')").bind(first).bind("a".repeat(64)).execute(&db.pool).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        db.runtime
            .issue(&mut tx, &l, l.owner, None, Command::Return, l.id, extra)
            .await,
        Err(Error::Conflict)
    ));
    assert!(
        db.runtime
            .authority()
            .retained(
                &mut tx,
                id(text(&escrow.value, "operation_id").unwrap()).unwrap()
            )
            .await
            .unwrap()
            .is_some()
    );
    tx.rollback().await.unwrap();
    db.end().await;
    eprintln!(
        "postgres_return_publication_uses_live_dispositions_and_revokes_conflicts: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a3_l6_01_coin_payout_capacity_precedes_item_and_character_handoff() {
    use crate::{Currency, Reservation, Reservations, Settlement};
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let root = sqlx::PgPool::connect(&url).await.unwrap();
    let schema = format!("capacity_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let search = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |db, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&pool)
        .await
        .unwrap();
    let funds = CoinReservations {
        authority: authority(),
    };
    for kind in ["item", "character"] {
        let seller = Uuid::now_v7();
        let buyer = Uuid::now_v7();
        let listing = Uuid::now_v7();
        let sale_id = Uuid::now_v7();
        let mut tx = pool.begin().await.unwrap();
        for (account, amount) in [(seller, oteryn_wallet::MAX_BALANCE), (buyer, 100)] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$1::text||'@example.invalid','test','active',now(),'en')").bind(account).execute(&mut *tx).await.unwrap();
            funds.lock_accounts(&mut tx, &[account]).await.unwrap();
            let op = Uuid::now_v7();
            sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,$3)").bind(op).bind(account).bind(amount).execute(&mut *tx).await.unwrap();
            oteryn_wallet::credit_in(&mut tx, op, account, amount)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,$3::bazaar_kind,'auction','coins',100,false,now()+interval '1 day')").bind(listing).bind(seller).bind(kind).execute(&mut *tx).await.unwrap();
        funds
            .reserve(
                &mut tx,
                Reservation {
                    id: listing,
                    account: buyer,
                    currency: Currency::Coins,
                    amount: 100,
                },
            )
            .await
            .unwrap();
        sqlx::query("INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key) VALUES($1,$2,$3,$4,'coins',100,10,90,$2,'capacity')").bind(sale_id).bind(listing).bind(buyer).bind(seller).execute(&mut *tx).await.unwrap();
        let sale = Settlement {
            id: sale_id,
            listing_id: listing,
            buyer,
            seller,
            currency: Currency::Coins,
            amount: 100,
            commission: 10,
            seller_net: 90,
            reservation: listing,
        };
        assert!(
            matches!(
                funds.settle(&mut tx, &sale).await,
                Err(crate::Error::Overflow)
            ),
            "{kind} seller at cap must refuse freeze"
        );
        let dispositions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_coin_dispositions WHERE settlement_id=$1",
        )
        .bind(sale_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(dispositions, 0);
        let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(outbox, 0);
        // Make exactly the payout capacity available and freeze it durably.
        let op = Uuid::now_v7();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,-90)").bind(op).bind(seller).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,-90,0)").bind(Uuid::now_v7()).bind(op).bind(seller).execute(&mut *tx).await.unwrap();
        funds.settle(&mut tx, &sale).await.unwrap();
        funds.handoff_capacity(&mut tx, &sale).await.unwrap();
        tx.commit().await.unwrap();
        let competing_pool = pool.clone();
        tokio::spawn(async move {
            let mut credit = competing_pool.begin().await.unwrap();
            CoinReservations { authority: authority() }.lock_accounts(&mut credit,&[seller]).await.unwrap();
            let op = Uuid::now_v7();
            sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,1)").bind(op).bind(seller).execute(&mut *credit).await.unwrap();
            assert!(matches!(oteryn_wallet::credit_in(&mut credit,op,seller,1).await,Err(oteryn_wallet::Error::Refused(oteryn_wallet::Refusal::Overflow))));
            credit.rollback().await.unwrap();
        }).await.unwrap();
        let capacity: i64 =
            sqlx::query_scalar("SELECT amount FROM wallet_incoming_capacity WHERE id=$1")
                .bind(sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(capacity, 90);
        let mut handoff = pool.begin().await.unwrap();
        funds.handoff_capacity(&mut handoff, &sale).await.unwrap();
        handoff.rollback().await.unwrap();
    }
    // Exercise upgrade backfill as well as the fresh migration path above.
    sqlx::query("DROP TABLE wallet_incoming_capacity")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115225000_fix_a3-l6_incoming_capacity.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let promises: (i64, i64) =
        sqlx::query_as("SELECT count(*),sum(amount)::bigint FROM wallet_incoming_capacity")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(promises, (2, 180));
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    eprintln!(
        "PG a3_l6_01_coin_payout_capacity_precedes_item_and_character_handoff {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn r7_returned_draft_unlocks() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = returned_character_in_state("coins", "draft").await;
    sqlx::query("UPDATE bazaar_authority_assets SET cancel_requested=true WHERE listing_id=$1")
        .bind(uid("30"))
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(listing_state(&db).await, crate::State::Cancelled);
    assert_eq!(outbox_count(&db, Command::Unlock).await, 1);
    db.end().await;
}

#[tokio::test]
async fn r7_stale_batch_keeps_open_auction() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=100")
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.advance(uid("20")).await.unwrap();
    let returns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_game_outbox WHERE command='ReturnBazaarItem'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(returns, 0, "open auction dispatched a return");
    db.end().await;
}

#[tokio::test]
async fn r7_refused_draft_quota() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = returned_character_in_state("coins", "draft").await;
    sqlx::query("UPDATE bazaar_authority_assets SET refusal_code='REFUSED' WHERE listing_id=$1")
        .bind(uid("30"))
        .execute(&db.pool)
        .await
        .unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        runtime::listing_count(&mut tx, uid("11")).await.unwrap(),
        1,
        "only the active item consumes quota"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

async fn expired_item_with_bid(seller_balance: i64) -> BrowserFixture {
    let db = BrowserFixture::new().await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=100")
        .execute(&db.pool)
        .await
        .unwrap();
    for (account, amount) in [(uid("11"), seller_balance), (uid("13"), 10000)] {
        let mut tx = db.pool.begin().await.unwrap();
        let funds = CoinReservations {
            authority: db.runtime.authority().clone(),
        };
        use crate::Reservations;
        funds.lock_accounts(&mut tx, &[account]).await.unwrap();
        let op = Uuid::now_v7();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,$3)").bind(op).bind(account).bind(amount).execute(&mut *tx).await.unwrap();
        oteryn_wallet::credit_in(&mut tx, op, account, amount)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(db.bid(10000).await, axum::http::StatusCode::SEE_OTHER);
    sqlx::query("UPDATE bazaar_listings SET ends_at=now()-interval '1 second' WHERE id=$1")
        .bind(uid("20"))
        .execute(&db.pool)
        .await
        .unwrap();
    db
}

#[tokio::test]
async fn r7_hold_blocks_handoff() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,1,$2,'staff suspended sale')").bind(uid("20")).bind(uid("11")).execute(&db.pool).await.unwrap();
    db.runtime.tick().await.unwrap();
    let deliveries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_game_outbox WHERE command='DeliverBazaarItem'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(deliveries, 0, "held sale dispatched a delivery");
    let held: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=$1)",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(held);
    db.end().await;
}

#[tokio::test]
async fn r7_overflow_refunds_and_returns() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(oteryn_wallet::MAX_BALANCE).await;
    db.runtime.tick().await.unwrap();
    let holds: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_coin_holds WHERE account_id=$1 AND amount>0",
    )
    .bind(uid("13"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(holds, 0, "buyer reservation survived payout refusal");
    db.runtime.tick().await.unwrap();
    let returns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_game_outbox WHERE command='ReturnBazaarItem'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(returns, 1);
    db.end().await;
}

#[tokio::test]
async fn r7_held_delivery_captures() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let delivery = newest_intent(&db, uid("20"), Command::Deliver).await;
    sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,1,$2,'staff suspended sale')").bind(uid("20")).bind(uid("11")).execute(&db.pool).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = vector("delivery_receipt")
        .replace(
            &uid("4").to_string(),
            text(&delivery.value, "operation_id").unwrap(),
        )
        .replace("1790000002", &now.to_string());
    let authority = db.runtime.authority();
    authority
        .receive(
            &mut tx,
            &authority.config.receipt_authority,
            receipt.as_bytes(),
            now,
        )
        .await
        .unwrap();
    assert_eq!(
        audit(authority, &mut tx, &delivery, &receipt, now).await,
        ReceiptResult::Stored
    );
    let status = vector("delivered_status")
        .replace(
            &uid("4").to_string(),
            text(&delivery.value, "operation_id").unwrap(),
        )
        .replace("1790000002", &now.to_string());
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        &mut tx,
        &db.runtime.authority().config.projection_authority,
        status.as_bytes(),
        now,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state: crate::State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, crate::State::Completed);
    db.runtime.tick().await.unwrap();
    let balance: i64 = sqlx::query_scalar(
        "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(uid("11"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(balance, 9001);
    db.end().await;
}

#[tokio::test]
async fn r7_auction_maximum_uses_offer_hold() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    sqlx::query("UPDATE bazaar_listings SET currency='gold'")
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime
        .bid(uid("13"), uid("20"), 10000, uid("14"), Uuid::now_v7())
        .await
        .unwrap();
    let intent = newest_intent(&db, uid("20"), Command::Hold).await;
    assert_eq!(text(&intent.value, "hold_purpose").unwrap(), "OFFER");
    db.end().await;
}

#[tokio::test]
async fn r7_gold_delivery_requires_fresh_watermark() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let hold = Uuid::now_v7();
    let sale = Uuid::now_v7();
    gold_fact(&mut tx, Command::Hold, 10, uid("20"), hold, (0, 1, 10000)).await;
    sqlx::query("INSERT INTO bazaar_authority_gold_sales(id,listing_id,buyer,seller,gross_amount,fee_amount,seller_amount,hold_id,hold_revision) VALUES($1,$2,$3,$4,10000,250,9750,$5,1)").bind(sale).bind(uid("20")).bind(uid("13")).bind(uid("11")).bind(hold).execute(&mut *tx).await.unwrap();
    let intent = decode(
        serde_json::to_string(&serde_json::json!({
            "listing_id":uid("20"),"sale_id":sale,"seller_account_id":uid("11"),
            "funding":{"reservation_id":hold,"hold_revision":"1","gross_amount":"10000"}
        }))
        .unwrap()
        .as_bytes(),
        4096,
    )
    .unwrap();
    let a = db.runtime.authority();
    runtime::gold::funded(a, &mut tx, &intent, uid("13"))
        .await
        .unwrap();
    stale_watermark(a, &mut tx).await;
    assert!(!a.fresh(&mut tx).await.unwrap());
    assert!(
        matches!(
            runtime::gold::funded(a, &mut tx, &intent, uid("13")).await,
            Err(Error::Unavailable)
        ),
        "gold funding accepted without a fresh watermark"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[path = "invalidation_tests.rs"]
mod invalidation_tests;

async fn stale_watermark(a: &Authority, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) {
    sqlx::query("DELETE FROM bazaar_game_projections WHERE kind='WATERMARK'")
        .execute(&mut **tx)
        .await
        .unwrap();
    let now = runtime::now(tx).await.unwrap();
    let watermark = format!(
        r#"{{"contract_version":2,"operation":"PublishBazaarWatermarkV1","source_authority":"oteryn:game:primary","projection_epoch":"3","complete_through":"{}","observed_at":"{now}"}}"#,
        now - 60
    );
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
    .ingest(
        tx,
        &a.config.projection_authority,
        watermark.as_bytes(),
        now,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn r8_stale_coin_handoff() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let a = db.runtime.authority();
    stale_watermark(a, &mut tx).await;
    assert!(!a.fresh(&mut tx).await.unwrap());
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let deliveries:i64=sqlx::query_scalar("SELECT count(*) FROM bazaar_authority_dispatch JOIN bazaar_game_outbox USING(operation_id) WHERE listing_id=$1 AND command='DeliverBazaarItem'").bind(uid("20")).fetch_one(&db.pool).await.unwrap();
    assert_eq!(
        deliveries, 0,
        "delivery published without current custody watermark"
    );
    db.end().await;
}

#[tokio::test]
async fn r8_suspend_settled_listing() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let result = crate::moderation::execute_staff(
        &mut tx,
        uid("11"),
        &crate::moderation::StaffCommand::Suspend {
            listing: uid("20"),
            reason: "staff suspended sale".into(),
        },
    )
    .await;
    assert!(
        matches!(result, Err(crate::Error::IllegalTransition)),
        "suspension stranded settled sale"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r8_reconfirmed_escrow_digest() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let old = vector("escrow_receipt").replace(
        "\"recovery_generation\":\"3\"",
        "\"recovery_generation\":\"2\"",
    );
    let receipt = Receipt::parse(old.as_bytes()).unwrap();
    sqlx::query("INSERT INTO bazaar_authority_receipts(operation_id,recovery_generation,source_authority,intent_digest,receipt_digest,receipt,outcome) SELECT operation_id,2,source_authority,intent_digest,$1,$2,outcome FROM bazaar_authority_receipts WHERE operation_id=$3").bind(receipt.digest()).bind(old).bind(receipt.operation).execute(&mut *tx).await.unwrap();
    let wire = vector("delivery_intent").replace(
        &digest(vector("escrow_receipt").as_bytes()),
        &receipt.digest(),
    );
    let delivery = PreparedIntent::parse(&wire).unwrap();
    store_intent(&mut tx, &delivery).await;
    let a = db.runtime.authority();
    let result = a
        .receive(
            &mut tx,
            &a.config.receipt_authority,
            vector("delivery_receipt").as_bytes(),
            1790000002,
        )
        .await;
    assert!(
        matches!(result, Ok(ReceiptResult::Stored)),
        "reconfirmed escrow unavailable by original digest: {result:?}"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r8_replay_hold_before_release() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let hold = Uuid::now_v7();
    let op = gold_fact(&mut tx, Command::Hold, 10, uid("20"), hold, (0, 1, 10000)).await;
    gold_fact(
        &mut tx,
        Command::Release,
        11,
        uid("20"),
        hold,
        (1, 2, 10000),
    )
    .await;
    let mut a = authority();
    a.fence.recovery_generation = 4;
    let (intent, receipt) = authority().retained(&mut tx, op).await.unwrap().unwrap();
    let result =
        runtime::gold::validate_hold(&a, &mut tx, &intent, receipt.value.get("result_data")).await;
    assert!(
        result.is_ok(),
        "hold replay waited for its later release: {result:?}"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r8_suspended_gold_seller() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let request = Uuid::now_v7();
    let op = gold_fact(
        &mut tx,
        Command::Hold,
        10,
        uid("20"),
        Uuid::now_v7(),
        (0, 1, 12000),
    )
    .await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=10")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_listings SET currency='gold' WHERE id=$1")
        .bind(uid("20"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$2,$3,$4,12000,$5)").bind(request).bind(uid("20")).bind(uid("13")).bind(uid("14")).bind(op).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(uid("11"))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick_gold_bids().await.unwrap();
    let phase: String =
        sqlx::query_scalar("SELECT phase FROM bazaar_authority_gold_bids WHERE request_id=$1")
            .bind(request)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        phase, "releasing",
        "funded bid on suspended seller must release"
    );
    db.end().await;
}

#[tokio::test]
async fn r8_stale_coin_pull() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let i = newest_intent(&db, uid("20"), Command::Deliver).await;
    let op = text(&i.value, "operation_id").unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let a = db.runtime.authority();
    let peer = &a.config.projection_authority;
    let request = format!(
        r#"{{"contract_version":2,"operation":"ReadBazaarCommandIntentV1","source_authority":"{peer}","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","command":"DeliverBazaarItem","operation_id":"{op}"}}"#
    );
    assert!(a.exchange(&mut tx, peer, request.as_bytes()).await.is_ok());
    stale_watermark(a, &mut tx).await;
    assert!(
        matches!(
            a.exchange(&mut tx, peer, request.as_bytes()).await,
            Err(Error::Unavailable)
        ),
        "delivery readable after custody watermark expired"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}
#[tokio::test]
async fn unresynced_deliver_pull_requires_expiry_plus_skew_and_allows_receipt_replay() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let original = newest_intent(&db, uid("20"), Command::Deliver).await;
    let op = text(&original.value, "operation_id").unwrap();
    let a = db.runtime.authority();
    let peer = &a.config.projection_authority;
    let request = format!(
        r#"{{"contract_version":2,"operation":"ReadBazaarCommandIntentV1","source_authority":"{peer}","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","command":"DeliverBazaarItem","operation_id":"{op}"}}"#
    );
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query("DELETE FROM bazaar_authority_resync")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(matches!(
        a.finish_resync(&mut tx, &[uid("12")]).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        a.exchange(&mut tx, peer, request.as_bytes()).await,
        Err(Error::Unavailable)
    ));
    for age in [1, 6] {
        let now = runtime::now(&mut tx).await.unwrap();
        let mut value = original.value.clone();
        let Value::Object(fields) = &mut value else {
            panic!("expected intent object")
        };
        for (name, field) in fields {
            match name.as_str() {
                "issued_at_source" => *field = Value::String((now - 60).to_string()),
                "expires_at_source" => *field = Value::String((now - age).to_string()),
                _ => {}
            }
        }
        let raw = String::from_utf8(canonical::encode(&value).unwrap()).unwrap();
        let intent = PreparedIntent::parse(&raw).unwrap();
        sqlx::query(
            "UPDATE bazaar_game_outbox SET intent=$2,binding_digest=$3 WHERE operation_id=$1",
        )
        .bind(id(op).unwrap())
        .bind(&raw)
        .bind(intent.binding_digest())
        .execute(&mut *tx)
        .await
        .unwrap();
        let result = a.exchange(&mut tx, peer, request.as_bytes()).await;
        if age < 5 {
            assert!(matches!(result, Err(Error::Unavailable)));
        } else {
            assert_eq!(result.unwrap(), raw);
        }
    }
    tx.commit().await.unwrap();
    let expired = newest_intent(&db, uid("20"), Command::Deliver).await;
    reject_expired(&db, &expired).await;
    let mut tx = db.pool.begin().await.unwrap();
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    assert!(a.resynced(&mut tx).await.unwrap());
    tx.commit().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r9_fractional_gold_window() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let request = Uuid::now_v7();
    let op = gold_fact(
        &mut tx,
        Command::Hold,
        10,
        uid("20"),
        Uuid::now_v7(),
        (0, 1, 12000),
    )
    .await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=10")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE bazaar_listings SET currency='gold' WHERE id=$1")
        .bind(uid("20"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$2,$3,$4,12000,$5)").bind(request).bind(uid("20")).bind(uid("13")).bind(uid("14")).bind(op).execute(&mut *tx).await.unwrap();
    let deadline: time::OffsetDateTime = sqlx::query_scalar("UPDATE bazaar_listings SET ends_at=date_trunc('second',clock_timestamp())+interval '300 seconds' WHERE id=$1 RETURNING ends_at").bind(uid("20")).fetch_one(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick_gold_bids().await.unwrap();
    let accepted: time::OffsetDateTime =
        sqlx::query_scalar("SELECT ends_at FROM bazaar_bids WHERE listing_id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        accepted,
        deadline + time::Duration::minutes(5),
        "accepted gold bid must extend fractional window"
    );
    db.end().await;
}

#[tokio::test]
async fn r9_retired_settlement_then_bind_refusal() {
    retired_settlement_then_bind_refusal(false).await;
}

#[tokio::test]
async fn reattested_retired_transfer_permits_compensation() {
    retired_settlement_then_bind_refusal(true).await;
}

async fn retired_settlement_then_bind_refusal(restored: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("coins").await;
    db.runtime.tick().await.unwrap();
    let first = newest_intent(&db, uid("30"), Command::Bind).await;
    let transfer = id(text(&first.value, "transfer_operation_id").unwrap()).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"3\",\"transfer_operation_id\":\"{transfer}\",\"transfer_binding_digest\":\"{}\"}}",
        uid("31"),
        uid("32"),
        text(&first.value, "transfer_binding_digest").unwrap()
    );
    committed_fact(db.runtime.authority(), &mut tx, &first, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let receipt = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishCharacterCommandReceiptV1\",\"source_authority\":\"oteryn:character-authority:primary\",\"command\":\"TransferCharacterOwnership\",\"operation_id\":\"{transfer}\",\"outcome\":\"REJECTED\",\"result_code\":\"CHAR_CMD_INTENT_EXPIRED\",\"recovery_generation\":\"3\",\"decided_at\":\"{now}\"}}"
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(transfer)
        .bind(&receipt)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let replacement = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_ne!(replacement.bytes(), first.bytes());
    reject(&db, &replacement, "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
    if restored {
        let mut a = db.runtime.authority().clone();
        a.fence.recovery_generation = 4;
        let mut tx = db.pool.begin().await.unwrap();
        recovery_tests::replay(&a, &mut tx, uid("30")).await;
        assert!(
            a.settlement_bind_refused(&mut tx, uid("30"))
                .await
                .unwrap()
                .is_none()
        );
        sqlx::query("UPDATE character_commands SET state='PENDING',receipt_json=NULL WHERE id=$1")
            .bind(transfer)
            .execute(&mut *tx)
            .await
            .unwrap();
        let current = receipt.replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"4""#,
        );
        sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
            .bind(transfer)
            .bind(current)
            .execute(&mut *tx)
            .await
            .unwrap();
        assert!(
            a.settlement_bind_refused(&mut tx, uid("30"))
                .await
                .unwrap()
                .is_some()
        );
        tx.rollback().await.unwrap();
    }
    db.runtime.tick().await.unwrap();
    let returning = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_eq!(
        text(&returning.value, "purpose").unwrap(),
        "BAZAAR_ESCROW_RETURN"
    );
    db.end().await;
}

#[tokio::test]
async fn r9_restored_escrow_staging() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("coins").await;
    let mut tx = db.pool.begin().await.unwrap();
    let mut recovered = db.runtime.authority().clone();
    recovered.fence.recovery_generation = 4;
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let listing = uid("30");
    for command in [Command::Lock, Command::Bind] {
        let intent = newest_intent(&db, listing, command).await;
        let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
        let raw: String = sqlx::query_scalar("SELECT receipt FROM bazaar_authority_receipts WHERE operation_id=$1 AND recovery_generation=3").bind(operation).fetch_one(&mut *tx).await.unwrap();
        let replay = raw.replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"4\"",
        );
        let result = recovered
            .receive(
                &mut tx,
                &recovered.config.receipt_authority,
                replay.as_bytes(),
                now,
            )
            .await;
        assert!(
            matches!(result, Ok(ReceiptResult::Stored)),
            "replayed {command:?}: {result:?}"
        );
        assert_eq!(
            audit(&recovered, &mut tx, &intent, &replay, now).await,
            ReceiptResult::Stored
        );
    }
    let (transfer, old_receipt): (Uuid,String) = sqlx::query_as("SELECT c.id,c.receipt_json FROM character_commands c JOIN bazaar_current_transfers t ON t.operation_id=c.id WHERE t.listing_id=$1").bind(listing).fetch_one(&mut *tx).await.unwrap();
    sqlx::query("UPDATE character_commands SET state='PENDING',receipt_json=NULL WHERE id=$1")
        .bind(transfer)
        .execute(&mut *tx)
        .await
        .unwrap();
    let replay = old_receipt.replace(
        "\"recovery_generation\":\"3\"",
        "\"recovery_generation\":\"4\"",
    );
    sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1")
        .bind(transfer)
        .bind(replay)
        .execute(&mut *tx)
        .await
        .unwrap();
    let row = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let status = recovered
        .projection(&mut tx, "STATUS", listing)
        .await
        .unwrap()
        .unwrap();
    assert!(
        runtime::characters::custody(&recovered, &mut tx, &row, &status, "locked")
            .await
            .unwrap()
            .is_some()
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r10_suspend_settled_character() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("coins").await;
    let mut tx = db.pool.begin().await.unwrap();
    let result = crate::moderation::execute_staff(
        &mut tx,
        uid("11"),
        &crate::moderation::StaffCommand::Suspend {
            listing: uid("30"),
            reason: "staff suspended sale".into(),
        },
    )
    .await;
    assert!(
        matches!(result, Err(crate::Error::IllegalTransition)),
        "suspension stranded settled sale"
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn r10_replay_winning_hold() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("gold").await;
    replay_sale_receipts(&db, &[Command::Hold]).await;
    db.end().await;
}

async fn replay_sale_receipts(db: &BrowserFixture, commands: &[Command]) {
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let mut recovered = db.runtime.authority().clone();
    recovered.fence.recovery_generation = 4;
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    for &command in commands {
        let intent = newest_intent(db, uid("30"), command).await;
        let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
        let raw: String = sqlx::query_scalar("SELECT receipt FROM bazaar_authority_receipts WHERE operation_id=$1 AND recovery_generation=3").bind(operation).fetch_one(&mut *tx).await.unwrap();
        let replay = raw.replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"4""#,
        );
        let result = recovered
            .receive(
                &mut tx,
                &recovered.config.receipt_authority,
                replay.as_bytes(),
                now,
            )
            .await;
        assert!(
            matches!(result, Ok(ReceiptResult::Stored)),
            "{command:?} reattestation: {result:?}"
        );
        assert_eq!(
            audit(&recovered, &mut tx, &intent, &replay, now).await,
            ReceiptResult::Stored
        );
    }
    tx.rollback().await.unwrap();
}

async fn delivered_character_sale(currency: &str) -> BrowserFixture {
    let db = settling_character_fixture(currency).await;
    finish_character_handoff(&db, uid("13"), "DELIVERED").await;
    db
}
async fn finish_character_handoff(db: &BrowserFixture, recipient: Uuid, status: &str) {
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(db, uid("30"), Command::Bind).await;
    let transfer = id(text(&bind.value, "transfer_operation_id").unwrap()).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"3","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
        uid("31"),
        uid("32"),
        text(&bind.value, "transfer_binding_digest").unwrap()
    );
    committed_fact(db.runtime.authority(), &mut tx, &bind, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    character_transfer_receipt(&mut tx, transfer, (uid("40"), recipient), now).await;
    character_status(db, &mut tx, status, transfer, ("3", "3"), now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
}

#[tokio::test]
async fn r10_replay_captured_coin_bind() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = delivered_character_sale("coins").await;
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT completed FROM bazaar_coin_holds WHERE id=$1")
            .bind(uid("30"))
            .fetch_one(&db.pool)
            .await
            .unwrap()
    );
    replay_sale_receipts(&db, &[Command::Lock, Command::Bind]).await;
    db.end().await;
}

#[tokio::test]
async fn r10_held_item_refusal_refunds() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    item_wallet_scenario(true, false, ItemScenario::HeldSale).await;
}

#[tokio::test]
async fn r10_replay_gold_payout_before_resync() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = delivered_character_sale("gold").await;
    let payout = newest_intent(&db, uid("30"), Command::Payout).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"2","sale_id":"{}","gross_amount":"100","fee_amount":"3","seller_amount":"97","remaining_held":"0","buyer_ledger_entry_id":"{}","seller_ledger_entry_id":"{}","burn_entry_id":"{}","delivery_operation_id":"{}","delivery_receipt_digest":"{}"}}"#,
        uid("34"),
        uid("33"),
        uid("51"),
        uid("52"),
        uid("53"),
        text(&payout.value, "delivery_operation_id").unwrap(),
        text(&payout.value, "delivery_receipt_digest").unwrap()
    );
    committed_fact(db.runtime.authority(), &mut tx, &payout, &data, now).await;
    tx.commit().await.unwrap();
    replay_sale_receipts(&db, &[Command::Payout]).await;
    db.end().await;
}

#[tokio::test]
async fn restricted_buyer_blocks_settlement_bind_publication() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = settling_character_fixture("coins").await;
    sqlx::query(
        "UPDATE accounts SET email_change_blocked_until=now()+interval '1 hour' WHERE id=$1",
    )
    .bind(uid("13"))
    .execute(&db.pool)
    .await
    .unwrap();
    db.runtime.tick().await.unwrap();
    let published: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch JOIN bazaar_game_outbox USING(operation_id) WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer' AND intent::jsonb->>'purpose'='BAZAAR_SETTLEMENT')")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    db.end().await;
    assert!(!published, "settlement Bind published for restricted buyer");
}

#[tokio::test]
async fn r13_replay_gold_bind() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = delivered_character_sale("gold").await;
    replay_sale_receipts(&db, &[Command::Lock, Command::Bind]).await;
    db.end().await;
}

#[path = "recovery_tests.rs"]
mod recovery_tests;

mod worker_tests {
    use super::*;
    #[tokio::test]
    async fn seller_ban_after_settlement_still_delivers() {
        let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
        let db = expired_item_with_bid(1).await;
        db.runtime.tick().await.unwrap();
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(uid("11"))
            .execute(&db.pool)
            .await
            .unwrap();
        db.runtime.tick().await.unwrap();
        let issued:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox JOIN bazaar_authority_dispatch USING(operation_id) WHERE command='DeliverBazaarItem' AND listing_id=$1)").bind(uid("20")).fetch_one(&db.pool).await.unwrap();
        assert!(issued, "seller suspension blocked delivery publication");
        recovery_tests::item_effect(&db, Command::Deliver, false).await;
        db.runtime.tick().await.unwrap();
        db.runtime.tick().await.unwrap();
        let state: crate::State =
            sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
                .bind(uid("20"))
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(state, crate::State::Completed);
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM wallet_entries WHERE account_id=$1 AND reserved_delta<0",
        )
        .bind(uid("13"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
        db.end().await;
    }

    #[tokio::test]
    async fn restricted_buyer_before_delivery_gets_refund_after_return() {
        let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
        let db = expired_item_with_bid(1).await;
        db.runtime.tick().await.unwrap();
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(uid("13"))
            .execute(&db.pool)
            .await
            .unwrap();
        db.runtime.tick().await.unwrap();
        recovery_tests::item_effect(&db, Command::Return, false).await;
        db.runtime.tick().await.unwrap();
        let state: crate::State =
            sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
                .bind(uid("20"))
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(state, crate::State::Cancelled);
        let balance:(i64,i64)=sqlx::query_as("SELECT sum(available_delta)::bigint,sum(reserved_delta)::bigint FROM wallet_entries WHERE account_id=$1").bind(uid("13")).fetch_one(&db.pool).await.unwrap();
        assert_eq!(balance, (10000, 0));
        db.end().await;
    }
}

#[path = "lifecycle_table_tests.rs"]
mod lifecycle_table_tests;
