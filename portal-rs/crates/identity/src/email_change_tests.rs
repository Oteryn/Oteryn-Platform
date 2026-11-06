#![allow(clippy::unwrap_used)]
use super::*;
#[path = "email_change_test_db.rs"]
mod database;
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Authority;
impl Security for Authority {
    fn proof<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: Uuid,
        _: bool,
        proof: &'a str,
    ) -> FutureResult<'a, bool> {
        Box::pin(async move { Ok(proof == "verified-test-proof") })
    }
    fn revoke<'a>(&'a self, _: &'a mut PgConnection, _: Uuid) -> FutureResult<'a, bool> {
        Box::pin(async { Ok(true) })
    }
}
async fn setup() -> Result<Option<(database::Database, Service, Uuid)>, Box<dyn std::error::Error>>
{
    let Some(db) = database::Database::open().await? else {
        return Ok(None);
    };
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'old@example.invalid',$2,'active','en',now())").bind(id).bind(password::hash("test-password")?).execute(&db.pool).await?;
    let s = Service {
        pool: db.pool.clone(),
        enabled: true,
        origin: "https://portal.example.invalid".into(),
        security: Arc::new(Authority),
    };
    Ok(Some((db, s, id)))
}
fn input(recovery: bool, backup: bool) -> Request {
    Request {
        email: "new@example.invalid".into(),
        email_confirmation: "new@example.invalid".into(),
        current_password: "test-password".into(),
        proof: "verified-test-proof".into(),
        recovery,
        backup,
    }
}
async fn tokens(pool: &PgPool) -> Result<(String, String), sqlx::Error> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT body FROM account_email_change_mail ORDER BY created_at,id")
            .fetch_all(pool)
            .await?;
    let find = |kind: &str| {
        rows.iter()
            .find(|r| r.contains(kind))
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .to_owned()
    };
    Ok((find("/confirm/"), find("/revert/")))
}
#[test]
fn validation_and_translations() {
    assert!(input(false, false).valid());
    let mut r = input(false, false);
    r.email = "bad".into();
    assert!(!r.valid());
    assert!(!valid_token("bad"));
    assert!(!valid_token(&"A".repeat(64)));
    for locale in ["pl", "en"] {
        for key in [
            "title",
            "confirm",
            "revert",
            "email",
            "repeat",
            "password",
            "proof",
            "recovery",
            "backup",
            "submit",
            "failed",
            "requested",
            "confirmed",
            "changed",
            "cancelled",
            "reverted",
        ] {
            assert!(!mail::text(locale, key).unwrap().is_empty());
        }
    }
    assert!(mail::text("unknown", "title").is_err());
}
#[tokio::test]
async fn immediate_change_revert_and_cooldown() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    assert_eq!(s.confirm(&confirm).await?, Outcome::Refused);
    let mut tx = db.pool.begin().await?;
    assert!(!trade_allowed(&mut tx, id).await?);
    tx.rollback().await?;
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Refused);
    assert_eq!(s.revert(&revert).await?, Outcome::Reverted);
    assert_eq!(s.revert(&revert).await?, Outcome::Refused);
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(email, "old@example.invalid");
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn delayed_change_requires_confirmation_and_worker() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(
        s.request(id, input(false, false)).await?,
        Outcome::Requested
    );
    assert_eq!(s.apply_due().await?, 0);
    let (confirm, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Confirmed);
    assert_eq!(s.apply_due().await?, 0);
    sqlx::query("UPDATE account_email_changes SET apply_after=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(s.apply_due().await?, 1);
    assert_eq!(s.apply_due().await?, 0);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn old_address_cancels_and_expired_tokens_deny() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    s.request(id, input(false, false)).await?;
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.revert(&revert).await?, Outcome::Cancelled);
    assert_eq!(s.confirm(&confirm).await?, Outcome::Refused);
    s.request(id, input(false, false)).await?;
    sqlx::query("UPDATE account_email_changes SET expires_at=now()-interval '1 second',revert_until=now()-interval '1 second'").execute(&db.pool).await?;
    let rows: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT confirmation_hash FROM account_email_changes")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(s.confirm("malformed").await?, Outcome::Refused);
    assert_eq!(s.revert(&revert).await?, Outcome::Refused);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn backup_confirmation_and_swap() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(false, true)).await?, Outcome::Requested);
    let (confirm, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(email, "old@example.invalid");
    sqlx::query("DELETE FROM account_email_change_mail")
        .execute(&db.pool)
        .await?;
    s.request(id, input(true, false)).await?;
    let (confirm, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    let backup: String =
        sqlx::query_scalar("SELECT email FROM account_backup_emails WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(backup, "old@example.invalid");
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn refusals_bad_input_unknown_authority_and_disabled_flag() -> TestResult {
    let Some((db, mut s, id)) = setup().await? else {
        return Ok(());
    };
    let mut bad = input(true, false);
    bad.current_password = "wrong".into();
    assert_eq!(s.request(id, bad).await?, Outcome::Refused);
    let mut bad = input(true, false);
    bad.email_confirmation = "other@example.invalid".into();
    assert_eq!(s.request(id, bad).await?, Outcome::Refused);
    let mut bad = input(true, false);
    bad.proof = "unknown".into();
    assert_eq!(s.request(id, bad).await?, Outcome::Refused);
    s.enabled = false;
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Refused);
    s.enabled = true;
    s.security = Arc::new(DenySecurity);
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Refused);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_email_changes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn delivery_uses_frozen_recipients_and_clears_tokens() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    s.request(id, input(true, false)).await?;
    let (confirm, _) = tokens(&db.pool).await?;
    s.confirm(&confirm).await?;
    let sender = Arc::new(oteryn_notify::CaptureSender::default());
    let worker = Worker {
        service: s,
        sender: sender.clone(),
    };
    worker.tick().await.map_err(std::io::Error::other)?;
    worker.tick().await.map_err(std::io::Error::other)?;
    {
        let emails = sender.emails.lock().unwrap();
        assert_eq!(emails.len(), 2);
        assert!(emails.iter().any(|e| e.recipient == "old@example.invalid"));
        assert!(emails.iter().any(|e| e.recipient == "new@example.invalid"));
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_email_change_mail WHERE body<>''")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn replay_concurrency_session_revocation_and_collision() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 day','test',$3)").bind(vec![0u8;32]).bind(id).bind("a".repeat(64)).execute(&db.pool).await?;
    s.request(id, input(true, false)).await?;
    let (confirm, _) = tokens(&db.pool).await?;
    let (a, b) = tokio::join!(s.confirm(&confirm), s.confirm(&confirm));
    let outcomes = [a?, b?];
    assert_eq!(
        outcomes.iter().filter(|o| **o == Outcome::Changed).count(),
        1
    );
    assert_eq!(
        outcomes.iter().filter(|o| **o == Outcome::Refused).count(),
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(generation, 2);
    sqlx::query("UPDATE accounts SET email_change_blocked_until='-infinity'")
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('taken@example.invalid','unused','active','pl',now())").execute(&db.pool).await?;
    let mut r = input(true, false);
    r.email = "taken@example.invalid".into();
    r.email_confirmation = r.email.clone();
    assert_eq!(s.request(id, r).await?, Outcome::Refused);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn revocation_failure_rolls_back_and_expired_revert_denies() -> TestResult {
    let Some((db, mut s, id)) = setup().await? else {
        return Ok(());
    };
    s.request(id, input(true, false)).await?;
    let (confirm, revert) = tokens(&db.pool).await?;
    s.security = Arc::new(DenySecurity);
    assert_eq!(s.confirm(&confirm).await?, Outcome::Refused);
    let confirmed: Option<OffsetDateTime> =
        sqlx::query_scalar("SELECT confirmed_at FROM account_email_changes")
            .fetch_one(&db.pool)
            .await?;
    assert!(confirmed.is_none());
    s.security = Arc::new(Authority);
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    sqlx::query("UPDATE account_email_changes SET revert_until=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(s.revert(&revert).await?, Outcome::Refused);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn suspended_account_and_unconfirmed_backup_deny() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    s.request(id, input(false, true)).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_backup_emails")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let (confirm, _) = tokens(&db.pool).await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Refused);
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Refused);
    assert!(!trade_allowed(&mut *db.pool.acquire().await?, id).await?);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_integrated_recovery_proof_confirm_revert_and_refusals() -> TestResult {
    let Some((db, mut service, account)) = setup().await? else {
        return Ok(());
    };
    service.security = Arc::new(TransactionalSecurity(crate::mfa::Mfa::new(
        db.pool.clone(),
        [9; 32],
    )));
    let mut keys = crate::recovery_key::RecoveryKeys::new(db.pool.clone());
    keys.enabled = crate::recovery_key::Enabled(true);
    let key = keys.generate(account, "test-password").await?;
    keys.confirm(account, "test-password", &key).await?;
    let mut request = input(true, false);
    request.proof = key.clone();
    request.current_password = "wrong password".into();
    assert_eq!(service.request(account, request).await?, Outcome::Refused);
    let used: bool = sqlx::query_scalar(
        "SELECT used_at IS NOT NULL FROM account_recovery_keys WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    assert!(!used);
    let mut request = input(true, false);
    request.proof = key.clone();
    request.email = "invalid".into();
    assert_eq!(service.request(account, request).await?, Outcome::Refused);
    let mut request = input(true, false);
    request.proof = key.clone();
    assert_eq!(service.request(account, request).await?, Outcome::Requested);
    let mut replay = input(true, false);
    replay.proof = key;
    assert_eq!(service.request(account, replay).await?, Outcome::Refused);
    sqlx::query("INSERT INTO oauth_token_families(id,account_id,client_id,credential_hash) VALUES(uuidv7(),$1,'test',decode(repeat('00',32),'hex'))").bind(account).execute(&db.pool).await?;
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(service.confirm(&confirm).await?, Outcome::Changed);
    let state:(String,i64,bool)=sqlx::query_as("SELECT email,native_security_generation,(SELECT bool_and(revoked_at IS NOT NULL) FROM oauth_token_families WHERE account_id=$1) FROM accounts WHERE id=$1").bind(account).fetch_one(&db.pool).await?;
    assert_eq!(state, ("new@example.invalid".into(), 2, true));
    assert_eq!(service.confirm(&confirm).await?, Outcome::Refused);
    assert_eq!(service.revert(&revert).await?, Outcome::Reverted);
    assert_eq!(service.revert(&revert).await?, Outcome::Refused);
    let state: (String, i64) =
        sqlx::query_as("SELECT email,native_security_generation FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(state, ("old@example.invalid".into(), 3));
    db.close().await?;
    Ok(())
}
