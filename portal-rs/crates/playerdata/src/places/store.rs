use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Row};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub aggregate: bool,
    pub ingest: bool,
}
#[derive(Debug)]
pub enum Error {
    Disabled,
    Denied,
    Invalid,
    Conflict,
    Database(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Normalized T73 evidence, not a proposed replacement for the Game wire.
/// Only single-place, solo, complete durable-logout sessions may contribute.
/// All segments must resolve to the same catalogue, place, level band and vocation.
/// Profit uses one reviewed common NPC valuation, never personal price overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub source_authority: String,
    pub session_id: Uuid,
    pub account_id: Uuid,
    pub character_id: Uuid,
    pub world_id: Uuid,
    pub character_revision: i64,
    pub projection_revision: i64,
    pub projection_epoch: i64,
    pub recovery_generation: i64,
    pub stream_generation: Uuid,
    pub catalog_revision: String,
    pub price_revision: String,
    pub place_key: String,
    pub vocation_key: String,
    pub level: u32,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_seconds: u32,
    pub experience: u64,
    pub profit: i64,
}

/// Trusted Platform adapter; never implemented by a browser or wire deserializer.
/// `verify` must lock/read current evidence using this same transaction connection:
/// all segment indices and sealed drops reconciled; contiguous applied watermark;
/// durable logout (not merely a final segment); current restore generation;
/// authoritative transfer-receipt cut; account binding and retention; homogeneous
/// solo place/level-band/vocation; accepted catalogue and common NPC prices.
/// Return false for missing, ambiguous, stale, transferred or erased evidence.
/// Reads recheck every contribution, so invalidation cannot revive a cached cohort.
pub trait JournalSource: Send + Sync + std::fmt::Debug {
    /// True only after producer and privacy qualification, not merely configuration.
    fn ready(&self) -> bool {
        false
    }
    fn verify<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        session: &'a Session,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + 'a>>;
}
#[derive(Debug)]
pub struct UnresolvedSource;
impl JournalSource for UnresolvedSource {
    fn verify<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a Session,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + 'a>> {
        Box::pin(async { Ok(false) })
    }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
impl Session {
    fn validate(&self, now: i64) -> Result<(), Error> {
        if [
            self.session_id,
            self.account_id,
            self.character_id,
            self.world_id,
            self.stream_generation,
        ]
        .iter()
        .any(|id| id.get_version_num() != 7)
            || [
                self.character_revision,
                self.projection_revision,
                self.projection_epoch,
                self.recovery_generation,
            ]
            .iter()
            .any(|r| *r <= 0)
            || [
                &self.source_authority,
                &self.catalog_revision,
                &self.price_revision,
                &self.place_key,
                &self.vocation_key,
            ]
            .iter()
            .any(|s| !token(s))
            || !self.place_key.starts_with("place:")
            || !self.vocation_key.starts_with("vocation:")
            || self.level == 0
            || self.level > 100_000
            || self.started_at <= 0
            || self.ended_at <= self.started_at
            || self.ended_at > now
            || self.ended_at <= now - 90 * 86400
            || self.duration_seconds == 0
            || i64::from(self.duration_seconds) > self.ended_at - self.started_at
            || self.experience > 999_999_999_999
            || self.profit.unsigned_abs() > 999_999_999_999
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

/// No account/character/session IDs or rare occurrences leave the aggregation boundary.
/// Cohorts are disjoint; no arbitrary filters, totals, rolling windows or drilldowns.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Cohort {
    pub utc_day: String,
    pub world_id: Uuid,
    pub catalog_revision: String,
    pub price_revision: String,
    pub place_key: String,
    pub vocation_key: String,
    pub level_min: u32,
    pub level_max: u32,
    pub sessions: usize,
    pub experience_per_hour: String,
    pub profit_per_hour: String,
}
#[derive(Debug, Clone)]
pub struct PlacesStore {
    pool: PgPool,
    flags: Flags,
    source: Arc<dyn JournalSource>,
}
impl PlacesStore {
    pub fn new(pool: PgPool, flags: Flags, source: Arc<dyn JournalSource>) -> Self {
        Self {
            pool,
            flags,
            source,
        }
    }
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    pub(super) fn enabled(&self) -> Result<(), Error> {
        if self.flags.aggregate {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    async fn active(connection: &mut PgConnection, actor: Uuid, owner: Uuid) -> Result<(), Error> {
        if actor != owner {
            return Err(Error::Denied);
        }
        let active: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL)")
            .bind(actor).fetch_one(connection).await?;
        if !active {
            return Err(Error::Denied);
        }
        Ok(())
    }
    /// Actor comes from the server-side authenticated session, never a form field.
    pub async fn preference(&self, actor: Uuid, owner: Uuid) -> Result<bool, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor, owner).await?;
        let value = sqlx::query_scalar(
            "SELECT participating FROM pd_places_preferences WHERE account_id=$1",
        )
        .bind(owner)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(false);
        tx.commit().await?;
        Ok(value)
    }
    pub async fn set_preference(
        &self,
        actor: Uuid,
        owner: Uuid,
        participating: bool,
    ) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        // Serialize publication, contribution and privacy changes. Publication never
        // caches outputs: after this commit the next read recomputes its thresholds.
        sqlx::query("SELECT pg_advisory_xact_lock(81005230117)")
            .execute(&mut *tx)
            .await?;
        Self::active(&mut tx, actor, owner).await?;
        sqlx::query("INSERT INTO pd_places_preferences(account_id,participating) VALUES($1,$2) ON CONFLICT(account_id) DO UPDATE SET participating=$2, privacy_revision=pd_places_preferences.privacy_revision+1,updated_at=now()")
            .bind(owner).bind(participating).execute(&mut *tx).await?;
        // Every preference change starts a fresh contribution era. Re-enabling never
        // resurrects previous sessions, even when the ingest backlog is replayed.
        sqlx::query("INSERT INTO pd_places_replay_fences SELECT source_authority,session_id,ended_at+interval '120 days' FROM pd_places_sessions WHERE account_id=$1 ON CONFLICT DO NOTHING")
            .bind(owner).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_places_sessions WHERE account_id=$1")
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Platform-only normalized ingest. G-9 raw endpoints belong to T72 and remain off.
    pub async fn ingest(&self, session: &Session) -> Result<(), Error> {
        self.enabled()?;
        if !self.flags.ingest {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(81005230117)")
            .execute(&mut *tx)
            .await?;
        let now: i64 = sqlx::query_scalar("SELECT extract(epoch FROM now())::bigint")
            .fetch_one(&mut *tx)
            .await?;
        session.validate(now)?;
        Self::active(&mut tx, session.account_id, session.account_id).await?;
        let pref=sqlx::query("SELECT participating,privacy_revision,extract(epoch FROM updated_at)::bigint AS since FROM pd_places_preferences WHERE account_id=$1")
            .bind(session.account_id).fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        if !pref.try_get::<bool, _>("participating")?
            || session.started_at < pref.try_get::<i64, _>("since")?
        {
            return Err(Error::Denied);
        }
        let fenced: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_places_replay_fences WHERE source_authority=$1 AND session_id=$2)")
            .bind(&session.source_authority).bind(session.session_id).fetch_one(&mut *tx).await?;
        if fenced || !self.source.verify(&mut tx, session).await? {
            return Err(Error::Denied);
        }
        let value = serde_json::to_value(session).map_err(|_| Error::Invalid)?;
        let existing: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT snapshot FROM pd_places_sessions WHERE source_authority=$1 AND session_id=$2",
        )
        .bind(&session.source_authority)
        .bind(session.session_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing) = existing {
            if existing != value {
                return Err(Error::Conflict);
            }
        } else {
            sqlx::query("INSERT INTO pd_places_sessions(source_authority,session_id,account_id,privacy_revision,ended_at,snapshot) VALUES($1,$2,$3,$4,to_timestamp($5::double precision),$6)")
                .bind(&session.source_authority).bind(session.session_id).bind(session.account_id).bind(pref.try_get::<i64,_>("privacy_revision")?).bind(session.ended_at as f64).bind(value).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn daily(&self, actor: Uuid) -> Result<Vec<Cohort>, Error> {
        self.enabled()?;
        if !self.source.ready() {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(81005230117)")
            .execute(&mut *tx)
            .await?;
        Self::active(&mut tx, actor, actor).await?;
        // Conservative delay: a complete UTC day, then a further 24 hours.
        let now: i64 = sqlx::query_scalar("SELECT extract(epoch FROM now())::bigint")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM pd_places_sessions WHERE ended_at<=now()-interval '90 days'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM pd_places_replay_fences WHERE expires_at<=now()")
            .execute(&mut *tx)
            .await?;
        let rows: Vec<serde_json::Value>=sqlx::query_scalar("SELECT s.snapshot FROM pd_places_sessions s JOIN pd_places_preferences p USING(account_id) JOIN accounts a ON a.id=s.account_id WHERE p.participating AND p.privacy_revision=s.privacy_revision AND a.status='active' AND a.email_verified_at IS NOT NULL AND s.ended_at<=date_trunc('day', now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'-interval '24 hours'")
            .fetch_all(&mut *tx).await?;
        let mut sessions = Vec::new();
        for row in rows {
            let session: Session = serde_json::from_value(row).map_err(|_| Error::Denied)?;
            session.validate(now)?;
            if !self.source.verify(&mut tx, &session).await? {
                return Err(Error::Denied);
            }
            sessions.push(session);
        }
        let result = aggregate(&sessions, now);
        tx.commit().await?;
        Ok(result)
    }
}

type Key = (i64, Uuid, String, String, String, String, u32);
pub(super) fn aggregate(sessions: &[Session], now: i64) -> Vec<Cohort> {
    let mut groups: BTreeMap<Key, Vec<&Session>> = BTreeMap::new();
    // A journal session may contribute to exactly one cell, irrespective of replay.
    let mut seen = BTreeSet::new();
    for s in sessions {
        if s.validate(now).is_err() || !seen.insert(s.session_id) {
            continue;
        }
        let day = s.ended_at / 86400;
        if (day + 2) * 86400 > now {
            continue;
        }
        groups
            .entry((
                day,
                s.world_id,
                s.catalog_revision.clone(),
                s.price_revision.clone(),
                s.place_key.clone(),
                s.vocation_key.clone(),
                (s.level - 1) / 50 * 50 + 1,
            ))
            .or_default()
            .push(s);
    }
    groups
        .into_iter()
        .filter_map(
            |((day, world_id, catalog, price, place, vocation, level), rows)| {
                let accounts: BTreeSet<_> = rows.iter().map(|s| s.account_id).collect();
                if rows.len() < 10 || accounts.len() < 5 {
                    return None;
                }
                let exp: u128 = rows
                    .iter()
                    .map(|s| u128::from(s.experience) * 3600 / u128::from(s.duration_seconds))
                    .sum();
                let profit: i128 = rows
                    .iter()
                    .map(|s| i128::from(s.profit) * 3600 / i128::from(s.duration_seconds))
                    .sum();
                Some(Cohort {
                    utc_day: time::OffsetDateTime::from_unix_timestamp(day * 86400)
                        .ok()?
                        .date()
                        .to_string(),
                    world_id,
                    catalog_revision: catalog,
                    price_revision: price,
                    place_key: place,
                    vocation_key: vocation,
                    level_min: level,
                    level_max: level + 49,
                    sessions: rows.len(),
                    experience_per_hour: (exp / rows.len() as u128).to_string(),
                    profit_per_hour: (profit / rows.len() as i128).to_string(),
                })
            },
        )
        .collect()
}
