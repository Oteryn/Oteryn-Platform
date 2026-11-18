use super::*;
use crate::State;
use crate::moderation::{StaffCommand, execute_staff};

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

#[derive(Clone, Copy)]
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

macro_rules! listing_cases {
    ($($name:ident, $state:ident, $event:ident, $outcome:expr;)*) => {
        $(
            #[tokio::test]

            async fn $name() {
                listing_event(State::$state, ListingEvent::$event, $outcome).await;
            }
        )*
    };
}

listing_cases! {
    lifecycle_draft_bid, Draft, Bid, Err("bazaar authorization denied");
    lifecycle_draft_cancel, Draft, Cancel, Ok(());
    lifecycle_draft_suspend, Draft, Suspend, Ok(());
    lifecycle_active_bid, Active, Bid, Ok(());
    lifecycle_active_cancel, Active, Cancel, Ok(());
    lifecycle_active_suspend, Active, Suspend, Ok(());
    lifecycle_matched_bid, Matched, Bid, Err("bazaar authorization denied");
    lifecycle_matched_cancel, Matched, Cancel, Err("bazaar authorization denied");
    lifecycle_matched_suspend, Matched, Suspend, Err("illegal bazaar transition");
    lifecycle_awaiting_delivery_bid, AwaitingDelivery, Bid, Err("bazaar authorization denied");
    lifecycle_awaiting_delivery_cancel, AwaitingDelivery, Cancel, Err("bazaar authorization denied");
    lifecycle_awaiting_delivery_suspend, AwaitingDelivery, Suspend, Err("illegal bazaar transition");
    lifecycle_completed_bid, Completed, Bid, Err("bazaar game integration unavailable");
    lifecycle_completed_cancel, Completed, Cancel, Err("bazaar authorization denied");
    lifecycle_completed_suspend, Completed, Suspend, Err("illegal bazaar transition");
    lifecycle_cancelled_bid, Cancelled, Bid, Err("bazaar authorization denied");
    lifecycle_cancelled_cancel, Cancelled, Cancel, Err("bazaar authorization denied");
    lifecycle_cancelled_suspend, Cancelled, Suspend, Err("illegal bazaar transition");
    lifecycle_expired_bid, Expired, Bid, Err("bazaar authorization denied");
    lifecycle_expired_cancel, Expired, Cancel, Err("bazaar authorization denied");
    lifecycle_expired_suspend, Expired, Suspend, Err("illegal bazaar transition");
}

#[tokio::test]
async fn lifecycle_draft_advance_escrowed() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Draft).await;
    authorize_seller(&db).await;
    db.runtime.tick().await.unwrap();
    let state = item_state(&db).await;
    db.end().await;
    assert_eq!(state, State::Active);
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
    std::sync::Arc::make_mut(&mut db.runtime.config).worlds = vec![uid("12")];
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[derive(Clone, Copy)]
enum StatusEvent {
    Newer,
    Older,
    Replay,
    Conflict,
}

async fn custody_status_event(state: &str, event: StatusEvent) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Active).await;
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
    let (incoming, result) = match event {
        StatusEvent::Newer => (status_publication(state, 11, 3, now), IngestResult::Applied),
        StatusEvent::Older => (
            status_publication(state, 9, 1, now),
            IngestResult::Superseded,
        ),
        StatusEvent::Replay => (original.clone(), IngestResult::Duplicate),
        StatusEvent::Conflict => (
            status_publication(state, 10, 3, now),
            IngestResult::Quarantined,
        ),
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
    tx.commit().await.unwrap();
    let subject = if matches!(state, "locked" | "UNLOCKED") {
        uid("30")
    } else {
        uid("20")
    };
    let stored: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='STATUS' AND subject=$1",
    )
    .bind(subject)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let quarantined: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_game_quarantine WHERE kind='STATUS' AND subject=$1)",
    )
    .bind(subject)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let listing = item_state(&db).await;
    db.end().await;
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

macro_rules! status_cases {
    ($($state:literal, $newer:ident, $older:ident, $replay:ident, $conflict:ident;)*) => {
        $(
            #[tokio::test]

            async fn $newer() { custody_status_event($state, StatusEvent::Newer).await; }

            #[tokio::test]

            async fn $older() { custody_status_event($state, StatusEvent::Older).await; }

            #[tokio::test]

            async fn $replay() { custody_status_event($state, StatusEvent::Replay).await; }

            #[tokio::test]

            async fn $conflict() { custody_status_event($state, StatusEvent::Conflict).await; }
        )*
    };
}

status_cases! {
    "locked", lifecycle_locked_status_newer, lifecycle_locked_status_older,
        lifecycle_locked_status_replay, lifecycle_locked_status_conflict;
    "ESCROWED", lifecycle_escrowed_status_newer, lifecycle_escrowed_status_older,
        lifecycle_escrowed_status_replay, lifecycle_escrowed_status_conflict;
    "DELIVERED", lifecycle_delivered_status_newer, lifecycle_delivered_status_older,
        lifecycle_delivered_status_replay, lifecycle_delivered_status_conflict;
    "RETURNED", lifecycle_returned_status_newer, lifecycle_returned_status_older,
        lifecycle_returned_status_replay, lifecycle_returned_status_conflict;
    "UNLOCKED", lifecycle_unlocked_status_newer, lifecycle_unlocked_status_older,
        lifecycle_unlocked_status_replay, lifecycle_unlocked_status_conflict;
    "INVALID", lifecycle_invalid_status_newer, lifecycle_invalid_status_older,
        lifecycle_invalid_status_replay, lifecycle_invalid_status_conflict;
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

#[tokio::test]
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
    config.worlds = vec![uid("12")];
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn bz03_status_committed_behind_cursor_is_rechecked() {
    dirty_status_rechecked(false).await;
}

#[tokio::test]
async fn bz03_status_committed_after_certification_is_rechecked() {
    dirty_status_rechecked(true).await;
}

#[tokio::test]
async fn bz05_drift_does_not_starve_a_later_page() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    insert_resync_fillers(&db, 23).await;
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
    let original = newest_intent(&db, uid("20"), Command::Deliver).await;
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

#[tokio::test]
async fn bz06_missing_configured_world_does_not_charge_delivery_retries() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_in(State::Completed).await;
    let next = rebuild_epoch(&db).await;
    set_resync_fence(
        &mut db,
        next.fence.projection_epoch,
        next.fence.recovery_generation,
    );
    let mut tx = db.pool.begin().await.unwrap();
    let policy: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='POLICY' AND subject=$1",
    )
    .bind(uid("12"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let alternate = uid("76");
    let policy = policy.replace(&uid("12").to_string(), &alternate.to_string());
    let now = runtime::now(&mut tx).await.unwrap();
    assert_eq!(
        projection_ingest()
            .ingest(
                &mut tx,
                &next.config.projection_authority,
                policy.as_bytes(),
                now
            )
            .await
            .unwrap(),
        IngestResult::Applied
    );
    tx.commit().await.unwrap();
    std::sync::Arc::make_mut(&mut db.runtime.config).worlds = vec![alternate];
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
    std::sync::Arc::make_mut(&mut db.runtime.config).worlds = vec![uid("12")];
    db.runtime.tick().await.unwrap();
    assert!(resync_progress(&db).await.0);
    db.end().await;
}
