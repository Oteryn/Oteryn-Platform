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
    let handle = crate::native_recovery_test_support::handle(&db.pool, id).await?;
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm("not-a-token").await?, Outcome::Refused);
    assert!(!crate::native_recovery_test_support::revoked(&db.pool, &handle).await?);
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    assert!(crate::native_recovery_test_support::revoked(&db.pool, &handle).await?);
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

#[tokio::test]
async fn audit02_registration_respects_pending_and_backup_claims() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (token, _) = tokens(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    assert!(
        crate::accounts::Accounts::create(&mut tx, "NEW@example.invalid", "unused", "en")
            .await?
            .is_none()
    );
    tx.commit().await?;
    assert_eq!(s.confirm(&token).await?, Outcome::Changed);
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'backup@example.invalid',now())").bind(id).execute(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    assert!(
        crate::accounts::Accounts::create(&mut tx, "backup@example.invalid", "unused", "en")
            .await?
            .is_none()
    );
    tx.commit().await?;
    db.close().await
}

#[tokio::test]
async fn audit02_worker_pages_past_hundred_blocked_changes() -> TestResult {
    let started = std::time::Instant::now();
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(
        s.request(id, input(false, false)).await?,
        Outcome::Requested
    );
    let (token, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&token).await?, Outcome::Confirmed);
    sqlx::query("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) SELECT 'blocked'||n||'@example.invalid','unused','suspended','en',now() FROM generate_series(1,100) n").execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_email_changes(account_id,kind,old_email,new_email,confirmation_hash,revert_hash,immediate,confirmed_at,apply_after,expires_at,revert_until) SELECT id,'primary',email,'next-'||email,sha256(convert_to(email,'UTF8')),sha256(convert_to('revert-'||email,'UTF8')),false,now(),now()-interval '2 days',now()+interval '2 days',now()+interval '3 days' FROM accounts WHERE status='suspended'").execute(&db.pool).await?;
    sqlx::query(
        "UPDATE account_email_changes SET apply_after=now()-interval '1 day' WHERE account_id=$1",
    )
    .bind(id)
    .execute(&db.pool)
    .await?;
    assert_eq!(s.apply_due().await?, 1);
    assert_eq!(s.apply_due().await?, 0);
    db.close().await?;
    eprintln!(
        "audit02_worker_pages_past_hundred_blocked_changes: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn audit02_email_change_revokes_preexisting_reset_link() -> TestResult {
    struct ProofAndRevocation;
    impl Security for ProofAndRevocation {
        fn proof<'a>(
            &'a self,
            _: &'a mut PgConnection,
            _: Uuid,
            _: bool,
            _: &'a str,
        ) -> FutureResult<'a, bool> {
            Box::pin(async { Ok(true) })
        }
        fn revoke<'a>(&'a self, conn: &'a mut PgConnection, id: Uuid) -> FutureResult<'a, bool> {
            Box::pin(async move {
                crate::recovery_key::revoke_authorizations(conn, id).await?;
                Ok(true)
            })
        }
    }
    let Some((db, mut s, id)) = setup().await? else {
        return Ok(());
    };
    s.security = Arc::new(ProofAndRevocation);
    let raw = "c".repeat(64);
    sqlx::query("INSERT INTO account_password_resets(account_id,token_hash,expires_at) VALUES($1,$2,now()+interval '1 hour')").bind(id).bind(token_hash(&raw)).execute(&db.pool).await?;
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirmation, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirmation).await?, Outcome::Changed);
    crate::breach::tests::allow(&db.pool, "attacker password").await?;
    assert_eq!(
        crate::reset::complete(
            &db.pool,
            crate::reset::Input {
                email: "new@example.invalid".into(),
                token: raw,
                password: "attacker password".into(),
                password_confirmation: "attacker password".into()
            }
        )
        .await?,
        crate::reset::Completion::Refused
    );
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(password::verify("test-password", &hash));
    db.close().await
}

#[tokio::test]
async fn audit02_reversion_expires_after_waiting_for_account_lock() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    sqlx::query("UPDATE account_email_changes SET revert_until=clock_timestamp()+interval '300 milliseconds' WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    let task = tokio::spawn(async move { s.revert(&revert).await });
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    assert!(!task.is_finished());
    lock.commit().await?;
    assert_eq!(task.await??, Outcome::Refused);
    db.close().await
}

#[tokio::test]
async fn audit02_concurrent_registration_and_reservation_have_one_winner() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    let create = async {
        let mut tx = db.pool.begin().await?;
        let account =
            crate::accounts::Accounts::create(&mut tx, "new@example.invalid", "unused", "en")
                .await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(account)
    };
    let (registration, reservation) = tokio::join!(create, s.request(id, input(true, false)));
    let registration = registration?;
    let reservation = reservation?;
    assert_ne!(registration.is_some(), reservation == Outcome::Requested);
    if reservation == Outcome::Requested {
        let (token, _) = tokens(&db.pool).await?;
        assert_eq!(s.confirm(&token).await?, Outcome::Changed);
    }
    db.close().await
}

#[tokio::test]
async fn audit02_reversion_rechecks_deadline_after_normalized_address_lock_wait() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE accounts SET email='OLD@EXAMPLE.INVALID' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    sqlx::query("UPDATE account_email_changes SET revert_until=clock_timestamp()+interval '300 milliseconds' WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    let mut claim = db.pool.begin().await?;
    crate::accounts::claim_lock(&mut claim, "old@example.invalid").await?;
    let task = tokio::spawn(async move { s.revert(&revert).await });
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    assert!(!task.is_finished());
    claim.commit().await?;
    assert_eq!(task.await??, Outcome::Refused);
    db.close().await
}

#[derive(Debug)]
struct RejectOld;
impl oteryn_notify::EmailSender for RejectOld {
    fn send<'a>(&'a self, email: &'a oteryn_notify::Email) -> oteryn_notify::NotifyFuture<'a, ()> {
        Box::pin(async move {
            if email.recipient == "old@example.invalid" {
                Err("recipient refused".into())
            } else {
                Ok(())
            }
        })
    }
}
#[tokio::test]
async fn email_change_mail_quarantines_invalid_links_and_retries_other_origins() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    sqlx::query(
        "WITH account AS (
            INSERT INTO accounts(email,password_hash,status,locale)
            VALUES ('old@example.invalid','!fixture','active','en') RETURNING id
         ), change AS (
            INSERT INTO account_email_changes(account_id,kind,old_email,new_email,
                confirmation_hash,revert_hash,immediate,apply_after,expires_at,revert_until)
            SELECT id,'primary','old@example.invalid','new@example.invalid',
                sha256('confirm'::bytea),sha256('revert'::bytea),false,
                now()+interval '1 day',now()+interval '2 days',now()+interval '30 days'
            FROM account RETURNING id
         )
         INSERT INTO account_email_change_mail(change_id,recipient,subject,body)
         SELECT id,recipient,'Fixture',body FROM change CROSS JOIN (VALUES
            ('missing@example.invalid','missing link'),
            ('bad-token@example.invalid',E'message\n\nhttps://portal.example.invalid/email-change/confirm/bad'),
            ('bad-route@example.invalid',E'message\n\nhttps://portal.example.invalid/email-change/cancel/' || repeat('a',64)),
            ('query@example.invalid',E'message\n\nhttps://portal.example.invalid/email-change/revert/' || repeat('a',64) || '?extra=1'),
            ('fragment@example.invalid',E'message\n\nhttps://portal.example.invalid/email-change/revert/' || repeat('a',64) || '#extra'),
            ('userinfo@example.invalid',E'message\n\nhttps://other.example.invalid@portal.example.invalid/email-change/revert/' || repeat('a',64)),
            ('confirm@example.invalid',E'message\n\nhttps://other.example.invalid/email-change/confirm/' || repeat('a',64)),
            ('revert@example.invalid',E'message\n\nhttps://portal.example.invalid:8443/email-change/revert/' || repeat('b',64))
         ) AS mail(recipient,body)",
    )
    .execute(&db.pool)
    .await?;
    let service = Service {
        pool: db.pool.clone(),
        enabled: true,
        origin: "https://portal.example.invalid".into(),
        security: Arc::new(TransactionalSecurity(crate::mfa::Mfa::new(
            db.pool.clone(),
            [9; 32],
        ))),
    };
    let worker = super::mail::Worker {
        service,
        sender: Arc::new(
            oteryn_notify::SmtpSender::new(
                "localhost",
                "local".into(),
                "unused".into(),
                "portal@example.invalid".into(),
            )
            .map_err(|_| "local SMTP configuration invalid")?,
        ),
    };
    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct MailState {
        recipient: String,
        digest: Vec<u8>,
        erased: bool,
        unsent: bool,
        quarantined: bool,
        delivery_attempts: i32,
        backed_off: bool,
        updated_at: OffsetDateTime,
    }
    let state = || {
        sqlx::query_as::<_, MailState>(
            "SELECT recipient, sha256(convert_to(body, 'UTF8')) AS digest,
                    body='' AS erased, delivered_at IS NULL AS unsent,
                    next_attempt_at='infinity' AS quarantined, delivery_attempts,
                    next_attempt_at BETWEEN updated_at+interval '29 seconds'
                                        AND updated_at+interval '31 seconds' AS backed_off, updated_at
             FROM account_email_change_mail ORDER BY recipient",
        )
        .fetch_all(&db.pool)
    };
    let before = state().await?;
    worker.tick().await?;
    let first = state().await?;
    assert_eq!(first.len(), before.len());
    assert_eq!(first.len(), 8);
    for (after, before) in first.iter().zip(&before) {
        assert_eq!(after.recipient, before.recipient);
        assert!(after.unsent);
        match after.recipient.as_str() {
            "confirm@example.invalid" | "revert@example.invalid" => {
                assert_eq!(after.digest, before.digest);
                assert!(!after.erased);
                assert!(!after.quarantined);
                assert_eq!(after.delivery_attempts, 1);
                assert!(after.backed_off);
            }
            _ => {
                assert!(after.erased);
                assert!(after.quarantined);
                assert_eq!(after.delivery_attempts, 0);
            }
        }
    }
    worker.tick().await?;
    assert_eq!(state().await?, first);
    db.close().await
}

#[tokio::test]
async fn bzb_failed_email_does_not_starve_later_delivery() -> TestResult {
    let started = std::time::Instant::now();
    let Some((db, service, account)) = setup().await? else {
        return Ok(());
    };
    service.request(account, input(false, false)).await?;
    sqlx::query("UPDATE account_email_change_mail SET created_at=now()-interval '1 day' WHERE recipient='old@example.invalid'").execute(&db.pool).await?;
    let worker = super::mail::Worker {
        service,
        sender: Arc::new(RejectOld),
    };
    worker.tick().await?;
    let rows: Vec<(String, bool, i32, bool)> = sqlx::query_as("SELECT recipient,delivered_at IS NOT NULL,delivery_attempts,next_attempt_at>clock_timestamp() FROM account_email_change_mail ORDER BY recipient").fetch_all(&db.pool).await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("new@example.invalid".into(), true, 0, false));
    assert_eq!(rows[1], ("old@example.invalid".into(), false, 1, true));
    worker.tick().await?;
    let attempts: i32 = sqlx::query_scalar("SELECT delivery_attempts FROM account_email_change_mail WHERE recipient='old@example.invalid'").fetch_one(&db.pool).await?;
    assert_eq!(attempts, 1);
    db.close().await?;
    eprintln!(
        "bzb_failed_email_does_not_starve_later_delivery: {:?}",
        started.elapsed()
    );
    Ok(())
}

/// r4 lockemail: the normalized address claim precedes the account row, so a
/// request or confirmation waiting on the claim retains no account lock.
#[tokio::test]
async fn r4_address_claim_wait_retains_no_account_row() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    let address = format!("r4-{}@example.invalid", Uuid::now_v7().simple());
    for phase in ["request", "confirm"] {
        let mut claim = db.pool.begin().await?;
        let holder: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *claim)
            .await?;
        crate::accounts::claim_lock(&mut claim, &address).await?;
        let task = if phase == "request" {
            let mut request = input(true, false);
            request.email = address.clone();
            request.email_confirmation = address.clone();
            let s = s.clone();
            tokio::spawn(async move { s.request(id, request).await })
        } else {
            let (token, _) = tokens(&db.pool).await?;
            let s = s.clone();
            tokio::spawn(async move { s.confirm(&token).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let waiting: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
                )
                .bind(holder)
                .fetch_one(&db.pool)
                .await?;
                if waiting {
                    return Ok::<_, sqlx::Error>(());
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .map_err(|_| format!("{phase} never waited on the address claim"))??;
        let mut probe = db.pool.begin().await?;
        let free = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT")
            .bind(id)
            .execute(&mut *probe)
            .await;
        probe.rollback().await?;
        claim.commit().await?;
        let outcome = task.await??;
        assert!(
            free.is_ok(),
            "{phase} retained the account row while waiting on the address claim: {free:?}"
        );
        let expected = if phase == "request" {
            Outcome::Requested
        } else {
            Outcome::Changed
        };
        assert_eq!(outcome, expected, "{phase}");
    }
    db.close().await
}

#[tokio::test]
async fn d1_password_reset_cancels_pending_email_change() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirmation, _) = tokens(&db.pool).await?;
    // The owner recovers the account through the real reset path.
    let key = crate::reset::DeliveryKey::parse(&"11".repeat(32)).unwrap();
    crate::reset::request(&db.pool, "old@example.invalid", &key).await?;
    let payload: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM outbox WHERE kind='email.password-reset' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&db.pool)
    .await?;
    let reset_token = key
        .open(&serde_json::from_value::<Vec<u8>>(
            payload["ciphertext"].clone(),
        )?)
        .unwrap();
    crate::breach::tests::allow(&db.pool, "owner recovered password").await?;
    assert_eq!(
        crate::reset::complete(
            &db.pool,
            crate::reset::Input {
                email: "old@example.invalid".into(),
                token: reset_token,
                password: "owner recovered password".into(),
                password_confirmation: "owner recovered password".into(),
            }
        )
        .await?,
        crate::reset::Completion::Completed
    );
    let outcome = s.confirm(&confirmation).await?;
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        (outcome, email.as_str()),
        (Outcome::Refused, "old@example.invalid")
    );
    db.close().await
}

#[tokio::test]
async fn d1_recovery_key_cancels_pending_email_change() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    let mut keys = crate::recovery_key::RecoveryKeys::new(db.pool.clone());
    keys.enabled = crate::recovery_key::Enabled(true);
    let key = keys.generate(id, "test-password").await?;
    keys.confirm(id, "test-password", &key).await?;
    crate::breach::tests::allow(&db.pool, "owner recovered password").await?;
    assert_eq!(
        s.request(id, input(false, false)).await?,
        Outcome::Requested
    );
    let (confirmation, _) = tokens(&db.pool).await?;
    // The owner recovers the account through the real recovery-key path.
    keys.recover(
        "old@example.invalid",
        &key,
        "owner recovered password",
        "owner recovered password",
    )
    .await?;
    let outcome = s.confirm(&confirmation).await?;
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        (outcome, email.as_str()),
        (Outcome::Refused, "old@example.invalid")
    );
    db.close().await
}

#[tokio::test]
async fn d1_recovery_cancels_confirmed_change_waiting_for_worker() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    let mut keys = crate::recovery_key::RecoveryKeys::new(db.pool.clone());
    keys.enabled = crate::recovery_key::Enabled(true);
    let key = keys.generate(id, "test-password").await?;
    keys.confirm(id, "test-password", &key).await?;
    crate::breach::tests::allow(&db.pool, "owner recovered password").await?;
    assert_eq!(
        s.request(id, input(false, false)).await?,
        Outcome::Requested
    );
    let (confirmation, _) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirmation).await?, Outcome::Confirmed);
    // The owner recovers the account before the delayed change is applied.
    keys.recover(
        "old@example.invalid",
        &key,
        "owner recovered password",
        "owner recovered password",
    )
    .await?;
    sqlx::query("UPDATE account_email_changes SET apply_after=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    let applied = s.apply_due().await?;
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!((applied, email.as_str()), (0, "old@example.invalid"));
    db.close().await
}
/// Review 5: the old primary address of an applied change stays reserved for
/// its account during the revert window, so registering it cannot block the
/// owner's "this was not me" revert.
#[tokio::test]
async fn r5_old_address_stays_reserved_through_the_revert_window() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    let mut tx = db.pool.begin().await?;
    assert!(
        crate::accounts::Accounts::create(&mut tx, "OLD@example.invalid", "unused", "en")
            .await?
            .is_none()
    );
    tx.commit().await?;
    assert_eq!(s.revert(&revert).await?, Outcome::Reverted);
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(email, "old@example.invalid");
    sqlx::query("UPDATE accounts SET email='later@example.invalid' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    let mut tx = db.pool.begin().await?;
    assert!(
        crate::accounts::Accounts::create(&mut tx, "old@example.invalid", "unused", "en")
            .await?
            .is_some()
    );
    tx.commit().await?;
    db.close().await
}

#[tokio::test]
async fn r5_no_chained_primary_change_inside_the_revert_window() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    assert_eq!(s.request(id, input(true, false)).await?, Outcome::Requested);
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    // Past the 72-hour block, the first change can still be reverted.
    sqlx::query("UPDATE accounts SET email_change_blocked_until=now() WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    let next = || {
        let mut r = input(true, false);
        r.email = "third@example.invalid".into();
        r.email_confirmation = r.email.clone();
        r
    };
    assert_eq!(s.request(id, next()).await?, Outcome::Refused);
    assert_eq!(s.revert(&revert).await?, Outcome::Reverted);
    let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(email, "old@example.invalid");
    // Once nothing is revertible, a new change is accepted again.
    sqlx::query("UPDATE accounts SET email_change_blocked_until=now() WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert_eq!(s.request(id, next()).await?, Outcome::Requested);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn primary_revert_removes_new_backup() -> TestResult {
    let Some((db, s, id)) = setup().await? else {
        return Ok(());
    };
    s.request(id, input(true, false)).await?;
    let (confirm, revert) = tokens(&db.pool).await?;
    assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'later@example.test',now())").bind(id).execute(&db.pool).await?;
    assert_eq!(s.revert(&revert).await?, Outcome::Reverted);
    let verified: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM account_backup_emails WHERE account_id=$1 AND verified_at IS NOT NULL)").bind(id).fetch_one(&db.pool).await?;
    assert!(!verified, "later backup remains verified");
    db.close().await
}

#[tokio::test]
async fn delayed_primary_revert_removes_backup_confirmed_during_wait() -> TestResult {
    for (backup, expected) in [
        ("later@example.test", None),
        ("new@example.invalid", Some("new@example.invalid")),
    ] {
        let Some((db, s, id)) = setup().await? else {
            return Ok(());
        };
        assert_eq!(
            s.request(id, input(false, false)).await?,
            Outcome::Requested
        );
        let (confirm, revert) = tokens(&db.pool).await?;
        assert_eq!(s.confirm(&confirm).await?, Outcome::Confirmed);
        sqlx::query("DELETE FROM account_email_change_mail")
            .execute(&db.pool)
            .await?;

        let mut request = input(false, true);
        request.email = backup.into();
        request.email_confirmation = backup.into();
        assert_eq!(s.request(id, request).await?, Outcome::Requested);
        let (confirm, _) = tokens(&db.pool).await?;
        assert_eq!(s.confirm(&confirm).await?, Outcome::Changed);
        assert_eq!(s.apply_due().await?, 0);
        sqlx::query("UPDATE account_email_changes SET apply_after=now()-interval '1 second' WHERE account_id=$1 AND kind='primary'")
            .bind(id)
            .execute(&db.pool)
            .await?;
        assert_eq!(s.apply_due().await?, 1);
        assert_eq!(s.revert(&revert).await?, Outcome::Reverted);
        let remaining: Option<String> =
            sqlx::query_scalar("SELECT email FROM account_backup_emails WHERE account_id=$1")
                .bind(id)
                .fetch_optional(&db.pool)
                .await?;
        assert_eq!(
            remaining.as_deref(),
            expected,
            "backup after reverting delayed primary change"
        );
        db.close().await?;
    }
    Ok(())
}
