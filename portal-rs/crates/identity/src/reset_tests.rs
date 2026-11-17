use super::*;
#[path = "reset_test_db.rs"]
mod db;
fn input(email: &str, token: &str) -> Input {
    Input {
        email: email.into(),
        token: token.into(),
        password: "new secure password".into(),
        password_confirmation: "new secure password".into(),
    }
}
fn key() -> DeliveryKey {
    DeliveryKey([17; 32])
}
async fn account(pool: &PgPool) -> Result<AccountId, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES ('reset@example.test','old','en','active',now()) RETURNING id").fetch_one(pool).await
}
async fn token(pool: &PgPool) -> Result<String, Box<dyn std::error::Error>> {
    let p:serde_json::Value=sqlx::query_scalar("SELECT payload FROM outbox WHERE kind='email.password-reset' ORDER BY created_at DESC LIMIT 1").fetch_one(pool).await?;
    key()
        .open(&serde_json::from_value::<Vec<u8>>(p["ciphertext"].clone())?)
        .ok_or_else(|| "invalid delivery ciphertext".into())
}
#[test]
fn input_and_ciphertext_refusals() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!input("bad", &"a".repeat(64)).valid());
    assert!(!input("a@example.test", "bad").valid());
    assert!(!input("a@example.test", &"z".repeat(64)).valid());
    let mut long = input("a@example.test", &"a".repeat(64));
    long.password = "x".repeat(257);
    long.password_confirmation = long.password.clone();
    assert!(!long.valid());
    let mut i = input("a@example.test", &"a".repeat(64));
    i.password = "short".into();
    assert!(!i.valid());
    i.password = "long enough password".into();
    assert!(!i.valid());
    assert!(DeliveryKey::parse("bad").is_none());
    assert!(DeliveryKey::parse(&"é".repeat(32)).is_none());
    assert!(key().open(&[]).is_none());
    let mut c = key().seal("secret")?;
    assert_eq!(key().open(&c).as_deref(), Some("secret"));
    c[15] ^= 1;
    assert!(key().open(&c).is_none());
    assert!(DeliveryKey([18; 32]).open(&key().seal("secret")?).is_none());
    Ok(())
}
#[tokio::test]
async fn success_revokes_and_replay_denied() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let id = account(p).await?;
    let handle = crate::native_recovery_test_support::handle(p, id).await?;
    request(p, " RESET@example.test ", &key()).await?;
    let t = token(p).await?;
    assert!(!crate::native_recovery_test_support::revoked(p, &handle).await?);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES (decode(repeat('ab',32),'hex'),$1,now()+interval '1 hour',now()+interval '1 day','test',repeat('c',64))").bind(id).execute(p).await?;
    assert!(complete(p, input("reset@example.test", &t)).await?);
    let h: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(p)
        .await?;
    assert!(password::verify("new secure password", &h));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(p)
        .await?;
    assert_eq!(n, 0);
    assert!(crate::native_recovery_test_support::revoked(p, &handle).await?);
    assert!(!complete(p, input("reset@example.test", &t)).await?);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='identity.password_reset_completed'",
    )
    .fetch_one(p)
    .await?;
    assert_eq!(n, 1);
    db.close().await
}
#[tokio::test]
async fn refusal_matrix_and_generic_requests() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let id = account(p).await?;
    let handle = crate::native_recovery_test_support::handle(p, id).await?;
    for e in ["absent@example.test", "bad"] {
        request(p, e, &key()).await?;
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox")
        .fetch_one(p)
        .await?;
    assert_eq!(n, 0);
    request(p, "reset@example.test", &key()).await?;
    let t = token(p).await?;
    request(p, "reset@example.test", &key()).await?;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox")
        .fetch_one(p)
        .await?;
    assert_eq!(n, 1);
    for i in [
        input("absent@example.test", &t),
        input("reset@example.test", &"0".repeat(64)),
        input("reset@example.test", ""),
        input("bad", &t),
    ] {
        assert!(!complete(p, i).await?);
    }
    let mut i = input("reset@example.test", &t);
    i.password_confirmation = "different".into();
    assert!(!complete(p, i).await?);
    // A refused or malformed reset leaves the recovery handles live.
    assert!(!crate::native_recovery_test_support::revoked(p, &handle).await?);
    for status in ["suspended", "closed"] {
        sqlx::query("UPDATE accounts SET status=$2 WHERE id=$1")
            .bind(id)
            .bind(status)
            .execute(p)
            .await?;
        request(p, "reset@example.test", &key()).await?;
        assert!(!complete(p, input("reset@example.test", &t)).await?);
    }
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(id)
        .execute(p)
        .await?;
    sqlx::query("UPDATE account_password_resets SET expires_at=now()-interval '1 second'")
        .execute(p)
        .await?;
    assert!(!complete(p, input("reset@example.test", &t)).await?);
    let h: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(p)
        .await?;
    assert_eq!(h, "old");
    db.close().await
}
#[tokio::test]
async fn replacement_and_concurrent_use() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    account(p).await?;
    request(p, "reset@example.test", &key()).await?;
    let old = token(p).await?;
    sqlx::query("UPDATE account_password_resets SET created_at=now()-interval '61 seconds'")
        .execute(p)
        .await?;
    request(p, "reset@example.test", &key()).await?;
    let new = token(p).await?;
    assert_ne!(old, new);
    assert!(!complete(p, input("reset@example.test", &old)).await?);
    let (a, b) = tokio::join!(
        complete(p, input("reset@example.test", &new)),
        complete(p, input("reset@example.test", &new))
    );
    assert_ne!(a?, b?);
    db.close().await
}

#[tokio::test]
async fn login_started_before_reset_cannot_restore_session()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let id = account(p).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,last_seen_at,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES (decode(repeat('cd',32),'hex'),now()-interval '1 second',now()+interval '1 hour',now()+interval '1 day','test',repeat('c',64))").execute(p).await?;
    request(p, "reset@example.test", &key()).await?;
    let t = token(p).await?;
    assert!(complete(p, input("reset@example.test", &t)).await?);
    let result = sqlx::query(
        "UPDATE web_sessions SET account_id=$1 WHERE id_hash=decode(repeat('cd',32),'hex')",
    )
    .bind(id)
    .execute(p)
    .await;
    assert!(result.is_err());
    // Activity refresh cannot authorize the earlier anonymous session.
    sqlx::query("UPDATE web_sessions SET last_seen_at=clock_timestamp() WHERE id_hash=decode(repeat('cd',32),'hex')").execute(p).await?;
    assert!(
        sqlx::query(
            "UPDATE web_sessions SET account_id=$1 WHERE id_hash=decode(repeat('cd',32),'hex')"
        )
        .bind(id)
        .execute(p)
        .await
        .is_err()
    );
    // A fresh password proof carries the immutable current security generation.
    sqlx::query("UPDATE web_sessions SET authorization_generation=(SELECT native_security_generation FROM accounts WHERE id=$1),account_id=$1 WHERE id_hash=decode(repeat('cd',32),'hex')").bind(id).execute(p).await?;
    db.close().await
}

#[tokio::test]
async fn audit_failure_rolls_back_password_token_and_sessions()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let id = account(p).await?;
    request(p, "reset@example.test", &key()).await?;
    let t = token(p).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES (decode(repeat('ab',32),'hex'),$1,now()+interval '1 hour',now()+interval '1 day','test',repeat('c',64))").bind(id).execute(p).await?;
    sqlx::raw_sql("CREATE FUNCTION refuse_reset_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'audit unavailable'; END; $$; CREATE TRIGGER refuse_reset_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_reset_audit();").execute(p).await?;
    assert!(complete(p, input("reset@example.test", &t)).await.is_err());
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(p)
        .await?;
    assert_eq!(hash, "old");
    let unused:bool=sqlx::query_scalar("SELECT used_at IS NULL AND last_reset_at IS NULL FROM account_password_resets WHERE account_id=$1").bind(id).fetch_one(p).await?;
    assert!(unused);
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(p)
        .await?;
    assert_eq!(sessions, 1);
    db.close().await
}

#[tokio::test]
async fn audit02_password_proof_is_revoked_despite_anonymous_activity_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    let hash = password::hash("initial password")?;
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(hash)
        .execute(&db.pool)
        .await?;
    let proof = crate::login::authenticate_authorized(
        &db.pool,
        crate::login::Credentials {
            email: "reset@example.test".into(),
            password: "initial password".into(),
        },
    )
    .await?
    .ok_or("authentication")?;
    assert!(proof.current(&db.pool).await?);
    request(&db.pool, "reset@example.test", &key()).await?;
    let token = token(&db.pool).await?;
    assert!(complete(&db.pool, input("reset@example.test", &token)).await?);
    // Refreshing anonymous activity after reset cannot change the proof generation.
    sqlx::query("INSERT INTO web_sessions(id_hash,last_seen_at,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES(decode(repeat('ef',32),'hex'),clock_timestamp(),now()+interval '1 hour',now()+interval '1 day','test',repeat('d',64))").execute(&db.pool).await?;
    sqlx::query("UPDATE web_sessions SET last_seen_at=clock_timestamp() WHERE account_id IS NULL")
        .execute(&db.pool)
        .await?;
    assert!(!proof.current(&db.pool).await?);
    db.close().await
}

#[tokio::test]
async fn audit02_revoker_invalidates_outstanding_reset_without_replacing_password()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    request(&db.pool, "reset@example.test", &key()).await?;
    let old = token(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    crate::recovery_key::revoke_authorizations(&mut tx, id).await?;
    sqlx::query("UPDATE accounts SET password_hash=$2,native_security_generation=native_security_generation+1 WHERE id=$1").bind(id).bind(password::hash("replacement password")?).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(!complete(&db.pool, input("reset@example.test", &old)).await?);
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(password::verify("replacement password", &hash));
    db.close().await
}

#[tokio::test]
async fn audit02_confirmed_backup_recovers_and_replacement_invalidates_link()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    request(&db.pool, "backup@example.test", &key()).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_password_resets")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'backup@example.test',clock_timestamp())").bind(id).execute(&db.pool).await?;
    request(&db.pool, "backup@example.test", &key()).await?;
    let old = token(&db.pool).await?;
    sqlx::query("UPDATE account_backup_emails SET email='replacement@example.test',verified_at=clock_timestamp() WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    assert!(!complete(&db.pool, input("backup@example.test", &old)).await?);
    assert!(!complete(&db.pool, input("reset@example.test", &old)).await?);
    sqlx::query("UPDATE account_password_resets SET created_at=now()-interval '61 seconds'")
        .execute(&db.pool)
        .await?;
    request(&db.pool, "replacement@example.test", &key()).await?;
    let new = token(&db.pool).await?;
    assert!(complete(&db.pool, input("replacement@example.test", &new)).await?);
    assert!(!complete(&db.pool, input("replacement@example.test", &new)).await?);
    db.close().await
}

#[tokio::test]
async fn audit02_verification_and_session_expire_while_waiting_for_account_lock()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    sqlx::query(
        "UPDATE accounts SET status='pending_verification',email_verified_at=NULL WHERE id=$1",
    )
    .bind(id)
    .execute(&db.pool)
    .await?;
    let token = "a".repeat(64);
    sqlx::query("INSERT INTO account_email_tokens(account_id,token_hash,purpose,expires_at) VALUES($1,$2,'verify_email',clock_timestamp()+interval '300 milliseconds')").bind(id).bind(crate::verify::token_hash(&token)).execute(&db.pool).await?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    let pool = db.pool.clone();
    let consume = tokio::spawn(async move { crate::verify::consume(&pool, &token).await });
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    assert!(!consume.is_finished());
    lock.commit().await?;
    assert!(consume.await??.is_none());
    sqlx::query("UPDATE accounts SET status='active',email_verified_at=now() WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    let hash = vec![7_u8; 32];
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,repeat('a',64),'audit',clock_timestamp()+interval '300 milliseconds',now()+interval '1 day')").bind(&hash).bind(id).execute(&db.pool).await?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    let pool = db.pool.clone();
    let authorize = tokio::spawn(async move {
        let mut tx = pool.begin().await?;
        crate::credentials::authenticated(&mut tx, &hash).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    assert!(!authorize.is_finished());
    lock.commit().await?;
    assert!(authorize.await??.is_none());
    db.close().await
}

#[tokio::test]
async fn audit02_password_change_refuses_preexisting_reset_link()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(password::hash("initial password")?)
        .execute(&db.pool)
        .await?;
    let hash = vec![8_u8; 32];
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,repeat('a',64),'audit',now()+interval '1 hour',now()+interval '1 day')").bind(&hash).bind(id).execute(&db.pool).await?;
    request(&db.pool, "reset@example.test", &key()).await?;
    let old = token(&db.pool).await?;
    let service = crate::credentials::Credentials {
        pool: db.pool.clone(),
        enabled: true,
        revoker: std::sync::Arc::new(crate::credentials::TransactionalRevoker),
        notifications: std::sync::Arc::new(
            crate::sessions::security_notifications(db.pool.clone()).map_err(|e| e.to_string())?,
        ),
    };
    assert_eq!(
        service
            .change(
                &hash,
                crate::credentials::PasswordChange {
                    current_password: "initial password".into(),
                    password: "replacement password".into(),
                    password_confirmation: "replacement password".into()
                }
            )
            .await?,
        crate::credentials::Outcome::Changed
    );
    assert!(!complete(&db.pool, input("reset@example.test", &old)).await?);
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(password::verify("replacement password", &hash));
    db.close().await
}

#[tokio::test]
async fn audit02_mfa_challenge_expires_during_account_lock_wait()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(password::hash("test password")?)
        .execute(&db.pool)
        .await?;
    let mfa = crate::mfa::Mfa::new(db.pool.clone(), [7; 32]);
    let enrollment = mfa.enroll(id, "test password").await?.ok_or("enroll")?;
    let bytes = totp_rs::Secret::Encoded(enrollment.secret).to_bytes()?;
    let code = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, bytes)?.generate_current()?;
    let codes = mfa
        .confirm(id, "test password", &code)
        .await?
        .ok_or("confirm")?;
    let challenge = mfa
        .begin_login(&crate::login::Credentials {
            email: "reset@example.test".into(),
            password: "test password".into(),
        })
        .await?
        .ok_or("challenge")?;
    sqlx::query("UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '300 milliseconds' WHERE account_id=$1 AND purpose='challenge'").bind(id).execute(&db.pool).await?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    let task = tokio::spawn(async move { mfa.complete(&challenge.token, &codes[0], false).await });
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    assert!(!task.is_finished());
    lock.commit().await?;
    assert!(task.await??.is_none());
    db.close().await
}

#[tokio::test]
async fn audit02_backup_recovery_refuses_legacy_cross_account_address_collision()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let id = account(&db.pool).await?;
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'colliding@example.test',now())").bind(id).execute(&db.pool).await?;
    // Reproduce the historical registration defect, bypassing the repaired claim API.
    sqlx::query("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('colliding@example.test',$1,'active','en',now())").bind(password::hash("foreign owner password")?).execute(&db.pool).await?;
    request(&db.pool, "colliding@example.test", &key()).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_password_resets")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    let raw = "a".repeat(64);
    sqlx::query("INSERT INTO account_password_resets(account_id,token_hash,expires_at,recovery_email,backup_verified_at) SELECT $1,$2,now()+interval '1 hour',email,verified_at FROM account_backup_emails WHERE account_id=$1").bind(id).bind(token_hash(&raw)).execute(&db.pool).await?;
    assert!(!complete(&db.pool, input("colliding@example.test", &raw)).await?);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT password_hash FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        "old"
    );
    db.close().await
}
