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
    fn inspect(&self, request: &Request) -> impl Future<Output = Result<Value, Error>> + Send;

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
    async fn inspect(&self, request: &Request) -> Result<Value, Error> {
        match request.command {
            Command::Adjust { amount } => {
                let balance = self
                    .wallet
                    .balance(request.target)
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
            Command::Refund | Command::Reconcile => self.payments.inspect(request).await,
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
        match request.command {
            Command::Grant { days: _ } | Command::Revoke => {
                let change = match request.command {
                    Command::Grant { days } => premium::Change::Grant(days),
                    _ => premium::Change::Revoke,
                };
                let result = self
                    .premium
                    .mutate_in(
                        tx,
                        &premium::Operator {
                            actor,
                            confirmed_mfa: true,
                            permission: rbac::Permission::ManagePremiumTime.as_str().to_owned(),
                        },
                        &premium::Mutation {
                            account: request.target,
                            request: request.request_id,
                            change,
                            reason: request.reason.clone(),
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
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let mut tx = pool.begin().await?;
        let row = sqlx::query("SELECT r.intent_id,r.account_id,r.amount,r.currency,r.provider,r.provider_reference FROM commerce_refunds r JOIN commerce_operations o ON o.id=r.operation_id WHERE r.operation_id=$1 AND o.approved_by IS NOT NULL AND o.result IS NOT NULL FOR UPDATE OF r")
            .bind(operation).fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        if row
            .try_get::<Option<String>, _>("provider_reference")?
            .is_some()
        {
            return Ok(());
        }
        if row.try_get::<String, _>("provider")? != self.provider.name() {
            return Err(Error::Denied);
        }
        let intent = self
            .payments
            .intent(row.try_get("intent_id")?)
            .await
            .map_err(payment_error)?;
        if intent.account_id != row.try_get::<Uuid, _>("account_id")?
            || intent.amount.minor_units != row.try_get::<i64, _>("amount")?
            || intent.amount.currency != row.try_get::<String, _>("currency")?
        {
            return Err(Error::Conflict);
        }
        let reference = match intent.state {
            IntentState::Succeeded => self
                .provider
                .refund(intent.id, &intent.amount)
                .await
                .map_err(payment_error)?,
            IntentState::Refunded => "verified-by-payment-ledger".to_owned(),
            _ => return Err(Error::Conflict),
        };
        if reference.is_empty() || reference.len() > 128 || reference.chars().any(char::is_control)
        {
            return Err(Error::Unavailable);
        }
        sqlx::query("UPDATE commerce_refunds SET provider_reference=$2,updated_at=now() WHERE operation_id=$1").bind(operation).bind(reference).execute(&mut *tx).await?;
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: None,
                    request_id: operation,
                },
                action: "commerce.refund.dispatched",
                target: &intent.id.to_string(),
                before: json!(null),
                after: json!({"operation":operation,"awaiting_verified_settlement":true}),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
impl PaymentActions for TestPaymentAdapter {
    async fn inspect(&self, request: &Request) -> Result<Value, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let intent = self
            .payments
            .intent(request.target)
            .await
            .map_err(payment_error)?;
        if matches!(request.command, Command::Refund) && intent.state != IntentState::Succeeded {
            return Err(Error::Conflict);
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
            .intent(request.target)
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
                    10,
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
    async fn inspect(&self, _: &Request) -> Result<Value, Error> {
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
        self.adapter
            .deliver_refund(&self.pool, payload.operation)
            .await
    }
}
