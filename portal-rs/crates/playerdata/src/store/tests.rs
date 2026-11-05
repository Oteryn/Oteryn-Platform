#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use tokio::sync::{OwnedRwLockReadGuard, RwLock};
use tower::ServiceExt;
use uuid::Uuid;

const NOW: u64 = 1793923200000;
const ACCOUNT: &str = "019a0000-0000-7000-8000-000000000001";
const CHARACTER: &str = "019a0000-0000-7000-8000-000000000003";
fn clock() -> TrustedTime {
    TrustedTime {
        unix_ms: NOW,
        uncertainty_ms: 0,
    }
}
fn flags() -> Flags {
    Flags {
        items_ingest: true,
        progress_ingest: true,
        items_read: true,
        history_read: true,
        progress_read: true,
        login_read: true,
        families: format!(
            "{} ITEMS BANK MARKET PLAYER_TRADE",
            playerdata::PROGRESS_FAMILIES
        )
        .split_whitespace()
        .map(str::to_owned)
        .collect(),
    }
}
// Synthetic contract vectors use the schemas in docs/contracts/G-10 and G-11.
fn progress(revision: u64) -> Vec<u8> {
    format!("{{\"contract_version\":1,\"operation\":\"PublishGameJournalProgressV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"character_id\":\"{CHARACTER}\",\"world_id\":\"019a0000-0000-7000-8000-000000000002\",\"projection_revision\":\"12\",\"projection_epoch\":\"1\",\"recovery_generation\":\"3\",\"family\":\"stamina\",\"source_revision\":\"{revision}\",\"record_id\":\"{}\",\"entity_key\":\"stamina\",\"character_revision\":\"9\",\"occurred_at_unix_ms\":\"{NOW}\",\"record_kind\":\"UPSERT\",\"payload\":{{\"stamina_minutes\":2400,\"content_revision\":\"9\",\"build_occurrence_id\":\"019a0000-0000-7000-8000-000000000011\"}}}}",Uuid::now_v7()).into_bytes()
}
fn watermark(through: u64, observed: u64) -> Watermark {
    playerdata::watermark(format!("{{\"contract_version\":1,\"operation\":\"PublishGameJournalProgressWatermarkV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"projection_epoch\":\"1\",\"recovery_generation\":\"3\",\"family\":\"stamina\",\"complete_through\":\"{through}\",\"observed_at_unix_ms\":\"{observed}\",\"baseline_complete\":true}}").as_bytes()).unwrap()
}
fn bank(revision: u64, previous: Option<&str>, before: u64, amount: u64) -> Vec<u8> {
    let previous = previous.map_or_else(|| "null".to_owned(), |x| format!("\"{x}\""));
    format!("{{\"contract_version\":1,\"operation\":\"PublishGameJournalRecordV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"world_id\":\"019a0000-0000-7000-8000-000000000002\",\"character_id\":null,\"character_revision\":null,\"from_projection_revision\":null,\"to_projection_revision\":null,\"projection_epoch\":null,\"family\":\"BANK\",\"recovery_generation\":\"3\",\"source_revision\":\"{revision}\",\"record_id\":\"{}\",\"occurred_at\":\"{}\",\"coverage_start_at\":\"{}\",\"kind\":\"BANK_ENTRY\",\"payload\":{{\"operation_occurrence_id\":\"{}\",\"event_id\":\"{}\",\"entry_id\":\"{}\",\"previous_entry_id\":{previous},\"kind\":\"DEPOSIT\",\"direction\":\"CREDIT\",\"amount\":\"{amount}\",\"balance_before\":\"{before}\",\"balance_after\":\"{}\"}}}}",Uuid::now_v7(),NOW/1000,NOW/1000,Uuid::now_v7(),Uuid::now_v7(),Uuid::now_v7(),before+amount).into_bytes()
}
fn snapshot(revision: u64, entity: &str, end: bool) -> Record {
    let kind = if end { "ITEMS_END" } else { "ITEMS_BEGIN" };
    let payload = if end {
        format!(
            "{{\"snapshot_id\":\"{entity}\",\"page_count\":0,\"item_count\":0,\"pages_sha256\":\"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\"}}"
        )
    } else {
        format!(
            "{{\"snapshot_id\":\"{entity}\",\"character_revision\":\"9\",\"page_count\":0,\"item_count\":0,\"depots\":[],\"houses\":[]}}"
        )
    };
    let bytes = format!(
        "{{\"contract_version\":1,\"operation\":\"PublishGameJournalRecordV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"world_id\":\"019a0000-0000-7000-8000-000000000002\",\"character_id\":\"{CHARACTER}\",\"character_revision\":\"9\",\"from_projection_revision\":\"11\",\"to_projection_revision\":\"12\",\"projection_epoch\":\"1\",\"family\":\"ITEMS\",\"recovery_generation\":\"3\",\"source_revision\":\"{revision}\",\"record_id\":\"{}\",\"occurred_at\":\"{}\",\"coverage_start_at\":\"{}\",\"kind\":\"{kind}\",\"payload\":{payload}}}",
        Uuid::now_v7(),
        NOW / 1000,
        NOW / 1000
    );
    playerdata::items(bytes.as_bytes()).unwrap()
}
#[derive(Debug, Clone, Default)]
struct TestAuthority(Arc<RwLock<Vec<Scope>>>);
#[derive(Debug)]
struct TestLease(OwnedRwLockReadGuard<Vec<Scope>>);
impl Lease for TestLease {
    fn validate(&self, r: &Record) -> Result<()> {
        if self.0.contains(&r.scope) {
            Ok(())
        } else {
            Err(Error::Conflict)
        }
    }
}
impl Authority for TestAuthority {
    fn acquire<'a>(&'a self, s: &'a Scope) -> LeaseFuture<'a> {
        Box::pin(async move {
            let guard = self.0.clone().read_owned().await;
            if !guard.contains(s) {
                return Err(Error::Conflict);
            };
            Ok(Box::new(TestLease(guard)) as Box<dyn Lease>)
        })
    }
}
impl TestAuthority {
    async fn allow(&self, s: Scope) {
        self.0.write().await.push(s);
    }
    async fn fence(&self) {
        self.0.write().await.clear();
    }
}
fn watermark_scope(s: &Scope) -> Scope {
    let mut s = s.clone();
    s.character = None;
    s.world = None;
    s.character_revision = None;
    s.projection_revision = None;
    s
}
#[derive(Debug)]
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn open() -> std::result::Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL journal test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
        let schema = format!("pd_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105230108_t72_private_journal.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn close(self) -> std::result::Result<(), sqlx::Error> {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
    fn store(&self, a: &TestAuthority) -> Store {
        Store::new(self.pool.clone(), Arc::new(a.clone()), flags())
    }
}
#[test]
fn closed_codecs_and_limits() {
    let valid = progress(1);
    assert!(playerdata::progress(&valid).is_ok());
    for bad in [
        String::from_utf8(valid.clone())
            .unwrap()
            .replace("2400", "2521"),
        String::from_utf8(valid.clone())
            .unwrap()
            .replace("\"source_revision\":\"1\"", "\"source_revision\":1"),
        String::from_utf8(valid.clone()).unwrap().replace(
            "\"contract_version\":1",
            "\"contract_version\":1,\"extra\":false",
        ),
        format!("{}\n", String::from_utf8(valid).unwrap()),
    ] {
        assert!(playerdata::progress(bad.as_bytes()).is_err());
    }
    let b = bank(1, None, 0, 100);
    assert!(playerdata::items(&b).is_ok());
    assert!(
        playerdata::items(
            String::from_utf8(b)
                .unwrap()
                .replace("\"balance_after\":\"100\"", "\"balance_after\":\"101\"")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        playerdata::items(br#"{"stream_generation":"019a0000-0000-7000-8000-000000000001"}"#)
            .is_err()
    );
    assert_eq!(clock().now(true), Ok(NOW));
    assert_eq!(
        TrustedTime {
            unix_ms: NOW,
            uncertainty_ms: 5001
        }
        .now(true),
        Err(Error::Unavailable)
    );
}
#[tokio::test]
async fn postgres_progress_replay_watermark_and_owner_denial()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    let scope = r.scope.clone();
    a.allow(scope.clone()).await;
    a.allow(watermark_scope(&scope)).await;
    let store = db.store(&a);
    assert_eq!(
        store.watermark(watermark(1, NOW), clock()).await,
        Err(Error::Conflict)
    );
    let ack = store.ingest(r.clone(), clock()).await?;
    assert_eq!(store.ingest(r.clone(), clock()).await?, ack);
    assert!(matches!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await,
        Err(Error::Unavailable)
    ));
    store.watermark(watermark(1, NOW), clock()).await?;
    let view = store
        .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
        .await?;
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.freshness, Freshness::Fresh);
    for actor in [
        Actor::Anonymous,
        Actor::Staff,
        Actor::Owner(Uuid::now_v7().to_string()),
    ] {
        assert!(matches!(
            store.read(&actor, &scope, clock()).await,
            Err(Error::Denied)
        ));
    }
    let changed = String::from_utf8(r.bytes.clone())?.replace("2400", "2399");
    assert_eq!(
        store
            .ingest(playerdata::progress(changed.as_bytes())?, clock())
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store
            .ingest(playerdata::progress(&progress(3))?, clock())
            .await,
        Err(Error::Conflict)
    );
    let c: String = sqlx::query_scalar("SELECT stored_through::text FROM pd_streams")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(c, "1");
    a.fence().await;
    assert!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await
            .is_err()
    );
    assert!(store.ingest(r, clock()).await.is_err());
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_concurrent_retry_and_restart()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    let (one, two) = tokio::join!(
        store.ingest(r.clone(), clock()),
        store.ingest(r.clone(), clock())
    );
    assert_eq!(one?, two?);
    let restarted = db.store(&a);
    assert!(restarted.ingest(r, clock()).await.is_ok());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_bank_chain_and_retention() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::items(&bank(1, None, 0, 100))?;
    let scope = r.scope.clone();
    a.allow(scope.clone()).await;
    let store = db.store(&a);
    store.ingest(r.clone(), clock()).await?;
    let invalid = playerdata::items(&bank(2, Some(&Uuid::now_v7().to_string()), 100, 100))?;
    assert_eq!(store.ingest(invalid, clock()).await, Err(Error::Conflict));
    let second = playerdata::items(&bank(2, Some(&r.entity), 100, 100))?;
    store.ingest(second, clock()).await?;
    assert_eq!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await?
            .records
            .len(),
        2
    );
    let later = TrustedTime {
        unix_ms: NOW + 31 * 86400000,
        uncertainty_ms: 0,
    };
    assert_eq!(store.expire(later).await?, 2);
    assert_eq!(store.ingest(r.clone(), later).await?, acknowledgement(&r)?);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    let pruned = TrustedTime {
        unix_ms: NOW + 62 * 86400000,
        uncertainty_ms: 0,
    };
    store.expire(pruned).await?;
    assert_eq!(store.ingest(r, pruned).await, Err(Error::Conflict));
    let (cursor, last) = store.cursor(&scope).await?;
    assert_eq!(cursor.stored, 2);
    assert!(last.is_some());
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_snapshot_commit_and_sale_fence()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let entity = Uuid::now_v7().to_string();
    let begin = snapshot(1, &entity, false);
    let scope = begin.scope.clone();
    a.allow(scope.clone()).await;
    let store = db.store(&a);
    store.ingest(begin, clock()).await?;
    assert!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await
            .is_err()
    );
    let end = snapshot(2, &entity, true);
    let bad = String::from_utf8(end.bytes.clone())?.replace("e3b0c442", "03b0c442");
    assert_eq!(
        store
            .ingest(playerdata::items(bad.as_bytes())?, clock())
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(store.cursor(&scope).await?.0.stored, 1);
    store.ingest(end.clone(), clock()).await?;
    assert_eq!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await?
            .records
            .len(),
        2
    );
    a.fence().await;
    store.erase_character(ACCOUNT, CHARACTER, clock()).await?;
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(remaining, 0);
    assert!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
            .await
            .is_err()
    );
    assert_eq!(store.ingest(end, clock()).await, Err(Error::Conflict));
    let mut buyer = scope.clone();
    buyer.account = Uuid::now_v7().to_string();
    buyer.character_revision = Some("10".into());
    buyer.projection_revision = Some("13".into());
    a.allow(buyer.clone()).await;
    assert!(
        store
            .read(&Actor::Owner(buyer.account.clone()), &buyer, clock())
            .await
            .is_err()
    );
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_erasure_and_unknown_restore()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(r.clone(), clock()).await?;
    let unknown = String::from_utf8(r.bytes.clone())?.replace(
        "\"recovery_generation\":\"3\"",
        "\"recovery_generation\":\"4\"",
    );
    assert_eq!(
        store
            .ingest(playerdata::progress(unknown.as_bytes())?, clock())
            .await,
        Err(Error::Conflict)
    );
    store.erase_account(ACCOUNT, clock()).await?;
    assert_eq!(store.ingest(r, clock()).await, Err(Error::Conflict));
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_watermark_prefix_staleness_and_regression()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    let scope = r.scope.clone();
    a.allow(scope.clone()).await;
    a.allow(watermark_scope(&scope)).await;
    let store = db.store(&a);
    store.ingest(r, clock()).await?;
    store.watermark(watermark(1, NOW), clock()).await?;
    store
        .ingest(playerdata::progress(&progress(2))?, clock())
        .await?;
    let view = store
        .read(&Actor::Owner(ACCOUNT.into()), &scope, clock())
        .await?;
    assert_eq!(view.through, 1);
    assert_eq!(view.records.len(), 1);
    let stale = TrustedTime {
        unix_ms: NOW + 11000,
        uncertainty_ms: 0,
    };
    assert_eq!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, stale)
            .await?
            .freshness,
        Freshness::Stale
    );
    store.watermark(watermark(2, NOW), clock()).await?;
    assert_eq!(
        store.watermark(watermark(1, NOW), clock()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store.watermark(watermark(2, NOW - 1), clock()).await,
        Err(Error::Conflict)
    );
    db.close().await?;
    Ok(())
}
#[derive(Debug)]
struct TestClock;
impl http::Clock for TestClock {
    fn trusted_time(&self) -> Result<TrustedTime> {
        Ok(clock())
    }
}
#[tokio::test]
async fn http_disabled_peer_bad_input_and_no_store()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let app = http::configured_router(pool.clone());
    let response = app
        .oneshot(Request::post("/internal/v1/playerdata/progress/record").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let identity = Identity::san("journal-test")?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: "private-journal".into(),
    }])?);
    let state = http::HttpState::new(
        Store::new(pool, Arc::new(UnavailableAuthority), flags()),
        Arc::new(TestClock),
    );
    let app = http::router(state, identities);
    let response = app
        .clone()
        .oneshot(
            Request::post("/internal/v1/playerdata/progress/record")
                .header("content-type", "application/json")
                .body(Body::from(progress(1)))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(to_bytes(response.into_body(), 64).await?.is_empty());
    let mut request = Request::post("/internal/v1/playerdata/progress/record")
        .header("content-type", "application/json")
        .body(Body::from("{}"))?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(identity));
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

fn changed_field(v: &mut Value, k: &str, value: Value) {
    let Value::Object(rows) = v else {
        unreachable!()
    };
    rows.iter_mut().find(|(name, _)| name == k).unwrap().1 = value;
}
fn family_bytes(family: &str, entity: &str, payload: &str, account: bool) -> Vec<u8> {
    let mut value = canonical::decode(&progress(1), 262144, 9).unwrap();
    changed_field(&mut value, "family", Value::String(family.into()));
    changed_field(&mut value, "entity_key", Value::String(entity.into()));
    changed_field(
        &mut value,
        "payload",
        canonical::decode(payload.as_bytes(), 262144, 9).unwrap(),
    );
    if account {
        for name in [
            "character_id",
            "world_id",
            "character_revision",
            "projection_revision",
        ] {
            changed_field(&mut value, name, Value::Null);
        }
        if family == "premium_end" {
            changed_field(
                &mut value,
                "projection_revision",
                Value::String("12".into()),
            );
        }
    }
    canonical::encode(&value).unwrap()
}
#[test]
fn all_defined_progress_families_have_closed_payloads() {
    let cases = [
        (
            "bestiary",
            "race:rat",
            r#"{"race_key":"race:rat","race_definition_revision":"v1","kill_count":0,"final_kill_threshold":1,"charm_points_awarded":0}"#,
            false,
        ),
        (
            "boss_cooldown",
            "boss:rat",
            r#"{"boss_key":"boss:rat","definition_revision":"v1","kill_count":"0","available_at_unix_ms":null,"last_death_occurrence_id":null}"#,
            false,
        ),
        (
            "quest",
            "quest:test",
            r#"{"quest_key":"quest:test","pinned_content_revision":"v1","definition_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":false,"tracks":[{"track_key":"track:one","value":"-1"}]}"#,
            false,
        ),
        (
            "achievement",
            "oteryn:achievement/test",
            r#"{"achievement_key":"oteryn:achievement/test","definition_revision":"v1","progress":0,"target":1,"granted_at_unix_ms":null}"#,
            false,
        ),
        (
            "achievement",
            "oteryn:achievement/test",
            r#"{"achievement_key":"oteryn:achievement/test","definition_revision":"v1","progress":null,"target":null,"granted_at_unix_ms":"0"}"#,
            true,
        ),
        (
            "skill_history",
            "019a0000-0000-7000-8000-000000000011",
            r#"{"build_occurrence_id":"019a0000-0000-7000-8000-000000000011","cause":"logout","policy_revision":"v1","skills":[{"skill":"magic","level_before":0,"progress_before":"0","level_after":1,"progress_after":"0"}]}"#,
            false,
        ),
        (
            "death_recap",
            "019a0000-0000-7000-8000-000000000011",
            r#"{"death_occurrence_id":"019a0000-0000-7000-8000-000000000011","level_before":2,"level_after":1,"experience_lost":"1","blessings_before":[],"blessings_after":[],"lost_items":[],"recap_state":"UNAVAILABLE","window_ms":0,"actions":[]}"#,
            false,
        ),
        (
            "pvp",
            "019a0000-0000-7000-8000-000000000011",
            r#"{"death_occurrence_id":"019a0000-0000-7000-8000-000000000011","role":"VICTIM","counterpart_character_id":null,"justification":"UNKNOWN","points_milli":null}"#,
            false,
        ),
        (
            "skull",
            "skull",
            r#"{"skull":"NONE","skull_until_unix_ms":null,"kill_block_until_unix_ms":null}"#,
            false,
        ),
        (
            "premium_end",
            "premium",
            r#"{"observed_state":"EXPIRED","effective_until_unix_us":"0","authority_revision":"12"}"#,
            true,
        ),
        (
            "game_login",
            "019a0000-0000-7000-8000-000000000011",
            r#"{"login_event_id":"019a0000-0000-7000-8000-000000000011","entered_at":"0","left_at":null,"end_reason":null}"#,
            true,
        ),
    ];
    for (family, entity, payload, account) in cases {
        let raw = family_bytes(family, entity, payload, account);
        assert!(playerdata::progress(&raw).is_ok(), "{family}");
        let mut value = canonical::decode(&raw, 262144, 9).unwrap();
        let Value::Object(rows) = get_payload_mut(&mut value) else {
            unreachable!()
        };
        rows.push(("unexpected".into(), Value::Bool(false)));
        assert!(
            playerdata::progress(&canonical::encode(&value).unwrap()).is_err(),
            "{family}"
        );
    }
    let invalid = family_bytes("house_rent", "house:test", r#"{}"#, false);
    assert_eq!(
        playerdata::progress(&invalid).unwrap_err(),
        playerdata::Error::Unavailable
    );
}
fn get_payload_mut(v: &mut Value) -> &mut Value {
    let Value::Object(rows) = v else {
        unreachable!()
    };
    &mut rows
        .iter_mut()
        .find(|(name, _)| name == "payload")
        .unwrap()
        .1
}
#[tokio::test]
async fn postgres_http_ack_cursor_conflict_and_length()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let raw = bank(1, None, 0, 100);
    let r = playerdata::items(&raw)?;
    a.allow(r.scope.clone()).await;
    let identity = Identity::san("journal-test")?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: "private-journal".into(),
    }])?);
    let app = http::router(
        http::HttpState::new(db.store(&a), Arc::new(TestClock)),
        identities,
    );
    let request = |body: Vec<u8>| {
        let mut request = Request::post("/internal/v1/playerdata/journal/records")
            .header("content-type", "application/json")
            .header("content-length", body.len())
            .body(Body::from(body))
            .unwrap();
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(identity.clone()));
        request
    };
    for _ in 0..2 {
        let response = app.clone().oneshot(request(raw.clone())).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            to_bytes(response.into_body(), 64).await?.as_ref(),
            b"{\"contract_version\":1,\"result\":\"accepted\"}"
        );
    }
    let changed = String::from_utf8(raw.clone())?
        .replace("\"amount\":\"100\"", "\"amount\":\"101\"")
        .replace("\"balance_after\":\"100\"", "\"balance_after\":\"101\"");
    let response = app.clone().oneshot(request(changed.into_bytes())).await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let mut mismatch = request(raw);
    mismatch
        .headers_mut()
        .insert("content-length", "1".parse()?);
    assert_eq!(
        app.oneshot(mismatch).await?.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db.store(&a).cursor(&r.scope).await?.0.stored, 1);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_transfer_writer_serializes_without_lock_inversion()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    let mut blocker = db.pool.begin().await?;
    lock(&mut blocker, &r.scope.authority).await?;
    let pending = tokio::spawn({
        let store = store.clone();
        let r = r.clone();
        async move { store.ingest(r, clock()).await }
    });
    // Ingest must acquire its authority lease before waiting for the journal lock.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if a.0.try_write().is_err() {
                break;
            };
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let writer = tokio::spawn({
        let a = a.clone();
        let store = store.clone();
        async move {
            let mut fence = a.0.write().await;
            fence.clear();
            store.erase_character(ACCOUNT, CHARACTER, clock()).await
        }
    });
    blocker.commit().await?;
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        pending.await??;
        writer.await??;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    assert!(store.ingest(r, clock()).await.is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}
