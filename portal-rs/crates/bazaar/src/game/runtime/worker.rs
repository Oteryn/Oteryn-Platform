use super::characters::{lock_record, proof, staged};
use super::*;
use crate::{Evidence, Settlement, State};
use std::{future::Future, pin::Pin};
impl oteryn_db::outbox::Cleanup for Runtime {
    fn clean(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            self.tick()
                .await
                .map_err(|_| "bazaar authority reconciliation failed".to_owned())
        })
    }
}
impl Runtime {
    pub async fn tick(&self) -> Result<(), Error> {
        self.config.current()?;
        let mut resync = self.pool.begin().await?;
        match self
            .authority()
            .finish_resync(&mut resync, &self.config.worlds)
            .await
        {
            Ok(()) => resync.commit().await?,
            Err(Error::Database(e)) => return Err(Error::Database(e)),
            Err(_) => resync.rollback().await?,
        }
        self.tick_gold_bids().await?;
        let ids:Vec<Uuid>=sqlx::query_scalar("SELECT l.id FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id WHERE a.refusal_code IS NULL AND (l.state IN ('draft','awaiting_delivery') OR (l.state='active' AND (l.ends_at<=now() OR a.cancel_requested)) OR (l.state='completed' AND l.kind='character' AND NOT EXISTS(SELECT 1 FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id WHERE o.listing_id=l.id AND o.command='UnlockBazaarCharacter' AND r.outcome='COMMITTED' AND r.recovery_generation=$1::text::numeric))) ORDER BY CASE l.state WHEN 'awaiting_delivery' THEN 0 WHEN 'draft' THEN 1 ELSE 2 END,l.id LIMIT 24").bind(self.config.fence.recovery_generation.to_string()).fetch_all(&self.pool).await?;
        for listing in ids {
            match self.advance(listing).await {
                Ok(()) => {}
                Err(Error::Database(e)) => return Err(Error::Database(e)),
                Err(_) => {}
            }
        }
        Ok(())
    }
    pub(super) async fn stage_deposit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
        hash: Option<[u8; 32]>,
    ) -> Result<(), Error> {
        let row=sqlx::query("SELECT world_id,seller_character_id,asset_id,expected_revision::text AS revision,quantity,definition_key,definition_revision FROM bazaar_authority_assets WHERE listing_id=$1").bind(l.id).fetch_one(&mut **tx).await?;
        let world: Uuid = row.try_get("world_id")?;
        let policy = self
            .authority()
            .projection(tx, "POLICY", world)
            .await?
            .ok_or(Error::Unavailable)?;
        let revision = number(&policy, "eligibility_revision")?;
        if l.kind == Kind::Character {
            let now_coverage = now(tx).await?;
            if self.config.escrow_account.is_none()
                || self
                    .config
                    .fence
                    .character_history_complete_since
                    .is_none_or(|since| {
                        since == 0
                            || since > now_coverage.saturating_sub(self.config.cooldown_seconds)
                    })
            {
                return Err(Error::Unavailable);
            }
            let recent:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_assets a WHERE a.asset_id=$1 AND (EXISTS(SELECT 1 FROM bazaar_settlements s WHERE s.listing_id=a.listing_id AND s.completed_at>now()-make_interval(secs=>$2)) OR EXISTS(SELECT 1 FROM bazaar_authority_gold_sales s WHERE s.listing_id=a.listing_id AND s.completed_at>now()-make_interval(secs=>$2))))").bind(row.try_get::<Uuid,_>("asset_id")?).bind(self.config.cooldown_seconds as f64).fetch_one(&mut **tx).await?;
            if recent {
                return Err(Error::Denied);
            }
            self.issue(
                tx,
                l,
                l.owner,
                hash,
                Command::Lock,
                l.id,
                vec![
                    ("character_id", string(row.try_get::<Uuid, _>("asset_id")?)),
                    (
                        "expected_character_revision",
                        string(row.try_get::<String, _>("revision")?),
                    ),
                    ("eligibility_revision", string(revision)),
                    ("snapshot_schema_version", Value::Unsigned(1)),
                ],
            )
            .await?;
        } else {
            let Some(Value::Array(definitions)) = policy.get("definitions") else {
                return Err(Error::Unavailable);
            };
            let definition = row.try_get::<String, _>("definition_key")?;
            let version = row.try_get::<String, _>("definition_revision")?;
            let quantity = row.try_get::<i64, _>("quantity")? as u64;
            if !definitions.iter().any(|d| {
                d.get("tradeable") == Some(&Value::Bool(true))
                    && d.get("definition").is_some_and(|v| {
                        text(v, "family").ok() == Some("item")
                            && text(v, "production_key").ok() == Some(definition.as_str())
                            && text(v, "revision_ref").ok() == Some(version.as_str())
                    })
                    && matches!(d.get("max_quantity"),Some(Value::Unsigned(max)) if *max>=quantity)
            }) {
                return Err(Error::Denied);
            }
            self.issue(
                tx,
                l,
                l.owner,
                hash,
                Command::Escrow,
                l.id,
                vec![
                    (
                        "seller_character_id",
                        string(row.try_get::<Uuid, _>("seller_character_id")?),
                    ),
                    (
                        "item_instance_id",
                        string(row.try_get::<Uuid, _>("asset_id")?),
                    ),
                    (
                        "expected_state_revision",
                        string(row.try_get::<String, _>("revision")?),
                    ),
                    ("quantity", Value::Unsigned(quantity)),
                    ("transferability_revision", string(revision)),
                ],
            )
            .await?;
        }
        Ok(())
    }
    async fn advance(&self, id: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        self.authority().lock(&mut tx).await?;
        let row = sqlx::query(
            "SELECT session_hash,cancel_requested FROM bazaar_authority_assets WHERE listing_id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let hash: [u8; 32] = row
            .try_get::<Vec<u8>, _>("session_hash")?
            .try_into()
            .map_err(|_| Error::Invalid)?;
        // A definite refusal is visible as refusal, never guessed from absence/timeout.
        let refusals:Vec<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 ORDER BY source_revision DESC").bind(id).fetch_all(&mut *tx).await?;
        for operation in refusals {
            if let Some((_, receipt)) = self.authority().retained(&mut tx, operation).await?
                && !receipt.committed
                && !matches!(receipt.command, Command::Hold | Command::Release)
            {
                sqlx::query(
                    "UPDATE bazaar_authority_assets SET refusal_code=$2 WHERE listing_id=$1",
                )
                .bind(id)
                .bind(text(&receipt.value, "result_code")?)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(());
            }
        }
        if l.currency == Currency::Gold && l.state != State::Draft {
            self.advance_gold(&mut tx, &l).await?;
            tx.commit().await?;
            return Ok(());
        }
        match l.state {
            State::Draft => {
                if l.kind == Kind::Character {
                    if lock_record(self.authority(), &mut tx, id).await?.is_none() {
                        return Ok(());
                    }
                    self.transfer(&mut tx, &l, "BAZAAR_LISTING_ESCROW", None)
                        .await?;
                }
                if !self
                    .authority()
                    .escrow_ready(&mut tx, &l)
                    .await
                    .map_err(core_error)?
                {
                    tx.commit().await?;
                    return Ok(());
                }
                if row.try_get::<bool, _>("cancel_requested")? {
                    self.return_unsold(&mut tx, &l).await?;
                    tx.commit().await?;
                    return Ok(());
                }
                let state = self
                    .accounts(Some(hash))
                    .state(&mut tx, l.owner)
                    .await
                    .map_err(core_error)?
                    .ok_or(Error::Denied)?;
                if !state.allows_listing() {
                    return Err(Error::Denied);
                }
                sqlx::query("UPDATE bazaar_listings SET state='active',updated_at=now() WHERE id=$1 AND state='draft'").bind(id).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            State::Active => {
                if row.try_get::<bool, _>("cancel_requested")? || l.leader.is_none() {
                    self.return_unsold(&mut tx, &l).await?;
                    tx.commit().await?;
                    return Ok(());
                }
                tx.commit().await?;
                self.core(None)
                    .settle(id, &format!("close:{id}"))
                    .await
                    .map_err(core_error)?;
            }
            State::AwaitingDelivery => {
                let sale: Settlement =
                    sqlx::query_as("SELECT * FROM bazaar_settlements WHERE listing_id=$1")
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await?;
                if l.kind == Kind::Character {
                    self.transfer(&mut tx, &l, "BAZAAR_SETTLEMENT", Some(&sale))
                        .await?;
                } else {
                    let Some((_, escrow)) = self.latest(&mut tx, id, Command::Escrow).await? else {
                        return Err(Error::Unavailable);
                    };
                    if !escrow.committed {
                        return Err(Error::Denied);
                    }
                    let d = escrow.value.get("result_data").ok_or(Error::Invalid)?;
                    let destination:Uuid=sqlx::query_scalar("SELECT character_id FROM bazaar_authority_destinations WHERE listing_id=$1 AND buyer=$2").bind(id).bind(sale.buyer).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
                    self.issue(
                        &mut tx,
                        &l,
                        l.owner,
                        None,
                        Command::Deliver,
                        sale.id,
                        vec![
                            (
                                "escrow_id",
                                d.get("escrow_id").ok_or(Error::Invalid)?.clone(),
                            ),
                            (
                                "expected_escrow_revision",
                                d.get("escrow_revision").ok_or(Error::Invalid)?.clone(),
                            ),
                            ("buyer_account_id", string(sale.buyer)),
                            ("buyer_character_id", string(destination)),
                            ("escrow_receipt_digest", string(escrow.digest())),
                            ("sale_id", string(sale.id)),
                            ("funding", funding(&sale)),
                        ],
                    )
                    .await?;
                }
                tx.commit().await?;
                self.core(None)
                    .complete_delivery(id)
                    .await
                    .map_err(core_error)?;
            }
            State::Completed if l.kind == Kind::Character => {
                self.unlock(&mut tx, &l).await?;
                tx.commit().await?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) async fn transfer(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
        purpose: &str,
        sale: Option<&Settlement>,
    ) -> Result<(), Error> {
        let existing:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_authority_transfers WHERE listing_id=$1 AND purpose=$2").bind(l.id).bind(purpose).fetch_optional(&mut **tx).await?;
        let operation = if let Some(operation) = existing {
            operation
        } else {
            let Some((lock, _, _)) = lock_record(self.authority(), tx, l.id).await? else {
                return Err(Error::Unavailable);
            };
            let escrow = self.config.escrow_account.ok_or(Error::Unavailable)?;
            let (from, to) = match purpose {
                "BAZAAR_LISTING_ESCROW" => (l.owner, escrow),
                "BAZAAR_SETTLEMENT" => (escrow, sale.ok_or(Error::Invalid)?.buyer),
                "BAZAAR_ESCROW_RETURN" => (escrow, l.owner),
                _ => return Err(Error::Invalid),
            };
            if purpose != "BAZAAR_LISTING_ESCROW"
                && proof(self.authority(), tx, l.id, "BAZAAR_LISTING_ESCROW")
                    .await?
                    .is_none()
            {
                return Err(Error::Unavailable);
            }
            sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
                .execute(&mut **tx)
                .await?;
            let revision: i64 = sqlx::query_scalar(
                "SELECT nextval(pg_get_serial_sequence('character_commands','source_revision'))",
            )
            .fetch_one(&mut **tx)
            .await?;
            if revision <= 0 {
                return Err(Error::Unavailable);
            }
            let issued = now(tx).await?;
            let operation = oteryn_core::new_id();
            let decision = oteryn_core::new_id();
            let binding = Value::Object(vec![
                (
                    "character_id".into(),
                    lock.value
                        .get("character_id")
                        .ok_or(Error::Invalid)?
                        .clone(),
                ),
                ("expected_from_account_id".into(), string(from)),
                ("to_account_id".into(), string(to)),
                ("purpose".into(), string(purpose)),
            ]);
            let Value::Object(fields) = binding.clone() else {
                return Err(Error::Invalid);
            };
            let mut fields_all = vec![
                ("contract_version".into(), Value::Unsigned(1)),
                ("command".into(), string("TransferCharacterOwnership")),
                ("variant".into(), string("PLATFORM_BAZAAR_TRANSFER")),
                (
                    "issuer_authority".into(),
                    string("OTERYN_PLATFORM_CHARACTER_AUTHORITY"),
                ),
                ("issuer_decision_id".into(), string(decision)),
                ("source_revision".into(), string(revision)),
                ("operation_id".into(), string(operation)),
            ];
            fields_all.extend(fields);
            fields_all.extend([
                ("issued_at_source".into(), string(issued)),
                ("expires_at_source".into(), string(issued + 300)),
                ("audience".into(), string("OTERYN_GAME_CHARACTER_AUTHORITY")),
            ]);
            let intent = String::from_utf8(
                canonical::encode(&Value::Object(fields_all)).map_err(|_| Error::Invalid)?,
            )
            .map_err(|_| Error::Invalid)?;
            let binding =
                String::from_utf8(canonical::encode(&binding).map_err(|_| Error::Invalid)?)
                    .map_err(|_| Error::Invalid)?;
            sqlx::query("INSERT INTO bazaar_authority_transfers(operation_id,listing_id,purpose,decision_id,source_revision,intent,intent_digest,binding,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric,$10::text::numeric)")
                .bind(operation).bind(l.id).bind(purpose).bind(decision).bind(revision).bind(&intent).bind(digest(intent.as_bytes())).bind(binding).bind(self.config.fence.projection_epoch.to_string()).bind(self.config.fence.recovery_generation.to_string()).execute(&mut **tx).await?;
            operation
        };
        let transfer = staged(self.authority(), tx, operation).await?;
        let lock = lock_record(self.authority(), tx, l.id)
            .await?
            .ok_or(Error::Unavailable)?
            .1;
        let facts = lock.value.get("result_data").ok_or(Error::Invalid)?;
        let status = self
            .authority()
            .projection(tx, "STATUS", l.id)
            .await?
            .ok_or(Error::Unavailable)?;
        let raw = canonical::encode(&transfer).map_err(|_| Error::Invalid)?;
        let funding_value = if let Some(s) = sale {
            if s.currency == Currency::Gold {
                let revision: String = sqlx::query_scalar(
                    "SELECT hold_revision::text FROM bazaar_authority_gold_sales WHERE id=$1",
                )
                .bind(s.id)
                .fetch_one(&mut **tx)
                .await?;
                Value::Object(vec![
                    ("currency".into(), string("GOLD")),
                    ("reservation_id".into(), string(s.reservation)),
                    ("hold_revision".into(), string(revision)),
                    ("gross_amount".into(), string(s.amount)),
                ])
            } else {
                funding(s)
            }
        } else {
            Value::Null
        };
        self.issue(
            tx,
            l,
            l.owner,
            None,
            Command::Bind,
            operation,
            vec![
                (
                    "character_id",
                    transfer.get("character_id").ok_or(Error::Invalid)?.clone(),
                ),
                (
                    "lock_id",
                    facts.get("lock_id").ok_or(Error::Invalid)?.clone(),
                ),
                (
                    "expected_lock_revision",
                    status
                        .get("custody_revision")
                        .ok_or(Error::Invalid)?
                        .clone(),
                ),
                ("transfer_operation_id", string(operation)),
                ("transfer_binding_digest", string(digest(&raw))),
                ("purpose", string(purpose)),
                ("sale_id", sale.map_or(Value::Null, |s| string(s.id))),
                ("funding", funding_value),
            ],
        )
        .await?;
        let bind:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer' AND intent::jsonb->>'transfer_operation_id'=$2").bind(l.id).bind(operation.to_string()).fetch_optional(&mut **tx).await?;
        let Some(bind) = bind else {
            return Err(Error::Unavailable);
        };
        let Some((_, receipt)) = self.authority().retained(tx, bind).await? else {
            return Ok(());
        };
        if !receipt.committed {
            return Err(Error::Denied);
        }
        let already: Option<String> =
            sqlx::query_scalar("SELECT intent_json FROM character_commands WHERE id=$1")
                .bind(operation)
                .fetch_optional(&mut **tx)
                .await?;
        if let Some(existing) = already {
            if existing.as_bytes() != raw {
                return Err(Error::Conflict);
            }
            return Ok(());
        }
        sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
            .execute(&mut **tx)
            .await?;
        let revision = number(&transfer, "source_revision")?;
        let behind: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM character_commands WHERE source_revision>=$1)",
        )
        .bind(i64::try_from(revision).map_err(|_| Error::Invalid)?)
        .fetch_one(&mut **tx)
        .await?;
        let backlog: i64 =
            sqlx::query_scalar("SELECT count(*) FROM character_commands WHERE state='PENDING'")
                .fetch_one(&mut **tx)
                .await?;
        if behind || backlog >= 40 {
            return Err(Error::Unavailable);
        }
        let row=sqlx::query("SELECT decision_id,binding,intent FROM bazaar_authority_transfers WHERE operation_id=$1").bind(operation).fetch_one(&mut **tx).await?;
        sqlx::query("INSERT INTO character_commands(id,source_revision,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json) OVERRIDING SYSTEM VALUE VALUES($1,$2,$3,$4,'TransferCharacterOwnership',$5,$6,$7,$8)")
            .bind(operation).bind(revision as i64).bind(row.try_get::<Uuid,_>("decision_id")?).bind(format!("bazaar-transfer:{}:{purpose}:1",l.id)).bind(id(text(&transfer,"expected_from_account_id")?)?).bind(id(text(&transfer,"to_account_id")?)?).bind(row.try_get::<String,_>("binding")?).bind(row.try_get::<String,_>("intent")?).execute(&mut **tx).await?;
        Ok(())
    }
    pub(super) async fn unlock(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
    ) -> Result<(), Error> {
        let Some((_, lock, _)) = lock_record(self.authority(), tx, l.id).await? else {
            return Err(Error::Unavailable);
        };
        let facts = lock.value.get("result_data").ok_or(Error::Invalid)?;
        let status = self
            .authority()
            .projection(tx, "STATUS", l.id)
            .await?
            .ok_or(Error::Unavailable)?;
        self.issue(
            tx,
            l,
            l.owner,
            None,
            Command::Unlock,
            l.id,
            vec![
                (
                    "character_id",
                    facts.get("character_id").ok_or(Error::Invalid)?.clone(),
                ),
                (
                    "lock_id",
                    facts.get("lock_id").ok_or(Error::Invalid)?.clone(),
                ),
                (
                    "expected_lock_revision",
                    status
                        .get("custody_revision")
                        .ok_or(Error::Invalid)?
                        .clone(),
                ),
            ],
        )
        .await?;
        Ok(())
    }
    pub(super) async fn return_unsold(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
    ) -> Result<(), Error> {
        if l.leader.is_some() {
            return Err(Error::Denied);
        }
        if l.kind == Kind::Character {
            self.transfer(tx, l, "BAZAAR_ESCROW_RETURN", None).await?;
        } else {
            let Some((_, escrow)) = self.latest(tx, l.id, Command::Escrow).await? else {
                return Err(Error::Unavailable);
            };
            let d = escrow.value.get("result_data").ok_or(Error::Invalid)?;
            self.issue(
                tx,
                l,
                l.owner,
                None,
                Command::Return,
                l.id,
                vec![
                    (
                        "escrow_id",
                        d.get("escrow_id").ok_or(Error::Invalid)?.clone(),
                    ),
                    (
                        "expected_escrow_revision",
                        d.get("escrow_revision").ok_or(Error::Invalid)?.clone(),
                    ),
                    (
                        "seller_character_id",
                        d.get("seller_character_id").ok_or(Error::Invalid)?.clone(),
                    ),
                    ("escrow_receipt_digest", string(escrow.digest())),
                ],
            )
            .await?;
        }
        if self
            .authority()
            .cancellation_ready(tx, l)
            .await
            .map_err(core_error)?
        {
            if l.kind == Kind::Character {
                self.unlock(tx, l).await?;
            }
            sqlx::query(
                "UPDATE bazaar_listings SET state='cancelled',updated_at=now() WHERE id=$1",
            )
            .bind(l.id)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
fn funding(s: &Settlement) -> Value {
    Value::Object(vec![
        ("currency".into(), string("COINS")),
        ("reservation_id".into(), string(s.reservation)),
        ("hold_revision".into(), Value::Null),
        ("gross_amount".into(), string(s.amount)),
    ])
}
