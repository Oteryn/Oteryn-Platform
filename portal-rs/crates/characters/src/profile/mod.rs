//! Platform-owned preferences. Projection facts only establish presentation ownership;
//! they never authorize commands against the game.
mod http;
use crate::lcfa::{AccountId, CharacterId, Settings};
pub use http::{edit, update};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotFound,
    Unavailable,
    BadInput,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preferences {
    pub show_guild: bool,
    pub is_main_character: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_guild: true,
            is_main_character: false,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Service {
    pool: PgPool,
    settings: Settings,
    enabled: bool,
}
impl Service {
    pub fn new(pool: PgPool, settings: Settings, enabled: bool) -> Self {
        Self {
            pool,
            settings,
            enabled,
        }
    }
    async fn lock(&self) -> Result<Transaction<'_, Postgres>, Error> {
        if !self.enabled {
            return Err(Error::NotFound);
        }
        let s = &self.settings;
        // Preserve LCFA's environment and configuration restrictions. The locked
        // read below must share the preference transaction, rather than opening
        // a second projection transaction that can deadlock behind a publisher.
        if !s.enabled
            || !matches!(
                s.environment,
                oteryn_core::Environment::Testing | oteryn_core::Environment::Preprod
            )
            || !(1..=128).contains(&s.source_authority.len())
            || !s
                .source_authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
            || !(1..=120).contains(&s.freshness_seconds)
            || s.clock_uncertainty_seconds > 5
            || s.clock_uncertainty_seconds >= s.freshness_seconds
            || !(1..=600).contains(&s.requests_per_minute)
        {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await?;
        // Publishers lock the feed exclusively before replacing snapshots/rows.
        sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR SHARE")
            .fetch_one(&mut *tx)
            .await?;
        Ok(tx)
    }
    async fn characters(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        now: i64,
    ) -> Result<Vec<(Uuid, String)>, Error> {
        AccountId::parse(&account.to_string()).map_err(|_| Error::NotFound)?;
        let status = sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(&mut **tx)
            .await?;
        if status.as_deref() != Some("active") {
            return Err(Error::NotFound);
        }
        let feed = sqlx::query("SELECT highest_epoch,watermark_epoch,source_authority,complete_through FROM account_character_feed WHERE id=1").fetch_one(&mut **tx).await?;
        let highest: Option<String> = feed.try_get("highest_epoch")?;
        let watermark: Option<String> = feed.try_get("watermark_epoch")?;
        let authority: Option<String> = feed.try_get("source_authority")?;
        let complete: Option<i64> = feed.try_get("complete_through")?;
        let s = &self.settings;
        if now < 0
            || highest.is_none()
            || highest != watermark
            || authority.as_deref() != Some(&s.source_authority)
            || !complete.is_some_and(|t| {
                t <= now.saturating_add(i64::from(s.clock_uncertainty_seconds))
                    && now
                        .saturating_sub(t)
                        .saturating_add(i64::from(s.clock_uncertainty_seconds))
                        <= i64::from(s.freshness_seconds)
            })
        {
            return Err(Error::Unavailable);
        }
        let snapshot = sqlx::query("SELECT projection_epoch,source_authority,invalid FROM account_character_snapshots WHERE account_id=$1").bind(account).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
        if snapshot.try_get::<bool, _>("invalid")?
            || Some(snapshot.try_get::<String, _>("projection_epoch")?) != highest
            || snapshot.try_get::<String, _>("source_authority")? != s.source_authority
        {
            return Err(Error::Unavailable);
        }
        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT character_id,name FROM account_characters WHERE account_id=$1",
        )
        .bind(account)
        .fetch_all(&mut **tx)
        .await?;
        for (id, _) in &rows {
            CharacterId::parse(&id.to_string()).map_err(|_| Error::Unavailable)?;
        }
        Ok(rows)
    }
    async fn owned(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        name: &str,
        now: i64,
    ) -> Result<(Uuid, String), Error> {
        if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
            return Err(Error::BadInput);
        }
        let all = self.characters(tx, account, now).await?;
        let selected = if let Some(c) = all.iter().find(|c| c.1 == name) {
            c.clone()
        } else {
            let matches: Vec<_> = all
                .into_iter()
                .filter(|c| c.1.eq_ignore_ascii_case(name))
                .collect();
            if matches.len() != 1 {
                return Err(Error::NotFound);
            }
            matches.into_iter().next().ok_or(Error::NotFound)?
        };
        Self::unique_owner(tx, selected.0).await?;
        Ok(selected)
    }
    async fn unique_owner(
        tx: &mut Transaction<'_, Postgres>,
        character: Uuid,
    ) -> Result<(), Error> {
        let count: i64 = sqlx::query_scalar("SELECT count(DISTINCT c.account_id) FROM account_characters c JOIN accounts a ON a.id=c.account_id JOIN account_character_snapshots s ON s.account_id=c.account_id JOIN account_character_feed f ON f.id=1 WHERE c.character_id=$1 AND a.status='active' AND NOT s.invalid AND s.projection_epoch=f.highest_epoch AND s.source_authority=f.source_authority").bind(character).fetch_one(&mut **tx).await?;
        if count != 1 {
            return Err(Error::NotFound);
        }
        Ok(())
    }
    async fn preferences(
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Uuid,
    ) -> Result<Preferences, Error> {
        let row = sqlx::query_as::<_, (bool,bool)>("SELECT show_guild,is_main_character FROM character_profiles WHERE account_id=$1 AND character_id=$2").bind(account).bind(character).fetch_optional(&mut **tx).await?;
        if let Some((show_guild, is_main_character)) = row {
            return Ok(Preferences {
                show_guild,
                is_main_character,
            });
        }
        let hidden: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_profiles WHERE account_id=$1 AND character_id IS NULL AND NOT show_guild)").bind(account).fetch_one(&mut **tx).await?;
        Ok(Preferences {
            show_guild: !hidden,
            is_main_character: false,
        })
    }
    pub async fn edit(
        &self,
        account: Uuid,
        name: &str,
        now: i64,
    ) -> Result<(String, Preferences), Error> {
        let mut tx = self.lock().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let c = self.owned(&mut tx, account, name, now).await?;
        let p = Self::preferences(&mut tx, account, c.0).await?;
        tx.commit().await?;
        Ok((c.1, p))
    }
    pub async fn update(
        &self,
        account: Uuid,
        name: &str,
        value: Preferences,
        now: i64,
    ) -> Result<(), Error> {
        // Retry only rolled-back serialization/deadlock failures, with bounded attempts.
        for attempt in 0..3 {
            match self.save(account, name, value, now).await {
                Err(SaveError::Sql(e))
                    if attempt < 2
                        && e.as_database_error().is_some_and(|e| {
                            matches!(e.code().as_deref(), Some("40001" | "40P01"))
                        }) =>
                {
                    continue;
                }
                Ok(()) => return Ok(()),
                Err(SaveError::Domain(e)) => return Err(e),
                Err(SaveError::Sql(_)) => return Err(Error::Unavailable),
            }
        }
        Err(Error::Unavailable)
    }
    async fn save(
        &self,
        account: Uuid,
        name: &str,
        value: Preferences,
        now: i64,
    ) -> Result<(), SaveError> {
        let mut tx = self.lock().await.map_err(SaveError::Domain)?;
        let status =
            sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_optional(&mut *tx)
                .await?;
        if status.as_deref() != Some("active") {
            return Err(SaveError::Domain(Error::NotFound));
        }
        let c = self
            .owned(&mut tx, account, name, now)
            .await
            .map_err(SaveError::Domain)?;
        let id = c.0;
        let previous = Self::preferences(&mut tx, account, id)
            .await
            .map_err(SaveError::Domain)?;
        if value.is_main_character {
            sqlx::query("UPDATE character_profiles SET is_main_character=false,updated_at=now() WHERE account_id=$1 AND is_main_character AND character_id IS DISTINCT FROM $2").bind(account).bind(id).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO character_profiles(account_id,character_id,show_guild,is_main_character) VALUES($1,$2,$3,$4) ON CONFLICT(account_id,character_id) DO UPDATE SET show_guild=$3,is_main_character=$4,updated_at=now()").bind(account).bind(id).bind(value.show_guild).bind(value.is_main_character).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_profile_events(account_id,event_type) VALUES($1,'character.profile_preferences_updated')").bind(account).execute(&mut *tx).await?;
        if value.is_main_character && !previous.is_main_character {
            sqlx::query("INSERT INTO character_profile_events(account_id,event_type) VALUES($1,'character.main_character_selected')").bind(account).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    /// Resolves exactly one current active owner. Missing or ambiguous state denies.
    /// Game's guild permission remains an upper bound; this API cannot widen it.
    pub async fn public_preferences(&self, character: Uuid, now: i64) -> Option<Preferences> {
        self.read_public(character, now).await.ok()
    }
    async fn read_public(&self, character: Uuid, now: i64) -> Result<Preferences, Error> {
        let mut tx = self.lock().await?;
        let candidates = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT account_id FROM account_characters WHERE character_id=$1",
        )
        .bind(character)
        .fetch_all(&mut *tx)
        .await?;
        let mut owners = Vec::new();
        for account in candidates {
            match self.characters(&mut tx, account, now).await {
                Ok(chars) if chars.iter().any(|c| c.0 == character) => owners.push(account),
                Ok(_) | Err(Error::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
        if owners.len() != 1 {
            return Err(Error::NotFound);
        }
        // The Rust identity schema has one identity per canonical account.
        sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE")
            .bind(owners[0])
            .fetch_one(&mut *tx)
            .await?;
        let p = Self::preferences(&mut tx, owners[0], character).await?;
        tx.commit().await?;
        Ok(p)
    }
    /// Returns the saved main only while it belongs to this live, active account.
    /// The caller supplies the authenticated session account for private views.
    pub async fn main_character(&self, account: Uuid, now: i64) -> Option<Uuid> {
        self.read_main(account, now).await.ok().flatten()
    }
    async fn read_main(&self, account: Uuid, now: i64) -> Result<Option<Uuid>, Error> {
        let mut tx = self.lock().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let characters = self.characters(&mut tx, account, now).await?;
        let main = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT character_id FROM character_profiles WHERE account_id=$1 AND is_main_character",
        )
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let main = main.filter(|id| characters.iter().any(|c| c.0 == *id));
        if let Some(id) = main {
            Self::unique_owner(&mut tx, id).await?;
        }
        tx.commit().await?;
        Ok(main)
    }
    pub async fn guild_visible(&self, character: Uuid, game_visible: bool, now: i64) -> bool {
        game_visible
            && self
                .public_preferences(character, now)
                .await
                .is_some_and(|p| p.show_guild)
    }
}
enum SaveError {
    Domain(Error),
    Sql(sqlx::Error),
}
impl From<sqlx::Error> for SaveError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sql(e)
    }
}
#[cfg(test)]
mod tests;
