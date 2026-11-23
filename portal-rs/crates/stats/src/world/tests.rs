use super::*;
use crate::store::{Permissions, Snapshot, partitions, snapshot, source_event};
use sqlx::postgres::PgPoolOptions;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Public(bool);
impl Visibility for Public {
    fn permissions(&self, _: Uuid) -> Permissions {
        Permissions {
            visible: self.0,
            deaths: self.0,
            ..Permissions::default()
        }
    }
}
impl http::Policy for Public {
    fn available(&self) -> bool {
        true
    }
}
#[tokio::test]
async fn world_postgres_observations_privacy_and_gaps() -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping world PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("world_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105210000_t65_statistics.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261117230011_r9_sta.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"
    ))
    .execute(&db)
    .await?;
    let date = Date::from_calendar_date(2026, time::Month::November, 1)?;
    let end = date.next_day().ok_or("date")?;
    let world = Uuid::now_v7();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105190000_t25_world_registry.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::query("INSERT INTO world_status(id,name) VALUES($1,'Observation World')")
        .bind(world)
        .execute(&db)
        .await?;
    let id = Uuid::now_v7();
    let mut tx = db.begin().await?;
    partitions(&mut tx, date).await?;
    snapshot(
        &mut tx,
        &Snapshot {
            character_ref: id,
            day: date,
            level: 42,
            experience: None,
            skills: None,
            achievement_points: None,
            minutes_online: None,
            name: "Public".into(),
            world,
            guild: None,
            observed_at: 1,
        },
    )
    .await?;
    source_event(&mut tx, id, date, "death", "one", &serde_json::json!({})).await?;
    source_event(&mut tx, id, date, "death", "one", &serde_json::json!({})).await?;
    tx.commit().await?;
    let config = Config { enabled: true };
    let report = read(&db, config, world, date, end, &Public(true))
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(report.days[0].levels.get(&42), Some(&1));
    assert_eq!(report.days[0].deaths, Some(1));
    assert_eq!(report.days[1].deaths, None);
    assert!(report.online.is_none());
    let hidden = read(&db, config, world, date, end, &Public(false))
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        hidden
            .days
            .iter()
            .all(|d| d.levels.is_empty() && d.deaths.is_none())
    );
    assert_eq!(
        read(&db, config, world, end, date, &Public(true))
            .await
            .err(),
        Some(Error::BadInput)
    );
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for lang in ["pl", "en"] {
        let app = http::router::<()>(db.clone(), config, std::sync::Arc::new(Public(true)));
        let uri = format!("/{lang}/stats/world?world={world}&start=2026-11-01&end=2026-11-02");
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let body = axum::body::to_bytes(response.into_body(), 65536).await?;
        let body = std::str::from_utf8(&body)?;
        assert!(body.contains(&format!("lang=\"{lang}\"")));
        assert!(body.contains("42"));
        assert!(body.contains(if lang == "en" {
            "Monster and boss deaths (UTC day)"
        } else {
            "Zgony potworów i bossów (dzień UTC)"
        }));
        assert!(!body.contains(&id.to_string()));
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}
#[tokio::test]
async fn routes_disabled_bad_input_and_privacy_unavailable() -> Result {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let db = PgPool::connect_lazy("postgres://unused@localhost:1/unused")?;
    let request = |uri: &str| Request::builder().uri(uri).body(Body::empty());
    let disabled = http::router::<()>(
        db.clone(),
        Config::default(),
        std::sync::Arc::new(Public(true)),
    );
    assert_eq!(
        disabled
            .oneshot(request("/en/stats/world")?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let enabled = http::router::<()>(
        db.clone(),
        Config { enabled: true },
        std::sync::Arc::new(Public(true)),
    );
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            enabled.oneshot(request("/en/stats/world?world=bad")?)
        )
        .await??
        .status(),
        StatusCode::BAD_REQUEST
    );
    let future = format!(
        "/en/stats/world?world={}&start=2099-01-01&end=2099-01-01",
        Uuid::now_v7()
    );
    assert_eq!(
        http::router::<()>(
            db.clone(),
            Config { enabled: true },
            std::sync::Arc::new(Unavailable)
        )
        .oneshot(request(&future)?)
        .await?
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        read(
            &db,
            Config::default(),
            Uuid::now_v7(),
            Date::MIN,
            Date::MAX,
            &Public(true)
        )
        .await
        .err(),
        Some(Error::Disabled)
    );
    Ok(())
}

struct Unavailable;
impl Visibility for Unavailable {
    fn permissions(&self, _: Uuid) -> Permissions {
        Permissions::default()
    }
}
impl http::Policy for Unavailable {
    fn available(&self) -> bool {
        false
    }
}
#[tokio::test]
async fn missing_privacy_is_localized_unavailable() -> Result {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let db = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://unused@localhost:1/unused")?;
    let app = http::router::<()>(
        db,
        Config { enabled: true },
        std::sync::Arc::new(Unavailable),
    );
    let uri = format!(
        "/en/stats/world?world={}&start=2026-11-01&end=2026-11-02",
        Uuid::now_v7()
    );
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    assert!(std::str::from_utf8(&body)?.contains("Unavailable"));
    Ok(())
}

#[tokio::test]
async fn registered_world_picker_works_without_privacy_metrics() -> Result {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED picker PostgreSQL test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("picker_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    oteryn_db::migrate(&db).await?;
    let config = Config { enabled: true };
    let world = Uuid::now_v7();
    for lang in ["pl", "en"] {
        let app = http::router::<()>(db.clone(), config, std::sync::Arc::new(Unavailable));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/{lang}/stats/world"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
    }
    sqlx::query("INSERT INTO world_status(id,name) VALUES($1,$2)")
        .bind(world)
        .bind("Registered <World>")
        .execute(&db)
        .await?;
    for lang in ["pl", "en"] {
        for suffix in ["", "?"] {
            let app = http::router::<()>(db.clone(), config, std::sync::Arc::new(Unavailable));
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/{lang}/stats/world{suffix}"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["cache-control"], "private, no-store");
            let body = to_bytes(response.into_body(), 65536).await?;
            let body = std::str::from_utf8(&body)?;
            assert!(body.contains("name=\"world\""));
            assert!(body.contains("name=\"start\""));
            assert!(body.contains("name=\"end\""));
            assert!(body.contains(&world.to_string()));
            assert!(body.contains("Registered &#60;World&#62;"));
        }
        let app = http::router::<()>(db.clone(), config, std::sync::Arc::new(Unavailable));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/{lang}/stats/world?world={world}&start=2026-11-01&end=2026-11-02"
                    ))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}

struct KillCoverage {
    epoch: &'static str,
    start: i64,
    through: i64,
}

impl crate::online::CoveragePolicy for KillCoverage {
    fn coverage(
        &self,
        family: crate::online::Family,
    ) -> std::result::Result<crate::online::Coverage, crate::online::Error> {
        assert_eq!(family, crate::online::Family::WorldDailyKills);
        Ok(crate::online::Coverage {
            epoch: self.epoch.into(),
            start: self.start,
            through: self.through,
        })
    }
}

#[tokio::test]
async fn qualified_daily_kills_render_with_utc_coverage_and_freshness() -> Result {
    use crate::online::{self, Family, Receipt};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use time::{Duration, OffsetDateTime};
    use tower::ServiceExt;

    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("world_kills_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    oteryn_db::migrate(&db).await?;
    let world = Uuid::now_v7();
    sqlx::query("INSERT INTO world_status(id,name) VALUES($1,'Qualification World')")
        .bind(world)
        .execute(&db)
        .await?;

    let now = OffsetDateTime::now_utc();
    let start = now.date() - Duration::days(2);
    let day = start.midnight().assume_utc().unix_timestamp();
    let today = now.date().midnight().assume_utc().unix_timestamp();
    let at = now.unix_timestamp();
    let authority = "oteryn:game:qualification";
    let feed = online::Config {
        enabled: true,
        world_totals_enabled: true,
        source_authority: authority.into(),
        ..online::Config::default()
    };
    let publication = |day: i64, monsters: u64, bosses: u64, complete: bool| {
        let end = day + 86400;
        format!(
            r#"{{"contract_version":2,"operation":"PublishWorldDailyKillsV2","source_authority":"{authority}","world_id":"{world}","day_start":"{day}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{at}","days":[{{"day_end":"{end}","monsters_killed":"{monsters}","bosses_killed":"{bosses}","complete":{complete}}}]}}"#
        )
    };
    for data in [
        publication(day, 12345, 0, true),
        publication(today, 34, 2, false),
    ] {
        assert!(matches!(
            online::ingest(
                &db,
                &feed,
                Some(Family::WorldDailyKills),
                data.as_bytes(),
                at
            )
            .await,
            Ok(Receipt::Accepted)
        ));
    }
    let watermark = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{authority}","family":"world_daily_kills","projection_epoch":"1","complete_through":"{at}","observed_at":"{at}"}}"#
    );
    assert!(matches!(
        online::ingest(&db, &feed, None, watermark.as_bytes(), at).await,
        Ok(Receipt::Accepted)
    ));

    let app = http::router_with_kills::<()>(
        db.clone(),
        Config { enabled: true },
        Arc::new(Unavailable),
        Some(http::KillFeed {
            config: feed,
            coverage: Arc::new(KillCoverage {
                epoch: "1",
                start: day,
                through: at,
            }),
        }),
    );
    let path = format!(
        "/en/stats/world?world={world}&start={start}&end={}",
        now.date()
    );
    let request = |path: &str| Request::get(path).body(Body::empty());
    let response = app.clone().oneshot(request(&path)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let body = String::from_utf8(to_bytes(response.into_body(), 60_000).await?.to_vec())?;
    assert!(body.contains("<dd>12345</dd>"));
    assert!(body.contains("<dd>0</dd>"));
    assert!(body.contains("<dd>34</dd>"));
    assert!(body.contains("<p>Complete</p>"));
    assert!(body.contains("<p>Provisional</p>"));
    assert_eq!(body.matches("<dd>").count(), 4);

    let unknown = path.replace(&world.to_string(), &Uuid::now_v7().to_string());
    assert_eq!(
        app.clone().oneshot(request(&unknown)?).await?.status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE stats_v2_watermarks SET complete_through=$1")
        .bind(at - 31)
        .execute(&db)
        .await?;
    let response = app.oneshot(request(&path)?).await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(to_bytes(response.into_body(), 60_000).await?.to_vec())?;
    assert!(body.contains("Unavailable"));
    assert!(!body.contains("<dd>"));
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}
