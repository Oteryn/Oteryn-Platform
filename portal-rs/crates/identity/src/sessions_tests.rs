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
