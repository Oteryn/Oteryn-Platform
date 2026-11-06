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
        let mut parties = vec![id(text(&i.value, "seller_account_id")?)?];
        if let Some(Value::String(buyer)) = i.value.get("buyer_account_id") {
            parties.push(id(buyer)?);
        }
        parties.sort();
        parties.dedup();
        for party in parties {
            let state = accounts
                .state(db, party)
                .await
                .map_err(|_| Error::Denied)?
                .ok_or(Error::Denied)?;
            let compensation = matches!(
                i.command,
                Command::Return | Command::Unlock | Command::Release
            ) || (i.command == Command::Bind
                && text(&i.value, "purpose")? == "BAZAAR_ESCROW_RETURN");
            if if compensation {
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
        let already: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=$1)",
        )
        .bind(operation)
        .fetch_one(&mut **db)
        .await?;
        if already {
            return Ok(());
        }
        let previous:Option<String>=sqlx::query_scalar("SELECT max(o.source_revision)::text FROM bazaar_authority_dispatch d JOIN bazaar_game_outbox o USING(operation_id)").fetch_one(&mut **db).await?;
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
            // Serialize competing dispositions with the issuer lock. A timeout or
            // rejection does not erase the prior executable operation.
            let competing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch d JOIN bazaar_game_outbox o USING(operation_id) WHERE o.listing_id=$1 AND o.command IN ('DeliverBazaarItem','ReturnBazaarItem') AND o.operation_id<>$2)").bind(id(text(&i.value,"listing_id")?)?).bind(operation).fetch_one(&mut **db).await?;
            if competing {
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
                        Some(operation),
                    )
                    .await?;
                    sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,$2::text::numeric,$3::text::numeric) ON CONFLICT DO NOTHING").bind(operation).bind(self.fence.projection_epoch.to_string()).bind(self.fence.recovery_generation.to_string()).execute(&mut **db).await?;
                    return Ok(());
                }
                if text(f, "currency")? != "COINS" {
                    return Err(Error::Invalid);
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
