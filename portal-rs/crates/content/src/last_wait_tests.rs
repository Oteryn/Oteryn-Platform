use oteryn_admin::rbac::{Authorizer, CheckContext};
use sqlx::PgPool;
use uuid::Uuid;
pub(crate) type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub(crate) struct TestDb {
    pub(crate) pool: PgPool,
    root: PgPool,
    schema: String,
    pub(crate) context: CheckContext,
}
impl TestDb {
    pub(crate) async fn open() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
    pub(crate) async fn close(self) -> TestResult {
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
async fn postgres_last_wait_grant_expiry_rolls_back_content() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = TestDb::open().await?;
    let pool = db.pool.clone();
    let context = db.context;
    let result = audit_wait(&db.pool, context, async move {
        let auth = Authorizer::new(
            pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
        );
        crate::announcements::Announcements::new(pool)
            .save(
                &auth,
                context,
                None,
                crate::announcements::Input {
                    publication_state: "draft".into(),
                    severity: "info".into(),
                    starts_at: time::OffsetDateTime::now_utc(),
                    ends_at: None,
                    title: "Audit wait".into(),
                    body: "Body".into(),
                    action_label: None,
                    action_url: None,
                    title_pl: None,
                    body_pl: None,
                    action_label_pl: None,
                    translation_published_at: None,
                },
            )
            .await
    })
    .await?;
    assert!(matches!(
        result,
        Err(crate::announcements::Error::Authorization(
            oteryn_admin::rbac::Error::Denied
        ))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM announcements")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}
