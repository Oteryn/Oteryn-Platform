use super::*;

pub(super) async fn replay(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    listing: Uuid,
) {
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&mut **tx)
        .await
        .unwrap();
    let now = runtime::now(tx).await.unwrap();
    let rows: Vec<(String,String)> = sqlx::query_as("SELECT intent,receipt FROM bazaar_game_outbox JOIN bazaar_authority_receipts USING(operation_id) WHERE listing_id=$1 AND bazaar_authority_receipts.recovery_generation=3 ORDER BY source_revision").bind(listing).fetch_all(&mut **tx).await.unwrap();
    for (intent, raw) in rows {
        let intent = PreparedIntent::parse(&intent).unwrap();
        let raw = raw.replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"4""#,
        );
        let result = a
            .receive(tx, &a.config.receipt_authority, raw.as_bytes(), now)
            .await;
        assert!(
            matches!(result, Ok(ReceiptResult::Stored)),
            "{:?} replay: {result:?}",
            intent.command()
        );
        assert_eq!(
            audit(a, tx, &intent, &raw, now).await,
            ReceiptResult::Stored
        );
    }
}

async fn item_sale(gold: bool, rebid: bool) -> BrowserFixture {
    let db = if gold {
        let db = BrowserFixture::new().await;
        let mut tx = db.pool.begin().await.unwrap();
        if rebid {
            gold_fact(
                &mut tx,
                Command::Hold,
                10,
                uid("80"),
                uid("81"),
                (0, 1, 9000),
            )
            .await;
            gold_fact(
                &mut tx,
                Command::Release,
                11,
                uid("80"),
                uid("81"),
                (1, 2, 9000),
            )
            .await;
        }
        gold_fact(
            &mut tx,
            Command::Hold,
            12,
            uid("82"),
            uid("83"),
            (0, 1, 10000),
        )
        .await;
        sqlx::query("UPDATE bazaar_game_issuer SET source_revision=100")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE bazaar_listings SET currency='gold',leader=$1,leader_max=10000,reservation=id,ends_at=now()-interval '1 second'").bind(uid("13")).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO bazaar_authority_destinations(listing_id,buyer,character_id) VALUES($1,$2,$3)").bind(uid("20")).bind(uid("13")).bind(uid("14")).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        db
    } else {
        expired_item_with_bid(1).await
    };
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    db
}

pub(super) async fn item_effect(db: &BrowserFixture, command: Command, refused: bool) {
    let i = newest_intent(db, uid("20"), command).await;
    let op = text(&i.value, "operation_id").unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let a = db.runtime.authority();
    let mut raw = vector("delivery_receipt")
        .replace(&uid("4").to_string(), op)
        .replace("1790000002", &now.to_string());
    let mut status = vector("delivered_status")
        .replace(&uid("4").to_string(), op)
        .replace("1790000002", &now.to_string());
    if refused {
        raw = format!(
            r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"DeliverBazaarItem","operation_id":"{op}","outcome":"REJECTED","result_code":"CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE","result_data":null,"recovery_generation":"3","decided_at":"{now}"}}"#
        );
    } else if command == Command::Return {
        raw = raw
            .replace("DeliverBazaarItem", "ReturnBazaarItem")
            .replace("000000000013", "000000000011")
            .replace("000000000014", "000000000021");
        status = status.replace("DELIVERED", "RETURNED");
    }
    let raw = raw.replace(
        r#""recovery_generation":"3""#,
        &format!(r#""recovery_generation":"{}""#, a.fence.recovery_generation),
    );
    assert_eq!(
        a.receive(&mut tx, &a.config.receipt_authority, raw.as_bytes(), now)
            .await
            .unwrap(),
        ReceiptResult::Stored
    );
    assert_eq!(
        audit(a, &mut tx, &i, &raw, now).await,
        ReceiptResult::Stored
    );
    if !refused {
        Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: false,
            },
        }
        .ingest(
            &mut tx,
            &a.config.projection_authority,
            status.as_bytes(),
            now,
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

async fn recover(db: &mut BrowserFixture) {
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.fence.recovery_generation = 4;
    config.worlds = vec![uid("12")];
    let fence = serde_json::json!({"projection_epoch":3,"recovery_generation":4,"incarnation":config.fence.incarnation,"character_history_complete_since":null});
    std::fs::write(&config.fence_file, fence.to_string()).unwrap();
    let a = &mut config.authority;
    a.fence.recovery_generation = 4;
    let mut tx = db.pool.begin().await.unwrap();
    replay(a, &mut tx, uid("20")).await;
    tx.commit().await.unwrap();
}

async fn state(db: &BrowserFixture) -> crate::State {
    sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(uid("20"))
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

async fn payout(rebid: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_sale(true, rebid).await;
    item_effect(&db, Command::Deliver, false).await;
    db.runtime.tick().await.unwrap();
    let i = newest_intent(&db, uid("20"), Command::Payout).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"2","sale_id":"{}","gross_amount":"10000","fee_amount":"250","seller_amount":"9750","remaining_held":"0","buyer_ledger_entry_id":"{}","seller_ledger_entry_id":"{}","burn_entry_id":"{}","delivery_operation_id":"{}","delivery_receipt_digest":"{}"}}"#,
        uid("83"),
        text(&i.value, "sale_id").unwrap(),
        uid("51"),
        uid("52"),
        uid("53"),
        text(&i.value, "delivery_operation_id").unwrap(),
        text(&i.value, "delivery_receipt_digest").unwrap()
    );
    if rebid {
        committed_fact(db.runtime.authority(), &mut tx, &i, &data, now).await;
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
        assert_eq!(state(&db).await, crate::State::Completed);
        recover(&mut db).await;
    } else {
        tx.rollback().await.unwrap();
        recover(&mut db).await;
        let mut tx = db.pool.begin().await.unwrap();
        let a = db.runtime.authority();
        let op = text(&i.value, "operation_id").unwrap();
        let raw = format!(
            r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"PayoutBazaarGold","operation_id":"{op}","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","result_data":{data},"recovery_generation":"4","decided_at":"{now}"}}"#
        );
        assert_eq!(
            a.receive(&mut tx, &a.config.receipt_authority, raw.as_bytes(), now)
                .await
                .unwrap(),
            ReceiptResult::Stored
        );
        assert_eq!(
            audit(a, &mut tx, &i, &raw, now).await,
            ReceiptResult::Stored
        );
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
        assert_eq!(state(&db).await, crate::State::Completed);
    }
    db.end().await;
}

#[tokio::test]
async fn r13_previous_winner_hold() {
    payout(true).await;
}
#[tokio::test]
async fn r13_missing_payout() {
    payout(false).await;
}

async fn worker(returning: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_sale(false, false).await;
    item_effect(&db, Command::Deliver, returning).await;
    if returning {
        db.runtime.tick().await.unwrap();
        item_effect(&db, Command::Return, false).await;
    }
    recover(&mut db).await;
    db.runtime.tick().await.unwrap();
    assert_eq!(
        state(&db).await,
        if returning {
            crate::State::Cancelled
        } else {
            crate::State::Completed
        }
    );
    db.end().await;
}
#[tokio::test]
async fn r13_worker_deliver() {
    worker(false).await;
}
#[tokio::test]
async fn r13_worker_return() {
    worker(true).await;
}

#[tokio::test]
async fn r13_compensated_release() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_sale(true, false).await;
    item_effect(&db, Command::Deliver, true).await;
    db.runtime.tick().await.unwrap();
    item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    let i = newest_intent(&db, uid("20"), Command::Release).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"2","released_amount":"10000","remaining_held":"0","hold_state":"RELEASED"}}"#,
        uid("83")
    );
    committed_fact(db.runtime.authority(), &mut tx, &i, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(state(&db).await, crate::State::Cancelled);
    recover(&mut db).await;
    db.end().await;
}

#[tokio::test]
async fn r14_lock_replay_after_epoch_change_finishes_resync() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let mut a = db.runtime.authority().clone();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'character','auction','gold',100,false,now())").bind(uid("30")).bind(uid("11")).execute(&mut *tx).await.unwrap();
    commit_character_lock(&a, &mut tx, now).await;
    a.fence.projection_epoch = 4;
    a.fence.recovery_generation = 4;
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    replay(&a, &mut tx, uid("30")).await;
    replay(&a, &mut tx, uid("20")).await;
    replay_projections(&a, &mut tx, now).await;
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    assert!(a.resynced(&mut tx).await.unwrap());
    tx.commit().await.unwrap();
    db.end().await;
}

async fn admit_gold_payout(db: &BrowserFixture, listing: Uuid, revision: u64) {
    let payout = newest_intent(db, listing, Command::Payout).await;
    let field = |key| text(&payout.value, key).unwrap();
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"{revision}","sale_id":"{}","gross_amount":"{}","fee_amount":"{}","seller_amount":"{}","remaining_held":"0","buyer_ledger_entry_id":"{}","seller_ledger_entry_id":"{}","burn_entry_id":"{}","delivery_operation_id":"{}","delivery_receipt_digest":"{}"}}"#,
        field("hold_id"),
        field("sale_id"),
        field("gross_amount"),
        field("fee_amount"),
        field("seller_amount"),
        uid("51"),
        uid("52"),
        uid("53"),
        field("delivery_operation_id"),
        field("delivery_receipt_digest")
    );
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    committed_fact(db.runtime.authority(), &mut tx, &payout, &data, now).await;
    tx.commit().await.unwrap();
}

async fn finish_gold_payout(db: &BrowserFixture, listing: Uuid, revision: u64) {
    admit_gold_payout(db, listing, revision).await;
    db.runtime.tick().await.unwrap();
    let state: crate::State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, crate::State::Completed);
}

#[tokio::test]
async fn r14_excess_release_replays_before_delivery() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let buyer = uid("13");
    let hold = uid("83");
    gold_fact(&mut tx, Command::Hold, 10, uid("82"), hold, (0, 1, 10000)).await;
    gold_fact(&mut tx, Command::Hold, 11, uid("82"), hold, (1, 2, 12000)).await;
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=11")
        .execute(&mut *tx)
        .await
        .unwrap();
    let listing: Listing = sqlx::query_as("UPDATE bazaar_listings SET currency='gold',leader=$1,leader_max=10000,reservation=id,ends_at=now()-interval '1 second' RETURNING *").bind(buyer).fetch_one(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO bazaar_authority_destinations(listing_id,buyer,character_id) VALUES($1,$2,$3)",
    )
    .bind(listing.id)
    .bind(buyer)
    .bind(uid("14"))
    .execute(&mut *tx)
    .await
    .unwrap();
    db.runtime
        .issue(
            &mut tx,
            &listing,
            listing.owner,
            None,
            Command::Release,
            uid("82"),
            vec![
                ("buyer_account_id", Value::String(buyer.to_string())),
                ("hold_id", Value::String(hold.to_string())),
                ("expected_hold_revision", Value::String("2".into())),
                ("release_reason", Value::String("EXCESS".into())),
                ("amount", Value::String("2000".into())),
            ],
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let release = newest_intent(&db, listing.id, Command::Release).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"hold_id":"{hold}","hold_revision":"3","released_amount":"2000","remaining_held":"10000","hold_state":"ACTIVE"}}"#
    );
    committed_fact(db.runtime.authority(), &mut tx, &release, &data, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(state(&db).await, crate::State::AwaitingDelivery);
    recover(&mut db).await;
    db.runtime.tick().await.unwrap();
    item_effect(&db, Command::Deliver, false).await;
    db.runtime.tick().await.unwrap();
    finish_gold_payout(&db, listing.id, 4).await;
    db.end().await;
}

async fn reattest_transfers(db: &BrowserFixture, listing: Uuid) {
    let mut tx = db.pool.begin().await.unwrap();
    let receipts: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id,receipt_json FROM character_commands WHERE state='TERMINAL'")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    for (operation, raw) in receipts {
        sqlx::query("UPDATE character_commands SET receipt_json=NULL,state='PENDING' WHERE id=$1")
            .bind(operation)
            .execute(&mut *tx)
            .await
            .unwrap();
        let current = raw.replace(
            r#""recovery_generation":"3""#,
            r#""recovery_generation":"4""#,
        );
        sqlx::query("UPDATE character_commands SET receipt_json=$2,state='TERMINAL' WHERE id=$1")
            .bind(operation)
            .bind(current)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    replay(db.runtime.authority(), &mut tx, listing).await;
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn r14_character_payout_retries_after_transfer_reattestation() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = delivered_character_sale("gold").await;
    let listing = uid("30");
    let payout = newest_intent(&db, listing, Command::Payout).await;
    reject_expired(&db, &payout).await;
    recover(&mut db).await;
    reattest_transfers(&db, listing).await;
    db.runtime.tick().await.unwrap();
    let retry = newest_intent(&db, listing, Command::Payout).await;
    assert_ne!(
        retry.value.get("operation_id"),
        payout.value.get("operation_id")
    );
    assert_ne!(
        retry.value.get("delivery_receipt_digest"),
        payout.value.get("delivery_receipt_digest")
    );
    finish_gold_payout(&db, listing, 2).await;
    db.end().await;
}

#[tokio::test]
async fn character_sale_settles_after_projection_epoch_advance_with_reattested_lock() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = recovered_character_sale().await;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(uid("11"))
        .execute(&db.pool)
        .await
        .unwrap();
    finish_character_handoff(&db, uid("13"), "DELIVERED").await;
    assert_eq!(listing_state(&db).await, crate::State::Completed);
    db.runtime.tick().await.unwrap();
    assert_eq!(outbox_count(&db, Command::Unlock).await, 1);
    db.end().await;
}

async fn replay_projections(
    a: &Authority,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    now: u64,
) {
    let bodies: Vec<String> = sqlx::query_scalar(
        "SELECT canonical_body FROM bazaar_game_projections WHERE projection_epoch=3",
    )
    .fetch_all(&mut **tx)
    .await
    .unwrap();
    for body in bodies {
        let body = body.replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#);
        Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: false,
            },
        }
        .ingest(tx, &a.config.projection_authority, body.as_bytes(), now)
        .await
        .unwrap();
    }
}

async fn recovered_character_sale() -> BrowserFixture {
    let mut db = settling_character_fixture("coins").await;
    recover_character(&mut db).await;
    db
}

pub(super) async fn recover_character(db: &mut BrowserFixture) {
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.fence.projection_epoch = 4;
    config.fence.recovery_generation = 4;
    config.worlds = vec![uid("12")];
    let fence = serde_json::json!({"projection_epoch":4,"recovery_generation":4,"incarnation":config.fence.incarnation,"character_history_complete_since":null});
    std::fs::write(&config.fence_file, fence.to_string()).unwrap();
    config.authority.fence.projection_epoch = 4;
    config.authority.fence.recovery_generation = 4;
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    let receipts: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id,receipt_json FROM character_commands WHERE state='TERMINAL'")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    for (operation, raw) in receipts {
        sqlx::query("UPDATE character_commands SET receipt_json=NULL,state='PENDING' WHERE id=$1")
            .bind(operation)
            .execute(&mut *tx)
            .await
            .unwrap();
        let current = raw
            .replace(r#""projection_epoch":"3""#, r#""projection_epoch":"4""#)
            .replace(
                r#""recovery_generation":"3""#,
                r#""recovery_generation":"4""#,
            );
        sqlx::query("UPDATE character_commands SET receipt_json=$2,state='TERMINAL' WHERE id=$1")
            .bind(operation)
            .bind(current)
            .execute(&mut *tx)
            .await
            .unwrap();
    }

    replay(a, &mut tx, uid("30")).await;
    replay(a, &mut tx, uid("20")).await;
    replay_projections(a, &mut tx, now).await;
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    assert!(a.resynced(&mut tx).await.unwrap());

    tx.commit().await.unwrap();
}

#[tokio::test]
async fn restricted_character_buyer_after_epoch_advance_gets_refund_and_unlock() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = recovered_character_sale().await;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(uid("13"))
        .execute(&db.pool)
        .await
        .unwrap();
    finish_character_handoff(&db, uid("11"), "RETURNED").await;
    assert_eq!(listing_state(&db).await, crate::State::Cancelled);
    assert_eq!(coin_refunds(&db).await, (1, 100));
    assert_eq!(outbox_count(&db, Command::Unlock).await, 1);
    db.end().await;
}

#[tokio::test]
async fn delayed_first_lock_uses_issuance_epoch() {
    let db = BrowserFixture::new().await;
    let mut a = db.runtime.authority().clone();
    let mut tx = db.pool.begin().await.unwrap();
    a.fence.projection_epoch = 4;
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    commit_character_lock(&a, &mut tx, now).await;
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn accepted_restore_notice_fences_bazaar() {
    let db = BrowserFixture::new().await;
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    a.lock(&mut tx).await.unwrap();
    let mut notice = db.pool.begin().await.unwrap();
    let waiting =
        sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR UPDATE NOWAIT")
            .execute(&mut *notice)
            .await
            .unwrap_err();
    assert_eq!(
        waiting.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    notice.rollback().await.unwrap();
    sqlx::query("INSERT INTO character_restore_notices(restore_event_id,notice_json,new_recovery_generation) VALUES(gen_random_uuid(),'{}','00000000000000000004')").execute(&mut *tx).await.unwrap();
    assert!(matches!(a.lock(&mut tx).await, Err(Error::Unavailable)));
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn epoch_rebuild_retains_character_transfer() {
    let db = settling_character_fixture("coins").await;
    let mut a = db.runtime.authority().clone();
    a.fence.projection_epoch = 4;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    replay_projections(&a, &mut tx, now).await;
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    assert!(a.resynced(&mut tx).await.unwrap());
    tx.rollback().await.unwrap();
    db.end().await;
}

#[test]
fn escrow_receipt_accepts_five_seconds_of_issue_skew() {
    let intent = PreparedIntent::parse(&vector("escrow_intent")).unwrap();
    let issued = number(&intent.value, "issued_at_source").unwrap();
    let raw = vector("escrow_receipt");
    let decided = number(&Receipt::parse(raw.as_bytes()).unwrap().value, "decided_at").unwrap();
    for skew in [6, 5, 1] {
        let raw = raw.replace(
            &format!(r#""decided_at":"{decided}""#),
            &format!(r#""decided_at":"{}""#, issued - skew),
        );
        assert_eq!(
            Receipt::parse(raw.as_bytes())
                .unwrap()
                .bind(&intent, issued)
                .is_ok(),
            skew <= 5
        );
    }
}

async fn reversed_sale(gold: bool) {
    let db = item_sale(gold, false).await;
    item_effect(&db, Command::Deliver, false).await;
    db.runtime.tick().await.unwrap();
    if gold {
        finish_gold_payout(&db, uid("20"), 2).await;
    }
    assert_eq!(state(&db).await, crate::State::Completed);
    let command = if gold {
        Command::Payout
    } else {
        Command::Deliver
    };
    let intent = newest_intent(&db, uid("20"), command).await;
    let mut a = db.runtime.authority().clone();
    a.fence.recovery_generation = 4;
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET recovery_generation=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let operation = text(&intent.value, "operation_id").unwrap();
    let raw = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"{}","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_INTENT_EXPIRED","result_data":null,"recovery_generation":"4","decided_at":"{now}"}}"#,
        command.wire()
    );
    assert_eq!(
        a.receive(&mut tx, &a.config.receipt_authority, raw.as_bytes(), now)
            .await
            .unwrap(),
        ReceiptResult::Quarantined
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM bazaar_authority_conflicts WHERE operation_id=$1)"
        )
        .bind(id(operation).unwrap())
        .fetch_one(&mut *tx)
        .await
        .unwrap()
    );
    assert_eq!(
        audit(&a, &mut tx, &intent, &raw, now).await,
        ReceiptResult::Stored
    );
    assert!(
        a.retained(&mut tx, id(operation).unwrap())
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn restored_coin_delivery_reversal_is_quarantined() {
    reversed_sale(false).await;
}
#[tokio::test]
async fn restored_gold_payout_reversal_is_quarantined() {
    reversed_sale(true).await;
}

#[tokio::test]
async fn rejected_deposit_rebuild_needs_no_custody_status() {
    let db = BrowserFixture::new().await;
    let listing = uid("90");
    let mut tx = db.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,now())").bind(listing).bind(uid("11")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding,refusal_code) VALUES($1,$1,$2,$3,$1,7,$4,1,true,'refused','REFUSED')").bind(listing).bind(uid("12")).bind(uid("21")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    let raw = vector("escrow_intent")
        .replace(&uid("20").to_string(), &listing.to_string())
        .replace(&uid("2").to_string(), &uid("91").to_string())
        .replace(r#""source_revision":"18""#, r#""source_revision":"101""#);
    let intent = PreparedIntent::parse(&raw).unwrap();
    store_intent(&mut tx, &intent).await;
    tx.commit().await.unwrap();
    reject(&db, &intent, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
    let mut a = db.runtime.authority().clone();
    a.fence.projection_epoch = 4;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=4")
        .execute(&mut *tx)
        .await
        .unwrap();
    replay_projections(&a, &mut tx, now).await;
    a.finish_resync(&mut tx, &[uid("12")]).await.unwrap();
    assert!(a.resynced(&mut tx).await.unwrap());
    tx.rollback().await.unwrap();
    db.end().await;
}

#[tokio::test]
async fn first_release_after_return_reattestation() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = item_sale(true, false).await;
    item_effect(&db, Command::Deliver, true).await;
    db.runtime.tick().await.unwrap();
    item_effect(&db, Command::Return, false).await;
    db.runtime.tick().await.unwrap();
    recover(&mut db).await;
    let i = newest_intent(&db, uid("20"), Command::Release).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"hold_id":"{}","hold_revision":"2","released_amount":"10000","remaining_held":"0","hold_state":"RELEASED"}}"#,
        uid("83")
    );
    committed_fact(db.runtime.authority(), &mut tx, &i, &data, now).await;
    tx.commit().await.unwrap();

    db.end().await;
}

#[tokio::test]
async fn gold_hold_accepts_decision_before_platform_issue_time() {
    let db = BrowserFixture::new().await;
    let mut tx = db.pool.begin().await.unwrap();
    let operation = gold_fact(
        &mut tx,
        Command::Hold,
        100,
        uid("82"),
        uid("83"),
        (0, 1, 10000),
    )
    .await;
    let (i,r): (String,String) = sqlx::query_as("SELECT intent,receipt FROM bazaar_game_outbox JOIN bazaar_authority_receipts USING(operation_id) WHERE operation_id=$1")
        .bind(operation).fetch_one(&mut *tx).await.unwrap();
    let intent = PreparedIntent::parse(&i).unwrap();
    let issued = number(&intent.value, "issued_at_source").unwrap();
    let skewed = r.replace("1790000002", &(issued - 1).to_string());
    Receipt::parse(skewed.as_bytes())
        .unwrap()
        .bind(&intent, issued)
        .unwrap();
    tx.rollback().await.unwrap();
    db.end().await;
}

#[test]
fn transfer_rejection_accepts_five_seconds_of_issue_skew() {
    let a = authority();
    let operation = uid("92");
    let issued = 1790000000;
    let transfer = transfer_fixture(operation, 17, issued);
    for skew in [6, 5, 1] {
        let decided = issued - skew;
        let raw = format!(
            r#"{{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"TransferCharacterOwnership","operation_id":"{operation}","outcome":"REJECTED","result_code":"CHAR_CMD_OWNER_MISMATCH","recovery_generation":"3","decided_at":"{decided}"}}"#
        );
        let receipt = decode(raw.as_bytes(), 1024).unwrap();
        assert_eq!(
            runtime::characters::rejected_receipt(&a, &receipt, &transfer, issued).unwrap(),
            skew <= 5
        );
    }
}

#[tokio::test]
async fn first_payout_after_transfer_reattestation() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = delivered_character_sale("gold").await;
    let listing = uid("30");

    recover(&mut db).await;
    reattest_transfers(&db, listing).await;
    admit_gold_payout(&db, listing, 2).await;
    db.end().await;
}

#[tokio::test]
async fn first_gold_bind_after_restore() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = settling_character_fixture("gold").await;
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    let transfer = id(text(&bind.value, "transfer_operation_id").unwrap()).unwrap();
    recover(&mut db).await;
    let mut tx = db.pool.begin().await.unwrap();
    replay(db.runtime.authority(), &mut tx, uid("30")).await;
    let now = runtime::now(&mut tx).await.unwrap();
    let data = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"3","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
        uid("31"),
        uid("32"),
        text(&bind.value, "transfer_binding_digest").unwrap()
    );
    committed_fact(db.runtime.authority(), &mut tx, &bind, &data, now).await;
    tx.commit().await.unwrap();
    db.end().await;
}
