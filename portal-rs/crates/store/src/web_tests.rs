#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request as HttpRequest,
};
use oteryn_wallet::{AccountStateReader, AuditHook, TransferBlockCheck};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED shop PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("shop_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
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
#[derive(Clone)]
struct Policy;
impl AccountStateReader for Policy {
    async fn state(&self, _: &mut sqlx::PgConnection, _: Uuid) -> oteryn_wallet::AccountState {
        oteryn_wallet::AccountState::Active
    }
}
impl TransferBlockCheck for Policy {
    async fn permission(
        &self,
        _: &mut sqlx::PgConnection,
        _: Uuid,
    ) -> oteryn_wallet::TransferPermission {
        oteryn_wallet::TransferPermission::Allowed
    }
}
impl AuditHook for Policy {
    async fn adjustment(
        &self,
        _: &mut sqlx::PgConnection,
        _: &oteryn_wallet::AdjustmentAudit,
    ) -> bool {
        true
    }
}
impl Access for Policy {
    fn active(&self, _: Uuid) -> FutureResult<'_, bool> {
        Box::pin(async { Ok(true) })
    }
    fn target(&self, _: Uuid, _: Uuid, _: Uuid) -> FutureResult<'_, bool> {
        Box::pin(async { Ok(false) })
    }
}
fn domain(pool: PgPool) -> Domain<Policy, Policy, Policy> {
    Domain {
        wallet: oteryn_wallet::Ledger::new(
            pool.clone(),
            oteryn_wallet::WalletFlags { enabled: true },
            Policy,
            Policy,
            Policy,
        ),
        catalog: crate::catalog::CatalogStore::new(
            pool,
            crate::catalog::CatalogFlags { enabled: true },
        ),
        access: Arc::new(Policy),
        purchases: None,
        top_ups: None,
    }
}
fn app(pool: PgPool, backend: Arc<dyn Backend>) -> Router {
    router(
        pool,
        Flags {
            shop: true,
            transfers: true,
            purchases: true,
            top_ups: true,
        },
        backend,
        vec!["https://portal.test".into()],
    )
}
async fn session(pool: &PgPool, account: Uuid) -> Result<(String, String), sqlx::Error> {
    use sha2::{Digest, Sha256};
    let token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    let csrf = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(&csrf).bind(account).execute(pool).await?;
    Ok((
        format!("{}={token}", oteryn_web::session::COOKIE_NAME),
        csrf,
    ))
}
async fn body(response: Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 1_000_000).await?.to_vec(),
    )?)
}
fn post_request(
    path: &str,
    cookie: &str,
    body: String,
) -> Result<HttpRequest<Body>, axum::http::Error> {
    HttpRequest::post(path)
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, "https://portal.test")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
}
#[test]
fn strict_amounts_and_default_flags() {
    for input in [
        "",
        "0",
        "-1",
        "01",
        "+1",
        "1.0",
        " 1",
        "9223372036854775808",
    ] {
        assert_eq!(amount(input), Err(Error::Invalid));
    }
    assert_eq!(amount("9223372036854775807"), Ok(i64::MAX));
    let f = Flags::default();
    assert!(!f.shop && !f.transfers && !f.top_ups && !f.purchases);
    for lang in ["pl", "en"] {
        let l = Locale::parse(lang).unwrap();
        for key in [
            "shop",
            "coins",
            "history",
            "transfer",
            "unavailable",
            "invalid",
            "pending",
            "denied",
            "conflict",
            "limited",
        ] {
            assert!(!l.text(key).is_empty());
        }
    }
}
#[tokio::test]
async fn postgres_transfer_success_replay_refusal_and_bad_input() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let sender = Uuid::now_v7();
    let recipient = Uuid::now_v7();
    let d = domain(db.pool.clone());
    d.wallet
        .adjust(Uuid::now_v7(), sender, 50, Uuid::now_v7(), "fixture")
        .await?;
    let app = app(db.pool.clone(), Arc::new(d));
    let (cookie, csrf) = session(&db.pool, sender).await?;
    for (lang, label) in [
        ("pl", "Twój identyfikator do przelewów"),
        ("en", "Your transfer ID"),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get(format!("/{lang}/shop/transfer"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = body(response).await?;
        assert!(html.contains(label));
        assert!(html.contains(&format!("value=\"{sender}\" readonly")));
        assert!(html.contains("pattern=\"[0-9a-fA-F]{8}"));
    }
    let key = Uuid::now_v7();
    let form = format!("_csrf={csrf}&operation={key}&recipient={recipient}&amount=20");
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(post_request("/en/shop/transfer", &cookie, form.clone())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(sender)
            .await?
            .available,
        30
    );
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(recipient)
            .await?
            .available,
        20
    );
    for (amount, status) in [
        ("31", StatusCode::FORBIDDEN),
        ("-1", StatusCode::UNPROCESSABLE_ENTITY),
        ("01", StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let form = format!(
            "_csrf={csrf}&operation={}&recipient={recipient}&amount={amount}",
            Uuid::now_v7()
        );
        assert_eq!(
            app.clone()
                .oneshot(post_request("/en/shop/transfer", &cookie, form)?)
                .await?
                .status(),
            status
        );
    }
    let response = app
        .clone()
        .oneshot(post_request(
            "/en/shop/transfer",
            &cookie,
            format!("_csrf=bad&operation={key}&recipient={recipient}&amount=20"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .oneshot(
            HttpRequest::get("/en/shop/history")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await?;
    assert!(html.contains(&key.to_string()));
    assert!(html.contains("-20"));
    db.close().await
}
#[tokio::test]
async fn postgres_pages_authentication_flags_and_pagination() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let backend: Arc<dyn Backend> = Arc::new(domain(db.pool.clone()));
    let disabled = router(
        db.pool.clone(),
        Flags::default(),
        backend.clone(),
        Vec::new(),
    );
    assert_eq!(
        disabled
            .oneshot(HttpRequest::get("/en/shop").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let app = app(db.pool.clone(), backend);
    let (cookie, _) = session(&db.pool, account).await?;
    for lang in ["pl", "en"] {
        for path in ["shop", "shop/coins", "shop/history", "shop/transfer"] {
            let response = app
                .clone()
                .oneshot(HttpRequest::get(format!("/{lang}/{path}")).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let response = app
                .clone()
                .oneshot(
                    HttpRequest::get(format!("/{lang}/{path}"))
                        .header(header::COOKIE, &cookie)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let html = body(response).await?;
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(html.contains("width=device-width"));
        }
    }
    for query in [
        "before=bad",
        "before=00000000-0000-0000-0000-000000000000",
        "account=other",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(
                    HttpRequest::get(format!("/en/shop/history?{query}"))
                        .header(header::COOKIE, &cookie)
                        .body(Body::empty())?
                )
                .await?
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    db.close().await
}
#[tokio::test]
async fn postgres_unconfigured_mutations_and_rate_limit_deny() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let (cookie, csrf) = session(&db.pool, account).await?;
    let app = app(db.pool.clone(), Arc::new(domain(db.pool.clone())));
    let form = format!("_csrf={csrf}&operation={}&package=unknown", Uuid::now_v7());
    assert_eq!(
        app.clone()
            .oneshot(post_request("/en/shop/coins", &cookie, form)?)
            .await?
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for _ in 0..9 {
        let _ = app
            .clone()
            .oneshot(post_request(
                "/en/shop/transfer",
                &cookie,
                format!("_csrf={csrf}&amount=bad"),
            )?)
            .await?;
    }
    let response = app
        .oneshot(post_request(
            "/en/shop/transfer",
            &cookie,
            format!("_csrf={csrf}&amount=bad"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    db.close().await
}

impl crate::catalog::CatalogAccess for Policy {
    async fn account_state(
        &self,
        _: Uuid,
        _: &mut sqlx::PgConnection,
    ) -> Result<crate::catalog::AccountState, crate::catalog::Error> {
        Ok(crate::catalog::AccountState::Active)
    }
    async fn permitted(
        &self,
        _: Uuid,
        _: crate::catalog::Action,
        _: &mut sqlx::PgConnection,
    ) -> Result<bool, crate::catalog::Error> {
        Ok(true)
    }
}
impl crate::purchase::Accounts for Policy {
    async fn state(
        &self,
        _: &mut sqlx::PgConnection,
        _: Uuid,
    ) -> Result<crate::purchase::AccountState, crate::purchase::Error> {
        Ok(crate::purchase::AccountState::Active)
    }
}
struct Target {
    character: Uuid,
    world: Uuid,
}
impl Access for Target {
    fn active(&self, _: Uuid) -> FutureResult<'_, bool> {
        Box::pin(async { Ok(true) })
    }
    fn target(&self, _: Uuid, character: Uuid, world: Uuid) -> FutureResult<'_, bool> {
        Box::pin(async move { Ok(self.character == character && self.world == world) })
    }
}
const CATALOG: &[u8] = br#"{"contract_version":1,"offers":[{"offer_key":"potion-pack-10","currency":"COINS","unit_price":"250","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.store.item_grant","product_version":"2","unit_kind":"ITEM","grant_key":"item:great_health_potion","grant_revision":"12","quantity":10}]}]}"#;
async fn catalog(
    domain: &Domain<Policy, Policy, Policy>,
) -> Result<String, Box<dyn std::error::Error>> {
    let actor = Uuid::now_v7();
    let revision = domain.catalog.import(CATALOG, actor, &Policy).await?;
    domain.catalog.activate(&revision, actor, &Policy).await?;
    Ok(revision)
}
#[tokio::test]
async fn postgres_purchase_uses_quote_ownership_and_domain_idempotency() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let buyer = Uuid::now_v7();
    let character = Uuid::now_v7();
    let world = Uuid::now_v7();
    let mut d = domain(db.pool.clone());
    let revision = catalog(&d).await?;
    d.wallet
        .adjust(Uuid::now_v7(), buyer, 1000, Uuid::now_v7(), "fixture")
        .await?;
    d.access = Arc::new(Target { character, world });
    d.purchases = Some(Arc::new(crate::purchase::Purchases::new(
        db.pool.clone(),
        crate::purchase::Settings {
            purchase_enabled: true,
            delivery_enabled: true,
            environment: "testing".into(),
            source_authority: "shop-test".into(),
            peer_identity: "shop-test-peer".into(),
            enabled_unit_kinds: vec!["ITEM".into()],
            ..Default::default()
        },
        domain(db.pool.clone()).wallet,
        Policy,
        crate::premium::repository::Ledger::new(
            db.pool.clone(),
            std::sync::Arc::new(crate::loyalty::stream::http::DatabaseAccounts),
            false,
        ),
    )));
    let app = app(db.pool.clone(), Arc::new(d));
    let (cookie, csrf) = session(&db.pool, buyer).await?;
    let key = Uuid::now_v7();
    let form = format!(
        "_csrf={csrf}&operation={key}&character={character}&world={world}&revision={revision}&offer=potion-pack-10&price=250&quantity=1"
    );
    for _ in 0..2 {
        assert_eq!(
            app.clone()
                .oneshot(post_request("/en/shop", &cookie, form.clone())?)
                .await?
                .status(),
            StatusCode::SEE_OTHER
        );
    }
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(buyer)
            .await?
            .available,
        750
    );
    let response = app
        .clone()
        .oneshot(post_request(
            "/en/shop",
            &cookie,
            form.replace("quantity=1", "quantity=2"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let wrong = form
        .replace(&key.to_string(), &Uuid::now_v7().to_string())
        .replace(&character.to_string(), &Uuid::now_v7().to_string());
    assert_eq!(
        app.clone()
            .oneshot(post_request("/en/shop", &cookie, wrong)?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    let stale = form
        .replace(&key.to_string(), &Uuid::now_v7().to_string())
        .replace(&revision, &"a".repeat(64));
    assert_eq!(
        app.clone()
            .oneshot(post_request("/en/shop", &cookie, stale)?)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let invalid = form.replace("quantity=1", "quantity=0");
    assert_eq!(
        app.oneshot(post_request("/en/shop", &cookie, invalid)?)
            .await?
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(buyer)
            .await?
            .available,
        750
    );
    db.close().await
}

#[derive(Default)]
struct TestTopUps(tokio::sync::Mutex<std::collections::HashMap<Uuid, (Uuid, String, Uuid)>>);
impl TopUps for TestTopUps {
    fn packages(&self) -> Vec<Package> {
        vec![Package {
            key: "fixture".into(),
            coins: 100,
            minor_units: 250,
            currency: "PLN".into(),
        }]
    }
    fn create(&self, account: Uuid, operation: Uuid, package: String) -> FutureResult<'_, Uuid> {
        Box::pin(async move {
            let mut records = self.0.lock().await;
            if let Some((a, p, id)) = records.get(&operation) {
                return if *a == account && *p == package {
                    Ok(*id)
                } else {
                    Err(Error::Conflict)
                };
            }
            if package != "fixture" {
                return Err(Error::Invalid);
            }
            let id = Uuid::now_v7();
            records.insert(operation, (account, package, id));
            Ok(id)
        })
    }
}
#[tokio::test]
async fn postgres_topup_pending_replay_and_no_browser_credit() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let buyer = Uuid::now_v7();
    let mut d = domain(db.pool.clone());
    let topups = Arc::new(TestTopUps::default());
    d.top_ups = Some(topups.clone());
    let app = app(db.pool.clone(), Arc::new(d));
    let (cookie, csrf) = session(&db.pool, buyer).await?;
    let key = Uuid::now_v7();
    let form = format!("_csrf={csrf}&operation={key}&package=fixture");
    let first = app
        .clone()
        .oneshot(post_request("/en/shop/coins", &cookie, form.clone())?)
        .await?;
    assert_eq!(first.status(), StatusCode::OK);
    let first = body(first).await?;
    assert!(first.contains("Payment pending"));
    let second = body(
        app.clone()
            .oneshot(post_request("/en/shop/coins", &cookie, form.clone())?)
            .await?,
    )
    .await?;
    assert_eq!(first, second);
    assert_eq!(topups.0.lock().await.len(), 1);
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(buyer)
            .await?
            .available,
        0
    );
    for suffix in ["&amount=1", "&account=other", "&package=other"] {
        assert_eq!(
            app.clone()
                .oneshot(post_request(
                    "/en/shop/coins",
                    &cookie,
                    format!("{form}{suffix}")
                )?)
                .await?
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    db.close().await
}

#[tokio::test]
#[ignore = "requires Chromium and PORTAL_BROWSER_MODULES"]
async fn postgres_browser_at_360px() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let recipient = Uuid::now_v7();
    let mut d = domain(db.pool.clone());
    d.wallet
        .adjust(
            Uuid::now_v7(),
            account,
            50,
            Uuid::now_v7(),
            "browser-fixture",
        )
        .await?;
    catalog(&d).await?;
    d.top_ups = Some(Arc::new(TestTopUps::default()));
    let (cookie, _) = session(&db.pool, account).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = router(
        db.pool.clone(),
        Flags {
            shop: true,
            transfers: true,
            top_ups: true,
            purchases: true,
        },
        Arc::new(d),
        vec![format!("http://{address}")],
    )
    .route(
        "/static/css/tokens.css",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/css")],
                include_str!("../../../static/css/tokens.css"),
            )
        }),
    )
    .route(
        "/static/css/components.css",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/css")],
                include_str!("../../../static/css/components.css"),
            )
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(
            std::env::var("PORTAL_BROWSER_NODE").unwrap_or_else(|_| "node".into()),
        )
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/src/web_browser.cjs"))
        .env("PORTAL_BROWSER_URL", format!("http://{address}"))
        .env("PORTAL_BROWSER_COOKIE", cookie)
        .env("PORTAL_BROWSER_RECIPIENT", recipient.to_string())
        .status()
    })
    .await?;
    server.abort();
    db.close().await?;
    assert!(result?.success());
    Ok(())
}

#[tokio::test]
async fn postgres_configured_identity_d14_transfer_and_ownership_deny() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let backend = backend::configured(db.pool.clone(), None);
    let app = app(db.pool.clone(), backend);
    let (mut cookie, mut csrf) = session(&db.pool, account).await?;
    for state in [
        "missing",
        "pending_verification",
        "suspended",
        "closed",
        "active",
    ] {
        if state != "missing" {
            sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'fixture@example.invalid',now(),'unused-test-hash',$2,'en') ON CONFLICT(id) DO UPDATE SET status=$2")
                .bind(account).bind(state).execute(&db.pool).await?;
            // A status change revokes the account's sessions; sign in again for this state.
            (cookie, csrf) = session(&db.pool, account).await?;
        }
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get("/en/shop/history")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(
            response.status(),
            if state == "active" {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
    }
    domain(db.pool.clone())
        .wallet
        .adjust(Uuid::now_v7(), account, 100, Uuid::now_v7(), "fixture")
        .await?;
    let form = format!(
        "_csrf={csrf}&operation={}&recipient={}&amount=20",
        Uuid::now_v7(),
        Uuid::now_v7()
    );
    // Unknown recipient denies before the unknown transfer policy can grant anything.
    let response = app
        .clone()
        .oneshot(post_request("/en/shop/transfer", &cookie, form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let form = format!(
        "_csrf={csrf}&operation={}&character={}&world={}&revision={}&offer=potion-pack-10&price=250&quantity=1",
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        "a".repeat(64)
    );
    assert_eq!(
        app.clone()
            .oneshot(post_request("/en/shop", &cookie, form)?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        domain(db.pool.clone())
            .wallet
            .balance(account)
            .await?
            .available,
        100
    );
    let recipient = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'recipient@example.invalid',now(),'unused-test-hash','active','en')").bind(recipient).execute(&db.pool).await?;
    let backend = backend::configured(db.pool.clone(), None);
    assert!(
        backend
            .transfer(account, Uuid::now_v7(), recipient, 20)
            .await
            .is_ok()
    );
    for blocked in [account, recipient] {
        sqlx::query(
            "UPDATE accounts SET email_change_blocked_until=now()+interval '72 hours' WHERE id=$1",
        )
        .bind(blocked)
        .execute(&db.pool)
        .await?;
        assert_eq!(
            backend
                .transfer(account, Uuid::now_v7(), recipient, 1)
                .await,
            Err(Error::Denied)
        );
        sqlx::query("UPDATE accounts SET email_change_blocked_until='-infinity' WHERE id=$1")
            .bind(blocked)
            .execute(&db.pool)
            .await?;
    }
    assert_eq!(
        backend
            .transfer(account, Uuid::now_v7(), recipient, 0)
            .await,
        Err(Error::Invalid)
    );
    // Another account cannot select this history through form/query parameters.
    let (other_cookie, _) = session(&db.pool, Uuid::now_v7()).await?;
    assert_eq!(
        app.oneshot(
            HttpRequest::get("/en/shop/history")
                .header(header::COOKIE, other_cookie)
                .body(Body::empty())?
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    db.close().await
}

#[test]
fn localized_currency_and_coin_plurals() {
    let pl = Locale("pl");
    let en = Locale("en");
    for (minor, polish, english) in [
        (1000, "10,00 zł", "PLN 10.00"),
        (4500, "45,00 zł", "PLN 45.00"),
        (1, "0,01 zł", "PLN 0.01"),
    ] {
        assert_eq!(pl.currency(minor, "PLN"), polish);
        assert_eq!(en.currency(minor, "PLN"), english);
    }
    for (count, suffix) in [
        (1, "coin"),
        (2, "coiny"),
        (4, "coiny"),
        (5, "coinów"),
        (12, "coinów"),
        (22, "coiny"),
        (250, "coinów"),
    ] {
        assert_eq!(
            pl.coin_count(count).replace(['\u{2068}', '\u{2069}'], ""),
            format!("{count} {suffix}")
        );
        let suffix = if count == 1 { "coin" } else { "coins" };
        assert_eq!(
            en.coin_count(count).replace(['\u{2068}', '\u{2069}'], ""),
            format!("{count} {suffix}")
        );
    }
}

#[tokio::test]
async fn postgres_value_eligibility_held_account_lock_sheds_within_read_budget() -> TestResult {
    use sqlx::Connection;
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'held@example.invalid',now(),'unused-test-hash','active','en')").bind(account).execute(&db.pool).await?;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await?;
    let mut blocker =
        sqlx::PgConnection::connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?).await?;
    sqlx::query("BEGIN").execute(&mut blocker).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM {schema}.accounts WHERE id=$1 FOR UPDATE"
    )))
    .bind(account)
    .execute(&mut blocker)
    .await?;

    let backend = backend::configured(db.pool.clone(), None);
    let started = Instant::now();
    let observed = tokio::time::timeout(Duration::from_secs(10), backend.active(account)).await;
    let elapsed = started.elapsed();
    sqlx::query("ROLLBACK").execute(&mut blocker).await?;
    blocker.close().await?;
    db.close().await?;

    let observed =
        observed.map_err(|_| "eligibility still waiting at 10 s on a held account lock")?;
    assert_eq!(observed, Err(Error::Unavailable));
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
    Ok(())
}

#[test]
fn r8_currency_minor_unit_scale() {
    assert_eq!(Locale("en").currency(500, "JPY"), "JPY 500");
    assert_eq!(Locale("pl").currency(500, "JPY"), "500 JPY");
    assert_eq!(Locale("en").currency(500, "KWD"), "KWD 0.500");
    assert_eq!(Locale("pl").currency(500, "CLF"), "0,0500 CLF");
}

#[tokio::test]
async fn order_history_handlers_scope_reads_and_reject_malformed_references() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL is required")?;
    let account = Uuid::now_v7();
    let other = Uuid::now_v7();
    for id in [account, other] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at) VALUES($1,$2,'test','active',now())")
            .bind(id)
            .bind(format!("{id}@example.test"))
            .execute(&db.pool)
            .await?;
    }
    let (cookie, _) = session(&db.pool, account).await?;
    let catalog = crate::catalog::Catalog::parse(br#"{"contract_version":1,"offers":[{"offer_key":"test.offer","currency":"COINS","unit_price":"10","max_quantity":1,"purchasable":true,"units":[{"product_id":"premium.test","product_version":"1","unit_kind":"PREMIUM_TIME","grant_key":"test","grant_revision":"1","quantity":1}]}]}"#)?;
    let order = Uuid::now_v7();
    let request = Uuid::now_v7();
    let operation = Uuid::now_v7();
    let mut tx = db.pool.begin().await?;
    crate::catalog::insert_revision(&mut tx, &catalog, account).await?;
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'store_purchase',$2,-10)")
        .bind(operation).bind(account).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO store_purchase_requests(purchase_request_id,account_id,request_bytes,request_digest,result,answer_bytes) VALUES($1,$2,'{}',$3,'ACCEPTED','{}')")
        .bind(request).bind(account).bind(catalog.revision()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO store_orders(order_id,purchase_request_id,account_id,character_id,world_id,wallet_operation_id,wallet_idempotency_key,currency,total_price) VALUES($1,$2,$3,$4,$5,$6,$7,'COINS',10)")
        .bind(order).bind(request).bind(account).bind(Uuid::now_v7()).bind(Uuid::now_v7())
        .bind(operation).bind(order.to_string()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO store_order_lines(line_id,order_id,catalog_revision,offer_key,quantity,unit_price) VALUES($1,$2,$3,'test.offer',1,10)")
        .bind(Uuid::now_v7()).bind(order).bind(catalog.revision()).execute(&mut *tx).await?;
    tx.commit().await?;
    let (other_cookie, _) = session(&db.pool, other).await?;
    let app = super::orders::routes(db.pool.clone()).layer(Extension(oteryn_ui::Chrome::default()));
    for (path, session, status) in [
        (
            "/pl/account/store-orders".to_owned(),
            cookie.as_str(),
            StatusCode::OK,
        ),
        (
            "/pl/account/store-orders".to_owned(),
            "",
            StatusCode::SEE_OTHER,
        ),
        (
            "/pl/account/store-orders?before=bad".to_owned(),
            cookie.as_str(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/pl/account/store-orders/bad".to_owned(),
            cookie.as_str(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!("/pl/account/store-orders/{order}"),
            cookie.as_str(),
            StatusCode::OK,
        ),
        (
            format!("/pl/account/store-orders/{order}"),
            other_cookie.as_str(),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get(&path)
                    .header(header::COOKIE, session)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), status, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    sqlx::query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    for path in [
        "/pl/account/store-orders".to_owned(),
        format!("/pl/account/store-orders/{order}"),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get(path)
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!body(response).await?.contains("test.offer"));
    }
    db.close().await
}
