use oteryn_admin::rbac::{Authorizer, CheckContext};
use sqlx::PgPool;
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

struct TestDb {
    pool: PgPool,
    root: PgPool,
    schema: String,
    context: CheckContext,
}
impl TestDb {
    async fn open() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPool::connect(&url).await?;
        let schema = format!("last_wait_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'fixture-password','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
        let context = CheckContext {
            account: actor,
            require_staff_mfa: true,
            mfa_verified: true,
            session_proof: Some(crate::test_staff::proof(&pool, actor).await?),
        };
        Ok(Self {
            pool,
            root,
            schema,
            context,
        })
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}

// Session/grant rows are retained FOR UPDATE by authorization, so arrange a
// database-clock grant deadline before starting, then observe it elapse while
// the real mutation is blocked on this backend's audit table lock.
async fn audit_wait<F, T>(
    pool: &PgPool,
    context: CheckContext,
    operation: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let mut blocker = pool.begin().await?;
    sqlx::query("LOCK TABLE audit_log IN SHARE MODE")
        .execute(&mut *blocker)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '1 second' WHERE account_id=$1").bind(context.account).execute(pool).await?;
    let task = tokio::spawn(operation);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *blocker).await?;
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock')").bind(pid).fetch_one(&mut *blocker).await?;
            if waiting { break; }
            tokio::task::yield_now().await;
        }
        loop {
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *blocker).await?;
            let expired_while_waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))) AND EXISTS(SELECT 1 FROM account_mfa_tokens WHERE account_id=$2 AND expires_at<=clock_timestamp())").bind(pid).bind(context.account).fetch_one(&mut *blocker).await?;
            if expired_while_waiting { break; }
            tokio::task::yield_now().await;
        }
        Ok::<(), sqlx::Error>(())
    }).await.map_err(|_| "mutation must reach audit wait and grant expiry within two seconds")??;
    blocker.rollback().await?;
    Ok(tokio::time::timeout(std::time::Duration::from_secs(2), task).await??)
}

#[tokio::test]
async fn postgres_last_wait_grant_expiry_rolls_back_world() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = TestDb::open().await?;
    let version = Uuid::now_v7();
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
        .bind(version)
        .bind("a".repeat(40))
        .execute(&db.pool)
        .await?;
    let pool = db.pool.clone();
    let context = db.context;
    let result = audit_wait(&db.pool, context, async move {
        let panel = super::Panel {
            auth: std::sync::Arc::new(Authorizer::new(
                pool.clone(),
                oteryn_admin::web::PortalIdentity::configured(pool.clone()),
            )),
            pool,
            enabled: true,
        };
        super::mutate(
            &panel,
            context,
            super::ChangeForm {
                action: "publish".into(),
                version,
                kind: None,
                key: None,
                allowed: None,
                pl: None,
                en: None,
            },
        )
        .await
    })
    .await?;
    assert_eq!(result, Err(axum::http::StatusCode::FORBIDDEN));
    assert_eq!(
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT version_id FROM world_active_version WHERE singleton"
        )
        .fetch_one(&db.pool)
        .await?,
        None
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}

// A denied staff request records its denial in the audit log. A held audit
// table lock must make it answer unavailable within the mutation budget.
#[tokio::test]
async fn postgres_denied_audit_answers_unavailable_under_a_held_audit_lock() -> TestResult {
    use sqlx::Connection;
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = TestDb::open().await?;
    let outsider = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'fixture-password','active','en',now())").bind(outsider).bind(format!("{outsider}@example.invalid")).execute(&db.pool).await?;
    let context = CheckContext {
        account: outsider,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(&db.pool, outsider).await?),
    };
    let mut blocker =
        sqlx::PgConnection::connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?).await?;
    sqlx::query("BEGIN").execute(&mut blocker).await?;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {schema}.audit_log IN SHARE MODE"
    )))
    .execute(&mut blocker)
    .await?;
    let panel = super::Panel {
        auth: std::sync::Arc::new(Authorizer::new(
            db.pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(db.pool.clone()),
        )),
        pool: db.pool.clone(),
        enabled: true,
    };
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        super::authorize(
            &panel,
            context,
            oteryn_admin::rbac::Permission::ViewGameCatalogSnapshots,
        ),
    )
    .await;
    let elapsed = started.elapsed();
    sqlx::query("ROLLBACK").execute(&mut blocker).await?;
    blocker.close().await?;
    let result = result.map_err(|_| "denied audit waited past 10 s on a held audit lock")?;
    assert_eq!(result, Err(axum::http::StatusCode::SERVICE_UNAVAILABLE));
    assert!(elapsed < std::time::Duration::from_secs(4), "{elapsed:?}");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}
