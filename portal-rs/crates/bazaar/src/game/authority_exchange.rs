//! Bounded exact-wire pull API; called only after purpose-bound mTLS authentication.
use super::authority_wire::{decode, number};
use super::game_wire::{decimal, id, ordered, text};
use super::{Authority, Command, Error, PreparedIntent};
use oteryn_internal_api::canonical::Value;
use sqlx::Row;
use uuid::Uuid;
impl Authority {
    /// Publish only in the same transaction as the local saga authorization and
    /// funding fence. Staging is deliberately insufficient for listing a command.
    pub async fn publish_intent(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        accounts: &impl crate::Accounts,
        operation: Uuid,
    ) -> Result<(), Error> {
        self.enabled()?;
        let row=sqlx::query("SELECT intent,binding_digest,projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_outbox WHERE operation_id=$1").bind(operation).fetch_optional(&mut **db).await?.ok_or(Error::Invalid)?;
        let i = PreparedIntent::parse(row.try_get("intent")?)?;
        if i.binding_digest() != row.try_get::<String, _>("binding_digest")?
            || decimal(row.try_get("epoch")?, false, u64::MAX)? != self.fence.projection_epoch
            || decimal(row.try_get("generation")?, false, u64::MAX)?
                != self.fence.recovery_generation
        {
            return Err(Error::Denied);
        }
        let seller = id(text(&i.value, "seller_account_id")?)?;
        let mut parties = vec![seller];
        if let Some(Value::String(buyer)) = i.value.get("buyer_account_id") {
            parties.push(id(buyer)?);
        } else if i.command == Command::Bind && text(&i.value, "purpose")? == "BAZAAR_SETTLEMENT" {
            parties.push(super::runtime::characters::settlement_buyer(db, &i).await?);
        }
        parties.sort();
        parties.dedup();
        accounts
            .prepare(db, &parties)
            .await
            .map_err(|_| Error::Denied)?;
        for party in parties {
            let state = accounts
                .state(db, party)
                .await
                .map_err(|_| Error::Denied)?
                .ok_or(Error::Denied)?;
            if if i.permits_safe_return(party, seller)? {
                !accounts
                    .safe_return(db, party)
                    .await
                    .map_err(|_| Error::Denied)?
            } else {
                !state.allows_trade()
                    || (matches!(i.command, Command::Lock | Command::Escrow)
                        && !state.allows_listing())
            } {
                return Err(Error::Denied);
            }
        }
        self.lock(db).await?;
        if i.is_sale_handoff()?
            && self
                .return_started(db, id(text(&i.value, "listing_id")?)?)
                .await?
        {
            return Err(Error::Denied);
        }
        let already: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=$1)",
        )
        .bind(operation)
        .fetch_one(&mut **db)
        .await?;
        if already {
            return Ok(());
        }
        let previous: Option<String> = sqlx::query_scalar(
            "SELECT source_revision::text FROM bazaar_authority_dispatch_head WHERE singleton",
        )
        .fetch_one(&mut **db)
        .await?;
        if previous.is_some_and(|s| {
            s.parse::<u64>().map_or(true, |n| {
                number(&i.value, "source_revision").map_or(true, |revision| revision <= n)
            })
        }) {
            return Err(Error::Conflict);
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut **db)
                .await?;
        let now = u64::try_from(now).map_err(|_| Error::Invalid)?;
        if number(&i.value, "issued_at_source")? > now.saturating_add(5)
            || number(&i.value, "expires_at_source")? <= now
        {
            return Err(Error::Denied);
        }
        if i.command == Command::Bind {
            super::runtime::characters::validate_bind(self, db, &i).await?;
        }
        if i.command == Command::Unlock {
            super::runtime::characters::verify_unlock(self, db, &i, None).await?;
        }
        match i.command {
            Command::Hold => super::runtime::gold::validate_hold(self, db, &i, None).await?,
            Command::Release => super::runtime::gold::validate_release(self, db, &i, None).await?,
            Command::Payout => super::runtime::gold::validate_payout(self, db, &i, None).await?,
            _ => {}
        }
        if i.command == Command::Hold && !self.config.gold_fee_and_headroom_accepted {
            return Err(Error::Unavailable);
        }
        if matches!(i.command, Command::Deliver | Command::Return) {
            let escrow = self
                .prerequisite(db, text(&i.value, "escrow_receipt_digest")?)
                .await?;
            if !escrow.committed || escrow.command != Command::Escrow {
                return Err(Error::Denied);
            }
            let original: String =
                sqlx::query_scalar("SELECT intent FROM bazaar_game_outbox WHERE operation_id=$1")
                    .bind(escrow.operation)
                    .fetch_one(&mut **db)
                    .await?;
            let original = PreparedIntent::parse(&original)?;
            for k in ["listing_id", "seller_account_id", "world_id"] {
                super::authority_wire::same(&i.value, k, &original.value, k)?;
            }
            let d = escrow.value.get("result_data").ok_or(Error::Invalid)?;
            super::authority_wire::same(&i.value, "escrow_id", d, "escrow_id")?;
            super::authority_wire::same(
                &i.value,
                "expected_escrow_revision",
                d,
                "escrow_revision",
            )?;
            // The issuer lock makes no-effect verification and compensation
            // publication atomic. Keep every dispatch and receipt for audit.
            let listing = id(text(&i.value, "listing_id")?)?;
            let unsafe_dispatch: bool = if i.command == Command::Deliver {
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command IN ('DeliverBazaarItem','ReturnBazaarItem') AND operation_id<>$2)")
                    .bind(listing).bind(operation).fetch_one(&mut **db).await?
            } else {
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command IN ('DeliverBazaarItem','ReturnBazaarItem') AND operation_id<>$2 AND no_effect_generation<$3::text::numeric) OR EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command IN ('DeliverBazaarItem','ReturnBazaarItem') AND operation_id<>$2 AND no_effect_generation>$3::text::numeric)")
                    .bind(listing).bind(operation).bind(self.fence.recovery_generation.to_string()).fetch_one(&mut **db).await?
            };
            if unsafe_dispatch {
                return Err(Error::Conflict);
            }
            if i.command == Command::Deliver {
                let f = i.value.get("funding").ok_or(Error::Invalid)?;
                if text(f, "currency")? == "GOLD" {
                    super::runtime::gold::funded(
                        self,
                        db,
                        &i.value,
                        id(text(&i.value, "buyer_account_id")?)?,
                    )
                    .await?;
                    sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,$2::text::numeric,$3::text::numeric) ON CONFLICT DO NOTHING").bind(operation).bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).execute(&mut **db).await?;
                    return Ok(());
                }
                if text(f, "currency")? != "COINS" {
                    return Err(Error::Invalid);
                }
                if !self.fresh(db).await? {
                    return Err(Error::Unavailable);
                }
                let sale = id(text(&i.value, "sale_id")?)?;
                let reservation = id(text(f, "reservation_id")?)?;
                let buyer = id(text(&i.value, "buyer_account_id")?)?;
                let gross = number(f, "gross_amount")?;
                let fenced:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_coin_holds h JOIN bazaar_settlements s ON s.id=h.settlement_id JOIN bazaar_coin_dispositions d ON d.settlement_id=s.id WHERE h.id=$1 AND h.account_id=$2 AND s.id=$3 AND s.listing_id=$4 AND s.seller=$5 AND s.buyer=$2 AND s.amount=$6 AND h.amount>=$6 AND NOT h.completed AND NOT d.completed)").bind(reservation).bind(buyer).bind(sale).bind(id(text(&i.value,"listing_id")?)?).bind(id(text(&i.value,"seller_account_id")?)?).bind(i64::try_from(gross).map_err(|_|Error::Invalid)?).fetch_one(&mut **db).await?;
                if !fenced {
                    return Err(Error::Denied);
                }
            }
        }
        sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,$2::text::numeric,$3::text::numeric) ON CONFLICT DO NOTHING").bind(operation).bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).execute(&mut **db).await?;
        Ok(())
    }
    /// Every published delivery must have current, audited terminal no-effect
    /// evidence before compensation can consume the settlement fence.
    pub(crate) async fn delivery_refused(
        &self,
        db: &mut sqlx::PgConnection,
        listing: Uuid,
    ) -> Result<bool, Error> {
        let refused: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command='DeliverBazaarItem') AND NOT EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command IN ('DeliverBazaarItem','PayoutBazaarGold') AND no_effect_generation<$2::text::numeric) AND NOT EXISTS(SELECT 1 FROM bazaar_disposition_fences WHERE listing_id=$1 AND command IN ('DeliverBazaarItem','PayoutBazaarGold') AND no_effect_generation>$2::text::numeric)")
            .bind(listing).bind(self.fence.recovery_generation.to_string()).fetch_one(db).await?;
        Ok(refused)
    }
    /// The newest Character settlement attempt is compensable only on audited
    /// terminal no-effect evidence: its Bind was refused for a reason other than
    /// expiry (an expiry is a retry) and its transfer never reached Game.
    /// Returns the refused Bind and its never-published transfer operation.
    pub(crate) async fn settlement_bind_refused(
        &self,
        db: &mut sqlx::PgConnection,
        listing: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, Error> {
        self.bind_refused(db, listing, "BAZAAR_SETTLEMENT").await
    }

    pub(crate) async fn bind_refused(
        &self,
        db: &mut sqlx::PgConnection,
        listing: Uuid,
        purpose: &str,
    ) -> Result<Option<(Uuid, Uuid)>, Error> {
        let transfer: Option<Uuid> = sqlx::query_scalar("SELECT operation_id FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose=$2 ORDER BY source_revision DESC LIMIT 1")
            .bind(listing).bind(purpose).fetch_optional(&mut *db).await?;
        let Some(transfer) = transfer else {
            return Ok(None);
        };
        let bind: Option<Uuid> = sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer' AND intent::jsonb->>'transfer_operation_id'=$2 ORDER BY source_revision DESC LIMIT 1")
            .bind(listing).bind(transfer.to_string()).fetch_optional(&mut *db).await?;
        let Some(bind) = bind else {
            return Ok(None);
        };
        let Some((intent, receipt)) = self.retained(db, bind).await? else {
            return Ok(None);
        };
        if receipt.committed
            || text(&receipt.value, "result_code")? == "CHAR_CMD_INTENT_EXPIRED"
            || text(&intent.value, "purpose")? != purpose
        {
            return Ok(None);
        }
        let staged = super::runtime::characters::staged(self, db, transfer).await?;
        let raw = oteryn_internal_api::canonical::encode(&staged).map_err(|_| Error::Invalid)?;
        if text(&intent.value, "transfer_binding_digest")? != super::game_wire::digest(&raw) {
            return Err(Error::Denied);
        }
        // A published or bound leg can still execute unless reconciliation
        // retired it against its own current-generation terminal rejection.
        let executable: bool = sqlx::query_scalar(
            "WITH candidates(operation_id) AS (
                SELECT t.operation_id FROM bazaar_all_transfers t
                JOIN character_commands c ON c.id=t.operation_id
                WHERE t.listing_id=$1 AND t.purpose=$3
                UNION ALL
                SELECT (o.intent::jsonb->>'transfer_operation_id')::uuid
                FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r USING(operation_id)
                WHERE o.listing_id=$1 AND o.command='BindBazaarCharacterTransfer'
                  AND o.intent::jsonb->>'purpose'=$3 AND r.outcome='COMMITTED'
            )
            SELECT EXISTS(SELECT 1 FROM candidates c WHERE NOT EXISTS(
                SELECT 1 FROM bazaar_authority_transfer_retirements t
                WHERE t.operation_id=c.operation_id AND t.evidence_operation_id=t.operation_id
                  AND t.recovery_generation=$2::text::numeric))",
        )
        .bind(listing)
        .bind(self.fence.recovery_generation.to_string())
        .bind(purpose)
        .fetch_one(&mut *db)
        .await?;
        if executable {
            return Ok(None);
        }
        Ok(Some((bind, transfer)))
    }
    pub(super) async fn handoff_unissued(
        &self,
        db: &mut sqlx::PgConnection,
        listing: Uuid,
    ) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE listing_id=$1 AND (command IN ('DeliverBazaarItem','PayoutBazaarGold') OR (command='BindBazaarCharacterTransfer' AND intent::jsonb->>'purpose'='BAZAAR_SETTLEMENT'))) AND NOT EXISTS(SELECT 1 FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_SETTLEMENT')").bind(listing).fetch_one(db).await?)
    }

    pub(super) async fn return_started(
        &self,
        db: &mut sqlx::PgConnection,
        listing: Uuid,
    ) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE listing_id=$1 AND (command='ReturnBazaarItem' OR (command='BindBazaarCharacterTransfer' AND intent::jsonb->>'purpose'='BAZAAR_ESCROW_RETURN')))").bind(listing).fetch_one(db).await?)
    }

    pub(crate) async fn compensation_ready(
        &self,
        db: &mut sqlx::PgConnection,
        listing: &crate::Listing,
    ) -> Result<bool, Error> {
        let refused = match listing.kind {
            crate::Kind::Item => self.delivery_refused(db, listing.id).await?,
            crate::Kind::Character => self
                .settlement_bind_refused(db, listing.id)
                .await?
                .is_some(),
        };
        // Before any sale handoff is issued, proven return custody suffices.
        // A published sale still requires its current terminal no-effect proof.
        let safe = refused || self.handoff_unissued(db, listing.id).await?;
        Ok(safe && self.custody(db, listing, "RETURNED").await?.is_some())
    }
    /// Close only after the return and financial release are both durable in
    /// this transaction. The append-only record authorizes the terminal state.
    pub(crate) async fn finish_compensation(
        &self,
        db: &mut sqlx::PgConnection,
        listing: &crate::Listing,
    ) -> Result<(), Error> {
        self.lock(db).await?;
        if !self.compensation_ready(db, listing).await? {
            return Err(Error::Unavailable);
        }
        let released = if listing.currency == crate::Currency::Gold {
            let sale = super::runtime::gold::sale(db, listing.id).await?;
            super::runtime::gold::current_hold(self, db, listing.id, sale.buyer, None)
                .await?
                .is_some_and(|hold| hold.id == sale.reservation && hold.amount == 0)
        } else {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_settlements s JOIN bazaar_coin_dispositions d ON d.settlement_id=s.id JOIN bazaar_coin_holds h ON h.settlement_id=s.id AND h.id=s.reservation AND h.account_id=s.buyer WHERE s.listing_id=$1 AND d.completed AND h.completed AND h.amount=0)")
                .bind(listing.id).fetch_one(&mut *db).await?
        };
        if !released {
            return Err(Error::Unavailable);
        }
        let Some((_, returning, _)) = self.custody(db, listing, "RETURNED").await? else {
            return Err(Error::Unavailable);
        };
        sqlx::query("INSERT INTO bazaar_authority_compensations(listing_id,return_operation_id,recovery_generation) VALUES($1,$2,$3::text::numeric) ON CONFLICT DO NOTHING")
            .bind(listing.id).bind(id(text(&returning.value, "operation_id")?)?).bind(self.fence.recovery_generation.to_string()).execute(&mut *db).await?;
        sqlx::query("DELETE FROM bazaar_moderation_holds WHERE listing_id=$1")
            .bind(listing.id)
            .execute(&mut *db)
            .await?;
        sqlx::query("UPDATE bazaar_listings SET state='cancelled',updated_at=now() WHERE id=$1 AND state='awaiting_delivery'")
            .bind(listing.id).execute(db).await?;
        Ok(())
    }
    pub async fn exchange(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        peer: &str,
        raw: &[u8],
    ) -> Result<String, Error> {
        self.lock(db).await?;
        if peer != self.config.projection_authority {
            return Err(Error::Denied);
        }
        let v = decode(raw, 512)?;
        if v.get("contract_version") != Some(&Value::Unsigned(2))
            || text(&v, "source_authority")? != peer
        {
            return Err(Error::Denied);
        }
        let operation = text(&v, "operation")?;
        if operation == "ReadBazaarCommandIntentV1" {
            ordered(
                &v,
                &[
                    "contract_version",
                    "operation",
                    "source_authority",
                    "issuer_authority",
                    "command",
                    "operation_id",
                ],
            )?;
            if text(&v, "issuer_authority")? != "OTERYN_PLATFORM_BAZAAR_AUTHORITY" {
                return Err(Error::Denied);
            }
            let row=sqlx::query("SELECT o.command,o.intent FROM bazaar_game_outbox o JOIN bazaar_authority_dispatch d USING(operation_id) WHERE o.operation_id=$1 AND d.projection_epoch<=$2::text::numeric AND d.recovery_generation<=$3::text::numeric").bind(id(text(&v,"operation_id")?)?).bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).fetch_optional(&mut **db).await?.ok_or(Error::NotFound)?;
            if row.try_get::<String, _>("command")? != text(&v, "command")? {
                return Err(Error::Conflict);
            }
            if text(&v, "command")? == Command::Deliver.wire() && !self.fresh(db).await? {
                let intent = PreparedIntent::parse(row.try_get("intent")?)?;
                let expires = number(&intent.value, "expires_at_source")?;
                let now = super::runtime::now(db).await?;
                if now.checked_sub(expires).is_none_or(|age| age < 5) {
                    return Err(Error::Unavailable);
                }
            }
            return Ok(row.try_get("intent")?);
        }
        if !matches!(
            operation,
            "ListPendingBazaarCommandsV1" | "ListBazaarCommandStatesV1"
        ) {
            return Err(Error::Invalid);
        }
        ordered(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "after_source_revision",
                "after_operation_id",
                "max_entries",
            ],
        )?;
        let revision = decimal(text(&v, "after_source_revision")?, true, u64::MAX)?;
        let cursor = if revision == 0 {
            if text(&v, "after_operation_id")? != "00000000-0000-0000-0000-000000000000" {
                return Err(Error::Invalid);
            }
            Uuid::nil()
        } else {
            id(text(&v, "after_operation_id")?)?
        };
        let Some(Value::Unsigned(limit @ 1..=32)) = v.get("max_entries") else {
            return Err(Error::Invalid);
        };
        let pending = operation == "ListPendingBazaarCommandsV1";
        let rows=sqlx::query("SELECT o.operation_id,o.command,o.source_revision::text AS revision,o.state,r.outcome,r.receipt_digest,r.recovery_generation::text AS generation FROM bazaar_game_outbox o JOIN bazaar_authority_dispatch d USING(operation_id) LEFT JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id AND r.recovery_generation=$4::text::numeric WHERE (o.source_revision,o.operation_id)>($1::text::numeric,$2) AND (NOT $5 OR r.operation_id IS NULL) AND d.projection_epoch<=$3::text::numeric AND d.recovery_generation<=$4::text::numeric ORDER BY o.source_revision,o.operation_id LIMIT $6").bind(revision.to_string()).bind(cursor).bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).bind(pending).bind(*limit as i64).fetch_all(&mut **db).await?;
        let mut entries = Vec::new();
        for row in rows {
            let mut value = format!(
                "{{\"command\":\"{}\",\"operation_id\":\"{}\",\"source_revision\":\"{}\"",
                row.try_get::<String, _>("command")?,
                row.try_get::<Uuid, _>("operation_id")?,
                row.try_get::<String, _>("revision")?
            );
            if !pending {
                let outcome: Option<String> = row.try_get("outcome")?;
                let state = outcome.unwrap_or_else(|| {
                    if row.try_get::<String, _>("state").ok().as_deref() == Some("REOPENED") {
                        "REOPENED".into()
                    } else {
                        "PENDING".into()
                    }
                });
                let digest: Option<String> = row.try_get("receipt_digest")?;
                let generation: Option<String> = row.try_get("generation")?;
                value.push_str(&format!(
                    ",\"state\":\"{state}\",\"receipt_digest\":{},\"recovery_generation\":{}",
                    digest.map_or("null".into(), |s| format!("\"{s}\"")),
                    generation.map_or("null".into(), |s| format!("\"{s}\""))
                ));
            }
            value.push('}');
            entries.push(value);
        }
        let response = format!(
            "{{\"contract_version\":2,\"{}\":[{}]}}",
            if pending { "pending" } else { "operations" },
            entries.join(",")
        );
        if response.len() > if pending { 8192 } else { 12288 } {
            return Err(Error::Invalid);
        }
        Ok(response)
    }
}
