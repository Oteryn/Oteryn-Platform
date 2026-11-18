use super::authority_wire::{decode, number, same};
use super::game_wire::{id, text};
use super::{Authority, Command, Error, PreparedIntent, Receipt};
use crate::lifecycle::{Context, Current, Event, transition};
use crate::{Currency, Evidence, Kind, Listing, Offer, Settlement};
use oteryn_internal_api::canonical::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

const DELIVERY_EVIDENCE_RETRIES: i64 = 8;

enum DeliveryEvidence {
    Pending,
    Audited(PreparedIntent, Receipt),
    Drift(&'static str),
}

impl Authority {
    /// Called by the restore coordinator after complete enumeration and receipt
    /// replay. Every retained listing (including terminal) and policy world must
    /// be present; missing subjects never mean cancellation. No command replay.
    pub async fn finish_resync(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        worlds: &[Uuid],
    ) -> Result<(), Error> {
        self.lock(db).await?;
        if worlds.is_empty() || worlds.len() > 32 || worlds.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::Invalid);
        }
        if self.resynced(db).await? {
            return Ok(());
        }
        let worlds_binding = worlds
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let progress=sqlx::query_as::<_, (Uuid, String, String)>("SELECT listing_cursor,operation_cursor::text,worlds FROM bazaar_resync_progress WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric FOR UPDATE").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).fetch_optional(&mut **db).await?;
        let (listing_cursor, operation_cursor) = match progress {
            Some((listing, operation, worlds)) if worlds == worlds_binding => (listing, operation),
            Some(_) => return Err(Error::Conflict),
            None => (Uuid::nil(), "0".to_owned()),
        };
        for world in worlds {
            id(&world.to_string())?;
            self.projection(db, "POLICY", *world)
                .await?
                .ok_or(Error::Unavailable)?;
        }
        // The operation scan below independently verifies the audit for every
        // deposit. A refused or never-issued deposit has no custody ID for STATUS.
        let listings: Vec<(Uuid, bool, bool)> = sqlx::query_as(
            "SELECT l.id, (a.refusal_code IS NOT NULL OR l.state IN ('draft','cancelled'))
                AND NOT EXISTS (
                    SELECT 1 FROM bazaar_game_outbox o WHERE o.listing_id=l.id
                      AND o.command IN ('LockBazaarCharacter','EscrowBazaarItem')
                      AND (EXISTS (SELECT 1 FROM bazaar_authority_dispatch d WHERE d.operation_id=o.operation_id)
                           OR EXISTS (SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=o.operation_id))
                      AND (EXISTS (SELECT 1 FROM bazaar_authority_receipts r
                                   WHERE r.operation_id=o.operation_id AND r.outcome='COMMITTED')
                           OR NOT EXISTS (SELECT 1 FROM bazaar_authority_receipts r
                                   WHERE r.operation_id=o.operation_id AND r.outcome='REJECTED'
                                     AND r.recovery_generation=$2::text::numeric))),
                (l.state='completed' AND l.kind='item')
             FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id
             WHERE l.id>$1 ORDER BY l.id LIMIT 24",
        )
        .bind(listing_cursor)
        .bind(self.fence.recovery_generation.to_string())
        .fetch_all(&mut **db)
        .await?;
        let next_listing = listings.last().map(|(id, _, _)| *id);
        let listing_count = listings.len();
        for (listing, no_deposit_effect, completed_item) in listings {
            if no_deposit_effect && !completed_item {
                continue;
            }
            let status = self.projection(db, "STATUS", listing).await?;
            if completed_item {
                self.resync_completed_delivery(db, listing, status.as_ref(), worlds)
                    .await?;
                continue;
            }
            let status = status.ok_or(Error::Unavailable)?;
            let world = id(text(&status, "world_id")?)?;
            let state = text(&status, "state")?;
            let operation = if status.get("last_operation_id") == Some(&Value::Null) {
                None
            } else {
                Some(id(text(&status, "last_operation_id")?)?)
            };
            if !worlds.contains(&world) {
                return Err(Error::Unavailable);
            }
            if operation.is_none() && matches!(state, "locked" | "ESCROWED" | "INVALID") {
                continue;
            }
            let retained = if let Some(operation) = operation {
                self.retained(db, operation).await?
            } else {
                None
            };
            if retained.is_none() {
                // The existing CC1 ownership leg is not a v2 Bazaar receipt.
                // Rebuild its proof from the unchanged receipt plus its v2 bind.
                let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
                    .bind(listing)
                    .fetch_one(&mut **db)
                    .await?;
                if l.kind != Kind::Character
                    || super::runtime::characters::custody(
                        self,
                        db,
                        &l,
                        &status,
                        text(&status, "state")?,
                    )
                    .await?
                    .is_none()
                {
                    return Err(Error::Unavailable);
                }
            }
        }
        sqlx::query("INSERT INTO bazaar_resync_progress(projection_epoch,recovery_generation,worlds) VALUES($1::text::numeric,$2::text::numeric,$3) ON CONFLICT DO NOTHING").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).bind(&worlds_binding).execute(&mut **db).await?;
        if let Some(cursor) = next_listing {
            sqlx::query("UPDATE bazaar_resync_progress SET listing_cursor=$3 WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).bind(cursor).execute(&mut **db).await?;
        }
        if listing_count == 24 {
            return Ok(());
        }
        let operations=sqlx::query("SELECT o.operation_id,o.source_revision::text AS revision, EXISTS(SELECT 1 FROM bazaar_authority_dispatch d WHERE d.operation_id=o.operation_id) AS dispatched, EXISTS(SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=o.operation_id) AS admitted FROM bazaar_game_outbox o WHERE o.source_revision>$1::text::numeric ORDER BY o.source_revision LIMIT 24").bind(operation_cursor).fetch_all(&mut **db).await?;
        for operation in &operations {
            let dispatched = operation.try_get("dispatched")?;
            let admitted = operation.try_get("admitted")?;
            let retained = admitted
                && self
                    .retained(db, operation.try_get("operation_id")?)
                    .await?
                    .is_some();
            transition(
                Current::Intent {
                    dispatched,
                    admitted,
                },
                Event::ResyncIntent,
                Context {
                    retained,
                    ..Context::default()
                },
            )
            .map_err(|_| Error::Unavailable)?;
        }
        if let Some(last) = operations.last() {
            sqlx::query("UPDATE bazaar_resync_progress SET operation_cursor=$3::text::numeric WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).bind(last.try_get::<String,_>("revision")?).execute(&mut **db).await?;
        }
        if operations.len() == 24 {
            return Ok(());
        }
        sqlx::query("INSERT INTO bazaar_authority_resync(projection_epoch,recovery_generation) VALUES($1::text::numeric,$2::text::numeric) ON CONFLICT DO NOTHING").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).execute(&mut **db).await?;
        Ok(())
    }
    pub(super) async fn projection(
        &self,
        db: &mut PgConnection,
        kind: &str,
        subject: Uuid,
    ) -> Result<Option<Value>, Error> {
        let row=sqlx::query("SELECT canonical_body,content_digest,projection_revision::text AS revision,source_observed_at::text AS observed FROM bazaar_game_projections p WHERE source_authority=$1 AND kind=$2 AND subject=$3 AND projection_epoch=$4::text::numeric AND NOT EXISTS(SELECT 1 FROM bazaar_game_quarantine q WHERE q.source_authority=p.source_authority AND q.kind=p.kind AND q.subject=p.subject)").bind(&self.config.projection_authority).bind(kind).bind(subject).bind(self.fence.projection_epoch.to_string()).fetch_optional(&mut *db).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let body: String = row.try_get("canonical_body")?;
        let value = decode(body.as_bytes(), 262144)?;
        let (operation, subject_key, revision_key, observed_key) = match kind {
            "POLICY" => (
                "PublishBazaarTransferabilityV1",
                Some("world_id"),
                "projection_revision",
                "source_observed_at",
            ),
            "STATUS" => (
                "PublishBazaarListingStatusV1",
                Some("listing_id"),
                "projection_revision",
                "source_observed_at",
            ),
            "WATERMARK" => (
                "PublishBazaarWatermarkV1",
                None,
                "complete_through",
                "observed_at",
            ),
            _ => return Err(Error::Invalid),
        };
        if text(&value, "operation")? != operation
            || text(&value, "source_authority")? != self.config.projection_authority
            || number(&value, "projection_epoch")? != self.fence.projection_epoch
            || number(&value, revision_key)?.to_string() != row.try_get::<String, _>("revision")?
            || number(&value, observed_key)?.to_string() != row.try_get::<String, _>("observed")?
        {
            return Err(Error::Denied);
        }
        if let Some(key) = subject_key
            && id(text(&value, key)?)? != subject
        {
            return Err(Error::Denied);
        }
        let Value::Object(members) = &value else {
            return Err(Error::Invalid);
        };
        let content = Value::Object(
            members
                .iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "contract_version"
                            | "operation"
                            | "source_authority"
                            | "projection_epoch"
                            | "projection_revision"
                            | "source_observed_at"
                            | "observed_at"
                    )
                })
                .cloned()
                .collect(),
        );
        let hash = super::game_wire::digest(
            &oteryn_internal_api::canonical::encode(&content).map_err(|_| Error::Invalid)?,
        );
        if hash != row.try_get::<String, _>("content_digest")? {
            return Err(Error::Denied);
        }
        Ok(Some(value))
    }
    pub(super) async fn resynced(&self, db: &mut PgConnection) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_resync WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric)").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).fetch_one(&mut *db).await?)
    }
    pub(super) async fn fresh(&self, db: &mut PgConnection) -> Result<bool, Error> {
        if self.lock(db).await.is_err() {
            return Ok(false);
        }
        if !self.resynced(db).await? {
            return Ok(false);
        }
        let Some(watermark) = self.projection(db, "WATERMARK", Uuid::nil()).await? else {
            return Ok(false);
        };
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *db)
                .await?;
        let now = u64::try_from(now).map_err(|_| Error::Invalid)?;
        let complete = number(&watermark, "complete_through")?;
        Ok(number(&watermark, "observed_at")? <= now.saturating_add(1)
            && complete <= number(&watermark, "observed_at")?
            && now.saturating_sub(complete).saturating_add(1) <= 30)
    }
    async fn resync_completed_delivery(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
        status: Option<&Value>,
        worlds: &[Uuid],
    ) -> Result<(), Error> {
        let sale = self
            .delivery_sale_id(db, listing)
            .await?
            .unwrap_or_else(Uuid::nil);
        let key = super::game_store::issuance_key(listing, Command::Deliver, sale);
        let (retries, reported): (i64, bool) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE after_row->>'action'='delivery_evidence_pending'),
                    COALESCE(bool_or(after_row->>'action'='delivery_custody_drift'),false)
             FROM bazaar_audit WHERE relation_name='bazaar_delivery_evidence' AND row_id=$1
               AND after_row->>'issuance_key'=$2",
        )
        .bind(listing)
        .bind(&key)
        .fetch_one(&mut *db)
        .await?;
        if reported {
            return Err(Error::CustodyDrift);
        }

        let evidence = self.completed_delivery(db, listing, sale).await?;
        let reason = match (evidence, status) {
            (DeliveryEvidence::Drift(reason), _) => Some(reason),
            (DeliveryEvidence::Audited(intent, receipt), Some(status)) => {
                let world = id(text(status, "world_id")?)?;
                let delivered = text(status, "state")? == "DELIVERED"
                    && text(status, "asset_kind")? == "ITEM"
                    && status.get("last_operation_id") == receipt.value.get("operation_id")
                    && text(status, "seller_account_id")?
                        == text(&intent.value, "seller_account_id")?
                    && text(status, "world_id")? == text(&intent.value, "world_id")?
                    && item_custody_matches(
                        status,
                        receipt.value.get("result_data").ok_or(Error::Invalid)?,
                    )?;
                if transition(
                    Current::Custody {
                        completed_item: true,
                    },
                    Event::ResyncCustody,
                    Context {
                        delivered,
                        ..Context::default()
                    },
                )
                .is_ok()
                {
                    if worlds.contains(&world) {
                        return Ok(());
                    }
                    None
                } else {
                    Some("status_contradicts_delivery")
                }
            }
            _ => None,
        };
        let reason = match reason {
            Some(reason) => reason,
            None if retries >= DELIVERY_EVIDENCE_RETRIES => "evidence_retry_exhausted",
            None => {
                crate::lifecycle::audit_delivery_evidence(
                    db,
                    listing,
                    &key,
                    self.fence.recovery_generation,
                    "delivery_evidence_pending",
                    "missing_receipt_audit_or_status",
                )
                .await?;
                return Err(Error::DeliveryEvidencePending);
            }
        };
        let bytes = match status {
            Some(status) => {
                oteryn_internal_api::canonical::encode(status).map_err(|_| Error::Invalid)?
            }
            None => key.as_bytes().to_vec(),
        };
        crate::lifecycle::quarantine_custody(
            db,
            &self.config.projection_authority,
            listing,
            &super::game_wire::digest(&bytes),
        )
        .await?;
        crate::lifecycle::audit_delivery_evidence(
            db,
            listing,
            &key,
            self.fence.recovery_generation,
            "delivery_custody_drift",
            reason,
        )
        .await?;
        Err(Error::CustodyDrift)
    }

    pub(super) async fn delivery_sale_id(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
    ) -> Result<Option<Uuid>, Error> {
        // Both item sale workers use their immutable sale ID as the Deliver step.
        Ok(sqlx::query_scalar(
            "SELECT s.id FROM bazaar_settlements s JOIN bazaar_listings l ON l.id=s.listing_id
             WHERE l.id=$1 AND l.currency='coins'
             UNION ALL
             SELECT s.id FROM bazaar_authority_gold_sales s JOIN bazaar_listings l ON l.id=s.listing_id
             WHERE l.id=$1 AND l.currency='gold'",
        )
        .bind(listing)
        .fetch_optional(&mut *db)
        .await?)
    }

    pub(super) async fn sale_delivery_operation(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
        sale: Uuid,
    ) -> Result<Option<Uuid>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT operation_id FROM bazaar_game_outbox
             WHERE issuance_key=$1 AND listing_id=$2 AND command='DeliverBazaarItem'",
        )
        .bind(super::game_store::issuance_key(
            listing,
            Command::Deliver,
            sale,
        ))
        .bind(listing)
        .fetch_optional(db)
        .await?)
    }

    async fn completed_delivery(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
        sale: Uuid,
    ) -> Result<DeliveryEvidence, Error> {
        let operation = self.sale_delivery_operation(db, listing, sale).await?;
        let current = match operation {
            Some(operation) => self.audited_receipt(db, operation, true).await?,
            None => None,
        };
        if let Some((_, receipt, conflicted)) = &current
            && (*conflicted || !receipt.committed)
        {
            return Ok(DeliveryEvidence::Drift("delivery_reversal_or_conflict"));
        }

        // A second operation can carry the same sale step under another key.
        // An unchanged reattestation of the canonical operation carries the
        // same commitment. Changed facts or a second operation are drift.
        let commitments: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT o.operation_id,r.recovery_generation::text
             FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r USING(operation_id)
             WHERE o.listing_id=$1 AND o.command='DeliverBazaarItem'
               AND o.intent::jsonb->>'sale_id'=$2 AND r.outcome='COMMITTED'
               AND r.source_authority=$3 AND r.recovery_generation<=$4::text::numeric
               AND (o.operation_id IS DISTINCT FROM $5 OR r.recovery_generation<$4::text::numeric)
               AND EXISTS(SELECT 1 FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id
                 AND d.recovery_generation=r.recovery_generation AND d.source_authority=$6)",
        )
        .bind(listing).bind(sale.to_string()).bind(&self.config.receipt_authority)
        .bind(self.fence.recovery_generation.to_string()).bind(operation).bind(&self.config.projection_authority)
        .fetch_all(&mut *db).await?;
        for (candidate, generation) in commitments {
            let generation = generation.parse().map_err(|_| Error::Invalid)?;
            let Some((_, receipt, _)) = self
                .audited_receipt_at(db, candidate, generation, true)
                .await?
            else {
                continue;
            };
            if Some(candidate) != operation {
                return Ok(DeliveryEvidence::Drift("second_delivery_commitment"));
            }
            if let Some((_, receipt_now, _)) = &current
                && receipt.value.get("result_data") != receipt_now.value.get("result_data")
            {
                return Ok(DeliveryEvidence::Drift("changed_delivery_commitment"));
            }
        }
        match current {
            Some((intent, receipt, _))
                if id(text(&intent.value, "sale_id")?)? == sale
                    && id(text(&intent.value, "listing_id")?)? == listing =>
            {
                Ok(DeliveryEvidence::Audited(intent, receipt))
            }
            Some(_) => Ok(DeliveryEvidence::Drift("invalid_delivery_binding")),
            None => Ok(DeliveryEvidence::Pending),
        }
    }

    pub(super) async fn retained(
        &self,
        db: &mut PgConnection,
        operation: Uuid,
    ) -> Result<Option<(PreparedIntent, Receipt)>, Error> {
        let Some((intent, receipt, _)) = self.audited_receipt(db, operation, false).await? else {
            return Ok(None);
        };
        // Row-level conflict state prevents a concurrent quarantine from being
        // overwritten by a reader that verified an earlier snapshot.
        sqlx::query("UPDATE bazaar_disposition_fences SET no_effect_generation=$2::text::numeric,terminal_committed=$3 WHERE operation_id=$1 AND NOT quarantined")
            .bind(operation).bind(if receipt.committed { "0".to_owned() } else { self.fence.recovery_generation.to_string() }).bind(receipt.committed).execute(&mut *db).await?;
        Ok(Some((intent, receipt)))
    }

    async fn audited_receipt(
        &self,
        db: &mut PgConnection,
        operation: Uuid,
        include_conflicts: bool,
    ) -> Result<Option<(PreparedIntent, Receipt, bool)>, Error> {
        self.audited_receipt_at(
            db,
            operation,
            self.fence.recovery_generation,
            include_conflicts,
        )
        .await
    }

    async fn audited_receipt_at(
        &self,
        db: &mut PgConnection,
        operation: Uuid,
        generation: u64,
        include_conflicts: bool,
    ) -> Result<Option<(PreparedIntent, Receipt, bool)>, Error> {
        let row = sqlx::query(
            "SELECT o.intent,o.binding_digest,r.receipt,r.receipt_digest,d.payload AS audit_payload,
                    c.conflicted
             FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r USING(operation_id)
             CROSS JOIN LATERAL (
                SELECT payload FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id
                 AND d.recovery_generation=r.recovery_generation AND d.source_authority=$4 LIMIT 1
             ) d
             CROSS JOIN LATERAL (
                SELECT EXISTS(SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id=o.operation_id)
                 AS conflicted
             ) c
             WHERE o.operation_id=$1 AND r.recovery_generation=$2::text::numeric AND r.source_authority=$3
               AND ($5 OR NOT c.conflicted)",
        )
        .bind(operation)
        .bind(generation.to_string())
        .bind(&self.config.receipt_authority)
        .bind(&self.config.projection_authority)
        .bind(include_conflicts)
        .fetch_optional(&mut *db)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let i = PreparedIntent::parse(row.try_get("intent")?)?;
        let raw: String = row.try_get("receipt")?;
        let r = Receipt::parse(raw.as_bytes())?;
        if i.binding_digest() != row.try_get::<String, _>("binding_digest")?
            || r.digest() != row.try_get::<String, _>("receipt_digest")?
        {
            return Ok(None);
        }
        use prost::Message;
        let payload: Option<Vec<u8>> = row.try_get("audit_payload")?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let decision = super::BazaarAuthorityDecisionV1::decode(payload.as_slice())
            .map_err(|_| Error::Invalid)?;
        decision.verify(i.bytes(), r.bytes(), generation)?;
        if r.generation != generation
            || r.operation != operation
            || r.command != i.command()
            || text(&r.value, "source_authority")? != self.config.receipt_authority
        {
            return Err(Error::Denied);
        }
        Ok(Some((i, r, row.try_get("conflicted")?)))
    }
    pub(super) async fn custody(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        state: &str,
    ) -> Result<Option<(Value, PreparedIntent, Receipt)>, Error> {
        if !self.fresh(db).await? {
            return Ok(None);
        }
        let held: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=$1) OR EXISTS(SELECT 1 FROM bazaar_listings WHERE id=$1 AND kind='item' AND state='active' AND bazaar_item_restricted(id,price))",
        )
        .bind(listing.id)
        .fetch_one(&mut *db)
        .await?;
        let cancelling:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_assets WHERE listing_id=$1 AND cancel_requested)").bind(listing.id).fetch_one(&mut *db).await?;
        if cancelling
            && matches!(state, "ESCROWED" | "locked")
            && listing.state != crate::State::Draft
        {
            return Ok(None);
        }
        if held && state != "RETURNED" {
            return Ok(None);
        }
        let Some(s) = self.projection(db, "STATUS", listing.id).await? else {
            return Ok(None);
        };
        if text(&s, "state")? != state
            || id(text(&s, "seller_account_id")?)? != listing.owner
            || text(&s, "asset_kind")?
                != if listing.kind == Kind::Item {
                    "ITEM"
                } else {
                    "CHARACTER"
                }
        {
            return Ok(None);
        }
        if listing.kind == Kind::Character {
            let Some((intent, receipt, signed)) =
                super::runtime::characters::custody(self, db, listing, &s, state).await?
            else {
                return Ok(None);
            };
            // An active auction can prove custody after rebuilding, but an old
            // snapshot cannot authorize publication or a new bid.
            if state == "locked"
                && listing.state == crate::State::Active
                && number(
                    signed.get("snapshot").ok_or(Error::Invalid)?,
                    "projection_epoch",
                )? != self.fence.projection_epoch
            {
                return Ok(None);
            }
            return Ok(Some((s, intent, receipt)));
        }
        let record = if state == "ESCROWED" && s.get("last_operation_id") == Some(&Value::Null) {
            super::runtime::gold::record(self, db, listing.id, Command::Escrow).await?
        } else {
            self.retained(db, id(text(&s, "last_operation_id")?)?)
                .await?
        };
        let Some((i, r)) = record else {
            return Ok(None);
        };
        if !r.committed || id(text(&i.value, "listing_id")?)? != listing.id {
            return Ok(None);
        }
        for k in ["seller_account_id", "world_id"] {
            same(&s, k, &i.value, k)?;
        }
        let d = r.value.get("result_data").ok_or(Error::Invalid)?;
        match listing.kind {
            Kind::Item => {
                if !matches!(
                    (state, r.command),
                    ("ESCROWED", Command::Escrow)
                        | ("DELIVERED", Command::Deliver)
                        | ("RETURNED", Command::Return)
                ) {
                    return Ok(None);
                }
                if !item_custody_matches(&s, d)? {
                    return Err(Error::Denied);
                }
                if state == "ESCROWED" {
                    let item = d.get("item").ok_or(Error::Invalid)?;
                    let row=sqlx::query("SELECT world_id,seller_character_id,item_instance_id,quantity,definition_key,definition_revision FROM bazaar_item_details WHERE id=$1").bind(listing.id).fetch_optional(&mut *db).await?;
                    let Some(row) = row else {
                        return Ok(None);
                    };
                    if row.try_get::<Uuid, _>("world_id")? != id(text(item, "world_id")?)?
                        || row.try_get::<Option<Uuid>, _>("seller_character_id")?
                            != Some(id(text(d, "seller_character_id")?)?)
                        || row.try_get::<Option<Uuid>, _>("item_instance_id")?
                            != Some(id(text(item, "item_instance_id")?)?)
                        || item.get("quantity")
                            != Some(&Value::Unsigned(row.try_get::<i64, _>("quantity")? as u64))
                    {
                        return Ok(None);
                    }
                    let definition = item.get("definition").ok_or(Error::Invalid)?;
                    if text(definition, "production_key")?
                        != row.try_get::<String, _>("definition_key")?
                        || text(definition, "revision_ref")?
                            != row.try_get::<String, _>("definition_revision")?
                    {
                        return Ok(None);
                    }
                    if text(definition, "family")? != "item"
                        || !self.tradeable(db, &s, item, listing.price).await?
                    {
                        return Ok(None);
                    }
                }
            }
            // Character ownership transfer receipts are CC v1 and cannot be
            // substituted by a v2 binding or snapshot. Keep pending meanwhile.
            Kind::Character => return Ok(None),
        }
        Ok(Some((s, i, r)))
    }
    async fn tradeable(
        &self,
        db: &mut PgConnection,
        s: &Value,
        item: &Value,
        value: i64,
    ) -> Result<bool, Error> {
        let world = id(text(s, "world_id")?)?;
        let Some(policy) = self.projection(db, "POLICY", world).await? else {
            return Ok(false);
        };
        let definition = item.get("definition").ok_or(Error::Invalid)?;
        if !crate::moderation::item_allowed(
            db,
            world,
            (
                text(definition, "family")?,
                text(definition, "production_key")?,
                text(definition, "revision_ref")?,
            ),
            Some(true),
            value,
        )
        .await
        .map_err(|_| Error::Denied)?
        {
            return Ok(false);
        }
        let Some(Value::Array(definitions)) = policy.get("definitions") else {
            return Ok(false);
        };
        Ok(definitions.iter().any(|d| d.get("definition")==Some(definition) && d.get("tradeable")==Some(&Value::Bool(true)) && matches!((d.get("max_quantity"),item.get("quantity")),(Some(Value::Unsigned(max)),Some(Value::Unsigned(n))) if n<=max) && match (d.get("allowed_attribute_keys"),item.get("attributes")) {
            (Some(Value::Array(keys)),Some(Value::Array(attrs)))=>attrs.iter().all(|a|a.get("key").is_some_and(|k|keys.contains(k))),_=>false,
        }))
    }
}
fn domain(e: Error) -> crate::Error {
    match e {
        Error::Database(e) => crate::Error::Database(e),
        _ => crate::Error::Denied,
    }
}
impl Evidence for Authority {
    async fn cancellation_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<bool, crate::Error> {
        self.custody(db, listing, "RETURNED")
            .await
            .map(|v| v.is_some())
            .map_err(domain)
    }

    async fn offer_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        offer: &Offer,
    ) -> Result<bool, crate::Error> {
        if listing.form == crate::Form::WantToBuy
            || offer.listing_id != listing.id
            || offer.account == listing.owner
        {
            return Ok(false);
        }
        self.escrow_ready(db, listing).await
    }
    async fn escrow_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<bool, crate::Error> {
        self.custody(
            db,
            listing,
            if listing.kind == Kind::Item {
                "ESCROWED"
            } else {
                "locked"
            },
        )
        .await
        .map(|v| v.is_some())
        .map_err(domain)
    }
    async fn delivered(
        &self,
        db: &mut PgConnection,
        sale: &Settlement,
    ) -> Result<bool, crate::Error> {
        let listing: Option<Listing> = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(sale.listing_id)
            .fetch_optional(&mut *db)
            .await?;
        let Some(listing) = listing else {
            return Ok(false);
        };
        if listing.kind == Kind::Character {
            return super::runtime::characters::delivered(self, db, &listing, sale)
                .await
                .map_err(domain);
        }

        let Some((_, i, receipt)) = self
            .custody(db, &listing, "DELIVERED")
            .await
            .map_err(domain)?
        else {
            return Ok(false);
        };
        if sale.seller != listing.owner
            || sale.buyer == sale.seller
            || self
                .sale_delivery_operation(db, sale.listing_id, sale.id)
                .await
                .map_err(domain)?
                != Some(receipt.operation)
        {
            return Ok(false);
        }
        let v = &i.value;
        let funding = v.get("funding").ok_or(crate::Error::Denied)?;
        Ok(
            id(text(v, "buyer_account_id").map_err(domain)?).map_err(domain)? == sale.buyer
                && id(text(v, "sale_id").map_err(domain)?).map_err(domain)? == sale.id
                && text(funding, "currency").map_err(domain)?
                    == if sale.currency == Currency::Gold {
                        "GOLD"
                    } else {
                        "COINS"
                    }
                && id(text(funding, "reservation_id").map_err(domain)?).map_err(domain)?
                    == sale.reservation
                && number(funding, "gross_amount").map_err(domain)? == sale.amount as u64,
        )
    }
}

fn item_custody_matches(status: &Value, facts: &Value) -> Result<bool, Error> {
    let item = facts.get("item").ok_or(Error::Invalid)?;
    Ok(text(status, "asset_id")? == text(item, "item_instance_id")?
        && text(status, "custody_id")? == text(facts, "escrow_id")?
        && text(status, "custody_revision")? == text(facts, "escrow_revision")?)
}
