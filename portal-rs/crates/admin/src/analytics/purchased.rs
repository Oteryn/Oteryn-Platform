//! Platform ledger aggregates only. Never part of the Game ingestion schema.
use sqlx::{Postgres, Transaction};

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct PaidCurrency {
    pub currency: String,
    pub count: String,
    pub coins: String,
    pub minor_units: String,
}
#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct Purchased {
    pub count: String,
    pub coins: String,
    pub paid: Vec<PaidCurrency>,
}
/// One half-open UTC day, measured at the committed wallet credit, across all
/// worlds. Money stays partitioned by payment currency. No identities escape.
pub async fn read_in(
    tx: &mut Transaction<'_, Postgres>,
    start: u64,
    end: u64,
) -> Result<Purchased, sqlx::Error> {
    if !start.is_multiple_of(86400) || start.checked_add(86400) != Some(end) {
        return Err(sqlx::Error::Protocol("one UTC day required".into()));
    }
    let timestamp = |value| {
        i64::try_from(value)
            .ok()
            .and_then(|v| time::OffsetDateTime::from_unix_timestamp(v).ok())
            .ok_or_else(|| sqlx::Error::Protocol("invalid UTC day".into()))
    };
    let start = timestamp(start)?;
    let end = timestamp(end)?;
    // The intent ID is the deterministic top-up wallet operation key. Entry
    // uniqueness and the classified operation exclude requests and replays.
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT t.currency,count(*)::text,sum(e.available_delta)::text,sum(t.minor_units)::text,
                sum(count(*)) OVER ()::text, sum(sum(e.available_delta)) OVER ()::text
         FROM wallet_entries e JOIN wallet_operations o ON o.id=e.operation_id
         JOIN store_web_topups t ON t.payment_intent_id=o.id
         WHERE o.kind='top_up' AND e.account_id=t.account_id AND o.account_id=t.account_id
           AND e.available_delta=t.coins AND o.amount=t.coins AND e.reserved_delta=0
           AND e.created_at >= $1
           AND e.created_at < $2
         GROUP BY t.currency ORDER BY t.currency COLLATE \"C\"",
    )
    .bind(start)
    .bind(end)
    .fetch_all(&mut **tx)
    .await?;
    // PostgreSQL numeric sums avoid bigint overflow; decimal strings keep exact
    // values through JSON and browser rendering. Totals never sum money units.
    let (count, coins) = rows
        .first()
        .map(|r| (r.4.clone(), r.5.clone()))
        .unwrap_or_else(|| ("0".into(), "0".into()));
    let mut paid = Vec::new();
    for (currency, count, coins, minor_units, _, _) in rows {
        paid.push(PaidCurrency {
            currency,
            count,
            coins,
            minor_units,
        });
    }
    Ok(Purchased { count, coins, paid })
}
