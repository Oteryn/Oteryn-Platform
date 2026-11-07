//! Platform-owned preferences. Projection facts only establish presentation ownership;
//! they never authorize commands against the game.
pub mod authority;
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
    RateLimited,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Preferences {
    pub comment: String,
    pub hidden: bool,
    pub hide_all_characters: bool,
    pub show_account: bool,
    pub show_status: bool,
    pub show_house: bool,
    pub show_skills: bool,
    pub show_deaths: bool,
    pub show_kills: bool,
    pub show_online_time: bool,

    pub show_guild: bool,
    pub is_main_character: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_guild: false,
            is_main_character: false,
            comment: String::new(),
            hidden: true,
            hide_all_characters: false,
            show_account: false,
            show_status: false,
            show_house: false,
            show_skills: false,
            show_deaths: false,
            show_kills: false,
            show_online_time: false,
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
    async fn feed(&self, tx: &mut Transaction<'_, Postgres>, now: i64) -> Result<String, Error> {
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
        highest.ok_or(Error::Unavailable)
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
        let highest = self.feed(tx, now).await?;
        let s = &self.settings;
        let snapshot = sqlx::query("SELECT projection_epoch,source_authority,invalid FROM account_character_snapshots WHERE account_id=$1").bind(account).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
        if snapshot.try_get::<bool, _>("invalid")?
            || Some(snapshot.try_get::<String, _>("projection_epoch")?) != Some(highest)
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
        let row = sqlx::query_as::<_, Preferences>("SELECT p.show_guild,p.is_main_character,p.hidden,p.comment,p.show_account,p.show_status,p.show_house,p.show_skills,p.show_deaths,p.show_kills,p.show_online_time,a.character_profiles_hidden AS hide_all_characters FROM character_profiles p JOIN accounts a ON a.id=p.account_id WHERE p.account_id=$1 AND p.character_id=$2").bind(account).bind(character).fetch_optional(&mut **tx).await?;
        if let Some(row) = row {
            return Ok(row);
        }
        let hide_all_characters: bool =
            sqlx::query_scalar("SELECT character_profiles_hidden FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&mut **tx)
                .await?;
        Ok(Preferences {
            hide_all_characters,
            ..Preferences::default()
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
        if value.comment.chars().count() > 500
            || value.comment.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(Error::BadInput);
        }
        // Retry only rolled-back serialization/deadlock failures, with bounded attempts.
        for attempt in 0..3 {
            match self.save(account, name, value.clone(), now).await {
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
        let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM character_profile_events WHERE account_id=$1 AND event_type='character.profile_preferences_updated' AND created_at > now()-interval '1 minute'").bind(account).fetch_one(&mut *tx).await?;
        if attempts >= 30 {
            return Err(SaveError::Domain(Error::RateLimited));
        }
        sqlx::query("UPDATE accounts SET character_profiles_hidden=$2,privacy_revision=privacy_revision+1 WHERE id=$1").bind(account).bind(value.hide_all_characters).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_profiles(account_id,character_id,show_guild,is_main_character,hidden,comment,show_account,show_status,show_house,show_skills,show_deaths,show_kills,show_online_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT(account_id,character_id) DO UPDATE SET show_guild=$3,is_main_character=$4,hidden=$5,comment=$6,show_account=$7,show_status=$8,show_house=$9,show_skills=$10,show_deaths=$11,show_kills=$12,show_online_time=$13,updated_at=now()")
            .bind(account).bind(id).bind(value.show_guild).bind(value.is_main_character).bind(value.hidden).bind(value.comment).bind(value.show_account).bind(value.show_status).bind(value.show_house).bind(value.show_skills).bind(value.show_deaths).bind(value.show_kills).bind(value.show_online_time).execute(&mut *tx).await?;
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
        self.read_public_reference(character, now, None).await
    }
    async fn read_public_reference(
        &self,
        character: Uuid,
        now: i64,
        reference: Option<(&str, &str)>,
    ) -> Result<Preferences, Error> {
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
        if let Some((world, name)) = reference {
            let matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_characters WHERE account_id=$1 AND character_id=$2 AND world_id::text=$3 AND name=$4)").bind(owners[0]).bind(character).bind(world).bind(name).fetch_one(&mut *tx).await?;
            if !matches {
                return Err(Error::NotFound);
            }
        }
        // The Rust identity schema has one identity per canonical account.
        sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE")
            .bind(owners[0])
            .fetch_one(&mut *tx)
            .await?;
        let saved: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_profiles WHERE account_id=$1 AND character_id=$2)").bind(owners[0]).bind(character).fetch_one(&mut *tx).await?;
        if !saved {
            return Err(Error::NotFound);
        }
        let mut p = Self::preferences(&mut tx, owners[0], character).await?;
        // Same current T09c policy and verified-account predicate, in the locked
        // transaction: no second pool checkout while ownership/account locks are held.
        let account = sqlx::query_as::<_, oteryn_identity::privacy::Preferences>("SELECT public_account_association,public_status_visible,locale,privacy_revision FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL").bind(owners[0]).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        p.hidden |= p.hide_all_characters;
        p.show_account &= account.public_account_association;
        p.show_status &= account.public_status_visible;
        p.show_online_time &= account.public_status_visible;
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
                .is_some_and(|p| !p.hidden && p.show_guild)
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
