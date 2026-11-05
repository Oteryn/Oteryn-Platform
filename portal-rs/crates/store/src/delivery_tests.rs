#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use ::http::{Request, StatusCode};
use axum::{
    Router,
    body::{Body, to_bytes},
};
use oteryn_internal_api::guard::{Binding, Identity, VerifiedClient};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
const SOURCE: &str = "oteryn-game/store-consumer";
const PEER: &str = "store-delivery-test";
fn settings() -> Settings {
    Settings {
        delivery_enabled: true,
        environment: "testing".into(),
        source_authority: SOURCE.into(),
        peer_identity: PEER.into(),
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
fn val(raw: &[u8]) -> Value {
    decode(raw, 4096).unwrap()
}
fn set(v: &mut Value, key: &str, new: Value) {
    let Value::Object(fields) = v else {
        panic!("object")
    };
    fields.iter_mut().find(|(k, _)| k == key).unwrap().1 = new;
}
fn change(raw: &[u8], key: &str, new: Value) -> Vec<u8> {
    let mut v = val(raw);
    set(&mut v, key, new);
    canonical::encode(&v).unwrap()
}
fn page_request(reconcile: bool, revision: u64, id: Uuid, max: u64) -> Vec<u8> {
    encode(
        &object([
            ("contract_version", number(1)),
            (
                "operation",
                text(if reconcile {
                    "ListStoreOperationStatesV1"
                } else {
                    "ListPendingStoreOperationsV1"
                }),
            ),
            ("source_authority", text(SOURCE)),
            ("after_source_revision", text(revision)),
            ("after_operation_id", text(id)),
            ("max_entries", number(max)),
        ]),
        512,
    )
    .unwrap()
}
fn read_request(kind: &str, id: Uuid) -> Vec<u8> {
    encode(
        &object([
            ("contract_version", number(1)),
            ("operation", text("ReadStoreOperationIntentV1")),
            ("source_authority", text(SOURCE)),
            ("issuer_authority", text(ISSUER)),
            ("operation_kind", text(kind)),
            ("operation_id", text(id)),
        ]),
        512,
    )
    .unwrap()
}
fn entries(v: &Value, reconcile: bool) -> &[Value] {
    let Some(Value::Array(a)) = v.get(if reconcile { "operations" } else { "pending" }) else {
        panic!("array")
    };
    a
}
#[test]
fn contract_fixtures_and_closed_receipt_members() {
    for name in [
        "receipt.deliver-item.committed.json",
        "receipt.deliver-premium-time.committed.json",
        "receipt.claim.committed.json",
        "receipt.revoke-unclaimed.rejected-already-claimed.json",
        "receipt.reclaim.committed-shortfall.json",
        "receipt.flag.committed.json",
    ] {
        let raw = fixture(name);
        let v = val(&raw);
        head(&v, "PublishStoreDeliveryReceiptV1").unwrap();
        receipt(&v).unwrap();
        assert!(decode(&[raw.clone(), b"\n".to_vec()].concat(), 1024).is_err());
        assert!(receipt(&val(&change(&raw, "result_code", text("UNKNOWN")))).is_err());
        for field in ["product_version", "recovery_generation", "decided_at"] {
            assert!(receipt(&val(&change(&raw, field, text("18446744073709551616")))).is_err());
        }
        let mut reordered = v.clone();
        if let Value::Object(f) = &mut reordered {
            f.swap(3, 4);
        }
        assert!(receipt(&reordered).is_err());
        let mut extra = v.clone();
        if let Value::Object(f) = &mut extra {
            f.push(("unknown".into(), number(1)));
        }
        assert!(receipt(&extra).is_err());
    }
    assert!(decode(br#"{"a":1,"a":1}"#, 512).is_err());
}
#[test]
fn cursor_pair_and_limits_are_strict() {
    assert!(page(&val(&page_request(false, u64::MAX, Uuid::now_v7(), 32))).is_ok());
    for (r, id, n) in [
        (0, Uuid::now_v7(), 1),
        (1, Uuid::nil(), 1),
        (0, Uuid::nil(), 0),
        (0, Uuid::nil(), 33),
    ] {
        assert!(page(&val(&page_request(false, r, id, n))).is_err());
    }
}
#[tokio::test]
async fn flags_fail_closed_and_allow_delivery_without_purchases() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    assert!(!Delivery::new(pool.clone(), Settings::default()).enabled());
    assert!(Delivery::new(pool.clone(), settings()).enabled());
    for field in ["production", "unknown"] {
        let mut s = settings();
        s.environment = field.into();
        assert!(!Delivery::new(pool.clone(), s).enabled());
    }
}
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    delivery: Delivery,
}
impl Db {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T30 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("delivery_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(12)
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
        oteryn_db::migrate(&pool).await.unwrap();
        let delivery = Delivery::new(pool.clone(), settings());
        Some(Self {
            pool,
            admin,
            schema,
            delivery,
        })
    }
    async fn close(self) {
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
    async fn seed(&self, item: bool) -> (Uuid, Vec<u8>, Vec<u8>) {
        let mut intent = val(&fixture(if item {
            "intent.deliver-item.json"
        } else {
            "intent.deliver-premium-time.json"
        }));
        let id = Uuid::now_v7();
        let entitlement = Uuid::now_v7();
        let account = Uuid::now_v7();
        let order = Uuid::now_v7();
        let request = Uuid::now_v7();
        let line = Uuid::now_v7();
        let wallet = Uuid::now_v7();
        let revision = "a".repeat(64);
        sqlx::query("INSERT INTO store_price_list_imports(catalog_revision,canonical_bytes,imported_by) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(&revision).bind(b"{}".as_slice()).bind(account).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO store_offers(catalog_revision,offer_key,currency,unit_price,max_quantity,purchasable,canonical_bytes) VALUES($1,'test','COINS',1,1,true,$2) ON CONFLICT DO NOTHING").bind(&revision).bind(b"{}".as_slice()).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
            .bind(account)
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'store_purchase',$2,-1)").bind(wallet).bind(account).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO store_purchase_requests(purchase_request_id,account_id,request_bytes,request_digest,result,answer_bytes) VALUES($1,$2,$3,$4,'ACCEPTED',$3)").bind(request).bind(account).bind(b"{}".as_slice()).bind("b".repeat(64)).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO store_orders(order_id,purchase_request_id,account_id,character_id,world_id,wallet_operation_id,wallet_idempotency_key,currency,total_price) VALUES($1,$2,$3,$4,$5,$6,$7,'COINS',1)").bind(order).bind(request).bind(account).bind(Uuid::now_v7()).bind(Uuid::now_v7()).bind(wallet).bind(wallet.to_string()).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO store_order_lines(line_id,order_id,catalog_revision,offer_key,quantity,unit_price) VALUES($1,$2,$3,'test',1,1)").bind(line).bind(order).bind(&revision).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO store_fulfilment_units(entitlement_id,line_id,unit_index,account_id,product_id,product_version,unit_kind,grant_key,grant_revision,quantity) VALUES($1,$2,0,$3,$4,$5,$6,$7,$8,$9)").bind(entitlement).bind(line).bind(account).bind(get(&intent,"product_id").unwrap()).bind(get(&intent,"product_version").unwrap()).bind(get(&intent,"unit_kind").unwrap()).bind(get(&intent,"grant_key").unwrap()).bind(get(&intent,"grant_revision").unwrap()).bind(integer(&intent,"quantity",1,65535).unwrap() as i32).execute(&self.pool).await.unwrap();
        let sequence: i64 = sqlx::query_scalar(
            "SELECT nextval(pg_get_serial_sequence('store_operations','source_revision'))",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();
        for (field, value) in [
            ("operation_id", text(id)),
            ("entitlement_id", text(entitlement)),
            ("account_id", text(account)),
            ("order_id", text(order)),
            ("source_revision", text(sequence)),
            ("catalog_revision", text(revision)),
        ] {
            set(&mut intent, field, value);
        }
        let bytes = canonical::encode(&intent).unwrap();
        sqlx::query("INSERT INTO store_operations(source_revision,operation_id,issuer_authority,operation_kind,entitlement_id,intent_bytes,binding_digest) VALUES($1,$2,$3,'DELIVER',$4,$5,$6)").bind(sequence).bind(id).bind(ISSUER).bind(entitlement).bind(&bytes).bind(format!("{:x}",Sha256::digest(&bytes))).execute(&self.pool).await.unwrap();
        let mut receipt = val(&fixture(if item {
            "receipt.deliver-item.committed.json"
        } else {
            "receipt.deliver-premium-time.committed.json"
        }));
        for field in ["operation_id", "entitlement_id", "account_id"] {
            set(&mut receipt, field, intent.get(field).unwrap().clone());
        }
        set(
            &mut receipt,
            "binding_digest",
            text(format!("{:x}", Sha256::digest(&bytes))),
        );
        (id, bytes, canonical::encode(&receipt).unwrap())
    }
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}
#[tokio::test]
async fn postgres_delivery_exact_read_receipt_replay_and_reconcile() {
    let Some(db) = Db::new().await else { return };
    let (id, intent, receipt) = db.seed(false).await;
    assert_eq!(
        db.delivery
            .read(&read_request("DELIVER", id))
            .await
            .unwrap(),
        intent
    );
    assert_eq!(
        entries(
            &val(&db
                .delivery
                .pending(&page_request(false, 0, Uuid::nil(), 32))
                .await
                .unwrap()),
            false
        )
        .len(),
        1
    );
    assert_eq!(db.delivery.receipt(&receipt).await.unwrap(), ACCEPTED);
    assert_eq!(db.delivery.receipt(&receipt).await.unwrap(), ACCEPTED);
    assert_eq!(db.count("store_receipts").await, 1);
    let before = db.count("store_operations").await;
    for _ in 0..3 {
        let v = val(&db
            .delivery
            .reconcile(&page_request(true, 0, Uuid::nil(), 32))
            .await
            .unwrap());
        let e = &entries(&v, true)[0];
        assert_eq!(get(e, "state").unwrap(), "COMMITTED");
        assert_eq!(
            get(e, "receipt_digest").unwrap(),
            format!("{:x}", Sha256::digest(&receipt))
        );
    }
    assert_eq!(db.count("store_operations").await, before);
    assert!(
        entries(
            &val(&db
                .delivery
                .pending(&page_request(false, 0, Uuid::nil(), 32))
                .await
                .unwrap()),
            false
        )
        .is_empty()
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_concurrent_first_receipts_and_conflicts() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    let (a, b) = tokio::join!(db.delivery.receipt(&raw), db.delivery.receipt(&raw));
    assert_eq!(a.unwrap(), ACCEPTED);
    assert_eq!(b.unwrap(), ACCEPTED);
    assert_eq!(
        db.delivery
            .receipt(&change(&raw, "decided_at", text("123")))
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(db.count("store_receipts").await, 1);
    db.close().await;
}
#[tokio::test]
async fn postgres_refusals_do_not_change_delivery() {
    let Some(db) = Db::new().await else { return };
    let (id, _, raw) = db.seed(false).await;
    for (field, new, error) in [
        ("binding_digest", text("0".repeat(64)), Error::Conflict),
        ("account_id", text(Uuid::now_v7()), Error::Invalid),
        ("product_version", text("99"), Error::Invalid),
        ("delivered_quantity", number(1), Error::Invalid),
        ("operation_id", text(Uuid::now_v7()), Error::UnknownAccount),
    ] {
        assert_eq!(
            db.delivery.receipt(&change(&raw, field, new)).await,
            Err(error)
        );
    }
    assert_eq!(
        db.delivery.read(&read_request("CLAIM", id)).await,
        Err(Error::UnknownAccount)
    );
    assert_eq!(db.count("store_receipts").await, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_rejected_delivery_enters_manual_review() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    let mut v = val(&raw);
    set(&mut v, "outcome", text("REJECTED"));
    set(&mut v, "result_code", text("STORE_ACCOUNT_NOT_FOUND"));
    if let Value::Object(f) = &mut v {
        f.retain(|(k, _)| k != "delivered_quantity");
    }
    let raw = canonical::encode(&v).unwrap();
    db.delivery.receipt(&raw).await.unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM store_fulfilment_units")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, "MANUAL_REVIEW");
    assert_eq!(db.count("store_operations").await, 1);
    db.close().await;
}
#[tokio::test]
async fn postgres_item_delivery_creates_one_stable_claim() {
    let Some(db) = Db::new().await else { return };
    let (id, _, raw) = db.seed(true).await;
    db.delivery.receipt(&raw).await.unwrap();
    db.delivery.receipt(&raw).await.unwrap();
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT intent_bytes FROM store_operations WHERE operation_kind='CLAIM'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let v = val(&bytes);
    assert_eq!(get(&v, "delivery_operation_id").unwrap(), id.to_string());
    assert_ne!(get(&v, "operation_id").unwrap(), id.to_string());
    assert_eq!(db.count("store_operations").await, 2);
    let mut claim = val(&fixture("receipt.claim.committed.json"));
    for field in [
        "operation_id",
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
    ] {
        set(&mut claim, field, v.get(field).unwrap().clone());
    }
    set(
        &mut claim,
        "binding_digest",
        text(format!("{:x}", Sha256::digest(&bytes))),
    );
    set(&mut claim, "recovery_generation", text("6"));
    db.delivery
        .receipt(&canonical::encode(&claim).unwrap())
        .await
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM store_fulfilment_units")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, "CLAIMED");
    // Restore loses DELIVER at generation 7 but preserves the earlier CLAIM.
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        db.delivery
            .reopen_in(&mut tx, Uuid::now_v7(), 7, 8)
            .await
            .unwrap(),
        1
    );
    tx.commit().await.unwrap();
    db.delivery
        .receipt(&change(&raw, "recovery_generation", text("8")))
        .await
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM store_fulfilment_units")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, "CLAIMED");
    assert_eq!(db.count("store_operations").await, 2);
    db.close().await;
}
#[tokio::test]
async fn postgres_restore_reopens_retains_history_and_fences_old_receipts() {
    let Some(db) = Db::new().await else { return };
    let (id, intent, raw) = db.seed(true).await;
    db.delivery.receipt(&raw).await.unwrap();
    let event = Uuid::now_v7();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        db.delivery.reopen_in(&mut tx, event, 7, 8).await.unwrap(),
        1
    );
    tx.commit().await.unwrap();
    assert_eq!(db.delivery.receipt(&raw).await, Err(Error::Conflict));
    let listing = val(&db
        .delivery
        .reconcile(&page_request(true, 0, Uuid::nil(), 32))
        .await
        .unwrap());
    assert_eq!(
        get(&entries(&listing, true)[0], "state").unwrap(),
        "REOPENED"
    );
    assert_eq!(
        db.delivery
            .read(&read_request("DELIVER", id))
            .await
            .unwrap(),
        intent
    );
    let next = change(&raw, "recovery_generation", text("8"));
    db.delivery.receipt(&next).await.unwrap();
    assert_eq!(db.count("store_receipts").await, 2);
    assert_eq!(db.count("store_operations").await, 2);
    let unresolved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM store_reconcile_state WHERE resolved_receipt_id IS NULL",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(unresolved, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_restore_carries_unresolved_and_preserves_current_exact_replay() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    db.delivery.receipt(&raw).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        db.delivery
            .reopen_in(&mut tx, Uuid::now_v7(), 8, 9)
            .await
            .unwrap(),
        0
    );
    tx.commit().await.unwrap();
    assert_eq!(db.delivery.receipt(&raw).await.unwrap(), ACCEPTED);
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        db.delivery
            .reopen_in(&mut tx, Uuid::now_v7(), 7, 10)
            .await
            .unwrap(),
        1
    );
    tx.commit().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        db.delivery
            .reopen_in(&mut tx, Uuid::now_v7(), 9, 11)
            .await
            .unwrap(),
        1
    );
    tx.commit().await.unwrap();
    assert_eq!(db.count("store_reconcile_state").await, 2);
    db.delivery
        .receipt(&change(&raw, "recovery_generation", text("11")))
        .await
        .unwrap();
    db.close().await;
}
#[tokio::test]
async fn postgres_pages_cover_all_operations_and_uint64_end() {
    let Some(db) = Db::new().await else { return };
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(db.seed(false).await.0);
    }
    let mut revision = 0;
    let mut cursor = Uuid::nil();
    let mut seen = Vec::new();
    loop {
        let v = val(&db
            .delivery
            .pending(&page_request(false, revision, cursor, 1))
            .await
            .unwrap());
        let a = entries(&v, false);
        if a.is_empty() {
            break;
        }
        revision = counter(get(&a[0], "source_revision").unwrap(), false).unwrap();
        cursor = uuid(get(&a[0], "operation_id").unwrap()).unwrap();
        seen.push(cursor);
    }
    assert_eq!(seen, ids);
    assert!(
        entries(
            &val(&db
                .delivery
                .reconcile(&page_request(true, u64::MAX, Uuid::now_v7(), 32))
                .await
                .unwrap()),
            true
        )
        .is_empty()
    );
    db.close().await;
}
async fn call(
    router: Router,
    path: &str,
    raw: Vec<u8>,
    peer: Option<&str>,
    header: Option<(&str, &str)>,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(raw))
        .unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(peer).unwrap(),
            ));
    }
    if let Some((k, v)) = header {
        request.headers_mut().insert(
            ::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
            v.parse().unwrap(),
        );
    }
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
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
    let bytes = to_bytes(response.into_body(), 16384)
        .await
        .unwrap()
        .to_vec();
    if status != StatusCode::OK {
        assert!(bytes.is_empty());
    }
    (status, bytes)
}
#[tokio::test]
async fn postgres_http_peer_transport_canonical_and_source_order() {
    let Some(db) = Db::new().await else { return };
    let app = http::router(db.delivery.clone(), vec![]);
    let raw = page_request(false, 0, Uuid::nil(), 32);
    assert_eq!(
        call(app.clone(), http::PENDING_PATH, raw.clone(), None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            raw.clone(),
            Some("other"),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            raw.clone(),
            Some(PEER),
            Some(("content-length", "513"))
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            raw.clone(),
            Some(PEER),
            Some(("transfer-encoding", "chunked"))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            change(&raw, "source_authority", text("other")),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            [raw.clone(), b"\n".to_vec()].concat(),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            app,
            http::PENDING_PATH,
            raw,
            Some(PEER),
            Some(("content-type", "Application/JSON; charset=utf-8"))
        )
        .await
        .0,
        StatusCode::OK
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_http_default_off_overlap_and_rate_limit() {
    let Some(db) = Db::new().await else { return };
    let raw = page_request(false, 0, Uuid::nil(), 32);
    let app = http::router(Delivery::new(db.pool.clone(), Settings::default()), vec![]);
    assert_eq!(
        call(app, http::PENDING_PATH, raw.clone(), Some(PEER), None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let app = http::router(
        db.delivery.clone(),
        vec![Binding {
            identity: Identity::san(PEER).unwrap(),
            group: "other".into(),
        }],
    );
    assert_eq!(
        call(app, http::PENDING_PATH, raw.clone(), Some(PEER), None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut s = settings();
    s.requests_per_minute = 1;
    let app = http::router(Delivery::new(db.pool.clone(), s), vec![]);
    assert_eq!(
        call(
            app.clone(),
            http::PENDING_PATH,
            raw.clone(),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(app, http::PENDING_PATH, raw, Some(PEER), None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    db.close().await;
}

#[tokio::test]
async fn postgres_restore_lock_serializes_receipts_and_rollback_keeps_evidence() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    let mut tx = db.pool.begin().await.unwrap();
    lock_restore(&mut tx, true).await.unwrap();
    let service = db.delivery.clone();
    let bytes = raw.clone();
    let task = tokio::spawn(async move { service.receipt(&bytes).await });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    tx.rollback().await.unwrap();
    assert_eq!(task.await.unwrap().unwrap(), ACCEPTED);
    let mut tx = db.pool.begin().await.unwrap();
    db.delivery
        .reopen_in(&mut tx, Uuid::now_v7(), 7, 8)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(db.delivery.receipt(&raw).await.unwrap(), ACCEPTED);
    assert_eq!(db.count("store_reconcile_state").await, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_missing_restore_lock_and_ambiguous_state_deny() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    sqlx::query("DELETE FROM character_restore_locks")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(db.delivery.receipt(&raw).await, Err(Error::Unavailable));
    assert_eq!(db.count("store_receipts").await, 0);
    sqlx::query("UPDATE store_operations SET state='COMMITTED'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.delivery
            .reconcile(&page_request(true, 0, Uuid::nil(), 32))
            .await,
        Err(Error::Unavailable)
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_receipt_history_is_immutable() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(false).await;
    db.delivery.receipt(&raw).await.unwrap();
    assert!(
        sqlx::query("UPDATE store_receipts SET result_code='STORE_UNSUPPORTED'")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM store_receipts")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert_eq!(db.delivery.receipt(&raw).await.unwrap(), ACCEPTED);
    db.close().await;
}
#[tokio::test]
async fn postgres_reclaim_binds_claim_digest_quantity_and_target() {
    let Some(db) = Db::new().await else { return };
    let (_, _, raw) = db.seed(true).await;
    db.delivery.receipt(&raw).await.unwrap();
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT intent_bytes FROM store_operations WHERE operation_kind='CLAIM'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let intent = val(&bytes);
    let mut claim = val(&fixture("receipt.claim.committed.json"));
    for field in [
        "operation_id",
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
    ] {
        set(&mut claim, field, intent.get(field).unwrap().clone());
    }
    set(
        &mut claim,
        "binding_digest",
        text(format!("{:x}", Sha256::digest(&bytes))),
    );
    let claim_raw = canonical::encode(&claim).unwrap();
    db.delivery.receipt(&claim_raw).await.unwrap();
    let mut reclaim_intent = val(&fixture("intent.reclaim.json"));
    let id = Uuid::now_v7();
    for field in [
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
    ] {
        set(
            &mut reclaim_intent,
            field,
            intent.get(field).unwrap().clone(),
        );
    }
    set(&mut reclaim_intent, "operation_id", text(id));
    set(
        &mut reclaim_intent,
        "claim_operation_id",
        intent.get("operation_id").unwrap().clone(),
    );
    set(
        &mut reclaim_intent,
        "claim_receipt_digest",
        text(format!("{:x}", Sha256::digest(&claim_raw))),
    );
    let sequence: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('store_operations','source_revision'))",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    set(&mut reclaim_intent, "source_revision", text(sequence));
    let bytes = canonical::encode(&reclaim_intent).unwrap();
    sqlx::query("INSERT INTO store_operations(source_revision,operation_id,issuer_authority,operation_kind,entitlement_id,intent_bytes,binding_digest) VALUES($1,$2,$3,'RECLAIM',$4,$5,$6)").bind(sequence).bind(id).bind(ISSUER).bind(uuid(get(&intent,"entitlement_id").unwrap()).unwrap()).bind(&bytes).bind(format!("{:x}",Sha256::digest(&bytes))).execute(&db.pool).await.unwrap();
    let mut receipt = val(&fixture("receipt.reclaim.committed-shortfall.json"));
    for field in [
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
    ] {
        set(&mut receipt, field, intent.get(field).unwrap().clone());
    }
    set(&mut receipt, "operation_id", text(id));
    set(
        &mut receipt,
        "binding_digest",
        text(format!("{:x}", Sha256::digest(&bytes))),
    );
    let raw = canonical::encode(&receipt).unwrap();
    assert_eq!(
        db.delivery
            .receipt(&change(&raw, "shortfall_quantity", number(9)))
            .await,
        Err(Error::Invalid)
    );
    assert_eq!(
        db.delivery
            .receipt(&change(&raw, "character_id", text(Uuid::now_v7())))
            .await,
        Err(Error::Invalid)
    );
    db.delivery.receipt(&raw).await.unwrap();
    assert_eq!(db.delivery.receipt(&raw).await.unwrap(), ACCEPTED);
    let state: String = sqlx::query_scalar("SELECT state FROM store_fulfilment_units")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, "REVERSAL_PARTIAL");
    db.close().await;
}

#[tokio::test]
async fn postgres_http_read_receipt_and_reconcile_round_trip() {
    let Some(db) = Db::new().await else { return };
    let (id, intent, receipt) = db.seed(false).await;
    let app = http::router(db.delivery.clone(), vec![]);
    let (status, bytes) = call(
        app.clone(),
        http::READ_PATH,
        read_request("DELIVER", id),
        Some(PEER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, intent);
    assert_eq!(
        call(
            app.clone(),
            http::READ_PATH,
            read_request("CLAIM", id),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            app.clone(),
            http::READ_PATH,
            b"{}".to_vec(),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for _ in 0..2 {
        let (status, body) = call(
            app.clone(),
            http::RECEIPT_PATH,
            receipt.clone(),
            Some(PEER),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ACCEPTED);
    }
    assert_eq!(
        call(
            app.clone(),
            http::RECEIPT_PATH,
            change(&receipt, "decided_at", text("1")),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            app.clone(),
            http::RECEIPT_PATH,
            b"{}".to_vec(),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, body) = call(
        app.clone(),
        http::RECONCILE_PATH,
        page_request(true, 0, Uuid::nil(), 32),
        Some(PEER),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        get(&entries(&val(&body), true)[0], "state").unwrap(),
        "COMMITTED"
    );
    assert_eq!(
        call(
            app.clone(),
            http::RECONCILE_PATH,
            page_request(true, 0, Uuid::nil(), 32),
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app,
            http::RECONCILE_PATH,
            page_request(true, 0, Uuid::nil(), 33),
            Some(PEER),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db.count("store_receipts").await, 1);
    db.close().await;
}
