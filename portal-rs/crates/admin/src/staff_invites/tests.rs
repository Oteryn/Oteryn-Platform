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
                Uuid::now_v7(),
            )
            .await?;
        let payload: String = sqlx::query_scalar(
            "SELECT payload::text FROM outbox WHERE kind=$1 AND payload->>'invite'=$2",
        )
        .bind(MAIL_KIND)
        .bind(id.to_string())
        .fetch_one(&self.invites.pool)
        .await?;
        let DeliveryDecision::Send {
            email,
            locale,
            token,
            ..
        } = delivery(&self.invites.pool, &self.key, &payload).await?
        else {
            return Err("invite should be deliverable".into());
        };
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

    let proof = f.recipient.session_proof.ok_or("missing proof")?;
    // Include a remembered device, pending challenges, and a grant from
    // another session, all bound to the recipient's current credentials.
    let previous_tokens = sqlx::query(
        "INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
         SELECT sha256(convert_to(credential.purpose,'UTF8')),t.account_id,t.generation,t.credential_hash,
                credential.purpose,t.expires_at
         FROM account_mfa_tokens t CROSS JOIN unnest(ARRAY['trusted','challenge','email','grant']) credential(purpose)
         WHERE t.token_hash=$1",
    )
    .bind(proof.grant_hash.as_slice())
    .execute(&f.invites.pool)
    .await?;
    assert_eq!(previous_tokens.rows_affected(), 4);
    let old_csrf: String =
        sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE id_hash=$1")
            .bind(proof.session_hash.as_slice())
            .fetch_one(&f.invites.pool)
            .await?;
    // Both requests race the real database mutation. Exactly one can succeed.
    let (first, second) = tokio::join!(
        f.invites.accept(f.recipient, &token),
        f.invites.accept(f.recipient, &token)
    );
    assert!(matches!(
        (first, second),
        (Ok(_), Err(Error::Denied)) | (Err(Error::Denied), Ok(_))
    ));
    let (new_id, new_csrf, same_grant): (Vec<u8>, String, bool) = sqlx::query_as(
        "SELECT id_hash,csrf_token,sha256(convert_to(data->>'mfa_grant','UTF8'))=$2
         FROM web_sessions WHERE account_id=$1",
    )
    .bind(f.recipient.account)
    .bind(proof.grant_hash.as_slice())
    .fetch_one(&f.invites.pool)
    .await?;
    assert!(new_id != proof.session_hash && new_csrf != old_csrf && same_grant);
    let surviving_tokens: Vec<(Vec<u8>, String)> =
        sqlx::query_as("SELECT token_hash,purpose FROM account_mfa_tokens WHERE account_id=$1")
            .bind(f.recipient.account)
            .fetch_all(&f.invites.pool)
            .await?;
    assert_eq!(surviving_tokens, [(proof.grant_hash.to_vec(), "grant".into())]);
    assert!(
        !rbac::durable_session_in(
            &mut *f.invites.pool.acquire().await?,
            f.recipient.account,
            proof
        )
        .await
        .map_err(|_| "durable session lookup failed")?
    );
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
                "en",
                Uuid::now_v7(),
            )
            .await,
        Err(Error::Denied)
    ));
    let (id, token) = f.create().await?;
    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind=$1 AND idempotency_key=$2")
            .bind(MAIL_KIND)
            .bind(id.to_string())
            .fetch_one(&f.invites.pool)
            .await?;
    // The inviter retains management access but loses the requested role.
    let manager: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('invite_manager') RETURNING id")
            .fetch_one(&f.invites.pool)
            .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.manage'),($1,'admin.access')")
        .bind(manager).execute(&f.invites.pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(f.inviter.account)
        .bind(manager)
        .execute(&f.invites.pool)
        .await?;
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=$2")
        .bind(f.inviter.account)
        .bind(f.role)
        .execute(&f.invites.pool)
        .await?;
    assert!(matches!(
        delivery(&f.invites.pool, &f.key, &payload).await?,
        DeliveryDecision::Closed
    ));
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    // Restoring the requested role while removing management access also
    // leaves the issuer ineligible to delegate it.
    sqlx::query("UPDATE account_roles SET role_id=$2 WHERE account_id=$1 AND role_id=$3")
        .bind(f.inviter.account)
        .bind(f.role)
        .bind(admin)
        .execute(&f.invites.pool)
        .await?;
    assert!(matches!(
        delivery(&f.invites.pool, &f.key, &payload).await?,
        DeliveryDecision::Send { .. }
    ));
    assert!(matches!(
        f.invites
            .create(
                f.inviter,
                &f.key,
                "recipient@example.invalid",
                vec![admin],
                "en",
                Uuid::now_v7(),
            )
            .await,
        Err(Error::Denied)
    ));
    sqlx::query("DELETE FROM role_permissions WHERE role_id=$1 AND permission='admin.roles.manage'")
        .bind(manager)
        .execute(&f.invites.pool)
        .await?;
    assert!(matches!(
        delivery(&f.invites.pool, &f.key, &payload).await?,
        DeliveryDecision::Closed
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

#[tokio::test]
async fn privileged_invite_requires_independent_current_authority() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let pool = &f.invites.pool;
    let platform: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(pool)
        .await?;
    let id = f
        .invites
        .create(
            f.inviter,
            &f.key,
            "recipient@example.invalid",
            vec![platform],
            "en",
            Uuid::now_v7(),
        )
        .await?;
    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind=$1 AND idempotency_key=$2")
            .bind(MAIL_KIND)
            .bind(id.to_string())
            .fetch_one(pool)
            .await?;
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::AwaitingApproval
    ));
    let job: Delivery = serde_json::from_str(&payload)?;
    let token = f.key.open(&job.ciphertext).ok_or("missing token")?;
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        f.invites.approve(f.inviter, id).await,
        Err(Error::Denied)
    ));
    assert_eq!(f.memberships().await?, 0);

    let second = account(pool, "approver@example.invalid").await?;
    assert!(matches!(
        f.invites.approve(second, id).await,
        Err(Error::Denied)
    ));
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2),($3,$2)")
        .bind(second.account)
        .bind(platform)
        .bind(f.recipient.account)
        .execute(pool)
        .await?;
    assert!(matches!(
        f.invites.approve(f.recipient, id).await,
        Err(Error::Denied)
    ));
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(f.recipient.account)
        .execute(pool)
        .await?;
    f.invites.approve(second, id).await?;
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::Send { .. }
    ));

    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(second.account)
        .execute(pool)
        .await?;
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(second.account)
        .bind(platform)
        .execute(pool)
        .await?;
    f.invites.accept(f.recipient, &token).await?;
    assert_eq!(f.memberships().await?, 1);
    assert_eq!(
        f.actions(id).await?,
        [
            ("staff.invite.created".into(), f.inviter.account),
            ("staff.invite.approved".into(), second.account),
            ("staff.invite.accepted".into(), f.recipient.account),
        ]
    );
    f.close().await
}

#[tokio::test]
async fn issuance_retry_and_visibility_are_scoped_to_the_issuer() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let pool = &f.invites.pool;
    let other = account(pool, "other-manager@example.invalid").await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
        .bind(other.account).execute(pool).await?;
    let request = Uuid::now_v7();
    let issue = || {
        f.invites.create(
            f.inviter,
            &f.key,
            "recipient@example.invalid",
            vec![f.role],
            "en",
            request,
        )
    };
    let (first, retry) = tokio::join!(issue(), issue());
    let id = first?;
    assert_eq!(retry?, id);
    let pending = pending_for(pool, f.inviter.account, None).await?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, id);
    assert!(pending_for(pool, other.account, None).await?.is_empty());
    assert!(matches!(
        f.invites.revoke(other, id).await,
        Err(Error::Denied)
    ));
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox WHERE kind=$1")
        .bind(MAIL_KIND)
        .fetch_one(pool)
        .await?;
    assert_eq!(jobs, 1);
    f.invites.revoke(f.inviter, id).await?;
    assert_eq!(issue().await?, id);
    assert!(pending_for(pool, f.inviter.account, None).await?.is_empty());
    f.close().await
}

#[tokio::test]
async fn redemption_bounds_the_first_invitation_read() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (_, token) = f.create().await?;
    let mut ddl = f.invites.pool.begin().await?;
    sqlx::query("LOCK TABLE staff_invites IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *ddl)
        .await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        f.invites.accept(f.recipient, &token),
    )
    .await?;
    assert!(
        matches!(result, Err(Error::Storage(sqlx::Error::Database(error)))
        if matches!(error.code().as_deref(), Some("55P03" | "57014")))
    );
    ddl.rollback().await?;
    assert_eq!(f.memberships().await?, 0);
    f.invites.accept(f.recipient, &token).await?;
    f.close().await
}

#[tokio::test]
async fn pending_invite_follows_role_reclassification() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (id, token) = f.create().await?;
    let pool = &f.invites.pool;
    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind=$1 AND idempotency_key=$2")
            .bind(MAIL_KIND)
            .bind(id.to_string())
            .fetch_one(pool)
            .await?;
    assert!(!pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.manage')")
        .bind(f.role)
        .execute(pool)
        .await?;
    assert!(pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::AwaitingApproval
    ));
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    let second = account(pool, "approver@example.invalid").await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(second.account)
        .bind(f.role)
        .execute(pool)
        .await?;
    f.invites.approve(second, id).await?;
    assert!(!pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::Send { .. }
    ));
    f.invites.accept(f.recipient, &token).await?;
    assert_eq!(f.memberships().await?, 1);
    f.close().await
}

#[tokio::test]
async fn stale_approval_blocks_delivery_and_can_be_replaced() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let f = Fixture::open().await?;
    let (id, token) = f.create().await?;
    let pool = &f.invites.pool;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.manage')")
        .bind(f.role)
        .execute(pool)
        .await?;

    let original = account(pool, "original-approver@example.invalid").await?;
    let replacement = account(pool, "replacement-approver@example.invalid").await?;
    // Both approvers also hold a separate role that keeps their management
    // permissions when the original approver loses the invited support role.
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id)
         SELECT manager.id,r.id FROM unnest($1::uuid[]) AS manager(id)
         CROSS JOIN roles r WHERE r.key='platform_admin'",
    )
    .bind([original.account, replacement.account].as_slice())
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$3),($2,$3)")
        .bind(original.account)
        .bind(replacement.account)
        .bind(f.role)
        .execute(pool)
        .await?;
    f.invites.approve(original, id).await?;
    assert!(matches!(
        f.invites.approve(replacement, id).await,
        Err(Error::Denied)
    ));

    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind=$1 AND idempotency_key=$2")
            .bind(MAIL_KIND)
            .bind(id.to_string())
            .fetch_one(pool)
            .await?;
    assert!(!pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::Send { .. }
    ));

    sqlx::query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=$2")
        .bind(original.account)
        .bind(f.role)
        .execute(pool)
        .await?;
    assert!(pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    assert!(matches!(
        delivery(pool, &f.key, &payload).await?,
        DeliveryDecision::AwaitingApproval
    ));
    assert!(matches!(
        f.invites.accept(f.recipient, &token).await,
        Err(Error::Denied)
    ));
    assert_eq!(f.memberships().await?, 0);

    // The original 48-hour window has only one hour remaining at reapproval.
    let expiry: OffsetDateTime = sqlx::query_scalar(
        "UPDATE staff_invites SET created_at=statement_timestamp()-interval '47 hours',
         expires_at=statement_timestamp()+interval '1 hour' WHERE id=$1 RETURNING expires_at",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    sqlx::query("UPDATE outbox SET run_after=$3 WHERE kind=$1 AND idempotency_key=$2")
        .bind(MAIL_KIND)
        .bind(id.to_string())
        .bind(expiry)
        .execute(pool)
        .await?;
    let (first, second) = tokio::join!(
        f.invites.approve(replacement, id),
        f.invites.approve(replacement, id),
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(Error::Denied)) | (Err(Error::Denied), Ok(()))
    ));

    assert!(!pending_for(pool, f.inviter.account, None).await?[0].awaiting_approval);
    assert!(
        matches!(delivery(pool, &f.key, &payload).await?, DeliveryDecision::Send { expires_at, .. } if expires_at == expiry)
    );
    let ready: bool = sqlx::query_scalar(
        "SELECT state='pending' AND run_after<=clock_timestamp() FROM outbox WHERE kind=$1 AND idempotency_key=$2",
    ).bind(MAIL_KIND).bind(id.to_string()).fetch_one(pool).await?;
    assert!(ready);
    f.invites.accept(f.recipient, &token).await?;
    assert_eq!(f.memberships().await?, 1);
    assert_eq!(
        f.actions(id).await?,
        [
            ("staff.invite.created".into(), f.inviter.account),
            ("staff.invite.approved".into(), original.account),
            ("staff.invite.approved".into(), replacement.account),
            ("staff.invite.accepted".into(), f.recipient.account),
        ],
    );
    f.close().await
}

#[test]
fn invitation_mail_localizes_the_actual_utc_expiry() -> TestResult {
    let expires_at = OffsetDateTime::parse(
        "2026-11-24T14:42:00+02:00",
        &time::format_description::well_known::Rfc3339,
    )?;
    let (_, en) = mail_text("en", expires_at);
    let (_, pl) = mail_text("pl", expires_at);
    assert!(en.starts_with("This invitation expires on "));
    assert!(en.contains("2026-11-24 12:42:00 UTC"));
    assert!(pl.starts_with("Zaproszenie wygasa "));
    assert!(pl.contains("24.11.2026 12:42:00 UTC"));
    assert!(!en.contains("48 hours") && !pl.contains("48 godzin"));
    Ok(())
}
