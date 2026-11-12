#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::catalog::{
    AccountState as CatalogAccountState, Action, Catalog, CatalogAccess, CatalogFlags, CatalogStore,
};
use ::http::{Request, StatusCode};
use axum::{
    Router,
    body::{Body, to_bytes},
};
use oteryn_internal_api::{
    canonical::Value,
    guard::{Binding, Identity, VerifiedClient},
};
use oteryn_wallet::{
    AccountStateReader, AdjustmentAudit, AuditHook, Ledger, TransferBlockCheck, TransferPermission,
    WalletFlags,
};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;
const ACCOUNT: &str = "0192b6a0-4c00-7000-8000-000000000101";
const PEER: &str = "store-test-peer";
const SOURCE: &str = "oteryn-game/store-consumer";
const PRICE_LIST: &str = r#"{"contract_version":1,"offers":[{"offer_key":"premium-30d","currency":"COINS","unit_price":"250","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.premium_time","product_version":"1","unit_kind":"PREMIUM_TIME","grant_key":"premium_time","grant_revision":"1","quantity":30}]}]}"#;
fn settings() -> Settings {
    Settings {
        purchase_enabled: true,
        delivery_enabled: true,
        environment: "testing".into(),
        source_authority: SOURCE.into(),
        peer_identity: PEER.into(),
        enabled_unit_kinds: vec!["PREMIUM_TIME".into()],
        purchases_per_account_per_minute: 60,
        ..Settings::default()
    }
}
fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/store-wire-v1")
        .join(name);
    canonical::encode(&serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap())
        .unwrap()
}
fn value(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn set(value: &mut Value, key: &str, new: Value) {
    let Value::Object(members) = value else {
        panic!("object required")
    };
    let member = members.iter_mut().find(|(k, _)| k == key).unwrap();
    member.1 = new;
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    let Some(Value::String(s)) = value.get(key) else {
        panic!("string required: {key}")
    };
    s
}
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    let Some(Value::Array(values)) = value.get(key) else {
        panic!("array required: {key}")
    };
    values
}
fn request(revision: &str) -> Vec<u8> {
    let mut value = value(&fixture("purchase-request.valid.json"));
    // SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87,
    // crates/gameplay/native-admission-source/src/store_wire.rs: catalog revisions
    // are SHA-256 after owner Q2, as in Laravel StorePurchaseInput.php.
    set(&mut value, "catalog_revision", text(revision));
    canonical::encode(&value).unwrap()
}
fn change(raw: &[u8], key: &str, new: Value) -> Vec<u8> {
    let mut value = value(raw);
    set(&mut value, key, new);
    canonical::encode(&value).unwrap()
}
fn unique(raw: &[u8]) -> Vec<u8> {
    change(raw, "purchase_request_id", text(Uuid::now_v7()))
}
fn read_request(history: bool, account: &str, before: &str, max: u8) -> Vec<u8> {
    let mut value = value(&fixture(if history {
        "history-request.valid.json"
    } else {
        "balance-request.valid.json"
    }));
    set(&mut value, "account_id", text(account));
    if history {
        set(&mut value, "before_entry_id", text(before));
        set(&mut value, "max_entries", number(u64::from(max)));
    }
    canonical::encode(&value).unwrap()
}
#[test]
fn wire_schemas_carry_owner_decision_d1() {
    let schema = |name: &str| -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/contracts/fixtures/store-wire-v1")
            .join(name);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    let pattern = serde_json::json!("^[a-z0-9][a-z0-9_.:-]{0,127}$");
    let coins = serde_json::json!(["COINS"]);
    for (name, pointer, expected) in [
        (
            "purchase-request.schema.json",
            "/properties/offer_key/pattern",
            &pattern,
        ),
        (
            "purchase-request.schema.json",
            "/properties/currency/enum",
            &coins,
        ),
        (
            "intent.schema.json",
            "/oneOf/0/properties/offer_key/pattern",
            &pattern,
        ),
        (
            "history-response.schema.json",
            "/properties/entries/items/properties/offer_key/oneOf/0/pattern",
            &pattern,
        ),
        (
            "history-response.schema.json",
            "/properties/entries/items/properties/currency/enum",
            &coins,
        ),
        (
            "purchase-answer.schema.json",
            "/oneOf/0/properties/currency/enum",
            &coins,
        ),
        (
            "balance-response.schema.json",
            "/properties/balances/items/properties/currency/enum",
            &coins,
        ),
        (
            "balance-response.schema.json",
            "/properties/balances/minItems",
            &serde_json::json!(1),
        ),
        (
            "balance-response.schema.json",
            "/properties/balances/maxItems",
            &serde_json::json!(1),
        ),
    ] {
        assert_eq!(
            schema(name).pointer(pointer),
            Some(expected),
            "{name}{pointer}"
        );
    }
}
#[test]
fn wire_requests_are_closed_canonical_and_bounded() {
    for history in [false, true] {
        let bytes = read_request(history, ACCOUNT, "18446744073709551615", 32);
        assert!(ReadRequest::decode(&bytes, history).is_ok());
        assert!(ReadRequest::decode(&[bytes.clone(), b"\n".to_vec()].concat(), history).is_err());
    }
    let raw = request(&"a".repeat(64));
    assert!(PurchaseRequest::decode(&raw).is_ok());
    // The shared fixture carries the hex64 revision (contract §15 Q2); the
    // obsolete numeric form is deliberately refused.
    assert!(PurchaseRequest::decode(&fixture("purchase-request.valid.json")).is_ok());
    assert!(PurchaseRequest::decode(&request("37")).is_err());
    // Game offer keys keep `:` and reach 128 bytes (owner D1, 2026-11-02).
    for key in [
        "store:exercise-sword".to_owned(),
        "0.a_b:c-d".to_owned(),
        format!("a{}", "b".repeat(127)),
    ] {
        assert!(
            PurchaseRequest::decode(&change(&raw, "offer_key", text(&key))).is_ok(),
            "{key}"
        );
    }
    for (key, new) in [
        ("currency", text("TRANSFERABLE_COINS")),
        ("offer_key", text("")),
        ("offer_key", text(":store")),
        ("offer_key", text("_store")),
        ("offer_key", text("store/sword")),
        ("offer_key", text("store sword")),
        ("offer_key", text(format!("a{}", "b".repeat(128)))),
        ("contract_version", number(2)),
        ("operation", text("Other")),
        ("source_authority", text("bad source")),
        (
            "purchase_request_id",
            text("0192B6A0-4c00-7000-8000-000000000401"),
        ),
        ("account_id", text(Uuid::nil())),
        ("character_id", text("invalid")),
        ("world_id", text("invalid")),
        ("catalog_revision", text("A".repeat(64))),
        ("offer_key", text("Upper")),
        ("currency", text("GOLD")),
        ("unit_price", text("0")),
        ("unit_price", text("01")),
        ("unit_price", text("9223372036854775808")),
        ("quantity", number(0)),
        ("quantity", number(101)),
        ("quantity", text("1")),
    ] {
        assert!(
            PurchaseRequest::decode(&change(&raw, key, new)).is_err(),
            "{key}"
        );
    }
    for bad in [
        format!("{}\n", String::from_utf8(raw.clone()).unwrap()),
        String::from_utf8(raw.clone()).unwrap().replacen(
            "\"contract_version\":1",
            "\"contract_version\":1,\"contract_version\":1",
            1,
        ),
        String::from_utf8(raw.clone())
            .unwrap()
            .replace("premium-30d", "premium\\u002d30d"),
    ] {
        assert!(PurchaseRequest::decode(bad.as_bytes()).is_err());
    }
    for before in ["01", "-1", "18446744073709551616"] {
        assert!(ReadRequest::decode(&read_request(true, ACCOUNT, before, 32), true).is_err());
    }
    for max in [0, 33] {
        assert!(ReadRequest::decode(&read_request(true, ACCOUNT, "0", max), true).is_err());
    }
    assert!(PurchaseRequest::decode(&vec![b' '; 1025]).is_err());
    assert!(PurchaseRequest::decode(&[]).is_err());
}
#[test]
fn configuration_is_default_off_and_fails_closed() {
    assert!(!Settings::default().valid());
    assert!(!Settings::from_lookup(|_| None).valid());
    assert!(settings().valid());
    let mut cases = Vec::new();
    let mut s = settings();
    s.delivery_enabled = false;
    cases.push(s);
    let mut s = settings();
    s.environment = "production".into();
    cases.push(s);
    let mut s = settings();
    s.requests_per_minute = 0;
    cases.push(s);
    let mut s = settings();
    s.max_pending_operations = 100001;
    cases.push(s);
    let mut s = settings();
    s.source_authority.clear();
    cases.push(s);
    let mut s = settings();
    s.peer_identity = "\n".into();
    cases.push(s);
    let mut s = settings();
    s.enabled_unit_kinds = vec!["OTHER".into()];
    cases.push(s);
    let mut s = settings();
    s.enabled_unit_kinds.push("PREMIUM_TIME".into());
    cases.push(s);
    for s in cases {
        assert!(!s.valid());
    }
}
#[derive(Debug, Clone, Copy)]
struct TestAccounts;
impl Accounts for TestAccounts {
    async fn state(&self, c: &mut PgConnection, id: Uuid) -> Result<AccountState, Error> {
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM purchase_test_accounts WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(c)
                .await?;
        Ok(match state.as_deref() {
            Some("active") => AccountState::Active,
            Some("restricted") => AccountState::Restricted,
            None => AccountState::Missing,
            _ => AccountState::Unknown,
        })
    }
}
impl AccountStateReader for TestAccounts {
    async fn state(&self, c: &mut PgConnection, id: Uuid) -> oteryn_wallet::AccountState {
        match Accounts::state(self, c, id).await {
            Ok(AccountState::Active) => oteryn_wallet::AccountState::Active,
            Ok(AccountState::Restricted) => oteryn_wallet::AccountState::Inactive,
            _ => oteryn_wallet::AccountState::Unknown,
        }
    }
}
impl CatalogAccess for TestAccounts {
    async fn account_state(
        &self,
        _: Uuid,
        _: &mut PgConnection,
    ) -> Result<CatalogAccountState, crate::catalog::Error> {
        Ok(CatalogAccountState::Active)
    }
    async fn permitted(
        &self,
        _: Uuid,
        _: Action,
        _: &mut PgConnection,
    ) -> Result<bool, crate::catalog::Error> {
        Ok(true)
    }
}
#[derive(Debug, Clone, Copy)]
struct Blocks;
impl TransferBlockCheck for Blocks {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Blocked
    }
}
#[derive(Debug, Clone)]
struct Audit {
    deny: bool,
    pause: Option<Arc<tokio::sync::Semaphore>>,
}
impl AuditHook for Audit {
    async fn adjustment(&self, c: &mut PgConnection, event: &AdjustmentAudit) -> bool {
        if self.deny {
            return false;
        }
        if let Some(pause) = &self.pause {
            let permit = pause.acquire().await.unwrap();
            permit.forget();
        }
        sqlx::query("INSERT INTO purchase_test_audit(id) VALUES($1)")
            .bind(event.operation_id)
            .execute(c)
            .await
            .is_ok()
    }
}
type TestWallet = Ledger<TestAccounts, Blocks, Audit>;
type TestPurchases = Purchases<TestAccounts, Blocks, Audit, TestAccounts>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    revision: String,
}
impl Database {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T29 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("purchase_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(24)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(&mut *c)
                        .await?;
                    sqlx::query("SET statement_timeout='15s'")
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        sqlx::raw_sql("CREATE TABLE purchase_test_accounts(id uuid PRIMARY KEY,state text NOT NULL);CREATE TABLE purchase_test_audit(id uuid PRIMARY KEY)").execute(&pool).await.unwrap();
        let catalog = Catalog::parse(PRICE_LIST.as_bytes()).unwrap();
        let db = Self {
            pool,
            admin,
            schema,
            revision: catalog.revision().into(),
        };
        db.account(ACCOUNT, "active").await;
        let store = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
        let actor = uuid(ACCOUNT).unwrap();
        store
            .import(PRICE_LIST.as_bytes(), actor, &TestAccounts)
            .await
            .unwrap();
        store
            .activate(&db.revision, actor, &TestAccounts)
            .await
            .unwrap();
        Some(db)
    }
    async fn account(&self, id: &str, state: &str) {
        sqlx::query("INSERT INTO purchase_test_accounts(id,state) VALUES($1,$2) ON CONFLICT(id) DO UPDATE SET state=excluded.state").bind(uuid(id).unwrap()).bind(state).execute(&self.pool).await.unwrap();
    }
    fn wallet(&self) -> TestWallet {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            TestAccounts,
            Blocks,
            Audit {
                deny: false,
                pause: None,
            },
        )
    }
    fn service(&self, s: Settings) -> TestPurchases {
        Purchases::new(self.pool.clone(), s, self.wallet(), TestAccounts)
    }
    async fn fund(&self, account: &str, amount: i64) {
        self.wallet()
            .adjust(
                Uuid::now_v7(),
                uuid(account).unwrap(),
                amount,
                Uuid::now_v7(),
                "test funding",
            )
            .await
            .unwrap();
    }
    async fn count(&self, table: &str) -> i64 {
        // Call sites pass only test-owned constant table identifiers.
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn finish(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
#[tokio::test]
async fn db_wire_golden_purchase_balance_history_and_intent() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1250).await;
    let service = db.service(settings());
    let balance = service
        .read(&read_request(false, ACCOUNT, "0", 32), false)
        .await
        .unwrap();
    let actual = value(&balance);
    let mut expected = value(&fixture("balance-response.valid.json"));
    set(
        &mut expected,
        "read_at",
        actual.get("read_at").unwrap().clone(),
    );
    assert_eq!(balance, canonical::encode(&expected).unwrap());
    let raw = request(&db.revision);
    let bytes = service.purchase(&raw).await.unwrap();
    let actual = value(&bytes);
    let mut expected = value(&fixture("purchase-answer.accepted.json"));
    for key in ["request_digest", "order_id", "units", "decided_at"] {
        set(&mut expected, key, actual.get(key).unwrap().clone());
    }
    assert_eq!(
        string(&actual, "request_digest"),
        format!("{:x}", Sha256::digest(&raw))
    );
    assert_eq!(bytes, canonical::encode(&expected).unwrap());
    assert_eq!(array(&actual, "units").len(), 1);
    let unit = &array(&actual, "units")[0];
    for key in ["entitlement_id", "delivery_operation_id"] {
        assert!(uuid(string(unit, key)).is_ok());
    }
    assert_eq!(string(unit, "unit_kind"), "PREMIUM_TIME");
    let row = sqlx::query("SELECT intent_bytes,binding_digest FROM store_operations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let intent: Vec<u8> = row.get("intent_bytes");
    let v = value(&intent);
    let mut expected = value(&fixture("intent.deliver-premium-time.json"));
    for key in [
        "source_revision",
        "operation_id",
        "order_id",
        "entitlement_id",
        "catalog_revision",
        "issued_at_source",
    ] {
        set(&mut expected, key, v.get(key).unwrap().clone());
    }
    assert_eq!(intent, canonical::encode(&expected).unwrap());
    assert_eq!(
        row.get::<String, _>("binding_digest"),
        format!("{:x}", Sha256::digest(&intent))
    );
    assert_eq!(
        string(&v, "operation_id"),
        string(unit, "delivery_operation_id")
    );
    assert_eq!(string(&v, "entitlement_id"), string(unit, "entitlement_id"));
    let history = service
        .read(&read_request(true, ACCOUNT, "0", 32), true)
        .await
        .unwrap();
    let v = value(&history);
    let entries = array(&v, "entries");
    assert_eq!(entries.len(), 2);
    let mut expected = value(&fixture("history-response.valid.json"));
    let mut gold = array(&expected, "entries").to_vec();
    for i in 0..2 {
        for key in ["entry_id", "created_at"] {
            set(&mut gold[i], key, entries[i].get(key).unwrap().clone());
        }
    }
    set(
        &mut gold[0],
        "order_id",
        actual.get("order_id").unwrap().clone(),
    );
    set(&mut gold[1], "kind", text("OPERATOR_ADJUSTMENT"));
    set(&mut gold[1], "available_delta", text("1250"));
    set(&mut expected, "entries", Value::Array(gold));
    set(&mut expected, "next_before_entry_id", text("0"));
    assert_eq!(history, canonical::encode(&expected).unwrap());
    assert_eq!(db.count("store_orders").await, 1);
    assert_eq!(db.count("store_operations").await, 1);
    assert_eq!(db.count("wallet_operations").await, 2);
    assert_eq!(db.count("purchase_test_audit").await, 2);
    let kind: String = sqlx::query_scalar("SELECT kind FROM wallet_operations WHERE id=$1")
        .bind(uuid(string(&value(&raw), "purchase_request_id")).unwrap())
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(kind, "store_purchase");
    db.finish().await;
}
#[tokio::test]
async fn db_exact_replay_and_conflicting_reuse() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1000).await;
    let service = db.service(settings());
    let raw = request(&db.revision);
    let first = service.purchase(&raw).await.unwrap();
    db.account(ACCOUNT, "restricted").await;
    sqlx::query("UPDATE store_active_catalog SET catalog_revision=NULL WHERE id=1")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(service.purchase(&raw).await.unwrap(), first);
    assert_eq!(
        service.purchase(&change(&raw, "quantity", number(2))).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        service
            .purchase(&change(&raw, "account_id", text(Uuid::now_v7())))
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(db.count("wallet_entries").await, 2);
    assert_eq!(db.count("store_purchase_requests").await, 1);
    assert_eq!(db.count("store_operations").await, 1);
    assert_eq!(db.count("purchase_test_audit").await, 2);
    db.finish().await;
}
#[tokio::test]
async fn db_refusals_are_terminal_and_create_only_answers() {
    let Some(db) = Database::new().await else {
        return;
    };
    let service = db.service(settings());
    let raw = request(&db.revision);
    let insufficient = service.purchase(&raw).await.unwrap();
    let v = value(&insufficient);
    let mut golden = value(&fixture("purchase-answer.insufficient-balance.json"));
    for key in ["request_digest", "decided_at"] {
        set(&mut golden, key, v.get(key).unwrap().clone());
    }
    assert_eq!(insufficient, canonical::encode(&golden).unwrap());
    assert_eq!(db.count("wallet_accounts").await, 0);
    assert_eq!(db.count("wallet_operations").await, 0);
    assert_eq!(db.count("purchase_test_audit").await, 0);
    db.fund(ACCOUNT, 1000).await;
    assert_eq!(service.purchase(&raw).await.unwrap(), insufficient);
    for (key, new, result) in [
        ("catalog_revision", text("0".repeat(64)), "STALE_QUOTE"),
        ("offer_key", text("missing"), "OFFER_UNAVAILABLE"),
        ("unit_price", text("251"), "STALE_QUOTE"),
        ("quantity", number(11), "OFFER_UNAVAILABLE"),
    ] {
        let input = change(&unique(&raw), key, new);
        let bytes = service.purchase(&input).await.unwrap();
        assert_eq!(string(&value(&bytes), "result"), result);
        assert_eq!(service.purchase(&input).await.unwrap(), bytes);
    }
    // D1: the Store wire carries `COINS` only, so another currency is a bad request.
    let answers = db.count("store_purchase_requests").await;
    let other = change(&unique(&raw), "currency", text("TRANSFERABLE_COINS"));
    assert!(matches!(
        service.purchase(&other).await,
        Err(Error::Invalid)
    ));
    assert_eq!(db.count("store_purchase_requests").await, answers);
    let mut off = settings();
    off.enabled_unit_kinds.clear();
    let bytes = db.service(off).purchase(&unique(&raw)).await.unwrap();
    assert_eq!(string(&value(&bytes), "result"), "PURCHASE_DISABLED");
    for state in ["restricted", "unknown"] {
        db.account(ACCOUNT, state).await;
        let bytes = service.purchase(&unique(&raw)).await.unwrap();
        assert_eq!(string(&value(&bytes), "result"), "ACCOUNT_RESTRICTED");
    }
    let bytes = service
        .purchase(&change(&unique(&raw), "account_id", text(Uuid::now_v7())))
        .await
        .unwrap();
    assert_eq!(string(&value(&bytes), "result"), "ACCOUNT_RESTRICTED");
    assert_eq!(db.count("store_orders").await, 0);
    assert_eq!(db.count("wallet_entries").await, 1);
    assert_eq!(db.count("store_operations").await, 0);
    db.finish().await;
}
#[tokio::test]
async fn db_concurrent_purchases_and_identical_retries_charge_once() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1000).await;
    let service = Arc::new(db.service(settings()));
    let raw = request(&db.revision);
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let service = service.clone();
        let input = unique(&raw);
        tasks.push(tokio::spawn(async move {
            service.purchase(&input).await.unwrap()
        }));
    }
    let mut accepted = 0;
    let mut refused = 0;
    for task in tasks {
        match string(&value(&task.await.unwrap()), "result") {
            "ACCEPTED" => accepted += 1,
            "INSUFFICIENT_BALANCE" => refused += 1,
            other => panic!("{other}"),
        }
    }
    assert_eq!((accepted, refused), (4, 12));
    assert_eq!(
        db.wallet()
            .balance(uuid(ACCOUNT).unwrap())
            .await
            .unwrap()
            .available,
        0
    );
    db.fund(ACCOUNT, 250).await;
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let service = service.clone();
        let input = raw.clone();
        tasks.push(tokio::spawn(async move {
            service.purchase(&input).await.unwrap()
        }));
    }
    let mut answers = Vec::new();
    for task in tasks {
        answers.push(task.await.unwrap());
    }
    assert!(answers.iter().all(|a| a == &answers[0]));
    assert_eq!(string(&value(&answers[0]), "result"), "ACCEPTED");
    assert_eq!(db.count("store_orders").await, 5);
    assert_eq!(db.count("store_operations").await, 5);
    assert_eq!(db.count("wallet_entries").await, 7);
    assert_eq!(db.count("store_purchase_requests").await, 17);
    db.finish().await;
}

#[tokio::test]
async fn db_history_cursor_is_stable_private_and_zero_reads_do_not_create_rows() {
    let Some(db) = Database::new().await else {
        return;
    };
    let service = db.service(settings());
    let zero = service
        .read(&read_request(false, ACCOUNT, "0", 32), false)
        .await
        .unwrap();
    assert_eq!(
        string(&array(&value(&zero), "balances")[0], "available"),
        "0"
    );
    assert_eq!(db.count("wallet_accounts").await, 0);
    let empty = service
        .read(&read_request(true, ACCOUNT, "0", 32), true)
        .await
        .unwrap();
    assert!(array(&value(&empty), "entries").is_empty());
    assert_eq!(db.count("wallet_accounts").await, 0);
    let other = Uuid::now_v7().to_string();
    db.account(&other, "active").await;
    db.fund(&other, 999).await;
    for _ in 0..3 {
        db.fund(ACCOUNT, 250).await;
    }
    service.purchase(&request(&db.revision)).await.unwrap();
    let first = value(
        &service
            .read(&read_request(true, ACCOUNT, "0", 1), true)
            .await
            .unwrap(),
    );
    assert_eq!(array(&first, "entries").len(), 1);
    let cursor = string(&first, "next_before_entry_id");
    assert_ne!(cursor, "0");
    assert!(cursor.parse::<u64>().is_ok());
    let raw = read_request(true, ACCOUNT, cursor, 2);
    let older = service.read(&raw, true).await.unwrap();
    db.fund(ACCOUNT, 25).await;
    assert_eq!(service.read(&raw, true).await.unwrap(), older);
    let older = value(&older);
    let entries = array(&older, "entries");
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| string(e,"entry_id").parse::<u64>().unwrap()<cursor.parse::<u64>().unwrap()));
    assert!(
        entries
            .iter()
            .all(|e| string(e, "available_delta") == "250")
    );
    for e in entries {
        assert_eq!(e.get("order_id"), Some(&Value::Null));
        assert_eq!(e.get("offer_key"), Some(&Value::Null));
        assert_eq!(e.get("quantity"), Some(&Value::Null));
    }
    let oldest = service
        .read(
            &read_request(true, ACCOUNT, string(&older, "next_before_entry_id"), 32),
            true,
        )
        .await
        .unwrap();
    let oldest = value(&oldest);
    assert_eq!(array(&oldest, "entries").len(), 1);
    assert_eq!(string(&oldest, "next_before_entry_id"), "0");
    let all = value(
        &service
            .read(
                &read_request(true, ACCOUNT, "18446744073709551615", 32),
                true,
            )
            .await
            .unwrap(),
    );
    assert_eq!(array(&all, "entries").len(), 5);
    assert!(
        !String::from_utf8(canonical::encode(&all).unwrap())
            .unwrap()
            .contains("operation_id")
    );
    assert_eq!(
        service
            .read(
                &read_request(false, &Uuid::now_v7().to_string(), "0", 32),
                false
            )
            .await,
        Err(Error::UnknownAccount)
    );
    db.account(ACCOUNT, "restricted").await;
    assert!(
        service
            .read(&read_request(false, ACCOUNT, "0", 32), false)
            .await
            .is_ok()
    );
    db.account(ACCOUNT, "unknown").await;
    assert_eq!(
        service
            .read(&read_request(false, ACCOUNT, "0", 32), false)
            .await,
        Err(Error::Unavailable)
    );
    db.finish().await;
}
#[tokio::test]
async fn db_storage_and_audit_failures_roll_back_debit_answer_order_and_outbox() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1000).await;
    sqlx::raw_sql("CREATE FUNCTION reject_purchase_intent() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test insertion failure'; END $$;CREATE TRIGGER reject_intent BEFORE INSERT ON store_operations FOR EACH STATEMENT EXECUTE FUNCTION reject_purchase_intent()").execute(&db.pool).await.unwrap();
    let raw = request(&db.revision);
    let service = db.service(settings());
    assert_eq!(service.purchase(&raw).await, Err(Error::Unavailable));
    for table in [
        "store_orders",
        "store_purchase_requests",
        "store_operations",
        "store_order_lines",
        "store_fulfilment_units",
    ] {
        assert_eq!(db.count(table).await, 0, "{table}");
    }
    assert_eq!(db.count("wallet_entries").await, 1);
    assert_eq!(db.count("wallet_operations").await, 1);
    assert_eq!(db.count("purchase_test_audit").await, 1);
    assert_eq!(
        db.wallet()
            .balance(uuid(ACCOUNT).unwrap())
            .await
            .unwrap()
            .available,
        1000
    );
    sqlx::query("DROP TRIGGER reject_intent ON store_operations")
        .execute(&db.pool)
        .await
        .unwrap();
    let wallet = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        TestAccounts,
        Blocks,
        Audit {
            deny: true,
            pause: None,
        },
    );
    assert_eq!(
        Purchases::new(db.pool.clone(), settings(), wallet, TestAccounts)
            .purchase(&raw)
            .await,
        Err(Error::Unavailable)
    );
    assert_eq!(db.count("store_purchase_requests").await, 0);
    // The exact same request can succeed after the transient failure is repaired.
    assert_eq!(
        string(&value(&service.purchase(&raw).await.unwrap()), "result"),
        "ACCEPTED"
    );
    assert_eq!(db.count("wallet_entries").await, 2);
    assert_eq!(db.count("store_operations").await, 1);
    db.finish().await;
}
#[tokio::test]
async fn db_pending_capacity_is_atomic_across_accounts_and_retryable() {
    let Some(db) = Database::new().await else {
        return;
    };
    let other = Uuid::now_v7().to_string();
    db.account(&other, "active").await;
    db.fund(ACCOUNT, 250).await;
    db.fund(&other, 250).await;
    let mut s = settings();
    s.max_pending_operations = 1;
    let service = Arc::new(db.service(s));
    let raw = request(&db.revision);
    let second = change(&unique(&raw), "account_id", text(&other));
    let a = service.clone();
    let b = service.clone();
    let aa = raw.clone();
    let bb = second.clone();
    let (a, b) = tokio::join!(a.purchase(&aa), b.purchase(&bb));
    assert!(a.is_ok() != b.is_ok());
    assert!(a == Err(Error::Unavailable) || b == Err(Error::Unavailable));
    assert_eq!(db.count("store_orders").await, 1);
    assert_eq!(db.count("store_purchase_requests").await, 1);
    assert_eq!(db.count("wallet_entries").await, 3);
    sqlx::query("UPDATE store_operations SET state='COMMITTED'")
        .execute(&db.pool)
        .await
        .unwrap();
    let retry = if a.is_err() { raw } else { second };
    assert_eq!(
        string(&value(&service.purchase(&retry).await.unwrap()), "result"),
        "ACCEPTED"
    );
    assert_eq!(db.count("store_orders").await, 2);
    db.finish().await;
}
#[tokio::test]
async fn db_catalog_activation_waits_for_purchase_decision() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1000).await;
    let changed = PRICE_LIST.replace("\"250\"", "\"300\"");
    let catalog = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let actor = uuid(ACCOUNT).unwrap();
    let next = catalog
        .import(changed.as_bytes(), actor, &TestAccounts)
        .await
        .unwrap();
    let pause = Arc::new(tokio::sync::Semaphore::new(0));
    let wallet = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        TestAccounts,
        Blocks,
        Audit {
            deny: false,
            pause: Some(pause.clone()),
        },
    );
    let service = Arc::new(Purchases::new(
        db.pool.clone(),
        settings(),
        wallet,
        TestAccounts,
    ));
    let input = request(&db.revision);
    let s = service.clone();
    let task = tokio::spawn(async move { s.purchase(&input).await.unwrap() });
    // Wait for the observable uncommitted account/catalog locks, without sleeps.
    // A lock timeout proves the pointer cannot be activated during the debit.
    let mut connection = db.pool.acquire().await.unwrap();
    let mut seen = false;
    for _ in 0..1000 {
        let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation='store_active_catalog'::regclass AND mode='RowShareLock' AND granted AND pid<>pg_backend_pid())").fetch_one(&mut *connection).await.unwrap();
        if waiting {
            seen = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(seen);
    let mut activation = Box::pin(catalog.activate(&next, actor, &TestAccounts));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut activation)
            .await
            .is_err()
    );
    pause.add_permits(1);
    let answer = task.await.unwrap();
    assert_eq!(string(&value(&answer), "total_price"), "250");
    activation.await.unwrap();
    let bytes = db
        .service(settings())
        .purchase(&unique(&request(&db.revision)))
        .await
        .unwrap();
    assert_eq!(string(&value(&bytes), "result"), "STALE_QUOTE");
    drop(connection);
    db.finish().await;
}
#[tokio::test]
async fn db_wallet_conflicts_and_unknown_state_deny_without_stored_answer() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 1000).await;
    let raw = request(&db.revision);
    let key = uuid(string(&value(&raw), "purchase_request_id")).unwrap();
    db.wallet()
        .adjust(
            key,
            uuid(ACCOUNT).unwrap(),
            1,
            Uuid::now_v7(),
            "unrelated adjustment",
        )
        .await
        .unwrap();
    assert_eq!(
        db.service(settings()).purchase(&raw).await,
        Err(Error::Unavailable)
    );
    assert_eq!(db.count("store_purchase_requests").await, 0);
    assert_eq!(db.count("store_orders").await, 0);
    assert_eq!(
        db.wallet()
            .balance(uuid(ACCOUNT).unwrap())
            .await
            .unwrap()
            .available,
        1001
    );
    let mut tx = db.pool.begin().await.unwrap();
    let wallet = db
        .wallet()
        .with_adjustment_kind(oteryn_wallet::AdjustmentKind::StorePurchase);
    for (amount, actor, reason) in [
        (1, uuid(ACCOUNT).unwrap(), format!("store-purchase:{key}")),
        (-1, Uuid::now_v7(), format!("store-purchase:{key}")),
        (-1, uuid(ACCOUNT).unwrap(), "bad reason".into()),
    ] {
        assert!(
            wallet
                .adjust_in(
                    &mut tx,
                    Uuid::now_v7(),
                    uuid(ACCOUNT).unwrap(),
                    amount,
                    actor,
                    &reason
                )
                .await
                .is_err()
        );
    }
    assert!(
        wallet
            .numeric_history_in(
                &mut tx,
                uuid(ACCOUNT).unwrap(),
                oteryn_wallet::NumericHistoryPage {
                    before: 0,
                    limit: 0
                }
            )
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    db.finish().await;
}
fn http_request(path: &str, raw: Vec<u8>, peer: bool) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(raw))
        .unwrap();
    if peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(PEER).unwrap(),
            ));
    }
    request
}
async fn call(router: Router, request: Request<Body>, status: StatusCode) -> Vec<u8> {
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["cache-control"], "no-store");
    if status == StatusCode::TOO_MANY_REQUESTS {
        assert!(
            response.headers()["retry-after"]
                .to_str()
                .unwrap()
                .parse::<u32>()
                .unwrap()
                >= 1
        );
    }
    let bytes = to_bytes(response.into_body(), 12289)
        .await
        .unwrap()
        .to_vec();
    if status != StatusCode::OK {
        assert!(bytes.is_empty());
    }
    bytes
}
#[tokio::test]
async fn db_http_guard_transport_and_source_failures_are_empty() {
    let Some(db) = Database::new().await else {
        return;
    };
    let raw = read_request(false, ACCOUNT, "0", 32);
    let router = http::router(db.service(settings()), vec![]);
    call(
        router.clone(),
        http_request(http::BALANCE_PATH, raw.clone(), true),
        StatusCode::OK,
    )
    .await;
    let mut forged = http_request(http::BALANCE_PATH, raw.clone(), false);
    forged.headers_mut().insert(
        "ssl-client-verify",
        ::http::HeaderValue::from_static("SUCCESS"),
    );
    call(router.clone(), forged, StatusCode::UNAUTHORIZED).await;
    let mut wrong = http_request(http::BALANCE_PATH, raw.clone(), false);
    wrong
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(
            Identity::san("other").unwrap(),
        ));
    call(router.clone(), wrong, StatusCode::UNAUTHORIZED).await;
    call(
        http::router(db.service(Settings::default()), vec![]),
        http_request(http::BALANCE_PATH, vec![], false),
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    let overlap = vec![Binding {
        identity: Identity::san(PEER).unwrap(),
        group: "premium".into(),
    }];
    call(
        http::router(db.service(settings()), overlap),
        http_request(http::BALANCE_PATH, raw.clone(), true),
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    let alias = vec![Binding {
        identity: Identity::san("other").unwrap(),
        group: http::GROUP.into(),
    }];
    call(
        http::router(db.service(settings()), alias),
        http_request(http::BALANCE_PATH, raw.clone(), true),
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    call(
        router.clone(),
        http_request(
            http::BALANCE_PATH,
            change(&raw, "source_authority", text("other-source")),
            true,
        ),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    call(
        router.clone(),
        http_request(
            http::BALANCE_PATH,
            [raw.clone(), b"\n".to_vec()].concat(),
            true,
        ),
        StatusCode::BAD_REQUEST,
    )
    .await;
    for (name, value, status) in [
        ("content-length", "513", StatusCode::PAYLOAD_TOO_LARGE),
        ("content-length", "00", StatusCode::PAYLOAD_TOO_LARGE),
        ("transfer-encoding", "chunked", StatusCode::BAD_REQUEST),
        ("content-encoding", "gzip", StatusCode::BAD_REQUEST),
        ("content-type", "text/plain", StatusCode::BAD_REQUEST),
    ] {
        let mut r = http_request(http::BALANCE_PATH, raw.clone(), true);
        r.headers_mut().insert(
            ::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            ::http::HeaderValue::from_str(value).unwrap(),
        );
        call(router.clone(), r, status).await;
    }
    call(
        router.clone(),
        http_request(
            http::BALANCE_PATH,
            read_request(false, &Uuid::now_v7().to_string(), "0", 32),
            true,
        ),
        StatusCode::NOT_FOUND,
    )
    .await;
    call(
        router.clone(),
        http_request(
            http::HISTORY_PATH,
            read_request(true, ACCOUNT, "0", 32),
            true,
        ),
        StatusCode::OK,
    )
    .await;
    let mut parameters = http_request(http::BALANCE_PATH, raw.clone(), true);
    parameters.headers_mut().insert(
        "content-type",
        ::http::HeaderValue::from_static("Application/JSON; charset=utf-8"),
    );
    parameters.headers_mut().insert(
        "content-encoding",
        ::http::HeaderValue::from_static("IDENTITY"),
    );
    call(router.clone(), parameters, StatusCode::OK).await;
    call(
        router.clone(),
        http_request(http::BALANCE_PATH, vec![b' '; 513], true),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let mut both = http_request(http::BALANCE_PATH, raw.clone(), true);
    both.headers_mut()
        .insert("content-length", ::http::HeaderValue::from_static("513"));
    both.headers_mut()
        .insert("content-type", ::http::HeaderValue::from_static("bad"));
    call(router.clone(), both, StatusCode::PAYLOAD_TOO_LARGE).await;
    let wrong_source = change(
        &read_request(true, ACCOUNT, "bad", 32),
        "source_authority",
        text("other-source"),
    );
    call(
        router.clone(),
        http_request(http::HISTORY_PATH, wrong_source, true),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    let purchase = request(&db.revision);
    let bytes = call(
        router.clone(),
        http_request(http::PURCHASE_PATH, purchase.clone(), true),
        StatusCode::OK,
    )
    .await;
    assert_eq!(string(&value(&bytes), "result"), "INSUFFICIENT_BALANCE");
    call(
        router,
        http_request(
            http::PURCHASE_PATH,
            change(&purchase, "quantity", number(2)),
            true,
        ),
        StatusCode::CONFLICT,
    )
    .await;
    db.finish().await;
}
#[tokio::test]
async fn db_peer_and_account_limits_have_contract_precedence_and_shared_scope() {
    let Some(db) = Database::new().await else {
        return;
    };
    let mut s = settings();
    s.requests_per_minute = 2;
    let router = http::router(db.service(s), vec![]);
    for _ in 0..2 {
        call(
            router.clone(),
            http_request(http::BALANCE_PATH, vec![], true),
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    call(
        router.clone(),
        http_request(http::BALANCE_PATH, vec![], true),
        StatusCode::TOO_MANY_REQUESTS,
    )
    .await;
    call(
        router,
        http_request(http::BALANCE_PATH, vec![], false),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    sqlx::query("DELETE FROM store_wire_limits")
        .execute(&db.pool)
        .await
        .unwrap();
    let mut s = settings();
    s.purchases_per_account_per_minute = 1;
    s.reads_per_account_per_minute = 1;
    let a = http::router(db.service(s.clone()), vec![]);
    let b = http::router(db.service(s), vec![]);
    let raw = request(&db.revision);
    call(
        a.clone(),
        http_request(http::PURCHASE_PATH, raw.clone(), true),
        StatusCode::OK,
    )
    .await;
    call(
        b.clone(),
        http_request(http::PURCHASE_PATH, raw, true),
        StatusCode::TOO_MANY_REQUESTS,
    )
    .await;
    call(
        a,
        http_request(
            http::BALANCE_PATH,
            read_request(false, ACCOUNT, "0", 32),
            true,
        ),
        StatusCode::OK,
    )
    .await;
    call(
        b,
        http_request(
            http::HISTORY_PATH,
            read_request(true, ACCOUNT, "0", 32),
            true,
        ),
        StatusCode::TOO_MANY_REQUESTS,
    )
    .await;
    assert_eq!(db.count("store_purchase_requests").await, 1);
    db.finish().await;
}

#[tokio::test]
async fn db_offer_bounds_and_unit_flags_follow_first_refusal_order() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(ACCOUNT, 10000).await;
    let catalog = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let actor = uuid(ACCOUNT).unwrap();
    for (document, price, quantity, expected, kinds) in [
        (
            PRICE_LIST.replace("\"purchasable\":true", "\"purchasable\":false"),
            "251",
            1,
            "OFFER_UNAVAILABLE",
            vec![],
        ),
        (
            PRICE_LIST.replace("PREMIUM_TIME", "ITEM"),
            "251",
            1,
            "PURCHASE_DISABLED",
            vec!["PREMIUM_TIME"],
        ),
        (
            PRICE_LIST.replace("PREMIUM_TIME", "COSMETIC"),
            "250",
            1,
            "PURCHASE_DISABLED",
            vec![],
        ),
        (
            PRICE_LIST.replace("COINS", "TRANSFERABLE_COINS"),
            "250",
            1,
            "STALE_QUOTE",
            vec!["PREMIUM_TIME"],
        ),
        (
            PRICE_LIST.replace("\"250\"", "\"9223372036854775807\""),
            "9223372036854775807",
            2,
            "OFFER_UNAVAILABLE",
            vec!["PREMIUM_TIME"],
        ),
        (
            PRICE_LIST.replace("\"quantity\":30", "\"quantity\":65535"),
            "250",
            2,
            "OFFER_UNAVAILABLE",
            vec!["PREMIUM_TIME"],
        ),
        (
            PRICE_LIST.replace("PREMIUM_TIME", "ITEM"),
            "250",
            2,
            "ACCEPTED",
            vec!["ITEM"],
        ),
        (
            PRICE_LIST.replace("PREMIUM_TIME", "COSMETIC"),
            "250",
            1,
            "ACCEPTED",
            vec!["COSMETIC"],
        ),
    ] {
        let revision = catalog
            .import(document.as_bytes(), actor, &TestAccounts)
            .await
            .unwrap();
        catalog
            .activate(&revision, actor, &TestAccounts)
            .await
            .unwrap();
        let mut raw = unique(&request(&revision));
        raw = change(&raw, "unit_price", text(price));
        raw = change(&raw, "quantity", number(quantity));
        let mut s = settings();
        s.enabled_unit_kinds = kinds.into_iter().map(str::to_owned).collect();
        let bytes = db.service(s).purchase(&raw).await.unwrap();
        assert_eq!(string(&value(&bytes), "result"), expected, "{document}");
    }
    assert_eq!(db.count("store_orders").await, 2);
    assert_eq!(db.count("wallet_entries").await, 3);
    let quantities: Vec<i32> =
        sqlx::query_scalar("SELECT quantity FROM store_fulfilment_units ORDER BY quantity")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(quantities, vec![30, 60]);
    db.finish().await;
}
#[tokio::test]
async fn db_wallet_upgrade_preserves_existing_rows_and_uuid_history() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED T29 PostgreSQL upgrade test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("purchase_upgrade_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let search = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |c, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105151420_t26_wallet.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE purchase_test_accounts(id uuid PRIMARY KEY,state text NOT NULL);CREATE TABLE purchase_test_audit(id uuid PRIMARY KEY)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO purchase_test_accounts VALUES($1,'active')")
        .bind(uuid(ACCOUNT).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let wallet = Ledger::new(
        pool.clone(),
        WalletFlags { enabled: true },
        TestAccounts,
        Blocks,
        Audit {
            deny: false,
            pause: None,
        },
    );
    let account = uuid(ACCOUNT).unwrap();
    wallet
        .adjust(
            Uuid::now_v7(),
            account,
            1000,
            Uuid::now_v7(),
            "before upgrade",
        )
        .await
        .unwrap();
    wallet.reserve(Uuid::now_v7(), account, 50).await.unwrap();
    let before = wallet
        .history(account, oteryn_wallet::HistoryPage::default())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105220208_t29_wallet_wire.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        wallet
            .history(account, oteryn_wallet::HistoryPage::default())
            .await
            .unwrap(),
        before
    );
    assert_eq!(wallet.balance(account).await.unwrap().available, 950);
    let mut tx = pool.begin().await.unwrap();
    let numeric = wallet
        .numeric_history_in(
            &mut tx,
            account,
            oteryn_wallet::NumericHistoryPage {
                before: 0,
                limit: 32,
            },
        )
        .await
        .unwrap();
    assert_eq!(numeric.len(), 2);
    assert!(numeric[0].sequence > numeric[1].sequence);
    tx.commit().await.unwrap();
    // The default T55 API retains the operator classification after migration.
    let key = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    wallet
        .adjust_in(&mut tx, key, account, -10, Uuid::now_v7(), "after upgrade")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let kind: String = sqlx::query_scalar("SELECT kind FROM wallet_operations WHERE id=$1")
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kind, "adjustment");
    assert!(
        sqlx::query("UPDATE wallet_entries SET available_delta=0")
            .execute(&pool)
            .await
            .is_err()
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
