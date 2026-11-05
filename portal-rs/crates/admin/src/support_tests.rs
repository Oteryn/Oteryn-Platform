#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::rbac::{AccountState, AccountStateUnavailable};
#[test]
fn transitions_success_refusal_and_bad_input() {
    let reply = Command::Reply {
        body: "hello".into(),
        internal: true,
    };
    assert_eq!(reply.transition("open"), Ok(()));
    assert_eq!(reply.transition("closed"), Err(Error::Conflict));
    assert_eq!(reply.transition("unknown"), Err(Error::Conflict));
    assert_eq!(
        Command::Reply {
            body: " ".into(),
            internal: false
        }
        .validate(),
        Err(Error::Invalid)
    );
    assert_eq!(
        Command::Reply {
            body: "a".repeat(8001),
            internal: false
        }
        .validate(),
        Err(Error::Invalid)
    );
    let report = Command::Moderate {
        status: "actioned".into(),
        public_outcome: "Reviewed".into(),
        moderator_notes: "private".into(),
    };
    assert_eq!(report.transition("submitted"), Ok(()));
    assert_eq!(report.transition("closed"), Err(Error::Conflict));
    let missing = Command::Moderate {
        status: "rejected".into(),
        public_outcome: String::new(),
        moderator_notes: String::new(),
    };
    assert_eq!(missing.validate(), Err(Error::Invalid));
    let appeal = Command::Appeal {
        appeal_status: "accepted".into(),
        appeal_outcome: "Revoked".into(),
    };
    assert_eq!(appeal.transition("requested"), Ok(()));
    assert_eq!(appeal.transition("accepted"), Err(Error::Conflict));
    assert_eq!(appeal.transition("none"), Err(Error::Conflict));
}
#[test]
fn temporary_sanctions_only() {
    let command = Command::Sanction {
        create: false,
        status: "active".into(),
        target: Uuid::now_v7(),
        category: "suspension".into(),
        public_reason: "Reason".into(),
        moderator_notes: String::new(),
        effective_at: 100,
        expires_at: 101,
    };
    assert_eq!(command.validate(), Ok(()));
    assert_eq!(command.transition("unknown"), Err(Error::Conflict));
    let mut creation = command.clone();
    if let Command::Sanction { create, .. } = &mut creation {
        *create = true;
    }
    assert_eq!(creation.transition("absent"), Ok(()));
    assert_eq!(creation.transition("active"), Err(Error::Conflict));

    let Command::Sanction {
        target,
        category,
        public_reason,
        moderator_notes,
        effective_at,
        ..
    } = command
    else {
        panic!()
    };
    let permanent = Command::Sanction {
        create: false,
        status: "active".into(),
        target,
        category,
        public_reason,
        moderator_notes,
        effective_at,
        expires_at: 0,
    };
    assert_eq!(permanent.validate(), Err(Error::Invalid));
}
#[tokio::test]
async fn missing_backend_denies_reads() {
    assert_eq!(
        PendingSupport.list(Area::Tickets, 1).await.err(),
        Some(Error::Unavailable)
    );
    assert_eq!(
        PendingSupport
            .load(Area::Enforcement, Uuid::now_v7())
            .await
            .err(),
        Some(Error::Unavailable)
    );
}
#[derive(Clone)]
struct Active;
impl AccountStateReader for Active {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
#[derive(Clone)]
struct TestBackend;
impl Backend for TestBackend {
    async fn list(&self, _: Area, _: u32) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }
    async fn load(&self, _: Area, _: Uuid) -> Result<Record, Error> {
        Err(Error::NotFound)
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        _: Uuid,
        _: Uuid,
        version: i64,
        command: &Command,
    ) -> Result<(serde_json::Value, serde_json::Value), Error> {
        if version != 1 {
            return Err(Error::Conflict);
        }
        command.transition("open")?;
        sqlx::query("INSERT INTO support_probe VALUES (1)")
            .execute(&mut **tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok((
            serde_json::json!({"version":1}),
            serde_json::json!({"version":2}),
        ))
    }
}
#[tokio::test]
async fn postgres_support_atomic_audit_and_refusal() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t51_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let search_path = format!("SET search_path TO {schema}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |c, _| {
            let query = search_path.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(query)).execute(c).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    sqlx::query("CREATE TABLE support_probe (value integer)")
        .execute(&pool)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await.unwrap();
    let auth = Authorizer::new(pool.clone(), Active);
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let command = Command::Reply {
        body: "private".into(),
        internal: true,
    };
    let id = Uuid::now_v7();
    execute(&pool, &auth, &TestBackend, context, id, 1, &command)
        .await
        .unwrap();
    assert_eq!(
        execute(&pool, &auth, &TestBackend, context, id, 2, &command).await,
        Err(Error::Conflict)
    );
    let denied = CheckContext {
        account: Uuid::now_v7(),
        ..context
    };
    assert_eq!(
        execute(&pool, &auth, &TestBackend, denied, id, 1, &command).await,
        Err(Error::Denied)
    );
    assert_eq!(
        execute(&pool, &auth, &TestBackend, context, id, 0, &command).await,
        Err(Error::Invalid)
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='support.ticket_internal_note_added'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    sqlx::query("ALTER TABLE audit_log ADD CONSTRAINT reject_support CHECK (action NOT LIKE 'support.%') NOT VALID").execute(&pool).await.unwrap();
    assert_eq!(
        execute(&pool, &auth, &TestBackend, context, id, 1, &command).await,
        Err(Error::Unavailable)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM support_probe")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
}

#[tokio::test]
async fn disabled_routes_hide_all_support_surfaces() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let app = web::router(
        pool,
        crate::web::PanelConfig::default(),
        crate::web::PendingAuthentication,
        Active,
        PendingSupport,
    );
    for path in [
        "/admin/support/tickets",
        "/admin/moderation/reports",
        "/admin/moderation/enforcement",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", "admin.example.test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
