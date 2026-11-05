use oteryn_db::outbox;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    AccountReader, AccountState, Amount, Error, Intent, IntentState, PaymentFlags, PaymentProvider,
    TopUpJob, core::identifier,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    Applied,
    Replay,
}

/// Core API only: callers provide account policy and the provider; no wallet dependency.
#[derive(Debug)]
pub struct Payments<P, A> {
    pool: PgPool,
    provider: P,
    accounts: A,
    flags: PaymentFlags,
}

impl<P: PaymentProvider, A: AccountReader> Payments<P, A> {
    pub fn new(pool: PgPool, provider: P, accounts: A, flags: PaymentFlags) -> Self {
        Self {
            pool,
            provider,
            accounts,
            flags,
        }
    }

    fn enabled(&self) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        Ok(())
    }

    async fn eligible(&self, account_id: Uuid) -> Result<(), Error> {
        if !matches!(
            self.accounts.state(account_id).await,
            Ok(AccountState::Active)
        ) {
            return Err(Error::AccountDenied);
        }
        Ok(())
    }

    pub async fn create_intent(&self, account_id: Uuid, amount: Amount) -> Result<Intent, Error> {
        self.enabled()?;
        amount.validate()?;
        if account_id.is_nil() {
            return Err(Error::InvalidInput);
        }
        identifier(self.provider.name())?;
        self.eligible(account_id).await?;
        let id = oteryn_core::new_id();
        let reference = self.provider.create_intent(id, &amount).await?;
        identifier(&reference)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO payment_intents (id, account_id, provider, provider_reference, amount, currency, state) VALUES ($1,$2,$3,$4,$5,$6,'created')")
            .bind(id).bind(account_id).bind(self.provider.name()).bind(&reference)
            .bind(amount.minor_units).bind(&amount.currency).execute(&mut *tx).await?;
        IntentState::Created.transition(IntentState::Pending)?;
        sqlx::query(
            "UPDATE payment_intents SET state = 'pending', updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Intent {
            id,
            account_id,
            amount,
            state: IntentState::Pending,
            provider_reference: reference,
        })
    }

    pub async fn intent(&self, id: Uuid) -> Result<Intent, Error> {
        self.enabled()?;
        let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2")
            .bind(id).bind(self.provider.name()).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let account_id = row.try_get("account_id")?;
        self.eligible(account_id).await?;
        Ok(Intent {
            id,
            account_id,
            amount: Amount {
                minor_units: row.try_get("amount")?,
                currency: row.try_get("currency")?,
            },
            state: IntentState::parse(row.try_get("state")?)?,
            provider_reference: row.try_get("provider_reference")?,
        })
    }

    /// A duplicate event is a no-op, including concurrent deliveries. Validation, state change,
    /// invoice and outbox enqueue either all commit or all roll back.
    pub async fn webhook(&self, body: &[u8], signature: &[u8]) -> Result<WebhookOutcome, Error> {
        self.enabled()?;
        if body.len() > 65536 {
            return Err(Error::InvalidInput);
        }
        self.provider.verify_webhook_signature(body, signature)?;
        let event = self.provider.parse_event(body)?;
        event.validate()?;
        identifier(self.provider.name())?;
        let mut tx = self.pool.begin().await?;
        // Lock before inserting an event: distinct concurrent events cannot upgrade FK locks.
        let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR UPDATE")
            .bind(event.intent_id).bind(self.provider.name()).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let inserted = sqlx::query("INSERT INTO payment_events (id, provider, provider_event_id, intent_id, state) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (provider,provider_event_id) DO NOTHING")
            .bind(oteryn_core::new_id()).bind(self.provider.name()).bind(&event.event_id).bind(event.intent_id).bind(event.state.as_str())
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            tx.commit().await?;
            return Ok(WebhookOutcome::Replay);
        }
        let amount: i64 = row.try_get("amount")?;
        let currency: String = row.try_get("currency")?;
        let account_id: Uuid = row.try_get("account_id")?;
        if amount != event.amount.minor_units
            || currency != event.amount.currency
            || row.try_get::<String, _>("provider_reference")? != event.provider_reference
        {
            return Err(Error::InvalidInput);
        }
        IntentState::parse(row.try_get("state")?)?.transition(event.state)?;
        self.eligible(account_id).await?;
        sqlx::query("UPDATE payment_intents SET state=$2, updated_at=now() WHERE id=$1")
            .bind(event.intent_id)
            .bind(event.state.as_str())
            .execute(&mut *tx)
            .await?;
        if event.state == IntentState::Succeeded {
            sqlx::query("INSERT INTO invoices (id,intent_id,account_id,amount,currency) VALUES ($1,$2,$3,$4,$5)")
                .bind(oteryn_core::new_id()).bind(event.intent_id).bind(account_id).bind(amount).bind(&currency).execute(&mut *tx).await?;
            let payload = serde_json::to_string(&TopUpJob {
                intent_id: event.intent_id,
                account_id,
                amount,
                currency,
            })
            .map_err(|_| Error::InvalidInput)?;
            outbox::enqueue(
                &mut tx,
                "wallet.top_up",
                &payload,
                &event.intent_id.to_string(),
                outbox::OffsetDateTime::now_utc(),
                10,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(WebhookOutcome::Applied)
    }
}
