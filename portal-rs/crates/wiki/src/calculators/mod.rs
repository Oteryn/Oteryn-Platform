//! Read-only estimates. SOURCE: Oteryn/Oteryn-Game at
//! 857f15f376907c592bb28d854fccff18a15cdf87:
//! - rulesets/character/experience/experience-table.json (finite thresholds)
//! - crates/gameplay/characters/src/stamina.rs (OFFLINE-0)
//! - crates/gameplay/progression/src/loop_training.rs (integer exercise gains)
//! - crates/gameplay/dispatch/src/gameplay_transport/loop_training/exercise.rs (2s cadence).
//!
//! T14 currently imports content/**, not rulesets/**. Missing inputs remain
//! unavailable: no extrapolated experience curve or fallback training rates.

mod web;
pub use web::{CalculatorOptions, WorldKey, configured_router, router};

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculationError {
    BadInput,
    Unavailable,
}

/// Uses the finite world table, including its exclusive terminal bound.
/// Current experience beyond the target is invalid, rather than a negative estimate.
pub fn experience_to_level(
    data: &Value,
    current: u64,
    target: u32,
) -> Result<u64, CalculationError> {
    #[derive(Deserialize)]
    struct Table {
        schema: String,
        levels: Vec<Row>,
        terminal_exclusive_experience: String,
    }
    #[derive(Deserialize)]
    struct Row {
        level: u32,
        minimum_experience: String,
    }
    let table: Table =
        serde_json::from_value(data.clone()).map_err(|_| CalculationError::Unavailable)?;
    if table.schema != "OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1" || table.levels.len() != 2000 {
        return Err(CalculationError::Unavailable);
    }
    let mut previous = None;
    let mut threshold = None;
    for (index, row) in table.levels.iter().enumerate() {
        let value = decimal(&row.minimum_experience)?;
        if row.level != index as u32 + 1
            || (index == 0 && value != 0)
            || previous.is_some_and(|prior| value <= prior)
        {
            return Err(CalculationError::Unavailable);
        }
        previous = Some(value);
        if row.level == target {
            threshold = Some(value);
        }
    }
    let terminal = decimal(&table.terminal_exclusive_experience)?;
    if previous.is_none_or(|last| terminal <= last) {
        return Err(CalculationError::Unavailable);
    }
    if current >= terminal {
        return Err(CalculationError::BadInput);
    }
    threshold
        .ok_or(CalculationError::BadInput)?
        .checked_sub(current)
        .ok_or(CalculationError::BadInput)
}

fn decimal(text: &str) -> Result<u64, CalculationError> {
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(CalculationError::Unavailable);
    }
    text.parse::<u64>()
        .ok()
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or(CalculationError::Unavailable)
}

/// Stamina minutes after one uninterrupted offline interval (seconds).
/// Game has no populated stamina data record; these are the implemented OFFLINE-0
/// rules, not a substitute for a world data value. The 21-day cap precedes delay.
pub fn stamina_regeneration(current: u64, offline_seconds: u64) -> Result<u64, CalculationError> {
    if current > 2520 {
        return Err(CalculationError::BadInput);
    }
    let recoverable = offline_seconds.min(1_814_400).saturating_sub(600);
    let missing_normal = 2340 - current.min(2340);
    let normal_gain = recoverable / 180;
    let after = if normal_gain <= missing_normal {
        current + normal_gain
    } else {
        current.max(2340) + (recoverable - 180 * missing_normal) / 360
    };
    Ok(after.min(2520))
}

/// Exercise estimate from remaining mana/tries, with no level-curve assumptions.
/// One charge every two seconds; each charge truncates its percentage-adjusted
/// gain as the Game does. Returns active seconds and charges; excludes interruptions.
pub fn skill_training_time(
    data: &Value,
    remaining: u64,
    family: usize,
    dummy_key: &str,
) -> Result<(u64, u64), CalculationError> {
    #[derive(Deserialize)]
    struct Rates {
        schema_version: u8,
        enabled: bool,
        exercise_gains: [u64; 8],
        percent_scale: u64,
        dummies: std::collections::BTreeMap<String, u64>,
    }
    let rates: Rates =
        serde_json::from_value(data.clone()).map_err(|_| CalculationError::Unavailable)?;
    if rates.schema_version != 1 || !rates.enabled || rates.percent_scale != 100 {
        return Err(CalculationError::Unavailable);
    }
    if family > 6 || remaining > i64::MAX as u64 {
        return Err(CalculationError::BadInput);
    }
    let percentage = *rates
        .dummies
        .get(dummy_key)
        .ok_or(CalculationError::BadInput)?;
    if !matches!(percentage, 100 | 110) {
        return Err(CalculationError::Unavailable);
    }
    let base = rates.exercise_gains[family];
    if base == 0 || base > 1_000_000 {
        return Err(CalculationError::Unavailable);
    }
    let gain = base
        .checked_mul(percentage)
        .ok_or(CalculationError::Unavailable)?
        / rates.percent_scale;
    if gain == 0 {
        return Err(CalculationError::Unavailable);
    }
    let charges = remaining.div_ceil(gain);
    let seconds = charges.checked_mul(2).ok_or(CalculationError::BadInput)?;
    Ok((seconds, charges))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootSplit {
    pub per_member: u64,
    pub remainder: u64,
}

/// Equal split of a user-entered net loot total in indivisible units. The
/// remainder stays unassigned; this is an estimate, never a currency transfer.
pub fn loot_split(total: u64, members: u64) -> Result<LootSplit, CalculationError> {
    if !(1..=1000).contains(&members) || total > i64::MAX as u64 {
        return Err(CalculationError::BadInput);
    }
    Ok(LootSplit {
        per_member: total / members,
        remainder: total % members,
    })
}

#[cfg(test)]
mod tests;
