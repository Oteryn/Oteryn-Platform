use oteryn_admin::rbac::{
    AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, CheckContext,
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
use oteryn_content::events::{Draft, Events, Translation};
#[tokio::test]
async fn event_save_and_transition_complete_with_one_connection() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let events = Events::new(db.pool.clone());
    let now = time::OffsetDateTime::now_utc();
    let draft = Draft {
        starts_at: now,
        ends_at: now + time::Duration::hours(1),
        featured: false,
        en: Translation {
            title: "Pool regression event".into(),
            slug: "pool-event".into(),
            summary: "Concurrent administration".into(),
            body: "Event publication regression coverage.".into(),
        },
        pl: None,
    };
    let id = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        events.save(&auth, db.actor, None, draft),
    )
    .await??;
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        events.transition(&auth, db.actor, id, 1, "published"),
    )
    .await??;
    assert_eq!(events.load(id).await?.0.status, "published");
    db.close().await?;
    eprintln!(
        "event_save_and_transition_complete_with_one_connection: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn announcement_save_completes_with_one_connection() -> TestResult {
    use oteryn_content::announcements::{Announcements, Input};
    let started = std::time::Instant::now();
    let db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let announcements = Announcements::new(db.pool.clone());
    let input = Input {
        publication_state: "published".into(),
        severity: "info".into(),
        starts_at: time::OffsetDateTime::now_utc(),
        ends_at: None,
        title: "Pool regression notice".into(),
        body: "Administration is available.".into(),
        action_label: None,
        action_url: None,
        title_pl: None,
        body_pl: None,
        action_label_pl: None,
        translation_published_at: None,
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        announcements.save(&auth, db.actor, None, input),
    )
    .await??;
    assert_eq!(
        announcements
            .active("en", time::OffsetDateTime::now_utc(), 1)
            .await?
            .len(),
        1
    );
    db.close().await?;
    eprintln!(
        "announcement_save_completes_with_one_connection: {:?}",
        started.elapsed()
    );
    Ok(())
}
