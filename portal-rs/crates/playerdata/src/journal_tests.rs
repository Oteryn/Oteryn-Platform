//! T72 ingest through the composed routes, byte for byte against the golden
//! payloads of the accepted G-10 and G-11 revision 3 contracts (the contracts
//! carry their goldens inline; there is no separate fixture directory).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::store::{Error, Lease, LeaseFuture, TrustedTime};
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
    fn trusted_time(&self) -> std::result::Result<TrustedTime, Error> {
        Ok(TrustedTime {
            unix_ms: self.0,
            uncertainty_ms: 0,
        })
    }
}
/// Stand-in for the accepted ownership adapter: allows exactly the listed scopes.
#[derive(Debug, Clone, Default)]
struct Allowed(Arc<RwLock<Vec<Scope>>>);
#[derive(Debug)]
struct AllowedLease;
impl Lease for AllowedLease {
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
        for migration in [
            include_str!("../../../migrations/20261105230108_t72_private_journal.sql"),
            include_str!("../../../migrations/20261112090003_t72_game_journal.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
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
    assert_eq!(blocks(PROGRESS_CONTRACT).len(), 21);
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
    for (family, entity, needle) in [
        ("stamina", "stamina", "\"stamina_minutes\":2521"),
        ("skull", "skull", "\"skull\":\"BLACK\""),
        (
            "game_login",
            "01934f10-7c04-7001-805b-3b1122334413",
            "\"ip_address\"",
        ),
        ("quest", "quest/example", "\"value\":\"02\""),
    ] {
        let head = head
            .replace(
                "\"family\":\"stamina\"",
                &format!("\"family\":\"{family}\""),
            )
            .replace(
                "\"entity_key\":\"stamina\"",
                &format!("\"entity_key\":\"{entity}\""),
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
