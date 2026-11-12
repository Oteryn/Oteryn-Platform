//! The PostgreSQL admission authority against real ownership rows, through the
//! composed journal routes, with the contract fixtures byte for byte.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::items::store::{Access, Evidence};
use crate::journal::{
    FAMILIES, ITEMS_ENABLED, ITEMS_GROUP, ITEMS_SOURCE, PROGRESS_ENABLED, PROGRESS_GROUP,
    PROGRESS_SOURCE, Settings, router_with,
};
use crate::logins::Authority as _;
use crate::store::{Consumer, Store, http};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

const ITEMS_SAN: &str = "journal-items.oteryn.internal";
const PROGRESS_SAN: &str = "journal-progress.oteryn.internal";
const ITEMS_AUTHORITY: &str = "OTERYN_GAME_JOURNAL";
const PROGRESS_AUTHORITY: &str = "oteryn:journal:primary";
/// G-11 §10 and G-10 goldens' fixed trusted clocks.
const PROGRESS_NOW: u64 = 1790000000000;
const ITEMS_NOW: u64 = 1793836800000 + 3600000;
/// Subjects of the G-11 goldens.
const SELLER: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6";
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334401";
const WORLD: &str = "01934f10-7c04-7001-805b-3b1122334402";
const BUYER: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c7";

fn fixtures(dir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures")
        .join(dir)
}
fn fixture(dir: &str, name: &str) -> String {
    std::fs::read_to_string(fixtures(dir).join(name)).unwrap()
}
fn progress(name: &str) -> String {
    fixture("game-journal-progress-v1", name)
}
fn items(name: &str) -> String {
    fixture("game-journal-items-v1", name)
}

#[test]
fn fixtures_are_the_contract_goldens_byte_for_byte() {
    for (dir, contract, count) in [
        (
            "game-journal-items-v1",
            "OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md",
            9,
        ),
        (
            "game-journal-progress-v1",
            "OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md",
            22,
        ),
    ] {
        let doc = std::fs::read_to_string(fixtures(dir).join("../..").join(contract)).unwrap();
        let goldens: Vec<&str> = doc
            .lines()
            .collect::<Vec<_>>()
            .windows(3)
            .filter(|w| w[0].trim() == "```json" && w[2].trim() == "```")
            .map(|w| w[1])
            .collect();
        let mut seen = 0;
        for entry in std::fs::read_dir(fixtures(dir)).unwrap() {
            let bytes = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            assert!(
                !bytes.ends_with('\n'),
                "{dir}: fixtures carry no trailing newline"
            );
            assert!(
                goldens.contains(&bytes.as_str()),
                "{dir}: {bytes} is not a contract golden"
            );
            seen += 1;
        }
        assert_eq!(seen, count, "{dir}");
    }
}

struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T72 admission PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = oteryn_db::lazy_pool(&url).unwrap();
        let schema = format!("pd_admission_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
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
    async fn exec(&self, sql: &str, binds: &[&str]) {
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()));
        for b in binds {
            q = q.bind(*b);
        }
        q.execute(&self.pool).await.unwrap();
    }
    async fn account(&self, id: &str, epoch: &str, revision: &str) {
        self.exec(
            "INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1::uuid,$1||'@example.test','x','active','en')",
            &[id],
        )
        .await;
        self.exec(
            "INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1::uuid,'test',$2,$3,0,decode(repeat('00',32),'hex'))",
            &[id, epoch, revision],
        )
        .await;
    }
    async fn character(&self, account: &str, character: &str, world: &str) {
        self.exec(
            "INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1::uuid,$2::uuid,$3::uuid,'Hero','AVAILABLE')",
            &[account, character, world],
        )
        .await;
    }
    /// An accepted initial-ownership baseline: (cr, from, to, epoch, recovery).
    async fn baseline(&self, account: &str, character: &str, world: &str, cut: [&str; 5]) {
        self.exec(
            "INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at) VALUES($2::uuid,$1::uuid,$3::uuid,$4::numeric,$5::numeric,$6::numeric,$7::numeric,$8::numeric,now()-interval '1 hour')",
            &[account, character, world, cut[0], cut[1], cut[2], cut[3], cut[4]],
        )
        .await;
    }
    async fn pending_transfer(&self, character: &str) -> Uuid {
        let id = Uuid::now_v7();
        self.exec(
            "INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json) VALUES($1::uuid,$1::uuid,$1,'TransferCharacterOwnership',$2::uuid,$3::uuid,json_build_object('character_id',$4)::text,'')",
            &[&id.to_string(), SELLER, BUYER, character],
        )
        .await;
        id
    }
    /// Commits the transfer receipt and moves the projection row to the buyer.
    async fn commit_transfer(&self, id: Uuid, character: &str) {
        let receipt = format!(
            r#"{{"contract_version":1,"operation":"CharacterCommandReceiptV1","source_authority":"test","command":"TransferCharacterOwnership","operation_id":"{id}","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"{character}","from_account_id":"{SELLER}","to_account_id":"{BUYER}","character_revision":"10","projection_epoch":"1","from_projection_revision":"12","to_projection_revision":"13","recovery_generation":"3","decided_at":"1789999000"}}"#
        );
        self.exec(
            "UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1::uuid",
            &[&id.to_string(), &receipt],
        )
        .await;
        self.exec(
            "UPDATE account_characters SET account_id=$2::uuid WHERE character_id=$1::uuid",
            &[character, BUYER],
        )
        .await;
    }
    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
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
    }
}

#[derive(Debug)]
struct FixedClock(u64);
impl http::Clock for FixedClock {
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
fn settings(on: bool) -> Settings {
    let flag = if on { "true" } else { "false" };
    let values: std::collections::BTreeMap<&str, &str> = [
        (ITEMS_ENABLED, flag),
        (PROGRESS_ENABLED, flag),
        (ITEMS_SOURCE, ITEMS_AUTHORITY),
        (PROGRESS_SOURCE, PROGRESS_AUTHORITY),
        (FAMILIES, "ITEMS,BANK,stamina,game_login"),
    ]
    .into_iter()
    .collect();
    Settings::from_lookup(|key| values.get(key).map(|v| (*v).to_owned()))
}
fn app(db: &Db, clock: Arc<dyn http::Clock>, on: bool) -> Router {
    let settings = settings(on);
    let consumers: Vec<Arc<dyn Consumer>> = vec![Arc::new(crate::journal::login_promoter(
        db.pool.clone(),
        &settings,
    ))];
    router_with(
        db.pool.clone(),
        Arc::new(DbAuthority),
        clock,
        consumers,
        &settings,
        identities(),
    )
}
async fn post(app: &Router, san: &str, path: &str, body: &str) -> (StatusCode, String) {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from(body.to_owned()))
        .unwrap();
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(
            Identity::san(san).unwrap(),
        ));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}
async fn record(app: &Router, body: &str) -> (StatusCode, String) {
    post(app, PROGRESS_SAN, http::PROGRESS_RECORD_PATH, body).await
}
/// The seller owns the golden character at its cut (cr 9, projection 12).
async fn seller(db: &Db) {
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "20").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.baseline(SELLER, CHARACTER, WORLD, ["9", "11", "12", "1", "3"])
        .await;
}
/// The golden stamina record at another revision and record id.
fn stamina_at(revision: &str, suffix: &str) -> String {
    progress("record-stamina.valid.json")
        .replace(
            "\"source_revision\":\"1\"",
            &format!("\"source_revision\":\"{revision}\""),
        )
        .replace("805b-3b1122334403", &format!("805b-3b11223344{suffix}"))
}

#[tokio::test]
async fn postgres_owner_is_admitted_replay_is_idempotent_and_conflict_is_409() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let stamina = progress("record-stamina.valid.json");
    let accepted = progress("response-accepted.valid.json");
    for _ in 0..2 {
        assert_eq!(
            record(&app, &stamina).await,
            (StatusCode::OK, accepted.clone())
        );
    }
    let changed = stamina.replace("\"stamina_minutes\":2400", "\"stamina_minutes\":2401");
    assert_eq!(record(&app, &changed).await.0, StatusCode::CONFLICT);
    let watermark = progress("watermark-baseline.valid.json");
    let ok = post(
        &app,
        PROGRESS_SAN,
        http::PROGRESS_WATERMARK_PATH,
        &watermark,
    )
    .await;
    assert_eq!(ok.0, StatusCode::OK, "{}", ok.1);
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 1);
    db.close().await;
}

#[tokio::test]
async fn postgres_wrong_owner_unproven_owner_and_closed_account_are_refused() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    // The buyer never owned the character: 409, nothing stored.
    let stranger = progress("record-stamina.valid.json").replace(SELLER, BUYER);
    assert_eq!(record(&app, &stranger).await.0, StatusCode::CONFLICT);
    // A stale cut of the real owner is a fence too.
    let stale = progress("record-stamina.valid.json").replace(
        "\"projection_revision\":\"12\"",
        "\"projection_revision\":\"11\"",
    );
    assert_eq!(record(&app, &stale).await.0, StatusCode::CONFLICT);
    // Without a receipt or an accepted baseline nothing proves ownership: 503.
    db.exec("DELETE FROM pd_ownership_baselines", &[]).await;
    let stamina = progress("record-stamina.valid.json");
    assert_eq!(
        record(&app, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // A closed account is a fence.
    db.baseline(SELLER, CHARACTER, WORLD, ["9", "11", "12", "1", "3"])
        .await;
    db.exec(
        "UPDATE accounts SET status='closed' WHERE id=$1::uuid",
        &[SELLER],
    )
    .await;
    assert_eq!(record(&app, &stamina).await.0, StatusCode::CONFLICT);
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

#[tokio::test]
async fn postgres_pending_transfer_then_committed_transfer_moves_every_read() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let stamina = progress("record-stamina.valid.json");
    assert_eq!(record(&app, &stamina).await.0, StatusCode::OK);
    let transfer = db.pending_transfer(CHARACTER).await;
    // Pending: ambiguous, denied for everyone, nothing stored.
    assert_eq!(
        record(&app, &stamina_at("2", "b3")).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let login = JournalLoginAuthority {
        source: PROGRESS_AUTHORITY.into(),
    };
    let evidence = JournalEvidence {
        source: ITEMS_AUTHORITY.into(),
    };
    let (seller_id, buyer_id, character, world) = (
        Uuid::parse_str(SELLER).unwrap(),
        Uuid::parse_str(BUYER).unwrap(),
        Uuid::parse_str(CHARACTER).unwrap(),
        Uuid::parse_str(WORLD).unwrap(),
    );
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        evidence
            .resolve(&mut tx, seller_id, world, Some(character), "ITEMS")
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();

    db.commit_transfer(transfer, CHARACTER).await;
    // The seller's replay meets the transfer fence; a new seller record is the
    // terminal refusal of the closed character scope.
    assert_eq!(record(&app, &stamina).await.0, StatusCode::CONFLICT);
    let (status, body) = record(&app, &stamina_at("2", "b3")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("\"result\":\"refused\"") && body.contains("CHARACTER_SCOPE_DELETED"),
        "{body}"
    );
    // The buyer is admitted at the new cut only.
    let buyer = |pr: &str, cr: &str| {
        progress("record-stamina.valid.json")
            .replace(SELLER, BUYER)
            .replace(
                "\"projection_revision\":\"12\"",
                &format!("\"projection_revision\":\"{pr}\""),
            )
            .replace(
                "\"character_revision\":\"9\"",
                &format!("\"character_revision\":\"{cr}\""),
            )
            .replace("805b-3b1122334403", "805b-3b11223344c3")
    };
    assert_eq!(
        record(&app, &buyer("12", "9")).await.0,
        StatusCode::CONFLICT
    );
    let (status, body) = record(&app, &buyer("13", "10")).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        login.current(&mut tx, seller_id, Some(character)).await,
        Err(crate::logins::Error::Denied)
    ));
    let err = evidence
        .resolve(&mut tx, seller_id, world, Some(character), "ITEMS")
        .await
        .unwrap_err();
    assert!(matches!(err, crate::items::Error::Denied));
    let scope = evidence
        .resolve(&mut tx, buyer_id, world, Some(character), "ITEMS")
        .await
        .unwrap();
    let cut = scope.cut.clone().unwrap();
    assert_eq!(
        (
            cut.character_revision.as_str(),
            cut.to_projection_revision.as_str()
        ),
        ("10", "13")
    );
    // The seller's old cut is no longer current for anyone.
    let mut old = scope.clone();
    old.cut.as_mut().unwrap().to_projection_revision = "12".into();
    assert!(matches!(
        evidence
            .authorize(&mut tx, &old, Access::Read(buyer_id))
            .await,
        Err(crate::items::Error::Denied)
    ));
    assert!(matches!(
        evidence
            .authorize(&mut tx, &scope, Access::Read(seller_id))
            .await,
        Err(crate::items::Error::Denied)
    ));
    // The current owner is proven, yet without a verified G-10 baseline: 503.
    assert!(matches!(
        evidence
            .authorize(&mut tx, &scope, Access::Read(buyer_id))
            .await,
        Err(crate::items::Error::Unavailable)
    ));
    tx.rollback().await.unwrap();
    db.close().await;
}

#[tokio::test]
async fn postgres_deleted_scope_gets_the_revision_4_refusal() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let store = Store::new(
        db.pool.clone(),
        Arc::new(DbAuthority),
        settings(true).flags(),
    );
    store
        .erase_character(
            SELLER,
            CHARACTER,
            TrustedTime {
                unix_ms: PROGRESS_NOW,
                uncertainty_ms: 0,
            },
        )
        .await
        .unwrap();
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let refused = progress("response-character-scope-deleted.json");
    for _ in 0..2 {
        assert_eq!(
            record(&app, &progress("record-stamina.valid.json")).await,
            (StatusCode::OK, refused.clone())
        );
    }
    db.close().await;
}

#[tokio::test]
async fn postgres_clock_uncertainty_and_disabled_flag_store_nothing() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let stamina = progress("record-stamina.valid.json");
    // A local clock 10 s away from the database exceeds the 5 s G-11 bound.
    let skewed = app(
        &db,
        Arc::new(DbClock::skewed(db.pool.clone(), 10_000)),
        true,
    );
    assert_eq!(
        record(&skewed, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let time = DbClock::new(db.pool.clone()).trusted_time().await.unwrap();
    assert!(time.uncertainty_ms <= 5000, "{}", time.uncertainty_ms);
    let off = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), false);
    assert_eq!(
        record(&off, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let production =
        crate::journal::configured_router(db.pool.clone(), &settings(false), identities());
    assert_eq!(
        record(&production, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

#[tokio::test]
async fn postgres_items_cut_must_equal_the_ownership_cut() {
    let Some(db) = Db::open().await else {
        return;
    };
    let (account, world, character) = (
        "019a0000-0000-7000-8000-000000000001",
        "019a0000-0000-7000-8000-000000000002",
        "019a0000-0000-7000-8000-000000000003",
    );
    db.account(account, "1", "1").await;
    db.character(account, character, world).await;
    let app = app(&db, Arc::new(FixedClock(ITEMS_NOW)), true);
    let begin = items("record-items-begin.valid.json");
    let ack = r#"{"contract_version":1,"result":"accepted"}"#;
    // No ownership evidence yet: 503.
    let none = post(&app, ITEMS_SAN, http::RECORDS_PATH, &begin).await;
    assert_eq!(none.0, StatusCode::SERVICE_UNAVAILABLE);
    // A baseline at another cut does not prove this one: 409.
    db.baseline(account, character, world, ["2", "1", "1", "1", "1"])
        .await;
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, &begin).await.0,
        StatusCode::CONFLICT
    );
    db.exec(
        "UPDATE pd_ownership_baselines SET character_revision=1",
        &[],
    )
    .await;
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, &begin).await,
        (StatusCode::OK, ack.into())
    );
    // The account bank stream needs no character cut.
    let bank = items("record-bank-entry.valid.json");
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, &bank).await,
        (StatusCode::OK, ack.into())
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 2);
    db.close().await;
}

#[tokio::test]
async fn postgres_game_login_is_promoted_by_a_baseline_watermark_only() {
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let event = "01934f10-7c04-7001-805b-3b1122334413";
    let payload = progress("payload-game-login.valid.json")
        .replace("1790000000000", &(now - 1000).to_string());
    let login = progress("record-stamina.valid.json")
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace("\"entity_key\":\"stamina\"", &format!("\"entity_key\":\"{event}\""))
        .replace("\"occurred_at_unix_ms\":\"1790000000000\"", &format!("\"occurred_at_unix_ms\":\"{}\"", now - 1000))
        .replace(
            r#"{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}"#,
            &payload,
        );
    let watermark = |baseline: bool| {
        progress("watermark-baseline.valid.json")
            .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
            .replace("1790000000000", &now.to_string())
            .replace(
                "\"baseline_complete\":true",
                &format!("\"baseline_complete\":{baseline}"),
            )
    };
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let (status, body) = record(&app, &login).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let logins = "SELECT count(*) FROM pd_game_logins";
    // Silence and an incomplete baseline promote nothing.
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_WATERMARK_PATH,
            &watermark(false)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(db.count(logins).await, 0);
    let (status, body) = post(
        &app,
        PROGRESS_SAN,
        http::PROGRESS_WATERMARK_PATH,
        &watermark(true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(db.count(logins).await, 1);
    let mut tx = db.pool.begin().await.unwrap();
    let cut = JournalLoginAuthority {
        source: PROGRESS_AUTHORITY.into(),
    }
    .current(
        &mut tx,
        Uuid::parse_str(SELLER).unwrap(),
        Some(Uuid::parse_str(CHARACTER).unwrap()),
    )
    .await
    .unwrap();
    assert!(
        cut.baseline_complete && cut.complete_through == 1 && cut.projection_revision == Some(12)
    );
    tx.rollback().await.unwrap();
    db.close().await;
}
