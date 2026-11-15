use crate::rbac::{Authorizer, CheckContext, Permission};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
struct Fixture {
    pool: PgPool,
    root: PgPool,
    schema: String,
    actor: CheckContext,
}
impl Fixture {
    async fn open(size: u32) -> TestResult<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        let schema = format!("pool_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(size)
            .acquire_timeout(std::time::Duration::from_secs(1))
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = CheckContext {
            account: Uuid::now_v7(),
            require_staff_mfa: true,
            mfa_verified: true,
        };
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute(&pool).await?;
        Ok(Self {
            pool,
            root,
            schema,
            actor,
        })
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}
use crate::web::{PanelConfig, PortalIdentity, StaffIdentity, StaffSession, router};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_web::session::BrowserSession;
use tower::ServiceExt;
#[derive(Clone)]
struct Session(Uuid);
impl StaffSession for Session {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
fn app(db: &Fixture) -> Router {
    router(
        db.pool.clone(),
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(db.actor.account),
        PortalIdentity::configured(db.pool.clone()),
    )
}
async fn insert_account(db: &Fixture, id: Uuid, status: &str) -> TestResult {
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'unused',$3,'en',now())")
        .bind(id).bind(format!("{id}@local.test")).bind(status).execute(&db.pool).await?;
    Ok(())
}
async fn browser(db: &Fixture) -> TestResult<(String, String)> {
    let response = app(db)
        .oneshot(
            Request::get("/admin/roles")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get("set-cookie")
        .ok_or("missing session cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("invalid cookie")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("missing CSRF")?
        .to_owned();
    assert!(html.contains("value=\"create\""));
    assert!(html.contains("value=\"set-permissions\""));
    Ok((cookie, csrf))
}
async fn post(db: &Fixture, cookie: &str, csrf: &str, body: String) -> TestResult<StatusCode> {
    Ok(app(db)
        .oneshot(
            Request::post("/admin/roles")
                .header("host", "staff.test")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("origin", "https://staff.test")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))?,
        )
        .await?
        .status())
}
#[tokio::test]
async fn panel_refuses_last_eligible_administrator_revocation() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    for status in ["suspended", "closed"] {
        let id = Uuid::now_v7();
        insert_account(&db, id, status).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
            .bind(id)
            .bind(role)
            .execute(&db.pool)
            .await?;
    }
    let missing = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(missing)
        .bind(role)
        .execute(&db.pool)
        .await?;
    let (cookie, csrf) = browser(&db).await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=revoke&account={}&role={role}", db.actor.account)
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2)",
    )
    .bind(db.actor.account)
    .bind(role)
    .fetch_one(&db.pool)
    .await?;
    assert!(assigned);
    // An eligible second holder permits revocation; its eligibility is read in
    // the mutation transaction, using the same production identity adapter.
    let owner: Uuid = sqlx::query_scalar("INSERT INTO roles(key) VALUES('owner') RETURNING id")
        .fetch_one(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2),($3,$2)")
        .bind(db.actor.account)
        .bind(owner)
        .bind(missing)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=revoke&account={}&role={owner}", db.actor.account)
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    let eligible = Uuid::now_v7();
    insert_account(&db, eligible, "active").await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(eligible)
        .bind(role)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=revoke&account={}&role={role}", db.actor.account)
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    db.close().await?;
    eprintln!(
        "panel_refuses_last_eligible_administrator_revocation: {:?}",
        started.elapsed()
    );
    Ok(())
}
#[tokio::test]
async fn panel_creates_and_edits_roles_with_csrf_and_privilege_guards() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let (cookie, csrf) = browser(&db).await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            "invalid",
            "action=create&key=missing_csrf".into()
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            "action=create&key=panel_editor&permissions=admin.access".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='panel_editor'")
        .fetch_one(&db.pool)
        .await?;
    let target = Uuid::now_v7();
    insert_account(&db, target, "active").await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=grant&account={target}&role={role}")
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    let context = CheckContext {
        account: target,
        ..db.actor
    };
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    assert!(matches!(
        auth.check(context, Permission::ManageWikiArticles).await,
        Err(super::Error::Denied)
    ));
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!(
                "action=set-permissions&role={role}&permissions=admin.access%20{}",
                Permission::ManageWikiArticles.as_str()
            )
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    auth.check(context, Permission::ManageWikiArticles).await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=set-permissions&role={role}&permissions=admin.roles.manage")
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            "action=create&key=privileged_editor&permissions=admin.roles.manage".into()
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    let protected: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=set-permissions&role={protected}&permissions=")
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=set-permissions&role={role}&permissions=admin.access")
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert!(matches!(
        auth.check(context, Permission::ManageWikiArticles).await,
        Err(super::Error::Denied)
    ));
    db.close().await?;
    eprintln!(
        "panel_creates_and_edits_roles_with_csrf_and_privilege_guards: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn player_notes_complete_when_small_pool_is_queued_on_role_fence() -> TestResult {
    use crate::players::{Flags, IdentityAccounts, Players};
    let started = std::time::Instant::now();
    let db = Fixture::open(2).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let players = std::sync::Arc::new(Players::new(
        db.pool.clone(),
        Flags {
            players: true,
            ..Flags::default()
        },
        PortalIdentity::configured(db.pool.clone()),
        IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts {
                pool: db.pool.clone(),
            },
            projection: None,
        },
    ));
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let mut tasks = Vec::new();
    for body in [
        "First verified support note",
        "Second verified support note",
    ] {
        let players = players.clone();
        let actor = db.actor;
        tasks.push(tokio::spawn(async move {
            players.note(actor, actor.account, body).await
        }));
    }
    tokio::time::timeout(std::time::Duration::from_secs(2),async {
        loop {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query='SELECT pg_advisory_xact_lock(71010)'").fetch_one(&db.root).await?;
            if count >= 2 {return Ok::<_,sqlx::Error>(());}
            tokio::task::yield_now().await;
        }
    }).await??;
    blocker.commit().await?;
    for task in tasks {
        tokio::time::timeout(std::time::Duration::from_secs(3), task).await???;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM player_notes WHERE account_id=$1")
        .bind(db.actor.account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 2);
    db.close().await?;
    eprintln!(
        "player_notes_complete_when_small_pool_is_queued_on_role_fence: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn role_fence_wait_is_bounded_and_releases_pool_capacity() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    insert_account(&db, db.actor.account, "active").await?;
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        auth.change_from_panel(
            db.actor,
            Uuid::now_v7(),
            super::Change::CreateRole {
                key: "bounded_wait",
                permissions: &[],
            },
        ),
    )
    .await?;
    assert!(
        matches!(result,Err(super::Error::Storage(ref e)) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("55P03"))
    );
    blocker.rollback().await?;
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.close().await?;
    eprintln!(
        "role_fence_wait_is_bounded_and_releases_pool_capacity: {:?}",
        started.elapsed()
    );
    Ok(())
}
