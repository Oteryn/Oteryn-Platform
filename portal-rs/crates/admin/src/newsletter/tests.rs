use super::*;
use crate::web::PortalIdentity;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[tokio::test]
async fn campaign_checks_state_and_permission_and_queues_each_batch_once() -> TestResult {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await? else {
        eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("newsletter_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let namespace = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let namespace = namespace.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path', $1, false)")
                    .bind(namespace)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let result = exercise(&pool).await;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    result
}

async fn exercise(pool: &PgPool) -> TestResult {
    let password = "newsletter fixture password";
    let actor: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts (email, password_hash, status, locale, email_verified_at)
         VALUES ('newsletter-admin@example.invalid', $1, 'active', 'en', now())
         RETURNING id",
    )
    .bind(oteryn_identity::password::hash(password)?)
    .fetch_one(pool)
    .await?;
    let proof = crate::test_staff::proof(pool, actor).await?;
    let staff = Staff {
        context: CheckContext {
            account: actor,
            session_proof: Some(proof),
            mfa_verified: true,
            require_staff_mfa: true,
        },
        authorization: MutationAuthorization::new(Arc::new(rbac::Authorizer::new(
            pool.clone(),
            PortalIdentity::configured_from(pool.clone(), &|_| None),
        ))),
        request_id: Uuid::now_v7(),
    };
    let service = Newsletters {
        pool: pool.clone(),
        mailer: Some(Mailer::new(
            vec![17; 32],
            "https://example.invalid/en/unsubscribe".into(),
        )?),
    };
    let draft = || Draft {
        _csrf: String::new(),
        audience: Audience::Pl,
        subject_pl: "Wieści".into(),
        body_pl: "Witaj **podróżniku**!".into(),
        subject_en: "News".into(),
        body_en: "Hello **traveler**!".into(),
    };
    assert!(matches!(
        service.create(&staff, draft()).await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    grant_role(pool, actor, "content_editor").await?;
    let mut invalid = draft();
    invalid.subject_en = " ".into();
    assert!(matches!(
        service.create(&staff, invalid).await,
        Err(Error::Invalid)
    ));
    let id = service.create(&staff, draft()).await?;
    assert!(matches!(
        service.test(&staff, id, MailLanguage::En).await,
        Err(Error::SelfOptIn)
    ));
    sqlx::query(
        "INSERT INTO account_notification_prefs (account_id, category, email_enabled, portal_enabled)
         VALUES ($1, 'news', true, false)",
    )
    .bind(actor)
    .execute(pool)
    .await?;
    service.test(&staff, id, MailLanguage::En).await?;
    service.test(&staff, id, MailLanguage::En).await?;
    assert!(matches!(
        service.send(&staff, id, password.into()).await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    grant_role(pool, actor, "platform_admin").await?;
    assert!(matches!(
        service.send(&staff, id, "incorrect password".into()).await,
        Err(Error::RecentAuth)
    ));
    let untested = service.create(&staff, draft()).await?;
    assert!(matches!(
        service.send(&staff, untested, password.into()).await,
        Err(Error::State)
    ));

    sqlx::query(
        "WITH subscribers AS (
            INSERT INTO accounts (email, password_hash, status, locale, mail_locale, email_verified_at)
            SELECT 'newsletter-' || i || '@example.invalid', '!fixture', 'active', 'en', 'pl', now()
            FROM generate_series(1, $1::integer) i RETURNING id
         )
         INSERT INTO account_notification_prefs
            (account_id, category, email_enabled, portal_enabled)
         SELECT id, 'news', true, false FROM subscribers",
    )
    .bind(BATCH_SIZE as i32 + 1)
    .execute(pool)
    .await?;
    let approval = service.authorize_send(&staff, password.into()).await?;
    assert!(!service.batch(&staff, id, &approval).await?);
    let pending = Newsletters::load(&mut *pool.acquire().await?, id, false).await?;
    assert_eq!(pending.queued_count, BATCH_SIZE as i64);
    assert_eq!(pending.state(), "queuing");
    service.send(&staff, id, password.into()).await?;
    service.send(&staff, id, password.into()).await?;
    let done = Newsletters::load(&mut *pool.acquire().await?, id, false).await?;
    assert_eq!(done.queued_count, BATCH_SIZE as i64 + 1);
    assert_eq!(done.state(), "queued");
    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT subject, count(*) FROM email_outbox GROUP BY subject ORDER BY subject",
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(
        counts,
        vec![("News".into(), 1), ("Wieści".into(), BATCH_SIZE as i64 + 1)]
    );
    let audit: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_log WHERE target = $1 ORDER BY id")
            .bind(id.to_string())
            .fetch_all(pool)
            .await?;
    assert_eq!(
        audit,
        [
            "newsletter.campaign.saved",
            "newsletter.campaign.test_queued",
            "newsletter.campaign.batch_queued",
            "newsletter.campaign.batch_queued",
        ]
    );

    sqlx::query(
        "UPDATE account_notification_prefs SET email_enabled = false
         WHERE account_id = (SELECT id FROM accounts WHERE email = 'newsletter-1@example.invalid')",
    )
    .execute(pool)
    .await?;
    service.test(&staff, untested, MailLanguage::En).await?;
    let approval = service.authorize_send(&staff, password.into()).await?;
    assert!(service.batch(&staff, untested, &approval).await?);
    let exact = Newsletters::load(&mut *pool.acquire().await?, untested, false).await?;
    assert_eq!(exact.queued_count, BATCH_SIZE as i64);
    assert_eq!(exact.state(), "queued");

    let pending_test = service.create(&staff, draft()).await?;
    let mut writer = pool.begin().await?;
    Newsletters::load(&mut writer, pending_test, true).await?;
    sqlx::query(
        "INSERT INTO newsletter_campaign_tests (campaign_id, actor, locale) VALUES ($1, $2, 'en')",
    )
    .bind(pending_test)
    .bind(actor)
    .execute(&mut *writer)
    .await?;
    let mut reader = pool.begin().await?;
    let reader_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *reader)
        .await?;
    let commit_when_blocked = async move {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let blocked: bool =
                    sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1)) > 0")
                        .bind(reader_pid)
                        .fetch_one(pool)
                        .await?;
                if blocked {
                    return Ok::<_, sqlx::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await??;
        writer.commit().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (committed, campaign) = tokio::join!(
        commit_when_blocked,
        Newsletters::load(&mut reader, pending_test, true),
    );
    committed?;
    assert!(campaign?.tested);
    Ok(())
}

async fn grant_role(pool: &PgPool, actor: Uuid, role: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_roles (account_id, role_id) SELECT $1, id FROM roles WHERE key = $2",
    )
    .bind(actor)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(())
}
