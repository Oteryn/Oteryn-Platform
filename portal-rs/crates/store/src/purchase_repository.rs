use super::*;
use crate::catalog::Offer;
use oteryn_wallet::{
    AccountStateReader, AdjustmentKind, AuditHook, Ledger, NumericHistoryPage, Refusal,
    TransferBlockCheck,
};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Restricted,
    Missing,
    Unknown,
}
/// Read authoritative identity and lock its row in the supplied transaction.
/// The adapter must fence account termination, bans and other purchase restrictions.
pub trait Accounts: Send + Sync {
    fn state(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> impl Future<Output = Result<AccountState, Error>> + Send;
}
#[derive(Debug)]
pub struct Purchases<A, B, H, C> {
    pool: PgPool,
    settings: Settings,
    wallet: Ledger<A, B, H>,
    accounts: C,
}
impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook, C: Accounts>
    Purchases<A, B, H, C>
{
    /// Wallet adapters retain account authorization and transactional auditing.
    /// The audit hook must authorize the buyer's purchase debit, never staff power.
    pub fn new(pool: PgPool, settings: Settings, wallet: Ledger<A, B, H>, accounts: C) -> Self {
        Self {
            pool,
            settings,
            wallet: wallet.with_adjustment_kind(AdjustmentKind::StorePurchase),
            accounts,
        }
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.settings.valid() {
            Ok(())
        } else {
            Err(Error::Unavailable)
        }
    }
    fn source(&self, source: &str) -> Result<(), Error> {
        use subtle::ConstantTimeEq;
        if bool::from(
            self.settings
                .source_authority
                .as_bytes()
                .ct_eq(source.as_bytes()),
        ) {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
    fn source_head(&self, raw: &[u8], max: usize, operation: &str) -> Result<(), Error> {
        let value = canonical::decode(raw, max, 4).map_err(|_| Error::Invalid)?;
        if raw.contains(&b'\\') || canonical::encode(&value).map_err(|_| Error::Invalid)? != raw {
            return Err(Error::Invalid);
        }
        let Value::Object(members) = &value else {
            return Err(Error::Invalid);
        };
        if members.len() < 3
            || members[..3].iter().map(|(key, _)| key.as_str()).ne([
                "contract_version",
                "operation",
                "source_authority",
            ])
        {
            return Err(Error::Invalid);
        }
        if value.get("contract_version") != Some(&number(1))
            || value.get("operation") != Some(&text(operation))
        {
            return Err(Error::Invalid);
        }
        let Some(Value::String(authority)) = value.get("source_authority") else {
            return Err(Error::Invalid);
        };
        if !super::source(authority) {
            return Err(Error::Invalid);
        }
        self.source(authority)
    }
    pub(super) async fn peer_limit(&self) -> Result<(), Error> {
        self.enabled()?;
        self.limit(
            "peer",
            &format!(
                "{:x}",
                Sha256::digest(self.settings.peer_identity.as_bytes())
            ),
            self.settings.requests_per_minute,
        )
        .await
    }
    async fn limit(&self, scope: &str, subject: &str, limit: u32) -> Result<(), Error> {
        // The rate budget intentionally commits independently of a refused purchase.
        let row = sqlx::query("INSERT INTO store_wire_limits(scope,subject,attempts) VALUES($1,$2,1) ON CONFLICT(scope,subject) DO UPDATE SET window_start=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN clock_timestamp() ELSE store_wire_limits.window_start END, attempts=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN 1 ELSE LEAST(store_wire_limits.attempts+1,$3::integer+1) END, updated_at=clock_timestamp() RETURNING attempts,GREATEST(1,CEIL(EXTRACT(EPOCH FROM window_start+interval '60 seconds'-clock_timestamp())))::integer AS retry")
            .bind(scope).bind(subject).bind(limit as i32).fetch_one(&self.pool).await?;
        if row.try_get::<i32, _>("attempts")? > limit as i32 {
            return Err(Error::Limited(row.try_get::<i32, _>("retry")?.max(1) as u32));
        }
        Ok(())
    }
    async fn begin(&self) -> Result<Transaction<'_, Postgres>, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout='3s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
    pub async fn purchase(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        self.enabled()?;
        self.source_head(raw, 1024, "PurchaseStoreOfferV1")?;
        let request = PurchaseRequest::decode(raw)?;
        let account = uuid(&request.account_id)?;
        let key = uuid(&request.purchase_request_id)?;
        self.limit(
            "purchase",
            &request.account_id,
            self.settings.purchases_per_account_per_minute,
        )
        .await?;
        let mut tx = self.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,29))")
            .bind(format!("store-purchase:{key}"))
            .execute(&mut *tx)
            .await?;
        if let Some(row)=sqlx::query("SELECT request_bytes,answer_bytes FROM store_purchase_requests WHERE purchase_request_id=$1").bind(key).fetch_optional(&mut *tx).await? {
            if row.try_get::<Vec<u8>,_>("request_bytes")? != raw {
                tracing::warn!("products_entitlements.store_purchase_request_conflict");
                return Err(Error::Conflict);
            }
            let answer=row.try_get("answer_bytes")?;
            tx.commit().await?;
            return Ok(answer);
        }
        let referrals: bool =
            sqlx::query_scalar("SELECT to_regclass('referral_campaign_settings') IS NOT NULL")
                .fetch_one(&mut *tx)
                .await?;
        if referrals {
            sqlx::query("SELECT referral_campaign_prepare(ARRAY[$1::uuid] || ARRAY(SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=$1),$1)")
                .bind(account).execute(&mut *tx).await?;
        }
        let restricted = self.accounts.state(&mut tx, account).await? != AccountState::Active;
        let review: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM store_fulfilment_units WHERE account_id=$1 AND state IN ('MANUAL_REVIEW','REVERSAL_PARTIAL'))").bind(account).fetch_one(&mut *tx).await?;
        if restricted || review {
            return finish(tx, &request, raw, "ACCOUNT_RESTRICTED", vec![]).await;
        }
        // Lock the same singleton as activation, for the complete purchase decision.
        let active: Option<String> = sqlx::query_scalar(
            "SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR SHARE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let active = active.ok_or(Error::Unavailable)?;
        if request.catalog_revision != active {
            return finish(tx, &request, raw, "STALE_QUOTE", vec![]).await;
        }
        let bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT canonical_bytes FROM store_offers WHERE catalog_revision=$1 AND offer_key=$2",
        )
        .bind(&active)
        .bind(&request.offer_key)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(bytes) = bytes else {
            return finish(tx, &request, raw, "OFFER_UNAVAILABLE", vec![]).await;
        };
        let offer: Offer = serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
        if !offer.purchasable || offer.units.is_empty() {
            return finish(tx, &request, raw, "OFFER_UNAVAILABLE", vec![]).await;
        }
        // Only explicit unit-kind flags authorize a purchase; an import alone does not.
        if offer
            .units
            .iter()
            .any(|u| !self.settings.enabled_unit_kinds.contains(&u.unit_kind))
        {
            return finish(tx, &request, raw, "PURCHASE_DISABLED", vec![]).await;
        }
        if offer.currency != request.currency || offer.unit_price != request.unit_price {
            return finish(tx, &request, raw, "STALE_QUOTE", vec![]).await;
        }
        let price = offer
            .unit_price
            .parse::<i64>()
            .map_err(|_| Error::Unavailable)?;
        let total = price.checked_mul(i64::from(request.quantity));
        if request.currency != "COINS"
            || request.quantity > offer.max_quantity
            || total.is_none_or(|v| v <= 0)
            || offer.units.len() > 8
        {
            return finish(tx, &request, raw, "OFFER_UNAVAILABLE", vec![]).await;
        }
        let total = total.ok_or(Error::Unavailable)?;
        let mut units = offer.units;
        units.sort_by(|a, b| {
            a.product_id
                .cmp(&b.product_id)
                .then_with(|| a.product_version.len().cmp(&b.product_version.len()))
                .then_with(|| a.product_version.cmp(&b.product_version))
        });
        for unit in &mut units {
            let quantity = u32::from(unit.quantity) * u32::from(request.quantity);
            if quantity > 65535 {
                return finish(tx, &request, raw, "OFFER_UNAVAILABLE", vec![]).await;
            }
            unit.quantity = quantity as u16;
        }
        // Serialize capacity reservations across accounts before charging anything.
        sqlx::query("SELECT pg_advisory_xact_lock(29,1)")
            .execute(&mut *tx)
            .await?;
        let pending: i64=sqlx::query_scalar("SELECT COUNT(*) FROM store_operations WHERE operation_kind='DELIVER' AND state IN ('PENDING','REOPENED')").fetch_one(&mut *tx).await?;
        if pending + units.len() as i64 > i64::from(self.settings.max_pending_operations) {
            return Err(Error::Unavailable);
        }
        // A savepoint removes a first wallet row and idempotency claim on refusal.
        sqlx::query("SAVEPOINT purchase_debit")
            .execute(&mut *tx)
            .await?;
        let reason = format!("store-purchase:{key}");
        match self
            .wallet
            .adjust_in(&mut tx, key, account, -total, account, &reason)
            .await
        {
            Ok(receipt) if !receipt.replayed => (),
            Ok(_) => {
                tracing::warn!("products_entitlements.store_wallet_idempotency_conflict");
                return Err(Error::Unavailable);
            }
            Err(oteryn_wallet::Error::Refused(Refusal::InsufficientFunds)) => {
                sqlx::query("ROLLBACK TO SAVEPOINT purchase_debit")
                    .execute(&mut *tx)
                    .await?;
                return finish(tx, &request, raw, "INSUFFICIENT_BALANCE", vec![]).await;
            }
            Err(error) => {
                if matches!(
                    error,
                    oteryn_wallet::Error::Refused(Refusal::IdempotencyConflict)
                ) {
                    tracing::warn!("products_entitlements.store_wallet_idempotency_conflict");
                }
                return Err(Error::Unavailable);
            }
        }
        let after = self.wallet.read_balance_in(&mut tx, account).await?;
        let order = Uuid::now_v7();
        let line = Uuid::now_v7();
        let answer_units: Vec<_> = units
            .iter()
            .map(|unit| (unit, Uuid::now_v7(), Uuid::now_v7()))
            .collect();
        let extra = vec![
            ("order_id", text(order)),
            ("currency", text("COINS")),
            ("total_price", text(total)),
            ("available_after", text(after.available)),
            (
                "units",
                Value::Array(
                    answer_units
                        .iter()
                        .map(|(unit, entitlement, operation)| {
                            object([
                                ("unit_kind", text(&unit.unit_kind)),
                                ("entitlement_id", text(entitlement)),
                                ("delivery_operation_id", text(operation)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ];
        let answer = answer(&request, raw, "ACCEPTED", extra)?;
        save_answer(&mut tx, &request, raw, "ACCEPTED", &answer).await?;
        sqlx::query("INSERT INTO store_orders(order_id,purchase_request_id,account_id,character_id,world_id,wallet_operation_id,wallet_idempotency_key,currency,total_price) VALUES($1,$2,$3,$4,$5,$2,$6,'COINS',$7)")
            .bind(order).bind(key).bind(account).bind(uuid(&request.character_id)?).bind(uuid(&request.world_id)?).bind(reason).bind(total).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO store_order_lines(line_id,order_id,catalog_revision,offer_key,quantity,unit_price) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(line).bind(order).bind(&active).bind(&request.offer_key).bind(i32::from(request.quantity)).bind(price).execute(&mut *tx).await?;
        let issued = now()?;
        for (index, (unit, entitlement, operation)) in answer_units.iter().enumerate() {
            sqlx::query("INSERT INTO store_fulfilment_units(entitlement_id,line_id,unit_index,account_id,product_id,product_version,unit_kind,grant_key,grant_revision,quantity) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
                .bind(entitlement).bind(line).bind(index as i32).bind(account).bind(&unit.product_id).bind(&unit.product_version).bind(&unit.unit_kind).bind(&unit.grant_key).bind(&unit.grant_revision).bind(i32::from(unit.quantity)).execute(&mut *tx).await?;
            let sequence: i64 = sqlx::query_scalar(
                "SELECT nextval(pg_get_serial_sequence('store_operations','source_revision'))",
            )
            .fetch_one(&mut *tx)
            .await?;
            let intent = encode(
                &object([
                    ("contract_version", number(1)),
                    ("operation_kind", text("DELIVER")),
                    ("issuer_authority", text("OTERYN_PLATFORM_STORE_AUTHORITY")),
                    ("source_revision", text(sequence)),
                    ("operation_id", text(operation)),
                    ("order_id", text(order)),
                    ("entitlement_id", text(entitlement)),
                    ("account_id", text(account)),
                    ("product_id", text(&unit.product_id)),
                    ("product_version", text(&unit.product_version)),
                    ("unit_kind", text(&unit.unit_kind)),
                    ("catalog_revision", text(&active)),
                    ("offer_key", text(&request.offer_key)),
                    ("grant_key", text(&unit.grant_key)),
                    ("grant_revision", text(&unit.grant_revision)),
                    ("quantity", number(u64::from(unit.quantity))),
                    ("issued_at_source", text(issued)),
                    ("audience", text("OTERYN_GAME_STORE")),
                ]),
                4096,
            )?;
            sqlx::query("INSERT INTO store_operations(source_revision,operation_id,issuer_authority,operation_kind,entitlement_id,intent_bytes,binding_digest) VALUES($1,$2,'OTERYN_PLATFORM_STORE_AUTHORITY','DELIVER',$3,$4,$5)")
                .bind(sequence).bind(operation).bind(entitlement).bind(&intent).bind(format!("{:x}",Sha256::digest(&intent))).execute(&mut *tx).await?;
        }
        if referrals {
            let days: i32 = units
                .iter()
                .filter(|unit| unit.unit_kind == "PREMIUM_TIME")
                .map(|unit| i32::from(unit.quantity))
                .sum();
            sqlx::query("SELECT referral_campaign_purchase($1,$2,clock_timestamp())")
                .bind(account)
                .bind(days)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(answer)
    }
    pub async fn read(&self, raw: &[u8], history: bool) -> Result<Vec<u8>, Error> {
        self.enabled()?;
        self.source_head(
            raw,
            512,
            if history {
                "ReadStoreHistoryPageV1"
            } else {
                "ReadStoreBalanceV1"
            },
        )?;
        let request = ReadRequest::decode(raw, history)?;
        self.limit(
            "read",
            &request.account_id,
            self.settings.reads_per_account_per_minute,
        )
        .await?;
        let account = uuid(&request.account_id)?;
        let mut tx = self.begin().await?;
        match self.accounts.state(&mut tx, account).await? {
            AccountState::Missing => return Err(Error::UnknownAccount),
            AccountState::Unknown => return Err(Error::Unavailable),
            AccountState::Active | AccountState::Restricted => (),
        }
        let mut members = vec![
            ("contract_version", number(1)),
            ("result", text("OK")),
            ("account_id", text(account)),
        ];
        if history {
            let limit = request.max_entries.ok_or(Error::Invalid)?;
            let page = NumericHistoryPage {
                before: decimal(
                    request.before_entry_id.as_deref().ok_or(Error::Invalid)?,
                    true,
                )
                .ok_or(Error::Invalid)?,
                limit,
            };
            let entries = self
                .wallet
                .numeric_history_in(&mut tx, account, page)
                .await?;
            let more = entries.len() > usize::from(limit);
            let mut values = Vec::new();
            let mut cursor = 0;
            for entry in entries.into_iter().take(usize::from(limit)) {
                cursor = entry.sequence;
                let order=sqlx::query("SELECT o.order_id,l.offer_key,l.quantity FROM store_orders o JOIN store_order_lines l USING(order_id) WHERE o.wallet_operation_id=$1 AND o.account_id=$2")
                    .bind(entry.operation_id).bind(account).fetch_optional(&mut *tx).await?;
                let store = entry.kind == "store_purchase";
                if store && order.is_none() {
                    return Err(Error::Unavailable);
                }
                let kind = match entry.kind.as_str() {
                    "store_purchase" => "STORE_PURCHASE",
                    "adjustment" => "OPERATOR_ADJUSTMENT",
                    _ => "OTHER",
                };
                let (order_id, offer_key, quantity) = match order.filter(|_| store) {
                    Some(row) => (
                        text(row.try_get::<Uuid, _>("order_id")?),
                        text(row.try_get::<String, _>("offer_key")?),
                        number(row.try_get::<i32, _>("quantity")? as u64),
                    ),
                    None => (Value::Null, Value::Null, Value::Null),
                };
                values.push(object([
                    ("entry_id", text(entry.sequence)),
                    ("kind", text(kind)),
                    ("currency", text("COINS")),
                    ("available_delta", text(entry.available_delta)),
                    ("reserved_delta", text(entry.reserved_delta)),
                    ("order_id", order_id),
                    ("offer_key", offer_key),
                    ("quantity", quantity),
                    ("created_at", text(entry.created_at.unix_timestamp())),
                ]));
            }
            members.push(("entries", Value::Array(values)));
            members.push(("next_before_entry_id", text(if more { cursor } else { 0 })));
        } else {
            let balance = self.wallet.read_balance_in(&mut tx, account).await?;
            members.push((
                "balances",
                Value::Array(vec![object([
                    ("currency", text("COINS")),
                    ("available", text(balance.available)),
                    ("reserved", text(balance.reserved)),
                ])]),
            ));
            members.push(("read_at", text(now()?)));
        }
        let bytes = encode(&object(members), if history { 12288 } else { 512 })?;
        tx.commit().await?;
        Ok(bytes)
    }
}
fn now() -> Result<i64, Error> {
    let seconds = OffsetDateTime::now_utc().unix_timestamp();
    if seconds <= 0 {
        Err(Error::Unavailable)
    } else {
        Ok(seconds)
    }
}
fn answer(
    request: &PurchaseRequest,
    raw: &[u8],
    result: &str,
    extra: Vec<(&'static str, Value)>,
) -> Result<Vec<u8>, Error> {
    let mut members = vec![
        ("contract_version", number(1)),
        ("purchase_request_id", text(&request.purchase_request_id)),
        ("request_digest", text(format!("{:x}", Sha256::digest(raw)))),
        ("result", text(result)),
    ];
    members.extend(extra);
    members.push(("decided_at", text(now()?)));
    encode(&object(members), 2048)
}
async fn save_answer(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    raw: &[u8],
    result: &str,
    bytes: &[u8],
) -> Result<(), Error> {
    sqlx::query("INSERT INTO store_purchase_requests(purchase_request_id,account_id,request_bytes,request_digest,result,answer_bytes) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(uuid(&request.purchase_request_id)?).bind(uuid(&request.account_id)?).bind(raw).bind(format!("{:x}",Sha256::digest(raw))).bind(result).bind(bytes).execute(&mut **tx).await?;
    Ok(())
}
async fn finish(
    mut tx: Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    raw: &[u8],
    result: &str,
    extra: Vec<(&'static str, Value)>,
) -> Result<Vec<u8>, Error> {
    let bytes = answer(request, raw, result, extra)?;
    save_answer(&mut tx, request, raw, result, &bytes).await?;
    tx.commit().await?;
    Ok(bytes)
}
