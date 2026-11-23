use oteryn_admin::rbac::{
    AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, Change, CheckContext,
    Permission,
};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone)]
struct Active;
impl AccountStateReader for Active {
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
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
            .min_connections(size)
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
    async fn add_verified_platform_admin(&self) -> TestResult {
        sqlx::query("WITH survivor AS (
                        INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale)
                        VALUES(uuidv7(),'verified-admin@example.test','test-only','active',now(),'en')
                        RETURNING id
                     ) INSERT INTO account_roles(account_id,role_id)
                     SELECT survivor.id,roles.id FROM survivor CROSS JOIN roles WHERE roles.key='platform_admin'")
            .execute(&self.pool).await?;
        Ok(())
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
use super::{ChangeForm, Panel, mutate};
use axum::http::StatusCode;
use std::sync::Arc;
fn form(version: Uuid) -> ChangeForm {
    ChangeForm {
        action: "visibility".into(),
        version,
        kind: Some("monster".into()),
        key: Some("dragon".into()),
        allowed: Some("true".into()),
        pl: None,
        en: None,
    }
}
async fn prepare(db: &Fixture) -> TestResult<Uuid> {
    let version = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready')",
    )
    .bind(version)
    .execute(&db.pool)
    .await?;
    sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
        .bind(version)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'monster','dragon','{}',true)").bind(version).execute(&db.pool).await?;
    Ok(version)
}
async fn wait_for_fence(db: &Fixture, count: i64) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND datid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND wait_event_type='Lock' AND query='SELECT pg_advisory_xact_lock(71010)'").bind(&db.schema).fetch_one(&db.root).await?;
            if waiting >= count { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    Ok(())
}
#[tokio::test]
async fn world_two_queued_mutations_complete_with_two_connections() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(2).await?;
    let version = prepare(&db).await?;
    let panel = Arc::new(Panel {
        pool: db.pool.clone(),
        auth: Arc::new(Authorizer::new(db.pool.clone(), Active)),
        enabled: true,
    });
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let mut observer = db.root.acquire().await?;
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let panel = panel.clone();
        let actor = db.actor;
        tasks.push(tokio::spawn(async move {
            mutate(&panel, actor, form(version)).await
        }));
    }
    // One mutation waits at authorization; the other queues at its resource.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND datid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND wait_event_type='Lock' AND (query='SELECT pg_advisory_xact_lock(71010)' OR query='SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE')").bind(&db.schema).fetch_one(&mut *observer).await?;
            if waiting >= 2 { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    drop(observer);
    blocker.commit().await?;
    for task in tasks {
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(3), task).await??,
            Ok(())
        );
    }
    db.close().await?;
    eprintln!(
        "world_two_queued_mutations_complete_with_two_connections: {:?}",
        started.elapsed()
    );
    Ok(())
}
#[tokio::test]
async fn world_refuses_revoked_prerequisite_with_action_retained() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    db.add_verified_platform_admin().await?;
    let version = prepare(&db).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let manager = CheckContext {
        account: Uuid::now_v7(),
        ..db.actor
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(manager.account).execute(&db.pool).await?;
    let role = auth
        .change(
            manager,
            Uuid::now_v7(),
            Change::CreateRole {
                key: "visibility_only",
                permissions: &[
                    Permission::Access,
                    Permission::ViewGameCatalogSnapshots,
                    Permission::ManageGameCatalogProfiles,
                ],
            },
        )
        .await?
        .ok_or("missing role")?;
    auth.change(
        manager,
        Uuid::now_v7(),
        Change::Grant {
            account: db.actor.account,
            role,
        },
    )
    .await?;
    let platform: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    let panel = Panel {
        pool: db.pool.clone(),
        auth: Arc::new(auth.clone()),
        enabled: true,
    };
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let actor = db.actor;
    let writer_auth = auth.clone();
    let writer = tokio::spawn(async move {
        writer_auth
            .change(
                manager,
                Uuid::now_v7(),
                Change::Revoke {
                    account: actor.account,
                    role: platform,
                },
            )
            .await
    });
    wait_for_fence(&db, 1).await?;
    let mutation = tokio::spawn(async move { mutate(&panel, actor, form(version)).await });
    wait_for_fence(&db, 2).await?;
    blocker.commit().await?;
    tokio::time::timeout(std::time::Duration::from_secs(3), writer).await???;
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(3), mutation).await??,
        Err(StatusCode::FORBIDDEN)
    );
    auth.check(actor, Permission::ManageGameCatalogProfiles)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM world_visibility")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await?;
    eprintln!(
        "world_refuses_revoked_prerequisite_with_action_retained: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn publication_wait_does_not_hold_global_authorization_fence() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(5).await?;
    db.add_verified_platform_admin().await?;
    let version = prepare(&db).await?;
    let panel = Panel {
        pool: db.pool.clone(),
        auth: Arc::new(Authorizer::new(db.pool.clone(), Active)),
        enabled: true,
    };
    let manager = CheckContext {
        account: Uuid::now_v7(),
        ..db.actor
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execute(&db.pool).await?;
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&db.pool)
        .await?;
    let mut observer = db.root.acquire().await?;
    for (table, resource_query) in [
        (
            "world_catalog_publication",
            "SELECT snapshot_id FROM world_catalog_publication WHERE singleton FOR UPDATE",
        ),
        (
            "world_active_version",
            "SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE",
        ),
    ] {
        let unrelated = Uuid::now_v7();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
            .bind(unrelated)
            .bind(role)
            .execute(&db.pool)
            .await?;
        let mut blocker = db.pool.begin().await?;
        sqlx::query(AssertSqlSafe(format!(
            "SELECT singleton FROM {table} WHERE singleton FOR UPDATE"
        )))
        .fetch_one(&mut *blocker)
        .await?;
        let panel = Panel {
            pool: panel.pool.clone(),
            auth: panel.auth.clone(),
            enabled: true,
        };
        let actor = db.actor;
        let revoker = panel.auth.clone();
        let mutation = tokio::spawn(async move { mutate(&panel, actor, form(version)).await });
        // Wait for the foreground transaction to reach its resource wait.
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND datid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND wait_event_type='Lock' AND query=$2)").bind(&db.schema).bind(resource_query).fetch_one(&mut *observer).await?;
                if waiting { break Ok::<_, sqlx::Error>(()); }
                tokio::task::yield_now().await;
            }
        }).await.map_err(|_| format!("world mutation did not wait for {table}"))??;
        tokio::time::timeout(
            std::time::Duration::from_millis(150),
            revoker.change(
                manager,
                Uuid::now_v7(),
                Change::Revoke {
                    account: unrelated,
                    role,
                },
            ),
        )
        .await
        .map_err(|_| format!("unrelated role revocation stalled while {table} was locked"))??;
        let remains: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1)")
                .bind(unrelated)
                .fetch_one(&db.pool)
                .await?;
        assert!(!remains);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), mutation).await??,
            Err(StatusCode::SERVICE_UNAVAILABLE)
        );
        blocker.rollback().await?;
    }
    drop(observer);
    db.close().await?;
    eprintln!(
        "publication_wait_does_not_hold_global_authorization_fence: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[derive(Clone)]
struct AliasStaff(Uuid);
impl oteryn_admin::web::StaffSession for AliasStaff {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
#[tokio::test]
async fn postgres_a3_alias_controls_use_canonical_state_after_staff_dispatch() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    let version = prepare(&db).await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'creatures','rat','{}',true),($1,'Creature','rat','{}',true)").bind(version).execute(&db.pool).await?;
    sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES('creatures','rat',true),('Creature','rat',true)").execute(&db.pool).await?;
    let app = super::router(
        db.pool.clone(),
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        crate::WorldOptions { enabled: true },
        AliasStaff(db.actor.account),
        Active,
    );
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/admin/world?version={version}"))
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get("set-cookie")
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing identity")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("missing csrf")?;
    for fields in [
        "action=visibility&kind=creatures&key=rat&allowed=false",
        "action=names&kind=creatures&key=rat&pl=Szczur&en=Rat",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/admin/world")
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "_csrf={csrf}&version={version}&{fields}"
                    )))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }
    let response = app
        .oneshot(
            Request::get(format!("/admin/world?version={version}"))
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    assert_eq!(html.matches("<h3>Creature / rat</h3>").count(), 1);
    assert!(!html.contains("<h3>creatures / rat</h3>"));
    let row = html
        .split("<h3>Creature / rat</h3>")
        .nth(1)
        .ok_or("missing canonical entity")?
        .split("</article>")
        .next()
        .ok_or("missing controls")?;
    assert!(row.contains("value=\"false\" selected"));
    assert!(row.contains("value=\"Szczur\""));
    assert!(row.contains("value=\"Rat\""));
    db.close().await?;
    eprintln!(
        "postgres_a3_alias_controls_use_canonical_state_after_staff_dispatch duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_a3_world_fence_deadline_releases_publication_pointers() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    let version = prepare(&db).await?;
    let panel = Panel {
        pool: db.pool.clone(),
        auth: Arc::new(Authorizer::new(db.pool.clone(), Active)),
        enabled: true,
    };
    let mut blocker = db.root.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await?;
    let wait = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        mutate(&panel, db.actor, form(version)),
    )
    .await?;
    assert_eq!(result, Err(StatusCode::SERVICE_UNAVAILABLE));
    assert!(wait.elapsed() >= std::time::Duration::from_millis(2500));
    let mut probe = db.pool.begin().await?;
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        sqlx::query("SELECT singleton FROM world_active_version WHERE singleton FOR UPDATE")
            .execute(&mut *probe),
    )
    .await??;
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        sqlx::query("SELECT singleton FROM world_catalog_publication WHERE singleton FOR UPDATE")
            .execute(&mut *probe),
    )
    .await??;
    probe.rollback().await?;
    blocker.rollback().await?;
    db.close().await?;
    eprintln!(
        "postgres_a3_world_fence_deadline_releases_publication_pointers duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
