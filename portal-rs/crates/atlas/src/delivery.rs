//! Quest delivery and item-source discovery over visible, versioned world entities.
//! No writes or live game calls. T84/T14b integrate through links and public views.
pub mod estimate;
mod http;
mod overlay;
mod shops;
mod sprite;
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
    pub reward: bool,
    pub item: Option<String>,
    pub count: Option<u32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Delivery {
    pub version: Uuid,
    pub item: String,
    pub sources: Vec<Source>,
    pub shops: Vec<shops::ShopOffer>,
    pub requirements: Vec<QuestItem>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    pub version: Option<Uuid>,
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
        if self.item.as_deref().is_some_and(|key| !item_key(key))
            || self.count.is_some_and(|v| v == 0 || v > 100_000)
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

fn key_segment(part: &str) -> bool {
    part.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn content_key(key: &str) -> bool {
    key.len() <= 256
        && key.split_once(':').map_or_else(
            || key_segment(key),
            |(ns, id)| key_segment(ns) && key_segment(id),
        )
}

fn item_key(key: &str) -> bool {
    content_key(key)
        && match key.split_once(':') {
            Some(("oteryn", id)) => {
                id.strip_prefix("item.").is_some_and(key_segment)
                    && !id.bytes().any(|b| b.is_ascii_uppercase())
            }
            Some(("item", _)) | None => true,
            Some(_) => false,
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
            let envelope = if e.kind == "Spawn.Source" {
                "declaration"
            } else {
                "definition"
            };
            if let Some(inner) = e.data.get_mut(envelope) {
                e.data = inner.take();
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
        self.kind("Item")
            .map(|(key, _)| key)
            .filter(|key| item_key(key))
            .map(str::to_owned)
            .collect()
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
                if matches!(
                    s.get("kind").and_then(Value::as_str),
                    Some("collect" | "item")
                ) {
                    if let Some(items) = s.get("target_items").and_then(Value::as_array) {
                        result.extend(
                            items
                                .iter()
                                .map(|target| self.quest_item(quest, s, target, false)),
                        );
                    } else {
                        result.push(self.quest_item(quest, s, s, false));
                    }
                }
                if let Some(rewards) = s.get("reward_items").and_then(Value::as_array) {
                    result.extend(
                        rewards
                            .iter()
                            .map(|reward| self.quest_item(quest, s, reward, true)),
                    );
                }
            }
        }
        result
    }
    fn quest_item(&self, quest: &str, step: &Value, target: &Value, reward: bool) -> QuestItem {
        // Resolution markers veto keys. Objectives and rewards cannot be inferred from names.
        let item =
            if step.get("item_resolution").is_some() || target.get("item_resolution").is_some() {
                None
            } else {
                target
                    .get("item")
                    .and_then(reference)
                    .filter(|key| item_key(key) && self.get("Item", key).is_some())
            };
        let count = target
            .get("count")
            .or_else(|| if reward { None } else { step.get("count") });
        QuestItem {
            quest: quest.into(),
            step: step
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            reward,
            item: item.map(str::to_owned),
            count: count
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| (1..=100_000).contains(n)),
        }
    }
    pub fn discover(&self, options: Options, filters: &Filters) -> Result<Delivery, Error> {
        if !options.enabled {
            return Err(Error::Disabled);
        }
        filters.validate()?;
        let item = filters.item.as_deref().ok_or(Error::InvalidInput)?;
        if filters
            .version
            .is_some_and(|version| version != self.version)
            || self.get("Item", item).is_none()
        {
            return Err(Error::Unavailable);
        }
        let mut sources = Vec::new();
        let mut estimate_cache = BTreeMap::new();
        // Each distinct fixed estimate uses three bounded binary searches. Cap
        // their aggregate summation terms; exhausted estimates stay unavailable.
        let mut remaining_terms = 4096u32;
        // Index once per request rather than rescanning all spawn documents per source.
        let mut spawn_index: BTreeMap<&str, Vec<(&Value, &Value)>> = BTreeMap::new();
        for (_, spawn) in self.kind("Spawn.Source") {
            for point in spawn
                .get("points")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(creature) = point.get("creature").and_then(reference) {
                    spawn_index
                        .entry(creature)
                        .or_default()
                        .push((spawn, point));
                }
            }
        }
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
            for (spawn, point) in spawn_index.get(creature).into_iter().flatten() {
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
                    || u32::try_from(x).is_err()
                    || u32::try_from(y).is_err()
                    || u8::try_from(floor).is_err()
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
            if (filters.floor.is_some() || filters.city.is_some()) && spawns.is_empty() {
                continue;
            }
            spawns.sort_by_key(|p| (p.floor, p.x, p.y));
            let estimate = if min == max
                && let Some(target) = filters.count
            {
                let key = (target, min, ppm);
                estimate_cache
                    .entry(key)
                    .or_insert_with(|| {
                        let cost = target.div_ceil(min);
                        if cost > remaining_terms {
                            return None;
                        }
                        remaining_terms -= cost;
                        estimate::fixed(target, min, ppm).ok()
                    })
                    .clone()
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
            shops: if filters.tab.as_deref() == Some("creatures") || filters.creature.is_some() {
                Vec::new()
            } else {
                self.shop_offers(item)
            },
            requirements: self.requirements(),
        })
    }
}
fn reference(v: &Value) -> Option<&str> {
    v.as_str().or_else(|| v.get("key").and_then(Value::as_str))
}

/// Read allow-listed entities using one database snapshot of the active world.
pub async fn load(store: &WorldStore) -> Result<Snapshot, Error> {
    let (version, entities) = store
        .public_snapshot(&[
            "Item",
            "Creature",
            "Loot",
            "Spawn.Source",
            "Quest",
            "NPC",
            shops::KIND,
            sprite::KIND,
        ])
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
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
/// Shared flag decision for wiki entry points and delivery routes.
pub fn wiki_map_links_enabled() -> bool {
    let flags = crate::config::Flags::configured();
    flags.delivery && flags.world
}
/// Wiki T14b can mount this stable item/creature link without depending on private internals.
pub fn wiki_map_link(locale: &str, kind: &str, key: &str) -> Result<String, Error> {
    if !["pl", "en"].contains(&locale)
        || !["items", "creatures"].contains(&kind)
        || (kind == "items" && !item_key(key))
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
