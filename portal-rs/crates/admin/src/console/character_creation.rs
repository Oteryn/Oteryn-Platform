//! Aggregate Platform command status for the existing authorized ops dashboard.
use super::{DisplayRecord, Section, text};
use oteryn_characters::create_web;
use sqlx::PgPool;

pub(super) async fn section(pool: &PgPool, lang: &str) -> Section {
    let t = |key| text(lang, key);
    let browser = create_web::Settings::configured();
    let exchange_configured = matches!(
        std::env::var("OTERYN_ENV").as_deref(),
        Ok("testing" | "preprod")
    ) && std::env::var("PORTAL_CHARACTER_COMMANDS_ENABLED").as_deref()
        == Ok("true")
        && std::env::var("PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY").as_deref()
            == Ok(browser.commands.source_authority.as_str())
        && browser.available();
    let availability = |configured| {
        t(if configured {
            "creation-configured"
        } else {
            "unavailable"
        })
    };
    let mut fields = vec![
        (t("creation-issuance"), availability(browser.available())),
        (t("creation-exchange"), availability(exchange_configured)),
    ];
    let stats = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT
            count(*) FILTER (WHERE state='PENDING'),
            count(*) FILTER (WHERE state='PENDING'
                AND (intent_json::jsonb->>'expires_at_source')::bigint
                    <= extract(epoch FROM statement_timestamp())),
            count(*) FILTER (WHERE receipt_json::jsonb->>'outcome'='COMMITTED'),
            count(*) FILTER (WHERE receipt_json::jsonb->>'outcome'='REJECTED')
         FROM character_commands
         WHERE command='CreateCharacter'
           AND (state='PENDING' OR updated_at >= statement_timestamp()-interval '24 hours')",
    )
    .fetch_one(pool)
    .await;
    if let Ok((pending, expired, committed, rejected)) = stats {
        fields.extend([
            (t("creation-pending"), pending.to_string()),
            (t("creation-expired"), expired.to_string()),
            (t("creation-committed"), committed.to_string()),
            (t("creation-rejected"), rejected.to_string()),
        ]);
    }
    Section {
        title: t("creation-status"),
        source: t("platform-source"),
        state: t(if stats.is_ok() {
            "fresh"
        } else {
            "unavailable"
        }),
        freshness: t(if stats.is_ok() { "now" } else { "unknown" }),
        revision: String::new(),
        records: vec![DisplayRecord {
            name: t("creation-status"),
            fields,
        }],
        operation: "",
        can_query: false,
    }
}
