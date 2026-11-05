use oteryn_stats::world::kills::{Config, Error, Receipt, ingest, presentation, purge};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "oteryn:game:primary";
const GOLDEN: &str = include_str!("fixtures/world_kills.json");
const NOW: u64 = 1790035201;
fn config() -> Config {
    Config {
        public_projections_enabled: true,
        ingest_enabled: true,
        presentation_enabled: false,
    }
}
#[tokio::test]
async fn world_kills_refuses_disabled_and_invalid_wire() -> Result {
    let db = PgPool::connect_lazy("postgres://unused@localhost:1/unused")?;
    assert_eq!(
        ingest(&db, Config::default(), SOURCE, GOLDEN.as_bytes(), NOW).await,
        Err(Error::Disabled)
    );
    for bytes in [
        include_str!("fixtures/world_kills_numeric.json").to_owned(),
        GOLDEN.replace("\"contract_version\":2", "\"contract_version\":1"),
        GOLDEN.replace("PublishWorldDailyKillsV2", "PublishUnknownV2"),
        GOLDEN.replace("1789948800", "1789948801"),
        GOLDEN.replace("\"12345\"", "\"012345\""),
        GOLDEN.replace(
            "{\"contract_version\":2",
            "{\"contract_version\":2,\"contract_version\":2",
        ),
        format!("{GOLDEN}\n"),
        "x".repeat(2049),
    ] {
        assert_eq!(
            ingest(&db, config(), SOURCE, bytes.as_bytes(), NOW).await,
            Err(Error::BadInput)
        );
    }
    assert_eq!(
        ingest(&db, config(), "wrong:authority", GOLDEN.as_bytes(), NOW).await,
        Err(Error::Unauthorized)
    );
    assert_eq!(
        ingest(&db, config(), SOURCE, GOLDEN.as_bytes(), NOW - 2).await,
        Err(Error::BadInput)
    );
    assert_eq!(
        presentation(Config {
            presentation_enabled: true,
            ..config()
        }),
        Err(Error::Unavailable)
    );
    Ok(())
}
#[tokio::test]
async fn world_kills_postgres_replay_conflict_epochs_and_retention() -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping world kills PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("world_kills_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105231302_t69b_world_kills.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    for _ in 0..2 {
        assert_eq!(
            ingest(&db, config(), SOURCE, GOLDEN.as_bytes(), NOW).await,
            Ok(Receipt::Accepted)
        );
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM stats_world_kills_v2 WHERE payload->0->>'monsters_killed'='12345'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(count, 1);
    assert_eq!(
        ingest(
            &db,
            config(),
            SOURCE,
            include_bytes!("fixtures/world_kills_conflict.json"),
            NOW
        )
        .await,
        Err(Error::Conflict)
    );
    let replace_revision = |v| {
        GOLDEN.replace(
            "\"projection_revision\":\"2\"",
            &format!("\"projection_revision\":\"{v}\""),
        )
    };
    assert_eq!(
        ingest(&db, config(), SOURCE, replace_revision(1).as_bytes(), NOW).await,
        Ok(Receipt::Superseded)
    );
    let higher_epoch = GOLDEN.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\"");
    assert_eq!(
        ingest(&db, config(), SOURCE, higher_epoch.as_bytes(), NOW).await,
        Err(Error::Unavailable)
    );
    // Higher revisions replace cumulative totals, including explicit zero and
    // provisional days. A tombstone is unavailable rather than a zero day.
    let zero = replace_revision(3)
        .replace("12345", "0")
        .replace("\"12\"", "\"0\"")
        .replace("true", "false");
    assert_eq!(
        ingest(&db, config(), SOURCE, zero.as_bytes(), NOW).await,
        Ok(Receipt::Accepted)
    );
    let is_zero: bool = sqlx::query_scalar("SELECT payload->0->>'monsters_killed'='0' AND payload->0->>'complete'='false' FROM stats_world_kills_v2").fetch_one(&db).await?;
    assert!(is_zero);
    let prefix = replace_revision(4);
    let tombstone = format!("{}[]}}", prefix.split("[{").next().ok_or("fixture body")?);
    assert_eq!(
        ingest(&db, config(), SOURCE, tombstone.as_bytes(), NOW).await,
        Ok(Receipt::Accepted)
    );
    let is_null: bool = sqlx::query_scalar("SELECT payload IS NULL FROM stats_world_kills_v2")
        .fetch_one(&db)
        .await?;
    assert!(is_null);
    assert_eq!(
        ingest(&db, config(), SOURCE, replace_revision(5).as_bytes(), NOW).await,
        Ok(Receipt::Accepted)
    );
    let expiry = 1790035200 + 400 * 86400;
    assert_eq!(purge(&db, expiry - 1).await, Ok(0));
    assert_eq!(purge(&db, expiry).await, Ok(1));
    assert_eq!(
        ingest(
            &db,
            config(),
            SOURCE,
            replace_revision(6).as_bytes(),
            expiry as u64
        )
        .await,
        Ok(Receipt::Accepted)
    );
    let erased: bool =
        sqlx::query_scalar("SELECT payload IS NULL AND retired FROM stats_world_kills_v2")
            .fetch_one(&db)
            .await?;
    assert!(erased);
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
        .execute(&db)
        .await?;
    assert_eq!(
        ingest(&db, config(), SOURCE, GOLDEN.as_bytes(), expiry as u64).await,
        Ok(Receipt::Superseded)
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}
