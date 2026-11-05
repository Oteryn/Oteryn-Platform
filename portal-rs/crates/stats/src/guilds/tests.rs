use super::*;
use crate::store::{Permissions, Snapshot, partitions, snapshot};
use std::sync::Arc;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone)]
struct Fixture {
    guilds: Vec<Guild>,
    hidden: Uuid,
}
impl Visibility for Fixture {
    fn permissions(&self, id: Uuid) -> Permissions {
        Permissions {
            visible: id != self.hidden,
            guild: id != self.hidden,
            ..Permissions::default()
        }
    }
}
impl Source for Fixture {
    fn current(&self) -> std::result::Result<Vec<Guild>, Error> {
        Ok(self.guilds.clone())
    }
}
fn fixture() -> Fixture {
    let visible = Member {
        id: Uuid::now_v7(),
        rank: 2,
        name: "Visible <member>".into(),
        level: Some(10),
        experience: Some(100),
    };
    let hidden = Member {
        id: Uuid::now_v7(),
        rank: 1,
        name: "Private".into(),
        level: Some(1000),
        experience: Some(99999),
    };
    Fixture {
        hidden: hidden.id,
        guilds: vec![Guild {
            id: Uuid::now_v7(),
            world: Uuid::now_v7(),
            name: "Rose".into(),
            founded_at: 1,
            members: vec![hidden, visible],
        }],
    }
}
#[test]
fn aggregates_refuse_hidden_bad_and_missing_evidence() -> Result {
    let mut source = fixture();
    let config = Config { enabled: true };
    let summaries = list(config, &source).map_err(|e| format!("{e:?}"))?;
    assert_eq!(summaries[0].members, 1);
    assert_eq!(summaries[0].total_level, Some(10));
    assert_eq!(summaries[0].experience, Some(100));
    assert!(summaries[0].achievement_points.is_none());
    assert_eq!(
        list(Config::default(), &source).err(),
        Some(Error::Disabled)
    );
    source.guilds[0].members[1].level = None;
    assert!(
        list(config, &source).map_err(|e| format!("{e:?}"))?[0]
            .total_level
            .is_none()
    );
    source.guilds[0].members[1].rank = 0;
    assert_eq!(list(config, &source).err(), Some(Error::BadInput));
    Ok(())
}
#[tokio::test]
async fn http_success_refusal_bad_input_and_localization() -> Result {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let db = PgPool::connect_lazy("postgres://unused@localhost:1/unused")?;
    for lang in ["pl", "en"] {
        let source = fixture();
        let hidden = source.hidden.to_string();
        let app = http::router::<()>(db.clone(), Config { enabled: true }, Arc::new(source));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/{lang}/stats/guilds"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let body = to_bytes(response.into_body(), 100000).await?;
        let body = std::str::from_utf8(&body)?;
        assert!(!body.contains("<member>"));
        assert!(body.contains("Visible"));
        assert!(!body.contains("Private"));
        assert!(!body.contains(&hidden));
        assert!(body.contains(&format!("lang=\"{lang}\"")));
    }
    for (enabled, uri, status) in [
        (false, "/stats/guilds", StatusCode::NOT_FOUND),
        (true, "/stats/guilds?days=32", StatusCode::BAD_REQUEST),
        (true, "/stats/guilds?days=1&days=2", StatusCode::BAD_REQUEST),
        (true, "/stats/guilds?compare=Rose", StatusCode::BAD_REQUEST),
        (
            true,
            "/stats/guilds?guild=missing",
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let app = http::router::<()>(db.clone(), Config { enabled }, Arc::new(fixture()));
        assert_eq!(
            app.oneshot(Request::builder().uri(uri).body(Body::empty())?)
                .await?
                .status(),
            status
        );
    }
    Ok(())
}
#[tokio::test]
async fn postgres_guild_history_growth_privacy_and_missing_endpoints() -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping guild PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("guild_test_{}", Uuid::now_v7().simple());
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
    let source = fixture();
    let guild = &source.guilds[0];
    let end = time::OffsetDateTime::now_utc().date();
    let start = end.previous_day().ok_or("date")?;
    let mut tx = db.begin().await?;
    partitions(&mut tx, start).await?;
    for member in &guild.members {
        for (day, xp) in [(start, "100"), (end, "150")] {
            snapshot(
                &mut tx,
                &Snapshot {
                    character_ref: member.id,
                    day,
                    level: 10,
                    experience: Some(xp.into()),
                    skills: None,
                    achievement_points: None,
                    minutes_online: None,
                    name: member.name.clone(),
                    world: guild.world,
                    guild: Some(guild.id),
                    observed_at: 1,
                },
            )
            .await?;
        }
    }
    tx.commit().await?;
    let config = Config { enabled: true };
    let report = history(&db, config, &source, guild.id, start, end)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(report.observations.len(), 2);
    assert_eq!(report.experience_gain, Some(50));
    assert!(report.observations.iter().all(|o| o.member != "Private"));
    let missing = history(
        &db,
        config,
        &source,
        guild.id,
        start,
        end.next_day().ok_or("date")?,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    assert!(missing.experience_gain.is_none());
    assert_eq!(
        history(&db, config, &source, guild.id, end, start)
            .await
            .err(),
        Some(Error::BadInput)
    );
    assert_eq!(
        history(&db, Config::default(), &source, guild.id, start, end)
            .await
            .err(),
        Some(Error::Disabled)
    );
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut comparison = source.clone();
    comparison.guilds.push(Guild {
        id: Uuid::now_v7(),
        world: guild.world,
        name: "Second".into(),
        founded_at: 1,
        members: vec![],
    });
    let app = http::router::<()>(db.clone(), config, Arc::new(comparison));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/en/stats/guilds?guild=Rose&compare=Second&days=2")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 100000).await?;
    let body = std::str::from_utf8(&body)?;
    assert!(body.contains("Rose") && body.contains("Second"));
    assert!(body.contains("50"));
    assert!(!body.contains("Private"));
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}

#[test]
fn ranking_is_stable_and_duplicate_members_deny() -> Result {
    let mut source = fixture();
    let mut second = source.guilds[0].clone();
    second.id = Uuid::now_v7();
    second.name = "Second".into();
    second.members.clear();
    second.members.push(Member {
        id: Uuid::now_v7(),
        rank: 1,
        name: "Second member".into(),
        level: Some(20),
        experience: None,
    });
    source.guilds.push(second);
    let config = Config { enabled: true };
    let ranked = list(config, &source).map_err(|e| format!("{e:?}"))?;
    assert_eq!(ranked[0].guild.name, "Second");
    assert!(ranked[0].experience.is_none());
    source.guilds[1].members[0].level = Some(10);
    let tied = list(config, &source).map_err(|e| format!("{e:?}"))?;
    assert!(tied[0].guild.id < tied[1].guild.id);
    source.guilds[1].members[0].id = source.guilds[0].members[1].id;
    assert_eq!(list(config, &source).err(), Some(Error::BadInput));
    Ok(())
}
