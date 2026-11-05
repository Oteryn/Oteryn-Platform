use super::*;
#[path = "credentials_test_db.rs"]
mod database;
#[derive(Debug)]
struct Accounts(crate::accounts::Accounts);
impl oteryn_notify::AccountState for Accounts {
    fn active_verified(
        &self,
        id: AccountId,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async move {
            Ok(self
                .0
                .find_by_id(id)
                .await?
                .filter(|a| a.status == "active" && a.email_verified_at.is_some())
                .map(|a| oteryn_notify::Account {
                    email: a.email,
                    locale: oteryn_notify::Locale::En,
                }))
        })
    }
}
struct Revoke {
    fail: bool,
}
impl AuthorizationRevoker for Revoke {
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> EffectFuture<'a> {
        Box::pin(async move {
            sqlx::query("UPDATE test_authorizations SET revoked=true WHERE account_id=$1")
                .bind(account)
                .execute(&mut **tx)
                .await?;
            if self.fail {
                return Err(sqlx::Error::Protocol("test refusal".into()));
            }
            Ok(())
        })
    }
}
fn input(current: &str, new: &str) -> PasswordChange {
    PasswordChange {
        current_password: current.into(),
        password: new.into(),
        password_confirmation: new.into(),
    }
}
#[test]
fn validates_password_change() {
    assert!(input("old password", "new password").valid());
    assert!(!input("old password", "old password").valid());
    assert!(!input("", "new password").valid());
    assert!(!input("old password", "short").valid());
    let mut change = input("old password", "new password");
    change.password_confirmation.clear();
    assert!(!change.valid());
}
#[tokio::test]
async fn postgres_change_revocation_rollback_and_sudo() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = db.pool.clone();
    sqlx::query(
        "CREATE TABLE test_authorizations(account_id uuid,kind text,revoked boolean DEFAULT false)",
    )
    .execute(&pool)
    .await?;
    let hash = password::hash("old password")?;
    let id:AccountId=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(format!("{}@example.test",AccountId::now_v7())).bind(&hash).fetch_one(&pool).await?;
    for kind in [
        "oauth_code",
        "oauth_access",
        "oauth_refresh",
        "device",
        "trusted_device",
    ] {
        sqlx::query("INSERT INTO test_authorizations(account_id,kind) VALUES($1,$2)")
            .bind(id)
            .bind(kind)
            .execute(&pool)
            .await?;
    }
    let token = vec![1u8; 32];
    for n in [1u8, 2] {
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')").bind(vec![n;32]).bind(id).bind("a".repeat(64)).execute(&pool).await?;
    }
    let notifications = Arc::new(oteryn_notify::Notifications {
        pool: pool.clone(),
        accounts: Arc::new(Accounts(crate::accounts::Accounts { pool: pool.clone() })),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![7; 32],
            "https://example.test/unsubscribe".into(),
        )
        .map_err(|e| e.to_string())?,
    });
    let mut service = Credentials {
        pool: pool.clone(),
        enabled: false,
        revoker: Arc::new(Revoke { fail: false }),
        notifications,
    };
    assert_eq!(
        service
            .change(&token, input("old password", "new password"))
            .await?,
        Outcome::Refused
    );
    service.enabled = true;
    assert_eq!(
        service
            .change(&token, input("bad password", "new password"))
            .await?,
        Outcome::Refused
    );
    assert_eq!(
        service
            .change(&token, input("old password", "short"))
            .await?,
        Outcome::Invalid
    );
    assert_eq!(
        service
            .change(&[9; 32], input("old password", "new password"))
            .await?,
        Outcome::Refused
    );
    assert_eq!(service.require_sudo(&token).await?, None);
    sqlx::query("UPDATE web_sessions SET sudo_until=now()+interval '10 minutes' WHERE id_hash=$1")
        .bind(&token)
        .execute(&pool)
        .await?;
    assert_eq!(service.require_sudo(&token).await?, Some(id));
    sqlx::query("UPDATE web_sessions SET sudo_until=now() WHERE id_hash=$1")
        .bind(&token)
        .execute(&pool)
        .await?;
    assert_eq!(service.require_sudo(&token).await?, None);
    service.revoker = Arc::new(Revoke { fail: true });
    assert!(
        service
            .change(&token, input("old password", "new password"))
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_authorizations WHERE revoked")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(stored, hash);
    service.revoker = Arc::new(UnavailableRevoker);
    assert!(
        service
            .change(&token, input("old password", "new password"))
            .await
            .is_err()
    );
    service.revoker = Arc::new(Revoke { fail: false });
    sqlx::query(
        "ALTER TABLE email_outbox ADD CONSTRAINT reject_test_email CHECK (false) NOT VALID",
    )
    .execute(&pool)
    .await?;
    assert!(
        service
            .change(&token, input("old password", "new password"))
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_authorizations WHERE revoked")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 2);
    sqlx::query("ALTER TABLE email_outbox DROP CONSTRAINT reject_test_email")
        .execute(&pool)
        .await?;
    assert_eq!(
        service
            .change(&token, input("old password", "new password"))
            .await?,
        Outcome::Changed
    );
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert!(password::verify("new password", &stored));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_authorizations WHERE revoked")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 5);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox WHERE account_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(generation, 2);
    assert_eq!(
        service
            .change(&token, input("new password", "next password"))
            .await?,
        Outcome::Refused
    );
    db.close().await?;
    Ok(())
}
