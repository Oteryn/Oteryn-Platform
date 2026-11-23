use super::*;
use oteryn_payments::{
    AccountReader, IntentState, PaymentFlags, PaymentProvider, Payments, TestProvider,
};
use oteryn_store::premium;
use oteryn_wallet as wallet;

/// Composition point for the public T26 and T27 transaction APIs. Account and
/// audit adapters must resolve current authority, never a client-supplied claim.
#[derive(Debug)]
pub struct LedgerEffects<W, B, H, P> {
    pub wallet: wallet::Ledger<W, B, H>,
    pub premium: premium::repository::Ledger,
    pub payments: P,
}
pub trait PaymentActions: Send + Sync {
    fn inspect(
        &self,
        connection: &mut sqlx::PgConnection,
        request: &Request,
    ) -> impl Future<Output = Result<Value, Error>> + Send;

    fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &Request,
    ) -> impl Future<Output = Result<Value, Error>> + Send;
}
impl<
    W: wallet::AccountStateReader,
    B: wallet::TransferBlockCheck,
    H: wallet::AuditHook,
    P: PaymentActions,
> Effects for LedgerEffects<W, B, H, P>
{
    async fn inspect(
        &self,
        connection: &mut sqlx::PgConnection,
        request: &Request,
    ) -> Result<Value, Error> {
        match request.command {
            Command::Adjust { amount } => {
                let balance = self
                    .wallet
                    .balance_in(connection, request.target)
                    .await
                    .map_err(|e| match e {
                        wallet::Error::Refused(wallet::Refusal::AccountDenied) => Error::Denied,
                        _ => Error::Unavailable,
                    })?;
                let after = balance
                    .available
                    .checked_add(amount)
                    .filter(|v| *v >= 0)
                    .ok_or(Error::Conflict)?;
                Ok(json!({"before":balance.available,"after":after,"amount":amount}))
            }
            Command::Refund | Command::Reconcile => {
                self.payments.inspect(connection, request).await
            }
            _ => Ok(json!(null)),
        }
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        request: &Request,
        reason_hash: &str,
    ) -> Result<Value, Error> {
        if matches!(
            request.command,
            Command::Grant { .. } | Command::Adjust { amount: 1.. }
        ) && !wallet::value_eligible_in(tx, request.target).await?
        {
            return Err(Error::Denied);
        }
        match request.command {
            Command::Grant { days: _ } | Command::Revoke => {
                let change = match request.command {
                    Command::Grant { days } => premium::Change::Grant(days),
                    _ => premium::Change::Revoke,
                };
                // An approved grant has no plaintext reason left, only its digest.
                let result = self
                    .premium
                    .mutate_digest_in(
                        tx,
                        &premium::Operator {
                            actor,
                            confirmed_mfa: true,
                            permission: rbac::Permission::ManagePremiumTime.as_str().to_owned(),
                        },
                        &premium::repository::DigestMutation {
                            account: request.target,
                            request: request.request_id,
                            change,
                            reason_sha256: reason_digest(reason_hash)?,
                        },
                    )
                    .await
                    .map_err(|e| match e {
                        premium::Error::Invalid => Error::Invalid,
                        premium::Error::Denied | premium::Error::UnknownAccount => Error::Denied,
                        premium::Error::Conflict
                        | premium::Error::NotActive
                        | premium::Error::Horizon => Error::Conflict,
                        _ => Error::Unavailable,
                    })?;
                Ok(
                    json!({"entitlement":result.id,"revision":result.revision,"from":result.from,"until":result.until,"revoked":result.revoked}),
                )
            }
            Command::Adjust { amount } => {
                let result = self
                    .wallet
                    .adjust_in(
                        tx,
                        request.request_id,
                        request.target,
                        amount,
                        actor,
                        reason_hash,
                    )
                    .await
                    .map_err(|e| match e {
                        wallet::Error::Storage(e) => Error::Storage(e),
                        wallet::Error::Refused(
                            wallet::Refusal::AccountDenied | wallet::Refusal::AuditDenied,
                        ) => Error::Denied,
                        wallet::Error::Refused(wallet::Refusal::Disabled) => Error::Disabled,
                        _ => Error::Conflict,
                    })?;
                Ok(json!({"operation":result.operation_id}))
            }
            Command::Reconcile | Command::Refund => self.payments.apply(tx, request).await,
        }
    }
}
/// Decodes the stored lowercase or uppercase hex SHA-256 of a request reason.
pub(super) fn reason_digest(hex: &str) -> Result<[u8; 32], Error> {
    fn nibble(byte: u8) -> Result<u8, Error> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err(Error::Invalid),
        }
    }
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return Err(Error::Invalid);
    }
    let mut digest = [0; 32];
    for (slot, [high, low]) in digest.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *slot = (nibble(*high)? << 4) | nibble(*low)?;
    }
    Ok(digest)
}
/// A `refunding` claim younger than this belongs to a live attempt. It is four
/// times the worker's 30 s execution budget, so a call still running in a live
/// worker is never re-claimed, and short enough that a crashed attempt is
/// re-claimed well before the job's ten outbox attempts are spent.
/// Outbox attempts for one refund job; after the last one the job is dead.
pub const REFUND_MAX_ATTEMPTS: i32 = 10;
pub const REFUND_CLAIM_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(120);
/// T31 has no reconciliation writer or transactional refund API. This narrow
/// adapter uses its public intent reader and provider interface. Review only
/// records staff evidence; refund dispatch never manufactures webhook truth.
#[derive(Debug)]
pub struct TestPaymentAdapter {
    payments: Payments<TestProvider, PaymentAccounts>,
    provider: TestProvider,
    enabled: bool,
}
impl TestPaymentAdapter {
    pub fn new(pool: PgPool, provider: TestProvider, flags: PaymentFlags) -> Self {
        let flags = PaymentFlags {
            enabled: flags.enabled && std::env::var("OTERYN_ENV").as_deref() != Ok("production"),
        };
        let accounts = PaymentAccounts(oteryn_identity::accounts::Accounts { pool: pool.clone() });
        Self {
            payments: Payments::new(pool, provider.clone(), accounts, flags),
            provider,
            enabled: flags.enabled,
        }
    }
    /// Worker integration point. The provider contract requires a stable refund
    /// idempotency key per intent, so a crash after the call can safely retry.
    /// A signed T31 webhook remains the only authority for payment state.
    pub async fn deliver_refund(&self, pool: &PgPool, operation: Uuid) -> Result<(), Error> {
        self.deliver_refund_with(&self.provider, pool, operation)
            .await
    }
    /// Claim, call, finalize: the provider call runs outside any transaction
    /// and holds no row lock. Its idempotency key is the intent id (one refund
    /// per intent), so a re-claimed attempt repeats the same remote refund.
    pub(super) async fn deliver_refund_with<R: PaymentProvider>(
        &self,
        provider: &R,
        pool: &PgPool,
        operation: Uuid,
    ) -> Result<(), Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let Some((attempt, intent)) = self.claim_refund(provider, pool, operation).await? else {
            return Ok(());
        };
        let reference = match provider.refund(intent.id, &intent.amount).await {
            Ok(reference)
                if !reference.is_empty()
                    && reference.len() <= 128
                    && !reference.chars().any(char::is_control) =>
            {
                reference
            }
            // These provider responses reject the bound request. An unavailable
            // provider or a malformed success leaves the remote outcome unknown.
            Err(
                oteryn_payments::Error::InvalidInput
                | oteryn_payments::Error::AccountDenied
                | oteryn_payments::Error::NotFound
                | oteryn_payments::Error::IllegalTransition,
            ) => {
                return refuse_refund(pool, operation, intent.id, attempt).await;
            }
            outcome => {
                // A failed release leaves the claim to go stale and be re-claimed;
                // the caller still sees the provider outcome.
                let _ = release_refund(pool, operation, attempt).await;
                return Err(outcome.map_or_else(payment_error, |_| Error::Unavailable));
            }
        };
        let mut tx = pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        if record_dispatch(&mut tx, operation, intent.id, &reference, Some(attempt)).await? {
            tx.commit().await?;
            return Ok(());
        }
        // Another attempt re-claimed the row; only its outcome may be recorded.
        let settled: Option<String> = sqlx::query_scalar(
            "SELECT provider_reference FROM commerce_refunds WHERE operation_id=$1",
        )
        .bind(operation)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        settled.map(|_| ()).ok_or(Error::Unavailable)
    }
    /// Short transaction: validate the approved refund and move it to
    /// `refunding` under a fresh attempt id. `None` means already settled.
    async fn claim_refund<R: PaymentProvider>(
        &self,
        provider: &R,
        pool: &PgPool,
        operation: Uuid,
    ) -> Result<Option<(Uuid, oteryn_payments::Intent)>, Error> {
        let mut tx = pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        // Reconciliation, webhook intake and staff mutations lock the payer
        // before the refund and intent. Take the shared account lock first.
        let payer: Uuid =
            sqlx::query_scalar("SELECT account_id FROM commerce_refunds WHERE operation_id=$1")
                .bind(operation)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::Denied)?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
            .bind(payer)
            .execute(&mut *tx)
            .await?;
        rbac::mutation_budget(&mut tx).await?;
        let row = sqlx::query("SELECT r.intent_id,r.account_id,r.amount,r.currency,r.provider,r.provider_reference,o.result->>'refund_outcome' AS refund_outcome,r.state='refunding' AND r.claimed_at > now() - make_interval(secs => $2) AS claim_live FROM commerce_refunds r JOIN commerce_operations o ON o.id=r.operation_id WHERE r.operation_id=$1 AND r.account_id=$3 AND o.approved_by IS NOT NULL AND o.result IS NOT NULL FOR UPDATE OF r,o")
            .bind(operation).bind(REFUND_CLAIM_STALE_AFTER.as_secs_f64()).bind(payer).fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        if row
            .try_get::<Option<String>, _>("provider_reference")?
            .is_some()
            || row
                .try_get::<Option<String>, _>("refund_outcome")?
                .as_deref()
                == Some("refused")
        {
            return Ok(None);
        }
        if row.try_get::<String, _>("provider")? != provider.name() {
            return Err(Error::Denied);
        }
        let intent = self
            .payments
            .intent_in(&mut tx, row.try_get("intent_id")?)
            .await
            .map_err(payment_error)?;
        if intent.account_id != row.try_get::<Uuid, _>("account_id")?
            || intent.amount.minor_units != row.try_get::<i64, _>("amount")?
            || intent.amount.currency != row.try_get::<String, _>("currency")?
        {
            return Err(Error::Conflict);
        }
        match intent.state {
            IntentState::Succeeded => ensure_uncredited(&mut tx, intent.id).await?,
            IntentState::Refunded => {
                record_dispatch(
                    &mut tx,
                    operation,
                    intent.id,
                    "verified-by-payment-ledger",
                    None,
                )
                .await?;
                tx.commit().await?;
                return Ok(None);
            }
            _ => return Err(Error::Conflict),
        }
        if row.try_get::<Option<bool>, _>("claim_live")? == Some(true) {
            return Err(Error::Unavailable);
        }
        let attempt = Uuid::now_v7();
        sqlx::query("UPDATE commerce_refunds SET state='refunding',attempt_id=$2,claimed_at=now(),updated_at=now() WHERE operation_id=$1")
            .bind(operation).bind(attempt).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some((attempt, intent)))
    }
}
/// Records the provider reference once. With an attempt id it applies only
/// while that attempt still owns the `refunding` claim. Returns whether it wrote.
async fn record_dispatch(
    tx: &mut Transaction<'_, Postgres>,
    operation: Uuid,
    intent: Uuid,
    reference: &str,
    attempt: Option<Uuid>,
) -> Result<bool, Error> {
    let written = sqlx::query("UPDATE commerce_refunds SET provider_reference=$2,state='done',updated_at=now() WHERE operation_id=$1 AND provider_reference IS NULL AND ($3::uuid IS NULL OR (attempt_id=$3 AND state='refunding'))")
        .bind(operation).bind(reference).bind(attempt).execute(&mut **tx).await?.rows_affected();
    if written != 1 {
        return Ok(false);
    }
    audit::append(
        tx,
        audit::Entry {
            context: audit::AuditContext {
                actor: None,
                request_id: operation,
            },
            action: "commerce.refund.dispatched",
            target: &intent.to_string(),
            before: json!(null),
            after: json!({"operation":operation,"awaiting_verified_settlement":true}),
        },
    )
    .await?;
    Ok(true)
}
/// The operation result records a definitive provider refusal independently of
/// the payment ledger. Only the current claimant may release wallet delivery.
async fn refuse_refund(
    pool: &PgPool,
    operation: Uuid,
    intent: Uuid,
    attempt: Uuid,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    let released = sqlx::query("UPDATE commerce_refunds SET state='pending',attempt_id=NULL,claimed_at=NULL,updated_at=now() WHERE operation_id=$1 AND attempt_id=$2 AND state='refunding' AND provider_reference IS NULL")
        .bind(operation).bind(attempt).execute(&mut *tx).await?.rows_affected();
    if released != 1 {
        return Err(Error::Unavailable);
    }
    sqlx::query("UPDATE commerce_operations SET result=result || jsonb_build_object('refund_outcome','refused'::text,'queued',false),updated_at=now() WHERE id=$1")
        .bind(operation).execute(&mut *tx).await?;
    // The original job can be dead or finishing its final attempt. A fresh
    // delivery uses the same intent key in the wallet ledger.
    sqlx::query("INSERT INTO outbox(kind,payload,idempotency_key,max_attempts) SELECT kind,payload,$2,max_attempts FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1 AND state<>'done' ON CONFLICT(kind,idempotency_key) DO NOTHING")
        .bind(intent.to_string()).bind(format!("refund-refused:{operation}")).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
/// Returns a failed attempt's claim to `pending` so the outbox retry can
/// dispatch again at once; a re-claimed or settled row is left untouched.
async fn release_refund(pool: &PgPool, operation: Uuid, attempt: Uuid) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    sqlx::query("UPDATE commerce_refunds SET state='pending',attempt_id=NULL,claimed_at=NULL,updated_at=now() WHERE operation_id=$1 AND attempt_id=$2 AND state='refunding' AND provider_reference IS NULL")
        .bind(operation).bind(attempt).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
// The caller holds the payment intent lock, which wallet delivery also acquires.
async fn ensure_uncredited(connection: &mut sqlx::PgConnection, intent: Uuid) -> Result<(), Error> {
    let credited: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM wallet_operations WHERE id=$1 AND kind='top_up')",
    )
    .bind(intent)
    .fetch_one(connection)
    .await?;
    if credited {
        return Err(Error::Conflict);
    }
    Ok(())
}
impl PaymentActions for TestPaymentAdapter {
    async fn inspect(
        &self,
        connection: &mut sqlx::PgConnection,
        request: &Request,
    ) -> Result<Value, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let intent = self
            .payments
            .intent_in(connection, request.target)
            .await
            .map_err(payment_error)?;
        if matches!(request.command, Command::Refund) {
            if intent.state != IntentState::Succeeded {
                return Err(Error::Conflict);
            }
            ensure_uncredited(connection, intent.id).await?;
        }
        Ok(
            json!({"account":intent.account_id,"amount":intent.amount.minor_units,"currency":intent.amount.currency,"state":intent.state}),
        )
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &Request,
    ) -> Result<Value, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let intent = self
            .payments
            .intent_in(tx, request.target)
            .await
            .map_err(payment_error)?;
        match request.command {
            Command::Reconcile => Ok(
                json!({"resolution_code":"reviewed_no_payment_state_change","state":intent.state,"intent":intent.id}),
            ),
            Command::Refund => {
                if intent.state != IntentState::Succeeded {
                    return Err(Error::Conflict);
                }
                ensure_uncredited(tx, intent.id).await?;
                let inserted = sqlx::query("INSERT INTO commerce_refunds(operation_id,intent_id,account_id,amount,currency,provider) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
                    .bind(request.request_id).bind(intent.id).bind(intent.account_id).bind(intent.amount.minor_units).bind(&intent.amount.currency).bind(self.provider.name()).execute(&mut **tx).await?.rows_affected();
                if inserted != 1 {
                    return Err(Error::Conflict);
                }
                oteryn_db::outbox::enqueue(
                    tx,
                    "commerce.refund",
                    &json!({"operation":request.request_id}).to_string(),
                    &intent.id.to_string(),
                    oteryn_db::outbox::OffsetDateTime::now_utc(),
                    REFUND_MAX_ATTEMPTS,
                )
                .await?;
                Ok(
                    json!({"intent":intent.id,"queued":true,"amount":intent.amount.minor_units,"currency":intent.amount.currency}),
                )
            }
            _ => Err(Error::Invalid),
        }
    }
}
fn payment_error(error: oteryn_payments::Error) -> Error {
    match error {
        oteryn_payments::Error::Disabled => Error::Disabled,
        oteryn_payments::Error::AccountDenied => Error::Denied,
        oteryn_payments::Error::InvalidInput | oteryn_payments::Error::NotFound => Error::Invalid,
        oteryn_payments::Error::IllegalTransition => Error::Conflict,
        _ => Error::Unavailable,
    }
}
/// Missing domain authority always denies; enabling a route cannot grant it.
#[derive(Debug, Clone)]
pub struct PendingEffects;
impl Effects for PendingEffects {
    async fn inspect(&self, _: &mut sqlx::PgConnection, _: &Request) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    async fn apply(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: Uuid,
        _: &Request,
        _: &str,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
}

#[derive(Debug)]
struct PaymentAccounts(oteryn_identity::accounts::Accounts);
impl AccountReader for PaymentAccounts {
    async fn state(
        &self,
        id: Uuid,
    ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
        Ok(match self.0.find_by_id(id).await? {
            Some(account) if account.status == "active" => oteryn_payments::AccountState::Active,
            _ => oteryn_payments::AccountState::Unknown,
        })
    }
    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
        let active: bool =
            sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR SHARE")
                .bind(id)
                .fetch_optional(connection)
                .await?
                .unwrap_or(false);
        Ok(if active {
            oteryn_payments::AccountState::Active
        } else {
            oteryn_payments::AccountState::Unknown
        })
    }
}

/// At-least-once worker adapter. It accepts only the owning job kind and a
/// bounded, typed payload; provider dispatch is fenced by the durable refund row.
#[derive(Debug)]
pub struct RefundDelivery {
    pub pool: PgPool,
    pub adapter: TestPaymentAdapter,
}
impl RefundDelivery {
    pub async fn run(&self, job: &oteryn_db::outbox::Job) -> Result<(), Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Payload {
            operation: Uuid,
        }
        if job.kind != "commerce.refund" || job.payload.len() > 256 {
            return Err(Error::Invalid);
        }
        let payload: Payload = serde_json::from_str(&job.payload).map_err(|_| Error::Invalid)?;
        if payload.operation.get_version_num() != 7 {
            return Err(Error::Invalid);
        }
        let result = self
            .adapter
            .deliver_refund(&self.pool, payload.operation)
            .await;
        if let Err(error) = &result
            && let Some((state, attempt)) =
                exhausted_refund(&self.pool, job.attempts, payload.operation).await
        {
            tracing::error!(
                operation = %payload.operation,
                state,
                attempt_id = ?attempt,
                ?error,
                "commerce refund retries exhausted; the refund row needs operator reconciliation"
            );
        }
        result
    }
}
/// On the job's last attempt, the state and attempt id of a refund that is
/// still unsettled; `None` before the last attempt or once settled. A failed
/// read reports `unreadable` so the caller still logs and keeps its error.
pub(super) async fn exhausted_refund(
    pool: &PgPool,
    attempts: i32,
    operation: Uuid,
) -> Option<(String, Option<Uuid>)> {
    if attempts < REFUND_MAX_ATTEMPTS {
        return None;
    }
    sqlx::query_as::<_, (String, Option<Uuid>)>(
        "SELECT state,attempt_id FROM commerce_refunds WHERE operation_id=$1 AND provider_reference IS NULL",
    )
    .bind(operation)
    .fetch_optional(pool)
    .await
    .unwrap_or_else(|_| Some(("unreadable".into(), None)))
}
