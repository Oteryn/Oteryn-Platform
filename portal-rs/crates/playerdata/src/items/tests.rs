#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::{model::*, store::*, web};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

// SOURCE: synthetic mappings from G-10 revision 2, Platform proposal; no Game
// fixture is copied. Read-only Game reference: 857f15f376907c592bb28d854fccff18a15cdf87.
const NOW: u64 = 1793836800;
fn ident(n: u64) -> String {
    format!("019a0000-0000-7000-8000-{n:012x}")
}
fn actor() -> Uuid {
    Uuid::parse_str(&ident(1)).unwrap()
}
fn world() -> Uuid {
    Uuid::parse_str(&ident(2)).unwrap()
}
fn character() -> Uuid {
    Uuid::parse_str(&ident(3)).unwrap()
}
fn cut() -> Cut {
    Cut {
        character_revision: "1".into(),
        from_projection_revision: "1".into(),
        to_projection_revision: "1".into(),
        projection_epoch: "1".into(),
        recovery_generation: "1".into(),
    }
}
fn item(n: u64) -> Item {
    Item {
        item_instance_id: ident(n),
        definition_family: "ITEM".into(),
        definition_production_key: "oteryn:content.item.backpack".into(),
        definition_revision_ref: "rev-1".into(),
        quantity: 1,
        root: Root::Equipment,
        town_id: None,
        depot_box: None,
        house_id: None,
        spatial_position_hex: None,
        equipment_slot: Some(3),
        parent_item_instance_id: None,
        placement_ordinal: n.to_string(),
    }
}
fn record(revision: u64, payload: Payload) -> Record {
    let (family, kind) = match payload {
        Payload::Begin(_) => ("ITEMS", "ITEMS_BEGIN"),
        Payload::Page(_) => ("ITEMS", "ITEMS_PAGE"),
        Payload::End(_) => ("ITEMS", "ITEMS_END"),
        Payload::Bank(_) => ("BANK", "BANK_ENTRY"),
        Payload::Market(_) => ("MARKET", "MARKET_EVENT"),
        Payload::Trade(_) => ("PLAYER_TRADE", "PLAYER_TRADE_EVENT"),
    };
    let bank = family == "BANK";
    Record {
        contract_version: 1,
        operation: "PublishGameJournalRecordV1".into(),
        source_authority: "OTERYN_GAME_JOURNAL".into(),
        account_id: ident(1),
        world_id: ident(2),
        character_id: (!bank).then(|| ident(3)),
        character_revision: (!bank).then(|| "1".into()),
        from_projection_revision: (!bank).then(|| "1".into()),
        to_projection_revision: (!bank).then(|| "1".into()),
        projection_epoch: (!bank).then(|| "1".into()),
        family: family.into(),
        recovery_generation: "1".into(),
        source_revision: revision.to_string(),
        record_id: ident(100 + revision),
        occurred_at: NOW.to_string(),
        coverage_start_at: NOW.to_string(),
        kind: kind.into(),
        payload,
    }
}
fn snapshot(items: Vec<Item>) -> Vec<Record> {
    let count = items.len() as u32;
    let pages = items
        .chunks(32)
        .enumerate()
        .map(|(i, items)| Page {
            snapshot_id: ident(5),
            page_index: i as u32,
            items: items.to_vec(),
        })
        .collect::<Vec<_>>();
    let begin = Begin {
        snapshot_id: ident(5),
        character_revision: "1".into(),
        page_count: pages.len() as u32,
        item_count: count,
        depots: vec![0, 10],
        houses: vec![],
    };
    let mut hash = Sha256::new();
    let mut records = vec![record(1, Payload::Begin(begin.clone()))];
    for (i, p) in pages.into_iter().enumerate() {
        hash.update(serde_json::to_vec(&p).unwrap());
        records.push(record(i as u64 + 2, Payload::Page(p)));
    }
    records.push(record(
        records.len() as u64 + 1,
        Payload::End(End {
            snapshot_id: ident(5),
            page_count: begin.page_count,
            item_count: count,
            pages_sha256: format!("{:x}", hash.finalize()),
        }),
    ));
    records
}
fn bank() -> Record {
    record(
        1,
        Payload::Bank(Bank {
            operation_occurrence_id: ident(8),
            event_id: ident(9),
            entry_id: ident(10),
            previous_entry_id: None,
            kind: "DEPOSIT".into(),
            direction: "CREDIT".into(),
            amount: "100".into(),
            balance_before: "0".into(),
            balance_after: "100".into(),
        }),
    )
}
fn bytes(r: &Record) -> Vec<u8> {
    serde_json::to_vec(r).unwrap()
}

#[test]
fn canonical_contract_and_invalid_inputs() {
    for r in snapshot(vec![item(11)]).into_iter().chain([bank()]) {
        assert!(Record::parse(&bytes(&r)).is_ok());
    }
    let r = bank();
    let canonical = String::from_utf8(bytes(&r)).unwrap();
    let invalid = [
        format!(" {canonical}"),
        canonical.replace("\"contract_version\":1", "\"contract_version\":2"),
        canonical.replace("\"amount\":\"100\"", "\"amount\":100"),
        canonical.replace("\"balance_after\":\"100\"", "\"balance_after\":\"99\""),
        canonical.replace("\"DEPOSIT\"", "\"FEE\""),
        canonical.replace("\"source_revision\":\"1\"", "\"source_revision\":\"01\""),
        canonical.replace("\"character_id\":null,", ""),
        canonical.replace(
            "\"contract_version\":1,",
            "\"contract_version\":1,\"contract_version\":1,",
        ),
        canonical.replace("\"operation\":", "\"unknown\":true,\"operation\":"),
        canonical.replace("\"account_id\":\"019a", "\"account_id\":\"019A"),
        canonical.replace("\"amount\":\"100\"", "\"amount\":\"1000000000000\""),
        canonical.replace(
            "\"contract_version\":1,\"operation\":\"PublishGameJournalRecordV1\"",
            "\"operation\":\"PublishGameJournalRecordV1\",\"contract_version\":1",
        ),
    ];
    for b in invalid {
        assert!(
            Record::parse(b.as_bytes()).is_err(),
            "accepted malformed bytes"
        );
    }
    assert!(counter("18446744073709551616", false).is_err());
    assert!(Record::parse(&vec![b'a'; 65537]).is_err());
}
#[test]
fn complete_snapshot_digest_and_graph() {
    let mut parent = item(11);
    let mut child = item(12);
    child.parent_item_instance_id = Some(parent.item_instance_id.clone());
    let records = snapshot(vec![parent.clone(), child.clone()]);
    assert!(assemble(&records[..2]).unwrap().is_none());
    assert_eq!(assemble(&records).unwrap().unwrap().items.len(), 2);
    assert!(
        assemble(&snapshot(vec![]))
            .unwrap()
            .unwrap()
            .items
            .is_empty()
    );
    let mut bad = records.clone();
    if let Payload::End(e) = &mut bad[2].payload {
        e.pages_sha256 = "0".repeat(64);
    }
    assert!(assemble(&bad).is_err());
    child.parent_item_instance_id = Some(ident(999));
    assert!(assemble(&snapshot(vec![parent.clone(), child.clone()])).is_err());
    child.parent_item_instance_id = Some(parent.item_instance_id.clone());
    parent.parent_item_instance_id = Some(child.item_instance_id.clone());
    assert!(assemble(&snapshot(vec![parent, child])).is_err());
    let mut bad = records.clone();
    bad[2].projection_epoch = Some("2".into());
    assert!(assemble(&bad).is_err());
    let mut bad = records.clone();
    bad[2].occurred_at = (NOW + 1).to_string();
    assert!(assemble(&bad).is_err());
    let mut misplaced = item(12);
    misplaced.root = Root::Depot;
    misplaced.equipment_slot = None;
    misplaced.town_id = Some(99);
    misplaced.depot_box = Some(1);
    assert!(assemble(&snapshot(vec![misplaced])).is_err());
    let mut dup = item(12);
    dup.placement_ordinal = "11".into();
    assert!(assemble(&snapshot(vec![item(11), dup])).is_err());
}
#[test]
fn roots_depth_and_house_fail_closed() {
    let mut rows = vec![];
    for n in 11..44 {
        let mut i = item(n);
        if n > 11 {
            i.parent_item_instance_id = Some(ident(n - 1));
        }
        rows.push(i);
    }
    assert!(assemble(&snapshot(rows)).is_err());
    let mut i = item(11);
    i.root = Root::Inbox;
    i.equipment_slot = None;
    assert!(Record::parse(&bytes(&snapshot(vec![i.clone()])[1])).is_ok());
    i.root = Root::House;
    i.house_id = Some("oteryn:content.house.example".into());
    assert!(Record::parse(&bytes(&snapshot(vec![i])[1])).is_err());
    let mut r = snapshot(vec![]).remove(0);
    if let Payload::Begin(b) = &mut r.payload {
        b.houses.push("oteryn:content.house.example".into());
    }
    assert!(Record::parse(&bytes(&r)).is_err());
}
#[test]
fn market_and_trade_validation() {
    let m = Market {
        event_id: ident(12),
        offer_id: ident(13),
        side: "SELL".into(),
        state: "FILLED".into(),
        end_state: None,
        definition_family: "ITEM".into(),
        definition_production_key: "oteryn:content.item.gold_coin".into(),
        definition_revision_ref: "rev-1".into(),
        amount: 10,
        remaining: 0,
        piece_price: "2".into(),
        fee_gold: "0".into(),
        held_credit_gold: "0".into(),
    };
    let r = record(1, Payload::Market(m.clone()));
    assert!(Record::parse(&bytes(&r)).is_ok());
    let mut m = m;
    m.held_credit_gold = "20".into();
    assert!(Record::parse(&bytes(&record(1, Payload::Market(m.clone())))).is_err());
    m.state = "CREDIT_HELD".into();
    m.end_state = Some("FILLED".into());
    assert!(Record::parse(&bytes(&record(1, Payload::Market(m)))).is_ok());
    let line = |n| Line {
        item_instance_id: ident(n),
        definition_family: "ITEM".into(),
        definition_production_key: "item.coin".into(),
        definition_revision_ref: "rev-1".into(),
        quantity: 10,
    };
    let mut t = Trade {
        occurrence_id: ident(14),
        transaction_id: ident(15),
        event_id: ident(16),
        given: vec![line(17)],
        received: vec![line(18)],
    };
    assert!(Record::parse(&bytes(&record(1, Payload::Trade(t.clone())))).is_ok());
    t.received[0].quantity = 101;
    assert!(Record::parse(&bytes(&record(1, Payload::Trade(t)))).is_err());
}

#[derive(Debug)]
struct TestEvidence {
    deny: AtomicBool,
    now: std::sync::atomic::AtomicU64,
    unpriced: AtomicBool,
}
impl TestEvidence {
    fn new() -> Self {
        Self {
            deny: AtomicBool::new(false),
            now: NOW.into(),
            unpriced: AtomicBool::new(false),
        }
    }
}
#[async_trait]
impl Evidence for TestEvidence {
    async fn authorize(&self, _: &mut PgConnection, s: &Scope, a: Access) -> Result<Proof> {
        if self.deny.load(Ordering::SeqCst)
            || s.account_id != ident(1)
            || s.world_id != ident(2)
            || s.recovery_generation != "1"
            || (s.family != "BANK"
                && (s.character_id.as_deref() != Some(ident(3).as_str()) || s.cut != Some(cut())))
        {
            return Err(Error::Denied);
        }
        if matches!(a,Access::Read(who) if who!=actor()) {
            return Err(Error::Denied);
        }
        Ok(Proof {
            now: self.now.load(Ordering::SeqCst),
            coverage_start: NOW,
            coverage_through: self.now.load(Ordering::SeqCst),
        })
    }
    async fn definition(&self, _: &mut PgConnection, _: &Item) -> Result<Definition> {
        Ok(Definition {
            maximum_quantity: 100,
            container: true,
            tradeable: true,
            estimated_gold: (!self.unpriced.load(Ordering::SeqCst)).then_some(10),
            price_source: Some("NPC".into()),
        })
    }
    async fn resolve(
        &self,
        _: &mut PgConnection,
        a: Uuid,
        w: Uuid,
        c: Option<Uuid>,
        family: &str,
    ) -> Result<Scope> {
        if a != actor() {
            return Err(Error::Denied);
        }
        Ok(Scope {
            source_authority: "OTERYN_GAME_JOURNAL".into(),
            account_id: a.to_string(),
            world_id: w.to_string(),
            character_id: c.map(|v| v.to_string()),
            cut: c.map(|_| cut()),
            family: family.into(),
            recovery_generation: "1".into(),
        })
    }
}
struct TestDb {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl TestDb {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t76_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {search}")))
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105230112_t76_items.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    fn projection(&self) -> (Projection, Arc<TestEvidence>) {
        let e = Arc::new(TestEvidence::new());
        (
            Projection {
                pool: self.pool.clone(),
                flags: Flags {
                    ingest: true,
                    items: true,
                    history: true,
                    bazaar_shortcut: false,
                },
                evidence: e.clone(),
            },
            e,
        )
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
async fn ingest(p: &Projection, r: &Record) -> Result<()> {
    p.ingest(&bytes(r), "OTERYN_GAME_JOURNAL").await
}

#[tokio::test]
async fn pg_atomic_snapshot_search_valuation_and_restart() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    let records = snapshot(vec![item(11)]);
    ingest(&p, &records[0]).await.unwrap();
    ingest(&p, &records[1]).await.unwrap();
    assert!(matches!(
        p.inventory(actor(), world(), character(), "").await,
        Err(Error::Unavailable)
    ));
    ingest(&p, &records[2]).await.unwrap();
    let restarted = Projection {
        pool: db.pool.clone(),
        flags: p.flags,
        evidence: p.evidence.clone(),
    };
    let v = restarted
        .inventory(actor(), world(), character(), "backpack")
        .await
        .unwrap();
    assert_eq!(v.items.len(), 1);
    assert_eq!(v.estimated_total, Some(10));
    assert!(!v.items[0].bazaar_shortcut);
    assert!(
        restarted
            .inventory(actor(), world(), character(), "missing")
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(matches!(
        p.inventory(actor(), world(), character(), &"x".repeat(129))
            .await,
        Err(Error::Invalid)
    ));
    for lang in ["pl", "en"] {
        let html = web::render_inventory(
            p.inventory(actor(), world(), character(), "")
                .await
                .unwrap(),
            lang,
        );
        assert!(html.contains("label for=\"q\""));
        assert!(html.contains("name=\"viewport\""));
        std::fs::write(format!("/tmp/t76-{lang}.html"), html).unwrap();
    }
    db.close().await;
}
#[tokio::test]
async fn pg_owner_staff_transfer_generation_and_peer_denials() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, e) = db.projection();
    for r in snapshot(vec![item(11)]) {
        ingest(&p, &r).await.unwrap();
    }
    let foreign = Uuid::parse_str(&ident(90)).unwrap();
    assert!(matches!(
        p.inventory(foreign, world(), character(), "").await,
        Err(Error::Denied)
    ));
    // Staff uses the same unprivileged actor path: no role can bypass ownership.
    assert!(matches!(
        p.history(foreign, world(), None, "BANK").await,
        Err(Error::Denied)
    ));
    e.deny.store(true, Ordering::SeqCst);
    assert!(matches!(
        p.inventory(actor(), world(), character(), "").await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        ingest(&p, &snapshot(vec![])[0]).await,
        Err(Error::Denied)
    ));
    e.deny.store(false, Ordering::SeqCst);
    let mut r = bank();
    r.recovery_generation = "2".into();
    assert!(matches!(ingest(&p, &r).await, Err(Error::Denied)));
    assert!(matches!(
        p.ingest(&bytes(&bank()), "OTHER_PURPOSE").await,
        Err(Error::Denied)
    ));
    let mut old = snapshot(vec![]).remove(0);
    old.character_revision = Some("2".into());
    if let Payload::Begin(b) = &mut old.payload {
        b.character_revision = "2".into();
    }
    assert!(matches!(ingest(&p, &old).await, Err(Error::Denied)));
    db.close().await;
}
#[tokio::test]
async fn pg_exact_retry_conflict_gap_and_concurrency() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    let r = bank();
    let (a, b) = tokio::join!(ingest(&p, &r), ingest(&p, &r));
    a.unwrap();
    b.unwrap();
    assert_eq!(
        p.history(actor(), world(), None, "BANK")
            .await
            .unwrap()
            .records
            .len(),
        1
    );
    let mut conflict = r.clone();
    if let Payload::Bank(b) = &mut conflict.payload {
        b.amount = "101".into();
        b.balance_after = "101".into();
    }
    assert!(matches!(ingest(&p, &conflict).await, Err(Error::Conflict)));
    let mut gap = r.clone();
    gap.source_revision = "3".into();
    gap.record_id = ident(103);
    assert!(matches!(ingest(&p, &gap).await, Err(Error::Conflict)));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_t76_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let cursor: String = sqlx::query_scalar("SELECT cursor::text FROM pd_t76_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(cursor, "1");
    db.close().await;
}
#[tokio::test]
async fn pg_bad_end_rolls_back_and_empty_is_proven() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    let rs = snapshot(vec![]);
    ingest(&p, &rs[0]).await.unwrap();
    let mut bad = rs[1].clone();
    if let Payload::End(e) = &mut bad.payload {
        e.pages_sha256 = "0".repeat(64);
    }
    assert!(matches!(ingest(&p, &bad).await, Err(Error::Conflict)));
    assert!(
        p.inventory(actor(), world(), character(), "")
            .await
            .is_err()
    );
    ingest(&p, &rs[1]).await.unwrap();
    let v = p
        .inventory(actor(), world(), character(), "")
        .await
        .unwrap();
    assert!(v.items.is_empty());
    assert_eq!(v.estimated_total, Some(0));
    db.close().await;
}
#[tokio::test]
async fn pg_retention_unpriced_and_replay_floor() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, e) = db.projection();
    let rs = snapshot(vec![item(11)]);
    for r in &rs {
        ingest(&p, r).await.unwrap();
    }
    e.unpriced.store(true, Ordering::SeqCst);
    let v = p
        .inventory(actor(), world(), character(), "")
        .await
        .unwrap();
    assert_eq!(v.estimated_total, None);
    assert_eq!(v.unpriced, 1);
    e.now.store(NOW + RETENTION, Ordering::SeqCst);
    assert!(matches!(
        p.inventory(actor(), world(), character(), "").await,
        Err(Error::Unavailable)
    ));
    p.cleanup(NOW + 61 * 86400).await.unwrap();
    let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT snapshot FROM pd_t76_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(bytes.is_none());
    e.now.store(NOW, Ordering::SeqCst);
    assert!(matches!(ingest(&p, &rs[0]).await, Err(Error::Conflict)));
    db.close().await;
}
#[tokio::test]
async fn pg_bank_chain_account_scope_and_default_off() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (mut p, e) = db.projection();
    let r = bank();
    ingest(&p, &r).await.unwrap();
    let mut next = r.clone();
    next.source_revision = "2".into();
    next.record_id = ident(102);
    if let Payload::Bank(b) = &mut next.payload {
        b.event_id = ident(19);
        b.operation_occurrence_id = ident(18);
        b.entry_id = ident(20);
        b.previous_entry_id = Some(ident(999));
        b.balance_before = "100".into();
        b.balance_after = "200".into();
    }
    assert!(matches!(ingest(&p, &next).await, Err(Error::Conflict)));
    if let Payload::Bank(b) = &mut next.payload {
        b.previous_entry_id = Some(ident(10));
    }
    ingest(&p, &next).await.unwrap();
    assert_eq!(
        p.history(actor(), world(), None, "BANK")
            .await
            .unwrap()
            .records
            .len(),
        2
    );
    assert!(matches!(
        p.history(actor(), world(), Some(character()), "BANK").await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        p.history(actor(), world(), None, "MARKET").await,
        Err(Error::Invalid)
    ));
    // Character ownership becoming unavailable does not rebind an account bank stream.
    let history = p.history(actor(), world(), None, "BANK").await.unwrap();
    assert_eq!(history.coverage_start, NOW);
    e.now.store(NOW + RETENTION, Ordering::SeqCst);
    assert!(
        p.history(actor(), world(), None, "BANK")
            .await
            .unwrap()
            .records
            .is_empty()
    );
    p.flags = Flags::default();
    assert!(matches!(ingest(&p, &r).await, Err(Error::Unavailable)));
    assert!(matches!(
        p.inventory(actor(), world(), character(), "").await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        p.history(actor(), world(), None, "BANK").await,
        Err(Error::Unavailable)
    ));
    db.close().await;
}

#[tokio::test]
async fn pg_private_http_ingest_cursor_and_denials() {
    use super::ingest::{self, CursorRequest, JournalPeer};
    use axum::{
        Extension,
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    let uri = "/internal/v1/playerdata/journal/records";
    let request = |body: Vec<u8>| {
        Request::post(uri)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    };
    let no_peer = ingest::router(p.clone())
        .oneshot(request(bytes(&bank())))
        .await
        .unwrap();
    assert_eq!(no_peer.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(no_peer.headers()["cache-control"], "no-store");
    assert!(
        no_peer
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    let peer = JournalPeer::from_trusted_listener("OTERYN_GAME_JOURNAL".into()).unwrap();
    let app = ingest::router(p.clone()).layer(Extension(peer));
    let r = app.clone().oneshot(request(bytes(&bank()))).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        &r.into_body().collect().await.unwrap().to_bytes()[..],
        br#"{"contract_version":1,"result":"accepted"}"#
    );
    let r = app.clone().oneshot(request(b"{}".to_vec())).await.unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert!(r.into_body().collect().await.unwrap().to_bytes().is_empty());
    let req = CursorRequest {
        contract_version: 1,
        operation: "ReadGameJournalCursorV1".into(),
        source_authority: "OTERYN_GAME_JOURNAL".into(),
        account_id: ident(1),
        world_id: ident(2),
        character_id: None,
        character_revision: None,
        from_projection_revision: None,
        to_projection_revision: None,
        projection_epoch: None,
        family: "BANK".into(),
        recovery_generation: "1".into(),
    };
    let r = app
        .clone()
        .oneshot(
            Request::post("/internal/v1/playerdata/journal/cursor")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["after_source_revision"], "1");
    assert_eq!(v["last_record_id"], ident(101));
    let r = app.oneshot(request(vec![b'x'; 65537])).await.unwrap();
    assert_eq!(r.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let mut disabled = p;
    disabled.flags = Flags::default();
    let r = ingest::router(disabled.clone())
        .oneshot(request(vec![]))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        web::router(disabled)
            .oneshot(Request::get("/en/panel/items").body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    db.close().await;
}

#[tokio::test]
async fn pg_browser_owner_foreign_and_unknown_evidence() {
    use axum::{
        Extension, Router,
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use http_body_util::BodyExt;
    use oteryn_web::session::{BrowserSession, SessionLayer};
    use tower::ServiceExt;
    async fn login(
        Extension(session): Extension<BrowserSession>,
        State(who): State<Uuid>,
    ) -> StatusCode {
        oteryn_web::session::rotate(&session, oteryn_web::session::PrivilegeChange::Login(who))
            .await
            .unwrap();
        StatusCode::OK
    }
    async fn cookie(pool: PgPool, who: Uuid) -> String {
        let login = Router::new()
            .route("/login", get(login))
            .route_layer(middleware::from_fn_with_state(
                SessionLayer::new(pool),
                oteryn_web::session::middleware,
            ))
            .with_state(who);
        let r = login
            .oneshot(Request::get("/login").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        r.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, e) = db.projection();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&db.pool)
    .await
    .unwrap();
    for r in snapshot(vec![item(11)]) {
        ingest(&p, &r).await.unwrap();
    }
    let uri = format!("/en/panel/items/{}/{}", world(), character());
    let app = web::router(p);
    let owner = cookie(db.pool.clone(), actor()).await;
    let foreign = cookie(db.pool.clone(), Uuid::parse_str(&ident(99)).unwrap()).await;
    let request = |cookie: &str| {
        Request::get(&uri)
            .header("cookie", cookie)
            .body(Body::empty())
            .unwrap()
    };
    let r = app.clone().oneshot(request(&owner)).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["cache-control"], "private, no-store");
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert!(
        std::str::from_utf8(&body)
            .unwrap()
            .contains("Items at logout")
    );
    let r = app.clone().oneshot(request(&foreign)).await.unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert!(!std::str::from_utf8(&body).unwrap().contains("backpack"));
    let r = app.clone().oneshot(request("")).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    e.deny.store(true, Ordering::SeqCst);
    let r = app.oneshot(request(&owner)).await.unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(r.headers()["cache-control"], "private, no-store");
    db.close().await;
}

#[tokio::test]
async fn pg_erasure_fences_character_and_preserves_account_bank() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    for r in snapshot(vec![item(11)]) {
        ingest(&p, &r).await.unwrap();
    }
    let mut bank = bank();
    bank.record_id = ident(150);
    ingest(&p, &bank).await.unwrap();
    p.erase(actor(), Some(character())).await.unwrap();
    assert!(matches!(
        p.inventory(actor(), world(), character(), "").await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        p.history(actor(), world(), None, "BANK")
            .await
            .unwrap()
            .records
            .len(),
        1
    );
    assert!(matches!(
        ingest(&p, &snapshot(vec![item(11)])[0]).await,
        Err(Error::Conflict)
    ));
    p.erase(actor(), None).await.unwrap();
    assert!(matches!(
        p.history(actor(), world(), None, "BANK").await,
        Err(Error::Unavailable)
    ));
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_t76_records WHERE canonical_bytes IS NOT NULL")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    db.close().await;
}

#[tokio::test]
async fn pg_multiple_pages_market_and_trade_are_not_wealth() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (p, _) = db.projection();
    for r in snapshot((11..44).map(item).collect()) {
        ingest(&p, &r).await.unwrap();
    }
    let mut market = record(
        1,
        Payload::Market(Market {
            event_id: ident(212),
            offer_id: ident(213),
            side: "SELL".into(),
            state: "CREDIT_HELD".into(),
            end_state: Some("FILLED".into()),
            definition_family: "ITEM".into(),
            definition_production_key: "item.coin".into(),
            definition_revision_ref: "rev-1".into(),
            amount: 10,
            remaining: 0,
            piece_price: "2".into(),
            fee_gold: "0".into(),
            held_credit_gold: "20".into(),
        }),
    );
    market.record_id = ident(210);
    ingest(&p, &market).await.unwrap();
    ingest(&p, &market).await.unwrap();
    let line = |n| Line {
        item_instance_id: ident(n),
        definition_family: "ITEM".into(),
        definition_production_key: "item.coin".into(),
        definition_revision_ref: "rev-1".into(),
        quantity: 10,
    };
    let mut trade = record(
        1,
        Payload::Trade(Trade {
            occurrence_id: ident(214),
            transaction_id: ident(215),
            event_id: ident(216),
            given: vec![line(217)],
            received: vec![line(218)],
        }),
    );
    trade.record_id = ident(211);
    ingest(&p, &trade).await.unwrap();
    assert_eq!(
        p.history(actor(), world(), Some(character()), "MARKET")
            .await
            .unwrap()
            .records
            .len(),
        1
    );
    assert_eq!(
        p.history(actor(), world(), Some(character()), "PLAYER_TRADE")
            .await
            .unwrap()
            .records
            .len(),
        1
    );
    assert_eq!(
        p.inventory(actor(), world(), character(), "")
            .await
            .unwrap()
            .estimated_total,
        Some(330)
    );
    assert_eq!(
        p.inventory(actor(), world(), character(), "")
            .await
            .unwrap()
            .items
            .len(),
        33
    );
    db.close().await;
}
