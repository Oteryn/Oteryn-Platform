//! Durable Platform alarms, independently readable when the game channel is offline.
use super::{DisplayRecord, Section, text};
use sqlx::PgPool;

pub(super) async fn section(pool: &PgPool, lang: &str) -> Section {
    let mut section = Section {
        title: text(lang, "alerts"),
        source: text(lang, "platform-source"),
        freshness: text(lang, "unknown"),
        state: text(lang, "unavailable"),
        revision: String::new(),
        records: vec![],
        operation: "",
        can_query: false,
    };
    if let Ok(records) = records(pool).await {
        section.records = records;
        section.state = text(lang, "fresh");
        section.freshness = text(lang, "now");
    }
    section
}
async fn records(pool: &PgPool) -> Result<Vec<DisplayRecord>, sqlx::Error> {
    let rows: Vec<(String,String,String,String,String,String)> = sqlx::query_as("SELECT id::text,created_at::text,after->>'authority',after->>'family',after->>'record_id',target FROM audit_log WHERE actor IS NULL AND action='ops.playerdata_binding_conflict' AND created_at>clock_timestamp()-interval '7 days' ORDER BY created_at DESC,id DESC LIMIT 100").fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, at, authority, family, record, binding)| DisplayRecord {
                name: "IMMUTABLE_BINDING".into(),
                fields: vec![
                    ("Alarm".into(), id),
                    ("UTC".into(), at),
                    ("Authority".into(), authority),
                    ("Family".into(), family),
                    ("Record".into(), record),
                    ("Binding".into(), binding),
                ],
            },
        )
        .collect())
}
