#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use oteryn_stats::{
    online::{self, Config, Coverage, CoveragePolicy, Error, Family, Receipt},
    rankings::online::{
        OnlineWindow, OwnedCharacter, OwnerFuture, OwnerPolicy, owner_online_ranking,
        public_online_ranking,
    },
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const OPEN: &str = include_str!("fixtures/open.json");
const CLOSED: &str = include_str!("fixtures/closed.json");
const WORLD: &str = include_str!("fixtures/world.json");
const NOW: i64 = 1790035203;
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334401";
const WORLD_ID: &str = "01934f10-7c02-7001-805b-3b1122334401";
fn config() -> Config {
    Config {
        enabled: true,
        intervals_enabled: true,
        world_totals_enabled: true,
        owner_rankings_enabled: true,
        source_authority: "oteryn:game:primary".into(),
    }
}
fn bytes(s: &str) -> &[u8] {
    s.trim_end().as_bytes()
}
async fn database() -> Result<Option<PgPool>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping online PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("online_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    for migration in [
        include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
        include_str!("../../../migrations/20261105210000_t65_statistics.sql"),
        include_str!("../../../migrations/20261105231301_t67b_statistics_v2.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&db).await?;
    }
    Ok(Some(db))
}
async fn dispose(db: PgPool) -> TestResult {
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    db.close().await;
    Ok(())
}
fn app(db: PgPool, c: Config) -> Router {
    online::router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san("projection-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        c,
    )
}
async fn send(app: Router, path: &str, body: &str, peer: Option<&str>) -> axum::response::Response {
    let mut req = Request::post(format!("/internal/v2/game-public/{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    if let Some(peer) = peer {
        req.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(peer).unwrap(),
            ));
    }
    app.oneshot(req).await.unwrap()
}
async fn apply(db: &PgPool, family: Option<Family>, s: &str, now: i64) -> Result<Receipt, Error> {
    online::ingest(db, &config(), family, bytes(s), now).await
}
fn watermark(family: Family, epoch: &str, through: i64, observed: i64) -> String {
    format!(
        "{{\"contract_version\":2,\"operation\":\"PublishPublicProjectionWatermarkV2\",\"source_authority\":\"oteryn:game:primary\",\"family\":\"{}\",\"projection_epoch\":\"{epoch}\",\"complete_through\":\"{through}\",\"observed_at\":\"{observed}\"}}",
        family.name()
    )
}
struct Baseline {
    epoch: String,
    start: i64,
    through: i64,
}
impl CoveragePolicy for Baseline {
    fn coverage(&self, _: Family) -> Result<Coverage, Error> {
        Ok(Coverage {
            epoch: self.epoch.clone(),
            start: self.start,
            through: self.through,
        })
    }
}
fn baseline() -> Baseline {
    Baseline {
        epoch: "1".into(),
        start: 1789000000,
        through: NOW,
    }
}
struct Owner {
    account: Uuid,
    characters: Vec<OwnedCharacter>,
}
impl OwnerPolicy for Owner {
    fn characters(&self, account: Uuid, _: i64) -> OwnerFuture<'_> {
        Box::pin(async move {
            if account != self.account {
                Err(Error::Unauthorized)
            } else {
                Ok(self.characters.clone())
            }
        })
    }
}
fn owner() -> Owner {
    Owner {
        account: Uuid::now_v7(),
        characters: vec![OwnedCharacter {
            character: Uuid::parse_str(CHARACTER).unwrap(),
            owned_since: 1789000000,
        }],
    }
}
fn window() -> OnlineWindow {
    OnlineWindow {
        start: 1790000000,
        end: 1790007200,
        now: NOW,
    }
}
#[tokio::test]
async fn transport_flags_peer_bounds_and_bad_input() -> TestResult {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    for (c, peer, status) in [
        (Config::default(), Some("projection-test"), 503),
        (config(), None, 401),
        (config(), Some("wrong-purpose"), 401),
    ] {
        let r = send(
            app(db.clone(), c),
            "world-daily-kills",
            WORLD.trim_end(),
            peer,
        )
        .await;
        assert_eq!(r.status().as_u16(), status);
        assert_eq!(r.headers()["cache-control"], "no-store, private");
        assert_eq!(r.headers()["pragma"], "no-cache");
        assert!(to_bytes(r.into_body(), 256).await?.is_empty());
    }
    for (path, body) in [
        ("world-daily-kills", "x".repeat(2049)),
        ("character-online-session", "x".repeat(2049)),
        ("watermark", "x".repeat(513)),
        (
            "watermark",
            watermark(Family::WorldDailyKills, "1", NOW, NOW)
                .replace("world_daily_kills", "unknown"),
        ),
        (
            "character-online-session",
            include_str!("fixtures/null_end.json").trim_end().to_owned(),
        ),
        (
            "world-daily-kills",
            include_str!("fixtures/numeric_count.json")
                .trim_end()
                .to_owned(),
        ),
        (
            "world-daily-kills",
            WORLD
                .trim_end()
                .replace("\"contract_version\":2", "\"contract_version\":3"),
        ),
        (
            "world-daily-kills",
            WORLD.trim_end().replace(
                "\"contract_version\":2",
                "\"contract_version\":2,\"contract_version\":2",
            ),
        ),
        ("world-daily-kills", format!("{}\n", WORLD.trim_end())),
    ] {
        let r = send(
            app(db.clone(), config()),
            path,
            &body,
            Some("projection-test"),
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "{body}");
        assert!(to_bytes(r.into_body(), 256).await?.is_empty());
    }
    assert_eq!(public_online_ranking(), Err(Error::Unavailable));
    Ok(())
}
#[tokio::test]
async fn bad_semantics_without_database_access() -> TestResult {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    for body in [
        CLOSED.replace("1790003600\"]", "1789999999\"]"),
        CLOSED.replace("1790003600\"]", "1790003601\"]"),
        CLOSED.replace(
            "\"projection_revision\":\"2\"",
            "\"projection_revision\":\"02\"",
        ),
        CLOSED.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"0\""),
        CLOSED.replace("7001-805b", "6001-805b"),
        CLOSED.replace("\"sessions\":", "\"extra\":true,\"sessions\":"),
        CLOSED.replace("\"started_at\":\"1790000000\"", "\"started_at\":\"0\""),
        CLOSED.replace("\"contract_version\":2", "\"contract_version\":1"),
    ] {
        assert_eq!(
            apply(&db, Some(Family::CharacterOnlineSessions), &body, NOW).await,
            Err(Error::Invalid)
        );
    }
    for body in [
        WORLD.replace("1789948800", "1789948801"),
        WORLD.replace("1790035200", "1790035201"),
        WORLD.replace("12345", "18446744073709551616"),
        WORLD.replace("12345", "01"),
        WORLD.replace("1790035201", "1790000000"),
    ] {
        assert_eq!(
            apply(&db, Some(Family::WorldDailyKills), &body, NOW).await,
            Err(Error::Invalid)
        );
    }
    assert_eq!(
        apply(
            &db,
            None,
            &watermark(Family::WorldDailyKills, "1", NOW + 2, NOW + 2),
            NOW
        )
        .await,
        Err(Error::Invalid)
    );
    Ok(())
}
#[tokio::test]
async fn postgres_stable_ids_retries_conflicts_and_replacements() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    for s in [OPEN, CLOSED, CLOSED] {
        assert_eq!(
            apply(&db, Some(Family::CharacterOnlineSessions), s, NOW).await,
            Ok(Receipt::Accepted)
        );
    }
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), OPEN, NOW).await,
        Ok(Receipt::Superseded)
    );
    assert_eq!(
        apply(
            &db,
            Some(Family::CharacterOnlineSessions),
            &CLOSED.replace("1790000000", "1790000001"),
            NOW
        )
        .await,
        Err(Error::Conflict)
    );
    // A changed observation does not change content, nor renew the first observation.
    assert_eq!(
        apply(
            &db,
            Some(Family::CharacterOnlineSessions),
            &CLOSED.replace(
                "\"source_observed_at\":\"1790003600\"",
                "\"source_observed_at\":\"1790003700\""
            ),
            NOW
        )
        .await,
        Ok(Receipt::Accepted)
    );
    let observed: i64 = sqlx::query_scalar("SELECT observed_at FROM stats_v2_subjects")
        .fetch_one(&db)
        .await?;
    assert_eq!(observed, 1790003600);
    assert_eq!(
        apply(&db, Some(Family::WorldDailyKills), WORLD, NOW).await,
        Ok(Receipt::Accepted)
    );
    assert_eq!(
        apply(
            &db,
            Some(Family::WorldDailyKills),
            include_str!("fixtures/conflict.json"),
            NOW
        )
        .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        apply(
            &db,
            Some(Family::WorldDailyKills),
            &WORLD
                .replace(
                    "\"projection_revision\":\"2\"",
                    "\"projection_revision\":\"3\""
                )
                .replace("12345", "12346"),
            NOW
        )
        .await,
        Ok(Receipt::Accepted)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_v2_subjects")
        .fetch_one(&db)
        .await?;
    assert_eq!(count, 2);
    let leaked: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_character_daily")
        .fetch_one(&db)
        .await?;
    assert_eq!(leaked, 0);
    dispose(db).await
}
#[tokio::test]
async fn postgres_owner_ordering_coverage_and_privacy_denials() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut o = owner();
    let other = Uuid::now_v7();
    let foreign = Uuid::now_v7();
    o.characters.push(OwnedCharacter {
        character: other,
        owned_since: 1789000000,
    });
    for s in [
        CLOSED.to_owned(),
        CLOSED
            .replace(CHARACTER, &other.to_string())
            .replace("1790003600", "1790001800"),
        CLOSED.replace(CHARACTER, &foreign.to_string()),
    ] {
        assert_eq!(
            apply(&db, Some(Family::CharacterOnlineSessions), &s, NOW).await,
            Ok(Receipt::Accepted)
        );
    }
    let wm = watermark(Family::CharacterOnlineSessions, "1", NOW, NOW);
    assert_eq!(apply(&db, None, &wm, NOW).await, Ok(Receipt::Accepted));
    let result = owner_online_ranking(&db, &config(), o.account, window(), &o, &baseline())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].seconds, 3600);
    assert_eq!(result[1].seconds, 1800);
    assert!(result.iter().all(|r| r.character != foreign));
    let clipped = OnlineWindow {
        start: 1790000900,
        end: 1790002700,
        ..window()
    };
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, clipped, &o, &baseline())
            .await
            .unwrap()[0]
            .seconds,
        1800
    );
    assert_eq!(
        owner_online_ranking(&db, &config(), Uuid::now_v7(), window(), &o, &baseline()).await,
        Err(Error::Unauthorized)
    );
    o.characters[0].owned_since = 1790000001;
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &baseline()).await,
        Err(Error::Unauthorized)
    );
    o.characters[0].owned_since = 1789000000;
    assert_eq!(
        owner_online_ranking(
            &db,
            &Config {
                owner_rankings_enabled: false,
                ..config()
            },
            o.account,
            window(),
            &o,
            &baseline()
        )
        .await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        owner_online_ranking(
            &db,
            &config(),
            o.account,
            OnlineWindow {
                end: window().start,
                ..window()
            },
            &o,
            &baseline()
        )
        .await,
        Err(Error::Invalid)
    );
    let missing = Baseline {
        start: window().start + 1,
        ..baseline()
    };
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &missing).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        owner_online_ranking(
            &db,
            &config(),
            o.account,
            OnlineWindow {
                now: NOW + 30,
                ..window()
            },
            &o,
            &baseline()
        )
        .await,
        Err(Error::Unavailable)
    );
    dispose(db).await
}
#[tokio::test]
async fn postgres_open_overlap_zero_and_suppression() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), OPEN, NOW).await,
        Ok(Receipt::Accepted)
    );
    let o = owner();
    apply(
        &db,
        None,
        &watermark(Family::CharacterOnlineSessions, "1", NOW, NOW),
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &baseline()).await,
        Err(Error::Unavailable)
    );
    let overlap = CLOSED.replace(
        "01934f10-7c08-7001-805b-3b1122334401",
        &Uuid::now_v7().to_string(),
    );
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), &overlap, NOW).await,
        Err(Error::Invalid)
    );
    apply(&db, Some(Family::CharacterOnlineSessions), CLOSED, NOW)
        .await
        .unwrap();
    let reopen = OPEN.replace(
        "\"projection_revision\":\"1\"",
        "\"projection_revision\":\"3\"",
    );
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), &reopen, NOW).await,
        Err(Error::Invalid)
    );
    let zero = overlap.replace("1790003600\"]", "1790000000\"]");
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), &zero, NOW).await,
        Ok(Receipt::Accepted)
    );
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &baseline())
            .await
            .unwrap()[0]
            .seconds,
        3600
    );
    apply(
        &db,
        Some(Family::CharacterOnlineSessions),
        include_str!("fixtures/suppressed.json"),
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &baseline()).await,
        Err(Error::Unavailable)
    );
    let reopen = OPEN.replace(
        "\"projection_revision\":\"1\"",
        "\"projection_revision\":\"4\"",
    );
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), &reopen, NOW).await,
        Err(Error::Invalid)
    );
    dispose(db).await
}
#[tokio::test]
async fn postgres_public_world_totals_complete_provisional_zero_and_missing() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let world = Uuid::parse_str(WORLD_ID)?;
    apply(&db, Some(Family::WorldDailyKills), WORLD, NOW)
        .await
        .unwrap();
    assert_eq!(
        online::world_total(&db, &config(), world, 1789948800, NOW, &baseline()).await,
        Err(Error::Unavailable)
    );
    apply(
        &db,
        None,
        &watermark(Family::WorldDailyKills, "1", NOW, NOW),
        NOW,
    )
    .await
    .unwrap();
    let total = online::world_total(&db, &config(), world, 1789948800, NOW, &baseline())
        .await
        .unwrap();
    assert_eq!(total.monsters_killed, 12345);
    assert_eq!(total.bosses_killed, 12);
    assert!(total.complete);
    let zero = WORLD
        .replace(
            "\"projection_revision\":\"2\"",
            "\"projection_revision\":\"3\"",
        )
        .replace("12345", "0")
        .replace("\"12\"", "\"0\"")
        .replace("true", "false");
    apply(&db, Some(Family::WorldDailyKills), &zero, NOW)
        .await
        .unwrap();
    let total = online::world_total(&db, &config(), world, 1789948800, NOW, &baseline())
        .await
        .unwrap();
    assert_eq!(total.monsters_killed, 0);
    assert!(!total.complete);
    assert_eq!(
        online::world_total(&db, &config(), Uuid::now_v7(), 1789948800, NOW, &baseline()).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        online::world_total(&db, &config(), world, 1789948801, NOW, &baseline()).await,
        Err(Error::Invalid)
    );
    let payload = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM stats_v2_subjects WHERE family='world_daily_kills'",
    )
    .fetch_one(&db)
    .await?;
    assert!(payload.get("character_id").is_none());
    assert!(payload.get("sessions").is_none());
    dispose(db).await
}
#[tokio::test]
async fn postgres_retention_purge_and_replay_cannot_resurrect() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    apply(&db, Some(Family::CharacterOnlineSessions), CLOSED, NOW)
        .await
        .unwrap();
    let expired = 1790003600 + 30 * 86400;
    assert_eq!(online::purge(&db, expired - 1).await, Ok(0));
    assert_eq!(online::purge(&db, expired).await, Ok(1));
    let retained_times: (i64, i64) =
        sqlx::query_as("SELECT observed_at, expires_at FROM stats_v2_subjects")
            .fetch_one(&db)
            .await?;
    assert_eq!(retained_times, (1, 1));
    assert_eq!(online::purge(&db, expired).await, Ok(0));
    assert_eq!(
        apply(&db, Some(Family::CharacterOnlineSessions), CLOSED, expired).await,
        Ok(Receipt::Accepted)
    );
    let corrected = CLOSED
        .replace(
            "\"projection_revision\":\"2\"",
            "\"projection_revision\":\"3\"",
        )
        .replace("1790003600", &(expired - 1).to_string());
    assert_eq!(
        apply(
            &db,
            Some(Family::CharacterOnlineSessions),
            &corrected,
            expired
        )
        .await,
        Ok(Receipt::Accepted)
    );
    let resync = corrected.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\"");
    apply(&db, Some(Family::CharacterOnlineSessions), &resync, expired)
        .await
        .unwrap();
    let payload: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT payload FROM stats_v2_subjects")
            .fetch_one(&db)
            .await?;
    assert!(payload.is_none());
    apply(
        &db,
        Some(Family::WorldDailyKills),
        &WORLD.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\""),
        expired,
    )
    .await
    .unwrap();
    let world_expiry = 1790035200 + 400 * 86400;
    apply(
        &db,
        None,
        &watermark(Family::WorldDailyKills, "2", world_expiry, world_expiry),
        world_expiry,
    )
    .await
    .unwrap();
    let cut = Baseline {
        epoch: "2".into(),
        through: world_expiry,
        ..baseline()
    };
    assert_eq!(
        online::world_total(
            &db,
            &config(),
            Uuid::parse_str(WORLD_ID)?,
            1789948800,
            world_expiry,
            &cut
        )
        .await,
        Err(Error::Unavailable)
    );
    assert_eq!(online::purge(&db, world_expiry).await, Ok(2));
    dispose(db).await
}
#[tokio::test]
async fn postgres_expired_open_interval_is_never_auto_closed() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    apply(&db, Some(Family::CharacterOnlineSessions), OPEN, NOW)
        .await
        .unwrap();
    let expired = 1790000000 + 30 * 86400;
    // Serve-time expiry denies even before cleanup runs.
    apply(
        &db,
        None,
        &watermark(Family::CharacterOnlineSessions, "1", expired, expired),
        expired,
    )
    .await
    .unwrap();
    let o = owner();
    let w = OnlineWindow {
        start: expired - 3600,
        end: expired,
        now: expired,
    };
    let cut = Baseline {
        through: expired,
        ..baseline()
    };
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, w, &o, &cut).await,
        Err(Error::Unavailable)
    );
    assert_eq!(online::purge(&db, expired).await, Ok(1));
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, w, &o, &cut).await,
        Err(Error::Unavailable)
    );
    let payload: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT payload FROM stats_v2_subjects")
            .fetch_one(&db)
            .await?;
    assert!(payload.is_none());
    dispose(db).await
}
#[tokio::test]
async fn postgres_shared_epoch_fence_and_watermark_monotonicity() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    apply(&db, Some(Family::WorldDailyKills), WORLD, NOW)
        .await
        .unwrap();
    apply(
        &db,
        None,
        &watermark(Family::WorldDailyKills, "1", NOW, NOW),
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(
        apply(
            &db,
            None,
            &watermark(Family::WorldDailyKills, "1", NOW - 1, NOW),
            NOW
        )
        .await,
        Ok(Receipt::Superseded)
    );
    // A new interval-family epoch immediately invalidates world totals and v1 readers.
    apply(
        &db,
        Some(Family::CharacterOnlineSessions),
        &CLOSED.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\""),
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(
        online::world_total(
            &db,
            &config(),
            Uuid::parse_str(WORLD_ID)?,
            1789948800,
            NOW,
            &baseline()
        )
        .await,
        Err(Error::Unavailable)
    );
    let highest: String =
        sqlx::query_scalar("SELECT highest_epoch::text FROM public_projection_state")
            .fetch_one(&db)
            .await?;
    assert_eq!(highest, "2");
    assert_eq!(
        apply(&db, Some(Family::WorldDailyKills), WORLD, NOW).await,
        Ok(Receipt::Superseded)
    );
    // The same persisted public feed floor also fences v2 when another consumer raises it.
    sqlx::query("UPDATE public_projection_state SET highest_epoch=3 WHERE id=1")
        .execute(&db)
        .await?;
    assert_eq!(
        apply(
            &db,
            Some(Family::CharacterOnlineSessions),
            &CLOSED.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\""),
            NOW
        )
        .await,
        Ok(Receipt::Superseded)
    );
    dispose(db).await
}
#[tokio::test]
async fn postgres_http_acknowledgements_authority_and_family_switches() -> TestResult {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let app = app(db.clone(), config());
    for _ in 0..2 {
        let r = send(
            app.clone(),
            "world-daily-kills",
            WORLD.trim_end(),
            Some("projection-test"),
        )
        .await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(r.into_body(), 256).await?.as_ref(),
            Receipt::Accepted.body().as_bytes()
        );
    }
    let r = send(
        app.clone(),
        "world-daily-kills",
        include_str!("fixtures/conflict.json").trim_end(),
        Some("projection-test"),
    )
    .await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert!(to_bytes(r.into_body(), 256).await?.is_empty());
    let wrong = WORLD
        .trim_end()
        .replace("oteryn:game:primary", "oteryn:game:other");
    let r = send(
        app.clone(),
        "world-daily-kills",
        &wrong,
        Some("projection-test"),
    )
    .await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let off = Config {
        world_totals_enabled: false,
        ..config()
    };
    let r = send(
        online::router(
            db.clone(),
            Arc::new(IdentityMap::new(vec![Binding {
                identity: Identity::san("projection-test")?,
                group: "game-public".into(),
            }])?),
            off,
        ),
        "world-daily-kills",
        WORLD.trim_end(),
        Some("projection-test"),
    )
    .await;
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    dispose(db).await
}

#[tokio::test]
async fn rate_limit_and_invalid_content_length_fail_closed() -> TestResult {
    let db = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?;
    let app = app(db, config());
    for _ in 0..1200 {
        assert_eq!(
            send(app.clone(), "watermark", "{}", Some("projection-test"))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let r = send(app.clone(), "watermark", "{}", Some("projection-test")).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.headers()["retry-after"], "60");
    assert!(to_bytes(r.into_body(), 256).await?.is_empty());
    for length in ["02049", "2049", "bad"] {
        let req = Request::post("/internal/v2/game-public/character-online-session")
            .header("content-type", "application/json")
            .header("content-length", length)
            .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                "projection-test",
            )?))
            .body(Body::from("{}"))?;
        assert_eq!(
            app.clone().oneshot(req).await?.status(),
            StatusCode::BAD_REQUEST
        );
    }
    Ok(())
}

#[tokio::test]
async fn postgres_policy_outage_and_authority_change_deny_existing_data() -> TestResult {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Changing {
        inner: Owner,
        calls: AtomicUsize,
    }
    impl OwnerPolicy for Changing {
        fn characters(&self, account: Uuid, now: i64) -> OwnerFuture<'_> {
            Box::pin(async move {
                if self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
                    Err(Error::Unauthorized)
                } else {
                    self.inner.characters(account, now).await
                }
            })
        }
    }
    struct Missing;
    impl CoveragePolicy for Missing {
        fn coverage(&self, _: Family) -> Result<Coverage, Error> {
            Err(Error::Unavailable)
        }
    }
    let Some(db) = database().await? else {
        return Ok(());
    };
    apply(&db, Some(Family::CharacterOnlineSessions), CLOSED, NOW)
        .await
        .unwrap();
    apply(
        &db,
        None,
        &watermark(Family::CharacterOnlineSessions, "1", NOW, NOW),
        NOW,
    )
    .await
    .unwrap();
    let o = owner();
    assert_eq!(
        owner_online_ranking(&db, &config(), o.account, window(), &o, &Missing).await,
        Err(Error::Unavailable)
    );
    let changing = Changing {
        inner: o,
        calls: AtomicUsize::new(0),
    };
    assert_eq!(
        owner_online_ranking(
            &db,
            &config(),
            changing.inner.account,
            window(),
            &changing,
            &baseline()
        )
        .await,
        Err(Error::Unauthorized)
    );
    dispose(db).await
}
