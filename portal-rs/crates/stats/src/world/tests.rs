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
    let date = Date::from_calendar_date(2026, time::Month::November, 1)?;
    let end = date.next_day().ok_or("date")?;
    let world = Uuid::now_v7();
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
        enabled
            .oneshot(request("/en/stats/world?world=bad")?)
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
    let db = PgPool::connect_lazy("postgres://unused@localhost:1/unused")?;
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
