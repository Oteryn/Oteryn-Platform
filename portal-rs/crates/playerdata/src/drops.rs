//! Private drops, records and chronicle derived from T73's proven complete sessions.
//!
//! This is a projection seam, never a substitute for the G-9 wire receiver. The
//! upstream adapter must validate canonical bytes, source partition, catalogue,
//! contiguous applied watermark, segment indices and exact rare-drop counts.
//! No HTTP ingest is mounted until that adapter and purpose-bound guard exist.

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

/// Independently controlled capabilities; disabled even when a database exists.
#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub ingest: bool,
    pub read: bool,
}

/// The transfer receipt cut, not a separate ownership epoch or revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cut {
    pub character_revision: i64,
    pub projection_revision: i64,
    pub projection_epoch: i64,
    pub recovery_generation: i64,
}

impl Cut {
    fn valid(self) -> bool {
        [
            self.character_revision,
            self.projection_revision,
            self.projection_epoch,
            self.recovery_generation,
        ]
        .into_iter()
        .all(|v| v > 0)
    }
}

/// T72/T73 adapter contract. Implementations must consult current durable fences,
/// erasure and consent state, not cached request claims. Unknown state returns None.
/// `session` returns only durable-logout, complete sessions in the current cut;
/// hunting-area and inactivity segments alone never establish a complete session.
/// Calls occur inside the projection transaction, so adapters can lock authority
/// rows against concurrent transfer, deletion and restore. No permissive adapter
/// is supplied by this module.
#[allow(async_fn_in_trait)]
pub trait Journal {
    async fn current_cut(
        &self,
        tx: &mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
    ) -> Result<Option<Cut>, Error>;
    async fn session(
        &self,
        tx: &mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
        session: Uuid,
    ) -> Result<Option<Session>, Error>;
}

/// A rare event already included in session loot; never sum it into loot again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drop {
    pub event_id: Uuid,
    pub occurred_at: i64,
    pub catalog_revision: String,
    pub item_key: String,
    pub quantity: i64,
    pub monster_key: Option<String>,
    pub place_key: Option<String>,
    pub rarity_rule_key: String,
}

/// Platform valuation is optional. Missing valuation is unavailable, never zero.
/// Values use the same pinned price basis for the whole session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Valuation {
    pub basis: String,
    pub loot: i64,
    pub supplies: i64,
}

/// Internal normalized T73 result, not a G-9 wire schema. IDs are journal IDs,
/// unrelated to admission, runtime session or lease capabilities. Boss-first
/// evidence is deliberately absent: G-9 kill totals do not prove a first kill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub session_id: Uuid,
    pub cut: Cut,
    pub started_at: i64,
    pub ended_at: i64,
    pub active_seconds: i64,
    pub experience_gained: i64,
    pub valuation: Option<Valuation>,
    pub drops: Vec<Drop>,
}

fn key(value: &str) -> bool {
    let Some((prefix, suffix)) = value.split_once(':') else {
        return false;
    };
    (3..=512).contains(&value.len())
        && !prefix.is_empty()
        && !suffix.is_empty()
        && prefix.as_bytes()[0].is_ascii_alphanumeric()
        && prefix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        && suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b))
}

impl Session {
    fn validate(&self) -> Result<(), Error> {
        if self.session_id.get_version_num() != 7
            || !self.cut.valid()
            || self.started_at <= 0
            || self.ended_at < self.started_at
            || self.ended_at > 253402300799
            || self.active_seconds < 0
            || self.active_seconds > self.ended_at - self.started_at
            || self.experience_gained < 0
            || self.drops.len() > 65535
        {
            return Err(Error::BadInput);
        }
        if let Some(v) = &self.valuation
            && (v.loot < 0
                || v.supplies < 0
                || v.basis.is_empty()
                || v.basis.len() > 128
                || !v
                    .basis
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b)))
        {
            return Err(Error::BadInput);
        }
        let mut ids = std::collections::BTreeSet::new();
        for drop in &self.drops {
            if drop.event_id.get_version_num() != 7
                || !ids.insert(drop.event_id)
                || drop.occurred_at < self.started_at
                || drop.occurred_at >= self.ended_at
                || drop.quantity <= 0
                || !key(&drop.item_key)
                || !key(&drop.rarity_rule_key)
                || drop.monster_key.as_deref().is_some_and(|v| !key(v))
                || drop.place_key.as_deref().is_some_and(|v| !key(v))
                || drop.catalog_revision.is_empty()
                || drop.catalog_revision.len() > 128
                || !drop
                    .catalog_revision
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
            {
                return Err(Error::BadInput);
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("feature disabled")]
    Disabled,
    #[error("journal unavailable")]
    Unavailable,
    #[error("access denied")]
    Denied,
    #[error("invalid journal data")]
    BadInput,
    #[error("immutable session conflict")]
    Conflict,
    #[error("storage failure")]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct Repository {
    pool: PgPool,
    flags: Flags,
}

impl Repository {
    pub fn new(pool: PgPool, flags: Flags) -> Self {
        Self { pool, flags }
    }

    /// Materialize only a source-proven complete session. Exact retries are inert.
    /// Raw requests cannot call this with their own payload or ownership claims.
    pub async fn ingest(
        &self,
        source: &impl Journal,
        account: Uuid,
        character: Uuid,
        id: Uuid,
    ) -> Result<(), Error> {
        if !self.flags.ingest {
            return Err(Error::Disabled);
        }
        validate_ids(&[account, character, id])?;
        let mut tx = self.pool.begin().await?;
        let cut = source
            .current_cut(&mut tx, account, character)
            .await?
            .ok_or(Error::Unavailable)?;
        let session = source
            .session(&mut tx, account, character, id)
            .await?
            .ok_or(Error::Unavailable)?;
        session.validate()?;
        if session.cut != cut || session.session_id != id {
            return Err(Error::Unavailable);
        }
        let inserted = sqlx::query("INSERT INTO pd_drop_sessions (account_id, character_id, session_id, character_revision, projection_revision, projection_epoch, recovery_generation, payload) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(account).bind(character).bind(id).bind(cut.character_revision)
            .bind(cut.projection_revision).bind(cut.projection_epoch).bind(cut.recovery_generation)
            .bind(Json(&session)).execute(&mut *tx).await?;
        if inserted.rows_affected() == 0 {
            let (stored,): (Json<Session>,) = sqlx::query_as("SELECT payload FROM pd_drop_sessions WHERE account_id=$1 AND character_id=$2 AND session_id=$3")
                .bind(account).bind(character).bind(id).fetch_one(&mut *tx).await?;
            if stored.0 != session {
                return Err(Error::Conflict);
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// The authenticated actor must be supplied by the session layer, never a URL
    /// or form. Staff, sharing links and previous owners receive no exception.
    /// Every read rechecks authority and excludes obsolete transfer/restore cuts.
    pub async fn read(
        &self,
        source: &impl Journal,
        actor: Uuid,
        account: Uuid,
        character: Uuid,
    ) -> Result<Presentation, Error> {
        if !self.flags.read {
            return Err(Error::Disabled);
        }
        if actor != account {
            return Err(Error::Denied);
        }
        validate_ids(&[actor, account, character])?;
        let mut tx = self.pool.begin().await?;
        let cut = source
            .current_cut(&mut tx, account, character)
            .await?
            .filter(|c| c.valid())
            .ok_or(Error::Unavailable)?;
        let rows: Vec<(Json<Session>,)> = sqlx::query_as("SELECT payload FROM pd_drop_sessions WHERE account_id=$1 AND character_id=$2 AND character_revision=$3 AND projection_revision=$4 AND projection_epoch=$5 AND recovery_generation=$6 ORDER BY session_id")
            .bind(account).bind(character).bind(cut.character_revision).bind(cut.projection_revision)
            .bind(cut.projection_epoch).bind(cut.recovery_generation).fetch_all(&mut *tx).await?;
        let mut sessions = Vec::with_capacity(rows.len());
        for (Json(session),) in rows {
            session.validate()?;
            // Revalidate completeness against the upstream applied watermark.
            if source
                .session(&mut tx, account, character, session.session_id)
                .await?
                .as_ref()
                != Some(&session)
            {
                return Err(Error::Unavailable);
            }
            sessions.push(session);
        }
        tx.commit().await?;
        Presentation::from_sessions(sessions)
    }
}

fn validate_ids(ids: &[Uuid]) -> Result<(), Error> {
    if ids.iter().all(|id| id.get_version_num() == 7) {
        Ok(())
    } else {
        Err(Error::BadInput)
    }
}

/// Exact rational rates avoid overflow, rounding and fabricated zero-duration rates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rate {
    pub numerator: i128,
    pub seconds: i64,
}
impl Rate {
    fn better_than(&self, other: &Self) -> bool {
        self.numerator * i128::from(other.seconds) > other.numerator * i128::from(self.seconds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Record<T> {
    pub session_id: Uuid,
    pub value: T,
}

/// Private presentation model. Empty history and first-boss evidence are explicitly
/// unavailable. Records cover only the proven visible history, never all-time claims.
/// Chronicle entries retain occurrence times and immutable journal session links.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Presentation {
    pub drops: Vec<Drop>,
    pub chronicle: Vec<Session>,
    pub best_experience_per_hour: Option<Record<Rate>>,
    pub best_profit_per_hour_by_basis: std::collections::BTreeMap<String, Record<Rate>>,
    pub largest_loot_by_basis: std::collections::BTreeMap<String, Record<i64>>,
    pub longest_session: Option<Record<i64>>,
    pub first_boss_kill: Option<Uuid>,
}

impl Presentation {
    fn from_sessions(mut sessions: Vec<Session>) -> Result<Self, Error> {
        sessions.sort_by_key(|s| (s.ended_at, s.session_id));
        let mut result = Self::default();
        let mut event_ids = std::collections::BTreeSet::new();
        for s in &sessions {
            s.validate()?;
            for drop in &s.drops {
                if !event_ids.insert(drop.event_id) {
                    return Err(Error::Conflict);
                }
                result.drops.push(drop.clone());
            }
            if result
                .longest_session
                .as_ref()
                .is_none_or(|r| s.active_seconds > r.value)
            {
                result.longest_session = Some(Record {
                    session_id: s.session_id,
                    value: s.active_seconds,
                });
            }
            if s.active_seconds > 0 {
                let rate = Rate {
                    numerator: i128::from(s.experience_gained) * 3600,
                    seconds: s.active_seconds,
                };
                if result
                    .best_experience_per_hour
                    .as_ref()
                    .is_none_or(|r| rate.better_than(&r.value))
                {
                    result.best_experience_per_hour = Some(Record {
                        session_id: s.session_id,
                        value: rate,
                    });
                }
            }
            if let Some(v) = &s.valuation {
                if result
                    .largest_loot_by_basis
                    .get(&v.basis)
                    .is_none_or(|r| v.loot > r.value)
                {
                    result.largest_loot_by_basis.insert(
                        v.basis.clone(),
                        Record {
                            session_id: s.session_id,
                            value: v.loot,
                        },
                    );
                }
                if s.active_seconds > 0 {
                    let rate = Rate {
                        numerator: (i128::from(v.loot) - i128::from(v.supplies)) * 3600,
                        seconds: s.active_seconds,
                    };
                    if result
                        .best_profit_per_hour_by_basis
                        .get(&v.basis)
                        .is_none_or(|r| rate.better_than(&r.value))
                    {
                        result.best_profit_per_hour_by_basis.insert(
                            v.basis.clone(),
                            Record {
                                session_id: s.session_id,
                                value: rate,
                            },
                        );
                    }
                }
            }
        }
        result
            .drops
            .sort_by_key(|d| (std::cmp::Reverse(d.occurred_at), d.event_id));
        sessions.reverse();
        result.chronicle = sessions;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "drops/tests.rs"]
mod tests;

#[path = "drops/view.rs"]
mod view;
pub use view::Language;

impl Repository {
    /// SSR adapter for the authenticated panel. The host supplies its normal
    /// security headers and Cache-Control: no-store; no private share route exists.
    pub async fn page(
        &self,
        source: &impl Journal,
        actor: Uuid,
        account: Uuid,
        character: Uuid,
        language: Language,
    ) -> Result<String, Error> {
        let data = self.read(source, actor, account, character).await?;
        view::render(&data, language)
    }
}
