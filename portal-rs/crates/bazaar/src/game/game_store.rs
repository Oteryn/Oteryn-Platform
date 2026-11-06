use super::{
    Command, Error, PreparedIntent, Settings,
    game_wire::{decimal, id, text},
};
use crate::Accounts;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StagedCommand {
    pub operation_id: Uuid,
    pub binding_digest: String,
    pub intent: String,
    pub state: String,
}
/// All operations use the caller's transaction. Commit together with the local
/// authorized saga decision; never enqueue using a separately committed connection.
#[derive(Debug, Clone, Copy, Default)]
pub struct Outbox {
    pub settings: Settings,
}
impl Outbox {
    /// Prepare immutable bytes for later qualification, even with game dispatch off.
    /// This never reserves gold, certifies custody or makes an intent executable.
    /// Only first attempts are preparable: terminal refusal mappings are unresolved.
    #[allow(clippy::too_many_arguments)]
    pub async fn stage(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        accounts: &impl Accounts,
        actor: Uuid,
        step: Uuid,
        intent: &PreparedIntent,
        epoch: u64,
        generation: u64,
    ) -> Result<StagedCommand, Error> {
        if !self.settings.preparation_enabled {
            return Err(Error::Unavailable);
        }
        id(&actor.to_string())?;
        id(&step.to_string())?;
        if epoch == 0 || generation == 0 {
            return Err(Error::Denied);
        }
        let listing = id(text(&intent.value, "listing_id")?)?;
        let seller = id(text(&intent.value, "seller_account_id")?)?;
        let buyer = intent
            .value
            .get("buyer_account_id")
            .map(|_| text(&intent.value, "buyer_account_id").and_then(id))
            .transpose()?;
        let owner = if intent.command == Command::Hold {
            buyer.ok_or(Error::Invalid)?
        } else {
            seller
        };
        if actor != owner {
            return Err(Error::Denied);
        }
        let state = accounts
            .state(db, actor)
            .await
            .map_err(|_| Error::Denied)?
            .ok_or(Error::Denied)?;
        let allows = if matches!(intent.command, Command::Lock | Command::Escrow) {
            state.allows_listing()
        } else {
            state.allows_trade()
        };
        if !allows {
            return Err(Error::Denied);
        }
        // Lock the issuer until the caller commits, serializing revision assignment,
        // restore floors, backlog and immutable key comparisons.
        let floor=sqlx::query("SELECT source_revision::text AS revision, projection_epoch::text AS epoch, recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton FOR UPDATE").fetch_one(&mut **db).await?;
        let floor_epoch = decimal(floor.try_get("epoch")?, true, u64::MAX)?;
        let floor_generation = decimal(floor.try_get("generation")?, true, u64::MAX)?;
        if epoch < floor_epoch || generation < floor_generation {
            return Err(Error::Denied);
        }
        // A higher fence must go through reopen, never silently certify old intents.
        if epoch != floor_epoch || generation != floor_generation {
            return Err(Error::Denied);
        }
        let key = format!("bazaar:{listing}:{}:{step}:1", intent.command.wire());
        if let Some(row)=sqlx::query_as::<_,StagedCommand>("SELECT operation_id,binding_digest,intent,state FROM bazaar_game_outbox WHERE issuance_key=$1").bind(&key).fetch_optional(&mut **db).await? {
            if row.intent!=intent.raw {return Err(Error::Conflict);} return Ok(row);
        }
        let revision = decimal(text(&intent.value, "source_revision")?, false, u64::MAX)?;
        let previous = decimal(floor.try_get("revision")?, true, u64::MAX)?;
        if revision <= previous {
            return Err(Error::Conflict);
        }
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox o WHERE NOT EXISTS(SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=o.operation_id AND r.recovery_generation=$1::text::numeric)")
            .bind(generation.to_string()).fetch_one(&mut **db).await?;
        if pending >= 24 {
            return Err(Error::Backlog);
        }
        let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox WHERE actor_id=$1 AND created_at>now()-interval '60 seconds'")
            .bind(actor).fetch_one(&mut **db).await?;
        if recent >= 20 {
            return Err(Error::Backlog);
        }
        let operation = id(text(&intent.value, "operation_id")?)?;
        let existing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE operation_id=$1)",
        )
        .bind(operation)
        .fetch_one(&mut **db)
        .await?;
        if existing {
            return Err(Error::Conflict);
        }
        let result=sqlx::query_as::<_,StagedCommand>("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9::text::numeric,$10::text::numeric) RETURNING operation_id,binding_digest,intent,state")
            .bind(operation).bind(key).bind(listing).bind(actor).bind(intent.command.wire()).bind(revision.to_string()).bind(&intent.raw).bind(intent.binding_digest()).bind(epoch.to_string()).bind(generation.to_string()).fetch_one(&mut **db).await?;
        sqlx::query(
            "UPDATE bazaar_game_issuer SET source_revision=$1::text::numeric WHERE singleton",
        )
        .bind(revision.to_string())
        .execute(&mut **db)
        .await?;
        Ok(result)
    }
    /// Called under the existing restore coordinator lock after authenticating its
    /// notice. Floors only increase. This never releases or replays any effect.
    pub async fn reopen(
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        epoch: u64,
        generation: u64,
    ) -> Result<(), Error> {
        if epoch == 0 || generation == 0 {
            return Err(Error::Invalid);
        }
        let row=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton FOR UPDATE").fetch_one(&mut **db).await?;
        let old_epoch = decimal(row.try_get("epoch")?, true, u64::MAX)?;
        let old_generation = decimal(row.try_get("generation")?, true, u64::MAX)?;
        if epoch < old_epoch || generation < old_generation {
            return Err(Error::Denied);
        }
        if epoch == old_epoch && generation == old_generation {
            return Ok(());
        }
        sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=$1::text::numeric,recovery_generation=$2::text::numeric WHERE singleton").bind(epoch.to_string()).bind(generation.to_string()).execute(&mut **db).await?;
        sqlx::query("UPDATE bazaar_game_outbox SET state='REOPENED',updated_at=now() WHERE projection_epoch<$1::text::numeric OR recovery_generation<$2::text::numeric").bind(epoch.to_string()).bind(generation.to_string()).execute(&mut **db).await?;
        Ok(())
    }
}

/// T57's evidence seam deliberately denies until CHAR_CMD result bindings and
/// snapshot/facts retention have an accepted representation. Status is not proof.
impl crate::Evidence for Outbox {
    async fn offer_ready(
        &self,
        _db: &mut sqlx::PgConnection,
        _listing: &crate::Listing,
        _offer: &crate::Offer,
    ) -> Result<bool, crate::Error> {
        Ok(false)
    }
    async fn escrow_ready(
        &self,
        _db: &mut sqlx::PgConnection,
        _listing: &crate::Listing,
    ) -> Result<bool, crate::Error> {
        Ok(false)
    }
    async fn delivered(
        &self,
        _db: &mut sqlx::PgConnection,
        _settlement: &crate::Settlement,
    ) -> Result<bool, crate::Error> {
        Ok(false)
    }
}
