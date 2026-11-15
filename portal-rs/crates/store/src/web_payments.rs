//! Binary composition for the T31 adapter. Kept outside the store crate module
//! graph because payments already uses store for account premium projections.
use std::{future::Future, pin::Pin, sync::Arc};

use oteryn_payments::{AccountReader, AccountState, Amount, PaymentFlags, Payments, TestProvider};
use oteryn_store::web::{Error, Package, TopUps};
use sqlx::{PgPool, Row};
use uuid::Uuid;

struct Accounts(PgPool);
impl AccountReader for Accounts {
    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<AccountState, oteryn_payments::Error> {
        Ok(if oteryn_wallet::value_eligible_in(connection, id).await? {
            AccountState::Active
        } else {
            AccountState::Restricted
        })
    }
    async fn refund_state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<AccountState, oteryn_payments::Error> {
        // Retain the browser refund identity rules independently of restrictions
        // on acquiring new value. Delivered checkouts are fenced by Payments.
        let active: Option<bool> = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE")
            .bind(id).fetch_optional(connection).await?;
        Ok(match active {
            Some(true) => AccountState::Active,
            Some(false) => AccountState::Restricted,
            None => AccountState::Unknown,
        })
    }

    async fn state(&self, id: Uuid) -> Result<AccountState, oteryn_payments::Error> {
        let mut tx = self.0.begin().await?;
        let allowed = oteryn_wallet::value_eligible_in(&mut tx, id).await?;
        tx.commit().await?;
        Ok(if allowed {
            AccountState::Active
        } else {
            AccountState::Restricted
        })
    }
}

struct PaymentTopUps {
    pool: PgPool,
    payments: Payments<TestProvider, Accounts>,
    packages: Vec<Package>,
    signer: TestProvider,
}
impl TopUps for PaymentTopUps {
    fn test_checkout(&self) -> bool {
        true
    }
    fn checkout(
        &self,
        account: Uuid,
        id: Uuid,
        action: Option<String>,
    ) -> oteryn_store::web::FutureResult<'_, Option<oteryn_store::web::Checkout>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT package_key,coins,minor_units,currency FROM store_web_topups WHERE payment_intent_id=$1 AND account_id=$2")
                .bind(id).bind(account).fetch_optional(&self.pool).await.map_err(|_| Error::Unavailable)?;
            let Some(row) = row else {
                return Ok(None);
            };
            let mut intent = self
                .payments
                .intent(id)
                .await
                .map_err(|_| Error::Unavailable)?;
            if intent.account_id != account {
                return Ok(None);
            }
            use oteryn_payments::{IntentState, PaymentEvent};
            if let Some(action) = action {
                let state = match action.as_str() {
                    "pay" => IntentState::Succeeded,
                    "reject" | "cancel" => IntentState::Failed,
                    _ => return Err(Error::Invalid),
                };
                if matches!(intent.state, IntentState::Created | IntentState::Pending) {
                    // Stable event IDs deduplicate identical decisions; the payment
                    // row lock serializes competing terminal transitions.
                    let event = PaymentEvent {
                        event_id: format!(
                            "checkout:{id}:{}",
                            if state == IntentState::Succeeded {
                                "paid"
                            } else {
                                "failed"
                            }
                        ),
                        intent_id: id,
                        provider_reference: intent.provider_reference.clone(),
                        amount: intent.amount.clone(),
                        state,
                    };
                    let body = serde_json::to_vec(&event).map_err(|_| Error::Unavailable)?;
                    let signature = self.signer.sign(&body).map_err(|_| Error::Unavailable)?;
                    let outcome = self.payments.webhook(&body, &signature).await;
                    intent = self
                        .payments
                        .intent(id)
                        .await
                        .map_err(|_| Error::Unavailable)?;
                    if outcome.is_err()
                        && matches!(intent.state, IntentState::Created | IntentState::Pending)
                    {
                        return Err(Error::Unavailable);
                    }
                }
            }
            let credited = if intent.state == IntentState::Succeeded {
                sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM wallet_operations WHERE id=$1 AND account_id=$2 AND kind='top_up')")
                    .bind(id).bind(account).fetch_one(&self.pool).await.map_err(|_| Error::Unavailable)?
            } else {
                false
            };
            Ok(Some(oteryn_store::web::Checkout {
                package: Package {
                    key: row.try_get("package_key").map_err(|_| Error::Unavailable)?,
                    coins: row.try_get("coins").map_err(|_| Error::Unavailable)?,
                    minor_units: row.try_get("minor_units").map_err(|_| Error::Unavailable)?,
                    currency: row.try_get("currency").map_err(|_| Error::Unavailable)?,
                },
                pending: matches!(intent.state, IntentState::Created | IntentState::Pending),
                status: match intent.state {
                    IntentState::Created | IntentState::Pending => "checkout-pending",
                    IntentState::Succeeded if credited => "checkout-credited",
                    IntentState::Succeeded => "checkout-succeeded",
                    IntentState::Failed => "checkout-failed",
                    IntentState::Refunded => "checkout-refunded",
                }
                .into(),
            }))
        })
    }

    fn packages(&self) -> Vec<Package> {
        self.packages.clone()
    }
    fn create(
        &self,
        account: Uuid,
        operation: Uuid,
        key: String,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, Error>> + Send + '_>> {
        Box::pin(async move {
            if operation.get_version_num() != 7 || operation.get_variant() != uuid::Variant::RFC4122
            {
                return Err(Error::Invalid);
            }
            // Resolve durable browser operations before consulting mutable packages.
            let prior = sqlx::query(
                "SELECT account_id,package_key FROM store_web_topups WHERE operation_id=$1",
            )
            .bind(operation)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| Error::Unavailable)?;
            if prior.is_none() {
                let p = self
                    .packages
                    .iter()
                    .find(|p| p.key == key)
                    .ok_or(Error::Invalid)?;
                // Reserve the provider key atomically with the commercial terms.
                // Never delete uncertain claims: ensure_intent resumes this key.
                sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,intended_payment_intent_id) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(operation_id) DO NOTHING")
                    .bind(operation).bind(account).bind(&p.key).bind(p.coins).bind(p.minor_units).bind(&p.currency).bind(Uuid::now_v7()).execute(&self.pool).await.map_err(|_| Error::Unavailable)?;
            }
            let row = sqlx::query("SELECT account_id,package_key,minor_units,currency,payment_intent_id,intended_payment_intent_id FROM store_web_topups WHERE operation_id=$1")
                .bind(operation).fetch_one(&self.pool).await.map_err(|_| Error::Unavailable)?;
            if row
                .try_get::<Uuid, _>("account_id")
                .map_err(|_| Error::Unavailable)?
                != account
                || row
                    .try_get::<String, _>("package_key")
                    .map_err(|_| Error::Unavailable)?
                    != key
            {
                return Err(Error::Conflict);
            }
            if let Some(id) = row
                .try_get::<Option<Uuid>, _>("payment_intent_id")
                .map_err(|_| Error::Unavailable)?
            {
                return Ok(id);
            }
            // Legacy unlinked claims have no known provider key. Preserve them.
            let id = row
                .try_get::<Option<Uuid>, _>("intended_payment_intent_id")
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::Unavailable)?;
            let intent = self
                .payments
                .ensure_intent(
                    id,
                    account,
                    Amount {
                        minor_units: row.try_get("minor_units").map_err(|_| Error::Unavailable)?,
                        currency: row.try_get("currency").map_err(|_| Error::Unavailable)?,
                    },
                )
                .await
                .map_err(|_| Error::Unavailable)?;
            let updated = sqlx::query("UPDATE store_web_topups SET payment_intent_id=$2, updated_at=now() WHERE operation_id=$1 AND (payment_intent_id IS NULL OR payment_intent_id=$2)")
                .bind(operation).bind(intent.id).execute(&self.pool).await.map_err(|_| Error::Unavailable)?;
            if updated.rows_affected() != 1 {
                return Err(Error::Unavailable);
            }
            Ok(intent.id)
        })
    }
}
fn valid_packages(packages: &[Package]) -> bool {
    !packages.is_empty()
        && packages.len() <= 20
        && packages.iter().enumerate().all(|(i, p)| {
            p.coins > 0
                && p.minor_units > 0
                && (1..=64).contains(&p.key.len())
                && p.key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                && p.currency.len() == 3
                && p.currency.bytes().all(|b| b.is_ascii_uppercase())
                && !packages[..i].iter().any(|old| old.key == p.key)
        })
}
fn packages_for_creation(raw: Option<&str>) -> Vec<Package> {
    raw.and_then(|raw| serde_json::from_str::<Vec<Package>>(raw).ok())
        .filter(|packages| valid_packages(packages))
        .unwrap_or_default()
}
pub(super) fn router(pool: PgPool) -> axum::Router {
    // T31 currently supplies a local test provider only. It is never selected in
    // production, and no payment or wallet effect is inferred from a return URL.
    let testing = std::env::var("OTERYN_ENV").as_deref() == Ok("testing");
    let enabled = std::env::var("OTERYN_SHOP_TEST_PAYMENTS_ENABLED").as_deref() == Ok("true");
    router_for(pool, testing, enabled)
}
fn router_for(pool: PgPool, testing: bool, enabled: bool) -> axum::Router {
    let topups = if testing && enabled {
        // Missing/removed/invalid packages disable new creation, while durable
        // operations can still resume using their persisted commercial terms.
        let raw = std::env::var("OTERYN_SHOP_TOP_UP_PACKAGES").ok();
        let packages = packages_for_creation(raw.as_deref());
        std::env::var("OTERYN_SHOP_TEST_PAYMENT_KEY")
            .ok()
            .and_then(|key| {
                let provider = TestProvider::new(key.as_bytes()).ok()?;
                Some(Arc::new(PaymentTopUps {
                    signer: provider.clone(),
                    payments: Payments::new(
                        pool.clone(),
                        provider,
                        Accounts(pool.clone()),
                        PaymentFlags {
                            enabled: std::env::var("OTERYN_PAYMENTS_ENABLED").as_deref()
                                == Ok("true"),
                        },
                    ),
                    pool: pool.clone(),
                    packages,
                }) as Arc<dyn TopUps>)
            })
    } else {
        None
    };
    let mut router = oteryn_store::web::configured_router_with_topups(pool.clone(), topups);
    // An explicit ephemeral test key enables signed local-provider ingress only
    // in testing. Production has no route, key or real-provider integration.
    if testing
        && enabled
        && let Ok(key) = std::env::var("OTERYN_SHOP_TEST_PAYMENT_KEY")
        && let Ok(provider) = TestProvider::new(key.as_bytes())
    {
        let payments = Arc::new(Payments::new(
            pool.clone(),
            provider,
            Accounts(pool),
            PaymentFlags {
                enabled: std::env::var("OTERYN_PAYMENTS_ENABLED").as_deref() == Ok("true"),
            },
        ));
        router = router.merge(
            axum::Router::new()
                .route(
                    "/__test/payments/webhook",
                    axum::routing::post(test_webhook),
                )
                .layer(axum::extract::DefaultBodyLimit::max(65536))
                .with_state(payments),
        );
    }
    router
}

async fn test_webhook(
    axum::extract::State(payments): axum::extract::State<Arc<Payments<TestProvider, Accounts>>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::http::StatusCode {
    use base64::Engine;
    let signature = headers
        .get("x-test-payment-signature")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok());
    let Some(signature) = signature else {
        return axum::http::StatusCode::BAD_REQUEST;
    };
    match payments.webhook(&body, &signature).await {
        Ok(_) => axum::http::StatusCode::NO_CONTENT,
        Err(_) => axum::http::StatusCode::BAD_REQUEST,
    }
}

#[cfg(test)]
#[path = "web_payments_tests.rs"]
mod tests;
