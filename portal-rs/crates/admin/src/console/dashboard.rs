//! Platform-owned daily metrics; UTC boundaries and separate totals per currency.
use super::{DisplayRecord, Section, text};
use sqlx::PgPool;
pub const PERMISSION: &str = "game.ops.read";

pub(super) async fn section(pool: &PgPool, lang: &str) -> Section {
    let mut section = Section {
        title: text(lang, "platform-metrics"),
        source: text(lang, "platform-source"),
        freshness: text(lang, "unknown"),
        state: text(lang, "unavailable"),
        revision: String::new(),
        records: vec![],
        operation: "",
        can_query: false,
    };
    if let Ok(records) = metrics(pool, lang).await {
        section.records = records;
        section.freshness = text(lang, "now");
        section.state = text(lang, "fresh");
    }
    section
}
async fn metrics(pool: &PgPool, lang: &str) -> Result<Vec<DisplayRecord>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let (registrations,logins): (i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM accounts WHERE created_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AND created_at <= now()), (SELECT count(*) FROM security_events WHERE kind='login_succeeded' AND created_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AND created_at <= now())").fetch_one(&mut *tx).await?;
    // Provider success time is authoritative for today's payments, not intent creation.
    // A later refund is excluded. The first success prevents duplicate daily attribution.
    let revenue:Vec<(String,String)>=sqlx::query_as("SELECT i.currency, sum(i.amount)::text FROM payment_intents i JOIN (SELECT intent_id,min(created_at) AS paid_at FROM payment_events WHERE state='succeeded' GROUP BY intent_id) e ON e.intent_id=i.id WHERE i.state='succeeded' AND e.paid_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' AND e.paid_at <= now() GROUP BY i.currency ORDER BY i.currency").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut fields = vec![
        (text(lang, "registrations"), registrations.to_string()),
        (text(lang, "logins"), logins.to_string()),
    ];
    if revenue.is_empty() {
        fields.push((text(lang, "revenue"), text(lang, "no-revenue")));
    }
    for (currency, amount) in revenue {
        fields.push((format!("{} ({currency})", text(lang, "revenue")), amount));
    }
    Ok(vec![DisplayRecord {
        name: text(lang, "today"),
        fields,
    }])
}
