use super::authority_wire::{decode, number, same};
use super::game_wire::{id, text};
use super::{Authority, Command, Error, PreparedIntent, Receipt};
use crate::{Currency, Evidence, Kind, Listing, Offer, Settlement};
use oteryn_internal_api::canonical::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

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
        if worlds.is_empty() || worlds.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::Invalid);
        }
        for world in worlds {
            id(&world.to_string())?;
            self.projection(db, "POLICY", *world)
                .await?
                .ok_or(Error::Unavailable)?;
        }
        let listings: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM bazaar_listings ORDER BY id")
            .fetch_all(&mut **db)
            .await?;
        for listing in listings {
            let status = self
                .projection(db, "STATUS", listing)
                .await?
                .ok_or(Error::Unavailable)?;
            let world = id(text(&status, "world_id")?)?;
            if !worlds.contains(&world) {
                return Err(Error::Unavailable);
            }
            let operation = id(text(&status, "last_operation_id")?)?;
            self.retained(db, operation)
                .await?
                .ok_or(Error::Unavailable)?;
        }
        let missing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox o WHERE NOT EXISTS(SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=o.operation_id AND r.recovery_generation=$1::text::numeric))").bind(self.fence.recovery_generation.to_string()).fetch_one(&mut **db).await?;
        if missing {
            return Err(Error::Unavailable);
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
    pub(super) async fn fresh(&self, db: &mut PgConnection) -> Result<bool, Error> {
        if self.lock(db).await.is_err() {
            return Ok(false);
        }
        let resynced:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_resync WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric)").bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).fetch_one(&mut *db).await?;
        if !resynced {
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
    pub(super) async fn retained(
        &self,
        db: &mut PgConnection,
        operation: Uuid,
    ) -> Result<Option<(PreparedIntent, Receipt)>, Error> {
        let row=sqlx::query("SELECT o.intent,o.binding_digest,r.receipt,r.receipt_digest,(SELECT d.payload FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id AND d.recovery_generation=r.recovery_generation AND d.source_authority=$4 LIMIT 1) AS audit_payload FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id WHERE o.operation_id=$1 AND r.recovery_generation=$2::text::numeric AND r.source_authority=$3 AND NOT EXISTS(SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id=o.operation_id) AND EXISTS(SELECT 1 FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id AND d.recovery_generation=r.recovery_generation AND d.source_authority=$4)").bind(operation).bind(self.fence.recovery_generation.to_string()).bind(&self.config.receipt_authority).bind(&self.config.projection_authority).fetch_optional(&mut *db).await?;
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
        decision.verify(i.bytes(), r.bytes(), self.fence.recovery_generation)?;
        if r.generation != self.fence.recovery_generation
            || r.operation != operation
            || r.command != i.command()
            || text(&r.value, "source_authority")? != self.config.receipt_authority
        {
            return Err(Error::Denied);
        }
        Ok(Some((i, r)))
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
            "SELECT EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=$1)",
        )
        .bind(listing.id)
        .fetch_one(&mut *db)
        .await?;
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
        let operation = id(text(&s, "last_operation_id")?)?;
        let Some((i, r)) = self.retained(db, operation).await? else {
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
                let item = d.get("item").ok_or(Error::Invalid)?;
                same(&s, "asset_id", item, "item_instance_id")?;
                same(&s, "custody_id", d, "escrow_id")?;
                same(&s, "custody_revision", d, "escrow_revision")?;
                if state == "ESCROWED" {
                    let row=sqlx::query("SELECT world_id,seller_character_id,item_instance_id,quantity,definition_key,definition_revision FROM bazaar_item_details WHERE id=$1 FOR SHARE").bind(listing.id).fetch_optional(&mut *db).await?;
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
                        || !self.tradeable(db, &s, item).await?
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
    ) -> Result<bool, Error> {
        let world = id(text(s, "world_id")?)?;
        let Some(policy) = self.projection(db, "POLICY", world).await? else {
            return Ok(false);
        };
        let definition = item.get("definition").ok_or(Error::Invalid)?;
        let restricted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_item_restrictions WHERE world_id=$1 AND family=$2 AND production_key=$3 AND revision_ref=$4)").bind(world).bind(text(definition,"family")?).bind(text(definition,"production_key")?).bind(text(definition,"revision_ref")?).fetch_one(&mut *db).await?;
        if restricted {
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
        let listing: Option<Listing> =
            sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR SHARE")
                .bind(sale.listing_id)
                .fetch_optional(&mut *db)
                .await?;
        let Some(listing) = listing else {
            return Ok(false);
        };
        let Some((_, i, _)) = self
            .custody(db, &listing, "DELIVERED")
            .await
            .map_err(domain)?
        else {
            return Ok(false);
        };
        if sale.currency != Currency::Coins
            || sale.seller != listing.owner
            || sale.buyer == sale.seller
        {
            return Ok(false);
        }
        let v = &i.value;
        let funding = v.get("funding").ok_or(crate::Error::Denied)?;
        Ok(
            id(text(v, "buyer_account_id").map_err(domain)?).map_err(domain)? == sale.buyer
                && id(text(v, "sale_id").map_err(domain)?).map_err(domain)? == sale.id
                && text(funding, "currency").map_err(domain)? == "COINS"
                && id(text(funding, "reservation_id").map_err(domain)?).map_err(domain)?
                    == sale.reservation
                && number(funding, "gross_amount").map_err(domain)? == sale.amount as u64,
        )
    }
}
