//! T72 ingest through the composed routes, byte for byte against the golden
//! payloads of the accepted G-10 and G-11 revision 3 contracts (the contracts
//! carry their goldens inline; `docs/contracts/fixtures/game-journal-*` holds
//! byte copies, checked against the contracts in `authority::tests`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::store::{Admission, Error, FenceFuture, Lease, LeaseFuture, TrustedTime};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use oteryn_internal_api::{
    guard::{Binding, Identity, VerifiedClient},
    playerdata::{self, Record, Scope},
};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

const ITEMS_CONTRACT: &str =
    include_str!("../../../../docs/contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md");
const PROGRESS_CONTRACT: &str =
    include_str!("../../../../docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md");
const ITEMS_SAN: &str = "journal-items.oteryn.internal";
const PROGRESS_SAN: &str = "journal-progress.oteryn.internal";
const STRANGER_SAN: &str = "stranger.oteryn.internal";
/// The G-10 goldens' authority and a trusted clock one hour after their logout.
const ITEMS_AUTHORITY: &str = "OTERYN_GAME_JOURNAL";
const ITEMS_NOW: u64 = 1793836800000 + 3600000;
/// G-11 §10: authority and the fixed trusted clock of its goldens.
const PROGRESS_AUTHORITY: &str = "oteryn:journal:primary";
const PROGRESS_NOW: u64 = 1790000000000;

/// Single-line wire blocks of one contract, in document order.
fn blocks(contract: &'static str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut lines = contract.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "```json" {
            let body: Vec<&str> = lines.by_ref().take_while(|l| l.trim() != "```").collect();
            assert_eq!(body.len(), 1, "golden blocks are single-line wire bytes");
            out.push(body[0]);
        }
    }
    out
}
/// The one golden block containing every needle.
fn golden(contract: &'static str, needles: &[&str]) -> &'static str {
    let found: Vec<&str> = blocks(contract)
        .into_iter()
        .filter(|b| needles.iter().all(|n| b.contains(n)))
        .collect();
    assert_eq!(found.len(), 1, "{needles:?}");
    found[0]
}
fn items(needles: &[&str]) -> &'static str {
    golden(ITEMS_CONTRACT, needles)
}
fn progress(needles: &[&str]) -> &'static str {
    golden(PROGRESS_CONTRACT, needles)
}

#[derive(Debug)]
struct FixedClock(u64);
impl Clock for FixedClock {
    fn trusted_time(&self) -> http::ClockFuture<'_> {
        let unix_ms = self.0;
        Box::pin(async move {
            Ok(TrustedTime {
                unix_ms,
                uncertainty_ms: 0,
            })
        })
    }
}
/// Stand-in for the accepted ownership adapter: allows exactly the listed scopes.
#[derive(Debug, Clone, Default)]
struct Allowed(Arc<RwLock<Vec<Scope>>>);
#[derive(Debug)]
struct AllowedLease;
impl Lease for AllowedLease {
    fn fence<'a>(&'a self, _: &'a mut sqlx::PgConnection, _: &'a Scope) -> FenceFuture<'a> {
        Box::pin(async { Ok(Admission::Owner) })
    }
    fn validate(&self, _: &Record) -> std::result::Result<(), Error> {
        Ok(())
    }
}
impl Authority for Allowed {
    fn acquire<'a>(&'a self, s: &'a Scope) -> LeaseFuture<'a> {
        Box::pin(async move {
            if self.0.read().await.contains(s) {
                Ok(Box::new(AllowedLease) as Box<dyn Lease>)
            } else {
                Err(Error::Conflict)
            }
        })
    }
}
impl Allowed {
    async fn allow(&self, s: Scope) {
        self.0.write().await.push(s);
    }
}

fn identities() -> Arc<IdentityMap> {
    Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san(ITEMS_SAN).unwrap(),
                group: ITEMS_GROUP.into(),
            },
            Binding {
                identity: Identity::san(PROGRESS_SAN).unwrap(),
                group: PROGRESS_GROUP.into(),
            },
        ])
        .unwrap(),
    )
}
fn settings() -> Settings {
    let values: std::collections::BTreeMap<&str, &str> = [
        (ITEMS_ENABLED, "true"),
        (PROGRESS_ENABLED, "true"),
        (ITEMS_SOURCE, ITEMS_AUTHORITY),
        (PROGRESS_SOURCE, PROGRESS_AUTHORITY),
        (
            FAMILIES,
            "ITEMS, BANK,MARKET,PLAYER_TRADE,stamina,skull,quest,game_login",
        ),
    ]
    .into_iter()
    .collect();
    Settings::from_lookup(|key| values.get(key).map(|v| (*v).to_owned()))
}
struct Answer {
    status: StatusCode,
    cache: Option<String>,
    retry_after: Option<String>,
    body: Vec<u8>,
}
async fn post(app: &Router, san: &str, path: &str, body: &[u8]) -> Answer {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from(body.to_vec()))
        .unwrap();
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(
            Identity::san(san).unwrap(),
        ));
    let response = app.clone().oneshot(request).await.unwrap();
    let text = |name| {
        response
            .headers()
            .get(name)
            .map(|v: &header::HeaderValue| v.to_str().unwrap().to_owned())
    };
    let (status, cache, retry_after) = (
        response.status(),
        text(header::CACHE_CONTROL),
        text(header::RETRY_AFTER),
    );
    let body = to_bytes(response.into_body(), 4096).await.unwrap().to_vec();
    Answer {
        status,
        cache,
        retry_after,
        body,
    }
}
fn lazy_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://journal@127.0.0.1:1/unreachable")
        .unwrap()
}

struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL journal ingest test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = sqlx::postgres::PgPoolOptions::new()
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("pd_ingest_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
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
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn records(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM pd_records")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}

#[test]
fn defaults_are_off_and_families_parse() {
    let defaults = Settings::from_lookup(|_| None);
    assert!(!defaults.items_ingest && !defaults.progress_ingest);
    assert!(defaults.families.is_empty());
    assert!(defaults.sources.items.is_empty() && defaults.sources.progress.is_empty());
    let on = settings();
    assert!(on.items_ingest && on.progress_ingest);
    assert!(on.families.contains("ITEMS") && on.families.contains("BANK"));
    assert_eq!(on.families.len(), 8);
    // Only the literal `true` enables.
    let yes = Settings::from_lookup(|key| (key == ITEMS_ENABLED).then(|| "TRUE".to_owned()));
    assert!(!yes.items_ingest);
}

/// Every golden of both contracts parses as the contract says, so a test below
/// that names a golden really sends the contract's bytes.
#[test]
fn contract_goldens_decode_or_refuse_as_specified() {
    assert_eq!(blocks(ITEMS_CONTRACT).len(), 10);
    // 21 revision 3 goldens plus the terminal refusal answer (2026-11-12 amendment).
    assert_eq!(blocks(PROGRESS_CONTRACT).len(), 22);
    assert!(
        playerdata::cursor(items(&["ReadGameJournalCursorV1", "\"character_id\":null"]).as_bytes())
            .is_ok()
    );
    assert!(
        playerdata::cursor(
            items(&["ReadGameJournalCursorV1", "\"character_id\":\"019a"]).as_bytes()
        )
        .is_err()
    );
    for kind in ["ITEMS_BEGIN", "ITEMS_END", "BANK_ENTRY"] {
        assert!(
            playerdata::items(items(&[kind]).as_bytes()).is_ok(),
            "{kind}"
        );
    }
    let bank = items(&["BANK_ENTRY"]);
    let begin = items(&["ITEMS_BEGIN"]);
    let envelope = |base: &str, family: &str, kind: &str, payload: &str| {
        let head = &base[..base.find("\"payload\":").unwrap()];
        head.replacen(
            "\"family\":\"ITEMS\"",
            &format!("\"family\":\"{family}\""),
            1,
        )
        .replacen(
            "\"kind\":\"ITEMS_BEGIN\"",
            &format!("\"kind\":\"{kind}\""),
            1,
        ) + "\"payload\":"
            + payload
            + "}"
    };
    let page = items(&["\"page_index\":0"]);
    let market = items(&["\"offer_id\""]);
    let trade = items(&["\"given\""]);
    assert!(playerdata::items(envelope(begin, "ITEMS", "ITEMS_PAGE", page).as_bytes()).is_ok());
    assert!(
        playerdata::items(envelope(begin, "MARKET", "MARKET_EVENT", market).as_bytes()).is_ok()
    );
    assert!(
        playerdata::items(envelope(begin, "PLAYER_TRADE", "PLAYER_TRADE_EVENT", trade).as_bytes())
            .is_ok()
    );
    let invalid_bank = items(&["\"amount\":100"]);
    let head = &bank[..bank.find("\"payload\":").unwrap()];
    assert!(playerdata::items(format!("{head}\"payload\":{invalid_bank}}}").as_bytes()).is_err());

    let stamina = progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]);
    assert!(playerdata::progress(stamina.as_bytes()).is_ok());
    assert!(playerdata::progress(progress(&["\"source_revision\":1,"]).as_bytes()).is_err());
    for needles in [
        &["\"complete_through\":\"1\""][..],
        &["\"complete_through\":\"2\""][..],
    ] {
        assert!(playerdata::watermark(progress(needles).as_bytes()).is_ok());
    }
}

#[tokio::test]
async fn flag_off_wrong_identity_wrong_source_and_bad_body() {
    let bank = items(&["BANK_ENTRY"]).as_bytes();
    let stamina =
        progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]).as_bytes();
    // Defaults: every route unavailable, even to its own identity.
    let off = configured_router(lazy_pool(), &Settings::default(), identities());
    for (san, path, body) in [
        (ITEMS_SAN, http::RECORDS_PATH, bank),
        (ITEMS_SAN, http::CURSOR_PATH, bank),
        (PROGRESS_SAN, http::PROGRESS_RECORD_PATH, stamina),
        (PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, stamina),
    ] {
        let a = post(&off, san, path, body).await;
        assert_eq!(a.status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(a.cache.as_deref(), Some("no-store"));
        assert!(a.body.is_empty());
    }
    // A flag without its source authority stays off.
    let mut unbound = settings();
    unbound.sources.items.clear();
    let app = configured_router(lazy_pool(), &unbound, identities());
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, bank).await.status,
        StatusCode::SERVICE_UNAVAILABLE
    );

    let app = configured_router(lazy_pool(), &settings(), identities());
    // Each purpose refuses the other purpose's identity and an unknown one.
    for (san, path, body) in [
        (PROGRESS_SAN, http::RECORDS_PATH, bank),
        (STRANGER_SAN, http::RECORDS_PATH, bank),
        (PROGRESS_SAN, http::CURSOR_PATH, bank),
        (ITEMS_SAN, http::PROGRESS_RECORD_PATH, stamina),
        (STRANGER_SAN, http::PROGRESS_WATERMARK_PATH, stamina),
    ] {
        let a = post(&app, san, path, body).await;
        assert_eq!(a.status, StatusCode::UNAUTHORIZED, "{san} {path}");
        assert!(a.body.is_empty());
        assert_eq!(a.cache.as_deref(), Some("no-store"));
    }
    // A body naming another source authority is a peer/source failure.
    let foreign = String::from_utf8(bank.to_vec())
        .unwrap()
        .replace(ITEMS_AUTHORITY, "OTERYN_GAME_OTHER");
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, foreign.as_bytes())
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let foreign = String::from_utf8(stamina.to_vec())
        .unwrap()
        .replace(PROGRESS_AUTHORITY, "oteryn:journal:other");
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            foreign.as_bytes()
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
    // Bad bodies are 400 with an empty body, before any authority or storage.
    for (san, path, body) in [
        (ITEMS_SAN, http::RECORDS_PATH, &b"not json"[..]),
        (ITEMS_SAN, http::RECORDS_PATH, b"{}"),
        (
            ITEMS_SAN,
            http::CURSOR_PATH,
            items(&["ReadGameJournalCursorV1", "\"character_id\":\"019a"]).as_bytes(),
        ),
        (
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            progress(&["\"source_revision\":1,"]).as_bytes(),
        ),
        (PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, b"[]"),
    ] {
        let a = post(&app, san, path, body).await;
        assert_eq!(a.status, StatusCode::BAD_REQUEST, "{path}");
        assert!(a.body.is_empty());
    }
    // Oversize is 413, not truncated.
    let big = vec![b' '; 65537];
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, &big).await.status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Enabled in production composition: no ownership adapter exists yet, so a
    // valid record is 503 (no acknowledgement), never accepted.
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, bank).await.status,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post(&app, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, stamina)
            .await
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn items_budget_is_per_purpose_and_answers_429_with_retry_after() {
    let app = configured_router(lazy_pool(), &settings(), identities());
    for _ in 0..120 {
        assert_eq!(
            post(&app, ITEMS_SAN, http::RECORDS_PATH, b"{}")
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    let a = post(&app, ITEMS_SAN, http::CURSOR_PATH, b"{}").await;
    assert_eq!(a.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(a.retry_after.as_deref(), Some("60"));
    assert!(a.body.is_empty());
    // The progress purpose keeps its own budget.
    assert_eq!(
        post(&app, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, b"{}")
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn postgres_items_goldens_ack_replay_conflict_order_and_rollback() {
    let Some(db) = Db::open().await else {
        return;
    };
    let ack = items(&["\"result\":\"accepted\"}"]);
    assert_eq!(ack, r#"{"contract_version":1,"result":"accepted"}"#);
    let bank = items(&["BANK_ENTRY"]);
    let begin = items(&["ITEMS_BEGIN"]);
    let end = items(&["ITEMS_END"]);
    let cursor = items(&["ReadGameJournalCursorV1", "\"character_id\":null"]);
    let allowed = Allowed::default();
    allowed
        .allow(playerdata::items(bank.as_bytes()).unwrap().scope)
        .await;
    allowed
        .allow(playerdata::items(begin.as_bytes()).unwrap().scope)
        .await;
    let app = router(
        db.pool.clone(),
        Arc::new(allowed.clone()),
        Arc::new(FixedClock(ITEMS_NOW)),
        &settings(),
        identities(),
    );

    // Unregistered stream: 409, never cursor zero.
    let a = post(&app, ITEMS_SAN, http::CURSOR_PATH, cursor.as_bytes()).await;
    assert_eq!(a.status, StatusCode::CONFLICT);
    assert!(a.body.is_empty());

    // First write and exact replay: the same accepted bytes, one row.
    for _ in 0..2 {
        let a = post(&app, ITEMS_SAN, http::RECORDS_PATH, bank.as_bytes()).await;
        assert_eq!(a.status, StatusCode::OK);
        assert_eq!(a.cache.as_deref(), Some("no-store"));
        assert_eq!(a.body, ack.as_bytes());
    }
    assert_eq!(db.records().await, 1);
    let expected_cursor = br#"{"contract_version":1,"after_source_revision":"1","last_record_id":"019a0000-0000-7000-8000-000000000007"}"#;
    let a = post(&app, ITEMS_SAN, http::CURSOR_PATH, cursor.as_bytes()).await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(a.body, expected_cursor);

    // Changed amount/balance under the same keys: 409, nothing written.
    let changed = bank
        .replace("\"amount\":\"100\"", "\"amount\":\"101\"")
        .replace("\"balance_after\":\"100\"", "\"balance_after\":\"101\"");
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, changed.as_bytes())
            .await
            .status,
        StatusCode::CONFLICT
    );
    // Revision 3 before 2: 409, the cursor does not move.
    let ahead = bank
        .replace("\"source_revision\":\"1\"", "\"source_revision\":\"3\"")
        .replace(
            "019a0000-0000-7000-8000-000000000007",
            "019a0000-0000-7000-8000-0000000000a7",
        );
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, ahead.as_bytes())
            .await
            .status,
        StatusCode::CONFLICT
    );
    // The invalid economic payload golden: 400.
    let head = &bank[..bank.find("\"payload\":").unwrap()];
    let invalid = format!("{head}\"payload\":{}}}", items(&["\"amount\":100"]));
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, invalid.as_bytes())
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db.records().await, 1);
    assert_eq!(
        post(&app, ITEMS_SAN, http::CURSOR_PATH, cursor.as_bytes())
            .await
            .body,
        expected_cursor
    );

    // Empty logout snapshot: an END whose digest does not match rolls back
    // completely; the exact END then completes the pair.
    let a = post(&app, ITEMS_SAN, http::RECORDS_PATH, begin.as_bytes()).await;
    assert_eq!(
        (a.status, a.body.as_slice()),
        (StatusCode::OK, ack.as_bytes())
    );
    let wrong = end.replace("e3b0c442", "03b0c442");
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, wrong.as_bytes())
            .await
            .status,
        StatusCode::CONFLICT
    );
    assert_eq!(db.records().await, 2);
    let heads: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_item_heads")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(heads, 0);
    let a = post(&app, ITEMS_SAN, http::RECORDS_PATH, end.as_bytes()).await;
    assert_eq!(
        (a.status, a.body.as_slice()),
        (StatusCode::OK, ack.as_bytes())
    );
    let snapshot: String = sqlx::query_scalar("SELECT snapshot_id FROM pd_item_heads")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(snapshot, "019a0000-0000-7000-8000-000000000005");

    // Flag rollback: off answers 503 and keeps every stored row; back on, the
    // exact replay is acknowledged again without a second row.
    let mut off = settings();
    off.items_ingest = false;
    let disabled = router(
        db.pool.clone(),
        Arc::new(allowed.clone()),
        Arc::new(FixedClock(ITEMS_NOW)),
        &off,
        identities(),
    );
    assert_eq!(
        post(&disabled, ITEMS_SAN, http::RECORDS_PATH, bank.as_bytes())
            .await
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(db.records().await, 3);
    let a = post(&app, ITEMS_SAN, http::RECORDS_PATH, bank.as_bytes()).await;
    assert_eq!(
        (a.status, a.body.as_slice()),
        (StatusCode::OK, ack.as_bytes())
    );
    assert_eq!(db.records().await, 3);
    db.close().await;
}

#[tokio::test]
async fn postgres_progress_goldens_ack_watermark_replay_and_conflicts() {
    let Some(db) = Db::open().await else {
        return;
    };
    let stamina = progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]);
    let ack = progress(&["\"accepted_source_revision\":\"1\""]);
    let watermark = progress(&["\"complete_through\":\"1\""]);
    let ahead = progress(&["\"complete_through\":\"2\""]);
    let record = playerdata::progress(stamina.as_bytes()).unwrap();
    let allowed = Allowed::default();
    allowed.allow(record.scope.clone()).await;
    let mut account = record.scope.clone();
    account.character = None;
    account.world = None;
    account.character_revision = None;
    account.projection_revision = None;
    allowed.allow(account).await;
    let app = router(
        db.pool.clone(),
        Arc::new(allowed),
        Arc::new(FixedClock(PROGRESS_NOW)),
        &settings(),
        identities(),
    );

    // A watermark ahead of the data is 409 and publishes nothing.
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_WATERMARK_PATH,
            ahead.as_bytes()
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    for _ in 0..2 {
        let a = post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            stamina.as_bytes(),
        )
        .await;
        assert_eq!(a.status, StatusCode::OK);
        assert_eq!(a.cache.as_deref(), Some("no-store"));
        assert_eq!(a.body, ack.as_bytes());
    }
    assert_eq!(db.records().await, 1);
    for _ in 0..2 {
        let a = post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_WATERMARK_PATH,
            watermark.as_bytes(),
        )
        .await;
        assert_eq!(a.status, StatusCode::OK);
        assert_eq!(
            a.body,
            br#"{"contract_version":1,"result":"accepted","complete_through":"1"}"#
        );
    }
    // Revision 2 is missing: the golden rule-invalid watermark stays 409.
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_WATERMARK_PATH,
            ahead.as_bytes()
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    // Changed payload under the same record id: 409.
    let changed = stamina.replace("\"stamina_minutes\":2400", "\"stamina_minutes\":2399");
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            changed.as_bytes()
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    // Revision 3 before 2 under a new record id: 409.
    let gap = stamina
        .replace("\"source_revision\":\"1\"", "\"source_revision\":\"3\"")
        .replace(
            "01934f10-7c04-7001-805b-3b1122334403",
            "01934f10-7c04-7001-805b-3b11223344a3",
        );
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            gap.as_bytes()
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    // The invalid full request and the invalid family payload goldens: 400.
    let mut invalid = vec![progress(&["\"source_revision\":1,"]).to_owned()];
    let head = &stamina[..stamina.find("\"payload\":").unwrap()];
    // Each family gets an envelope of its own scope (game_login: the account),
    // proven valid with that family's valid golden payload, so only the
    // payload makes the request invalid.
    let stamina_payload = &stamina[stamina.find("\"payload\":").unwrap() + 10..stamina.len() - 1];
    for (family, entity, valid, needle, account) in [
        (
            "stamina",
            "stamina",
            stamina_payload,
            "\"stamina_minutes\":2521",
            false,
        ),
        (
            "skull",
            "skull",
            progress(&["\"skull\":\"RED\""]),
            "\"skull\":\"BLACK\"",
            false,
        ),
        (
            "game_login",
            "01934f10-7c04-7001-805b-3b1122334413",
            progress(&["\"end_reason\":null}"]),
            "\"ip_address\"",
            true,
        ),
        (
            "quest",
            "quest/example",
            progress(&["\"value\":\"2\"}"]),
            "\"value\":\"02\"",
            false,
        ),
    ] {
        let mut head = head
            .replace(
                "\"family\":\"stamina\"",
                &format!("\"family\":\"{family}\""),
            )
            .replace(
                "\"entity_key\":\"stamina\"",
                &format!("\"entity_key\":\"{entity}\""),
            );
        if account {
            for member in [
                "\"character_id\":\"01934f10-7c04-7001-805b-3b1122334401\"",
                "\"world_id\":\"01934f10-7c04-7001-805b-3b1122334402\"",
                "\"projection_revision\":\"12\"",
                "\"character_revision\":\"9\"",
            ] {
                let name = &member[..member.find(':').unwrap()];
                assert_eq!(head.matches(member).count(), 1, "{member}");
                head = head.replace(member, &format!("{name}:null"));
            }
        }
        let accepted = format!("{head}\"payload\":{valid}}}");
        assert!(
            playerdata::progress(accepted.as_bytes()).is_ok(),
            "{accepted}"
        );
        invalid.push(format!("{head}\"payload\":{}}}", progress(&[needle])));
    }
    for body in &invalid {
        let a = post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            body.as_bytes(),
        )
        .await;
        assert_eq!(a.status, StatusCode::BAD_REQUEST, "{body}");
        assert!(a.body.is_empty());
    }
    assert_eq!(db.records().await, 1);
    db.close().await;
}

/// G-11 §8.2 terminal refusal through the route: the golden record of a deleted
/// character answers the golden refusal bytes, again on replay, and consumes
/// revision 1 so the next revision of another character is accepted.
#[tokio::test]
async fn postgres_progress_terminal_refusal_golden_and_stream_continues() {
    let Some(db) = Db::open().await else {
        return;
    };
    let stamina = progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]);
    let refused = progress(&["\"result\":\"refused\""]);
    let record = playerdata::progress(stamina.as_bytes()).unwrap();
    let next = stamina
        .replace("\"source_revision\":\"1\"", "\"source_revision\":\"2\"")
        .replace(
            "01934f10-7c04-7001-805b-3b1122334403",
            "01934f10-7c04-7001-805b-3b11223344b3",
        )
        .replace(
            "01934f10-7c04-7001-805b-3b1122334401",
            "01934f10-7c04-7001-805b-3b11223344b1",
        );
    let other = playerdata::progress(next.as_bytes()).unwrap();
    let allowed = Allowed::default();
    allowed.allow(record.scope.clone()).await;
    allowed.allow(other.scope.clone()).await;
    let store = Store::new(
        db.pool.clone(),
        Arc::new(allowed.clone()),
        settings().flags(),
    );
    store
        .erase_character(
            &record.scope.account,
            record.scope.character.as_deref().unwrap(),
            TrustedTime {
                unix_ms: PROGRESS_NOW,
                uncertainty_ms: 0,
            },
        )
        .await
        .unwrap();
    let app = router(
        db.pool.clone(),
        Arc::new(allowed),
        Arc::new(FixedClock(PROGRESS_NOW)),
        &settings(),
        identities(),
    );
    for _ in 0..2 {
        let a = post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            stamina.as_bytes(),
        )
        .await;
        assert_eq!(a.status, StatusCode::OK);
        assert_eq!(a.cache.as_deref(), Some("no-store"));
        assert_eq!(a.body, refused.as_bytes());
    }
    let changed = stamina.replace("\"stamina_minutes\":2400", "\"stamina_minutes\":2399");
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            changed.as_bytes()
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    let a = post(
        &app,
        PROGRESS_SAN,
        http::PROGRESS_RECORD_PATH,
        next.as_bytes(),
    )
    .await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(
        a.body,
        br#"{"contract_version":1,"result":"accepted","record_id":"01934f10-7c04-7001-805b-3b11223344b3","accepted_source_revision":"2"}"#
    );
    assert_eq!(db.records().await, 2);
    db.close().await;
}

/// Retry-After and an empty no-store body: the 429 shape of both purposes.
fn assert_too_many(a: &Answer, retry_after: &str) {
    assert_eq!(a.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(a.retry_after.as_deref(), Some(retry_after));
    assert_eq!(a.cache.as_deref(), Some("no-store"));
    assert!(a.body.is_empty());
}

/// G-11 §3: 600 requests per minute per peer across both progress routes,
/// separate from the G-10 budget; G-10/G-11 §3 order 503, 401, 429, then
/// transport, so an over-budget peer gets 429 even for an oversized body.
#[tokio::test]
async fn progress_budget_is_600_shared_by_its_routes_and_precedes_transport() {
    let app = configured_router(lazy_pool(), &settings(), identities());
    for i in 0..600 {
        let path = if i % 2 == 0 {
            http::PROGRESS_RECORD_PATH
        } else {
            http::PROGRESS_WATERMARK_PATH
        };
        assert_eq!(
            post(&app, PROGRESS_SAN, path, b"{}").await.status,
            StatusCode::BAD_REQUEST,
            "{i}"
        );
    }
    assert_too_many(
        &post(&app, PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, b"{}").await,
        "60",
    );
    let big = vec![b' '; 262145];
    assert_too_many(
        &post(&app, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, &big).await,
        "60",
    );
    // Peer identity still comes first, and G-10 keeps its own budget.
    assert_eq!(
        post(&app, STRANGER_SAN, http::PROGRESS_RECORD_PATH, &big)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, b"{}")
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    // Flag and configuration still come before the budget.
    let mut off = settings();
    off.progress_ingest = false;
    let off = configured_router(lazy_pool(), &off, identities());
    assert_eq!(
        post(&off, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, &big)
            .await
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn items_over_budget_peer_gets_429_before_size_checks() {
    let app = configured_router(lazy_pool(), &settings(), identities());
    for _ in 0..120 {
        assert_eq!(
            post(&app, ITEMS_SAN, http::RECORDS_PATH, b"{}")
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    let big = vec![b' '; 65537];
    assert_too_many(&post(&app, ITEMS_SAN, http::RECORDS_PATH, &big).await, "60");
    assert_eq!(
        post(&app, STRANGER_SAN, http::RECORDS_PATH, &big)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

/// Ownership adapter that holds every caller until the test opens it.
#[derive(Debug)]
struct Gate {
    entered: std::sync::atomic::AtomicUsize,
    arrived: tokio::sync::Notify,
    open: tokio::sync::Semaphore,
}
impl Authority for Gate {
    fn acquire<'a>(&'a self, _: &'a Scope) -> LeaseFuture<'a> {
        Box::pin(async move {
            self.entered
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.arrived.notify_one();
            let _permit = self.open.acquire().await;
            Err(Error::Unavailable)
        })
    }
}

/// G-11 §3: 32 in-flight exchanges per peer; the 33rd is 429 with Retry-After 1.
#[tokio::test]
async fn progress_allows_32_exchanges_in_flight() {
    let gate = Arc::new(Gate {
        entered: std::sync::atomic::AtomicUsize::new(0),
        arrived: tokio::sync::Notify::new(),
        open: tokio::sync::Semaphore::new(0),
    });
    let app = router(
        lazy_pool(),
        gate.clone(),
        Arc::new(FixedClock(PROGRESS_NOW)),
        &settings(),
        identities(),
    );
    let stamina =
        progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]).as_bytes();
    let held: Vec<_> = (0..32)
        .map(|_| {
            let app = app.clone();
            let mut request = Request::post(http::PROGRESS_RECORD_PATH)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, stamina.len())
                .body(Body::from(stamina))
                .unwrap();
            request
                .extensions_mut()
                .insert(VerifiedClient::from_verified_tls13_http11(
                    Identity::san(PROGRESS_SAN).unwrap(),
                ));
            tokio::spawn(async move { app.oneshot(request).await.unwrap().status() })
        })
        .collect();
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while gate.entered.load(std::sync::atomic::Ordering::SeqCst) < 32 {
            gate.arrived.notified().await;
        }
    })
    .await
    .expect("32 exchanges reach the authority");
    assert_too_many(
        &post(&app, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, stamina).await,
        "1",
    );
    assert_eq!(gate.entered.load(std::sync::atomic::Ordering::SeqCst), 32);
    gate.open.add_permits(1);
    for task in held {
        assert_eq!(task.await.unwrap(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

/// G-11 §3: 60 progress records per minute per account; another account keeps
/// its own window and the refused record changes nothing.
#[tokio::test]
async fn postgres_progress_records_are_limited_per_account() {
    let Some(db) = Db::open().await else {
        return;
    };
    let stamina = progress(&["PublishGameJournalProgressV1", "\"source_revision\":\"1\""]);
    let other = stamina
        .replace(
            "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6",
            "0190f2a1-3b4c-7d5e-8f60-718293a4b5c7",
        )
        .replace(
            "01934f10-7c04-7001-805b-3b1122334403",
            "01934f10-7c04-7001-805b-3b11223344a3",
        );
    let allowed = Allowed::default();
    for body in [stamina, other.as_str()] {
        allowed
            .allow(playerdata::progress(body.as_bytes()).unwrap().scope)
            .await;
    }
    let app = router(
        db.pool.clone(),
        Arc::new(allowed),
        Arc::new(FixedClock(PROGRESS_NOW)),
        &settings(),
        identities(),
    );
    for i in 0..60 {
        let a = post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            stamina.as_bytes(),
        )
        .await;
        assert_eq!(a.status, StatusCode::OK, "{i}");
    }
    assert_too_many(
        &post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_RECORD_PATH,
            stamina.as_bytes(),
        )
        .await,
        "60",
    );
    let a = post(
        &app,
        PROGRESS_SAN,
        http::PROGRESS_RECORD_PATH,
        other.as_bytes(),
    )
    .await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(db.records().await, 2);
    db.close().await;
}

/// G-10 §3 / G-11 §3: one `source_authority` configured for both purposes is an
/// overlapping binding, so both purposes stay unavailable.
#[tokio::test]
async fn shared_source_authority_disables_both_purposes() {
    let mut shared = settings();
    shared.sources.progress = shared.sources.items.clone();
    let app = configured_router(lazy_pool(), &shared, identities());
    for (san, path) in [
        (ITEMS_SAN, http::RECORDS_PATH),
        (ITEMS_SAN, http::CURSOR_PATH),
        (PROGRESS_SAN, http::PROGRESS_RECORD_PATH),
        (PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH),
    ] {
        assert_eq!(
            post(&app, san, path, b"{}").await.status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{path}"
        );
    }
}

/// The progress purpose stays off without a valid source authority (a contract
/// `key`: 1-128 bytes, no space), like the items purpose.
#[tokio::test]
async fn unbound_or_invalid_source_authority_keeps_its_purpose_off() {
    let long = "a".repeat(129);
    let edge = "a".repeat(128);
    for (progress_side, value, on) in [
        (true, "", false),
        (true, "oteryn journal", false),
        (true, long.as_str(), false),
        (true, edge.as_str(), true),
        (false, "OTERYN GAME", false),
        (false, long.as_str(), false),
        (false, edge.as_str(), true),
    ] {
        let mut s = settings();
        let (san, paths) = if progress_side {
            s.sources.progress = value.to_owned();
            (
                PROGRESS_SAN,
                [http::PROGRESS_RECORD_PATH, http::PROGRESS_WATERMARK_PATH],
            )
        } else {
            s.sources.items = value.to_owned();
            (ITEMS_SAN, [http::RECORDS_PATH, http::CURSOR_PATH])
        };
        let app = configured_router(lazy_pool(), &s, identities());
        for path in paths {
            assert_eq!(
                post(&app, san, path, b"{}").await.status,
                if on {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                },
                "{path} {value:?}"
            );
        }
    }
}

/// Each route's byte limit: the limit itself reaches the codec (400), one byte
/// more is 413.
#[tokio::test]
async fn each_route_answers_413_one_byte_past_its_limit() {
    for (san, path, max) in [
        (ITEMS_SAN, http::RECORDS_PATH, 65536),
        (ITEMS_SAN, http::CURSOR_PATH, 1024),
        (PROGRESS_SAN, http::PROGRESS_RECORD_PATH, 262144),
        (PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, 1024),
    ] {
        let app = configured_router(lazy_pool(), &settings(), identities());
        assert_eq!(
            post(&app, san, path, &vec![b' '; max]).await.status,
            StatusCode::BAD_REQUEST,
            "{path} at {max}"
        );
        assert_eq!(
            post(&app, san, path, &vec![b' '; max + 1]).await.status,
            StatusCode::PAYLOAD_TOO_LARGE,
            "{path} past {max}"
        );
    }
}
