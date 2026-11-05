#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use time::Month;
struct Source {
    hidden: Option<Uuid>,
}
impl Current for Source {
    fn character(&self, id: Uuid) -> CurrentFuture<'_> {
        Box::pin(async move {
            Ok(PublicCharacter {
                name: "<Public>".into(),
                permissions: Permissions {
                    visible: self.hidden != Some(id),
                    skills: true,
                    guild: false,
                    deaths: true,
                    online: true,
                },
                ranks: None,
            })
        })
    }
    fn experience_at_level(&self, _: Uuid, _: u32) -> Option<u64> {
        Some(1000)
    }
}
fn day() -> Date {
    Date::from_calendar_date(2026, Month::November, 1).unwrap()
}
fn daily(day: Date, experience: Option<&str>) -> Daily {
    Daily {
        day,
        level: 10,
        experience: experience.map(str::to_owned),
        skills: None,
        achievement_points: None,
        minutes_online: None,
    }
}
#[test]
fn exact_gains_prediction_and_missing_input() {
    let end = day();
    let rows: Vec<_> = (0..8)
        .map(|i| {
            daily(
                end - Duration::days(7 - i),
                Some(&(100 + i * 100).to_string()),
            )
        })
        .collect();
    assert_eq!(gain(&rows, end, 7), Some(700));
    assert_eq!(
        predict(&rows, end, Some(1000)),
        Some(end + Duration::days(2))
    );
    assert_eq!(predict(&rows[1..], end, Some(1000)), None);
    assert_eq!(predict(&rows, end, None), None);
    assert_eq!(predict(&rows, end, Some(700)), None);
    assert_eq!(
        gain(
            &[
                daily(end.previous_day().unwrap(), Some("100")),
                daily(end, Some("90"))
            ],
            end,
            1
        ),
        Some(-10)
    );
    assert_eq!(xp(&daily(end, Some("01"))), None);
}
#[tokio::test]
async fn disabled_hidden_and_bad_input_refuse_before_database() {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused").unwrap();
    let id = Uuid::now_v7();
    let mut service = Characters {
        db,
        enabled: false,
        current: Arc::new(Source { hidden: Some(id) }),
    };
    assert!(matches!(
        service.load(id, day(), day(), None).await,
        Err(Error::Disabled)
    ));
    service.enabled = true;
    assert!(matches!(
        service.load(id, day(), day(), None).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service.load(Uuid::nil(), day(), day(), None).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        service.compare(&[id, id], day(), day()).await,
        Err(Error::Invalid)
    ));
}
#[tokio::test]
async fn postgres_character_history_privacy_and_comparison()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping character PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("character_stats_{}", Uuid::now_v7().simple());
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
    let a = Uuid::now_v7();
    let b = Uuid::now_v7();
    let mut tx = db.begin().await?;
    store::partitions(&mut tx, day()).await?;
    for id in [a, b] {
        for i in 0..=2 {
            store::snapshot(
                &mut tx,
                &store::Snapshot {
                    character_ref: id,
                    day: day() + Duration::days(i),
                    level: 10 + i,
                    experience: Some((100 + i * 100).to_string()),
                    skills: Some(json!({"sword_level":20})),
                    achievement_points: Some(999),
                    minutes_online: Some(100),
                    name: "Observed".into(),
                    world: Uuid::now_v7(),
                    guild: None,
                    observed_at: i,
                },
            )
            .await?;
        }
        store::source_event(
            &mut tx,
            id,
            day(),
            "death",
            "one",
            &json!({"level":"10","occurred_at":time::OffsetDateTime::now_utc().unix_timestamp().to_string(),"death_occurrence_id":Uuid::now_v7().to_string(),"private":"never render"}),
        )
        .await?;
    }
    tx.commit().await?;
    let mut service = Characters {
        db: db.clone(),
        enabled: true,
        current: Arc::new(Source { hidden: None }),
    };
    let report = service
        .load(a, day(), day() + Duration::days(2), None)
        .await
        .unwrap();
    assert_eq!(report.gains[0], Some(100));
    assert_eq!(report.history.daily.len(), 3);
    assert_eq!(
        report
            .history
            .events
            .iter()
            .filter(|e| e.kind == "death")
            .count(),
        1
    );
    assert!(
        report
            .history
            .daily
            .iter()
            .all(|d| d.achievement_points.is_none() && d.minutes_online.is_none())
    );
    assert!(
        report
            .history
            .events
            .iter()
            .all(|e| !e.kind.starts_with("guild"))
    );
    assert_eq!(
        service
            .compare(&[b, a], day(), day() + Duration::days(2))
            .await
            .unwrap()
            .len(),
        2
    );
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let app = http::router(service.clone());
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/{lang}/stats/characters/{a}/experience?start=2026-11-01&end=2026-11-03"
                    ))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store, private");
        let body = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
        assert!(!body.contains("<Public>"));
        assert!(body.contains("&#60;Public&#62;") || body.contains("&lt;Public&gt;"));
        assert!(!body.contains("never render"));
        assert!(body.contains("viewBox"));
    }
    let response=app.clone().oneshot(Request::builder().uri(format!("/en/stats/compare?ids={a},{b}&start=2026-11-01&end=2026-11-03&metric=sword_level")).body(Body::empty())?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/en/stats/characters/{a}/history?start=%"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let deaths = http::router(service.clone())
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/en/stats/characters/{a}/deaths?start=2026-11-01&end=2026-11-03"
                ))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(deaths.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(deaths.into_body(), 1_000_000).await?.to_vec())?;
    assert!(body.contains("Death"));
    assert!(!body.contains("never render"));
    service.current = Arc::new(Source { hidden: Some(b) });
    assert!(matches!(
        service.compare(&[a, b], day(), day()).await,
        Err(Error::NotFound)
    ));
    let response = http::router(service)
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/en/stats/compare?ids={a},{b}&start=2026-11-01&end=2026-11-03"
                ))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    assert!(!body.contains("&lt;Public&gt;"));
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    db.close().await;
    Ok(())
}
