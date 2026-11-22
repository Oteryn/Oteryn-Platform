//! Immutable Game-authored price lists and explicit, audited activation.
use oteryn_internal_api::canonical::{self, Shape, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::{collections::HashSet, fmt};
use uuid::Uuid;

#[path = "catalog_runtime.rs"]
mod runtime;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_OFFERS: usize = 10_000;
const UNIT: Shape = Shape::Object(&[
    ("product_id", Shape::Scalar),
    ("product_version", Shape::Scalar),
    ("unit_kind", Shape::Scalar),
    ("grant_key", Shape::Scalar),
    ("grant_revision", Shape::Scalar),
    ("quantity", Shape::Scalar),
]);
const OFFER: Shape = Shape::Object(&[
    ("offer_key", Shape::Scalar),
    ("currency", Shape::Scalar),
    ("unit_price", Shape::Scalar),
    ("max_quantity", Shape::Scalar),
    ("purchasable", Shape::Scalar),
    ("units", Shape::Array(&UNIT)),
]);
const DOCUMENT: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("offers", Shape::Array(&OFFER)),
]);

#[derive(Debug)]
pub enum Error {
    InvalidDocument,
    InvalidRevision,
    Disabled,
    Denied,
    NotImported,
    Conflict,
    Database(sqlx::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "store catalog: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub product_id: String,
    pub product_version: String,
    pub unit_kind: String,
    pub grant_key: String,
    pub grant_revision: String,
    pub quantity: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub offer_key: String,
    pub currency: String,
    pub unit_price: String,
    pub max_quantity: u8,
    pub purchasable: bool,
    pub units: Vec<Unit>,
}
impl Offer {
    /// Owner-approved rollout kinds; importing provenance never enables purchases.
    pub fn available(&self) -> bool {
        self.purchasable
            && self.currency == "COINS"
            && self.units.iter().all(|u| u.unit_kind == "PREMIUM_TIME")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    contract_version: u8,
    offers: Vec<Offer>,
}
#[derive(Debug, Clone)]
pub struct Catalog {
    revision: String,
    canonical_bytes: Vec<u8>,
    offers: Vec<Offer>,
}
impl Catalog {
    pub fn parse(raw: &[u8]) -> Result<Self, Error> {
        if raw.is_empty() || raw.len() > MAX_BYTES {
            return Err(Error::InvalidDocument);
        }
        // Existing S-5 imports retain their byte identity and can still be read and
        // activated. Game runtime imports use the D2(b) digest of the source artifact.
        Self::parse_s5(raw).or_else(|_| runtime::parse(raw))
    }

    fn parse_s5(raw: &[u8]) -> Result<Self, Error> {
        let value = canonical::decode_canonical(raw, MAX_BYTES, 8, &DOCUMENT)
            .map_err(|_| Error::InvalidDocument)?;
        let bytes = canonical::encode(&value).map_err(|_| Error::InvalidDocument)?;
        let document: Document =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidDocument)?;
        if document.contract_version != 1 || !(1..=MAX_OFFERS).contains(&document.offers.len()) {
            return Err(Error::InvalidDocument);
        }
        Self::from_offers(bytes, document.offers)
    }

    fn from_offers(bytes: Vec<u8>, offers: Vec<Offer>) -> Result<Self, Error> {
        let mut seen = HashSet::new();
        for offer in &offers {
            if !crate::purchase::offer_key(&offer.offer_key)
                || !seen.insert(&offer.offer_key)
                // S-5 follows the Store wire: `COINS` only (owner D1, 2026-11-02).
                || offer.currency != "COINS"
                || positive(&offer.unit_price).is_none_or(|v| v > i64::MAX as u64)
                || !(1..=100).contains(&offer.max_quantity)
                || !(1..=8).contains(&offer.units.len())
            {
                return Err(Error::InvalidDocument);
            }
            for unit in &offer.units {
                let parts: Vec<_> = unit.product_id.split('.').collect();
                if unit.product_id.len() > 64
                    || !(2..=4).contains(&parts.len())
                    || !parts.iter().all(|p| {
                        (1..=32).contains(&p.len())
                            && p.as_bytes()[0].is_ascii_lowercase()
                            && p.bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
                    || positive(&unit.product_version).is_none()
                    || positive(&unit.grant_revision).is_none()
                    || !matches!(
                        unit.unit_kind.as_str(),
                        "PREMIUM_TIME" | "COSMETIC" | "ITEM"
                    )
                    || !grant_key(&unit.grant_key)
                    || unit.quantity == 0
                {
                    return Err(Error::InvalidDocument);
                }
            }
        }
        Ok(Self {
            revision: format!("{:x}", Sha256::digest(&bytes)),
            canonical_bytes: bytes,
            offers,
        })
    }
    /// Create a Platform price revision while preserving every imported grant binding.
    pub fn with_price(&self, offer_key: &str, unit_price: i64) -> Result<Self, Error> {
        if unit_price <= 0 {
            return Err(Error::InvalidDocument);
        }
        let mut offers = self.offers.clone();
        let offer = offers
            .iter_mut()
            .find(|offer| offer.offer_key == offer_key)
            .ok_or(Error::InvalidDocument)?;
        if unit_price
            .checked_mul(i64::from(offer.max_quantity))
            .is_none()
        {
            return Err(Error::InvalidDocument);
        }
        offer.unit_price = unit_price.to_string();
        let bytes = serde_json::to_vec(&Document {
            contract_version: 1,
            offers,
        })
        .map_err(|_| Error::InvalidDocument)?;
        Self::parse_s5(&bytes)
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub fn offers(&self) -> &[Offer] {
        &self.offers
    }
}
fn positive(s: &str) -> Option<u64> {
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        s.parse().ok()
    }
}
/// `^[a-z0-9][a-z0-9._:-]{0,63}$`.
fn grant_key(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
}
fn revision_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Denied,
    Unknown,
}
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Import,
    Activate,
}
/// Implementations must read current state and RBAC through the supplied transaction.
/// The account-state check must lock the actor until transaction end, before the
/// store acquires the shared RBAC writer fence. Missing, conflicting or unavailable
/// account state must return Unknown or an error.
pub trait CatalogAccess: Sync {
    fn account_state(
        &self,
        actor: Uuid,
        connection: &mut PgConnection,
    ) -> impl Future<Output = Result<AccountState, Error>> + Send;
    fn permitted(
        &self,
        actor: Uuid,
        action: Action,
        connection: &mut PgConnection,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}
#[derive(Debug, Default, Clone, Copy)]
pub struct CatalogFlags {
    pub enabled: bool,
}
#[derive(Debug, Clone)]
pub struct CatalogStore {
    pool: PgPool,
    flags: CatalogFlags,
    audit_log: bool,
}
impl CatalogStore {
    pub fn new(pool: PgPool, flags: CatalogFlags) -> Self {
        Self {
            pool,
            flags,
            audit_log: false,
        }
    }
    /// Record each operator change in the same transaction as its source history.
    pub fn with_audit_log(mut self) -> Self {
        self.audit_log = true;
        self
    }
    async fn authorize(
        &self,
        actor: Uuid,
        action: Action,
        access: &impl CatalogAccess,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        if actor.is_nil() || access.account_state(actor, connection).await? != AccountState::Active
        {
            return Err(Error::Denied);
        }
        // Account first, then the same fence used exclusively by role writers.
        // Keep this shared lock through permission checking and catalog commit,
        // so a revocation cannot overtake an already-authorized mutation.
        sqlx::query("SET LOCAL lock_timeout = '3s'")
            .execute(&mut *connection)
            .await?;
        sqlx::query("SELECT pg_advisory_xact_lock_shared(71010)")
            .execute(&mut *connection)
            .await?;
        if !access.permitted(actor, action, connection).await? {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub async fn import(
        &self,
        raw: &[u8],
        actor: Uuid,
        access: &impl CatalogAccess,
    ) -> Result<String, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        let catalog = Catalog::parse(raw)?;
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        self.authorize(actor, Action::Import, access, &mut tx)
            .await?;
        if insert_revision(&mut tx, &catalog, actor).await? {
            if self.audit_log {
                write_audit(&mut tx, actor, "store.catalog.imported", catalog.revision(),
                    serde_json::json!({"catalog_revision": catalog.revision(), "offers": catalog.offers().len()}),
                    Uuid::now_v7()).await?;
            }
        }
        tx.commit().await?;
        Ok(catalog.revision)
    }
    pub async fn activate(
        &self,
        revision: &str,
        actor: Uuid,
        access: &impl CatalogAccess,
    ) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        if !revision_valid(revision) {
            return Err(Error::InvalidRevision);
        }
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        // Purchases lock accounts (including referral participants) before this pointer.
        self.authorize(actor, Action::Activate, access, &mut tx)
            .await?;
        let current: Option<String> = sqlx::query_scalar(
            "SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM store_price_list_imports WHERE catalog_revision=$1)",
        )
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::NotImported);
        }
        // The catalog pointer may have waited after the initial authority check.
        self.authorize(actor, Action::Activate, access, &mut tx)
            .await?;
        if current.as_deref() != Some(revision) {
            let activation_id = Uuid::now_v7();
            sqlx::query("INSERT INTO store_catalog_activations (previous_revision,catalog_revision,actor_id,activation_id) VALUES ($1,$2,$3,$4)").bind(&current).bind(revision).bind(actor).bind(activation_id).execute(&mut *tx).await?;
            sqlx::query("UPDATE store_active_catalog SET catalog_revision=$1, activated_by=$2, updated_at=now() WHERE id=1").bind(revision).bind(actor).execute(&mut *tx).await?;
            if self.audit_log {
                write_audit(
                    &mut tx,
                    actor,
                    "store.catalog.activated",
                    revision,
                    serde_json::json!({"catalog_revision": revision, "previous_revision": current}),
                    activation_id,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn active(&self) -> Result<Option<Catalog>, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        let row = sqlx::query("SELECT i.canonical_bytes FROM store_active_catalog a JOIN store_price_list_imports i USING (catalog_revision) WHERE a.id=1").fetch_optional(&self.pool).await?;
        row.map(|r| Catalog::parse(&r.try_get::<Vec<u8>, _>("canonical_bytes")?))
            .transpose()
    }
}

/// Store validated immutable catalog data in the caller's audited transaction.
pub async fn insert_revision(
    conn: &mut PgConnection,
    catalog: &Catalog,
    actor: Uuid,
) -> Result<bool, Error> {
    let inserted = sqlx::query("INSERT INTO store_price_list_imports (catalog_revision, canonical_bytes, imported_by) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING").bind(catalog.revision()).bind(catalog.canonical_bytes()).bind(actor).execute(&mut *conn).await?.rows_affected();
    if inserted == 0 {
        let existing: Vec<u8> = sqlx::query_scalar(
            "SELECT canonical_bytes FROM store_price_list_imports WHERE catalog_revision=$1",
        )
        .bind(catalog.revision())
        .fetch_one(&mut *conn)
        .await?;
        if existing != catalog.canonical_bytes {
            return Err(Error::Conflict);
        }
    } else {
        for offer in catalog.offers() {
            let value: Value = canonical::decode(
                &serde_json::to_vec(offer).map_err(|_| Error::InvalidDocument)?,
                MAX_BYTES,
                8,
            )
            .map_err(|_| Error::InvalidDocument)?;
            let bytes = canonical::encode(&value).map_err(|_| Error::InvalidDocument)?;
            sqlx::query("INSERT INTO store_offers (catalog_revision,offer_key,currency,unit_price,max_quantity,purchasable,canonical_bytes) VALUES ($1,$2,$3,$4,$5,$6,$7)").bind(catalog.revision()).bind(&offer.offer_key).bind(&offer.currency).bind(offer.unit_price.parse::<i64>().map_err(|_| Error::InvalidDocument)?).bind(i32::from(offer.max_quantity)).bind(offer.purchasable).bind(bytes).execute(&mut *conn).await?;
        }
    }
    Ok(inserted != 0)
}

async fn write_audit(
    connection: &mut PgConnection,
    actor: Uuid,
    action: &str,
    revision: &str,
    after: serde_json::Value,
    request_id: Uuid,
) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(281510004)")
        .execute(&mut *connection)
        .await?;
    sqlx::query("INSERT INTO audit_log (actor,action,target,before,after,request_id) VALUES ($1,$2,$3,'{}'::jsonb,$4,$5)")
        .bind(actor).bind(action).bind(revision).bind(after).bind(request_id)
        .execute(connection).await?;
    Ok(())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
