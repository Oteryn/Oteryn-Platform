//! Privacy-filtered discovery. No game I/O and no private snapshot deserialization.
//!
//! T59/T60 supply the approved projection through `SearchProjection`. The adapter must
//! read current publication consent, field policy, clock safety, highest epoch and
//! watermark in the supplied transaction. A signature alone is never publication consent.
//! Unsupported catalog semantics must be omitted, never inferred from private snapshots.
use std::{collections::BTreeMap, future::Future};

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{Accounts, Error};

/// Independent discovery gates. Enabling the trading core never enables public
/// projections or account preferences. Keep all gates off pending G-7 qualification.
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchFlags {
    pub public_listings_enabled: bool,
    pub account_features_enabled: bool,
    pub gold_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchTab {
    #[default]
    Current,
    Ended,
    MyBids,
    MyListings,
    Watched,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Ending,
    Newest,
    Oldest,
    EndingDesc,
    PriceAsc,
    PriceDesc,
    LevelAsc,
    LevelDesc,
    SkillAsc,
    SkillDesc,
}

/// Closed field names prevent arbitrary snapshot keys becoming searchable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    World,
    Pvp,
    Location,
    Vocation,
    Category,
    ItemType,
    Item,
    StoreGrant,
    Outfit,
    Mount,
    Quest,
    Achievement,
    Attribute,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchQuery {
    pub text: String,
    pub kind: Option<String>,
    pub form: Option<String>,
    pub currency: Option<String>,
    pub level_min: Option<i64>,
    pub level_max: Option<i64>,
    pub achievement_points_min: Option<i64>,
    pub achievement_points_max: Option<i64>,
    pub price_min: Option<i64>,
    pub price_max: Option<i64>,
    pub skill: Option<String>,
    pub skill_min: Option<i64>,
    pub skill_max: Option<i64>,
    pub facets: BTreeMap<SearchField, String>,
    pub with_bids: bool,
    pub rare_name: bool,
    pub with_price: bool,
    pub tab: SearchTab,
    pub sort: SearchSort,
    /// Zero-based page, 24 results per page, matching the existing public listing query.
    pub page: u32,
}

impl SearchQuery {
    pub fn validate(&self) -> Result<(), Error> {
        fn range(low: Option<i64>, high: Option<i64>, floor: i64, ceiling: i64) -> bool {
            low.is_none_or(|v| (floor..=ceiling).contains(&v))
                && high.is_none_or(|v| (floor..=ceiling).contains(&v))
                && low.zip(high).is_none_or(|(a, b)| a <= b)
        }
        fn choice(value: &Option<String>, allowed: &[&str]) -> bool {
            value.as_ref().is_none_or(|v| allowed.contains(&v.as_str()))
        }
        if self.text.len() > 100
            || self.text.chars().any(char::is_control)
            || self.page > 10_000
            || !choice(&self.kind, &["character", "item"])
            || !choice(&self.form, &["auction", "fixed_price", "want_to_buy"])
            || !choice(&self.currency, &["coins", "gold"])
            || !choice(
                &self.skill,
                &[
                    "fist",
                    "club",
                    "sword",
                    "axe",
                    "distance",
                    "shielding",
                    "fishing",
                    "magic",
                ],
            )
            || !range(self.level_min, self.level_max, 1, 5000)
            || !range(
                self.achievement_points_min,
                self.achievement_points_max,
                0,
                i64::MAX,
            )
            || !range(self.price_min, self.price_max, 0, 999_999_999_999)
            || !range(self.skill_min, self.skill_max, 0, i64::MAX)
            || ((self.skill_min.is_some()
                || self.skill_max.is_some()
                || matches!(self.sort, SearchSort::SkillAsc | SearchSort::SkillDesc))
                && self.skill.is_none())
            || self
                .facets
                .values()
                .any(|v| v.is_empty() || v.len() > 128 || v.chars().any(char::is_control))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

/// Only explicitly approved visible facts. Never include owner, bidder, proxy maximum,
/// signature envelope, account grouping or the underlying private game snapshot.
#[derive(Debug, Clone)]
pub struct SearchListing {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub form: String,
    pub currency: String,
    pub price: Option<i64>,
    pub level: Option<i64>,
    pub achievement_points: Option<i64>,
    pub skills: BTreeMap<String, i64>,
    pub facets: BTreeMap<SearchField, Vec<String>>,
    pub has_bids: bool,
    pub rare_name: Option<bool>,
    pub created_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub ended: bool,
    /// Current, approved final sale price; not an uncommitted bid or proxy maximum.
    pub sale_price: Option<i64>,
}

/// Private selection metadata is consumed before results are returned.
#[derive(Debug)]
pub struct SearchCandidate {
    pub listing: SearchListing,
    pub publication_authorized: bool,
    pub privacy_current: bool,
    pub clock_safe: bool,
    pub projection_epoch: u64,
    pub highest_epoch: u64,
    pub complete_through: OffsetDateTime,
    pub observed_at: OffsetDateTime,
    pub mine: bool,
    pub my_bid: bool,
}

impl SearchCandidate {
    fn visible(&self, now: OffsetDateTime) -> bool {
        !self.listing.id.is_nil()
            && matches!(self.listing.kind.as_str(), "character" | "item")
            && matches!(
                self.listing.form.as_str(),
                "auction" | "fixed_price" | "want_to_buy"
            )
            && (self.listing.kind != "character" || self.listing.form == "auction")
            && matches!(self.listing.currency.as_str(), "coins" | "gold")
            && self
                .listing
                .price
                .is_none_or(|p| p > 0 && p <= 999_999_999_999)
            && self
                .listing
                .sale_price
                .is_none_or(|p| p > 0 && p <= 999_999_999_999)
            && self.listing.ends_at > self.listing.created_at
            && self.publication_authorized
            && self.privacy_current
            && self.clock_safe
            && self.projection_epoch > 0
            && self.projection_epoch == self.highest_epoch
            && self.complete_through <= self.observed_at
            && self.observed_at <= now + Duration::seconds(1)
            && now - self.complete_through + Duration::seconds(1) <= Duration::seconds(30)
    }
}

pub trait SearchProjection: Send + Sync {
    /// Return all candidates for a consistent read, at most 10,000; overflow must
    /// error, never truncate. Actor comes from authentication, never query input.
    /// Unknown state, unapproved fields and cancelled/draft listings must be absent.
    fn candidates(
        &self,
        db: &mut PgConnection,
        actor: Option<Uuid>,
    ) -> impl Future<Output = Result<Vec<SearchCandidate>, Error>> + Send;
}

#[derive(Debug)]
pub struct SearchPage {
    pub listings: Vec<SearchListing>,
    pub total: usize,
}

fn matches_query(row: &SearchListing, q: &SearchQuery) -> bool {
    fn within(value: Option<i64>, low: Option<i64>, high: Option<i64>) -> bool {
        (low.is_none() && high.is_none())
            || value.is_some_and(|v| low.is_none_or(|l| v >= l) && high.is_none_or(|h| v <= h))
    }
    q.kind.as_ref().is_none_or(|v| v == &row.kind)
        && q.form.as_ref().is_none_or(|v| v == &row.form)
        && q.currency.as_ref().is_none_or(|v| v == &row.currency)
        && row
            .name
            .to_lowercase()
            .contains(&q.text.trim().to_lowercase())
        && within(row.level, q.level_min, q.level_max)
        && within(
            row.achievement_points,
            q.achievement_points_min,
            q.achievement_points_max,
        )
        && within(row.price, q.price_min, q.price_max)
        && q.skill.as_ref().is_none_or(|s| {
            row.skills
                .get(s)
                .is_some_and(|v| within(Some(*v), q.skill_min, q.skill_max))
        })
        && q.facets.iter().all(|(key, value)| {
            row.facets
                .get(key)
                .is_some_and(|values| values.contains(value))
        })
        && (!q.with_bids || row.has_bids)
        && (!q.rare_name || row.rare_name == Some(true))
        && (!q.with_price || row.price.is_some())
        && (!matches!(q.sort, SearchSort::PriceAsc | SearchSort::PriceDesc) || row.price.is_some())
        && (!matches!(q.sort, SearchSort::LevelAsc | SearchSort::LevelDesc) || row.level.is_some())
}

#[derive(Debug)]
pub struct BazaarSearch<A, P> {
    pool: PgPool,
    accounts: A,
    projection: P,
    flags: SearchFlags,
}

impl<A: Accounts, P: SearchProjection> BazaarSearch<A, P> {
    pub fn new(pool: PgPool, accounts: A, projection: P, flags: SearchFlags) -> Self {
        Self {
            pool,
            accounts,
            projection,
            flags,
        }
    }

    fn enabled(&self) -> Result<(), Error> {
        if self.flags.public_listings_enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }

    async fn authorize(&self, db: &mut PgConnection, actor: Uuid) -> Result<(), Error> {
        if !self.flags.account_features_enabled {
            return Err(Error::Disabled);
        }
        if actor.is_nil()
            || !self
                .accounts
                .state(db, actor)
                .await?
                .is_some_and(|a| a.active)
        {
            return Err(Error::Denied);
        }
        Ok(())
    }

    pub async fn search(
        &self,
        actor: Option<Uuid>,
        query: &SearchQuery,
    ) -> Result<SearchPage, Error> {
        self.enabled()?;
        query.validate()?;
        if actor.is_none() && !matches!(query.tab, SearchTab::Current | SearchTab::Ended) {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        if let Some(actor) = actor {
            self.authorize(&mut tx, actor).await?;
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT transaction_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let watched: Vec<Uuid> = if query.tab == SearchTab::Watched {
            sqlx::query_scalar("SELECT listing_id FROM bazaar_watches WHERE account=$1")
                .bind(actor)
                .fetch_all(&mut *tx)
                .await?
        } else {
            Vec::new()
        };
        let candidates = self.projection.candidates(&mut tx, actor).await?;
        if candidates.len() > 10_000 {
            return Err(Error::Denied);
        }
        let mut listings: Vec<_> = candidates
            .into_iter()
            .filter(|r| {
                r.visible(now)
                    && (self.flags.gold_enabled || r.listing.currency != "gold")
                    && match query.tab {
                        SearchTab::Current => !r.listing.ended && r.listing.ends_at > now,
                        SearchTab::Ended => r.listing.ended,
                        SearchTab::MyBids => r.my_bid,
                        SearchTab::MyListings => r.mine,
                        SearchTab::Watched => watched.contains(&r.listing.id),
                    }
            })
            .map(|r| {
                let mut listing = r.listing;
                if listing.ended {
                    listing.price = listing.sale_price;
                }
                listing
            })
            .filter(|r| matches_query(r, query))
            .collect();
        listings.sort_by(|a, b| {
            let skill =
                |r: &SearchListing| query.skill.as_ref().and_then(|s| r.skills.get(s)).copied();
            let order = match query.sort {
                SearchSort::Ending => a.ends_at.cmp(&b.ends_at),
                SearchSort::Newest => b.created_at.cmp(&a.created_at),
                SearchSort::Oldest => a.created_at.cmp(&b.created_at),
                SearchSort::EndingDesc => b.ends_at.cmp(&a.ends_at),
                SearchSort::PriceAsc => a.price.cmp(&b.price),
                SearchSort::PriceDesc => b.price.cmp(&a.price),
                SearchSort::LevelAsc => a.level.cmp(&b.level),
                SearchSort::LevelDesc => b.level.cmp(&a.level),
                SearchSort::SkillAsc => skill(a).cmp(&skill(b)),
                SearchSort::SkillDesc => skill(b).cmp(&skill(a)),
            };
            order
                .then_with(|| a.ends_at.cmp(&b.ends_at))
                .then(a.id.cmp(&b.id))
        });
        let total = listings.len();
        let listings = listings
            .into_iter()
            .skip(query.page as usize * 24)
            .take(24)
            .collect();
        tx.commit().await?;
        Ok(SearchPage { listings, total })
    }

    /// Idempotent set/remove, scoped to the authenticated account. Removing a hidden
    /// listing remains possible; adding one requires current public evidence.
    pub async fn watch(&self, actor: Uuid, listing: Uuid, enabled: bool) -> Result<(), Error> {
        self.enabled()?;
        if listing.is_nil() {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        if enabled {
            let now: OffsetDateTime = sqlx::query_scalar("SELECT transaction_timestamp()")
                .fetch_one(&mut *tx)
                .await?;
            if !self
                .projection
                .candidates(&mut tx, Some(actor))
                .await?
                .iter()
                .any(|r| {
                    r.listing.id == listing
                        && r.visible(now)
                        && (self.flags.gold_enabled || r.listing.currency != "gold")
                })
            {
                return Err(Error::NotFound);
            }
            sqlx::query("INSERT INTO bazaar_watches(account,listing_id) VALUES($1,$2) ON CONFLICT DO NOTHING")
                .bind(actor).bind(listing).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM bazaar_watches WHERE account=$1 AND listing_id=$2")
                .bind(actor)
                .bind(listing)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Names are account-local keys. Saving twice replaces the same named query.
    /// Saved queries contain criteria only; reads always recheck privacy and freshness.
    pub async fn save(&self, actor: Uuid, name: &str, query: &SearchQuery) -> Result<(), Error> {
        self.enabled()?;
        query.validate()?;
        validate_name(name)?;
        let body = serde_json::to_string(query).map_err(|_| Error::InvalidInput)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,61))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_saved_searches WHERE account=$1 AND name<>$2",
        )
        .bind(actor)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        if count >= 50 {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO bazaar_saved_searches(account,name,query) VALUES($1,$2,$3) ON CONFLICT(account,name) DO UPDATE SET query=EXCLUDED.query,updated_at=clock_timestamp()")
            .bind(actor).bind(name).bind(body).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn saved(&self, actor: Uuid) -> Result<Vec<(String, SearchQuery)>, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT name,query FROM bazaar_saved_searches WHERE account=$1 ORDER BY name",
        )
        .bind(actor)
        .fetch_all(&mut *tx)
        .await?;
        rows.into_iter()
            .map(|(name, body)| {
                let query: SearchQuery =
                    serde_json::from_str(&body).map_err(|_| Error::InvalidInput)?;
                query.validate()?;
                Ok((name, query))
            })
            .collect()
    }

    pub async fn delete_saved(&self, actor: Uuid, name: &str) -> Result<(), Error> {
        self.enabled()?;
        validate_name(name)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        sqlx::query("DELETE FROM bazaar_saved_searches WHERE account=$1 AND name=$2")
            .bind(actor)
            .bind(name)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 80
        || name.trim() != name
        || name.chars().any(char::is_control)
    {
        Err(Error::InvalidInput)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
