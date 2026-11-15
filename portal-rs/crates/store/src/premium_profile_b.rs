//! Profile-B entitlement authority wire, Platform producer (wire contract §7-§10).
//! B1 reads one account's authority for every configured product; B2 pages the
//! lifecycle change hints. Both are read-only apart from the account's B1
//! authority row, which every answer advances (§10.2).
use super::{AccountState, Accounts, Error, MAX_REVISION};
use sqlx::{PgConnection, PgPool};
use std::{future::Future, pin::Pin, sync::Arc};
use time::OffsetDateTime;
use uuid::Uuid;

#[path = "premium_profile_b_http.rs"]
pub mod http;

pub const LEASE_POLICY: &str = "oteryn.profile_b.lease.v1";
pub const MAX_PRODUCTS: usize = 16;
const PREMIUM_PRODUCT: &str = "oteryn.premium_time";

/// One configured product/version (`PRODUCTS_ENTITLEMENTS_PROFILE_B_PRODUCTS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: String,
    pub version: i64,
}

/// Store wire `product_id` form: `^[a-z][a-z0-9_]{0,31}(\.[a-z][a-z0-9_]{0,31}){1,3}$`.
pub fn product_id(value: &str) -> bool {
    let labels: Vec<&str> = value.split('.').collect();
    value.len() <= 64
        && (2..=4).contains(&labels.len())
        && labels.iter().all(|label| {
            (1..=32).contains(&label.len())
                && label.as_bytes()[0].is_ascii_lowercase()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

/// Canonical decimal from 1 through `max`.
pub(super) fn positive(text: &str, max: i64) -> Option<i64> {
    let value: i64 = text.parse().ok()?;
    (value >= 1 && value <= max && value.to_string() == text).then_some(value)
}

/// The §8.5 product list. Unset is `[]`; anything malformed, longer than 16,
/// duplicated, or naming `oteryn.premium_time@1` is `None` and disables B1.
pub fn products(raw: Option<&str>) -> Option<Vec<Product>> {
    let Some(raw) = raw else {
        return Some(Vec::new());
    };
    let entries: Vec<String> = serde_json::from_str(raw).ok()?;
    if entries.len() > MAX_PRODUCTS {
        return None;
    }
    let mut products: Vec<Product> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let (id, version) = entry.rsplit_once('@')?;
        let version = positive(version, MAX_REVISION)?;
        if !product_id(id)
            || (id == PREMIUM_PRODUCT && version == 1)
            || products.iter().any(|p| p.id == id && p.version == version)
        {
            return None;
        }
        products.push(Product {
            id: id.to_owned(),
            version,
        });
    }
    Some(products)
}

/// The current entitlement of one account for one product/version, at most one
/// (owner decision P7). `until: None` is an open-ended commercial interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: Uuid,
    pub revoked: bool,
    pub revision: i64,
    pub from: i64,
    pub until: Option<i64>,
    pub lease_policy: String,
}
impl Record {
    /// Invalid durable state, or a record under another lease policy, never
    /// issues authority.
    pub fn validate(&self) -> Result<(), Error> {
        if self.id.get_version_num() != 7
            || self.id.get_variant() != uuid::Variant::RFC4122
            || !(1..=MAX_REVISION).contains(&self.revision)
            || self.until.is_some_and(|until| self.from >= until)
            || self.lease_policy != LEASE_POLICY
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    /// Premium snapshot §5.3 order; `NONE` is the absence of a record.
    pub fn classify(&self, now: i64) -> &'static str {
        if self.revoked {
            "REVOKED"
        } else if self.until.is_some_and(|until| now >= until) {
            "EXPIRED"
        } else if now < self.from {
            "NOT_YET_EFFECTIVE"
        } else {
            "ACTIVE"
        }
    }
}

pub type Read<'a> = Pin<Box<dyn Future<Output = Result<Option<Record>, Error>> + Send + 'a>>;

/// Entitlement source, read in the B1 transaction. A product contract that
/// declares Profile B supplies its own ledger here (#1467 decides the generic one).
pub trait Entitlements: Send + Sync + std::fmt::Debug {
    fn read<'a>(
        &'a self,
        account: Uuid,
        product: &'a Product,
        connection: &'a mut PgConnection,
    ) -> Read<'a>;
}

/// No generic entitlement ledger exists yet (#1467) and no product contract has
/// declared Profile B, so a configured product has no source of truth. Answering
/// `NONE` would be invented negative evidence; this fails closed (`503`) instead.
/// With the default empty product list it is never called.
#[derive(Debug)]
pub struct NoGenericLedger;
impl Entitlements for NoGenericLedger {
    fn read<'a>(&'a self, _: Uuid, _: &'a Product, _: &'a mut PgConnection) -> Read<'a> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}

fn uuid7(value: Uuid) -> bool {
    value.get_version_num() == 7 && value.get_variant() == uuid::Variant::RFC4122
}

/// B1 issuer. One transaction: account check, the account's authority row
/// locked, the entitlements and their published lifecycle revisions read, the
/// revision advanced, then commit.
#[derive(Debug, Clone)]
pub struct Authority {
    pool: PgPool,
    accounts: Arc<dyn Accounts>,
    entitlements: Arc<dyn Entitlements>,
    enabled: bool,
}
impl Authority {
    pub fn new(
        pool: PgPool,
        accounts: Arc<dyn Accounts>,
        entitlements: Arc<dyn Entitlements>,
        enabled: bool,
    ) -> Self {
        Self {
            pool,
            accounts,
            entitlements,
            enabled,
        }
    }
    pub async fn read(
        &self,
        account: Uuid,
        nonce: &str,
        producer_revision: &str,
        products: &[Product],
    ) -> Result<Vec<u8>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !uuid7(account) || products.len() > MAX_PRODUCTS {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        // Decided before any revision is allocated (§8.3).
        match self.accounts.state(account, &mut tx).await? {
            AccountState::Known => (),
            AccountState::Unknown => return Err(Error::UnknownAccount),
            AccountState::Ambiguous => return Err(Error::Unavailable),
        }
        sqlx::query(
            "INSERT INTO profile_b_authority(account_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(account)
        .execute(&mut *tx)
        .await?;
        let (authority, last): (i64, Option<OffsetDateTime>) = sqlx::query_as(
            "SELECT authority_revision,last_issued_at FROM profile_b_authority WHERE account_id=$1 FOR UPDATE",
        )
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
        // An exhausted or out-of-range revision cannot be advanced.
        if !(0..MAX_REVISION).contains(&authority) {
            return Err(Error::Unavailable);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let issued = now.unix_timestamp();
        // A previous answer issued after the database clock means the clock or
        // the row moved backwards; never issue an older-looking authority.
        if last.is_some_and(|last| last.unix_timestamp() > issued) {
            return Err(Error::Unavailable);
        }
        let mut records = Vec::with_capacity(products.len());
        for product in products {
            let record = self.entitlements.read(account, product, &mut tx).await?;
            if let Some(record) = &record {
                record.validate()?;
            }
            fence(&mut tx, account, product, record.as_ref()).await?;
            records.push(record);
        }
        let items: Vec<_> = products
            .iter()
            .zip(records.iter().map(Option::as_ref))
            .collect();
        let bytes = super::wire::authority(
            &items,
            issued,
            authority + 1,
            account,
            nonce,
            producer_revision,
        )?;
        let issued_at =
            OffsetDateTime::from_unix_timestamp(issued).map_err(|_| Error::Unavailable)?;
        sqlx::query("UPDATE profile_b_authority SET authority_revision=$2,last_issued_at=$3,updated_at=clock_timestamp() WHERE account_id=$1")
            .bind(account)
            .bind(authority + 1)
            .bind(issued_at)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(bytes)
    }
}

/// Every lifecycle change publishes exactly one B2 hint (§8.4), so the record's
/// lifecycle revision must equal the newest published one. A lower revision is
/// stale (Game would re-read after a hint and get older state); a higher one
/// was never published. Either is invalid durable state.
async fn fence(
    connection: &mut PgConnection,
    account: Uuid,
    product: &Product,
    record: Option<&Record>,
) -> Result<(), Error> {
    let published: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(lifecycle_revision) FROM profile_b_changes WHERE account_id=$1 AND product_id=$2 AND product_version=$3",
    )
    .bind(account)
    .bind(product.id.as_str())
    .bind(product.version)
    .fetch_one(&mut *connection)
    .await?;
    if published.unwrap_or(0) != record.map_or(0, |r| r.revision) {
        return Err(Error::Unavailable);
    }
    Ok(())
}

/// One B2 hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeItem {
    pub account_id: Uuid,
    pub product_id: String,
    pub product_version: i64,
    pub lifecycle_revision: i64,
}

/// B2 reader over the append-only journal.
#[derive(Debug, Clone)]
pub struct Changes {
    pool: PgPool,
    enabled: bool,
}
impl Changes {
    pub fn new(pool: PgPool, enabled: bool) -> Self {
        Self { pool, enabled }
    }
    /// `after` is a published position; a cursor beyond the journal head is invalid.
    pub async fn read(&self, after: Option<i64>, limit: i64) -> Result<Vec<u8>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !(1..=500).contains(&limit) || after.is_some_and(|n| n < 1) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let head: i64 =
            sqlx::query_scalar("SELECT position FROM profile_b_change_head WHERE singleton")
                .fetch_one(&mut *tx)
                .await?;
        let after = after.unwrap_or(0);
        if after > head {
            return Err(Error::Invalid);
        }
        let rows: Vec<(i64, Uuid, String, i64, i64)> = sqlx::query_as(
            "SELECT position,account_id,product_id,product_version,lifecycle_revision FROM profile_b_changes WHERE position>$1 ORDER BY position LIMIT $2",
        )
        .bind(after)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let mut items = Vec::with_capacity(rows.len());
        for (position, account_id, product, product_version, lifecycle_revision) in rows {
            if !uuid7(account_id)
                || !product_id(&product)
                || !(1..=MAX_REVISION).contains(&product_version)
                || !(1..=MAX_REVISION).contains(&lifecycle_revision)
            {
                return Err(Error::Unavailable);
            }
            items.push((
                position,
                ChangeItem {
                    account_id,
                    product_id: product,
                    product_version,
                    lifecycle_revision,
                },
            ));
        }
        super::wire::change_page(&items)
    }
}

#[cfg(test)]
#[path = "premium_profile_b_tests.rs"]
mod tests;
