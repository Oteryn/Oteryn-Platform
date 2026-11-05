#![allow(clippy::expect_used)]
use super::*;
use oteryn_playerdata::progress::{Bestiary, Boss};
fn definition() -> Definition {
    Definition {
        key: "rat".into(),
        revision: "r1".into(),
        class: "mammal".into(),
        difficulty: "easy".into(),
        stars: Some(1),
        thresholds: [1, 5, 10],
        points: Points::Charms(5),
    }
}
fn spawn() -> Spawn {
    Spawn {
        key: "nest".into(),
        creature: "rat".into(),
        cell: super::super::spawns::Cell {
            x: 100,
            y: 200,
            floor: 7,
        },
        policy: SpawnPolicy::Ordinary,
    }
}
fn progress(kills: u32) -> Payload {
    Payload::Bestiary(Bestiary {
        race_key: "rat".into(),
        race_definition_revision: "r1".into(),
        kill_count: kills,
        final_kill_threshold: 10,
        charm_points_awarded: if kills >= 10 { 5 } else { 0 },
    })
}
fn map(d: &[Definition], s: &[Spawn], p: &[Payload]) -> Result<Map> {
    build(d, s, p, &[], 20, &Filters::default(), 1000)
}
#[test]
fn progress_states_and_committed_points() {
    for (kills, state, points) in [
        (0, ProgressState::NotStarted, 0),
        (4, ProgressState::InProgress, 0),
        (11, ProgressState::Completed, 5),
    ] {
        let m = map(&[definition()], &[spawn()], &[progress(kills)]).expect("map");
        assert_eq!(m.entries[0].state, state);
        assert_eq!(m.entries[0].points, points);
    }
    assert!(
        map(&[definition()], &[spawn()], &[])
            .expect("unknown omitted")
            .entries
            .is_empty()
    );
}
#[test]
fn refuses_revision_mismatch_duplicates_and_bad_thresholds() {
    let mut d = definition();
    d.revision = "r2".into();
    assert!(matches!(
        map(&[d], &[spawn()], &[progress(1)]),
        Err(Error::Unavailable)
    ));
    assert!(map(&[definition()], &[spawn()], &[progress(1), progress(1)]).is_err());
    let mut d = definition();
    d.thresholds = [1, 1, 10];
    assert!(map(&[d], &[spawn()], &[progress(1)]).is_err());
    assert_eq!(
        Filters {
            stars: Some(6),
            ..Default::default()
        }
        .validate(),
        Err(Error::BadInput)
    );
}
#[test]
fn boss_stages_cooldowns_and_policy_privacy() {
    let mut d = definition();
    d.points = Points::BossStages([5, 10, 15]);
    let mut s = spawn();
    s.policy = SpawnPolicy::FixedAtReset { chance: 0.25 };
    let p = Payload::Boss(Boss {
        boss_key: "rat".into(),
        definition_revision: "r1".into(),
        kill_count: "5".into(),
        available_at_unix_ms: None,
        last_death_occurrence_id: None,
    });
    let m = map(&[d.clone()], &[s.clone()], std::slice::from_ref(&p)).expect("boss");
    assert_eq!(m.entries[0].points, 15);
    assert_eq!(m.entries[0].stage, 2);
    assert_eq!(m.entries[0].cooldown_remaining_ms, None);
    assert_eq!(m.entries[0].spawns[0].reset_chance, Some(0.25));
    for policy in [
        SpawnPolicy::Room,
        SpawnPolicy::Encounter,
        SpawnPolicy::Unknown,
        SpawnPolicy::Ordinary,
        SpawnPolicy::FixedAtReset { chance: f64::NAN },
    ] {
        s.policy = policy;
        assert!(
            map(
                std::slice::from_ref(&d),
                std::slice::from_ref(&s),
                std::slice::from_ref(&p)
            )
            .expect("hidden")
            .entries
            .is_empty()
        );
    }
}
#[test]
fn recommendations_filters_and_grouped_locations() {
    let rates = [Pace {
        creature: "rat".into(),
        min_level: 10,
        max_level: 30,
        kills_per_hour: 3.0,
    }];
    let m = build(
        &[definition()],
        &[spawn(), spawn()],
        &[progress(4)],
        &rates,
        20,
        &Filters::default(),
        1000,
    )
    .expect("recommendation");
    assert_eq!(m.recommendations, vec!["rat"]);
    assert_eq!(m.entries[0].estimated_hours, Some(2.0));
    assert_eq!(m.entries[0].spawns.len(), 2);
    let f = Filters {
        class: Some("other".into()),
        ..Default::default()
    };
    assert!(
        build(
            &[definition()],
            &[spawn()],
            &[progress(1)],
            &rates,
            20,
            &f,
            1000
        )
        .expect("filtered")
        .entries
        .is_empty()
    );
    assert!(
        build(
            &[definition()],
            &[spawn()],
            &[progress(1)],
            &rates,
            1,
            &Filters::default(),
            1000
        )
        .expect("level")
        .recommendations
        .is_empty()
    );
}
struct Source {
    owner: Uuid,
    character: Uuid,
    observed: u64,
}
impl web::Source for Source {
    fn load_owned(
        &self,
        _: Uuid,
        _: Uuid,
        _: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<web::Snapshot>> + Send + '_>>
    {
        Box::pin(async {
            Ok(web::Snapshot {
                world_version: "857f15f376907c592bb28d854fccff18a15cdf87".into(),
                account: self.owner,
                character: self.character,
                observed_at_ms: self.observed,
                baseline_complete: true,
                level: 20,
                definitions: vec![definition()],
                spawns: vec![spawn()],
                progress: vec![progress(4)],
                pace: vec![],
            })
        })
    }
}
#[tokio::test]
async fn actor_to_localized_page_refuses_other_owner_stale_and_bad_input() {
    let source = Source {
        owner: Uuid::now_v7(),
        character: Uuid::now_v7(),
        observed: 1000,
    };
    for lang in ["pl", "en"] {
        let html = web::show(
            &source,
            source.owner,
            source.character,
            lang,
            &Filters::default(),
            1000,
        )
        .await
        .expect("page");
        if let Ok(directory) = std::env::var("PORTAL_BESTIARY_TEST_OUTPUT") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("bestiary-{lang}.html")),
                &html,
            )
            .expect("synthetic browser fixture");
        }
        assert!(html.contains("viewport"));
        assert!(html.contains("100, 200, 7"));
        assert!(html.contains(if lang == "en" {
            "In progress"
        } else {
            "W trakcie"
        }));
    }
    assert!(matches!(
        web::show(
            &source,
            Uuid::now_v7(),
            source.character,
            "en",
            &Filters::default(),
            1000
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        web::show(
            &source,
            source.owner,
            source.character,
            "en",
            &Filters::default(),
            11001
        )
        .await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        web::show(
            &source,
            source.owner,
            source.character,
            "de",
            &Filters::default(),
            1000
        )
        .await,
        Err(Error::BadInput)
    ));
}
#[tokio::test]
async fn disabled_route_is_absent() {
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .expect("lazy pool");
    let source = Source {
        owner: Uuid::now_v7(),
        character: Uuid::now_v7(),
        observed: 1000,
    };
    let response = web::router(pool, false, std::sync::Arc::new(source))
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/atlas/bestiary/01934f10-7c04-7001-805b-3b1122334401")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn postgres_journal_baseline_and_owner_refusal() {
    use oteryn_playerdata::progress::{Flags, Subject, Watermark};
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping T86 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("PostgreSQL");
    let schema = format!("t86_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await
        .expect("schema");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&pool)
        .await
        .expect("search path");
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105230113_t77_progress.sql"
    ))
    .execute(&pool)
    .await
    .expect("T77 migration");
    struct Trust(Uuid);
    impl Authority for Trust {
        fn permits(&self, s: Subject<'_>) -> bool {
            s.account == self.0 && s.epoch == "1" && s.recovery == "1"
        }
        fn source(&self) -> &str {
            "oteryn:journal:primary"
        }
        fn catalog_accepts(&self, _: &str, _: &Payload) -> bool {
            true
        }
    }
    let account = Uuid::now_v7();
    let character = Uuid::now_v7();
    let trust = Trust(account);
    let store = ProgressStore::new(
        pool.clone(),
        Flags {
            ingest: true,
            read: true,
        },
        vec!["bestiary".into(), "boss_cooldown".into()],
    );
    assert!(
        journal(
            &store,
            Actor::Owner(account),
            account,
            character,
            &trust,
            1000
        )
        .await
        .is_err()
    );
    // SOURCE: synthetic G-11 revision 2 bestiary envelope, no copied character data.
    let record: oteryn_playerdata::progress::Record = serde_json::from_value(serde_json::json!({
        "contract_version":1,"operation":"PublishGameJournalProgressV1",
        "source_authority":trust.source(),"account_id":account,"character_id":character,
        "world_id":Uuid::now_v7(),"projection_revision":"1","projection_epoch":"1",
        "recovery_generation":"1","family":"bestiary","source_revision":"1",
        "record_id":Uuid::now_v7(),"entity_key":"rat","character_revision":"1",
        "occurred_at_unix_ms":"1000","record_kind":"UPSERT",
        "payload":{"race_key":"rat","race_definition_revision":"r1","kill_count":10,
            "final_kill_threshold":10,"charm_points_awarded":5}
    }))
    .expect("synthetic G-11 envelope");
    store
        .ingest(&serde_json::to_vec(&record).expect("record"), &trust)
        .await
        .expect("committed progress");
    for family in ["bestiary", "boss_cooldown"] {
        let w = Watermark {
            contract_version: 1,
            operation: "PublishGameJournalProgressWatermarkV1".into(),
            source_authority: trust.source().into(),
            account_id: account,
            projection_epoch: "1".into(),
            recovery_generation: "1".into(),
            family: family.into(),
            complete_through: if family == "bestiary" { "1" } else { "0" }.into(),
            observed_at_unix_ms: "1000".into(),
            baseline_complete: true,
        };
        store
            .watermark(&serde_json::to_vec(&w).expect("watermark"), &trust, 1000)
            .await
            .expect("baseline");
    }
    let rows = journal(
        &store,
        Actor::Owner(account),
        account,
        character,
        &trust,
        1000,
    )
    .await
    .expect("private progress");
    let m = map(&[definition()], &[spawn()], &rows).expect("journal to map");
    assert_eq!(m.entries[0].state, ProgressState::Completed);
    assert_eq!(m.entries[0].points, 5);
    assert!(matches!(
        journal(
            &store,
            Actor::Owner(Uuid::now_v7()),
            account,
            character,
            &trust,
            1000
        )
        .await,
        Err(Error::Denied)
    ));
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await
        .expect("cleanup");
    pool.close().await;
}
