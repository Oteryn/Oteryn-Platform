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
    if let Ok(records) = records(pool, lang).await {
        section.records = records;
        section.state = text(lang, "fresh");
        section.freshness = text(lang, "now");
    }
    section
}
async fn records(pool: &PgPool, lang: &str) -> Result<Vec<DisplayRecord>, sqlx::Error> {
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id::text,created_at::text,after->>'authority',after->>'family',after->>'record_id',target
         FROM audit_log
         WHERE actor IS NULL AND action='ops.playerdata_binding_conflict'
           AND created_at>clock_timestamp()-interval '7 days'
           AND jsonb_typeof(after->'authority')='string'
           AND jsonb_typeof(after->'family')='string'
           AND jsonb_typeof(after->'record_id')='string'
         ORDER BY created_at DESC,id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, at, authority, family, record, binding)| DisplayRecord {
                name: "IMMUTABLE_BINDING".into(),
                fields: vec![
                    (text(lang, "alert-alarm"), id),
                    (text(lang, "alert-utc"), at),
                    (text(lang, "alert-authority"), authority),
                    (text(lang, "alert-family"), family),
                    (text(lang, "alert-record"), record),
                    (text(lang, "alert-binding"), binding),
                ],
            },
        )
        .collect())
}
