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
    fn locale<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
    ) -> PolicyFuture<'a, Locale> {
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
    oteryn_db::migrate(&pool).await?;
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
    let template_kind: String = sqlx::query_scalar("SELECT template_kind FROM email_outbox")
        .fetch_one(&pool)
        .await?;
    assert_eq!(template_kind, "news");
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

#[tokio::test]
async fn readiness_shows_localized_source_states() -> Result<()> {
    let db = PgPool::connect_lazy("postgres://unused@localhost:1/unused")?;
    let app = http::configured_with(db, web::Readiness::default());
    for (lang, title, unavailable) in [
        ("en", "Statistics availability", "Unavailable"),
        ("pl", "Dostępność statystyk", "Niedostępne"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(format!("/{lang}/stats")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let body = axum::body::to_bytes(response.into_body(), 64_000).await?;
        let html = std::str::from_utf8(&body)?;
        assert!(html.contains(title));
        assert!(html.contains(unavailable));
        assert!(!html.contains("/account/stats-watches"));
    }
    let invalid = app
        .oneshot(Request::get("/fr/stats").body(Body::empty())?)
        .await?;
    assert_eq!(invalid.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn postgres_configured_watch_access_lifecycle() -> Result<()> {
    use sha2::{Digest, Sha256};
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES ('watch@example.invalid','fixture','en','active',now()) RETURNING id")
        .fetch_one(&pool).await?;
    let session = "a".repeat(64);
    let csrf = "b".repeat(64);
    let session_hash = Sha256::digest(&session).to_vec();
    sqlx::query("INSERT INTO web_sessions(id_hash, account_id, csrf_token, user_agent_summary, idle_expires_at, absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 hour')")
        .bind(&session_hash).bind(account).bind(&csrf).execute(&pool).await?;
    let mut config = web::Readiness::default();
    config.watches = true;
    let app = http::configured_with(pool.clone(), config);
    let settings = "/en/account/stats-watches";
    let cookie = format!("{}={session}", oteryn_web::session::COOKIE_NAME);
    for (body, expected) in [
        ("action=issue".to_owned(), StatusCode::FORBIDDEN),
        (
            format!("action=invalid&_csrf={csrf}"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(settings)
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    let issued = app
        .clone()
        .oneshot(
            Request::post(settings)
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("action=issue&_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(issued.status(), StatusCode::OK);
    assert_eq!(issued.headers()["cache-control"], "private, no-store");
    let body = axum::body::to_bytes(issued.into_body(), 64_000).await?;
    let (_, rest) = std::str::from_utf8(&body)?
        .split_once("id=\"watch-token\" type=\"text\" value=\"")
        .ok_or("missing token field")?;
    let (token, _) = rest.split_once('"').ok_or("missing token field end")?;
    let watch = Subscription {
        target_kind: TargetKind::Guild,
        target_id: Uuid::now_v7(),
        event_kind: EventKind::GuildChange,
    };
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO stats_watches(account_id,target_kind,target_id,event_kind) VALUES($1,'guild',$3,'guild_change'),($2,'guild',$3,'guild_change')")
        .bind(account).bind(other).bind(watch.target_id).execute(&pool).await?;
    let path = "/api/v1/stats/watches";
    for (method, data, expected) in [
        ("GET", String::new(), StatusCode::SERVICE_UNAVAILABLE),
        (
            "POST",
            serde_json::to_string(&watch)?,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        ("DELETE", "{}".into(), StatusCode::UNPROCESSABLE_ENTITY),
        (
            "DELETE",
            serde_json::to_string(&watch)?,
            StatusCode::NO_CONTENT,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(data))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    let remaining: Vec<Uuid> = sqlx::query_scalar("SELECT account_id FROM stats_watches")
        .fetch_all(&pool)
        .await?;
    assert_eq!(remaining, vec![other]);
    sqlx::query("INSERT INTO stats_watches(account_id,target_kind,target_id,event_kind) VALUES($1,'guild',$2,'guild_change')")
        .bind(account).bind(watch.target_id).execute(&pool).await?;
    for (body, expected) in [
        ("action=remove_all".to_owned(), StatusCode::FORBIDDEN),
        (
            format!("action=remove_all&_csrf={csrf}"),
            StatusCode::SEE_OTHER,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(settings)
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    let remaining: Vec<Uuid> = sqlx::query_scalar("SELECT account_id FROM stats_watches")
        .fetch_all(&pool)
        .await?;
    assert_eq!(remaining, vec![other]);
    let authorization = format!("Bearer {token}");
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    let refused = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("authorization", &authorization)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert_eq!(
        access::owner(&pool, token)
            .await
            .map_err(|code| code.to_string())?,
        None
    );

    // Account status changes revoke the browser session. Sign in again before
    // exercising expiration and explicit revocation for a new credential.
    let session = "c".repeat(64);
    let csrf = "d".repeat(64);
    let session_hash = Sha256::digest(&session).to_vec();
    let cookie = format!("{}={session}", oteryn_web::session::COOKIE_NAME);
    sqlx::query("INSERT INTO web_sessions(id_hash, account_id, csrf_token, user_agent_summary, idle_expires_at, absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 hour')")
        .bind(&session_hash).bind(account).bind(&csrf).execute(&pool).await?;
    let (token, _) = access::update(&pool, account, &session_hash, access::Action::Issue)
        .await
        .map_err(|code| code.to_string())?
        .ok_or("credential was not issued")?;
    let authorization = format!("Bearer {token}");
    let expired = sqlx::query("UPDATE stats_watch_access SET expires_at=now() WHERE token_hash=$1")
        .bind(Sha256::digest(&token).to_vec())
        .execute(&pool)
        .await?;
    assert_eq!(expired.rows_affected(), 1);
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("authorization", &authorization)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let revoked = app
        .oneshot(
            Request::post(settings)
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("action=revoke&_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(revoked.status(), StatusCode::SEE_OTHER);
    let remaining_tokens: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_watch_access")
        .fetch_one(&pool)
        .await?;
    assert_eq!(remaining_tokens, 0);
    let (previous, _) = access::update(&pool, account, &session_hash, access::Action::Issue)
        .await
        .map_err(|code| code.to_string())?
        .ok_or("credential was not issued")?;
    assert_eq!(
        access::owner(&pool, &previous)
            .await
            .map_err(|code| code.to_string())?,
        Some(account)
    );
    let (replacement, _) = access::update(&pool, account, &session_hash, access::Action::Issue)
        .await
        .map_err(|code| code.to_string())?
        .ok_or("credential was not replaced")?;
    assert!(
        access::owner(&pool, &previous)
            .await
            .map_err(|code| code.to_string())?
            .is_none()
    );
    assert_eq!(
        access::owner(&pool, &replacement)
            .await
            .map_err(|code| code.to_string())?,
        Some(account)
    );
    sqlx::query("UPDATE web_sessions SET id_hash=$1 WHERE id_hash=$2")
        .bind(Sha256::digest("e".repeat(64)).to_vec())
        .bind(session_hash)
        .execute(&pool)
        .await?;
    assert!(
        access::owner(&pool, &replacement)
            .await
            .map_err(|code| code.to_string())?
            .is_none()
    );
    dispose(pool).await
}
