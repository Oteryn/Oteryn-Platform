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
    progress_for(revision, CHARACTER)
}
fn progress_for(revision: u64, character: &str) -> Vec<u8> {
    format!("{{\"contract_version\":1,\"operation\":\"PublishGameJournalProgressV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"character_id\":\"{character}\",\"world_id\":\"019a0000-0000-7000-8000-000000000002\",\"projection_revision\":\"12\",\"projection_epoch\":\"1\",\"recovery_generation\":\"3\",\"family\":\"stamina\",\"source_revision\":\"{revision}\",\"record_id\":\"{}\",\"entity_key\":\"stamina\",\"character_revision\":\"9\",\"occurred_at_unix_ms\":\"{NOW}\",\"record_kind\":\"UPSERT\",\"payload\":{{\"stamina_minutes\":2400,\"content_revision\":\"9\",\"build_occurrence_id\":\"019a0000-0000-7000-8000-000000000011\"}}}}",Uuid::now_v7()).into_bytes()
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
    fn fence<'a>(&'a self, _: &'a mut sqlx::PgConnection, _: &'a Scope) -> FenceFuture<'a> {
        Box::pin(async { Ok(Admission::Owner) })
    }
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
        // The whole schema: the journal migrations plus T72 admission's
        // `pd_streams.watermark_recovery`, which needs the character tables.
        oteryn_db::migrate(&pool)
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
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
    let invalid = playerdata::items(&bank(2, Some(&r.entity), 99, 100))?;
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
    assert_eq!(
        store.ingest(r.clone(), clock()).await?,
        acknowledgement(&r)?
    );
    let later = TrustedTime {
        unix_ms: NOW + 31 * 86400000,
        uncertainty_ms: 0,
    };
    assert_eq!(store.expire(later).await?, 2);
    assert_eq!(store.ingest(r.clone(), later).await, Err(Error::Conflict));
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
/// One purpose bound to the authority of the synthetic vectors above; binding
/// both to one authority would be an overlap that disables both.
fn sources(progress: bool) -> http::Sources {
    let (items, progress) = if progress {
        ("", "oteryn:journal:primary")
    } else {
        ("oteryn:journal:primary", "")
    };
    http::Sources {
        items: items.into(),
        progress: progress.into(),
    }
}
#[derive(Debug)]
struct TestClock;
impl http::Clock for TestClock {
    fn trusted_time(&self) -> http::ClockFuture<'_> {
        Box::pin(async { Ok(clock()) })
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
        group: http::PROGRESS_GROUP.into(),
    }])?);
    let state = http::HttpState::new(
        Store::new(pool, Arc::new(UnavailableAuthority), flags()),
        Arc::new(TestClock),
    )
    .with_sources(sources(true));
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
    let occurred_in = |body, seconds: u64| {
        String::from_utf8(body)
            .unwrap()
            .replacen(
                &(NOW / 1000).to_string(),
                &(NOW / 1000 + seconds).to_string(),
                1,
            )
            .into_bytes()
    };
    let raw = occurred_in(bank(1, None, 0, 100), 2);
    let r = playerdata::items(&raw)?;
    a.allow(r.scope.clone()).await;
    let identity = Identity::san("journal-test")?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: http::ITEMS_GROUP.into(),
    }])?);
    let app = http::router(
        http::HttpState::new(db.store(&a), Arc::new(TestClock)).with_sources(sources(false)),
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
    let too_far = occurred_in(bank(2, Some(&r.entity), 100, 50), 31);
    assert_eq!(
        app.clone().oneshot(request(too_far)).await?.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db.store(&a).cursor(&r.scope).await?.0.stored, 1);
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

/// Revision 3 and 4 (G-10 retention, G-11 §8.2): a deleted scope refuses a
/// first delivery even while the authority adapter still allows it, answers an
/// exact replay with its stored answer, and keeps only a content-free digest
/// that expires after 120 days.
#[tokio::test]
async fn postgres_deleted_scope_refuses_replay_and_first_delivery()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let first = playerdata::progress(&progress(1))?;
    let scope = first.scope.clone();
    a.allow(scope.clone()).await;
    let store = db.store(&a);
    store.ingest(first.clone(), clock()).await?;
    let bank_first = playerdata::items(&bank(1, None, 0, 100))?;
    a.allow(bank_first.scope.clone()).await;
    store.ingest(bank_first.clone(), clock()).await?;

    store.erase_character(ACCOUNT, CHARACTER, clock()).await?;
    // Revision 4: an exact replay of a record accepted before the deletion
    // returns its stored `accepted` answer, nothing more.
    assert_eq!(
        store.ingest(first.clone(), clock()).await?,
        acknowledgement(&first)?
    );
    // A first delivery of the deleted character on the account-wide G-11 stream
    // reaches the tombstone check, is refused terminally and consumes its
    // revision (G-11 §8.2 terminal refusals, revision 4); it is never inserted.
    let second = playerdata::progress(&progress(2))?;
    assert_eq!(
        store.ingest(second.clone(), clock()).await?,
        refusal(&second, "CHARACTER_SCOPE_DELETED")
    );
    assert_eq!(store.cursor(&scope).await?.0.stored, 2);
    // Account-scoped bank history does not follow the character.
    assert_eq!(
        store.ingest(bank_first.clone(), clock()).await?,
        b"{\"contract_version\":1,\"result\":\"accepted\"}".to_vec()
    );

    store.erase_account(ACCOUNT, clock()).await?;
    // A stream the account never used before is inside the deleted scope too.
    let fresh = String::from_utf8(bank(1, None, 0, 100))?.replace(
        "\"recovery_generation\":\"3\"",
        "\"recovery_generation\":\"4\"",
    );
    let fresh = playerdata::items(fresh.as_bytes())?;
    a.allow(fresh.scope.clone()).await;
    assert_eq!(store.ingest(fresh, clock()).await, Err(Error::Conflict));
    let streams: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_streams")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(streams, 2);

    let tombstones: Vec<i32> =
        sqlx::query_scalar("SELECT octet_length(scope_digest) FROM pd_deleted_scopes")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(tombstones, vec![32, 32]);
    store
        .expire(TrustedTime {
            unix_ms: NOW + 119 * 86400000,
            uncertainty_ms: 0,
        })
        .await?;
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_deleted_scopes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(kept, 2);
    store
        .expire(TrustedTime {
            unix_ms: NOW + 121 * 86400000,
            uncertainty_ms: 0,
        })
        .await?;
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_deleted_scopes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(kept, 0);
    db.close().await?;
    Ok(())
}

fn watermark_for(family: &str, through: u64) -> Watermark {
    playerdata::watermark(format!("{{\"contract_version\":1,\"operation\":\"PublishGameJournalProgressWatermarkV1\",\"source_authority\":\"oteryn:journal:primary\",\"account_id\":\"{ACCOUNT}\",\"projection_epoch\":\"1\",\"recovery_generation\":\"3\",\"family\":\"{family}\",\"complete_through\":\"{through}\",\"observed_at_unix_ms\":\"{NOW}\",\"baseline_complete\":true}}").as_bytes()).unwrap()
}
/// The backend waiting on `holder`, polled until it appears, `done` or a deadline.
async fn blocked_by(
    admin: &PgPool,
    holder: i32,
    done: impl Fn() -> bool,
) -> std::result::Result<Option<i32>, sqlx::Error> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let pid: Option<i32> = sqlx::query_scalar(
            "SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)) LIMIT 1",
        )
        .bind(holder)
        .fetch_optional(admin)
        .await?;
        if pid.is_some() || done() || tokio::time::Instant::now() > deadline {
            return Ok(pid);
        }
        tokio::task::yield_now().await;
    }
}
async fn streams(db: &Db) -> std::result::Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM pd_streams")
        .fetch_one(&db.pool)
        .await
}

/// G-10/G-11 §8.2: a watermark or cursor into a deleted scope is refused like a
/// record, and a watermark never creates a stream for a deleted account.
#[tokio::test]
async fn postgres_watermark_and_cursor_refuse_a_deleted_scope()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let store = db.store(&a);
    let begin = snapshot(1, &Uuid::now_v7().to_string(), false);
    let items = begin.scope.clone();
    a.allow(items.clone()).await;
    let first = playerdata::progress(&progress(1))?;
    let progress_scope = first.scope.clone();
    a.allow(progress_scope.clone()).await;
    a.allow(watermark_scope(&progress_scope)).await;
    let mut skull = watermark_scope(&progress_scope);
    skull.family = "skull".into();
    a.allow(skull).await;
    let bank_first = playerdata::items(&bank(1, None, 0, 100))?;
    a.allow(bank_first.scope.clone()).await;
    store.ingest(begin, clock()).await?;
    store.ingest(first.clone(), clock()).await?;
    store.ingest(bank_first.clone(), clock()).await?;
    assert!(
        store
            .watermark(watermark_for("stamina", 1), clock())
            .await
            .is_ok()
    );
    assert_eq!(store.cursor(&items).await?.0.stored, 1);

    // Erased character: its G-10 ITEMS stream cursor is closed.
    store.erase_character(ACCOUNT, CHARACTER, clock()).await?;
    assert_eq!(store.cursor(&items).await, Err(Error::Conflict));

    // Erased account: an exact replay, a cursor and a watermark on a stream the
    // account never used are all refused, and no stream row is created.
    store.erase_account(ACCOUNT, clock()).await?;
    let before = streams(&db).await?;
    assert_eq!(store.ingest(first, clock()).await, Err(Error::Conflict));
    assert_eq!(
        store.ingest(bank_first.clone(), clock()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store.watermark(watermark_for("stamina", 1), clock()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store.watermark(watermark_for("skull", 0), clock()).await,
        Err(Error::Conflict)
    );
    assert_eq!(store.cursor(&bank_first.scope).await, Err(Error::Conflict));
    assert_eq!(streams(&db).await?, before);
    db.close().await?;
    Ok(())
}

/// Erasure and a first delivery of an authority the account has no stream for
/// yet serialize: whichever commits first, the account ends deleted with no
/// readable record. The external transaction holds the new stream row so the
/// delivery has passed its tombstone check when the erasure starts.
#[tokio::test]
async fn postgres_erasure_and_first_delivery_serialize()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    let mut blocker = db.pool.begin().await?;
    sqlx::query("INSERT INTO pd_streams(stream_key,authority,account_id,family,progress) VALUES($1,$2,$3,$4,true)")
        .bind(stream_key(&r.scope))
        .bind(&r.scope.authority)
        .bind(&r.scope.account)
        .bind(&r.scope.family)
        .execute(&mut *blocker)
        .await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    let ingest = tokio::spawn({
        let store = store.clone();
        let r = r.clone();
        async move { store.ingest(r, clock()).await }
    });
    // The delivery passed its tombstone check and waits on the held stream row.
    let ingest_pid = blocked_by(&db.admin, blocker_pid, || false).await?;
    assert!(ingest_pid.is_some());
    let erase = tokio::spawn({
        let store = store.clone();
        async move { store.erase_account(ACCOUNT, clock()).await }
    });
    // Serialized, the erasure waits behind the delivery; otherwise it finishes.
    if let Some(pid) = ingest_pid {
        blocked_by(&db.admin, pid, || erase.is_finished()).await?;
    }
    blocker.rollback().await?;
    let _ = ingest.await?;
    erase.await??;
    let readable: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(readable, 0);
    let open: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_streams WHERE NOT erased")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(open, 0);
    assert_eq!(store.ingest(r, clock()).await, Err(Error::Conflict));
    db.close().await?;
    Ok(())
}

/// Revision 3 §8.2 tombstone lifetime: a tombstone is kept until exactly 120
/// days after deletion, and a repeated erasure only ever extends it.
#[tokio::test]
async fn postgres_tombstones_expire_at_120_days_and_double_erase_extends()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let store = db.store(&a);
    let at = |ms: u64| TrustedTime {
        unix_ms: ms,
        uncertainty_ms: 0,
    };
    let day = 86400000;
    store.erase_character(ACCOUNT, CHARACTER, at(NOW)).await?;
    store.erase_account(ACCOUNT, at(NOW + 5 * day)).await?;
    // An erasure recorded with an earlier time never shortens the lifetime.
    store.erase_account(ACCOUNT, at(NOW)).await?;
    let kept = |db: &Db| {
        let pool = db.pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pd_deleted_scopes")
                .fetch_one(&pool)
                .await
        }
    };
    for (now, expected) in [
        (NOW + 120 * day - 1, 2),
        (NOW + 120 * day, 1),
        (NOW + 125 * day - 1, 1),
        (NOW + 125 * day, 0),
    ] {
        store.expire(at(now)).await?;
        assert_eq!(kept(&db).await?, expected, "{now}");
    }
    db.close().await?;
    Ok(())
}

const OTHER: &str = "019a0000-0000-7000-8000-000000000004";
/// The stream row and the counts of records and conflicts: the whole durable
/// state one G-11 delivery can change.
async fn state(db: &Db) -> std::result::Result<(String, i64, i64), sqlx::Error> {
    let stream: String = sqlx::query_scalar(
        "SELECT concat_ws('/',highest_seen,stored_through,applied_through,last_record_id) FROM pd_streams",
    )
    .fetch_one(&db.pool)
    .await?;
    let records: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    let conflicts: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_conflicts")
        .fetch_one(&db.pool)
        .await?;
    Ok((stream, records, conflicts))
}
async fn stored(db: &Db) -> std::result::Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT stored_through::text FROM pd_streams")
        .fetch_one(&db.pool)
        .await
}
/// A deleted character, another live character of the same account, and the
/// deleted character's revision 1 already refused on the account-wide stream.
async fn refused_first(
    db: &Db,
) -> std::result::Result<(TestAuthority, Store, Record), Box<dyn std::error::Error>> {
    let a = TestAuthority::default();
    let refused = playerdata::progress(&progress(1))?;
    a.allow(refused.scope.clone()).await;
    let other = playerdata::progress(&progress_for(1, OTHER))?;
    a.allow(other.scope.clone()).await;
    a.allow(watermark_scope(&other.scope)).await;
    let store = db.store(&a);
    store.erase_character(ACCOUNT, CHARACTER, clock()).await?;
    assert_eq!(
        store.ingest(refused.clone(), clock()).await?,
        refusal(&refused, "CHARACTER_SCOPE_DELETED")
    );
    Ok((a, store, refused))
}

/// Option A: a G-11 record of a deleted character is a terminal refusal that
/// consumes its revision, so the account-wide stream keeps flowing; an exact
/// replay answers the same bytes without a write, other bytes stay a conflict.
#[tokio::test]
async fn postgres_terminal_refusal_consumes_its_revision()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (_a, store, refused) = refused_first(&db).await?;
    assert_eq!(
        refusal(&refused, "CHARACTER_SCOPE_DELETED"),
        format!(
            "{{\"contract_version\":1,\"result\":\"refused\",\"record_id\":\"{}\",\"consumed_source_revision\":\"1\",\"reason\":\"CHARACTER_SCOPE_DELETED\"}}",
            refused.id
        )
        .into_bytes()
    );
    let mut last = None;
    for revision in 2..=4 {
        let r = playerdata::progress(&progress_for(revision, OTHER))?;
        assert_eq!(
            store.ingest(r.clone(), clock()).await?,
            acknowledgement(&r)?
        );
        last = Some(r);
    }
    let last = last.ok_or("no record")?;
    let (stream, records, conflicts) = state(&db).await?;
    assert_eq!(stream, format!("4/4/4/{}", last.id));
    assert_eq!((records, conflicts), (4, 0));
    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM pd_records ORDER BY source_revision")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(
        kinds,
        ["CHARACTER_SCOPE_DELETED", "UPSERT", "UPSERT", "UPSERT"]
    );
    // The refusal row is content-free: ids, revision, digest and reason only.
    let row = sqlx::query("SELECT record_id,entity_key,character_id,scope_key,occurred_ms,coverage_ms,payload IS NULL AS empty,digest FROM pd_records WHERE source_revision=1")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(row.try_get::<String, _>("record_id")?, refused.id);
    assert_eq!(row.try_get::<String, _>("entity_key")?, "");
    assert_eq!(row.try_get::<Option<String>, _>("character_id")?, None);
    assert_eq!(row.try_get::<String, _>("scope_key")?, "");
    assert_eq!(row.try_get::<i64, _>("occurred_ms")?, 0);
    assert_eq!(row.try_get::<i64, _>("coverage_ms")?, 0);
    assert!(row.try_get::<bool, _>("empty")?);
    assert_eq!(row.try_get::<Vec<u8>, _>("digest")?, hash(&refused.bytes));
    // The consumed revision counts as applied: a watermark may cover it.
    store
        .watermark(watermark(4, NOW), clock())
        .await
        .map(|_| ())?;

    // Exact replay: identical bytes, nothing written.
    let before = state(&db).await?;
    for _ in 0..2 {
        assert_eq!(
            store.ingest(refused.clone(), clock()).await?,
            refusal(&refused, "CHARACTER_SCOPE_DELETED")
        );
    }
    assert_eq!(state(&db).await?, before);

    // Other bytes at the refused revision: same record id, or another one.
    let changed = String::from_utf8(refused.bytes.clone())?.replace("2400", "2399");
    assert_eq!(
        store
            .ingest(playerdata::progress(changed.as_bytes())?, clock())
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store
            .ingest(playerdata::progress(&progress(1))?, clock())
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(state(&db).await?.0, before.0);
    db.close().await?;
    Ok(())
}

/// Option B, gap: a missing revision blocks the stream, also for a record of the
/// deleted character; nothing advances.
#[tokio::test]
async fn postgres_gap_after_terminal_refusal_does_not_advance()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (_a, store, _) = refused_first(&db).await?;
    let before = state(&db).await?;
    for character in [CHARACTER, OTHER] {
        assert_eq!(
            store
                .ingest(playerdata::progress(&progress_for(3, character))?, clock())
                .await,
            Err(Error::Conflict)
        );
    }
    assert_eq!(state(&db).await?, before);
    assert_eq!(stored(&db).await?, "1");
    db.close().await?;
    Ok(())
}

/// Option B, revision outside the window: an already consumed revision under a
/// new record id, and the highest representable revision, are both refused.
#[tokio::test]
async fn postgres_revision_outside_window_does_not_advance()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (_a, store, _) = refused_first(&db).await?;
    let before = state(&db).await?;
    for (revision, character) in [
        (1, OTHER),
        (1, CHARACTER),
        (u64::MAX, OTHER),
        (u64::MAX, CHARACTER),
    ] {
        assert_eq!(
            store
                .ingest(
                    playerdata::progress(&progress_for(revision, character))?,
                    clock()
                )
                .await,
            Err(Error::Conflict),
            "{revision} {character}"
        );
    }
    assert_eq!(state(&db).await?.0, before.0);
    assert_eq!(stored(&db).await?, "1");
    db.close().await?;
    Ok(())
}

/// Option B, validation: noncanonical bytes and a future occurrence are refused
/// before any lock, for the deleted character too; nothing advances.
#[tokio::test]
async fn postgres_invalid_record_does_not_advance()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (_a, store, _) = refused_first(&db).await?;
    let before = state(&db).await?;
    for character in [CHARACTER, OTHER] {
        let mut noncanonical = playerdata::progress(&progress_for(2, character))?;
        noncanonical.bytes.push(b'\n');
        assert_eq!(
            store.ingest(noncanonical, clock()).await,
            Err(Error::Invalid)
        );
        let future = String::from_utf8(progress_for(2, character))?.replace(
            &format!("\"occurred_at_unix_ms\":\"{NOW}\""),
            &format!("\"occurred_at_unix_ms\":\"{}\"", NOW + 1),
        );
        assert_eq!(
            store
                .ingest(playerdata::progress(future.as_bytes())?, clock())
                .await,
            Err(Error::Invalid)
        );
    }
    assert_eq!(state(&db).await?, before);
    db.close().await?;
    Ok(())
}

/// Option B, transient database error: unavailable, never a refusal or advance.
#[tokio::test]
async fn postgres_transient_database_error_does_not_advance()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (a, _, _) = refused_first(&db).await?;
    let before = state(&db).await?;
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let down = Store::new(unreachable, Arc::new(a), flags());
    for character in [CHARACTER, OTHER] {
        assert_eq!(
            down.ingest(playerdata::progress(&progress_for(2, character))?, clock())
                .await,
            Err(Error::Unavailable)
        );
    }
    assert_eq!(state(&db).await?, before);
    db.close().await?;
    Ok(())
}

/// Account deletion is not a terminal refusal: it closes the whole account
/// stream, so every later record is refused without advance (G-11 §8.2).
#[tokio::test]
async fn postgres_deleted_account_closes_the_stream_without_advance()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let (_a, store, refused) = refused_first(&db).await?;
    store.erase_account(ACCOUNT, clock()).await?;
    let before = state(&db).await?;
    for character in [CHARACTER, OTHER] {
        assert_eq!(
            store
                .ingest(playerdata::progress(&progress_for(2, character))?, clock())
                .await,
            Err(Error::Conflict)
        );
    }
    assert_eq!(store.ingest(refused, clock()).await, Err(Error::Conflict));
    assert_eq!(state(&db).await?, before);
    db.close().await?;
    Ok(())
}

/// Revision 4, lost response: accept, lose the answer, delete the character,
/// resend the same bytes. A conforming adapter still grants the lease for the
/// deleted character (the tombstone is the store's check), and the replay is
/// answered from the stored record before the tombstone check, so it stays
/// `accepted`; it writes nothing and advances nothing.
#[tokio::test]
async fn postgres_lost_response_replay_after_deletion_stays_accepted()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let first = playerdata::progress(&progress(1))?;
    let scope = first.scope.clone();
    a.allow(scope.clone()).await;
    let store = db.store(&a);
    let answer = store.ingest(first.clone(), clock()).await?;
    store.erase_character(ACCOUNT, CHARACTER, clock()).await?;
    assert_eq!(store.ingest(first.clone(), clock()).await?, answer);
    assert_eq!(answer, acknowledgement(&first)?);
    assert_eq!(store.cursor(&scope).await?.0.stored, 1);
    // Different bytes under the same record are still not a replay.
    let other = playerdata::progress(&progress(1))?;
    assert_eq!(store.ingest(other, clock()).await, Err(Error::Conflict));
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_bank_continues_after_payload_and_metadata_expiry()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let first = playerdata::items(&bank(1, None, 0, 100))?;
    a.allow(first.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(first.clone(), clock()).await?;
    for (seq, days, prior) in [(2, 31, first.entity)] {
        let time = TrustedTime {
            unix_ms: NOW + days * 86400000,
            uncertainty_ms: 0,
        };
        store.expire(time).await?;
        let fresh = |before| -> std::result::Result<Record, Box<dyn std::error::Error>> {
            let raw = String::from_utf8(bank(seq, Some(&prior), before, 100))?.replace(
                &format!("\"occurred_at\":\"{}\"", NOW / 1000),
                &format!("\"occurred_at\":\"{}\"", time.unix_ms / 1000),
            );
            Ok(playerdata::items(raw.as_bytes())?)
        };
        assert_eq!(store.ingest(fresh(99)?, time).await, Err(Error::Conflict));
        let second = fresh(100)?;
        store.ingest(second.clone(), time).await?;
        let later = TrustedTime {
            unix_ms: NOW + 100 * 86400000,
            uncertainty_ms: 0,
        };
        store.expire(later).await?;
        let third = |before| -> std::result::Result<Record, Box<dyn std::error::Error>> {
            let raw = String::from_utf8(bank(3, Some(&second.entity), before, 100))?.replace(
                &format!("\"occurred_at\":\"{}\"", NOW / 1000),
                &format!("\"occurred_at\":\"{}\"", later.unix_ms / 1000),
            );
            Ok(playerdata::items(raw.as_bytes())?)
        };
        assert_eq!(store.ingest(third(199)?, later).await, Err(Error::Conflict));
        store.ingest(third(200)?, later).await?;
    }
    db.close().await?;
    eprintln!(
        "PG postgres_bank_continues_after_payload_and_metadata_expiry {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_production_retention_dispatch_erases_payload_preserves_replay()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    use oteryn_db::outbox::Cleanup;
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await?;
    let old = now as u64 - 31 * 86400000;
    let time = TrustedTime {
        unix_ms: old,
        uncertainty_ms: 0,
    };
    let r = playerdata::progress(
        String::from_utf8(progress(1))?
            .replace(&NOW.to_string(), &old.to_string())
            .as_bytes(),
    )?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(r.clone(), time).await?;
    for revision in 2..=513 {
        let raw =
            String::from_utf8(progress(revision))?.replace(&NOW.to_string(), &old.to_string());
        store
            .ingest(playerdata::progress(raw.as_bytes())?, time)
            .await?;
    }
    // Supersession retention starts at publication, so publish the old
    // replacement prefix before exercising the bounded cleanup dispatch.
    a.allow(watermark_scope(&r.scope)).await;
    store.watermark(watermark(513, old), time).await?;
    let event = Uuid::now_v7();
    let login = format!(
        "{{\"login_event_id\":\"{event}\",\"entered_at\":\"{old}\",\"left_at\":null,\"end_reason\":null}}"
    );
    let stamina_payload = canonical::encode(&r.payload)?;
    let raw = String::from_utf8(r.bytes.clone())?
        .replace(&r.id, &Uuid::now_v7().to_string())
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace(
            "\"entity_key\":\"stamina\"",
            &format!("\"entity_key\":\"{event}\""),
        )
        .replace(std::str::from_utf8(&stamina_payload)?, &login);
    let login = playerdata::progress(raw.as_bytes())?;
    a.allow(login.scope.clone()).await;
    store.ingest(login.clone(), time).await?;
    crate::retention::Retention(db.pool.clone()).clean().await?;
    let state: (i64, i64) = sqlx::query_as("SELECT count(*),count(payload) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        state,
        (514, 2),
        "one dispatch must expire at most 512 payloads"
    );
    crate::retention::Retention(db.pool.clone()).clean().await?;
    let state: (i64, i64) = sqlx::query_as("SELECT count(*),count(payload) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(state, (514, 1), "only current progress remains");
    assert_eq!(
        store.ingest(login.clone(), time).await?,
        acknowledgement(&login)?
    );
    assert_eq!(store.ingest(r.clone(), time).await?, acknowledgement(&r)?);
    db.close().await?;
    eprintln!(
        "PG postgres_production_retention_dispatch_erases_payload_preserves_replay {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_shared_authority_preserves_cross_account_record_binding()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let raw = bank(1, None, 0, 100);
    let first = playerdata::items(&raw)?;
    let other_account = Uuid::now_v7().to_string();
    let second = playerdata::items(
        String::from_utf8(raw)?
            .replace(ACCOUNT, &other_account)
            .as_bytes(),
    )?;
    a.allow(first.scope.clone()).await;
    a.allow(second.scope.clone()).await;
    let store = db.store(&a);
    let (one, two) = tokio::join!(store.ingest(first, clock()), store.ingest(second, clock()));
    assert!(matches!(
        (one, two),
        (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await?;
    eprintln!(
        "PG postgres_shared_authority_preserves_cross_account_record_binding {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_conflicting_bindings_raise_one_durable_operator_alarm()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let db = Db::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    for progress_route in [false, true] {
        let a = TestAuthority::default();
        let raw = if progress_route {
            progress(1)
        } else {
            bank(1, None, 0, 100)
        };
        let r = if progress_route {
            playerdata::progress(&raw)?
        } else {
            playerdata::items(&raw)?
        };
        a.allow(r.scope.clone()).await;
        let identity = Identity::san("journal-alarm-test")?;
        let app = http::router(
            http::HttpState::new(db.store(&a), Arc::new(TestClock))
                .with_sources(sources(progress_route)),
            Arc::new(IdentityMap::new(vec![Binding {
                identity: identity.clone(),
                group: if progress_route {
                    http::PROGRESS_GROUP
                } else {
                    http::ITEMS_GROUP
                }
                .into(),
            }])?),
        );
        let request = |body: Vec<u8>| {
            let mut req = Request::post(if progress_route {
                http::PROGRESS_RECORD_PATH
            } else {
                http::RECORDS_PATH
            })
            .header("content-type", "application/json")
            .header("content-length", body.len())
            .body(Body::from(body))
            .unwrap();
            req.extensions_mut()
                .insert(VerifiedClient::from_verified_tls13_http11(identity.clone()));
            req
        };
        assert_eq!(
            app.clone().oneshot(request(raw.clone())).await?.status(),
            StatusCode::OK
        );
        for amount in [101, 102, 101] {
            let changed = if progress_route {
                String::from_utf8(raw.clone())?.replace(
                    "\"stamina_minutes\":2400",
                    &format!("\"stamina_minutes\":{amount}"),
                )
            } else {
                String::from_utf8(raw.clone())?
                    .replace("\"amount\":\"100\"", &format!("\"amount\":\"{amount}\""))
                    .replace(
                        "\"balance_after\":\"100\"",
                        &format!("\"balance_after\":\"{amount}\""),
                    )
            };
            assert_eq!(
                app.clone()
                    .oneshot(request(changed.into_bytes()))
                    .await?
                    .status(),
                StatusCode::CONFLICT
            );
        }
        let alarms: Vec<serde_json::Value> = sqlx::query_scalar("SELECT after FROM audit_log WHERE action='ops.playerdata_binding_conflict' ORDER BY created_at,id").fetch_all(&db.pool).await?;
        assert_eq!(alarms.len(), if progress_route { 2 } else { 1 });
        let alarm = alarms.last().ok_or("alarm missing")?;
        assert_eq!(alarm["reason"], "IMMUTABLE_BINDING");
        assert_eq!(alarm["record_id"], r.id);
        assert_eq!(alarm["source_revision"], "1");
        let serialized = alarm.to_string();
        assert!(!serialized.contains("stamina_minutes"));
        assert!(!serialized.contains("balance_after"));
        assert!(!serialized.contains(ACCOUNT));
        assert_eq!(db.store(&a).cursor(&r.scope).await?.0.stored, 1);
    }
    db.close().await?;
    eprintln!(
        "postgres_conflicting_bindings_raise_one_durable_operator_alarm duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_pd2_03_production_cleanup_prunes_bank_replay_metadata()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await?;
    let old = (now as u64 - 62 * 86400000) / 1000 * 1000;
    let bytes = String::from_utf8(bank(1, None, 0, 100))?
        .replace(&(NOW / 1000).to_string(), &(old / 1000).to_string());
    let r = playerdata::items(bytes.as_bytes())?;
    let a = TestAuthority::default();
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    let time = TrustedTime {
        unix_ms: now as u64,
        uncertainty_ms: 0,
    };
    store
        .ingest(
            r.clone(),
            TrustedTime {
                unix_ms: old,
                uncertainty_ms: 0,
            },
        )
        .await?;
    sqlx::query(
        "INSERT INTO pd_conflicts VALUES(decode(repeat('00',32),'hex'),'IMMUTABLE_BINDING',$1)",
    )
    .bind(now - 1)
    .execute(&db.pool)
    .await?;
    sqlx::query("INSERT INTO pd_deleted_scopes VALUES(decode(repeat('01',32),'hex'),$1,$2,NULL)")
        .bind(now - 121 * 86400000_i64)
        .bind(now - 1)
        .execute(&db.pool)
        .await?;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_records")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let floor: String = sqlx::query_scalar("SELECT retention_floor::text FROM pd_streams")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(floor, "1");
    assert_eq!(store.ingest(r.clone(), time).await, Err(Error::Conflict));
    let identity = Identity::san("journal-test")?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: http::ITEMS_GROUP.into(),
    }])?);
    let app = http::router(
        http::HttpState::new(db.store(&a), Arc::new(TestClock)).with_sources(sources(false)),
        identities,
    );
    let mut request = Request::post(http::RECORDS_PATH)
        .header("content-type", "application/json")
        .header("content-length", bytes.len())
        .body(Body::from(bytes))?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(identity));
    assert_eq!(app.oneshot(request).await?.status(), StatusCode::CONFLICT);
    let count: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM pd_conflicts)+(SELECT count(*) FROM pd_deleted_scopes)",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 0);
    // A fresh entry must still chain to the last balance after production cleanup.
    let next = String::from_utf8(bank(2, Some(&r.entity), 100, 100))?
        .replace(
            &format!(r#""occurred_at":"{}""#, NOW / 1000),
            &format!(r#""occurred_at":"{}""#, time.unix_ms / 1000),
        )
        .replace(
            &format!(r#""coverage_start_at":"{}""#, NOW / 1000),
            &format!(r#""coverage_start_at":"{}""#, old / 1000),
        );
    let result = store
        .ingest(playerdata::items(next.as_bytes())?, time)
        .await;
    assert!(result.is_ok(), "BANK append after retention: {result:?}");
    db.close().await?;
    eprintln!(
        "postgres_pd2_03_production_cleanup_prunes_bank_replay_metadata duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_a3_core_expiry_at_publication()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await?;
    let now = now as u64;
    let old = now - 8 * 86400000;
    let time = |unix_ms| TrustedTime {
        unix_ms,
        uncertainty_ms: 0,
    };
    let a = TestAuthority::default();
    let first = playerdata::progress(
        String::from_utf8(progress(1))?
            .replace(&NOW.to_string(), &old.to_string())
            .as_bytes(),
    )?;
    let scope = first.scope.clone();
    a.allow(scope.clone()).await;
    a.allow(watermark_scope(&scope)).await;
    let store = db.store(&a);
    store.ingest(first, time(old)).await?;
    store.watermark(watermark(1, old), time(old)).await?;
    let next = playerdata::progress(
        String::from_utf8(progress(2))?
            .replace(&NOW.to_string(), &old.to_string())
            .as_bytes(),
    )?;
    store.ingest(next, time(old)).await?;
    store.watermark(watermark(1, now), time(now)).await?;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await?;
    assert_eq!(
        store
            .read(&Actor::Owner(ACCOUNT.into()), &scope, time(now))
            .await?
            .records
            .len(),
        1
    );
    store.watermark(watermark(2, now), time(now)).await?;
    let expires: i64 =
        sqlx::query_scalar("SELECT expires_ms FROM pd_records WHERE source_revision=1")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(expires as u64, now + 7 * 86400000);
    // Upgrade an existing current value with an acceptance-era deadline.
    sqlx::query("UPDATE pd_records SET expires_ms=$1 WHERE source_revision=2")
        .bind(old as i64)
        .execute(&db.pool)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115213000_fix_a3-03-playerdata_publication_expiry.sql"
    ))
    .execute(&db.pool)
    .await?;
    let repaired: i64 =
        sqlx::query_scalar("SELECT expires_ms FROM pd_records WHERE source_revision=2")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(repaired as u64, old + 36500 * 86400000);
    let superseded: i64 =
        sqlx::query_scalar("SELECT expires_ms FROM pd_records WHERE source_revision=1")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(superseded, expires);
    db.close().await?;
    eprintln!(
        "postgres_a3_core_expiry_at_publication duration={:?}",
        started.elapsed()
    );
    Ok(())
}

const MIGRATION_BACKFILL_SYNC: &str =
    include_str!("../../../../migrations/20261115224000_fix_a3-15-migrations_backfill_sync.sql");

// Wait for PostgreSQL to observe the competing write waiting on the migration's
// table lock, rather than relying on task scheduling or a wall-clock sleep.
async fn assert_migration_blocks(db: &Db, pid: i32) -> std::result::Result<(), sqlx::Error> {
    for _ in 0..100 {
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND NOT granted)",
        )
        .bind(pid)
        .fetch_one(&db.pool)
        .await?;
        if waiting {
            return Ok(());
        }
        sqlx::query("SELECT pg_sleep(0.01)")
            .execute(&db.pool)
            .await?;
    }
    Err(sqlx::Error::Protocol(
        "legacy writer did not wait for the migration lock".into(),
    ))
}

#[tokio::test]
async fn postgres_mig01_reminder_repair_and_concurrent_legacy_write()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let db = Db::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let missing = Uuid::now_v7();
    let stale = Uuid::now_v7();
    let owner = Uuid::now_v7();
    sqlx::query("INSERT INTO pd_reminders(id,account_id,kind,entity_key,channel,due_at_ms) VALUES($1,$2,'premium','stale','email',100)")
        .bind(stale).bind(owner).execute(&db.pool).await?;
    // Reproduce writes committed between the original copy and trigger install.
    sqlx::raw_sql("ALTER TABLE pd_reminders DISABLE TRIGGER USER")
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO pd_reminders(id,account_id,kind,entity_key,channel,due_at_ms,state) VALUES($1,$2,'premium','missing','email',100,'delivered')")
        .bind(missing).bind(owner).execute(&db.pool).await?;
    sqlx::query("UPDATE pd_reminders SET state='cancelled' WHERE id=$1")
        .bind(stale)
        .execute(&db.pool)
        .await?;
    sqlx::raw_sql("ALTER TABLE pd_reminders ENABLE TRIGGER USER")
        .execute(&db.pool)
        .await?;
    sqlx::raw_sql("DROP TRIGGER pd_bank_continuation_sync ON pd_records; DROP FUNCTION pd_bank_continuation_sync()")
        .execute(&db.pool).await?;
    // A new scheduler could also have recreated the missed identity meanwhile.
    // Repair must restore the original legacy ID and its delivery state.
    sqlx::query("INSERT INTO pd_scoped_reminders(id,account_id,kind,entity_key,channel,due_at_ms) VALUES($1,$2,'premium','missing','email',100)")
        .bind(Uuid::now_v7()).bind(owner).execute(&db.pool).await?;
    let mut migration = db.pool.begin().await?;
    let (copy, rest) = MIGRATION_BACKFILL_SYNC
        .split_once("UPDATE pd_reminders l")
        .unwrap();
    sqlx::raw_sql(copy).execute(&mut *migration).await?;
    let states: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,state FROM pd_scoped_reminders WHERE id IN ($1,$2) ORDER BY state",
    )
    .bind(missing)
    .bind(stale)
    .fetch_all(&mut *migration)
    .await?;
    assert_eq!(
        states,
        vec![(stale, "cancelled".into()), (missing, "delivered".into())]
    );
    let mut writer = db.pool.acquire().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await?;
    let concurrent = Uuid::now_v7();
    let write = tokio::spawn(async move {
        sqlx::query("INSERT INTO pd_reminders(id,account_id,kind,entity_key,channel,due_at_ms,state) VALUES($1,$2,'premium','concurrent','email',100,'cancelled') RETURNING id")
            .bind(concurrent).bind(owner).fetch_one(&mut *writer).await
    });
    assert_migration_blocks(&db, pid).await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("UPDATE pd_reminders l{rest}")))
        .execute(&mut *migration)
        .await?;
    migration.commit().await?;
    let row = write.await??;
    assert_eq!(row.try_get::<Uuid, _>("id")?, concurrent);
    let state: String = sqlx::query_scalar("SELECT state FROM pd_scoped_reminders WHERE id=$1")
        .bind(concurrent)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(state, "cancelled");
    db.close().await?;
    eprintln!(
        "PG postgres_mig01_reminder_repair_and_concurrent_legacy_write {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn legacy_bank_append(pool: &PgPool, r: &Record) -> std::result::Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let key = stream_key(&r.scope);
    sqlx::query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms) VALUES($1,$2::numeric,$3,$4,$5,$6,NULL,$7,$8,$9,$10,$11,$12,$13)")
        .bind(&key).bind(r.revision.to_string()).bind(&r.scope.authority).bind(&r.scope.recovery)
        .bind(&r.id).bind(&r.entity).bind(subject_key(&r.scope)).bind(&r.kind)
        .bind(r.occurred_ms as i64).bind(r.coverage_ms as i64).bind(hash(&r.bytes))
        .bind(canonical::encode(&r.payload).unwrap()).bind((NOW+86400000) as i64)
        .execute(&mut *tx).await?;
    sqlx::query("UPDATE pd_streams SET highest_seen=$2::numeric,stored_through=$2::numeric,applied_through=$2::numeric,last_record_id=$3 WHERE stream_key=$1")
        .bind(key).bind(r.revision.to_string()).bind(&r.id).execute(&mut *tx).await?;
    tx.commit().await
}

#[tokio::test]
async fn postgres_mig01_bank_repair_and_concurrent_legacy_write()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let db = Db::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let a = TestAuthority::default();
    let first = playerdata::items(&bank(1, None, 0, 100))?;
    a.allow(first.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(first.clone(), clock()).await?;
    sqlx::raw_sql("DROP TRIGGER pd_bank_continuation_sync ON pd_records; DROP FUNCTION pd_bank_continuation_sync()")
        .execute(&db.pool).await?;
    let second = playerdata::items(&bank(2, Some(&first.entity), 100, 100))?;
    legacy_bank_append(&db.pool, &second).await?;
    let third = playerdata::items(&bank(3, Some(&second.entity), 200, 100))?;
    assert_eq!(
        store.ingest(third.clone(), clock()).await,
        Err(Error::Conflict)
    );
    let mut migration = db.pool.begin().await?;
    sqlx::raw_sql(MIGRATION_BACKFILL_SYNC)
        .execute(&mut *migration)
        .await?;
    let repaired: String =
        sqlx::query_scalar("SELECT entry_id FROM pd_bank_continuations WHERE stream_key=$1")
            .bind(stream_key(&first.scope))
            .fetch_one(&mut *migration)
            .await?;
    assert_eq!(repaired, second.entity);
    let mut writer = db.pool.acquire().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await?;
    let record = third.clone();
    let write = tokio::spawn(async move {
        let mut tx = sqlx::Connection::begin(&mut *writer).await?;
        let key = stream_key(&record.scope);
        sqlx::query("INSERT INTO pd_records SELECT stream_key,$2::numeric,authority,recovery_generation,$3,$4,character_id,scope_key,kind,occurred_ms,coverage_ms,$5,$6,expires_ms FROM pd_records WHERE stream_key=$1 AND source_revision=2")
            .bind(&key).bind(record.revision.to_string()).bind(&record.id).bind(&record.entity)
            .bind(hash(&record.bytes)).bind(canonical::encode(&record.payload).unwrap())
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_streams SET highest_seen=3,stored_through=3,applied_through=3 WHERE stream_key=$1")
            .bind(key).execute(&mut *tx).await?;
        tx.commit().await
    });
    assert_migration_blocks(&db, pid).await?;
    migration.commit().await?;
    write.await??;
    let fourth = playerdata::items(&bank(4, Some(&third.entity), 300, 100))?;
    store.ingest(fourth, clock()).await?;
    db.close().await?;
    eprintln!(
        "PG postgres_mig01_bank_repair_and_concurrent_legacy_write {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_retention_keeps_live_login_digest_after_payload_expiry()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await?;
    let old = now as u64 - 62 * 86400000;
    let time = TrustedTime {
        unix_ms: old,
        uncertainty_ms: 0,
    };
    let event = Uuid::now_v7();
    let base = playerdata::progress(&progress(1))?;
    let payload = canonical::encode(&base.payload)?;
    let login_payload = format!(
        "{{\"login_event_id\":\"{event}\",\"entered_at\":\"{old}\",\"left_at\":null,\"end_reason\":null}}"
    );
    let raw = String::from_utf8(base.bytes)?
        .replace(&NOW.to_string(), &old.to_string())
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace(
            "\"entity_key\":\"stamina\"",
            &format!("\"entity_key\":\"{event}\""),
        )
        .replace(std::str::from_utf8(&payload)?, &login_payload)
        .into_bytes();
    let login = playerdata::progress(&raw)?;
    a.allow(login.scope.clone()).await;
    let store = db.store(&a);
    let accepted = store.ingest(login, time).await?;
    let bank = playerdata::items(
        String::from_utf8(bank(1, None, 0, 100))?
            .replace(&(NOW / 1000).to_string(), &(old / 1000).to_string())
            .as_bytes(),
    )?;
    a.allow(bank.scope.clone()).await;
    store.ingest(bank, time).await?;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await?;
    let state: (i64, i64) = sqlx::query_as(
        "SELECT count(*),count(payload) FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE s.progress"
    ).fetch_one(&db.pool).await?;
    assert_eq!(
        state,
        (1, 0),
        "live login digest must survive payload expiry"
    );
    let floor: String =
        sqlx::query_scalar("SELECT retention_floor::text FROM pd_streams WHERE NOT progress")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(floor, "1", "ordinary stream still advances its floor");
    let identity = Identity::san("journal-test")?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: identity.clone(),
        group: http::PROGRESS_GROUP.into(),
    }])?);
    let app = http::router(
        http::HttpState::new(store, Arc::new(TestClock)).with_sources(sources(true)),
        identities,
    );
    let mut request = Request::post("/internal/v1/playerdata/progress/record")
        .header("content-type", "application/json")
        .header("content-length", raw.len())
        .body(Body::from(raw))?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(identity));
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 4096).await?.as_ref(),
        accepted
    );
    // Erased progress keeps its digest for 120 days, then releases it.
    sqlx::query("UPDATE pd_streams SET erased=true,erased_ms=$1 WHERE progress")
        .bind(now - 119 * 86400000_i64)
        .execute(&db.pool)
        .await?;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pd_records")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    sqlx::query("UPDATE pd_streams SET erased_ms=$1 WHERE progress")
        .bind(now - 121 * 86400000_i64)
        .execute(&db.pool)
        .await?;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pd_records")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r7_snapshot_revisions_cross_ten() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let store = db.store(&a);
    for revision in (1..12).step_by(2) {
        let entity = Uuid::now_v7().to_string();
        let begin = snapshot(revision, &entity, false);
        a.allow(begin.scope.clone()).await;
        store.ingest(begin, clock()).await?;
        let end = store
            .ingest(snapshot(revision + 1, &entity, true), clock())
            .await;
        assert!(end.is_ok(), "END {}: {end:?}", revision + 1);
    }
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r8_expired_bank_retry() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::items(&bank(1, None, 0, 100))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(r.clone(), clock()).await?;
    let later = TrustedTime {
        unix_ms: NOW + 31 * 86400000,
        uncertainty_ms: 0,
    };
    store.expire(later).await?;
    let tombstones: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pd_records WHERE payload IS NULL AND digest IS NOT NULL",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(tombstones, 1);
    assert_eq!(store.ingest(r, later).await, Err(Error::Conflict));
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r9_bank_event_identity() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let authority = TestAuthority::default();
    let first = playerdata::items(&bank(1, None, 0, 10))?;
    authority.allow(first.scope.clone()).await;
    let store = db.store(&authority);
    store.ingest(first.clone(), clock()).await?;
    let raw = String::from_utf8(bank(2, Some(&first.entity), 10, 10))?;
    let candidate = playerdata::items(raw.as_bytes())?;
    for field in ["event_id", "operation_occurrence_id"] {
        let id = |r: &Record| {
            playerdata::text(playerdata::get(&r.payload, field).unwrap())
                .unwrap()
                .to_owned()
        };
        let repeated = raw.replace(&id(&candidate), &id(&first));
        assert!(
            matches!(
                store
                    .ingest(playerdata::items(repeated.as_bytes())?, clock())
                    .await,
                Err(Error::Conflict)
            ),
            "duplicate {field} accepted"
        );
    }
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r9_history_coverage_after_pruning() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let authority = TestAuthority::default();
    let store = db.store(&authority);
    for (family, payload) in [
        (
            "MARKET",
            include_str!(
                "../../../../../docs/contracts/fixtures/game-journal-items-v1/payload-market-event.valid.json"
            ),
        ),
        (
            "PLAYER_TRADE",
            include_str!(
                "../../../../../docs/contracts/fixtures/game-journal-items-v1/payload-player-trade-event.valid.json"
            ),
        ),
    ] {
        let later = TrustedTime {
            unix_ms: NOW + 100 * 86400000,
            ..clock()
        };
        let make = |rev, occurred, coverage| {
            let envelope =
                String::from_utf8(snapshot(rev, &Uuid::now_v7().to_string(), false).bytes).unwrap();
            let prefix = envelope
                .split_once("\"payload\":")
                .unwrap()
                .0
                .replace("\"ITEMS\"", &format!("\"{family}\""))
                .replace("ITEMS_BEGIN", &format!("{family}_EVENT"))
                .replace(
                    &format!(r#""occurred_at":"{}""#, NOW / 1000),
                    &format!(r#""occurred_at":"{occurred}""#),
                )
                .replace(
                    &format!("\"coverage_start_at\":\"{}\"", NOW / 1000),
                    &format!("\"coverage_start_at\":\"{coverage}\""),
                );
            playerdata::items(format!("{prefix}\"payload\":{payload}}}").as_bytes())
        };
        let first = make(1, NOW / 1000, NOW / 1000)?;
        authority.allow(first.scope.clone()).await;
        store.ingest(first, clock()).await?;
        store.expire(later).await?;
        let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_records WHERE kind=$1")
            .bind(format!("{family}_EVENT"))
            .fetch_one(&db.pool)
            .await?;
        assert_eq!(retained, 0);
        assert!(
            matches!(
                store
                    .ingest(make(2, later.unix_ms / 1000, NOW / 1000 - 1)?, later)
                    .await,
                Err(Error::Conflict)
            ),
            "changed {family} coverage accepted"
        );
        store
            .ingest(make(2, later.unix_ms / 1000, NOW / 1000)?, later)
            .await?;
    }
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r9_account_erasure_batch() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let authority = TestAuthority::default();
    let first = playerdata::progress(&progress(1))?;
    authority.allow(first.scope.clone()).await;
    db.store(&authority).ingest(first, clock()).await?;
    sqlx::query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms) SELECT stream_key,i,authority,recovery_generation,uuidv7()::text,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms FROM pd_records CROSS JOIN generate_series(2,513) i").execute(&db.pool).await?;
    sqlx::query("INSERT INTO pd_erasure_jobs(account_id) VALUES($1)")
        .bind(ACCOUNT)
        .execute(&db.pool)
        .await?;
    let worker = crate::retention::Retention(db.pool.clone());
    for expected in [1, 0] {
        worker.expire_batch().await?;
        let state:(i64,i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE scope_key<>''),(SELECT count(*) FROM pd_erasure_jobs) FROM pd_records").fetch_one(&db.pool).await?;
        assert_eq!(
            state,
            (expected, expected),
            "bounded erasure and pending job"
        );
    }
    db.close().await?;
    Ok(())
}

const SKILL_EVENT: &str = "019a0000-0000-7000-8000-000000000011";
fn skill_change(revision: u64, after: u32) -> Vec<u8> {
    let payload = format!(
        r#"{{"build_occurrence_id":"{SKILL_EVENT}","cause":"training","policy_revision":"v1","skills":[{{"skill":"sword","level_before":10,"progress_before":"0","level_after":{after},"progress_after":"0"}}]}}"#
    );
    let raw = family_bytes("skill_history", SKILL_EVENT, &payload, false);
    String::from_utf8(raw)
        .unwrap()
        .replace(
            r#""source_revision":"1""#,
            &format!(r#""source_revision":"{revision}""#),
        )
        .into_bytes()
}

#[tokio::test]
async fn pd2_skill_occurrence_requires_removal()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let authority = TestAuthority::default();
    let first = playerdata::progress(&skill_change(1, 11))?;
    authority.allow(first.scope.clone()).await;
    let store = db.store(&authority);
    store.ingest(first.clone(), clock()).await?;
    store.ingest(first, clock()).await?;
    let next = playerdata::progress(&skill_change(2, 12))?;
    assert!(
        matches!(store.ingest(next, clock()).await, Err(Error::Conflict)),
        "immutable skill occurrence replaced"
    );
    let raw = String::from_utf8(skill_change(2, 12))?;
    let prefix = raw
        .split_once(r#""payload":"#)
        .unwrap()
        .0
        .replace("UPSERT", "REMOVE");
    store
        .ingest(
            playerdata::progress(format!(r#"{prefix}"payload":null}}"#).as_bytes())?,
            clock(),
        )
        .await?;
    store
        .ingest(playerdata::progress(&skill_change(3, 12))?, clock())
        .await?;
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r10_bank_fee_predecessor() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let authority = TestAuthority::default();
    let deposit = playerdata::items(&bank(1, None, 0, 100))?;
    authority.allow(deposit.scope.clone()).await;
    let store = db.store(&authority);
    store.ingest(deposit, clock()).await?;
    // The native fee consumed ten gold. Its entry is the withdrawal's
    // predecessor even though the fee has no G-10 record.
    let raw = String::from_utf8(bank(2, Some(&Uuid::now_v7().to_string()), 90, 10))?
        .replace("DEPOSIT", "WITHDRAW")
        .replace("CREDIT", "DEBIT")
        .replace(r#""balance_after":"100""#, r#""balance_after":"80""#);
    let result = store
        .ingest(playerdata::items(raw.as_bytes())?, clock())
        .await;
    assert!(result.is_ok(), "withdraw after fee: {result:?}");
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r12_bank_zero_net_gap() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let db = Db::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let authority = TestAuthority::default();
    let first = playerdata::items(&bank(1, None, 0, 100))?;
    let first_id = first.entity.clone();
    authority.allow(first.scope.clone()).await;
    let store = db.store(&authority);
    store.ingest(first, clock()).await?;

    // Unexported native entries offset each other before the next deposit.
    let native_id = Uuid::now_v7().to_string();
    let next = playerdata::items(&bank(2, Some(&native_id), 100, 10))?;
    store.ingest(next, clock()).await?;

    let fork = playerdata::items(&bank(3, Some(&first_id), 110, 10))?;
    assert_eq!(store.ingest(fork, clock()).await, Err(Error::Conflict));
    db.close().await?;
    Ok(())
}

#[test]
fn r11_bank_self_predecessor() {
    let raw = bank(1, None, 0, 10);
    let entry = playerdata::items(&raw).unwrap().entity;
    let raw = String::from_utf8(raw).unwrap().replace(
        r#""previous_entry_id":null"#,
        &format!(r#""previous_entry_id":"{entry}""#),
    );
    assert_eq!(
        playerdata::items(raw.as_bytes()).err(),
        Some(playerdata::Error::Invalid)
    );
}

#[tokio::test]
async fn r11_zero_page_snapshot() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let entity = Uuid::now_v7().to_string();
    let begin = snapshot(1, &entity, false);
    a.allow(begin.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(begin, clock()).await?;
    let end = snapshot(2, &entity, true);
    let envelope = std::str::from_utf8(&end.bytes)?
        .split_once(r#""kind":"ITEMS_END""#)
        .unwrap()
        .0;
    let item = r#"{"item_instance_id":"019a0000-0000-7000-8000-000000000010","definition_family":"item","definition_production_key":"item","definition_revision_ref":"rev","quantity":1,"root":"INBOX","town_id":null,"depot_box":null,"house_id":null,"spatial_position_hex":null,"equipment_slot":null,"parent_item_instance_id":null,"placement_ordinal":"1"}"#;
    let raw = format!(
        r#"{envelope}"kind":"ITEMS_PAGE","payload":{{"snapshot_id":"{entity}","page_index":0,"items":[{item}]}}}}"#
    );
    let page = playerdata::items(raw.as_bytes())?;
    assert_eq!(store.ingest(page, clock()).await, Err(Error::Conflict));
    store.ingest(end, clock()).await?;
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r11_character_replay_expiry() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Db::open().await? else {
        return Ok(());
    };
    let a = TestAuthority::default();
    let r = playerdata::progress(&progress(1))?;
    a.allow(r.scope.clone()).await;
    let store = db.store(&a);
    store.ingest(r, clock()).await?;
    let other = playerdata::progress(&progress_for(2, &Uuid::now_v7().to_string()))?;
    a.allow(other.scope.clone()).await;
    store.ingest(other, clock()).await?;
    let db_now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await?;
    let purge = TrustedTime {
        unix_ms: db_now as u64 - 119 * 86400000,
        uncertainty_ms: 0,
    };
    store
        .erase_character_cut(
            ACCOUNT,
            CHARACTER,
            purge,
            Some("9"),
            None,
            purge.unix_ms as i64,
        )
        .await?;
    let retention = crate::retention::Retention(db.pool.clone());
    retention.expire_batch().await?;
    let revisions = || {
        sqlx::query_scalar::<_, String>(
            "SELECT source_revision::text FROM pd_records ORDER BY pd_records.source_revision",
        )
        .fetch_all(&db.pool)
    };
    assert_eq!(revisions().await?, ["1", "2"]);
    // Age both records equally, crossing the erased character's deadline.
    sqlx::query("UPDATE pd_records SET expires_ms=expires_ms-2*86400000::bigint")
        .execute(&db.pool)
        .await?;
    retention.expire_batch().await?;
    assert_eq!(revisions().await?, ["2"]);
    db.close().await?;
    Ok(())
}
