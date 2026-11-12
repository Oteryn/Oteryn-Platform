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
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261112090003_t72_game_journal.sql"
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
    let raw = bank(1, None, 0, 100);
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
