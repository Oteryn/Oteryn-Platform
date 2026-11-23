#![allow(clippy::unwrap_used)]

use super::{Filters, store};
use crate::{
    rbac::{Authorizer, CheckContext, MutationAuthorization},
    web::PortalIdentity,
};
use sqlx::{
    AssertSqlSafe, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn validates_filter_fields_and_calendar_range() {
    for raw in [
        "state=done",
        "kind=commerce.refund",
        "from=2026-02-29",
        "from=2026-12-01&to=2026-11-30",
        "from=2026-1-01",
        "recipient=look%20up%40example.test",
        "recipient=example.test",
    ] {
        let mut filters = serde_urlencoded::from_str::<Filters>(raw).unwrap();
        assert!(filters.validate().is_err(), "{raw}");
    }
    let mut valid = serde_urlencoded::from_str::<Filters>(
        "state=failed&kind=notify.news&from=2028-02-29&to=2028-02-29&recipient=Player%40Example.Test",
    )
    .unwrap();
    valid.validate().unwrap();
    assert_eq!(valid.recipient, "player@example.test");
}

#[tokio::test]
async fn delivery_operations_respect_state_scope_and_batch_size() {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await.unwrap() else {
        return;
    };
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let root = PgPool::connect(&url).await.unwrap();
    let database = format!("mail_ops_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&root)
        .await
        .unwrap();
    let options: PgConnectOptions = url.parse().unwrap();
    let pool = PgPoolOptions::new()
        .connect_with(options.database(&database))
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let actor: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,locale,status,email_verified_at)
         VALUES('operator@example.test','fixture','en','active',now()) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('mail_operator') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO role_permissions(role_id,permission) VALUES
        ($1,'admin.access'),($1,'mail.outbox.view'),($1,'mail.outbox.retry'),($1,'mail.outbox.retry_batch')",
    )
    .bind(role)
    .execute(&pool)
    .await
    .unwrap();
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(&pool, actor).await.unwrap()),
    };
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured_from(pool.clone(), &|_| None),
    )));
    sqlx::query(
        "INSERT INTO outbox(kind,payload,idempotency_key,attempts,last_error,created_at)
         SELECT 'email.account_exists',jsonb_build_object('email','recipient@example.test'), 'fixture-' || n,
             2,'verification delivery failed',now() - make_interval(hours => n)
         FROM generate_series(1,55) n",
    )
    .execute(&pool)
    .await
    .unwrap();
    let pending: Uuid = sqlx::query_scalar(
        "INSERT INTO outbox(kind,payload,idempotency_key) VALUES('email.verify','{}','pending') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let counts = store::counts(&pool).await.unwrap();
    let failed = counts.iter().find(|row| row.state == "failed").unwrap();
    assert_eq!((failed.day, failed.week), (23, 55));
    assert_eq!(
        counts.iter().find(|row| row.state == "sent").unwrap().week,
        0
    );
    let filters = Filters {
        recipient: "recipient@example.test".into(),
        state: "failed".into(),
        ..Filters::default()
    };
    let masked = store::list(&pool, &filters, false).await.unwrap();
    assert_eq!(masked.len(), store::PAGE_SIZE + 1);
    assert!(masked[0].has_recipient);
    assert_eq!(masked[0].recipient, None);
    assert_eq!(
        store::list(&pool, &filters, true).await.unwrap()[0]
            .recipient
            .as_deref(),
        Some("recipient@example.test")
    );

    assert!(matches!(
        store::apply(&pool, context, &auth, store::Action::Cancel(pending)).await,
        Err(super::Error::Authorization(crate::rbac::Error::Denied))
    ));
    assert!(matches!(
        store::apply(&pool, context, &auth, store::Action::Retry(pending)).await,
        Err(super::Error::Conflict)
    ));
    assert!(matches!(
        store::apply(
            &pool,
            context,
            &auth,
            store::Action::RetryKind("commerce.refund")
        )
        .await,
        Err(super::Error::Invalid)
    ));
    let count = store::apply(
        &pool,
        context,
        &auth,
        store::Action::RetryKind("email.account_exists"),
    )
    .await
    .unwrap();
    assert_eq!(count, store::BATCH_LIMIT);
    let audited: (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(DISTINCT request_id) FROM audit_log
         WHERE actor=$1 AND action='mail.outbox.batch_retried' AND after->>'attempts'='2'",
    )
    .bind(actor)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, (i64::from(count), 1));
    let last = store::list(&pool, &filters, false).await.unwrap();
    assert_eq!(last.len(), 5);
    assert_eq!(
        store::apply(&pool, context, &auth, store::Action::Retry(last[0].id))
            .await
            .unwrap(),
        1
    );

    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'mail.outbox.cancel')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        store::apply(&pool, context, &auth, store::Action::Cancel(pending))
            .await
            .unwrap(),
        1
    );
    let state: String = sqlx::query_scalar("SELECT state FROM admin_mail_outbox WHERE id=$1")
        .bind(pending)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "cancelled");
    assert!(matches!(
        store::apply(&pool, context, &auth, store::Action::Cancel(pending)).await,
        Err(super::Error::Conflict)
    ));

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {database}")))
        .execute(&root)
        .await
        .unwrap();
}
