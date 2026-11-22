//! Current Character Authority ownership from the portfolio and committed receipts.
use sqlx::{PgConnection, Row};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[error("character ownership is unavailable")]
pub struct Unavailable;

impl From<sqlx::Error> for Unavailable {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}

type Result<T> = std::result::Result<T, Unavailable>;

/// The character-command issuance lock (`characters` commands storage). Held
/// shared, so no transfer is issued while an admission holds its fence.
const ISSUANCE_LOCK: i64 = 22_005_160_000;
/// More transfer commands than this for one character is not read as proof.
const MAX_TRANSFERS: i64 = 256;
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
        return Err(Unavailable);
    }
    s.parse().map_err(|_| Unavailable)
}
fn opt_counter(s: Option<&str>) -> Result<u64> {
    counter(s.ok_or(Unavailable)?)
}
fn uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| Unavailable)
}

/// Shared fences against transfer issuance and restore, in the issuer's order.
pub async fn shared_fences(tx: &mut PgConnection) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(ISSUANCE_LOCK)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE")
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Unavailable)?;
    Ok(())
}

/// The account's current projection epoch, from a valid snapshot only.
pub async fn account_epoch(tx: &mut PgConnection, account: Uuid) -> Result<u64> {
    let feed = live_feed(tx).await?;
    account_epoch_with_feed(tx, account, &feed).await
}

pub async fn account_epoch_with_feed(
    tx: &mut PgConnection,
    account: Uuid,
    feed: &LiveFeed,
) -> Result<u64> {
    let row = sqlx::query("SELECT projection_epoch,invalid,source_authority FROM account_character_snapshots WHERE account_id=$1 FOR SHARE")
        .bind(account).fetch_optional(&mut *tx).await?.ok_or(Unavailable)?;
    let epoch: String = row.try_get("projection_epoch")?;
    if row.try_get::<bool, _>("invalid")?
        || epoch != feed.highest
        || row.try_get::<String, _>("source_authority")? != feed.source
    {
        return Err(Unavailable);
    }
    counter(&epoch)
}

/// The current portfolio publication fence, held for the transaction.
#[derive(Debug)]
pub struct LiveFeed {
    highest: String,
    source: String,
}

/// Locks the authoritative LCFA feed before any ownership snapshot is read.
pub async fn live_feed(tx: &mut PgConnection) -> Result<LiveFeed> {
    // LCFA serializes epoch changes and snapshot replacement on this singleton.
    // Keep its shared lock before reading any account or character snapshot.
    let feed = sqlx::query("SELECT highest_epoch,watermark_epoch,source_authority,complete_through FROM account_character_feed WHERE id=1 FOR SHARE")
        .fetch_optional(&mut *tx).await?.ok_or(Unavailable)?;
    let highest: Option<String> = feed.try_get("highest_epoch")?;
    let watermark: Option<String> = feed.try_get("watermark_epoch")?;
    let source: Option<String> = feed.try_get("source_authority")?;
    let complete: Option<i64> = feed.try_get("complete_through")?;
    // Read time after obtaining the lock, including any wait for an LCFA writer.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx)
            .await?;
    // LCFA's production readers use 30 seconds with one second uncertainty.
    if highest.is_none()
        || highest != watermark
        || source.as_deref().is_none_or(str::is_empty)
        || complete.is_none_or(|cut| {
            cut > now.saturating_add(1) || now.saturating_sub(cut).saturating_add(1) > 30
        })
    {
        return Err(Unavailable);
    }
    Ok(LiveFeed {
        highest: highest.ok_or(Unavailable)?,
        source: source.ok_or(Unavailable)?,
    })
}

/// A committed transfer receipt as (from account, cut); a rejection is `None`.
fn receipt(json: &str) -> Result<Option<(Uuid, OwnershipCut)>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| Unavailable)?;
    let s = |k: &str| v.get(k).and_then(serde_json::Value::as_str);
    if s("outcome") != Some("COMMITTED") || s("result_code") != Some("CHAR_CMD_OK") {
        return Ok(None);
    }
    let from = uuid(s("from_account_id").ok_or(Unavailable)?)?;
    Ok(Some((
        from,
        OwnershipCut {
            account: uuid(s("to_account_id").ok_or(Unavailable)?)?,
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
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| Unavailable)?;
    let s = |k: &str| v.get(k).and_then(serde_json::Value::as_str);
    if s("command") != Some("CreateCharacter")
        || s("outcome") != Some("COMMITTED")
        || s("result_code") != Some("CHAR_CMD_OK")
        || uuid(s("character_id").ok_or(Unavailable)?)? != character
    {
        return Err(Unavailable);
    }
    // A creation has no prior owner: both projection revisions are the creator's.
    let revision = opt_counter(s("projection_revision"))?;
    Ok(OwnershipCut {
        account: uuid(s("account_id").ok_or(Unavailable)?)?,
        world: uuid(s("world_id").ok_or(Unavailable)?)?,
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
        _ => return Err(Unavailable),
    }
    let base = sqlx::query("SELECT account_id,world_id,character_revision::text AS cr,from_projection_revision::text AS f,to_projection_revision::text AS t,projection_epoch::text AS e,recovery_generation::text AS r,extract(epoch FROM accepted_at)::bigint AS at FROM pd_ownership_baselines WHERE character_id=$1 FOR SHARE")
        .bind(character)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Unavailable)?;
    Ok(OwnershipCut {
        account: base.try_get("account_id")?,
        world: base.try_get("world_id")?,
        character_revision: counter(&base.try_get::<String, _>("cr")?)?,
        from_projection_revision: counter(&base.try_get::<String, _>("f")?)?,
        to_projection_revision: counter(&base.try_get::<String, _>("t")?)?,
        epoch: counter(&base.try_get::<String, _>("e")?)?,
        recovery: counter(&base.try_get::<String, _>("r")?)?,
        decided_at: u64::try_from(base.try_get::<i64, _>("at")?).map_err(|_| Unavailable)?,
    })
}

/// Proves who owns `character` now. Pending or ambiguous evidence denies
/// (`Unavailable`). Locks the character's commands and projection rows shared.
pub async fn ownership(tx: &mut PgConnection, account: Uuid, character: Uuid) -> Result<Ownership> {
    let feed = live_feed(tx).await?;
    ownership_with_feed(tx, account, character, &feed).await
}

pub async fn ownership_with_feed(
    tx: &mut PgConnection,
    account: Uuid,
    character: Uuid,
    feed: &LiveFeed,
) -> Result<Ownership> {
    let rows = sqlx::query("SELECT state,receipt_json FROM character_commands WHERE command='TransferCharacterOwnership' AND binding_json::jsonb->>'character_id'=$1 ORDER BY source_revision LIMIT $2 FOR SHARE")
        .bind(character.to_string())
        .bind(MAX_TRANSFERS + 1)
        .fetch_all(&mut *tx)
        .await?;
    if rows.len() as i64 > MAX_TRANSFERS {
        return Err(Unavailable);
    }
    let mut transfers: Vec<(Uuid, OwnershipCut)> = Vec::new();
    for row in &rows {
        if row.try_get::<String, _>("state")? != "TERMINAL" {
            return Err(Unavailable);
        }
        let json: String = row
            .try_get::<Option<String>, _>("receipt_json")?
            .ok_or(Unavailable)?;
        transfers.extend(receipt(&json)?);
    }
    // Committed transfers chain from the initial cut: each seller is the
    // previous owner, at a later character revision. (A baseline's acceptance
    // time is no decision time; the login read refuses an inverted period.)
    let mut chain = vec![initial_cut(tx, character).await?];
    for (from, cut) in transfers {
        let prev = chain.last().ok_or(Unavailable)?;
        if from != prev.account || cut.character_revision <= prev.character_revision {
            return Err(Unavailable);
        }
        chain.push(cut);
    }
    let projection = sqlx::query("SELECT c.account_id,c.world_id,s.projection_revision FROM account_characters c JOIN account_character_snapshots s ON s.account_id=c.account_id WHERE c.character_id=$1 FOR SHARE OF c, s")
        .bind(character)
        .fetch_all(&mut *tx)
        .await?;
    let [row] = projection.as_slice() else {
        return Err(Unavailable);
    };
    let owner: Uuid = row.try_get("account_id")?;
    let epoch = account_epoch_with_feed(tx, owner, feed).await?;
    let world: Uuid = row.try_get("world_id")?;
    let revision = counter(&row.try_get::<String, _>("projection_revision")?)?;
    // The initial cut names the world; transfer receipts carry none.
    if chain[0].world != world {
        return Err(Unavailable);
    }
    for cut in &mut chain {
        cut.world = world;
    }
    let current = chain.last().ok_or(Unavailable)?;
    // LCFA orders publications by (epoch, revision). A validated resync in a
    // higher epoch can restart revisions while the receipt still proves lineage.
    if current.account != owner
        || (epoch, revision) < (current.epoch, current.to_projection_revision)
    {
        return Err(Unavailable);
    }
    // The buyer row proves inclusion. The seller's publication must also
    // cover the transfer cut for the missing character row to prove removal.
    if let Some(seller) = chain.iter().rev().nth(1) {
        let revision: Option<String> = sqlx::query_scalar(
            "SELECT projection_revision FROM account_character_snapshots
             WHERE account_id=$1 AND source_authority=$2 AND projection_epoch=$3 AND NOT invalid
             FOR SHARE",
        )
        .bind(seller.account)
        .bind(&feed.source)
        .bind(&feed.highest)
        .fetch_optional(&mut *tx)
        .await?;
        if (epoch, opt_counter(revision.as_deref())?)
            < (current.epoch, current.from_projection_revision)
        {
            return Err(Unavailable);
        }
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
