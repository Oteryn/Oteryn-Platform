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
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query='SELECT pg_advisory_xact_lock(71010)'").fetch_one(&db.root).await?;
            if waiting >= count { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    Ok(())
}
#[tokio::test]
async fn world_two_queued_mutations_complete_with_two_connections() -> TestResult {
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
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND (query='SELECT pg_advisory_xact_lock(71010)' OR query='SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE')").fetch_one(&db.root).await?;
            if waiting >= 2 { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
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
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    let version = prepare(&db).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let manager = CheckContext {
        account: Uuid::now_v7(),
        ..db.actor
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execute(&db.pool).await?;
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
    let started = std::time::Instant::now();
    let db = Fixture::open(5).await?;
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
    for table in ["world_catalog_publication", "world_active_version"] {
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
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%world_%version%FOR UPDATE%' OR wait_event_type='Lock' AND query LIKE '%world_catalog_publication%FOR UPDATE%')").fetch_one(&db.pool).await?;
                if waiting { break Ok::<_, sqlx::Error>(()); }
                tokio::task::yield_now().await;
            }
        }).await??;
        let unrelated = Uuid::now_v7();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
            .bind(unrelated)
            .bind(role)
            .execute(&db.pool)
            .await?;
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
        .await??;
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
    db.close().await?;
    eprintln!(
        "publication_wait_does_not_hold_global_authorization_fence: {:?}",
        started.elapsed()
    );
    Ok(())
}
