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
        self.exec("UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$1,source_authority='test',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1", &[epoch]).await;
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
    /// A committed `CreateCharacter` receipt at epoch 1, recovery 3:
    /// (character revision, projection revision, decided at seconds).
    async fn create(&self, account: &str, character: &str, world: &str, cut: [&str; 3]) {
        self.create_at_generation(account, character, world, cut, "3")
            .await;
    }
    async fn create_at_generation(
        &self,
        account: &str,
        character: &str,
        world: &str,
        cut: [&str; 3],
        generation: &str,
    ) {
        let id = Uuid::now_v7();
        let [cr, pr, at] = cut;
        let receipt = format!(
            r#"{{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"test","command":"CreateCharacter","operation_id":"{id}","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"{character}","account_id":"{account}","world_id":"{world}","name":"Hero","character_revision":"{cr}","projection_epoch":"1","projection_revision":"{pr}","recovery_generation":"{generation}","decided_at":"{at}"}}"#
        );
        self.exec(
            "INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,binding_json,intent_json,receipt_json,state) VALUES($1::uuid,$1::uuid,$1,'CreateCharacter',$2::uuid,'{}','',$3,'TERMINAL')",
            &[&id.to_string(), account, &receipt],
        )
        .await;
    }
    /// A committed transfer at epoch 1, recovery 3, moving the projection row:
    /// (character revision, from and to projection revision, decided at seconds).
    async fn transfer(&self, character: &str, from: &str, to: &str, cut: [&str; 4]) {
        let id = Uuid::now_v7();
        let [cr, fpr, tpr, at] = cut;
        let receipt = format!(
            r#"{{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"test","command":"TransferCharacterOwnership","operation_id":"{id}","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"{character}","from_account_id":"{from}","to_account_id":"{to}","character_revision":"{cr}","projection_epoch":"1","from_projection_revision":"{fpr}","to_projection_revision":"{tpr}","recovery_generation":"3","decided_at":"{at}"}}"#
        );
        self.exec(
            "INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json,receipt_json,state) VALUES($1::uuid,$1::uuid,$1,'TransferCharacterOwnership',$2::uuid,$3::uuid,json_build_object('character_id',$4)::text,'',$5,'TERMINAL')",
            &[&id.to_string(), from, to, character, &receipt],
        )
        .await;
        self.exec(
            "UPDATE account_characters SET account_id=$2::uuid WHERE character_id=$1::uuid",
            &[character, to],
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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

/// Another account and a second character of the seller.
const OTHER: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c8";
const KEPT: &str = "01934f10-7c04-7001-805b-3b11223344a5";

fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}
async fn proof(db: &Db, account: &str, character: &str) -> Result<Ownership> {
    let mut tx = db.pool.begin().await.unwrap();
    let proof = ownership(&mut tx, id(account), id(character)).await;
    tx.rollback().await.unwrap();
    proof
}
/// The seller's create cut as the authority reads it.
fn created(cr: u64, decided_at: u64) -> OwnershipCut {
    OwnershipCut {
        account: id(SELLER),
        world: id(WORLD),
        character_revision: cr,
        from_projection_revision: 12,
        to_projection_revision: 12,
        epoch: 1,
        recovery: 3,
        decided_at,
    }
}
/// The golden stamina record of the buyer at its transfer cut (pr 13, cr 10).
fn buyer_stamina() -> String {
    progress("record-stamina.valid.json")
        .replace(SELLER, BUYER)
        .replace(
            "\"projection_revision\":\"12\"",
            "\"projection_revision\":\"13\"",
        )
        .replace(
            "\"character_revision\":\"9\"",
            "\"character_revision\":\"10\"",
        )
}

#[tokio::test]
async fn postgres_create_receipt_is_the_initial_cut_and_its_absence_is_503() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    db.account(SELLER, "1", "12").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let stamina = progress("record-stamina.valid.json");
    // No create receipt and no baseline: unavailable, never a default owner.
    assert!(matches!(
        proof(&db, SELLER, CHARACTER).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        record(&app, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // Another character's creation proves nothing about this one.
    db.character(SELLER, KEPT, WORLD).await;
    db.create(SELLER, KEPT, WORLD, ["9", "12", "1789990000"])
        .await;
    assert_eq!(
        record(&app, &stamina).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.create(SELLER, CHARACTER, WORLD, ["9", "12", "1789990000"])
        .await;
    assert_eq!(
        proof(&db, SELLER, CHARACTER).await.unwrap(),
        Ownership::Owner(created(9, 1789990000))
    );
    assert_eq!(
        proof(&db, BUYER, CHARACTER).await.unwrap(),
        Ownership::Stranger
    );
    assert_eq!(
        record(&app, &stamina).await,
        (StatusCode::OK, progress("response-accepted.valid.json"))
    );
    // A second creation of the same character is ambiguous.
    db.create(OTHER, CHARACTER, WORLD, ["9", "12", "1789990000"])
        .await;
    assert!(matches!(
        proof(&db, SELLER, CHARACTER).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 1);
    db.close().await;
}

#[tokio::test]
async fn postgres_transfers_chain_from_the_create_receipt() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "20").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.create(SELLER, CHARACTER, WORLD, ["9", "12", "1789990000"])
        .await;
    db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", "1789999000"])
        .await;
    assert_eq!(
        proof(&db, BUYER, CHARACTER).await.unwrap(),
        Ownership::Owner(OwnershipCut {
            account: id(BUYER),
            to_projection_revision: 13,
            ..created(10, 1789999000)
        })
    );
    assert_eq!(
        proof(&db, SELLER, CHARACTER).await.unwrap(),
        Ownership::Former
    );
    assert_eq!(
        proof(&db, OTHER, CHARACTER).await.unwrap(),
        Ownership::Stranger
    );
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let (status, body) = record(&app, &buyer_stamina()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    db.close().await;
}

#[tokio::test]
async fn postgres_first_transfer_not_from_the_creator_is_a_broken_chain() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "20").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    // The other account created the character; the seller never received it.
    db.create(OTHER, CHARACTER, WORLD, ["9", "12", "1789990000"])
        .await;
    db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", "1789999000"])
        .await;
    for account in [BUYER, SELLER, OTHER] {
        assert!(matches!(
            proof(&db, account, CHARACTER).await,
            Err(Error::Unavailable)
        ));
    }
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(
        record(&app, &buyer_stamina()).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

/// A `game_login` record of `account` for `character` at its cut (pr, cr).
fn login_record(
    account: &str,
    character: &str,
    cut: [&str; 2],
    revision: u64,
    event: Uuid,
    entered: i64,
) -> String {
    let [pr, cr] = cut;
    let record = Uuid::now_v7();
    format!(
        r#"{{"contract_version":1,"operation":"PublishGameJournalProgressV1","source_authority":"{PROGRESS_AUTHORITY}","account_id":"{account}","character_id":"{character}","world_id":"{WORLD}","projection_revision":"{pr}","projection_epoch":"1","recovery_generation":"3","family":"game_login","source_revision":"{revision}","record_id":"{record}","entity_key":"{event}","character_revision":"{cr}","occurred_at_unix_ms":"{entered}","record_kind":"UPSERT","payload":{{"login_event_id":"{event}","entered_at":"{entered}","left_at":null,"end_reason":null}}}}"#
    )
}
async fn db_now(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
/// A baseline-complete `game_login` watermark of `account`.
async fn login_watermark(app: &Router, db: &Db, account: &str, through: u64) {
    let now = db_now(db).await;
    let body = format!(
        r#"{{"contract_version":1,"operation":"PublishGameJournalProgressWatermarkV1","source_authority":"{PROGRESS_AUTHORITY}","account_id":"{account}","projection_epoch":"1","recovery_generation":"3","family":"game_login","complete_through":"{through}","observed_at_unix_ms":"{now}","baseline_complete":true}}"#
    );
    let (status, body) = post(app, PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, &body).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
/// The login history of `account` as event ids, newest first.
async fn history(db: &Db, account: &str) -> Vec<Uuid> {
    crate::logins::Logins {
        pool: db.pool.clone(),
        flags: crate::logins::Flags {
            progress_ingest_enabled: true,
            progress_read_enabled: true,
            game_login_history_enabled: true,
        },
        authority: Arc::new(JournalLoginAuthority {
            source: PROGRESS_AUTHORITY.into(),
        }),
    }
    .history(id(account))
    .await
    .unwrap()
    .into_iter()
    .map(|entry| entry.login_event_id)
    .collect()
}

#[tokio::test]
async fn postgres_sold_character_history_is_hidden_from_both_sides_of_the_cut() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    let now = db_now(&db).await;
    let secs = now / 1000;
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "20").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.character(SELLER, KEPT, WORLD).await;
    let born = (secs - 100).to_string();
    db.create(SELLER, CHARACTER, WORLD, ["9", "12", &born])
        .await;
    db.create(SELLER, KEPT, WORLD, ["5", "12", &born]).await;
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    // Sessions on the sold character before and after the transfer cut, one on
    // the kept character, and one more on the sold character that is stored
    // before the sale but published only after it.
    let (early, late, kept, unpublished) = (
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    );
    for body in [
        login_record(SELLER, CHARACTER, ["12", "9"], 1, early, now - 60_000),
        login_record(SELLER, CHARACTER, ["12", "9"], 2, late, now - 10_000),
        login_record(SELLER, KEPT, ["12", "5"], 3, kept, now - 30_000),
        login_record(SELLER, CHARACTER, ["12", "9"], 4, unpublished, now - 40_000),
    ] {
        let (status, body) = record(&app, &body).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    login_watermark(&app, &db, SELLER, 3).await;
    assert_eq!(db.count("SELECT count(*) FROM pd_game_logins").await, 3);
    assert_eq!(history(&db, SELLER).await, vec![late, kept, early]);

    // The sale: its cut falls between the early and the late session.
    db.transfer(
        CHARACTER,
        SELLER,
        BUYER,
        ["10", "12", "13", &(secs - 30).to_string()],
    )
    .await;
    // The seller sees no row of the sold character, before or after the cut;
    // the kept character is unaffected and the page does not fail.
    assert_eq!(history(&db, SELLER).await, vec![kept]);
    // A seller record published after the sale is never promoted.
    login_watermark(&app, &db, SELLER, 4).await;
    assert_eq!(db.count("SELECT count(*) FROM pd_game_logins").await, 3);
    assert_eq!(history(&db, SELLER).await, vec![kept]);

    // The buyer's sessions from before its cut never project; after it they do.
    let (before, after) = (Uuid::now_v7(), Uuid::now_v7());
    for body in [
        login_record(BUYER, CHARACTER, ["13", "10"], 1, before, now - 50_000),
        login_record(BUYER, CHARACTER, ["13", "10"], 2, after, now - 5_000),
    ] {
        let (status, body) = record(&app, &body).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    login_watermark(&app, &db, BUYER, 2).await;
    let buyer = history(&db, BUYER).await;
    assert_eq!(buyer, vec![after]);
    for pre_cut in [early, unpublished, before] {
        assert!(!buyer.contains(&pre_cut), "buyer saw pre-cut row {pre_cut}");
    }
    assert_eq!(history(&db, SELLER).await, vec![kept]);

    // A pending transfer leaves ownership unprovable: that character is
    // hidden, the rest of each history stays readable.
    db.pending_transfer(CHARACTER).await;
    assert_eq!(history(&db, SELLER).await, vec![kept]);
    assert!(history(&db, BUYER).await.is_empty());
    db.close().await;
}

#[tokio::test]
async fn postgres_global_feed_invalidation_refuses_admission_and_ownership() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let stamina = progress("record-stamina.valid.json");
    assert_eq!(record(&app, &stamina).await.0, StatusCode::OK);
    for update in [
        "UPDATE account_character_feed SET highest_epoch='2',watermark_epoch=NULL,source_authority=NULL,complete_through=NULL,observed_at=NULL WHERE id=1",
        "UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test',complete_through=0,observed_at=0 WHERE id=1",
        "UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='wrong-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1",
    ] {
        db.exec(update, &[]).await;
        assert_eq!(
            record(&app, &stamina).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        let mut tx = db.pool.begin().await.unwrap();
        assert!(matches!(
            ownership(&mut tx, uuid(SELLER).unwrap(), uuid(CHARACTER).unwrap()).await,
            Err(Error::Unavailable)
        ));
        tx.rollback().await.unwrap();
    }
    db.close().await;
    eprintln!(
        "PG postgres_global_feed_invalidation_refuses_admission_and_ownership {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    super::reads::PROMOTION_BATCH_MAX.store(0, std::sync::atomic::Ordering::Relaxed);
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    let other_character = Uuid::now_v7().to_string();
    db.character(BUYER, &other_character, WORLD).await;
    db.baseline(BUYER, &other_character, WORLD, ["9", "19", "20", "1", "3"])
        .await;
    let now: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let base = progress("record-stamina.valid.json");
    let staging = Store::new(
        db.pool.clone(),
        Arc::new(DbAuthority),
        settings(true).flags(),
    );
    for revision in 1..=257 {
        let event = Uuid::now_v7().to_string();
        let payload = progress("payload-game-login.valid.json")
            .replace("1790000000000", &(now - 1000).to_string())
            .replace("01934f10-7c04-7001-805b-3b1122334413", &event);
        let login = base.replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
            .replace("\"entity_key\":\"stamina\"", &format!("\"entity_key\":\"{event}\""))
            .replace("\"source_revision\":\"1\"", &format!("\"source_revision\":\"{revision}\""))
            .replace("01934f10-7c04-7001-805b-3b1122334403", &Uuid::now_v7().to_string())
            .replace("\"occurred_at_unix_ms\":\"1790000000000\"", &format!("\"occurred_at_unix_ms\":\"{}\"", now-1000))
            .replace(r#"{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}"#, &payload);
        // Accumulated delivery spans admission windows; exercise the durable
        // production store without spending a single HTTP minute's budget.
        staging
            .ingest(
                oteryn_internal_api::playerdata::progress(login.as_bytes()).unwrap(),
                TrustedTime {
                    unix_ms: now as u64,
                    uncertainty_ms: 0,
                },
            )
            .await
            .unwrap();
    }
    // Pause the real promoter after it holds its admission locks. A second
    // account must still ingest under the same source authority.
    db.exec("CREATE FUNCTION pause_login_promotion() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_revision=1 THEN PERFORM pg_advisory_xact_lock(326787645); END IF; RETURN NEW; END $$", &[]).await;
    db.exec("CREATE TRIGGER pause_login_promotion BEFORE INSERT ON pd_game_logins FOR EACH ROW EXECUTE FUNCTION pause_login_promotion()", &[]).await;
    let mut gate = db.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(326787645)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    let watermark = progress("watermark-baseline.valid.json")
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace("\"complete_through\":\"1\"", "\"complete_through\":\"257\"")
        .replace("1790000000000", &now.to_string());
    let publishing = {
        let app = app.clone();
        tokio::spawn(async move {
            post(
                &app,
                PROGRESS_SAN,
                http::PROGRESS_WATERMARK_PATH,
                &watermark,
            )
            .await
        })
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND $1=ANY(pg_blocking_pids(pid)) AND locktype='advisory' AND objid=326787645 AND NOT granted)").bind(blocker_pid).fetch_one(&db.pool).await.unwrap();
            if waiting { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("promoter reached gate");
    assert_eq!(
        db.count("SELECT count(*) FROM pd_game_logins").await,
        0,
        "staging must remain invisible"
    );
    let other = base
        .replace(SELLER, BUYER)
        .replace(CHARACTER, &other_character)
        .replace(
            "\"projection_revision\":\"12\"",
            "\"projection_revision\":\"20\"",
        )
        .replace(
            "01934f10-7c04-7001-805b-3b1122334403",
            &Uuid::now_v7().to_string(),
        );
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(2), record(&app, &other)).await;
    gate.commit().await.unwrap();
    let (status, body) = publishing.await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        result.expect("another account must progress").0,
        StatusCode::OK
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_game_logins").await, 257);
    assert_eq!(
        super::reads::PROMOTION_BATCH_MAX.load(std::sync::atomic::Ordering::Relaxed),
        128,
        "the PostgreSQL consumer must materialize bounded batches"
    );
    db.close().await;
    eprintln!(
        "PG postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_account_stream_recovery_never_regresses() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else {
        return;
    };
    db.account(SELLER, "1", "12").await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW + 10)), true);
    let w = progress("watermark-baseline.valid.json")
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace("\"complete_through\":\"1\"", "\"complete_through\":\"0\"");
    assert_eq!(
        post(&app, PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, &w)
            .await
            .0,
        StatusCode::OK
    );
    let before: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(s) FROM pd_streams s")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let lower = w
        .replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"2\"",
        )
        .replace("1790000000000", "1790000000001");
    assert_eq!(
        post(&app, PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, &lower)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let after: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(s) FROM pd_streams s")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    // Account-scoped records carry no character ownership recovery floor.
    let value = progress("record-stamina.valid.json")
        .replace(&format!("\"character_id\":\"{CHARACTER}\""), "\"character_id\":null")
        .replace(&format!("\"world_id\":\"{WORLD}\""), "\"world_id\":null")
        .replace("\"character_revision\":\"9\"", "\"character_revision\":null")
        .replace("\"projection_revision\":\"12\"", "\"projection_revision\":null")
        .replace("\"family\":\"stamina\"", "\"family\":\"game_login\"")
        .replace("\"entity_key\":\"stamina\"", "\"entity_key\":\"01934f10-7c04-7001-805b-3b1122334413\"")
        .replace(r#"{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}"#, &progress("payload-game-login.valid.json"));
    let lower_record = value.replace(
        "\"recovery_generation\":\"3\"",
        "\"recovery_generation\":\"2\"",
    );
    assert_eq!(record(&app, &lower_record).await.0, StatusCode::CONFLICT);
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    // The first watermark fixes this epoch's generation for both delivery paths.
    let higher_record = value.replace(
        r#""recovery_generation":"3""#,
        r#""recovery_generation":"4""#,
    );
    assert_eq!(record(&app, &higher_record).await.0, StatusCode::CONFLICT);
    let higher_watermark = w.replace(
        r#""recovery_generation":"3""#,
        r#""recovery_generation":"4""#,
    );
    assert_eq!(
        post(
            &app,
            PROGRESS_SAN,
            http::PROGRESS_WATERMARK_PATH,
            &higher_watermark
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    assert_eq!(record(&app, &value).await.0, StatusCode::OK);
    let conflict_record = value.replace("805b-3b1122334403", "805b-3b11223344b3");
    assert_eq!(record(&app, &conflict_record).await.0, StatusCode::CONFLICT);
    assert_eq!(db.count("SELECT count(*) FROM pd_conflicts").await, 1);
    let high: String = sqlx::query_scalar("SELECT recovery_high_water::text FROM pd_streams")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        high, "3",
        "a committed conflict alarm is not accepted recovery evidence"
    );
    assert_eq!(
        post(&app, PROGRESS_SAN, http::PROGRESS_WATERMARK_PATH, &w)
            .await
            .0,
        StatusCode::OK
    );
    db.exec("UPDATE account_character_feed SET highest_epoch='2',watermark_epoch='2',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1", &[]).await;
    db.exec(
        "UPDATE account_character_snapshots SET projection_epoch='2' WHERE account_id=$1::uuid",
        &[SELLER],
    )
    .await;
    let next_epoch = higher_record
        .replace(r#""projection_epoch":"1""#, r#""projection_epoch":"2""#)
        .replace("805b-3b1122334403", "805b-3b11223344b3");
    assert_eq!(record(&app, &next_epoch).await.0, StatusCode::OK);
    db.close().await;
    eprintln!(
        "postgres_account_stream_recovery_never_regresses duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_watermarks_share_the_record_account_budget() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    for mixed in [false, true] {
        let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
        let w = progress("watermark-baseline.valid.json")
            .replace("\"complete_through\":\"1\"", "\"complete_through\":\"0\"");
        for i in 0..60 {
            let (path, body) = if mixed && i % 2 == 1 {
                (
                    http::PROGRESS_RECORD_PATH,
                    progress("record-stamina.valid.json"),
                )
            } else {
                (http::PROGRESS_WATERMARK_PATH, w.clone())
            };
            assert_eq!(
                post(&app, PROGRESS_SAN, path, &body).await.0,
                StatusCode::OK,
                "mixed={mixed} i={i}"
            );
        }
        let mut request = Request::post(http::PROGRESS_WATERMARK_PATH)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, w.len())
            .body(Body::from(w))
            .unwrap();
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(PROGRESS_SAN).unwrap(),
            ));
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    }
    db.close().await;
    eprintln!(
        "postgres_watermarks_share_the_record_account_budget duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pending_lifecycle_erasure_hides_character_before_worker() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else {
        return;
    };
    seller(&db).await;
    db.exec("INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json) VALUES($1::uuid,$2::uuid,1,1,'10','{\"lifecycle\":\"DELETED\"}')", &[SELLER, CHARACTER]).await;
    assert_eq!(db.count("SELECT count(*) FROM pd_erasure_jobs").await, 1);
    assert_eq!(db.count("SELECT count(*) FROM pd_deleted_scopes").await, 0);
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(
        record(&app, &progress("record-stamina.valid.json")).await,
        (
            StatusCode::OK,
            progress("response-character-scope-deleted.json")
        )
    );
    assert_eq!(
        db.count("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .await,
        0
    );
    db.close().await;
    eprintln!(
        "postgres_pending_lifecycle_erasure_hides_character_before_worker duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_01_lifecycle_purges_recent_promoted_logins() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for action in ["close", "delete", "transfer"] {
        let Some(db) = Db::open().await else { return };
        let now = db_now(&db).await;
        db.account(SELLER, "1", "12").await;
        db.account(BUYER, "1", "13").await;
        db.character(SELLER, CHARACTER, WORLD).await;
        db.character(BUYER, KEPT, WORLD).await;
        let born = (now / 1000 - 100).to_string();
        db.create(SELLER, CHARACTER, WORLD, ["9", "12", &born])
            .await;
        db.create(BUYER, KEPT, WORLD, ["5", "13", &born]).await;
        let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
        for (account, character, cut) in
            [(SELLER, CHARACTER, ["12", "9"]), (BUYER, KEPT, ["13", "5"])]
        {
            let body = login_record(account, character, cut, 1, Uuid::now_v7(), now - 1000);
            assert_eq!(record(&app, &body).await.0, StatusCode::OK);
            login_watermark(&app, &db, account, 1).await;
        }
        assert_eq!(db.count("SELECT count(*) FROM pd_game_logins").await, 2);
        match action {
            "close" => db.exec("UPDATE accounts SET status='closed' WHERE id=$1::uuid", &[SELLER]).await,
            "delete" => db.exec("INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json) VALUES($1::uuid,$2::uuid,1,1,'10','{\"lifecycle\":\"DELETED\"}')", &[SELLER, CHARACTER]).await,
            _ => db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", &(now/1000).to_string()]).await,
        }
        crate::retention::Retention(db.pool.clone())
            .expire_batch()
            .await
            .unwrap();
        assert_eq!(db.count("SELECT count(*) FROM pd_erasure_jobs").await, 0);
        assert_eq!(
            db.count("SELECT count(*) FROM pd_game_logins").await,
            1,
            "{action}"
        );
        db.close().await;
    }
    eprintln!(
        "postgres_pd2_01_lifecycle_purges_recent_promoted_logins duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_02_return_transfer_admits_only_new_ownership_cut() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else { return };
    let now = db_now(&db).await;
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "13").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.create(
        SELLER,
        CHARACTER,
        WORLD,
        ["9", "12", &(now / 1000 - 100).to_string()],
    )
    .await;
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let old = login_record(
        SELLER,
        CHARACTER,
        ["12", "9"],
        1,
        Uuid::now_v7(),
        now - 60_000,
    );
    assert_eq!(record(&app, &old).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 1).await;
    db.transfer(
        CHARACTER,
        SELLER,
        BUYER,
        ["10", "12", "13", &(now / 1000 - 30).to_string()],
    )
    .await;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    db.transfer(
        CHARACTER,
        BUYER,
        SELLER,
        ["11", "13", "14", &(now / 1000 - 10).to_string()],
    )
    .await;
    db.exec("UPDATE account_character_snapshots SET projection_revision=CASE WHEN account_id=$1::uuid THEN '14' ELSE '13' END", &[SELLER]).await;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    assert_eq!(record(&app, &old).await.0, StatusCode::CONFLICT);
    let event = Uuid::now_v7();
    let fresh = login_record(SELLER, CHARACTER, ["14", "11"], 2, event, now - 1000);
    assert_eq!(record(&app, &fresh).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 2).await;
    assert_eq!(history(&db, SELLER).await, vec![event]);
    // The G-10 read adapter must likewise reach the current cut, not a stale
    // tombstone. Baseline verification still refuses access independently.
    let mut conn = db.pool.acquire().await.unwrap();
    let evidence = JournalEvidence {
        source: ITEMS_AUTHORITY.into(),
    };
    assert!(
        evidence
            .resolve(
                &mut conn,
                id(SELLER),
                id(WORLD),
                Some(id(CHARACTER)),
                "ITEMS"
            )
            .await
            .is_ok()
    );
    drop(conn);
    db.close().await;
    eprintln!(
        "postgres_pd2_02_return_transfer_admits_only_new_ownership_cut duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_04_hidden_final_page_refuses_stale_empty_history() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else { return };
    let now = db_now(&db).await;
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "13").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.create(
        SELLER,
        CHARACTER,
        WORLD,
        ["9", "12", &(now / 1000 - 100).to_string()],
    )
    .await;
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let body = login_record(
        SELLER,
        CHARACTER,
        ["12", "9"],
        1,
        Uuid::now_v7(),
        now - 1000,
    );
    assert_eq!(record(&app, &body).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 1).await;
    db.transfer(
        CHARACTER,
        SELLER,
        BUYER,
        ["10", "12", "13", &(now / 1000).to_string()],
    )
    .await;
    db.exec("UPDATE pd_streams SET observed_ms=(extract(epoch FROM clock_timestamp())*1000)::bigint-11000", &[]).await;
    let logins = crate::logins::Logins {
        pool: db.pool.clone(),
        flags: crate::logins::Flags {
            progress_ingest_enabled: true,
            progress_read_enabled: true,
            game_login_history_enabled: true,
        },
        authority: Arc::new(JournalLoginAuthority {
            source: PROGRESS_AUTHORITY.into(),
        }),
    };
    assert!(matches!(
        logins.history(id(SELLER)).await,
        Err(crate::logins::Error::Unavailable)
    ));
    let layer = oteryn_web::session::SessionLayer::new(db.pool.clone());
    let routes = crate::logins::web::router(logins, layer.clone()).merge(
        Router::new()
            .route(
                "/test-login",
                axum::routing::get(
                    |axum::Extension(session): axum::Extension<
                        oteryn_web::session::BrowserSession,
                    >| async move {
                        oteryn_web::session::rotate(
                            &session,
                            oteryn_web::session::PrivilegeChange::Login(id(SELLER)),
                        )
                        .await
                        .unwrap();
                        StatusCode::NO_CONTENT
                    },
                ),
            )
            .route_layer(axum::middleware::from_fn_with_state(
                layer,
                oteryn_web::session::middleware,
            )),
    );
    let response = routes
        .clone()
        .oneshot(Request::get("/test-login").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    for path in [
        "/account/game-logins",
        "/en/account/game-logins",
        "/pl/account/game-logins",
    ] {
        let response = routes
            .clone()
            .oneshot(
                Request::get(path)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
    }
    db.close().await;
    eprintln!(
        "postgres_pd2_04_hidden_final_page_refuses_stale_empty_history duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_05_erasure_keeps_only_minimal_replay_records() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for character_only in [true, false] {
        let Some(db) = Db::open().await else { return };
        seller(&db).await;
        let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
        let body = progress("record-stamina.valid.json");
        let accepted = record(&app, &body).await;
        assert_eq!(accepted.0, StatusCode::OK);
        let digest: Vec<u8> = sqlx::query_scalar("SELECT digest FROM pd_records")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        if character_only {
            db.exec(
                "INSERT INTO pd_erasure_jobs(account_id,character_id) VALUES($1,$2)",
                &[SELLER, CHARACTER],
            )
            .await;
        } else {
            db.exec(
                "UPDATE accounts SET status='closed' WHERE id=$1::uuid",
                &[SELLER],
            )
            .await;
        }
        crate::retention::Retention(db.pool.clone())
            .expire_batch()
            .await
            .unwrap();
        assert_eq!(db.count("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL OR entity_key<>'' OR occurred_ms<>0 OR coverage_ms<>0 OR scope_key<>'' OR character_id IS NOT NULL").await, 0);
        let retained: Vec<u8> = sqlx::query_scalar("SELECT digest FROM pd_records")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(digest, retained);
        if character_only {
            assert_eq!(record(&app, &body).await, accepted);
        } else {
            assert_eq!(record(&app, &body).await.0, StatusCode::CONFLICT);
        }
        db.close().await;
    }
    eprintln!(
        "postgres_pd2_05_erasure_keeps_only_minimal_replay_records duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_pd2_02_upgrade_recovers_acknowledged_transfer_purges() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else { return };
    let now = db_now(&db).await;
    db.account(SELLER, "1", "12").await;
    db.account(BUYER, "1", "13").await;
    db.character(SELLER, CHARACTER, WORLD).await;
    db.create(
        SELLER,
        CHARACTER,
        WORLD,
        ["9", "12", &(now / 1000 - 100).to_string()],
    )
    .await;
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let old = login_record(
        SELLER,
        CHARACTER,
        ["12", "9"],
        1,
        Uuid::now_v7(),
        now - 60_000,
    );
    assert_eq!(record(&app, &old).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 1).await;
    db.transfer(
        CHARACTER,
        SELLER,
        BUYER,
        ["10", "12", "13", &(now / 1000 - 30).to_string()],
    )
    .await;
    // Reproduce the pre-upgrade worker's durable tombstone and acknowledged
    // handoff, with the recent login projection still present.
    let store = Store::new(
        db.pool.clone(),
        Arc::new(crate::store::UnavailableAuthority),
        crate::store::Flags::default(),
    );
    store
        .erase_character(
            SELLER,
            CHARACTER,
            TrustedTime {
                unix_ms: now as u64,
                uncertainty_ms: 0,
            },
        )
        .await
        .unwrap();
    db.exec("DELETE FROM pd_erasure_jobs", &[]).await;
    db.exec(
        "ALTER TABLE pd_erasure_jobs DROP COLUMN closed_revision",
        &[],
    )
    .await;
    db.exec(
        "ALTER TABLE pd_deleted_scopes DROP COLUMN closed_revision",
        &[],
    )
    .await;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115120004_fix_a2-03-playerdata_ownership.sql"
    ))
    .execute(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        db.count("SELECT count(*) FROM pd_erasure_jobs WHERE closed_revision=10")
            .await,
        1
    );
    db.transfer(
        CHARACTER,
        BUYER,
        SELLER,
        ["11", "13", "14", &(now / 1000 - 10).to_string()],
    )
    .await;
    db.exec(
        "UPDATE account_character_snapshots SET projection_revision='14' WHERE account_id=$1::uuid",
        &[SELLER],
    )
    .await;
    let event = Uuid::now_v7();
    let fresh = login_record(SELLER, CHARACTER, ["14", "11"], 2, event, now - 1000);
    assert_eq!(
        record(&app, &fresh).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    assert_eq!(db.count("SELECT count(*) FROM pd_game_logins").await, 0);
    assert_eq!(record(&app, &fresh).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 2).await;
    assert_eq!(history(&db, SELLER).await, vec![event]);
    assert_eq!(record(&app, &old).await.0, StatusCode::CONFLICT);
    db.close().await;
    eprintln!(
        "postgres_pd2_02_upgrade_recovers_acknowledged_transfer_purges duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn drc_creation_account_precedes_admission_issuance_fence() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    db.account(SELLER, "1", "1").await;
    let account = Uuid::parse_str(SELLER).unwrap();
    // Retain exactly the existing-account lock used by create_web::submit.
    let mut creation = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR NO KEY UPDATE")
        .bind(account)
        .execute(&mut *creation)
        .await
        .unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *creation)
        .await
        .unwrap();
    let pool = db.pool.clone();
    let admission = tokio::spawn(async move {
        let mut tx = pool.begin().await.unwrap();
        let scope = Scope {
            authority: "test".into(),
            account: account.to_string(),
            world: None,
            character: None,
            character_revision: None,
            projection_revision: None,
            from_projection_revision: None,
            epoch: None,
            recovery: "0".into(),
            family: "ITEMS".into(),
        };
        admit(&mut tx, &scope).await.unwrap();
        tx.commit().await.unwrap();
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND $1=ANY(pg_blocking_pids(pid)))").bind(blocker).fetch_one(&db.pool).await.unwrap();
            if waiting { break }
            tokio::task::yield_now().await;
        }
    }).await.expect("admission never reached the creation account lock");
    // Creation must be able to publish while admission waits for its account.
    let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(22005160000)")
        .fetch_one(&mut *creation)
        .await
        .unwrap();
    creation.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), admission)
        .await
        .expect("admission did not finish after creation")
        .unwrap();
    assert!(
        available,
        "admission retained issuance before account, deadlocking character creation"
    );
    db.close().await;
}

#[tokio::test]
async fn dr5_creation_account_precedes_session_issuance_fence() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    db.account(SELLER, "1", "1").await;
    let account = Uuid::parse_str(SELLER).unwrap();
    // Retain exactly the existing-account lock used by create_web::submit.
    let mut creation = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR NO KEY UPDATE")
        .bind(account)
        .execute(&mut *creation)
        .await
        .unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *creation)
        .await
        .unwrap();
    let pool = db.pool.clone();
    let admission = tokio::spawn(async move {
        let mut tx = pool.begin().await.unwrap();
        session_fence(&mut tx, account).await.unwrap();
        tx.commit().await.unwrap();
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND $1=ANY(pg_blocking_pids(pid)))").bind(blocker).fetch_one(&db.pool).await.unwrap();
            if waiting { break }
            tokio::task::yield_now().await;
        }
    }).await.expect("admission never reached the creation account lock");
    // Creation must be able to publish while admission waits for its account.
    let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(22005160000)")
        .fetch_one(&mut *creation)
        .await
        .unwrap();
    creation.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), admission)
        .await
        .expect("admission did not finish after creation")
        .unwrap();
    assert!(
        available,
        "G-9 session admission retained issuance before account, deadlocking character creation"
    );
    db.close().await;
}

#[tokio::test]
async fn postgres_a3_scrubbed_remove_publication() {
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else { return };
    let now = db_now(&db).await;
    let retained = Uuid::now_v7().to_string();
    db.account(SELLER, "1", "12").await;
    for character in [CHARACTER, retained.as_str()] {
        db.character(SELLER, character, WORLD).await;
        db.create(
            SELLER,
            character,
            WORLD,
            ["9", "12", &(now / 1000 - 100).to_string()],
        )
        .await;
    }
    let app = app(&db, Arc::new(DbClock::new(db.pool.clone())), true);
    let upsert = login_record(
        SELLER,
        CHARACTER,
        ["12", "9"],
        1,
        Uuid::now_v7(),
        now - 1000,
    );
    let remove = format!(
        "{}null}}",
        upsert
            .split_once("\"payload\":")
            .unwrap()
            .0
            .replace("UPSERT", "REMOVE")
            + "\"payload\":"
    );
    assert_eq!(record(&app, &remove).await.0, StatusCode::OK);
    Store::new(
        db.pool.clone(),
        Arc::new(crate::store::UnavailableAuthority),
        crate::store::Flags::default(),
    )
    .erase_character(
        SELLER,
        CHARACTER,
        TrustedTime {
            unix_ms: now as u64,
            uncertainty_ms: 0,
        },
    )
    .await
    .unwrap();
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let event = Uuid::now_v7();
    assert_eq!(
        record(
            &app,
            &login_record(SELLER, &retained, ["12", "9"], 2, event, now - 500)
        )
        .await
        .0,
        StatusCode::OK
    );
    login_watermark(&app, &db, SELLER, 2).await;
    assert_eq!(history(&db, SELLER).await, vec![event]);
    let upsert = login_record(SELLER, &retained, ["12", "9"], 3, event, now);
    let remove = format!(
        "{}null}}",
        upsert
            .split_once("\"payload\":")
            .unwrap()
            .0
            .replace("UPSERT", "REMOVE")
            + "\"payload\":"
    );
    assert_eq!(record(&app, &remove).await.0, StatusCode::OK);
    login_watermark(&app, &db, SELLER, 3).await;
    assert!(history(&db, SELLER).await.is_empty());
    db.close().await;
    eprintln!(
        "postgres_a3_scrubbed_remove_publication duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_g9_returned_owner_honors_transfer_tombstone_cutoff() {
    let db = Db::open().await.unwrap();
    seller(&db).await;
    db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", "1789999000"])
        .await;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let cut: String = sqlx::query_scalar(
        "SELECT closed_revision::text FROM pd_deleted_scopes WHERE scope_digest=$1",
    )
    .bind(crate::store::scope_digest(SELLER, Some(CHARACTER)))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(cut, "10");
    db.transfer(CHARACTER, BUYER, SELLER, ["11", "13", "12", "1789999100"])
        .await;
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    let generation = "019a5000-0000-7000-8000-000000000001";
    db.exec("INSERT INTO pd_g9_streams(source_authority,account_id,stream_generation,admitted,recovery_generation) VALUES('game.test',$1::uuid,$2::uuid,true,3)", &[SELLER, generation]).await;
    for key in ["vocation:knight", "place:cave", "item:ruby"] {
        db.exec(
            "INSERT INTO pd_g9_catalogue_keys VALUES('game.test','fixture.v1',$1)",
            &[key],
        )
        .await;
    }
    let identities = Arc::new(
        IdentityMap::new(vec![Binding {
            identity: Identity::san("sessions.oteryn.internal").unwrap(),
            group: crate::sessions::GROUP.into(),
        }])
        .unwrap(),
    );
    let app = crate::sessions::router(
        db.pool.clone(),
        crate::sessions::Settings {
            ingest: true,
            commands: false,
            source: "game.test".into(),
        },
        identities,
    );
    let doc = fixture("..", "OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md");
    let golden = doc
        .lines()
        .find(|line| {
            line.starts_with(r#"{"contract_version":1,"operation":"PublishGameJournalRecordV1""#)
                && line.contains(r#""record_kind":"HUNT_SEGMENT""#)
        })
        .unwrap();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let body = golden
        .replace("019a5000-0000-7000-8000-000000000002", SELLER)
        .replace("019a5000-0000-7000-8000-000000000005", CHARACTER)
        .replace("019a5000-0000-7000-8000-000000000006", WORLD)
        .replace(
            r#""character_revision":"1""#,
            r#""character_revision":"11""#,
        )
        .replace("1793836800", &(now - 60).to_string())
        .replace("1793836860", &now.to_string());
    let send = |body: String| {
        let app = app.clone();
        async move {
            post(
                &app,
                "sessions.oteryn.internal",
                crate::sessions::RECORDS,
                &body,
            )
            .await
            .0
        }
    };
    assert_eq!(
        send(body.clone()).await,
        StatusCode::OK,
        "returned owner must ingest beyond the transfer tombstone cutoff"
    );
    for revision in ["9", "10"] {
        assert_eq!(
            send(body.replace(
                r#""character_revision":"11""#,
                &format!(r#""character_revision":"{revision}""#)
            ))
            .await,
            StatusCode::CONFLICT
        );
    }
    sqlx::query(
        "INSERT INTO pd_deleted_scopes(scope_digest,deleted_ms,expires_ms) VALUES($1,0,9223372036854775807)",
    )
    .bind(crate::store::scope_digest(SELLER, None))
    .execute(&db.pool)
    .await
    .unwrap();
    assert_eq!(send(body).await, StatusCode::CONFLICT);
    db.close().await;
}

#[tokio::test]
async fn r7_items_recovery_matches_cut() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    db.create_at_generation(account, character, world, ["1", "1", "1789999000"], "1")
        .await;
    let newer = begin.replace(
        r#""recovery_generation":"1""#,
        r#""recovery_generation":"2""#,
    );
    assert_eq!(
        post(&app, ITEMS_SAN, http::RECORDS_PATH, &newer).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

#[tokio::test]
async fn r7_generation_requires_restore() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    seller(&db).await;
    db.exec("INSERT INTO character_restore_notices(restore_event_id,notice_json,new_recovery_generation,acknowledged_at) VALUES(gen_random_uuid(),'{}','00000000000000000003',now())",&[]).await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(
        record(
            &app,
            &progress("record-stamina.valid.json").replace(
                r#""recovery_generation":"3""#,
                r#""recovery_generation":"4""#
            )
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    assert_eq!(db.count("SELECT count(*) FROM pd_streams").await, 0);
    assert_eq!(
        record(&app, &progress("record-stamina.valid.json")).await.0,
        StatusCode::OK
    );
    db.close().await;
}

#[tokio::test]
async fn r7_invalid_seller_snapshot() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    seller(&db).await;
    db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", "1789999000"])
        .await;
    db.exec(
        "UPDATE account_character_snapshots SET invalid=true WHERE account_id=$1::uuid",
        &[SELLER],
    )
    .await;
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(
        record(&app, &buyer_stamina()).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

#[tokio::test]
async fn r7_returned_owner_pending_purge() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    seller(&db).await;
    db.transfer(CHARACTER, SELLER, BUYER, ["10", "12", "13", "1789999000"])
        .await;
    db.transfer(CHARACTER, BUYER, SELLER, ["11", "13", "14", "1789999001"])
        .await;
    assert!(db.count("SELECT count(*) FROM pd_erasure_jobs").await > 0);
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(
        record(&app, &progress("record-stamina.valid.json")).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 0);
    db.close().await;
}

#[tokio::test]
async fn r7_same_epoch_rejects_new_generation_without_restore() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    seller(&db).await;
    assert_eq!(
        db.count("SELECT count(*) FROM character_restore_notices")
            .await,
        0
    );
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    assert_eq!(record(&app, &stamina_at("1", "03")).await.0, StatusCode::OK);
    let next = stamina_at("2", "b3");
    let newer = next.replace(
        r#""recovery_generation":"3""#,
        r#""recovery_generation":"4""#,
    );
    assert_eq!(record(&app, &newer).await.0, StatusCode::CONFLICT);
    assert_eq!(db.count("SELECT count(*) FROM pd_records").await, 1);
    assert_eq!(
        db.count("SELECT recovery_high_water::bigint FROM pd_streams")
            .await,
        3
    );
    assert_eq!(record(&app, &next).await.0, StatusCode::OK);
    db.close().await;
}

#[tokio::test]
async fn r7_deleted_character_absent_from_projection() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    seller(&db).await;
    db.exec("INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json) VALUES($1::uuid,$2::uuid,1,1,'10','{\"lifecycle\":\"DELETED\"}')", &[SELLER, CHARACTER]).await;
    db.exec(
        "DELETE FROM account_characters WHERE character_id=$1::uuid",
        &[CHARACTER],
    )
    .await;
    assert_eq!(
        db.count("SELECT count(*) FROM pd_erasure_jobs WHERE closed_revision IS NULL")
            .await,
        1
    );
    let app = app(&db, Arc::new(FixedClock(PROGRESS_NOW)), true);
    let refused = progress("response-character-scope-deleted.json");
    let delivery = record(&app, &stamina_at("1", "03")).await;
    assert_eq!(delivery.0, StatusCode::OK);
    assert_eq!(delivery.1, refused);
    crate::retention::Retention(db.pool.clone())
        .expire_batch()
        .await
        .unwrap();
    assert_eq!(db.count("SELECT count(*) FROM pd_erasure_jobs").await, 0);
    assert_eq!(
        db.count("SELECT count(*) FROM pd_deleted_scopes WHERE closed_revision IS NULL")
            .await,
        1
    );
    let (status, body) = record(&app, &stamina_at("2", "b3")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("CHARACTER_SCOPE_DELETED"), "{body}");
    assert_eq!(
        db.count("SELECT stored_through::bigint FROM pd_streams")
            .await,
        2
    );
    db.close().await;
}
