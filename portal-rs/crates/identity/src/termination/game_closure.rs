//! Pinned PLAT3 candidate adapter. Transport and owner acceptance remain explicit seams.
//! No network I/O runs under an account lock. Commands/replay evidence are durable;
//! timeouts, partial detach and unsigned input never establish closure eligibility.
mod wire;
use super::*;
use ed25519_dalek::{Signature, VerifyingKey};
use prost::Message;
use sha2::{Digest, Sha256};
pub use wire::{AccountClosureDecisionV1, Intent, Projection, Receipt, Watermark};

/// Each owner question is independent. No environment flag silently answers one.
#[derive(Debug, Clone, Copy, Default)]
pub struct Settings {
    pub enabled: bool,
    pub v2_qualified: bool,
    pub reversible_quarantine_accepted: bool,
    pub disposition_and_retention_accepted: bool,
    pub conflict_operator_configured: bool,
}
impl Settings {
    fn ready(self) -> Result<()> {
        if self.enabled
            && self.v2_qualified
            && self.reversible_quarantine_accepted
            && self.disposition_and_retention_accepted
            && self.conflict_operator_configured
        {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
}
pub type RecoveryFuture<'a> = Pin<Box<dyn Future<Output = Result<u64>> + Send + 'a>>;
/// Trusted reconciler must lock its nonrollback fence in this transaction. A restore
/// cannot race proof use. Native session/security generations are not this fence.
pub trait RecoveryFence: std::fmt::Debug + Send + Sync {
    fn current<'a>(&'a self, tx: &'a mut Transaction<'_, Postgres>) -> RecoveryFuture<'a>;
}
#[derive(Debug)]
pub struct UnavailableRecoveryFence;
impl RecoveryFence for UnavailableRecoveryFence {
    fn current<'a>(&'a self, _: &'a mut Transaction<'_, Postgres>) -> RecoveryFuture<'a> {
        Box::pin(async { Err(Error::UnknownState) })
    }
}
/// Wrap the existing sudo/blocker/credential adapter. Never substitute an account
/// identifier for ActorProof. Default application wiring remains unchanged.
#[derive(Debug)]
pub struct GameBoundary {
    settings: Settings,
    local: Arc<dyn Boundary>,
    recovery: Arc<dyn RecoveryFence>,
    publisher: VerifyingKey,
    authority: String,
}
impl GameBoundary {
    pub fn new(
        settings: Settings,
        local: Arc<dyn Boundary>,
        recovery: Arc<dyn RecoveryFence>,
        publisher: VerifyingKey,
        authority: String,
    ) -> Result<Self> {
        if authority.is_empty()
            || authority.len() > 128
            || !authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            settings,
            local,
            recovery,
            publisher,
            authority,
        })
    }
    fn verify(&self, raw: &[u8], signature: &[u8], max: usize) -> Result<()> {
        self.settings.ready()?;
        if raw.is_empty() || raw.len() > max {
            return Err(Error::InvalidInput);
        }
        let signature = Signature::from_slice(signature).map_err(|_| Error::Unauthorized)?;
        self.publisher
            .verify_strict(raw, &signature)
            .map_err(|_| Error::Unauthorized)
    }
    async fn recovery(&self, tx: &mut Transaction<'_, Postgres>) -> Result<u64> {
        let generation = self.recovery.current(tx).await?;
        if generation == 0 {
            return Err(Error::UnknownState);
        }
        Ok(generation)
    }
    /// Authenticated internal transport supplies a detached signature over EXACT bytes.
    /// This is not a new wire envelope and does not substitute for Character mTLS.
    /// Requeries retain the first receipt; changed replay never overwrites it.
    pub async fn ingest_receipt(&self, pool: &PgPool, raw: &[u8], signature: &[u8]) -> Result<()> {
        self.verify(raw, signature, 2048)?;
        let receipt = Receipt::parse(raw)?;
        if receipt.source_authority != self.authority {
            return Err(Error::Unauthorized);
        }
        let mut tx = pool.begin().await?;
        let operation = wire::id(&receipt.operation_id)?;
        let command: Option<(Vec<u8>,Vec<u8>,AccountId)>=sqlx::query_as("SELECT c.intent,c.intent_sha256,w.account_id FROM plat3_closure_commands c JOIN plat3_closure_work w USING(termination_id) WHERE c.operation_id=$1")
            .bind(operation).fetch_optional(&mut *tx).await?;
        let (intent_raw, intent_hash, account) = command.ok_or(Error::UnknownState)?;
        lock(&mut tx, account).await?.ok_or(Error::UnknownState)?;
        let current = self.recovery(&mut tx).await?;
        if wire::number(&receipt.decided_at, false)? > now(&mut tx).await?.saturating_add(5) {
            return Err(Error::UnknownState);
        }
        let intent = Intent::parse(&intent_raw)?;
        receipt.bind(&intent, current)?;
        if digest(&intent_raw) != intent_hash {
            return Err(Error::UnknownState);
        }
        let existing: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT receipt FROM plat3_closure_receipts WHERE operation_id=$1")
                .bind(operation)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(existing) = existing {
            if existing != raw {
                sqlx::query("UPDATE plat3_closure_work SET invalid=true WHERE termination_id=$1")
                    .bind(wire::id(&intent.termination_id)?)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Err(Error::UnknownState);
            }
            tx.commit().await?;
            return Ok(());
        }
        let hash = digest(raw);
        sqlx::query("INSERT INTO plat3_closure_receipts(operation_id,receipt,signature,receipt_sha256,recovery_generation) VALUES($1,$2,$3,$4,$5)")
            .bind(operation).bind(raw).bind(signature).bind(&hash).bind(&receipt.recovery_generation).execute(&mut *tx).await?;
        let payload = AccountClosureDecisionV1 {
            intent_sha256: intent_hash,
            receipt: raw.to_vec(),
            receipt_sha256: hash,
            recovery_generation: current,
        }
        .encode_to_vec();
        sqlx::query("INSERT INTO plat3_closure_audit_outbox(operation_id,payload) VALUES($1,$2)")
            .bind(operation)
            .bind(payload)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn ingest_projection(
        &self,
        pool: &PgPool,
        raw: &[u8],
        signature: &[u8],
    ) -> Result<()> {
        self.verify(raw, signature, 2048)?;
        let projection = Projection::parse(raw)?;
        if projection.source_authority != self.authority {
            return Err(Error::Unauthorized);
        }
        let mut tx = pool.begin().await?;
        let account = wire::id(&projection.account_id)?;
        lock(&mut tx, account).await?.ok_or(Error::UnknownState)?;
        let now = now(&mut tx).await?;
        if wire::number(&projection.source_observed_at, false)? > now + 5 {
            return Err(Error::InvalidInput);
        }
        let (epoch, invalid): (String, bool) = sqlx::query_as(
            "SELECT highest_epoch,invalid FROM plat3_closure_feed WHERE id=1 FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let incoming = wire::number(&projection.projection_epoch, true)?;
        let highest = wire::number(&epoch, false)?;
        if invalid || incoming < highest {
            return Err(Error::UnknownState);
        }
        if incoming > highest {
            sqlx::query("UPDATE plat3_closure_feed SET highest_epoch=$1,watermark=NULL,signature=NULL WHERE id=1")
                .bind(&projection.projection_epoch).execute(&mut *tx).await?;
        }
        let existing:Option<(Vec<u8>,String,String,bool)>=sqlx::query_as("SELECT snapshot,epoch,revision,invalid FROM plat3_closure_snapshots WHERE account_id=$1 FOR UPDATE")
            .bind(account).fetch_optional(&mut *tx).await?;
        let mut gap = false;
        if let Some((old, epoch, revision, invalid)) = existing {
            let pair = (wire::number(&epoch, true)?, wire::number(&revision, true)?);
            let new = (
                incoming,
                wire::number(&projection.projection_revision, true)?,
            );
            if new < pair {
                tx.commit().await?;
                return Ok(());
            }
            if new == pair {
                let old = Projection::parse(&old)?;
                if old.closures != projection.closures {
                    sqlx::query(
                        "UPDATE plat3_closure_snapshots SET invalid=true WHERE account_id=$1",
                    )
                    .bind(account)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
                    return Err(Error::UnknownState);
                }
                tx.commit().await?;
                return if invalid {
                    Err(Error::UnknownState)
                } else {
                    Ok(())
                };
            }
            gap = invalid || (new.0 == pair.0 && new.1 != pair.1.saturating_add(1));
        }
        sqlx::query("INSERT INTO plat3_closure_snapshots(account_id,snapshot,signature,epoch,revision,invalid) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(account_id) DO UPDATE SET snapshot=EXCLUDED.snapshot,signature=EXCLUDED.signature,epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,invalid=EXCLUDED.invalid")
            .bind(account).bind(raw).bind(signature).bind(&projection.projection_epoch).bind(&projection.projection_revision).bind(gap).execute(&mut *tx).await?;
        tx.commit().await?;
        if gap {
            Err(Error::UnknownState)
        } else {
            Ok(())
        }
    }
    /// Watermark freshness never repairs a revision conflict or a recovery cut.
    /// Such subjects stay pending until the owning full-resync adapter is qualified.
    pub async fn ingest_watermark(
        &self,
        pool: &PgPool,
        raw: &[u8],
        signature: &[u8],
    ) -> Result<()> {
        self.verify(raw, signature, 512)?;
        let watermark = Watermark::parse(raw)?;
        if watermark.source_authority != self.authority {
            return Err(Error::Unauthorized);
        }
        let mut tx = pool.begin().await?;
        let (epoch, old, invalid): (String, Option<Vec<u8>>, bool) = sqlx::query_as(
            "SELECT highest_epoch,watermark,invalid FROM plat3_closure_feed WHERE id=1 FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let now = now(&mut tx).await?;
        if invalid
            || epoch != watermark.projection_epoch
            || wire::number(&watermark.observed_at, false)? > now + 5
        {
            return Err(Error::UnknownState);
        }
        if let Some(old) = old {
            let old = Watermark::parse(&old)?;
            if wire::number(&watermark.complete_through, false)?
                < wire::number(&old.complete_through, false)?
            {
                return Err(Error::UnknownState);
            }
        }
        sqlx::query("UPDATE plat3_closure_feed SET watermark=$1,signature=$2 WHERE id=1")
            .bind(raw)
            .bind(signature)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Durable CC outbox access for an authenticated, rate-limited Character
    /// transport adapter. No background sender or page-render network call exists.
    /// Expiry never removes an unreceipted command from the pending rescan.
    pub async fn pending_intents(
        &self,
        pool: &PgPool,
        after_revision: i64,
        limit: i64,
    ) -> Result<Vec<Vec<u8>>> {
        self.settings.ready()?;
        if after_revision < 0 || !(1..=40).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        Ok(sqlx::query_scalar("SELECT c.intent FROM plat3_closure_commands c WHERE c.source_revision>$1 AND NOT EXISTS(SELECT 1 FROM plat3_closure_receipts r WHERE r.operation_id=c.operation_id) ORDER BY c.source_revision LIMIT $2")
            .bind(after_revision).bind(limit).fetch_all(pool).await?)
    }
    pub async fn read_intent(&self, pool: &PgPool, operation: Uuid) -> Result<Option<Vec<u8>>> {
        self.settings.ready()?;
        wire::id(&operation.to_string())?;
        Ok(
            sqlx::query_scalar("SELECT intent FROM plat3_closure_commands WHERE operation_id=$1")
                .bind(operation)
                .fetch_optional(pool)
                .await?,
        )
    }
    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        termination: &Termination,
        decision: Option<&str>,
    ) -> Result<()> {
        self.settings.ready()?;
        let (generation,close,resolution):(i64,Option<Uuid>,Option<String>)=sqlx::query_as("SELECT generation,close_operation_id,resolution FROM plat3_closure_work WHERE termination_id=$1 AND account_id=$2 FOR UPDATE")
            .bind(termination.id).bind(termination.account_id).fetch_optional(&mut **tx).await?.ok_or(Error::UnknownState)?;
        if let Some(resolution) = resolution {
            return if Some(resolution.as_str()) == decision {
                Ok(())
            } else {
                Err(Error::UnknownState)
            };
        }
        // Cancellation before a close intent has no remote effect to reverse.
        if decision == Some("CANCEL") && close.is_none() {
            advance(tx, termination.account_id, generation).await?;
            sqlx::query(
                "UPDATE plat3_closure_work SET resolution='CANCEL' WHERE termination_id=$1",
            )
            .bind(termination.id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }
        let recovery = self.recovery(tx).await?;
        if decision.is_none()
            && let Some(operation) = close
        {
            let previous: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT receipt FROM plat3_closure_receipts WHERE operation_id=$1",
            )
            .bind(operation)
            .fetch_optional(&mut **tx)
            .await?;
            let Some(previous) = previous else {
                return Ok(());
            };
            let receipt = Receipt::parse(&previous)?;
            if receipt.outcome != "REJECTED"
                || !matches!(
                    receipt.result_code.as_str(),
                    "CHAR_CMD_INTENT_EXPIRED" | "CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION"
                )
            {
                return Ok(());
            }
            // Only an authenticated terminal refusal plus the locked reconciled
            // recovery floor can authorize a fresh attempt; never a timeout.
            if wire::number(&receipt.recovery_generation, true)? > recovery {
                return Err(Error::UnknownState);
            }
        }
        let current: i64 = sqlx::query_scalar(
            "SELECT generation FROM plat3_closure_lifecycle WHERE account_id=$1 FOR UPDATE",
        )
        .bind(termination.account_id)
        .fetch_one(&mut **tx)
        .await?;
        if current != generation {
            return Err(Error::UnknownState);
        }
        let now = now(tx).await?;
        let operation = Uuid::now_v7();
        let next = if decision.is_some() {
            generation.checked_add(1).ok_or(Error::UnknownState)?
        } else {
            generation
        };
        // Serialize issuance/backlog across accounts; resolution and close use the
        // same bounded CC queue. No expiry is treated as a terminal receipt.
        sqlx::query("SELECT id FROM plat3_closure_feed WHERE id=1 FOR UPDATE")
            .execute(&mut **tx)
            .await?;
        let pending:i64=sqlx::query_scalar("SELECT count(*) FROM plat3_closure_commands c WHERE NOT EXISTS(SELECT 1 FROM plat3_closure_receipts r WHERE r.operation_id=c.operation_id)").fetch_one(&mut **tx).await?;
        if pending >= 40 {
            return Err(Error::UnknownState);
        }
        let mut intent = Intent::new(termination, operation, next, recovery, now, close, decision)?;
        let revision: i64 = sqlx::query_scalar("SELECT nextval('plat3_closure_source_revision')")
            .fetch_one(&mut **tx)
            .await?;
        intent.source_revision = revision.to_string();
        let bytes = intent.bytes()?;
        sqlx::query("INSERT INTO plat3_closure_commands(operation_id,termination_id,command,intent,intent_sha256,source_revision) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(operation).bind(termination.id).bind(&intent.command).bind(&bytes).bind(digest(&bytes)).bind(revision).execute(&mut **tx).await?;
        if let Some(decision) = decision {
            advance(tx, termination.account_id, generation).await?;
            sqlx::query("UPDATE plat3_closure_work SET resolution=$2 WHERE termination_id=$1")
                .bind(termination.id)
                .bind(decision)
                .execute(&mut **tx)
                .await?;
        } else {
            sqlx::query(
                "UPDATE plat3_closure_work SET close_operation_id=$2 WHERE termination_id=$1",
            )
            .bind(termination.id)
            .bind(operation)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
    async fn proof(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        termination: &Termination,
    ) -> Result<()> {
        self.settings.ready()?;
        let current = self.recovery(tx).await?;
        let (generation,operation):(i64,Uuid)=sqlx::query_as("SELECT w.generation,w.close_operation_id FROM plat3_closure_work w JOIN plat3_closure_lifecycle l ON l.account_id=w.account_id AND l.generation=w.generation WHERE w.termination_id=$1 AND w.account_id=$2 AND w.resolution IS NULL AND NOT w.invalid AND w.close_operation_id IS NOT NULL FOR UPDATE OF w,l")
            .bind(termination.id).bind(termination.account_id).fetch_optional(&mut **tx).await?.ok_or(Error::UnknownState)?;
        let (intent_raw,receipt_raw,signature,hash):(Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>)=sqlx::query_as("SELECT c.intent,r.receipt,r.signature,r.receipt_sha256 FROM plat3_closure_commands c JOIN plat3_closure_receipts r USING(operation_id) WHERE c.operation_id=$1")
            .bind(operation).fetch_optional(&mut **tx).await?.ok_or(Error::UnknownState)?;
        self.verify(&receipt_raw, &signature, 2048)?;
        let intent = Intent::parse(&intent_raw)?;
        let receipt = Receipt::parse(&receipt_raw)?;
        receipt.bind(&intent, current)?;
        if receipt.source_authority != self.authority
            || receipt.outcome != "COMMITTED"
            || digest(&receipt_raw) != hash
        {
            return Err(Error::UnknownState);
        }
        if intent.account_id != termination.account_id.to_string()
            || intent.termination_id != termination.id.to_string()
            || intent.scheduled_for != termination.scheduled_for.unix_timestamp().to_string()
            || intent.lifecycle_generation != generation.to_string()
        {
            return Err(Error::UnknownState);
        }
        let result = receipt.result_data.as_ref().ok_or(Error::UnknownState)?;
        let (epoch,watermark,signature,invalid):(String,Option<Vec<u8>>,Option<Vec<u8>>,bool)=sqlx::query_as("SELECT highest_epoch,watermark,signature,invalid FROM plat3_closure_feed WHERE id=1 FOR UPDATE").fetch_one(&mut **tx).await?;
        if invalid {
            return Err(Error::UnknownState);
        }
        let wm_raw = watermark.ok_or(Error::UnknownState)?;
        self.verify(&wm_raw, &signature.ok_or(Error::UnknownState)?, 512)?;
        let wm = Watermark::parse(&wm_raw)?;
        let now = now(tx).await?;
        if wm.source_authority != self.authority
            || wm.projection_epoch != epoch
            || !wm.fresh(now)?
        {
            return Err(Error::UnknownState);
        }
        let (raw,signature,invalid):(Vec<u8>,Vec<u8>,bool)=sqlx::query_as("SELECT snapshot,signature,invalid FROM plat3_closure_snapshots WHERE account_id=$1 FOR UPDATE").bind(termination.account_id).fetch_optional(&mut **tx).await?.ok_or(Error::UnknownState)?;
        self.verify(&raw, &signature, 2048)?;
        let projection = Projection::parse(&raw)?;
        let record = projection.closures.first().ok_or(Error::UnknownState)?;
        if invalid
            || projection.source_authority != self.authority
            || projection.projection_epoch != epoch
            || wire::number(&projection.source_observed_at, false)?
                > wire::number(&wm.complete_through, false)?
            || wire::number(&projection.source_observed_at, false)?
                < wire::number(&receipt.decided_at, false)?
            || record.termination_id != termination.id.to_string()
            || record.operation_id != operation.to_string()
            || record.lifecycle_generation != generation.to_string()
            || record.closure_state != "CLOSED_DETACHED"
            || record.receipt_digest != wire::hex(&hash)
            || record.proof_sha256 != result.proof_sha256
        {
            return Err(Error::UnknownState);
        }
        // LCFA is independently authenticated by its owning adapter. Lock both feed
        // and complete snapshot so a withdrawal cannot race the final commit.
        sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE")
            .execute(&mut **tx)
            .await?;
        sqlx::query(
            "SELECT account_id FROM account_character_snapshots WHERE account_id=$1 FOR UPDATE",
        )
        .bind(termination.account_id)
        .execute(&mut **tx)
        .await?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_character_feed f JOIN account_character_snapshots s ON s.account_id=$1 WHERE f.id=1 AND f.highest_epoch=$2 AND f.watermark_epoch=$2 AND s.projection_epoch=$2 AND s.projection_revision=$3 AND f.source_authority=$4 AND s.source_authority=$4 AND NOT s.invalid AND f.complete_through >= s.source_observed_at AND f.complete_through >= $5 AND f.complete_through <= $6+1 AND f.observed_at <= $6+1 AND f.observed_at >= f.complete_through AND $6+1-f.complete_through <= 30) AND NOT EXISTS(SELECT 1 FROM account_characters WHERE account_id=$1)")
            .bind(termination.account_id).bind(&result.projection_epoch).bind(&result.projection_revision).bind(&self.authority).bind(wire::time(&receipt.decided_at)?).bind(i64::try_from(now).map_err(|_| Error::UnknownState)?).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(Error::UnknownState);
        }
        // The feed lock serializes all LCFA ingests in the owning adapter.
        Ok(())
    }
}
impl Boundary for GameBoundary {
    fn authorize<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        proof: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            self.settings.ready()?;
            self.local.authorize(tx, account, proof).await
        })
    }
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            self.settings.ready()?;
            self.local.check_blockers(tx, account).await
        })
    }
    fn requested<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        termination: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            self.settings.ready()?;
            let generation:i64=sqlx::query_scalar("INSERT INTO plat3_closure_lifecycle(account_id,generation) VALUES($1,1) ON CONFLICT(account_id) DO UPDATE SET generation=plat3_closure_lifecycle.generation+1 RETURNING generation").bind(termination.account_id).fetch_one(&mut **tx).await?;
            sqlx::query("INSERT INTO plat3_closure_work(termination_id,account_id,generation) VALUES($1,$2,$3)").bind(termination.id).bind(termination.account_id).bind(generation).execute(&mut **tx).await?;
            Ok(())
        })
    }
    fn prepare<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        termination: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move { self.enqueue(tx, termination, None).await })
    }
    fn cancelled<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        termination: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move { self.enqueue(tx, termination, Some("CANCEL")).await })
    }
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        finalizing: bool,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            self.settings.ready()?;
            if finalizing {
                let termination = pending(tx, account).await?.ok_or(Error::NoPending)?;
                self.proof(tx, &termination).await?;
                self.enqueue(tx, &termination, Some("FINALIZE")).await?;
                crate::recovery_key::revoke_authorizations(tx, account).await?;
                for statement in [
                    "DELETE FROM account_passkeys WHERE account_id=$1",
                    "DELETE FROM passkey_ceremonies WHERE account_id=$1",
                    "DELETE FROM account_mfa_totp WHERE account_id=$1",
                    "DELETE FROM account_recovery_codes WHERE account_id=$1",
                    "DELETE FROM account_email_tokens WHERE account_id=$1",
                    "UPDATE account_password_resets SET used_at=coalesce(used_at,now()),updated_at=now() WHERE account_id=$1",
                    "UPDATE account_recovery_keys SET revoked_at=coalesce(revoked_at,now()),updated_at=now() WHERE account_id=$1",
                    "DELETE FROM web_known_devices WHERE account_id=$1",
                ] {
                    sqlx::query(statement)
                        .bind(account)
                        .execute(&mut **tx)
                        .await?;
                }
            }
            // Reuse all credential-family revocation, never the old finalization refusal.
            self.local.revoke(tx, account, false).await
        })
    }
}
fn digest(raw: &[u8]) -> Vec<u8> {
    Sha256::digest(raw).to_vec()
}
async fn now(tx: &mut Transaction<'_, Postgres>) -> Result<u64> {
    let now: i64 = sqlx::query_scalar("SELECT extract(epoch FROM clock_timestamp())::bigint")
        .fetch_one(&mut **tx)
        .await?;
    u64::try_from(now).map_err(|_| Error::UnknownState)
}
async fn advance(
    tx: &mut Transaction<'_, Postgres>,
    account: AccountId,
    generation: i64,
) -> Result<()> {
    let next = generation.checked_add(1).ok_or(Error::UnknownState)?;
    if sqlx::query(
        "UPDATE plat3_closure_lifecycle SET generation=$3 WHERE account_id=$1 AND generation=$2",
    )
    .bind(account)
    .bind(generation)
    .bind(next)
    .execute(&mut **tx)
    .await?
    .rows_affected()
        != 1
    {
        return Err(Error::UnknownState);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
