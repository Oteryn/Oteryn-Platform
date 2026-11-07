//! Serve-time privacy authority. No cached allow and no name-based ownership join.
use super::{Error, Preferences, Service};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone, Debug)]
pub struct Authority {
    pub service: Service,
}
impl Authority {
    pub fn configured(pool: PgPool) -> Self {
        Self {
            service: Service::new(
                pool,
                crate::overview::configured_projection(),
                std::env::var("OTERYN_CHARACTER_PROFILES_ENABLED").as_deref() == Ok("true"),
            ),
        }
    }
    pub async fn ready(&self) -> Result<(), Error> {
        let mut tx = self.service.lock().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_secs()).ok())
            .ok_or(Error::Unavailable)?;
        self.service.feed(&mut tx, now).await?;
        sqlx::query("SELECT hidden,comment,show_status FROM character_profiles LIMIT 1")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT character_profiles_hidden,privacy_revision FROM accounts LIMIT 1")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn reference(
        &self,
        character: Uuid,
        world: &str,
        name: &str,
        now: i64,
    ) -> Result<Option<Preferences>, Error> {
        match self
            .service
            .read_public_reference(character, now, Some((world, name)))
            .await
        {
            Ok(p) => Ok(Some(p)),
            Err(Error::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub async fn current(&self, character: Uuid, now: i64) -> Result<Option<Preferences>, Error> {
        match self.service.read_public(character, now).await {
            Ok(p) => Ok(Some(p)),
            Err(Error::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
