//! Anonymous statistics over approved projections (G-7 revision 2, §§1 and 9).
//!
//! T59/T60 are not present in the contract base. `StatsSource` is their narrow read seam;
//! no production adapter or route is installed. It must read settlement completion,
//! current privacy authorization and recovery fences atomically, never raw snapshots or bids.
//! No game commands, outbox dispatch, money mutations or retained caches are needed here.
//!
//! Provisional calculation bounds: at most 90 days and 10,000 observations, five
//! samples per published cohort, similarity within ten percent of target level.
//! These are conservative implementation choices, not an accepted public privacy
//! policy. Production adapters must remain unavailable until that policy (including
//! permitted cohorts and protection against differencing queries) is approved.

use crate::{Currency, Error};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
};
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

/// Conservative suppression pending acceptance of G-7's public aggregation policy.
const MIN_SAMPLES: usize = 5;
const MAX_ROWS: usize = 10_000;

#[derive(Debug, Clone, Copy, Default)]
pub struct StatsFlags {
    pub enabled: bool,
}

/// Only explicitly understood public facts may enter a cohort. Item variants must
/// include the complete approved attribute digest and pinned definition revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatsAsset {
    Item {
        definition: String,
        revision: String,
        attributes_hash: [u8; 32],
    },
    Character {
        vocation: u8,
        level: u32,
    },
}

impl StatsAsset {
    fn validate(&self) -> Result<(), Error> {
        match self {
            Self::Item {
                definition,
                revision,
                attributes_hash,
            } => {
                if [definition, revision].iter().any(|s| {
                    s.is_empty()
                        || s.len() > 128
                        || s.trim() != s.as_str()
                        || s.chars().any(char::is_control)
                }) || *attributes_hash == [0; 32]
                {
                    return Err(Error::InvalidInput);
                }
            }
            Self::Character { vocation, level }
                if *vocation > 20 || !(1..=5000).contains(level) =>
            {
                return Err(Error::InvalidInput);
            }
            Self::Character { .. } => {}
        }
        Ok(())
    }
}

/// Half-open observation window. Callers cannot request future or unbounded history.
#[derive(Debug, Clone)]
pub struct StatsQuery {
    pub world: Uuid,
    pub currency: Currency,
    pub asset: StatsAsset,
    pub from: OffsetDateTime,
    pub until: OffsetDateTime,
}

impl StatsQuery {
    fn validate(&self, now: OffsetDateTime) -> Result<(), Error> {
        self.asset.validate()?;
        if self.world.is_nil()
            || self.from >= self.until
            || self.until > now
            || self.until - self.from > Duration::days(90)
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    fn matches(&self, row: &StatsObservation) -> bool {
        row.world == self.world
            && row.currency == self.currency
            && row.asset == self.asset
            && row.closed_at >= self.from
            && row.closed_at < self.until
    }
}

/// A sale is eligible only after the entire saga, including payout and unlock,
/// is durably complete. Delivery alone is not completion. Unknown state is omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsOutcome {
    Completed {
        unit_price: i64,
        quantity: u32,
    },
    /// Fully reconciled unsold auction, including return/unlock and hold release.
    Unsold,
}

/// Trusted adapter input, never a public response. No account or bidder fields.
#[derive(Debug, Clone)]
pub struct StatsObservation {
    pub listing: Uuid,
    pub world: Uuid,
    pub currency: Currency,
    pub asset: StatsAsset,
    pub closed_at: OffsetDateTime,
    pub auction: bool,
    pub outcome: StatsOutcome,
}

/// Only the current seller-approved level and supported skills, with unsupported
/// sections represented by None. Never reconstruct missing sections as zero.
#[derive(Debug, Clone)]
pub struct StatsCharacter {
    pub listing: Uuid,
    pub world: Uuid,
    pub vocation: u8,
    pub level: u32,
    pub skills: Option<BTreeMap<StatsSkill, u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatsSkill {
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    Shielding,
    Fishing,
    Magic,
}

/// One consistent read of current approved public projections. Absence of any
/// required authority, version, watermark, privacy or recovery fence returns None.
/// Adapters exclude pending/refused/ambiguous/cancelled sales and revoked facts,
/// deduplicate by listing, and return None rather than truncate a large result.
/// `valid_until` must reflect the earliest expiry of all supporting evidence.
/// No raw Game ingest or user-provided object may implement this authority seam.
#[derive(Debug, Clone)]
pub struct StatsProjection {
    /// Source completeness cut, not the local time of reading the projection.
    pub observed_at: OffsetDateTime,
    pub valid_until: OffsetDateTime,
    pub observations: Vec<StatsObservation>,
    pub characters: Vec<StatsCharacter>,
}

pub trait StatsSource: Send + Sync {
    fn read(&self) -> impl Future<Output = Result<Option<StatsProjection>, Error>> + Send;
}

/// Default integration until approved listing adapters and capabilities exist.
#[derive(Debug, Default)]
pub struct UnavailableStatsSource;
impl StatsSource for UnavailableStatsSource {
    async fn read(&self) -> Result<Option<StatsProjection>, Error> {
        Ok(None)
    }
}

/// Twice a median unit price, preserving half units without floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketPrice {
    pub twice_units: i128,
}

impl MarketPrice {
    /// Inclusive ±25% warning, using exact integer cross multiplication.
    pub fn warns(&self, unit_price: i64) -> Result<bool, Error> {
        if unit_price <= 0 || self.twice_units <= 0 {
            return Err(Error::InvalidInput);
        }
        let quoted = i128::from(unit_price) * 200;
        Ok(
            quoted <= self.twice_units.checked_mul(75).ok_or(Error::Overflow)?
                || quoted >= self.twice_units.checked_mul(125).ok_or(Error::Overflow)?,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceHistoryPoint {
    pub day: Date,
    pub sales: usize,
    pub market_price: MarketPrice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsSummary {
    pub sales: usize,
    pub turnover: i128,
    pub market_price: MarketPrice,
    pub record_unit_price: i64,
    /// Successful and total terminal auctions; None when the cohort is too small.
    pub auction_success: Option<(usize, usize)>,
    /// Small daily groups are suppressed, not represented as zero.
    pub history: Vec<PriceHistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterComparison {
    pub level_difference: i64,
    pub same_world: bool,
    pub same_vocation: bool,
    /// Difference is right minus left. Only keys supported on both sides occur.
    pub skill_differences: Option<BTreeMap<StatsSkill, i64>>,
}

#[derive(Debug)]
pub struct BazaarStats<S> {
    source: S,
    flags: StatsFlags,
}

impl<S: StatsSource> BazaarStats<S> {
    pub fn new(source: S, flags: StatsFlags) -> Self {
        Self { source, flags }
    }

    async fn projection(&self, now: OffsetDateTime) -> Result<StatsProjection, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        let projection = self.source.read().await?.ok_or(Error::Denied)?;
        if projection.observed_at > now
            || projection.valid_until <= now
            || projection.valid_until <= projection.observed_at
            || projection.observations.len() > MAX_ROWS
            || projection.characters.len() > MAX_ROWS
        {
            return Err(Error::Denied);
        }
        let mut seen = BTreeSet::new();
        for row in &projection.observations {
            row.asset.validate()?;
            if row.listing.is_nil()
                || row.world.is_nil()
                || !seen.insert(row.listing)
                || row.closed_at > projection.observed_at
            {
                return Err(Error::Denied);
            }
            match row.outcome {
                StatsOutcome::Completed {
                    unit_price,
                    quantity,
                } => {
                    if unit_price <= 0
                        || quantity == 0
                        || (row.currency == Currency::Gold
                            && i128::from(unit_price) * i128::from(quantity) > 999_999_999_999)
                        || (matches!(row.asset, StatsAsset::Character { .. }) && quantity != 1)
                    {
                        return Err(Error::InvalidInput);
                    }
                }
                StatsOutcome::Unsold if !row.auction => return Err(Error::Denied),
                StatsOutcome::Unsold => {}
            }
        }
        seen.clear();
        for character in &projection.characters {
            StatsAsset::Character {
                vocation: character.vocation,
                level: character.level,
            }
            .validate()?;
            if character.listing.is_nil()
                || character.world.is_nil()
                || !seen.insert(character.listing)
            {
                return Err(Error::Denied);
            }
        }
        Ok(projection)
    }

    /// Every call rereads current privacy evidence; responses must not be publicly cached.
    /// No samples or small cohorts are unavailable (None), never a zero market price.
    pub async fn summary(
        &self,
        query: &StatsQuery,
        now: OffsetDateTime,
    ) -> Result<Option<StatsSummary>, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        query.validate(now)?;
        let projection = self.projection(now).await?;
        if query.until > projection.observed_at {
            return Err(Error::Denied);
        }
        summarize(
            projection
                .observations
                .iter()
                .filter(|row| query.matches(row)),
        )
    }

    /// Completed character sales grouped by vocation, without account grouping.
    /// Suppression applies independently to every profession and to each day.
    pub async fn vocation_breakdown(
        &self,
        world: Uuid,
        currency: Currency,
        from: OffsetDateTime,
        until: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<BTreeMap<u8, StatsSummary>, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        StatsQuery {
            world,
            currency,
            asset: StatsAsset::Character {
                vocation: 0,
                level: 1,
            },
            from,
            until,
        }
        .validate(now)?;
        let projection = self.projection(now).await?;
        if until > projection.observed_at {
            return Err(Error::Denied);
        }
        let mut result = BTreeMap::new();
        for vocation in 0..=20 {
            let summary = summarize(projection.observations.iter().filter(|row| {
                row.world == world && row.currency == currency && row.closed_at >= from && row.closed_at < until
                    && matches!(row.asset, StatsAsset::Character { vocation: actual, .. } if actual == vocation)
            }))?;
            if let Some(summary) = summary {
                result.insert(vocation, summary);
            }
        }
        Ok(result)
    }

    /// Conservative similarity: same world/vocation and level within ten percent.
    /// No skill/item premium is invented. Returned estimate is a cohort median,
    /// not a guaranteed selling price. Missing approved character denies.
    pub async fn estimate_character(
        &self,
        listing: Uuid,
        currency: Currency,
        from: OffsetDateTime,
        until: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<Option<StatsSummary>, Error> {
        let projection = self.projection(now).await?;
        if until > projection.observed_at {
            return Err(Error::Denied);
        }
        let character = projection
            .characters
            .iter()
            .find(|c| c.listing == listing)
            .ok_or(Error::NotFound)?;
        let query = StatsQuery {
            world: character.world,
            currency,
            asset: StatsAsset::Character {
                vocation: character.vocation,
                level: character.level,
            },
            from,
            until,
        };
        query.validate(now)?;
        summarize(projection.observations.iter().filter(|row| {
            row.listing != listing && row.world == character.world && row.currency == currency
                && row.closed_at >= from && row.closed_at < until
                && matches!(row.asset, StatsAsset::Character { vocation, level } if vocation == character.vocation && u64::from(level.abs_diff(character.level)) * 10 <= u64::from(character.level))
        }))
    }

    pub async fn compare_characters(
        &self,
        left: Uuid,
        right: Uuid,
        now: OffsetDateTime,
    ) -> Result<CharacterComparison, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        if left.is_nil() || right.is_nil() || left == right {
            return Err(Error::InvalidInput);
        }
        let projection = self.projection(now).await?;
        let left = projection
            .characters
            .iter()
            .find(|c| c.listing == left)
            .ok_or(Error::NotFound)?;
        let right = projection
            .characters
            .iter()
            .find(|c| c.listing == right)
            .ok_or(Error::NotFound)?;
        let skill_differences = left
            .skills
            .as_ref()
            .zip(right.skills.as_ref())
            .map(|(a, b)| {
                a.iter()
                    .filter_map(|(key, value)| {
                        b.get(key)
                            .map(|other| (*key, i64::from(*other) - i64::from(*value)))
                    })
                    .collect()
            });
        Ok(CharacterComparison {
            level_difference: i64::from(right.level) - i64::from(left.level),
            same_world: left.world == right.world,
            same_vocation: left.vocation == right.vocation,
            skill_differences,
        })
    }
}

fn median(prices: &mut [i64]) -> MarketPrice {
    prices.sort_unstable();
    let middle = prices.len() / 2;
    let upper = i128::from(prices[middle]);
    let lower = if prices.len().is_multiple_of(2) {
        i128::from(prices[middle - 1])
    } else {
        upper
    };
    MarketPrice {
        twice_units: lower + upper,
    }
}

fn summarize<'a>(
    rows: impl Iterator<Item = &'a StatsObservation>,
) -> Result<Option<StatsSummary>, Error> {
    let mut prices = Vec::new();
    let mut daily = BTreeMap::<Date, Vec<i64>>::new();
    let mut turnover = 0_i128;
    let mut auctions = 0;
    let mut successful = 0;
    for row in rows {
        if row.auction {
            auctions += 1;
        }
        if let StatsOutcome::Completed {
            unit_price,
            quantity,
        } = row.outcome
        {
            prices.push(unit_price);
            daily
                .entry(row.closed_at.to_offset(time::UtcOffset::UTC).date())
                .or_default()
                .push(unit_price);
            turnover = turnover
                .checked_add(i128::from(unit_price) * i128::from(quantity))
                .ok_or(Error::Overflow)?;
            if row.auction {
                successful += 1;
            }
        }
    }
    if prices.len() < MIN_SAMPLES {
        return Ok(None);
    }
    let market_price = median(&mut prices);
    let record_unit_price = prices[prices.len() - 1];
    let history = daily
        .into_iter()
        .filter(|(_, p)| p.len() >= MIN_SAMPLES)
        .map(|(day, mut p)| PriceHistoryPoint {
            day,
            sales: p.len(),
            market_price: median(&mut p),
        })
        .collect();
    Ok(Some(StatsSummary {
        sales: prices.len(),
        turnover,
        market_price,
        record_unit_price,
        auction_success: (auctions >= MIN_SAMPLES).then_some((successful, auctions)),
        history,
    }))
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
