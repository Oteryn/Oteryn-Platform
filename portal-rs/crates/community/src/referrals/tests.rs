#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::get,
};
use oteryn_store::{catalog, purchase};
use oteryn_wallet::{AccountStateReader, AdjustmentAudit, AuditHook, TransferBlockCheck};
use tower::ServiceExt;

#[derive(Debug, Clone, Copy)]
struct Checks;
impl AccountStateReader for Checks {
    async fn state(&self, conn: &mut PgConnection, account: Uuid) -> oteryn_wallet::AccountState {
        if DatabasePolicy
            .allowed(conn, account, account, Kind::New, Stage::Statistics)
            .await
            .unwrap_or(false)
        {
            oteryn_wallet::AccountState::Active
        } else {
            oteryn_wallet::AccountState::Unknown
        }
    }
}
impl oteryn_notify::AccountState for Checks {
    fn active_verified(
        &self,
        _: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async {
            Ok(Some(oteryn_notify::Account {
                email: "player@example.test".into(),
                locale: oteryn_notify::Locale::En,
            }))
        })
    }
}
impl TransferBlockCheck for Checks {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> oteryn_wallet::TransferPermission {
        oteryn_wallet::TransferPermission::Allowed
    }
}
impl AuditHook for Checks {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
impl purchase::Accounts for Checks {
    async fn state(
        &self,
        conn: &mut PgConnection,
        account: Uuid,
    ) -> Result<purchase::AccountState, purchase::Error> {
        Ok(
            if DatabasePolicy
                .allowed(conn, account, account, Kind::New, Stage::Statistics)
                .await
                .map_err(|_| purchase::Error::Unavailable)?
            {
                purchase::AccountState::Active
            } else {
                purchase::AccountState::Restricted
            },
        )
    }
}
impl catalog::CatalogAccess for Checks {
    async fn account_state(
        &self,
        _: Uuid,
        _: &mut PgConnection,
    ) -> Result<catalog::AccountState, catalog::Error> {
        Ok(catalog::AccountState::Active)
    }
    async fn permitted(
        &self,
        _: Uuid,
        _: catalog::Action,
        _: &mut PgConnection,
    ) -> Result<bool, catalog::Error> {
        Ok(true)
    }
}
const CATALOG: &str = r#"{"contract_version":1,"offers":[{"offer_key":"premium-30d","currency":"COINS","unit_price":"250","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.premium_time","product_version":"1","unit_kind":"PREMIUM_TIME","grant_key":"premium_time","grant_revision":"1","quantity":30}]}]}"#;
type Purchases = purchase::Purchases<Checks, Checks, Checks, Checks>;
struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
    campaign: Campaign,
    revision: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED N012 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("referrals_campaign_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(12)
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
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        sqlx::query("UPDATE referral_campaign_settings SET enabled=true")
            .execute(&pool)
            .await
            .unwrap();
        let store =
            catalog::CatalogStore::new(pool.clone(), catalog::CatalogFlags { enabled: true });
        let actor = Uuid::now_v7();
        let revision = store
            .import(CATALOG.as_bytes(), actor, &Checks)
            .await
            .unwrap();
        store.activate(&revision, actor, &Checks).await.unwrap();
        let campaign = Campaign::new(
            pool.clone(),
            Flags { enabled: true },
            DatabasePolicy,
            "https://portal.example.test".into(),
        )
        .unwrap();
        Some(Self {
            pool,
            root,
            schema,
            campaign,
            revision,
        })
    }
    async fn account(&self) -> Uuid {
        sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES($1,'test-only','en','active',clock_timestamp()) RETURNING id")
            .bind(format!("{}@example.test",Uuid::now_v7())).fetch_one(&self.pool).await.unwrap()
    }
    async fn email(&self, account: Uuid) -> String {
        sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn inactive(&self, account: Uuid, months: i32) {
        sqlx::query("UPDATE accounts SET created_at=clock_timestamp()-make_interval(months=>$2+1) WHERE id=$1").bind(account).bind(months).execute(&self.pool).await.unwrap();
        sqlx::query("UPDATE referral_activity SET last_activity=clock_timestamp()-make_interval(months=>$2)-interval '1 minute',created_at=clock_timestamp()-make_interval(months=>$2)-interval '1 minute' WHERE account_id=$1").bind(account).bind(months).execute(&self.pool).await.unwrap();
    }
    async fn recruit(&self, inviter: Uuid) -> Uuid {
        let code = self.campaign.link(inviter).await.unwrap();
        let invitee = self.account().await;
        self.campaign.accept(invitee, code).await.unwrap();
        invitee
    }
    fn wallet(&self) -> oteryn_wallet::Ledger<Checks, Checks, Checks> {
        oteryn_wallet::Ledger::new(
            self.pool.clone(),
            oteryn_wallet::WalletFlags { enabled: true },
            Checks,
            Checks,
            Checks,
        )
    }
    fn purchases(&self) -> Purchases {
        purchase::Purchases::new(
            self.pool.clone(),
            purchase::Settings {
                purchase_enabled: true,
                delivery_enabled: true,
                environment: "testing".into(),
                source_authority: "referral-tests".into(),
                peer_identity: "referral.test".into(),
                enabled_unit_kinds: vec!["PREMIUM_TIME".into()],
                purchases_per_account_per_minute: 60,
                ..purchase::Settings::default()
            },
            self.wallet(),
            Checks,
        )
    }
    fn request(&self, account: Uuid, quantity: u8, key: Uuid) -> Vec<u8> {
        serde_json::to_vec(&purchase::PurchaseRequest {
            contract_version: 1,
            operation: "PurchaseStoreOfferV1".into(),
            source_authority: "referral-tests".into(),
            purchase_request_id: key.to_string(),
            account_id: account.to_string(),
            character_id: Uuid::now_v7().to_string(),
            world_id: Uuid::now_v7().to_string(),
            catalog_revision: self.revision.clone(),
            offer_key: "premium-30d".into(),
            currency: "COINS".into(),
            unit_price: "250".into(),
            quantity,
        })
        .unwrap()
    }
    async fn fund(&self, account: Uuid) {
        self.wallet()
            .adjust(
                Uuid::now_v7(),
                account,
                10000,
                account,
                "Referral test funding",
            )
            .await
            .unwrap();
    }
    async fn buy(&self, account: Uuid, quantity: u8) {
        self.fund(account).await;
        let raw = self.request(account, quantity, Uuid::now_v7());
        let answer = self.purchases().purchase(&raw).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&answer).unwrap()["result"],
            "ACCEPTED"
        );
        assert_eq!(self.purchases().purchase(&raw).await.unwrap(), answer);
    }
    async fn count(&self, kind: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM referral_campaign_rewards WHERE kind=$1")
            .bind(kind)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
}
#[tokio::test]
async fn referrals_email_rate_limit_25_per_day() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    for n in 0..25 {
        db.campaign
            .email(owner, &format!("friend{n}@example.test"), Kind::New)
            .await
            .unwrap();
    }
    assert_eq!(
        db.campaign
            .email(owner, "last@example.test", Kind::New)
            .await,
        Err(Error::Denied)
    );
    assert_eq!(
        db.campaign.email(owner, "bad input", Kind::New).await,
        Err(Error::Invalid)
    );
    sqlx::query("UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '24 hours 1 second'").execute(&db.pool).await.unwrap();
    db.campaign
        .email(owner, "last@example.test", Kind::New)
        .await
        .unwrap();
    let sender = Arc::new(oteryn_notify::CaptureSender::default());
    Worker {
        campaign: db.campaign.clone(),
        sender: Some(sender.clone()),
    }
    .process()
    .await
    .unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 25);
    db.close().await;
}
#[tokio::test]
async fn referrals_resend_after_14_days() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    db.campaign
        .email(owner, "friend@example.test", Kind::New)
        .await
        .unwrap();
    assert_eq!(
        db.campaign
            .email(owner, " FRIEND@example.test ", Kind::New)
            .await,
        Err(Error::Denied)
    );
    sqlx::query("UPDATE referral_email_invitations SET created_at=clock_timestamp()-interval '14 days 1 second'").execute(&db.pool).await.unwrap();
    db.campaign
        .email(owner, "friend@example.test", Kind::New)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM referral_email_attempts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    db.close().await;
}
#[tokio::test]
async fn referrals_outfit_on_first_paid_purchase() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    assert_eq!(db.count("outfit").await, 0);
    let refusal = db
        .purchases()
        .purchase(&db.request(invitee, 1, Uuid::now_v7()))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&refusal).unwrap()["result"],
        "INSUFFICIENT_BALANCE"
    );
    assert!(db.purchases().purchase(b"{}").await.is_err());
    assert_eq!(db.count("outfit").await, 0);
    db.buy(invitee, 1).await;
    db.buy(invitee, 1).await;
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}
#[tokio::test]
async fn referrals_addons_at_3_and_10() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    for n in 1..=10 {
        let invitee = db.recruit(owner).await;
        db.buy(invitee, 1).await;
        assert_eq!(db.count("addon1").await, i64::from(n >= 3));
        assert_eq!(db.count("addon2").await, i64::from(n >= 10));
    }
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}
#[tokio::test]
async fn referrals_loyalty_1_per_6() {
    let Some(db) = Database::open().await else {
        return;
    };
    let grandparent = db.account().await;
    let owner = db.recruit(grandparent).await;
    let invitee = db.recruit(owner).await;
    for points in [5, 6, 6] {
        sqlx::query("SELECT referral_campaign_loyalty($1,$2)")
            .bind(invitee)
            .bind(points)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let points: i64 =
        sqlx::query_scalar("SELECT sum(points)::bigint FROM referral_loyalty_credits")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(points, 1);
    // A controlled historical premium interval lets the real snapshot consume 13 days.
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(invitee)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES($1,$2,$2,'GRANT',30,$3,$4,1,2592000,date_trunc('second',now())-interval '13 days 1 minute',date_trunc('second',now())+interval '17 days'-interval '1 minute')")
        .bind(Uuid::now_v7()).bind(invitee).bind(vec![0_u8;32]).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    let ledger =
        oteryn_store::loyalty::Ledger::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    for _ in 0..2 {
        let bytes = ledger
            .snapshot(invitee, &"a".repeat(32), &"b".repeat(40))
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["loyalty_points"],
            13
        );
    }
    for _ in 0..2 {
        let bytes = ledger
            .snapshot(owner, &"a".repeat(32), &"b".repeat(40))
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["loyalty_points"],
            2
        );
    }
    let points: i64 =
        sqlx::query_scalar("SELECT sum(points)::bigint FROM referral_loyalty_credits")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(points, 2);
    sqlx::query("INSERT INTO referral_loyalty_credits(operation_key,account_id,points) VALUES('historical-referral-credit',$1,12)").bind(owner).execute(&db.pool).await.unwrap();
    for _ in 0..2 {
        let bytes = ledger
            .snapshot(owner, &"a".repeat(32), &"b".repeat(40))
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["loyalty_points"],
            14
        );
    }
    let upstream: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(points),0)::bigint FROM referral_loyalty_credits WHERE account_id=$1",
    )
    .bind(grandparent)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(upstream, 0);
    db.close().await;
}
#[tokio::test]
async fn referrals_buddy_requires_12_months_inactive() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let buddy = db.account().await;
    let email = db.email(buddy).await;
    // Old retained activity alone does not prove twelve months of coverage.
    sqlx::query("UPDATE referral_activity SET last_activity=clock_timestamp()-interval '13 months' WHERE account_id=$1").bind(buddy).execute(&db.pool).await.unwrap();
    assert_eq!(
        db.campaign.email(owner, &email, Kind::Returning).await,
        Err(Error::Denied)
    );
    db.inactive(buddy, 11).await;
    assert_eq!(
        db.campaign.email(owner, &email, Kind::Returning).await,
        Err(Error::Denied)
    );
    db.inactive(buddy, 12).await;
    db.campaign
        .email(owner, &email, Kind::Returning)
        .await
        .unwrap();
    db.close().await;
}
#[tokio::test]
async fn referrals_buddy_limit_20_per_30_days() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    for _ in 0..20 {
        let buddy = db.account().await;
        db.inactive(buddy, 13).await;
        db.campaign
            .email(owner, &db.email(buddy).await, Kind::Returning)
            .await
            .unwrap();
    }
    let buddy = db.account().await;
    db.inactive(buddy, 13).await;
    assert_eq!(
        db.campaign
            .email(owner, &db.email(buddy).await, Kind::Returning)
            .await,
        Err(Error::Denied)
    );
    sqlx::query("UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '30 days 1 second'").execute(&db.pool).await.unwrap();
    db.campaign
        .email(owner, &db.email(buddy).await, Kind::Returning)
        .await
        .unwrap();
    db.close().await;
}
#[tokio::test]
async fn referrals_buddy_premium_5_and_10_days_within_90() {
    let Some(db) = Database::open().await else {
        return;
    };
    for (quantity, expected, expired) in [(1, 5, false), (3, 10, false), (3, 0, true)] {
        let owner = db.account().await;
        let buddy = db.account().await;
        db.inactive(buddy, 13).await;
        db.campaign
            .email(owner, &db.email(buddy).await, Kind::Returning)
            .await
            .unwrap();
        if expired {
            sqlx::query("UPDATE referral_campaign_invitations SET created_at=clock_timestamp()-interval '90 days 1 second' WHERE invitee_id=$1").bind(buddy).execute(&db.pool).await.unwrap();
        }
        db.buy(buddy, quantity).await;
        let worker = Worker {
            campaign: db.campaign.clone(),
            sender: None,
        };
        worker.process().await.unwrap();
        worker.process().await.unwrap();
        let days: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(duration_days),0)::bigint FROM premium_ledger WHERE account_id=$1",
        )
        .bind(owner)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(days, expected);
        // Further purchases neither extend the deadline nor change an awarded tier.
        db.buy(buddy, 3).await;
        worker.process().await.unwrap();
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM premium_ledger WHERE account_id=$1")
                .bind(owner)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(rows, i64::from(!expired));
    }
    db.close().await;
}
#[tokio::test]
async fn referrals_rewards_once_only_under_concurrency() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    db.fund(invitee).await;
    let raw = db.request(invitee, 1, Uuid::now_v7());
    let service = Arc::new(db.purchases());
    let mut jobs = Vec::new();
    for _ in 0..12 {
        let service = service.clone();
        let raw = raw.clone();
        jobs.push(tokio::spawn(async move {
            service.purchase(&raw).await.unwrap()
        }));
    }
    let mut results = Vec::new();
    for job in jobs {
        results.push(job.await.unwrap());
    }
    assert!(results.iter().all(|r| r == &results[0]));
    assert_eq!(db.count("outfit").await, 1);
    let mut competing = Vec::new();
    for _ in 0..6 {
        let service = service.clone();
        let raw = db.request(invitee, 1, Uuid::now_v7());
        competing.push(tokio::spawn(async move {
            service.purchase(&raw).await.unwrap()
        }));
    }
    for task in competing {
        task.await.unwrap();
    }
    assert_eq!(db.count("outfit").await, 1);
    for _ in 0..12 {
        sqlx::query("SELECT referral_campaign_loyalty($1,12)")
            .bind(invitee)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let credits: i64 =
        sqlx::query_scalar("SELECT sum(points)::bigint FROM referral_loyalty_credits")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(credits, 2);
    drop(service);
    db.close().await;
}
#[tokio::test]
async fn referrals_self_referral_denied() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    assert_eq!(db.campaign.accept(owner, code).await, Err(Error::Denied));
    assert_eq!(
        db.campaign
            .email(owner, &db.email(owner).await, Kind::New)
            .await,
        Err(Error::Denied)
    );
    assert_eq!(
        db.campaign.accept(owner, Uuid::nil()).await,
        Err(Error::Invalid)
    );
    let other = db.account().await;
    let mut tx = db.campaign.begin(&[]).await.unwrap();
    db.campaign
        .bind(&mut tx, owner, other, Kind::Returning)
        .await
        .unwrap_err();
    tx.rollback().await.unwrap();
    let mut off = db.campaign.clone();
    off.flags.enabled = false;
    assert_eq!(off.link(owner).await, Err(Error::Disabled));
    db.close().await;
}
#[tokio::test]
async fn referrals_starter_and_first_returning_character_pending_entitlements() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    let (amount,seconds):(i32,i64)=sqlx::query_as("SELECT r.amount,extract(epoch FROM r.expires_at-a.created_at)::bigint FROM referral_campaign_rewards r JOIN accounts a ON a.id=r.account_id WHERE r.kind='starter' AND r.account_id=$1").bind(invitee).fetch_one(&db.pool).await.unwrap();
    assert_eq!((amount, seconds), (3, 604800));
    let code = db.campaign.link(owner).await.unwrap();
    db.campaign.accept(invitee, code).await.unwrap();
    assert_eq!(db.count("starter").await, 1);
    let buddy = db.account().await;
    db.inactive(buddy, 13).await;
    db.campaign
        .email(owner, &db.email(buddy).await, Kind::Returning)
        .await
        .unwrap();
    let first = Uuid::now_v7();
    let login = Uuid::now_v7();
    sqlx::query("INSERT INTO pd_game_logins(account_id,login_event_id,projection_epoch,recovery_generation,source_revision,entered_at) VALUES($1,$2,1,1,1,(extract(epoch FROM clock_timestamp())*1000)::bigint)")
        .bind(buddy).bind(login).execute(&db.pool).await.unwrap();
    assert_eq!(db.count("welcome").await, 0);
    sqlx::query("UPDATE pd_game_logins SET character_id=$3,world_id=$4,projection_revision=1,character_revision=1,source_revision=2 WHERE account_id=$1 AND login_event_id=$2")
        .bind(buddy).bind(login).bind(first).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    {
        let character = Uuid::now_v7();
        sqlx::query("INSERT INTO pd_game_logins(account_id,login_event_id,character_id,world_id,projection_epoch,recovery_generation,projection_revision,character_revision,source_revision,entered_at) VALUES($1,$2,$3,$4,1,1,1,1,1,(extract(epoch FROM clock_timestamp())*1000)::bigint)").bind(buddy).bind(Uuid::now_v7()).bind(character).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    }
    let character: Uuid = sqlx::query_scalar(
        "SELECT character_id FROM referral_campaign_rewards WHERE kind='welcome'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(character, first);
    assert_eq!(db.count("welcome").await, 1);
    let held:i64=sqlx::query_scalar("SELECT count(*) FROM outbox WHERE kind='referral.entitlement' AND state='pending' AND run_after='infinity'").fetch_one(&db.pool).await.unwrap();
    assert_eq!(held, 2);
    db.close().await;
}
async fn test_login(
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Path(account): Path<Uuid>,
) -> String {
    oteryn_web::session::rotate(
        &session,
        oteryn_web::session::PrivilegeChange::Login(account),
    )
    .await
    .unwrap();
    session.csrf_token().await.unwrap()
}
use axum::extract::{Extension, Path};
#[tokio::test]
async fn referrals_http_real_router_session_csrf_locales_and_bad_input() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let mut layer = oteryn_web::session::SessionLayer::new(db.pool.clone());
    layer.csrf.trusted_origins = vec![db.campaign.origin.clone()];
    let app = web::routes(db.campaign.clone())
        .merge(Router::new().route("/test-login/{account}", get(test_login)))
        .layer(axum::middleware::from_fn_with_state(
            layer,
            oteryn_web::session::middleware,
        ));
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/test-login/{owner}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let csrf =
        String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap();
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{lang}/account/referrals"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = String::from_utf8(
            to_bytes(response.into_body(), 32768)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(body.contains("width=device-width"));
        assert!(body.contains("for=\"new-email\""));
        assert!(body.contains("for=\"buddy-email\""));
        assert!(body.contains(&format!("lang=\"{lang}\"")));
        if let Ok(directory) = std::env::var("PORTAL_REFERRALS_RENDER_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{lang}.html")),
                &body,
            )
            .unwrap();
        }
    }
    let send = |body: String| {
        Request::post("/en/account/referrals")
            .header("cookie", &cookie)
            .header("origin", "https://portal.example.test")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(send("email=a%40example.test&kind=new&_csrf=bad".into()))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(send(format!("email=bad&kind=new&_csrf={csrf}")))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.clone()
            .oneshot(send(format!(
                "email=friend%40example.test&kind=new&_csrf={csrf}"
            )))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let sent: i64 = sqlx::query_scalar("SELECT count(*) FROM referral_email_attempts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(sent, 1);
    assert_eq!(
        app.clone()
            .oneshot(
                Request::get("/en/account/referrals")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        web::router(
            Campaign::new(
                db.pool.clone(),
                Flags::default(),
                DatabasePolicy,
                db.campaign.origin.clone()
            )
            .unwrap()
        )
        .oneshot(
            Request::get("/en/account/referrals")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
    drop(app);
    db.close().await;
}

#[tokio::test]
async fn referrals_atomic_rollback_and_disabled_hooks() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    db.fund(invitee).await;
    sqlx::raw_sql("CREATE FUNCTION reject_referral_reward() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'reward test failure'; END $$; CREATE TRIGGER reject_reward BEFORE INSERT ON referral_campaign_rewards FOR EACH ROW EXECUTE FUNCTION reject_referral_reward();").execute(&db.pool).await.unwrap();
    assert!(
        db.purchases()
            .purchase(&db.request(invitee, 1, Uuid::now_v7()))
            .await
            .is_err()
    );
    assert_eq!(db.wallet().balance(invitee).await.unwrap().available, 10000);
    let orders: i64 = sqlx::query_scalar("SELECT count(*) FROM store_orders")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(orders, 0);
    sqlx::query("UPDATE referral_campaign_settings SET enabled=false")
        .execute(&db.pool)
        .await
        .unwrap();
    let response = db
        .purchases()
        .purchase(&db.request(invitee, 1, Uuid::now_v7()))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response).unwrap()["result"],
        "ACCEPTED"
    );
    assert_eq!(db.count("outfit").await, 0);
    assert_eq!(db.campaign.link(owner).await, Err(Error::Disabled));
    db.close().await;
}

#[tokio::test]
async fn referrals_link_survives_real_session_rotation_and_reconciles_signup() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let mut layer = oteryn_web::session::SessionLayer::new(db.pool.clone());
    layer.csrf.trusted_origins = vec![db.campaign.origin.clone()];
    let app = web::routes(db.campaign.clone())
        .merge(Router::new().route("/test-login/{account}", get(test_login)))
        .layer(axum::middleware::from_fn_with_state(
            layer,
            oteryn_web::session::middleware,
        ));
    let path = format!("/en/referrals/{code}");
    let response = app
        .clone()
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 32768)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("origin", "https://portal.example.test")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["location"], "/en/register");
    let invitee = db.account().await;
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/test-login/{invitee}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // The first purchase must qualify immediately after the real session rotation.
    db.buy(invitee, 1).await;
    let paid: bool = sqlx::query_scalar(
        "SELECT first_paid_at IS NOT NULL FROM referral_campaign_invitations WHERE invitee_id=$1",
    )
    .bind(invitee)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(paid);
    Worker {
        campaign: db.campaign.clone(),
        sender: None,
    }
    .process()
    .await
    .unwrap();
    let binding: Uuid = sqlx::query_scalar(
        "SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=$1",
    )
    .bind(invitee)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(binding, owner);
    assert_eq!(db.count("starter").await, 1);
    drop(app);
    db.close().await;
}

async fn attribution_session(db: &Database, account: Option<Uuid>, code: &str) -> Vec<u8> {
    let hash = Uuid::now_v7().as_bytes().repeat(2);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 day','regression',$3,jsonb_build_object('referral.code',$4::text))")
        .bind(&hash).bind(account).bind("a".repeat(64)).bind(code).execute(&db.pool).await.unwrap();
    hash
}

#[tokio::test]
async fn nwave_completed_attribution_does_not_starve_later_batches() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let first = db.account().await;
    db.campaign.accept(first, code).await.unwrap();
    // Seed a backlog while synchronous binding is off, as on the prior version.
    sqlx::query("UPDATE referral_campaign_settings SET enabled=false")
        .execute(&db.pool)
        .await
        .unwrap();
    for _ in 0..100 {
        attribution_session(&db, Some(first), &code.to_string()).await;
    }
    sqlx::query("UPDATE referral_campaign_settings SET enabled=true")
        .execute(&db.pool)
        .await
        .unwrap();
    for n in 0..101 {
        let account = db.account().await;
        let code = if n == 0 {
            Uuid::nil().to_string()
        } else {
            format!("invalid-{n}")
        };
        attribution_session(&db, Some(account), &code).await;
    }
    let worker = Worker {
        campaign: db.campaign.clone(),
        sender: None,
    };
    for _ in 0..4 {
        worker.process().await.unwrap();
    }
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE data ? 'referral.code'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    db.close().await;
}

#[tokio::test]
async fn nwave_signup_attribution_precedes_first_purchase() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    // Exercise both orders: verification before login, and login before verification.
    for verify_first in [true, false] {
        let hash = attribution_session(&db, None, &code.to_string()).await;
        let invitee = db.account().await;
        sqlx::query(
            "UPDATE accounts SET status='pending_verification',email_verified_at=NULL WHERE id=$1",
        )
        .bind(invitee)
        .execute(&db.pool)
        .await
        .unwrap();
        if verify_first {
            sqlx::query("UPDATE accounts SET status='active',email_verified_at=clock_timestamp() WHERE id=$1").bind(invitee).execute(&db.pool).await.unwrap();
        }
        sqlx::query("UPDATE web_sessions SET account_id=$2 WHERE id_hash=$1")
            .bind(hash)
            .bind(invitee)
            .execute(&db.pool)
            .await
            .unwrap();
        if !verify_first {
            let bound: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=$1)",
            )
            .bind(invitee)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            assert!(!bound);
            sqlx::query("UPDATE accounts SET status='active',email_verified_at=clock_timestamp() WHERE id=$1").bind(invitee).execute(&db.pool).await.unwrap();
        }
        // No reconciliation worker runs before the real store purchase.
        db.buy(invitee, 1).await;
        let paid: Option<bool> = sqlx::query_scalar("SELECT first_paid_at IS NOT NULL FROM referral_campaign_invitations WHERE invitee_id=$1").bind(invitee).fetch_optional(&db.pool).await.unwrap();
        assert_eq!(paid, Some(true));
    }
    assert_eq!(db.count("outfit").await, 1);
    assert_eq!(db.count("starter").await, 2);
    let late = db.account().await;
    db.buy(late, 1).await;
    attribution_session(&db, Some(late), &code.to_string()).await;
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=$1)",
    )
    .bind(late)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(!bound);
    db.close().await;
}

#[tokio::test]
async fn nwave_session_end_is_retained_activity_and_backfilled() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let buddy = db.account().await;
    db.inactive(buddy, 13).await;
    let event = Uuid::now_v7();
    sqlx::query("INSERT INTO pd_game_logins(account_id,login_event_id,projection_epoch,recovery_generation,source_revision,entered_at) VALUES($1,$2,1,1,1,(extract(epoch FROM clock_timestamp()-interval '13 months')*1000)::bigint)").bind(buddy).bind(event).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE pd_game_logins SET left_at=(extract(epoch FROM clock_timestamp()-interval '1 day')*1000)::bigint,end_reason='LOGOUT' WHERE account_id=$1 AND login_event_id=$2").bind(buddy).bind(event).execute(&db.pool).await.unwrap();
    let recent: bool = sqlx::query_scalar("SELECT last_activity>clock_timestamp()-interval '2 days' FROM referral_activity WHERE account_id=$1").bind(buddy).fetch_one(&db.pool).await.unwrap();
    assert!(recent);
    // Re-run the migration's real backfill against the same ended session.
    // The migration backfill starts with an empty activity table.
    sqlx::query("DELETE FROM referral_activity")
        .execute(&db.pool)
        .await
        .unwrap();
    let migration = include_str!("../../../../migrations/20261111140004_n012_campaign.sql");
    let backfill = migration
        .split("INSERT INTO referral_activity(account_id,last_activity)")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "INSERT INTO referral_activity(account_id,last_activity){backfill};"
    )))
    .execute(&db.pool)
    .await
    .unwrap();
    let recent: bool = sqlx::query_scalar("SELECT last_activity>clock_timestamp()-interval '2 days' FROM referral_activity WHERE account_id=$1").bind(buddy).fetch_one(&db.pool).await.unwrap();
    assert!(recent);
    sqlx::query("DELETE FROM pd_game_logins WHERE account_id=$1")
        .bind(buddy)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.campaign
            .email(owner, &db.email(buddy).await, Kind::Returning)
            .await,
        Err(Error::Denied)
    );
    db.close().await;
}

#[tokio::test]
async fn nwave_loyalty_stream_tracks_credit_only_changes() {
    let Some(db) = Database::open().await else {
        return;
    };
    let account = db.account().await;
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES($1,$2,$2,'GRANT',2,$3,$4,1,172800,date_trunc('second',now())-interval '3 days',date_trunc('second',now())-interval '1 day')").bind(Uuid::now_v7()).bind(account).bind(vec![0_u8;32]).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let first = stream.read(None, 500).await.unwrap();
    for (key, points, expected, revision) in
        [("credit-1", 1, 3, 2), ("credit-2", i32::MAX, i32::MAX, 3)]
    {
        sqlx::query("INSERT INTO referral_loyalty_credits(operation_key,account_id,points) VALUES($1,$2,$3)").bind(key).bind(account).bind(points).execute(&db.pool).await.unwrap();
        let page = stream.read(Some(first.next), 500).await.unwrap();
        let item = page.items.last().unwrap();
        assert_eq!(
            (
                item.spent_premium_days,
                item.loyalty_points,
                item.platform_revision
            ),
            (2, expected, revision)
        );
        let snapshot =
            oteryn_store::loyalty::Ledger::new(db.pool.clone(), Arc::new(DatabasePolicy), true)
                .snapshot(account, &"a".repeat(32), &"b".repeat(40))
                .await
                .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&snapshot).unwrap()["loyalty_points"],
            item.loyalty_points
        );
        assert!(
            stream
                .read(Some(page.next), 500)
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    let credited_only = db.account().await;
    sqlx::query("INSERT INTO referral_loyalty_credits(operation_key,account_id,points) VALUES('credit-only',$1,1)")
        .bind(credited_only).execute(&db.pool).await.unwrap();
    let page = stream.read(None, 500).await.unwrap();
    let item = page
        .items
        .iter()
        .find(|i| i.account_id == credited_only)
        .unwrap();
    assert_eq!(
        (
            item.spent_premium_days,
            item.loyalty_points,
            item.platform_revision
        ),
        (0, 1, 1)
    );
    db.close().await;
}

#[tokio::test]
async fn nwave_stream_alone_awards_referral_consumption_credits() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(invitee)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES($1,$2,$2,'GRANT',6,$3,$4,1,518400,date_trunc('second',now())-interval '7 days',date_trunc('second',now())-interval '1 day')")
        .bind(Uuid::now_v7()).bind(invitee).bind(vec![0_u8;32]).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    // No snapshot endpoint or campaign worker is called.
    let page = stream.read(None, 500).await.unwrap();
    let item = page
        .items
        .iter()
        .find(|i| i.account_id == owner)
        .expect("inviter credit in the same poll");
    assert_eq!(
        (
            item.spent_premium_days,
            item.loyalty_points,
            item.platform_revision
        ),
        (0, 1, 1)
    );
    assert_eq!(
        page.items
            .iter()
            .find(|i| i.account_id == invitee)
            .unwrap()
            .loyalty_points,
        6
    );
    assert!(
        stream
            .read(Some(page.next), 500)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let credits: i64 = sqlx::query_scalar(
        "SELECT sum(points)::bigint FROM referral_loyalty_credits WHERE account_id=$1",
    )
    .bind(owner)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(credits, 1);
    db.close().await;
}

#[tokio::test]
async fn nwave_mail_backlog_respects_delivery_budget_across_workers() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    for day in 0..2 {
        for n in 0..25 {
            db.campaign
                .email(owner, &format!("backlog{day}-{n}@example.test"), Kind::New)
                .await
                .unwrap();
        }
        sqlx::query("UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '2 days' WHERE delivered_at IS NULL").execute(&db.pool).await.unwrap();
    }
    let sender = Arc::new(oteryn_notify::CaptureSender::default());
    let first = Worker {
        campaign: db.campaign.clone(),
        sender: Some(sender.clone()),
    };
    let second = Worker {
        campaign: db.campaign.clone(),
        sender: Some(sender.clone()),
    };
    let (a, b) = tokio::join!(first.process(), second.process());
    a.unwrap();
    b.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 25);
    first.process().await.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 25);
    let other = db.account().await;
    db.campaign
        .email(other, "other@example.test", Kind::New)
        .await
        .unwrap();
    first.process().await.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 26);
    sqlx::query("UPDATE referral_email_attempts SET delivered_at=clock_timestamp()-interval '24 hours 1 second' WHERE inviter_id=$1 AND delivered_at IS NOT NULL").bind(owner).execute(&db.pool).await.unwrap();
    first.process().await.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 51);
    db.close().await;
}

#[tokio::test]
async fn nwave_returning_mail_backlog_respects_monthly_delivery_budget() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    for batch in 0..2 {
        for _ in 0..20 {
            let buddy = db.account().await;
            db.inactive(buddy, 13).await;
            db.campaign
                .email(owner, &db.email(buddy).await, Kind::Returning)
                .await
                .unwrap();
        }
        if batch == 0 {
            sqlx::query("UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '31 days'").execute(&db.pool).await.unwrap();
        }
    }
    let sender = Arc::new(oteryn_notify::CaptureSender::default());
    let worker = Worker {
        campaign: db.campaign.clone(),
        sender: Some(sender.clone()),
    };
    worker.process().await.unwrap();
    worker.process().await.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 20);
    sqlx::query("UPDATE referral_email_attempts SET delivered_at=clock_timestamp()-interval '31 days' WHERE delivered_at IS NOT NULL").execute(&db.pool).await.unwrap();
    worker.process().await.unwrap();
    assert_eq!(sender.emails.lock().unwrap().len(), 40);
    db.close().await;
}

#[tokio::test]
async fn nwave_verification_binds_before_revoking_browser_sessions() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    sqlx::query(
        "UPDATE accounts SET status='pending_verification',email_verified_at=NULL WHERE id=$1",
    )
    .bind(invitee)
    .execute(&db.pool)
    .await
    .unwrap();
    attribution_session(&db, Some(invitee), &code.to_string()).await;
    sqlx::query(
        "UPDATE accounts SET status='active',email_verified_at=clock_timestamp() WHERE id=$1",
    )
    .bind(invitee)
    .execute(&db.pool)
    .await
    .unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(invitee)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        sessions, 0,
        "verification must still invalidate browser proofs"
    );
    let binding: Option<Uuid> = sqlx::query_scalar(
        "SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=$1",
    )
    .bind(invitee)
    .fetch_optional(&db.pool)
    .await
    .unwrap();
    assert_eq!(binding, Some(owner));
    assert_eq!(db.count("starter").await, 1);
    db.buy(invitee, 1).await;
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}

#[derive(Debug, Default)]
struct PausedMail {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl oteryn_notify::EmailSender for PausedMail {
    fn send<'a>(&'a self, _: &'a oteryn_notify::Email) -> oteryn_notify::NotifyFuture<'a, ()> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(())
        })
    }
}
#[tokio::test]
async fn nwave_smtp_does_not_block_purchase_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    db.fund(invitee).await;
    db.campaign
        .email(owner, "mail-delay@example.test", Kind::New)
        .await
        .unwrap();
    let sender = Arc::new(PausedMail::default());
    let worker = Worker {
        campaign: db.campaign.clone(),
        sender: Some(sender.clone()),
    };
    let sending = tokio::spawn(async move { worker.process().await });
    tokio::time::timeout(std::time::Duration::from_secs(5), sender.entered.notified())
        .await
        .unwrap();
    let purchases = db.purchases();
    let raw = db.request(invitee, 1, Uuid::now_v7());
    let purchase =
        tokio::time::timeout(std::time::Duration::from_secs(1), purchases.purchase(&raw)).await;
    sender.release.notify_one();
    sending.await.unwrap().unwrap();
    assert!(
        purchase.is_ok(),
        "SMTP held the referral qualification lock"
    );
    assert!(purchase.unwrap().is_ok());
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}

#[derive(Debug)]
struct ConcurrentPurchaseLock {
    pool: PgPool,
    later: Uuid,
    checked: std::sync::atomic::AtomicBool,
}
impl oteryn_store::premium::Accounts for ConcurrentPurchaseLock {
    fn state<'a>(
        &'a self,
        account: Uuid,
        conn: &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        oteryn_store::premium::AccountState,
                        oteryn_store::premium::Error,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            if account == self.later
                && !self.checked.swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                // Model a purchase that already owns the later account row and
                // now needs qualification, before the stream locks that row.
                let mut purchase = self.pool.begin().await.unwrap();
                sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                    .bind(account)
                    .execute(&mut *purchase)
                    .await
                    .unwrap();
                let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(5600)")
                    .fetch_one(&mut *purchase)
                    .await
                    .unwrap();
                purchase.rollback().await.unwrap();
                assert!(
                    available,
                    "stream retained 5600 while locking the next account"
                );
            }
            oteryn_store::premium::Accounts::state(&DatabasePolicy, account, conn).await
        })
    }
}
#[tokio::test]
async fn nwave_stream_releases_qualification_lock_before_next_account() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let first = db.recruit(owner).await;
    let later = db.recruit(owner).await;
    for account in [first, later] {
        sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
            .bind(account)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let accounts = Arc::new(ConcurrentPurchaseLock {
        pool: db.pool.clone(),
        later,
        checked: std::sync::atomic::AtomicBool::new(false),
    });
    // Streaming must work with one pool connection; qualification must not
    // request a second connection while retaining the publication transaction.
    let schema = db.schema.clone();
    let single_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |conn, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(schema)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let stream =
        oteryn_store::loyalty::stream::Stream::new(single_pool.clone(), accounts.clone(), true);
    let page = tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(None, 500))
        .await
        .expect("stream must not require a second pool connection")
        .unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(accounts.checked.load(std::sync::atomic::Ordering::SeqCst));
    single_pool.close().await;
    db.close().await;
}

// Wait for the competing transaction to reach a real PostgreSQL lock wait.
async fn wait_for_blocker(pool: &PgPool, blocker: i32) -> i32 {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting: Option<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) LIMIT 1",
            )
            .bind(blocker)
            .fetch_optional(pool)
            .await
            .unwrap();
            if let Some(pid) = waiting {
                break pid;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("competing transaction did not reach its lock wait")
}

#[tokio::test]
async fn nwave_first_link_locks_account_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let account = db.account().await;
    let mut purchase = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *purchase)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut *purchase)
        .await
        .unwrap();
    let campaign = db.campaign.clone();
    let link = tokio::spawn(async move { campaign.link(account).await });
    let waiting = wait_for_blocker(&db.pool, pid).await;
    let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)")
        .bind(waiting).fetch_one(&mut *purchase)
        .await
        .unwrap();
    purchase.commit().await.unwrap();
    link.await.unwrap().unwrap();
    assert!(
        available,
        "first link held 5600 while waiting for the purchase account lock"
    );
    db.close().await;
}

#[derive(Debug)]
struct PausedStreamAccount {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl oteryn_store::premium::Accounts for PausedStreamAccount {
    fn state<'a>(
        &'a self,
        account: Uuid,
        conn: &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        oteryn_store::premium::AccountState,
                        oteryn_store::premium::Error,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let state =
                oteryn_store::premium::Accounts::state(&DatabasePolicy, account, conn).await?;
            self.entered.notify_one();
            self.release.notified().await;
            Ok(state)
        })
    }
}

#[tokio::test]
async fn nwave_stream_creates_authority_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let account = db.account().await;
    sqlx::query("INSERT INTO referral_loyalty_credits(operation_key,account_id,points) VALUES('concurrent-credit',$1,1)")
        .bind(account).execute(&db.pool).await.unwrap();
    let accounts = Arc::new(PausedStreamAccount {
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), accounts.clone(), true);
    let reading = tokio::spawn(async move { stream.read(None, 500).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        accounts.entered.notified(),
    )
    .await
    .unwrap();
    // A snapshot creates its authority before calling referral_campaign_loyalty.
    let mut snapshot = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *snapshot)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(account)
        .execute(&mut *snapshot)
        .await
        .unwrap();
    accounts.release.notify_one();
    let waiting = wait_for_blocker(&db.pool, pid).await;
    let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)")
        .bind(waiting).fetch_one(&mut *snapshot)
        .await
        .unwrap();
    snapshot.commit().await.unwrap();
    let page = reading.await.unwrap().unwrap();
    assert_eq!(page.items[0].loyalty_points, 1);
    assert!(
        available,
        "stream held 5600 while awaiting a snapshot authority insertion"
    );
    db.close().await;
}

#[tokio::test]
async fn nwave_stream_locks_existing_authority_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let account = db.account().await;
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut snapshot = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *snapshot)
        .await
        .unwrap();
    sqlx::query("SELECT account_id FROM premium_authority WHERE account_id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut *snapshot)
        .await
        .unwrap();
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let reading = tokio::spawn(async move { stream.read(None, 500).await });
    let waiting = wait_for_blocker(&db.pool, pid).await;
    let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)").bind(waiting).fetch_one(&mut *snapshot).await.unwrap();
    snapshot.commit().await.unwrap();
    reading.await.unwrap().unwrap();
    assert!(
        available,
        "stream held 5600 while awaiting an existing authority row"
    );
    db.close().await;
}

#[tokio::test]
async fn nwave_accept_locks_inviter_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    let mut purchase = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *purchase)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .execute(&mut *purchase)
        .await
        .unwrap();
    let campaign = db.campaign.clone();
    let accepting = tokio::spawn(async move { campaign.accept(invitee, code).await });
    let waiting = wait_for_blocker(&db.pool, pid).await;
    let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)").bind(waiting).fetch_one(&mut *purchase).await.unwrap();
    purchase.commit().await.unwrap();
    accepting.await.unwrap().unwrap();
    assert!(
        available,
        "accept held 5600 while awaiting the inviter account"
    );
    db.close().await;
}

#[tokio::test]
async fn nwave_snapshot_locks_inviter_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(invitee)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES($1,$2,$2,'GRANT',6,$3,$4,1,518400,date_trunc('second',now())-interval '7 days',date_trunc('second',now())-interval '1 day')").bind(Uuid::now_v7()).bind(invitee).bind(vec![0u8;32]).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    let mut purchase = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *purchase)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .execute(&mut *purchase)
        .await
        .unwrap();
    let ledger =
        oteryn_store::loyalty::Ledger::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let reading = tokio::spawn(async move {
        ledger
            .snapshot(
                invitee,
                "4f1c2a9be07d4c3a8b6e5d4c3b2a1908",
                "0123456789abcdef0123456789abcdef01234567",
            )
            .await
    });
    let waiting = wait_for_blocker(&db.pool, pid).await;
    let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)").bind(waiting).fetch_one(&mut *purchase).await.unwrap();
    purchase.commit().await.unwrap();
    reading.await.unwrap().unwrap();
    assert!(
        available,
        "snapshot held 5600 while awaiting the inviter account"
    );
    let points: i64 = sqlx::query_scalar(
        "SELECT sum(points)::bigint FROM referral_loyalty_credits WHERE account_id=$1",
    )
    .bind(owner)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(points, 1);
    db.close().await;
}

#[tokio::test]
async fn nwave_signup_and_email_lock_accounts_before_qualification() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    sqlx::query(
        "UPDATE accounts SET status='pending_verification',email_verified_at=NULL WHERE id=$1",
    )
    .bind(invitee)
    .execute(&db.pool)
    .await
    .unwrap();
    attribution_session(&db, Some(invitee), &code.to_string()).await;
    for signup in [true, false] {
        let mut purchase = db.pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *purchase)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(owner)
            .execute(&mut *purchase)
            .await
            .unwrap();
        let campaign = db.campaign.clone();
        let operation = tokio::spawn(async move {
            if signup {
                sqlx::query("UPDATE accounts SET status='active',email_verified_at=clock_timestamp() WHERE id=$1")
                    .bind(invitee).execute(&campaign.pool).await.unwrap();
                let bound: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=$1)")
                    .bind(invitee).fetch_one(&campaign.pool).await.unwrap();
                assert!(bound);
            } else {
                campaign
                    .email(owner, "new@example.invalid", Kind::New)
                    .await
                    .unwrap();
            }
        });
        let waiting = wait_for_blocker(&db.pool, pid).await;
        let available: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND classid=0 AND objid=5600 AND objsubid=1 AND granted)").bind(waiting).fetch_one(&mut *purchase).await.unwrap();
        purchase.commit().await.unwrap();
        operation.await.unwrap();
        assert!(
            available,
            "signup/email held 5600 while awaiting an account"
        );
    }
    db.close().await;
}

#[tokio::test]
async fn nwave_signup_expanded_preparation_preserves_session_for_worker() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    let hash = attribution_session(&db, None, &code.to_string()).await;
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query("SELECT referral_campaign_lock(ARRAY[$1::uuid])")
        .bind(invitee)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE web_sessions SET account_id=$2 WHERE id_hash=$1")
        .bind(&hash)
        .bind(invitee)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let evidence: String =
        sqlx::query_scalar("SELECT data->>'referral.code' FROM web_sessions WHERE id_hash=$1")
            .bind(&hash)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(evidence, code.to_string());
    Worker {
        campaign: db.campaign.clone(),
        sender: None,
    }
    .process()
    .await
    .unwrap();
    let inviter: Uuid = sqlx::query_scalar(
        "SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=$1",
    )
    .bind(invitee)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(inviter, owner);
    assert_eq!(db.count("starter").await, 1);
    db.close().await;
}

#[tokio::test]
async fn nwave_purchase_retries_entire_transaction_and_replays_original_answer() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    db.fund(invitee).await;
    // A nontransactional sequence makes only the first qualification fail,
    // after debit and order creation, just like stale inviter discovery.
    sqlx::raw_sql("CREATE SEQUENCE retry_qualification; ALTER FUNCTION referral_campaign_purchase(uuid,integer,timestamptz) RENAME TO original_campaign_purchase;
        CREATE FUNCTION referral_campaign_purchase(a uuid,days integer,paid_at timestamptz) RETURNS void LANGUAGE plpgsql AS $$
        BEGIN
            IF nextval('retry_qualification')=1 THEN
                RAISE EXCEPTION 'simulated expanded inviter set' USING ERRCODE='40001';
            END IF;
            PERFORM original_campaign_purchase(a,days,paid_at);
        END $$;").execute(&db.pool).await.unwrap();
    let raw = db.request(invitee, 1, Uuid::now_v7());
    let purchases = db.purchases();
    let answer = purchases.purchase(&raw).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&answer).unwrap()["result"],
        "ACCEPTED"
    );
    assert_eq!(purchases.purchase(&raw).await.unwrap(), answer);
    let orders: i64 = sqlx::query_scalar("SELECT count(*) FROM store_orders WHERE account_id=$1")
        .bind(invitee)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(orders, 1);
    assert_eq!(
        db.wallet().balance(invitee).await.unwrap().transferable,
        9750
    );
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}

#[tokio::test]
async fn nwave_transaction_retry_is_bounded_and_only_for_retryable_sqlstates() {
    let Some(db) = Database::open().await else {
        return;
    };
    sqlx::raw_sql("CREATE SEQUENCE retry_attempts; CREATE TABLE retry_effects(id integer)")
        .execute(&db.pool)
        .await
        .unwrap();
    for (state, attempts) in [("40001", 3_i64), ("40P01", 3), ("23505", 1), ("55P03", 1)] {
        sqlx::query("ALTER SEQUENCE retry_attempts RESTART")
            .execute(&db.pool)
            .await
            .unwrap();
        let result: Result<(), Error> = oteryn_db::retry::transaction(|| async {
            let mut tx = db.pool.begin().await?;
            sqlx::query("SELECT nextval('retry_attempts')")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO retry_effects VALUES(1)")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "DO $$ BEGIN RAISE EXCEPTION 'test retry' USING ERRCODE='{state}'; END $$"
            )))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(())
        })
        .await;
        assert_eq!(result, Err(Error::Unavailable));
        let actual: i64 = sqlx::query_scalar("SELECT last_value FROM retry_attempts")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(actual, attempts, "SQLSTATE {state}");
        let effects: i64 = sqlx::query_scalar("SELECT count(*) FROM retry_effects")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(effects, 0, "failed transactions must roll back");
    }
    db.close().await;
}

#[tokio::test]
async fn nwave_returning_snapshot_and_stream_prepare_all_fk_targets() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let buddy = db.account().await;
    db.inactive(buddy, 13).await;
    db.campaign
        .email(owner, &db.email(buddy).await, Kind::Returning)
        .await
        .unwrap();
    let snapshot =
        oteryn_store::loyalty::Ledger::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    snapshot
        .snapshot(buddy, &"a".repeat(32), &"b".repeat(40))
        .await
        .unwrap();
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let page = stream.read(None, 500).await.unwrap();
    assert!(page.items.iter().any(|item| item.account_id == buddy));
    assert_eq!(page.items.len(), 1);
    let owner_authority: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM premium_authority WHERE account_id=$1)")
            .bind(owner)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(!owner_authority);
    assert_eq!(db.count("outfit").await, 0);
    db.close().await;
}

#[tokio::test]
async fn nwave_snapshot_stream_campaign_and_worker_retry_fresh_transactions() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    let pending = db.account().await;
    // Leave evidence for the worker rather than synchronously binding it.
    sqlx::query("UPDATE referral_campaign_settings SET enabled=false")
        .execute(&db.pool)
        .await
        .unwrap();
    attribution_session(&db, Some(pending), &code.to_string()).await;
    sqlx::query("UPDATE referral_campaign_settings SET enabled=true")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE SEQUENCE retry_lock; ALTER FUNCTION referral_campaign_lock(uuid[]) RENAME TO original_campaign_lock;
        CREATE FUNCTION referral_campaign_lock(ids uuid[]) RETURNS void LANGUAGE plpgsql AS $$
        BEGIN
            IF nextval('retry_lock')=1 THEN
                RAISE EXCEPTION 'test lock discovery failure' USING ERRCODE='40P01';
            END IF;
            PERFORM original_campaign_lock(ids);
        END $$;").execute(&db.pool).await.unwrap();
    let snapshot =
        oteryn_store::loyalty::Ledger::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let bytes = snapshot
        .snapshot(invitee, &"a".repeat(32), &"b".repeat(40))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["loyalty_revision"],
        1
    );
    assert_retry_lock_restarted(&db).await;
    let stream =
        oteryn_store::loyalty::stream::Stream::new(db.pool.clone(), Arc::new(DatabasePolicy), true);
    let page = stream.read(None, 500).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].platform_revision, 1);
    assert_retry_lock_restarted(&db).await;
    assert_eq!(db.campaign.link(owner).await.unwrap(), code);
    assert_retry_lock_restarted(&db).await;
    db.campaign
        .email(owner, "retry@example.test", Kind::New)
        .await
        .unwrap();
    assert_retry_lock_restarted(&db).await;
    let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM referral_email_attempts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(attempts, 1);
    db.campaign.accept(invitee, code).await.unwrap();
    assert_retry_lock_restarted(&db).await;
    assert_eq!(db.campaign.invitation_kind(code).await.unwrap(), Kind::New);
    assert_retry_lock_restarted(&db).await;
    Worker {
        campaign: db.campaign.clone(),
        sender: None,
    }
    .process()
    .await
    .unwrap();
    assert_retry_lock_restarted(&db).await;
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=$1)",
    )
    .bind(pending)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(bound);
    assert_eq!(db.count("starter").await, 2);
    db.close().await;
}

async fn assert_retry_lock_restarted(db: &Database) {
    let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM retry_lock")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(attempts >= 2, "operation did not retry its transaction");
    sqlx::query("ALTER SEQUENCE retry_lock RESTART")
        .execute(&db.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn nwave_policy_read_failure_preserves_worker_attribution() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let code = db.campaign.link(owner).await.unwrap();
    let invitee = db.account().await;
    sqlx::query("UPDATE referral_campaign_settings SET enabled=false")
        .execute(&db.pool)
        .await
        .unwrap();
    let hash = attribution_session(&db, Some(invitee), &code.to_string()).await;
    sqlx::query("UPDATE referral_campaign_settings SET enabled=true")
        .execute(&db.pool)
        .await
        .unwrap();
    // Fail only the eligibility read, after discovery and worker candidate selection.
    sqlx::raw_sql(
        "ALTER TABLE accounts RENAME TO policy_accounts;
        CREATE FUNCTION policy_read_fault() RETURNS boolean LANGUAGE plpgsql AS $$ BEGIN
        IF current_query() LIKE 'SELECT count(*) FROM accounts%' THEN
            RAISE EXCEPTION 'eligibility read temporarily unavailable' USING ERRCODE='57014';
        END IF; RETURN true; END $$;
        CREATE VIEW accounts AS SELECT * FROM policy_accounts WHERE policy_read_fault();",
    )
    .execute(&db.pool)
    .await
    .unwrap();
    let worker = Worker {
        campaign: db.campaign.clone(),
        sender: None,
    };
    assert_eq!(worker.process().await, Err(Error::Unavailable));
    let evidence: Option<String> =
        sqlx::query_scalar("SELECT data->>'referral.code' FROM web_sessions WHERE id_hash=$1")
            .bind(&hash)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(evidence, Some(code.to_string()));
    sqlx::raw_sql("DROP VIEW accounts; ALTER TABLE policy_accounts RENAME TO accounts;")
        .execute(&db.pool)
        .await
        .unwrap();
    worker.process().await.unwrap();
    db.buy(invitee, 1).await;
    assert_eq!(db.count("outfit").await, 1);
    db.close().await;
}

#[tokio::test]
async fn nwave_legacy_invite_checks_campaign_graph() {
    let Some(db) = Database::open().await else {
        return;
    };
    let owner = db.account().await;
    let invitee = db.recruit(owner).await;
    let third = db.account().await;
    let reward_actor = db.account().await;
    let service = super::super::Referrals::new(
        db.pool.clone(),
        Flags { enabled: true },
        DatabasePolicy,
        db.wallet(),
        oteryn_notify::Notifications {
            pool: db.pool.clone(),
            accounts: Arc::new(Checks),
            unsubscribe: oteryn_notify::Unsubscribe::new(
                vec![7; 32],
                "https://portal.example.test/unsubscribe".into(),
            )
            .unwrap(),
        },
        super::super::Reward {
            actor: reward_actor,
            amount: 25,
        },
    );
    for (inviter, target) in [(invitee, owner), (third, invitee)] {
        let request = super::super::Invitation {
            id: Uuid::now_v7(),
            inviter,
            invitee: target,
            kind: Kind::New,
            expires_at: time::OffsetDateTime::now_utc() + time::Duration::days(7),
        };
        assert_eq!(service.invite(&request).await, Err(Error::Denied));
    }
    db.close().await;
}

#[tokio::test]
async fn nwave_preparation_prevents_account_and_authority_lock_upgrades_after_5600() {
    let Some(db) = Database::open().await else {
        return;
    };
    let account = db.account().await;
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES($1)")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut prepared = db.pool.begin().await.unwrap();
    sqlx::query("SELECT referral_campaign_lock(ARRAY[$1::uuid])")
        .bind(account)
        .execute(&mut *prepared)
        .await
        .unwrap();
    for query in [
        "SELECT id FROM accounts WHERE id=$1 FOR KEY SHARE NOWAIT",
        "SELECT account_id FROM premium_authority WHERE account_id=$1 FOR KEY SHARE NOWAIT",
    ] {
        let mut contender = db.pool.begin().await.unwrap();
        let error = sqlx::query(query)
            .bind(account)
            .execute(&mut *contender)
            .await
            .expect_err("preparation must take strongest row locks before 5600");
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("55P03")
        );
        contender.rollback().await.unwrap();
    }
    prepared.rollback().await.unwrap();
    db.close().await;
}
