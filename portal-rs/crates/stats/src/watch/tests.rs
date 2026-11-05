use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::atomic::{AtomicBool, Ordering};
use tower::ServiceExt;

#[derive(Debug)]
struct TestPolicy {
    owner: Uuid,
    visible: AtomicBool,
}
impl Policy for TestPolicy {
    fn owner<'a>(&'a self, token: &'a str) -> PolicyFuture<'a, Option<Uuid>> {
        Box::pin(async move { Ok((token == "test-access").then_some(self.owner)) })
    }
    fn allowed<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
        _: Subscription,
        _: Option<Event>,
    ) -> PolicyFuture<'a, bool> {
        Box::pin(async { Ok(self.visible.load(Ordering::SeqCst)) })
    }
    fn locale(&self, _: Uuid) -> PolicyFuture<'_, Locale> {
        Box::pin(async { Ok(Locale::En) })
    }
}
#[derive(Debug)]
struct Accounts;
impl oteryn_notify::AccountState for Accounts {
    fn active_verified(
        &self,
        _: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async {
            Ok(Some(oteryn_notify::Account {
                email: "watch@example.invalid".into(),
                locale: Locale::En,
            }))
        })
    }
}
async fn database() -> Result<Option<PgPool>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping watches PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("watch_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105231206_t70_watches.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105130000_t36_notifications.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105120000_t37_outbox.sql"
    ))
    .execute(&pool)
    .await?;
    Ok(Some(pool))
}
async fn dispose(pool: PgPool) -> Result<()> {
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
fn setup(pool: PgPool) -> (Watches, Arc<TestPolicy>, Subscription) {
    let policy = Arc::new(TestPolicy {
        owner: Uuid::now_v7(),
        visible: AtomicBool::new(true),
    });
    let watches = Watches {
        pool,
        enabled: true,
        policy: policy.clone(),
    };
    let s = Subscription {
        target_kind: TargetKind::Character,
        target_id: Uuid::now_v7(),
        event_kind: EventKind::Death,
    };
    (watches, policy, s)
}
#[tokio::test]
async fn postgres_subscriptions_owner_isolation_and_privacy() -> Result<()> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let (w, p, s) = setup(pool.clone());
    w.subscribe(p.owner, s).await?;
    w.subscribe(p.owner, s).await?;
    assert_eq!(w.list(p.owner).await?.len(), 1);
    assert!(w.list(Uuid::now_v7()).await?.is_empty());
    w.unsubscribe(Uuid::now_v7(), s).await?;
    assert_eq!(w.list(p.owner).await?.len(), 1);
    p.visible.store(false, Ordering::SeqCst);
    assert!(w.list(p.owner).await?.is_empty());
    assert!(w.subscribe(p.owner, s).await.is_err());
    assert!(
        w.subscribe(
            p.owner,
            Subscription {
                target_id: Uuid::nil(),
                ..s
            }
        )
        .await
        .is_err()
    );
    w.unsubscribe(p.owner, s).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_watches")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    dispose(pool).await
}
#[tokio::test]
async fn postgres_notification_outbox_retry_and_hidden_guild_member() -> Result<()> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let (w, p, s) = setup(pool.clone());
    let guild = Uuid::now_v7();
    w.subscribe(p.owner, s).await?;
    w.subscribe(
        p.owner,
        Subscription {
            target_kind: TargetKind::Guild,
            target_id: guild,
            ..s
        },
    )
    .await?;
    let notify = Notifications {
        pool: pool.clone(),
        accounts: Arc::new(Accounts),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![7; 32],
            "https://example.invalid/unsubscribe".into(),
        )?,
    };
    let mut tx = pool.begin().await?;
    notify
        .preferences(&mut tx, p.owner, oteryn_notify::Category::News, true, true)
        .await?;
    tx.commit().await?;
    let event = Event {
        id: Uuid::now_v7(),
        character: s.target_id,
        guild: Some(guild),
        kind: s.event_kind,
    };
    let denied_notify = Notifications {
        pool: pool.clone(),
        accounts: Arc::new(oteryn_notify::DenyAccounts),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![7; 32],
            "https://example.invalid/unsubscribe".into(),
        )?,
    };
    assert!(w.dispatch(&denied_notify, event).await.is_err());
    assert_eq!(w.dispatch(&notify, event).await?, 1);
    assert_eq!(w.dispatch(&notify, event).await?, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox WHERE kind='notify.email'")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    for kind in [
        EventKind::Online,
        EventKind::LevelUp,
        EventKind::GuildChange,
    ] {
        w.subscribe(
            p.owner,
            Subscription {
                event_kind: kind,
                ..s
            },
        )
        .await?;
        let next = Event {
            id: Uuid::now_v7(),
            kind,
            ..event
        };
        assert_eq!(w.dispatch(&notify, next).await?, 1);
        assert_eq!(w.dispatch(&notify, next).await?, 0);
    }
    let mut tx = pool.begin().await?;
    notify
        .preferences(
            &mut tx,
            p.owner,
            oteryn_notify::Category::News,
            false,
            false,
        )
        .await?;
    tx.commit().await?;
    assert_eq!(
        w.dispatch(
            &notify,
            Event {
                id: Uuid::now_v7(),
                ..event
            }
        )
        .await?,
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 4);
    p.visible.store(false, Ordering::SeqCst);
    assert_eq!(
        w.dispatch(
            &notify,
            Event {
                id: Uuid::now_v7(),
                ..event
            }
        )
        .await?,
        0
    );
    let body: String = sqlx::query_scalar("SELECT body FROM notifications")
        .fetch_one(&pool)
        .await?;
    assert!(!body.contains(&s.target_id.to_string()));
    assert!(!body.contains("death"));
    dispose(pool).await
}
#[tokio::test]
async fn postgres_http_authentication_input_and_feature_gate() -> Result<()> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let (w, _, s) = setup(pool.clone());
    let app = http::router(w);
    for (token, body, status) in [
        (
            "Bearer test-access",
            serde_json::to_string(&s)?,
            StatusCode::NO_CONTENT,
        ),
        (
            "Bearer test-access",
            "{\"unknown\":true}".into(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "Bearer bad",
            serde_json::to_string(&s)?,
            StatusCode::UNAUTHORIZED,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/stats/watches")
                    .header("authorization", token)
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
    }
    let cookie_only = app
        .clone()
        .oneshot(
            Request::get("/api/v1/stats/watches")
                .header("cookie", "session=test-access")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(cookie_only.status(), StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE stats_watch_budgets SET requests=60")
        .execute(&pool)
        .await?;
    let limited = app
        .oneshot(
            Request::get("/api/v1/stats/watches")
                .header("authorization", "Bearer test-access")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let (mut w, _, _) = setup(pool.clone());
    w.enabled = false;
    let response = http::router(w)
        .oneshot(Request::get("/api/v1/stats/watches").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    dispose(pool).await
}
#[test]
fn localized_messages_and_bad_input() -> Result<()> {
    assert!(message(Locale::Pl)?.0.contains("Obserwowane"));
    assert_eq!(message(Locale::En)?.0, "Watch list");
    assert!(serde_json::from_str::<Subscription>("{}").is_err());
    assert!(serde_json::from_str::<EventKind>("\"unknown\"").is_err());
    Ok(())
}
