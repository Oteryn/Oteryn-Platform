use super::*;
use crate::web::PortalIdentity;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Fixture {
    root: PgPool,
    schema: String,
    invites: Invites<PortalIdentity>,
    key: DeliveryKey,
    inviter: CheckContext,
    recipient: CheckContext,
    role: Uuid,
}

impl Fixture {
    async fn open() -> TestResult<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPool::connect(&url).await?;
        let schema = format!("staff_invites_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .after_connect(move |connection, _| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(search_path))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;

        let inviter = account(&pool, "manager@example.invalid").await?;
        let recipient = account(&pool, "recipient@example.invalid").await?;
        let role = sqlx::query_scalar("SELECT id FROM roles WHERE key='support'")
            .fetch_one(&pool)
            .await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('platform_admin','support')")
            .bind(inviter.account).execute(&pool).await?;

        let identity = PortalIdentity::configured(pool.clone());
        Ok(Self {
            root,
            schema,
            invites: Invites {
                auth: Authorizer::new(pool.clone(), identity),
                pool,
            },
            key: DeliveryKey::parse(&"ab".repeat(32)).ok_or("test delivery key")?,
            inviter,
            recipient,
            role,
        })
    }

    async fn create(&self) -> TestResult<(Uuid, String)> {
        let id = self
            .invites
            .create(
                self.inviter,
                &self.key,
                " Recipient@Example.Invalid ",
                vec![self.role],
                "en",
            )
            .await?;
        let payload: String = sqlx::query_scalar(
            "SELECT payload::text FROM outbox WHERE kind=$1 AND payload->>'invite'=$2",
        )
        .bind(MAIL_KIND)
        .bind(id.to_string())
        .fetch_one(&self.invites.pool)
        .await?;
        let (email, locale, token) = delivery(&self.invites.pool, &self.key, &payload)
            .await?
            .ok_or("invite should be deliverable")?;
        assert_eq!(email, "recipient@example.invalid");
        assert_eq!(locale, "en");
        assert!(!payload.contains(&token), "outbox stores only ciphertext");
        Ok((id, token))
    }

    async fn memberships(&self) -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM account_roles WHERE account_id=$1")
                .bind(self.recipient.account)
                .fetch_one(&self.invites.pool)
                .await?,
        )
    }

    async fn actions(&self, id: Uuid) -> TestResult<Vec<(String, Uuid)>> {
        Ok(sqlx::query_as(
            "SELECT action,actor FROM audit_log WHERE target=$1 ORDER BY created_at,id",
        )
        .bind(id.to_string())
        .fetch_all(&self.invites.pool)
        .await?)
    }

    async fn close(self) -> TestResult {
        self.invites.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        Ok(())
    }
}

async fn account(pool: &PgPool, email: &str) -> TestResult<CheckContext> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,status,email_verified_at,password_hash,locale) VALUES($1,$2,'active',now(),'unused','en')")
        .bind(id).bind(email).execute(pool).await?;
    let mut context = CheckContext::new(id);
    context.mfa_verified = true;
    context.session_proof = Some(crate::test_staff::proof(pool, id).await?);
    Ok(context)
}

#[tokio::test]
async fn success_requires_mfa_and_consumes_once() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (id, token) = f.create().await?;
    let expiry: bool = sqlx::query_scalar(
        "SELECT expires_at-created_at=interval '48 hours' FROM staff_invites WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&f.invites.pool)
    .await?;
    assert!(expiry);
    assert_eq!(f.memberships().await?, 0);
    let mut without_mfa = f.recipient;
    without_mfa.session_proof = None;
    assert!(matches!(
        f.invites.accept(without_mfa, &token).await,
        Err(Error::Denied)
    ));

    // Both requests race the real database mutation. Exactly one can succeed.
    let (first, second) = tokio::join!(
        f.invites.accept(f.recipient, &token),
        f.invites.accept(f.recipient, &token)
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(Error::Denied)) | (Err(Error::Denied), Ok(()))
    ));
    assert_eq!(f.memberships().await?, 1);
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    assert_eq!(
        f.actions(id).await?,
        [
            ("staff.invite.created".into(), f.inviter.account),
            ("staff.invite.accepted".into(), f.recipient.account),
        ]
    );
    f.close().await
}

#[tokio::test]
async fn escalation_is_refused_at_issue_and_after_loss_of_role() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (admin, other): (Uuid, Uuid) = sqlx::query_as(
        "SELECT a.id,b.id FROM roles a CROSS JOIN roles b WHERE a.key='platform_admin' AND b.key='content_editor'",
    ).fetch_one(&f.invites.pool).await?;
    assert!(matches!(
        f.invites
            .create(
                f.inviter,
                &f.key,
                "recipient@example.invalid",
                vec![other],
                "en"
            )
            .await,
        Err(Error::Denied)
    ));
    let (_, token) = f.create().await?;
    // The inviter keeps a separate management role and loses only their
    // platform admin and support memberships.
    let manager: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('invite_manager') RETURNING id")
            .fetch_one(&f.invites.pool)
            .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.manage'),($1,'admin.access')")
        .bind(manager).execute(&f.invites.pool).await?;
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(f.inviter.account)
        .execute(&f.invites.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(f.inviter.account)
        .bind(manager)
        .execute(&f.invites.pool)
        .await?;
    assert!(matches!(
        f.invites
            .create(
                f.inviter,
                &f.key,
                "recipient@example.invalid",
                vec![admin],
                "en"
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    assert_eq!(f.memberships().await?, 0);
    f.close().await
}

#[tokio::test]
async fn expired_revoked_and_wrong_account_are_refused() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (expired, expired_token) = f.create().await?;
    sqlx::query("UPDATE staff_invites SET created_at=now()-interval '49 hours',expires_at=now()-interval '1 hour' WHERE id=$1")
        .bind(expired).execute(&f.invites.pool).await?;
    assert!(matches!(
        f.invites.accept(f.recipient, &expired_token).await,
        Err(Error::Denied)
    ));

    let (revoked, revoked_token) = f.create().await?;
    f.invites.revoke(f.inviter, revoked).await?;
    assert!(matches!(
        f.invites.accept(f.recipient, &revoked_token).await,
        Err(Error::Denied)
    ));
    assert_eq!(
        f.actions(revoked).await?,
        [
            ("staff.invite.created".into(), f.inviter.account),
            ("staff.invite.revoked".into(), f.inviter.account),
        ]
    );

    let (wrong, wrong_token) = f.create().await?;
    let wrong_account = account(&f.invites.pool, "other@example.invalid").await?;
    assert!(matches!(
        f.invites.accept(wrong_account, &wrong_token).await,
        Err(Error::Denied)
    ));
    assert_eq!(
        f.actions(wrong).await?,
        [("staff.invite.created".into(), f.inviter.account)]
    );
    assert_eq!(f.memberships().await?, 0);
    f.invites.accept(f.recipient, &wrong_token).await?;
    f.close().await
}
