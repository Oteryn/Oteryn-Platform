#![allow(clippy::unwrap_used)]
use crate::{store::*, worker::StatisticsJob};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::{Date, Month};
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
async fn database() -> Result<Option<PgPool>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping statistics PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let schema = format!("stats_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105210000_t65_statistics.sql"
    ))
    .execute(&pool)
    .await?;
    Ok(Some(pool))
}
async fn dispose(pool: PgPool) -> Result {
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
struct Policy {
    hidden: Option<Uuid>,
    fields: bool,
}
impl Visibility for Policy {
    fn permissions(&self, c: Uuid) -> Permissions {
        Permissions {
            visible: self.hidden != Some(c),
            skills: self.fields,
            online: self.fields,
            guild: self.fields,
            deaths: self.fields,
        }
    }
}
fn sample() -> Snapshot {
    Snapshot {
        character_ref: Uuid::now_v7(),
        day: Date::from_calendar_date(2026, Month::November, 1).unwrap(),
        level: 10,
        experience: Some("100".into()),
        skills: Some(json!({"sword":20})),
        achievement_points: Some(10),
        minutes_online: Some(60),
        name: "First".into(),
        world: Uuid::now_v7(),
        guild: Some(Uuid::now_v7()),
        observed_at: 1,
    }
}
#[tokio::test]
async fn snapshot_upsert_and_all_events_once() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut s = sample();
    let mut tx = db.begin().await?;
    partitions(&mut tx, s.day).await?;
    snapshot(&mut tx, &s).await?;
    snapshot(&mut tx, &s).await?;
    s.day = s.day.next_day().unwrap();
    s.level = 11;
    s.name = "Second".into();
    s.world = Uuid::now_v7();
    s.guild = Some(Uuid::now_v7());
    s.observed_at = 2;
    for _ in 0..2 {
        snapshot(&mut tx, &s).await?;
        source_event(
            &mut tx,
            s.character_ref,
            s.day,
            "death",
            "death-1",
            &json!({"level":10}),
        )
        .await?;
        source_event(
            &mut tx,
            s.character_ref,
            s.day,
            "bazaar_sale",
            "sale-1",
            &json!({"sale_ref":"sale-1"}),
        )
        .await?;
    }
    tx.commit().await?;
    let history = history(
        &db,
        s.character_ref,
        s.day.previous_day().unwrap(),
        s.day,
        &Policy {
            hidden: None,
            fields: true,
        },
    )
    .await?;
    assert_eq!(history.daily.len(), 2);
    assert_eq!(history.events.len(), 7);
    let count: i64 = sqlx::query_scalar("SELECT count(DISTINCT kind) FROM stats_character_events")
        .fetch_one(&db)
        .await?;
    assert_eq!(count, 7);
    let restricted = crate::store::history(
        &db,
        s.character_ref,
        s.day.previous_day().unwrap(),
        s.day,
        &Policy {
            hidden: None,
            fields: false,
        },
    )
    .await?;
    assert_eq!(restricted.events.len(), 4);
    let hidden = crate::store::history(
        &db,
        s.character_ref,
        s.day.previous_day().unwrap(),
        s.day,
        &Policy {
            hidden: Some(s.character_ref),
            fields: true,
        },
    )
    .await?;
    assert!(hidden.daily.is_empty() && hidden.events.is_empty());
    dispose(db).await
}
#[tokio::test]
async fn partitions_created_ahead() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut tx = db.begin().await?;
    partitions(&mut tx, sample().day).await?;
    partitions(&mut tx, sample().day).await?;
    tx.commit().await?;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_inherits WHERE inhparent='stats_character_daily'::regclass",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(n, 3);
    dispose(db).await
}
#[tokio::test]
async fn retention_rollup_is_atomic_and_repeatable() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let world = Uuid::now_v7();
    sqlx::query("INSERT INTO stats_world_samples VALUES ($1,date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' - interval '92 days',10),($1,date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' - interval '92 days' + interval '5 minutes',30),($1,now(),50)").bind(world).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.retention().await?, 1);
    assert_eq!(job.retention().await?, 0);
    let row: (i64, i64, i32, i32) =
        sqlx::query_as("SELECT samples,players_sum,players_min,players_max FROM stats_world_daily")
            .fetch_one(&db)
            .await?;
    assert_eq!(row, (2, 40, 10, 30));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_world_samples")
        .fetch_one(&db)
        .await?;
    assert_eq!(n, 1);
    dispose(db).await
}
#[tokio::test]
async fn growth_privacy_and_bad_input() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut a = sample();
    let mut b = sample();
    let start = a.day;
    let end = start.next_day().unwrap();
    let mut tx = db.begin().await?;
    partitions(&mut tx, start).await?;
    snapshot(&mut tx, &a).await?;
    snapshot(&mut tx, &b).await?;
    a.day = end;
    a.experience = Some("300".into());
    b.day = end;
    b.experience = Some("1000".into());
    snapshot(&mut tx, &a).await?;
    snapshot(&mut tx, &b).await?;
    tx.commit().await?;
    let policy = Policy {
        hidden: Some(b.character_ref),
        fields: false,
    };
    let top = top_growth(&db, start, end, 1, &policy).await?;
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].character_ref, a.character_ref);
    assert_eq!(top[0].experience_gain, "200");
    assert!(
        history(&db, b.character_ref, start, end, &policy)
            .await?
            .daily
            .is_empty()
    );
    let h = history(&db, a.character_ref, start, end, &policy).await?;
    assert!(h.daily[0].skills.is_none());
    assert!(h.daily[0].minutes_online.is_none());
    assert!(top_growth(&db, end, start, 1, &policy).await.is_err());
    assert!(top_growth(&db, start, end, 0, &policy).await.is_err());
    let mut tx = db.begin().await?;
    a.level = 0;
    assert!(snapshot(&mut tx, &a).await.is_err());
    assert!(
        source_event(&mut tx, a.character_ref, end, "other", "key", &json!({}))
            .await
            .is_err()
    );
    tx.rollback().await?;
    dispose(db).await
}
#[tokio::test]
async fn poll_watermark_replay_and_same_timestamp_revision() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let c = Uuid::now_v7();
    let world = Uuid::now_v7();
    let timestamp = 1793491200_i64;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    let payload = json!({"character_id":c.to_string(),"profile":[{"name":"First","level":"10","world_id":world.to_string(),"guild":[],"guild_visible":true}]});
    sqlx::query("INSERT INTO public_snapshots (family,subject,epoch,version,watermark,payload) VALUES ('character_profile',$1,1,1,$2,$3),('world_online',$4,1,1,$2,$5),('character_deaths',$1,1,1,$2,$6)").bind(c.to_string()).bind(timestamp).bind(payload).bind(world.to_string()).bind(json!({"characters":[{}]})).bind(json!({"deaths":[{"occurred_at":timestamp.to_string(),"level":"10"}]})).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.run().await?, 0);
    sqlx::query("INSERT INTO public_snapshots (family,subject,is_watermark,epoch,version,watermark,payload) SELECT family,'',true,1,1,$1,'{}' FROM public_snapshots").bind(timestamp).execute(&db).await?;
    assert_eq!(job.run().await?, 3);
    assert_eq!(job.run().await?, 0);
    sqlx::query("UPDATE public_snapshots SET version=2,payload=jsonb_set(payload,'{profile,0,level}','\"11\"') WHERE family='character_profile' AND NOT is_watermark").execute(&db).await?;
    assert_eq!(job.run().await?, 1);
    assert_eq!(job.run().await?, 0);
    let level: i64 = sqlx::query_scalar("SELECT level FROM stats_character_daily")
        .fetch_one(&db)
        .await?;
    assert_eq!(level, 11);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_character_events")
        .fetch_one(&db)
        .await?;
    assert_eq!(n, 1);
    dispose(db).await
}
#[tokio::test]
async fn disabled_job_refuses_database_access() -> Result {
    let job = StatisticsJob {
        pool: PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused")?,
        enabled: false,
    };
    assert_eq!(job.run().await?, 0);
    assert_eq!(job.retention().await?, 0);
    Ok(())
}

#[tokio::test]
async fn malformed_projection_rolls_back_cursor_and_effects() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO public_snapshots (family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('character_profile',$1,false,1,1,1,'{}'),('character_profile','',true,1,1,1,'{}')").bind(Uuid::now_v7().to_string()).execute(&db).await?;
    assert!(
        StatisticsJob {
            pool: db.clone(),
            enabled: true
        }
        .run()
        .await
        .is_err()
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_projection_cursors")
        .fetch_one(&db)
        .await?;
    assert_eq!(n, 0);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_character_daily")
        .fetch_one(&db)
        .await?;
    assert_eq!(n, 0);
    dispose(db).await
}
