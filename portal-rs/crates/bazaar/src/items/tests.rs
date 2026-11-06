use super::*;
use crate::{AccountState, Reservation, Settlement};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
struct TestAuthority(Arc<AtomicBool>);
impl ItemAuthority for TestAuthority {
    async fn cancellation_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
    ) -> std::result::Result<bool, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }

    async fn preflight(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: &CreateItem,
    ) -> std::result::Result<bool, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
    async fn publication(
        &self,
        _: &mut PgConnection,
        row: &Listing,
    ) -> std::result::Result<Option<Publication>, Error> {
        if !self.0.load(Ordering::SeqCst) {
            return Ok(None);
        }
        Ok(Some(Publication {
            name: "Test sword".into(),
            attributes: vec![],
            escrow_id: (row.form != Form::WantToBuy).then(Uuid::now_v7),
            escrow_receipt_digest: (row.form != Form::WantToBuy).then(|| "a".repeat(64)),
            projection_epoch: "1".into(),
            recovery_generation: "1".into(),
        }))
    }
    async fn current(&self, _: &mut PgConnection, _: &Listing) -> std::result::Result<bool, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
    async fn offer_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
        _: Uuid,
        _: Uuid,
    ) -> std::result::Result<bool, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
#[derive(Clone)]
struct TestAccounts;
impl Accounts for TestAccounts {
    async fn state(
        &self,
        _: &mut PgConnection,
        _: Uuid,
    ) -> std::result::Result<Option<AccountState>, Error> {
        Ok(Some(AccountState {
            active: true,
            email_verified: true,
            mfa: true,
            sudo: true,
            bazaar_blocked: false,
        }))
    }
}
#[derive(Clone)]
struct TestFunds;
impl Reservations for TestFunds {
    async fn lock_accounts(
        &self,
        db: &mut PgConnection,
        _: &[Uuid],
    ) -> std::result::Result<(), Error> {
        sqlx::query("SELECT pg_advisory_xact_lock(600)")
            .execute(db)
            .await?;
        Ok(())
    }
    async fn reserve(
        &self,
        db: &mut PgConnection,
        hold: Reservation,
    ) -> std::result::Result<(), Error> {
        if hold.amount > 100_000 {
            return Err(Error::InsufficientFunds);
        }
        let used: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(amount),0)::bigint FROM test_holds WHERE account=$1 AND id<>$2",
        )
        .bind(hold.account)
        .bind(hold.id)
        .fetch_one(&mut *db)
        .await?;
        if used + hold.amount > 100_000 {
            return Err(Error::InsufficientFunds);
        }
        sqlx::query("INSERT INTO test_holds VALUES ($1,$2,$3) ON CONFLICT(id,account) DO UPDATE SET amount=excluded.amount").bind(hold.id).bind(hold.account).bind(hold.amount).execute(db).await?;
        Ok(())
    }
    async fn release(
        &self,
        db: &mut PgConnection,
        id: Uuid,
        account: Uuid,
    ) -> std::result::Result<(), Error> {
        sqlx::query("DELETE FROM test_holds WHERE id=$1 AND account=$2")
            .bind(id)
            .bind(account)
            .execute(db)
            .await?;
        Ok(())
    }
    async fn settle(&self, _: &mut PgConnection, _: &Settlement) -> std::result::Result<(), Error> {
        Err(Error::Denied)
    }
    async fn complete(
        &self,
        _: &mut PgConnection,
        _: &Settlement,
    ) -> std::result::Result<(), Error> {
        Err(Error::Denied)
    }
}
struct Harness {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    api: Items<TestAccounts, TestFunds, TestAuthority>,
    authority: TestAuthority,
    owner: Uuid,
    buyer: Uuid,
}
impl Harness {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED item PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("items_test_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
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
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105152830_t57_bazaar_core.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105230103_t60_items.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE test_holds(id uuid,account uuid,amount bigint,PRIMARY KEY(id,account))",
        )
        .execute(&pool)
        .await?;
        let authority = TestAuthority::default();
        authority.0.store(true, Ordering::SeqCst);
        let api = Items::new(
            pool.clone(),
            TestAccounts,
            TestFunds,
            authority.clone(),
            ItemFlags { enabled: true },
        );
        Ok(Some(Self {
            admin,
            pool,
            schema,
            api,
            authority,
            owner: Uuid::now_v7(),
            buyer: Uuid::now_v7(),
        }))
    }
    async fn listing(&self, mode: Mode) -> Result<Uuid> {
        let id = self.api.create(self.owner, input(mode)).await?;
        self.api.publish(id, self.owner).await?;
        Ok(id)
    }
    async fn holds(&self) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT COALESCE(sum(amount),0)::bigint FROM test_holds")
                .fetch_one(&self.pool)
                .await?,
        )
    }
    async fn finish(self) -> Result {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
fn input(mode: Mode) -> CreateItem {
    CreateItem {
        request_id: Uuid::now_v7(),
        mode,
        definition_key: "fixture-sword".into(),
        definition_revision: "v1".into(),
        world_id: Uuid::now_v7(),
        seller_character_id: (mode != Mode::Buy).then(Uuid::now_v7),
        item_instance_id: (mode != Mode::Buy).then(Uuid::now_v7),
        quantity: 2,
        unit_price: if mode == Mode::Offers {
            None
        } else {
            Some(100)
        },
        buy_now_unit_price: None,
        auction_days: (mode == Mode::Auction).then_some(1),
        message: "Synthetic listing".into(),
    }
}

#[test]
fn rejects_bad_input_and_ambiguous_publication() {
    let mut x = input(Mode::Auction);
    assert!(x.validate().is_ok());
    x.quantity = 0;
    assert!(x.validate().is_err());
    x = input(Mode::Fixed);
    x.unit_price = Some(i64::MAX);
    assert!(x.validate().is_err());
    x = input(Mode::Buy);
    x.item_instance_id = Some(Uuid::now_v7());
    assert!(x.validate().is_err());
    let p = Publication {
        name: "Test".into(),
        attributes: vec![],
        escrow_id: None,
        escrow_receipt_digest: None,
        projection_epoch: "01".into(),
        recovery_generation: "1".into(),
    };
    assert!(p.validate(false).is_err());
}
#[tokio::test]
async fn pg_create_replay_and_publish_after_escrow() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let x = input(Mode::Fixed);
    let id = h.api.create(h.owner, x.clone()).await?;
    assert_eq!(h.api.create(h.owner, x.clone()).await?, id);
    let mut changed = x;
    changed.quantity = 3;
    assert!(matches!(
        h.api.create(h.owner, changed).await,
        Err(Error::IdempotencyConflict)
    ));
    assert!(matches!(h.api.view(id).await, Err(Error::NotFound)));
    h.authority.0.store(false, Ordering::SeqCst);
    assert!(matches!(
        h.api.publish(id, h.owner).await,
        Err(Error::Denied)
    ));
    h.authority.0.store(true, Ordering::SeqCst);
    h.api.publish(id, h.owner).await?;
    let view = h.api.view(id).await?;
    assert_eq!((view.quantity, view.unit_price), (2, Some(100)));
    let blocked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_item_outbox WHERE dispatch_state='blocked'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(blocked, 1);
    h.finish().await
}
#[tokio::test]
async fn pg_negotiated_offer_reserves_stack_and_accepts_once() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Offers).await?;
    assert_eq!(h.api.view(id).await?.unit_price, None);
    let request = Uuid::now_v7();
    let o = h.api.offer(id, h.buyer, 150, request).await?;
    assert_eq!(o.amount, 300);
    assert_eq!(h.holds().await?, 300);
    assert_eq!(h.api.offer(id, h.buyer, 150, request).await?.id, o.id);
    assert!(matches!(
        h.api.accept(id, h.buyer, o.id).await,
        Err(Error::Denied)
    ));
    h.api.accept(id, h.owner, o.id).await?;
    h.api.accept(id, h.owner, o.id).await?;
    assert!(matches!(
        h.api.reject(id, h.owner, o.id).await,
        Err(Error::Denied)
    ));
    let deliveries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_item_outbox WHERE step='delivery'")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(deliveries, 1);
    h.finish().await
}
#[tokio::test]
async fn pg_fixed_price_refuses_self_wrong_amount_and_unfunded_offer() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Fixed).await?;
    assert!(h.api.offer(id, h.owner, 100, Uuid::now_v7()).await.is_err());
    assert!(h.api.offer(id, h.buyer, 99, Uuid::now_v7()).await.is_err());
    let o = h.api.offer(id, h.buyer, 100, Uuid::now_v7()).await?;
    h.api.reject(id, h.owner, o.id).await?;
    assert_eq!(h.holds().await?, 0);
    assert!(h.api.cancel(id, h.owner).await.is_err());
    let expensive = h
        .api
        .create(h.owner, {
            let mut x = input(Mode::Fixed);
            x.unit_price = Some(100_000);
            x
        })
        .await?;
    h.api.publish(expensive, h.owner).await?;
    assert!(matches!(
        h.api
            .offer(expensive, h.buyer, 100_000, Uuid::now_v7())
            .await,
        Err(Error::InsufficientFunds)
    ));
    assert_eq!(h.holds().await?, 0);
    h.finish().await
}
#[tokio::test]
async fn pg_buy_notice_requires_seller_escrow_and_buyer_funding() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Buy).await?;
    let o = h.api.offer(id, h.buyer, 90, Uuid::now_v7()).await?;
    assert_eq!(h.holds().await?, 0);
    h.authority.0.store(false, Ordering::SeqCst);
    assert!(matches!(
        h.api.accept(id, h.owner, o.id).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.holds().await?, 0);
    h.authority.0.store(true, Ordering::SeqCst);
    h.api.accept(id, h.owner, o.id).await?;
    let payer: Uuid = sqlx::query_scalar("SELECT account FROM test_holds")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(payer, h.owner);
    assert_eq!(h.holds().await?, 180);
    h.finish().await
}
#[tokio::test]
async fn pg_auction_proxy_retry_and_sniping() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Auction).await?;
    let end:OffsetDateTime=sqlx::query_scalar("UPDATE bazaar_listings SET ends_at=clock_timestamp()+interval '1 minute' WHERE id=$1 RETURNING ends_at").bind(id).fetch_one(&h.pool).await?;
    let request = Uuid::now_v7();
    let first = h.api.bid(id, h.buyer, 500, request).await?;
    assert_eq!(first.price, 200);
    assert_eq!(first.ends_at, end + Duration::minutes(5));
    assert_eq!(h.api.bid(id, h.buyer, 500, request).await?, first);
    let other = Uuid::now_v7();
    let second = h.api.bid(id, other, 200, Uuid::now_v7()).await?;
    assert_eq!(second.leader, h.buyer);
    assert_eq!(second.price, 402);
    assert_eq!(h.holds().await?, 1000);
    assert!(h.api.cancel(id, h.owner).await.is_err());
    assert!(h.api.bid(id, h.owner, 900, Uuid::now_v7()).await.is_err());
    h.finish().await
}
#[tokio::test]
async fn pg_buy_now_disabled_after_bid_and_idempotent() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let mut x = input(Mode::Auction);
    x.buy_now_unit_price = Some(500);
    let id = h.api.create(h.owner, x.clone()).await?;
    h.api.publish(id, h.owner).await?;
    let request = Uuid::now_v7();
    h.api.buy_now(id, h.buyer, request).await?;
    h.api.buy_now(id, h.buyer, request).await?;
    assert_eq!(h.holds().await?, 1000);
    x.request_id = Uuid::now_v7();
    let second = h.api.create(h.owner, x).await?;
    h.api.publish(second, h.owner).await?;
    h.api.bid(second, h.buyer, 200, Uuid::now_v7()).await?;
    assert!(
        h.api
            .buy_now(second, Uuid::now_v7(), Uuid::now_v7())
            .await
            .is_err()
    );
    h.finish().await
}
#[tokio::test]
async fn pg_cancel_expire_renew_keep_blocked_compensation() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Fixed).await?;
    h.api.renew(id, h.owner, Uuid::now_v7()).await?;
    h.api.cancel(id, h.owner).await?;
    h.api.cancel(id, h.owner).await?;
    assert!(h.api.view(id).await.is_err());
    let id = h.listing(Mode::Fixed).await?;
    h.api.offer(id, h.buyer, 100, Uuid::now_v7()).await?;
    sqlx::query(
        "UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(id)
    .execute(&h.pool)
    .await?;
    h.api.expire(id).await?;
    assert_eq!(h.holds().await?, 0);
    let returns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_item_outbox WHERE step='return' AND dispatch_state='blocked'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(returns, 2);
    h.finish().await
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pg_competing_acceptance_commits_one_delivery() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Offers).await?;
    let a = h.api.offer(id, h.buyer, 150, Uuid::now_v7()).await?;
    let b = h.api.offer(id, Uuid::now_v7(), 200, Uuid::now_v7()).await?;
    let (a, b) = tokio::join!(
        h.api.accept(id, h.owner, a.id),
        h.api.accept(id, h.owner, b.id)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let accepted: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_offers WHERE state='accepted'")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(accepted, 1);
    assert!([300, 400].contains(&h.holds().await?));
    h.finish().await
}
#[tokio::test]
async fn pg_stale_authority_denies_reads_bids_and_creates() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let id = h.listing(Mode::Auction).await?;
    h.authority.0.store(false, Ordering::SeqCst);
    assert!(matches!(h.api.view(id).await, Err(Error::Denied)));
    assert!(matches!(
        h.api.bid(id, h.buyer, 500, Uuid::now_v7()).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        h.api.create(h.owner, input(Mode::Fixed)).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.holds().await?, 0);
    h.finish().await
}
#[tokio::test]
async fn default_router_is_localized_and_disabled() -> Result {
    use tower::ServiceExt;
    let response = web::configured_router()
        .oneshot(
            axum::http::Request::builder()
                .uri("/pl/bazaar/items")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body = axum::body::to_bytes(response.into_body(), 8192).await?;
    assert!(std::str::from_utf8(&body)?.contains("tymczasowo"));
    Ok(())
}

#[tokio::test]
async fn pg_browser_authenticated_form_success_refusal_and_invalid_input() -> Result {
    use axum::{
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Request, StatusCode},
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&h.pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105160300_t04a_rate_limits.sql"
    ))
    .execute(&h.pool)
    .await?;
    let token = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES ($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',$3)")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(h.owner).bind(&csrf).execute(&h.pool).await?;
    let api = Items::new(
        h.pool.clone(),
        TestAccounts,
        TestFunds,
        h.authority.clone(),
        ItemFlags { enabled: true },
    );
    let app = web::router(
        api,
        h.pool.clone(),
        oteryn_web::csrf::CsrfConfig {
            trusted_origins: vec!["https://example.test".into()],
            ..Default::default()
        },
    );
    let cookie = format!("{}={token}", oteryn_web::session::COOKIE_NAME);
    let peer = ConnectInfo("127.0.0.1:44000".parse::<std::net::SocketAddr>()?);
    let x = input(Mode::Fixed);
    let form = format!(
        "_csrf={csrf}&request_id={}&mode=fixed&definition_key=fixture-sword&definition_revision=v1&world_id={}&seller_character_id={}&item_instance_id={}&quantity=2&unit_price=100&message=Hello",
        x.request_id,
        x.world_id,
        x.seller_character_id.ok_or("missing character")?,
        x.item_instance_id.ok_or("missing item")?
    );
    let post = |path: &str, body: String| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("cookie", &cookie)
            .header("origin", "https://example.test")
            .header("content-type", "application/x-www-form-urlencoded")
            .extension(peer)
            .body(Body::from(body))
    };
    let response = app
        .clone()
        .oneshot(post("/en/bazaar/items", form.clone())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()["location"].to_str()?.to_string();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&location)
                .header("cookie", &cookie)
                .extension(peer)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Publish after deposit confirmation"));
    assert!(!html.contains("leader_max"));
    std::fs::create_dir_all("/tmp/t60-ui")?;
    std::fs::write("/tmp/t60-ui/item.html", &html)?;
    let response = app
        .clone()
        .oneshot(post(
            &format!("{location}/offer"),
            format!("_csrf={csrf}&request_id={}&amount=100", Uuid::now_v7()),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let response = app
        .clone()
        .oneshot(post(
            "/en/bazaar/items",
            form.replace("quantity=2", "quantity=0"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .oneshot(post(
            &format!("{location}/publish"),
            format!("_csrf=wrong&request_id={}", Uuid::now_v7()),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    h.finish().await
}

#[tokio::test]
async fn pg_auction_close_freezes_sale_without_payout_or_release() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let listing = h.listing(Mode::Auction).await?;
    h.api.bid(listing, h.buyer, 500, Uuid::now_v7()).await?;
    sqlx::query(
        "UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(listing)
    .execute(&h.pool)
    .await?;
    h.api.close_auction(listing).await?;
    let sale: (Uuid, Uuid, i64, i64) = sqlx::query_as(
        "SELECT buyer,seller,gross_amount,fee_amount FROM bazaar_item_sales WHERE listing_id=$1",
    )
    .bind(listing)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(sale, (h.buyer, h.owner, 200, 20));
    assert_eq!(h.holds().await?, 1000);
    assert!(h.api.close_auction(listing).await.is_err());
    assert!(h.api.cancel(listing, h.owner).await.is_err());
    assert!(h.api.expire(listing).await.is_err());
    h.finish().await
}
#[tokio::test]
async fn default_flags_deny_before_database_access() -> Result {
    let pool = PgPoolOptions::new().connect_lazy("postgres://unused@localhost/unused")?;
    let api = Items::new(
        pool,
        TestAccounts,
        TestFunds,
        Unavailable,
        ItemFlags::default(),
    );
    assert!(matches!(
        api.create(Uuid::now_v7(), input(Mode::Fixed)).await,
        Err(Error::Disabled)
    ));
    Ok(())
}

#[tokio::test]
async fn cancellation_without_return_stays_pending_and_keeps_the_same_request() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let listing = h.listing(Mode::Fixed).await?;
    h.authority.0.store(false, Ordering::SeqCst);
    h.api.cancel(listing, h.owner).await?;
    h.api.cancel(listing, h.owner).await?;
    let state: String = sqlx::query_scalar("SELECT state::text FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(state, "active");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_item_outbox WHERE listing_id=$1 AND step='return'",
    )
    .bind(listing)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    h.finish().await
}
