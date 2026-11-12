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
const PRICE_LIST: &str = r#"{"contract_version":1,"offers":[{"offer_key":"premium-30d","currency":"COINS","unit_price":"250","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.premium_time","product_version":"1","unit_kind":"PREMIUM_TIME","grant_key":"premium_time","grant_revision":"1","quantity":30}]}]}"#;

fn values() -> Values {
    [
        ("OTERYN_ENV", "testing"),
        ("APP_ENV", "testing"),
        ("PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED", "true"),
        ("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED", "true"),
        ("PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY", SOURCE),
        (
            "PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS",
            r#"["PREMIUM_TIME"]"#,
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
/// `golden` for an answer with `units`: each fixture unit keeps its
/// `unit_kind`, the units their count and order; only the ids the server
/// generates per unit (`entitlement_id`, `delivery_operation_id`) are taken
/// from the unit of `actual` at the same position.
fn golden_units(name: &str, actual: &[u8], generated: &[&str]) -> Vec<u8> {
    let decoded = decode(actual);
    let mut expected = decode(&golden(name, actual, generated));
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
        Routes::configured(self.pool.clone(), values)
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
    let scoped = crate::internal_tls::scoped(app.clone(), client, "127.0.0.1:9".parse().unwrap());
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
    assert_eq!(
        first,
        golden_units(
            "purchase-answer.accepted.json",
            &first,
            &["request_digest", "order_id", "decided_at"],
        )
    );
    assert_eq!(
        decode(&first).get("request_digest"),
        Some(&text(format!("{:x}", Sha256::digest(&raw))))
    );
    assert_eq!(db.written().await, [1, 1, 1, 1, 1]);
    assert_eq!(db.available().await, 1000);

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
    assert_eq!(db.available().await, 1000);

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
        db.fund(250).await;
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
    assert_eq!(db.available().await, 750);

    // The production account adapter: an active restriction bars buying.
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','test',now()-interval '1 minute',now()+interval '1 day',$2)")
        .bind(Uuid::parse_str(ACCOUNT).unwrap()).bind(Uuid::parse_str(MODERATOR).unwrap()).execute(&db.pool).await.unwrap();
    let (status, bytes) = call(&app, STORE, PURCHASE, &request(&db.revision, None)).await;
    assert_eq!(
        (status, result(&bytes).as_str()),
        (StatusCode::OK, "ACCOUNT_RESTRICTED")
    );
    assert_eq!(db.written().await, [1, 1, 1, 3, 1]);
    assert_eq!(db.available().await, 750);
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
    assert_eq!(db.available().await, 750);
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
        assert_eq!(db.available_of(&buyer).await, 750, "{state:?}");
    }
    db.finish().await;
}
