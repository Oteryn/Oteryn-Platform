use std::ffi::OsString;

use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use uuid::Uuid;

use super::*;

#[test]
fn system_language_honors_preference_order_and_cookie() {
    use axum::http::{HeaderValue, header};
    let mut headers = HeaderMap::new();
    for (value, expected) in [
        ("de-DE, pl;q=0.3, EN-GB;q=0.8", "en"),
        ("pl;q=0.9, en;q=0.9", "pl"),
        ("en;q=0, de", "pl"),
    ] {
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static(value));
        assert_eq!(language(&headers, None), expected);
    }
    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("portal_language=en"),
    );
    assert_eq!(language(&headers, None), "en");
}

#[test]
fn flags_are_strict_booleans_and_labels_exist_in_both_locales() {
    for lang in ["en", "pl"] {
        let locale = Locale::new(lang);
        let flags = registry::flags(&locale, |name| match name {
            "OTERYN_ADMIN_PANEL" => Some("true".into()),
            "OTERYN_ACCOUNT_LOGIN" => Some("TRUE".into()),
            "OTERYN_ACCOUNT_MFA" => Some("false".into()),
            "OTERYN_ACCOUNT_SESSIONS" => Some("1".into()),
            "OTERYN_ACCOUNT_PASSKEYS" => Some("true ".into()),
            _ => None,
        });
        assert!(flags.iter().all(|flag| !flag.description.is_empty()));
        let enabled: Vec<_> = flags
            .iter()
            .filter(|flag| flag.enabled)
            .map(|flag| flag.name)
            .collect();
        assert_eq!(enabled, ["OTERYN_ADMIN_PANEL"]);
        assert!(flags.windows(2).all(|pair| pair[0].name < pair[1].name));
    }
}

#[test]
fn flag_values_account_for_legacy_aliases() {
    let locale = Locale::new("en");
    let flags = registry::flags(&locale, |name| match name {
        "PORTAL_ATLAS_DELIVERY_ENABLED" | "PORTAL_WORLD_ENABLED" => Some("1".into()),
        "OTERYN_WORLD_ENABLED" => Some("false".into()),
        _ => None,
    });
    for (name, enabled) in [
        ("OTERYN_ATLAS_DELIVERY_ENABLED", true),
        ("OTERYN_WORLD_ENABLED", false),
    ] {
        assert_eq!(
            flags
                .iter()
                .find(|flag| flag.name == name)
                .map(|flag| flag.enabled),
            Some(enabled)
        );
    }
}

#[test]
fn configuration_reports_presence_even_when_a_value_looks_boolean() {
    let lookup = |key: &str| match key {
        "OTERYN_DATABASE_URL" => Some(OsString::from("synthetic-value")),
        "OTERYN_SMTP_PASSWORD" => Some(OsString::from("true")),
        _ => None,
    };
    let settings = registry::configuration(|key| lookup(key).is_some());
    let encoded = serde_json::json!(settings);
    let rendered = encoded.to_string();
    assert!(!rendered.contains("synthetic-value"));
    assert!(!rendered.contains("\"true\""));
    for name in ["OTERYN_DATABASE_URL", "OTERYN_SMTP_PASSWORD"] {
        assert!(encoded.as_array().is_some_and(|rows| {
            rows.contains(&serde_json::json!({ "name": name, "state": "set" }))
        }));
    }
}

#[tokio::test]
async fn database_probes_cover_fresh_stale_absent_and_timed_out_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("sysops_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path', $1, false)")
                    .bind(search_path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;

    assert!(matches!(probes::postgres(&pool).await, Probe::Ok(_)));
    assert!(matches!(probes::worker(&pool).await, Probe::Unknown));
    assert!(matches!(probes::gateway(&pool).await, Probe::Unknown));
    assert!(matches!(probes::game_link(&pool).await, Probe::Unknown));
    assert!(matches!(
        probes::migrations(&pool).await,
        Probe::Mismatch(_)
    ));
    oteryn_db::migrate(&pool).await?;
    assert!(matches!(probes::migrations(&pool).await, Probe::Ok(_)));

    probes::record_worker_heartbeat(&pool).await?;
    assert!(matches!(probes::worker(&pool).await, Probe::Ok(0..=5)));
    sqlx::query(
        "UPDATE portal_worker_heartbeat SET seen_at = clock_timestamp() - interval '1 minute'",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(probes::worker(&pool).await, Probe::Stale(60..)));
    probes::record_worker_heartbeat(&pool).await?;
    assert!(matches!(probes::worker(&pool).await, Probe::Ok(0..=5)));

    sqlx::query(
        "UPDATE portal_worker_heartbeat SET seen_at = clock_timestamp() + interval '1 minute'",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(probes::worker(&pool).await, Probe::Stale(..=-1)));
    probes::record_worker_heartbeat(&pool).await?;
    assert!(matches!(probes::worker(&pool).await, Probe::Ok(0..=5)));

    sqlx::query(
        "INSERT INTO native_admissions
         (attempt_ref, ticket_hash, request_digest, account_id, response_body, expires_at_ms, created_at)
         VALUES ($1, $2, $2, $3, $4, 0, $5::timestamptz)",
    )
    .bind(Uuid::now_v7())
    .bind([1_u8; 32].as_slice())
    .bind(Uuid::now_v7())
    .bind(b"{}".as_slice())
    .bind("2026-11-23T12:34:56Z")
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO admin_control_agents
         (agent_id, build, catalog_digest, advertised_operations, served_worlds, served_nodes, owner_epoch, last_seen_at)
         VALUES ('test', 'test', $1, '[]', '{}', '{}', 1, $2::timestamptz)",
    )
    .bind([1_u8; 32].as_slice())
    .bind("2026-11-23T12:35:00Z")
    .execute(&pool)
    .await?;
    assert!(matches!(
        probes::gateway(&pool).await,
        Probe::Observed(stamp) if stamp == "2026-11-23T12:34:56Z"
    ));
    assert!(matches!(
        probes::game_link(&pool).await,
        Probe::Observed(stamp) if stamp == "2026-11-23T12:35:00Z"
    ));

    sqlx::query(
        "WITH mail AS (
             INSERT INTO email_outbox(account_id, category, recipient, subject, text_body, html_body)
             SELECT uuidv7(), 'security', state || '@example.invalid', 'Status', 'Status', 'Status'
             FROM unnest(ARRAY['pending', 'dead', 'orphan']) AS state
             RETURNING id, recipient
         )
         INSERT INTO outbox(kind, payload, idempotency_key, state)
         SELECT 'notify.email', to_jsonb(id::text), id::text, split_part(recipient, '@', 1)
         FROM mail WHERE recipient <> 'orphan@example.invalid'",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(probes::mail_outbox(&pool).await, Probe::Ok(1)));
    sqlx::query("DELETE FROM outbox WHERE kind='notify.email'")
        .execute(&pool)
        .await?;

    let status = Status {
        db: pool.clone(),
        observations: Mutex::new(None),
        started_at: Instant::now(),
        environment: None,
    };
    let (first, second) = tokio::join!(status.observations(), status.observations());
    assert!(Arc::ptr_eq(&first, &second));
    *status.observations.lock().await =
        Some((Instant::now() - PROBE_REFRESH_INTERVAL, first.clone()));
    assert!(!Arc::ptr_eq(&first, &status.observations().await));

    assert!(matches!(probes::mail_outbox(&pool).await, Probe::Ok(0)));
    let mut writer = pool.begin().await?;
    sqlx::query("LOCK TABLE outbox IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await?;
    assert!(matches!(probes::outbox(&pool).await, Probe::Unavailable));
    assert!(matches!(probes::postgres(&pool).await, Probe::Ok(_)));
    writer.rollback().await?;

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
