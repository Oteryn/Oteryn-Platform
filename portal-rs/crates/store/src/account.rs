//! Read-only Premium ledger projection for Portal account and staff views.
use crate::premium::{
    Entitlement, Error,
    repository::{current, recorded},
};
use sqlx::{PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

/// Viewing the panel never issues a snapshot or advances authority revisions.
pub async fn premium(pool: &PgPool, account: Uuid) -> Result<Option<Entitlement>, Error> {
    let mut tx = read_transaction(pool).await?;
    let value = current(&mut tx, account).await?;
    tx.commit().await?;
    Ok(value)
}

pub const PREMIUM_PAGE_SIZE: usize = 20;

#[derive(Debug)]
pub struct PremiumEvent {
    pub entitlement: Entitlement,
    pub days: Option<i32>,
    pub recorded_at: OffsetDateTime,
    pub purchase: Option<PremiumPurchase>,
}

#[derive(Debug)]
pub struct PremiumPurchase {
    pub order: Uuid,
    pub coins: i64,
    pub purchased_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct PremiumHistory {
    pub current: Option<Entitlement>,
    pub events: Vec<PremiumEvent>,
}

#[derive(Clone, Copy, Debug)]
pub enum PremiumHistoryQuery {
    Page(u32),
    Receipt(Uuid),
}

async fn read_transaction(pool: &PgPool) -> Result<Transaction<'_, Postgres>, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

/// Read status and paged events from one snapshot. A coin receipt is backed by
/// the accepted order, its wallet debit and the matching Premium grant.
pub async fn premium_history(
    pool: &PgPool,
    account: Uuid,
    query: PremiumHistoryQuery,
) -> Result<PremiumHistory, Error> {
    let (offset, order) = match query {
        PremiumHistoryQuery::Page(page @ 1..=10_000) => {
            (i64::from(page - 1) * PREMIUM_PAGE_SIZE as i64, None)
        }
        PremiumHistoryQuery::Page(_) => return Err(Error::Invalid),
        PremiumHistoryQuery::Receipt(order) => (0, Some(order)),
    };
    let mut tx = read_transaction(pool).await?;
    let current = current(&mut tx, account).await?;
    let rows = sqlx::query(
        "SELECT p.entitlement_id,p.lifecycle_revision,p.event_type,p.duration_days,
                p.effective_from,p.effective_until,p.created_at,
                purchase.order_id,purchase.total_price,purchase.purchased_at
         FROM premium_ledger p
         LEFT JOIN LATERAL (
             SELECT o.order_id,o.total_price,o.created_at AS purchased_at
             FROM store_fulfilment_units u
             JOIN store_order_lines l USING (line_id)
             JOIN store_orders o USING (order_id)
             JOIN store_purchase_requests r USING (purchase_request_id)
             JOIN wallet_operations w ON w.id=o.wallet_operation_id
               AND w.id=r.purchase_request_id
             JOIN wallet_entries e ON e.operation_id=w.id AND e.account_id=p.account_id
             WHERE u.entitlement_id=p.request_id AND u.account_id=p.account_id
               AND o.account_id=p.account_id AND r.account_id=p.account_id
               AND w.account_id=p.account_id AND r.result='ACCEPTED'
               AND w.kind='store_purchase' AND w.amount=-o.total_price
               AND e.available_delta=-o.total_price AND e.reserved_delta=0
               AND p.event_type='GRANT' AND u.unit_kind='PREMIUM_TIME'
               AND u.quantity=p.duration_days
         ) purchase ON true
         WHERE p.account_id=$1 AND ($2::uuid IS NULL OR purchase.order_id=$2)
         ORDER BY p.lifecycle_revision DESC LIMIT $3 OFFSET $4",
    )
    .bind(account)
    .bind(order)
    .bind((PREMIUM_PAGE_SIZE + 1) as i64)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    let events = rows
        .iter()
        .map(|row| {
            let entitlement = recorded(row)?;
            let purchase = row
                .try_get::<Option<Uuid>, _>("order_id")?
                .map(|order| -> Result<_, sqlx::Error> {
                    Ok(PremiumPurchase {
                        order,
                        coins: row.try_get("total_price")?,
                        purchased_at: row.try_get("purchased_at")?,
                    })
                })
                .transpose()?;
            Ok(PremiumEvent {
                entitlement,
                days: row.try_get("duration_days")?,
                recorded_at: row.try_get("created_at")?,
                purchase,
            })
        })
        .collect::<Result<_, Error>>()?;
    tx.commit().await?;
    Ok(PremiumHistory { current, events })
}
