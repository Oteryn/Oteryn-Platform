//! PostgreSQL admission authority for the T72 journal (G-10 items rev 3, G-11
//! progress rev 4) and the trusted database clock.
//!
//! Ownership evidence is read only from the character projection
//! (`account_characters`, `account_character_snapshots`) and the committed
//! character-command receipts in `character_commands`: the `CreateCharacter`
//! receipt is the initial cut (an accepted `pd_ownership_baselines` row stands
//! in for it only where no create receipt exists) and every committed
//! `TransferCharacterOwnership` receipt chains from it. Journal rows are never
//! evidence. Every read takes a row lock shared with the writer it must
//! serialize with, inside the store transaction, so a transfer, erasure or
//! restore cannot commit between the proof and the journal write.
use crate::store::{
    Admission, Authority, Error, FenceFuture, Lease, LeaseFuture, TrustedTime,
    http::{Clock, ClockFuture},
};
use oteryn_internal_api::{
    canonical::Value,
    playerdata::{Record, Scope},
};
use sqlx::{PgConnection, PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use oteryn_characters::ownership::{
    LiveFeed, account_epoch_with_feed, live_feed, ownership_with_feed, shared_fences,
};
pub use oteryn_characters::ownership::{Ownership, OwnershipCut};

type Result<T> = std::result::Result<T, Error>;

pub async fn account_epoch(tx: &mut PgConnection, account: Uuid) -> Result<u64> {
    Ok(oteryn_characters::ownership::account_epoch(tx, account).await?)
}

pub async fn ownership(tx: &mut PgConnection, account: Uuid, character: Uuid) -> Result<Ownership> {
    Ok(oteryn_characters::ownership::ownership(tx, account, character).await?)
}

/// G-10 families; every other family is a G-11 progress family.
const ITEM_FAMILIES: [&str; 4] = ["ITEMS", "BANK", "MARKET", "PLAYER_TRADE"];
/// G-11 families whose catalog or custody mapping is not accepted yet.
const UNPROVEN_PROGRESS: [&str; 6] = [
    "bestiary",
    "boss_cooldown",
    "quest",
    "achievement",
    "death_recap",
    "house_rent",
];

fn counter(s: &str) -> Result<u64> {
    let ok = !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit());
    if !ok {
        return Err(Error::Unavailable);
    }
    s.parse().map_err(|_| Error::Unavailable)
}
fn opt_counter(s: Option<&str>) -> Result<u64> {
    counter(s.ok_or(Error::Unavailable)?)
}
fn uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| Error::Unavailable)
}

/// The newest restore generation the platform accepted; a restore that is not
/// acknowledged yet is ambiguous and denies.
async fn restore_generation(tx: &mut PgConnection) -> Result<u64> {
    let row = sqlx::query("SELECT max(new_recovery_generation) AS g, bool_or(acknowledged_at IS NULL) AS open FROM character_restore_notices")
        .fetch_one(&mut *tx)
        .await?;
    if row.try_get::<Option<bool>, _>("open")?.unwrap_or(false) {
        return Err(Error::Unavailable);
    }
    match row.try_get::<Option<String>, _>("g")? {
        Some(g) => counter(&g),
        None => Ok(0),
    }
}

/// The account must exist and not be closed; locked against closure.
async fn account(tx: &mut PgConnection, account: Uuid) -> Result<()> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?;
    match status.as_deref() {
        Some("active" | "suspended") => Ok(()),
        Some("closed") => Err(Error::Conflict),
        _ => Err(Error::Unavailable),
    }
}

fn opt_uuid(s: Option<&str>) -> Result<Uuid> {
    uuid(s.ok_or(Error::Conflict)?)
}

/// The full fence of one journal scope.
pub async fn admit(tx: &mut PgConnection, scope: &Scope) -> Result<Admission> {
    let account_id = uuid(&scope.account).map_err(|_| Error::Conflict)?;
    let items = ITEM_FAMILIES.contains(&scope.family.as_str());
    account(tx, account_id).await?;
    shared_fences(tx).await?;
    let restored = restore_generation(tx).await?;
    let recovery = counter(&scope.recovery).map_err(|_| Error::Conflict)?;
    // Zero denotes no restore notice. Once acknowledged, the generation
    // is authoritative for account streams. Character item cuts are proven below.
    if scope.family == "BANK" {
        let announced = restored.max(1);
        if recovery > announced {
            return Err(Error::Unavailable);
        }
        if recovery < announced {
            return Err(Error::Conflict);
        }
    } else if recovery < restored
        || ((!items || scope.character.is_none()) && restored != 0 && recovery != restored)
    {
        return Err(Error::Conflict);
    }
    // Before the first restore, an established BANK ledger cannot switch
    // generations. Its cursor survives both payload and replay digest expiry.
    if scope.family == "BANK" && restored == 0 {
        let key = crate::store::stream_key(scope);
        let (ledger, _) = key.rsplit_once('|').ok_or(Error::Invalid)?;
        let changed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pd_streams
            WHERE account_id=$1 AND authority=$2 AND family='BANK' AND stored_through>0
            AND starts_with(stream_key,$3) AND stream_key<>$4)",
        )
        .bind(&scope.account)
        .bind(&scope.authority)
        .bind(format!("{ledger}|"))
        .bind(&key)
        .fetch_one(&mut *tx)
        .await?;
        if changed {
            return Err(Error::Conflict);
        }
    }
    // A G-11 stream is bound to the account's current projection epoch; a G-10
    // cut's epoch is proven with its receipt below.
    if !items && let Some(epoch) = scope.epoch.as_deref() {
        if counter(epoch).map_err(|_| Error::Conflict)? != account_epoch(tx, account_id).await? {
            return Err(Error::Conflict);
        }
        // The store holds the account lock across admission and acceptance.
        // A new epoch has its own stream and can establish its generation.
        let newer_generation: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pd_streams WHERE stream_key=$1
               AND GREATEST(recovery_high_water,watermark_recovery) < $2::text::numeric)",
        )
        .bind(crate::store::stream_key(scope))
        .bind(&scope.recovery)
        .fetch_one(&mut *tx)
        .await?;
        if newer_generation {
            return Err(Error::Conflict);
        }
    }
    let Some(character) = scope.character.as_deref() else {
        return Ok(Admission::Owner);
    };
    let character_id = uuid(character).map_err(|_| Error::Conflict)?;
    let cut = match ownership(tx, account_id, character_id).await {
        Ok(Ownership::Owner(cut)) => cut,
        Ok(Ownership::Former) => return Ok(Admission::Transferred),
        Ok(Ownership::Stranger) => return Err(Error::Conflict),
        Err(Error::Unavailable)
            if crate::store::character_permanently_deleted(tx, &scope.account, character)
                .await? =>
        {
            return Ok(Admission::Deleted);
        }
        Err(Error::Unavailable) if !items && scope.character_revision.is_some() => {
            let closed: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS(SELECT 1 FROM account_characters WHERE character_id=$5)
                    AND (EXISTS(SELECT 1 FROM pd_deleted_scopes WHERE scope_digest=$1 AND closed_revision >= $4::text::numeric)
                      OR EXISTS(SELECT 1 FROM pd_erasure_jobs WHERE account_id=$2 AND character_id=$3 AND closed_revision >= $4::text::numeric))",
            )
            .bind(crate::store::scope_digest(&scope.account, Some(character)))
            .bind(&scope.account)
            .bind(character)
            .bind(&scope.character_revision)
            .bind(character_id)
            .fetch_one(&mut *tx)
            .await?;
            return if closed {
                Ok(Admission::Transferred)
            } else {
                Err(Error::Unavailable)
            };
        }
        Err(e) => return Err(e),
    };
    // A pending purge of an older period must not consume a new period's
    // revision as a terminal deletion. Retry after cleanup establishes its fence.
    let awaiting_purge: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_erasure_jobs WHERE account_id=$1 AND character_id=$2 AND closed_revision<$3::text::numeric)")
        .bind(&scope.account).bind(character).bind(cut.character_revision.to_string()).fetch_one(&mut *tx).await?;
    if awaiting_purge {
        return Err(Error::Unavailable);
    }
    if crate::store::character_deleted(tx, &scope.account, character, Some(cut.character_revision))
        .await?
    {
        return Ok(Admission::Deleted);
    }
    let cr = opt_counter(scope.character_revision.as_deref()).map_err(|_| Error::Conflict)?;
    if !items
        && cr < cut.character_revision
        && crate::store::character_deleted(tx, &scope.account, character, Some(cr)).await?
    {
        // A closed ownership period remains transferred after reacquisition.
        // First deliveries consume their revision; previously accepted replays conflict.
        return Ok(Admission::Transferred);
    }
    if opt_uuid(scope.world.as_deref())? != cut.world {
        return Err(Error::Conflict);
    }
    let pr = scope.projection_revision.as_deref();
    let exact = if items {
        cr == cut.character_revision
            && opt_counter(scope.from_projection_revision.as_deref()).ok()
                == Some(cut.from_projection_revision)
            && opt_counter(pr).ok() == Some(cut.to_projection_revision)
            && opt_counter(scope.epoch.as_deref()).ok() == Some(cut.epoch)
            && recovery == cut.recovery
    } else {
        cr >= cut.character_revision
            && recovery >= cut.recovery
            && (scope.family == "skull" || opt_counter(pr).ok() == Some(cut.to_projection_revision))
    };
    if exact {
        Ok(Admission::CharacterOwner(cut.character_revision))
    } else {
        Err(Error::Conflict)
    }
}

/// The PG-backed authority. `acquire` is cheap: the proof is the fence, taken
/// inside the store transaction.
#[derive(Debug, Clone, Copy, Default)]
pub struct DbAuthority;
impl Authority for DbAuthority {
    fn acquire<'a>(&'a self, _scope: &'a Scope) -> LeaseFuture<'a> {
        Box::pin(async { Ok(Box::new(DbLease) as Box<dyn Lease>) })
    }
}
#[derive(Debug, Clone, Copy)]
pub struct DbLease;
impl Lease for DbLease {
    fn fence<'a>(&'a self, tx: &'a mut PgConnection, scope: &'a Scope) -> FenceFuture<'a> {
        Box::pin(admit(tx, scope))
    }
    fn validate(&self, record: &Record) -> Result<()> {
        let unproven = match (record.scope.family.as_str(), record.kind.as_str()) {
            // Item definitions need the accepted catalog mapping (G-10 open question).
            (_, "ITEMS_PAGE" | "MARKET_EVENT" | "PLAYER_TRADE_EVENT") => true,
            // House custody is not proven by any platform source.
            (_, "ITEMS_BEGIN") => {
                !matches!(record.payload.get("houses"), Some(Value::Array(a)) if a.is_empty())
            }
            (family, _) => UNPROVEN_PROGRESS.contains(&family),
        };
        if unproven {
            Err(Error::Unavailable)
        } else {
            Ok(())
        }
    }
}

/// Trusted time from the database clock: the uncertainty is the round trip
/// plus the disagreement with the local clock at its midpoint.
#[derive(Debug, Clone)]
pub struct DbClock {
    pool: PgPool,
    local_offset_ms: i64,
}
impl DbClock {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            local_offset_ms: 0,
        }
    }
    /// A clock whose local reading is skewed, to prove the uncertainty bound.
    pub fn skewed(pool: PgPool, local_offset_ms: i64) -> Self {
        Self {
            pool,
            local_offset_ms,
        }
    }
    fn local_ms(&self) -> Result<i64> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?
            .as_millis();
        i64::try_from(ms)
            .ok()
            .and_then(|ms| ms.checked_add(self.local_offset_ms))
            .ok_or(Error::Unavailable)
    }
    async fn read(&self) -> Result<TrustedTime> {
        let before = self.local_ms()?;
        let db: i64 =
            sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&self.pool)
                .await?;
        let after = self.local_ms()?;
        let rtt = after
            .checked_sub(before)
            .filter(|r| *r >= 0)
            .ok_or(Error::Unavailable)?;
        let midpoint = before + rtt / 2;
        let uncertainty = rtt
            .checked_add(
                db.abs_diff(midpoint)
                    .try_into()
                    .map_err(|_| Error::Unavailable)?,
            )
            .ok_or(Error::Unavailable)?;
        Ok(TrustedTime {
            unix_ms: u64::try_from(db).map_err(|_| Error::Unavailable)?,
            uncertainty_ms: u64::try_from(uncertainty).map_err(|_| Error::Unavailable)?,
        })
    }
}
impl Clock for DbClock {
    fn trusted_time(&self) -> ClockFuture<'_> {
        Box::pin(self.read())
    }
}

mod reads;
pub use reads::{CollectionEvidence, JournalEvidence, JournalLoginAuthority, LoginPromoter};

#[cfg(test)]
mod tests;

/// G-9 uses the same issuance, closure and restore fence as the other private
/// journals; UUID stream admission alone cannot prove a reconciled restore.
pub(crate) async fn session_fence(tx: &mut PgConnection, subject: Uuid) -> Result<u64> {
    account(tx, subject).await?;
    shared_fences(tx).await?;
    restore_generation(tx).await
}
