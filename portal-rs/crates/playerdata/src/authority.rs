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

type Result<T> = std::result::Result<T, Error>;

/// The character-command issuance lock (`characters` commands storage). Held
/// shared, so no transfer is issued while an admission holds its fence.
const ISSUANCE_LOCK: i64 = 22_005_160_000;
/// More transfer commands than this for one character is not read as proof.
const MAX_TRANSFERS: i64 = 256;
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

/// The receipt cut that made an account the owner of a character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipCut {
    pub account: Uuid,
    pub world: Uuid,
    pub character_revision: u64,
    pub from_projection_revision: u64,
    pub to_projection_revision: u64,
    pub epoch: u64,
    pub recovery: u64,
    /// Unix seconds of the receipt decision (or baseline acceptance).
    pub decided_at: u64,
}

/// The result of proving one character inside a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ownership {
    /// The account owns the character at this cut.
    Owner(OwnershipCut),
    /// The account owned the character before a committed transfer.
    Former,
    /// The account never owned the character.
    Stranger,
}

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

/// Shared fences against transfer issuance and restore, in the issuer's order.
async fn shared_fences(tx: &mut PgConnection) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(ISSUANCE_LOCK)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE")
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Unavailable)?;
    Ok(())
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

/// The account's current projection epoch, from a valid snapshot only.
pub async fn account_epoch(tx: &mut PgConnection, account: Uuid) -> Result<u64> {
    let row = sqlx::query("SELECT projection_epoch,invalid FROM account_character_snapshots WHERE account_id=$1 FOR SHARE")
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Unavailable)?;
    if row.try_get::<bool, _>("invalid")? {
        return Err(Error::Unavailable);
    }
    counter(&row.try_get::<String, _>("projection_epoch")?)
}

/// A committed transfer receipt as (from account, cut); a rejection is `None`.
fn receipt(json: &str) -> Result<Option<(Uuid, OwnershipCut)>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::Unavailable)?;
    let s = |k: &str| v.get(k).and_then(serde_json::Value::as_str);
    if s("outcome") != Some("COMMITTED") || s("result_code") != Some("CHAR_CMD_OK") {
        return Ok(None);
    }
    let from = uuid(s("from_account_id").ok_or(Error::Unavailable)?)?;
    Ok(Some((
        from,
        OwnershipCut {
            account: uuid(s("to_account_id").ok_or(Error::Unavailable)?)?,
            // The receipt carries no world; the projection row supplies it.
            world: Uuid::nil(),
            character_revision: opt_counter(s("character_revision"))?,
            from_projection_revision: opt_counter(s("from_projection_revision"))?,
            to_projection_revision: opt_counter(s("to_projection_revision"))?,
            epoch: opt_counter(s("projection_epoch"))?,
            recovery: opt_counter(s("recovery_generation"))?,
            decided_at: opt_counter(s("decided_at"))?,
        },
    )))
}

/// The committed `CreateCharacter` receipt of `character` as its initial cut.
/// Any other receipt is not creation evidence.
fn create_receipt(json: &str, character: Uuid) -> Result<OwnershipCut> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::Unavailable)?;
    let s = |k: &str| v.get(k).and_then(serde_json::Value::as_str);
    if s("command") != Some("CreateCharacter")
        || s("outcome") != Some("COMMITTED")
        || s("result_code") != Some("CHAR_CMD_OK")
        || uuid(s("character_id").ok_or(Error::Unavailable)?)? != character
    {
        return Err(Error::Unavailable);
    }
    // A creation has no prior owner: both projection revisions are the creator's.
    let revision = opt_counter(s("projection_revision"))?;
    Ok(OwnershipCut {
        account: uuid(s("account_id").ok_or(Error::Unavailable)?)?,
        world: uuid(s("world_id").ok_or(Error::Unavailable)?)?,
        character_revision: opt_counter(s("character_revision"))?,
        from_projection_revision: revision,
        to_projection_revision: revision,
        epoch: opt_counter(s("projection_epoch"))?,
        recovery: opt_counter(s("recovery_generation"))?,
        decided_at: opt_counter(s("decided_at"))?,
    })
}

/// The initial ownership cut: the character's one committed `CreateCharacter`
/// receipt, else an accepted baseline. Neither, or more than one creation, is
/// unavailable; there is never a default owner.
async fn initial_cut(tx: &mut PgConnection, character: Uuid) -> Result<OwnershipCut> {
    let created = sqlx::query("SELECT receipt_json FROM character_commands WHERE command='CreateCharacter' AND state='TERMINAL' AND receipt_json::jsonb->>'character_id'=$1 LIMIT 2 FOR SHARE")
        .bind(character.to_string())
        .fetch_all(&mut *tx)
        .await?;
    match created.as_slice() {
        [row] => return create_receipt(&row.try_get::<String, _>("receipt_json")?, character),
        [] => {}
        _ => return Err(Error::Unavailable),
    }
    let base = sqlx::query("SELECT account_id,world_id,character_revision::text AS cr,from_projection_revision::text AS f,to_projection_revision::text AS t,projection_epoch::text AS e,recovery_generation::text AS r,extract(epoch FROM accepted_at)::bigint AS at FROM pd_ownership_baselines WHERE character_id=$1 FOR SHARE")
        .bind(character)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Unavailable)?;
    Ok(OwnershipCut {
        account: base.try_get("account_id")?,
        world: base.try_get("world_id")?,
        character_revision: counter(&base.try_get::<String, _>("cr")?)?,
        from_projection_revision: counter(&base.try_get::<String, _>("f")?)?,
        to_projection_revision: counter(&base.try_get::<String, _>("t")?)?,
        epoch: counter(&base.try_get::<String, _>("e")?)?,
        recovery: counter(&base.try_get::<String, _>("r")?)?,
        decided_at: u64::try_from(base.try_get::<i64, _>("at")?).map_err(|_| Error::Unavailable)?,
    })
}

/// Proves who owns `character` now. Pending or ambiguous evidence denies
/// (`Unavailable`). Locks the character's commands and projection rows shared.
pub async fn ownership(tx: &mut PgConnection, account: Uuid, character: Uuid) -> Result<Ownership> {
    let rows = sqlx::query("SELECT state,receipt_json FROM character_commands WHERE command='TransferCharacterOwnership' AND binding_json::jsonb->>'character_id'=$1 ORDER BY source_revision LIMIT $2 FOR SHARE")
        .bind(character.to_string())
        .bind(MAX_TRANSFERS + 1)
        .fetch_all(&mut *tx)
        .await?;
    if rows.len() as i64 > MAX_TRANSFERS {
        return Err(Error::Unavailable);
    }
    let mut transfers: Vec<(Uuid, OwnershipCut)> = Vec::new();
    for row in &rows {
        if row.try_get::<String, _>("state")? != "TERMINAL" {
            return Err(Error::Unavailable);
        }
        let json: String = row
            .try_get::<Option<String>, _>("receipt_json")?
            .ok_or(Error::Unavailable)?;
        transfers.extend(receipt(&json)?);
    }
    // Committed transfers chain from the initial cut: each seller is the
    // previous owner, at a later character revision. (A baseline's acceptance
    // time is no decision time; the login read refuses an inverted period.)
    let mut chain = vec![initial_cut(tx, character).await?];
    for (from, cut) in transfers {
        let prev = chain.last().ok_or(Error::Unavailable)?;
        if from != prev.account || cut.character_revision <= prev.character_revision {
            return Err(Error::Unavailable);
        }
        chain.push(cut);
    }
    let projection = sqlx::query("SELECT c.account_id,c.world_id,s.projection_epoch,s.projection_revision,s.invalid FROM account_characters c JOIN account_character_snapshots s ON s.account_id=c.account_id WHERE c.character_id=$1 FOR SHARE OF c, s")
        .bind(character)
        .fetch_all(&mut *tx)
        .await?;
    let [row] = projection.as_slice() else {
        return Err(Error::Unavailable);
    };
    if row.try_get::<bool, _>("invalid")? {
        return Err(Error::Unavailable);
    }
    let owner: Uuid = row.try_get("account_id")?;
    let world: Uuid = row.try_get("world_id")?;
    let epoch = counter(&row.try_get::<String, _>("projection_epoch")?)?;
    let revision = counter(&row.try_get::<String, _>("projection_revision")?)?;
    // The initial cut names the world; transfer receipts carry none.
    if chain[0].world != world {
        return Err(Error::Unavailable);
    }
    for cut in &mut chain {
        cut.world = world;
    }
    let current = chain.last().ok_or(Error::Unavailable)?;
    if current.account != owner
        || current.epoch != epoch
        || revision < current.to_projection_revision
    {
        return Err(Error::Unavailable);
    }
    if current.account == account {
        return Ok(Ownership::Owner(current.clone()));
    }
    Ok(if chain.iter().any(|c| c.account == account) {
        Ownership::Former
    } else {
        Ownership::Stranger
    })
}

fn opt_uuid(s: Option<&str>) -> Result<Uuid> {
    uuid(s.ok_or(Error::Conflict)?)
}

/// The full fence of one journal scope.
pub async fn admit(tx: &mut PgConnection, scope: &Scope) -> Result<Admission> {
    let account_id = uuid(&scope.account).map_err(|_| Error::Conflict)?;
    let items = ITEM_FAMILIES.contains(&scope.family.as_str());
    shared_fences(tx).await?;
    account(tx, account_id).await?;
    let restored = restore_generation(tx).await?;
    let recovery = counter(&scope.recovery).map_err(|_| Error::Conflict)?;
    if recovery < restored {
        return Err(Error::Conflict);
    }
    // A G-11 stream is bound to the account's current projection epoch; a G-10
    // cut's epoch is proven with its receipt below.
    if !items
        && let Some(epoch) = scope.epoch.as_deref()
        && counter(epoch).map_err(|_| Error::Conflict)? != account_epoch(tx, account_id).await?
    {
        return Err(Error::Conflict);
    }
    let Some(character) = scope.character.as_deref() else {
        return Ok(Admission::Owner);
    };
    let character_id = uuid(character).map_err(|_| Error::Conflict)?;
    let deleted = crate::store::character_deleted(tx, &scope.account, character).await?;
    let proof = ownership(tx, account_id, character_id).await;
    let cut = match proof {
        Ok(Ownership::Owner(_)) | Err(_) if deleted => return Ok(Admission::Deleted),
        Ok(Ownership::Owner(cut)) => cut,
        Ok(Ownership::Former) => return Ok(Admission::Transferred),
        Ok(Ownership::Stranger) => return Err(Error::Conflict),
        Err(e) => return Err(e),
    };
    if opt_uuid(scope.world.as_deref())? != cut.world {
        return Err(Error::Conflict);
    }
    let cr = opt_counter(scope.character_revision.as_deref()).map_err(|_| Error::Conflict)?;
    let pr = scope.projection_revision.as_deref();
    let exact = if items {
        cr == cut.character_revision
            && opt_counter(scope.from_projection_revision.as_deref()).ok()
                == Some(cut.from_projection_revision)
            && opt_counter(pr).ok() == Some(cut.to_projection_revision)
            && opt_counter(scope.epoch.as_deref()).ok() == Some(cut.epoch)
            && recovery >= cut.recovery
    } else {
        cr >= cut.character_revision
            && recovery >= cut.recovery
            && (scope.family == "skull" || opt_counter(pr).ok() == Some(cut.to_projection_revision))
    };
    if exact {
        Ok(Admission::Owner)
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
pub use reads::{JournalEvidence, JournalLoginAuthority, LoginPromoter};

#[cfg(test)]
mod tests;
