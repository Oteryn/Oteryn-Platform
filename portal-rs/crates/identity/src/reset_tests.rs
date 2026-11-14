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
    // A fresh request verifies the replacement password before rotating the session.
    sqlx::query("UPDATE web_sessions SET last_seen_at=clock_timestamp() WHERE id_hash=decode(repeat('cd',32),'hex')").execute(p).await?;
    sqlx::query(
        "UPDATE web_sessions SET account_id=$1 WHERE id_hash=decode(repeat('cd',32),'hex')",
    )
    .bind(id)
    .execute(p)
    .await?;
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
