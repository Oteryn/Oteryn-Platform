#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool, Row, postgres::PgPoolOptions};

async fn database() -> Option<(PgPool, PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL audit test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("audit_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(path)).execute(conn).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    Some((pool, root, schema))
}
async fn close(pool: PgPool, root: PgPool, schema: String) {
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
fn record() -> AdminControlAudit {
    AdminControlAudit {
        context: AuditContext {
            actor: Some(Uuid::now_v7()),
            request_id: Uuid::now_v7(),
        },
        admin_operation_id: Uuid::now_v7(),
        operation: "player.character.get".into(),
        risk_class: RiskClass::R1,
        target: json!({"character_id":"test-character"}),
        reason: Some(
            json!({"code":"INVESTIGATION", "sql":"SELECT character_id FROM admin_read.characters"}),
        ),
        approvals: json!([]),
        ip: Some("192.0.2.10".into()),
        user_agent: Some("audit-test".into()),
        correlation_id: Uuid::now_v7(),
        outcome: AuditOutcome::Succeeded,
        game_ledger_id: Some("test-ledger".into()),
    }
}
const WRITER: AdminControlAuditWriter = AdminControlAuditWriter { enabled: true };

#[tokio::test]
async fn db_control_round_trip_and_legacy_compatibility() {
    let Some((pool, root, schema)) = database().await else {
        return;
    };
    let evidence = record();
    let mut tx = pool.begin().await.unwrap();
    let id = WRITER.append(&mut tx, evidence.clone()).await.unwrap();
    append(
        &mut tx,
        Entry {
            context: evidence.context,
            action: "legacy.test",
            target: "legacy-target",
            before: Value::Null,
            after: json!({"ok":true}),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let row = sqlx::query("SELECT * FROM audit_log WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<Uuid, _>("actor"), evidence.context.actor.unwrap());
    assert_eq!(
        row.get::<Uuid, _>("request_id"),
        evidence.context.request_id
    );
    assert_eq!(
        row.get::<Uuid, _>("admin_operation_id"),
        evidence.admin_operation_id
    );
    assert_eq!(
        row.get::<Uuid, _>("correlation_id"),
        evidence.correlation_id
    );
    assert_eq!(row.get::<String, _>("operation"), evidence.operation);
    assert_eq!(row.get::<String, _>("risk_class"), "R1");
    assert_eq!(row.get::<Value, _>("admin_control_target"), evidence.target);
    assert_eq!(row.get::<Option<Value>, _>("reason"), evidence.reason);
    assert_eq!(row.get::<Value, _>("approvals"), evidence.approvals);
    assert_eq!(row.get::<Option<String>, _>("ip"), evidence.ip);
    assert_eq!(
        row.get::<Option<String>, _>("user_agent"),
        evidence.user_agent
    );
    assert_eq!(row.get::<String, _>("outcome"), "succeeded");
    assert_eq!(
        row.get::<Option<String>, _>("game_ledger_id"),
        evidence.game_ledger_id
    );
    assert!(
        sqlx::query("UPDATE audit_log SET outcome='failed' WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .is_err()
    );
    close(pool, root, schema).await;
}

#[tokio::test]
async fn db_read_audit_per_view_and_masking_defaults() {
    let Some((pool, root, schema)) = database().await else {
        return;
    };
    let mut tx = pool.begin().await.unwrap();
    for view in [
        ReadView::Dossier,
        ReadView::Inventory,
        ReadView::History,
        ReadView::Relink,
        ReadView::Export,
        ReadView::DatabaseSql,
        ReadView::AnalyticsSql,
    ] {
        let mut evidence = record();
        evidence.operation = view.operation().into();
        if matches!(view, ReadView::Export) {
            evidence.risk_class = RiskClass::R2;
        }
        evidence.target = json!({"character_id":"test", "email":"test@example.invalid", "nested":[{"ip":"192.0.2.2", "payment":{"reference":"test"}}]});
        let id = WRITER
            .read_audit(&mut tx, view, evidence.clone())
            .await
            .unwrap();
        let row = sqlx::query("SELECT admin_control_target,reason FROM audit_log WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(
            row.get::<Value, _>("admin_control_target"),
            json!({"character_id":"test", "email":"[masked]", "nested":[{"ip":"[masked]", "payment":"[masked]"}]})
        );
        assert_eq!(row.get::<Option<Value>, _>("reason"), evidence.reason);
        WRITER.read_audit(&mut tx, view, evidence).await.unwrap();
    }
    tx.commit().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 14);
    close(pool, root, schema).await;
}

#[tokio::test]
async fn db_disabled_refusal_bad_input_and_rollback() {
    let Some((pool, root, schema)) = database().await else {
        return;
    };
    let mut tx = pool.begin().await.unwrap();
    assert!(matches!(
        AdminControlAuditWriter::default()
            .append(&mut tx, record())
            .await,
        Err(crate::rbac::Error::Denied)
    ));
    for invalid in 0..7 {
        let mut evidence = record();
        match invalid {
            0 => evidence.context.actor = None,
            1 => evidence.context.request_id = Uuid::nil(),
            2 => evidence.target = Value::Null,
            3 => evidence.ip = Some("invalid".into()),
            4 => evidence.approvals = json!({}),
            5 => evidence.operation = "bad operation".into(),
            _ => evidence.user_agent = Some("bad\nagent".into()),
        }
        assert!(matches!(
            WRITER.append(&mut tx, evidence).await,
            Err(crate::rbac::Error::InvalidInput)
        ));
    }
    assert!(matches!(
        WRITER
            .read_audit(&mut tx, ReadView::Inventory, record())
            .await,
        Err(crate::rbac::Error::InvalidInput)
    ));
    let mut evidence = record();
    evidence.operation = ReadView::DatabaseSql.operation().into();
    evidence.reason = None;
    assert!(matches!(
        WRITER
            .read_audit(&mut tx, ReadView::DatabaseSql, evidence)
            .await,
        Err(crate::rbac::Error::InvalidInput)
    ));
    WRITER.append(&mut tx, record()).await.unwrap();
    tx.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    close(pool, root, schema).await;
}

#[test]
fn sensitive_field_masks_fail_closed() {
    for input in ["", "invalid", "test@example.invalid", "2001:db8::1"] {
        assert_eq!(mask_email(input), "[masked]");
        assert_eq!(mask_ip(input), "[masked]");
    }
    assert_eq!(mask_payment(&json!({"token":"test"})), json!("[masked]"));
    assert_eq!(
        mask_player_fields(
            &json!({"EmailAddress":"test", "last_ip":"test", "paymentDetails":{}, "card_number":"test", "safe":1})
        ),
        json!({"EmailAddress":"[masked]", "last_ip":"[masked]", "paymentDetails":"[masked]", "card_number":"[masked]", "safe":1})
    );
}
