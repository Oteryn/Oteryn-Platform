//! Explicit profile selections. Missing authority or source facts never imply eligibility.
use sqlx::{PgConnection, PgPool};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
};
use uuid::Uuid;

pub mod http;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("feature disabled")]
    Disabled,
    #[error("invalid selection")]
    Invalid,
    #[error("selection denied")]
    Denied,
    #[error("facts unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub badges: Vec<String>,
    pub achievements: Vec<String>,
    pub profile: Option<Uuid>,
    pub loyalty: Option<Uuid>,
}
#[derive(Debug, Clone)]
pub struct Character {
    pub id: Uuid,
    pub owner: Uuid,
    pub name: String,
    /// Durable ownership generation, not a display name or family watermark.
    pub ownership: String,
    pub public: bool,
    /// None until G-4 attribution, catalogue and ownership-transition floor are proven.
    pub earned: Option<BTreeSet<String>>,
}
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Server-computed eligible badges; values are whole years, points or levels.
    pub badges: Vec<(String, i64)>,
    pub characters: Vec<Character>,
    /// Exact compatible catalogue names, English then Polish; never client text.
    pub achievement_names: BTreeMap<String, (String, String)>,
}
/// T45/G-4 adapter must lock current ownership and privacy through this connection.
/// It must refuse stale, ambiguous, transferred or incomplete data. Levels must cover
/// all owned characters/professions; loyalty must use consumed Premium ledger days.
/// No public projection or client-supplied fact can implement this authority.
pub trait Evidence: Send + Sync {
    fn load<'a>(
        &'a self,
        account: Uuid,
        connection: &'a mut PgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<Facts, Error>> + Send + 'a>>;
}
/// Safe fallback while the T45 profile dependency and G-4 are unavailable.
#[derive(Debug)]
pub struct LocalEvidence;
impl Evidence for LocalEvidence {
    fn load<'a>(
        &'a self,
        account: Uuid,
        connection: &'a mut PgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<Facts, Error>> + Send + 'a>> {
        Box::pin(async move {
            let years: i64 = sqlx::query_scalar("SELECT EXTRACT(YEAR FROM age(CURRENT_TIMESTAMP,created_at))::bigint FROM accounts WHERE id=$1 AND created_at<=CURRENT_TIMESTAMP")
                .bind(account).fetch_optional(connection).await?.ok_or(Error::Unavailable)?;
            Ok(Facts {
                badges: vec![("account-age".into(), years)],
                characters: vec![],
                achievement_names: BTreeMap::new(),
            })
        })
    }
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn character(facts: &Facts, id: Uuid) -> Result<&Character, Error> {
    let mut rows = facts.characters.iter().filter(|c| c.id == id);
    let row = rows.next().ok_or(Error::Denied)?;
    if rows.next().is_some() || !valid_id(id) || !row.public || row.ownership.is_empty() {
        return Err(Error::Denied);
    }
    Ok(row)
}
fn validate(
    selection: &Selection,
    facts: &Facts,
) -> Result<(Option<String>, Option<String>), Error> {
    if selection.badges.len() > 4
        || selection.achievements.len() > 5
        || selection.badges.iter().collect::<BTreeSet<_>>().len() != selection.badges.len()
        || selection.achievements.iter().collect::<BTreeSet<_>>().len()
            != selection.achievements.len()
    {
        return Err(Error::Invalid);
    }
    for key in &selection.badges {
        if !["account-age", "loyalty", "total-levels", "all-professions"].contains(&key.as_str()) {
            return Err(Error::Invalid);
        }
        if facts.badges.iter().filter(|(k, _)| k == key).count() != 1
            || facts.badges.iter().any(|(k, v)| k == key && *v < 0)
        {
            return Err(Error::Unavailable);
        }
    }
    for key in &selection.achievements {
        let slug = key
            .strip_prefix("oteryn:achievement/")
            .ok_or(Error::Invalid)?;
        if key.len() > 160
            || slug.is_empty()
            || !slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(Error::Invalid);
        }
    }
    let profile = selection
        .profile
        .map(|id| character(facts, id))
        .transpose()?;
    if !selection.achievements.is_empty() {
        let earned = profile
            .ok_or(Error::Denied)?
            .earned
            .as_ref()
            .ok_or(Error::Unavailable)?;
        if selection
            .achievements
            .iter()
            .any(|key| !earned.contains(key))
        {
            return Err(Error::Denied);
        }
    }
    for key in &selection.achievements {
        if !facts
            .achievement_names
            .get(key)
            .is_some_and(|names| !names.0.is_empty() && !names.1.is_empty())
        {
            return Err(Error::Unavailable);
        }
    }
    let loyalty = selection
        .loyalty
        .map(|id| character(facts, id))
        .transpose()?;
    Ok((
        profile.map(|c| c.ownership.clone()),
        loyalty.map(|c| c.ownership.clone()),
    ))
}

#[derive(Debug)]
pub struct Service<E> {
    pub pool: PgPool,
    pub enabled: bool,
    pub evidence: E,
}
#[derive(Debug, Clone)]
pub struct OwnerView {
    pub selection: Selection,
    pub facts: Facts,
}
/// Only approved values, never earned times, totals of achievements or account IDs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublicView {
    pub badges: Vec<(String, i64)>,
    pub achievements: Vec<String>,
}

type Stored = (
    Vec<String>,
    Vec<String>,
    Option<Uuid>,
    Option<String>,
    Option<Uuid>,
    Option<String>,
);
impl<E: Evidence> Service<E> {
    async fn begin(&self, account: Uuid) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !valid_id(account) {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_optional(&mut *tx)
                .await?;
        if status.as_deref() != Some("active") {
            return Err(Error::Denied);
        }
        Ok(tx)
    }
    async fn stored(connection: &mut PgConnection, account: Uuid) -> Result<Stored, Error> {
        Ok(sqlx::query_as("SELECT badge_keys,achievement_keys,profile_id,profile_ownership,loyalty_id,loyalty_ownership FROM account_badge_selections WHERE account_id=$1")
            .bind(account).fetch_optional(connection).await?.unwrap_or_default())
    }
    pub async fn owner(&self, account: Uuid) -> Result<OwnerView, Error> {
        let mut tx = self.begin(account).await?;
        let facts = self.evidence.load(account, &mut tx).await?;
        if facts
            .characters
            .iter()
            .any(|character| character.owner != account)
        {
            return Err(Error::Denied);
        }
        let stored = Self::stored(&mut tx, account).await?;
        let selection = Selection {
            badges: stored.0,
            achievements: stored.1,
            profile: stored.2,
            loyalty: stored.4,
        };
        // Revalidate at every read; a changed owner/hidden character cannot revive a selection.
        let selection = match validate(&selection, &facts) {
            Ok((p, l)) if p == stored.3 && l == stored.5 => selection,
            _ => Selection::default(),
        };
        tx.commit().await?;
        Ok(OwnerView { selection, facts })
    }
    /// Caller must authenticate the browser session and verify synchronizer CSRF before calling.
    pub async fn save(&self, account: Uuid, selection: &Selection) -> Result<(), Error> {
        let mut tx = self.begin(account).await?;
        let facts = self.evidence.load(account, &mut tx).await?;
        if facts
            .characters
            .iter()
            .any(|character| character.owner != account)
        {
            return Err(Error::Denied);
        }
        let (profile, loyalty) = validate(selection, &facts)?;
        sqlx::query("INSERT INTO account_badge_selections(account_id,badge_keys,achievement_keys,profile_id,profile_ownership,loyalty_id,loyalty_ownership) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(account_id) DO UPDATE SET badge_keys=$2,achievement_keys=$3,profile_id=$4,profile_ownership=$5,loyalty_id=$6,loyalty_ownership=$7,updated_at=now()")
            .bind(account).bind(&selection.badges).bind(&selection.achievements).bind(selection.profile).bind(profile).bind(selection.loyalty).bind(loyalty).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Trusted profile resolver supplies the owner; callers never accept it from public input.
    pub async fn public(&self, owner: Uuid, profile: Uuid) -> Result<PublicView, Error> {
        let view = self.owner(owner).await?;
        if view.selection.profile != Some(profile) {
            return Ok(PublicView::default());
        }
        Ok(PublicView {
            badges: view
                .facts
                .badges
                .into_iter()
                .filter(|(k, _)| view.selection.badges.contains(k))
                .collect(),
            achievements: view.selection.achievements,
        })
    }
    /// Ranking consumers revalidate this before each materialization/serve.
    pub async fn loyalty_character(&self, owner: Uuid) -> Result<Option<Uuid>, Error> {
        Ok(self.owner(owner).await?.selection.loyalty)
    }
}
