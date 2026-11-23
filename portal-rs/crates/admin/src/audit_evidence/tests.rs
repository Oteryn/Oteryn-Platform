#![allow(clippy::unwrap_used)]

use super::*;
use crate::{
    audit::control_tests::{close, database},
    rbac::Permission,
};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

struct Auditor(bool);

impl rbac::MutationCheck for Auditor {
    fn check<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: CheckContext,
        permission: Permission,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), rbac::Error>> + Send + 'a>> {
        Box::pin(async move {
            if self.0 || permission == Permission::Access {
                Ok(())
            } else {
                Err(rbac::Error::Denied)
            }
        })
    }
}

#[test]
fn canonical_calendar_day_and_form() {
    assert!(parse_day("2028-02-29").is_ok());
    for value in [
        "2026-02-29",
        "2026-1-01",
        "20261123",
        "2026-11-23 ",
        "0000-01-01",
    ] {
        assert!(matches!(parse_day(value), Err(Error::Invalid)), "{value}");
    }
    assert!(serde_urlencoded::from_str::<ExportForm>("day=2026-11-23&day=2026-11-24").is_err());
    assert!(serde_urlencoded::from_str::<ExportForm>("day=2026-11-23&actor=someone").is_err());
}

#[tokio::test]
async fn daily_evidence_lifecycle() {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await.unwrap() else {
        return;
    };
    let Some((pool, root, schema)) = database().await else {
        return;
    };
    let (day, second_day, today, expired): (String, String, String, String) = sqlx::query_as(
        "SELECT (utc-1)::text, (utc-2)::text, utc::text, (utc-91)::text
         FROM (SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date AS utc) dates",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let account = Uuid::now_v7();
    let actor = CheckContext::new(account);
    let request = Uuid::now_v7();
    let sql = "INSERT INTO audit_log (actor, action, target, before, after, request_id, created_at)
               VALUES ($1, 'test.action', 'test-target', '{\"private\":true}', '{}', $2,
                       $3::text::date::timestamp AT TIME ZONE 'UTC')
               RETURNING evidence_day::text";
    let assigned: String = sqlx::query_scalar(sql)
        .bind(account)
        .bind(request)
        .bind(&day)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(assigned, day);
    assert!(matches!(
        store::export(&pool, &Auditor(false), actor, &day).await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    for invalid_day in [&today, &expired] {
        assert!(matches!(
            store::export(&pool, &Auditor(true), actor, invalid_day).await,
            Err(Error::OutsideWindow)
        ));
    }
    let first = store::export(&pool, &Auditor(true), actor, &day)
        .await
        .unwrap();
    let evidence: Value = serde_json::from_slice(&first).unwrap();
    let entry = &evidence["entries"][0]["record"];
    assert_eq!(entry["actor"], account.to_string());
    assert_eq!(entry["request_id"], request.to_string());
    assert_eq!(entry["actor_kind"], "account");
    assert_eq!(entry["action"], "test.action");
    assert!(entry.get("before").is_none());
    assert_eq!(entry["before_digest"].as_str().unwrap().len(), 64);
    let assigned: String = sqlx::query_scalar(sql)
        .bind(account)
        .bind(Uuid::now_v7())
        .bind(&day)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(assigned, today);
    let second = store::export(&pool, &Auditor(true), actor, &day)
        .await
        .unwrap();
    assert_eq!(first, second);

    sqlx::query_scalar::<_, String>(sql)
        .bind(account)
        .bind(Uuid::nil())
        .bind(&second_day)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store::export(&pool, &Auditor(true), actor, &second_day).await,
        Err(Error::Integrity)
    ));
    close(pool, root, schema).await;
}

#[tokio::test]
async fn daily_export_requires_a_matching_seal() {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await.unwrap() else {
        return;
    };
    let Some((pool, root, schema)) = database().await else {
        return;
    };
    let day: String = sqlx::query_scalar(
        "INSERT INTO audit_evidence_seals (day, head, entry_count)
         VALUES ((clock_timestamp() AT TIME ZONE 'UTC')::date - 1, sha256('fixture'::bytea), 2)
         RETURNING day::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(matches!(
        store::export(
            &pool,
            &Auditor(true),
            CheckContext::new(Uuid::now_v7()),
            &day
        )
        .await,
        Err(Error::Integrity)
    ));
    close(pool, root, schema).await;
}
