#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use oteryn_db::outbox::{self, OffsetDateTime, Uuid};
use oteryn_notify::*;
use std::sync::Arc;

#[derive(Debug)]
struct Accounts;
impl AccountState for Accounts {
    fn active_verified(&self, id: Uuid) -> NotifyFuture<'_, Option<Account>> {
        Box::pin(async move {
            Ok((!id.is_nil()).then(|| Account {
                email: "reader@example.invalid".into(),
                locale: Locale::En,
            }))
        })
    }
}
fn signer() -> Unsubscribe {
    Unsubscribe::new(
        vec![42; 32],
        "https://portal.example.invalid/unsubscribe".into(),
    )
    .unwrap()
}
#[test]
fn templates_and_tokens() {
    let id = Uuid::now_v7();
    let signer = signer();
    let token = signer.token(id, Category::News, 100).unwrap();
    assert_eq!(signer.verify(&token, 99).unwrap(), (id, Category::News));
    assert!(signer.verify(&token, 100).is_err());
    assert!(signer.verify(&(token + "0"), 99).is_err());
    assert!(signer.token(id, Category::Security, 100).is_err());
    assert!(Unsubscribe::new(vec![0; 2], "http://invalid".into()).is_err());
    for locale in [Locale::Pl, Locale::En] {
        for template in [
            EmailTemplate::PasswordChanged,
            EmailTemplate::NewDevice {
                device: "<script>".into(),
            },
            EmailTemplate::News {
                title: "News".into(),
                body: "<script>".into(),
            },
            EmailTemplate::Support {
                body: "Hello".into(),
            },
        ] {
            let rendered = template.render(locale).unwrap();
            assert!(!rendered.html.contains("<script>"));
            assert!(!rendered.text.is_empty());
        }
    }
    assert!(
        EmailTemplate::News {
            title: "\r\nheader".into(),
            body: "ok".into()
        }
        .render(Locale::En)
        .is_err()
    );
}

struct Database {
    pool: outbox::PgPool,
    admin: outbox::PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = outbox::PgPoolOptions::new().connect(&url).await?;
        let schema = format!("t36_{}", Uuid::now_v7().simple());
        outbox::query(outbox::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = outbox::PgPoolOptions::new()
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    outbox::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    fn service(&self) -> Notifications {
        Notifications {
            pool: self.pool.clone(),
            accounts: Arc::new(Accounts),
            unsubscribe: signer(),
        }
    }
    async fn close(self) -> Result<()> {
        self.pool.close().await;
        outbox::query(outbox::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
#[tokio::test]
async fn transaction_preferences_security_and_read_state() -> Result<()> {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let account = Uuid::now_v7();
    let mut tx = db.pool.begin().await?;
    assert!(
        service
            .enqueue(&mut tx, Uuid::nil(), &EmailTemplate::PasswordChanged)
            .await
            .is_err()
    );
    assert!(
        service
            .preferences(&mut tx, account, Category::Security, false, true)
            .await
            .is_err()
    );
    service
        .enqueue(&mut tx, account, &EmailTemplate::PasswordChanged)
        .await?;
    tx.rollback().await?;
    assert_eq!(outbox::counts(&db.pool).await?.pending, 0);
    assert!(service.list(account, false, 10).await?.is_empty());
    let mut tx = db.pool.begin().await?;
    let news = EmailTemplate::News {
        title: "News".into(),
        body: "Hello".into(),
    };
    assert!(service.enqueue(&mut tx, account, &news).await?.is_none());
    service
        .preferences(&mut tx, account, Category::News, true, true)
        .await?;
    service.enqueue(&mut tx, account, &news).await?;
    service
        .enqueue(&mut tx, account, &EmailTemplate::PasswordChanged)
        .await?;
    tx.commit().await?;
    let jobs = outbox::claim(&db.pool, 10, 60).await?;
    let mut tx = db.pool.begin().await?;
    let token = signer().token(
        account,
        Category::News,
        OffsetDateTime::now_utc().unix_timestamp() + 60,
    )?;
    assert!(
        service
            .unsubscribe(&mut tx, &(token.clone() + "x"))
            .await
            .is_err()
    );
    service.unsubscribe(&mut tx, &token).await?;
    tx.commit().await?;
    let capture = CaptureSender::default();
    for job in &jobs {
        service.deliver(job, &capture).await?;
        service.deliver(job, &capture).await?;
        outbox::finish(&db.pool, job, None, false).await?;
    }
    let emails = capture.emails.lock().unwrap().clone();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].unsubscribe.is_none());
    let notifications = service.list(account, true, 10).await?;
    assert_eq!(notifications.len(), 3);
    assert!(
        !service
            .mark_read(Uuid::now_v7(), notifications[0].id, true)
            .await?
    );
    assert!(
        service
            .mark_read(account, notifications[0].id, true)
            .await?
    );
    assert_eq!(service.list(account, true, 10).await?.len(), 2);
    assert!(
        service
            .mark_read(account, notifications[0].id, false)
            .await?
    );
    assert!(service.list(account, false, 0).await.is_err());
    db.close().await
}
#[derive(Debug)]
struct Failure;
impl EmailSender for Failure {
    fn send<'a>(&'a self, _: &'a Email) -> NotifyFuture<'a, ()> {
        Box::pin(async { Err("temporary SMTP failure".into()) })
    }
}
#[tokio::test]
async fn delivery_retry_then_dead_and_deny() -> Result<()> {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let account = Uuid::now_v7();
    let mut tx = db.pool.begin().await?;
    service
        .enqueue(
            &mut tx,
            account,
            &EmailTemplate::NewDevice {
                device: "Browser".into(),
            },
        )
        .await?;
    tx.commit().await?;
    outbox::query("UPDATE outbox SET max_attempts=2")
        .execute(&db.pool)
        .await?;
    for attempt in 1..=2 {
        let job = outbox::claim(&db.pool, 1, 60).await?.remove(0);
        assert_eq!(job.attempts, attempt);
        assert!(service.deliver(&job, &Failure).await.is_err());
        let deny = Notifications {
            pool: db.pool.clone(),
            accounts: Arc::new(DenyAccounts),
            unsubscribe: signer(),
        };
        assert!(deny.deliver(&job, &CaptureSender::default()).await.is_err());
        assert!(outbox::finish(&db.pool, &job, Some("SMTP failure"), false).await?);
        if attempt == 1 {
            assert_eq!(outbox::counts(&db.pool).await?.pending, 1);
            outbox::query("UPDATE outbox SET run_after=now()")
                .execute(&db.pool)
                .await?;
        }
    }
    assert_eq!(outbox::counts(&db.pool).await?.dead, 1);
    let bad = outbox::Job {
        id: Uuid::now_v7(),
        kind: EMAIL_JOB.into(),
        payload: "invalid".into(),
        attempts: 1,
    };
    assert!(
        service
            .deliver(&bad, &CaptureSender::default())
            .await
            .is_err()
    );
    db.close().await
}

#[tokio::test]
async fn opted_in_email_delivery_and_expired_unsubscribe() -> Result<()> {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let account = Uuid::now_v7();
    assert_eq!(
        service.preference(account, Category::News).await?,
        (false, true)
    );
    assert_eq!(
        service.preference(account, Category::Security).await?,
        (true, true)
    );
    let mut tx = db.pool.begin().await?;
    service
        .preferences(&mut tx, account, Category::News, true, false)
        .await?;
    service
        .enqueue(
            &mut tx,
            account,
            &EmailTemplate::News {
                title: "News".into(),
                body: "Hello <friend>".into(),
            },
        )
        .await?;
    let expired = signer().token(account, Category::News, 1)?;
    assert!(service.unsubscribe(&mut tx, &expired).await.is_err());
    tx.commit().await?;
    let job = outbox::claim(&db.pool, 1, 60).await?.remove(0);
    let capture = CaptureSender::default();
    service.deliver(&job, &capture).await?;
    let email = capture.emails.lock().unwrap()[0].clone();
    assert_eq!(email.text, "News\n\nHello <friend>\n");
    assert!(!email.html.contains("<friend>"));
    assert!(email.html.contains("&#60;friend&#62;") || email.html.contains("&lt;friend&gt;"));
    let url = email.unsubscribe.unwrap();
    let token = url.split_once("?token=").unwrap().1;
    assert_eq!(
        signer().verify(token, OffsetDateTime::now_utc().unix_timestamp())?,
        (account, Category::News)
    );
    assert!(service.list(account, false, 10).await?.is_empty());
    outbox::finish(&db.pool, &job, None, false).await?;
    db.close().await
}
