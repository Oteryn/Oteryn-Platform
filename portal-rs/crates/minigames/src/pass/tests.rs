use super::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::Duration;
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

// Synthetic source authority proves test facts only; never mounted in production.
#[derive(Debug)]
struct Trusted;
impl Authority for Trusted {
    async fn account(&self, tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<()> {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active')",
        )
        .bind(account)
        .fetch_one(&mut **tx)
        .await?;
        if active { Ok(()) } else { Err(Error::Denied) }
    }
    async fn publish(&self, _: &mut Transaction<'_, Postgres>, _: Uuid) -> Result<()> {
        Ok(())
    }
    async fn event(&self, _: &mut Transaction<'_, Postgres>, event: &Event) -> Result<()> {
        if event.metric == "synthetic" {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
#[derive(Debug)]
struct Wire;
impl Rewards for Wire {
    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        operation: Uuid,
        reward: &Reward,
    ) -> Result<Uuid> {
        sqlx::query("INSERT INTO synthetic_deliveries (id,account_id,grant_key) VALUES ($1,$2,$3)")
            .bind(operation)
            .bind(account)
            .bind(&reward.grant_key)
            .execute(&mut **tx)
            .await?;
        Ok(operation)
    }
}
#[derive(Debug)]
struct Coins;
impl Payment for Coins {
    async fn debit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        operation: Uuid,
        amount: i64,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE synthetic_wallet SET balance=balance-$2 WHERE account_id=$1 AND balance>=$2",
        )
        .bind(account)
        .bind(amount)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO synthetic_debits VALUES ($1)")
            .bind(operation)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
fn season(now: OffsetDateTime) -> Season {
    Season {
        id: Uuid::now_v7(),
        starts_at: now + Duration::hours(1),
        ends_at: now + Duration::hours(1) + Duration::days(42),
        price: 100,
        challenges: vec![
            Challenge {
                id: Uuid::now_v7(),
                source: Source::Journal,
                metric: "synthetic".into(),
                target: 2,
                xp: 10,
            },
            Challenge {
                id: Uuid::now_v7(),
                source: Source::Portal,
                metric: "synthetic".into(),
                target: 1,
                xp: 5,
            },
        ],
        rewards: vec![
            Reward {
                id: Uuid::now_v7(),
                paid: false,
                xp: 10,
                grant_key: "synthetic.frame".into(),
                grant_revision: "1".into(),
            },
            Reward {
                id: Uuid::now_v7(),
                paid: true,
                xp: 15,
                grant_key: "synthetic.title".into(),
                grant_revision: "1".into(),
            },
        ],
    }
}
struct Db {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    account: Uuid,
    season: Season,
}
impl Db {
    async fn new() -> std::result::Result<Option<Self>, Box<dyn std::error::Error>> {
        let url = match std::env::var("PORTAL_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => {
                eprintln!("SKIPPED PostgreSQL season pass test: PORTAL_TEST_DATABASE_URL is unset");
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("pass_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
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
            .await?;
        oteryn_db::migrate(&pool).await?;
        sqlx::raw_sql("CREATE TABLE synthetic_wallet(account_id uuid PRIMARY KEY,balance bigint NOT NULL CHECK(balance>=0)); CREATE TABLE synthetic_debits(id uuid PRIMARY KEY); CREATE TABLE synthetic_deliveries(id uuid PRIMARY KEY,account_id uuid,grant_key text);").execute(&pool).await?;
        let account = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'pass@example.invalid','unused','active',now(),'en')").bind(account).execute(&pool).await?;
        sqlx::query("INSERT INTO synthetic_wallet VALUES($1,200)")
            .bind(account)
            .execute(&pool)
            .await?;
        let now = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&pool)
            .await?;
        let season = season(now);
        let db = Self {
            admin,
            pool,
            schema,
            account,
            season,
        };
        db.service().publish(db.account, &db.season).await?;
        sqlx::query("UPDATE pass_seasons SET starts_at=starts_at-interval '2 hours',ends_at=ends_at-interval '2 hours'").execute(&db.pool).await?;
        Ok(Some(db))
    }
    fn service(&self) -> Passes<Trusted, Coins, Wire> {
        Passes::new(
            self.pool.clone(),
            Flags {
                enabled: true,
                paid: true,
                rewards: true,
            },
            Trusted,
            Coins,
            Wire,
        )
    }
    fn event(&self, index: usize) -> Event {
        let c = &self.season.challenges[index];
        Event {
            id: Uuid::now_v7(),
            account: self.account,
            season: self.season.id,
            challenge: c.id,
            source: c.source,
            metric: c.metric.clone(),
            amount: c.target,
            occurred_at: OffsetDateTime::now_utc() - Duration::seconds(1),
        }
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
#[test]
fn definitions_reject_invalid_duration_unknown_keys_and_overflow() {
    let mut s = season(OffsetDateTime::now_utc());
    assert!(s.validate().is_ok());
    s.ends_at += Duration::days(15);
    assert!(matches!(s.validate(), Err(Error::Invalid)));
    s.ends_at -= Duration::days(15);
    s.rewards[0].grant_key = "../bad".into();
    assert!(s.validate().is_err());
    s.rewards[0].grant_key = "frame".into();
    s.challenges[0].xp = i64::MAX;
    assert!(s.validate().is_err());
    assert!(!valid_id(Uuid::nil()));
    assert!(key("1.frame"));
    assert!(wire_revision("18446744073709551615"));
    for invalid in ["v1", "0", "01", "18446744073709551616"] {
        assert!(!wire_revision(invalid));
    }
}
#[tokio::test]
async fn postgres_verified_sources_replay_and_payload_conflict() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let event = db.event(0);
    assert_eq!(service.record(&event).await?.xp, 10);
    assert_eq!(service.record(&event).await?.xp, 10);
    let mut altered = event.clone();
    altered.amount = 1;
    assert!(matches!(
        service.record(&altered).await,
        Err(Error::Conflict)
    ));
    assert_eq!(service.record(&db.event(1)).await?.xp, 15);
    assert_eq!(service.record(&db.event(0)).await?.xp, 15);
    db.close().await
}
#[tokio::test]
async fn postgres_bad_input_unknown_authority_and_time_deny() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let mut event = db.event(0);
    event.amount = 0;
    assert!(matches!(service.record(&event).await, Err(Error::Invalid)));
    event = db.event(0);
    event.source = Source::Portal;
    assert!(matches!(service.record(&event).await, Err(Error::Denied)));
    event = db.event(0);
    event.occurred_at += Duration::days(1);
    assert!(matches!(service.record(&event).await, Err(Error::Denied)));
    event = db.event(0);
    event.occurred_at -= Duration::days(2);
    assert!(matches!(service.record(&event).await, Err(Error::Denied)));
    let unavailable = Passes::new(
        db.pool.clone(),
        Flags {
            enabled: true,
            paid: true,
            rewards: true,
        },
        Unavailable,
        Coins,
        Wire,
    );
    assert!(matches!(
        unavailable.record(&db.event(0)).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(db.account)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        service.progress(db.account, db.season.id).await,
        Err(Error::Denied)
    ));
    db.close().await
}
#[tokio::test]
async fn postgres_purchase_concurrency_and_insufficient_funds() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let operation = Uuid::now_v7();
    let (a, b) = tokio::join!(
        service.buy(db.account, db.season.id, operation),
        service.buy(db.account, db.season.id, operation)
    );
    assert!(a?.paid && b?.paid);
    let balance: i64 = sqlx::query_scalar("SELECT balance FROM synthetic_wallet")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(balance, 100);
    assert!(matches!(
        service.buy(db.account, db.season.id, Uuid::now_v7()).await,
        Err(Error::Conflict)
    ));
    sqlx::query("UPDATE pass_progress SET paid=false,purchase_key=NULL")
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE synthetic_wallet SET balance=0")
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        service.buy(db.account, db.season.id, Uuid::now_v7()).await,
        Err(Error::Denied)
    ));
    assert!(!service.progress(db.account, db.season.id).await?.paid);
    db.close().await
}
#[tokio::test]
async fn postgres_claim_eligibility_replay_and_atomic_wire_failure() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let reward = db.season.rewards[0].id;
    let op = Uuid::now_v7();
    assert!(matches!(
        service.claim(db.account, db.season.id, reward, op).await,
        Err(Error::Denied)
    ));
    service.record(&db.event(0)).await?;
    let blocked = Passes::new(
        db.pool.clone(),
        Flags {
            enabled: true,
            paid: true,
            rewards: true,
        },
        Trusted,
        Coins,
        Unavailable,
    );
    assert!(matches!(
        blocked.claim(db.account, db.season.id, reward, op).await,
        Err(Error::Unavailable)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pass_claims")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let (a, b) = tokio::join!(
        service.claim(db.account, db.season.id, reward, op),
        service.claim(db.account, db.season.id, reward, op)
    );
    assert_eq!(a?, b?);
    assert!(matches!(
        service
            .claim(db.account, db.season.id, reward, Uuid::now_v7())
            .await,
        Err(Error::Conflict)
    ));
    service.record(&db.event(1)).await?;
    assert!(matches!(
        service
            .claim(
                db.account,
                db.season.id,
                db.season.rewards[1].id,
                Uuid::now_v7()
            )
            .await,
        Err(Error::Denied)
    ));
    service
        .buy(db.account, db.season.id, Uuid::now_v7())
        .await?;
    service
        .claim(
            db.account,
            db.season.id,
            db.season.rewards[1].id,
            Uuid::now_v7(),
        )
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM synthetic_deliveries")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 2);
    db.close().await
}
#[tokio::test]
async fn postgres_disabled_flags_and_ended_season_deny() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let disabled = Passes::new(db.pool.clone(), Flags::default(), Trusted, Coins, Wire);
    assert!(matches!(
        disabled.record(&db.event(0)).await,
        Err(Error::Disabled)
    ));
    let free = Passes::new(
        db.pool.clone(),
        Flags {
            enabled: true,
            ..Flags::default()
        },
        Trusted,
        Coins,
        Wire,
    );
    assert!(matches!(
        free.buy(db.account, db.season.id, Uuid::now_v7()).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        free.claim(
            db.account,
            db.season.id,
            db.season.rewards[0].id,
            Uuid::now_v7()
        )
        .await,
        Err(Error::Disabled)
    ));
    sqlx::query("UPDATE pass_seasons SET starts_at=starts_at-interval '60 days',ends_at=ends_at-interval '60 days'").execute(&db.pool).await?;
    assert!(matches!(
        db.service().record(&db.event(0)).await,
        Err(Error::Denied)
    ));
    db.close().await
}

#[derive(Debug, Clone, Copy)]
struct WalletAuthority;
impl oteryn_wallet::AccountStateReader for WalletAuthority {
    async fn state(&self, _: &mut sqlx::PgConnection, _: Uuid) -> oteryn_wallet::AccountState {
        oteryn_wallet::AccountState::Active
    }
}
impl oteryn_wallet::TransferBlockCheck for WalletAuthority {
    async fn permission(
        &self,
        _: &mut sqlx::PgConnection,
        _: Uuid,
    ) -> oteryn_wallet::TransferPermission {
        oteryn_wallet::TransferPermission::Allowed
    }
}
impl oteryn_wallet::AuditHook for WalletAuthority {
    async fn adjustment(
        &self,
        _: &mut sqlx::PgConnection,
        _: &oteryn_wallet::AdjustmentAudit,
    ) -> bool {
        true
    }
}
#[tokio::test]
async fn postgres_real_wallet_debit_replay_and_disabled_wallet() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let ledger = oteryn_wallet::Ledger::new(
        db.pool.clone(),
        oteryn_wallet::WalletFlags { enabled: true },
        WalletAuthority,
        WalletAuthority,
        WalletAuthority,
    );
    ledger
        .adjust(
            Uuid::now_v7(),
            db.account,
            200,
            db.account,
            "synthetic test credit",
        )
        .await?;
    let payment = Wallet::new(ledger.clone());
    let service = Passes::new(
        db.pool.clone(),
        Flags {
            enabled: true,
            paid: true,
            rewards: true,
        },
        Trusted,
        payment,
        Wire,
    );
    let operation = Uuid::now_v7();
    service.buy(db.account, db.season.id, operation).await?;
    service.buy(db.account, db.season.id, operation).await?;
    assert_eq!(ledger.balance(db.account).await?.available, 100);
    sqlx::query("UPDATE pass_progress SET paid=false,purchase_key=NULL")
        .execute(&db.pool)
        .await?;
    let disabled = oteryn_wallet::Ledger::new(
        db.pool.clone(),
        oteryn_wallet::WalletFlags::default(),
        WalletAuthority,
        WalletAuthority,
        WalletAuthority,
    );
    let blocked = Passes::new(
        db.pool.clone(),
        Flags {
            enabled: true,
            paid: true,
            rewards: true,
        },
        Trusted,
        Wallet::new(disabled),
        Wire,
    );
    assert!(matches!(
        blocked.buy(db.account, db.season.id, Uuid::now_v7()).await,
        Err(Error::Denied)
    ));
    assert!(!blocked.progress(db.account, db.season.id).await?.paid);
    db.close().await
}

#[tokio::test]
async fn postgres_browser_session_csrf_purchase_and_claim() -> TestResult {
    use axum::{
        Extension, Router,
        body::{Body, to_bytes},
        extract::State,
        http::{Request, StatusCode, header},
        routing::get,
    };
    use oteryn_web::{
        csrf::CsrfConfig,
        session::{self, BrowserSession, PrivilegeChange, SessionLayer},
    };
    use tower::ServiceExt;
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    async fn login(
        State(account): State<Uuid>,
        Extension(session): Extension<BrowserSession>,
    ) -> std::result::Result<String, StatusCode> {
        session::rotate(&session, PrivilegeChange::Login(account))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
    let login = Router::new()
        .route("/test-login", get(login))
        .layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            session::middleware,
        ))
        .with_state(db.account);
    let response = login
        .oneshot(Request::get("/test-login").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing session cookie")?
        .to_owned();
    let csrf = String::from_utf8(to_bytes(response.into_body(), 4096).await?.to_vec())?;
    let router = web::router(
        db.pool.clone(),
        db.service(),
        Flags {
            enabled: true,
            paid: true,
            rewards: true,
        },
        CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..CsrfConfig::default()
        },
    );
    let response = router
        .clone()
        .oneshot(
            Request::get("/en/pass")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let html = String::from_utf8(to_bytes(response.into_body(), 64000).await?.to_vec())?;
    assert!(html.contains("Game journal challenge"));
    assert!(html.contains("Portal challenge"));
    assert!(html.contains("Buy paid track"));
    let operation = Uuid::now_v7();
    let uri = format!("/en/pass/{}/buy", db.season.id);
    let response = router
        .clone()
        .oneshot(
            Request::post(&uri)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("operation={operation}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!db.service().progress(db.account, db.season.id).await?.paid);
    let response = router
        .clone()
        .oneshot(
            Request::post(&uri)
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&operation={operation}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    db.service().record(&db.event(0)).await?;
    let response = router
        .clone()
        .oneshot(
            Request::post(format!("/en/pass/{}/claim", db.season.id))
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&operation={}&reward={}",
                    Uuid::now_v7(),
                    db.season.rewards[0].id
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let response = router
        .oneshot(
            Request::post(&uri)
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&operation=bad")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    db.close().await
}
