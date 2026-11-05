#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
    routing::get,
};
use oteryn_community::today::{self, Config, Error};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
fn config() -> Config {
    Config {
        ingest_enabled: true,
        public_enabled: true,
        source_authority: "oteryn:game:primary".into(),
        worlds: [Uuid::parse_str(WORLD).unwrap()].into(),
    }
}
fn app(db: PgPool, c: Config) -> Router {
    today::router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san("daily-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        c,
    )
}
fn at() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
fn snapshot(now: i64, epoch: u64, revision: u64) -> String {
    format!(
        r#"{{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"{WORLD}","projection_epoch":"{epoch}","projection_revision":"{revision}","source_observed_at":"{now}","daily":[{{"reset_epoch":"1","effective_from":"{}","effective_until":"{}","creature_key":"rat","boss_key":"boss","history_complete_from":"{}","history":[{{"reset_epoch":"0","effective_from":"{}","effective_until":"{}","creature_key":"rat","boss_key":"boss"}}],"raids":[{{"raid_key":"raid:badger","scheduled_at":"{}","state":"running"}}]}}]}}"#,
        now - 100,
        now + 100,
        now - 200,
        now - 200,
        now - 100,
        now - 10
    )
}
fn watermark(now: i64, epoch: u64) -> String {
    format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"world_daily","projection_epoch":"{epoch}","complete_through":"{now}","observed_at":"{now}"}}"#
    )
}
async fn send(app: Router, path: &str, body: String, identity: Option<&str>) -> Response {
    let mut r = Request::post(format!("/internal/v2/game-public/{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if let Some(identity) = identity {
        r.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(identity).unwrap(),
            ));
    }
    app.oneshot(r).await.unwrap()
}
async fn ack(app: Router, path: &str, body: String, result: &str) {
    let r = send(app, path, body, Some("daily-test")).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(r.into_body(), 1024).await.unwrap(),
        format!(r#"{{"contract_version":2,"result":"{result}"}}"#)
    );
}
async fn empty(r: Response, status: StatusCode) {
    assert_eq!(r.status(), status);
    assert!(
        r.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    assert!(to_bytes(r.into_body(), 1024).await.unwrap().is_empty());
}
struct Database {
    root: PgPool,
    db: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping Today PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let root = PgPool::connect(&url).await?;
        // Identifier contains only a fixed prefix and generated hexadecimal digits.
        let schema = format!("today_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let db = PgPool::connect_with(
            root.connect_options()
                .as_ref()
                .clone()
                .options([("search_path", schema.as_str())]),
        )
        .await?;
        for migration in [
            include_str!("../../../migrations/20261105151225_t14_world_store.sql"),
            include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
            include_str!("../../../migrations/20261105190000_t25_world_registry.sql"),
            include_str!("../../../migrations/20261105220207_t14c_world_names.sql"),
            include_str!("../../../migrations/20261105231303_t88_world_daily.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&db).await?;
        }
        sqlx::query("INSERT INTO world_status(id,name) VALUES ($1,'Test World')")
            .bind(Uuid::parse_str(WORLD)?)
            .execute(&db)
            .await?;
        let version = Uuid::now_v7();
        sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES ($1,$2,'ready')")
            .bind(version)
            .bind("a".repeat(40))
            .execute(&db)
            .await?;
        sqlx::query("UPDATE world_active_version SET version_id=$1")
            .bind(version)
            .execute(&db)
            .await?;
        for (kind, key, en, pl) in [
            ("Creature", "rat", "Rat", "Szczur"),
            ("Creature", "boss", "Boss", "Boss PL"),
            ("Raid", "raid:badger", "Badger raid", "Rajd borsuków"),
        ] {
            sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ($1,$2,$3,'{}',true)").bind(version).bind(kind).bind(key).execute(&db).await?;
            sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES ($1,$2,true)")
                .bind(kind)
                .bind(key)
                .execute(&db)
                .await?;
            sqlx::query("INSERT INTO world_admin_names(kind,key,en,pl) VALUES ($1,$2,$3,$4)")
                .bind(kind)
                .bind(key)
                .bind(en)
                .bind(pl)
                .execute(&db)
                .await?;
        }
        eprintln!("Today PostgreSQL test started");
        Ok(Some(Self { root, db, schema }))
    }
    async fn close(self) -> Result {
        self.db.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
    async fn publish(&self, now: i64) {
        let a = app(self.db.clone(), config());
        ack(a.clone(), "world-daily", snapshot(now, 2, 17), "accepted").await;
        ack(a, "watermark", watermark(now, 2), "accepted").await;
    }
    async fn view(&self, now: i64) -> std::result::Result<today::DailyView, Error> {
        today::read(
            &self.db,
            &config(),
            Uuid::parse_str(WORLD).unwrap(),
            "en",
            now,
        )
        .await
    }
}
#[tokio::test]
async fn today_transport_refusals_and_bad_input() -> Result {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let a = app(db.clone(), config());
    empty(
        send(a.clone(), "world-daily", snapshot(at(), 2, 17), None).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    empty(
        send(
            a.clone(),
            "world-daily",
            snapshot(at(), 2, 17),
            Some("other"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    for (path, body) in [
        ("world-daily", "x".repeat(131073)),
        ("watermark", "x".repeat(513)),
        ("world-daily", "{}".into()),
        (
            "watermark",
            watermark(at(), 2).replace("world_daily", "unknown"),
        ),
        (
            "world-daily",
            snapshot(at(), 2, 17).replace("\"contract_version\":2", "\"contract_version\":1"),
        ),
    ] {
        empty(
            send(a.clone(), path, body, Some("daily-test")).await,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    for (name, value) in [
        ("content-length", "131073"),
        ("content-length", "01"),
        ("content-length", "1"),
        ("content-encoding", "gzip"),
        ("transfer-encoding", "chunked"),
    ] {
        let request = Request::post("/internal/v2/game-public/world-daily")
            .header("content-type", "application/json")
            .header(name, value)
            .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                "daily-test",
            )?))
            .body(Body::from(snapshot(at(), 2, 17)))?;
        empty(a.clone().oneshot(request).await?, StatusCode::BAD_REQUEST).await;
    }
    let mut disabled = config();
    disabled.ingest_enabled = false;
    empty(
        send(
            app(db, disabled),
            "world-daily",
            snapshot(at(), 2, 17),
            Some("daily-test"),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    Ok(())
}
#[tokio::test]
async fn today_disabled_preserves_existing_page_without_database() -> Result {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let a = today::mount(
        Router::new().route(
            "/pl/today",
            get(|| async { axum::response::Html("<body><main>Editorial</main></body>") }),
        ),
        db,
        Config::default(),
    );
    let r = a
        .oneshot(Request::get("/pl/today").body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(r.into_body(), 4096).await?,
        "<body><main>Editorial</main></body>"
    );
    Ok(())
}
#[tokio::test]
async fn today_postgres_success_replay_history_and_public_render() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    let a = app(d.db.clone(), config());
    ack(a.clone(), "world-daily", snapshot(now, 2, 17), "accepted").await;
    ack(
        a.clone(),
        "world-daily",
        snapshot(now, 2, 17).replace(
            &format!("\"source_observed_at\":\"{now}\""),
            &format!("\"source_observed_at\":\"{}\"", now + 1),
        ),
        "accepted",
    )
    .await;
    let v = d.view(now).await.unwrap();
    assert_eq!(v.current.creature, "Rat");
    assert_eq!(v.history.len(), 1);
    assert_eq!(v.raids[0].name, "Badger raid");
    let page = oteryn_content::today::router(d.db.clone());
    let r = today::mount(page, d.db.clone(), config())
        .oneshot(Request::get("/pl/today").body(Body::empty())?)
        .await?;
    let html = String::from_utf8(to_bytes(r.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Ogłoszenia"));
    assert!(html.contains("Aktualności"));
    assert!(html.contains("Szczur"));
    assert!(html.contains("Historia wzmocnień"));
    assert!(html.contains("Część źródeł jest niedostępna."));
    assert!(!html.contains("Wszystkie źródła są teraz niedostępne."));
    assert!(html.contains("<header>"));
    assert!(!html.contains("reset_epoch"));
    if let Ok(path) = std::env::var("PORTAL_TODAY_RENDER_PATH") {
        std::fs::write(path, &html)?;
    }
    ack(a, "world-daily", snapshot(now, 2, 16), "superseded").await;
    d.close().await
}
#[tokio::test]
async fn today_postgres_conflict_quarantines_after_commit_and_retry() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    let a = app(d.db.clone(), config());
    empty(
        send(
            a.clone(),
            "world-daily",
            snapshot(now, 2, 17).replace("\"creature_key\":\"rat\"", "\"creature_key\":\"boss\""),
            Some("daily-test"),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    ack(a.clone(), "world-daily", snapshot(now, 2, 17), "accepted").await;
    ack(a, "world-daily", snapshot(now, 2, 18), "accepted").await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}
#[tokio::test]
async fn today_postgres_freshness_interval_and_privacy_fences() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    assert!(d.view(now + 29).await.is_ok());
    assert_eq!(d.view(now + 30).await.unwrap_err(), Error::Unavailable);
    assert_eq!(d.view(now - 2).await.unwrap_err(), Error::Unavailable);
    let edge = snapshot(now, 2, 18)
        .replace(
            &format!("\"effective_from\":\"{}\"", now - 100),
            &format!("\"effective_from\":\"{now}\""),
        )
        .replace(
            &format!("\"effective_until\":\"{}\"", now + 100),
            &format!("\"effective_until\":\"{}\"", now + 3),
        )
        .replace(
            &format!("\"scheduled_at\":\"{}\"", now - 10),
            &format!("\"scheduled_at\":\"{now}\""),
        );
    ack(app(d.db.clone(), config()), "world-daily", edge, "accepted").await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    assert!(d.view(now + 1).await.is_ok());
    assert_eq!(d.view(now + 2).await.unwrap_err(), Error::Unavailable);
    ack(
        app(d.db.clone(), config()),
        "world-daily",
        snapshot(now, 2, 19),
        "accepted",
    )
    .await;

    sqlx::query("UPDATE world_visibility SET allowed=false WHERE key='boss'")
        .execute(&d.db)
        .await?;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    sqlx::query("UPDATE world_visibility SET allowed=true")
        .execute(&d.db)
        .await?;
    sqlx::query("DELETE FROM world_admin_names WHERE key='rat'")
        .execute(&d.db)
        .await?;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}
#[tokio::test]
async fn today_postgres_epoch_raise_and_baseline_withdrawal() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    let a = app(d.db.clone(), config());
    ack(a.clone(), "watermark", watermark(now, 3), "accepted").await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    ack(a.clone(), "world-daily", snapshot(now, 2, 99), "superseded").await;
    ack(a.clone(), "world-daily", snapshot(now, 3, 1), "accepted").await;
    assert!(d.view(now).await.is_ok());
    let raw = snapshot(now, 3, 2);
    let withdrawal = format!("{}\"daily\":[]}}", raw.split("\"daily\":").next().unwrap());
    ack(a, "world-daily", withdrawal, "accepted").await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}
#[tokio::test]
async fn today_postgres_bad_schema_scope_and_unknown_content() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    let a = app(d.db.clone(), config());
    let raw = snapshot(now, 2, 17);
    for body in [
        format!("{raw}\n"),
        raw.replace("\"reset_epoch\":\"1\"", "\"reset_epoch\":\"01\""),
        raw.replace("running", "scheduled"),
        raw.replace(
            &format!("\"scheduled_at\":\"{}\"", now - 10),
            &format!("\"scheduled_at\":\"{}\"", now + 1),
        ),
        raw.replace("\"daily\":[", "\"extra\":0,\"daily\":["),
        raw.replace(
            "\"contract_version\":2",
            "\"contract_version\":2,\"contract_version\":2",
        ),
    ] {
        empty(
            send(a.clone(), "world-daily", body, Some("daily-test")).await,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    empty(
        send(
            a.clone(),
            "world-daily",
            raw.replace(WORLD, "01934f10-7c02-7001-805b-3b1122334402"),
            Some("daily-test"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    ack(
        a.clone(),
        "world-daily",
        raw.replace("\"boss_key\":\"boss\"", "\"boss_key\":\"unknown\""),
        "accepted",
    )
    .await;
    ack(a, "watermark", watermark(now, 2), "accepted").await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}
#[tokio::test]
async fn today_postgres_data_unavailable_and_bad_query() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    let a = today::mount(Router::new(), d.db.clone(), config());
    for (path, status) in [
        ("/en/today/data", StatusCode::OK),
        ("/en/today/data?world=bad", StatusCode::BAD_REQUEST),
        ("/en/today/data?score=1", StatusCode::BAD_REQUEST),
        ("/en/today/widget", StatusCode::OK),
    ] {
        let r = a
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(r.status(), status);
        assert!(r.headers()["cache-control"].to_str()?.contains("no-store"));
    }
    sqlx::query("UPDATE today_watermarks SET complete_through=complete_through-31")
        .execute(&d.db)
        .await?;
    let r = a
        .clone()
        .oneshot(Request::get("/en/today/data").body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(to_bytes(r.into_body(), 1024).await?.to_vec())?;
    assert!(!body.contains("Rat"));
    let r = a
        .oneshot(Request::get("/today/widget?lang=en").body(Body::empty())?)
        .await?;
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(to_bytes(r.into_body(), 4096).await?.to_vec())?;
    assert!(body.contains("World daily data is currently unavailable."));
    assert!(!body.contains("Dzienne dane"));
    d.close().await
}
#[tokio::test]
async fn today_postgres_retention_keeps_fences_and_global_epoch() -> Result {
    use oteryn_db::outbox::Cleanup;
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    sqlx::query("UPDATE today_snapshots SET received_at=now()-interval '31 days'")
        .execute(&d.db)
        .await?;
    today::Retention(d.db.clone())
        .clean()
        .await
        .map_err(std::io::Error::other)?;
    let content: Option<String> = sqlx::query_scalar("SELECT content FROM today_snapshots")
        .fetch_one(&d.db)
        .await?;
    assert!(content.is_none());
    ack(
        app(d.db.clone(), config()),
        "world-daily",
        snapshot(now, 2, 16),
        "superseded",
    )
    .await;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=4")
        .execute(&d.db)
        .await?;
    ack(
        app(d.db.clone(), config()),
        "watermark",
        watermark(now, 3),
        "superseded",
    )
    .await;
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}

#[tokio::test]
async fn today_postgres_complete_world_baseline_and_concurrent_conflict() -> Result {
    let Some(d) = Database::new().await? else {
        return Ok(());
    };
    let now = at();
    d.publish(now).await;
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO world_status(id,name) VALUES ($1,'Second World')")
        .bind(other)
        .execute(&d.db)
        .await?;
    let mut c = config();
    c.worlds.insert(other);
    assert_eq!(
        today::read(&d.db, &c, Uuid::parse_str(WORLD)?, "en", now)
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    let a = app(d.db.clone(), c.clone());
    ack(
        a.clone(),
        "world-daily",
        snapshot(now, 2, 17).replace(WORLD, &other.to_string()),
        "accepted",
    )
    .await;
    assert!(
        today::read(&d.db, &c, Uuid::parse_str(WORLD)?, "en", now)
            .await
            .is_ok()
    );
    // A lost acknowledgement is recovered using exactly the same publication identity.
    drop(
        send(
            a.clone(),
            "world-daily",
            snapshot(now, 2, 18),
            Some("daily-test"),
        )
        .await,
    );
    ack(a.clone(), "world-daily", snapshot(now, 2, 18), "accepted").await;
    let first = send(
        a.clone(),
        "world-daily",
        snapshot(now, 2, 19),
        Some("daily-test"),
    );
    let second = send(
        a,
        "world-daily",
        snapshot(now, 2, 19).replace("running", "done"),
        Some("daily-test"),
    );
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.status(), second.status()];
    assert!(statuses.contains(&StatusCode::OK));
    assert!(statuses.contains(&StatusCode::CONFLICT));
    assert_eq!(d.view(now).await.unwrap_err(), Error::Unavailable);
    d.close().await
}
