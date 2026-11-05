use crate::*;
use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

/// Each API operation commits atomically or rolls back. Adapters are required;
/// there is no permissive production default.
#[derive(Debug, Clone)]
pub struct Ledger<A, B, H> {
    pool: PgPool,
    flags: WalletFlags,
    accounts: A,
    blocks: B,
    audit: H,
}
#[derive(Debug)]
struct Operation<'a> {
    key: Uuid,
    kind: &'static str,
    account: Uuid,
    counterparty: Option<Uuid>,
    amount: i64,
    reference: Option<Uuid>,
    reason: Option<&'a str>,
}
impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    pub fn new(pool: PgPool, flags: WalletFlags, accounts: A, blocks: B, audit: H) -> Self {
        Self {
            pool,
            flags,
            accounts,
            blocks,
            audit,
        }
    }
    fn enabled(&self) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Refusal::Disabled.into());
        }
        Ok(())
    }
    async fn active(&self, connection: &mut PgConnection, account: Uuid) -> Result<(), Error> {
        if self.accounts.state(connection, account).await != AccountState::Active {
            return Err(Refusal::AccountDenied.into());
        }
        Ok(())
    }
    async fn allowed(&self, connection: &mut PgConnection, account: Uuid) -> Result<(), Error> {
        match self.blocks.permission(connection, account).await {
            TransferPermission::Allowed => Ok(()),
            TransferPermission::Blocked => Err(Refusal::TransferBlocked.into()),
            TransferPermission::Unknown => Err(Refusal::TransferStateUnknown.into()),
        }
    }
    /// Compute the entry sum while holding the account row lock.
    pub async fn balance(&self, account: Uuid) -> Result<Balance, Error> {
        self.enabled()?;
        valid_id(account)?;
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let result = balance(&mut tx, account).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn transfer(
        &self,
        key: Uuid,
        sender: Uuid,
        recipient: Uuid,
        amount: i64,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(sender)?;
        valid_id(recipient)?;
        positive(amount)?;
        if sender == recipient {
            return Err(Refusal::SameAccount.into());
        }
        let mut tx = self.pool.begin().await?;
        for account in lock_order(sender, recipient) {
            lock(&mut tx, account).await?;
        }
        self.active(&mut tx, sender).await?;
        self.active(&mut tx, recipient).await?;
        let operation = Operation {
            key,
            kind: "transfer",
            account: sender,
            counterparty: Some(recipient),
            amount,
            reference: None,
            reason: None,
        };
        if claim(&mut tx, &operation).await? {
            tx.commit().await?;
            return Ok(receipt(key, true));
        }
        self.allowed(&mut tx, sender).await?;
        self.allowed(&mut tx, recipient).await?;
        entry(&mut tx, key, sender, -amount, 0).await?;
        entry(&mut tx, key, recipient, amount, 0).await?;
        sqlx::query("INSERT INTO wallet_transfers (id, sender_id, recipient_id, amount) VALUES ($1,$2,$3,$4)")
            .bind(key).bind(sender).bind(recipient).bind(amount).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(receipt(key, false))
    }
    /// Staff callers enforce RBAC; the audit adapter verifies the actor and
    /// persists the before/after event in the same transaction.
    pub async fn adjust(
        &self,
        key: Uuid,
        account: Uuid,
        amount: i64,
        actor: Uuid,
        reason: &str,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        valid_id(actor)?;
        if amount == 0 || amount == i64::MIN {
            return Err(Refusal::InvalidAmount.into());
        }
        if reason.trim().is_empty() || reason.chars().count() > 1000 {
            return Err(Refusal::InvalidReason.into());
        }
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let operation = Operation {
            key,
            kind: "adjustment",
            account,
            counterparty: None,
            amount,
            reference: Some(actor),
            reason: Some(reason),
        };
        if claim(&mut tx, &operation).await? {
            tx.commit().await?;
            return Ok(receipt(key, true));
        }
        let before = balance(&mut tx, account).await?;
        let after = before.apply(amount, 0)?;
        let event = AdjustmentAudit {
            operation_id: key,
            account_id: account,
            actor_id: actor,
            amount,
            reason: reason.to_owned(),
            before,
            after,
        };
        if !self.audit.adjustment(&mut tx, &event).await {
            return Err(Refusal::AuditDenied.into());
        }
        entry(&mut tx, key, account, amount, 0).await?;
        sqlx::query("INSERT INTO wallet_adjustments (id, account_id, actor_id, amount, reason) VALUES ($1,$2,$3,$4,$5)")
            .bind(key).bind(account).bind(actor).bind(amount).bind(reason).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(receipt(key, false))
    }
    /// The operation key is also the reservation ID.
    pub async fn reserve(&self, key: Uuid, account: Uuid, amount: i64) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        positive(amount)?;
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let operation = Operation {
            key,
            kind: "reserve",
            account,
            counterparty: None,
            amount,
            reference: None,
            reason: None,
        };
        if claim(&mut tx, &operation).await? {
            tx.commit().await?;
            return Ok(receipt(key, true));
        }
        entry(&mut tx, key, account, -amount, amount).await?;
        sqlx::query("INSERT INTO wallet_reservations (id, account_id, amount) VALUES ($1,$2,$3)")
            .bind(key)
            .bind(account)
            .bind(amount)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(receipt(key, false))
    }
    pub async fn release(
        &self,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
    ) -> Result<Receipt, Error> {
        self.settle(key, account, reservation, false).await
    }
    /// Consume a reservation in full; capture and release are mutually exclusive.
    pub async fn capture(
        &self,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
    ) -> Result<Receipt, Error> {
        self.settle(key, account, reservation, true).await
    }
    async fn settle(
        &self,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
        capture: bool,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        valid_key(reservation)?;
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let row = sqlx::query("SELECT amount, status FROM wallet_reservations WHERE id=$1 AND account_id=$2 FOR UPDATE")
            .bind(reservation).bind(account).fetch_optional(&mut *tx).await?.ok_or(Refusal::ReservationMissing)?;
        let amount: i64 = row.try_get("amount")?;
        let status: String = row.try_get("status")?;
        let kind = if capture { "capture" } else { "release" };
        let operation = Operation {
            key,
            kind,
            account,
            counterparty: None,
            amount,
            reference: Some(reservation),
            reason: None,
        };
        if claim(&mut tx, &operation).await? {
            tx.commit().await?;
            return Ok(receipt(key, true));
        }
        if status != "active" {
            return Err(Refusal::ReservationSettled.into());
        }
        entry(
            &mut tx,
            key,
            account,
            if capture { 0 } else { amount },
            -amount,
        )
        .await?;
        sqlx::query("UPDATE wallet_reservations SET status=$2, updated_at=now() WHERE id=$1")
            .bind(reservation)
            .bind(if capture { "captured" } else { "released" })
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(receipt(key, false))
    }
    /// Viewer authorization is the caller's responsibility.
    pub async fn history(
        &self,
        account: Uuid,
        page: HistoryPage,
    ) -> Result<Vec<HistoryEntry>, Error> {
        self.enabled()?;
        valid_id(account)?;
        if !(1..=100).contains(&page.limit) {
            return Err(Refusal::InvalidPage.into());
        }
        let mut tx = self.pool.begin().await?;
        self.active(&mut tx, account).await?;
        let rows = sqlx::query("SELECT e.id, e.operation_id, o.kind, e.available_delta, e.reserved_delta, e.created_at FROM wallet_entries e JOIN wallet_operations o ON o.id=e.operation_id WHERE e.account_id=$1 AND ($2::uuid IS NULL OR e.id < $2) ORDER BY e.id DESC LIMIT $3")
            .bind(account).bind(page.before).bind(i64::from(page.limit)).fetch_all(&mut *tx).await?;
        let entries = rows
            .into_iter()
            .map(|row| {
                Ok(HistoryEntry {
                    id: row.try_get("id")?,
                    operation_id: row.try_get("operation_id")?,
                    kind: row.try_get("kind")?,
                    available_delta: row.try_get("available_delta")?,
                    reserved_delta: row.try_get("reserved_delta")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;
        tx.commit().await?;
        Ok(entries)
    }
}
fn receipt(key: Uuid, replayed: bool) -> Receipt {
    Receipt {
        operation_id: key,
        replayed,
    }
}
fn valid_id(id: Uuid) -> Result<(), Error> {
    if id.is_nil() {
        return Err(Refusal::InvalidIdentifier.into());
    }
    Ok(())
}
fn valid_key(id: Uuid) -> Result<(), Error> {
    valid_id(id)?;
    if id.get_version_num() != 7 {
        return Err(Refusal::InvalidIdentifier.into());
    }
    Ok(())
}
fn positive(amount: i64) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Refusal::InvalidAmount.into());
    }
    Ok(())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<(), Error> {
    // Creation also acquires a lock: callers order inserts as well as SELECTs.
    sqlx::query("INSERT INTO wallet_accounts (account_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account)
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}
async fn balance(tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<Balance, Error> {
    let (available, reserved): (i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(available_delta),0)::bigint, COALESCE(SUM(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1")
        .bind(account).fetch_one(&mut **tx).await?;
    Balance::default().apply(available, reserved)
}
async fn entry(
    tx: &mut Transaction<'_, Postgres>,
    key: Uuid,
    account: Uuid,
    available: i64,
    reserved: i64,
) -> Result<(), Error> {
    balance(tx, account).await?.apply(available, reserved)?;
    sqlx::query("INSERT INTO wallet_entries (id, operation_id, account_id, available_delta, reserved_delta) VALUES ($1,$2,$3,$4,$5)")
        .bind(Uuid::now_v7()).bind(key).bind(account).bind(available).bind(reserved).execute(&mut **tx).await?;
    Ok(())
}
/// Unique INSERT serializes key reuse even across unrelated accounts. Rollback
/// removes a failed claim, so the original request can safely be retried.
async fn claim(tx: &mut Transaction<'_, Postgres>, op: &Operation<'_>) -> Result<bool, Error> {
    let inserted = sqlx::query("INSERT INTO wallet_operations (id, kind, account_id, counterparty_id, amount, reference_id, reason) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
        .bind(op.key).bind(op.kind).bind(op.account).bind(op.counterparty).bind(op.amount).bind(op.reference).bind(op.reason)
        .execute(&mut **tx).await?.rows_affected();
    if inserted == 1 {
        return Ok(false);
    }
    let same: bool = sqlx::query_scalar("SELECT kind=$2 AND account_id=$3 AND counterparty_id IS NOT DISTINCT FROM $4 AND amount=$5 AND reference_id IS NOT DISTINCT FROM $6 AND reason IS NOT DISTINCT FROM $7 FROM wallet_operations WHERE id=$1")
        .bind(op.key).bind(op.kind).bind(op.account).bind(op.counterparty).bind(op.amount).bind(op.reference).bind(op.reason)
        .fetch_one(&mut **tx).await?;
    if !same {
        return Err(Refusal::IdempotencyConflict.into());
    }
    Ok(true)
}
