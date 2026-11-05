//! Guild statistics for explicitly permitted members. Rankings are derived observations.
use crate::store::Visibility;
use sqlx::{PgPool, Row};
use std::collections::BTreeSet;
use time::Date;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub enabled: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    BadInput,
    Unavailable,
}
/// T24e adapter seam: return only current, eligible guilds and same-world members.
/// Suppressed, ambiguous, stale or missing source evidence must return Unavailable.
/// Permission checks must use current T45 visibility, never historical snapshots.
pub trait Source: Visibility + Send + Sync {
    fn current(&self) -> Result<Vec<Guild>, Error>;
}
#[derive(Clone, Debug)]
pub struct Member {
    pub id: Uuid,
    pub rank: u8,
    pub name: String,
    pub level: Option<u32>,
    pub experience: Option<u64>,
}
#[derive(Clone, Debug)]
pub struct Guild {
    pub id: Uuid,
    pub world: Uuid,
    pub name: String,
    pub founded_at: i64,
    pub members: Vec<Member>,
}
#[derive(Debug)]
pub struct Summary {
    pub guild: Guild,
    pub members: usize,
    pub total_level: Option<u64>,
    pub average_level: Option<f64>,
    pub experience: Option<u128>,
    /// Owner-only achievement points and unsupported war statistics are unavailable.
    pub achievement_points: Option<u64>,
    pub wars: Option<u64>,
}
/// Derived ranking over the current visible cohort; not a Game ranking.
/// Internal stable IDs break ties but must never appear in public output.
pub fn list(config: Config, source: &dyn Source) -> Result<Vec<Summary>, Error> {
    if !config.enabled {
        return Err(Error::Disabled);
    }
    let guilds = source.current()?;
    if guilds.len() > 1000 {
        return Err(Error::BadInput);
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    let mut member_ids = BTreeSet::new();
    for mut guild in guilds {
        if guild.id.is_nil()
            || guild.world.is_nil()
            || guild.name.is_empty()
            || guild.name.len() > 128
            || guild.founded_at <= 0
            || !ids.insert(guild.id)
            || guild.members.len() > 2000
        {
            return Err(Error::BadInput);
        }
        let mut members = BTreeSet::new();
        for m in &guild.members {
            if m.id.is_nil()
                || !(1..=20).contains(&m.rank)
                || m.name.is_empty()
                || m.name.len() > 128
                || m.level == Some(0)
                || !members.insert(m.id)
                || !member_ids.insert(m.id)
            {
                return Err(Error::BadInput);
            }
        }
        guild.members.retain(|m| {
            let p = source.permissions(m.id);
            p.visible && p.guild
        });
        guild.members.sort_by_key(|m| (m.rank, m.id));
        let count = guild.members.len();
        let total_level = if count == 0 {
            None
        } else {
            guild
                .members
                .iter()
                .map(|m| m.level.map(u64::from))
                .sum::<Option<u64>>()
        };
        let experience = if count == 0 {
            None
        } else {
            guild
                .members
                .iter()
                .map(|m| m.experience.map(u128::from))
                .sum::<Option<u128>>()
        };
        result.push(Summary {
            members: count,
            average_level: total_level.map(|n| n as f64 / count as f64),
            total_level,
            experience,
            guild,
            achievement_points: None,
            wars: None,
        });
    }
    result.sort_by(|a, b| {
        b.total_level
            .cmp(&a.total_level)
            .then(a.guild.id.cmp(&b.guild.id))
    });
    Ok(result)
}
#[derive(Debug)]
pub struct Observation {
    pub day: Date,
    pub member: String,
    pub level: i64,
}
#[derive(Debug)]
pub struct History {
    /// Samples of currently permitted members, not a complete joins/leaves ledger.
    pub observations: Vec<Observation>,
    /// Same current members, same guild/world at both endpoints; missing evidence is unknown.
    pub experience_gain: Option<i128>,
}
pub async fn history(
    db: &PgPool,
    config: Config,
    source: &dyn Source,
    guild_id: Uuid,
    start: Date,
    end: Date,
) -> Result<History, Error> {
    if !config.enabled {
        return Err(Error::Disabled);
    }
    if guild_id.is_nil() || start > end || (end - start).whole_days() > 30 {
        return Err(Error::BadInput);
    }
    let guild = list(config, source)?
        .into_iter()
        .find(|s| s.guild.id == guild_id)
        .ok_or(Error::Unavailable)?
        .guild;
    let ids: Vec<_> = guild.members.iter().map(|m| m.id).collect();
    let rows = sqlx::query("SELECT character_ref,day,level,experience::text FROM stats_character_daily WHERE guild=$1 AND world=$2 AND character_ref=ANY($3) AND day BETWEEN $4 AND $5 ORDER BY day,character_ref LIMIT 62001")
        .bind(guild.id).bind(guild.world).bind(&ids).bind(start).bind(end)
        .fetch_all(db).await.map_err(|_|Error::Unavailable)?;
    if rows.len() > 62000 {
        return Err(Error::Unavailable);
    }
    let mut observations = Vec::new();
    let mut endpoints = std::collections::BTreeMap::<Uuid, (Option<i128>, Option<i128>)>::new();
    for row in rows {
        let id = row
            .try_get("character_ref")
            .map_err(|_| Error::Unavailable)?;
        let member = guild
            .members
            .iter()
            .find(|m| m.id == id)
            .ok_or(Error::Unavailable)?;
        let day = row.try_get("day").map_err(|_| Error::Unavailable)?;
        let value: Option<String> = row.try_get("experience").map_err(|_| Error::Unavailable)?;
        let xp = value
            .map(|v| v.parse::<i128>())
            .transpose()
            .map_err(|_| Error::Unavailable)?;
        let pair = endpoints.entry(id).or_default();
        if day == start {
            pair.0 = xp;
        }
        if day == end {
            pair.1 = xp;
        }
        observations.push(Observation {
            day,
            member: member.name.clone(),
            level: row.try_get("level").map_err(|_| Error::Unavailable)?,
        });
    }
    let experience_gain = if ids.is_empty() {
        None
    } else {
        ids.iter()
            .map(|id| {
                endpoints
                    .get(id)
                    .and_then(|(a, b)| b.as_ref()?.checked_sub(*a.as_ref()?))
            })
            .try_fold(0_i128, |sum, value| sum.checked_add(value?))
    };
    Ok(History {
        observations,
        experience_gain,
    })
}
pub mod http;
#[cfg(test)]
mod tests;
