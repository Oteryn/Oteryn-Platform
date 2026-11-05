#![allow(clippy::expect_used)]
use super::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
const NOW: u64 = 1790000000000;
#[derive(Debug)]
struct Trust {
    account: Uuid,
    character: Uuid,
    allow: bool,
}
impl Authority for Trust {
    fn source(&self) -> &str {
        "oteryn:journal:primary"
    }
    fn permits(&self, s: Subject<'_>) -> bool {
        self.allow
            && s.account == self.account
            && s.character.is_none_or(|c| c == self.character)
            && s.epoch == "1"
            && s.recovery == "3"
            && s.projection.is_none_or(|v| v == "12")
            && s.character_revision.is_none_or(|v| v == "9")
    }
    fn catalog_accepts(&self, _: &str, _: &Payload) -> bool {
        self.allow
    }
}
fn trust() -> Trust {
    Trust {
        account: Uuid::now_v7(),
        character: Uuid::now_v7(),
        allow: true,
    }
}
// SOURCE: synthetic envelope and payload shaped from G-11 revision 2 §10;
// Game semantics checked read-only at 857f15f376907c592bb28d854fccff18a15cdf87.
fn record(t: &Trust, seq: &str) -> Record {
    Record {contract_version:1,operation:"PublishGameJournalProgressV1".into(),source_authority:t.source().into(),account_id:t.account,character_id:Some(t.character),world_id:Some(Uuid::now_v7()),projection_revision:Some("12".into()),projection_epoch:"1".into(),recovery_generation:"3".into(),family:"bestiary".into(),source_revision:seq.into(),record_id:Uuid::now_v7(),entity_key:"rat".into(),character_revision:Some("9".into()),occurred_at_unix_ms:NOW.to_string(),record_kind:"UPSERT".into(),payload:Some(serde_json::value::RawValue::from_string("{\"race_key\":\"rat\",\"race_definition_revision\":\"r1\",\"kill_count\":100,\"final_kill_threshold\":100,\"charm_points_awarded\":5}".into()).expect("synthetic JSON"))}
}
fn bytes(r: &Record) -> Vec<u8> {
    serde_json::to_vec(r).expect("serialize fixture")
}
fn wm(t: &Trust, cut: &str) -> Vec<u8> {
    serde_json::to_vec(&Watermark {
        contract_version: 1,
        operation: "PublishGameJournalProgressWatermarkV1".into(),
        source_authority: t.source().into(),
        account_id: t.account,
        projection_epoch: "1".into(),
        recovery_generation: "3".into(),
        family: "bestiary".into(),
        complete_through: cut.into(),
        observed_at_unix_ms: NOW.to_string(),
        baseline_complete: true,
    })
    .expect("serialize fixture")
}
async fn db() -> Option<(PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping T77 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("test PostgreSQL");
    let schema = format!("t77_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await
        .expect("create schema");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&pool)
        .await
        .expect("set schema");
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105230113_t77_progress.sql"
    ))
    .execute(&pool)
    .await
    .expect("migration");
    Some((pool, schema))
}
async fn cleanup(pool: PgPool, schema: String) {
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await
        .expect("cleanup");
    pool.close().await;
}
fn store(pool: PgPool) -> ProgressStore {
    ProgressStore::new(
        pool,
        Flags {
            ingest: true,
            read: true,
        },
        vec!["bestiary".into()],
    )
}
#[test]
fn forecast_handles_success_refusal_and_bad_input() {
    let s = [
        Sample {
            at_ms: 100,
            value: 10,
        },
        Sample {
            at_ms: 200,
            value: 20,
        },
    ];
    assert!(matches!(forecast(&s, 30, 200), Ok(Some(300))));
    assert!(matches!(
        forecast(&[s[0], Sample { value: 10, ..s[1] }], 30, 200),
        Ok(None)
    ));
    assert!(forecast(&[s[1], s[0]], 30, 200).is_err());
    assert!(matches!(forecast(&s, 30, 86400201), Ok(None)));
}
#[test]
fn closed_records_reject_invalid_and_noncanonical_input() {
    let t = trust();
    let mut r = record(&t, "1");
    assert!(Record::decode(&bytes(&r)).is_ok());
    r.source_revision = "01".into();
    assert!(Record::decode(&bytes(&r)).is_err());
    r.source_revision = "1".into();
    r.entity_key = "other".into();
    assert!(Record::decode(&bytes(&r)).is_err());
    let mut b = bytes(&record(&t, "1"));
    b.push(b'\n');
    assert!(Record::decode(&b).is_err());
    assert_eq!(Error::BadInput.status(), 400);
}
#[test]
fn families_preserve_unknowns_and_validate_scopes() {
    let boss=Payload::decode("boss_cooldown",serde_json::json!({"boss_key":"boss/example","definition_revision":"r1","kill_count":"2","available_at_unix_ms":null,"last_death_occurrence_id":null}),false).expect("boss");
    assert!(matches!(boss,Payload::Boss(ref b) if b.cooldown_remaining_ms(NOW).is_none()));
    let q = serde_json::json!({"quest_key":"quest/example","pinned_content_revision":"r1","definition_hash":"a".repeat(64),"completed":false,"tracks":[{"track_key":"quest/example/progress","value":"2"}]});
    assert!(Payload::decode("quest", q.clone(), false).is_ok());
    let mut bad = q;
    bad["tracks"][0]["value"] = serde_json::json!("02");
    assert!(Payload::decode("quest", bad, false).is_err());
    let a = serde_json::json!({"achievement_key":"oteryn:achievement/example","definition_revision":"r1","progress":2,"target":10,"granted_at_unix_ms":null});
    assert!(Payload::decode("achievement", a.clone(), false).is_ok());
    assert!(Payload::decode("achievement", a, true).is_err());
    let h = serde_json::json!({"build_occurrence_id":Uuid::now_v7(),"cause":"training","policy_revision":"r1","skills":[{"skill":"sword","level_before":10,"progress_before":"9","level_after":11,"progress_after":"0"}]});
    assert!(Payload::decode("skill_history", h.clone(), false).is_ok());
    let mut bad = h;
    bad["skills"][0]["level_after"] = serde_json::json!(9);
    assert!(Payload::decode("skill_history", bad, false).is_err());
}
#[tokio::test]
async fn postgres_idempotency_conflict_and_bad_input() {
    let Some((pool, schema)) = db().await else {
        return;
    };
    let s = store(pool.clone());
    let t = trust();
    let mut r = record(&t, "1");
    let b = bytes(&r);
    assert_eq!(
        s.ingest(&b, &t).await.expect("accept"),
        s.ingest(&b, &t).await.expect("retry")
    );
    r.entity_key = "other".into();
    assert!(matches!(
        s.ingest(&bytes(&r), &t).await,
        Err(Error::BadInput)
    ));
    r.entity_key = "rat".into();
    r.record_id = Uuid::now_v7();
    assert!(matches!(
        s.ingest(&bytes(&r), &t).await,
        Err(Error::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_progress_records")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 1);
    cleanup(pool, schema).await;
}
#[tokio::test]
async fn postgres_gaps_baseline_owner_and_transfer_denials() {
    let Some((pool, schema)) = db().await else {
        return;
    };
    let s = store(pool.clone());
    let mut t = trust();
    let r = record(&t, "2");
    s.ingest(&bytes(&r), &t).await.expect("out of order");
    assert!(matches!(
        s.watermark(&wm(&t, "2"), &t, NOW).await,
        Err(Error::Conflict)
    ));
    let scope = ReadScope {
        account: t.account,
        character: Some(t.character),
        family: "bestiary",
    };
    assert!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW)
            .await
            .is_err()
    );
    s.ingest(&bytes(&record(&t, "1")), &t)
        .await
        .expect("gap fill");
    s.watermark(&wm(&t, "2"), &t, NOW).await.expect("baseline");
    assert_eq!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW)
            .await
            .expect("owner")
            .len(),
        1
    );
    for actor in [Actor::Owner(Uuid::now_v7()), Actor::Staff, Actor::Anonymous] {
        assert!(matches!(
            s.read(actor, &scope, &t, NOW).await,
            Err(Error::Denied)
        ));
    }
    t.allow = false;
    assert!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW)
            .await
            .is_err()
    );
    cleanup(pool, schema).await;
}
#[tokio::test]
async fn postgres_default_flags_empty_and_tombstone() {
    let Some((pool, schema)) = db().await else {
        return;
    };
    let t = trust();
    let disabled = ProgressStore::new(pool.clone(), Flags::default(), vec!["bestiary".into()]);
    assert!(matches!(
        disabled.ingest(&bytes(&record(&t, "1")), &t).await,
        Err(Error::Disabled)
    ));
    let s = store(pool.clone());
    let scope = ReadScope {
        account: t.account,
        character: Some(t.character),
        family: "bestiary",
    };
    s.watermark(&wm(&t, "0"), &t, NOW)
        .await
        .expect("empty baseline");
    assert!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW)
            .await
            .expect("empty")
            .is_empty()
    );
    s.ingest(&bytes(&record(&t, "1")), &t)
        .await
        .expect("record");
    let mut r = record(&t, "2");
    r.record_kind = "REMOVE".into();
    r.payload = None;
    s.ingest(&bytes(&r), &t).await.expect("remove");
    s.watermark(&wm(&t, "2"), &t, NOW).await.expect("watermark");
    assert!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW)
            .await
            .expect("removed")
            .is_empty()
    );
    assert!(
        s.read(Actor::Owner(t.account), &scope, &t, NOW + 86400001)
            .await
            .is_err()
    );
    cleanup(pool, schema).await;
}
#[tokio::test]
async fn postgres_goals_owner_input_and_cut_fences() {
    let Some((pool, schema)) = db().await else {
        return;
    };
    let s = store(pool.clone());
    let t = trust();
    let mut g = Goal {
        account: t.account,
        character: t.character,
        epoch: "1".into(),
        recovery: "3".into(),
        projection: "12".into(),
        character_revision: "9".into(),
        kind: "level".into(),
        entity_key: "level".into(),
        target: 100,
    };
    struct Pace;
    impl PrivatePace for Pace {
        async fn samples(&self, _: &Goal) -> Result<Vec<Sample>> {
            Ok(vec![
                Sample {
                    at_ms: NOW - 1000,
                    value: 10,
                },
                Sample {
                    at_ms: NOW,
                    value: 20,
                },
            ])
        }
    }
    let id = s
        .save_goal(Actor::Owner(t.account), &g, &t)
        .await
        .expect("goal");
    assert_eq!(
        s.goals(Actor::Owner(t.account), &g, &t)
            .await
            .expect("goals")[0]
            .0,
        id
    );
    assert!(matches!(
        s.save_goal(Actor::Staff, &g, &t).await,
        Err(Error::Denied)
    ));
    assert!(
        matches!(s.forecast_goal(Actor::Owner(t.account), &g, &t, &Pace, NOW).await, Ok(Some(value)) if value == NOW+8000)
    );
    let html = s
        .goals_page(
            Actor::Owner(t.account),
            &g,
            &t,
            &Pace,
            PresentationContext {
                language: ProgressLanguage::Polish,
                now_ms: NOW,
            },
        )
        .await
        .expect("goals SSR");
    assert!(html.contains("Prognozowana data"));
    assert!(
        s.goals_page(
            Actor::Staff,
            &g,
            &t,
            &Pace,
            PresentationContext {
                language: ProgressLanguage::English,
                now_ms: NOW
            }
        )
        .await
        .is_err()
    );
    g.target = 0;
    assert!(matches!(
        s.save_goal(Actor::Owner(t.account), &g, &t).await,
        Err(Error::BadInput)
    ));
    g.target = 100;
    g.projection = "13".into();
    assert!(s.goals(Actor::Owner(t.account), &g, &t).await.is_err());
    cleanup(pool, schema).await;
}
