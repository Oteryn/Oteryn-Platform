//! Owner-only map interpretation of G-11 progress. No gameplay writes or grants.
use oteryn_playerdata::progress::{Actor, Authority, Payload, ProgressStore, ReadScope};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::collections::{BTreeMap, BTreeSet};

pub mod web;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Denied,
    BadInput,
    Unavailable,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Catalog adapter must use one active, visible, ready world version. Revision
/// is the creature definition revision, never a guessed release identifier.
#[derive(Debug, Clone)]
pub struct Definition {
    pub key: String,
    pub revision: String,
    pub class: String,
    pub difficulty: String,
    pub stars: Option<u8>,
    pub thresholds: [u64; 3],
    pub points: Points,
}
#[derive(Debug, Clone, Copy)]
pub enum Points {
    Charms(u16),
    BossStages([u16; 3]),
}
/// Only fixed, owner-approved at-reset policies are eligible. Room, encounter
/// and unknown policies have no representation in the returned map.
#[derive(Debug, Clone)]
pub enum SpawnPolicy {
    Ordinary,
    FixedAtReset { chance: f64 },
    Room,
    Encounter,
    Unknown,
}
#[derive(Debug, Clone)]
pub struct Spawn {
    pub key: String,
    pub creature: String,
    pub cell: super::spawns::Cell,
    pub policy: SpawnPolicy,
}
/// Rates come from owner-authorized journal observations / privacy-qualified T81
/// aggregates for this character's level band. Unknown rates are omitted.
#[derive(Debug, Clone)]
pub struct Pace {
    pub creature: String,
    pub min_level: u32,
    pub max_level: u32,
    pub kills_per_hour: f64,
}
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(default, deserialize_with = "optional_text")]
    pub class: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    pub difficulty: Option<String>,
    #[serde(default, deserialize_with = "optional_stars")]
    pub stars: Option<u8>,
}
fn optional_text<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    let value = String::deserialize(d)?;
    Ok((!value.is_empty()).then_some(value))
}
fn optional_stars<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<u8>, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse::<u8>()
            .map(Some)
            .map_err(serde::de::Error::custom)
    }
}
impl Filters {
    pub fn validate(&self) -> Result<()> {
        if self.stars.is_some_and(|s| s > 5)
            || [&self.class, &self.difficulty].iter().any(|v| {
                v.as_ref().is_some_and(|s| {
                    s.is_empty() || s.len() > 128 || s.chars().any(char::is_control)
                })
            })
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }
    fn matches(&self, d: &Definition) -> bool {
        self.class.as_ref().is_none_or(|v| *v == d.class)
            && self.difficulty.as_ref().is_none_or(|v| *v == d.difficulty)
            && self.stars.is_none_or(|v| Some(v) == d.stars)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressState {
    NotStarted,
    InProgress,
    Completed,
}
#[derive(Debug, Serialize)]
pub struct Entry {
    pub boss: bool,
    pub creature: String,
    pub state: ProgressState,
    pub kills: u64,
    pub thresholds: [u64; 3],
    /// Charm points are the committed Game value. Boss points are a catalog
    /// interpretation of each reached stage, never a grant or wallet balance.
    pub points: u32,
    pub possible_points: u32,
    pub stage: usize,
    pub cooldown_remaining_ms: Option<u64>,
    pub spawns: Vec<MapPoint>,
    pub estimated_hours: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct MapPoint {
    pub spawn: String,
    pub cell: super::spawns::Cell,
    pub reset_chance: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct Map {
    pub entries: Vec<Entry>,
    /// Creature keys ordered by shortest estimated remaining time, then key.
    pub recommendations: Vec<String>,
}

/// Calls the public T77 API for both families. It enforces owner-only access,
/// baseline, catalog acceptance, current ownership cuts and recovery fences.
/// The caller supplies trusted T72 authority, never an HTTP-derived implementation.
pub async fn journal(
    store: &ProgressStore,
    actor: Actor,
    account: Uuid,
    character: Uuid,
    authority: &impl Authority,
    now_ms: u64,
) -> Result<Vec<Payload>> {
    let mut out = Vec::new();
    for family in ["bestiary", "boss_cooldown"] {
        let rows = store
            .read(
                actor,
                &ReadScope {
                    account,
                    character: Some(character),
                    family,
                },
                authority,
                now_ms,
            )
            .await
            .map_err(|e| match e {
                oteryn_playerdata::progress::Error::Denied => Error::Denied,
                _ => Error::Unavailable,
            })?;
        out.extend(rows);
    }
    Ok(out)
}

/// Pure interpretation after current ownership and fresh complete baseline proof.
/// Missing per-creature evidence is unknown and is omitted, never treated as zero.
pub fn build(
    definitions: &[Definition],
    spawns: &[Spawn],
    progress: &[Payload],
    pace: &[Pace],
    level: u32,
    filters: &Filters,
    now_ms: u64,
) -> Result<Map> {
    filters.validate()?;
    if level == 0
        || definitions.len() > 100_000
        || spawns.len() > 100_000
        || progress.len() > 100_000
    {
        return Err(Error::BadInput);
    }
    let mut records = BTreeMap::new();
    for p in progress {
        if matches!(p, Payload::Bestiary(_) | Payload::Boss(_))
            && records.insert(p.entity(), p).is_some()
        {
            return Err(Error::Unavailable);
        }
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for d in definitions {
        if d.key.is_empty()
            || d.revision.is_empty()
            || !seen.insert(&d.key)
            || d.thresholds[0] == 0
            || d.thresholds.windows(2).any(|w| w[0] >= w[1])
            || d.stars.is_some_and(|v| v > 5)
        {
            return Err(Error::Unavailable);
        }
        if !filters.matches(d) {
            continue;
        }
        let Some(p) = records.get(d.key.as_str()) else {
            continue;
        };
        let (kills, points, cooldown_remaining_ms) = match (d.points, *p) {
            (Points::Charms(_), Payload::Bestiary(b))
                if b.race_definition_revision == d.revision
                    && u64::from(b.final_kill_threshold) == d.thresholds[2] =>
            {
                (u64::from(b.kill_count), b.charm_points_awarded, None)
            }
            (Points::BossStages(points), Payload::Boss(b))
                if b.definition_revision == d.revision =>
            {
                let kills = b
                    .kill_count
                    .parse::<u64>()
                    .map_err(|_| Error::Unavailable)?;
                if kills.to_string() != b.kill_count {
                    return Err(Error::Unavailable);
                }
                let sum = points
                    .iter()
                    .zip(d.thresholds)
                    .filter(|(_, threshold)| kills >= *threshold)
                    .map(|(points, _)| u32::from(*points))
                    .sum();
                let cooldown = b
                    .available_at_unix_ms
                    .as_ref()
                    .map(|s| {
                        let n = s.parse::<u64>().map_err(|_| Error::Unavailable)?;
                        if n.to_string() != *s || n > 253402300799999 {
                            return Err(Error::Unavailable);
                        }
                        Ok(n.saturating_sub(now_ms))
                    })
                    .transpose()?;
                (kills, sum, cooldown)
            }
            _ => return Err(Error::Unavailable),
        };
        let possible_points = match d.points {
            Points::Charms(p) => u32::from(p),
            Points::BossStages(p) => p.iter().map(|v| u32::from(*v)).sum(),
        };
        let state = if kills >= d.thresholds[2] {
            ProgressState::Completed
        } else if kills > 0 {
            ProgressState::InProgress
        } else {
            ProgressState::NotStarted
        };
        let mut locations = Vec::new();
        for spawn in spawns.iter().filter(|s| s.creature == d.key) {
            let chance = match (&spawn.policy, d.points) {
                (SpawnPolicy::Ordinary, Points::Charms(_)) => None,
                (SpawnPolicy::FixedAtReset { chance }, Points::BossStages(_))
                    if chance.is_finite() && *chance > 0.0 && *chance <= 1.0 =>
                {
                    Some(*chance)
                }
                _ => continue,
            };
            if spawn.key.is_empty() || spawn.cell.floor > 15 {
                return Err(Error::Unavailable);
            }
            locations.push(MapPoint {
                spawn: spawn.key.clone(),
                cell: spawn.cell,
                reset_chance: chance,
            });
        }
        if locations.is_empty() {
            continue;
        }
        let rates: Vec<_> = pace
            .iter()
            .filter(|p| p.creature == d.key && p.min_level <= level && level <= p.max_level)
            .collect();
        let estimated_hours = if matches!(d.points, Points::Charms(_))
            && state != ProgressState::Completed
            && rates.len() == 1
        {
            let rate = rates[0].kills_per_hour;
            (rate.is_finite() && rate > 0.0)
                .then(|| (d.thresholds[2] - kills) as f64 / rate)
                .filter(|v| v.is_finite())
        } else {
            None
        };
        entries.push(Entry {
            boss: matches!(d.points, Points::BossStages(_)),
            creature: d.key.clone(),
            state,
            kills,
            thresholds: d.thresholds,
            points,
            possible_points,
            stage: d.thresholds.iter().filter(|v| kills >= **v).count(),
            cooldown_remaining_ms,
            spawns: locations,
            estimated_hours,
        });
    }
    entries.sort_by(|a, b| a.creature.cmp(&b.creature));
    let mut ranked: Vec<_> = entries
        .iter()
        .filter_map(|e| e.estimated_hours.map(|h| (h, e.creature.clone())))
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(Map {
        entries,
        recommendations: ranked.into_iter().take(10).map(|(_, k)| k).collect(),
    })
}
#[cfg(test)]
mod tests;
