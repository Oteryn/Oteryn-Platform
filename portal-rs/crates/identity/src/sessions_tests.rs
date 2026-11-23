#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "sessions_test_db.rs"]
mod database;
use super::*;
use oteryn_db::outbox;
use oteryn_notify::CaptureSender;

type TestResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
async fn account(pool: &PgPool) -> AccountId {
    sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES($1,'unused','active',now(),'en') RETURNING id")
        .bind(format!("{}@example.invalid", Uuid::now_v7())).fetch_one(pool).await.unwrap()
}
async fn browser(pool: &PgPool, account: AccountId, agent: &str) -> Actor {
    let hash = Sha256::digest(Uuid::now_v7().as_bytes()).to_vec();
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,repeat('a',64),now()+interval '1 hour',now()+interval '2 hours')")
        .bind(&hash).bind(account).bind(agent).execute(pool).await.unwrap();
    Actor { account, hash }
}
#[tokio::test]
async fn postgres_owner_revocation_history_and_expiry() -> TestResult {
    let Some(db) = database::Database::open()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let service = Sessions {
        pool: db.pool.clone(),
    };
    let owner = account(&db.pool).await;
    let other = account(&db.pool).await;
    let current = browser(&db.pool, owner, "Browser A").await;
    let sibling = browser(&db.pool, owner, "Browser B").await;
    let foreign = browser(&db.pool, other, "Foreign").await;
    let foreign_id = service.list(&foreign).await?.0[0].session_id;
    assert_eq!(service.list(&current).await?.0.len(), 2);
    assert_eq!(
        service
            .revoke(&current, Revocation::One(foreign_id))
            .await?,
        Outcome::Inactive
    );
    assert_eq!(service.list(&foreign).await?.0.len(), 1);
    assert_eq!(
        service.revoke(&current, Revocation::Others).await?,
        Outcome::Revoked
    );
    assert!(service.list(&sibling).await.is_err());
    let id = service.list(&current).await?.0[0].session_id;
    assert_eq!(
        service.revoke(&current, Revocation::One(id)).await?,
        Outcome::SignedOut
    );
    assert!(service.list(&current).await.is_err());
    let a = browser(&db.pool, owner, "A").await;
    let b = browser(&db.pool, owner, "B").await;
    assert_eq!(
        service.revoke(&a, Revocation::All).await?,
        Outcome::SignedOut
    );
    assert!(service.list(&b).await.is_err());
    let expired = browser(&db.pool, owner, "Expired").await;
    sqlx::query(
        "UPDATE web_sessions SET idle_expires_at=now()-interval '1 second' WHERE id_hash=$1",
    )
    .bind(&expired.hash)
    .execute(&db.pool)
    .await?;
    assert!(service.revoke(&expired, Revocation::All).await.is_err());
    let forged = Actor {
        account: other,
        hash: foreign.hash.clone(),
    };
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(other)
        .execute(&db.pool)
        .await?;
    assert!(service.list(&forged).await.is_err());
    db.close().await.map_err(|e| e.to_string())?;
    Ok(())
}
#[tokio::test]
async fn postgres_new_device_transaction_deduplication_and_delivery() -> TestResult {
    let Some(db) = database::Database::open()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let service = Sessions {
        pool: db.pool.clone(),
    };
    let owner = account(&db.pool).await;
    let first = browser(&db.pool, owner, "Browser <A>").await;
    service.record_login(&first).await?;
    service.record_login(&first).await?;
    let repeat = browser(&db.pool, owner, "Browser <A>").await;
    service.record_login(&repeat).await?;
    let new = browser(&db.pool, owner, "Browser B").await;
    let concurrent = browser(&db.pool, owner, "Browser B").await;
    let (a, b) = tokio::join!(
        service.record_login(&new),
        service.record_login(&concurrent)
    );
    a?;
    b?;
    assert_eq!(service.list(&first).await?.1.len(), 4);
    let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(queued, 2);
    let notifications = security_notifications(db.pool.clone())?;
    let sender = CaptureSender::default();
    for job in outbox::claim(&db.pool, 10, 60).await? {
        notifications.deliver(&job, &sender).await?;
    }
    assert_eq!(sender.emails.lock().unwrap().len(), 2);
    assert!(
        sender
            .emails
            .lock()
            .unwrap()
            .iter()
            .any(|email| email.html.contains("Browser &"))
    );
    assert!(
        sender
            .emails
            .lock()
            .unwrap()
            .iter()
            .all(|email| !email.html.contains("<A>"))
    );
    let blank = browser(&db.pool, owner, "").await;
    service.record_login(&blank).await?;
    let invalid = Actor {
        account: owner,
        hash: vec![0; 2],
    };
    assert!(service.record_login(&invalid).await.is_err());
    // An enqueue refusal rolls back both the login event and remembered device.
    sqlx::query("UPDATE accounts SET email='invalid mailbox' WHERE id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await?;
    let refused = browser(&db.pool, owner, "Rollback").await;
    assert!(service.record_login(&refused).await.is_err());
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM security_events WHERE kind='login_succeeded'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(events, 5);
    db.close().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::test]
async fn audit02_bulk_revocation_reports_current_session_without_identifier_collection()
-> TestResult {
    let Some(db) = database::Database::open()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let id = account(&db.pool).await;
    let actor = browser(&db.pool, id, "audit bulk").await;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) SELECT sha256(convert_to(n::text,'UTF8')),$1,'bulk',repeat('a',64),now()+interval '1 hour',now()+interval '1 day' FROM generate_series(1,1001) n").bind(id).execute(&db.pool).await?;
    let s = Sessions {
        pool: db.pool.clone(),
    };
    assert_eq!(
        s.revoke(&actor, Revocation::Others).await?,
        Outcome::Revoked
    );
    assert_eq!(s.revoke(&actor, Revocation::All).await?, Outcome::SignedOut);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::test]
async fn lk_configured_notification_accounts_use_one_connection() -> TestResult {
    let started = std::time::Instant::now();
    let db = database::Database::open()
        .await
        .unwrap()
        .expect("PORTAL_TEST_DATABASE_URL required");
    let account = account(&db.pool).await;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(schema)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
        .await?;
    let notifications = security_notifications(pool.clone())?;
    let mut tx = pool.begin().await?;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        notifications
            .preferences(&mut tx, account, oteryn_notify::Category::News, true, true)
            .await?;
        notifications
            .enqueue(&mut tx, account, &EmailTemplate::PasswordChanged)
            .await?;
        let token = notifications.unsubscribe.token(
            account,
            oteryn_notify::Category::News,
            OffsetDateTime::now_utc().unix_timestamp() + 60,
        )?;
        notifications.unsubscribe(&mut tx, &token).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await??;
    tx.commit().await?;
    // A new call observes revocation; transaction reuse must not become caching.
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    let mut tx = pool.begin().await?;
    assert!(
        notifications
            .enqueue(&mut tx, account, &EmailTemplate::PasswordChanged)
            .await
            .is_err()
    );
    tx.rollback().await?;
    pool.close().await;
    db.close().await.unwrap();
    eprintln!(
        "lk_configured_notification_accounts_use_one_connection duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_recent_game_revocations_are_owner_scoped_and_bounded() -> TestResult {
    let Some(db) = database::Database::open()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let service = Sessions {
        pool: db.pool.clone(),
    };
    let owner = account(&db.pool).await;
    let other = account(&db.pool).await;
    assert_eq!(owner.get_version_num(), 7);
    let current = browser(&db.pool, owner, "Owner").await;
    let foreign = browser(&db.pool, other, "Other").await;
    assert!(service.game_revocations(&current).await?.is_empty());
    for _ in 0..11 {
        sqlx::query("UPDATE accounts SET native_security_generation = native_security_generation + 1 WHERE id = $1")
            .bind(owner).execute(&db.pool).await?;
    }
    let records = service.game_revocations(&current).await?;
    assert_eq!(
        records
            .iter()
            .map(|row| row.native_security_generation)
            .collect::<Vec<_>>(),
        (3..=12).rev().collect::<Vec<_>>()
    );
    assert_eq!(records[0].cause_label(), "revocation-account-state");
    assert!(service.game_revocations(&foreign).await?.is_empty());
    let mismatched = Actor {
        account: owner,
        hash: foreign.hash,
    };
    assert!(service.game_revocations(&mismatched).await.is_err());
    db.close().await.map_err(|e| e.to_string())?;
    Ok(())
}
