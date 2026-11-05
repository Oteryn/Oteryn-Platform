//! Private estimated gold valuations (T74), never authoritative balances or receipts.
//!
//! T72 supplies Access from authenticated server context and accepted journal evidence.
//! No ingest or command route is introduced here. G-9 is still proposed: its older
//! ownership field does not replace the revision-2 receipt cut required below.
//! T63 supplies BazaarPrices through its public API when available; absence denies.

use std::future::Future;

use oteryn_world::WorldStore;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

mod npc;
#[cfg(test)]
mod tests;

pub const GOLD_CEILING: i64 = 999_999_999_999;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("pricing is disabled")]
    Disabled,
    #[error("private pricing access denied")]
    Denied,
    #[error("invalid pricing input")]
    BadInput,
    #[error("price or catalogue evidence unavailable")]
    Unavailable,
    #[error("pricing storage unavailable")]
    Storage(#[from] sqlx::Error),
}

/// All pricing reads and preference mutations are off by default.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub bazaar_enabled: bool,
}

/// Receipt cut, not an ownership epoch or inferred public projection watermark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptCut {
    pub character_revision: u64,
    pub from_projection_revision: u64,
    pub to_projection_revision: u64,
    pub projection_epoch: u64,
    pub recovery_generation: u64,
}

/// Obtained from validated, complete, durable journal storage by the composing T72 service.
/// Never deserialize this structure from browser or ingest request parameters.
#[derive(Debug, Clone)]
pub struct Context {
    pub account: Uuid,
    pub character: Uuid,
    pub world: Uuid,
    pub cut: ReceiptCut,
    pub catalogue_version: Uuid,
    pub catalogue_revision: String,
}

pub trait Access: Send + Sync {
    /// Lock current privacy/account/receipt fences through this transaction. Require
    /// active authenticated owner, accepted catalogue mapping and complete journal
    /// evidence after durable logout. Old-owner history is scoped to its capture cut;
    /// current character ownership alone never grants access to that history.
    /// Erasure, opt-out, missing freshness or ambiguous evidence returns false.
    fn allows(
        &self,
        db: &mut PgConnection,
        authenticated_account: Uuid,
        context: &Context,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAccess;
impl Access for DenyAccess {
    async fn allows(&self, _: &mut PgConnection, _: Uuid, _: &Context) -> Result<bool, Error> {
        Ok(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Loot,
    Supplies,
}
impl Purpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::Loot => "loot",
            Self::Supplies => "supplies",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub key: String,
    pub revision: String,
}

/// The choice is per item, world, revision and purpose. No silent source fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Npc {
        npc_key: String,
        service_key: String,
    },
    Bazaar,
    Custom {
        unit_gold: i64,
    },
}

/// T63's estimated gold price, already normalized per unit for this exact item/world.
/// Only completed gold sales qualify; never convert coin sales or use asking prices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketPrice {
    pub world: Uuid,
    pub item: Item,
    pub unit_gold: i64,
    pub observed_at: i64,
    pub valid_until: i64,
    pub sample_count: u32,
    pub provenance: String,
}

pub trait BazaarPrices: Send + Sync {
    fn estimate(
        &self,
        context: &Context,
        item: &Item,
        purpose: Purpose,
    ) -> impl Future<Output = Result<Option<MarketPrice>, Error>> + Send;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoBazaarPrices;
impl BazaarPrices for NoBazaarPrices {
    async fn estimate(
        &self,
        _: &Context,
        _: &Item,
        _: Purpose,
    ) -> Result<Option<MarketPrice>, Error> {
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    Npc {
        npc_key: String,
        service_key: String,
        catalogue_version: Uuid,
    },
    Bazaar(MarketPrice),
    Player {
        updated_at: i64,
    },
}

/// Presentation result: always an estimate in gold, never a gameplay fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate {
    pub item: Item,
    pub purpose: Purpose,
    pub quantity: i64,
    pub unit_gold: i64,
    pub total_gold: i64,
    pub calculated_at: i64,
    pub provenance: Provenance,
}

#[derive(Debug, Clone)]
pub struct Pricing<A = DenyAccess, B = NoBazaarPrices> {
    pool: PgPool,
    world: WorldStore,
    access: A,
    bazaar: B,
    flags: Flags,
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 192
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-/".contains(&b))
}
fn valid_item(item: &Item) -> bool {
    token(&item.key) && token(&item.revision)
}
fn gold(value: i64) -> bool {
    (0..=GOLD_CEILING).contains(&value)
}
fn validate_selection(selection: &Selection) -> Result<(), Error> {
    match selection {
        Selection::Npc {
            npc_key,
            service_key,
        } if token(npc_key) && token(service_key) => Ok(()),
        Selection::Bazaar => Ok(()),
        Selection::Custom { unit_gold } if gold(*unit_gold) => Ok(()),
        _ => Err(Error::BadInput),
    }
}

impl<A: Access, B: BazaarPrices> Pricing<A, B> {
    pub fn new(pool: PgPool, world: WorldStore, access: A, bazaar: B, flags: Flags) -> Self {
        Self {
            pool,
            world,
            access,
            bazaar,
            flags,
        }
    }

    fn check(&self, actor: Uuid, context: &Context, item: &Item) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        if actor != context.account {
            return Err(Error::Denied);
        }
        if [
            actor,
            context.character,
            context.world,
            context.catalogue_version,
        ]
        .iter()
        .any(|id| id.get_version_num() != 7)
            || context.cut.character_revision == 0
            || context.cut.from_projection_revision == 0
            || context.cut.to_projection_revision == 0
            || context.cut.projection_epoch == 0
            || context.cut.recovery_generation == 0
            || !token(&context.catalogue_revision)
            || !valid_item(item)
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }

    async fn authorize(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        context: &Context,
    ) -> Result<(), Error> {
        if self.access.allows(db, actor, context).await? {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }

    async fn catalogue_item(&self, context: &Context, item: &Item) -> Result<(), Error> {
        let entity = self
            .world
            .get_entity("Item", &item.key)
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if entity.version_id != context.catalogue_version
            || entity
                .data
                .pointer("/declaration/identity/key")
                .and_then(|v| v.as_str())
                != Some(item.key.as_str())
            || entity
                .data
                .pointer("/declaration/identity/revision")
                .and_then(|v| v.as_str())
                != Some(item.revision.as_str())
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }

    pub async fn set_selection(
        &self,
        actor: Uuid,
        context: &Context,
        item: &Item,
        purpose: Purpose,
        selection: Selection,
    ) -> Result<(), Error> {
        self.check(actor, context, item)?;
        validate_selection(&selection)?;
        if matches!(selection, Selection::Bazaar) && !self.flags.bazaar_enabled {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, context).await?;
        tx.commit().await?;
        self.catalogue_item(context, item).await?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, context).await?;
        sqlx::query("INSERT INTO pd_price_overrides (account_id, world_id, item_key, item_revision, purpose, selection) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (account_id, world_id, item_key, item_revision, purpose) DO UPDATE SET selection = EXCLUDED.selection, updated_at = now()")
            .bind(actor).bind(context.world).bind(&item.key).bind(&item.revision)
            .bind(purpose.as_str()).bind(sqlx::types::Json(selection)).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn clear_selection(
        &self,
        actor: Uuid,
        context: &Context,
        item: &Item,
        purpose: Purpose,
    ) -> Result<(), Error> {
        self.check(actor, context, item)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, context).await?;
        sqlx::query("DELETE FROM pd_price_overrides WHERE account_id=$1 AND world_id=$2 AND item_key=$3 AND item_revision=$4 AND purpose=$5")
            .bind(actor).bind(context.world).bind(&item.key).bind(&item.revision)
            .bind(purpose.as_str()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Server clock is read in the transaction. No cached private estimates survive
    /// authorization: every read checks current fences. Missing selection is unavailable.
    pub async fn estimate(
        &self,
        actor: Uuid,
        context: &Context,
        item: &Item,
        purpose: Purpose,
        quantity: i64,
    ) -> Result<Estimate, Error> {
        self.check(actor, context, item)?;
        if quantity <= 0 {
            return Err(Error::BadInput);
        }
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, context).await?;
        let row: Option<(sqlx::types::Json<Selection>, i64)> = sqlx::query_as(
            "SELECT selection, floor(extract(epoch FROM updated_at))::bigint FROM pd_price_overrides WHERE account_id=$1 AND world_id=$2 AND item_key=$3 AND item_revision=$4 AND purpose=$5")
            .bind(actor).bind(context.world).bind(&item.key).bind(&item.revision)
            .bind(purpose.as_str()).fetch_optional(&mut *tx).await?;
        let (sqlx::types::Json(selection), updated_at) = row.ok_or(Error::Unavailable)?;
        validate_selection(&selection).map_err(|_| Error::Unavailable)?;
        // Public world/bazaar APIs may need their own pool connection. Release
        // this transaction first, then reauthorize and bind the unchanged choice.
        tx.commit().await?;
        self.catalogue_item(context, item).await?;
        let original_selection = selection.clone();
        let (unit_gold, mut provenance) = match selection {
            Selection::Custom { unit_gold } => (unit_gold, Provenance::Player { updated_at }),
            Selection::Npc {
                npc_key,
                service_key,
            } => {
                let price =
                    npc::price(&self.world, context, item, purpose, &npc_key, &service_key).await?;
                (
                    price,
                    Provenance::Npc {
                        npc_key,
                        service_key,
                        catalogue_version: context.catalogue_version,
                    },
                )
            }
            Selection::Bazaar => {
                if !self.flags.bazaar_enabled {
                    return Err(Error::Disabled);
                }
                let market = self
                    .bazaar
                    .estimate(context, item, purpose)
                    .await?
                    .ok_or(Error::Unavailable)?;
                if market.world != context.world
                    || market.item != *item
                    || !gold(market.unit_gold)
                    || market.sample_count == 0
                    || market.observed_at <= 0
                    || market.valid_until <= market.observed_at
                    || !token(&market.provenance)
                {
                    return Err(Error::Unavailable);
                }
                (market.unit_gold, Provenance::Bazaar(market))
            }
        };
        let total_gold = quantity
            .checked_mul(unit_gold)
            .filter(|v| gold(*v))
            .ok_or(Error::BadInput)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, context).await?;
        let current: Option<(sqlx::types::Json<Selection>, i64)> = sqlx::query_as(
            "SELECT selection, floor(extract(epoch FROM updated_at))::bigint FROM pd_price_overrides WHERE account_id=$1 AND world_id=$2 AND item_key=$3 AND item_revision=$4 AND purpose=$5 FOR SHARE")
            .bind(actor).bind(context.world).bind(&item.key).bind(&item.revision)
            .bind(purpose.as_str()).fetch_optional(&mut *tx).await?;
        let (sqlx::types::Json(current), updated_at) = current.ok_or(Error::Unavailable)?;
        if current != original_selection {
            return Err(Error::Unavailable);
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        match &mut provenance {
            Provenance::Bazaar(market) if market.valid_until <= now || market.observed_at > now => {
                return Err(Error::Unavailable);
            }
            Provenance::Player {
                updated_at: timestamp,
            } => *timestamp = updated_at,
            _ => {}
        }
        tx.commit().await?;
        Ok(Estimate {
            item: item.clone(),
            purpose,
            quantity,
            unit_gold,
            total_gold,
            calculated_at: now,
            provenance,
        })
    }
}
