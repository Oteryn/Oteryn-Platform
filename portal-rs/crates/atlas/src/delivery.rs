//! Quest delivery and item-source discovery over visible, versioned world entities.
//! No writes or live game calls. T84/T14b integrate through links and public views.
pub mod estimate;
mod http;
#[cfg(test)]
mod tests;

pub use http::{configured_router, router};
use oteryn_world::{Entity, WorldStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    InvalidInput,
    Unavailable,
}
#[derive(Debug, Clone, Serialize)]
pub struct Spawn {
    pub x: i64,
    pub y: i64,
    pub floor: i64,
    pub respawn_ms: u64,
    /// Only authoritative labels are accepted; no nearest-city guesses.
    pub city: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub creature: String,
    pub probability_ppm: u32,
    pub min_count: u32,
    pub max_count: u32,
    pub level: Option<u32>,
    pub spawns: Vec<Spawn>,
    /// Relative heuristic from drop chance, point density and required level; not a game fact.
    pub tier: Option<u8>,
    pub estimate: Option<estimate::Estimate>,
}
#[derive(Debug, Clone, Serialize)]
pub struct QuestItem {
    pub quest: String,
    pub step: String,
    pub item: Option<String>,
    pub count: Option<u32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Delivery {
    pub version: Uuid,
    pub item: String,
    pub sources: Vec<Source>,
    pub requirements: Vec<QuestItem>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(default, deserialize_with = "empty")]
    pub item: Option<String>,
    #[serde(default, deserialize_with = "empty")]
    pub creature: Option<String>,
    #[serde(default, deserialize_with = "empty")]
    pub q: Option<String>,
    #[serde(default, deserialize_with = "empty")]
    pub level: Option<u32>,
    #[serde(default, deserialize_with = "empty")]
    pub floor: Option<i64>,
    #[serde(default, deserialize_with = "empty")]
    pub city: Option<String>,
    #[serde(default, deserialize_with = "empty")]
    pub rarity: Option<u32>,
    #[serde(default, deserialize_with = "empty")]
    pub count: Option<u32>,
    #[serde(default, deserialize_with = "empty")]
    pub tab: Option<String>,
}
fn empty<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
{
    let s = Option::<String>::deserialize(d)?;
    match s.filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => s
            .parse()
            .map(Some)
            .map_err(|_| serde::de::Error::custom("invalid filter")),
    }
}

impl Filters {
    pub fn validate(&self) -> Result<(), Error> {
        for key in [&self.item, &self.creature, &self.q, &self.city]
            .into_iter()
            .flatten()
        {
            if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
                return Err(Error::InvalidInput);
            }
        }
        if self.count.is_some_and(|v| v == 0 || v > 100_000)
            || self.level.is_some_and(|v| v == 0)
            || self.floor.is_some_and(|v| !(-128..=127).contains(&v))
            || self.rarity.is_some_and(|v| v > 1_000_000)
            || self
                .tab
                .as_deref()
                .is_some_and(|v| !["all", "creatures", "quests"].contains(&v))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

/// Snapshot seam for T84. Construct only from public WorldStore reads; mixed versions deny.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub version: Uuid,
    entities: BTreeMap<(String, String), Value>,
}
impl Snapshot {
    pub fn from_visible(version: Uuid, entities: Vec<Entity>) -> Result<Self, Error> {
        let mut map = BTreeMap::new();
        for mut e in entities {
            // World imports preserve the original record envelope.
            if let Some(inner) = e
                .data
                .get(if e.kind == "Spawn.Source" {
                    "declaration"
                } else {
                    "definition"
                })
                .cloned()
            {
                e.data = inner;
            }
            if e.version_id != version || map.insert((e.kind, e.key), e.data).is_some() {
                return Err(Error::Unavailable);
            }
        }
        Ok(Self {
            version,
            entities: map,
        })
    }
    fn get(&self, kind: &str, key: &str) -> Option<&Value> {
        self.entities.get(&(kind.into(), key.into()))
    }
    fn kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = (&'a str, &'a Value)> {
        self.entities
            .iter()
            .filter(move |((k, _), _)| k == kind)
            .map(|((_, key), v)| (key.as_str(), v))
    }
    pub fn items(&self) -> Vec<String> {
        self.kind("Item").map(|(k, _)| k.to_owned()).collect()
    }
    pub fn creature_items(&self, creature: &str) -> BTreeSet<String> {
        self.get("Creature", creature)
            .and_then(|v| v.get("loot"))
            .and_then(reference)
            .and_then(|k| self.get("Loot", k))
            .and_then(|v| v.get("entries"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| e.get("item").and_then(reference))
            .filter(|k| self.get("Item", k).is_some())
            .map(str::to_owned)
            .collect()
    }
    pub fn requirements(&self) -> Vec<QuestItem> {
        let mut result = Vec::new();
        for (quest, v) in self.kind("Quest") {
            let stages = v
                .get("stages")
                .or_else(|| v.pointer("/oteryn_recipe/payload/recipe/stages"))
                .or_else(|| v.pointer("/presentation/stages"))
                .and_then(Value::as_array);
            for s in stages.into_iter().flatten() {
                if !matches!(
                    s.get("kind").and_then(Value::as_str),
                    Some("collect" | "item")
                ) {
                    continue;
                }
                let targets: Vec<&Value> = s
                    .get("target_items")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().collect())
                    .unwrap_or_else(|| vec![s]);
                for target in targets {
                    // An unresolved marker vetoes a key. Never infer from objective/name.
                    let item = if s.get("item_resolution").is_some()
                        || target.get("item_resolution").is_some()
                    {
                        None
                    } else {
                        target
                            .get("item")
                            .and_then(reference)
                            .filter(|key| self.get("Item", key).is_some())
                    };
                    result.push(QuestItem {
                        quest: quest.into(),
                        step: s
                            .get("key")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .into(),
                        item: item.map(str::to_owned),
                        count: s
                            .get("count")
                            .and_then(Value::as_u64)
                            .and_then(|n| u32::try_from(n).ok())
                            .filter(|n| *n > 0),
                    });
                }
            }
        }
        result
    }
    pub fn discover(&self, options: Options, filters: &Filters) -> Result<Delivery, Error> {
        if !options.enabled {
            return Err(Error::Disabled);
        }
        filters.validate()?;
        let item = filters.item.as_deref().ok_or(Error::InvalidInput)?;
        if self.get("Item", item).is_none() {
            return Err(Error::Unavailable);
        }
        let mut sources = Vec::new();
        for (creature, v) in self.kind("Creature") {
            if filters.creature.as_deref().is_some_and(|k| k != creature)
                || filters
                    .q
                    .as_ref()
                    .is_some_and(|q| !creature.to_lowercase().contains(&q.to_lowercase()))
            {
                continue;
            }
            let Some(loot) = v
                .get("loot")
                .and_then(reference)
                .and_then(|key| self.get("Loot", key))
            else {
                continue;
            };
            if loot.get("algorithm").and_then(Value::as_str) != Some("IndependentBernoulliPpm") {
                continue;
            }
            let entries = loot
                .get("entries")
                .and_then(Value::as_array)
                .ok_or(Error::Unavailable)?;
            let matching: Vec<_> = entries
                .iter()
                .filter(|e| e.get("item").and_then(reference) == Some(item))
                .collect();
            // Duplicate independent entries need a combined PMF; never pick an arbitrary row.
            if matching.len() != 1 {
                continue;
            }
            let e = matching[0];
            let number = |key| {
                e.get(key)
                    .and_then(Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or(Error::Unavailable)
            };
            let ppm = number("probability_ppm")?;
            let min = number("min_count")?;
            let max = number("max_count")?;
            if ppm > 1_000_000 || min == 0 || max < min {
                return Err(Error::Unavailable);
            }
            if filters.rarity.is_some_and(|n| ppm > n) {
                continue;
            }
            let level = v
                .get("level")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok());
            if filters.level.is_some_and(|n| level.is_none_or(|l| l > n)) {
                continue;
            }
            let mut spawns = Vec::new();
            for (_, spawn) in self.kind("Spawn.Source") {
                for point in spawn
                    .get("points")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if point.get("creature").and_then(reference) != Some(creature) {
                        continue;
                    }
                    let Some(cell) = point.get("cell") else {
                        continue;
                    };
                    let (Some(x), Some(y), Some(floor), Some(respawn_ms)) = (
                        cell.get("x").and_then(Value::as_i64),
                        cell.get("y").and_then(Value::as_i64),
                        cell.get("floor").and_then(Value::as_i64),
                        point.get("respawn_ms").and_then(Value::as_u64),
                    ) else {
                        continue;
                    };
                    let city = spawn.get("city").and_then(Value::as_str).map(str::to_owned);
                    if respawn_ms == 0
                        || filters.floor.is_some_and(|n| n != floor)
                        || filters
                            .city
                            .as_ref()
                            .is_some_and(|n| city.as_ref() != Some(n))
                    {
                        continue;
                    }
                    spawns.push(Spawn {
                        x,
                        y,
                        floor,
                        respawn_ms,
                        city,
                    });
                }
            }
            if (filters.floor.is_some() || filters.city.is_some()) && spawns.is_empty() {
                continue;
            }
            spawns.sort_by_key(|p| (p.floor, p.x, p.y));
            let estimate = if min == max {
                estimate::fixed(filters.count.unwrap_or(1), min, ppm).ok()
            } else {
                None
            };
            sources.push(Source {
                creature: creature.into(),
                probability_ppm: ppm,
                min_count: min,
                max_count: max,
                level,
                spawns,
                tier: None,
                estimate,
            });
        }
        let score = |s: &Source| {
            s.level
                .filter(|n| *n > 0)
                .filter(|_| !s.spawns.is_empty())
                .map(|l| f64::from(s.probability_ppm) * s.spawns.len() as f64 / f64::from(l))
        };
        let best = sources.iter().filter_map(score).fold(0.0_f64, f64::max);
        for s in &mut sources {
            s.tier = score(s).filter(|_| best > 0.0).map(|v| {
                if v >= best * 0.75 {
                    1
                } else if v >= best * 0.25 {
                    2
                } else {
                    3
                }
            });
        }
        sources.sort_by(|a, b| {
            score(b)
                .unwrap_or(-1.0)
                .total_cmp(&score(a).unwrap_or(-1.0))
                .then_with(|| a.creature.cmp(&b.creature))
        });
        Ok(Delivery {
            version: self.version,
            item: item.into(),
            sources,
            requirements: self.requirements(),
        })
    }
}
fn reference(v: &Value) -> Option<&str> {
    v.as_str().or_else(|| v.get("key").and_then(Value::as_str))
}

/// Reads only allow-listed entities from a single active world. A concurrent activation denies.
pub async fn load(store: &WorldStore) -> Result<Snapshot, Error> {
    let version = store
        .active_version_id()
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    let mut entities = Vec::new();
    for kind in ["Item", "Creature", "Loot", "Spawn.Source", "Quest"] {
        let mut after = None;
        loop {
            let page = store
                .list_by_kind(kind, after.as_deref(), 1000)
                .await
                .map_err(|_| Error::Unavailable)?;
            if page.is_empty() {
                break;
            }
            after = page.last().map(|e| e.key.clone());
            entities.extend(page);
            if entities.len() > 200_000 {
                return Err(Error::Unavailable);
            }
        }
    }
    if store
        .active_version_id()
        .await
        .map_err(|_| Error::Unavailable)?
        != Some(version)
    {
        return Err(Error::Unavailable);
    }
    Snapshot::from_visible(version, entities)
}

pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
/// Wiki T14b can mount this stable item/creature link without depending on private internals.
pub fn wiki_map_link(locale: &str, kind: &str, key: &str) -> Result<String, Error> {
    if !["pl", "en"].contains(&locale)
        || !["items", "creatures"].contains(&kind)
        || key.is_empty()
        || key.len() > 256
        || key.chars().any(char::is_control)
    {
        return Err(Error::InvalidInput);
    }
    Ok(format!(
        "/{locale}/atlas/delivery?{}={}",
        if kind == "items" { "item" } else { "creature" },
        encode(key)
    ))
}
