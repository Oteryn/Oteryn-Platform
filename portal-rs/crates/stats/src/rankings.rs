//! Portal-derived rankings over durable daily statistics, never game highscores.
//! Current public metadata and privacy decisions are required on every read.
use sqlx::{PgPool, Row};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};
use time::Date;
use uuid::Uuid;

#[path = "rankings_http.rs"]
mod http;
pub use http::{configured_router, router};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Yesterday,
    Week,
    Month,
    Record,
    Oldest,
    Anniversaries,
    NameChanges,
    Transfers,
    Sold,
}
impl Kind {
    pub const ALL: [(Self, &'static str); 9] = [
        (Self::Yesterday, "yesterday"),
        (Self::Week, "week"),
        (Self::Month, "month"),
        (Self::Record, "record"),
        (Self::Oldest, "oldest"),
        (Self::Anniversaries, "anniversaries"),
        (Self::NameChanges, "name-changes"),
        (Self::Transfers, "transfers"),
        (Self::Sold, "sold"),
    ];
}
#[derive(Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub name: String,
    pub world: Uuid,
    pub world_name: String,
    pub vocation: u8,
    pub pvp: Pvp,
    /// Authoritative creation date, never the first observation date.
    pub created: Option<Date>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pvp {
    Open,
    Optional,
    Hardcore,
}
pub type CatalogFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Character>, Error>> + Send + 'a>>;
/// Return only currently visible, active characters, with current scope metadata.
/// Unknown, stale or failed privacy/topology state must return Unavailable.
/// An empty result must mean an authoritative empty public population.
/// Implementations must honor public-projection epoch/watermark fences.
/// Confirm coverage for the requested kind; absent creation, event or experience
/// sources are unavailable, including when there are no local rows.
pub trait PublicCatalog: Send + Sync {
    fn characters(&self, kind: Kind) -> CatalogFuture<'_>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Invalid,
    Unavailable,
    NotFound,
}
#[derive(Clone, Debug, Default)]
pub struct Filters {
    pub world: Option<Uuid>,
    pub vocation: Option<u8>,
    pub pvp: Option<Pvp>,
    pub page: usize,
}
#[derive(Debug)]
pub struct Entry {
    pub character: Character,
    pub value: String,
    pub day: Date,
}
#[derive(Debug)]
pub struct Ranking {
    pub worlds: Vec<(String, String)>,
    pub rows: Vec<Entry>,
    pub total: usize,
    pub page: usize,
}
#[derive(Clone)]
pub struct Rankings {
    pub db: PgPool,
    pub enabled: bool,
    pub catalog: Arc<dyn PublicCatalog>,
}
impl std::fmt::Debug for Rankings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rankings")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}
impl Rankings {
    pub async fn load(&self, kind: Kind, today: Date, filters: &Filters) -> Result<Ranking, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if filters.page == 0 || filters.page > 20_000 || filters.vocation.is_some_and(|v| v > 5) {
            return Err(Error::Invalid);
        }
        let public = self.catalog.characters(kind).await?;
        if public.len() > 100_000 {
            return Err(Error::Unavailable);
        }
        let mut characters = BTreeMap::new();
        for c in public {
            if c.name.is_empty()
                || c.name.len() > 256
                || c.world_name.is_empty()
                || c.world_name.len() > 256
                || c.vocation > 5
                || c.created.is_some_and(|d| d > today)
                || characters.contains_key(&c.id)
            {
                return Err(Error::Unavailable);
            }
            characters.insert(c.id, c);
        }
        if filters
            .world
            .is_some_and(|w| !characters.values().any(|c| c.world == w))
        {
            return Err(Error::NotFound);
        }
        let mut worlds = BTreeMap::new();
        for c in characters.values() {
            if let Some((name, pvp)) = worlds.insert(c.world, (c.world_name.clone(), c.pvp))
                && (name != c.world_name || pvp != c.pvp)
            {
                return Err(Error::Unavailable);
            }
        }
        let worlds = worlds
            .into_iter()
            .map(|(id, (name, _))| (id.to_string(), name))
            .collect();
        characters.retain(|_, c| {
            filters.world.is_none_or(|w| c.world == w)
                && filters.vocation.is_none_or(|v| c.vocation == v)
                && filters.pvp.is_none_or(|p| c.pvp == p)
        });
        let ids: Vec<Uuid> = characters.keys().copied().collect();
        let mut entries = Vec::new();
        if matches!(kind, Kind::Oldest | Kind::Anniversaries) {
            for c in characters.values() {
                let created = c.created.ok_or(Error::Unavailable)?;
                if kind == Kind::Oldest
                    || (created.month(), created.day()) == (today.month(), today.day())
                {
                    entries.push(Entry {
                        character: c.clone(),
                        value: (today.year()
                            - created.year()
                            - i32::from(
                                (today.month(), today.day()) < (created.month(), created.day()),
                            ))
                        .to_string(),
                        day: created,
                    });
                }
            }
            entries.sort_by_key(|e| (e.day, e.character.id));
        } else {
            let end = today.previous_day().ok_or(Error::Invalid)?;
            let days = match kind {
                Kind::Week => 7,
                Kind::Month => 30,
                _ => 1,
            };
            let start = end
                .checked_sub(time::Duration::days(days))
                .ok_or(Error::Invalid)?;
            let rows = match kind {
                Kind::Yesterday | Kind::Week | Kind::Month => sqlx::query("SELECT a.character_ref,(b.experience-a.experience)::text AS value,b.day FROM stats_character_daily a JOIN stats_character_daily b USING(character_ref) WHERE a.character_ref=ANY($3) AND a.day=$1 AND b.day=$2 AND a.experience IS NOT NULL AND b.experience IS NOT NULL ORDER BY b.experience-a.experience DESC,a.character_ref LIMIT 100001")
                    .bind(start).bind(end).bind(&ids).fetch_all(&self.db).await,
                Kind::Record => sqlx::query("WITH gains AS (SELECT b.character_ref,b.day,b.experience-a.experience AS gain FROM stats_character_daily a JOIN stats_character_daily b ON a.character_ref=b.character_ref AND b.day=a.day+1 WHERE b.character_ref=ANY($2) AND b.day <= $1 AND a.experience IS NOT NULL AND b.experience IS NOT NULL), best AS (SELECT DISTINCT ON (character_ref) character_ref,day,gain FROM gains ORDER BY character_ref,gain DESC,day) SELECT character_ref,day,gain::text AS value FROM best ORDER BY gain DESC,character_ref LIMIT 100001")
                    .bind(end).bind(&ids).fetch_all(&self.db).await,
                _ => {
                    let event = match kind { Kind::NameChanges => "name_change", Kind::Transfers => "world_change", Kind::Sold => "bazaar_sale", _ => return Err(Error::Invalid) };
                    sqlx::query("SELECT character_ref,day,kind AS value FROM stats_character_events WHERE character_ref=ANY($3) AND kind=$1 AND day <= $2 ORDER BY day DESC,character_ref,event_key LIMIT 100001")
                        .bind(event).bind(today).bind(&ids).fetch_all(&self.db).await
                }
            }.map_err(|_| Error::Unavailable)?;
            if rows.len() > 100_000 {
                return Err(Error::Unavailable);
            }
            for row in rows {
                let id = row
                    .try_get("character_ref")
                    .map_err(|_| Error::Unavailable)?;
                if let Some(c) = characters.get(&id) {
                    entries.push(Entry {
                        character: c.clone(),
                        value: row.try_get("value").map_err(|_| Error::Unavailable)?,
                        day: row.try_get("day").map_err(|_| Error::Unavailable)?,
                    });
                }
            }
            // Missing experience is unavailable, never a fabricated zero or empty ranking.
            if entries.is_empty()
                && !characters.is_empty()
                && matches!(
                    kind,
                    Kind::Yesterday | Kind::Week | Kind::Month | Kind::Record
                )
            {
                return Err(Error::Unavailable);
            }
        }
        let total = entries.len();
        if filters.page > total.div_ceil(50).max(1) {
            return Err(Error::NotFound);
        }
        Ok(Ranking {
            worlds,
            rows: entries
                .into_iter()
                .skip((filters.page - 1) * 50)
                .take(50)
                .collect(),
            total,
            page: filters.page,
        })
    }
}

#[cfg(test)]
#[path = "rankings_tests.rs"]
mod tests;

#[path = "rankings_online.rs"]
pub mod online;
