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

/// Discover the immutable intent owner before taking the intent row. Staff
/// reconciliation retains accounts before its intent read, so every provider
/// and delivery path must use the same edge. Eligibility is checked later,
/// after the retained intent wait, through the owning policy.
pub(crate) async fn prepare_intent_account(
    connection: &mut sqlx::PgConnection,
    intent: Uuid,
    provider: &str,
) -> Result<Uuid, Error> {
    oteryn_db::mutation_budget(connection).await?;
    let account: Uuid =
        sqlx::query_scalar("SELECT account_id FROM payment_intents WHERE id=$1 AND provider=$2")
            .bind(intent)
            .bind(provider)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or(Error::NotFound)?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
        .bind(account)
        .execute(connection)
        .await?;
    Ok(account)
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

    /// Only a read `Active` admits; a read non-active state denies. A failed read
    /// (timeout, storage error) is `Unavailable`: retryable, never a denial.
    async fn eligible(&self, account_id: Uuid) -> Result<(), Error> {
        match self.accounts.state(account_id).await {
            Ok(AccountState::Active) => Ok(()),
            Ok(_) => Err(Error::AccountDenied),
            Err(_) => Err(Error::Unavailable),
        }
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

    /// Idempotent creation under a caller-reserved UUIDv7 intent ID, which is also
    /// the provider creation key. A replay returns the stored intent when account
    /// and amount match; a different binding is refused.
    pub async fn ensure_intent(
        &self,
        id: Uuid,
        account_id: Uuid,
        amount: Amount,
    ) -> Result<Intent, Error> {
        self.enabled()?;
        amount.validate()?;
        if account_id.is_nil() || id.get_version_num() != 7 {
            return Err(Error::InvalidInput);
        }
        identifier(self.provider.name())?;
        let existing = |intent: Intent| {
            if intent.account_id == account_id && intent.amount == amount {
                Ok(intent)
            } else {
                Err(Error::InvalidInput)
            }
        };
        match self.intent(id).await {
            Ok(intent) => return existing(intent),
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
        }
        self.eligible(account_id).await?;
        let reference = self.provider.create_intent(id, &amount).await?;
        identifier(&reference)?;
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        match self.accounts.state_in(&mut tx, account_id).await {
            Ok(AccountState::Active) => {}
            Ok(_) => return Err(Error::AccountDenied),
            Err(_) => return Err(Error::Unavailable),
        }
        let inserted = sqlx::query("INSERT INTO payment_intents (id, account_id, provider, provider_reference, amount, currency, state) VALUES ($1,$2,$3,$4,$5,$6,'created') ON CONFLICT DO NOTHING")
            .bind(id).bind(account_id).bind(self.provider.name()).bind(&reference)
            .bind(amount.minor_units).bind(&amount.currency).execute(&mut *tx).await?.rows_affected();
        let intent = if inserted == 1 {
            IntentState::Created.transition(IntentState::Pending)?;
            sqlx::query(
                "UPDATE payment_intents SET state = 'pending', updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            Intent {
                id,
                account_id,
                amount,
                state: IntentState::Pending,
                provider_reference: reference,
            }
        } else {
            existing(self.intent_in(&mut tx, id).await?)?
        };
        tx.commit().await?;
        Ok(intent)
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

    /// Read and authorize without acquiring a second pool connection.
    pub async fn intent_in(
        &self,
        connection: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Intent, Error> {
        self.enabled()?;
        let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR SHARE")
            .bind(id).bind(self.provider.name()).fetch_optional(&mut *connection).await?.ok_or(Error::NotFound)?;
        let account_id = row.try_get("account_id")?;
        if self.accounts.state_in(connection, account_id).await? != AccountState::Active {
            return Err(Error::AccountDenied);
        }
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
        let prepared_account =
            prepare_intent_account(&mut tx, event.intent_id, self.provider.name()).await?;
        // Lock before inserting an event: distinct concurrent events cannot upgrade FK locks.
        let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR UPDATE")
            .bind(event.intent_id).bind(self.provider.name()).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if row.try_get::<Uuid, _>("account_id")? != prepared_account {
            return Err(Error::InvalidInput);
        }
        // Native coin orders apply their own events (`CoinPurchases::webhook`).
        let coin: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM coin_purchase_orders WHERE payment_intent_id=$1)",
        )
        .bind(event.intent_id)
        .fetch_one(&mut *tx)
        .await?;
        if coin {
            return Err(Error::NotFound);
        }
        let inserted = sqlx::query("INSERT INTO payment_events (id, provider, provider_event_id, intent_id, state) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (provider,provider_event_id) DO NOTHING")
            .bind(oteryn_core::new_id()).bind(self.provider.name()).bind(&event.event_id).bind(event.intent_id).bind(event.state.as_str())
            .execute(&mut *tx).await?.rows_affected();
        let amount: i64 = row.try_get("amount")?;
        let currency: String = row.try_get("currency")?;
        let account_id: Uuid = row.try_get("account_id")?;
        if amount != event.amount.minor_units
            || currency != event.amount.currency
            || row.try_get::<String, _>("provider_reference")? != event.provider_reference
        {
            return Err(Error::InvalidInput);
        }
        if inserted == 0 {
            let same: bool = sqlx::query_scalar("SELECT intent_id=$3 AND state=$4 FROM payment_events WHERE provider=$1 AND provider_event_id=$2")
                .bind(self.provider.name()).bind(&event.event_id).bind(event.intent_id).bind(event.state.as_str())
                .fetch_one(&mut *tx).await?;
            if !same {
                return Err(Error::InvalidInput);
            }
            tx.commit().await?;
            return Ok(WebhookOutcome::Replay);
        }
        if event.state == IntentState::Refunded {
            // Delivery takes this same intent lock before crediting. A refund
            // may win before delivery, but cannot erase an existing coin credit.
            // Generic top-ups without a browser checkout have no reconciliation
            // or reversal consumer, so their refunds are never acknowledged.
            let refundable: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM store_web_topups t WHERE t.payment_intent_id=$1) \
                 AND NOT EXISTS(SELECT 1 FROM store_web_topups t JOIN wallet_operations w ON w.id=t.payment_intent_id AND w.kind='top_up' WHERE t.payment_intent_id=$1)",
            )
            .bind(event.intent_id)
            .fetch_one(&mut *tx)
            .await?;
            if !refundable {
                return Err(Error::IllegalTransition);
            }
        }
        IntentState::parse(row.try_get("state")?)?.transition(event.state)?;
        let state = if event.state == IntentState::Refunded {
            self.accounts.refund_state_in(&mut tx, account_id).await
        } else {
            self.accounts.state_in(&mut tx, account_id).await
        };
        match state {
            Ok(AccountState::Active) => {}
            Ok(_) => return Err(Error::AccountDenied),
            Err(_) => return Err(Error::Unavailable),
        }
        if event.state == IntentState::Succeeded {
            let coins: Option<i64> = sqlx::query_scalar(
                "SELECT coins FROM store_web_topups WHERE payment_intent_id=$1 AND account_id=$2 FOR SHARE",
            )
            .bind(event.intent_id)
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(coins) = coins {
                check_wallet_capacity(&mut tx, account_id, coins).await?;
            }
        }
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

/// Serialize the checkout decision with wallet mutations. Include reserved
/// coins and capacity already promised to incoming payouts.
async fn check_wallet_capacity(
    tx: &mut sqlx::PgConnection,
    account: Uuid,
    coins: i64,
) -> Result<(), Error> {
    if !(1..=oteryn_wallet::MAX_BALANCE).contains(&coins) {
        return Err(Error::InvalidInput);
    }
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
    let incoming = oteryn_wallet::incoming_capacity_in(tx, account)
        .await
        .map_err(|_| Error::Unavailable)?;
    let fits: bool = sqlx::query_scalar(
        "SELECT COALESCE(sum(available_delta),0) + COALESCE(sum(reserved_delta),0) + $2::bigint + $3::bigint <= $4::bigint FROM wallet_entries WHERE account_id=$1",
    ).bind(account).bind(incoming).bind(coins).bind(oteryn_wallet::MAX_BALANCE).fetch_one(tx).await?;
    if !fits {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
