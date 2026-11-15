use super::*;
use sqlx::{Executor, PgConnection, PgPool};
use uuid::Uuid;

const ACCOUNT: &str = "01934f10-7c04-7001-805b-3b1122334401";
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334402";
const WORLD: &str = "01934f10-7c04-7001-805b-3b1122334403";
const OCCURRENCE: &str = "01934f10-7c04-7001-805b-3b1122334412";
const NOW: u64 = 1_790_000_000_000;
fn record(family: Family) -> Record {
    let payload = match family {
        Family::DeathRecap => Payload::Death(Death {
            death_occurrence_id: OCCURRENCE.into(),
            level_before: 20,
            level_after: 19,
            experience_lost: "1000".into(),
            blessings_before: vec!["blessing/example".into()],
            blessings_after: vec![],
            lost_items: vec![Loss {
                loss_index: 0,
                item_key: "item/example".into(),
                quantity: 1,
            }],
            recap_state: "UNAVAILABLE".into(),
            window_ms: 0,
            actions: vec![],
        }),
        Family::Pvp => Payload::Pvp(Pvp {
            death_occurrence_id: OCCURRENCE.into(),
            role: Role::Assist,
            counterpart_character_id: None,
            justification: Justification::Unknown,
            points_milli: None,
        }),
        Family::Skull => Payload::Skull(Skull {
            skull: SkullColor::Red,
            skull_until_unix_ms: Some((NOW - 1).to_string()),
            kill_block_until_unix_ms: None,
        }),
    };
    Record {
        contract_version: 1,
        operation: "PublishGameJournalProgressV1".into(),
        source_authority: "test/progress".into(),
        account_id: ACCOUNT.into(),
        character_id: CHARACTER.into(),
        world_id: WORLD.into(),
        projection_revision: "2".into(),
        projection_epoch: "1".into(),
        recovery_generation: "1".into(),
        family,
        source_revision: "1".into(),
        record_id: Uuid::now_v7().to_string(),
        entity_key: if family == Family::Skull {
            "skull".into()
        } else {
            OCCURRENCE.into()
        },
        character_revision: "3".into(),
        occurred_at_unix_ms: NOW.to_string(),
        record_kind: RecordKind::Upsert,
        payload: Some(payload),
    }
}
fn bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn mark(record: &Record, observed: u64) -> Watermark {
    Watermark {
        contract_version: 1,
        operation: "PublishGameJournalProgressWatermarkV1".into(),
        source_authority: record.source_authority.clone(),
        account_id: record.account_id.clone(),
        projection_epoch: record.projection_epoch.clone(),
        recovery_generation: record.recovery_generation.clone(),
        family: record.family,
        complete_through: record.source_revision.clone(),
        observed_at_unix_ms: observed.to_string(),
        baseline_complete: true,
    }
}
fn flags() -> Flags {
    Flags {
        progress_ingest_enabled: true,
        progress_read_enabled: true,
        enabled_families: vec![Family::DeathRecap, Family::Pvp, Family::Skull],
    }
}
#[test]
fn closed_wire_and_numeric_boundaries() {
    for family in [Family::DeathRecap, Family::Pvp, Family::Skull] {
        let r = record(family);
        assert_eq!(Record::parse(&bytes(&r)).unwrap(), r);
        let mut spaced = bytes(&r);
        spaced.push(b'\n');
        assert_eq!(Record::parse(&spaced), Err(Failure::BadInput));
        let mut extra = bytes(&r);
        extra.pop();
        extra.extend_from_slice(b",\"extra\":0}");
        assert_eq!(Record::parse(&extra), Err(Failure::BadInput));
        let mut duplicate = bytes(&r);
        duplicate.pop();
        duplicate.extend_from_slice(b",\"contract_version\":1}");
        assert_eq!(Record::parse(&duplicate), Err(Failure::BadInput));
    }
    for bad in ["", "01", "-0", "+1", "18446744073709551616", "1.0"] {
        assert!(decimal(bad, true, u64::MAX).is_err());
    }
    assert_eq!(
        decimal("18446744073709551615", false, u64::MAX),
        Ok(u64::MAX)
    );
    assert!(id("01934F10-7c04-7001-805b-3b1122334401").is_err());
    assert_eq!(Record::parse(&vec![b' '; 262145]), Err(Failure::TooLarge));
}
#[test]
fn refuses_unproven_actions_losses_and_blessings() {
    let mut r = record(Family::DeathRecap);
    let Some(Payload::Death(d)) = r.payload.as_mut() else {
        unreachable!()
    };
    d.actions.push(serde_json::json!({"damage": 1}));
    assert!(r.validate().is_err());
    let Some(Payload::Death(d)) = r.payload.as_mut() else {
        unreachable!()
    };
    d.actions.clear();
    d.blessings_after.push("blessing/unknown".into());
    assert!(r.validate().is_err());
    let Some(Payload::Death(d)) = r.payload.as_mut() else {
        unreachable!()
    };
    d.blessings_after.clear();
    d.lost_items.push(d.lost_items[0].clone());
    assert!(r.validate().is_err());
    let Some(Payload::Death(d)) = r.payload.as_mut() else {
        unreachable!()
    };
    d.lost_items = (0..=255)
        .map(|loss_index| Loss {
            loss_index,
            item_key: "item/example".into(),
            quantity: 65535,
        })
        .collect();
    assert!(r.validate().is_ok());
    let Some(Payload::Death(d)) = r.payload.as_mut() else {
        unreachable!()
    };
    d.lost_items[0].quantity = 0;
    assert!(r.validate().is_err());
}
#[test]
fn pvp_and_skull_are_game_assigned() {
    let mut r = record(Family::Pvp);
    let Some(Payload::Pvp(p)) = r.payload.as_mut() else {
        unreachable!()
    };
    p.points_milli = Some(0);
    assert!(r.validate().is_err());
    let Some(Payload::Pvp(p)) = r.payload.as_mut() else {
        unreachable!()
    };
    p.justification = Justification::Justified;
    assert!(r.validate().is_ok());
    let mut r = record(Family::Skull);
    let Some(Payload::Skull(s)) = r.payload.as_mut() else {
        unreachable!()
    };
    s.skull_until_unix_ms = None;
    assert!(r.validate().is_err());
    let Some(Payload::Skull(s)) = r.payload.as_mut() else {
        unreachable!()
    };
    s.skull = SkullColor::None;
    assert!(r.validate().is_ok());
}
#[test]
fn bilingual_semantic_presentation() {
    let projection = Projection {
        stale: false,
        records: vec![
            record(Family::DeathRecap),
            record(Family::Pvp),
            record(Family::Skull),
        ],
    };
    for language in [Language::English, Language::Polish] {
        let html = render(&projection, language).unwrap();
        assert!(html.contains("aria-labelledby"));
        assert!(html.contains("<dl>"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("<table"));
        assert!(!html.contains("style="));
    }
    assert!(
        render(&projection, Language::English)
            .unwrap()
            .contains("final seconds are unavailable")
    );
    assert!(
        render(&projection, Language::Polish)
            .unwrap()
            .contains("Podsumowanie zgonu")
    );
}

/// Synthetic authority represents T72 locks, never a production bypass.
#[derive(Debug)]
struct TestAuthority;
impl Authority for TestAuthority {
    async fn stream(
        &self,
        connection: &mut PgConnection,
        stream: &Stream,
        actor: Actor,
    ) -> Result<(), Failure> {
        let row: (bool,) =
            sqlx::query_as("SELECT allowed FROM test_authority WHERE id=1 FOR SHARE")
                .fetch_one(connection)
                .await
                .map_err(|_| Failure::Unavailable)?;
        if !row.0
            || stream.projection_epoch != "1"
            || stream.recovery_generation != "1"
            || stream.source_authority != "test/progress"
            || stream.account_id != ACCOUNT
            || !(matches!(actor, Actor::Publisher)
                || matches!(actor, Actor::Owner(owner) if owner == id(ACCOUNT).unwrap()))
        {
            return Err(Failure::Denied);
        }
        Ok(())
    }
    async fn subject(
        &self,
        _: &mut PgConnection,
        _: &Stream,
        character: Uuid,
        _: Actor,
    ) -> Result<(), Failure> {
        if character == id(CHARACTER).unwrap() {
            Ok(())
        } else {
            Err(Failure::Denied)
        }
    }
    async fn record(&self, _: &mut PgConnection, record: &Record, _: Actor) -> Result<(), Failure> {
        if record.character_id == CHARACTER
            && record.world_id == WORLD
            && record.character_revision == "3"
            && record.projection_revision == "2"
        {
            Ok(())
        } else {
            Err(Failure::Denied)
        }
    }
}
struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Option<Self> {
        Self::with_migrations(false).await
    }
    async fn with_migrations(all: bool) -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("PostgreSQL T78 test skipped: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        // Identifier consists solely of this fixed prefix and generated UUID hex.
        let schema = format!("t78_{}", Uuid::now_v7().simple());
        root.execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE SCHEMA {schema}"
        ))))
        .await
        .unwrap();
        let options = url
            .parse::<sqlx::postgres::PgConnectOptions>()
            .unwrap()
            .options([("search_path", schema.as_str())]);
        let pool = PgPool::connect_with(options).await.unwrap();
        let mut connection = pool.acquire().await.unwrap();
        if all {
            drop(connection);
            oteryn_db::migrate(&pool).await.unwrap();
            connection = pool.acquire().await.unwrap();
        } else {
            connection
                .execute(sqlx::raw_sql(include_str!(
                    "../../../../migrations/20261105230114_t78_deaths.sql"
                )))
                .await
                .unwrap();
        }
        connection
            .execute(
                "CREATE TABLE test_authority (id integer PRIMARY KEY, allowed boolean NOT NULL)",
            )
            .await
            .unwrap();
        connection
            .execute("INSERT INTO test_authority VALUES (1,true)")
            .await
            .unwrap();
        drop(connection);
        Some(Self { pool, root, schema })
    }
    fn repo(&self) -> Deaths<TestAuthority> {
        Deaths::new(self.pool.clone(), flags(), TestAuthority)
    }
    async fn close(self) {
        self.pool.close().await;
        self.root
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP SCHEMA {} CASCADE",
                self.schema
            ))))
            .await
            .unwrap();
        self.root.close().await;
    }
}
#[tokio::test]
async fn postgres_ingest_replay_watermark_and_conflict() {
    let Some(database) = Database::new().await else {
        return;
    };
    let repo = database.repo();
    for family in [Family::DeathRecap, Family::Pvp, Family::Skull] {
        let r = record(family);
        let ack = repo.ingest(&bytes(&r), NOW).await.unwrap();
        assert_eq!(ack, repo.ingest(&bytes(&r), NOW).await.unwrap());
        assert!(ack.len() <= 256);
        let owner = Actor::Owner(id(ACCOUNT).unwrap());
        assert_eq!(
            repo.read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW)
                .await
                .unwrap_err(),
            Failure::Unavailable
        );
        let mut gap = r.clone();
        gap.source_revision = "3".into();
        gap.record_id = Uuid::now_v7().to_string();
        assert_eq!(repo.ingest(&bytes(&gap), NOW).await, Err(Failure::Conflict));
        let mut conflict = r.clone();
        conflict.occurred_at_unix_ms = (NOW - 1).to_string();
        assert_eq!(
            repo.ingest(&bytes(&conflict), NOW).await,
            Err(Failure::Conflict)
        );
        let mut ahead = mark(&r, NOW);
        ahead.complete_through = "2".into();
        assert_eq!(
            repo.watermark(&bytes(&ahead), NOW).await,
            Err(Failure::Conflict)
        );
        let m = mark(&r, NOW);
        let ack = repo.watermark(&bytes(&m), NOW).await.unwrap();
        assert_eq!(repo.watermark(&bytes(&m), NOW + 1).await.unwrap(), ack);
        let result = repo
            .read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW)
            .await
            .unwrap();
        assert_eq!(result.records, vec![r]);
    }
    database.close().await;
}
#[tokio::test]
async fn postgres_owner_denials_and_transfer_recovery_fences() {
    let Some(database) = Database::new().await else {
        return;
    };
    let repo = database.repo();
    let r = record(Family::DeathRecap);
    repo.ingest(&bytes(&r), NOW).await.unwrap();
    repo.watermark(&bytes(&mark(&r, NOW)), NOW).await.unwrap();
    for actor in [
        Actor::Anonymous,
        Actor::Staff,
        Actor::Publisher,
        Actor::Owner(Uuid::now_v7()),
    ] {
        assert_eq!(
            repo.read(actor, &r.stream(), id(CHARACTER).unwrap(), NOW)
                .await
                .unwrap_err(),
            Failure::Denied
        );
    }
    assert_eq!(
        repo.read(
            Actor::Owner(id(ACCOUNT).unwrap()),
            &r.stream(),
            Uuid::now_v7(),
            NOW
        )
        .await
        .unwrap_err(),
        Failure::Denied
    );
    database
        .pool
        .execute("UPDATE test_authority SET allowed=false")
        .await
        .unwrap();
    assert_eq!(
        repo.read(
            Actor::Owner(id(ACCOUNT).unwrap()),
            &r.stream(),
            id(CHARACTER).unwrap(),
            NOW
        )
        .await
        .unwrap_err(),
        Failure::Denied
    );
    assert_eq!(repo.ingest(&bytes(&r), NOW).await, Err(Failure::Denied));
    database
        .pool
        .execute("UPDATE test_authority SET allowed=true")
        .await
        .unwrap();
    let mut wrong = r.clone();
    wrong.recovery_generation = "2".into();
    assert_eq!(repo.ingest(&bytes(&wrong), NOW).await, Err(Failure::Denied));
    wrong = r.clone();
    wrong.character_revision = "2".into();
    assert_eq!(repo.ingest(&bytes(&wrong), NOW).await, Err(Failure::Denied));
    let denied = Deaths::new(database.pool.clone(), flags(), DenyAll);
    assert_eq!(
        denied.ingest(&bytes(&r), NOW).await,
        Err(Failure::Unavailable)
    );
    database.close().await;
}
#[tokio::test]
async fn postgres_removal_retention_and_nonresurrection() {
    let Some(database) = Database::new().await else {
        return;
    };
    let repo = database.repo();
    let r = record(Family::DeathRecap);
    let owner = Actor::Owner(id(ACCOUNT).unwrap());
    repo.ingest(&bytes(&r), NOW).await.unwrap();
    repo.watermark(&bytes(&mark(&r, NOW)), NOW).await.unwrap();
    assert_eq!(repo.purge_expired(NOW + HISTORY_MS).await.unwrap(), 1);
    assert!(repo.ingest(&bytes(&r), NOW + HISTORY_MS).await.is_ok());
    repo.watermark(&bytes(&mark(&r, NOW + HISTORY_MS)), NOW + HISTORY_MS)
        .await
        .unwrap();
    assert!(
        repo.read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW + HISTORY_MS)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    let mut removed = record(Family::Pvp);
    repo.ingest(&bytes(&removed), NOW).await.unwrap();
    removed.source_revision = "2".into();
    removed.record_id = Uuid::now_v7().to_string();
    removed.record_kind = RecordKind::Remove;
    removed.payload = None;
    repo.ingest(&bytes(&removed), NOW).await.unwrap();
    repo.watermark(&bytes(&mark(&removed, NOW)), NOW)
        .await
        .unwrap();
    assert!(
        repo.read(owner, &removed.stream(), id(CHARACTER).unwrap(), NOW)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    repo.erase_account(id(ACCOUNT).unwrap()).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_t78_records")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    database.close().await;
}
#[tokio::test]
async fn postgres_freshness_skull_deadline_and_flags() {
    let Some(database) = Database::new().await else {
        return;
    };
    let repo = database.repo();
    let owner = Actor::Owner(id(ACCOUNT).unwrap());
    for family in [Family::DeathRecap, Family::Skull] {
        let r = record(family);
        repo.ingest(&bytes(&r), NOW).await.unwrap();
        repo.watermark(&bytes(&mark(&r, NOW)), NOW).await.unwrap();
        let projection = repo
            .read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW)
            .await
            .unwrap();
        if family == Family::Skull {
            assert!(
                matches!(&projection.records[0].payload,Some(Payload::Skull(s)) if s.skull==SkullColor::Red)
            );
        }
        let stale = repo
            .read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW + 10001)
            .await;
        if family == Family::Skull {
            assert_eq!(stale.unwrap_err(), Failure::Unavailable);
        } else {
            assert!(stale.unwrap().stale);
        }
        assert_eq!(
            repo.read(
                owner,
                &r.stream(),
                id(CHARACTER).unwrap(),
                NOW + STALE_MS + 1
            )
            .await
            .unwrap_err(),
            Failure::Unavailable
        );
    }
    let disabled = Deaths::new(database.pool.clone(), Flags::default(), TestAuthority);
    assert_eq!(
        disabled.ingest(b"bad", NOW).await,
        Err(Failure::Unavailable)
    );
    assert_eq!(
        disabled.watermark(b"bad", NOW).await,
        Err(Failure::Unavailable)
    );
    assert_eq!(
        disabled
            .read(
                owner,
                &record(Family::Skull).stream(),
                id(CHARACTER).unwrap(),
                NOW
            )
            .await
            .unwrap_err(),
        Failure::Unavailable
    );
    assert_eq!(repo.ingest(b"bad", NOW).await, Err(Failure::BadInput));
    database.close().await;
}

#[tokio::test]
async fn postgres_http_owner_page_success_denial_and_bad_input() {
    use axum::{
        Extension,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    let Some(database) = Database::new().await else {
        return;
    };
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let repo = database.repo();
    let mut streams = vec![];
    let ingest = internal_router(database.repo());
    for family in [Family::DeathRecap, Family::Pvp, Family::Skull] {
        let mut r = record(family);
        r.occurred_at_unix_ms = now.to_string();
        let response = ingest
            .clone()
            .oneshot(
                Request::post("/internal/v1/playerdata/progress/record")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(bytes(&r)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        repo.watermark(&bytes(&mark(&r, now)), now).await.unwrap();
        streams.push(r.stream());
    }
    let owner = Actor::Owner(id(ACCOUNT).unwrap());
    let page = owner_router(database.repo()).layer(Extension(ReadContext {
        actor: owner,
        streams: streams.clone(),
    }));
    let path = format!("/en/account/journal/{CHARACTER}/deaths");
    let response = page
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let html = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("width=device-width"));
    assert!(html.contains("Death recap"));
    let page = owner_router(database.repo()).layer(Extension(ReadContext {
        actor: Actor::Staff,
        streams,
    }));
    assert_eq!(
        page.oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = ingest
        .clone()
        .oneshot(
            Request::post("/internal/v1/playerdata/progress/record")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
    let response = ingest
        .oneshot(
            Request::post("/internal/v1/playerdata/progress/watermark")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b' '; 1025]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
    database.close().await;
}

#[tokio::test]
async fn postgres_expired_death_entities_do_not_hide_recent_history() {
    let started = std::time::Instant::now();
    let Some(database) = Database::new().await else {
        return;
    };
    let repo = database.repo();
    let mut r = record(Family::DeathRecap);
    r.occurred_at_unix_ms = (NOW - HISTORY_MS).to_string();
    for seq in 1..=201 {
        r.source_revision = seq.to_string();
        r.record_id = Uuid::now_v7().to_string();
        r.entity_key = Uuid::now_v7().to_string();
        if let Some(Payload::Death(p)) = &mut r.payload {
            p.death_occurrence_id = r.entity_key.clone();
        }
        repo.ingest(&bytes(&r), NOW).await.unwrap();
    }
    r.source_revision = "202".into();
    r.record_id = Uuid::now_v7().to_string();
    r.entity_key = Uuid::now_v7().to_string();
    r.occurred_at_unix_ms = NOW.to_string();
    if let Some(Payload::Death(p)) = &mut r.payload {
        p.death_occurrence_id = r.entity_key.clone();
    }
    repo.ingest(&bytes(&r), NOW).await.unwrap();
    repo.watermark(&bytes(&mark(&r, NOW)), NOW).await.unwrap();
    let owner = Actor::Owner(id(ACCOUNT).unwrap());
    assert_eq!(
        repo.read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW)
            .await
            .unwrap()
            .records,
        vec![r.clone()]
    );
    repo.purge_expired(NOW).await.unwrap();
    assert_eq!(
        repo.read(owner, &r.stream(), id(CHARACTER).unwrap(), NOW)
            .await
            .unwrap()
            .records,
        vec![r]
    );
    database.close().await;
    eprintln!(
        "PG postgres_expired_death_entities_do_not_hide_recent_history {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_production_retention_erases_death_payloads_preserves_fences() {
    use oteryn_db::outbox::Cleanup;
    let started = std::time::Instant::now();
    let Some(database) = Database::with_migrations(true).await else {
        return;
    };
    let repo = database.repo();
    let r = record(Family::DeathRecap);
    let accepted = repo.ingest(&bytes(&r), NOW).await.unwrap();
    sqlx::query("UPDATE pd_t78_records SET occurred_at_ms=0")
        .execute(&database.pool)
        .await
        .unwrap();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&database.pool)
            .await
            .unwrap();
    let mut retained = r.clone();
    retained.source_revision = "2".into();
    retained.record_id = Uuid::now_v7().to_string();
    retained.entity_key = Uuid::now_v7().to_string();
    retained.occurred_at_unix_ms = (now - 31 * 86400000_i64).to_string();
    if let Some(Payload::Death(p)) = &mut retained.payload {
        p.death_occurrence_id = retained.entity_key.clone();
    }
    repo.ingest(&bytes(&retained), now as u64).await.unwrap();
    crate::retention::Retention(database.pool.clone())
        .clean()
        .await
        .unwrap();
    let state: (i64, i64) = sqlx::query_as("SELECT count(*),count(payload) FROM pd_t78_records")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(state, (2, 1));
    let retained_payload: bool =
        sqlx::query_scalar("SELECT payload IS NOT NULL FROM pd_t78_records WHERE record_id=$1")
            .bind(id(&retained.record_id).unwrap())
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert!(
        retained_payload,
        "death history retains its full ninety-day window"
    );
    assert_eq!(repo.ingest(&bytes(&r), NOW).await.unwrap(), accepted);
    database.close().await;
    eprintln!(
        "PG postgres_production_retention_erases_death_payloads_preserves_fences {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_01_production_lifecycle_dispatches_death_erasure() {
    let started = std::time::Instant::now();
    let Some(database) = Database::with_migrations(true).await else {
        return;
    };
    let repo = database.repo();
    let r = record(Family::DeathRecap);
    let accepted = repo.ingest(&bytes(&r), NOW).await.unwrap();
    sqlx::query("INSERT INTO pd_erasure_jobs(account_id,character_id) VALUES($1,$2)")
        .bind(ACCOUNT)
        .bind(CHARACTER)
        .execute(&database.pool)
        .await
        .unwrap();
    crate::retention::Retention(database.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM pd_t78_records WHERE payload IS NOT NULL OR entity_key<>'' OR occurred_at_ms<>0").fetch_one(&database.pool).await.unwrap();
    assert_eq!(count, 0);
    assert_eq!(repo.ingest(&bytes(&r), NOW).await.unwrap(), accepted);
    sqlx::query("UPDATE pd_t78_records SET erased_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-120*86400000::bigint+3600000").execute(&database.pool).await.unwrap();
    crate::retention::Retention(database.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_t78_records")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("UPDATE pd_t78_records SET erased_at_ms=erased_at_ms-7200000")
        .execute(&database.pool)
        .await
        .unwrap();
    crate::retention::Retention(database.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_t78_records")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert!(matches!(
        repo.ingest(&bytes(&r), NOW).await,
        Err(Failure::Conflict)
    ));
    sqlx::query("INSERT INTO pd_erasure_jobs(account_id) VALUES($1)")
        .bind(ACCOUNT)
        .execute(&database.pool)
        .await
        .unwrap();
    crate::retention::Retention(database.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    assert!(matches!(
        repo.ingest(&bytes(&r), NOW).await,
        Err(Failure::Conflict)
    ));
    database.close().await;
    eprintln!(
        "postgres_pd2_01_production_lifecycle_dispatches_death_erasure duration={:?}",
        started.elapsed()
    );
}
