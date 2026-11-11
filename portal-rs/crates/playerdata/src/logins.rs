//! G-11 login projections. T72 owns authenticated canonical ingest, durable replay
//! digests, contiguous family sequences, baseline promotion and restore fencing.
//! This seam accepts only a promoted replacement in that same transaction. It is
//! deliberately not a second wire parser or a game session command surface.
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

#[cfg(test)]
mod tests;
pub mod web;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Unavailable,
    Denied,
    Invalid,
    Conflict,
    Database(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub progress_ingest_enabled: bool,
    pub progress_read_enabled: bool,
    pub game_login_history_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndReason {
    Logout,
    Disconnect,
    Revoked,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Login {
    pub login_event_id: Uuid,
    pub entered_at: String,
    pub left_at: Option<String>,
    pub end_reason: Option<EndReason>,
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn time(value: &str) -> Result<i64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::Invalid);
    }
    let value: i64 = value.parse().map_err(|_| Error::Invalid)?;
    if value > 253402300799999 {
        return Err(Error::Invalid);
    }
    Ok(value)
}
impl Login {
    /// Closed, ordered four-member payload, including explicit nullable members.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 512 {
            return Err(Error::Invalid);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        if serde_json::to_vec(&value).map_err(|_| Error::Invalid)? != bytes {
            return Err(Error::Invalid);
        }
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        if !valid_id(self.login_event_id) {
            return Err(Error::Invalid);
        }
        let entered = time(&self.entered_at)?;
        match (&self.left_at, &self.end_reason) {
            (None, None) => Ok(()),
            (Some(left), Some(_)) if time(left)? >= entered => Ok(()),
            _ => Err(Error::Invalid),
        }
    }
}
/// Independently verified current ownership and the promoted game_login family
/// cut. Implementations must lock/revalidate Identity, ownership, visibility and
/// restore authority in the supplied transaction, never trust request/journal IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    pub epoch: u64,
    pub recovery_generation: u64,
    pub complete_through: u64,
    pub observed_at_ms: i64,
    pub time_uncertainty_ms: u32,
    pub baseline_complete: bool,
    pub visibility_floor_ms: i64,
    pub projection_revision: Option<u64>,
    pub ownership_character_revision: Option<u64>,
    pub world_id: Option<Uuid>,
}
pub type AuthorityFuture<'a> = Pin<Box<dyn Future<Output = Result<Cut>> + Send + 'a>>;
pub trait Authority: Send + Sync + std::fmt::Debug {
    fn current<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Option<Uuid>,
    ) -> AuthorityFuture<'a>;
}
#[derive(Debug)]
pub struct UnavailableAuthority;
impl Authority for UnavailableAuthority {
    fn current<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: Uuid,
        _: Option<Uuid>,
    ) -> AuthorityFuture<'a> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
#[derive(Debug, Clone)]
pub struct Logins {
    pub pool: PgPool,
    pub flags: Flags,
    pub authority: Arc<dyn Authority>,
}
#[derive(Debug, Clone)]
pub struct PromotedLogin {
    pub account: Uuid,
    pub character: Option<Uuid>,
    pub character_revision: Option<u64>,
    pub cut: Cut,
    pub source_revision: u64,
    pub login: Login,
}
#[derive(Debug, sqlx::FromRow)]
struct Row {
    login_event_id: Uuid,
    character_id: Option<Uuid>,
    world_id: Option<Uuid>,
    epoch: String,
    recovery: String,
    character_revision: Option<String>,
    revision: Option<String>,
    source: String,
    entered_at: i64,
    left_at: Option<i64>,
    end_reason: Option<String>,
}
#[derive(Debug, sqlx::FromRow)]
struct ExistingLogin {
    entered_at: i64,
    left_at: Option<i64>,
    character_id: Option<Uuid>,
    end_reason: Option<String>,
}
#[derive(Debug)]
pub struct Entry {
    pub login_event_id: Uuid,
    pub character: Option<Uuid>,
    pub entered_at: i64,
    pub left_at: Option<i64>,
    pub end_reason: Option<String>,
    pub stale: bool,
}
async fn now(tx: &mut Transaction<'_, Postgres>) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}
fn check_cut(cut: &Cut, character: Option<Uuid>, now: i64) -> Result<()> {
    if now < 0 {
        return Err(Error::Unavailable);
    }
    if !cut.baseline_complete
        || cut.time_uncertainty_ms > 5000
        || cut.epoch == 0
        || cut.recovery_generation == 0
        || cut.observed_at_ms < 0
        || cut.observed_at_ms > now
        || now - cut.observed_at_ms > 86_400_000
        || cut.visibility_floor_ms < 0
        || character.is_some() != cut.projection_revision.is_some()
        || character.is_some() != cut.ownership_character_revision.is_some()
        || character.is_some() != cut.world_id.is_some()
        || cut.ownership_character_revision == Some(0)
        || cut.world_id.is_some_and(|id| !valid_id(id))
        || cut.projection_revision == Some(0)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
impl Logins {
    /// T72 calls after checking exact retry/conflict bytes, sequence, source and
    /// accepted baseline. No public HTTP handler can construct this trust boundary.
    /// Any error requires the caller to roll back its entire ingest transaction.
    pub async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: &PromotedLogin,
    ) -> Result<()> {
        if !self.flags.progress_ingest_enabled {
            return Err(Error::Unavailable);
        }
        record.login.validate()?;
        if !valid_id(record.account)
            || record.character.is_some_and(|id| !valid_id(id))
            || record.character.is_some() != record.character_revision.is_some()
            || record.character_revision == Some(0)
            || record.source_revision == 0
            || record.source_revision > record.cut.complete_through
        {
            return Err(Error::Invalid);
        }
        let current = self
            .authority
            .current(tx, record.account, record.character)
            .await?;
        let now = now(tx).await?;
        check_cut(&current, record.character, now)?;
        if current != record.cut
            || record
                .character_revision
                .zip(current.ownership_character_revision)
                .is_some_and(|(actual, floor)| actual < floor)
        {
            return Err(Error::Conflict);
        }
        let entered = time(&record.login.entered_at)?;
        if entered > now || entered < current.visibility_floor_ms {
            return Err(Error::Denied);
        }
        let left = record.login.left_at.as_deref().map(time).transpose()?;
        if left.is_some_and(|t| t > now) {
            return Err(Error::Invalid);
        }
        // Referral inactivity outlives the presentation payload. T72 has already
        // authenticated this cut, including a recent end of a very old session.
        sqlx::query("INSERT INTO referral_activity(account_id,last_activity) VALUES($1,to_timestamp($2::double precision/1000.0)) ON CONFLICT(account_id) DO UPDATE SET last_activity=GREATEST(referral_activity.last_activity,EXCLUDED.last_activity),updated_at=clock_timestamp()")
            .bind(record.account).bind(left.unwrap_or(entered)).execute(&mut **tx).await?;
        // Expired payloads are never resurrected. T72 retains the minimal replay
        // acknowledgement and sequence even when no presentation row is inserted.
        if entered < now - 30 * 86_400_000 {
            return Ok(());
        }
        let old: Option<ExistingLogin> = sqlx::query_as("SELECT entered_at,left_at,character_id,end_reason FROM pd_game_logins WHERE account_id=$1 AND login_event_id=$2 FOR UPDATE")
            .bind(record.account).bind(record.login.login_event_id).fetch_optional(&mut **tx).await?;
        let reason = record.login.end_reason.as_ref().map(|v| match v {
            EndReason::Logout => "LOGOUT",
            EndReason::Disconnect => "DISCONNECT",
            EndReason::Revoked => "REVOKED",
            EndReason::Unknown => "UNKNOWN",
        });
        if old.is_some_and(|previous| {
            let ExistingLogin {
                entered_at: start,
                left_at: end,
                character_id: character,
                end_reason: old_reason,
            } = previous;
            (old_reason.is_some() && old_reason.as_deref() != reason)
                || start != entered
                || end.is_some_and(|t| Some(t) != left)
                || (character.is_some() && character != record.character)
        }) {
            return Err(Error::Conflict);
        }
        let changed = sqlx::query("INSERT INTO pd_game_logins(account_id,login_event_id,character_id,world_id,projection_epoch,recovery_generation,projection_revision,character_revision,source_revision,entered_at,left_at,end_reason) VALUES($1,$2,$3,$12,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9,$10,$11) ON CONFLICT(account_id,login_event_id) DO UPDATE SET character_id=EXCLUDED.character_id,world_id=EXCLUDED.world_id,projection_epoch=EXCLUDED.projection_epoch,recovery_generation=EXCLUDED.recovery_generation,projection_revision=EXCLUDED.projection_revision,character_revision=EXCLUDED.character_revision,source_revision=EXCLUDED.source_revision,left_at=EXCLUDED.left_at,end_reason=EXCLUDED.end_reason,updated_at=now() WHERE pd_game_logins.source_revision < EXCLUDED.source_revision AND pd_game_logins.projection_epoch=EXCLUDED.projection_epoch AND pd_game_logins.recovery_generation=EXCLUDED.recovery_generation")
            .bind(record.account).bind(record.login.login_event_id).bind(record.character).bind(current.epoch.to_string()).bind(current.recovery_generation.to_string()).bind(current.projection_revision.map(|v| v.to_string())).bind(record.character_revision.map(|v| v.to_string())).bind(record.source_revision.to_string()).bind(entered).bind(left).bind(reason).bind(current.world_id).execute(&mut **tx).await?.rows_affected();
        if changed != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    /// Authenticate the actor in Identity before calling; there is deliberately
    /// no requested account parameter. Recheck accepted authority per subject.
    pub async fn history(&self, actor: Uuid) -> Result<Vec<Entry>> {
        if !self.flags.progress_read_enabled || !self.flags.game_login_history_enabled {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await?;
        let now = now(&mut tx).await?;
        let base = self.authority.current(&mut tx, actor, None).await?;
        check_cut(&base, None, now)?;
        let rows: Vec<Row> = sqlx::query_as("SELECT login_event_id,character_id,world_id,projection_epoch::text AS epoch,recovery_generation::text AS recovery,character_revision::text AS character_revision,projection_revision::text AS revision,source_revision::text AS source,entered_at,left_at,end_reason FROM pd_game_logins WHERE account_id=$1 AND entered_at >= $2 ORDER BY entered_at DESC,login_event_id DESC LIMIT 100")
            .bind(actor).bind(now - 30 * 86_400_000).fetch_all(&mut *tx).await?;
        if rows.is_empty() && now - base.observed_at_ms > 10_000 {
            return Err(Error::Unavailable);
        }
        let mut entries = Vec::new();
        for row in rows {
            let cut = self
                .authority
                .current(&mut tx, actor, row.character_id)
                .await?;
            check_cut(&cut, row.character_id, now)?;
            let character_revision = row
                .character_revision
                .as_deref()
                .map(str::parse::<u64>)
                .transpose()
                .map_err(|_| Error::Unavailable)?;
            if row.entered_at > now
                || row.left_at.is_some_and(|left| left > now)
                || row.world_id != cut.world_id
                || character_revision
                    .zip(cut.ownership_character_revision)
                    .is_some_and(|(actual, floor)| actual < floor)
                || cut.epoch != base.epoch
                || cut.recovery_generation != base.recovery_generation
                || row.epoch != cut.epoch.to_string()
                || row.recovery != cut.recovery_generation.to_string()
                || row.revision != cut.projection_revision.map(|v| v.to_string())
                || row.source.parse::<u64>().map_err(|_| Error::Unavailable)? > cut.complete_through
                || row.entered_at < cut.visibility_floor_ms
            {
                return Err(Error::Unavailable);
            }
            entries.push(Entry {
                login_event_id: row.login_event_id,
                character: row.character_id,
                entered_at: row.entered_at,
                left_at: row.left_at,
                end_reason: row.end_reason,
                stale: now - base.observed_at_ms > 10_000 || now - cut.observed_at_ms > 10_000,
            });
        }
        tx.commit().await?;
        Ok(entries)
    }
    /// Called by the owning T72 privacy/removal worker in its transaction.
    pub async fn remove(
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        occurrence: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM pd_game_logins WHERE account_id=$1 AND login_event_id=$2")
            .bind(account)
            .bind(occurrence)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
    /// Run in the owning retention worker; reads also enforce the exact 30-day cap.
    pub async fn purge_expired(&self) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM pd_game_logins WHERE entered_at < (extract(epoch FROM clock_timestamp())*1000)::bigint - 2592000000").execute(&self.pool).await?.rows_affected())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("private login history operation refused")
    }
}
impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Retention(pub PgPool);
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            sqlx::query("DELETE FROM pd_game_logins WHERE entered_at < (extract(epoch FROM clock_timestamp())*1000)::bigint - 2592000000").execute(&self.0).await.map_err(|_| "login retention unavailable".to_owned())?;
            Ok(())
        })
    }
}
