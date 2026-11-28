//! Test-only commerce seeds for Game's node-boot E2E (Store, coin transfer
//! quote, loyalty). Each goes through the owning crate's domain API under the
//! fixed [`SEED_ACTOR`], which is recorded in the audit rows those APIs write;
//! the actor is not an account and holds no RBAC role: the `account` seed
//! refuses its id, and every commerce seed refuses while an account holds it.
//! The `seed` role refuses all of them unless `OTERYN_ENV=testing`, and each
//! one also requires the store's [`TestingSeed`] read from that variable.
//!
//! | entity | domain call | output |
//! |---|---|---|
//! | `store-catalog` | `CatalogStore::import` + `activate` (audited) | `catalogRevision`, `offers`, `active` |
//! | `coin-credit` | `wallet::Ledger::grant` (`staff_grant`, keyed by `operationId`) | `accountId`, `operationId`, `amount` |
//! | `loyalty` | `premium::repository::Ledger::grant_consumed_for_testing` | `accountId`, `premiumDays`, `entitlementId`, `effectiveFrom`, `effectiveUntil` |
use super::{Args, SeedError, Settings, bad, bounded};
use oteryn_store::catalog::{
    self, AccountState, Action, CatalogAccess, CatalogFlags, CatalogStore,
};
use oteryn_store::premium::{self, repository::TestingSeed};
use oteryn_wallet as wallet;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use std::{io::Read, path::PathBuf, sync::Arc};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// The audited actor of every commerce seed; never an account.
pub const SEED_ACTOR: Uuid = Uuid::from_u128(0x019a0000_0000_7000_8000_000000005eed);
/// Largest catalogue file read, the same bound as the catalogue parser.
const MAX_CATALOG_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CREDIT: i64 = 1_000_000_000;
const LOYALTY_REASON: &str = "testing seed: consumed premium days";

/// Catalogue authority that admits only [`SEED_ACTOR`]. It takes the same RBAC
/// writer fence as the operator command before the store's shared fence.
#[derive(Debug)]
struct SeedCatalogAccess;
impl CatalogAccess for SeedCatalogAccess {
    async fn account_state(
        &self,
        actor: Uuid,
        connection: &mut PgConnection,
    ) -> Result<AccountState, catalog::Error> {
        oteryn_admin::rbac::lock_mutation(connection).await?;
        Ok(if actor == SEED_ACTOR {
            AccountState::Active
        } else {
            AccountState::Denied
        })
    }
    async fn permitted(
        &self,
        actor: Uuid,
        _: Action,
        _: &mut PgConnection,
    ) -> Result<bool, catalog::Error> {
        Ok(actor == SEED_ACTOR)
    }
}

/// The testing authority every commerce seed requires (fails closed).
fn testing(settings: &Settings) -> Result<TestingSeed, SeedError> {
    settings.testing_seed.ok_or(SeedError::Unavailable(
        "seed runs only with OTERYN_ENV=testing",
    ))
}

/// Audit rows under [`SEED_ACTOR`] must never be attributable to an account.
async fn actor_is_not_an_account(pool: &PgPool) -> Result<(), SeedError> {
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1)")
        .bind(SEED_ACTOR)
        .fetch_one(pool)
        .await?;
    if taken {
        return Err(SeedError::Conflict(
            "an account holds the commerce seed actor id".into(),
        ));
    }
    Ok(())
}

pub(super) async fn store_catalog(
    pool: &PgPool,
    settings: &Settings,
    args: &Args<'_>,
) -> Result<Value, SeedError> {
    testing(settings)?;
    args.only(&["file"])?;
    let path = PathBuf::from(args.required("file")?);
    let raw = tokio::task::spawn_blocking(move || read_catalog(&path))
        .await
        .map_err(|_| SeedError::Unavailable("catalogue file read"))??;
    actor_is_not_an_account(pool).await?;
    let store = CatalogStore::new(pool.clone(), CatalogFlags { enabled: true }).with_audit_log();
    let revision = store
        .import(&raw, SEED_ACTOR, &SeedCatalogAccess)
        .await
        .map_err(catalog_error)?;
    store
        .activate(&revision, SEED_ACTOR, &SeedCatalogAccess)
        .await
        .map_err(catalog_error)?;
    let offers = catalog::Catalog::parse(&raw)
        .map_err(catalog_error)?
        .offers()
        .len();
    Ok(json!({
        "entity": "store-catalog",
        "catalogRevision": revision,
        "offers": offers,
        "active": true,
    }))
}

/// Reads a regular file only. On Unix the open is non-blocking, so a FIFO
/// swapped in after the path check cannot stall it, and the opened descriptor
/// itself must be a regular file before anything is read.
fn read_catalog(path: &std::path::Path) -> Result<Vec<u8>, SeedError> {
    let unreadable = || bad("file is not a readable catalogue file");
    if !std::fs::metadata(path).is_ok_and(|meta| meta.is_file()) {
        return Err(unreadable());
    }
    let file = open_nonblocking(path).map_err(|_| unreadable())?;
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return Err(unreadable());
    }
    let mut raw = Vec::new();
    file.take(MAX_CATALOG_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| unreadable())?;
    if raw.len() as u64 > MAX_CATALOG_BYTES {
        return Err(bad("file exceeds 16 MiB"));
    }
    Ok(raw)
}

#[cfg(unix)]
pub(super) fn open_nonblocking(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
    Ok(rustix::fs::open(path, flags, Mode::empty())?.into())
}

#[cfg(not(unix))]
pub(super) fn open_nonblocking(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

fn catalog_error(error: catalog::Error) -> SeedError {
    match error {
        catalog::Error::InvalidDocument | catalog::Error::InvalidRevision => {
            bad("file is not a valid store price list")
        }
        catalog::Error::Conflict => {
            SeedError::Conflict("catalog revision was imported with other bytes".into())
        }
        catalog::Error::NotImported => SeedError::Unavailable("catalog import"),
        catalog::Error::Denied => SeedError::Unavailable("catalog authority"),
        catalog::Error::Disabled => SeedError::Unavailable("catalog disabled"),
        catalog::Error::Database(_) => SeedError::Unavailable("database"),
    }
}

/// Wallet adapters for the seed grant: the account must pass the shared value
/// eligibility gate, and only [`SEED_ACTOR`] grants are audited and accepted.
#[derive(Debug, Clone, Copy)]
struct SeedWallet;
impl wallet::AccountStateReader for SeedWallet {
    async fn state(&self, connection: &mut PgConnection, account: Uuid) -> wallet::AccountState {
        match wallet::value_eligible_in(connection, account).await {
            Ok(true) => wallet::AccountState::Active,
            Ok(false) => wallet::AccountState::Unknown,
            // A failed read is not an ineligible account.
            Err(_) => wallet::AccountState::Unavailable,
        }
    }
}
impl wallet::TransferBlockCheck for SeedWallet {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for SeedWallet {
    async fn adjustment(
        &self,
        connection: &mut PgConnection,
        event: &wallet::AdjustmentAudit,
    ) -> bool {
        if event.actor_id != SEED_ACTOR
            || event.amount <= 0
            || event.reason != credit_reason(event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'seed.wallet.credited',$2,$3,$4,$5)")
            .bind(SEED_ACTOR)
            .bind(event.account_id.to_string())
            .bind(json!({"available":event.before.available,"reserved":event.before.reserved}))
            .bind(json!({"available":event.after.available,"reserved":event.after.reserved,"amount":event.amount}))
            .bind(event.operation_id)
            .execute(connection)
            .await
            .is_ok()
    }
}
fn credit_reason(operation: Uuid) -> String {
    format!("testing-seed:{operation}")
}

pub(super) async fn coin_credit(
    pool: &PgPool,
    settings: &Settings,
    args: &Args<'_>,
) -> Result<Value, SeedError> {
    testing(settings)?;
    args.only(&["accountId", "email", "amount", "operationId"])?;
    let operation = args
        .uuid("operationId", true)?
        .ok_or_else(|| bad("operationId is required"))?;
    let amount = integer(args, "amount", 1, MAX_CREDIT)?;
    let account = account_ref(pool, args).await?;
    actor_is_not_an_account(pool).await?;
    let ledger = wallet::Ledger::new(
        pool.clone(),
        wallet::WalletFlags { enabled: true },
        SeedWallet,
        SeedWallet,
        SeedWallet,
    );
    let receipt = ledger
        .grant(
            operation,
            account,
            amount,
            SEED_ACTOR,
            &credit_reason(operation),
            wallet::GrantKind::StaffGrant,
        )
        .await
        .map_err(|error| match error {
            wallet::Error::Refused(wallet::Refusal::IdempotencyConflict) => {
                SeedError::Conflict("operationId was used with other input".into())
            }
            wallet::Error::Refused(wallet::Refusal::AccountDenied) => {
                bad("account is not eligible for value")
            }
            wallet::Error::Refused(wallet::Refusal::TransferStateUnknown) => {
                SeedError::Unavailable("account eligibility")
            }
            wallet::Error::Refused(wallet::Refusal::Overflow) => {
                bad("amount exceeds the balance ceiling")
            }
            wallet::Error::Refused(_) => SeedError::Unavailable("wallet refused the credit"),
            wallet::Error::Storage(_) => SeedError::Unavailable("database"),
        })?;
    Ok(json!({
        "entity": "coin-credit",
        "accountId": receipt.account_id.to_string(),
        "operationId": receipt.operation_id.to_string(),
        "amount": amount,
    }))
}

pub(super) async fn loyalty(
    pool: &PgPool,
    settings: &Settings,
    args: &Args<'_>,
) -> Result<Value, SeedError> {
    let testing = testing(settings)?;
    args.only(&["accountId", "email", "premiumDays"])?;
    let days = integer(args, "premiumDays", 1, 366)?;
    let days = u16::try_from(days).map_err(|_| bad("premiumDays must be 1..=366"))?;
    let account = account_ref(pool, args).await?;
    actor_is_not_an_account(pool).await?;
    let ledger = premium::repository::Ledger::new(
        pool.clone(),
        Arc::new(oteryn_store::loyalty::stream::http::DatabaseAccounts),
        true,
    );
    let entitlement = ledger
        .grant_consumed_for_testing(testing, SEED_ACTOR, account, days, LOYALTY_REASON)
        .await
        .map_err(|error| match error {
            premium::Error::Conflict => SeedError::Conflict(
                "account already has Premium history other than this seed".into(),
            ),
            premium::Error::Invalid => bad("premiumDays must be 1..=366"),
            premium::Error::UnknownAccount => bad("account is not seeded"),
            _ => SeedError::Unavailable("premium ledger"),
        })?;
    Ok(json!({
        "entity": "loyalty",
        "accountId": account.to_string(),
        "premiumDays": days,
        "entitlementId": entitlement.id.to_string(),
        "effectiveFrom": rfc3339(entitlement.from)?,
        "effectiveUntil": rfc3339(entitlement.until)?,
    }))
}

fn rfc3339(seconds: i64) -> Result<String, SeedError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|time| time.format(&Rfc3339).ok())
        .ok_or(SeedError::Unavailable("premium ledger"))
}

fn integer(args: &Args<'_>, key: &str, min: i64, max: i64) -> Result<i64, SeedError> {
    match args.0.get(key).and_then(Value::as_i64) {
        Some(value) if (min..=max).contains(&value) => Ok(value),
        _ => Err(bad(format!("{key} must be an integer {min}..={max}"))),
    }
}

/// The seeded account named by exactly one of `accountId` (UUIDv7) or `email`.
async fn account_ref(pool: &PgPool, args: &Args<'_>) -> Result<Uuid, SeedError> {
    let id = args.uuid("accountId", true)?;
    let email = args.text("email")?;
    let found: Option<Uuid> = match (id, email) {
        (Some(id), None) => {
            sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1")
                .bind(id)
                .fetch_optional(pool)
                .await?
        }
        (None, Some(email)) => {
            bounded("email", email, 254)?;
            sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1")
                .bind(email.to_ascii_lowercase())
                .fetch_optional(pool)
                .await?
        }
        _ => return Err(bad("exactly one of accountId or email is required")),
    };
    found.ok_or_else(|| bad("account is not seeded"))
}
