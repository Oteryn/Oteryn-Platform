use crate::rbac::{Authorizer, CheckContext, Permission};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
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
                    sqlx::query(AssertSqlSafe(path)).execute(&mut *c).await?;
                    sqlx::query("SELECT set_config('application_name',current_schema(),false)")
                        .execute(c)
                        .await?;
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
            session_proof: None,
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
struct Session(Uuid, PgPool);
impl StaffSession for Session {
    async fn resolve(&self, session: &BrowserSession) -> Option<StaffIdentity> {
        crate::test_staff_authenticate(&self.1, session, self.0)
            .await
            .ok()?;
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
        Session(db.actor.account, db.pool.clone()),
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
    crate::test_staff::browser(&db.pool, db.actor.account, &cookie).await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    crate::test_staff::recent_password(&db.pool, &cookie).await?;
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
            format!("action=grant&account={target}&role={role}&reason=role_assignment")
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
                "action=set-permissions&role={role}&permissions=admin.access%20{}&reason=role_assignment",
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
    let denial_count = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log
             WHERE actor=$1 AND action='admin.access.denied' AND target='/admin/roles'",
        )
        .bind(db.actor.account)
        .fetch_one(&db.pool)
    };
    let initial_denials = denial_count().await?;
    for (index, (account, role)) in [(db.actor.account, role), (target, protected)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            post(
                &db,
                &cookie,
                &csrf,
                format!("action=grant&account={account}&role={role}&reason=role_assignment")
            )
            .await?,
            StatusCode::FORBIDDEN
        );
        let granted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2)",
        )
        .bind(account)
        .bind(role)
        .fetch_one(&db.pool)
        .await?;
        assert!(!granted);
        assert_eq!(denial_count().await?, initial_denials + index as i64 + 1);
    }
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    use crate::players::{Flags, IdentityAccounts, Players};
    let started = std::time::Instant::now();
    let mut db = Fixture::open(2).await?;
    insert_account(&db, db.actor.account, "active").await?;
    db.actor.session_proof = Some(crate::test_staff::proof(&db.pool, db.actor.account).await?);
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
    // Same-account mutations now queue at account preparation before the fence.
    // Both connections must still be occupied and both operations must commit.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let count = staff_waiters(&db).await?;
            if count >= 2 {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let mut db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    insert_account(&db, db.actor.account, "active").await?;
    db.actor.session_proof = Some(crate::test_staff::proof(&db.pool, db.actor.account).await?);
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
        matches!(result,Err(super::Error::Storage(ref e)) if matches!(e.as_database_error().and_then(|e| e.code()).as_deref(), Some("55P03" | "57014")))
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

#[tokio::test]
async fn postgres_role_pages_and_survivor_batches_are_bounded() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    sqlx::query("INSERT INTO roles(key) SELECT 'batch_'||n FROM generate_series(1,125) n")
        .execute(&db.pool)
        .await?;
    let mut cursor = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let page = crate::web::role_page(&db.pool, cursor).await?;
        assert!(page.len() <= 50);
        if page.is_empty() {
            break;
        }
        cursor = page.last().map(|r| r.0);
        for (id, _) in page {
            assert!(seen.insert(id));
        }
    }
    assert!(seen.len() > 125);
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    // More disabled candidates than a batch, followed by one eligible survivor.
    let mut holders = Vec::new();
    for n in 1..=125_u128 {
        let id = Uuid::from_u128(n);
        insert_account(&db, id, if n == 125 { "active" } else { "suspended" }).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
            .bind(id)
            .bind(role)
            .execute(&db.pool)
            .await?;
        holders.push(id);
    }
    let batch = crate::web::holder_batch(&db.pool, role, db.actor.account, None).await?;
    assert_eq!(batch.len(), 50);
    assert_eq!(batch, holders[..50]);
    let (cookie, csrf) = browser(&db).await?;
    sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')")
        .bind(holders[124]).execute(&db.pool).await?;
    let revoke = format!("action=revoke&account={}&role={role}", db.actor.account);
    assert_eq!(
        post(&db, &cookie, &csrf, revoke).await?,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE account_terminations SET cancelled_at=now() WHERE account_id=$1")
        .bind(holders[124])
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
    // The remaining eligible administrator may not revoke itself.
    let last = Fixture {
        pool: db.pool.clone(),
        root: db.root.clone(),
        schema: db.schema.clone(),
        actor: CheckContext {
            account: holders[124],
            ..db.actor
        },
    };
    let (cookie, csrf) = browser(&last).await?;
    assert_eq!(
        post(
            &last,
            &cookie,
            &csrf,
            format!("action=revoke&account={}&role={role}", holders[124])
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    db.close().await?;
    eprintln!(
        "postgres_role_pages_and_survivor_batches_are_bounded duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[derive(Clone)]
struct PausedPortalIdentity {
    identity: PortalIdentity,
    authenticated: Arc<tokio::sync::Notify>,
    resume: Arc<tokio::sync::Notify>,
}
impl StaffSession for PausedPortalIdentity {
    async fn resolve(&self, session: &BrowserSession) -> Option<StaffIdentity> {
        let identity = self.identity.resolve(session).await?;
        self.authenticated.notify_one();
        self.resume.notified().await;
        Some(identity)
    }
}

#[tokio::test]
async fn postgres_a3_real_player_request_refuses_revoked_session_after_authentication() -> TestResult
{
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    use sha2::{Digest, Sha256};
    let started = std::time::Instant::now();
    let db = Fixture::open(4).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let target = Uuid::now_v7();
    insert_account(&db, target, "active").await?;
    let session_id = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let grant = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let csrf = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true)")
        .bind(db.actor.account)
        .bind(Sha256::digest(Uuid::now_v7().as_bytes()).to_vec())
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) SELECT $1,a.id,m.generation,$2,'grant',clock_timestamp()+interval '1 hour' FROM accounts a JOIN account_mfa_totp m ON m.account_id=a.id WHERE a.id=$3")
        .bind(Sha256::digest(grant.as_bytes()).to_vec()).bind(Sha256::digest(b"unused").to_vec()).bind(db.actor.account).execute(&db.pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,idle_expires_at,absolute_expires_at,data,csrf_token,authorization_generation) SELECT $1,id,'a3-browser',now()+interval '1 hour',now()+interval '1 day',jsonb_build_object('mfa_grant',$2::text),$3,native_security_generation FROM accounts WHERE id=$4")
        .bind(Sha256::digest(session_id.as_bytes()).to_vec()).bind(&grant).bind(&csrf).bind(db.actor.account).execute(&db.pool).await?;
    let identity = PortalIdentity::configured_from(db.pool.clone(), &|key| {
        (key == "OTERYN_MFA_KEY").then(|| "12".repeat(32))
    });
    let authenticated = Arc::new(tokio::sync::Notify::new());
    let resume = Arc::new(tokio::sync::Notify::new());
    let app = crate::players::http::router(
        db.pool.clone(),
        crate::players::Flags {
            players: true,
            ..Default::default()
        },
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        PausedPortalIdentity {
            identity: identity.clone(),
            authenticated: authenticated.clone(),
            resume: resume.clone(),
        },
        identity,
        crate::players::IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts {
                pool: db.pool.clone(),
            },
            projection: None,
        },
    );
    let request = Request::post(format!("/admin/players/{target}"))
        .header("host", "staff.test")
        .header("origin", "https://staff.test")
        .header("cookie", format!("__Host-oteryn_session={session_id}"))
        .header("x-csrf-token", csrf)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("action=note&body=Session+revocation+regression"))?;
    let task = tokio::spawn(async move { app.oneshot(request).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), authenticated.notified()).await?;
    let mut revoke = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(db.actor.account)
        .execute(&mut *revoke)
        .await?;
    oteryn_identity::sessions::staff_revoke_all(&mut revoke, db.actor.account).await?;
    revoke.commit().await?;
    resume.notify_one();
    assert_eq!(task.await??.status(), StatusCode::FORBIDDEN);
    let notes: i64 = sqlx::query_scalar("SELECT count(*) FROM player_notes WHERE account_id=$1")
        .bind(target)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(notes, 0);
    let success: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='players.note.added'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(success, 0);
    db.close().await?;
    eprintln!(
        "postgres_a3_real_player_request_refuses_revoked_session_after_authentication duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn staff_waiters(db: &Fixture) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND datid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND wait_event_type='Lock' AND (query='SELECT pg_advisory_xact_lock(71010)' OR query='SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE')")
        .bind(&db.schema).fetch_one(&db.root).await
}
#[tokio::test]
async fn postgres_a3_staff_barrier_ignores_another_fixture_waiter() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(2).await?;
    let other = Fixture::open(2).await?;
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let mut unrelated = other.pool.begin().await?;
    let unrelated_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *unrelated)
        .await?;
    let unrelated_task = tokio::spawn(async move {
        super::lock_mutation(&mut unrelated).await?;
        unrelated.commit().await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2),async {
        loop {
            let waiting: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')").bind(unrelated_pid).fetch_one(&db.root).await?;
            if waiting { break Ok::<(),sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    let mut own = db.pool.begin().await?;
    let resume = Arc::new(tokio::sync::Notify::new());
    let go = resume.clone();
    let own_task = tokio::spawn(async move {
        go.notified().await;
        super::lock_mutation(&mut own).await?;
        own.commit().await
    });
    assert_eq!(
        staff_waiters(&db).await?,
        0,
        "a foreign fixture must not open the barrier"
    );
    resume.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if staff_waiters(&db).await? == 1 {
                break Ok::<(), sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    blocker.commit().await?;
    own_task.await??;
    unrelated_task.await??;
    other.close().await?;
    db.close().await?;
    eprintln!(
        "postgres_a3_staff_barrier_ignores_another_fixture_waiter duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_a3_role_writer_rechecks_current_session_mfa_binding() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let mut context = db.actor;
    context.session_proof = Some(crate::test_staff::proof(&db.pool, context.account).await?);
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(context.account)
        .execute(&mut *blocker)
        .await?;
    let task = tokio::spawn(async move {
        auth.change_from_panel(
            context,
            Uuid::now_v7(),
            super::Change::CreateRole {
                key: "current_mfa_binding",
                permissions: &[],
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if staff_waiters(&db).await? == 1 {
                break Ok::<(), sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    // The credential-bound token remains live, but this session no longer has
    // an MFA proof. Reusing the captured token would authorize stale authority.
    sqlx::query("UPDATE web_sessions SET data=data-'mfa_grant' WHERE account_id=$1")
        .bind(context.account)
        .execute(&mut *blocker)
        .await?;
    blocker.commit().await?;
    assert!(matches!(task.await?, Err(super::Error::Denied)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM roles WHERE key='current_mfa_binding'")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log WHERE action='rbac.role.created'"
        )
        .fetch_one(&db.pool)
        .await?,
        0
    );
    db.close().await?;
    eprintln!(
        "postgres_a3_role_writer_rechecks_current_session_mfa_binding duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn generic_role_revocation_requires_a_nonterminating_survivor() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let target = db.actor.account;
    let pending = Uuid::now_v7();
    let actor = Uuid::now_v7();
    insert_account(&db, pending, "active").await?;
    insert_account(&db, actor, "active").await?;
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(pending)
        .bind(role)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')")
        .bind(pending).execute(&db.pool).await?;
    let manager: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('delegated_role_manager') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.manage')")
        .bind(manager)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(actor)
        .bind(manager)
        .execute(&db.pool)
        .await?;
    let context = CheckContext {
        account: actor,
        session_proof: Some(crate::test_staff::proof(&db.pool, actor).await?),
        ..db.actor
    };
    let authorizer = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    authorizer.check(context, Permission::ManageRoles).await?;
    assert!(matches!(
        authorizer
            .change(
                context,
                Uuid::now_v7(),
                super::Change::Revoke {
                    account: target,
                    role
                }
            )
            .await,
        Err(super::Error::Denied)
    ));
    sqlx::query("UPDATE account_terminations SET cancelled_at=now() WHERE account_id=$1")
        .bind(pending)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        authorizer
            .change(
                context,
                Uuid::now_v7(),
                super::Change::Revoke {
                    account: target,
                    role
                }
            )
            .await?,
        Some(role)
    );
    db.close().await
}

#[tokio::test]
async fn role_edits_cannot_delegate_grants_without_staff_action_controls() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let recipient = Uuid::now_v7();
    insert_account(&db, recipient, "active").await?;
    let management: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('delegation_manager') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    let extra: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('delegation_extra') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access'),($1,'admin.roles.manage')")
        .bind(management).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2),($1,$3)")
        .bind(recipient)
        .bind(management)
        .bind(extra)
        .execute(&db.pool)
        .await?;
    let (cookie, csrf) = browser(&db).await?;
    let edit = format!("action=set-permissions&role={extra}&permissions=admin.roles.grant");
    let confirmed = format!("{edit}&reason=role_assignment");
    let create =
        "action=create&key=delegation_new&permissions=admin.roles.grant&reason=role_assignment";
    let escrow = "DELETE FROM role_permissions WHERE role_id=(SELECT id FROM roles WHERE key='platform_admin') AND permission='admin.roles.grant'";
    sqlx::query(escrow).execute(&db.pool).await?;
    crate::test_staff::recent_password(&db.pool, &cookie).await?;
    for form in [edit.as_str(), confirmed.as_str(), create] {
        assert_eq!(
            post(&db, &cookie, &csrf, form.to_owned()).await?,
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.roles.grant' FROM roles WHERE key='platform_admin'")
        .execute(&db.pool).await?;
    sqlx::query("UPDATE web_sessions SET sudo_until=clock_timestamp() WHERE account_id=$1")
        .bind(db.actor.account)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        post(&db, &cookie, &csrf, confirmed.clone()).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&db, &cookie, &csrf, create.into()).await?,
        StatusCode::FORBIDDEN
    );
    crate::test_staff::recent_password(&db.pool, &cookie).await?;
    assert_eq!(
        post(&db, &cookie, &csrf, edit.clone()).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&db, &cookie, &csrf, format!("{edit}&reason=unapproved")).await?,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let changed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM role_permissions WHERE role_id=$1)
        OR EXISTS(SELECT 1 FROM roles WHERE key='delegation_new')
        OR EXISTS(SELECT 1 FROM audit_log WHERE action IN ('rbac.role.permissions_changed','rbac.role.created'))")
        .bind(extra).fetch_one(&db.pool).await?;
    assert!(!changed);

    assert_eq!(
        post(&db, &cookie, &csrf, confirmed).await?,
        StatusCode::SEE_OTHER
    );
    let permissions: Vec<String> =
        sqlx::query_scalar("SELECT permission FROM role_permissions WHERE role_id=$1")
            .bind(extra)
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(permissions, ["admin.roles.grant"]);
    let audit: Vec<(serde_json::Value, serde_json::Value, String)> = sqlx::query_as(
        "SELECT before,after,staff_reason FROM audit_log WHERE action='rbac.role.permissions_changed' AND target=$1",
    ).bind(extra.to_string()).fetch_all(&db.pool).await?;
    assert_eq!(
        audit,
        vec![(
            serde_json::json!({"permissions": []}),
            serde_json::json!({"permissions": ["admin.roles.grant"]}),
            "role_assignment".into()
        )]
    );
    assert_eq!(
        post(&db, &cookie, &csrf, create.into()).await?,
        StatusCode::SEE_OTHER
    );
    let created: (serde_json::Value, String) = sqlx::query_as(
        "SELECT l.after,l.staff_reason FROM audit_log l JOIN roles r ON l.target=r.id::text
         WHERE r.key='delegation_new' AND l.action='rbac.role.created'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(
        created,
        (
            serde_json::json!({"permissions": ["admin.roles.grant"]}),
            "role_assignment".into()
        )
    );
    db.close().await
}

#[tokio::test]
async fn widening_an_assigned_role_requires_the_direct_grant_controls() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Fixture::open(3).await?;
    insert_account(&db, db.actor.account, "active").await?;
    let recipient = Uuid::now_v7();
    insert_account(&db, recipient, "active").await?;
    let assigned: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('assigned_finance') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(recipient)
        .bind(assigned)
        .execute(&db.pool)
        .await?;
    let (cookie, csrf) = browser(&db).await?;
    let edit = format!("action=set-permissions&role={assigned}&permissions=payments.reconcile");
    let confirmed = format!("{edit}&reason=role_assignment");
    let grant_role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    sqlx::query("DELETE FROM role_permissions WHERE role_id=$1 AND permission='admin.roles.grant'")
        .bind(grant_role)
        .execute(&db.pool)
        .await?;
    crate::test_staff::recent_password(&db.pool, &cookie).await?;
    for form in [&edit, &confirmed] {
        assert_eq!(
            post(&db, &cookie, &csrf, form.clone()).await?,
            StatusCode::FORBIDDEN
        );
    }

    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.roles.grant')")
        .bind(grant_role)
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE web_sessions SET sudo_until=clock_timestamp() WHERE account_id=$1")
        .bind(db.actor.account)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        post(&db, &cookie, &csrf, confirmed.clone()).await?,
        StatusCode::FORBIDDEN
    );
    crate::test_staff::recent_password(&db.pool, &cookie).await?;
    assert_eq!(
        post(&db, &cookie, &csrf, edit.clone()).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&db, &cookie, "invalid", confirmed.clone()).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(&db, &cookie, &csrf, format!("{edit}&reason=unapproved")).await?,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let recipient_context = CheckContext {
        account: recipient,
        ..db.actor
    };
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    let evidence = || async {
        sqlx::query_as::<_, (serde_json::Value, serde_json::Value, String)>(
            "SELECT before,after,staff_reason FROM audit_log WHERE action='rbac.role.permissions_changed' AND target=$1",
        ).bind(assigned.to_string()).fetch_all(&db.pool).await
    };
    assert!(matches!(
        auth.check(recipient_context, Permission::ReconcilePayments)
            .await,
        Err(super::Error::Denied)
    ));
    assert!(evidence().await?.is_empty());
    assert_eq!(
        post(&db, &cookie, &csrf, confirmed.clone()).await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        post(&db, &cookie, &csrf, confirmed).await?,
        StatusCode::SEE_OTHER
    );
    auth.check(recipient_context, Permission::ReconcilePayments)
        .await?;
    assert_eq!(
        evidence().await?,
        vec![(
            serde_json::json!({"permissions": []}),
            serde_json::json!({"permissions": ["payments.reconcile"]}),
            "role_assignment".into()
        )]
    );
    assert_eq!(
        post(
            &db,
            &cookie,
            &csrf,
            format!("action=set-permissions&role={assigned}&permissions=")
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert!(matches!(
        auth.check(recipient_context, Permission::ReconcilePayments)
            .await,
        Err(super::Error::Denied)
    ));
    db.close().await
}
