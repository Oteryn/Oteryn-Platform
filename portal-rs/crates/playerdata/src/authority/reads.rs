//! Read-side consumers of the T72 journal: the G-11 login cut and promotion,
//! and the G-10 item-read evidence. Each re-proves ownership in its own
//! transaction with [`super::ownership`]; journal rows are never ownership.
use super::{Ownership, account, account_epoch, ownership, restore_generation, shared_fences};
use crate::items::{
    model::{self as item_model, Cut as ItemCut, Item},
    store::{Access, Definition, Evidence, Proof, Scope as ItemScope},
};
use crate::logins::{self, AuthorityFuture, Cut, Login, Logins, PromotedLogin};
use crate::store::{self, Consumer, ConsumerFuture, Error};
use async_trait::async_trait;
use oteryn_internal_api::playerdata::{Record, Scope};
use sqlx::{PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;

fn login_error(e: Error) -> logins::Error {
    match e {
        Error::Invalid => logins::Error::Invalid,
        Error::Denied => logins::Error::Denied,
        Error::Conflict => logins::Error::Conflict,
        Error::Unavailable => logins::Error::Unavailable,
    }
}
fn from_login(e: logins::Error) -> Error {
    match e {
        logins::Error::Invalid => Error::Invalid,
        logins::Error::Denied => Error::Denied,
        logins::Error::Conflict => Error::Conflict,
        logins::Error::Unavailable | logins::Error::Database(_) => Error::Unavailable,
    }
}
fn item_error(e: Error) -> item_model::Error {
    match e {
        Error::Invalid => item_model::Error::Invalid,
        Error::Denied => item_model::Error::Denied,
        Error::Conflict => item_model::Error::Conflict,
        Error::Unavailable => item_model::Error::Unavailable,
    }
}

/// A receipt decision time (unix seconds) as a login visibility bound.
fn bound_ms(decided_at: u64) -> Result<i64, Error> {
    i64::try_from(decided_at)
        .ok()
        .and_then(|s| s.checked_mul(1000))
        .ok_or(Error::Unavailable)
}

/// The G-11 `game_login` cut of one account (and optionally one character):
/// the published watermark of the account's stream at its current epoch, and
/// the transfer-receipt cut of a character the account owns now.
#[derive(Debug, Clone)]
pub struct JournalLoginAuthority {
    /// The configured G-11 source authority (`PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY`).
    pub source: String,
}
impl JournalLoginAuthority {
    async fn cut(
        &self,
        tx: &mut PgConnection,
        account_id: Uuid,
        character: Option<Uuid>,
    ) -> Result<Cut, Error> {
        shared_fences(tx).await?;
        account(tx, account_id).await?;
        let restored = restore_generation(tx).await?;
        // A character the account never owned is denied before any stream; a
        // former owner's cut is its own period, closed at the transfer cut.
        let owned = match character {
            None => None,
            Some(character) => {
                if crate::store::character_deleted(
                    tx,
                    &account_id.to_string(),
                    &character.to_string(),
                )
                .await?
                {
                    return Err(Error::Denied);
                }
                match ownership(tx, account_id, character).await? {
                    Ownership::Owner(owned) => Some((owned, None)),
                    Ownership::Former(period) => Some((period.cut, Some(period.until))),
                    Ownership::Stranger => return Err(Error::Denied),
                }
            }
        };
        let epoch = account_epoch(tx, account_id).await?;
        let key = store::stream_key(&Scope {
            authority: self.source.clone(),
            account: account_id.to_string(),
            world: None,
            character: None,
            character_revision: None,
            projection_revision: None,
            from_projection_revision: None,
            epoch: Some(epoch.to_string()),
            recovery: String::new(),
            family: "game_login".to_owned(),
        });
        let row = sqlx::query("SELECT published_through::text AS through,observed_ms,baseline_complete,watermark_recovery::text AS recovery,erased FROM pd_streams WHERE stream_key=$1 FOR SHARE")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Unavailable)?;
        // A baseline counts only once a watermark proved it, never from silence.
        let recovery = row
            .try_get::<Option<String>, _>("recovery")?
            .ok_or(Error::Unavailable)?;
        let recovery = super::counter(&recovery)?;
        if row.try_get::<bool, _>("erased")? || recovery < restored {
            return Err(Error::Unavailable);
        }
        let mut cut = Cut {
            epoch,
            recovery_generation: recovery,
            complete_through: super::counter(&row.try_get::<String, _>("through")?)?,
            observed_at_ms: row.try_get("observed_ms")?,
            // The store refused every watermark whose trusted time was more
            // than 5 s uncertain; nothing larger can reach this row.
            time_uncertainty_ms: 0,
            baseline_complete: row.try_get("baseline_complete")?,
            visibility_floor_ms: 0,
            visibility_ceiling_ms: None,
            projection_revision: None,
            ownership_character_revision: None,
            world_id: None,
        };
        if let Some((owned, until)) = owned {
            if owned.epoch != epoch {
                return Err(Error::Unavailable);
            }
            cut.projection_revision = Some(owned.to_projection_revision);
            cut.ownership_character_revision = Some(owned.character_revision);
            cut.world_id = Some(owned.world);
            cut.visibility_floor_ms = bound_ms(owned.decided_at)?;
            cut.visibility_ceiling_ms = until.map(bound_ms).transpose()?;
        }
        Ok(cut)
    }
}
impl logins::Authority for JournalLoginAuthority {
    fn current<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Option<Uuid>,
    ) -> AuthorityFuture<'a> {
        Box::pin(async move { self.cut(tx, account, character).await.map_err(login_error) })
    }
}

/// Promotes `game_login` records into the login projection when a watermark
/// with a proven baseline publishes them, in the watermark's transaction.
#[derive(Debug, Clone)]
pub struct LoginPromoter {
    pub logins: Logins,
}
impl LoginPromoter {
    async fn run(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        key: &str,
        scope: &Scope,
        after: u64,
        through: u64,
    ) -> Result<(), Error> {
        if scope.family != "game_login" {
            return Ok(());
        }
        let account_id = Uuid::parse_str(&scope.account).map_err(|_| Error::Invalid)?;
        let rows = sqlx::query("SELECT source_revision::text AS revision,kind,entity_key,character_id,payload FROM pd_records WHERE stream_key=$1 AND source_revision>$2::numeric AND source_revision<=$3::numeric ORDER BY source_revision")
            .bind(key)
            .bind(after.to_string())
            .bind(through.to_string())
            .fetch_all(&mut **tx)
            .await?;
        for row in rows {
            let entity: String = row.try_get("entity_key")?;
            if row.try_get::<String, _>("kind")? == "REMOVE" {
                let id = Uuid::parse_str(&entity).map_err(|_| Error::Unavailable)?;
                Logins::remove(tx, account_id, id)
                    .await
                    .map_err(from_login)?;
                continue;
            }
            // Expired payloads and terminal refusals project nothing.
            let Some(payload) = row.try_get::<Option<Vec<u8>>, _>("payload")? else {
                continue;
            };
            let character = row
                .try_get::<Option<String>, _>("character_id")?
                .map(|c| Uuid::parse_str(&c))
                .transpose()
                .map_err(|_| Error::Unavailable)?;
            // A character the account never owned, or a deleted scope, projects
            // nothing; a sold one projects only inside the seller's period,
            // which `apply` bounds by the transfer cut.
            let cut = match self
                .logins
                .authority
                .current(tx, account_id, character)
                .await
            {
                Ok(cut) => cut,
                Err(logins::Error::Denied) if character.is_some() => continue,
                Err(e) => return Err(from_login(e)),
            };
            let record = PromotedLogin {
                account: account_id,
                character,
                // The fence admitted the record at or above the ownership cut.
                character_revision: cut.ownership_character_revision,
                cut,
                source_revision: super::counter(&row.try_get::<String, _>("revision")?)?,
                login: Login::parse(&payload).map_err(from_login)?,
            };
            match self.logins.apply(tx, &record).await {
                // Before the transfer floor or ahead of the trusted clock: not shown.
                Err(logins::Error::Denied) => continue,
                other => other.map_err(from_login)?,
            }
        }
        Ok(())
    }
}
impl Consumer for LoginPromoter {
    fn record<'a>(
        &'a self,
        _: &'a mut Transaction<'static, Postgres>,
        _: &'a Record,
    ) -> ConsumerFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn promote<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        stream_key: &'a str,
        scope: &'a Scope,
        after: u64,
        through: u64,
    ) -> ConsumerFuture<'a> {
        Box::pin(self.run(tx, stream_key, scope, after, through))
    }
}

/// G-10 item-read evidence. Reads need the current owner at the current cut
/// and a verified baseline; no G-10 baseline verification exists yet, so after
/// the ownership proof every read and the projection's own ingest stay
/// unavailable, and definitions wait for the accepted catalog mapping.
#[derive(Debug, Clone)]
pub struct JournalEvidence {
    /// The configured G-10 source authority (`PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY`).
    pub source: String,
}
impl JournalEvidence {
    async fn current(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        world: Uuid,
        character: Option<Uuid>,
        family: &str,
    ) -> Result<ItemScope, Error> {
        shared_fences(db).await?;
        account(db, actor).await?;
        let restored = restore_generation(db).await?;
        let cut = match character {
            None => None,
            Some(character) => {
                if crate::store::character_deleted(db, &actor.to_string(), &character.to_string())
                    .await?
                {
                    return Err(Error::Denied);
                }
                let Ownership::Owner(cut) = ownership(db, actor, character).await? else {
                    return Err(Error::Denied);
                };
                if cut.world != world {
                    return Err(Error::Denied);
                }
                Some(cut)
            }
        };
        let recovery = cut.as_ref().map_or(restored, |c| c.recovery.max(restored));
        Ok(ItemScope {
            source_authority: self.source.clone(),
            account_id: actor.to_string(),
            world_id: world.to_string(),
            character_id: character.map(|c| c.to_string()),
            cut: cut.map(|c| ItemCut {
                character_revision: c.character_revision.to_string(),
                from_projection_revision: c.from_projection_revision.to_string(),
                to_projection_revision: c.to_projection_revision.to_string(),
                projection_epoch: c.epoch.to_string(),
                recovery_generation: c.recovery.to_string(),
            }),
            family: family.to_owned(),
            recovery_generation: recovery.to_string(),
        })
    }
    async fn prove(
        &self,
        db: &mut PgConnection,
        scope: &ItemScope,
        access: Access,
    ) -> Result<Proof, Error> {
        let Access::Read(actor) = access else {
            // The T72 store is the only ingest; the T76 projection ingest stays off.
            return Err(Error::Unavailable);
        };
        if scope.account_id != actor.to_string() || scope.source_authority != self.source {
            return Err(Error::Denied);
        }
        let world = Uuid::parse_str(&scope.world_id).map_err(|_| Error::Denied)?;
        let character = scope
            .character_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| Error::Denied)?;
        // The buyer's scope is the new cut only; an older cut is no longer current.
        let now = self
            .current(db, actor, world, character, &scope.family)
            .await?;
        if now.cut != scope.cut {
            return Err(Error::Denied);
        }
        Err(Error::Unavailable)
    }
}
#[async_trait]
impl Evidence for JournalEvidence {
    async fn authorize(
        &self,
        db: &mut PgConnection,
        scope: &ItemScope,
        access: Access,
    ) -> item_model::Result<Proof> {
        self.prove(db, scope, access).await.map_err(item_error)
    }
    async fn definition(&self, _: &mut PgConnection, _: &Item) -> item_model::Result<Definition> {
        Err(item_model::Error::Unavailable)
    }
    async fn resolve(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        world: Uuid,
        character: Option<Uuid>,
        family: &str,
    ) -> item_model::Result<ItemScope> {
        self.current(db, actor, world, character, family)
            .await
            .map_err(item_error)
    }
}
