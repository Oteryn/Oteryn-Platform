#![allow(clippy::unwrap_used)]
use super::*;
use crate::store::{self, Snapshot};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone)]
struct Catalog(Vec<Character>);
impl PublicCatalog for Catalog {
    fn characters(&self, _: Kind) -> CatalogFuture<'_> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}
async fn database() -> Result<Option<PgPool>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping rankings PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("rankings_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105210000_t65_statistics.sql"
    ))
    .execute(&db)
    .await?;
    Ok(Some(db))
}
async fn dispose(db: PgPool) -> Result {
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    db.close().await;
    Ok(())
}
fn character() -> Character {
    Character {
        id: Uuid::now_v7(),
        name: "<Public>".into(),
        world: Uuid::now_v7(),
        world_name: "Public world".into(),
        vocation: 2,
        pvp: Pvp::Optional,
        created: Some(Date::from_calendar_date(2020, time::Month::November, 5).unwrap()),
    }
}
fn state(db: PgPool, chars: Vec<Character>) -> Rankings {
    Rankings {
        db,
        enabled: true,
        catalog: Arc::new(Catalog(chars)),
    }
}
fn today() -> Date {
    Date::from_calendar_date(2026, time::Month::November, 5).unwrap()
}
async fn seed(db: &PgPool, c: &Character, day: Date, exp: Option<&str>) -> Result {
    let mut tx = db.begin().await?;
    store::partitions(&mut tx, day).await?;
    store::snapshot(
        &mut tx,
        &Snapshot {
            character_ref: c.id,
            day,
            level: 10,
            experience: exp.map(str::to_string),
            skills: None,
            achievement_points: None,
            minutes_online: None,
            name: c.name.clone(),
            world: c.world,
            guild: None,
            observed_at: 1,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
fn filters() -> Filters {
    Filters {
        page: 1,
        ..Default::default()
    }
}
#[tokio::test]
async fn postgres_experience_windows_records_privacy_and_filters() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let a = character();
    let hidden = character();
    let end = today().previous_day().unwrap();
    for c in [&a, &hidden] {
        for (days, value) in [(30, "10"), (7, "20"), (1, "30"), (0, "100")] {
            seed(&db, c, end - time::Duration::days(days), Some(value)).await?;
        }
    }
    let s = state(db.clone(), vec![a.clone()]);
    for (kind, gain) in [
        (Kind::Yesterday, "70"),
        (Kind::Week, "80"),
        (Kind::Month, "90"),
        (Kind::Record, "70"),
    ] {
        let r = s.load(kind, today(), &filters()).await.unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.rows[0].value, gain);
        assert_eq!(r.rows[0].character.id, a.id);
    }
    let f = Filters {
        world: Some(a.world),
        vocation: Some(2),
        pvp: Some(Pvp::Optional),
        ..filters()
    };
    assert_eq!(s.load(Kind::Week, today(), &f).await.unwrap().total, 1);
    assert_eq!(
        s.load(
            Kind::Week,
            today(),
            &Filters {
                pvp: Some(Pvp::Open),
                ..filters()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert_eq!(
        s.load(
            Kind::Week,
            today(),
            &Filters {
                page: 0,
                ..filters()
            }
        )
        .await
        .unwrap_err(),
        Error::Invalid
    );
    dispose(db).await
}
#[tokio::test]
async fn postgres_events_and_pagination_after_privacy() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let a = character();
    let hidden = character();
    for c in [&a, &hidden] {
        for kind in ["name_change", "world_change", "bazaar_sale"] {
            for i in 0..51 {
                sqlx::query("INSERT INTO stats_character_events VALUES ($1,$2,$3,$4,'{}')")
                    .bind(c.id)
                    .bind(today())
                    .bind(kind)
                    .bind(i.to_string())
                    .execute(&db)
                    .await?;
            }
        }
    }
    let s = state(db.clone(), vec![a.clone()]);
    for kind in [Kind::NameChanges, Kind::Transfers, Kind::Sold] {
        let first = s.load(kind, today(), &filters()).await.unwrap();
        assert_eq!(first.total, 51);
        assert_eq!(first.rows.len(), 50);
        let last = s
            .load(
                kind,
                today(),
                &Filters {
                    page: 2,
                    ..filters()
                },
            )
            .await
            .unwrap();
        assert_eq!(last.rows.len(), 1);
        assert_eq!(last.rows[0].character.id, a.id);
    }
    dispose(db).await
}
#[tokio::test]
async fn postgres_missing_experience_and_gaps_are_unavailable() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let a = character();
    let s = state(db.clone(), vec![a.clone()]);
    seed(&db, &a, today() - time::Duration::days(3), Some("10")).await?;
    seed(&db, &a, today() - time::Duration::days(1), Some("100")).await?;
    for kind in [Kind::Yesterday, Kind::Record] {
        assert_eq!(
            s.load(kind, today(), &filters()).await.unwrap_err(),
            Error::Unavailable
        );
    }
    dispose(db).await
}
#[tokio::test]
async fn oldest_anniversaries_and_unknown_creation() -> Result {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    let mut a = character();
    let s = state(db.clone(), vec![a.clone()]);
    assert_eq!(
        s.load(Kind::Oldest, today(), &filters())
            .await
            .unwrap()
            .rows[0]
            .value,
        "6"
    );
    assert_eq!(
        s.load(Kind::Anniversaries, today(), &filters())
            .await
            .unwrap()
            .total,
        1
    );
    a.created = None;
    assert_eq!(
        state(db.clone(), vec![a])
            .load(Kind::Oldest, today(), &filters())
            .await
            .unwrap_err(),
        Error::Unavailable
    );
    let s = Rankings {
        db,
        enabled: false,
        catalog: Arc::new(Catalog(vec![])),
    };
    assert_eq!(
        s.load(Kind::Oldest, today(), &filters()).await.unwrap_err(),
        Error::Disabled
    );
    Ok(())
}
#[tokio::test]
async fn http_success_refusal_bad_query_localization_and_escaping() -> Result {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    for (enabled, url, status, needle) in [
        (true, "/en/stats/rankings/oldest", 200, "&#60;Public&#62;"),
        (false, "/en/stats/rankings/oldest", 404, "Ranking not found"),
        (
            true,
            "/pl/stats/rankings/oldest?page=0",
            400,
            "Nieprawidłowe",
        ),
        (true, "/en/stats/rankings/online", 404, "Ranking not found"),
    ] {
        let mut s = state(db.clone(), vec![character()]);
        s.enabled = enabled;
        let r = router(s)
            .oneshot(Request::builder().uri(url).body(Body::empty())?)
            .await?;
        assert_eq!(r.status().as_u16(), status);
        assert_eq!(r.headers()["cache-control"], "no-store, private");
        let body = String::from_utf8(to_bytes(r.into_body(), 100_000).await?.to_vec())?;
        assert!(body.contains(needle), "{body}");
    }
    Ok(())
}

#[tokio::test]
async fn unavailable_catalog_never_serves_prior_allow_or_empty_success() -> Result {
    struct Denied;
    impl PublicCatalog for Denied {
        fn characters(&self, _: Kind) -> CatalogFuture<'_> {
            Box::pin(async { Err(Error::Unavailable) })
        }
    }
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    let response = router(Rankings {
        db,
        enabled: true,
        catalog: Arc::new(Denied),
    })
    .oneshot(
        Request::builder()
            .uri("/en/stats/rankings/oldest")
            .body(Body::empty())?,
    )
    .await?;
    assert_eq!(response.status().as_u16(), 503);
    let body = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    assert!(body.contains("Statistics are unavailable"));
    assert!(!body.contains("No matching public records"));
    Ok(())
}
