//! World statistics over observed daily snapshots, never an authoritative population.
//! Raw T65 presence counts lack privacy subjects and cannot be published safely.
use crate::store::Visibility;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
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
#[derive(Debug, PartialEq, Eq)]
pub struct Day {
    pub date: Date,
    pub levels: BTreeMap<i64, usize>,
    /// None means no eligible snapshot evidence, not zero deaths.
    pub deaths: Option<usize>,
}
#[derive(Debug)]
pub struct Report {
    pub days: Vec<Day>,
    /// Historical samples cannot be privacy-filtered with the current storage contract.
    pub online: Option<Vec<(i64, usize)>>,
}
/// Read a maximum of 31 UTC days. Each census uses only that day's snapshots;
/// deaths require a same-day world observation and current explicit permission.
/// No carry-forward guesses are made across missing days or world transfers.
pub async fn read(
    db: &PgPool,
    config: Config,
    world: Uuid,
    start: Date,
    end: Date,
    visibility: &impl Visibility,
) -> Result<Report, Error> {
    if !config.enabled {
        return Err(Error::Disabled);
    }
    if world.is_nil() || start > end || (end - start).whole_days() > 30 {
        return Err(Error::BadInput);
    }
    let rows = sqlx::query("SELECT d.character_ref,d.day,d.level,(SELECT count(*) FROM stats_character_events e WHERE e.character_ref=d.character_ref AND e.day=d.day AND e.kind='death') AS deaths FROM stats_character_daily d WHERE d.world=$1 AND d.day BETWEEN $2 AND $3 ORDER BY d.day,d.character_ref LIMIT 100001")
        .bind(world).bind(start).bind(end).fetch_all(db).await.map_err(|_|Error::Unavailable)?;
    if rows.len() > 100_000 {
        return Err(Error::Unavailable);
    }
    let mut days = BTreeMap::new();
    let mut date = start;
    loop {
        days.insert(
            date,
            Day {
                date,
                levels: BTreeMap::new(),
                deaths: None,
            },
        );
        if date == end {
            break;
        }
        date = date.next_day().ok_or(Error::BadInput)?;
    }
    for row in rows {
        let character = row
            .try_get("character_ref")
            .map_err(|_| Error::Unavailable)?;
        let permissions = visibility.current_permissions(character).await;
        if !permissions.visible {
            continue;
        }
        let date = row.try_get("day").map_err(|_| Error::Unavailable)?;
        let day = days.get_mut(&date).ok_or(Error::Unavailable)?;
        let level = row.try_get("level").map_err(|_| Error::Unavailable)?;
        *day.levels.entry(level).or_default() += 1;
        if permissions.deaths {
            let count: i64 = row.try_get("deaths").map_err(|_| Error::Unavailable)?;
            *day.deaths.get_or_insert(0) +=
                usize::try_from(count).map_err(|_| Error::Unavailable)?;
        }
    }
    Ok(Report {
        days: days.into_values().collect(),
        online: None,
    })
}

pub mod http;
pub mod kills;
#[cfg(test)]
mod tests;
