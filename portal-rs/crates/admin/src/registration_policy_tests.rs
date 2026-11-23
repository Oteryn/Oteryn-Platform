use super::*;
use crate::{rbac::Authorizer, web::PortalIdentity};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};

const FORM: &str = "revision=0&enabled=false&reason_pl=+Przerwa+&reason_en=+Maintenance+&work_bits=16&work_ttl_seconds=300&challenge_limit=5&attempt_limit=5&rate_window_seconds=60";

#[test]
fn validates_bilingual_closure_and_applies_numeric_bounds() -> Result<(), Box<dyn std::error::Error>>
{
    let (revision, closed) = serde_urlencoded::from_str::<Settings>(FORM)?.validate()?;
    assert_eq!(revision, 0);
    assert_eq!(closed.reason_pl, "Przerwa");
    assert_eq!(closed.reason_en, "Maintenance");
    assert!(!closed.enabled);
    for invalid in [
        FORM.replace("reason_pl=+Przerwa+", "reason_pl=+"),
        FORM.replace("reason_en=+Maintenance+", "reason_en=+"),
        FORM.replace(
            "reason_en=+Maintenance+",
            &format!("reason_en={}", "x".repeat(501)),
        ),
        FORM.replace("reason_en=+Maintenance+", "reason_en=A%00B"),
        FORM.replace("work_bits=16", "work_bits=15"),
        FORM.replace("work_bits=16", "work_bits=19"),
        FORM.replace("work_ttl_seconds=300", "work_ttl_seconds=901"),
        FORM.replace("challenge_limit=5", "challenge_limit=6"),
        FORM.replace("attempt_limit=5", "attempt_limit=0"),
        FORM.replace("rate_window_seconds=60", "rate_window_seconds=59"),
        FORM.replace("revision=0", "revision=-1"),
    ] {
        assert!(
            serde_urlencoded::from_str::<Settings>(&invalid)?
                .validate()
                .is_err()
        );
    }
    assert!(
        serde_urlencoded::from_str::<Settings>(&FORM.replace("enabled=false", "enabled=closed"))
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn saves_current_policy_with_confirmation_permission_and_history()
-> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("regpolicy_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,email_verified_at,password_hash,status,locale)
         VALUES('staff@example.test',now(),'fixture','active','en') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('signup_staff') RETURNING id")
            .fetch_one(&pool)
            .await?;
    for permission in [Permission::Access, Permission::ViewRegistrationPolicy] {
        sqlx::query("INSERT INTO role_permissions VALUES($1,$2)")
            .bind(role)
            .bind(permission.as_str())
            .execute(&pool)
            .await?;
    }
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(account)
        .bind(role)
        .execute(&pool)
        .await?;
    let proof = crate::test_staff::proof(&pool, account).await?;
    let context = CheckContext {
        account,
        mfa_verified: true,
        require_staff_mfa: true,
        session_proof: Some(proof),
    };
    let authority = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured(pool.clone()),
    )));
    let (revision, closed) = serde_urlencoded::from_str::<Settings>(FORM)?.validate()?;
    let attempt = || change(&pool, &authority, context, revision, closed.clone());
    assert!(matches!(
        attempt().await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    sqlx::query("INSERT INTO role_permissions VALUES($1,$2)")
        .bind(role)
        .bind(Permission::ManageRegistrationPolicy.as_str())
        .execute(&pool)
        .await?;
    assert!(matches!(attempt().await, Err(Error::ConfirmationRequired)));
    sqlx::query("UPDATE web_sessions SET sudo_until=now()+interval '10 minutes' WHERE id_hash=$1")
        .bind(proof.session_hash.as_slice())
        .execute(&pool)
        .await?;
    assert_eq!(policy::read(&pool).await?.revision, revision);
    attempt().await?;
    let saved = policy::read(&pool).await?;
    assert_eq!(saved.revision, revision + 1);
    assert_eq!(saved.values, closed);
    assert!(matches!(attempt().await, Err(Error::Stale)));
    change(&pool, &authority, context, saved.revision, closed.clone()).await?;
    let rows: Vec<(Uuid, Json<Policy>, Json<Policy>)> =
        sqlx::query_as("SELECT actor,before,after FROM audit_log WHERE action=$1 AND target=$2")
            .bind(CHANGE)
            .bind(TARGET)
            .fetch_all(&pool)
            .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, account);
    assert!(rows[0].1.reason_en.is_empty());
    assert_eq!(rows[0].2.0, closed);
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
