#![allow(clippy::unwrap_used)]

use super::model::{Action, DraftForm, MessageLocale, check_review};
use super::*;
use uuid::Uuid;

fn input(action: Action) -> DraftForm {
    DraftForm {
        _csrf: "browser".into(),
        id: Uuid::now_v7(),
        world: Uuid::now_v7(),
        action,
        starts: "2026-12-01T12:00".into(),
        ends: "2026-12-01T13:00".into(),
        ticket: "OPS-41".into(),
        target: None,
        event: None,
        locale: None,
        message: None,
    }
}

#[test]
fn valid_windows_and_operation_specific_fields() {
    let mut form = input(Action::Restart);
    assert!(matches!(form.validate(), Err(Error::Invalid)));
    form.target = Some(Uuid::now_v7());
    let (start, end) = form.validate().unwrap();
    assert_eq!(end - start, 3600);
    form.starts = "2026-02-29T09:00".into();
    assert!(matches!(form.validate(), Err(Error::Invalid)));

    let mut form = input(Action::EventStop);
    assert!(matches!(form.validate(), Err(Error::Invalid)));
    form.event = Some(Uuid::now_v7());
    assert!(form.validate().is_ok());

    let mut form = input(Action::Broadcast);
    form.locale = Some(MessageLocale::Pl);
    form.message = Some("Przerwa techniczna".into());
    assert!(form.validate().is_ok());
    form.message = Some("x".repeat(501));
    assert!(matches!(form.validate(), Err(Error::Invalid)));
    form.message = Some("x\ny".into());
    assert!(matches!(form.validate(), Err(Error::Invalid)));
}

#[test]
fn window_review_requires_a_distinct_actor_and_current_window() {
    let requester = Uuid::now_v7();
    let reviewer = Uuid::now_v7();
    assert!(check_review(requester, reviewer, 100, 99, false).is_ok());
    assert!(matches!(
        check_review(requester, requester, 100, 99, false),
        Err(Error::Denied)
    ));
    assert!(matches!(
        check_review(requester, reviewer, 100, 100, false),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        check_review(requester, reviewer, 100, 99, true),
        Err(Error::Conflict)
    ));
}

#[tokio::test]
async fn only_current_scoped_staff_can_record_a_window_decision()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        rbac::{Authorizer, CheckContext},
        web::PortalIdentity,
    };
    use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await? else {
        return Ok(());
    };
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("world_ops_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(search_path))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;

    async fn staff(pool: &PgPool, email: &str) -> Result<CheckContext, sqlx::Error> {
        let account = sqlx::query_scalar(
            "INSERT INTO accounts(email,password_hash,locale,status,email_verified_at)
             VALUES($1,'fixture','en','active',now()) RETURNING id",
        )
        .bind(email)
        .fetch_one(pool)
        .await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
            .bind(account)
            .execute(pool)
            .await?;
        Ok(CheckContext {
            account,
            require_staff_mfa: true,
            mfa_verified: true,
            session_proof: Some(crate::test_staff::proof(pool, account).await?),
        })
    }

    let outcome: Result<(), Box<dyn std::error::Error>> = async {
        let proposer = staff(&pool, "proposer@example.invalid").await?;
        let reviewer = staff(&pool, "reviewer@example.invalid").await?;
        let scoped_reader = staff(&pool, "reader@example.invalid").await?;
        let auth = Authorizer::new(pool.clone(), PortalIdentity::configured_from(pool.clone(), &|_| None));
        let world: Uuid = sqlx::query_scalar(
            "INSERT INTO world_status(id,name) VALUES (uuidv7(),'Test world') RETURNING id",
        )
        .fetch_one(&pool)
        .await?;
        let grant = "INSERT INTO world_operation_planning_grants(account_id,world_id,action,can_review)
             VALUES ($1,$4,'restart',true),($2,$4,'restart',true),($3,$4,'restart',false)";
        sqlx::query(grant)
            .bind(proposer.account)
            .bind(reviewer.account)
            .bind(scoped_reader.account)
            .bind(world)
            .execute(&pool)
            .await?;

        let (starts, ends): (String, String) = sqlx::query_as(
            "SELECT to_char(timezone('UTC',now())+interval '1 hour','YYYY-MM-DD\"T\"HH24:MI'),
                    to_char(timezone('UTC',now())+interval '2 hours','YYYY-MM-DD\"T\"HH24:MI')",
        )
        .fetch_one(&pool)
        .await?;
        let mut draft = input(Action::Restart);
        draft.target = Some(Uuid::now_v7());
        draft.starts = starts.clone();
        draft.ends = ends.clone();
        assert!(matches!(
            store::create(&pool, &auth, proposer, draft).await,
            Err(Error::Denied)
        ));

        let mut draft = input(Action::Restart);
        draft.world = world;
        draft.target = Some(Uuid::now_v7());
        draft.starts = starts;
        draft.ends = ends;
        let id = store::create(&pool, &auth, proposer, draft).await?;
        assert!(matches!(store::review(&pool, &auth, proposer, id, true).await, Err(Error::Denied)));
        assert!(matches!(store::review(&pool, &auth, scoped_reader, id, true).await, Err(Error::Denied)));

        sqlx::query("DELETE FROM world_operation_planning_grants WHERE account_id=$1")
            .bind(proposer.account)
            .execute(&pool)
            .await?;
        assert!(matches!(store::plan(&pool, proposer.account, id).await, Err(Error::NotFound)));
        assert!(matches!(store::review(&pool, &auth, reviewer, id, true).await, Err(Error::Denied)));
        sqlx::query("INSERT INTO world_operation_planning_grants(account_id,world_id,action) VALUES ($1,$2,'restart')")
            .bind(proposer.account)
            .bind(world)
            .execute(&pool)
            .await?;

        store::review(&pool, &auth, reviewer, id, true).await?;
        assert!(matches!(store::review(&pool, &auth, reviewer, id, false).await, Err(Error::Conflict)));
        let plan = store::plan(&pool, proposer.account, id).await?;
        assert_eq!(plan.reviewer, Some(reviewer.account));
        assert_eq!(plan.status(plan.starts), "ops-approved");
        assert_eq!(plan.status(plan.ends), "ops-expired");
        let actors: Vec<Uuid> = sqlx::query_scalar(
            "SELECT actor FROM audit_log WHERE target=$1 AND action IN
             ('admin.world_operations.proposed','admin.world_operations.reviewed') ORDER BY action",
        )
        .bind(id.to_string())
        .fetch_all(&pool)
        .await?;
        assert_eq!(actors, [proposer.account, reviewer.account]);
        Ok(())
    }
    .await;
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    outcome
}
