//! Transaction-local Platform adapters. Never access Game tables or services.
use super::Authority;
use crate::{
    AccountState, Accounts, Currency, Error, Evidence, Reservation, Reservations, Settlement,
};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

/// Bind sudo to the actual authenticated session. Missing session evidence denies
/// listing even when another session for this account has recently authenticated.
#[derive(Clone, Debug, Default)]
pub struct PlatformAccounts {
    pub enabled: bool,
    pub session_hash: Option<[u8; 32]>,
}
impl Accounts for PlatformAccounts {
    async fn prepare(&self, db: &mut PgConnection, accounts: &[Uuid]) -> Result<(), Error> {
        if self.enabled {
            crate::locking::accounts(db, accounts).await?;
        }
        Ok(())
    }

    async fn safe_return(&self, db: &mut PgConnection, account: Uuid) -> Result<bool, Error> {
        if !self.enabled {
            return Ok(false);
        }
        crate::locking::listing_gate(db).await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .fetch_optional(db)
                .await?;
        Ok(matches!(status.as_deref(), Some("active" | "suspended")))
    }

    async fn state(
        &self,
        db: &mut PgConnection,
        account: Uuid,
    ) -> Result<Option<AccountState>, Error> {
        if !self.enabled {
            return Ok(None);
        }
        crate::locking::listing_gate(db).await?;
        let row=sqlx::query("SELECT status,email_verified_at IS NOT NULL AS verified,email_change_blocked_until>now() AS blocked FROM accounts WHERE id=$1 FOR SHARE").bind(account).fetch_optional(&mut *db).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mfa: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM account_mfa_totp WHERE account_id=$1 AND confirmed)",
        )
        .bind(account)
        .fetch_one(&mut *db)
        .await?;
        let blocked = !oteryn_wallet::value_eligible_in(db, account).await?;
        let sudo = if let Some(hash) = self.session_hash {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2 AND sudo_until>now() AND idle_expires_at>now() AND absolute_expires_at>now())").bind(hash.to_vec()).bind(account).fetch_one(&mut *db).await?
        } else {
            false
        };
        Ok(Some(AccountState {
            active: row.try_get::<String, _>("status")? == "active",
            email_verified: row.try_get("verified")?,
            mfa,
            sudo,
            bazaar_blocked: blocked || row.try_get::<bool, _>("blocked")?,
        }))
    }
}
#[derive(Clone, Debug)]
pub struct CoinReservations {
    pub authority: Authority,
}
impl CoinReservations {
    /// Recheck the durable promise before either item delivery or character
    /// ownership transfer, including obligations frozen by an older runtime.
    pub(super) async fn handoff_capacity(
        &self,
        db: &mut PgConnection,
        s: &Settlement,
    ) -> Result<(), Error> {
        let mut accounts = [s.buyer, s.seller];
        accounts.sort();
        self.lock_accounts(db, &accounts).await?;
        let promised: Option<i64> = sqlx::query_scalar(
            "SELECT amount FROM wallet_incoming_capacity WHERE id=$1 AND account_id=$2",
        )
        .bind(s.id)
        .bind(s.seller)
        .fetch_optional(&mut *db)
        .await?;
        if promised != Some(s.seller_net) {
            return Err(Error::Denied);
        }
        let total: i64 = sqlx::query_scalar("SELECT COALESCE(sum(available_delta::numeric+reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1")
            .bind(s.seller).fetch_one(&mut *db).await?;
        let incoming = oteryn_wallet::incoming_capacity_in(db, s.seller)
            .await
            .map_err(wallet_error)?;
        if total
            .checked_add(incoming)
            .is_none_or(|n| n > oteryn_wallet::MAX_BALANCE)
        {
            return Err(Error::Overflow);
        }
        Ok(())
    }

    // Share the issuer/restore fence with evidence before taking any Wallet
    // locks. The same ordering is used by every Bazaar monetary port call.
    async fn fence(&self, db: &mut PgConnection) -> Result<(), Error> {
        crate::locking::listing_gate(db).await?;
        self.authority.lock(db).await.map_err(|e| match e {
            super::Error::Database(e) => Error::Database(e),
            super::Error::Unavailable => Error::Disabled,
            _ => Error::Denied,
        })
    }

    async fn account(db: &mut PgConnection, account: Uuid, obligation: bool) -> Result<(), Error> {
        // Existing obligations accept suspended identities; new reserves require active.
        // The complete identity set is acquired before any wallet row.
        crate::locking::listing_gate(db).await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .fetch_optional(&mut *db)
                .await?;
        if status.as_deref() != Some("active")
            && !(obligation && status.as_deref() == Some("suspended"))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn entry(
        db: &mut PgConnection,
        operation: Uuid,
        account: Uuid,
        available: i64,
        reserved: i64,
    ) -> Result<(), Error> {
        if available != 0 || reserved != 0 {
            sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,$4,$5)").bind(oteryn_core::new_id()).bind(operation).bind(account).bind(available).bind(reserved).execute(&mut *db).await?;
        }
        Ok(())
    }
    async fn operation(
        db: &mut PgConnection,
        kind: &str,
        account: Uuid,
        amount: i64,
        reference: Uuid,
        counterparty: Option<Uuid>,
    ) -> Result<Uuid, Error> {
        let operation = oteryn_core::new_id();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,counterparty_id,amount,reference_id,reason) VALUES($1,$2,$3,$4,$5,$6,'Bazaar authority v1')").bind(operation).bind(kind).bind(account).bind(counterparty).bind(amount).bind(reference).execute(&mut *db).await?;
        Ok(operation)
    }
    fn binding(s: &Settlement) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            s.listing_id,
            s.buyer,
            s.seller,
            s.amount,
            s.commission,
            s.seller_net,
            s.reservation,
            s.id
        )
    }
}
impl Reservations for CoinReservations {
    async fn lock_accounts(&self, db: &mut PgConnection, accounts: &[Uuid]) -> Result<(), Error> {
        crate::locking::accounts(db, accounts).await?;
        self.fence(db).await?;
        if accounts.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::InvalidInput);
        }
        for account in accounts {
            Self::account(db, *account, true).await?;
        }
        for &account in accounts {
            sqlx::query(
                "INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING",
            )
            .bind(account)
            .execute(&mut *db)
            .await?;
            sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
                .bind(account)
                .fetch_one(&mut *db)
                .await?;
        }
        Ok(())
    }
    async fn reserve(&self, db: &mut PgConnection, h: Reservation) -> Result<(), Error> {
        crate::locking::accounts(db, &[h.account]).await?;
        self.fence(db).await?;
        if h.currency == Currency::Gold {
            return super::runtime::gold::reserve(&self.authority, db, h)
                .await
                .map_err(gold_error);
        }
        if h.currency != Currency::Coins || h.amount <= 0 {
            return Err(Error::Denied);
        }
        Self::account(db, h.account, false).await?;
        let old=sqlx::query("SELECT amount,settlement_id,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(h.id).bind(h.account).fetch_optional(&mut *db).await?;
        let amount = if let Some(old) = old {
            if old.try_get::<Option<Uuid>, _>("settlement_id")?.is_some()
                || old.try_get::<bool, _>("completed")?
            {
                return Err(Error::Denied);
            }
            old.try_get::<i64, _>("amount")?
        } else {
            0
        };
        if amount == h.amount {
            return Ok(());
        }
        let delta = h.amount.checked_sub(amount).ok_or(Error::Overflow)?;
        // Coins still owed never leave through the market (owner decision
        // 2026-11-14); a lowered or replayed hold returns own coins.
        if delta > 0 {
            if !oteryn_wallet::value_eligible_in(db, h.account).await? {
                return Err(Error::Denied);
            }
            oteryn_wallet::unlocked_in(db, h.account)
                .await
                .map_err(wallet_error)?;
        }
        let available:i64=sqlx::query_scalar("SELECT COALESCE(sum(available_delta),0)::bigint FROM wallet_entries WHERE account_id=$1").bind(h.account).fetch_one(&mut *db).await?;
        if delta > available {
            return Err(Error::InsufficientFunds);
        }
        let op = Self::operation(
            db,
            if delta > 0 { "reserve" } else { "release" },
            h.account,
            delta.unsigned_abs() as i64,
            h.id,
            None,
        )
        .await?;
        Self::entry(db, op, h.account, -delta, delta).await?;
        sqlx::query("INSERT INTO bazaar_coin_holds(id,account_id,amount) VALUES($1,$2,$3) ON CONFLICT(id,account_id) DO UPDATE SET amount=EXCLUDED.amount").bind(h.id).bind(h.account).bind(h.amount).execute(&mut *db).await?;
        Ok(())
    }
    async fn release(&self, db: &mut PgConnection, id: Uuid, account: Uuid) -> Result<(), Error> {
        crate::locking::accounts(db, &[account]).await?;
        self.fence(db).await?;
        Self::account(db, account, true).await?;
        crate::locking::listing_gate(db).await?;
        let row=sqlx::query("SELECT amount,settlement_id FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(id).bind(account).fetch_optional(&mut *db).await?;
        let Some(row) = row else {
            if let Some(hold) =
                super::runtime::gold::current_hold(&self.authority, db, id, account, None)
                    .await
                    .map_err(gold_error)?
                && hold.amount != 0
            {
                return Err(Error::Denied);
            }
            return Ok(());
        };
        let settlement = row.try_get::<Option<Uuid>, _>("settlement_id")?;
        if let Some(sale) = settlement {
            let listing: crate::Listing = sqlx::query_as("SELECT l.* FROM bazaar_listings l JOIN bazaar_settlements s ON s.listing_id=l.id WHERE s.id=$1 AND s.buyer=$2 AND s.reservation=$3")
                .bind(sale).bind(account).bind(id).fetch_one(&mut *db).await?;
            let completed: bool = sqlx::query_scalar(
                "SELECT completed FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE",
            )
            .bind(sale)
            .fetch_one(&mut *db)
            .await?;
            if !self
                .authority
                .compensation_ready(db, &listing)
                .await
                .map_err(gold_error)?
            {
                return Err(Error::Denied);
            }
            if completed {
                return if row.try_get::<i64, _>("amount")? == 0 {
                    Ok(())
                } else {
                    Err(Error::Denied)
                };
            }
        }
        let amount: i64 = row.try_get("amount")?;
        if amount == 0 {
            return Ok(());
        }
        // Any executable handoff prevents release; timeout is not a refusal.
        let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE intent::jsonb->'funding'->>'reservation_id'=$1)").bind(id.to_string()).fetch_one(&mut *db).await?;
        if pending && settlement.is_none() {
            return Err(Error::Denied);
        }
        let op = Self::operation(
            db,
            "release",
            account,
            amount,
            settlement.unwrap_or(id),
            None,
        )
        .await?;
        Self::entry(db, op, account, amount, -amount).await?;
        sqlx::query("UPDATE bazaar_coin_holds SET amount=0 WHERE id=$1 AND account_id=$2")
            .bind(id)
            .bind(account)
            .execute(&mut *db)
            .await?;
        if let Some(sale) = settlement {
            // compensation_ready above proved that no delivery can execute.
            sqlx::query("DELETE FROM wallet_incoming_capacity WHERE id=$1")
                .bind(sale)
                .execute(&mut *db)
                .await?;
            sqlx::query(
                "UPDATE bazaar_coin_holds SET completed=true WHERE id=$1 AND account_id=$2",
            )
            .bind(id)
            .bind(account)
            .execute(&mut *db)
            .await?;
            sqlx::query(
                "UPDATE bazaar_coin_dispositions SET completed=true WHERE settlement_id=$1",
            )
            .bind(sale)
            .execute(&mut *db)
            .await?;
        }
        Ok(())
    }
    async fn settle(&self, db: &mut PgConnection, s: &Settlement) -> Result<(), Error> {
        self.authority.enabled().map_err(|_| Error::Disabled)?;
        if s.currency == Currency::Gold {
            return super::runtime::gold::settled(&self.authority, db, s, false)
                .await
                .map_err(gold_error);
        }
        if s.currency != Currency::Coins
            || s.buyer == s.seller
            || s.amount <= 0
            || s.commission != s.amount / 10
            || s.seller_net != s.amount - s.commission
        {
            return Err(Error::Denied);
        }
        let mut accounts = [s.buyer, s.seller];
        accounts.sort();
        self.lock_accounts(db, &accounts).await?;
        let old: Option<String> = sqlx::query_scalar(
            "SELECT binding FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE",
        )
        .bind(s.id)
        .fetch_optional(&mut *db)
        .await?;
        if let Some(old) = old {
            // An exact replay finishes an existing obligation (a delivered
            // capture stays completable after a later suspension).
            return if old == Self::binding(s) {
                Ok(())
            } else {
                Err(Error::IdempotencyConflict)
            };
        }
        // Locking participants also serves safe returns; a new sale still needs
        // current value eligibility for both participants.
        for account in accounts {
            if !oteryn_wallet::value_eligible_in(db, account).await? {
                return Err(Error::Denied);
            }
        }
        crate::locking::listing_gate(db).await?;
        let row=sqlx::query("SELECT amount,settlement_id FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(s.reservation).bind(s.buyer).fetch_optional(&mut *db).await?.ok_or(Error::InsufficientFunds)?;
        if row.try_get::<i64, _>("amount")? < s.amount
            || row.try_get::<Option<Uuid>, _>("settlement_id")?.is_some()
        {
            return Err(Error::Denied);
        }
        oteryn_wallet::reserve_incoming_in(db, s.id, s.seller, s.seller_net)
            .await
            .map_err(wallet_error)?;
        sqlx::query("INSERT INTO bazaar_coin_dispositions(settlement_id,binding) VALUES($1,$2)")
            .bind(s.id)
            .bind(Self::binding(s))
            .execute(&mut *db)
            .await?;
        sqlx::query("UPDATE bazaar_coin_holds SET settlement_id=$3 WHERE id=$1 AND account_id=$2")
            .bind(s.reservation)
            .bind(s.buyer)
            .bind(s.id)
            .execute(&mut *db)
            .await?;
        Ok(())
    }
    async fn complete(&self, db: &mut PgConnection, s: &Settlement) -> Result<(), Error> {
        if s.currency == Currency::Gold {
            return super::runtime::gold::settled(&self.authority, db, s, true)
                .await
                .map_err(gold_error);
        }
        self.settle(db, s).await?;
        if !self.authority.delivered(db, s).await? {
            return Err(Error::Denied);
        }
        let completed: bool = sqlx::query_scalar(
            "SELECT completed FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE",
        )
        .bind(s.id)
        .fetch_one(&mut *db)
        .await?;
        if completed {
            return Ok(());
        }
        let held: i64 = sqlx::query_scalar(
            "SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE",
        )
        .bind(s.reservation)
        .bind(s.buyer)
        .fetch_one(&mut *db)
        .await?;
        let (buyer_available,buyer_reserved):(i64,i64)=sqlx::query_as("SELECT COALESCE(sum(available_delta),0)::bigint,COALESCE(sum(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1").bind(s.buyer).fetch_one(&mut *db).await?;
        let (seller_available,seller_reserved):(i64,i64)=sqlx::query_as("SELECT COALESCE(sum(available_delta),0)::bigint,COALESCE(sum(reserved_delta),0)::bigint FROM wallet_entries WHERE account_id=$1").bind(s.seller).fetch_one(&mut *db).await?;
        if buyer_available < 0
            || buyer_reserved < held
            || seller_available < 0
            || seller_reserved < 0
            || seller_available
                .checked_add(seller_reserved)
                .and_then(|total| total.checked_add(s.seller_net))
                .is_none()
        {
            return Err(Error::Overflow);
        }
        let excess = held
            .checked_sub(s.amount)
            .filter(|n| *n >= 0)
            .ok_or(Error::InsufficientFunds)?;
        let op = Self::operation(db, "capture", s.buyer, s.amount, s.id, Some(s.seller)).await?;
        Self::entry(db, op, s.buyer, excess, -held).await?;
        // Consume only our promise under both wallet locks, in the payout
        // transaction; every other pending sale retains its capacity.
        sqlx::query("DELETE FROM wallet_incoming_capacity WHERE id=$1 AND account_id=$2")
            .bind(s.id)
            .bind(s.seller)
            .execute(&mut *db)
            .await?;
        // Proceeds are a credit: they repay the seller's coin debt first.
        oteryn_wallet::credit_in(db, op, s.seller, s.seller_net)
            .await
            .map_err(wallet_error)?;
        sqlx::query(
            "UPDATE bazaar_coin_holds SET amount=0,completed=true WHERE id=$1 AND account_id=$2",
        )
        .bind(s.reservation)
        .bind(s.buyer)
        .execute(&mut *db)
        .await?;
        sqlx::query("UPDATE bazaar_coin_dispositions SET completed=true WHERE settlement_id=$1")
            .bind(s.id)
            .execute(&mut *db)
            .await?;
        Ok(())
    }
}

fn gold_error(e: super::Error) -> Error {
    match e {
        super::Error::Database(e) => Error::Database(e),
        super::Error::Unavailable => Error::Disabled,
        _ => Error::Denied,
    }
}

/// Wallet refusals on the coin path. Outstanding coin debt reads as
/// insufficient funds: those coins are not the account's to spend.
fn wallet_error(e: oteryn_wallet::Error) -> Error {
    use oteryn_wallet::Refusal;
    match e {
        oteryn_wallet::Error::Storage(e) => Error::Database(e),
        oteryn_wallet::Error::Refused(Refusal::DebtOutstanding | Refusal::InsufficientFunds) => {
            Error::InsufficientFunds
        }
        oteryn_wallet::Error::Refused(Refusal::Overflow) => Error::Overflow,
        oteryn_wallet::Error::Refused(_) => Error::Denied,
    }
}

#[cfg(test)]
#[path = "coin_ports_lock_tests.rs"]
mod lock_tests;
