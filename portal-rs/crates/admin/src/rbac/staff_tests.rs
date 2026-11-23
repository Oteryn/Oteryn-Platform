use super::{Database, verified};
use crate::{
    rbac::{
        Error, MutationAuthorization,
        staff::{self, InvitationKey, InviteError, NewInvitation},
    },
    web::PortalIdentity,
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

async fn account(pool: &PgPool, email: &str, status: &str, verified: bool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO accounts(email,status,email_verified_at,password_hash,locale)
        VALUES($1,$2,CASE WHEN $3 THEN now() END,'fixture','en') RETURNING id",
    )
    .bind(email)
    .bind(status)
    .bind(verified)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn operator_grant_checks_account_and_role_and_audits_once() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let active = account(&db.pool, "active@example.invalid", "active", true).await;
    account(
        &db.pool,
        "pending@example.invalid",
        "pending_verification",
        false,
    )
    .await;
    account(&db.pool, "closed@example.invalid", "closed", true).await;
    for email in [
        "pending@example.invalid",
        "closed@example.invalid",
        "missing@example.invalid",
    ] {
        assert!(matches!(
            staff::operator_grant(&db.pool, email, "platform_admin").await,
            Err(Error::Denied)
        ));
    }
    assert!(matches!(
        staff::operator_grant(&db.pool, "invalid", "support").await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        staff::operator_grant(&db.pool, "active@example.invalid", "unknown_role").await,
        Err(Error::RoleNotFound)
    ));
    assert!(
        staff::operator_grant(&db.pool, " ACTIVE@example.invalid ", "platform_admin")
            .await
            .unwrap()
    );
    assert!(
        !staff::operator_grant(&db.pool, "active@example.invalid", "platform_admin")
            .await
            .unwrap()
    );
    let audits: Vec<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT actor,actor_source FROM audit_log WHERE action='rbac.role.granted' AND target=$1",
    )
    .bind(active.to_string())
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(audits, vec![(None, "operator-cli".to_owned())]);
    db.close().await;
}

#[tokio::test]
async fn invites_validate_scope_lifetime_and_recipient() {
    use crate::rbac::Authorizer;
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let pool = &db.pool;
    let admin = account(pool, "admin@example.invalid", "active", true).await;
    let foreign = account(pool, "foreign@example.invalid", "active", true).await;
    let restricted = account(pool, "restricted@example.invalid", "active", true).await;
    let admin_role = db.role("platform_admin").await;
    let support = db.role("support").await;
    let key = InvitationKey::from_secret(&"12".repeat(32)).unwrap();
    assert!(
        staff::operator_grant(pool, "admin@example.invalid", "platform_admin")
            .await
            .unwrap()
    );
    assert!(
        staff::operator_grant(pool, "restricted@example.invalid", "support")
            .await
            .unwrap()
    );
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured(pool.clone()),
    )));
    let mut actor = verified(admin);
    actor.session_proof = Some(crate::test_staff::proof(pool, admin).await.unwrap());
    let mut lower_actor = verified(restricted);
    lower_actor.session_proof = Some(crate::test_staff::proof(pool, restricted).await.unwrap());
    let input = |email, role, days| NewInvitation {
        email,
        role,
        days,
        locale: "en",
    };

    for (email, days) in [
        ("invalid", 1),
        ("recipient@example.invalid", 0),
        ("recipient@example.invalid", 8),
    ] {
        assert!(matches!(
            staff::create(pool, &auth, actor, &key, input(email, support, days)).await,
            Err(InviteError::Rbac(Error::InvalidInput))
        ));
    }
    for (context, email, role) in [
        (actor, "recipient@example.invalid", admin_role),
        (actor, "admin@example.invalid", support),
        (lower_actor, "recipient@example.invalid", support),
    ] {
        assert!(matches!(
            staff::create(pool, &auth, context, &key, input(email, role, 1)).await,
            Err(InviteError::Rbac(Error::Denied))
        ));
    }

    // A role that can create invitations is still limited by its other grants.
    sqlx::query(
        "INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.staff_invites.create')",
    )
    .bind(support)
    .execute(pool)
    .await
    .unwrap();
    assert!(matches!(
        staff::create(
            pool,
            &auth,
            lower_actor,
            &key,
            input(
                "recipient@example.invalid",
                db.role("content_editor").await,
                1
            )
        )
        .await,
        Err(InviteError::Rbac(Error::Denied))
    ));

    let id = staff::create(
        pool,
        &auth,
        actor,
        &key,
        input("recipient@example.invalid", support, 7),
    )
    .await
    .unwrap();
    let token = key.token(id);
    assert!(staff::valid_token(&token));
    assert_ne!(key.token(Uuid::now_v7()), token);
    assert_eq!(staff::pending(pool, admin).await.unwrap().len(), 1);
    assert!(staff::pending(pool, foreign).await.unwrap().is_empty());
    assert!(matches!(
        staff::create(
            pool,
            &auth,
            actor,
            &key,
            input("recipient@example.invalid", support, 1)
        )
        .await,
        Err(InviteError::Duplicate)
    ));
    let payload: serde_json::Value = sqlx::query_scalar("SELECT payload FROM outbox WHERE kind=$1")
        .bind(staff::MAIL_JOB)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(payload, serde_json::json!({"invitation": id}));
    let message = staff::mail(pool, &key, id, "https://portal.example.invalid")
        .await
        .unwrap()
        .unwrap();
    assert!(message.body.ends_with(&token));

    let foreign_proof = crate::test_staff::proof(pool, foreign).await.unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(admin)
        .execute(&mut *writer)
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            staff::accept(pool, foreign, &foreign_proof.session_hash, &token),
        )
        .await,
        Ok(Err(InviteError::Unavailable))
    ));
    writer.rollback().await.unwrap();
    let recipient = account(pool, "recipient@example.invalid", "active", true).await;
    let session_hash = oteryn_identity::verify::token_hash(&Uuid::now_v7().to_string());
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,
                                  user_agent_summary,csrf_token,authorization_generation)
         SELECT $1,id,now()+interval '1 hour',now()+interval '1 hour','invite-fixture',$2,
                native_security_generation FROM accounts WHERE id=$3",
    )
    .bind(&session_hash)
    .bind("b".repeat(64))
    .bind(recipient)
    .execute(pool)
    .await
    .unwrap();
    assert!(matches!(
        staff::accept(pool, recipient, &session_hash, &token).await,
        Err(InviteError::Mfa)
    ));
    let expiring_proof = crate::test_staff::proof(pool, recipient).await.unwrap();

    sqlx::raw_sql(
        "CREATE SEQUENCE staff_acceptance_fixture;
         CREATE FUNCTION staff_acceptance_fixture() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             PERFORM nextval('staff_acceptance_fixture');
             PERFORM pg_sleep(2);
             RETURN NEW;
         END $$;
         CREATE TRIGGER staff_acceptance_fixture BEFORE INSERT ON audit_log
         FOR EACH ROW WHEN (NEW.action='rbac.invitation.accepted')
         EXECUTE FUNCTION staff_acceptance_fixture();",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE web_sessions SET idle_expires_at=clock_timestamp()+interval '1.5 seconds'
         WHERE id_hash=$1",
    )
    .bind(expiring_proof.session_hash.as_slice())
    .execute(pool)
    .await
    .unwrap();
    let result = staff::accept(pool, recipient, &expiring_proof.session_hash, &token).await;
    let reached_audit: bool = sqlx::query_scalar("SELECT is_called FROM staff_acceptance_fixture")
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(reached_audit, "{result:?}");
    assert!(
        matches!(result, Err(InviteError::Unavailable)),
        "{result:?}"
    );
    sqlx::query("DROP TRIGGER staff_acceptance_fixture ON audit_log")
        .execute(pool)
        .await
        .unwrap();
    let proof = crate::test_staff::proof(pool, recipient).await.unwrap();

    let revoked = staff::create(
        pool,
        &auth,
        actor,
        &key,
        input("foreign@example.invalid", support, 1),
    )
    .await
    .unwrap();
    assert!(matches!(
        staff::revoke(pool, &auth, lower_actor, revoked).await,
        Err(InviteError::Rbac(Error::Denied))
    ));
    staff::revoke(pool, &auth, actor, revoked).await.unwrap();
    assert!(
        staff::mail(pool, &key, revoked, "https://portal.example.invalid")
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        staff::accept(
            pool,
            foreign,
            &foreign_proof.session_hash,
            &key.token(revoked)
        )
        .await,
        Err(InviteError::Unavailable)
    ));

    let expired = staff::create(
        pool,
        &auth,
        actor,
        &key,
        input("foreign@example.invalid", support, 1),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE staff_invitations SET created_at=now()-interval '2 days',expires_at=now()-interval '1 day' WHERE id=$1")
        .bind(expired).execute(pool).await.unwrap();
    assert!(matches!(
        staff::accept(
            pool,
            foreign,
            &foreign_proof.session_hash,
            &key.token(expired)
        )
        .await,
        Err(InviteError::Unavailable)
    ));
    sqlx::query(
        "INSERT INTO staff_invitations(inviter,email,role_id,locale,token_hash,expires_at)
        SELECT $1,'other@example.invalid',$2,'en',sha256(i::text::bytea),now()+interval '1 hour'
        FROM generate_series(1,50) i",
    )
    .bind(admin)
    .bind(support)
    .execute(pool)
    .await
    .unwrap();
    assert!(matches!(
        staff::create(
            pool,
            &auth,
            actor,
            &key,
            input("foreign@example.invalid", support, 1)
        )
        .await,
        Err(InviteError::Limit)
    ));

    let (first, second) = tokio::join!(
        staff::accept(pool, recipient, &proof.session_hash, &token),
        staff::accept(pool, recipient, &proof.session_hash, &token),
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(InviteError::Unavailable)) | (Err(InviteError::Unavailable), Ok(()))
    ));
    let roles: Vec<Uuid> =
        sqlx::query_scalar("SELECT role_id FROM account_roles WHERE account_id=$1")
            .bind(recipient)
            .fetch_all(pool)
            .await
            .unwrap();
    assert_eq!(roles, vec![support]);
    let accepted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE target=$1 AND action='rbac.invitation.accepted'",
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(accepted, 1);
    db.close().await;
}

#[test]
fn i18n_parity_includes_each_invitation_message() {
    let sources = [
        ("pl", include_str!("../../../../templates/admin/pl.ftl")),
        ("en", include_str!("../../../../templates/admin/en.ftl")),
    ];
    let keys = |source: &'static str| {
        source
            .lines()
            .filter_map(|line| line.split_once(" = "))
            .map(|(key, _)| key)
            .filter(|key| key.starts_with("invite-"))
            .collect::<Vec<_>>()
    };
    let labels = keys(sources[0].1);
    assert_eq!(labels, keys(sources[1].1));
    assert!(labels.len() >= 20);
    for (lang, _) in sources {
        for key in &labels {
            assert!(!crate::web::text(lang, key).is_empty(), "{lang}/{key}");
        }
    }
}
