#![allow(clippy::unwrap_used)]
use crate::{store::*, worker::StatisticsJob};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::{Date, Month, OffsetDateTime};
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
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105231101_t24d_deaths.sql"
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
    let timestamp = OffsetDateTime::now_utc().unix_timestamp() - 10;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    let payload = json!({"character_id":c.to_string(),"profile":[{"name":"First","level":"10","world_id":world.to_string(),"guild":[],"guild_visible":true}]});
    sqlx::query("INSERT INTO public_snapshots (family,subject,epoch,version,watermark,payload) VALUES ('character_profile',$1,1,1,$2,$3),('world_online',$4,1,1,$2,$5),('character_deaths',$1,1,1,$2,$6)").bind(c.to_string()).bind(timestamp).bind(payload).bind(world.to_string()).bind(json!({"characters":[{}]})).bind(json!({"deaths":[{"occurred_at":timestamp.to_string(),"level":"10"}]})).execute(&db).await?;
    sqlx::query("INSERT INTO public_deaths_v2(subject,authority,epoch,revision,digest,deaths) VALUES($1,'oteryn:game:primary',1,1,$2,$3)")
        .bind(c.to_string()).bind(vec![1_u8;32]).bind(json!([{"death_occurrence_id":Uuid::now_v7().to_string(),"occurred_at":timestamp.to_string(),"level":"10","pvp":false,"killers":[]}])).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.run().await?, 0);
    sqlx::query("INSERT INTO public_snapshots (family,subject,is_watermark,epoch,version,watermark,payload) SELECT family,'',true,1,1,$1,'{}' FROM public_snapshots").bind(timestamp).execute(&db).await?;
    sqlx::query("INSERT INTO public_deaths_v2_watermark(id,authority,epoch,complete_through) VALUES(1,'oteryn:game:primary',1,$1)").bind(timestamp).execute(&db).await?;
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

#[tokio::test]
async fn postgres_snapshot_corrections_replace_derived_events() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut original = sample();
    let mut tx = db.begin().await?;
    partitions(&mut tx, original.day).await?;
    snapshot(&mut tx, &original).await?;
    original.day = original.day.next_day().unwrap();
    original.observed_at = 3;
    let mut changed = original.clone();
    changed.name = "Changed".into();
    changed.world = Uuid::now_v7();
    changed.guild = Some(Uuid::now_v7());
    changed.level += 1;
    changed.observed_at = 2;
    snapshot(&mut tx, &changed).await?;
    source_event(
        &mut tx,
        original.character_ref,
        original.day,
        "death",
        "independent",
        &json!({"level":10}),
    )
    .await?;
    snapshot(&mut tx, &original).await?;
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM stats_character_events WHERE character_ref=$1 AND day=$2 ORDER BY kind",
    )
    .bind(original.character_ref)
    .bind(original.day)
    .fetch_all(&mut *tx)
    .await?;
    assert_eq!(kinds, vec!["death"]);
    tx.commit().await?;
    dispose(db).await
}

#[tokio::test]
async fn postgres_rollup_rejects_finalized_replay() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let world = Uuid::now_v7();
    let observed: i64 =
        sqlx::query_scalar("SELECT extract(epoch FROM now()-interval '100 days')::bigint")
            .fetch_one(&db)
            .await?;
    sqlx::query("INSERT INTO stats_world_samples(world,sampled_at,players_online) VALUES($1,to_timestamp(($2::bigint/300)*300),7)").bind(world).bind(observed).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    job.retention().await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('world_online',$1,false,2,1,$2,$3),('world_online','',true,2,1,$2,'{}')").bind(world.to_string()).bind(observed).bind(json!({"characters":[{},{}]})).execute(&db).await?;
    assert_eq!(job.run().await?, 1);
    let totals: (i64, String) =
        sqlx::query_as("SELECT samples,players_sum::text FROM stats_world_daily WHERE world=$1")
            .bind(world)
            .fetch_one(&db)
            .await?;
    assert_eq!(totals, (1, "7".into()));
    dispose(db).await
}

#[tokio::test]
async fn postgres_worker_batches_cover_backlog_once() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) SELECT 'world_online',uuidv7()::text,false,1,1,extract(epoch FROM now())::bigint,'{\"characters\":[]}' FROM generate_series(1,205)").execute(&db).await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('world_online','',true,1,1,extract(epoch FROM now())::bigint,'{}')").execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.run().await?, 100);
    assert_eq!(job.run().await?, 100);
    assert_eq!(job.run().await?, 5);
    assert_eq!(job.run().await?, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM stats_projection_cursors")
        .fetch_one(&db)
        .await?;
    assert_eq!(count, 205);
    dispose(db).await
}

#[tokio::test]
async fn postgres_history_refuses_oversized_events() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let s = sample();
    let mut tx = db.begin().await?;
    partitions(&mut tx, s.day).await?;
    snapshot(&mut tx, &s).await?;
    for n in 0..1001 {
        source_event(
            &mut tx,
            s.character_ref,
            s.day,
            "death",
            &n.to_string(),
            &json!({"level":10}),
        )
        .await?;
    }
    tx.commit().await?;
    assert!(
        history(
            &db,
            s.character_ref,
            s.day,
            s.day,
            &Policy {
                hidden: None,
                fields: true
            }
        )
        .await
        .is_err()
    );
    let hidden = history(
        &db,
        s.character_ref,
        s.day,
        s.day,
        &Policy {
            hidden: None,
            fields: false,
        },
    )
    .await?;
    assert!(hidden.events.is_empty());
    dispose(db).await
}

struct OnlyVisible(Uuid);
impl Visibility for OnlyVisible {
    fn permissions(&self, id: Uuid) -> Permissions {
        Permissions {
            visible: id == self.0,
            ..Permissions::default()
        }
    }
}
#[tokio::test]
async fn postgres_growth_privacy_crosses_multiple_batches() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let mut s = sample();
    s.day = s.day.next_day().unwrap();
    let mut tx = db.begin().await?;
    partitions(&mut tx, s.day).await?;
    sqlx::query("INSERT INTO stats_character_daily(character_ref,day,level,experience,name,world,observed_at) SELECT uuidv7(),$1,10,n,'Observed', $2,1 FROM generate_series(1,205) n").bind(s.day).bind(s.world).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO stats_character_daily(character_ref,day,level,experience,name,world,observed_at) SELECT character_ref,$1,level,0,name,world,0 FROM stats_character_daily").bind(s.day.previous_day().unwrap()).execute(&mut *tx).await?;
    let visible: Uuid =
        sqlx::query_scalar("SELECT character_ref FROM stats_character_daily WHERE experience=1")
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    let result = top_growth(
        &db,
        s.day.previous_day().unwrap(),
        s.day,
        1,
        &OnlyVisible(visible),
    )
    .await?;
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].character_ref, visible);
    assert_eq!(result[0].experience_gain, "1");
    dispose(db).await
}

struct DeathsVisible;
impl crate::character::Current for DeathsVisible {
    fn character(&self, _: Uuid) -> crate::character::CurrentFuture<'_> {
        Box::pin(async {
            Ok(crate::character::PublicCharacter {
                name: "Observed".into(),
                permissions: Permissions {
                    visible: true,
                    deaths: true,
                    ..Permissions::default()
                },
                ranks: None,
            })
        })
    }
}
#[tokio::test]
async fn postgres_v2_deaths_occurrences_replay_suppression_and_fences() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let character = Uuid::now_v7();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let occurrence_a = Uuid::now_v7();
    let occurrence_b = Uuid::now_v7();
    let deaths = json!([
        {"death_occurrence_id":occurrence_a.to_string(),"occurred_at":(now-10).to_string(),"level":"413","pvp":true,"killers":["Beren"]},
        {"death_occurrence_id":occurrence_b.to_string(),"occurred_at":(now-10).to_string(),"level":"412","pvp":false,"killers":[]},
        {"death_occurrence_id":Uuid::now_v7().to_string(),"occurred_at":(now-31*86400).to_string(),"level":"410","pvp":false,"killers":[]},
        {"death_occurrence_id":Uuid::now_v7().to_string(),"occurred_at":(now+3600).to_string(),"level":"414","pvp":false,"killers":[]}
    ]);
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    // This is the normalized persistence shape written by PublishCharacterDeathsV2.
    sqlx::query("INSERT INTO public_deaths_v2(subject,authority,epoch,revision,digest,deaths) VALUES($1,'oteryn:game:primary',1,1,$2,$3)")
        .bind(character.to_string()).bind(vec![1_u8;32]).bind(&deaths).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.run().await?, 0); // No completeness evidence.
    sqlx::query("INSERT INTO public_deaths_v2_watermark(id,authority,epoch,complete_through) VALUES(1,'oteryn:game:other',1,$1)").bind(now).execute(&db).await?;
    assert_eq!(job.run().await?, 0); // Authority mismatch.
    sqlx::query("UPDATE public_deaths_v2_watermark SET authority='oteryn:game:primary',epoch=2")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 0); // Epoch mismatch.
    sqlx::query("UPDATE public_deaths_v2_watermark SET epoch=1,complete_through=$1")
        .bind(now - 20)
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stats_character_events WHERE kind='death'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 0); // Neither same-second occurrence is complete yet.
    sqlx::query("UPDATE public_deaths_v2_watermark SET complete_through=$1")
        .bind(now)
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    assert_eq!(job.run().await?, 0);
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT event_key FROM stats_character_events WHERE kind='death' ORDER BY event_key",
    )
    .fetch_all(&db)
    .await?;
    let mut expected = vec![occurrence_a.to_string(), occurrence_b.to_string()];
    expected.sort();
    assert_eq!(keys, expected);
    let service = crate::character::Characters {
        db: db.clone(),
        enabled: true,
        current: std::sync::Arc::new(DeathsVisible),
    };
    let day = OffsetDateTime::from_unix_timestamp(now - 10)?.date();
    let report = service
        .load(character, day, day, None)
        .await
        .map_err(|e| format!("report unavailable: {e:?}"))?;
    assert_eq!(
        report
            .history
            .events
            .iter()
            .filter(|e| e.kind == "death")
            .count(),
        2
    );
    // Higher-epoch replay replaces the derived set rather than duplicating it.
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
        .execute(&db)
        .await?;
    sqlx::query("UPDATE public_deaths_v2 SET epoch=2")
        .execute(&db)
        .await?;
    sqlx::query("UPDATE public_deaths_v2_watermark SET epoch=2")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stats_character_events WHERE kind='death'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 2);
    sqlx::query("UPDATE public_deaths_v2 SET revision=2,deaths=$1")
        .bind(json!([deaths[0].clone()]))
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT event_key FROM stats_character_events WHERE kind='death'")
            .fetch_all(&db)
            .await?;
    assert_eq!(keys, vec![occurrence_a.to_string()]);
    sqlx::query("UPDATE public_deaths_v2 SET revision=3,deaths='[]'")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stats_character_events WHERE kind='death'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 0);
    sqlx::query("UPDATE public_deaths_v2 SET revision=4,deaths=$1")
        .bind(&deaths)
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    sqlx::query("UPDATE public_projection_state SET highest_epoch=3")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 0);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stats_character_events WHERE kind='death'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 0); // Fenced bodies disappear even before epoch 3 is populated.
    dispose(db).await
}

#[tokio::test]
async fn postgres_v2_deaths_expire_without_projection_updates() -> Result {
    let Some(db) = database().await? else {
        return Ok(());
    };
    let character = Uuid::now_v7();
    let occurred = OffsetDateTime::now_utc() - time::Duration::days(31);
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    let mut tx = db.begin().await?;
    let occurrence = Uuid::now_v7().to_string();
    source_event(&mut tx, character, occurred.date(), "death", &occurrence,
        &json!({"death_occurrence_id":occurrence,"occurred_at":occurred.unix_timestamp().to_string(),"level":"10","_statistics_epoch":"1"})).await?;
    source_event(
        &mut tx,
        character,
        occurred.date(),
        "bazaar_sale",
        "settled-sale",
        &json!({"sale_ref":"settled-sale"}),
    )
    .await?;
    tx.commit().await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    assert_eq!(job.run().await?, 0);
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM stats_character_events WHERE character_ref=$1 ORDER BY kind",
    )
    .bind(character)
    .fetch_all(&db)
    .await?;
    assert_eq!(kinds, vec!["bazaar_sale"]);
    dispose(db).await
}

#[tokio::test]
async fn postgres_death_batches_progress_with_advancing_watermark() -> Result {
    let started = std::time::Instant::now();
    let Some(db) = database().await? else {
        return Ok(());
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    let mut subjects = Vec::new();
    for _ in 0..205 {
        let subject = Uuid::now_v7();
        subjects.push(subject);
        sqlx::query("INSERT INTO public_deaths_v2(subject,authority,epoch,revision,digest,deaths) VALUES($1,'oteryn:game:primary',1,1,$2,$3)")
            .bind(subject.to_string()).bind(vec![1_u8;32]).bind(json!([{"death_occurrence_id":Uuid::now_v7().to_string(),"occurred_at":(now-10).to_string(),"level":"10"}])).execute(&db).await?;
    }
    sqlx::query("INSERT INTO public_deaths_v2_watermark(id,authority,epoch,complete_through) VALUES(1,'oteryn:game:primary',1,$1)").bind(now-20).execute(&db).await?;
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    // First batch is withheld by completeness, and must be revisited later.
    assert_eq!(job.run().await?, 100);
    for step in 0..6 {
        sqlx::query("UPDATE public_deaths_v2_watermark SET complete_through=$1")
            .bind(now - 9 + step)
            .execute(&db)
            .await?;
        job.run().await?;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM stats_character_events WHERE kind='death'"
        )
        .fetch_one(&db)
        .await?,
        205
    );
    sqlx::query("UPDATE public_deaths_v2 SET revision=2,deaths='[]' WHERE subject=$1")
        .bind(subjects[204].to_string())
        .execute(&db)
        .await?;
    sqlx::query("UPDATE public_deaths_v2_watermark SET complete_through=$1")
        .bind(now - 2)
        .execute(&db)
        .await?;
    job.run().await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM stats_character_events WHERE character_ref=$1"
        )
        .bind(subjects[204])
        .fetch_one(&db)
        .await?,
        0
    );
    dispose(db).await?;
    eprintln!(
        "postgres_death_batches_progress_with_advancing_watermark duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn audit_w3_01_quarantined_deaths_derivation_and_history() -> Result {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
    use tower::ServiceExt;
    let started = std::time::Instant::now();
    let db = database()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    // The publisher uses the current replay and admission schema.
    for migration in [
        include_str!("../../../migrations/20261105190000_t25_world_registry.sql"),
        include_str!("../../../migrations/20261105231103_t24f_houses.sql"),
        include_str!("../../../migrations/20261115110006_fix_06_projection_read_bounds.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&db).await?;
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261115110016_fix_w2a_ingest_budget.sql"
    ))
    .execute(&db)
    .await?;
    let character = Uuid::now_v7();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let day = OffsetDateTime::now_utc().date();
    let identity = Identity::san("stats-deaths-test").map_err(|e| format!("{e:?}"))?;
    let identities = IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: "game-public".into(),
    }])
    .map_err(|e| format!("{e:?}"))?;
    let app = oteryn_community::deaths::ingest_router(
        db.clone(),
        std::sync::Arc::new(identities),
        oteryn_community::deaths::IngestConfig {
            enabled: true,
            source_authority: "oteryn:game:primary".into(),
            requests_per_minute: 6000,
        },
    );
    let post = |path: &str, value: String| {
        let request = Request::post(format!("/internal/v2/game-public/{path}"))
            .header("content-type", "application/json")
            .extension(VerifiedClient::from_verified_tls13_http11(identity.clone()))
            .body(Body::from(value));
        let app = app.clone();
        async move { Ok::<_, Box<dyn std::error::Error>>(app.oneshot(request?).await?.status()) }
    };
    let occurrence = Uuid::now_v7();
    let publication = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterDeathsV2","source_authority":"oteryn:game:primary","character_id":"{character}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","deaths":[{{"death_occurrence_id":"{occurrence}","occurred_at":"{}","level":"10","pvp":false,"killers":[]}}]}}"#,
        now - 10
    );
    assert_eq!(
        post("character-deaths", publication.clone()).await?,
        StatusCode::OK
    );
    assert_eq!(post("watermark", format!(r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"character_deaths","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#)).await?, StatusCode::OK);
    let job = StatisticsJob {
        pool: db.clone(),
        enabled: true,
    };
    let policy = Policy {
        hidden: None,
        fields: true,
    };
    let mut snap = sample();
    snap.character_ref = character;
    snap.day = day;
    let mut tx = db.begin().await?;
    partitions(&mut tx, day).await?;
    snapshot(&mut tx, &snap).await?;
    tx.commit().await?;
    // Derive once, then quarantine: already copied evidence must disappear from readers.
    assert_eq!(job.run().await?, 1);
    assert_eq!(
        history(&db, character, day, day, &policy)
            .await?
            .events
            .iter()
            .filter(|e| e.kind == "death")
            .count(),
        1
    );
    let publication = publication.replace(r#""level":"10""#, r#""level":"11""#);
    assert_eq!(
        post("character-deaths", publication).await?,
        StatusCode::CONFLICT
    );
    assert!(
        history(&db, character, day, day, &policy)
            .await?
            .events
            .iter()
            .all(|e| e.kind != "death")
    );
    let report = crate::world::read(
        &db,
        crate::world::Config { enabled: true },
        snap.world,
        day,
        day,
        &policy,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(report.days[0].deaths, None);
    // Simulate polling for the first time after the same conflict, with the V2 watermark intact.
    sqlx::query("DELETE FROM stats_projection_cursors")
        .execute(&db)
        .await?;
    sqlx::query("DELETE FROM stats_character_events WHERE kind='death'")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stats_projection_cursors")
            .fetch_one(&db)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM stats_character_events WHERE kind='death'"
        )
        .fetch_one(&db)
        .await?,
        0
    );
    // Only authoritative reconciliation releases the persistent fence.
    sqlx::query("DELETE FROM public_projection_conflicts WHERE family='character_deaths'")
        .execute(&db)
        .await?;
    assert_eq!(job.run().await?, 1);
    assert!(
        history(&db, character, day, day, &policy)
            .await?
            .events
            .iter()
            .any(|e| e.kind == "death")
    );
    dispose(db).await?;
    eprintln!(
        "audit_w3_01_quarantined_deaths_derivation_and_history duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
