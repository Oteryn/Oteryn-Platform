#[path = "mfa_test_db.rs"]
mod database;
use super::*;
fn code(secret: &str, offset: u64) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = totp_rs::Secret::Encoded(secret.to_owned()).to_bytes()?;
    Ok(TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes)?.generate(now()? + offset))
}
#[tokio::test]
async fn encryption_validation_and_replay_window() -> Result<(), Box<dyn std::error::Error>> {
    let pool = oteryn_db::lazy_pool("postgres://localhost/test")?;
    let mfa = Mfa::new(pool, [7; 32]);
    let id = Uuid::now_v7();
    let secret = vec![3; 20];
    let encrypted = mfa.encrypt(id, &secret)?;
    assert_ne!(encrypted, secret);
    assert_eq!(mfa.decrypt(id, &encrypted)?, secret);
    assert!(mfa.decrypt(Uuid::now_v7(), &encrypted).is_err());
    assert!(mfa.decrypt(id, &[1; 12]).is_err());
    let mut tampered = encrypted.clone();
    tampered[13] ^= 1;
    assert!(mfa.decrypt(id, &tampered).is_err());
    let mut state = State {
        secret: encrypted,
        confirmed: true,
        last_step: -1,
        generation: Uuid::now_v7(),
    };
    let current = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret)?.generate(3000);
    assert_eq!(mfa.step(id, &state, &current, 3000)?, Some(100));
    state.last_step = 100;
    assert_eq!(mfa.step(id, &state, &current, 3000)?, None);
    for bad in ["", "12345", "1234567", "abcdef", "１２３４５６"] {
        assert_eq!(mfa.step(id, &state, bad, 3000)?, None);
    }
    assert!(!valid_token("bad"));
    Ok(())
}
async fn account(pool: &PgPool, email: &str) -> Result<AccountId, Box<dyn std::error::Error>> {
    let hash = password::hash("test password")?;
    Ok(sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(email).bind(hash).fetch_one(pool).await?)
}
async fn ready(mfa: &Mfa, id: AccountId) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let enrollment = mfa
        .enroll(id, "test password")
        .await?
        .ok_or("enroll refused")?;
    Ok(mfa
        .confirm(id, "test password", &code(&enrollment.secret, 0)?)
        .await?
        .ok_or("confirm refused")?)
}
fn credentials(email: &str, password: &str) -> Credentials {
    Credentials {
        email: email.to_owned(),
        password: password.to_owned(),
    }
}
#[tokio::test]
async fn postgres_mfa_enrollment_and_refusals() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id = account(&db.pool, "enroll@example.test").await?;
    assert!(mfa.enroll(id, "wrong").await?.is_none());
    assert!(mfa.enroll(Uuid::now_v7(), "test password").await?.is_none());
    assert!(mfa.confirm(id, "test password", "123456").await?.is_none());
    let enrollment = mfa.enroll(id, "test password").await?.ok_or("enroll")?;
    assert!(
        mfa.confirm(id, "wrong", &code(&enrollment.secret, 0)?)
            .await?
            .is_none()
    );
    assert!(mfa.confirm(id, "test password", "bad").await?.is_none());
    // Reuse the confirmed step's code: a fresh one could fall in the next 30 s window.
    let confirmed = code(&enrollment.secret, 0)?;
    let codes = mfa
        .confirm(id, "test password", &confirmed)
        .await?
        .ok_or("confirm")?;
    assert_eq!(codes.len(), 8);
    assert!(mfa.enroll(id, "test password").await?.is_none());
    assert!(
        mfa.confirm(id, "test password", &code(&enrollment.secret, 0)?)
            .await?
            .is_none()
    );
    assert!(
        crate::login::authenticate(
            &db.pool,
            credentials("enroll@example.test", "test password")
        )
        .await?
        .is_none()
    );
    let challenge = mfa
        .begin_login(&credentials("enroll@example.test", "test password"))
        .await?
        .ok_or("challenge")?;
    assert!(
        mfa.complete(&challenge.token, &confirmed, false)
            .await?
            .is_none()
    );
    assert!(
        mfa.complete(&challenge.token, "bad", false)
            .await?
            .is_none()
    );
    let auth = mfa
        .complete(&challenge.token, &code(&enrollment.secret, 30)?, false)
        .await?
        .ok_or("totp")?;
    assert!(mfa.satisfied(id, &auth.grant).await?);
    assert!(
        mfa.complete(&challenge.token, &codes[0], false)
            .await?
            .is_none()
    );
    assert!(
        mfa.begin_login(&credentials("enroll@example.test", "wrong"))
            .await?
            .is_none()
    );
    assert!(
        mfa.begin_login(&credentials("absent@example.test", "wrong"))
            .await?
            .is_none()
    );
    assert!(mfa.begin_login(&credentials("bad", "")).await?.is_none());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='mfa.refused'")
            .fetch_one(&db.pool)
            .await?;
    assert!(count >= 12);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_mfa_recovery_email_and_concurrent_consumption()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id = account(&db.pool, "codes@example.test").await?;
    let codes = ready(&mfa, id).await?;
    let creds = credentials("codes@example.test", "test password");
    let first = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let second = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let uppercase = codes[0].to_ascii_uppercase();
    let (a, b) = tokio::join!(
        mfa.complete(&first.token, &uppercase, false),
        mfa.complete(&second.token, &codes[0], false)
    );
    assert_eq!(usize::from(a?.is_some()) + usize::from(b?.is_some()), 1);
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let (_, old) = mfa.email_code(&challenge.token).await?.ok_or("email")?;
    let (_, email) = mfa.email_code(&challenge.token).await?.ok_or("email")?;
    assert!(mfa.complete(&challenge.token, &old, false).await?.is_none());
    let other = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    assert!(mfa.complete(&other.token, &email, false).await?.is_none());
    assert!(
        mfa.complete(&challenge.token, &email, false)
            .await?
            .is_some()
    );
    assert!(
        mfa.complete(&challenge.token, &email, false)
            .await?
            .is_none()
    );
    assert!(mfa.email_code("bad").await?.is_none());
    let expired = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    sqlx::query(
        "UPDATE account_mfa_tokens SET expires_at=now()-interval '1 second' WHERE token_hash=$1",
    )
    .bind(digest(&expired.token))
    .execute(&db.pool)
    .await?;
    assert!(mfa.pending_account(&expired.token).await?.is_none());
    assert!(
        mfa.complete(&expired.token, &codes[1], false)
            .await?
            .is_none()
    );
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let (_, email) = mfa.email_code(&challenge.token).await?.ok_or("email")?;
    sqlx::query(
        "UPDATE account_mfa_tokens SET expires_at=now()-interval '1 second' WHERE purpose='email'",
    )
    .execute(&db.pool)
    .await?;
    assert!(
        mfa.complete(&challenge.token, &email, false)
            .await?
            .is_none()
    );
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_mfa_trusted_device_ownership_revocation_and_credential_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id = account(&db.pool, "devices@example.test").await?;
    let codes = ready(&mfa, id).await?;
    let other = account(&db.pool, "other@example.test").await?;
    let other_codes = ready(&mfa, other).await?;
    let creds = credentials("devices@example.test", "test password");
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let auth = mfa
        .complete(&challenge.token, &codes[0], true)
        .await?
        .ok_or("complete")?;
    let device = auth.trusted.ok_or("device")?;
    assert!(!mfa.satisfied(other, &auth.grant).await?);
    assert!(!mfa.satisfied(id, "bad").await?);
    let foreign = mfa
        .begin_login(&credentials("other@example.test", "test password"))
        .await?
        .ok_or("other")?;
    assert!(mfa.trust(&foreign.token, &device).await?.is_none());
    assert!(
        mfa.complete(&foreign.token, &other_codes[0], false)
            .await?
            .is_some()
    );
    let identifiers = mfa.devices(id).await?;
    assert_eq!(identifiers.len(), 1);
    assert!(!mfa.remove_device(other, &identifiers[0]).await?);
    assert!(!mfa.remove_device(id, "bad").await?);
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    assert!(mfa.trust(&challenge.token, "bad").await?.is_none());
    let auth = mfa.trust(&challenge.token, &device).await?.ok_or("trust")?;
    assert!(mfa.trust(&challenge.token, &device).await?.is_none());
    let rotated = auth.trusted.ok_or("rotated")?;
    let identifiers = mfa.devices(id).await?;
    assert!(mfa.remove_device(id, &identifiers[0]).await?);
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    assert!(mfa.trust(&challenge.token, &rotated).await?.is_none());
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(password::hash("new password")?)
        .execute(&db.pool)
        .await?;
    assert!(!mfa.satisfied(id, &auth.grant).await?);
    assert!(mfa.pending_account(&challenge.token).await?.is_none());
    let challenge = mfa
        .begin_login(&credentials("devices@example.test", "new password"))
        .await?
        .ok_or("challenge")?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(
        mfa.complete(&challenge.token, &codes[1], false)
            .await?
            .is_none()
    );
    assert!(
        mfa.begin_login(&credentials("devices@example.test", "new password"))
            .await?
            .is_none()
    );
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_mfa_stale_generation_expiry_and_session_authorization()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_db::outbox::Cleanup;
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id = account(&db.pool, "state@example.test").await?;
    let codes = ready(&mfa, id).await?;
    let creds = credentials("state@example.test", "test password");
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    let auth = mfa
        .complete(&challenge.token, &codes[0], true)
        .await?
        .ok_or("complete")?;
    let authorization = mfa.authorize_session(&auth).await?.ok_or("authorization")?;
    assert!(authorization.current(&mfa).await?);
    assert!(
        mfa.password_session(credentials("state@example.test", "test password"))
            .await?
            .is_none()
    );
    let device = auth.trusted.as_deref().ok_or("device")?;
    sqlx::query("UPDATE account_mfa_tokens SET expires_at=now()-interval '1 second' WHERE purpose='trusted' OR purpose='grant'").execute(&db.pool).await?;
    assert!(!mfa.satisfied(id, &auth.grant).await?);
    assert!(mfa.authorize_session(&auth).await?.is_none());
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    assert!(mfa.trust(&challenge.token, device).await?.is_none());
    sqlx::query("UPDATE account_mfa_totp SET generation=uuidv7() WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(
        mfa.complete(&challenge.token, &codes[1], false)
            .await?
            .is_none()
    );
    let challenge = mfa.begin_login(&creds).await?.ok_or("challenge")?;
    sqlx::query("UPDATE account_mfa_totp SET confirmed=false WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(mfa.pending_account(&challenge.token).await?.is_none());
    assert!(mfa.email_code(&challenge.token).await?.is_none());
    sqlx::query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(mfa.enroll(id, "test password").await?.is_none());
    assert!(mfa.begin_login(&creds).await?.is_none());
    MfaCleanup {
        pool: db.pool.clone(),
    }
    .clean()
    .await
    .map_err(std::io::Error::other)?;
    let expired: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_mfa_tokens WHERE expires_at<=now()")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(expired, 0);
    let normal = account(&db.pool, "normal@example.test").await?;
    assert!(
        mfa.password_session(credentials("normal@example.test", "wrong"))
            .await?
            .is_none()
    );
    let authorization = mfa
        .password_session(credentials("normal@example.test", "test password"))
        .await?
        .ok_or("password authorization")?;
    assert_eq!(authorization.account, normal);
    assert!(authorization.current(&mfa).await?);
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(normal)
        .bind(password::hash("replacement password")?)
        .execute(&db.pool)
        .await?;
    assert!(!authorization.current(&mfa).await?);
    db.close().await?;
    Ok(())
}
