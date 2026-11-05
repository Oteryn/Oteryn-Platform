//! Binary composition for the T31 adapter. Kept outside the store crate module
//! graph because payments already uses store for account premium projections.
use std::{future::Future, pin::Pin, sync::Arc};

use oteryn_payments::{AccountReader, AccountState, Amount, PaymentFlags, Payments, TestProvider};
use oteryn_store::web::{Error, Package, TopUps};
use sqlx::{PgPool, Row};
use uuid::Uuid;

struct Accounts(PgPool);
impl AccountReader for Accounts {
    async fn state(&self, id: Uuid) -> Result<AccountState, oteryn_payments::Error> {
        let account = oteryn_identity::accounts::Accounts {
            pool: self.0.clone(),
        }
        .find_by_id(id)
        .await?;
        Ok(match account {
            Some(a) if a.status == "active" && a.email_verified_at.is_some() => {
                AccountState::Active
            }
            Some(_) => AccountState::Restricted,
            None => AccountState::Unknown,
        })
    }
}

struct PaymentTopUps {
    pool: PgPool,
    payments: Payments<TestProvider, Accounts>,
    packages: Vec<Package>,
}
impl TopUps for PaymentTopUps {
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
            if operation.get_version_num() != 7 {
                return Err(Error::Invalid);
            }
            let p = self
                .packages
                .iter()
                .find(|p| p.key == key)
                .ok_or(Error::Invalid)?;
            // Commit the claim before an external call. A crash/timeout never
            // releases it for automatic retry: unknown payment state denies.
            let claimed = sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(operation_id) DO NOTHING")
                .bind(operation).bind(account).bind(&p.key).bind(p.coins).bind(p.minor_units).bind(&p.currency).execute(&self.pool).await.map_err(|_| Error::Unavailable)?.rows_affected() == 1;
            if !claimed {
                let row = sqlx::query("SELECT account_id,package_key,coins,minor_units,currency,payment_intent_id FROM store_web_topups WHERE operation_id=$1").bind(operation).fetch_one(&self.pool).await.map_err(|_| Error::Unavailable)?;
                let matches = row.try_get::<Uuid, _>("account_id").ok() == Some(account)
                    && row.try_get::<String, _>("package_key").ok().as_deref() == Some(&p.key)
                    && row.try_get::<i64, _>("coins").ok() == Some(p.coins)
                    && row.try_get::<i64, _>("minor_units").ok() == Some(p.minor_units)
                    && row.try_get::<String, _>("currency").ok().as_deref() == Some(&p.currency);
                if !matches {
                    return Err(Error::Conflict);
                }
                return row
                    .try_get::<Option<Uuid>, _>("payment_intent_id")
                    .map_err(|_| Error::Unavailable)?
                    .ok_or(Error::Unavailable);
            }
            let intent = self
                .payments
                .create_intent(
                    account,
                    Amount {
                        minor_units: p.minor_units,
                        currency: p.currency.clone(),
                    },
                )
                .await
                .map_err(|_| Error::Unavailable)?;
            let updated = sqlx::query("UPDATE store_web_topups SET payment_intent_id=$2, updated_at=now() WHERE operation_id=$1 AND payment_intent_id IS NULL").bind(operation).bind(intent.id).execute(&self.pool).await.map_err(|_| Error::Unavailable)?;
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
pub(super) fn router(pool: PgPool) -> axum::Router {
    // T31 currently supplies a local test provider only. It is never selected in
    // production, and no payment or wallet effect is inferred from a return URL.
    let testing = std::env::var("OTERYN_ENV").as_deref() == Ok("testing");
    let enabled = std::env::var("OTERYN_SHOP_TEST_PAYMENTS_ENABLED").as_deref() == Ok("true");
    let topups = if testing && enabled {
        let packages = std::env::var("OTERYN_SHOP_TOP_UP_PACKAGES")
            .ok()
            .and_then(|raw| serde_json::from_str::<Vec<Package>>(&raw).ok());
        packages.filter(|p| valid_packages(p)).and_then(|packages| {
            let mut key = [0; 32];
            getrandom::fill(&mut key).ok()?;
            let provider = TestProvider::new(&key).ok()?;
            Some(Arc::new(PaymentTopUps {
                payments: Payments::new(
                    pool.clone(),
                    provider,
                    Accounts(pool.clone()),
                    PaymentFlags { enabled: true },
                ),
                pool: pool.clone(),
                packages,
            }) as Arc<dyn TopUps>)
        })
    } else {
        None
    };
    oteryn_store::web::configured_router_with_topups(pool, topups)
}

#[cfg(test)]
#[path = "web_payments_tests.rs"]
mod tests;
