#![allow(clippy::unwrap_used)]

use crate::operational_mail::{Audience, Draft};

#[test]
fn accepts_only_defined_audiences_and_valid_bilingual_mail() {
    for value in ["", "EN", "pl,en", "world:test", "a@example.test"] {
        assert!(value.parse::<Audience>().is_err(), "{value}");
    }
    assert!("all".parse::<Audience>().is_ok());
    assert!("pl".parse::<Audience>().is_ok());
    assert!("en".parse::<Audience>().is_ok());

    let mut draft = Draft {
        audience: Audience::All,
        subject_pl: "Przerwa techniczna".into(),
        body_pl: "Portal wróci rano.".into(),
        subject_en: "Maintenance".into(),
        body_en: "The portal will return in the morning.".into(),
    };
    assert!(draft.validate().is_ok());
    draft.subject_en = "Maintenance\r\nAnother header".into();
    assert!(draft.validate().is_err());
    draft.subject_en = "Maintenance".into();
    draft.body_pl = "   ".into();
    assert!(draft.validate().is_err());
    draft.subject_pl = "ę".repeat(200);
    draft.body_pl = "ą".repeat(4000);
    assert!(draft.validate().is_ok());
    draft.body_pl.push('ą');
    assert!(draft.validate().is_err());
}

use crate::{
    operational_mail::{Decision, Error, preview, review},
    rbac::{self, Authorizer, CheckContext, MutationAuthorization},
    web::PortalIdentity,
};
use oteryn_db::outbox;
use oteryn_identity::sessions::NotificationAccounts;
use oteryn_notify::{CaptureSender, Category, Notifications, Unsubscribe};
use sqlx::{
    AssertSqlSafe, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::sync::Arc;
use uuid::Uuid;

async fn account(pool: &PgPool, email: &str, lang: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,locale,status,email_verified_at)
         VALUES($1,'fixture',$2,'active',now()) RETURNING id",
    )
    .bind(email)
    .bind(lang)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn staff(pool: &PgPool, name: &str, permissions: &[&str]) -> CheckContext {
    let account = account(pool, &format!("{name}@example.test"), "en").await;
    let role: Uuid = sqlx::query_scalar("INSERT INTO roles(key) VALUES($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(account)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT $1,unnest($2::text[])")
        .bind(role)
        .bind(permissions)
        .execute(pool)
        .await
        .unwrap();
    CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(pool, account).await.unwrap()),
    }
}

async fn preference(notifications: &Notifications, account: Uuid, email: bool) {
    let mut tx = notifications.pool.begin().await.unwrap();
    notifications
        .preferences(&mut tx, account, Category::Operational, email, false)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn campaign_requires_independent_review_and_current_consent() {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await.unwrap() else {
        return;
    };
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let root = PgPool::connect(&url).await.unwrap();
    let database = format!("operational_mail_{}", Uuid::now_v7().simple());
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

    exercise_campaign(&pool).await;
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {database}")))
        .execute(&root)
        .await
        .unwrap();
}

async fn exercise_campaign(pool: &PgPool) {
    let author = staff(
        pool,
        "campaign_author",
        &[
            "admin.access",
            "mail.operational.compose",
            "mail.operational.review",
        ],
    )
    .await;
    let reviewer = staff(
        pool,
        "campaign_reviewer",
        &["admin.access", "mail.operational.review"],
    )
    .await;
    let viewer = staff(pool, "campaign_viewer", &["admin.access"]).await;
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured_from(pool.clone(), &|_| None),
    )));
    let unsubscribe =
        Unsubscribe::new(vec![7; 32], "https://example.test/pl/unsubscribe".into()).unwrap();
    let notifications = Notifications {
        pool: pool.clone(),
        accounts: Arc::new(NotificationAccounts(pool.clone())),
        unsubscribe: unsubscribe.clone(),
    };
    let ok = account(pool, "recipient@example.test", "pl").await;
    sqlx::query("UPDATE accounts SET mail_locale='en' WHERE id=$1")
        .bind(ok)
        .execute(pool)
        .await
        .unwrap();
    let before = account(pool, "leave-before-review@example.test", "en").await;
    let after = account(pool, "leave-after-review@example.test", "en").await;
    let paused = account(pool, "paused-at-dispatch@example.test", "en").await;
    let late = account(pool, "join-after-preview@example.test", "en").await;
    let other_locale = account(pool, "other-language@example.test", "pl").await;
    assert_eq!(
        notifications
            .preference(late, Category::Operational)
            .await
            .unwrap(),
        (false, false)
    );
    for id in [ok, before, after, paused, other_locale] {
        preference(&notifications, id, true).await;
    }

    let draft = Draft {
        audience: Audience::En,
        subject_pl: "Przerwa techniczna".into(),
        body_pl: "Portal wróci rano.".into(),
        subject_en: "Maintenance & events".into(),
        body_en: "The portal reopens in the morning. <maintenance>".into(),
    };
    let campaign = Uuid::now_v7();
    assert!(matches!(
        preview(pool, &auth, viewer, campaign, &draft).await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    assert!(matches!(
        preview(pool, &auth, author, Uuid::nil(), &draft).await,
        Err(Error::Invalid)
    ));
    preview(pool, &auth, author, campaign, &draft)
        .await
        .unwrap();
    preference(&notifications, late, true).await;
    preference(&notifications, before, false).await;
    preference(&notifications, before, true).await;
    preview(pool, &auth, author, campaign, &draft)
        .await
        .unwrap();

    let audited: i64 = sqlx::query_scalar(
        "SELECT (after->>'eligible')::bigint FROM audit_log WHERE target=$1 AND action='mail.operational.previewed'",
    ).bind(campaign.to_string()).fetch_one(pool).await.unwrap();
    assert_eq!(audited, 4);
    for actor in [viewer, author] {
        assert!(matches!(
            review(
                pool,
                &auth,
                actor,
                campaign,
                Decision::Approve,
                Some(&unsubscribe)
            )
            .await,
            Err(Error::Authorization(rbac::Error::Denied))
        ));
    }
    assert!(matches!(
        review(pool, &auth, reviewer, campaign, Decision::Approve, None).await,
        Err(Error::Unavailable)
    ));

    let (first, second) = tokio::join!(
        review(
            pool,
            &auth,
            reviewer,
            campaign,
            Decision::Approve,
            Some(&unsubscribe)
        ),
        review(
            pool,
            &auth,
            reviewer,
            campaign,
            Decision::Approve,
            Some(&unsubscribe)
        ),
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(()))
    ));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=ANY($1)")
        .bind([after, paused].as_slice())
        .execute(pool)
        .await
        .unwrap();
    let token = unsubscribe
        .token(after, Category::Operational, i64::MAX)
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    notifications.unsubscribe(&mut tx, &token).await.unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(after)
        .execute(pool)
        .await
        .unwrap();

    let sender = CaptureSender::default();
    let jobs = outbox::claim(pool, 10, 30).await.unwrap();
    assert_eq!(jobs.len(), 3);
    for job in &jobs {
        let (first, second) = tokio::join!(
            notifications.deliver(job, &sender),
            notifications.deliver(job, &sender),
        );
        first.unwrap();
        second.unwrap();
        assert!(outbox::finish(pool, job, None, false).await.unwrap());
    }
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(paused)
        .execute(pool)
        .await
        .unwrap();
    for job in &jobs {
        notifications.deliver(job, &sender).await.unwrap();
    }
    {
        let sent = sender.emails.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].recipient, "recipient@example.test");
        assert_eq!(sent[0].subject, draft.subject_en);
        assert!(sent[0].html.contains("&#60;maintenance&#62;"));
        assert!(
            sent[0]
                .text
                .contains("https://example.test/en/unsubscribe?token=")
        );
    }
    let outcomes: (i64, i64) = sqlx::query_as(
        "SELECT count(e.delivered_at), count(r.skipped_at)
         FROM operational_mail_recipients r LEFT JOIN email_outbox e ON e.id=r.mail_id
         WHERE r.campaign_id=$1",
    )
    .bind(campaign)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(outcomes, (1, 2));

    let rejected = Uuid::now_v7();
    preview(pool, &auth, author, rejected, &draft)
        .await
        .unwrap();
    review(pool, &auth, reviewer, rejected, Decision::Reject, None)
        .await
        .unwrap();
    assert!(matches!(
        review(
            pool,
            &auth,
            reviewer,
            rejected,
            Decision::Approve,
            Some(&unsubscribe)
        )
        .await,
        Err(Error::Conflict)
    ));

    let expired = Uuid::now_v7();
    preview(pool, &auth, author, expired, &draft).await.unwrap();
    sqlx::query(
        "UPDATE operational_mail_campaigns
         SET created_at=now()-interval '25 hours', expires_at=now()-interval '1 hour' WHERE id=$1",
    )
    .bind(expired)
    .execute(pool)
    .await
    .unwrap();
    assert!(matches!(
        review(pool, &auth, reviewer, expired, Decision::Reject, None).await,
        Err(Error::Conflict)
    ));

    sqlx::query(
        "WITH added AS (
            INSERT INTO accounts(email,password_hash,locale,status,email_verified_at)
            SELECT 'participant-'||n||'@example.test','fixture','en','active',now()
            FROM generate_series(1,1001) n RETURNING id
         )
         INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled)
         SELECT id,'operational',true,false FROM added",
    )
    .execute(pool)
    .await
    .unwrap();
    let oversized = Uuid::now_v7();
    assert!(matches!(
        preview(pool, &auth, author, oversized, &draft).await,
        Err(Error::AudienceLimit)
    ));
    let rolled_back: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM operational_mail_campaigns WHERE id=$1)",
    )
    .bind(oversized)
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(rolled_back);
}
