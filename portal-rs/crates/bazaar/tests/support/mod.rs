use oteryn_bazaar::*;
use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgConnection, PgPool, postgres::PgPoolOptions};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Default)]
pub struct TestAccounts(pub Arc<RwLock<BTreeMap<Uuid, AccountState>>>);
impl Accounts for TestAccounts {
    async fn state(
        &self,
        _: &mut PgConnection,
        account: Uuid,
    ) -> Result<Option<AccountState>, Error> {
        Ok(self
            .0
            .read()
            .map_err(|_| Error::Denied)?
            .get(&account)
            .copied())
    }
}
#[derive(Debug, Clone)]
pub struct TestEvidence {
    pub escrow: Arc<AtomicBool>,
    pub delivery: Arc<AtomicBool>,
}
impl Default for TestEvidence {
    fn default() -> Self {
        Self {
            escrow: Arc::new(AtomicBool::new(true)),
            delivery: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl Evidence for TestEvidence {
    async fn escrow_ready(&self, _: &mut PgConnection, _: &Listing) -> Result<bool, Error> {
        Ok(self.escrow.load(Ordering::SeqCst))
    }
    async fn offer_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
        _: &Offer,
    ) -> Result<bool, Error> {
        Ok(self.escrow.load(Ordering::SeqCst))
    }
    async fn delivered(&self, _: &mut PgConnection, _: &Settlement) -> Result<bool, Error> {
        Ok(self.delivery.load(Ordering::SeqCst))
    }
}

/// In-memory test ledger. The SQL port below snapshots it inside the caller's transaction so
/// rejected actions, failed statements and transaction rollback cannot leak memory side effects.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryLedger {
    pub balances: BTreeMap<String, i64>,
    pub holds: BTreeMap<String, (String, i64)>,
    pub pending: BTreeMap<String, (String, i64)>,
    pub commission: i64,
}
impl MemoryLedger {
    pub fn key(account: Uuid, currency: Currency) -> String {
        format!("{account}:{currency:?}")
    }
    fn hold_key(id: Uuid, account: Uuid) -> String {
        format!("{id}:{account}")
    }
    pub fn reserve(&mut self, h: Reservation) -> Result<(), Error> {
        let key = Self::key(h.account, h.currency);
        let hold_key = Self::hold_key(h.id, h.account);
        let other: i64 = self
            .holds
            .iter()
            .filter(|(id, (a, _))| **id != hold_key && *a == key)
            .map(|(_, (_, n))| n)
            .sum();
        if h.amount <= 0 || self.balances.get(&key).copied().unwrap_or(0) - other < h.amount {
            return Err(Error::InsufficientFunds);
        }
        self.holds.insert(hold_key, (key, h.amount));
        Ok(())
    }
    pub fn release(&mut self, id: Uuid, account: Uuid) {
        self.holds.remove(&Self::hold_key(id, account));
    }
    pub fn settle(&mut self, s: &Settlement) -> Result<(), Error> {
        if self.pending.contains_key(&s.id.to_string()) {
            return Ok(());
        }
        let key = Self::hold_key(s.reservation, s.buyer);
        let (account, maximum) = self.holds.get(&key).ok_or(Error::InsufficientFunds)?;
        if *maximum < s.amount {
            return Err(Error::InsufficientFunds);
        }
        let balance = self
            .balances
            .get_mut(account)
            .ok_or(Error::InsufficientFunds)?;
        *balance = balance
            .checked_sub(s.amount)
            .filter(|n| *n >= 0)
            .ok_or(Error::InsufficientFunds)?;
        self.holds.remove(&key);
        self.commission = self
            .commission
            .checked_add(s.commission)
            .ok_or(Error::Overflow)?;
        self.pending.insert(
            s.id.to_string(),
            (Self::key(s.seller, s.currency), s.seller_net),
        );
        Ok(())
    }
    pub fn complete(&mut self, s: &Settlement) -> Result<(), Error> {
        if let Some((account, amount)) = self.pending.remove(&s.id.to_string()) {
            let balance = self.balances.entry(account).or_default();
            *balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryReservations {
    pub fail_settlement: Arc<AtomicBool>,
}
impl MemoryReservations {
    async fn load(db: &mut PgConnection) -> Result<MemoryLedger, Error> {
        let data: String =
            sqlx::query_scalar("SELECT snapshot FROM test_ledger WHERE id=1 FOR UPDATE")
                .fetch_one(db)
                .await?;
        serde_json::from_str(&data).map_err(|_| Error::InvalidInput)
    }
    async fn save(db: &mut PgConnection, ledger: MemoryLedger) -> Result<(), Error> {
        let data = serde_json::to_string(&ledger).map_err(|_| Error::InvalidInput)?;
        sqlx::query("UPDATE test_ledger SET snapshot=$1 WHERE id=1")
            .bind(data)
            .execute(db)
            .await?;
        Ok(())
    }
}
impl Reservations for MemoryReservations {
    async fn lock_accounts(&self, db: &mut PgConnection, _: &[Uuid]) -> Result<(), Error> {
        Self::load(db).await?;
        Ok(())
    }
    async fn reserve(&self, db: &mut PgConnection, h: Reservation) -> Result<(), Error> {
        let mut ledger = Self::load(db).await?;
        ledger.reserve(h)?;
        Self::save(db, ledger).await
    }
    async fn release(&self, db: &mut PgConnection, id: Uuid, account: Uuid) -> Result<(), Error> {
        let mut ledger = Self::load(db).await?;
        ledger.release(id, account);
        Self::save(db, ledger).await
    }
    async fn settle(&self, db: &mut PgConnection, s: &Settlement) -> Result<(), Error> {
        let mut ledger = Self::load(db).await?;
        ledger.settle(s)?;
        Self::save(db, ledger).await?;
        // Failure after writing the ledger proves SQL rollback covers both domains.
        if self.fail_settlement.load(Ordering::SeqCst) {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn complete(&self, db: &mut PgConnection, s: &Settlement) -> Result<(), Error> {
        let mut ledger = Self::load(db).await?;
        ledger.complete(s)?;
        Self::save(db, ledger).await
    }
}

pub type Api = Bazaar<TestAccounts, MemoryReservations, TestEvidence>;
pub struct Harness {
    pub pool: PgPool,
    admin: PgPool,
    schema: String,
    pub accounts: TestAccounts,
    pub funds: MemoryReservations,
    pub evidence: TestEvidence,
    pub api: Api,
    pub owner: Uuid,
    pub a: Uuid,
    pub b: Uuid,
}
impl Harness {
    pub async fn new() -> TestResult<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED bazaar PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("bazaar_test_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
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
        sqlx::query("CREATE TABLE test_ledger(id integer PRIMARY KEY, snapshot text NOT NULL)")
            .execute(&pool)
            .await?;
        let owner = Uuid::now_v7();
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let accounts = TestAccounts::default();
        let mut ledger = MemoryLedger::default();
        for actor in [owner, a, b] {
            accounts.0.write().map_err(|_| "poisoned accounts")?.insert(
                actor,
                AccountState {
                    active: true,
                    email_verified: true,
                    mfa: true,
                    sudo: true,
                    bazaar_blocked: false,
                },
            );
            for currency in [Currency::Coins, Currency::Gold] {
                ledger
                    .balances
                    .insert(MemoryLedger::key(actor, currency), 1_000_000);
            }
        }
        sqlx::query("INSERT INTO test_ledger VALUES(1,$1)")
            .bind(serde_json::to_string(&ledger)?)
            .execute(&pool)
            .await?;
        let funds = MemoryReservations::default();
        let evidence = TestEvidence::default();
        let api = Bazaar::new(
            pool.clone(),
            accounts.clone(),
            funds.clone(),
            evidence.clone(),
            Flags {
                enabled: true,
                gold_enabled: true,
            },
        );
        Ok(Some(Self {
            pool,
            admin,
            schema,
            accounts,
            funds,
            evidence,
            api,
            owner,
            a,
            b,
        }))
    }
    pub fn flags(&self, flags: Flags) -> Api {
        Bazaar::new(
            self.pool.clone(),
            self.accounts.clone(),
            self.funds.clone(),
            self.evidence.clone(),
            flags,
        )
    }
    pub fn input(&self, form: Form, currency: Currency, price: i64) -> NewListing {
        NewListing {
            owner: self.owner,
            kind: Kind::Item,
            form,
            currency,
            price,
            accept_offers: form != Form::Auction,
            ends_at: OffsetDateTime::now_utc() + Duration::days(2),
        }
    }
    pub async fn listing(&self, form: Form, currency: Currency, price: i64) -> TestResult<Listing> {
        let listing = self.api.create(self.input(form, currency, price)).await?;
        self.api.publish(listing.id, self.owner).await?;
        Ok(self.api.get(listing.id).await?)
    }
    pub async fn end_in(&self, id: Uuid, seconds: i32) -> TestResult<OffsetDateTime> {
        Ok(sqlx::query_scalar(
            "UPDATE bazaar_listings SET ends_at=clock_timestamp()+make_interval(secs=>$2),
            updated_at=clock_timestamp() WHERE id=$1 RETURNING ends_at",
        )
        .bind(id)
        .bind(f64::from(seconds))
        .fetch_one(&self.pool)
        .await?)
    }
    pub async fn ledger(&self) -> TestResult<MemoryLedger> {
        let data: String = sqlx::query_scalar("SELECT snapshot FROM test_ledger WHERE id=1")
            .fetch_one(&self.pool)
            .await?;
        Ok(serde_json::from_str(&data)?)
    }
    pub async fn finish(self) -> TestResult {
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
