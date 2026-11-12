//! Durable private journal core. All authority adapters are fail closed by default.
//!
//! The private authority adapter supplies a current, purpose/account-partition-bound lease.
//! The lease must serialize with transfer, erasure and restore cutoffs until drop,
//! validate catalog/custody mappings, and deny pending or ambiguous transitions.
//! Journal rows and cached recipients are never ownership evidence. No adapter is
//! enabled by environment variables until those contracts have been accepted.
use oteryn_internal_api::{
    canonical::{self, Value},
    playerdata::{self, Record, Scope, Watermark},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
};

pub mod http;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Conflict,
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl From<playerdata::Error> for Error {
    fn from(e: playerdata::Error) -> Self {
        match e {
            playerdata::Error::Invalid => Self::Invalid,
            playerdata::Error::Unavailable => Self::Unavailable,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "private journal rejected: {self:?}")
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;
pub type LeaseFuture<'a> = Pin<Box<dyn Future<Output = Result<Box<dyn Lease>>> + Send + 'a>>;
/// Implementations must hold a fence shared with the authoritative cutoff writer.
/// A browser or an internal request must never construct an implementation.
pub trait Authority: Debug + Send + Sync {
    fn acquire<'a>(&'a self, scope: &'a Scope) -> LeaseFuture<'a>;
}
pub trait Lease: Debug + Send + Sync {
    /// Proves all catalog references, source-specific ordering, source coverage,
    /// house custody and initial ownership cut under the held authority fence.
    fn validate(&self, record: &Record) -> Result<()>;
}
#[derive(Debug)]
pub struct UnavailableAuthority;
impl Authority for UnavailableAuthority {
    fn acquire<'a>(&'a self, _: &'a Scope) -> LeaseFuture<'a> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
#[derive(Debug, Clone, Default)]
pub struct Flags {
    pub items_ingest: bool,
    pub progress_ingest: bool,
    pub items_read: bool,
    pub history_read: bool,
    pub progress_read: bool,
    pub login_read: bool,
    pub families: BTreeSet<String>,
}
#[derive(Debug, Clone, Copy)]
pub struct TrustedTime {
    pub unix_ms: u64,
    pub uncertainty_ms: u64,
}
impl TrustedTime {
    fn now(self, progress: bool) -> Result<u64> {
        if self.uncertainty_ms > if progress { 5000 } else { 30000 } {
            return Err(Error::Unavailable);
        };
        self.unix_ms
            .checked_add(self.uncertainty_ms)
            .filter(|x| *x <= i64::MAX as u64)
            .ok_or(Error::Unavailable)
    }
}
#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
    authority: Arc<dyn Authority>,
    pub flags: Flags,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub highest_seen: u64,
    pub stored: u64,
    pub applied: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
}
#[derive(Debug)]
pub struct View {
    pub freshness: Freshness,
    pub observed_ms: u64,
    pub coverage_start_ms: Option<u64>,
    pub through: u64,
    pub records: Vec<Value>,
}
/// Resolve this account from the authenticated session, never from query/body IDs.
#[derive(Debug, Clone)]
pub enum Actor {
    Owner(String),
    Anonymous,
    Staff,
}
fn enabled(flags: &Flags, scope: &Scope, ingest: bool) -> bool {
    flags.families.contains(&scope.family)
        && if playerdata::PROGRESS_FAMILIES
            .split_whitespace()
            .any(|x| x == scope.family)
        {
            if ingest {
                flags.progress_ingest
            } else {
                flags.progress_read && (scope.family != "game_login" || flags.login_read)
            }
        } else if ingest {
            flags.items_ingest
        } else if scope.family == "ITEMS" {
            flags.items_read
        } else {
            flags.history_read
        }
}
fn tuple(parts: Vec<Option<&str>>) -> String {
    parts
        .into_iter()
        .map(|part| match part {
            Some(value) => format!("{}:{value}", value.len()),
            None => "-".to_owned(),
        })
        .collect::<Vec<_>>()
        .join("|")
}
/// G-11 source ordering is account/family/epoch scoped; G-10 includes its full cut.
pub fn stream_key(s: &Scope) -> String {
    if playerdata::PROGRESS_FAMILIES
        .split_whitespace()
        .any(|x| x == s.family)
    {
        tuple(vec![
            Some(&s.authority),
            Some(&s.account),
            s.epoch.as_deref(),
            Some(&s.family),
        ])
    } else {
        tuple(vec![
            Some(&s.authority),
            Some(&s.account),
            s.world.as_deref(),
            s.character.as_deref(),
            s.character_revision.as_deref(),
            s.from_projection_revision.as_deref(),
            s.projection_revision.as_deref(),
            s.epoch.as_deref(),
            Some(&s.family),
            Some(&s.recovery),
        ])
    }
}
fn subject_key(s: &Scope) -> String {
    tuple(vec![
        s.character.as_deref(),
        s.world.as_deref(),
        s.projection_revision.as_deref(),
        s.epoch.as_deref(),
        Some(&s.recovery),
    ])
}
fn hash(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
/// Content-free deletion tombstones live 120 days (revision 3, G-10 and G-11 §8.2).
const TOMBSTONE_MS: u64 = 120 * 86400000;
/// SHA-256 of the canonical deleted-scope key: the account, or the account and
/// one of its characters. No identifier is stored in clear.
fn scope_digest(account: &str, character: Option<&str>) -> Vec<u8> {
    hash(tuple(vec![Some("deleted-scope"), Some(account), character]).as_bytes())
}
async fn tombstone(
    tx: &mut Transaction<'_, Postgres>,
    account: &str,
    character: Option<&str>,
    now: u64,
) -> Result<()> {
    sqlx::query("INSERT INTO pd_deleted_scopes(scope_digest,deleted_ms,expires_ms) VALUES($1,$2,$3) ON CONFLICT(scope_digest) DO UPDATE SET expires_ms=GREATEST(pd_deleted_scopes.expires_ms,EXCLUDED.expires_ms)")
        .bind(scope_digest(account, character))
        .bind(now as i64)
        .bind(now.saturating_add(TOMBSTONE_MS).min(i64::MAX as u64) as i64)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
/// The deleted scope covering a record, if any; the account wins over its character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Deleted {
    No,
    Account,
    Character,
}
/// Reason code of the one terminal refusal tied to subject state: a G-11 record
/// of a character whose scope is deleted (character deletion, opt-out or
/// transfer cut, G-11 §8.2).
const CHARACTER_SCOPE_DELETED: &str = "CHARACTER_SCOPE_DELETED";
/// Terminal, deterministic refusals tied to subject state (G-11 §8.2, coordinator
/// amendment of 2026-11-12). Only these consume their revision on the
/// account-wide G-11 stream; every other refusal fails closed without advance.
const TERMINAL_REFUSALS: [&str; 1] = [CHARACTER_SCOPE_DELETED];
/// A record inside a deleted scope is refused, an exact replay included.
async fn deleted(tx: &mut Transaction<'_, Postgres>, s: &Scope) -> Result<Deleted> {
    let account = scope_digest(&s.account, None);
    let mut digests = vec![account.clone()];
    if let Some(character) = s.character.as_deref() {
        digests.push(scope_digest(&s.account, Some(character)));
    }
    let found: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT scope_digest FROM pd_deleted_scopes WHERE scope_digest = ANY($1)",
    )
    .bind(digests)
    .fetch_all(&mut **tx)
    .await?;
    Ok(if found.contains(&account) {
        Deleted::Account
    } else if found.is_empty() {
        Deleted::No
    } else {
        Deleted::Character
    })
}
fn decimal_row(row: &sqlx::postgres::PgRow, name: &str) -> Result<u64> {
    row.try_get::<String, _>(name)?
        .parse()
        .map_err(|_| Error::Unavailable)
}
/// Serializes erasure with every delivery into one account, including the first
/// delivery of an authority the account has no stream for yet. Lock order is
/// global: this account lock first, then authority locks in ascending order, so
/// erasure and delivery cannot deadlock.
async fn lock_account(tx: &mut Transaction<'_, Postgres>, account: &str) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73))")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, authority: &str) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,72))")
        .bind(authority)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn stream(tx: &mut Transaction<'_, Postgres>, s: &Scope, progress: bool) -> Result<Cursor> {
    let key = stream_key(s);
    sqlx::query("INSERT INTO pd_streams(stream_key,authority,account_id,family,progress) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING").bind(&key).bind(&s.authority).bind(&s.account).bind(&s.family).bind(progress).execute(&mut **tx).await?;
    let row=sqlx::query("SELECT highest_seen::text,stored_through::text,applied_through::text,erased FROM pd_streams WHERE stream_key=$1 FOR UPDATE").bind(&key).fetch_one(&mut **tx).await?;
    if row.try_get::<bool, _>("erased")? {
        return Err(Error::Conflict);
    };
    Ok(Cursor {
        highest_seen: decimal_row(&row, "highest_seen")?,
        stored: decimal_row(&row, "stored_through")?,
        applied: decimal_row(&row, "applied_through")?,
    })
}
fn retention(r: &Record) -> u64 {
    let days = if !r.progress {
        30
    } else {
        match r.scope.family.as_str() {
            "game_login" => 30,
            "skill_history" | "death_recap" | "pvp" => 90,
            _ => 36500,
        }
    };
    days * 86400000
}
impl Store {
    pub fn new(pool: PgPool, authority: Arc<dyn Authority>, flags: Flags) -> Self {
        Self {
            pool,
            authority,
            flags,
        }
    }
    pub async fn ingest(&self, r: Record, time: TrustedTime) -> Result<Vec<u8>> {
        let r = if r.progress {
            playerdata::progress(&r.bytes)?
        } else {
            playerdata::items(&r.bytes)?
        };
        if !enabled(&self.flags, &r.scope, true) {
            return Err(Error::Unavailable);
        };
        let now = time.now(r.progress)?;
        if r.occurred_ms > now {
            return Err(Error::Invalid);
        };
        let lease = self.authority.acquire(&r.scope).await?;
        let mut tx = self.pool.begin().await?;
        lock_account(&mut tx, &r.scope.account).await?;
        lock(&mut tx, &r.scope.authority).await?;
        lease.validate(&r)?;
        // A G-11 stream is account-wide, so a record of a deleted character is a
        // terminal refusal that consumes its revision. A deleted account closes
        // the stream, and a G-10 stream with a character closes with it: both
        // fail closed without advance.
        let terminal = match deleted(&mut tx, &r.scope).await? {
            Deleted::No => None,
            Deleted::Character if r.progress => Some(CHARACTER_SCOPE_DELETED),
            Deleted::Character | Deleted::Account => return Err(Error::Conflict),
        };
        let key = stream_key(&r.scope);
        let c = stream(&mut tx, &r.scope, r.progress).await?;
        let digest = hash(&r.bytes);
        let existing=sqlx::query("SELECT stream_key,source_revision::text,digest,kind FROM pd_records WHERE (stream_key=$1 AND source_revision=$2::numeric) OR (authority=$3 AND recovery_generation=$4 AND record_id=$5)").bind(&key).bind(r.revision.to_string()).bind(&r.scope.authority).bind(if r.progress{""}else{&r.scope.recovery}).bind(&r.id).fetch_all(&mut *tx).await?;
        if !existing.is_empty() {
            if existing.len() != 1
                || existing[0].try_get::<String, _>("stream_key")? != key
                || decimal_row(&existing[0], "source_revision")? != r.revision
                || existing[0].try_get::<Vec<u8>, _>("digest")? != digest
            {
                sqlx::query("INSERT INTO pd_conflicts(reference,reason,expires_ms) VALUES($1,'IMMUTABLE_BINDING',$2) ON CONFLICT DO NOTHING")
                    .bind(&digest).bind(now.saturating_add(7*86400000) as i64).execute(&mut *tx).await?;
                tx.commit().await?;
                return Err(Error::Conflict);
            };
            let kind = existing[0].try_get::<String, _>("kind")?;
            tx.commit().await?;
            if let Some(reason) = TERMINAL_REFUSALS.iter().find(|reason| **reason == kind) {
                return Ok(refusal(&r, reason));
            };
            // A record accepted before its scope was deleted stays refused (§8.2).
            if terminal.is_some() {
                return Err(Error::Conflict);
            };
            return acknowledgement(&r);
        }
        // Strict contiguous delivery is permitted by G-11 and required by G-10. This
        // bounds staging and keeps a poisoned or absent revision from being skipped.
        if c.stored.checked_add(1) != Some(r.revision) {
            return Err(Error::Conflict);
        };
        if let Some(reason) = terminal {
            // Content-free refusal row: record id, revision, digest and reason code
            // only; no subject, entity, time or payload. Its empty scope key never
            // matches a subject read or supersession.
            sqlx::query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms) VALUES($1,$2::numeric,$3,'',$4,'',NULL,'',$5,0,0,$6,NULL,$7)")
                .bind(&key).bind(r.revision.to_string()).bind(&r.scope.authority).bind(&r.id).bind(reason).bind(&digest).bind(now as i64).execute(&mut *tx).await?;
            sqlx::query("UPDATE pd_streams SET highest_seen=$2::numeric,stored_through=$2::numeric,applied_through=$2::numeric,last_record_id=$3 WHERE stream_key=$1").bind(&key).bind(r.revision.to_string()).bind(&r.id).execute(&mut *tx).await?;
            tx.commit().await?;
            return Ok(refusal(&r, reason));
        }
        let expires = r
            .occurred_ms
            .saturating_add(retention(&r))
            .min(i64::MAX as u64);
        if expires <= now && !r.progress {
            return Err(Error::Conflict);
        };
        if !r.progress && r.scope.family != "ITEMS" {
            self.validate_history(&mut tx, &r, &key).await?;
        }
        if r.scope.family == "ITEMS" {
            self.validate_snapshot_step(&mut tx, &r, &key).await?;
        }
        let bytes = canonical::encode(&r.payload).map_err(|_| Error::Invalid)?;
        let payload = if expires <= now || r.kind == "REMOVE" {
            None
        } else {
            Some(bytes)
        };
        let expires = if r.scope.family == "ITEMS" && r.kind != "ITEMS_END" {
            expires.min(now.saturating_add(86400000))
        } else {
            expires
        };
        sqlx::query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms) VALUES($1,$2::numeric,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
   .bind(&key).bind(r.revision.to_string()).bind(&r.scope.authority).bind(if r.progress{""}else{&r.scope.recovery}).bind(&r.id).bind(&r.entity).bind(&r.scope.character).bind(subject_key(&r.scope)).bind(&r.kind).bind(r.occurred_ms as i64).bind(r.coverage_ms as i64).bind(digest).bind(payload).bind(expires as i64).execute(&mut *tx).await?;
        if r.scope.family == "ITEMS" && r.kind == "ITEMS_END" {
            self.finish_snapshot(&mut tx, &r, &key, now).await?;
        }
        // Superseded current-state payloads expire within seven days; occurrence
        // families retain their contract-bound history. REMOVE never revives a value.
        if r.progress
            && !matches!(
                r.scope.family.as_str(),
                "skill_history" | "death_recap" | "pvp"
            )
        {
            sqlx::query("UPDATE pd_records SET expires_ms=LEAST(expires_ms,$5) WHERE stream_key=$1 AND entity_key=$2 AND scope_key=$3 AND source_revision<$4::numeric").bind(&key).bind(&r.entity).bind(subject_key(&r.scope)).bind(r.revision.to_string()).bind(now.saturating_add(7*86400000) as i64).execute(&mut *tx).await?;
        }
        let applied = if r.scope.family == "ITEMS" && r.kind != "ITEMS_END" {
            c.applied
        } else {
            r.revision
        };
        sqlx::query("UPDATE pd_streams SET highest_seen=$2::numeric,stored_through=$2::numeric,applied_through=$3::numeric,last_record_id=$4 WHERE stream_key=$1").bind(&key).bind(r.revision.to_string()).bind(applied.to_string()).bind(&r.id).execute(&mut *tx).await?;
        tx.commit().await?;
        acknowledgement(&r)
    }
    pub async fn cursor(&self, s: &Scope) -> Result<(Cursor, Option<String>)> {
        if !enabled(&self.flags, s, true) {
            return Err(Error::Unavailable);
        };
        let _lease = self.authority.acquire(s).await?;
        let mut tx = self.pool.begin().await?;
        lock_account(&mut tx, &s.account).await?;
        lock(&mut tx, &s.authority).await?;
        // A G-11 stream is account-wide (its key has no character), so only an
        // account deletion closes it; a G-10 stream also closes with its character.
        let mut covered = s.clone();
        if playerdata::PROGRESS_FAMILIES
            .split_whitespace()
            .any(|x| x == s.family)
        {
            covered.character = None;
        }
        if deleted(&mut tx, &covered).await? != Deleted::No {
            return Err(Error::Conflict);
        };
        let key = stream_key(s);
        // Unknown streams cannot be bootstrapped by a cursor request.
        let row=sqlx::query("SELECT highest_seen::text,stored_through::text,applied_through::text,erased,last_record_id FROM pd_streams WHERE stream_key=$1").bind(&key).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
        if row.try_get::<bool, _>("erased")? {
            return Err(Error::Conflict);
        };
        let c = Cursor {
            highest_seen: decimal_row(&row, "highest_seen")?,
            stored: decimal_row(&row, "stored_through")?,
            applied: decimal_row(&row, "applied_through")?,
        };
        let id = row.try_get::<Option<String>, _>("last_record_id")?;
        tx.commit().await?;
        Ok((c, id))
    }
    pub async fn watermark(&self, w: Watermark, time: TrustedTime) -> Result<Vec<u8>> {
        let w = playerdata::watermark(&w.bytes)?;
        let scope = Scope {
            authority: w.authority.clone(),
            account: w.account.clone(),
            world: None,
            character: None,
            character_revision: None,
            projection_revision: None,
            from_projection_revision: None,
            epoch: Some(w.epoch.clone()),
            recovery: w.recovery.clone(),
            family: w.family.clone(),
        };
        if !enabled(&self.flags, &scope, true) {
            return Err(Error::Unavailable);
        };
        let now = time.now(true)?;
        if w.observed_ms > now {
            return Err(Error::Invalid);
        };
        let _lease = self.authority.acquire(&scope).await?;
        let mut tx = self.pool.begin().await?;
        lock_account(&mut tx, &scope.account).await?;
        lock(&mut tx, &scope.authority).await?;
        // Checked before `stream` so a deleted account never gets a new stream row.
        if deleted(&mut tx, &scope).await? != Deleted::No {
            return Err(Error::Conflict);
        };
        let c = stream(&mut tx, &scope, true).await?;
        let key = stream_key(&scope);
        let row=sqlx::query("SELECT observed_ms,published_through::text,baseline_complete,watermark_digest FROM pd_streams WHERE stream_key=$1").bind(&key).fetch_one(&mut *tx).await?;
        let previous = row.try_get::<i64, _>("observed_ms")? as u64;
        let previous_baseline = row.try_get::<bool, _>("baseline_complete")?;
        // Persisted stored/applied cursor can exceed the published scan prefix.
        let previous_bytes = row.try_get::<Option<Vec<u8>>, _>("watermark_digest")?;
        let digest = hash(&w.bytes);
        if w.through > c.applied
            || w.through < decimal_row(&row, "published_through")?
            || w.observed_ms < previous
            || (previous_baseline && !w.baseline)
        {
            return Err(Error::Conflict);
        };
        if previous_bytes.as_ref() != Some(&digest) {
            sqlx::query("UPDATE pd_streams SET observed_ms=$2,baseline_complete=$3,watermark_digest=$4,published_through=$5::numeric WHERE stream_key=$1").bind(&key).bind(w.observed_ms as i64).bind(w.baseline).bind(digest).bind(w.through.to_string()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(format!(
            "{{\"contract_version\":1,\"result\":\"accepted\",\"complete_through\":\"{}\"}}",
            w.through
        )
        .into_bytes())
    }
    pub async fn read(&self, actor: &Actor, s: &Scope, time: TrustedTime) -> Result<View> {
        if !enabled(&self.flags, s, false) {
            return Err(Error::Unavailable);
        };
        if !matches!(actor,Actor::Owner(account) if account==&s.account) {
            return Err(Error::Denied);
        };
        let progress = playerdata::PROGRESS_FAMILIES
            .split_whitespace()
            .any(|x| x == s.family);
        let now = time.now(progress)?;
        let _lease = self.authority.acquire(s).await?;
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, &s.authority).await?;
        let key = stream_key(s);
        let row=sqlx::query("SELECT stored_through::text,published_through::text,observed_ms,baseline_complete,erased FROM pd_streams WHERE stream_key=$1").bind(&key).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
        if row.try_get::<bool, _>("erased")? {
            return Err(Error::Unavailable);
        };
        let mut through = decimal_row(&row, "stored_through")?;
        let mut observed = row.try_get::<i64, _>("observed_ms")? as u64;
        let mut coverage_start_ms = None;
        let rows = if progress {
            if !row.try_get::<bool, _>("baseline_complete")? {
                return Err(Error::Unavailable);
            };
            through = decimal_row(&row, "published_through")?;
            if observed > now || now.saturating_sub(observed) > 86400000 {
                return Err(Error::Unavailable);
            };
            // Only the latest entity inside the proven prefix can be presented. A later
            // REMOVE or expired value cannot reveal an earlier retained replacement.
            sqlx::query("SELECT DISTINCT ON (entity_key) kind,payload,expires_ms FROM pd_records WHERE stream_key=$1 AND scope_key=$2 AND source_revision<=$3::numeric ORDER BY entity_key,source_revision DESC")
   .bind(&key).bind(subject_key(s)).bind(through.to_string()).fetch_all(&mut *tx).await?
        } else if s.family == "ITEMS" {
            let head=sqlx::query("SELECT begin_revision::text,end_revision::text,occurred_ms FROM pd_item_heads WHERE stream_key=$1").bind(&key).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
            observed = head.try_get::<i64, _>("occurred_ms")? as u64;
            through = decimal_row(&head, "end_revision")?;
            coverage_start_ms = Some(observed);
            if now.saturating_sub(observed) >= 30 * 86400000 {
                return Err(Error::Unavailable);
            };
            sqlx::query("SELECT kind,payload,expires_ms FROM pd_records WHERE stream_key=$1 AND source_revision BETWEEN $2::numeric AND $3::numeric ORDER BY source_revision")
   .bind(&key).bind(head.try_get::<String,_>("begin_revision")?).bind(head.try_get::<String,_>("end_revision")?).fetch_all(&mut *tx).await?
        } else {
            let coverage = sqlx::query("SELECT min(coverage_ms) AS start,max(occurred_ms) AS observed FROM pd_records WHERE stream_key=$1 AND scope_key=$2").bind(&key).bind(subject_key(s)).fetch_one(&mut *tx).await?;
            coverage_start_ms = coverage
                .try_get::<Option<i64>, _>("start")?
                .map(|v| v as u64);
            observed = coverage
                .try_get::<Option<i64>, _>("observed")?
                .ok_or(Error::Unavailable)? as u64;
            // History completeness and the initial ledger boundary must be independently
            // proven by the authority lease; an empty query alone proves nothing.
            sqlx::query("SELECT kind,payload,expires_ms FROM pd_records WHERE stream_key=$1 AND scope_key=$2 ORDER BY source_revision").bind(&key).bind(subject_key(s)).fetch_all(&mut *tx).await?
        };
        let mut records = Vec::new();
        for row in rows {
            let kind = row.try_get::<String, _>("kind")?;
            if kind == "REMOVE" {
                continue;
            };
            let expires = row.try_get::<i64, _>("expires_ms")? as u64;
            let payload = row.try_get::<Option<Vec<u8>>, _>("payload")?;
            if expires <= now || payload.is_none() {
                if !progress
                    || !matches!(
                        s.family.as_str(),
                        "skill_history" | "death_recap" | "pvp" | "game_login"
                    )
                {
                    return Err(Error::Unavailable);
                };
                continue;
            };
            if let Some(bytes) = payload {
                records.push(canonical::decode(&bytes, 262144, 9).map_err(|_| Error::Unavailable)?);
            }
        }
        tx.commit().await?;
        Ok(View {
            freshness: if progress && now.saturating_sub(observed) > 10000 {
                Freshness::Stale
            } else {
                Freshness::Fresh
            },
            observed_ms: observed,
            coverage_start_ms,
            through,
            records,
        })
    }
    /// The trusted transfer/character-erasure writer calls this while holding its
    /// exclusive authority fence. Other characters in the account keep their feed.
    pub async fn erase_character(
        &self,
        account: &str,
        character: &str,
        time: TrustedTime,
    ) -> Result<()> {
        let now = time.now(true)?;
        let mut tx = self.pool.begin().await?;
        lock_account(&mut tx, account).await?;
        let authorities = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT authority FROM pd_streams WHERE account_id=$1 ORDER BY authority",
        )
        .bind(account)
        .fetch_all(&mut *tx)
        .await?;
        for authority in authorities {
            lock(&mut tx, &authority).await?;
        }
        sqlx::query("DELETE FROM pd_item_heads WHERE stream_key IN (SELECT r.stream_key FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE s.account_id=$1 AND r.character_id=$2)").bind(account).bind(character).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_records SET payload=NULL,expires_ms=LEAST(expires_ms,$3) WHERE character_id=$2 AND stream_key IN (SELECT stream_key FROM pd_streams WHERE account_id=$1)").bind(account).bind(character).bind(now as i64).execute(&mut *tx).await?;
        tombstone(&mut tx, account, Some(character), now).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Account lifecycle adapter calls this under its account erasure fence. Digest
    /// and sequence metadata remain to refuse resurrection by queued replay.
    pub async fn erase_account(&self, account: &str, time: TrustedTime) -> Result<()> {
        let now = time.now(true)?;
        let mut tx = self.pool.begin().await?;
        lock_account(&mut tx, account).await?;
        // Lock every authority before updating streams in the same order as ingest.
        let authorities = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT authority FROM pd_streams WHERE account_id=$1 ORDER BY authority",
        )
        .bind(account)
        .fetch_all(&mut *tx)
        .await?;
        for authority in authorities {
            lock(&mut tx, &authority).await?;
        }
        sqlx::query(
            "UPDATE pd_streams SET erased=true,baseline_complete=false,erased_ms=COALESCE(erased_ms,$2) WHERE account_id=$1",
        )
        .bind(account)
        .bind(now as i64)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE pd_records SET payload=NULL WHERE stream_key IN (SELECT stream_key FROM pd_streams WHERE account_id=$1)").bind(account).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_item_heads WHERE stream_key IN (SELECT stream_key FROM pd_streams WHERE account_id=$1)").bind(account).execute(&mut *tx).await?;
        tombstone(&mut tx, account, None, now).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Expiration never changes digest, revision, baseline or ownership fences.
    pub async fn expire(&self, time: TrustedTime) -> Result<u64> {
        let now = time.now(true)?;
        let mut tx = self.pool.begin().await?;
        let authorities = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT authority FROM pd_streams ORDER BY authority",
        )
        .fetch_all(&mut *tx)
        .await?;
        for authority in authorities {
            lock(&mut tx, &authority).await?;
        }
        let changed = sqlx::query(
            "UPDATE pd_records SET payload=NULL WHERE payload IS NOT NULL AND expires_ms<=$1",
        )
        .bind(now as i64)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        // Prune only a proven prefix. The durable floor refuses older replay
        // after its exact digest has aged out, rather than resurrecting history.
        sqlx::query("UPDATE pd_streams s SET retention_floor=GREATEST(retention_floor,COALESCE((SELECT min(r.source_revision)-1 FROM pd_records r WHERE r.stream_key=s.stream_key AND (r.payload IS NOT NULL OR r.expires_ms>$1)),stored_through)) WHERE progress=false")
            .bind(now.saturating_sub(31*86400000) as i64).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_streams SET retention_floor=stored_through WHERE progress=true AND erased=true AND erased_ms<=$1")
            .bind(now.saturating_sub(90*86400000) as i64).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_records r USING pd_streams s WHERE r.stream_key=s.stream_key AND r.source_revision<=s.retention_floor").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_deleted_scopes WHERE expires_ms<=$1")
            .bind(now as i64)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM pd_conflicts WHERE expires_ms<=$1")
            .bind(now as i64)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(changed)
    }
    async fn validate_history(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        r: &Record,
        key: &str,
    ) -> Result<()> {
        let rows=sqlx::query("SELECT entity_key,coverage_ms,payload FROM pd_records WHERE stream_key=$1 ORDER BY source_revision DESC LIMIT 1").bind(key).fetch_all(&mut **tx).await?;
        if let Some(last) = rows.first() {
            if last.try_get::<i64, _>("coverage_ms")? != r.coverage_ms as i64 {
                return Err(Error::Conflict);
            };
            if r.scope.family == "BANK" {
                let prior = last
                    .try_get::<Option<Vec<u8>>, _>("payload")?
                    .ok_or(Error::Unavailable)?;
                let prior = canonical::decode(&prior, 65536, 9).map_err(|_| Error::Unavailable)?;
                let previous = playerdata::get(&r.payload, "previous_entry_id")?;
                if playerdata::text(previous)? != last.try_get::<String, _>("entity_key")?
                    || playerdata::get(&prior, "balance_after")?
                        != playerdata::get(&r.payload, "balance_before")?
                {
                    return Err(Error::Conflict);
                };
            }
        }
        let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_records WHERE authority=$1 AND entity_key=$2 AND stream_key IN (SELECT stream_key FROM pd_streams WHERE account_id=$3 AND family=$4))").bind(&r.scope.authority).bind(&r.entity).bind(&r.scope.account).bind(&r.scope.family).fetch_one(&mut **tx).await?;
        if duplicate {
            return Err(Error::Conflict);
        };
        Ok(())
    }
    async fn validate_snapshot_step(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        r: &Record,
        key: &str,
    ) -> Result<()> {
        let last=sqlx::query("SELECT entity_key,kind,occurred_ms,payload,source_revision::text FROM pd_records WHERE stream_key=$1 ORDER BY source_revision DESC LIMIT 1").bind(key).fetch_optional(&mut **tx).await?;
        if r.kind == "ITEMS_BEGIN" {
            if last.is_some_and(|row| {
                row.try_get::<String, _>("kind")
                    .is_ok_and(|k| k != "ITEMS_END")
            }) {
                return Err(Error::Conflict);
            };
            return Ok(());
        }
        let last = last.ok_or(Error::Conflict)?;
        if last.try_get::<String, _>("entity_key")? != r.entity
            || last.try_get::<String, _>("kind")? == "ITEMS_END"
            || last.try_get::<i64, _>("occurred_ms")? != r.occurred_ms as i64
        {
            return Err(Error::Conflict);
        };
        let bytes = last
            .try_get::<Option<Vec<u8>>, _>("payload")?
            .ok_or(Error::Conflict)?;
        let p = canonical::decode(&bytes, 65536, 9).map_err(|_| Error::Unavailable)?;
        if r.kind == "ITEMS_PAGE" {
            let expected = if last.try_get::<String, _>("kind")? == "ITEMS_BEGIN" {
                0
            } else {
                json_u64(&p, "page_index")?
                    .checked_add(1)
                    .ok_or(Error::Conflict)?
            };
            if json_u64(&r.payload, "page_index")? != expected {
                return Err(Error::Conflict);
            };
        }
        Ok(())
    }
    async fn finish_snapshot(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        r: &Record,
        key: &str,
        now: u64,
    ) -> Result<()> {
        let rows=sqlx::query("SELECT source_revision::text,kind,payload,expires_ms FROM pd_records WHERE stream_key=$1 AND entity_key=$2 ORDER BY source_revision").bind(key).bind(&r.entity).fetch_all(&mut **tx).await?;
        let first = rows.first().ok_or(Error::Conflict)?;
        if first.try_get::<String, _>("kind")? != "ITEMS_BEGIN"
            || first.try_get::<i64, _>("expires_ms")? <= now as i64
        {
            return Err(Error::Conflict);
        };
        let first_bytes = first
            .try_get::<Option<Vec<u8>>, _>("payload")?
            .ok_or(Error::Conflict)?;
        let begin = canonical::decode(&first_bytes, 65536, 9).map_err(|_| Error::Unavailable)?;
        let pages = json_u64(&begin, "page_count")?;
        let count = json_u64(&begin, "item_count")?;
        if pages != json_u64(&r.payload, "page_count")?
            || count != json_u64(&r.payload, "item_count")?
            || rows.len() as u64 != pages + 2
        {
            return Err(Error::Conflict);
        };
        let mut digest = Sha256::new();
        let mut items = Vec::new();
        for row in rows.iter().skip(1).take(pages as usize) {
            if row.try_get::<String, _>("kind")? != "ITEMS_PAGE"
                || row.try_get::<i64, _>("expires_ms")? <= now as i64
            {
                return Err(Error::Conflict);
            };
            let bytes = row
                .try_get::<Option<Vec<u8>>, _>("payload")?
                .ok_or(Error::Conflict)?;
            digest.update(&bytes);
            let page = canonical::decode(&bytes, 65536, 9).map_err(|_| Error::Unavailable)?;
            let Value::Array(a) = playerdata::get(&page, "items")? else {
                return Err(Error::Conflict);
            };
            items.extend(a.clone());
        }
        let actual = format!("{:x}", digest.finalize());
        if playerdata::text(playerdata::get(&r.payload, "pages_sha256")?)? != actual
            || items.len() as u64 != count
        {
            return Err(Error::Conflict);
        };
        validate_tree(&begin, &items)?;
        let begin_revision = first.try_get::<String, _>("source_revision")?;
        // Only the complete new cut remains available. Old snapshot metadata remains
        // immutable for deduplication; no previous owner content is rebound.
        sqlx::query("UPDATE pd_records SET payload=NULL WHERE stream_key=$1 AND entity_key<>$2")
            .bind(key)
            .bind(&r.entity)
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE pd_records SET expires_ms=$3 WHERE stream_key=$1 AND entity_key=$2")
            .bind(key)
            .bind(&r.entity)
            .bind(r.occurred_ms.saturating_add(30 * 86400000) as i64)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO pd_item_heads(stream_key,snapshot_id,begin_revision,end_revision,occurred_ms) VALUES($1,$2,$3::numeric,$4::numeric,$5) ON CONFLICT(stream_key) DO UPDATE SET snapshot_id=EXCLUDED.snapshot_id,begin_revision=EXCLUDED.begin_revision,end_revision=EXCLUDED.end_revision,occurred_ms=EXCLUDED.occurred_ms").bind(key).bind(&r.entity).bind(begin_revision).bind(r.revision.to_string()).bind(r.occurred_ms as i64).execute(&mut **tx).await?;
        Ok(())
    }
}
fn json_u64(v: &Value, k: &str) -> Result<u64> {
    match playerdata::get(v, k)? {
        Value::Unsigned(n) => Ok(*n),
        Value::Signed(n) if *n >= 0 => Ok(*n as u64),
        _ => Err(Error::Invalid),
    }
}
fn validate_tree(begin: &Value, items: &[Value]) -> Result<()> {
    let mut ids = HashMap::new();
    let mut siblings = HashSet::new();
    let mut previous = "";
    for item in items {
        let id = playerdata::text(playerdata::get(item, "item_instance_id")?)?;
        if id <= previous {
            return Err(Error::Conflict);
        };
        previous = id;
        ids.insert(id, item);
        let root = playerdata::text(playerdata::get(item, "root")?)?;
        if root == "DEPOT" {
            let Value::Array(a) = playerdata::get(begin, "depots")? else {
                return Err(Error::Conflict);
            };
            if !a.contains(playerdata::get(item, "town_id")?) {
                return Err(Error::Conflict);
            }
        }
        if root == "HOUSE" {
            let Value::Array(a) = playerdata::get(begin, "houses")? else {
                return Err(Error::Conflict);
            };
            if !a.contains(playerdata::get(item, "house_id")?) {
                return Err(Error::Conflict);
            }
        }
        let coordinates = [
            "root",
            "town_id",
            "depot_box",
            "house_id",
            "spatial_position_hex",
            "equipment_slot",
            "parent_item_instance_id",
            "placement_ordinal",
        ]
        .iter()
        .map(|k| playerdata::get(item, k).cloned())
        .collect::<std::result::Result<Vec<_>, _>>()?;
        let encoded = canonical::encode(&Value::Array(coordinates)).map_err(|_| Error::Invalid)?;
        if !siblings.insert(encoded) {
            return Err(Error::Conflict);
        };
    }
    for item in items {
        let mut path = HashSet::new();
        let mut current = item;
        loop {
            let id = playerdata::text(playerdata::get(current, "item_instance_id")?)?;
            if !path.insert(id) || path.len() > 32 {
                return Err(Error::Conflict);
            };
            let parent = playerdata::get(current, "parent_item_instance_id")?;
            if matches!(parent, Value::Null) {
                break;
            };
            let parent = ids.get(playerdata::text(parent)?).ok_or(Error::Conflict)?;
            for field in [
                "root",
                "town_id",
                "depot_box",
                "house_id",
                "spatial_position_hex",
                "equipment_slot",
            ] {
                if playerdata::get(item, field)? != playerdata::get(parent, field)? {
                    return Err(Error::Conflict);
                }
            }
            current = parent;
        }
    }
    Ok(())
}
fn acknowledgement(r: &Record) -> Result<Vec<u8>> {
    Ok(if r.progress {
        format!("{{\"contract_version\":1,\"result\":\"accepted\",\"record_id\":\"{}\",\"accepted_source_revision\":\"{}\"}}",r.id,r.revision).into_bytes()
    } else {
        b"{\"contract_version\":1,\"result\":\"accepted\"}".to_vec()
    })
}
/// G-11 terminal refusal answer: the consumed revision and its reason code. An
/// exact replay returns the same bytes.
fn refusal(r: &Record, reason: &str) -> Vec<u8> {
    format!("{{\"contract_version\":1,\"result\":\"refused\",\"record_id\":\"{}\",\"consumed_source_revision\":\"{}\",\"reason\":\"{reason}\"}}",r.id,r.revision).into_bytes()
}
