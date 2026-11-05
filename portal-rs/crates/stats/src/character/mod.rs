//! Bounded character observations. No account facts or inferred online intervals.
use crate::store::{self, Daily, History, Permissions, Visibility};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use time::{Date, Duration};
use uuid::Uuid;

pub mod http;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Invalid,
    NotFound,
    Unavailable,
}
#[derive(Debug, Clone)]
pub struct PublicCharacter {
    pub name: String,
    pub permissions: Permissions,
    /// Approved public rank observations, ordered by (day, rank, stable id).
    pub ranks: Option<Vec<(Date, u32)>>,
}
pub type CurrentFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PublicCharacter, Error>> + Send + 'a>>;
/// Resolve current eligible profile and T45 privacy on every read. Missing subjects
/// are NotFound; stale epochs, watermarks or unknown privacy are Unavailable.
/// Rank history requires retained authoritative rank evidence, never current ranks
/// retroactively applied to older days. No owner-only progress enters this port.
pub trait Current: Send + Sync {
    fn character(&self, id: Uuid) -> CurrentFuture<'_>;
    /// Accepted world-specific XP threshold. None means no approved formula.
    fn experience_at_level(&self, _id: Uuid, _level: u32) -> Option<u64> {
        None
    }
}
#[derive(Clone)]
pub struct Characters {
    pub db: PgPool,
    pub enabled: bool,
    pub current: Arc<dyn Current>,
}
impl std::fmt::Debug for Characters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Characters")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub struct Report {
    pub id: Uuid,
    pub name: String,
    pub history: History,
    /// Exact endpoint differences; missing dates remain missing.
    pub gains: [Option<i128>; 3],
    pub record: Option<i128>,
    pub ranks: Option<Vec<(Date, u32)>>,
    /// Derived estimate from seven consecutive observed daily intervals.
    pub predicted: Option<Date>,
}
struct Policy(Uuid, Permissions);
impl Visibility for Policy {
    fn permissions(&self, id: Uuid) -> Permissions {
        if id == self.0 {
            self.1
        } else {
            Permissions::default()
        }
    }
}
fn xp(d: &Daily) -> Option<i128> {
    let raw = d.experience.as_deref()?;
    let value = raw.parse::<u64>().ok()?;
    (value.to_string() == raw).then_some(i128::from(value))
}
fn gain(daily: &[Daily], end: Date, days: i64) -> Option<i128> {
    let start = end.checked_sub(Duration::days(days))?;
    Some(xp(daily.iter().find(|d| d.day == end)?)? - xp(daily.iter().find(|d| d.day == start)?)?)
}
fn predict(daily: &[Daily], end: Date, threshold: Option<u64>) -> Option<Date> {
    let threshold = i128::from(threshold?);
    for days in 0..=7 {
        let date = end.checked_sub(Duration::days(days))?;
        xp(daily.iter().find(|d| d.day == date)?)?;
    }
    let current = xp(daily.iter().find(|d| d.day == end)?)?;
    let delta = gain(daily, end, 7)?;
    if delta <= 0 || threshold <= current {
        return None;
    }
    let numerator = (threshold - current).checked_mul(7)?;
    let days = (numerator + delta - 1) / delta;
    if days > 3650 {
        return None;
    }
    end.checked_add(Duration::days(i64::try_from(days).ok()?))
}
impl Characters {
    pub async fn load(
        &self,
        id: Uuid,
        start: Date,
        end: Date,
        target: Option<u32>,
    ) -> Result<Report, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if id.is_nil()
            || id.get_version_num() != 7
            || start > end
            || (end - start).whole_days() > 365
            || end > time::OffsetDateTime::now_utc().date()
            || target == Some(0)
        {
            return Err(Error::Invalid);
        }
        let public = self.current.character(id).await?;
        if !public.permissions.visible {
            return Err(Error::NotFound);
        }
        if public.name.is_empty() || public.name.len() > 256 {
            return Err(Error::Unavailable);
        }
        let policy = Policy(
            id,
            Permissions {
                online: false,
                ..public.permissions
            },
        );
        let mut history = store::history(&self.db, id, start, end, &policy)
            .await
            .map_err(|_| Error::Unavailable)?;
        if history.daily.len() > 366 || history.events.len() > 10_000 {
            return Err(Error::Unavailable);
        }
        // Achievement points can be account-scoped. Never expose them publicly.
        for day in &mut history.daily {
            day.achievement_points = None;
            day.minutes_online = None;
        }
        // Public event presentation uses a closed field allowlist in the HTTP layer.
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        history.events.retain(|e| {
            if e.kind == "death" {
                return e
                    .details
                    .get("occurred_at")
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.parse::<i64>().ok())
                    .is_some_and(|t| t > 0 && t <= now && now - t <= 30 * 86400);
            }
            matches!(
                e.kind.as_str(),
                "name_change"
                    | "world_change"
                    | "guild_join"
                    | "guild_leave"
                    | "level_up"
                    | "bazaar_sale"
                    | "death"
            )
        });
        let mut deaths: Vec<_> = history
            .events
            .iter()
            .filter(|e| e.kind == "death")
            .filter_map(|e| {
                let raw = e.details.get("death_occurrence_id")?.as_str()?;
                let id = Uuid::parse_str(raw).ok()?;
                if id.is_nil() || id.get_version_num() != 7 || id.to_string() != raw {
                    return None;
                }
                let occurred = e
                    .details
                    .get("occurred_at")?
                    .as_str()?
                    .parse::<i64>()
                    .ok()?;
                Some((occurred, id))
            })
            .collect();
        deaths.sort_by(|a, b| b.cmp(a));
        deaths.dedup();
        deaths.truncate(20);
        history.events.retain(|e| {
            e.kind != "death"
                || e.details
                    .get("death_occurrence_id")
                    .and_then(|v| v.as_str())
                    .and_then(|v| Uuid::parse_str(v).ok())
                    .is_some_and(|id| deaths.iter().any(|(_, allowed)| *allowed == id))
        });
        let gains = [1, 7, 30].map(|days| gain(&history.daily, end, days));
        let record = history
            .daily
            .windows(2)
            .filter_map(|pair| {
                if pair[0].day.next_day() != Some(pair[1].day) {
                    return None;
                }
                Some(xp(&pair[1])? - xp(&pair[0])?)
            })
            .max();
        let threshold = target
            .filter(|level| {
                history
                    .daily
                    .last()
                    .is_some_and(|d| i64::from(*level) > d.level)
            })
            .and_then(|level| self.current.experience_at_level(id, level));
        let predicted = predict(&history.daily, end, threshold);
        if public
            .ranks
            .as_ref()
            .is_some_and(|rows| rows.len() > 366 || rows.iter().any(|(_, rank)| *rank == 0))
        {
            return Err(Error::Unavailable);
        }
        let ranks = public.ranks.map(|mut rows| {
            rows.retain(|(d, r)| *d >= start && *d <= end && *r > 0);
            rows.sort();
            rows
        });
        // Recheck after storage I/O so a concurrent privacy suppression fails closed.
        let latest = self.current.character(id).await?;
        if !latest.permissions.visible {
            return Err(Error::NotFound);
        }
        if latest.permissions.skills != public.permissions.skills
            || latest.permissions.guild != public.permissions.guild
            || latest.permissions.deaths != public.permissions.deaths
            || latest.name != public.name
        {
            return Err(Error::Unavailable);
        }
        Ok(Report {
            id,
            name: public.name,
            history,
            gains,
            record,
            ranks,
            predicted,
        })
    }
    /// All-or-nothing comparison: hidden subjects cannot be inferred from partial output.
    pub async fn compare(
        &self,
        ids: &[Uuid],
        start: Date,
        end: Date,
    ) -> Result<Vec<Report>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !(2..=5).contains(&ids.len())
            || ids.iter().enumerate().any(|(i, id)| ids[..i].contains(id))
        {
            return Err(Error::Invalid);
        }
        let mut reports = Vec::new();
        for id in ids {
            reports.push(self.load(*id, start, end, None).await?);
        }
        for report in &reports {
            let current = self.current.character(report.id).await?;
            if !current.permissions.visible {
                return Err(Error::NotFound);
            }
            if current.name != report.name
                || (!current.permissions.skills
                    && report.history.daily.iter().any(|d| d.skills.is_some()))
            {
                return Err(Error::Unavailable);
            }
        }
        reports.sort_by_key(|r| r.id);
        Ok(reports)
    }
}
