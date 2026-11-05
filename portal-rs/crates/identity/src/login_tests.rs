#[path = "login_test_db.rs"]
mod database;
use super::*;
#[test]
fn validation_and_redaction() {
    let mut input = Credentials {
        email: " A@example.test ".into(),
        password: "short".into(),
    };
    assert!(input.valid());
    assert!(!format!("{input:?}").contains("short"));
    input.password.clear();
    assert!(!input.valid());
    input.password = "x".repeat(1025);
    assert!(!input.valid());
    input.password = "pw".into();
    input.email = "bad".into();
    assert!(!input.valid());
    assert!(!password::verify("wrong", DUMMY_HASH));
    assert!(!password::verify("wrong", "malformed"));
}
#[tokio::test]
async fn postgres_login_rehash_refusals_and_bad_input() -> Result<(), Box<dyn std::error::Error>> {
    let Some(database) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = database.pool.clone();
    let email = format!("login-{}@example.test", uuid::Uuid::now_v7());
    let old = password::PasswordConfig {
        memory_kib: 8192,
        iterations: 1,
        parallelism: 1,
    }
    .hash("test password")?;
    let id: AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(&email).bind(&old).fetch_one(&pool).await?;
    let credentials = |email: &str, pass: &str| Credentials {
        email: email.into(),
        password: pass.into(),
    };
    assert_eq!(
        authenticate(&pool, credentials(&email.to_uppercase(), "test password")).await?,
        Some(id)
    );
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_ne!(old, hash);
    assert!(!password::needs_rehash(&hash));
    assert!(password::verify("test password", &hash));
    assert_eq!(
        authenticate(&pool, credentials(&email, "wrong")).await?,
        None
    );
    assert_eq!(
        authenticate(&pool, credentials("absent@example.test", "wrong")).await?,
        None
    );
    assert_eq!(authenticate(&pool, credentials("bad", "")).await?, None);
    for status in ["suspended", "closed", "pending_verification"] {
        sqlx::query("UPDATE accounts SET status=$2 WHERE id=$1")
            .bind(id)
            .bind(status)
            .execute(&pool)
            .await?;
        assert_eq!(
            authenticate(&pool, credentials(&email, "test password")).await?,
            None
        );
    }
    sqlx::query("UPDATE accounts SET status='active',login_mfa_required=true WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        authenticate(&pool, credentials(&email, "test password")).await?,
        None
    );
    sqlx::query("UPDATE accounts SET login_mfa_required=false,email_verified_at=NULL WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        authenticate(&pool, credentials(&email, "test password")).await?,
        None
    );
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    database.close().await?;
    Ok(())
}
