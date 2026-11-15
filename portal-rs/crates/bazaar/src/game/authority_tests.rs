#![allow(clippy::unwrap_used, clippy::panic)]
use super::authority_wire::{decode, number};
use super::game_wire::{digest, id, text};
use super::*;
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
fn uid(s: &str) -> Uuid {
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
    sqlx::query("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,18,$6,$7,3,3)").bind(op).bind(format!("fixture:{}",Uuid::now_v7())).bind(uid("20")).bind(uid("11")).bind(intent.command().wire()).bind(std::str::from_utf8(intent.bytes()).unwrap()).bind(intent.binding_digest()).execute(&mut *tx).await.unwrap();
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
    let payload = BazaarAuthorityDecisionV1 {
        intent_sha256: Sha256::digest(intent.bytes()).to_vec(),
        receipt_sha256: Sha256::digest(receipt.as_bytes()).to_vec(),
        recovery_generation: 3,
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
        world_id: Some(uid("12").as_bytes().to_vec()),
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
    .unwrap()
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
    sqlx::query("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,3,3)").bind(id(text(&intent.value,"operation_id").unwrap()).unwrap()).bind(format!("fixture:{}",Uuid::now_v7())).bind(uid("20")).bind(uid("11")).bind(intent.command().wire()).bind(text(&intent.value,"source_revision").unwrap()).bind(std::str::from_utf8(intent.bytes()).unwrap()).bind(intent.binding_digest()).execute(&mut **tx).await.unwrap();
}
#[tokio::test]
async fn postgres_item_receipt_to_verified_delivery_to_wallet_completion() {
    item_wallet_scenario(false).await;
}
#[tokio::test]
async fn postgres_delivery_refusal_allows_return_and_refunds_fenced_coins() {
    let started = std::time::Instant::now();
    std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    item_wallet_scenario(true).await;
    eprintln!(
        "postgres_delivery_refusal_allows_return_and_refunds_fenced_coins: {:?}",
        started.elapsed()
    );
}
async fn item_wallet_scenario(compensate: bool) {
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
        funds.release(&mut tx, hold.id, hold.account).await.unwrap();
        funds.release(&mut tx, hold.id, hold.account).await.unwrap();
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
    // The seller's proceeds repay the seller's debt first. A buyer debt that
    // arose after the reservation does not stop the fill from completing.
    savepoint(&mut tx, "SAVEPOINT seller_debt").await;
    let seller_debt = owe(&mut tx, sale.seller, 4000).await;
    owe(&mut tx, sale.buyer, 500).await;
    funds.complete(&mut tx, &sale).await.unwrap();
    funds.complete(&mut tx, &sale).await.unwrap();
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
    // This isolated concurrency harness creates only Bazaar/Wallet tables. The
    // production migration also references account identities for browser keys.
    sqlx::query("CREATE TABLE accounts(id uuid PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    for migration in [
        include_str!("../../../../migrations/20261105151420_t26_wallet.sql"),
        include_str!("../../../../migrations/20261105152830_t57_bazaar_core.sql"),
        include_str!("../../../../migrations/20261105230101_t58_game_outbox.sql"),
        include_str!("../../../../migrations/20261106090001_plat3_bazaar_authority.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
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
