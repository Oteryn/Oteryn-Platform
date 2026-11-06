//! Candidate consumer. The nonrollback fence is supplied by the restore coordinator,
//! never recovered from this database or inferred from a receipt.
use super::authority_wire::{number, same};
use super::game_wire::{decimal, digest, id, text};
use super::runtime;
use super::{Error, PreparedIntent, Receipt, SnapshotBinding, verify_snapshot};
use ed25519_dalek::VerifyingKey;
use oteryn_internal_api::canonical::{self};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

pub const PINNED_CONTRACT_SHA256: [u8; 32] = [
    152, 142, 51, 37, 103, 223, 228, 111, 108, 48, 49, 204, 28, 123, 89, 0, 199, 97, 97, 250, 118,
    70, 9, 239, 160, 37, 214, 154, 142, 122, 125, 246,
];

/// Every unresolved owner decision is independently default-off. These are
/// qualification gates, not authorization to enable a deployment.
#[derive(Clone, Debug, Default)]
pub struct AuthorityConfig {
    pub enabled: bool,
    /// Exact reviewed Game contract bytes, never a moving branch or request field.
    pub contract_sha256: Option<[u8; 32]>,
    pub v2_receipts_accepted: bool,
    pub eligibility_and_escrow_accepted: bool,
    pub inbox_and_item_classes_accepted: bool,
    pub gold_fee_and_headroom_accepted: bool,
    pub common_recovery_scope_qualified: bool,
    pub signing_and_retention_accepted: bool,
    pub receipt_authority: String,
    pub projection_authority: String,
    pub snapshot_keys: Vec<(String, VerifyingKey)>,
    pub escrow_account_id: Option<Uuid>,
    pub character_cooldown_seconds: Option<u64>,
    pub character_history_complete_since: Option<u64>,
}
impl AuthorityConfig {
    pub fn available(&self) -> bool {
        self.enabled
            && self.contract_sha256 == Some(PINNED_CONTRACT_SHA256)
            && self.v2_receipts_accepted
            && self.eligibility_and_escrow_accepted
            && self.inbox_and_item_classes_accepted
            && self.common_recovery_scope_qualified
            && self.signing_and_retention_accepted
            && valid_authority(&self.receipt_authority)
            && valid_authority(&self.projection_authority)
            && !self.snapshot_keys.is_empty()
            && self.snapshot_keys.len() <= 2
            && self.snapshot_keys.iter().enumerate().all(|(n, (kid, _))| {
                kid.len() <= 64
                    && super::authority_wire::key(kid)
                    && !self.snapshot_keys[..n].iter().any(|(old, _)| old == kid)
            })
    }
}
fn valid_authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
#[derive(Clone, Copy, Debug)]
pub struct RecoveryFence {
    pub projection_epoch: u64,
    pub recovery_generation: u64,
}
#[derive(Clone, Debug)]
pub struct Authority {
    pub config: AuthorityConfig,
    pub fence: RecoveryFence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptResult {
    Stored,
    Duplicate,
    Quarantined,
}
impl Authority {
    pub(super) fn enabled(&self) -> Result<(), Error> {
        if !self.config.available()
            || self.fence.projection_epoch == 0
            || self.fence.recovery_generation == 0
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub(super) async fn lock(&self, db: &mut PgConnection) -> Result<(), Error> {
        self.enabled()?;
        let row=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton FOR UPDATE").fetch_one(&mut *db).await?;
        if decimal(row.try_get("epoch")?, false, u64::MAX)? != self.fence.projection_epoch
            || decimal(row.try_get("generation")?, false, u64::MAX)?
                != self.fence.recovery_generation
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    /// Called only by the existing Character receipt facility after its dedicated
    /// peer authentication. No Bazaar receipt route is created. Commit quarantine
    /// results; roll back errors. All durable effects share the caller transaction.
    pub async fn receive(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        peer: &str,
        raw: &[u8],
        now: u64,
    ) -> Result<ReceiptResult, Error> {
        self.lock(db).await?;
        if peer != self.config.receipt_authority {
            return Err(Error::Denied);
        }
        let receipt = Receipt::parse(raw)?;
        if text(&receipt.value, "source_authority")? != peer {
            return Err(Error::Denied);
        }
        let row=sqlx::query("SELECT intent,binding_digest,projection_epoch::text AS epoch FROM bazaar_game_outbox WHERE operation_id=$1 FOR UPDATE").bind(receipt.operation).fetch_optional(&mut **db).await?.ok_or(Error::NotFound)?;
        let intent = PreparedIntent::parse(row.try_get("intent")?)?;
        if intent.binding_digest() != row.try_get::<String, _>("binding_digest")? {
            return Err(Error::Denied);
        }
        // Replay is compared before time checks. A surviving older receipt can be
        // retained unchanged but can never satisfy a newer evidence floor.
        let old:Option<String>=sqlx::query_scalar("SELECT receipt FROM bazaar_authority_receipts WHERE operation_id=$1 AND recovery_generation=$2::text::numeric").bind(receipt.operation).bind(receipt.generation.to_string()).fetch_optional(&mut **db).await?;
        if let Some(old) = old {
            if old.as_bytes() == raw {
                return Ok(ReceiptResult::Duplicate);
            }
            sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'RECEIPT')").bind(receipt.operation).bind(receipt.digest()).execute(&mut **db).await?;
            return Ok(ReceiptResult::Quarantined);
        }
        if receipt.generation != self.fence.recovery_generation {
            return Err(Error::Conflict);
        }
        receipt.bind(&intent, now)?;
        if receipt.committed {
            self.verify_facts(db, &intent, &receipt, now).await?;
            if matches!(
                receipt.command,
                super::Command::Deliver | super::Command::Return
            ) {
                let data = receipt.value.get("result_data").ok_or(Error::Invalid)?;
                let previous:Vec<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_authority_receipts WHERE recovery_generation=$1::text::numeric AND outcome='COMMITTED' AND receipt::jsonb->>'command' IN ('DeliverBazaarItem','ReturnBazaarItem') AND receipt::jsonb->'result_data'->>'escrow_id'=$2 AND operation_id<>$3").bind(receipt.generation.to_string()).bind(text(data,"escrow_id")?).bind(receipt.operation).fetch_all(&mut **db).await?;
                if !previous.is_empty() {
                    for operation in previous
                        .into_iter()
                        .chain(std::iter::once(receipt.operation))
                    {
                        sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'RECEIPT')").bind(operation).bind(receipt.digest()).execute(&mut **db).await?;
                    }
                    return Ok(ReceiptResult::Quarantined);
                }
            }
        }
        sqlx::query("INSERT INTO bazaar_authority_receipts(operation_id,recovery_generation,source_authority,intent_digest,receipt_digest,receipt,outcome) VALUES($1,$2::text::numeric,$3,$4,$5,$6,$7)")
            .bind(receipt.operation).bind(receipt.generation.to_string()).bind(peer).bind(intent.binding_digest()).bind(receipt.digest()).bind(&receipt.raw).bind(if receipt.committed {"COMMITTED"}else{"REJECTED"}).execute(&mut **db).await?;
        Ok(ReceiptResult::Stored)
    }
    async fn verify_facts(
        &self,
        db: &mut PgConnection,
        intent: &PreparedIntent,
        receipt: &Receipt,
        now: u64,
    ) -> Result<(), Error> {
        use super::Command;
        let i = &intent.value;
        let d = receipt.value.get("result_data").ok_or(Error::Invalid)?;
        if matches!(
            receipt.command,
            Command::Hold | Command::Release | Command::Payout
        ) && !self.config.gold_fee_and_headroom_accepted
        {
            return Err(Error::Unavailable);
        }
        if receipt.command == Command::Lock {
            let signed = d.get("signed_snapshot").ok_or(Error::Invalid)?;
            let kid = text(signed, "kid")?;
            let key = &self
                .config
                .snapshot_keys
                .iter()
                .find(|(k, _)| k == kid)
                .ok_or(Error::Unavailable)?
                .1;
            let binding = SnapshotBinding {
                source_authority: &self.config.projection_authority,
                listing: id(text(i, "listing_id")?)?,
                seller: id(text(i, "seller_account_id")?)?,
                character: id(text(i, "character_id")?)?,
                world: id(text(i, "world_id")?)?,
                lock: id(text(d, "lock_id")?)?,
                lock_revision: number(d, "lock_revision")?,
                character_revision: number(i, "expected_character_revision")?,
                eligibility_revision: number(i, "eligibility_revision")?,
                projection_epoch: self.fence.projection_epoch,
                now,
            };
            verify_snapshot(
                &canonical::encode(signed).map_err(|_| Error::Invalid)?,
                &binding,
                kid,
                key,
            )?;
        }
        if matches!(receipt.command, Command::Deliver | Command::Return) {
            let prior = self
                .prerequisite(db, text(i, "escrow_receipt_digest")?)
                .await?;
            if prior.command != Command::Escrow || !prior.committed {
                return Err(Error::Denied);
            }
            let original: String =
                sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
                    .bind(prior.operation)
                    .fetch_one(&mut *db)
                    .await?;
            let original = PreparedIntent::parse(&original)?;
            for k in ["listing_id", "seller_account_id", "world_id"] {
                same(i, k, &original.value, k)?;
            }
            let p = prior.value.get("result_data").ok_or(Error::Invalid)?;
            same(d, "escrow_id", p, "escrow_id")?;
            same(i, "expected_escrow_revision", p, "escrow_revision")?;
            let before = p.get("item").ok_or(Error::Invalid)?;
            let after = d.get("item").ok_or(Error::Invalid)?;
            for k in [
                "item_instance_id",
                "world_id",
                "definition",
                "quantity",
                "attributes_hash",
                "attributes",
            ] {
                same(after, k, before, k)?;
            }
            if number(after, "state_revision")? <= number(before, "state_revision")? {
                return Err(Error::Denied);
            }
        }
        // Binding/payout require additional durable Character/financial interlocks.
        // Absence must remain pending; a hash supplied by Game is not that proof.
        match receipt.command {
            Command::Bind => runtime::characters::validate_bind(self, db, intent).await?,
            Command::Unlock => {
                runtime::characters::verify_unlock(self, db, intent, Some(d)).await?
            }
            Command::Hold => runtime::gold::validate_hold(self, db, intent, Some(d)).await?,
            Command::Release => runtime::gold::validate_release(self, db, intent, Some(d)).await?,
            Command::Payout => runtime::gold::validate_payout(self, db, intent, Some(d)).await?,
            _ => {}
        }
        Ok(())
    }
    pub(super) async fn prerequisite(
        &self,
        db: &mut PgConnection,
        hash: &str,
    ) -> Result<Receipt, Error> {
        let raw:Option<String>=sqlx::query_scalar("SELECT receipt FROM bazaar_authority_receipts WHERE receipt_digest=$1 AND recovery_generation=$2::text::numeric AND source_authority=$3").bind(hash).bind(self.fence.recovery_generation.to_string()).bind(&self.config.receipt_authority).fetch_optional(&mut *db).await?;
        let r = Receipt::parse(raw.ok_or(Error::Unavailable)?.as_bytes())?;
        if r.digest() != hash
            || r.generation != self.fence.recovery_generation
            || text(&r.value, "source_authority")? != self.config.receipt_authority
        {
            return Err(Error::Denied);
        }
        Ok(r)
    }
}

/// Field numbers and types mirror the pinned plat3_authority.proto. Hashes cover
/// retained exact wire bytes, never a reserialized protobuf or JSON object.
#[derive(Clone, PartialEq, prost::Message)]
pub struct BazaarAuthorityDecisionV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub intent_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub receipt_sha256: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub recovery_generation: u64,
}
impl BazaarAuthorityDecisionV1 {
    pub fn verify(&self, intent: &[u8], receipt: &[u8], generation: u64) -> Result<(), Error> {
        if generation == 0
            || self.recovery_generation != generation
            || self.intent_sha256.as_slice() != &Sha256::digest(intent)[..]
            || self.receipt_sha256.as_slice() != &Sha256::digest(receipt)[..]
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
/// Authenticated ANL envelope identity. The shared intake validates the complete
/// EventEnvelope, type 6/schema 1, DURABLE_AUDIT, restricted privacy and retention
/// PLAT3_AUTHORITY_AUDIT_RETENTION_V1 before passing this transaction reference.
#[derive(Clone, Debug)]
pub struct DecisionIdentity {
    pub event_id: Uuid,
    pub operation_id: Uuid,
    pub transaction_id: Uuid,
    pub ordinal: u32,
    pub count: u32,
}
impl Authority {
    pub(super) async fn decision(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        peer: &str,
        e: &DecisionIdentity,
        raw: &[u8],
        semantics: &[u8],
    ) -> Result<ReceiptResult, Error> {
        use prost::Message;
        self.lock(db).await?;
        if peer != self.config.projection_authority
            || raw.len() > 256
            || e.ordinal == 0
            || e.ordinal > e.count
            || e.count > i32::MAX as u32
        {
            return Err(Error::Denied);
        }
        for value in [e.event_id, e.operation_id, e.transaction_id] {
            id(&value.to_string())?;
        }
        let p = BazaarAuthorityDecisionV1::decode(raw).map_err(|_| Error::Invalid)?;
        let old=sqlx::query("SELECT envelope_semantics,event_id,operation_id,transaction_id,ordinal,event_count,payload,source_authority FROM bazaar_authority_decisions WHERE event_id=$1 OR (transaction_id=$2 AND ordinal=$3)").bind(e.event_id).bind(e.transaction_id).bind(e.ordinal as i32).fetch_optional(&mut **db).await?;
        if let Some(old) = old {
            if old.try_get::<Vec<u8>, _>("envelope_semantics")? == semantics
                && old.try_get::<Uuid, _>("event_id")? == e.event_id
                && old.try_get::<Uuid, _>("operation_id")? == e.operation_id
                && old.try_get::<Uuid, _>("transaction_id")? == e.transaction_id
                && old.try_get::<i32, _>("ordinal")? == e.ordinal as i32
                && old.try_get::<i32, _>("event_count")? == e.count as i32
                && old.try_get::<Vec<u8>, _>("payload")? == raw
                && old.try_get::<String, _>("source_authority")? == peer
            {
                return Ok(ReceiptResult::Duplicate);
            }
            sqlx::query("INSERT INTO bazaar_authority_conflicts(operation_id,incoming_digest,kind) VALUES($1,$2,'DECISION')").bind(e.operation_id).bind(digest(raw)).execute(&mut **db).await?;
            return Ok(ReceiptResult::Quarantined);
        }
        let row=sqlx::query("SELECT o.intent,r.receipt FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id WHERE o.operation_id=$1 AND r.recovery_generation=$2::text::numeric").bind(e.operation_id).bind(self.fence.recovery_generation.to_string()).fetch_optional(&mut **db).await?.ok_or(Error::Unavailable)?;
        let intent: String = row.try_get("intent")?;
        let receipt: String = row.try_get("receipt")?;
        p.verify(
            intent.as_bytes(),
            receipt.as_bytes(),
            self.fence.recovery_generation,
        )?;
        let count: Option<i32> = sqlx::query_scalar(
            "SELECT event_count FROM bazaar_authority_decisions WHERE transaction_id=$1 LIMIT 1",
        )
        .bind(e.transaction_id)
        .fetch_optional(&mut **db)
        .await?;
        if count.is_some_and(|n| n != e.count as i32) {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO bazaar_authority_decisions(event_id,transaction_id,ordinal,event_count,operation_id,recovery_generation,source_authority,payload,payload_digest,envelope_semantics) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9,$10)")
            .bind(e.event_id).bind(e.transaction_id).bind(e.ordinal as i32).bind(e.count as i32).bind(e.operation_id).bind(self.fence.recovery_generation.to_string()).bind(peer).bind(raw).bind(Sha256::digest(raw).to_vec()).bind(semantics).execute(&mut **db).await?;
        Ok(ReceiptResult::Stored)
    }
}

impl Authority {
    /// Accept the shared ANL EventEnvelope, type 6, never a second envelope.
    /// Authenticate the source namespace outside this method using the dedicated
    /// audit transport. A receipt must already be durably retained locally.
    pub async fn ingest_decision(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        peer: &str,
        raw: &[u8],
        now: u64,
    ) -> Result<ReceiptResult, Error> {
        use prost::Message;
        self.enabled()?;
        if raw.len() > 8192 {
            return Err(Error::Invalid);
        }
        let envelope = super::event_wire::EventEnvelope::decode(raw).map_err(|_| Error::Invalid)?;
        let e = &envelope;
        if e.envelope_revision != 1
            || e.event_type_id != 6
            || e.event_schema_revision != 1
            || e.durability_class != 2
            || !(3..=4).contains(&e.privacy_class)
            || e.retention_profile_id != "PLAT3_AUTHORITY_AUDIT_RETENTION_V1"
            || e.occurred_at_unix_ms <= 0
            || e.occurred_at_unix_ms as u64 > now.saturating_add(1).saturating_mul(1000)
            || e.server_build_id.is_empty()
            || e.server_build_id.len() > 128
            || e.payload_sha256.as_slice() != &Sha256::digest(&e.payload)[..]
        {
            return Err(Error::Invalid);
        }
        fn uuid(raw: &[u8]) -> Result<Uuid, Error> {
            let value = Uuid::from_slice(raw).map_err(|_| Error::Invalid)?;
            id(&value.to_string())
        }
        for value in [
            &e.world_id,
            &e.channel_id,
            &e.instance_id,
            &e.node_id,
            &e.game_session_id,
            &e.correlation_id,
        ]
        .into_iter()
        .flatten()
        {
            uuid(value)?;
        }
        if e.connection_generation == Some(0)
            || e.command_id == Some(0)
            || e.protocol_major == Some(0)
        {
            return Err(Error::Invalid);
        }
        if let Some(r) = &e.runtime_order
            && (r.scope_ownership_generation == 0 || r.runtime_execution_ordinal == 0)
        {
            return Err(Error::Invalid);
        }
        if let Some(a) = &e.analytics_actor {
            if !valid_authority(&a.identity_domain) || a.identity_epoch == 0 {
                return Err(Error::Invalid);
            }
            uuid(&a.analytics_actor_id)?;
        }
        if let Some(c) = &e.causation {
            use super::event_wire::Cause;
            match c.cause.as_ref().ok_or(Error::Invalid)? {
                Cause::EventId(v) | Cause::OperationId(v) | Cause::TransactionId(v) => {
                    uuid(v)?;
                }
                Cause::Command(c) => {
                    uuid(&c.game_session_id)?;
                    if c.command_id == 0 {
                        return Err(Error::Invalid);
                    }
                }
            }
        }
        let tx = e.transaction_event.as_ref().ok_or(Error::Invalid)?;
        let identity = DecisionIdentity {
            event_id: uuid(&e.event_id)?,
            operation_id: uuid(e.operation_id.as_ref().ok_or(Error::Invalid)?)?,
            transaction_id: uuid(&tx.transaction_id)?,
            ordinal: tx.ordinal,
            count: tx.count,
        };
        if let Some(world) = &e.world_id {
            let intent: Option<String> =
                sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
                    .bind(identity.operation_id)
                    .fetch_optional(&mut **db)
                    .await?;
            let intent = PreparedIntent::parse(&intent.ok_or(Error::Unavailable)?)?;
            if uuid(world)? != id(text(&intent.value, "world_id")?)? {
                return Err(Error::Denied);
            }
        }
        self.decision(db, peer, &identity, &e.payload, &e.encode_to_vec())
            .await
    }
}
