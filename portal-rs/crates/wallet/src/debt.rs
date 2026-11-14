//! Coin debt (owner decision 2026-11-14). A purchase reversal the balance could
//! not cover is recorded by payments as a held reversal: its `coins - debited`
//! less what credits have since repaid (`coin_outstanding_debts`) is still owed.
//! Every coin credit pays that debt first, oldest first, in the credit's
//! transaction and under the wallet account lock the credit already holds.
//! While anything is owed, outgoing transfers and market listings are refused.
//! The debt rows are read through the view only; no payments code is called.
use crate::{Error, Refusal};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

/// Coins the account still owes, 0 when none. The caller holds the wallet
/// account lock when the answer decides a mutation.
pub async fn outstanding_debt(connection: &mut PgConnection, account: Uuid) -> Result<i64, Error> {
    let owed: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(owed),0)::bigint FROM coin_outstanding_debts WHERE account_id=$1 AND owed>0",
    )
    .bind(account)
    .fetch_one(connection)
    .await?;
    Ok(owed)
}

/// Refuse an outgoing effect while debt is outstanding.
pub(crate) async fn unlocked(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
) -> Result<(), Error> {
    if outstanding_debt(tx, account).await? > 0 {
        return Err(Refusal::DebtOutstanding.into());
    }
    Ok(())
}

/// Pay the account's debt from a credit of `coins` that was just entered under
/// operation `credit`, with the wallet account row already locked. The held
/// reversals are locked oldest first; each one paid gets a settlement row and
/// an audit row. One `debt_settlement` operation takes the total back from the
/// balance, never more than the credit, so the balance cannot go negative.
/// Returns the coins applied.
pub(crate) async fn settle(
    tx: &mut Transaction<'_, Postgres>,
    credit: Uuid,
    account: Uuid,
    coins: i64,
) -> Result<i64, Error> {
    // Held reversals per account are few: each one places a purchase hold
    // that freezes new purchasing until staff release it.
    sqlx::query("SELECT id FROM coin_purchase_reversals WHERE payer_account_id=$1 AND outcome='held' ORDER BY created_at,id FOR UPDATE")
        .bind(account)
        .fetch_all(&mut **tx)
        .await?;
    let debts: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        "SELECT reversal_id,order_id,owed FROM coin_outstanding_debts WHERE account_id=$1 AND owed>0 ORDER BY created_at,reversal_id",
    )
    .bind(account)
    .fetch_all(&mut **tx)
    .await?;
    let mut remaining = coins;
    let mut steps = Vec::new();
    for (reversal, order, owed) in debts {
        if remaining == 0 {
            break;
        }
        let applied = remaining.min(owed);
        remaining -= applied;
        steps.push((reversal, order, owed, applied));
    }
    let total = coins - remaining;
    if total == 0 {
        return Ok(0);
    }
    let settlement = Uuid::now_v7();
    sqlx::query("INSERT INTO wallet_operations (id,kind,account_id,amount,reference_id) VALUES ($1,'debt_settlement',$2,$3,$4)")
        .bind(settlement).bind(account).bind(total).bind(credit)
        .execute(&mut **tx).await?;
    crate::ledger::entry(tx, settlement, account, -total, 0).await?;
    for (reversal, order, owed, applied) in steps {
        sqlx::query("INSERT INTO coin_debt_settlements (id,reversal_id,account_id,credit_operation_id,settlement_operation_id,coins,owed_before) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(Uuid::now_v7()).bind(reversal).bind(account).bind(credit).bind(settlement).bind(applied).bind(owed)
            .execute(&mut **tx).await?;
        sqlx::query("INSERT INTO audit_log (actor,action,target,before,after,request_id) VALUES (NULL,'wallet.coin_debt_settled',$1,jsonb_build_object('owed',$2::bigint),jsonb_build_object('owed',$2::bigint-$3::bigint,'applied',$3::bigint,'account_id',$4::uuid,'order_id',$5::uuid,'credit_operation_id',$6::uuid,'settlement_operation_id',$7::uuid),$6)")
            .bind(reversal.to_string()).bind(owed).bind(applied).bind(account).bind(order).bind(credit).bind(settlement)
            .execute(&mut **tx).await?;
    }
    Ok(total)
}
