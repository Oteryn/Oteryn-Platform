use super::*;
use crate::State;
use crate::moderation::{StaffCommand, execute_staff};
use std::{future::Future, pin::Pin};

async fn item_state(db: &BrowserFixture) -> State {
    sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

async fn item_in(state: State) -> BrowserFixture {
    if state == State::Completed {
        let db = pending_item_delivery().await;
        recovery_tests::item_effect(&db, Command::Deliver, false).await;
        db.runtime.tick().await.unwrap();
        return db;
    }
    let db = BrowserFixture::deposited_item(state != State::Draft).await;
    if !matches!(state, State::Draft | State::Active) {
        sqlx::query("UPDATE bazaar_listings SET state=$2 WHERE id=$1")
            .bind(uid("20"))
            .bind(state)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=100")
        .execute(&db.pool)
        .await
        .unwrap();
    db
}

async fn authorize_seller(db: &BrowserFixture) {
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true)")
        .bind(uid("11"))
        .bind([0u8; 32].as_slice())
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,
                                  idle_expires_at,absolute_expires_at,sudo_until)
         VALUES($1,$2,$3,'fixture',now()+interval '1 hour',now()+interval '1 day',now()+interval '15 minutes')",
    )
    .bind([1u8; 32].as_slice())
    .bind(uid("11"))
    .bind("c".repeat(64))
    .execute(&db.pool)
    .await
    .unwrap();
}

async fn fund_buyer(db: &BrowserFixture) {
    let mut tx = db.pool.begin().await.unwrap();
    let account = uid("13");
    use crate::Reservations;
    CoinReservations {
        authority: db.runtime.authority().clone(),
    }
    .lock_accounts(&mut tx, &[account])
    .await
    .unwrap();
    let op = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,10000)",
    )
    .bind(op)
    .bind(account)
    .execute(&mut *tx)
    .await
    .unwrap();
    oteryn_wallet::credit_in(&mut tx, op, account, 10000)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn staff_suspend(db: &BrowserFixture, listing: Uuid) -> Result<(), crate::Error> {
    let mut tx = db.pool.begin().await?;
    execute_staff(
        &mut tx,
        uid("81"),
        &StaffCommand::Suspend {
            listing,
            reason: "Lifecycle test suspension".into(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn expire_item(db: &BrowserFixture) {
    sqlx::query("UPDATE bazaar_listings SET ends_at=now()-interval '1 second' WHERE id=$1")
        .bind(uid("20"))
        .execute(&db.pool)
        .await
        .unwrap();
}

async fn dispatched(db: &BrowserFixture, listing: Uuid, command: Command) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_game_outbox o JOIN bazaar_authority_dispatch d USING(operation_id)
         WHERE o.listing_id=$1 AND o.command=$2",
    )
    .bind(listing)
    .bind(command.wire())
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListingEvent {
    Bid,
    Cancel,
    Suspend,
}

async fn listing_event(state: State, event: ListingEvent, expected: Result<(), &'static str>) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(state).await;
    let initial_leader: Option<Uuid> =
        sqlx::query_scalar("SELECT leader FROM bazaar_listings WHERE id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let result = match event {
        ListingEvent::Bid => {
            fund_buyer(&db).await;
            db.runtime
                .bid(uid("13"), uid("20"), 10000, uid("14"), Uuid::now_v7())
                .await
                .map_err(|e| e.to_string())
        }
        ListingEvent::Cancel => db
            .runtime
            .control(uid("11"), [1; 32], uid("20"), Uuid::now_v7(), "cancel")
            .await
            .map_err(|e| e.to_string()),
        ListingEvent::Suspend => staff_suspend(&db, uid("20"))
            .await
            .map_err(|e| e.to_string()),
    };
    let effects: (Option<Uuid>, bool, bool) = sqlx::query_as(
        "SELECT l.leader,a.cancel_requested,
                EXISTS(SELECT 1 FROM bazaar_moderation_holds h WHERE h.listing_id=l.id)
         FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id WHERE l.id=$1",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let actual_state = item_state(&db).await;
    db.end().await;
    assert_eq!(actual_state, state);
    assert_eq!(result.as_ref().copied().map_err(String::as_str), expected);
    assert_eq!(
        effects,
        (
            if matches!(event, ListingEvent::Bid) && expected.is_ok() {
                Some(uid("13"))
            } else {
                initial_leader
            },
            matches!(event, ListingEvent::Cancel) && expected.is_ok(),
            matches!(event, ListingEvent::Suspend) && expected.is_ok(),
        )
    );
}

const LISTING_STATES: [State; 7] = [
    State::Draft,
    State::Active,
    State::Matched,
    State::AwaitingDelivery,
    State::Completed,
    State::Cancelled,
    State::Expired,
];
const LISTING_EVENTS: [ListingEvent; 3] = [
    ListingEvent::Bid,
    ListingEvent::Cancel,
    ListingEvent::Suspend,
];
type ListingRow = (State, ListingEvent, Result<(), &'static str>);
const LISTING_ROWS: &[ListingRow] = &[
    (
        State::Draft,
        ListingEvent::Bid,
        Err("bazaar authorization denied"),
    ),
    (State::Draft, ListingEvent::Cancel, Ok(())),
    (State::Draft, ListingEvent::Suspend, Ok(())),
    (State::Active, ListingEvent::Bid, Ok(())),
    (State::Active, ListingEvent::Cancel, Ok(())),
    (State::Active, ListingEvent::Suspend, Ok(())),
    (
        State::Matched,
        ListingEvent::Bid,
        Err("bazaar authorization denied"),
    ),
    (
        State::Matched,
        ListingEvent::Cancel,
        Err("bazaar authorization denied"),
    ),
    (
        State::Matched,
        ListingEvent::Suspend,
        Err("illegal bazaar transition"),
    ),
    (
        State::AwaitingDelivery,
        ListingEvent::Bid,
        Err("bazaar authorization denied"),
    ),
    (
        State::AwaitingDelivery,
        ListingEvent::Cancel,
        Err("bazaar authorization denied"),
    ),
    (
        State::AwaitingDelivery,
        ListingEvent::Suspend,
        Err("illegal bazaar transition"),
    ),
    (
        State::Completed,
        ListingEvent::Bid,
        Err("bazaar game integration unavailable"),
    ),
    (
        State::Completed,
        ListingEvent::Cancel,
        Err("bazaar authorization denied"),
    ),
    (
        State::Completed,
        ListingEvent::Suspend,
        Err("illegal bazaar transition"),
    ),
    (
        State::Cancelled,
        ListingEvent::Bid,
        Err("bazaar authorization denied"),
    ),
    (
        State::Cancelled,
        ListingEvent::Cancel,
        Err("bazaar authorization denied"),
    ),
    (
        State::Cancelled,
        ListingEvent::Suspend,
        Err("illegal bazaar transition"),
    ),
    (
        State::Expired,
        ListingEvent::Bid,
        Err("bazaar authorization denied"),
    ),
    (
        State::Expired,
        ListingEvent::Cancel,
        Err("bazaar authorization denied"),
    ),
    (
        State::Expired,
        ListingEvent::Suspend,
        Err("illegal bazaar transition"),
    ),
];

async fn lifecycle_draft_advance_escrowed() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Draft).await;
    authorize_seller(&db).await;
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    db.end().await;
    assert_eq!(state, State::Active);
}

async fn lifecycle_draft_advance_cancel_requested() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Draft).await;
    db.runtime
        .control(uid("11"), [1; 32], uid("20"), Uuid::now_v7(), "cancel")
        .await
        .unwrap();
    sqlx::query("DELETE FROM bazaar_game_projections WHERE kind='POLICY'")
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    let pending = item_state(&db).await;
    let returns = dispatched(&db, uid("20"), Command::Return).await;
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    let returned = item_state(&db).await;
    db.end().await;
    assert_eq!(pending, State::Draft);
    assert_eq!(returns, 1);
    assert_eq!(returned, State::Cancelled);
}

async fn lifecycle_active_expire_unbid() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    expire_item(&db).await;
    db.runtime.tick().await.unwrap();
    let pending = item_state(&db).await;
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    let returned = item_state(&db).await;
    db.end().await;
    assert_eq!(pending, State::Active);
    assert_eq!(returned, State::Cancelled);
}

async fn lifecycle_active_expire_held() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    expire_item(&db).await;
    staff_suspend(&db, uid("20")).await.unwrap();
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    let returns = dispatched(&db, uid("20"), Command::Return).await;
    db.end().await;
    assert_eq!(state, State::Active);
    assert_eq!(returns, 0);
}

async fn lifecycle_active_expire_funded() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    db.end().await;
    assert_eq!(state, State::AwaitingDelivery);
}

async fn pending_item_delivery() -> BrowserFixture {
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    db
}

async fn lifecycle_awaiting_delivery_commit_delivery() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    recovery_tests::item_effect(&db, Command::Deliver, false).await;
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    let sales: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_settlements WHERE listing_id=$1 AND completed_at IS NOT NULL",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    db.end().await;
    assert_eq!(state, State::Completed);
    assert_eq!(sales, 1);
}

async fn lifecycle_awaiting_delivery_refuse_delivery() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    recovery_tests::item_effect(&db, Command::Deliver, true).await;
    db.runtime.tick().await.unwrap();
    let pending = item_state(&db).await;
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    let returned = item_state(&db).await;
    let remaining: i64 =
        sqlx::query_scalar("SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2")
            .bind(uid("20"))
            .bind(uid("13"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    db.end().await;
    assert_eq!(pending, State::AwaitingDelivery);
    assert_eq!(returned, State::Cancelled);
    assert_eq!(remaining, 0);
}

async fn lifecycle_active_invalidate() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let request = Uuid::now_v7();
    staff_suspend(&db, uid("20")).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    execute_staff(
        &mut tx,
        uid("81"),
        &StaffCommand::RequestInvalidation {
            id: request,
            listing: uid("20"),
            reason: "Lifecycle test invalidation".into(),
        },
    )
    .await
    .unwrap();
    execute_staff(
        &mut tx,
        uid("82"),
        &StaffCommand::ApproveInvalidation { id: request },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let pending = item_state(&db).await;
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    let returned = item_state(&db).await;
    let held: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=$1)",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    db.end().await;
    assert_eq!(pending, State::Active);
    assert_eq!(returned, State::Cancelled);
    assert!(!held);
}

async fn create_item(db: &mut BrowserFixture) -> Uuid {
    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .worlds = vec![uid("12")];
    authorize_seller(db).await;
    sqlx::query("INSERT INTO world_status(id,name) VALUES($1,'Fixture')")
        .bind(uid("12"))
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime
        .create(
            uid("11"),
            [1; 32],
            runtime::NewAuction {
                request_id: Uuid::now_v7(),
                currency: "COINS".into(),
                kind: "item".into(),
                world_id: uid("12"),
                seller_character_id: uid("21"),
                asset_id: uid("71"),
                expected_revision: "7".into(),
                quantity: 1,
                definition_key: "fixture-sword".into(),
                definition_revision: "v1".into(),
                starting_bid: 10000,
                duration_days: 2,
                publish_snapshot: true,
                _csrf: None,
            },
        )
        .await
        .unwrap()
}

async fn lifecycle_absent_create() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Active).await;
    let listing = create_item(&mut db).await;
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let escrow = dispatched(&db, listing, Command::Escrow).await;
    db.end().await;
    assert_eq!(state, State::Draft);
    assert_eq!(escrow, 1);
}

async fn new_character_draft(db: &BrowserFixture) -> PreparedIntent {
    let listing = uid("30");
    sqlx::query(
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at)
         VALUES($1,$2,'character','auction','coins',100,false,now()+interval '1 day')",
    )
    .bind(listing)
    .bind(uid("11"))
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO bazaar_authority_assets
         (listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,
          session_hash,quantity,publication_consent,input_binding)
         VALUES($1,$1,$2,$3,$3,7,$4,1,true,'lifecycle-character')",
    )
    .bind(listing)
    .bind(uid("12"))
    .bind(uid("31"))
    .bind([1u8; 32].as_slice())
    .execute(&db.pool)
    .await
    .unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let intent = character_intent(
        Command::Lock,
        101,
        now,
        vec![
            ("character_id", Value::String(uid("31").to_string())),
            ("expected_character_revision", Value::String("7".into())),
            ("eligibility_revision", Value::String("4".into())),
            ("snapshot_schema_version", Value::Unsigned(1)),
        ],
    );
    store_intent(&mut tx, &intent).await;
    db.runtime
        .authority()
        .publish_intent(
            &mut tx,
            &PlatformAccounts {
                enabled: true,
                session_hash: Some([1; 32]),
            },
            id(text(&intent.value, "operation_id").unwrap()).unwrap(),
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    intent
}

async fn resync(db: &BrowserFixture, a: &Authority) -> Result<(), Error> {
    let mut tx = db.pool.begin().await?;
    a.finish_resync(&mut tx, &[uid("12")]).await?;
    tx.commit().await?;
    Ok(())
}

fn projection_ingest() -> Outbox {
    Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    }
}

async fn rebuild_epoch(db: &BrowserFixture) -> Authority {
    let mut tx = db.pool.begin().await.unwrap();
    let a = db.runtime.authority();
    let now = runtime::now(&mut tx).await.unwrap();
    let projections: Vec<String> = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind IN ('POLICY','STATUS')
         ORDER BY kind",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    for projection in projections {
        let next = projection.replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#);
        assert_eq!(
            projection_ingest()
                .ingest(
                    &mut tx,
                    &a.config.projection_authority,
                    next.as_bytes(),
                    now
                )
                .await
                .unwrap(),
            IngestResult::Applied
        );
    }
    tx.commit().await.unwrap();
    let mut next = a.clone();
    next.fence.projection_epoch = 4;
    next
}

async fn lifecycle_fence_matching_new_epoch() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let next = rebuild_epoch(&db).await;
    let previous = resync(&db, db.runtime.authority()).await;
    let current = resync(&db, &next).await;
    let state = item_state(&db).await;
    let reopened: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox WHERE state='REOPENED'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    db.end().await;
    assert!(matches!(previous, Err(Error::Unavailable)));
    assert!(current.is_ok());
    assert_eq!(state, State::Active);
    assert_eq!(reopened, 1);
}

#[tokio::test]
async fn lifecycle_receipt_committed_replay() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = vector("escrow_receipt");
    let replay = a
        .receive(
            &mut tx,
            &a.config.receipt_authority,
            receipt.as_bytes(),
            now,
        )
        .await;
    let conflict = receipt.replace("1790000002", "1790000003");
    assert_ne!(conflict, receipt);
    let changed = a
        .receive(
            &mut tx,
            &a.config.receipt_authority,
            conflict.as_bytes(),
            now,
        )
        .await;
    tx.rollback().await.unwrap();
    db.end().await;
    assert!(matches!(replay, Ok(ReceiptResult::Duplicate)));
    assert!(matches!(changed, Ok(ReceiptResult::Quarantined)));
}

#[tokio::test]
async fn lifecycle_receipt_absent_stale_generation() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    let intent = newest_intent(&db, uid("20"), Command::Deliver).await;
    let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let a = db.runtime.authority();
    let receipt = vector("delivery_receipt")
        .replace(&uid("4").to_string(), &operation.to_string())
        .replace("1790000002", &now.to_string())
        .replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"2""#,
        );
    let result = a
        .receive(
            &mut tx,
            &a.config.receipt_authority,
            receipt.as_bytes(),
            now,
        )
        .await;
    tx.rollback().await.unwrap();
    let stored: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_receipts WHERE operation_id=$1)",
    )
    .bind(operation)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let state = item_state(&db).await;
    db.end().await;
    assert!(matches!(result, Err(Error::Conflict)));
    assert!(!stored);
    assert_eq!(state, State::AwaitingDelivery);
}

async fn lifecycle_completed_worker_tick() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    recovery_tests::item_effect(&db, Command::Deliver, false).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(item_state(&db).await, State::Completed);
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    let delivery_count = dispatched(&db, uid("20"), Command::Deliver).await;
    let return_count = dispatched(&db, uid("20"), Command::Return).await;
    db.end().await;
    assert_eq!(state, State::Completed);
    assert_eq!((delivery_count, return_count), (1, 0));
}

#[tokio::test]
async fn lifecycle_cancelled_worker_tick() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    expire_item(&db).await;
    db.runtime.tick().await.unwrap();
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(item_state(&db).await, State::Cancelled);
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    let delivery_count = dispatched(&db, uid("20"), Command::Deliver).await;
    let return_count = dispatched(&db, uid("20"), Command::Return).await;
    db.end().await;
    assert_eq!(state, State::Cancelled);
    assert_eq!((delivery_count, return_count), (0, 1));
}

async fn lifecycle_draft_pending_lock() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    authorize_seller(&db).await;
    new_character_draft(&db).await;
    db.runtime.tick().await.unwrap();
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let binds = dispatched(&db, uid("30"), Command::Bind).await;
    let locks = dispatched(&db, uid("30"), Command::Lock).await;
    db.end().await;
    assert_eq!(state, State::Draft);
    assert_eq!((locks, binds), (1, 0));
}

#[tokio::test]
async fn lifecycle_active_bid_foreign_destination() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    fund_buyer(&db).await;
    let result = db
        .runtime
        .bid(uid("13"), uid("20"), 10000, uid("21"), Uuid::now_v7())
        .await;
    let reservations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bazaar_coin_holds WHERE id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    db.end().await;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(reservations, 0);
}

async fn lifecycle_draft_refuse_escrow() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let listing = uid("70");
    sqlx::query(
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at)
         SELECT $1,owner,kind,form,currency,price,accept_offers,ends_at FROM bazaar_listings WHERE id=$2",
    )
    .bind(listing)
    .bind(uid("20"))
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO bazaar_authority_assets
         (listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,
          session_hash,quantity,publication_consent,input_binding)
         SELECT $1,$1,world_id,seller_character_id,$2,expected_revision,session_hash,quantity,
                publication_consent,'lifecycle-refused' FROM bazaar_authority_assets WHERE listing_id=$3",
    )
    .bind(listing)
    .bind(uid("71"))
    .bind(uid("20"))
    .execute(&db.pool)
    .await
    .unwrap();
    let intent = PreparedIntent::parse(
        &vector("escrow_intent")
            .replace(&uid("20").to_string(), &listing.to_string())
            .replace(&uid("22").to_string(), &uid("71").to_string())
            .replace(&uid("2").to_string(), &uid("72").to_string())
            .replace("0000000000a1", "0000000000b1")
            .replace(r#""source_revision":"18""#, r#""source_revision":"110""#),
    )
    .unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    store_intent(&mut tx, &intent).await;
    tx.commit().await.unwrap();
    reject(&db, &intent, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
    db.runtime.tick().await.unwrap();
    let next = rebuild_epoch(&db).await;
    let resynced = resync(&db, &next).await;
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    db.end().await;
    assert_eq!(state, State::Cancelled);
    assert!(resynced.is_ok(), "{resynced:?}");
}

#[tokio::test]
async fn lifecycle_completed_unlock_character() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = delivered_character_sale("coins").await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state = listing_state(&db).await;
    let unlock = dispatched(&db, uid("30"), Command::Unlock).await;
    db.end().await;
    assert_eq!(state, State::Completed);
    assert_eq!(unlock, 1);
}

#[tokio::test]
async fn lifecycle_fence_stale_restore_notice() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    // The durable notice represents the coordinator committing its restore cut.
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO character_restore_notices(restore_event_id,notice_json,new_recovery_generation)
         VALUES($1,'{}',$2)",
    )
    .bind(Uuid::now_v7())
    .bind(format!("{:020}", 4))
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let result = db.runtime.tick().await;
    let state = item_state(&db).await;
    db.end().await;
    assert!(matches!(result, Err(Error::Unavailable)));
    assert_eq!(state, State::Active);
}

fn status_publication(state: &str, revision: u64, custody_revision: u64, now: u64) -> String {
    let mut status = vector("delivered_status")
        .replace("DELIVERED", state)
        .replace(
            r#""projection_revision":"2""#,
            &format!(r#""projection_revision":"{revision}""#),
        )
        .replace(
            r#""custody_revision":"2""#,
            &format!(r#""custody_revision":"{custody_revision}""#),
        )
        .replace("1790000002", &now.to_string());
    if matches!(state, "locked" | "UNLOCKED") {
        status = status
            .replace(r#""asset_kind":"ITEM""#, r#""asset_kind":"CHARACTER""#)
            .replace(&uid("20").to_string(), &uid("30").to_string())
            .replace(&uid("22").to_string(), &uid("31").to_string())
            .replace(&uid("23").to_string(), &uid("32").to_string())
            .replace(
                r#""snapshot_hash":null"#,
                &format!(r#""snapshot_hash":"{}""#, "a".repeat(64)),
            );
    }
    if state == "INVALID" {
        status = status.replace(r#""reason_code":null"#, r#""reason_code":"STATE_CHANGED""#);
    }
    status
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusEvent {
    Newer,
    Older,
    Replay,
    Conflict,
}

async fn custody_status_event(
    db: &BrowserFixture,
    state: &str,
    event: StatusEvent,
    result: IngestResult,
) {
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let original = status_publication(state, 10, 2, now);
    let outbox = projection_ingest();
    assert_eq!(
        outbox
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                original.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Applied
    );
    let incoming = match event {
        StatusEvent::Newer => status_publication(state, 11, 3, now),
        StatusEvent::Older => status_publication(state, 9, 1, now),
        StatusEvent::Replay => original.clone(),
        StatusEvent::Conflict => status_publication(state, 10, 3, now),
    };
    let observed = outbox
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            incoming.as_bytes(),
            now,
        )
        .await
        .unwrap();
    let subject = if matches!(state, "locked" | "UNLOCKED") {
        uid("30")
    } else {
        uid("20")
    };
    let stored: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1",
    )
    .bind(subject)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let quarantined: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_game_quarantine WHERE kind='STATUS' AND subject=$1)",
    )
    .bind(subject)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let listing: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(observed, result);
    assert_eq!(
        stored,
        if matches!(event, StatusEvent::Newer) {
            incoming
        } else {
            original
        }
    );
    assert_eq!(quarantined, matches!(event, StatusEvent::Conflict));
    assert_eq!(
        listing,
        State::Active,
        "a STATUS projection never writes the listing enum"
    );
}

const STATUS_STATES: [&str; 6] = [
    "locked",
    "ESCROWED",
    "DELIVERED",
    "RETURNED",
    "UNLOCKED",
    "INVALID",
];
const STATUS_EVENTS: [(StatusEvent, IngestResult); 4] = [
    (StatusEvent::Newer, IngestResult::Applied),
    (StatusEvent::Older, IngestResult::Superseded),
    (StatusEvent::Replay, IngestResult::Duplicate),
    (StatusEvent::Conflict, IngestResult::Quarantined),
];

fn status_rows() -> impl Iterator<Item = (&'static str, StatusEvent, IngestResult)> {
    STATUS_STATES.into_iter().flat_map(|status| {
        STATUS_EVENTS
            .into_iter()
            .map(move |(event, expected)| (status, event, expected))
    })
}

async fn custody_status_table() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    for (status, event, expected) in status_rows() {
        eprintln!("status row: {status}, {event:?}");
        custody_status_event(&db, status, event, expected).await;
    }
    db.end().await;
}

async fn gold_bid(db: &BrowserFixture) -> Uuid {
    sqlx::query("UPDATE bazaar_listings SET currency='gold' WHERE id=$1")
        .bind(uid("20"))
        .execute(&db.pool)
        .await
        .unwrap();
    let request = Uuid::now_v7();
    db.runtime
        .bid(uid("13"), uid("20"), 10000, uid("14"), request)
        .await
        .unwrap();
    request
}

async fn funding_state(db: &BrowserFixture, request: Uuid) -> (String, Option<Uuid>) {
    sqlx::query_as(
        "SELECT b.phase,l.leader FROM bazaar_authority_gold_bids b
         JOIN bazaar_listings l ON l.id=b.listing_id WHERE b.request_id=$1",
    )
    .bind(request)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn lifecycle_active_bid_gold() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let request = gold_bid(&db).await;
    db.runtime.tick().await.unwrap();
    let (phase, leader) = funding_state(&db, request).await;
    let holds = dispatched(&db, uid("20"), Command::Hold).await;
    db.end().await;
    assert_eq!(phase, "pending");
    assert_eq!(leader, None);
    assert_eq!(holds, 1);
}

#[tokio::test]
async fn lifecycle_gold_pending_refuse_hold() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let request = gold_bid(&db).await;
    let hold = newest_intent(&db, uid("20"), Command::Hold).await;
    reject(&db, &hold, "CHAR_CMD_BAZAAR_HOLD_CONFLICT").await;
    db.runtime.tick().await.unwrap();
    let (phase, leader) = funding_state(&db, request).await;
    let releases = dispatched(&db, uid("20"), Command::Release).await;
    db.end().await;
    assert_eq!(phase, "refused");
    assert_eq!(leader, None);
    assert_eq!(releases, 0);
}

#[tokio::test]
async fn lifecycle_gold_pending_commit_hold() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let request = gold_bid(&db).await;
    let hold = newest_intent(&db, uid("20"), Command::Hold).await;
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"1","buyer_account_id":"{}","offer_id":"{}","hold_purpose":"{}","held_amount":"10000"}}"#,
        uid("83"),
        uid("13"),
        text(&hold.value, "offer_id").unwrap(),
        text(&hold.value, "hold_purpose").unwrap()
    );
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    committed_fact(db.runtime.authority(), &mut tx, &hold, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let (phase, leader) = funding_state(&db, request).await;
    db.end().await;
    assert_eq!(phase, "accepted");
    assert_eq!(leader, Some(uid("13")));
}

async fn stage_item_escrow(db: &BrowserFixture) -> Uuid {
    authorize_seller(db).await;
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let intent = PreparedIntent::parse(
        &vector("escrow_intent")
            .replace(&uid("2").to_string(), &uid("72").to_string())
            .replace("0000000000a1", "0000000000b1")
            .replace(r#""source_revision":"18""#, r#""source_revision":"101""#)
            .replace("1790000000", &now.to_string())
            .replace("1790000300", &(now + 300).to_string()),
    )
    .unwrap();
    let staged = projection_ingest()
        .stage(
            &mut tx,
            &PlatformAccounts {
                enabled: true,
                session_hash: Some([1; 32]),
            },
            uid("11"),
            uid("70"),
            &intent,
            a.fence.projection_epoch,
            a.fence.recovery_generation,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    staged.operation_id
}

async fn command_state(db: &BrowserFixture, a: &Authority, operation: Uuid) -> Option<String> {
    let mut tx = db.pool.begin().await.unwrap();
    let request = format!(
        r#"{{"contract_version":2,"operation":"ListBazaarCommandStatesV1","source_authority":"{}","after_source_revision":"0","after_operation_id":"{}","max_entries":32}}"#,
        a.config.projection_authority,
        Uuid::nil(),
    );
    let response = a
        .exchange(&mut tx, &a.config.projection_authority, request.as_bytes())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let page: serde_json::Value = serde_json::from_str(&response).unwrap();
    let operation = operation.to_string();
    page["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["operation_id"] == operation)
        .and_then(|entry| entry["state"].as_str().map(str::to_owned))
}

#[tokio::test]
async fn lifecycle_intent_staged_read_state() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let operation = stage_item_escrow(&db).await;
    let state = command_state(&db, db.runtime.authority(), operation).await;
    db.end().await;
    assert_eq!(state, None, "an unpublished intent is invisible to Game");
}

#[derive(Clone, Copy)]
enum PublishedIntent {
    Pending,
    Rejected,
    Committed,
    Reopened,
}

async fn published_deposit_state(target: PublishedIntent) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Active).await;
    let listing = create_item(&mut db).await;
    let intent = newest_intent(&db, listing, Command::Escrow).await;
    let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
    let mut a = db.runtime.authority().clone();
    let expected = match target {
        PublishedIntent::Pending => "PENDING",
        PublishedIntent::Rejected => {
            reject(&db, &intent, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
            "REJECTED"
        }
        PublishedIntent::Committed => {
            let mut tx = db.pool.begin().await.unwrap();
            let now = runtime::now(&mut tx).await.unwrap();
            // Keep the producer's closed result-data field order.
            let previous = Receipt::parse(&vector("escrow_receipt").into_bytes()).unwrap();
            let data = previous.value.get("result_data").unwrap();
            let data = String::from_utf8(canonical::encode(data).unwrap())
                .unwrap()
                .replace(&uid("22").to_string(), &uid("71").to_string());
            committed_fact(&a, &mut tx, &intent, &data, now).await;
            tx.commit().await.unwrap();
            "COMMITTED"
        }
        PublishedIntent::Reopened => {
            a = rebuild_epoch(&db).await;
            "REOPENED"
        }
    };
    let observed = command_state(&db, &a, operation).await;
    db.end().await;
    assert_eq!(observed.as_deref(), Some(expected));
}

macro_rules! intent_cases {
    ($($name:ident, $disposition:ident;)*) => {
        $(
            #[tokio::test]

            async fn $name() {
                published_deposit_state(PublishedIntent::$disposition).await;
            }
        )*
    };
}

intent_cases! {
    lifecycle_intent_pending_read_state, Pending;
    lifecycle_intent_rejected_read_state, Rejected;
    lifecycle_intent_committed_read_state, Committed;
    lifecycle_intent_reopened_read_state, Reopened;
}

async fn lifecycle_draft_refuse_lock() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    authorize_seller(&db).await;
    let intent = new_character_draft(&db).await;
    reject(&db, &intent, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let unlocks = dispatched(&db, uid("30"), Command::Unlock).await;
    db.end().await;
    assert_eq!(unlocks, 0, "a rejected Lock acquired no Game custody");
    assert_eq!(
        state,
        State::Cancelled,
        "a rejected Lock must close the draft"
    );
}

#[tokio::test]
async fn lifecycle_legacy_refused_lock_rejection_replay_closes_draft() {
    rejected_lock_replay(false).await;
}

#[tokio::test]
async fn lifecycle_refused_cancelled_lock_rejection_replay_stays_closed() {
    rejected_lock_replay(true).await;
}

async fn rejected_lock_replay(cancel_first: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Active).await;
    authorize_seller(&db).await;
    let intent = new_character_draft(&db).await;
    let refusal = "CHAR_CMD_BAZAAR_STALE_REVISION";
    reject(&db, &intent, refusal).await;
    // The old worker recorded the refusal while leaving the listing in draft.
    sqlx::query("UPDATE bazaar_authority_assets SET refusal_code=$2 WHERE listing_id=$1")
        .bind(uid("30"))
        .bind(refusal)
        .execute(&db.pool)
        .await
        .unwrap();
    if cancel_first {
        db.runtime.tick().await.unwrap();
        let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
            .bind(uid("30"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(state, State::Cancelled);
    }
    set_resync_fence(&mut db, 3, 4);
    let mut tx = db.pool.begin().await.unwrap();
    for listing in [uid("20"), uid("30")] {
        recovery_tests::replay(db.runtime.authority(), &mut tx, listing).await;
    }
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let unlocks = dispatched(&db, uid("30"), Command::Unlock).await;
    db.end().await;
    assert_eq!(state, State::Cancelled);
    assert_eq!(unlocks, 0);
}

#[tokio::test]
async fn lifecycle_intent_staged_resync() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
    let staged = stage_item_escrow(&db).await;
    let published: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=$1)",
    )
    .bind(staged)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(!published, "this intent has never been executable");
    let next = rebuild_epoch(&db).await;
    let result = resync(&db, &next).await;
    db.end().await;
    assert!(
        result.is_ok(),
        "an undispatched intent cannot require a Game receipt: {result:?}"
    );
}

pub(super) fn set_resync_fence(db: &mut BrowserFixture, epoch: u64, generation: u64) {
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.fence.projection_epoch = epoch;
    config.fence.recovery_generation = generation;
    config.authority.fence.projection_epoch = epoch;
    config.authority.fence.recovery_generation = generation;
    let fence = serde_json::json!({
        "projection_epoch": epoch,
        "recovery_generation": generation,
        "incarnation": config.fence.incarnation,
        "character_history_complete_since": config.fence.character_history_complete_since,
    });
    std::fs::write(&config.fence_file, fence.to_string()).unwrap();
}

async fn quarantine_count(db: &BrowserFixture) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_game_quarantine WHERE kind='STATUS' AND subject=$1",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

async fn resync_progress(db: &BrowserFixture) -> (bool, bool) {
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let certified = a.resynced(&mut tx).await.unwrap();
    let progress: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_resync_progress
         WHERE projection_epoch=$1::text::numeric AND recovery_generation=$2::text::numeric)",
    )
    .bind(a.fence.projection_epoch.to_string())
    .bind(a.fence.recovery_generation.to_string())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    (certified, progress)
}

async fn lifecycle_completed_resync_waits_for_current_receipt_and_audit() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    assert_eq!(item_state(&db).await, State::Completed);
    set_resync_fence(&mut db, 3, 4);
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&db.pool)
        .await
        .unwrap();

    let first_tick = db.runtime.tick().await;
    let before_receipt = (quarantine_count(&db).await, resync_progress(&db).await);

    let a = db.runtime.authority();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT intent,receipt FROM bazaar_game_outbox JOIN bazaar_authority_receipts USING(operation_id)
         WHERE listing_id=$1 AND bazaar_authority_receipts.recovery_generation=3 ORDER BY source_revision",
    )
    .bind(uid("20"))
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let mut delivery = None;
    for (intent, raw) in rows {
        let intent = PreparedIntent::parse(&intent).unwrap();
        let raw = raw.replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"4""#,
        );
        assert_eq!(
            a.receive(&mut tx, &a.config.receipt_authority, raw.as_bytes(), now)
                .await
                .unwrap(),
            ReceiptResult::Stored
        );
        if intent.command() == Command::Deliver {
            delivery = Some((intent, raw));
        } else {
            assert_eq!(
                audit(a, &mut tx, &intent, &raw, now).await,
                ReceiptResult::Stored
            );
        }
    }
    tx.commit().await.unwrap();
    let second_tick = db.runtime.tick().await;
    let before_audit = (quarantine_count(&db).await, resync_progress(&db).await);

    let (intent, raw) = delivery.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        audit(a, &mut tx, &intent, &raw, now).await,
        ReceiptResult::Stored
    );
    tx.commit().await.unwrap();
    let third_tick = db.runtime.tick().await;
    let after_audit = (quarantine_count(&db).await, resync_progress(&db).await);
    let state = item_state(&db).await;
    db.end().await;

    assert_eq!(before_receipt, (0, (false, false)), "{first_tick:?}");
    assert_eq!(before_audit, (0, (false, false)), "{second_tick:?}");
    assert_eq!(after_audit, (0, (true, true)), "{third_tick:?}");
    first_tick.unwrap();
    second_tick.unwrap();
    third_tick.unwrap();
    assert_eq!(state, State::Completed);
}

async fn completed_resync_contradiction(status_change: impl FnOnce(String) -> String) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    assert_eq!(item_state(&db).await, State::Completed);
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let projections: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind,canonical_body FROM bazaar_game_projections WHERE kind IN ('POLICY','STATUS')
         ORDER BY kind",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let mut status_change = Some(status_change);
    for (kind, body) in projections {
        let next = body.replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#);
        let next = if kind == "STATUS" {
            status_change.take().unwrap()(next)
        } else {
            next
        };
        assert_eq!(
            projection_ingest()
                .ingest(
                    &mut tx,
                    &a.config.projection_authority,
                    next.as_bytes(),
                    now
                )
                .await
                .unwrap(),
            IngestResult::Applied
        );
    }
    assert!(status_change.is_none());
    tx.commit().await.unwrap();
    set_resync_fence(&mut db, 4, 3);
    let result = db.runtime.tick().await;
    let state = item_state(&db).await;
    let quarantines = quarantine_count(&db).await;
    let (certified, _) = resync_progress(&db).await;
    db.end().await;
    assert_eq!(state, State::Completed);
    assert_eq!(quarantines, 1, "{result:?}");
    assert!(!certified);
    assert!(matches!(result, Err(Error::CustodyDrift)));
}

async fn lifecycle_completed_resync_escrow_status() {
    completed_resync_contradiction(|status| {
        let parsed = decode(status.as_bytes(), 262144).unwrap();
        let operation = text(&parsed, "last_operation_id").unwrap();
        status
            .replace("DELIVERED", "ESCROWED")
            .replace(&format!("\"{operation}\""), "null")
    })
    .await;
}

#[tokio::test]
async fn lifecycle_completed_resync_quarantines_wrong_asset() {
    completed_resync_contradiction(|status| {
        status.replace(&uid("22").to_string(), &uid("72").to_string())
    })
    .await;
}

#[tokio::test]
async fn lifecycle_completed_resync_quarantines_wrong_seller() {
    completed_resync_contradiction(|status| {
        status.replace(&uid("11").to_string(), &uid("71").to_string())
    })
    .await;
}

pub(super) async fn stage_newer_delivery(db: &BrowserFixture) {
    let delivery = newest_intent(db, uid("20"), Command::Deliver).await;
    let newer = uid("72");
    let raw = std::str::from_utf8(delivery.bytes())
        .unwrap()
        .replace(
            text(&delivery.value, "operation_id").unwrap(),
            &newer.to_string(),
        )
        .replace(
            &format!(
                r#""source_revision":"{}""#,
                text(&delivery.value, "source_revision").unwrap()
            ),
            r#""source_revision":"1000""#,
        );
    let mut tx = db.pool.begin().await.unwrap();
    store_intent_with_key(
        &mut tx,
        &PreparedIntent::parse(&raw).unwrap(),
        "different-delivery-step",
    )
    .await;
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn lifecycle_completed_resync_ignores_newer_staged_delivery() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    stage_newer_delivery(&db).await;
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );
    let tick = db.runtime.tick().await;
    let progress = resync_progress(&db).await;
    let quarantines = quarantine_count(&db).await;
    db.end().await;
    tick.unwrap();
    assert_eq!(progress, (true, true));
    assert_eq!(quarantines, 0);
}

#[tokio::test]
async fn lifecycle_legacy_refused_lock_recovery_only() {
    redecided_lock(false, false).await;
}

#[tokio::test]
async fn lifecycle_cancelled_refused_lock_redecided_is_unlocked() {
    redecided_lock(true, false).await;
}

#[tokio::test]
async fn bz04_refused_draft_initial_status_without_operation_unlocks() {
    redecided_lock(false, true).await;
}

#[tokio::test]
async fn bz04_refused_cancelled_initial_status_without_operation_unlocks() {
    redecided_lock(true, true).await;
}

async fn redecided_lock(cancel_first: bool, initial_status: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Active).await;
    authorize_seller(&db).await;
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.authority.config.escrow_account_id = Some(uid("40"));
    config.escrow_account = Some(uid("40"));
    let intent = new_character_draft(&db).await;
    let refusal = "CHAR_CMD_BAZAAR_STALE_REVISION";
    reject(&db, &intent, refusal).await;
    sqlx::query("UPDATE bazaar_authority_assets SET refusal_code=$2 WHERE listing_id=$1")
        .bind(uid("30"))
        .bind(refusal)
        .execute(&db.pool)
        .await
        .unwrap();
    if cancel_first {
        db.runtime.tick().await.unwrap();
        let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
            .bind(uid("30"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(state, State::Cancelled);
        assert_eq!(dispatched(&db, uid("30"), Command::Unlock).await, 0);
    }
    set_resync_fence(&mut db, 3, 4);
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay(a, &mut tx, uid("20")).await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=101")
        .execute(&mut *tx)
        .await
        .unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let operation = id(text(&intent.value, "operation_id").unwrap()).unwrap();
    let data = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"1","eligibility_revision":"4","signed_snapshot":{}}}"#,
        uid("31"),
        uid("32"),
        vector("locked_character_snapshot")
    );
    committed_fact(a, &mut tx, &intent, &data, now).await;
    let projected_operation = if initial_status {
        Uuid::nil()
    } else {
        operation
    };
    character_status(&db, &mut tx, "locked", projected_operation, ("1", "1"), now).await;
    tx.commit().await.unwrap();

    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let staged_escrow: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_LISTING_ESCROW'")
        .bind(uid("30")).fetch_one(&db.pool).await.unwrap();
    assert_eq!(staged_escrow, 0);
    assert_eq!(dispatched(&db, uid("30"), Command::Unlock).await, 1);

    let unlock = newest_intent(&db, uid("30"), Command::Unlock).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"2","current_account_id":"{}"}}"#,
        uid("31"),
        uid("32"),
        uid("11")
    );
    committed_fact(a, &mut tx, &unlock, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("30"))
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, State::Cancelled);
    assert_eq!(dispatched(&db, uid("30"), Command::Unlock).await, 1);
    db.end().await;
}

async fn lifecycle_matched_legacy_hold_retires_once() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    let listing = uid("20");
    let original: Uuid = sqlx::query_scalar(
        "UPDATE bazaar_listings SET state='matched', form='fixed_price' WHERE id=$1 RETURNING reservation",
    ).bind(listing).fetch_one(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,2,$2,'Legacy suspension')")
        .bind(listing).bind(uid("81")).execute(&db.pool).await.unwrap();
    let a = db.runtime.authority().clone();
    let core = crate::Bazaar::new(
        db.pool.clone(),
        PlatformAccounts {
            enabled: true,
            session_hash: None,
        },
        CoinReservations {
            authority: a.clone(),
        },
        a,
        crate::Flags {
            enabled: true,
            gold_enabled: true,
        },
    );
    assert!(core.settle(listing, "legacy-match").await.is_err());
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let settle = core.settle(listing, "legacy-match").await;
    assert!(settle.is_ok(), "{settle:?}");
    let sale = settle.unwrap();
    let replay = core.settle(listing, "legacy-match").await.unwrap();
    assert_eq!(sale.reservation, original);
    assert_eq!(replay.id, sale.id);
    let effects: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM bazaar_audit WHERE relation_name='bazaar_moderation_holds'
                 AND row_id=$1 AND before_row->>'revision'='2'
                 AND after_row->>'action'='release_legacy_matched_hold'),
                (SELECT count(*) FROM bazaar_settlements WHERE listing_id=$1 AND reservation=$2)",
    )
    .bind(listing)
    .bind(original)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(effects, (1, 1));
    db.end().await;
}

async fn lifecycle_delivery_pending_exhausts_retry_budget_once() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    set_resync_fence(&mut db, 3, 4);
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&db.pool)
        .await
        .unwrap();
    // Exactly eight ticks may wait for the missing receipt for this key.
    for _ in 0..8 {
        db.runtime.tick().await.unwrap();
        assert_eq!(quarantine_count(&db).await, 0);
        assert_eq!(resync_progress(&db).await, (false, false));
    }
    let result = db.runtime.tick().await;
    assert!(matches!(result, Err(Error::CustodyDrift)), "{result:?}");
    // A later exact replay cannot reopen a terminal custody report.
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay(db.runtime.authority(), &mut tx, uid("20")).await;
    tx.commit().await.unwrap();
    let result = db.runtime.clone().tick().await;
    assert!(matches!(result, Err(Error::CustodyDrift)), "{result:?}");
    assert_eq!(quarantine_count(&db).await, 1);
    let reports: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_audit WHERE row_id=$1
         AND after_row->>'action'='delivery_custody_drift'
         AND after_row->>'reason'='evidence_retry_exhausted'",
    )
    .bind(uid("20"))
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(reports, 1);
    assert_eq!(item_state(&db).await, State::Completed);
    db.end().await;
}

async fn bz01_alternate_delivery_cannot_win_publication() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(item_state(&db).await, State::AwaitingDelivery);
    let (sale, reservation): (Uuid, Uuid) =
        sqlx::query_as("SELECT id,reservation FROM bazaar_settlements WHERE listing_id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let now = runtime::now(&mut db.pool.acquire().await.unwrap())
        .await
        .unwrap();
    let mut intent = decode(vector("delivery_intent").as_bytes(), 262144).unwrap();
    let Value::Object(fields) = &mut intent else {
        panic!("fixture intent");
    };
    for (key, value) in fields {
        *value = match key.as_str() {
            "operation_id" => Value::String(uid("73").to_string()),
            "sale_id" => Value::String(sale.to_string()),
            "source_revision" => Value::String("101".into()),
            "issued_at_source" => Value::String(now.to_string()),
            "expires_at_source" => Value::String((now + 300).to_string()),
            "funding" => {
                let Value::Object(funding) = value else {
                    panic!("fixture funding");
                };
                for (field, value) in funding.iter_mut() {
                    if field == "reservation_id" {
                        *value = Value::String(reservation.to_string());
                    }
                }
                value.clone()
            }
            _ => value.clone(),
        };
    }
    let raw = String::from_utf8(canonical::encode(&intent).unwrap()).unwrap();
    let canonical = PreparedIntent::parse(&raw).unwrap();
    let alternate = PreparedIntent::parse(
        &raw.replace(&uid("73").to_string(), &uid("74").to_string())
            .replace(r#""source_revision":"101""#, r#""source_revision":"102""#),
    )
    .unwrap();
    let mut first = db.pool.acquire().await.unwrap();
    let mut second = db.pool.acquire().await.unwrap();
    use sqlx::Acquire;
    let mut tx = first.begin().await.unwrap();
    store_intent(&mut tx, &canonical).await;
    store_intent_with_key(&mut tx, &alternate, "alternate-delivery").await;
    tx.commit().await.unwrap();

    let a = db.runtime.authority();
    let mut tx = second.begin().await.unwrap();
    let rejected = a
        .publish_intent(&mut tx, &QualifiedAccounts, uid("74"))
        .await;
    match rejected {
        Ok(()) => tx.commit().await.unwrap(),
        Err(_) => tx.rollback().await.unwrap(),
    }
    assert!(matches!(rejected, Err(Error::Conflict)), "{rejected:?}");
    let mut tx = first.begin().await.unwrap();
    a.publish_intent(&mut tx, &QualifiedAccounts, uid("73"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    drop(first);
    drop(second);
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    db.end().await;
}

async fn insert_resync_fillers(db: &BrowserFixture, count: i32) {
    let mut tx = db.pool.begin().await.unwrap();
    for n in 0..count {
        let listing = uid(&format!("{:x}", 0x30 + n));
        sqlx::query(
            "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at)
                     VALUES($1,$2,'item','auction','coins',10000,false,now()+interval '1 day')",
        )
        .bind(listing)
        .bind(uid("11"))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,
                     asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding)
                     VALUES($1,$1,$2,$3,$1,1,$4,1,true,'filler')")
            .bind(listing).bind(uid("12")).bind(uid("21")).bind([0_u8;32].as_slice())
            .execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
}

async fn dirty_status_rechecked(after_certification: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    if !after_certification {
        insert_resync_fillers(&db, 24).await;
    }
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );

    // Keep the producer connection checked out for both ticks.
    let mut producer = db.pool.acquire().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(resync_progress(&db).await.0, after_certification);
    let cursor: Uuid = sqlx::query_scalar(
        "SELECT listing_cursor FROM bazaar_resync_progress WHERE projection_epoch=4 AND recovery_generation=3",
    ).fetch_one(&mut *producer).await.unwrap();
    assert!(cursor >= uid("20"));

    use sqlx::Acquire;
    let mut tx = producer.begin().await.unwrap();
    let (status, revision): (String, String) = sqlx::query_as(
        "SELECT canonical_body,projection_revision::text FROM bazaar_game_projections
         WHERE kind='STATUS' AND subject=$1",
    )
    .bind(uid("20"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let status = status
        .replace(&uid("22").to_string(), &uid("75").to_string())
        .replace(
            &format!(r#""projection_revision":"{revision}""#),
            &format!(
                r#""projection_revision":"{}""#,
                revision.parse::<u64>().unwrap() + 1
            ),
        );
    let now = runtime::now(&mut tx).await.unwrap();
    assert_eq!(
        projection_ingest()
            .ingest(
                &mut tx,
                &next.config.projection_authority,
                status.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Applied
    );
    // The producer's evidence is private until commit, including the dirty key.
    assert_eq!(resync_progress(&db).await.0, after_certification);
    tx.commit().await.unwrap();
    assert!(!resync_progress(&db).await.0);

    let result = db.runtime.tick().await;
    assert!(matches!(result, Err(Error::CustodyDrift)), "{result:?}");
    assert!(!resync_progress(&db).await.0);
    assert_eq!(quarantine_count(&db).await, 1);
    drop(producer);
    db.end().await;
}

async fn bz03_status_committed_behind_cursor_is_rechecked() {
    dirty_status_rechecked(false).await;
}

async fn bz03_status_committed_after_certification_is_rechecked() {
    dirty_status_rechecked(true).await;
}

async fn bz14_active_custody_conflict_behind_cursor_fences_certification() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Active).await;
    insert_resync_fillers(&db, 24).await;
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );
    let listing = uid("20");
    let listing_key = format!("resync-listing-dirty:{listing}");

    db.runtime.tick().await.unwrap();
    assert_eq!(resync_progress(&db).await, (false, true));
    let cursor: Uuid = sqlx::query_scalar(
        "SELECT listing_cursor FROM bazaar_resync_progress WHERE projection_epoch=4 AND recovery_generation=3",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(cursor > listing);
    assert_eq!(dispatched(&db, listing, Command::Deliver).await, 0);
    let dirty = "SELECT cursor_id FROM bazaar_work_cursors WHERE queue=$1";
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>(dirty)
            .bind(&listing_key)
            .fetch_optional(&db.pool)
            .await
            .unwrap(),
        None
    );

    let mut tx = db.pool.begin().await.unwrap();
    let status: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1",
    )
    .bind(listing)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let conflicting = status.replace(&uid("22").to_string(), &uid("75").to_string());
    let now = runtime::now(&mut tx).await.unwrap();
    assert_eq!(
        projection_ingest()
            .ingest(
                &mut tx,
                &next.config.projection_authority,
                conflicting.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Quarantined
    );
    let queued = sqlx::query_scalar::<_, Uuid>(dirty)
        .bind(&listing_key)
        .fetch_optional(&mut *tx)
        .await
        .unwrap();
    assert_eq!(quarantine_count(&db).await, 0);
    tx.commit().await.unwrap();
    assert_eq!(quarantine_count(&db).await, 1);

    db.runtime.tick().await.unwrap();
    let certification = resync_progress(&db).await;
    let certified_marker: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_resync
         WHERE projection_epoch=4 AND recovery_generation=3)",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let remaining = sqlx::query_scalar::<_, Uuid>(dirty)
        .bind(&listing_key)
        .fetch_optional(&db.pool)
        .await
        .unwrap();
    let state = item_state(&db).await;
    db.end().await;

    assert_eq!(
        certification,
        (false, true),
        "final page certified quarantined custody"
    );
    assert!(!certified_marker);
    assert_eq!(queued, Some(listing));
    assert_eq!(remaining, Some(listing));
    assert_eq!(state, State::Active);
}

async fn bz05_drift_does_not_starve_a_later_page() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    insert_resync_fillers(&db, 23).await;
    let later = insert_later_completed_sale(&db).await;
    set_resync_fence(&mut db, 3, 4);
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&db.pool)
        .await
        .unwrap();

    for _ in 0..22 {
        let _tick = db.runtime.tick().await;
    }
    let reports: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT row_id,count(*) FROM bazaar_audit WHERE after_row->>'action'='delivery_custody_drift'
         GROUP BY row_id ORDER BY row_id",
    ).fetch_all(&db.pool).await.unwrap();
    assert_eq!(reports, vec![(uid("20"), 1), (later, 1)]);
    assert!(!resync_progress(&db).await.0);
    db.end().await;
}

async fn bz06_missing_configured_world_does_not_charge_delivery_retries() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );
    configure_alternate_world(&mut db, uid("76")).await;
    for _ in 0..12 {
        let result = db.runtime.tick().await;
        assert!(
            matches!(result, Err(Error::ResyncWorldConfiguration)),
            "{result:?}"
        );
    }
    let reports: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_audit WHERE relation_name='bazaar_delivery_evidence' AND row_id=$1",
    ).bind(uid("20")).fetch_one(&db.pool).await.unwrap();
    assert_eq!(reports, 0);
    assert_eq!(quarantine_count(&db).await, 0);
    assert!(!resync_progress(&db).await.0);
    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .worlds = vec![uid("12")];
    db.runtime.tick().await.unwrap();
    assert!(resync_progress(&db).await.0);
    db.end().await;
}

async fn configure_alternate_world(db: &mut BrowserFixture, alternate: Uuid) {
    let mut tx = db.pool.begin().await.unwrap();
    let policy: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='POLICY' AND subject=$1",
    )
    .bind(uid("12"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let policy = policy.replace(&uid("12").to_string(), &alternate.to_string());
    let now = runtime::now(&mut tx).await.unwrap();
    assert_eq!(
        projection_ingest()
            .ingest(
                &mut tx,
                &db.runtime.authority().config.projection_authority,
                policy.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Applied
    );
    tx.commit().await.unwrap();
    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .worlds = vec![alternate];
}

async fn bz15_missing_legacy_world_preserves_delivery_retry_budget() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    let listing = uid("20");
    let effects = sale_effects(&db).await;
    sqlx::query("DELETE FROM bazaar_authority_assets WHERE listing_id=$1")
        .bind(listing)
        .execute(&db.pool)
        .await
        .unwrap();
    let world: Uuid = sqlx::query_scalar("SELECT world_id FROM bazaar_item_details WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(world, uid("12"));
    set_resync_fence(&mut db, 3, 4);
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay_except(
        db.runtime.authority(),
        &mut tx,
        listing,
        Some(Command::Deliver),
    )
    .await;
    tx.commit().await.unwrap();
    configure_alternate_world(&mut db, uid("76")).await;

    let mut missing_world_ticks = 0;
    for _ in 0..12 {
        let result = db.runtime.tick().await;
        assert!(!resync_progress(&db).await.0);
        if matches!(result, Err(Error::ResyncWorldConfiguration)) {
            missing_world_ticks += 1;
        }
    }
    let attempts = delivery_reports(&db, listing, "delivery_evidence_pending").await;
    let reports = delivery_reports(&db, listing, "delivery_custody_drift").await;
    let quarantined = quarantine_count(&db).await;

    std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .worlds = vec![world];
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay_except(
        db.runtime.authority(),
        &mut tx,
        listing,
        Some(Command::Escrow),
    )
    .await;
    tx.commit().await.unwrap();
    let restored = db.runtime.tick().await;
    let certification = resync_progress(&db).await;
    let restored_effects = sale_effects(&db).await;
    let state = item_state(&db).await;
    db.end().await;

    assert_eq!((attempts, reports, quarantined), (0, 0, 0));
    assert_eq!(missing_world_ticks, 12);
    assert!(restored.is_ok(), "{restored:?}");
    assert_eq!(certification, (true, true));
    assert_eq!(restored_effects, effects);
    assert_eq!(state, State::Completed);
}

async fn insert_later_completed_sale(db: &BrowserFixture) -> Uuid {
    let later = uid("80");
    let sale = uid("81");
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
                 SELECT $1,owner,kind,form,currency,price,accept_offers,state,ends_at FROM bazaar_listings WHERE id=$2")
        .bind(later).bind(uid("20")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,
                 expected_revision,session_hash,quantity,publication_consent,input_binding)
                 SELECT $1,$1,world_id,seller_character_id,$1,expected_revision,session_hash,quantity,
                 publication_consent,input_binding FROM bazaar_authority_assets WHERE listing_id=$2")
        .bind(later).bind(uid("20")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key,completed_at)
                 SELECT $1,$2,buyer,seller,currency,amount,commission,seller_net,$1,'later',now()
                 FROM bazaar_settlements WHERE listing_id=$3")
        .bind(sale).bind(later).bind(uid("20")).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let original = newest_intent(db, uid("20"), Command::Deliver).await;
    let raw = std::str::from_utf8(original.bytes())
        .unwrap()
        .replace(&uid("20").to_string(), &later.to_string())
        .replace(text(&original.value, "sale_id").unwrap(), &sale.to_string())
        .replace(
            text(&original.value, "operation_id").unwrap(),
            &uid("82").to_string(),
        )
        .replace(
            &format!(
                r#""source_revision":"{}""#,
                text(&original.value, "source_revision").unwrap()
            ),
            r#""source_revision":"1000""#,
        );
    let mut tx = db.pool.begin().await.unwrap();
    store_intent(&mut tx, &PreparedIntent::parse(&raw).unwrap()).await;
    tx.commit().await.unwrap();
    later
}

async fn sale_effects(db: &BrowserFixture) -> (i64, bool, bool, i64) {
    sale_effects_on(&mut db.pool.acquire().await.unwrap()).await
}

async fn sale_effects_on(db: &mut sqlx::PgConnection) -> (i64, bool, bool, i64) {
    sqlx::query_as(
        "SELECT h.amount, s.completed_at IS NOT NULL, d.completed,
            (SELECT count(*) FROM bazaar_authority_compensations WHERE listing_id=s.listing_id)
         FROM bazaar_settlements s JOIN bazaar_coin_dispositions d ON d.settlement_id=s.id
         JOIN bazaar_coin_holds h ON h.settlement_id=s.id WHERE s.listing_id=$1",
    )
    .bind(uid("20"))
    .fetch_one(db)
    .await
    .unwrap()
}

async fn delivery_reports(db: &BrowserFixture, listing: Uuid, action: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_audit WHERE relation_name='bazaar_delivery_evidence'
         AND row_id=$1 AND after_row->>'action'=$2",
    )
    .bind(listing)
    .bind(action)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

async fn bz08_awaiting_receipt_without_audit_exhausts_recovery_retries() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = pending_item_delivery().await;
    let delivery = newest_intent(&db, uid("20"), Command::Deliver).await;
    let operation = text(&delivery.value, "operation_id").unwrap();
    set_resync_fence(&mut db, 3, 4);
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay(a, &mut tx, uid("20")).await;
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"DeliverBazaarItem","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE","result_data":null,"recovery_generation":"4","decided_at":"{now}"}}"#
    );
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            receipt.as_bytes(),
            now
        )
        .await
        .unwrap(),
        ReceiptResult::Stored
    );
    tx.commit().await.unwrap();

    for attempt in 1..=8 {
        db.runtime.tick().await.unwrap();
        assert_eq!(
            delivery_reports(&db, uid("20"), "delivery_evidence_pending").await,
            attempt
        );
        assert_eq!(quarantine_count(&db).await, 0);
    }
    assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        audit(a, &mut tx, &delivery, &receipt, now).await,
        ReceiptResult::Stored,
    );
    tx.commit().await.unwrap();
    for _ in 0..2 {
        assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
    }
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        1
    );
    assert_eq!(quarantine_count(&db).await, 1);
    assert_eq!(sale_effects(&db).await, (10000, false, false, 0));
    assert_eq!(item_state(&db).await, State::AwaitingDelivery);
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    assert_eq!(dispatched(&db, uid("20"), Command::Return).await, 0);
    assert!(!resync_progress(&db).await.0);
    db.end().await;
}

async fn bz09_refunded_sale_quarantines_new_escrow_status() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = pending_item_delivery().await;
    recovery_tests::item_effect(&db, Command::Deliver, true).await;
    db.runtime.tick().await.unwrap();
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(item_state(&db).await, State::Cancelled);
    let refund = sale_effects(&db).await;
    assert_eq!(refund.0, 0);
    assert!(refund.2);
    assert_eq!(refund.3, 1);
    set_resync_fence(&mut db, 3, 3);
    db.runtime.tick().await.unwrap();
    assert!(resync_progress(&db).await.0);

    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let status = status_publication("ESCROWED", 3, 3, now).replace(
        &format!(r#""last_operation_id":"{}""#, uid("4")),
        r#""last_operation_id":null"#,
    );
    let a = db.runtime.authority();
    assert_eq!(
        projection_ingest()
            .ingest(
                &mut tx,
                &a.config.projection_authority,
                status.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Applied
    );
    tx.commit().await.unwrap();
    for _ in 0..2 {
        assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
    }
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        1
    );
    assert_eq!(sale_effects(&db).await, refund);
    assert_eq!(item_state(&db).await, State::Cancelled);
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    assert!(!resync_progress(&db).await.0);
    db.end().await;
}

async fn bz10_failed_rebuild_cannot_serve_obsolete_certification() {
    failed_rebuild_with_obsolete_binding(true).await;
    failed_rebuild_with_obsolete_binding(false).await;
}

async fn failed_rebuild_with_obsolete_binding(world_set_changed: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    set_resync_fence(&mut db, 3, 3);
    db.runtime.tick().await.unwrap();
    assert!(resync_progress(&db).await.0);
    let effects = sale_effects(&db).await;
    let mut second = db.pool.acquire().await.unwrap();
    use sqlx::Acquire;
    let mut tx = second.begin().await.unwrap();
    assert!(db.runtime.authority().fresh(&mut tx).await.unwrap());
    tx.rollback().await.unwrap();

    if world_set_changed {
        std::sync::Arc::make_mut(&mut db.runtime.config)
            .authority
            .config
            .worlds = vec![uid("12"), uid("76")];
    } else {
        sqlx::query("UPDATE bazaar_resync_progress SET worlds=$1")
            .bind(format!("delivery-sales-v2:{}", uid("12")))
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM bazaar_game_projections WHERE kind='POLICY'")
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let mut tx = second.begin().await.unwrap();
    assert!(!db.runtime.authority().fresh(&mut tx).await.unwrap());
    tx.rollback().await.unwrap();
    let _tick = db.runtime.tick().await;
    let mut tx = second.begin().await.unwrap();
    assert!(!db.runtime.authority().fresh(&mut tx).await.unwrap());
    tx.rollback().await.unwrap();
    assert_eq!(sale_effects(&db).await, effects);
    assert_eq!(item_state(&db).await, State::Completed);
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        0
    );
    drop(second);
    db.end().await;
}

async fn bz11_unpublished_alternate_does_not_fence_safe_return() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = expired_item_with_bid(1).await;
    db.runtime.tick().await.unwrap();
    let (sale, reservation): (Uuid, Uuid) =
        sqlx::query_as("SELECT id,reservation FROM bazaar_settlements WHERE listing_id=$1")
            .bind(uid("20"))
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let raw = vector("delivery_intent")
        .replace(
            text(
                &decode(vector("delivery_intent").as_bytes(), 262144).unwrap(),
                "sale_id",
            )
            .unwrap(),
            &sale.to_string(),
        )
        .replace(&uid("25").to_string(), &reservation.to_string())
        .replace(&uid("4").to_string(), &uid("74").to_string())
        .replace(r#""source_revision":"20""#, r#""source_revision":"1000""#);
    let staged = PreparedIntent::parse(&raw).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    store_intent_with_key(&mut tx, &staged, "alternate-delivery").await;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(uid("13"))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    db.runtime.tick().await.unwrap();
    assert_eq!(dispatched(&db, uid("20"), Command::Return).await, 1);
    assert_eq!(sale_effects(&db).await, (10000, false, false, 0));
    recovery_tests::item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(item_state(&db).await, State::Cancelled);
    let effects = sale_effects(&db).await;
    assert_eq!((effects.0, effects.2, effects.3), (0, true, 1));
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 0);
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        0
    );
    db.end().await;
}

async fn bz12_missing_first_page_status_does_not_starve_legacy_sale() {
    missing_first_page_status(false).await;
}

async fn legacy_sale_without_authority_asset_is_reconciled() {
    missing_first_page_status(true).await;
}

async fn missing_first_page_status(legacy_without_asset: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    insert_resync_fillers(&db, 23).await;
    let missing = uid("30");
    let later = insert_later_completed_sale(&db).await;
    if legacy_without_asset {
        sqlx::query("DELETE FROM bazaar_authority_assets WHERE listing_id=$1")
            .bind(later)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE bazaar_listings SET state=$2 WHERE id=$1")
        .bind(missing)
        .bind(State::Active)
        .execute(&db.pool)
        .await
        .unwrap();
    set_resync_fence(&mut db, 3, 4);
    let intent = newest_intent(&db, later, Command::Deliver).await;
    let operation = text(&intent.value, "operation_id").unwrap();
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay(a, &mut tx, uid("20")).await;
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"DeliverBazaarItem","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE","result_data":null,"recovery_generation":"4","decided_at":"{now}"}}"#
    );
    assert_eq!(
        a.receive(
            &mut tx,
            &a.config.receipt_authority,
            receipt.as_bytes(),
            now
        )
        .await
        .unwrap(),
        ReceiptResult::Stored,
    );
    assert_eq!(
        audit(a, &mut tx, &intent, &receipt, now).await,
        ReceiptResult::Stored
    );
    // Model a legacy completed row whose evidence predates dirty scheduling.
    super::super::delivery_work::DELIVERY
        .remove(&mut tx, later)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let effects = sale_effects(&db).await;

    db.runtime.tick().await.unwrap();
    assert_eq!(
        delivery_reports(&db, later, "delivery_custody_drift").await,
        0
    );
    for _ in 0..2 {
        assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
    }
    let reason: String = sqlx::query_scalar(
        "SELECT after_row->>'reason' FROM bazaar_audit
         WHERE row_id=$1 AND after_row->>'action'='delivery_custody_drift'",
    )
    .bind(later)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(reason, "delivery_outcome_contradicts_sale");
    assert_eq!(
        delivery_reports(&db, later, "delivery_custody_drift").await,
        1
    );
    assert!(!resync_progress(&db).await.0);
    let cursor: Uuid = sqlx::query_scalar(
        "SELECT listing_cursor FROM bazaar_resync_progress
         WHERE projection_epoch=3 AND recovery_generation=4",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(cursor, later);
    assert_eq!(sale_effects(&db).await, effects);
    assert_eq!(item_state(&db).await, State::Completed);
    db.end().await;
}

#[derive(Clone, Copy)]
struct LifecycleScenario {
    state: &'static str,
    event: &'static str,
    run: fn() -> Pin<Box<dyn Future<Output = ()>>>,
}

async fn rejected_delivery_return_waits_within_retry_bound() {
    return_recovery(true, false).await;
}

async fn pre_delivery_return_waits_within_retry_bound() {
    return_recovery(false, false).await;
}

async fn audited_return_rejection_reports_drift() {
    return_recovery(true, true).await;
}

async fn return_recovery(deliver_dispatched: bool, return_rejected: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = if deliver_dispatched {
        let db = pending_item_delivery().await;
        recovery_tests::item_effect(&db, Command::Deliver, true).await;
        db
    } else {
        let db = expired_item_with_bid(1).await;
        db.runtime.tick().await.unwrap();
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(uid("13"))
            .execute(&db.pool)
            .await
            .unwrap();
        db
    };
    db.runtime.tick().await.unwrap();
    assert_eq!(dispatched(&db, uid("20"), Command::Return).await, 1);
    let pending = "delivery_evidence_pending";
    assert_eq!(delivery_reports(&db, uid("20"), pending).await, 0);

    if return_rejected {
        let intent = newest_intent(&db, uid("20"), Command::Return).await;
        reject(&db, &intent, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
    } else {
        for attempt in 1..=8 {
            db.runtime.tick().await.unwrap();
            assert_eq!(delivery_reports(&db, uid("20"), pending).await, attempt);
        }
    }
    for _ in 0..2 {
        assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
    }
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        1
    );
    assert_eq!(item_state(&db).await, State::AwaitingDelivery);
    assert_eq!(sale_effects(&db).await, (10000, false, false, 0));
    assert_eq!(dispatched(&db, uid("20"), Command::Return).await, 1);
    assert_eq!(
        dispatched(&db, uid("20"), Command::Deliver).await,
        i64::from(deliver_dispatched)
    );
    assert!(!resync_progress(&db).await.0);
    db.end().await;
}

macro_rules! lifecycle_scenarios {
    ($($state:literal, $event:literal => $test:path;)*) => {
        const LIFECYCLE_SCENARIOS: &[LifecycleScenario] = &[
            $(LifecycleScenario {
                state: $state,
                event: $event,
                run: || Box::pin($test()),
            },)*
        ];
    };
}

lifecycle_scenarios! {
    "Absent", "Create" => lifecycle_absent_create;
    "Draft/no Game effect", "Lock pending" => lifecycle_draft_pending_lock;
    "Draft/Lock pending", "Audited Lock rejected" => lifecycle_draft_refuse_lock;
    "Draft/Escrow pending", "Audited Escrow rejected" => lifecycle_draft_refuse_escrow;
    "Draft/Escrow audited", "Advance" => lifecycle_draft_advance_escrowed;
    "Draft/Escrow audited", "Cancel requested" => lifecycle_draft_advance_cancel_requested;
    "Active/No bid", "Expiry" => lifecycle_active_expire_unbid;
    "Active/Moderation hold", "Expiry" => lifecycle_active_expire_held;
    "Active/Held funds", "Expiry" => lifecycle_active_expire_funded;
    "Active/Moderation hold", "Approved invalidation" => lifecycle_active_invalidate;
    "Matched/Legacy hold", "Retry tick" => lifecycle_matched_legacy_hold_retires_once;
    "AwaitingDelivery/Dispatched", "Audited committed Deliver and status" => lifecycle_awaiting_delivery_commit_delivery;
    "AwaitingDelivery/Dispatched", "Audited rejected Deliver and Return status" => lifecycle_awaiting_delivery_refuse_delivery;
    "AwaitingDelivery/Canonical and alternate staged", "Second connection attempts publication" => bz01_alternate_delivery_cannot_win_publication;
    "AwaitingDelivery/Receipt without audit", "BZ-08 Retry bound" => bz08_awaiting_receipt_without_audit_exhausts_recovery_retries;
    "AwaitingDelivery/Return dispatched after audited Deliver rejection", "Missing Return receipt through retry bound" => rejected_delivery_return_waits_within_retry_bound;
    "AwaitingDelivery/Return dispatched before Deliver", "Missing Return receipt through retry bound" => pre_delivery_return_waits_within_retry_bound;
    "AwaitingDelivery/Return dispatched after audited Deliver rejection", "Audited Return rejection" => audited_return_rejection_reports_drift;
    "AwaitingDelivery/Unpublished alternate", "BZ-11 Buyer eligibility changes" => bz11_unpublished_alternate_does_not_fence_safe_return;
    "Completed/Old evidence", "Receipt before audit" => lifecycle_completed_resync_waits_for_current_receipt_and_audit;
    "Completed/Old evidence", "STATUS ESCROWED" => lifecycle_completed_resync_escrow_status;
    "Completed/Old evidence", "Retry bound" => lifecycle_delivery_pending_exhausts_retry_budget_once;
    "Completed/Current evidence", "Worker tick" => lifecycle_completed_worker_tick;
    "Completed/Certified", "Status changes after certification" => bz03_status_committed_after_certification_is_rechecked;
    "Completed/Behind cursor", "Status producer commits between pages" => bz03_status_committed_behind_cursor_is_rechecked;
    "Completed/Obsolete marker", "BZ-10 Rebuild fails with world-set change" => bz10_failed_rebuild_cannot_serve_obsolete_certification;
    "Cancelled/Refunded", "BZ-09 STATUS ESCROWED contradicts return" => bz09_refunded_sale_quarantines_new_escrow_status;
    "Cancelled/Refunded", "Reversal of rejected Deliver" => recovery_tests::compensated_sale_reversal_is_reported_as_drift;
    "Rebuild/Old epoch", "Rebuild start" => lifecycle_fence_matching_new_epoch;
    "Rebuild/World absent", "Configured world missing" => bz06_missing_configured_world_does_not_charge_delivery_retries;
    "Rebuild/Early missing STATUS", "BZ-12 Page checkpoint before late legacy sale" => bz12_missing_first_page_status_does_not_starve_legacy_sale;
    "Rebuild/Completed legacy item without authority asset", "Page finds contradictory Deliver evidence" => legacy_sale_without_authority_asset_is_reconciled;
    "Rebuild/Active escrow behind cursor", "BZ-14 Conflicting STATUS commits between pages" => bz14_active_custody_conflict_behind_cursor_fences_certification;
    "Rebuild/Completed legacy item with missing configured world", "BZ-15 Retry checks without current Deliver evidence" => bz15_missing_legacy_world_preserves_delivery_retry_budget;
    "Rebuild/Early Drift", "Later page" => bz05_drift_does_not_starve_a_later_page;
    "Rebuild/Page in flight", "Writer crashes before commit" => lifecycle_crash_before_rebuild_page_commit_replays_page;
    "AwaitingDelivery/Return audited", "Second worker races" => lifecycle_workers_race_to_refund_once;
}

#[tokio::test]
async fn lifecycle_state_table_postgres() {
    listing_writer_table().await;
    sale_status_table().await;
    delivery_admission_table().await;
    for &(state, event, expected) in LISTING_ROWS {
        eprintln!("listing row: {state:?}, {event:?}");
        listing_event(state, event, expected).await;
    }
    custody_status_table().await;
    for row in LIFECYCLE_SCENARIOS {
        eprintln!("lifecycle row: {}, {}", row.state, row.event);
        (row.run)().await;
    }
}

#[test]
fn lifecycle_state_table_covers_state_event_products() {
    assert_eq!(
        LISTING_ROWS.len(),
        LISTING_STATES.len() * LISTING_EVENTS.len()
    );
    for state in LISTING_STATES {
        for event in LISTING_EVENTS {
            assert_eq!(
                LISTING_ROWS
                    .iter()
                    .filter(|&&(s, e, _)| (s, e) == (state, event))
                    .count(),
                1,
                "{state:?}, {event:?}",
            );
        }
    }
    let writer_rows: Vec<_> = listing_writer_rows().collect();
    assert_eq!(
        writer_rows.len(),
        LISTING_STATES.len() * LISTING_STATES.len()
    );
    for from in LISTING_STATES {
        for to in LISTING_STATES {
            assert_eq!(
                writer_rows
                    .iter()
                    .filter(|&&(s, e, _)| (s, e) == (from, to))
                    .count(),
                1
            );
        }
    }
    let rows: Vec<_> = status_rows().collect();
    assert_eq!(rows.len(), STATUS_STATES.len() * STATUS_EVENTS.len());
    for status in STATUS_STATES {
        for (event, _) in STATUS_EVENTS {
            assert_eq!(
                rows.iter()
                    .filter(|&&(s, e, _)| (s, e) == (status, event))
                    .count(),
                1,
                "{status}, {event:?}",
            );
        }
    }
    let sale_rows: Vec<_> = sale_status_rows().collect();
    assert_eq!(
        sale_rows.len(),
        SALE_EVIDENCE_STATES.len() * SALE_STATUS_EVENTS.len()
    );
    for state in SALE_EVIDENCE_STATES {
        for event in SALE_STATUS_EVENTS {
            assert_eq!(
                sale_rows
                    .iter()
                    .filter(|&&(s, e, _)| (s, e) == (state, event))
                    .count(),
                1
            );
        }
    }
    let admission_rows: Vec<_> = delivery_admission_rows().collect();
    assert_eq!(
        admission_rows.len(),
        ADMISSION_STATES.len() * ADMISSION_INPUTS.len()
    );
    for state in ADMISSION_STATES {
        for event in ADMISSION_INPUTS {
            assert_eq!(
                admission_rows
                    .iter()
                    .filter(|&&(s, e, ..)| (s, e) == (state, event))
                    .count(),
                1,
                "{state:?}, {event:?}",
            );
        }
    }
    let scenario_keys: std::collections::HashSet<_> = LIFECYCLE_SCENARIOS
        .iter()
        .map(|row| (row.state, row.event))
        .collect();
    assert_eq!(scenario_keys.len(), LIFECYCLE_SCENARIOS.len());
    for id in [
        "BZ-08", "BZ-09", "BZ-10", "BZ-11", "BZ-12", "BZ-14", "BZ-15",
    ] {
        assert_eq!(
            LIFECYCLE_SCENARIOS
                .iter()
                .filter(|row| row.event.starts_with(id))
                .count(),
            1
        );
    }
}

mod review_regressions {
    macro_rules! register {
        ($($name:ident),* $(,)?) => {
            $(
                #[tokio::test]
                async fn $name() {
                    super::$name().await;
                }
            )*
        };
    }
    register!(
        bz08_awaiting_receipt_without_audit_exhausts_recovery_retries,
        bz09_refunded_sale_quarantines_new_escrow_status,
        bz10_failed_rebuild_cannot_serve_obsolete_certification,
        bz11_unpublished_alternate_does_not_fence_safe_return,
        bz12_missing_first_page_status_does_not_starve_legacy_sale,
        bz14_active_custody_conflict_behind_cursor_fences_certification,
        bz15_missing_legacy_world_preserves_delivery_retry_budget,
    );
}

const LISTING_WRITER_ROWS: [[bool; 7]; 7] = [
    // Target order matches LISTING_STATES.
    [false, true, false, false, false, true, false],
    [false, false, true, true, false, true, true],
    [false, false, false, true, false, false, false],
    [false, false, false, false, true, false, false],
    [false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false],
];

fn listing_writer_rows() -> impl Iterator<Item = (State, State, bool)> {
    LISTING_STATES
        .into_iter()
        .zip(LISTING_WRITER_ROWS)
        .flat_map(|(from, targets)| {
            LISTING_STATES
                .into_iter()
                .zip(targets)
                .map(move |(to, allowed)| (from, to, allowed))
        })
}

async fn listing_writer_table() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    for (from, to, allowed) in listing_writer_rows() {
        let mut tx = db.pool.begin().await.unwrap();
        let listing = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
            VALUES($1,$2,'item','auction','coins',10000,false,$3,now()+interval '1 day')",
        )
        .bind(listing).bind(uid("11")).bind(from).execute(&mut *tx).await.unwrap();
        let result =
            crate::lifecycle::write_listing(&mut tx, listing, crate::lifecycle::Event::Listing(to))
                .await;
        if allowed {
            result.unwrap();
        } else {
            assert!(
                matches!(result, Err(crate::Error::IllegalTransition)),
                "{from:?} -> {to:?}: {result:?}"
            );
        }
        let observed: (State, bool, bool, bool, i64) = sqlx::query_as(
            "SELECT state,
                EXISTS(SELECT 1 FROM bazaar_coin_holds WHERE id=l.id),
                EXISTS(SELECT 1 FROM bazaar_settlements WHERE listing_id=l.id),
                EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE listing_id=l.id),
                (SELECT count(*) FROM bazaar_audit WHERE row_id=l.id)
             FROM bazaar_listings l WHERE l.id=$1",
        )
        .bind(listing)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            observed,
            (
                if allowed { to } else { from },
                false,
                false,
                false,
                if allowed { 2 } else { 1 }
            )
        );
        tx.rollback().await.unwrap();
    }
    db.end().await;
}

async fn lifecycle_workers_race_to_refund_once() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    recovery_tests::item_effect(&db, Command::Deliver, true).await;
    db.runtime.tick().await.unwrap();
    recovery_tests::item_effect(&db, Command::Return, false).await;
    let worker = db.runtime.clone();
    let (left, right) = tokio::join!(db.runtime.tick(), worker.tick());
    left.unwrap();
    right.unwrap();
    assert_eq!(item_state(&db).await, State::Cancelled);
    let effects = sale_effects(&db).await;
    assert_eq!((effects.0, effects.2, effects.3), (0, true, 1));
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    assert_eq!(dispatched(&db, uid("20"), Command::Return).await, 1);
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        0
    );
    db.end().await;
}

async fn lifecycle_crash_before_rebuild_page_commit_replays_page() {
    use sqlx::Acquire;
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    insert_resync_fillers(&db, 24).await;
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );
    let effects = sale_effects(&db).await;

    let mut producer = db.pool.acquire().await.unwrap();
    let mut observer = db.pool.acquire().await.unwrap();
    let mut abandoned = producer.begin().await.unwrap();
    next.finish_resync(&mut abandoned, &[uid("12")])
        .await
        .unwrap();
    let cursor = "SELECT listing_cursor FROM bazaar_resync_progress
                  WHERE projection_epoch=4 AND recovery_generation=3";
    assert!(
        sqlx::query_scalar::<_, Uuid>(cursor)
            .fetch_optional(&mut *abandoned)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        sqlx::query_scalar::<_, Uuid>(cursor)
            .fetch_optional(&mut *observer)
            .await
            .unwrap()
            .is_none()
    );
    // Dropping the writer and closing its connection rolls back its uncommitted page.
    drop(abandoned);
    producer.close().await.unwrap();
    assert!(
        sqlx::query_scalar::<_, Uuid>(cursor)
            .fetch_optional(&mut *observer)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!next.resynced(&mut observer).await.unwrap());

    db.runtime.tick().await.unwrap();
    assert!(
        sqlx::query_scalar::<_, Uuid>(cursor)
            .fetch_optional(&mut *observer)
            .await
            .unwrap()
            .is_some()
    );
    db.runtime.tick().await.unwrap();
    assert!(next.resynced(&mut observer).await.unwrap());
    assert_eq!(sale_effects(&db).await, effects);
    assert_eq!(dispatched(&db, uid("20"), Command::Deliver).await, 1);
    assert_eq!(
        delivery_reports(&db, uid("20"), "delivery_custody_drift").await,
        0
    );
    drop(observer);
    db.end().await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaleEvidenceState {
    Pending,
    Committed,
    Audited,
    Rejected,
    Refunded,
    Drift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaleStatusEvent {
    Escrowed,
    Delivered,
    Returned,
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Recheck {
    Certified,
    Waiting,
    Drift,
}

const SALE_STATUS_EVENTS: [SaleStatusEvent; 4] = [
    SaleStatusEvent::Escrowed,
    SaleStatusEvent::Delivered,
    SaleStatusEvent::Returned,
    SaleStatusEvent::Missing,
];
const SALE_EVIDENCE_STATES: [SaleEvidenceState; 6] = [
    SaleEvidenceState::Pending,
    SaleEvidenceState::Committed,
    SaleEvidenceState::Audited,
    SaleEvidenceState::Rejected,
    SaleEvidenceState::Refunded,
    SaleEvidenceState::Drift,
];

const SALE_STATUS_ROWS: [[Recheck; 4]; 6] = {
    use Recheck::{Certified, Drift, Waiting};
    [
        [Waiting, Waiting, Waiting, Waiting],
        [Waiting, Waiting, Waiting, Waiting],
        [Drift, Certified, Drift, Waiting],
        [Certified, Drift, Waiting, Waiting],
        [Drift, Drift, Certified, Waiting],
        [Drift, Drift, Drift, Drift],
    ]
};

fn sale_status_rows() -> impl Iterator<Item = (SaleEvidenceState, SaleStatusEvent, Recheck)> {
    SALE_EVIDENCE_STATES
        .into_iter()
        .zip(SALE_STATUS_ROWS)
        .flat_map(|(state, cases)| {
            SALE_STATUS_EVENTS
                .into_iter()
                .zip(cases)
                .map(move |(event, expected)| (state, event, expected))
        })
}

async fn sale_with_evidence(state: SaleEvidenceState) -> BrowserFixture {
    let db = pending_item_delivery().await;
    match state {
        SaleEvidenceState::Pending => {}
        SaleEvidenceState::Committed => {
            let intent = newest_intent(&db, uid("20"), Command::Deliver).await;
            let mut tx = db.pool.begin().await.unwrap();
            let now = runtime::now(&mut tx).await.unwrap();
            let raw = vector("delivery_receipt")
                .replace(
                    &uid("4").to_string(),
                    text(&intent.value, "operation_id").unwrap(),
                )
                .replace("1790000002", &now.to_string());
            let a = db.runtime.authority();
            assert_eq!(
                a.receive(&mut tx, &a.config.receipt_authority, raw.as_bytes(), now)
                    .await
                    .unwrap(),
                ReceiptResult::Stored,
            );
            tx.commit().await.unwrap();
        }
        SaleEvidenceState::Audited | SaleEvidenceState::Drift => {
            recovery_tests::item_effect(&db, Command::Deliver, false).await;
            db.runtime.tick().await.unwrap();
            assert_eq!(item_state(&db).await, State::Completed);
            if state == SaleEvidenceState::Drift {
                let mut tx = db.pool.begin().await.unwrap();
                apply_sale_status(&db, &mut tx, SaleStatusEvent::Escrowed).await;
                tx.commit().await.unwrap();
                assert!(matches!(db.runtime.tick().await, Err(Error::CustodyDrift)));
            }
        }
        SaleEvidenceState::Rejected | SaleEvidenceState::Refunded => {
            recovery_tests::item_effect(&db, Command::Deliver, true).await;
            if state == SaleEvidenceState::Refunded {
                db.runtime.tick().await.unwrap();
                recovery_tests::item_effect(&db, Command::Return, false).await;
                db.runtime.tick().await.unwrap();
                assert_eq!(item_state(&db).await, State::Cancelled);
            }
        }
    }
    db
}

async fn apply_sale_status(
    db: &BrowserFixture,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: SaleStatusEvent,
) {
    let raw: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1",
    )
    .bind(uid("20"))
    .fetch_one(&mut **tx)
    .await
    .unwrap();
    let status = decode(raw.as_bytes(), 262144).unwrap();
    let revision = number(&status, "projection_revision").unwrap();
    let now = runtime::now(tx).await.unwrap();
    let Value::Object(mut fields) = status else {
        panic!("status fixture");
    };
    let new_state = match event {
        SaleStatusEvent::Escrowed => "ESCROWED",
        SaleStatusEvent::Delivered => "DELIVERED",
        SaleStatusEvent::Returned => "RETURNED",
        SaleStatusEvent::Missing => {
            sqlx::query("DELETE FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1")
                .bind(uid("20"))
                .execute(&mut **tx)
                .await
                .unwrap();
            return;
        }
    };
    let current_state = fields
        .iter()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .clone();
    let published_state = Value::String(new_state.into());
    for (key, value) in &mut fields {
        *value = match key.as_str() {
            "state" => published_state.clone(),
            "projection_revision" => Value::String((revision + 1).to_string()),
            "source_observed_at" => Value::String(now.to_string()),
            "last_operation_id" if new_state == "ESCROWED" => Value::Null,
            "last_operation_id" if current_state != published_state => {
                Value::String(uid("79").to_string())
            }
            _ => value.clone(),
        };
    }
    let incoming = canonical::encode(&Value::Object(fields)).unwrap();
    let a = db.runtime.authority();
    let result = projection_ingest()
        .ingest(tx, &a.config.projection_authority, &incoming, now)
        .await
        .unwrap();
    assert!(
        matches!(result, IngestResult::Applied | IngestResult::Quarantined),
        "{result:?}"
    );
}

async fn sale_status_table() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    for state in SALE_EVIDENCE_STATES {
        let db = sale_with_evidence(state).await;
        let before = sale_effects(&db).await;
        let listing_before = item_state(&db).await;
        for (_, event, expected) in sale_status_rows().filter(|(s, ..)| *s == state) {
            let mut tx = db.pool.begin().await.unwrap();
            // Seed a restart at this fence. The event is applied by Game's
            // projection writer and consumed by the same rebuild as Runtime.
            sqlx::query("UPDATE bazaar_resync_progress SET worlds='obsolete'")
                .execute(&mut *tx)
                .await
                .unwrap();
            apply_sale_status(&db, &mut tx, event).await;
            let a = db.runtime.authority();
            let outcome = a.finish_resync(&mut tx, &[uid("12")]).await;
            let observed = match outcome {
                Ok(()) if a.resynced(&mut tx).await.unwrap() => Recheck::Certified,
                Err(Error::Unavailable | Error::DeliveryEvidencePending) => Recheck::Waiting,
                Err(Error::CustodyDrift) => Recheck::Drift,
                outcome => panic!("{state:?} × {event:?}: {outcome:?}"),
            };
            assert_eq!(observed, expected, "{state:?} × {event:?}");
            let reports: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM bazaar_audit WHERE row_id=$1 AND after_row->>'action'='delivery_custody_drift'",
            ).bind(uid("20")).fetch_one(&mut *tx).await.unwrap();
            assert_eq!(reports, i64::from(expected == Recheck::Drift));
            let operations: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM bazaar_game_outbox WHERE listing_id=$1 AND command='DeliverBazaarItem'",
            ).bind(uid("20")).fetch_one(&mut *tx).await.unwrap();
            assert_eq!(operations, 1);
            assert_eq!(sale_effects_on(&mut tx).await, before);
            let listing: State =
                sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
                    .bind(uid("20"))
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(listing, listing_before);
            tx.rollback().await.unwrap();
        }
        db.end().await;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeliveryAdmission {
    Pending,
    Committed,
    Rejected,
    AuditedCommitment,
    AuditedRejection,
    Drift,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeliveryInput {
    Commitment,
    Rejection,
    Audit,
}

const ADMISSION_STATES: [DeliveryAdmission; 6] = {
    use DeliveryAdmission::*;
    [
        Pending,
        Committed,
        Rejected,
        AuditedCommitment,
        AuditedRejection,
        Drift,
    ]
};
const ADMISSION_INPUTS: [DeliveryInput; 3] = [
    DeliveryInput::Commitment,
    DeliveryInput::Rejection,
    DeliveryInput::Audit,
];
const ADMISSION_ROWS: [[(DeliveryAdmission, Option<ReceiptResult>); 3]; 6] = {
    use DeliveryAdmission::*;
    use ReceiptResult::*;
    [
        [
            (Committed, Some(Stored)),
            (Rejected, Some(Stored)),
            (Pending, None),
        ],
        [
            (Committed, Some(Duplicate)),
            (Drift, Some(Quarantined)),
            (AuditedCommitment, Some(Stored)),
        ],
        [
            (Drift, Some(Quarantined)),
            (Rejected, Some(Duplicate)),
            (AuditedRejection, Some(Stored)),
        ],
        [
            (AuditedCommitment, Some(Duplicate)),
            (Drift, Some(Quarantined)),
            (AuditedCommitment, Some(Stored)),
        ],
        [
            (Drift, Some(Quarantined)),
            (AuditedRejection, Some(Duplicate)),
            (AuditedRejection, Some(Stored)),
        ],
        [
            (Drift, Some(Duplicate)),
            (Drift, Some(Quarantined)),
            (Drift, Some(Stored)),
        ],
    ]
};

fn delivery_admission_rows() -> impl Iterator<
    Item = (
        DeliveryAdmission,
        DeliveryInput,
        DeliveryAdmission,
        Option<ReceiptResult>,
    ),
> {
    ADMISSION_STATES
        .into_iter()
        .zip(ADMISSION_ROWS)
        .flat_map(|(state, results)| {
            ADMISSION_INPUTS
                .into_iter()
                .zip(results)
                .map(move |(input, (next, result))| (state, input, next, result))
        })
}

async fn delivery_admission_table() {
    use DeliveryAdmission::*;
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = pending_item_delivery().await;
    let a = db.runtime.authority();
    let intent = newest_intent(&db, uid("20"), Command::Deliver).await;
    let operation = text(&intent.value, "operation_id").unwrap();
    let operation_id = id(operation).unwrap();
    let before = sale_effects(&db).await;
    assert_eq!(before, (10000, false, false, 0));
    for (state, input, expected, intake) in delivery_admission_rows() {
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let committed = vector("delivery_receipt")
            .replace(&uid("4").to_string(), operation)
            .replace("1790000002", &now.to_string());
        let rejected = format!(
            r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"DeliverBazaarItem","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE","result_data":null,"recovery_generation":"3","decided_at":"{now}"}}"#,
        );
        let prior = if matches!(state, Rejected | AuditedRejection) {
            &rejected
        } else {
            &committed
        };
        if state != Pending {
            assert_eq!(
                a.receive(&mut tx, &a.config.receipt_authority, prior.as_bytes(), now)
                    .await
                    .unwrap(),
                ReceiptResult::Stored
            );
        }
        if matches!(state, AuditedCommitment | AuditedRejection | Drift) {
            assert_eq!(
                audit(a, &mut tx, &intent, prior, now).await,
                ReceiptResult::Stored
            );
        }
        sqlx::query("UPDATE bazaar_resync_progress SET worlds='obsolete'")
            .execute(&mut *tx)
            .await
            .unwrap();
        if state == Drift {
            assert_eq!(
                a.receive(
                    &mut tx,
                    &a.config.receipt_authority,
                    rejected.as_bytes(),
                    now
                )
                .await
                .unwrap(),
                ReceiptResult::Quarantined
            );
            assert!(matches!(
                a.finish_resync(&mut tx, &[uid("12")]).await,
                Err(Error::CustodyDrift)
            ));
        }

        let observed = match input {
            DeliveryInput::Commitment => {
                a.receive(
                    &mut tx,
                    &a.config.receipt_authority,
                    committed.as_bytes(),
                    now,
                )
                .await
            }
            DeliveryInput::Rejection => {
                a.receive(
                    &mut tx,
                    &a.config.receipt_authority,
                    rejected.as_bytes(),
                    now,
                )
                .await
            }
            DeliveryInput::Audit => audit_result(a, &mut tx, &intent, prior, now).await,
        };
        let label = format!("{state:?} × {input:?}");
        match intake {
            Some(result) => assert_eq!(observed.unwrap(), result, "{label}"),
            None => assert!(
                matches!(observed, Err(Error::Unavailable)),
                "{label}: {observed:?}"
            ),
        }

        let row: (Option<String>, bool, bool) = sqlx::query_as(
            "SELECT (SELECT outcome FROM bazaar_authority_receipts WHERE operation_id=$1),
                EXISTS(SELECT 1 FROM bazaar_authority_decisions WHERE operation_id=$1),
                EXISTS(SELECT 1 FROM bazaar_authority_conflicts WHERE operation_id=$1)",
        )
        .bind(operation_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let actual = match (row.0.as_deref(), row.1, row.2) {
            (_, _, true) => Drift,
            (None, false, false) => Pending,
            (Some("COMMITTED"), false, false) => Committed,
            (Some("REJECTED"), false, false) => Rejected,
            (Some("COMMITTED"), true, false) => AuditedCommitment,
            (Some("REJECTED"), true, false) => AuditedRejection,
            _ => panic!("{label}: {row:?}"),
        };
        assert_eq!(actual, expected, "{label}");
        let outcome = a.finish_resync(&mut tx, &[uid("12")]).await;
        match expected {
            Drift => assert!(
                matches!(outcome, Err(Error::CustodyDrift)),
                "{label}: {outcome:?}"
            ),
            AuditedRejection => assert!(outcome.is_ok(), "{label}: {outcome:?}"),
            _ => assert!(
                matches!(
                    outcome,
                    Err(Error::Unavailable | Error::DeliveryEvidencePending)
                ),
                "{label}: {outcome:?}"
            ),
        }
        assert_eq!(
            a.resynced(&mut tx).await.unwrap(),
            expected == AuditedRejection,
            "{label}"
        );
        assert_eq!(sale_effects_on(&mut tx).await, before, "{label}");
        let durable: (State, i64, String, i64, i64) = sqlx::query_as(
            "SELECT l.state,
                (SELECT count(*) FROM bazaar_game_outbox WHERE listing_id=l.id AND command='DeliverBazaarItem'),
                (SELECT canonical_body::jsonb->>'state' FROM bazaar_game_projections WHERE kind='STATUS' AND subject=l.id),
                (SELECT count(*) FROM bazaar_audit WHERE row_id=l.id AND after_row->>'action'='delivery_custody_drift'),
                (SELECT count(*) FROM bazaar_audit WHERE row_id=l.id AND after_row->>'action'='delivery_evidence_pending')
             FROM bazaar_listings l WHERE l.id=$1",
        )
        .bind(uid("20"))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            durable,
            (
                State::AwaitingDelivery,
                1,
                "ESCROWED".to_owned(),
                i64::from(expected == Drift),
                i64::from(!matches!(expected, Drift | AuditedRejection))
            ),
            "{label}",
        );
        tx.rollback().await.unwrap();
    }
    db.end().await;
}
