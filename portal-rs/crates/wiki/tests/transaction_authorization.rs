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
use oteryn_wiki::{
    domain::{Action, Error, Translation},
    repository::Repository,
};
#[tokio::test]
async fn publish_refuses_permission_revoked_while_waiting_on_guide() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(5).await?;
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('queued_publisher') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    for permission in [
        Permission::Access,
        Permission::WikiAccess,
        Permission::ManageWikiArticles,
        Permission::PublishWiki,
    ] {
        sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)")
            .bind(role)
            .bind(permission.as_str())
            .execute(&db.pool)
            .await?;
    }
    sqlx::query("UPDATE account_roles SET role_id=$2 WHERE account_id=$1")
        .bind(db.actor.account)
        .bind(role)
        .execute(&db.pool)
        .await?;
    let repo = Repository::new(db.pool.clone(), Active);
    let input: Vec<_> = ["en", "pl"]
        .into_iter()
        .map(|locale| Translation {
            locale: locale.into(),
            title: "Queued guide".into(),
            slug: "queued-guide".into(),
            summary: "Guide waiting for publication".into(),
            markdown: "A reviewed guide.".into(),
        })
        .collect();
    let id = repo
        .create(db.actor, &input, None, "Initial revision")
        .await?;
    repo.transition(db.actor, id, 1, Action::Submit).await?;
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM wiki_guides WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *blocker)
        .await?;
    let actor = db.actor;
    let task = tokio::spawn(async move { repo.transition(actor, id, 2, Action::Publish).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT id,status,version,revision_version,entity_key FROM wiki_guides%')").fetch_one(&db.pool).await?;
            if waiting { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    let manager = CheckContext {
        account: Uuid::now_v7(),
        ..db.actor
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execute(&db.pool).await?;
    Authorizer::new(db.pool.clone(), Active)
        .change(
            manager,
            Uuid::now_v7(),
            Change::SetPermissions {
                role,
                permissions: &[
                    Permission::Access,
                    Permission::WikiAccess,
                    Permission::ManageWikiArticles,
                ],
            },
        )
        .await?;
    blocker.commit().await?;
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), task).await??,
        Err(Error::Authorization(oteryn_admin::rbac::Error::Denied))
    ));
    let status: String = sqlx::query_scalar("SELECT status FROM wiki_guides WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(status, "in_review");
    db.close().await?;
    eprintln!(
        "publish_refuses_permission_revoked_while_waiting_on_guide: {:?}",
        started.elapsed()
    );
    Ok(())
}
