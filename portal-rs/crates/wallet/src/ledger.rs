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
