#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! PostgreSQL tests of the Store purchase route as `Routes::configured` mounts
//! it, with the production account and Wallet adapters, through the internal
//! listener adapter. Answers are compared with `docs/contracts/fixtures/store-wire-v1/`.
use super::*;
use axum::http::{StatusCode, header};
use oteryn_internal_api::guard::VerifiedClient;
use oteryn_store::catalog::{
    AccountState as CatalogState, Action, Catalog, CatalogAccess, CatalogFlags, CatalogStore,
};
use oteryn_wallet::{
    AccountState as WalletState, AccountStateReader, AdjustmentAudit, AuditHook, Ledger,
    TransferBlockCheck, TransferPermission, WalletFlags,
};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, postgres::PgPoolOptions};
use uuid::Uuid;

const ACCOUNT: &str = "0192b6a0-4c00-7000-8000-000000000101";
const MODERATOR: &str = "0192b6a0-4c00-7000-8000-000000000102";
const STORE: &str = "store.oteryn.internal";
const PREMIUM: &str = "premium.oteryn.internal";
const STRANGER: &str = "stranger.oteryn.internal";
const SOURCE: &str = "oteryn-game/store-consumer";
const PURCHASE: &str = oteryn_store::purchase::http::PURCHASE_PATH;
const BALANCE: &str = oteryn_store::purchase::http::BALANCE_PATH;
const PRICE_LIST: &str =
    include_str!("../../../../docs/contracts/fixtures/store-price-list-v1/price-list.valid.json");

fn values() -> Values {
    [
        ("OTERYN_ENV", "testing"),
        ("APP_ENV", "testing"),
        ("PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED", "true"),
        ("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED", "true"),
        ("PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY", SOURCE),
        (
            "PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS",
            r#"["ITEM"]"#,
        ),
        (
            "PRODUCTS_ENTITLEMENTS_STORE_PURCHASES_PER_ACCOUNT_PER_MINUTE",
            "60",
        ),
        (STORE_IDENTITY, STORE),
        (PREMIUM_IDENTITY, PREMIUM),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/store-wire-v1")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn decode(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn set(value: &mut Value, key: &str, new: Value) {
    let Value::Object(members) = value else {
        panic!("object required")
    };
    members.iter_mut().find(|(k, _)| k == key).unwrap().1 = new;
}
fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
fn result(bytes: &[u8]) -> String {
    match decode(bytes).get("result") {
        Some(Value::String(result)) => result.clone(),
        other => panic!("result required: {other:?}"),
    }
}
/// The fixture's request at the active revision, under a fresh or given id.
fn request(revision: &str, id: Option<&str>) -> Vec<u8> {
    request_for(revision, id, ACCOUNT)
}
/// As `request`, for the given buyer.
fn request_for(revision: &str, id: Option<&str>, account: &str) -> Vec<u8> {
    let mut value = fixture("purchase-request.valid.json");
    set(&mut value, "offer_key", text("test-supply"));
    set(&mut value, "unit_price", text("25"));
    set(&mut value, "account_id", text(account));
    set(&mut value, "catalog_revision", text(revision));
    if let Some(id) = id {
        set(&mut value, "purchase_request_id", text(id));
    } else {
        set(&mut value, "purchase_request_id", text(Uuid::now_v7()));
    }
    canonical::encode(&value).unwrap()
}
/// The fixture answer with the members a run generates taken from `actual`.
fn golden(name: &str, actual: &[u8], generated: &[&str]) -> Vec<u8> {
    let actual = decode(actual);
    let mut expected = fixture(name);
    for key in generated {
        set(&mut expected, key, actual.get(key).unwrap().clone());
    }
    canonical::encode(&expected).unwrap()
}
/// Expected answer for the 25-coin item. Only the per-run identifiers and timestamp vary.
fn accepted_item_answer(actual: &[u8]) -> Vec<u8> {
    let decoded = decode(actual);
    let mut expected = decode(&golden(
        "purchase-answer.accepted.json",
        actual,
        &["request_digest", "order_id", "decided_at"],
    ));
    set(&mut expected, "total_price", text("25"));
    set(&mut expected, "available_after", text("1225"));
    let Value::Object(members) = &mut expected else {
        panic!("object required")
    };
    let Some((_, Value::Array(units))) = members.iter_mut().find(|(k, _)| k == "units") else {
        panic!("units required")
    };
    let Some(Value::Array(actual_units)) = decoded.get("units") else {
        panic!("actual units required: {decoded:?}")
    };
    for (unit, actual) in units.iter_mut().zip(actual_units) {
        set(unit, "unit_kind", text("ITEM"));
        for key in ["entitlement_id", "delivery_operation_id"] {
            set(unit, key, actual.get(key).unwrap().clone());
        }
    }
    canonical::encode(&expected).unwrap()
}

/// Wallet policy for funding only; the route under test never uses it.
#[derive(Debug, Clone, Copy)]
struct Funding;
impl AccountStateReader for Funding {
    async fn state(&self, _: &mut PgConnection, _: Uuid) -> WalletState {
        WalletState::Active
    }
}
impl TransferBlockCheck for Funding {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Blocked
    }
}
impl AuditHook for Funding {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
impl CatalogAccess for Funding {
    async fn account_state(
        &self,
        _: Uuid,
        _: &mut PgConnection,
    ) -> Result<CatalogState, oteryn_store::catalog::Error> {
        Ok(CatalogState::Active)
    }
    async fn permitted(
        &self,
        _: Uuid,
        _: Action,
        _: &mut PgConnection,
    ) -> Result<bool, oteryn_store::catalog::Error> {
        Ok(true)
    }
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    revision: String,
}
impl Database {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!(
                "SKIPPED store purchase route PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        // A fixed prefix and generated hex only.
        let schema = format!("store_route_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(16)
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
        for (id, email) in [
            (ACCOUNT, "buyer@example.test"),
            (MODERATOR, "moderator@example.test"),
        ] {
            sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'x','active','en')")
                .bind(Uuid::parse_str(id).unwrap()).bind(email).execute(&pool).await.unwrap();
        }
        let revision = Catalog::parse(PRICE_LIST.as_bytes())
            .unwrap()
            .revision()
            .to_owned();
        let catalog = CatalogStore::new(pool.clone(), CatalogFlags { enabled: true });
        let actor = Uuid::parse_str(MODERATOR).unwrap();
        catalog
            .import(PRICE_LIST.as_bytes(), actor, &Funding)
            .await
            .unwrap();
        catalog.activate(&revision, actor, &Funding).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
            revision,
        })
    }
    async fn app(&self, values: &Values) -> Router {
        Routes::configured(self.pool.clone(), values, super::tests::environment(values))
            .await
            .mount(Router::new())
    }
    fn wallet(&self) -> Ledger<Funding, Funding, Funding> {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            Funding,
            Funding,
            Funding,
        )
    }
    async fn fund(&self, amount: i64) {
        self.fund_account(ACCOUNT, amount).await;
    }
    async fn fund_account(&self, account: &str, amount: i64) {
        self.wallet()
            .adjust(
                Uuid::now_v7(),
                Uuid::parse_str(account).unwrap(),
                amount,
                Uuid::parse_str(MODERATOR).unwrap(),
                "test funding",
            )
            .await
            .unwrap();
    }
    async fn available(&self) -> i64 {
        self.available_of(ACCOUNT).await
    }
    async fn available_of(&self, account: &str) -> i64 {
        self.wallet()
            .balance(Uuid::parse_str(account).unwrap())
            .await
            .unwrap()
            .available
    }
    async fn count(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await.unwrap()
    }
    /// Debits, orders, intents, stored answers and purchase audit rows.
    async fn written(&self) -> [i64; 5] {
        [
            self.count("SELECT COUNT(*) FROM wallet_operations WHERE kind='store_purchase'")
                .await,
            self.count("SELECT COUNT(*) FROM store_orders").await,
            self.count("SELECT COUNT(*) FROM store_operations").await,
            self.count("SELECT COUNT(*) FROM store_purchase_requests")
                .await,
            self.count("SELECT COUNT(*) FROM audit_log WHERE action='store.purchase.debited'")
                .await,
        ]
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

async fn call(app: &Router, san: &str, path: &str, body: &[u8]) -> (StatusCode, Vec<u8>) {
    let client = VerifiedClient::from_verified_tls13_http11(Identity::san(san).unwrap());
    let scoped = crate::internal_tls::scoped(
        app.clone(),
        Some(client),
        None,
        "127.0.0.1:9".parse().unwrap(),
    );
    let request = axum::http::Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_vec()))
        .unwrap();
    let response = scoped.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn db_mounted_purchase_accepts_once_replays_bytes_and_refuses_conflicts() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(1250).await;
    let app = db.app(&values()).await;
    let balance = canonical::encode(&fixture("balance-request.valid.json")).unwrap();
    let (status, bytes) = call(&app, STORE, BALANCE, &balance).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        bytes,
        golden("balance-response.valid.json", &bytes, &["read_at"])
    );
    let raw = request(&db.revision, Some("0192b6a0-4c00-7000-8000-000000000401"));
    let (status, first) = call(&app, STORE, PURCHASE, &raw).await;
    assert_eq!(status, StatusCode::OK);
    // Generated per run: the order id and the decision time; the digest is
    // checked against the raw request below.
    assert_eq!(first, accepted_item_answer(&first));
    assert_eq!(
        decode(&first).get("request_digest"),
        Some(&text(format!("{:x}", Sha256::digest(&raw))))
    );
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);
    assert_eq!(db.available().await, 1225);

    // Lost response: the identical retry returns the stored bytes, no second debit.
    assert_eq!(
        call(&app, STORE, PURCHASE, &raw).await,
        (StatusCode::OK, first)
    );
    // The same id with another binding writes nothing.
    let mut changed = decode(&raw);
    set(&mut changed, "quantity", Value::Unsigned(2));
    assert_eq!(
        call(&app, STORE, PURCHASE, &canonical::encode(&changed).unwrap()).await,
        (StatusCode::CONFLICT, Vec::new())
    );
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);
    assert_eq!(db.available().await, 1225);

    // Guard order on the mounted route: identity, then transport and body.
    assert_eq!(
        call(&app, STRANGER, PURCHASE, &request(&db.revision, None)).await,
        (StatusCode::UNAUTHORIZED, Vec::new())
    );
    assert_eq!(
        call(&app, PREMIUM, PURCHASE, &request(&db.revision, None)).await,
        (StatusCode::UNAUTHORIZED, Vec::new())
    );
    for body in [
        &b"not json"[..],
        b"{}",
        &[raw.clone(), b"\n".to_vec()].concat(),
    ] {
        assert_eq!(
            call(&app, STORE, PURCHASE, body).await,
            (StatusCode::BAD_REQUEST, Vec::new())
        );
    }
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);

    db.finish().await;
}

#[tokio::test]
async fn db_concurrent_overspend_accepts_exactly_one_and_never_goes_negative() {
    let Some(db) = Database::new().await else {
        return;
    };
    let app = db.app(&values()).await;
    for round in 1..=8 {
        db.fund(25).await;
        let tasks: Vec<_> = (0..2)
            .map(|_| {
                let app = app.clone();
                let raw = request(&db.revision, None);
                tokio::spawn(async move { call(&app, STORE, PURCHASE, &raw).await })
            })
            .collect();
        let mut results = Vec::new();
        for task in tasks {
            let (status, bytes) = task.await.unwrap();
            assert_eq!(status, StatusCode::OK);
            results.push(result(&bytes));
        }
        results.sort();
        assert_eq!(
            results,
            ["ACCEPTED", "INSUFFICIENT_BALANCE"],
            "round {round}"
        );
        assert_eq!(db.available().await, 0);
        assert_eq!(db.written().await, [round, round, round, 2 * round, round]);
    }
    db.finish().await;
}

#[tokio::test]
async fn db_refusals_and_failures_roll_back() {
    let Some(db) = Database::new().await else {
        return;
    };
    let app = db.app(&values()).await;

    // A refusal stores only its answer and stays terminal after a top-up.
    let raw = request(&db.revision, None);
    let (status, refused) = call(&app, STORE, PURCHASE, &raw).await;
    assert_eq!(status, StatusCode::OK);
    // Only the decision time is generated; the id and digest bind the request.
    let mut expected = fixture("purchase-answer.insufficient-balance.json");
    set(
        &mut expected,
        "decided_at",
        decode(&refused).get("decided_at").unwrap().clone(),
    );
    set(
        &mut expected,
        "purchase_request_id",
        decode(&raw).get("purchase_request_id").unwrap().clone(),
    );
    set(
        &mut expected,
        "request_digest",
        text(format!("{:x}", Sha256::digest(&raw))),
    );
    assert_eq!(refused, canonical::encode(&expected).unwrap());
    assert_eq!(db.written().await, [0, 0, 0, 1, 0]);
    db.fund(1000).await;
    assert_eq!(
        call(&app, STORE, PURCHASE, &raw).await,
        (StatusCode::OK, refused)
    );
    assert_eq!(db.written().await, [0, 0, 0, 1, 0]);

    // A failure after the debit rolls the debit, its audit row and the answer back.
    sqlx::raw_sql("CREATE FUNCTION store_route_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected'; END$$; CREATE TRIGGER store_route_fail BEFORE INSERT ON store_orders FOR EACH ROW EXECUTE FUNCTION store_route_fail()")
        .execute(&db.pool).await.unwrap();
    let raw = request(&db.revision, None);
    assert_eq!(
        call(&app, STORE, PURCHASE, &raw).await,
        (StatusCode::SERVICE_UNAVAILABLE, Vec::new())
    );
    assert_eq!(db.written().await, [0, 0, 0, 1, 0]);
    assert_eq!(db.available().await, 1000);
    sqlx::raw_sql("DROP TRIGGER store_route_fail ON store_orders")
        .execute(&db.pool)
        .await
        .unwrap();
    let (status, accepted) = call(&app, STORE, PURCHASE, &raw).await;
    assert_eq!(
        (status, result(&accepted).as_str()),
        (StatusCode::OK, "ACCEPTED")
    );
    assert_eq!(db.written().await, [1, 1, 1, 2, 1]);
    assert_eq!(db.available().await, 975);

    // The production account adapter: an active restriction bars buying.
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','test',now()-interval '1 minute',now()+interval '1 day',$2)")
        .bind(Uuid::parse_str(ACCOUNT).unwrap()).bind(Uuid::parse_str(MODERATOR).unwrap()).execute(&db.pool).await.unwrap();
    let (status, bytes) = call(&app, STORE, PURCHASE, &request(&db.revision, None)).await;
    assert_eq!(
        (status, result(&bytes).as_str()),
        (StatusCode::OK, "ACCOUNT_RESTRICTED")
    );
    assert_eq!(db.written().await, [1, 1, 1, 3, 1]);
    assert_eq!(db.available().await, 975);
    db.finish().await;
}

#[tokio::test]
async fn db_surface_off_or_production_answers_503_and_writes_nothing() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(1000).await;
    for (key, value) in [
        ("PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED", None),
        ("OTERYN_ENV", Some("production")),
        ("APP_ENV", Some("production")),
        ("APP_ENV", None),
        (STORE_IDENTITY, Some(PREMIUM)),
    ] {
        let mut values = values();
        match value {
            Some(value) => values.insert(key.into(), value.into()),
            None => values.remove(key),
        };
        let app = db.app(&values).await;
        assert_eq!(
            call(&app, STORE, PURCHASE, &request(&db.revision, None)).await,
            (StatusCode::SERVICE_UNAVAILABLE, Vec::new()),
            "{key}={value:?}"
        );
    }
    assert_eq!(db.written().await, [0, 0, 0, 0, 0]);
    assert_eq!(db.count("SELECT COUNT(*) FROM store_wire_limits").await, 0);
    assert_eq!(db.available().await, 1000);
    db.finish().await;
}

#[tokio::test]
async fn db_preproduction_app_env_accepts() {
    let Some(db) = Database::new().await else {
        return;
    };
    db.fund(1000).await;
    let mut values = values();
    values.insert("APP_ENV".into(), "preproduction".into());
    let app = db.app(&values).await;
    let (status, bytes) = call(&app, STORE, PURCHASE, &request(&db.revision, None)).await;
    assert_eq!(
        (status, result(&bytes).as_str()),
        (StatusCode::OK, "ACCEPTED")
    );
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);
    assert_eq!(db.available().await, 975);
    db.finish().await;
}

/// One buyer state for the production account gate, set up on a fresh buyer.
#[derive(Debug, Clone, Copy)]
enum Buyer {
    Suspended,
    Restricted,
    TypedBan,
    LiftedTypedBan,
    ExpiredTypedBan,
    ExpiredSuspension,
    PendingSuspension,
    UnknownCategory,
    UnknownStatus,
    NotActive,
    EmailNotVerified,
    EmailChangeCooldown,
    Unknown,
}

impl Database {
    /// A fresh funded buyer in the given state; `Unknown` has no account row.
    async fn buyer(&self, state: Buyer) -> String {
        let account = Uuid::now_v7();
        if matches!(state, Buyer::Unknown) {
            return account.to_string();
        }
        sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'x','active','en')")
            .bind(account).bind(format!("{}@example.test", account.simple())).execute(&self.pool).await.unwrap();
        self.fund_account(&account.to_string(), 1000).await;
        let moderator = Uuid::parse_str(MODERATOR).unwrap();
        let sanction = |category: &str, status: &str, effective: &str, expires: &str| {
            format!(
                "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'{category}','{status}','test',now()+interval '{effective}',now()+interval '{expires}',$2)"
            )
        };
        // Typed ban revisions, oldest first: (active, decided, expires).
        let ban: &[(bool, &str, Option<&str>)] = match state {
            Buyer::TypedBan => &[(true, "0 seconds", None)],
            // The latest revision lifts the ban.
            Buyer::LiftedTypedBan => &[(true, "-2 days", None), (false, "-1 day", None)],
            Buyer::ExpiredTypedBan => &[(true, "-2 days", Some("-1 day"))],
            _ => &[],
        };
        if !ban.is_empty() {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES($1,$2,'ban')")
                .bind(id).bind(account).execute(&self.pool).await.unwrap();
            for (revision, (active, decided, expires)) in (1_i64..).zip(ban) {
                sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) VALUES($1,$2,$3,'{}',$4,now()+$5::interval,now()+$6::interval)")
                    .bind(account).bind(revision).bind(id).bind(active).bind(decided).bind(expires)
                    .execute(&self.pool).await.unwrap();
            }
            return account.to_string();
        }
        let sql = match state {
            Buyer::Suspended => sanction("suspension", "active", "-1 minute", "1 day"),
            Buyer::Restricted => sanction("restriction", "active", "-1 minute", "1 day"),
            Buyer::ExpiredSuspension => sanction("suspension", "active", "-2 days", "-1 day"),
            Buyer::PendingSuspension => sanction("suspension", "active", "1 day", "2 days"),
            Buyer::UnknownCategory => {
                // A category this build does not know must bar buying.
                sqlx::raw_sql("ALTER TABLE sanctions DROP CONSTRAINT IF EXISTS sanctions_category_check")
                    .execute(&self.pool)
                    .await
                    .unwrap();
                sanction("future-category", "active", "-1 minute", "1 day")
            }
            Buyer::UnknownStatus => {
                // A status this build does not know must bar buying, even on a
                // warning, which never bars by itself.
                sqlx::raw_sql("ALTER TABLE sanctions DROP CONSTRAINT IF EXISTS sanctions_status_check")
                    .execute(&self.pool)
                    .await
                    .unwrap();
                sanction("warning", "future-status", "-1 minute", "1 day")
            }
            Buyer::TypedBan | Buyer::LiftedTypedBan | Buyer::ExpiredTypedBan => {
                panic!("handled above")
            }
            Buyer::NotActive => {
                "UPDATE accounts SET status='suspended' WHERE id=$1 AND $2::uuid IS NOT NULL".to_owned()
            }
            Buyer::EmailNotVerified => {
                "UPDATE accounts SET email_verified_at=NULL WHERE id=$1 AND $2::uuid IS NOT NULL"
                    .to_owned()
            }
            Buyer::EmailChangeCooldown => "UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1 AND $2::uuid IS NOT NULL".to_owned(),
            Buyer::Unknown => panic!("handled above"),
        };
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(account)
            .bind(moderator)
            .execute(&self.pool)
            .await
            .unwrap();
        account.to_string()
    }
}

#[tokio::test]
async fn db_account_gate_refuses_every_barred_buyer_and_writes_nothing() {
    let Some(db) = Database::new().await else {
        return;
    };
    let app = db.app(&values()).await;
    for state in [
        Buyer::Suspended,
        Buyer::Restricted,
        Buyer::TypedBan,
        Buyer::UnknownCategory,
        Buyer::UnknownStatus,
        Buyer::NotActive,
        Buyer::EmailNotVerified,
        Buyer::EmailChangeCooldown,
        Buyer::Unknown,
    ] {
        let buyer = db.buyer(state).await;
        let before = db.written().await;
        let (status, bytes) = call(
            &app,
            STORE,
            PURCHASE,
            &request_for(&db.revision, None, &buyer),
        )
        .await;
        assert_eq!(
            (status, result(&bytes).as_str()),
            (StatusCode::OK, "ACCOUNT_RESTRICTED"),
            "{state:?}"
        );
        // Only the refusal's own stored answer: no debit, order, intent or audit row.
        let [debits, orders, intents, answers, audits] = before;
        assert_eq!(
            db.written().await,
            [debits, orders, intents, answers + 1, audits],
            "{state:?}"
        );
        if !matches!(state, Buyer::Unknown) {
            assert_eq!(db.available_of(&buyer).await, 1000, "{state:?}");
        }
    }
    // A sanction that has expired, is not yet effective or was lifted does not
    // bar buying.
    for state in [
        Buyer::ExpiredSuspension,
        Buyer::PendingSuspension,
        Buyer::LiftedTypedBan,
        Buyer::ExpiredTypedBan,
    ] {
        let buyer = db.buyer(state).await;
        let (status, bytes) = call(
            &app,
            STORE,
            PURCHASE,
            &request_for(&db.revision, None, &buyer),
        )
        .await;
        assert_eq!(
            (status, result(&bytes).as_str()),
            (StatusCode::OK, "ACCEPTED"),
            "{state:?}"
        );
        assert_eq!(db.available_of(&buyer).await, 975, "{state:?}");
    }
    db.finish().await;
}

#[tokio::test]
async fn db_preprod_core_environment_accepts_purchase() {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    db.fund(1000).await;
    let mut values = values();
    values.insert("OTERYN_ENV".into(), "preprod".into());
    values.insert("APP_ENV".into(), "preproduction".into());
    let config = oteryn_core::Config::from_lookup(|key| {
        if key == "OTERYN_DATABASE_URL" {
            std::env::var("PORTAL_TEST_DATABASE_URL").ok()
        } else {
            values.get(key).cloned()
        }
    })
    .unwrap();
    let configuration = Configuration::new(values).unwrap();
    let app = mount(
        Router::new(),
        db.pool.clone(),
        config.environment,
        configuration,
    )
    .await;
    let (status, bytes) = call(&app, STORE, PURCHASE, &request(&db.revision, None)).await;
    assert_eq!(
        (status, result(&bytes).as_str()),
        (StatusCode::OK, "ACCEPTED")
    );
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);
    assert_eq!(db.available().await, 975);
    db.finish().await;
    eprintln!(
        "PG db_preprod_core_environment_accepts_purchase duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn db_binary_refuses_restore_with_committed_store_generation_seven() {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    db.fund(1000).await;
    let mut values = values();
    let app = db.app(&values).await;
    let (status, bytes) = call(&app, STORE, PURCHASE, &request(&db.revision, None)).await;
    assert_eq!(
        (status, result(&bytes).as_str()),
        (StatusCode::OK, "ACCEPTED")
    );
    let intent: Vec<u8> = sqlx::query_scalar("SELECT intent_bytes FROM store_operations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let intent_value = decode(&intent);
    let mut receipt = fixture("receipt.deliver-item.committed.json");
    set(&mut receipt, "product_id", text("oteryn.test_supply"));
    set(&mut receipt, "product_version", text("1"));
    set(&mut receipt, "delivered_quantity", Value::Unsigned(5));
    for key in ["operation_id", "entitlement_id", "account_id"] {
        set(&mut receipt, key, intent_value.get(key).unwrap().clone());
    }
    set(
        &mut receipt,
        "binding_digest",
        text(format!("{:x}", Sha256::digest(&intent))),
    );
    let delivery = delivery::Delivery::new(
        db.pool.clone(),
        purchase::Settings::from_lookup(|key| values.get(key).cloned()),
    );
    delivery
        .receipt(&canonical::encode(&receipt).unwrap())
        .await
        .unwrap();
    let generation: String = sqlx::query_scalar(
        "SELECT recovery_generation::text FROM store_receipts WHERE outcome='COMMITTED'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(generation, "7");
    values.insert("PORTAL_GAME_RESTORE_NOTICE_ENABLED".into(), "true".into());
    values.insert(RESTORE_IDENTITY.into(), "restore.oteryn.internal".into());
    assert!(Configuration::new(values.clone()).is_err());
    values.insert(
        "PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED".into(),
        "false".into(),
    );
    values.insert(
        "PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED".into(),
        "false".into(),
    );
    values.insert(
        "PORTAL_GAME_RESTORE_NOTICE_SOURCE_AUTHORITY".into(),
        "oteryn-game-eu1".into(),
    );
    Configuration::new(values.clone()).unwrap();
    let app = db.app(&values).await;
    let notice = include_bytes!("../../../crates/characters/src/restore/fixtures/notice.json");
    let (status, _) = call(
        &app,
        "restore.oteryn.internal",
        "/internal/v1/game-auth/game-restore/notice",
        notice,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM character_restore_notices WHERE acknowledged_at IS NOT NULL"
        )
        .await,
        0
    );
    assert_eq!(
        db.count(
            "SELECT count(*) FROM store_receipts WHERE superseded_by_restore_event_id IS NULL"
        )
        .await,
        1
    );
    values.insert(
        "PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED".into(),
        "true".into(),
    );
    let notice = std::str::from_utf8(notice)
        .unwrap()
        .replace(
            r#""restored_generation":"3""#,
            r#""restored_generation":"7""#,
        )
        .replace(
            r#""new_recovery_generation":"4""#,
            r#""new_recovery_generation":"8""#,
        );
    let app = db.app(&values).await;
    for _ in 0..2 {
        let (status, body) = call(
            &app,
            "restore.oteryn.internal",
            restore::PATH,
            notice.as_bytes(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let observed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(observed["status"], "RECONCILING");
        assert_eq!(observed["outstanding"], 1);
    }
    let (status, _) = call(
        &app,
        STORE,
        delivery::http::RECEIPT_PATH,
        &canonical::encode(&receipt).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    set(&mut receipt, "recovery_generation", text("8"));
    let (status, _) = call(
        &app,
        STORE,
        delivery::http::RECEIPT_PATH,
        &canonical::encode(&receipt).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call(
        &app,
        "restore.oteryn.internal",
        restore::PATH,
        notice.as_bytes(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let observed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(observed["status"], "ACKNOWLEDGED");
    assert_eq!(observed["outstanding"], 0);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM character_restore_notices WHERE acknowledged_at IS NOT NULL"
        )
        .await,
        1
    );
    db.finish().await;
    eprintln!(
        "PG db_binary_refuses_restore_with_committed_store_generation_seven duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn db_g9_revision3_goldens_through_binary_router() {
    g9_goldens_case("all").await;
}
#[tokio::test]
async fn db_l8_orphan_drop_blocks_application() {
    g9_goldens_case("orphan").await;
}
#[tokio::test]
async fn db_l8_transfer_straddling_segment_is_refused() {
    g9_goldens_case("transfer").await;
}
#[tokio::test]
async fn db_l8_g9_binding_alarms_are_durable_and_deduplicated() {
    g9_goldens_case("alarms").await;
}
#[tokio::test]
async fn db_l8_g9_tombstones_expire_without_replay() {
    g9_goldens_case("retention").await;
}
#[tokio::test]
async fn db_l8_g9_mixed_retention_closes_generation() {
    g9_goldens_case("retention_mixed").await;
}
async fn g9_goldens_case(regression: &str) {
    let started = std::time::Instant::now();
    let db = Database::new().await.expect("real PostgreSQL is required");
    let contract = include_str!(
        "../../../../docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md"
    );
    let golden = |needle: &str| {
        contract
            .lines()
            .find(|line| line.starts_with('{') && line.contains(needle))
            .unwrap()
            .as_bytes()
            .to_vec()
    };
    let segment = golden("\"record_kind\":\"HUNT_SEGMENT\"");
    let drop = golden("\"record_kind\":\"RARE_DROP\"");
    let intent = golden("\"command\":\"ReplayGameJournalRecords\"");
    let account = "019a5000-0000-7000-8000-000000000002";
    let generation = "019a5000-0000-7000-8000-000000000001";
    let character = "019a5000-0000-7000-8000-000000000005";
    let world = "019a5000-0000-7000-8000-000000000006";
    let san = "sessions.oteryn.internal";
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1::uuid,'g9@example.test','x','active','en')").bind(account).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1::uuid,'test','1','1',0,decode(repeat('00',32),'hex'))").bind(account).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1::uuid,$2::uuid,$3::uuid,'G9 Fixture','AVAILABLE')").bind(account).bind(character).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at) VALUES($1::uuid,$2::uuid,$3::uuid,1,1,1,1,1,to_timestamp(1793836700))").bind(character).bind(account).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO pd_g9_streams(source_authority,account_id,stream_generation,admitted) VALUES('game.test',$1::uuid,$2::uuid,true)").bind(account).bind(generation).execute(&db.pool).await.unwrap();
    for key in ["vocation:knight", "place:cave", "item:ruby", "rarity:rare"] {
        sqlx::query("INSERT INTO pd_g9_catalogue_keys VALUES('game.test','fixture.v1',$1)")
            .bind(key)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let settings: Values = [
        ("OTERYN_ENV", "testing"),
        (sessions::IDENTITY, san),
        ("PORTAL_PLAYERDATA_G9_INGEST_ENABLED", "true"),
        ("PORTAL_PLAYERDATA_G9_REPLAY_COMMANDS_ENABLED", "true"),
        ("PORTAL_PLAYERDATA_G9_SOURCE_AUTHORITY", "game.test"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect();
    let app = db.app(&settings).await;
    let accepted = br#"{"contract_version":1,"result":"accepted"}"#.to_vec();
    if matches!(regression, "all" | "orphan") {
        // L8-001: a sealed unrelated segment cannot resolve an orphan drop.
        let orphan = String::from_utf8(drop.clone()).unwrap().replace(
            "019a5000-0000-7000-8000-000000000004",
            "019a5000-0000-7000-8000-000000000044",
        );
        let empty_segment = String::from_utf8(segment.clone())
            .unwrap()
            .replace("\"rare_drop_count\":1", "\"rare_drop_count\":0");
        assert_ne!(orphan.as_bytes(), drop);
        assert_ne!(empty_segment.as_bytes(), segment);
        assert_eq!(
            call(&app, san, sessions::RECORDS, orphan.as_bytes())
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            call(&app, san, sessions::RECORDS, empty_segment.as_bytes())
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            db.count("SELECT count(*) FROM pd_g9_records WHERE resolved")
                .await,
            0
        );
        assert_eq!(
            db.count("SELECT applied_through::bigint FROM pd_g9_streams")
                .await,
            0
        );
        sqlx::query("DELETE FROM pd_g9_records")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE pd_g9_streams SET highest_seen=0,stored_through=0,applied_through=0")
            .execute(&db.pool)
            .await
            .unwrap();
    }
    if matches!(regression, "all" | "transfer") {
        // L8-002: the authoritative cut lies inside this otherwise valid segment.
        let parsed: serde_json::Value = serde_json::from_slice(&segment).unwrap();
        let start = parsed["record"]["started_at"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap();
        let end = parsed["record"]["ended_at"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap();
        let transfer = Uuid::now_v7().to_string();
        let receipt = serde_json::json!({"outcome":"COMMITTED","result_code":"CHAR_CMD_OK","from_account_id":MODERATOR,"to_account_id":account,"character_revision":"2","from_projection_revision":"1","to_projection_revision":"2","projection_epoch":"1","recovery_generation":"1","decided_at":((start+end)/2).to_string()}).to_string();
        sqlx::query("UPDATE pd_ownership_baselines SET account_id=$1::uuid")
            .bind(MODERATOR)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE account_character_snapshots SET projection_revision='2' WHERE account_id=$1::uuid").bind(account).execute(&db.pool).await.unwrap();
        // The seller's publication covers the transfer cut, so ownership is
        // proven and the refusal below is the straddle, not missing evidence.
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1::uuid,'test','1','1',0,decode(repeat('00',32),'hex'))").bind(MODERATOR).execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json,receipt_json,state) VALUES($1::uuid,$1::uuid,$1,'TransferCharacterOwnership',$2::uuid,$3::uuid,json_build_object('character_id',$4::text)::text,'',$5,'TERMINAL')")
        .bind(&transfer).bind(MODERATOR).bind(account).bind(character).bind(receipt).execute(&db.pool).await.unwrap();
        let straddling = String::from_utf8(segment.clone()).unwrap().replace(
            "\"character_revision\":\"1\"",
            "\"character_revision\":\"2\"",
        );
        assert_eq!(
            call(&app, san, sessions::RECORDS, straddling.as_bytes())
                .await
                .0,
            StatusCode::CONFLICT
        );
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 0);
        assert_eq!(db.count("SELECT highest_seen::bigint+stored_through::bigint+applied_through::bigint FROM pd_g9_streams").await, 0);
        sqlx::query("DELETE FROM character_commands WHERE id=$1::uuid")
            .bind(transfer)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE pd_ownership_baselines SET account_id=$1::uuid")
            .bind(account)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE account_character_snapshots SET projection_revision='1' WHERE account_id=$1::uuid").bind(account).execute(&db.pool).await.unwrap();
    }
    // Out-of-order delivery stores evidence but cannot claim a contiguous prefix.
    assert_eq!(
        call(&app, san, sessions::RECORDS, &drop).await,
        (StatusCode::OK, accepted.clone())
    );
    let stored: String = sqlx::query_scalar("SELECT stored_through::text FROM pd_g9_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(stored, "0");
    for bytes in [&segment, &drop, &segment] {
        assert_eq!(
            call(&app, san, sessions::RECORDS, bytes).await,
            (StatusCode::OK, accepted.clone())
        );
    }
    let stored: String = sqlx::query_scalar("SELECT stored_through::text FROM pd_g9_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(stored, "2");
    let applied: String = sqlx::query_scalar("SELECT applied_through::text FROM pd_g9_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(applied, "2");
    sqlx::query("UPDATE pd_g9_streams SET admitted=false")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        call(&app, san, sessions::RECORDS, &segment).await.0,
        StatusCode::OK
    );
    let late = String::from_utf8(segment.clone())
        .unwrap()
        .replace("\"source_revision\":\"1\"", "\"source_revision\":\"3\"");
    assert_eq!(
        call(&app, san, sessions::RECORDS, late.as_bytes()).await.0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE pd_g9_streams SET admitted=true")
        .execute(&db.pool)
        .await
        .unwrap();
    sessions::enqueue(&db.pool, "game.test", &intent)
        .await
        .unwrap();
    if matches!(regression, "all" | "alarms") {
        let changed_intent = String::from_utf8(intent.clone())
            .unwrap()
            .replace("\"source_revision\":\"1\"", "\"source_revision\":\"2\"");
        assert_ne!(changed_intent.as_bytes(), intent);
        for _ in 0..2 {
            assert_eq!(
                sessions::enqueue(&db.pool, "game.test", changed_intent.as_bytes()).await,
                Err(StatusCode::CONFLICT)
            );
        }
        assert_eq!(
            db.count(
                "SELECT count(*) FROM audit_log WHERE action='ops.playerdata_binding_conflict'"
            )
            .await,
            1
        );
    }
    let pending=br#"{"contract_version":1,"operation":"ListPendingGameJournalCommandsV1","source_authority":"game.test","after_source_revision":"0","after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}"#;
    assert_eq!(call(&app,san,sessions::COMMANDS[0],pending).await,(StatusCode::OK,br#"{"contract_version":1,"entries":[{"operation_id":"019a5000-0000-7000-8000-000000000009","source_revision":"1"}]}"#.to_vec()));
    let read=br#"{"contract_version":1,"operation":"ReadGameJournalCommandIntentV1","source_authority":"game.test","issuer_authority":"OTERYN_PLATFORM_GAME_JOURNAL","operation_id":"019a5000-0000-7000-8000-000000000009"}"#;
    assert_eq!(
        call(&app, san, sessions::COMMANDS[1], read).await,
        (StatusCode::OK, intent.clone())
    );
    let digest: String = Sha256::digest(&intent)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let receipt = format!(
        r#"{{"contract_version":1,"operation":"PublishGameJournalCommandReceiptV1","source_authority":"game.test","issuer_authority":"OTERYN_PLATFORM_GAME_JOURNAL","operation_id":"019a5000-0000-7000-8000-000000000009","binding_digest":"{digest}","outcome":"COMMITTED","result_code":"JOURNAL_REPLAY_QUEUED","decided_at":"1793836950"}}"#
    );
    for _ in 0..2 {
        assert_eq!(
            call(&app, san, sessions::COMMANDS[2], receipt.as_bytes()).await,
            (StatusCode::OK, accepted.clone())
        );
    }
    let changed = receipt.replace("1793836950", "1793836951");
    assert_eq!(
        call(&app, san, sessions::COMMANDS[2], changed.as_bytes())
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(&app, san, sessions::COMMANDS[0], pending).await,
        (
            StatusCode::OK,
            br#"{"contract_version":1,"entries":[]}"#.to_vec()
        )
    );
    let conflict = String::from_utf8(segment.clone()).unwrap().replace(
        "\"experience_gained\":\"100\"",
        "\"experience_gained\":\"101\"",
    );
    assert_eq!(
        call(&app, san, sessions::RECORDS, conflict.as_bytes())
            .await
            .0,
        StatusCode::CONFLICT
    );
    if matches!(regression, "all" | "alarms") {
        for _ in 0..2 {
            assert_eq!(
                call(&app, san, sessions::RECORDS, conflict.as_bytes())
                    .await
                    .0,
                StatusCode::CONFLICT
            );
            assert_eq!(
                call(&app, san, sessions::COMMANDS[2], changed.as_bytes())
                    .await
                    .0,
                StatusCode::CONFLICT
            );
        }
        let wrong_binding = receipt.replace(&digest, &"0".repeat(64));
        for _ in 0..2 {
            assert_eq!(
                call(&app, san, sessions::COMMANDS[2], wrong_binding.as_bytes())
                    .await
                    .0,
                StatusCode::CONFLICT
            );
        }
        assert_eq!(
            db.count(
                "SELECT count(*) FROM audit_log WHERE action='ops.playerdata_binding_conflict'"
            )
            .await,
            3
        );
        // Same non-null fields consumed by the durable operator alert reader.
        let alerts: Vec<(String,String,String)> = sqlx::query_as("SELECT after->>'authority',after->>'family',after->>'record_id' FROM audit_log WHERE action='ops.playerdata_binding_conflict'").fetch_all(&db.pool).await.unwrap();
        assert_eq!(alerts.len(), 3);
        assert!(
            alerts
                .iter()
                .all(|(authority, family, reference)| authority == "game.test"
                    && family == "G9"
                    && reference.len() == 64)
        );
        let original: Vec<u8> =
            sqlx::query_scalar("SELECT canonical_bytes FROM pd_g9_records WHERE source_revision=1")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(original, segment);
        let original: Vec<u8> = sqlx::query_scalar("SELECT receipt FROM pd_g9_commands")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(original, receipt.as_bytes());
    }
    let fenced = String::from_utf8(segment.clone())
        .unwrap()
        .replace(generation, "019a5000-0000-7000-8000-000000000099");
    assert_eq!(
        call(&app, san, sessions::RECORDS, fenced.as_bytes())
            .await
            .0,
        StatusCode::CONFLICT
    );
    for (path, limit) in [
        (sessions::RECORDS, 1048576),
        (sessions::COMMANDS[0], 512),
        (sessions::COMMANDS[1], 512),
        (sessions::COMMANDS[2], 1024),
    ] {
        assert_eq!(
            call(&app, san, path, &vec![b'x'; limit + 1]).await.0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            call(&app, STRANGER, path, &vec![b'x'; limit + 1]).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 2);
    if matches!(regression, "all" | "retention") {
        // L8-007: retain until 120 days after the later purge, then drop digests.
        sqlx::query("UPDATE pd_g9_records SET canonical_bytes=NULL,record=NULL,purged_at=floor(extract(epoch FROM clock_timestamp()))::bigint-120*86400+3600").execute(&db.pool).await.unwrap();
        oteryn_playerdata::retention::Retention(db.pool.clone())
            .expire_batch()
            .await
            .unwrap();
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 2);
        sqlx::query("UPDATE pd_g9_records SET purged_at=purged_at-7200")
            .execute(&db.pool)
            .await
            .unwrap();
        oteryn_playerdata::retention::Retention(db.pool.clone())
            .expire_batch()
            .await
            .unwrap();
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 0);
        sqlx::query("UPDATE pd_g9_streams SET conflicted=false")
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            call(&app, san, sessions::RECORDS, &segment).await.0,
            StatusCode::CONFLICT
        );
    }
    if regression == "retention_mixed" {
        sqlx::query("UPDATE pd_g9_records SET canonical_bytes=NULL,record=NULL,purged_at=floor(extract(epoch FROM clock_timestamp()))::bigint-121*86400 WHERE source_revision=2").execute(&db.pool).await.unwrap();
        oteryn_playerdata::retention::Retention(db.pool.clone())
            .expire_batch()
            .await
            .unwrap();
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 1);
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_streams WHERE blocked AND NOT admitted AND retention_floor=2 AND stored_through=2 AND applied_through=2").await, 1);
        sqlx::query("UPDATE pd_g9_streams SET conflicted=false")
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            call(&app, san, sessions::RECORDS, &drop).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(db.count("SELECT count(*) FROM pd_g9_records").await, 1);
    }
    db.finish().await;
    eprintln!(
        "PG g9_goldens_case/{regression} {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn l8_composed_refusals_preserve_wire_and_cache() {
    let started = std::time::Instant::now();
    let pool = oteryn_db::lazy_pool("postgres://unused@127.0.0.1/unused").unwrap();
    let mut values = values();
    values.insert(PREMIUM_IDENTITY.into(), STORE.into());
    let coins = oteryn_wallet::wire::router(
        Ledger::new(
            pool.clone(),
            WalletFlags { enabled: true },
            Funding,
            Funding,
            Funding,
        ),
        oteryn_wallet::wire::Flags {
            transfers: true,
            market: true,
        },
        IdentityMap::new(vec![Binding {
            identity: Identity::san(STORE).unwrap(),
            group: "game-coins".into(),
        }])
        .unwrap(),
    );
    let app = Routes::configured(pool, &values, oteryn_core::Environment::Testing)
        .await
        .mount(coins);
    for (path, cache, expected) in [
        (
            "/internal/v1/coins/transfers",
            "no-store",
            br#"{"result":"unavailable"}"#.as_slice(),
        ),
        (
            "/internal/v1/coins/market/offers",
            "no-store",
            br#"{"result":"unavailable"}"#.as_slice(),
        ),
        (
            "/internal/v1/game-auth/character-commands/pending",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            oteryn_playerdata::sessions::RECORDS,
            "no-store",
            b"".as_slice(),
        ),
        (
            "/internal/v1/game-auth/game-restore/notice",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v2/game-auth/character-services/state",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v1/game-auth/character-bootstrap-intents/read",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v1/products-entitlements/premium-snapshots/read",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v1/products-entitlements/loyalty-snapshots/read",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v1/products-entitlements/profile-b/authority/read",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v1/products-entitlements/store/operations/pending",
            "no-store",
            b"".as_slice(),
        ),
        (
            "/internal/v1/products-entitlements/store/purchases",
            "no-store",
            b"".as_slice(),
        ),
        (
            "/internal/v1/playerdata/progress/record",
            "no-store",
            b"".as_slice(),
        ),
        (
            "/internal/v2/game-public/character-online-session",
            "no-store, private",
            b"".as_slice(),
        ),
        (
            "/internal/v2/game-public/watermark",
            "private, no-store, max-age=0",
            b"".as_slice(),
        ),
    ] {
        for san in [STORE, STRANGER] {
            let client = VerifiedClient::from_verified_tls13_http11(Identity::san(san).unwrap());
            let scoped = crate::internal_tls::scoped(
                app.clone(),
                Some(client),
                None,
                "127.0.0.1:9".parse().unwrap(),
            );
            let response = scoped
                .oneshot(
                    axum::http::Request::post(path)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from("not json"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
            assert_eq!(response.headers()[header::CACHE_CONTROL], cache, "{path}");
            if path.contains("/coins/") {
                assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            }
            assert_eq!(
                to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
                expected
            );
        }
    }
    eprintln!(
        "l8_composed_refusals_preserve_wire_and_cache {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn w3_c06_001_production_watermark_dispatch_shared_publisher_budget() {
    let started = std::time::Instant::now();
    let db = Database::new().await.expect("PostgreSQL required");
    let mut config = values();
    for (key, value) in [
        ("PORTAL_PUBLIC_PROJECTIONS_ENABLED", "true"),
        ("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY", "oteryn:game:primary"),
        ("PORTAL_STATS_WORLD_TOTALS_ENABLED", "true"),
        ("PORTAL_G4_SKILLS_INGEST_ENABLED", "true"),
        (PUBLIC_IDENTITY, "public.oteryn.internal"),
    ] {
        config.insert(key.into(), value.into());
    }
    config.insert("APP_ENV".into(), "testing".into());
    let routes =
        Routes::configured(db.pool.clone(), &config, super::tests::environment(&config)).await;
    let identities = Arc::new(
        IdentityMap::new(vec![Binding {
            identity: Identity::san("public.oteryn.internal").unwrap(),
            group: "game-public".into(),
        }])
        .unwrap(),
    );
    let guild = oteryn_community::guilds::ingest_router(
        db.pool.clone(),
        identities.clone(),
        oteryn_community::guilds::IngestConfig {
            enabled: true,
            source_authority: "oteryn:game:primary".into(),
        },
    );
    let house = oteryn_community::houses::router(
        db.pool.clone(),
        identities.clone(),
        Arc::new(oteryn_community::houses::DenyOwners),
        oteryn_community::houses::Config {
            ingest: oteryn_community::ingest::Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
            serving_enabled: false,
        },
    );
    let deaths = oteryn_community::deaths::ingest_router(
        db.pool.clone(),
        identities.clone(),
        oteryn_community::deaths::IngestConfig {
            enabled: true,
            source_authority: "oteryn:game:primary".into(),
            requests_per_minute: 6000,
        },
    );
    let daily = oteryn_community::today::router(
        db.pool.clone(),
        identities,
        oteryn_community::today::Config::default(),
    );
    let app = routes.mount(oteryn_community::v2_watermark::router(
        deaths, guild, house, daily,
    ));
    // Fill all but the last slot, then consume the last slot through the guild dispatcher.
    let mut publisher = vec![1, u8::try_from("public.oteryn.internal".len()).unwrap()];
    publisher.extend_from_slice("public.oteryn.internal".as_bytes());
    sqlx::query("INSERT INTO community_ingest_budgets(publisher,route,admitted_at) SELECT $1,'v2/watermark',array_fill(extract(epoch FROM clock_timestamp())::double precision,ARRAY[5999])").bind(publisher).execute(&db.pool).await.unwrap();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    for (family, expected) in [
        ("guild", StatusCode::OK),
        ("house", StatusCode::TOO_MANY_REQUESTS),
        ("character_deaths", StatusCode::TOO_MANY_REQUESTS),
        ("character_skills", StatusCode::TOO_MANY_REQUESTS),
        ("world_daily_kills", StatusCode::TOO_MANY_REQUESTS),
    ] {
        let body = format!(
            r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"{family}","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
        );
        let (status, _) = call(
            &app,
            "public.oteryn.internal",
            oteryn_community::v2_watermark::PATH,
            body.as_bytes(),
        )
        .await;
        assert_eq!(status, expected, "{family}");
    }
    db.finish().await;
    eprintln!(
        "w3_c06_001_production_watermark_dispatch_shared_publisher_budget {:?}",
        started.elapsed()
    );
}
