//! Durable native coin orders: quotes, atomic order creation, the signed sandbox
//! webhook and the single wallet credit. Identity, character ownership and the
//! wallet are supplied by the composing binary through `CoinHost`.
use std::{future::Future, pin::Pin, sync::Arc};

use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction, postgres::PgRow};
use uuid::Uuid;

use crate::coin::{
    CoinBalance, CoinCatalogue, CoinOrder, CoinOrderRequest, CoinPackage, CoinSettings, MAX_COINS,
    MAX_RECENT_ORDERS, OrderState, Price, VERSION,
};
use crate::{
    AccountReader, AccountState, Amount, Error, Intent, IntentState, PaymentEvent, PaymentFlags,
    PaymentProvider, Payments, TestProvider, WebhookOutcome,
};

/// Refusals mapped one to one onto the contract's HTTP codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CoinError {
    #[error("invalid data")]
    Invalid,
    #[error("authentication required")]
    Unauthorized,
    #[error("not permitted")]
    Forbidden,
    #[error("unknown request")]
    NotFound,
    #[error("request conflicts with durable state")]
    Conflict,
    #[error("temporarily unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for CoinError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    NotOwned,
    Unavailable,
}

/// One consistent wallet read for the payer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletSnapshot {
    pub revision: i64,
    pub total: i64,
    pub available: i64,
    pub transferable: i64,
}

/// Adapters owned by the composing binary. Failures must never grant access.
pub trait CoinHost: Send + Sync {
    /// Resolve a `game:account-services` bearer token to its current account.
    fn payer<'a>(&'a self, bearer: &'a str) -> HostFuture<'a, Result<Uuid, CoinError>>;
    /// Whether the payer currently owns the character.
    fn ownership(&self, payer: Uuid, character: Uuid) -> HostFuture<'_, Ownership>;
    /// Purchase eligibility, read (and locked) in the caller's transaction.
    fn eligible_in<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        payer: Uuid,
    ) -> HostFuture<'a, Result<bool, CoinError>>;
    /// Credit transferable coins once, keyed by `key`, in the caller's transaction.
    fn credit_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<(), CoinError>>;
    fn wallet(&self, payer: Uuid) -> HostFuture<'_, Result<WalletSnapshot, CoinError>>;
}

/// Payments account policy through the host; errors deny.
pub struct HostAccounts {
    pool: PgPool,
    host: Arc<dyn CoinHost>,
}
impl std::fmt::Debug for HostAccounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostAccounts").finish_non_exhaustive()
    }
}
impl AccountReader for HostAccounts {
    async fn state(&self, account_id: Uuid) -> Result<AccountState, Error> {
        let mut connection = self.pool.acquire().await?;
        Ok(
            match self.host.eligible_in(&mut connection, account_id).await {
                Ok(true) => AccountState::Active,
                Ok(false) => AccountState::Restricted,
                Err(_) => AccountState::Unknown,
            },
        )
    }
}

macro_rules! order_columns {
    () => {
        "id,payer_account_id,payment_intent_id,request_id,revision,package_id,quote_id,character_id,coins,minor_units,currency,exponent,state,checkout_url,receipt_id,review_required,code"
    };
}

#[derive(Debug, Clone)]
struct OrderRow {
    order: CoinOrder,
    payer: Uuid,
    intent: Uuid,
}
fn order_row(row: &PgRow) -> Result<OrderRow, CoinError> {
    let exponent: i16 = row.try_get("exponent")?;
    let state: String = row.try_get("state")?;
    Ok(OrderRow {
        payer: row.try_get("payer_account_id")?,
        intent: row.try_get("payment_intent_id")?,
        order: CoinOrder {
            version: VERSION,
            order_id: row.try_get("id")?,
            request_id: row.try_get("request_id")?,
            revision: row.try_get("revision")?,
            package_id: row.try_get("package_id")?,
            quote_id: row.try_get("quote_id")?,
            character_id: row.try_get("character_id")?,
            coins: row.try_get("coins")?,
            price: Price {
                minor_units: row.try_get("minor_units")?,
                currency: row.try_get("currency")?,
                exponent: u8::try_from(exponent).map_err(|_| CoinError::Unavailable)?,
            },
            sandbox: true,
            state: OrderState::parse(&state).ok_or(CoinError::Unavailable)?,
            checkout_url: row.try_get("checkout_url")?,
            receipt_id: row.try_get("receipt_id")?,
            review_required: row.try_get("review_required")?,
            code: row.try_get("code")?,
        },
    })
}
/// An identical replay returns the current order; any changed binding conflicts.
fn replay(existing: OrderRow, request: &CoinOrderRequest) -> Result<CoinOrder, CoinError> {
    let order = existing.order;
    if order.package_id == request.package_id
        && order.quote_id == request.quote_id
        && order.character_id == request.character_id
    {
        Ok(order)
    } else {
        Err(CoinError::Conflict)
    }
}
enum Created {
    Inserted(OrderRow),
    Existing(OrderRow),
}

/// The sandbox coin purchase service. Only `TestProvider` is accepted.
pub struct CoinPurchases {
    pool: PgPool,
    settings: CoinSettings,
    provider: TestProvider,
    payments: Payments<TestProvider, HostAccounts>,
    host: Arc<dyn CoinHost>,
}
impl std::fmt::Debug for CoinPurchases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoinPurchases").finish_non_exhaustive()
    }
}

impl CoinPurchases {
    pub fn new(
        pool: PgPool,
        settings: CoinSettings,
        provider: TestProvider,
        host: Arc<dyn CoinHost>,
    ) -> Self {
        let payments = Payments::new(
            pool.clone(),
            provider.clone(),
            HostAccounts {
                pool: pool.clone(),
                host: host.clone(),
            },
            PaymentFlags {
                enabled: settings.enabled,
            },
        );
        Self {
            pool,
            settings,
            provider,
            payments,
            host,
        }
    }

    fn enabled(&self) -> Result<(), CoinError> {
        if self.settings.enabled {
            Ok(())
        } else {
            Err(CoinError::Unavailable)
        }
    }

    /// The payer always comes from the token, never from the request.
    pub async fn payer(&self, bearer: &str) -> Result<Uuid, CoinError> {
        self.enabled()?;
        self.host.payer(bearer).await
    }

    async fn owned(&self, payer: Uuid, character: Uuid) -> Result<(), CoinError> {
        match self.host.ownership(payer, character).await {
            Ownership::Owned => Ok(()),
            Ownership::NotOwned => Err(CoinError::Forbidden),
            Ownership::Unavailable => Err(CoinError::Unavailable),
        }
    }

    /// Issues one fresh quote per configured package for this payer and context.
    pub async fn catalogue(
        &self,
        payer: Uuid,
        character: Option<Uuid>,
    ) -> Result<CoinCatalogue, CoinError> {
        self.enabled()?;
        if let Some(character) = character {
            self.owned(payer, character).await?;
        }
        let mut tx = self.pool.begin().await?;
        let mut packages = Vec::with_capacity(self.settings.packages.len());
        for package in &self.settings.packages {
            let quote = oteryn_core::new_id();
            sqlx::query("INSERT INTO coin_purchase_quotes (id,payer_account_id,character_id,package_id,coins,minor_units,currency,exponent,environment,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,clock_timestamp()+make_interval(secs=>$10))")
                .bind(quote).bind(payer).bind(character).bind(package.id).bind(package.coins)
                .bind(package.price.minor_units).bind(&package.price.currency).bind(i16::from(package.price.exponent))
                .bind(&self.settings.environment).bind(f64::from(package.valid_for_seconds))
                .execute(&mut *tx).await?;
            packages.push(CoinPackage {
                id: package.id,
                quote_id: quote,
                title: package.title.clone(),
                coins: package.coins,
                price: package.price.clone(),
                valid_for_seconds: package.valid_for_seconds,
            });
        }
        let rows = sqlx::query(concat!(
            "SELECT ",
            order_columns!(),
            " FROM coin_purchase_orders WHERE payer_account_id=$1 ORDER BY created_at DESC, id DESC LIMIT $2"
        ))
        .bind(payer)
        .bind(MAX_RECENT_ORDERS)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let recent_orders = rows
            .iter()
            .map(|row| order_row(row).map(|row| row.order))
            .collect::<Result<_, _>>()?;
        Ok(CoinCatalogue {
            version: VERSION,
            sandbox: true,
            character_id: character,
            packages,
            recent_orders,
        })
    }

    /// Reservations are not available; review comes from the payer's orders.
    pub async fn balance(&self, payer: Uuid) -> Result<CoinBalance, CoinError> {
        self.enabled()?;
        let wallet = self.host.wallet(payer).await?;
        let review_required: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM coin_purchase_orders WHERE payer_account_id=$1 AND review_required)")
            .bind(payer).fetch_one(&self.pool).await?;
        let consistent = wallet.revision > 0
            && (0..=MAX_COINS).contains(&wallet.total)
            && (0..=wallet.total).contains(&wallet.available)
            && (0..=wallet.available).contains(&wallet.transferable);
        if !consistent {
            return Err(CoinError::Unavailable);
        }
        Ok(CoinBalance {
            version: VERSION,
            revision: wallet.revision,
            total_coins: wallet.total,
            available_coins: wallet.available,
            transferable_coins: wallet.transferable,
            review_required,
        })
    }

    async fn find(&self, payer: Uuid, request: Uuid) -> Result<Option<OrderRow>, CoinError> {
        let row = sqlx::query(concat!(
            "SELECT ",
            order_columns!(),
            " FROM coin_purchase_orders WHERE payer_account_id=$1 AND request_id=$2"
        ))
        .bind(payer)
        .bind(request)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(order_row).transpose()
    }

    async fn find_order(&self, payer: Uuid, order: Uuid) -> Result<OrderRow, CoinError> {
        let row = sqlx::query(concat!(
            "SELECT ",
            order_columns!(),
            " FROM coin_purchase_orders WHERE id=$1 AND payer_account_id=$2"
        ))
        .bind(order)
        .bind(payer)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(CoinError::NotFound)?;
        order_row(&row)
    }

    /// Status authorizes against the payer only, not current character ownership.
    pub async fn order(&self, payer: Uuid, request: Uuid) -> Result<CoinOrder, CoinError> {
        self.enabled()?;
        self.find(payer, request)
            .await?
            .map(|row| row.order)
            .ok_or(CoinError::NotFound)
    }

    /// Replays first (even after quote expiry), then ownership, then one
    /// transaction that rechecks eligibility and the quote and consumes it.
    /// Provider I/O happens after commit under the reserved intent ID.
    pub async fn create_order(
        &self,
        payer: Uuid,
        request: &CoinOrderRequest,
    ) -> Result<CoinOrder, CoinError> {
        self.enabled()?;
        if let Some(existing) = self.find(payer, request.request_id).await? {
            return replay(existing, request);
        }
        if let Some(character) = request.character_id {
            self.owned(payer, character).await?;
        }
        let row = match self.insert_order(payer, request).await? {
            Created::Inserted(row) => row,
            Created::Existing(row) => return replay(row, request),
        };
        // The checkout page retries a failed binding; the order is durable already.
        self.ensure_intent(&row).await.ok();
        Ok(row.order)
    }

    async fn insert_order(
        &self,
        payer: Uuid,
        request: &CoinOrderRequest,
    ) -> Result<Created, CoinError> {
        let mut tx = self.pool.begin().await?;
        if !self.host.eligible_in(&mut tx, payer).await? {
            return Err(CoinError::Conflict);
        }
        let quote = sqlx::query("SELECT payer_account_id,character_id,package_id,coins,minor_units,currency,exponent,environment,consumed_at IS NOT NULL AS consumed,expires_at>clock_timestamp() AS live FROM coin_purchase_quotes WHERE id=$1 FOR UPDATE")
            .bind(request.quote_id).fetch_optional(&mut *tx).await?.ok_or(CoinError::Conflict)?;
        if quote.try_get::<bool, _>("consumed")? {
            tx.rollback().await?;
            // A concurrent identical create may have consumed it first.
            return match self.find(payer, request.request_id).await? {
                Some(existing) => Ok(Created::Existing(existing)),
                None => Err(CoinError::Conflict),
            };
        }
        if quote.try_get::<Uuid, _>("payer_account_id")? != payer
            || quote.try_get::<Option<Uuid>, _>("character_id")? != request.character_id
            || quote.try_get::<Uuid, _>("package_id")? != request.package_id
            || quote.try_get::<String, _>("environment")? != self.settings.environment
            || !quote.try_get::<bool, _>("live")?
        {
            return Err(CoinError::Conflict);
        }
        let id = oteryn_core::new_id();
        let state = OrderState::AwaitingPayment;
        let inserted = sqlx::query(concat!(
            "INSERT INTO coin_purchase_orders (id,payer_account_id,request_id,quote_id,package_id,character_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,checkout_url,code) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) ON CONFLICT (payer_account_id,request_id) DO NOTHING RETURNING ",
            order_columns!()
        ))
        .bind(id).bind(payer).bind(request.request_id).bind(request.quote_id).bind(request.package_id)
        .bind(request.character_id).bind(quote.try_get::<i64, _>("coins")?)
        .bind(quote.try_get::<i64, _>("minor_units")?).bind(quote.try_get::<String, _>("currency")?)
        .bind(quote.try_get::<i16, _>("exponent")?).bind(&self.settings.environment)
        .bind(oteryn_core::new_id()).bind(state.as_str()).bind(self.settings.checkout_url(id))
        .bind(state.code())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(inserted) = inserted else {
            tx.rollback().await?;
            return self
                .find(payer, request.request_id)
                .await?
                .map(Created::Existing)
                .ok_or(CoinError::Unavailable);
        };
        sqlx::query(
            "UPDATE coin_purchase_quotes SET consumed_at=clock_timestamp(), updated_at=now() WHERE id=$1",
        )
        .bind(request.quote_id)
        .execute(&mut *tx)
        .await?;
        let row = order_row(&inserted)?;
        tx.commit().await?;
        Ok(Created::Inserted(row))
    }

    async fn ensure_intent(&self, row: &OrderRow) -> Result<Intent, Error> {
        self.payments
            .ensure_intent(
                row.intent,
                row.payer,
                Amount {
                    minor_units: row.order.price.minor_units,
                    currency: row.order.price.currency.clone(),
                },
            )
            .await
    }

    /// The hosted page view. Opening it never changes payment state or credits.
    pub async fn checkout(&self, payer: Uuid, order: Uuid) -> Result<CoinOrder, CoinError> {
        self.enabled()?;
        let row = self.find_order(payer, order).await?;
        if row.order.state == OrderState::AwaitingPayment && self.ensure_intent(&row).await.is_err()
        {
            return Err(CoinError::Unavailable);
        }
        Ok(row.order)
    }

    /// Sandbox decision by the payer: signs a `TestProvider` event and applies it
    /// through the verified webhook path, which alone transitions and credits.
    pub async fn sandbox_complete(
        &self,
        payer: Uuid,
        order: Uuid,
        paid: bool,
    ) -> Result<CoinOrder, CoinError> {
        self.enabled()?;
        let row = self.find_order(payer, order).await?;
        if row.order.state != OrderState::AwaitingPayment {
            return Ok(row.order);
        }
        let intent = self
            .ensure_intent(&row)
            .await
            .map_err(|_| CoinError::Unavailable)?;
        let event = PaymentEvent {
            event_id: format!(
                "coin-checkout:{}:{}",
                intent.id,
                if paid { "paid" } else { "failed" }
            ),
            intent_id: intent.id,
            provider_reference: intent.provider_reference,
            amount: intent.amount,
            state: if paid {
                IntentState::Succeeded
            } else {
                IntentState::Failed
            },
        };
        let body = serde_json::to_vec(&event).map_err(|_| CoinError::Unavailable)?;
        let signature = self
            .provider
            .sign(&body)
            .map_err(|_| CoinError::Unavailable)?;
        let outcome = self.webhook(&body, &signature).await;
        let row = self.find_order(payer, order).await?;
        if outcome.is_err() && row.order.state == OrderState::AwaitingPayment {
            return Err(CoinError::Unavailable);
        }
        Ok(row.order)
    }

    /// Verified provider events only. In one transaction: lock intent and order,
    /// record the event once, transition both, and for `paid` write the receipt
    /// and the single transferable credit keyed by the intent ID. Refund and
    /// chargeback events are refused here: phase A has no reversal handler.
    pub async fn webhook(&self, body: &[u8], signature: &[u8]) -> Result<WebhookOutcome, Error> {
        if !self.settings.enabled {
            return Err(Error::Disabled);
        }
        if body.len() > 65536 {
            return Err(Error::InvalidInput);
        }
        self.provider.verify_webhook_signature(body, signature)?;
        let event = self.provider.parse_event(body)?;
        let provider = self.provider.name();
        let mut tx = self.pool.begin().await?;
        let intent = sqlx::query("SELECT account_id,amount,currency,state,provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR UPDATE")
            .bind(event.intent_id).bind(provider).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let order = sqlx::query(concat!(
            "SELECT ",
            order_columns!(),
            " FROM coin_purchase_orders WHERE payment_intent_id=$1 FOR UPDATE"
        ))
        .bind(event.intent_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let order = order_row(&order).map_err(|_| Error::InvalidInput)?;
        let inserted = sqlx::query("INSERT INTO payment_events (id, provider, provider_event_id, intent_id, state) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (provider,provider_event_id) DO NOTHING")
            .bind(oteryn_core::new_id()).bind(provider).bind(&event.event_id).bind(event.intent_id).bind(event.state.as_str())
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            let same: bool = sqlx::query_scalar("SELECT intent_id=$3 AND state=$4 FROM payment_events WHERE provider=$1 AND provider_event_id=$2")
                .bind(provider).bind(&event.event_id).bind(event.intent_id).bind(event.state.as_str())
                .fetch_one(&mut *tx).await?;
            if !same {
                return Err(Error::InvalidInput);
            }
            tx.commit().await?;
            return Ok(WebhookOutcome::Replay);
        }
        let amount: i64 = intent.try_get("amount")?;
        let currency: String = intent.try_get("currency")?;
        if amount != event.amount.minor_units
            || currency != event.amount.currency
            || intent.try_get::<String, _>("provider_reference")? != event.provider_reference
            || intent.try_get::<Uuid, _>("account_id")? != order.payer
            || order.order.price.minor_units != amount
            || order.order.price.currency != currency
        {
            return Err(Error::InvalidInput);
        }
        IntentState::parse(intent.try_get("state")?)?.transition(event.state)?;
        let next = match event.state {
            IntentState::Succeeded => OrderState::Paid,
            IntentState::Failed => OrderState::Failed,
            _ => return Err(Error::IllegalTransition),
        };
        if !order.order.state.may_become(next) {
            return Err(Error::IllegalTransition);
        }
        sqlx::query("UPDATE payment_intents SET state=$2, updated_at=now() WHERE id=$1")
            .bind(event.intent_id)
            .bind(event.state.as_str())
            .execute(&mut *tx)
            .await?;
        let receipt = if next == OrderState::Paid {
            let receipt = oteryn_core::new_id();
            sqlx::query("INSERT INTO invoices (id,intent_id,account_id,amount,currency) VALUES ($1,$2,$3,$4,$5)")
                .bind(receipt).bind(event.intent_id).bind(order.payer).bind(amount).bind(&currency)
                .execute(&mut *tx).await?;
            self.host
                .credit_in(&mut tx, event.intent_id, order.payer, order.order.coins)
                .await
                .map_err(|_| Error::AccountDenied)?;
            Some(receipt)
        } else {
            None
        };
        sqlx::query("UPDATE coin_purchase_orders SET state=$2, revision=revision+1, checkout_url=NULL, receipt_id=$3, code=$4, updated_at=now() WHERE id=$1")
            .bind(order.order.order_id).bind(next.as_str()).bind(receipt).bind(next.code())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(WebhookOutcome::Applied)
    }
}
