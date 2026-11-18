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
    lifecycle_matched_suspend, Matched, Suspend, Ok(());
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

#[tokio::test]
async fn lifecycle_completed_resync_escrow_status() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = item_in(State::Completed).await;
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let policy: String = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE kind='POLICY'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let policy = policy.replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#);
    let status = vector("delivered_status")
        .replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#)
        .replace("1790000002", &now.to_string())
        .replace("DELIVERED", "ESCROWED")
        .replace(&format!("\"{}\"", uid("4")), "null");
    for projection in [policy, status] {
        assert_eq!(
            projection_ingest()
                .ingest(
                    &mut tx,
                    &a.config.projection_authority,
                    projection.as_bytes(),
                    now
                )
                .await
                .unwrap(),
            IngestResult::Applied
        );
    }
    tx.commit().await.unwrap();
    let mut current = a.clone();
    current.fence.projection_epoch = 4;
    let result = resync(&db, &current).await;
    let state = item_state(&db).await;
    db.end().await;
    assert_eq!(
        state,
        State::Completed,
        "recovery must preserve the commercial result"
    );
    assert!(
        result.is_err(),
        "resync accepted ESCROWED after committed delivery: {result:?}"
    );
}
