use crate::*;
use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

/// Each API operation commits atomically or rolls back. Adapters are required;
/// there is no permissive production default.
#[derive(Debug, Clone)]
pub struct Ledger<A, B, H> {
    pub(crate) pool: PgPool,
    flags: WalletFlags,
    accounts: A,
    blocks: B,
    audit: H,
    adjustment_kind: AdjustmentKind,
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
            adjustment_kind: AdjustmentKind::Operator,
        }
    }
    fn enabled(&self) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Refusal::Disabled.into());
        }
        Ok(())
    }
    async fn active(&self, connection: &mut PgConnection, account: Uuid) -> Result<(), Error> {
        match self.accounts.state(connection, account).await {
            AccountState::Active => Ok(()),
            AccountState::Inactive => Err(Refusal::AccountDenied.into()),
            AccountState::Unknown => Err(Refusal::AccountDenied.into()),
            AccountState::Unavailable => Err(Refusal::TransferStateUnknown.into()),
        }
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
            let result = receipt(&mut tx, key, true).await?;
            tx.commit().await?;
            return Ok(result);
        }
        self.allowed(&mut tx, sender).await?;
        self.allowed(&mut tx, recipient).await?;
        crate::debt::unlocked(&mut tx, sender).await?;
        entry(&mut tx, key, sender, -amount, 0).await?;
        credit(&mut tx, key, recipient, amount).await?;
        sqlx::query("INSERT INTO wallet_transfers (id, sender_id, recipient_id, amount) VALUES ($1,$2,$3,$4)")
            .bind(key).bind(sender).bind(recipient).bind(amount).execute(&mut *tx).await?;
        let result = receipt(&mut tx, key, false).await?;
        tx.commit().await?;
        Ok(result)
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
            let result = receipt(&mut tx, key, true).await?;
            tx.commit().await?;
            return Ok(result);
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
        credit(&mut tx, key, account, amount).await?;
        sqlx::query("INSERT INTO wallet_adjustments (id, account_id, actor_id, amount, reason) VALUES ($1,$2,$3,$4,$5)")
            .bind(key).bind(account).bind(actor).bind(amount).bind(reason).execute(&mut *tx).await?;
        let result = receipt(&mut tx, key, false).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Uses the caller transaction; the caller must roll back on any error.
    #[allow(clippy::too_many_arguments)]
    pub async fn adjust_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
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
        if self.adjustment_kind == AdjustmentKind::StorePurchase
            && (amount >= 0 || actor != account || reason != format!("store-purchase:{key}"))
        {
            return Err(Refusal::InvalidReason.into());
        }
        if self.adjustment_kind == AdjustmentKind::TopUp
            && (amount <= 0 || actor != account || reason != format!("wallet-top-up:{key}"))
        {
            return Err(Refusal::InvalidReason.into());
        }
        lock(tx, account).await?;
        self.active(tx, account).await?;
        let operation = Operation {
            key,
            kind: self.adjustment_kind.name(),
            account,
            counterparty: None,
            amount,
            reference: Some(actor),
            reason: Some(reason),
        };
        if claim(tx, &operation).await? {
            return receipt(tx, key, true).await;
        }
        let before = balance(tx, account).await?;
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
        if !self.audit.adjustment(tx, &event).await {
            return Err(Refusal::AuditDenied.into());
        }
        credit(tx, key, account, amount).await?;
        sqlx::query("INSERT INTO wallet_adjustments (id, account_id, actor_id, amount, reason) VALUES ($1,$2,$3,$4,$5)")
            .bind(key).bind(account).bind(actor).bind(amount).bind(reason).execute(&mut **tx).await?;
        receipt(tx, key, false).await
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
            let result = receipt(&mut tx, key, true).await?;
            tx.commit().await?;
            return Ok(result);
        }
        entry(&mut tx, key, account, -amount, amount).await?;
        sqlx::query("INSERT INTO wallet_reservations (id, account_id, amount) VALUES ($1,$2,$3)")
            .bind(key)
            .bind(account)
            .bind(amount)
            .execute(&mut *tx)
            .await?;
        let result = receipt(&mut tx, key, false).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn release(
        &self,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
    ) -> Result<Receipt, Error> {
        self.finish_reservation(key, account, reservation, false)
            .await
    }
    /// Consume a reservation in full; capture and release are mutually exclusive.
    pub async fn capture(
        &self,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
    ) -> Result<Receipt, Error> {
        self.finish_reservation(key, account, reservation, true)
            .await
    }
    async fn finish_reservation(
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
        let row = sqlx::query("SELECT amount-COALESCE((to_jsonb(wallet_reservations)->>'consumed')::bigint,0) AS amount, status FROM wallet_reservations WHERE id=$1 AND account_id=$2 FOR UPDATE")
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
            let result = receipt(&mut tx, key, true).await?;
            tx.commit().await?;
            return Ok(result);
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
        let result = receipt(&mut tx, key, false).await?;
        tx.commit().await?;
        Ok(result)
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
        let rows = sqlx::query("SELECT e.id, e.operation_id, o.kind, CASE WHEN e.account_id=o.account_id THEN o.counterparty_id ELSE o.account_id END AS counterparty, e.available_delta, e.reserved_delta, e.created_at FROM wallet_entries e JOIN wallet_operations o ON o.id=e.operation_id WHERE e.account_id=$1 AND ($2::uuid IS NULL OR e.id < $2) ORDER BY e.id DESC LIMIT $3")
            .bind(account).bind(page.before).bind(i64::from(page.limit)).fetch_all(&mut *tx).await?;
        let entries = rows
            .into_iter()
            .map(|row| {
                Ok(HistoryEntry {
                    counterparty_account_id: row.try_get("counterparty")?,
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
async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    key: Uuid,
    replayed: bool,
) -> Result<Receipt, Error> {
    let row = sqlx::query("SELECT o.kind,o.account_id,o.counterparty_id,o.amount,o.created_at,MAX((to_jsonb(e)->>'entry_sequence')::bigint) AS sequence FROM wallet_operations o JOIN wallet_entries e ON e.account_id=o.account_id AND (e.operation_id=o.id OR e.operation_id IN (SELECT d.id FROM wallet_operations d WHERE d.kind='debt_settlement' AND d.reference_id=o.id)) WHERE o.id=$1 GROUP BY o.id")
        .bind(key).fetch_optional(&mut **tx).await?.ok_or(Refusal::ReservationMissing)?;
    let account: Uuid = row.try_get("account_id")?;
    let sequence: Option<i64> = row.try_get("sequence")?;
    // Entries of either historical coin_class count toward the one balance.
    let (available, reserved): (i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(available_delta),0)::bigint,COALESCE(SUM(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1 AND (($2::bigint IS NOT NULL AND (to_jsonb(wallet_entries)->>'entry_sequence')::bigint<=$2) OR ($2::bigint IS NULL AND id <= (SELECT id FROM wallet_entries WHERE operation_id=$3 AND account_id=$1 ORDER BY id DESC LIMIT 1)))")
        .bind(account).bind(sequence).bind(key).fetch_one(&mut **tx).await?;
    Ok(Receipt {
        operation_id: key,
        replayed,
        kind: row.try_get("kind")?,
        account_id: account,
        counterparty_account_id: row.try_get("counterparty_id")?,
        amount: row.try_get("amount")?,
        available_after: available,
        reserved_after: reserved,
        occurred_at: row.try_get("created_at")?,
    })
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
async fn balance(tx: &mut PgConnection, account: Uuid) -> Result<Balance, Error> {
    // One coin class: entries of either historical coin_class count.
    let (available, reserved): (i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(available_delta),0)::bigint, COALESCE(SUM(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1")
        .bind(account).fetch_one(&mut *tx).await?;
    Balance::default().apply(available, reserved)
}
/// Every coin credit: the whole credit enters the balance, then pays the
/// account's outstanding coin debt first, in the same transaction under the
/// account lock the caller holds (owner decision 2026-11-14). A negative
/// adjustment is a debit and settles nothing.
pub(crate) async fn credit(
    tx: &mut PgConnection,
    key: Uuid,
    account: Uuid,
    amount: i64,
) -> Result<(), Error> {
    entry(tx, key, account, amount, 0).await?;
    if amount > 0 {
        crate::debt::settle(tx, key, account, amount).await?;
    }
    Ok(())
}
pub(crate) async fn entry(
    tx: &mut PgConnection,
    key: Uuid,
    account: Uuid,
    available: i64,
    reserved: i64,
) -> Result<(), Error> {
    balance(tx, account).await?.apply(available, reserved)?;
    sqlx::query("INSERT INTO wallet_entries (id,operation_id,account_id,available_delta,reserved_delta) VALUES ($1,$2,$3,$4,$5)")
        .bind(Uuid::now_v7()).bind(key).bind(account).bind(available).bind(reserved).execute(&mut *tx).await?;
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

impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    /// Opt in only the caller-owned adjustment transaction to a purchase debit.
    /// Existing adjustment, transfer and reservation APIs keep their semantics.
    pub fn with_adjustment_kind(mut self, kind: AdjustmentKind) -> Self {
        self.adjustment_kind = kind;
        self
    }

    /// Read without creating a wallet row. The caller owns authorization and
    /// the transaction; account authority still has to be known.
    pub async fn read_balance_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> Result<Balance, Error> {
        self.enabled()?;
        valid_id(account)?;
        self.known(tx, account).await?;
        balance(tx, account).await
    }

    /// Balance and wallet revision read by one statement, without creating a
    /// wallet row. The revision is 1 plus the count of the account's immutable
    /// entries, so it is positive and every committed ledger effect raises it.
    pub async fn snapshot_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> Result<(Balance, i64), Error> {
        self.enabled()?;
        valid_id(account)?;
        self.known(tx, account).await?;
        let (available, reserved, entries): (i64, i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(available_delta),0)::bigint, COALESCE(SUM(reserved_delta),0)::bigint, COUNT(*)::bigint FROM wallet_entries WHERE account_id=$1")
            .bind(account).fetch_one(&mut **tx).await?;
        let balance = Balance::default().apply(available, reserved)?;
        Ok((balance, entries.checked_add(1).ok_or(Refusal::Overflow)?))
    }

    /// Numeric wire cursor, independent of the existing UUID history API.
    /// Fetch one extra entry to determine whether an older page exists.
    pub async fn numeric_history_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        page: NumericHistoryPage,
    ) -> Result<Vec<NumericHistoryEntry>, Error> {
        self.enabled()?;
        valid_id(account)?;
        if !(1..=32).contains(&page.limit) {
            return Err(Refusal::InvalidPage.into());
        }
        self.known(tx, account).await?;
        let before = if page.before == 0 {
            None
        } else {
            i64::try_from(page.before).ok()
        };
        let rows = sqlx::query("SELECT e.entry_sequence,e.operation_id,o.kind,e.available_delta,e.reserved_delta,e.created_at FROM wallet_entries e JOIN wallet_operations o ON o.id=e.operation_id WHERE e.account_id=$1 AND ($2::bigint IS NULL OR e.entry_sequence<$2) ORDER BY e.entry_sequence DESC LIMIT $3")
            .bind(account).bind(before).bind(i64::from(page.limit)+1).fetch_all(&mut **tx).await?;
        rows.into_iter()
            .map(|row| {
                Ok(NumericHistoryEntry {
                    sequence: row.try_get("entry_sequence")?,
                    operation_id: row.try_get("operation_id")?,
                    kind: row.try_get("kind")?,
                    available_delta: row.try_get("available_delta")?,
                    reserved_delta: row.try_get("reserved_delta")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}

impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    async fn known(&self, connection: &mut PgConnection, account: Uuid) -> Result<(), Error> {
        if !matches!(
            self.accounts.state(connection, account).await,
            AccountState::Active | AccountState::Inactive
        ) {
            return Err(Refusal::AccountDenied.into());
        }
        Ok(())
    }
}

impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    /// Reserve in the caller transaction. Roll back the entire transaction on error.
    pub async fn reserve_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key: Uuid,
        account: Uuid,
        amount: i64,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        positive(amount)?;
        lock(tx, account).await?;
        self.active(tx, account).await?;
        let operation = Operation {
            key,
            kind: "reserve",
            account,
            counterparty: None,
            amount,
            reference: None,
            reason: None,
        };
        if claim(tx, &operation).await? {
            return receipt(tx, key, true).await;
        }
        entry(tx, key, account, -amount, amount).await?;
        sqlx::query("INSERT INTO wallet_reservations (id, account_id, amount) VALUES ($1,$2,$3)")
            .bind(key)
            .bind(account)
            .bind(amount)
            .execute(&mut **tx)
            .await?;
        receipt(tx, key, false).await
    }
    /// Settle once in the caller transaction; capture and release are mutually exclusive.
    /// Roll back the entire transaction on error. The key binds the settlement payload.
    pub async fn settle_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key: Uuid,
        account: Uuid,
        reservation: Uuid,
        capture: bool,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        valid_key(reservation)?;
        lock(tx, account).await?;
        self.active(tx, account).await?;
        let row = sqlx::query("SELECT amount-COALESCE((to_jsonb(wallet_reservations)->>'consumed')::bigint,0) AS amount, status FROM wallet_reservations WHERE id=$1 AND account_id=$2 FOR UPDATE")
            .bind(reservation).bind(account).fetch_optional(&mut **tx).await?.ok_or(Refusal::ReservationMissing)?;
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
        if claim(tx, &operation).await? {
            return receipt(tx, key, true).await;
        }
        if status != "active" {
            return Err(Refusal::ReservationSettled.into());
        }
        entry(tx, key, account, if capture { 0 } else { amount }, -amount).await?;
        sqlx::query("UPDATE wallet_reservations SET status=$2, updated_at=now() WHERE id=$1")
            .bind(reservation)
            .bind(if capture { "captured" } else { "released" })
            .execute(&mut **tx)
            .await?;
        receipt(tx, key, false).await
    }
}

impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    /// Immutable committed result for reconciliation. None also covers in-flight work.
    pub async fn operation(&self, key: Uuid) -> Result<Option<Receipt>, Error> {
        self.enabled()?;
        valid_key(key)?;
        let mut tx = self.pool.begin().await?;
        let account: Option<Uuid> =
            sqlx::query_scalar("SELECT account_id FROM wallet_operations WHERE id=$1")
                .bind(key)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(account) = account else {
            return Ok(None);
        };
        self.active(&mut tx, account).await?;
        let result = receipt(&mut tx, key, false).await?;
        tx.commit().await?;
        Ok(Some(result))
    }
    /// Audited reward or staff credit to the one coin balance: spendable,
    /// reservable and transferable like a top-up. The actor, reason and the
    /// transactional audit adapter are required; replay does not repeat it.
    #[allow(clippy::too_many_arguments)]
    pub async fn grant(
        &self,
        key: Uuid,
        account: Uuid,
        amount: i64,
        actor: Uuid,
        reason: &str,
        kind: GrantKind,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        valid_id(actor)?;
        positive(amount)?;
        if reason.trim().is_empty() || reason.chars().count() > 1000 {
            return Err(Refusal::InvalidReason.into());
        }
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let op = Operation {
            key,
            kind: kind.name(),
            account,
            counterparty: None,
            amount,
            reference: Some(actor),
            reason: Some(reason),
        };
        let replayed = claim(&mut tx, &op).await?;
        if !replayed {
            let before = balance(&mut tx, account).await?;
            let after = before.apply(amount, 0)?;
            if !self
                .audit
                .adjustment(
                    &mut tx,
                    &AdjustmentAudit {
                        operation_id: key,
                        account_id: account,
                        actor_id: actor,
                        amount,
                        reason: reason.into(),
                        before,
                        after,
                    },
                )
                .await
            {
                return Err(Refusal::AuditDenied.into());
            }
            credit(&mut tx, key, account, amount).await?;
        }
        let result = receipt(&mut tx, key, replayed).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// SELL price is part of the operation binding; gold remains Game-owned.
    pub async fn reserve_offer(
        &self,
        key: Uuid,
        account: Uuid,
        amount: i64,
        gold_price: i64,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(key)?;
        valid_id(account)?;
        positive(amount)?;
        positive(gold_price)?;
        let reason = format!("gold-price:{gold_price}");
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, account).await?;
        self.active(&mut tx, account).await?;
        let replayed = claim(
            &mut tx,
            &Operation {
                key,
                kind: "reserve",
                account,
                counterparty: None,
                amount,
                reference: None,
                reason: Some(&reason),
            },
        )
        .await?;
        if !replayed {
            self.allowed(&mut tx, account).await?;
            crate::debt::unlocked(&mut tx, account).await?;
            entry(&mut tx, key, account, -amount, amount).await?;
            sqlx::query("INSERT INTO wallet_reservations (id,account_id,amount) VALUES ($1,$2,$3)")
                .bind(key)
                .bind(account)
                .bind(amount)
                .execute(&mut *tx)
                .await?;
        }
        let result = receipt(&mut tx, key, replayed).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Atomic partial capture and buyer credit. All wallet rows lock in UUID order.
    pub async fn settle(
        &self,
        trade: Uuid,
        reservation: Uuid,
        buyer: Uuid,
        amount: i64,
    ) -> Result<Receipt, Error> {
        self.enabled()?;
        valid_key(trade)?;
        valid_key(reservation)?;
        valid_id(buyer)?;
        positive(amount)?;
        let mut tx = self.pool.begin().await?;
        // Resolve an existing key before the reservation lookup: changing a
        // used trade to a nonexistent reservation is still a payload conflict.
        let prior = sqlx::query("SELECT account_id,COALESCE(kind='settlement' AND reference_id=$2 AND counterparty_id=$3 AND amount=$4,false) AS same FROM wallet_operations WHERE id=$1")
            .bind(trade).bind(reservation).bind(buyer).bind(amount).fetch_optional(&mut *tx).await?;
        let seller: Uuid = if let Some(prior) = prior {
            if !prior.try_get::<bool, _>("same")? {
                return Err(Refusal::IdempotencyConflict.into());
            }
            prior.try_get("account_id")?
        } else {
            sqlx::query_scalar("SELECT account_id FROM wallet_reservations WHERE id=$1")
                .bind(reservation)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Refusal::ReservationMissing)?
        };
        if seller == buyer {
            return Err(Refusal::SameAccount.into());
        }
        for account in lock_order(seller, buyer) {
            lock(&mut tx, account).await?;
            self.active(&mut tx, account).await?;
        }
        let replayed = claim(
            &mut tx,
            &Operation {
                key: trade,
                kind: "settlement",
                account: seller,
                counterparty: Some(buyer),
                amount,
                reference: Some(reservation),
                reason: None,
            },
        )
        .await?;
        if !replayed {
            self.allowed(&mut tx, seller).await?;
            self.allowed(&mut tx, buyer).await?;
            let row = sqlx::query("SELECT amount-consumed AS remaining,status FROM wallet_reservations WHERE id=$1 FOR UPDATE").bind(reservation).fetch_one(&mut *tx).await?;
            let remaining: i64 = row.try_get("remaining")?;
            let status: String = row.try_get("status")?;
            if status != "active" || amount > remaining {
                return Err(Refusal::InsufficientFunds.into());
            }
            entry(&mut tx, trade, seller, 0, -amount).await?;
            credit(&mut tx, trade, buyer, amount).await?;
            sqlx::query("UPDATE wallet_reservations SET consumed=consumed+$2,status=CASE WHEN consumed+$2=amount THEN 'captured' ELSE 'active' END,updated_at=now() WHERE id=$1").bind(reservation).bind(amount).execute(&mut *tx).await?;
        }
        let result = receipt(&mut tx, trade, replayed).await?;
        tx.commit().await?;
        Ok(result)
    }
}

impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Ledger<A, B, H> {
    /// Wire preflight distinguishes absent identity from inactive and unavailable
    /// authority. Every mutation repeats authorization in its own transaction.
    pub async fn coin_account(&self, account: Uuid) -> Result<(), Error> {
        self.enabled()?;
        valid_id(account)?;
        let mut tx = self.pool.begin().await?;
        let result = match self.accounts.state(&mut tx, account).await {
            AccountState::Active => Ok(()),
            AccountState::Inactive => Err(Refusal::AccountDenied.into()),
            AccountState::Unknown => Err(Refusal::UnknownAccount.into()),
            AccountState::Unavailable => Err(Refusal::TransferStateUnknown.into()),
        };
        tx.commit().await?;
        result
    }
}
