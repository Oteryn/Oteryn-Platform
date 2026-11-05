//! Owner-scoped durations. G-8 does not permit public online-time rankings.
use crate::online::{self, Config, CoveragePolicy, Error, Family};
use sqlx::{PgPool, Row};
use std::{collections::BTreeMap, future::Future, pin::Pin};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedCharacter {
    pub character: Uuid,
    /// Authoritatively proven start of the current ownership interval.
    pub owned_since: i64,
}
pub type OwnerFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<OwnedCharacter>, Error>> + Send + 'a>>;
/// Freshly prove ownership, active profiles and independent privacy floors on
/// every read. Unknown/stale state denies; staff authority is insufficient.
pub trait OwnerPolicy: Send + Sync {
    fn characters(&self, account: Uuid, now: i64) -> OwnerFuture<'_>;
}
#[derive(Debug, PartialEq, Eq)]
pub struct OnlineEntry {
    pub character: Uuid,
    pub seconds: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct OnlineWindow {
    /// Inclusive UTC Unix second.
    pub start: i64,
    /// Exclusive UTC Unix second; never inferred from an open interval.
    pub end: i64,
    pub now: i64,
}
/// Private service only: use authenticated accounts and private/no-store responses.
/// No public page or cache may expose the returned ordering or durations.
pub async fn owner_online_ranking(
    db: &PgPool,
    config: &Config,
    account: Uuid,
    window: OnlineWindow,
    ownership: &impl OwnerPolicy,
    coverage: &impl CoveragePolicy,
) -> Result<Vec<OnlineEntry>, Error> {
    if !config.owner_rankings_enabled || !config.permits(Family::CharacterOnlineSessions) {
        return Err(Error::Unavailable);
    }
    let OnlineWindow { start, end, now } = window;
    if account.is_nil()
        || now <= 0
        || start <= 0
        || start >= end
        || end > now
        || start < now.saturating_sub(30 * 86400)
    {
        return Err(Error::Invalid);
    }
    let original = ownership.characters(account, now).await?;
    if original.len() > 1000 {
        return Err(Error::Unavailable);
    }
    let mut allowed = BTreeMap::new();
    for c in &original {
        if c.character.is_nil()
            || c.owned_since <= 0
            || c.owned_since > start
            || allowed.insert(c.character, c.owned_since).is_some()
        {
            return Err(Error::Unauthorized);
        }
    }
    let mut tx = db.begin().await?;
    let cut = online::storage::fresh(
        &mut tx,
        config,
        Family::CharacterOnlineSessions,
        now,
        coverage,
    )
    .await?;
    if start < cut.start || end > cut.through {
        return Err(Error::Unavailable);
    }
    let ids: Vec<String> = allowed.keys().map(Uuid::to_string).collect();
    let rows = sqlx::query("SELECT payload,open_unresolved,erased,observed_at FROM stats_v2_subjects WHERE source_authority=$1 AND family='character_online_sessions' AND epoch=$2::numeric AND split_part(subject,'/',1)=ANY($3) AND (expires_at>$4 OR open_unresolved) LIMIT 100001")
        .bind(&config.source_authority).bind(&cut.epoch).bind(&ids).bind(now).fetch_all(&mut *tx).await?;
    if rows.len() > 100_000 {
        return Err(Error::Unavailable);
    }
    let mut totals: BTreeMap<Uuid, u64> = allowed.keys().map(|id| (*id, 0)).collect();
    for row in rows {
        if row.try_get::<bool, _>("erased")? || row.try_get::<i64, _>("observed_at")? > cut.through
        {
            return Err(Error::Unavailable);
        }
        let payload: Option<serde_json::Value> = row.try_get("payload")?;
        let Some(payload) = payload else {
            continue;
        };
        let id = payload
            .get("character_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(Error::Unavailable)?;
        let sessions = payload
            .get("sessions")
            .and_then(|v| v.as_array())
            .ok_or(Error::Unavailable)?;
        if sessions.is_empty() {
            return Err(Error::Unavailable);
        }
        for session in sessions {
            let started = session
                .get("started_at")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<i64>().ok())
                .ok_or(Error::Unavailable)?;
            let ends = session
                .get("ended_at")
                .and_then(|v| v.as_array())
                .ok_or(Error::Unavailable)?;
            if ends.is_empty() {
                if started < end {
                    return Err(Error::Unavailable);
                }
                continue;
            }
            let ended = ends
                .first()
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<i64>().ok())
                .ok_or(Error::Unavailable)?;
            if ended <= start || started >= end {
                continue;
            }
            if started < *allowed.get(&id).ok_or(Error::Unauthorized)? {
                return Err(Error::Unauthorized);
            }
            let seconds = u64::try_from(ended.min(end) - started.max(start))
                .map_err(|_| Error::Unavailable)?;
            let total = totals.get_mut(&id).ok_or(Error::Unauthorized)?;
            *total = total.checked_add(seconds).ok_or(Error::Unavailable)?;
        }
    }
    if ownership.characters(account, now).await? != original
        || coverage.coverage(Family::CharacterOnlineSessions)? != cut
    {
        return Err(Error::Unauthorized);
    }
    tx.commit().await?;
    let mut result: Vec<_> = totals
        .into_iter()
        .map(|(character, seconds)| OnlineEntry { character, seconds })
        .collect();
    result.sort_by(|a, b| {
        b.seconds
            .cmp(&a.seconds)
            .then_with(|| a.character.cmp(&b.character))
    });
    Ok(result)
}
/// An enable flag cannot grant disclosure of owner-only source data.
pub fn public_online_ranking() -> Result<Vec<OnlineEntry>, Error> {
    Err(Error::Unavailable)
}
