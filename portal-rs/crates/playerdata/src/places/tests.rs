#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    middleware,
    routing::get,
};
use oteryn_web::session::{BrowserSession, PrivilegeChange, SessionLayer};
use sqlx::{PgConnection, PgPool};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Debug, Default)]
struct Source {
    revoked: AtomicBool,
}
impl JournalSource for Source {
    fn ready(&self) -> bool {
        true
    }
    fn verify<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a Session,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + 'a>> {
        Box::pin(async { Ok(!self.revoked.load(Ordering::SeqCst)) })
    }
}
fn session(account: Uuid, now: i64) -> Session {
    let end = (now / 86400 - 3) * 86400 + 3600;
    Session {
        source_authority: "fixture.test".into(),
        session_id: Uuid::now_v7(),
        account_id: account,
        character_id: Uuid::now_v7(),
        world_id: Uuid::parse_str("019a5000-0000-7000-8000-000000000006").unwrap(),
        character_revision: 1,
        projection_revision: 1,
        projection_epoch: 1,
        recovery_generation: 1,
        stream_generation: Uuid::now_v7(),
        catalog_revision: "fixture.v1".into(),
        price_revision: "npc.v1".into(),
        place_key: "place:cave".into(),
        vocation_key: "vocation:knight".into(),
        level: 20,
        started_at: end - 3600,
        ended_at: end,
        duration_seconds: 3600,
        experience: 1000,
        profit: -100,
    }
}
async fn database() -> Option<PgPool> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL places test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("places_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |conn, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "SET search_path TO {schema},public"
                )))
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    root.close().await;
    Some(pool)
}
async fn account(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'test-only','active','en',now())").bind(id).bind(format!("{id}@example.invalid")).execute(pool).await.unwrap();
    id
}
async fn opt_in(store: &PlacesStore, id: Uuid) {
    store.set_preference(id, id, true).await.unwrap();
    // Fictional prior consent, before capture; do not weaken the production cutoff.
    sqlx::query(
        "UPDATE pd_places_preferences SET updated_at=now()-interval '30 days' WHERE account_id=$1",
    )
    .bind(id)
    .execute(store.pool())
    .await
    .unwrap();
}
async fn now(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT extract(epoch FROM now())::bigint")
        .fetch_one(pool)
        .await
        .unwrap()
}
fn store(pool: &PgPool, source: Arc<dyn JournalSource>) -> PlacesStore {
    PlacesStore::new(
        pool.clone(),
        Flags {
            aggregate: true,
            ingest: true,
        },
        source,
    )
}

#[tokio::test]
async fn postgres_thresholds_delay_rates_and_immutable_retry() {
    let Some(pool) = database().await else { return };
    let store = store(&pool, Arc::new(Source::default()));
    let now = now(&pool).await;
    let mut accounts = Vec::new();
    for _ in 0..5 {
        let id = account(&pool).await;
        opt_in(&store, id).await;
        accounts.push(id)
    }
    let mut sessions = Vec::new();
    for i in 0..9 {
        let s = session(accounts[i % 5], now);
        store.ingest(&s).await.unwrap();
        sessions.push(s)
    }
    assert!(store.daily(accounts[0]).await.unwrap().is_empty());
    // Ten sessions but only four accounts is still suppressed in another place.
    for i in 0..10 {
        let mut s = session(accounts[i % 4], now);
        s.place_key = "place:forest".into();
        store.ingest(&s).await.unwrap();
    }
    assert!(store.daily(accounts[0]).await.unwrap().is_empty());
    let last = session(accounts[4], now);
    store.ingest(&last).await.unwrap();
    store.ingest(&last).await.unwrap();
    let rows = store.daily(accounts[0]).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sessions, 10);
    assert_eq!(rows[0].experience_per_hour, "1000");
    assert_eq!(rows[0].profit_per_hour, "-100");
    let mut changed = last.clone();
    changed.experience += 1;
    assert!(matches!(store.ingest(&changed).await, Err(Error::Conflict)));
    let json = serde_json::to_string(&rows).unwrap();
    for id in accounts {
        assert!(!json.contains(&id.to_string()))
    }
    // Yesterday is unavailable even after an early session's 24h birthday.
    for s in &mut sessions {
        s.session_id = Uuid::now_v7();
        s.started_at = (now / 86400 - 1) * 86400 + 1;
        s.ended_at = s.started_at + 3600;
        s.place_key = "place:recent".into();
        store.ingest(s).await.unwrap();
    }
    assert_eq!(store.daily(last.account_id).await.unwrap().len(), 1);
    eprintln!("PostgreSQL T81: thresholds/delay/idempotency passed");
    pool.close().await;
}

#[tokio::test]
async fn postgres_owner_optout_recompute_and_replay_fence() {
    let Some(pool) = database().await else { return };
    let store = store(&pool, Arc::new(Source::default()));
    let now = now(&pool).await;
    let mut accounts = Vec::new();
    for _ in 0..5 {
        let id = account(&pool).await;
        assert!(!store.preference(id, id).await.unwrap());
        opt_in(&store, id).await;
        accounts.push(id)
    }
    let mut first = None;
    for i in 0..10 {
        let s = session(accounts[i % 5], now);
        store.ingest(&s).await.unwrap();
        if i == 0 {
            first = Some(s)
        }
    }
    assert_eq!(store.daily(accounts[0]).await.unwrap().len(), 1);
    assert!(matches!(
        store.preference(accounts[0], accounts[1]).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        store.set_preference(accounts[0], accounts[1], false).await,
        Err(Error::Denied)
    ));
    store
        .set_preference(accounts[0], accounts[0], false)
        .await
        .unwrap();
    assert!(store.daily(accounts[1]).await.unwrap().is_empty());
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_places_sessions WHERE account_id=$1")
            .bind(accounts[0])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained, 0);
    opt_in(&store, accounts[0]).await;
    assert!(matches!(
        store.ingest(&first.unwrap()).await,
        Err(Error::Denied)
    ));
    assert!(store.daily(accounts[1]).await.unwrap().is_empty());
    eprintln!("PostgreSQL T81: owner denial/exclusion/replay passed");
    pool.close().await;
}

#[tokio::test]
async fn postgres_unknown_evidence_bad_input_and_flags_deny() {
    let Some(pool) = database().await else { return };
    let source = Arc::new(Source::default());
    let store = store(&pool, source.clone());
    let id = account(&pool).await;
    let now = now(&pool).await;
    let mut s = session(id, now);
    assert!(matches!(store.ingest(&s).await, Err(Error::Denied)));
    opt_in(&store, id).await;
    s.duration_seconds = 0;
    assert!(matches!(store.ingest(&s).await, Err(Error::Invalid)));
    s.duration_seconds = 3600;
    s.character_revision = 0;
    assert!(matches!(store.ingest(&s).await, Err(Error::Invalid)));
    s.character_revision = 1;
    s.place_key = "place:<script>".into();
    assert!(matches!(store.ingest(&s).await, Err(Error::Invalid)));
    s.place_key = "place:cave".into();
    store.ingest(&s).await.unwrap();
    source.revoked.store(true, Ordering::SeqCst);
    assert!(matches!(store.daily(id).await, Err(Error::Denied)));
    assert!(matches!(store.ingest(&s).await, Err(Error::Denied)));
    let unresolved = PlacesStore::new(
        pool.clone(),
        Flags {
            aggregate: true,
            ingest: true,
        },
        Arc::new(UnresolvedSource),
    );
    assert!(matches!(unresolved.ingest(&s).await, Err(Error::Denied)));
    assert!(matches!(unresolved.daily(id).await, Err(Error::Disabled)));
    let disabled = PlacesStore::new(pool.clone(), Flags::default(), source);
    assert!(matches!(disabled.ingest(&s).await, Err(Error::Disabled)));
    assert!(matches!(disabled.daily(id).await, Err(Error::Disabled)));
    sqlx::query(
        "UPDATE pd_places_sessions SET ended_at=now()-interval '91 days' WHERE account_id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    PlacesRetention { pool: pool.clone() }
        .purge()
        .await
        .unwrap();
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_places_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained, 0);
    eprintln!("PostgreSQL T81: bad input/unknown evidence/flags/retention passed");
    pool.close().await;
}

#[tokio::test]
async fn postgres_http_owner_form_csrf_and_localization() {
    let Some(pool) = database().await else { return };
    let id = account(&pool).await;
    let store = store(&pool, Arc::new(Source::default()));
    let login = Router::new()
        .route(
            "/test-login",
            get(
                move |Extension(session): Extension<BrowserSession>| async move {
                    oteryn_web::session::rotate(&session, PrivilegeChange::Login(id))
                        .await
                        .unwrap();
                    StatusCode::OK
                },
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ));
    let app = router(store.clone())
        .merge(login)
        .layer(Extension(oteryn_web::csrf::CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..Default::default()
        }));
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/en/atlas/places")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(denied.headers()[header::CACHE_CONTROL], "private, no-store");
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/test-login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/pl/atlas/places")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let html =
        String::from_utf8(to_bytes(page.into_body(), 65536).await.unwrap().to_vec()).unwrap();
    assert!(html.contains("Mapa łowisk"));
    assert!(html.contains("name=\"participating\""));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/en/atlas/places/participation")
                .method("POST")
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("participating=true"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::FORBIDDEN);
    for (value, status) in [
        ("true", StatusCode::SEE_OTHER),
        ("maybe", StatusCode::BAD_REQUEST),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/en/atlas/places/participation")
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(format!("_csrf={csrf}&participating={value}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let cross_origin = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/en/atlas/places/participation")
                .method("POST")
                .header(header::ORIGIN, "https://other.example.invalid")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&participating=false")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cross_origin.status(), StatusCode::FORBIDDEN);
    assert!(store.preference(id, id).await.unwrap());
    let filtered = app
        .oneshot(
            Request::builder()
                .uri("/en/atlas/places?account_id=other")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(filtered.status(), StatusCode::BAD_REQUEST);
    eprintln!("PostgreSQL T81: HTTP owner/CSRF/Polish page passed");
    pool.close().await;
}

#[test]
fn exact_daily_delay_boundary_and_distinct_sessions() {
    let now = 2_000_000_000 / 86400 * 86400;
    let accounts: Vec<_> = (0..5).map(|_| Uuid::now_v7()).collect();
    let mut sessions: Vec<_> = (0..10).map(|i| session(accounts[i % 5], now)).collect();
    let day = now / 86400 - 2;
    for s in &mut sessions {
        s.ended_at = day * 86400 + 3600;
        s.started_at = s.ended_at - 3600
    }
    assert!(super::store::aggregate(&sessions, now - 1).is_empty());
    assert_eq!(super::store::aggregate(&sessions, now).len(), 1);
    sessions[9] = sessions[0].clone();
    assert!(super::store::aggregate(&sessions, now).is_empty());
}
