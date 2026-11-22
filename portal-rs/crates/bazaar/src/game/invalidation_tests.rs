use super::*;
use crate::moderation::{StaffCommand, execute_staff};

async fn approve(db: &BrowserFixture, listing: Uuid) {
    let request = Uuid::now_v7();
    let mut tx = db.pool.begin().await.unwrap();
    for (actor, command) in [
        (
            uid("81"),
            StaffCommand::Suspend {
                listing,
                reason: "Fixture suspension".into(),
            },
        ),
        (
            uid("81"),
            StaffCommand::RequestInvalidation {
                id: request,
                listing,
                reason: "Fixture invalidation".into(),
            },
        ),
        (
            uid("82"),
            StaffCommand::ApproveInvalidation {
                id: request,
                reason: "Documented moderation decision".into(),
            },
        ),
    ] {
        execute_staff(&mut tx, actor, &command).await.unwrap();
    }
    tx.commit().await.unwrap();
}

async fn count(db: &BrowserFixture, listing: Uuid, command: Command) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox WHERE listing_id=$1 AND command=$2")
        .bind(listing)
        .bind(command.wire())
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

async fn item_scenario(gold: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let listing = uid("20");
    let hold = Uuid::now_v7();
    if gold {
        let mut tx = db.pool.begin().await.unwrap();
        gold_fact(
            &mut tx,
            Command::Hold,
            10,
            Uuid::now_v7(),
            hold,
            (0, 1, 12000),
        )
        .await;
        sqlx::query("UPDATE bazaar_listings SET currency='gold',leader=$2,leader_max=12000,reservation=id WHERE id=$1").bind(listing).bind(uid("13")).execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE bazaar_game_issuer SET source_revision=10")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=(SELECT max(source_revision) FROM bazaar_game_outbox)").execute(&db.pool).await.unwrap();
    approve(&db, listing).await;
    if gold {
        db.runtime.tick().await.unwrap();
        db.runtime.tick().await.unwrap();
        assert_eq!(count(&db, listing, Command::Release).await, 1);
        let release = newest_intent(&db, listing, Command::Release).await;
        assert_eq!(text(&release.value, "release_reason").unwrap(), "CANCELLED");
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let data = format!(
            r#"{{"hold_id":"{hold}","hold_revision":"2","released_amount":"12000","remaining_held":"0","hold_state":"RELEASED"}}"#
        );
        committed_fact(db.runtime.authority(), &mut tx, &release, &data, now).await;
        tx.commit().await.unwrap();
    }
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(
        count(&db, listing, Command::Return).await,
        1,
        "escrowed item needs one return"
    );
    let state: crate::State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        state,
        crate::State::Active,
        "custody return is still pending"
    );
    let returning = newest_intent(&db, listing, Command::Return).await;
    let op = text(&returning.value, "operation_id").unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = vector("delivery_receipt")
        .replace("DeliverBazaarItem", "ReturnBazaarItem")
        .replace(&uid("4").to_string(), op)
        .replace(&uid("13").to_string(), &uid("11").to_string())
        .replace(&uid("14").to_string(), &uid("21").to_string())
        .replace("1790000002", &now.to_string());
    let a = db.runtime.authority();
    a.receive(
        &mut tx,
        &a.config.receipt_authority,
        receipt.as_bytes(),
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        audit(a, &mut tx, &returning, &receipt, now).await,
        ReceiptResult::Stored
    );
    let status = vector("delivered_status")
        .replace("DELIVERED", "RETURNED")
        .replace(&uid("4").to_string(), op)
        .replace("1790000002", &now.to_string());
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
    tx.commit().await.unwrap();
    for _ in 0..2 {
        db.runtime.tick().await.unwrap();
        let state: crate::State =
            sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
                .bind(listing)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(state, crate::State::Cancelled);
        assert_eq!(count(&db, listing, Command::Return).await, 1);
        if gold {
            assert_eq!(count(&db, listing, Command::Release).await, 1);
            let mut tx = db.pool.begin().await.unwrap();
            assert_eq!(
                runtime::gold::current_hold(a, &mut tx, listing, uid("13"), None)
                    .await
                    .unwrap()
                    .unwrap()
                    .amount,
                0
            );
        }
    }
    db.end().await;
}

#[tokio::test]
async fn invalidation_returns_escrowed_item_once() {
    item_scenario(false).await;
}

#[tokio::test]
async fn invalidation_releases_funded_gold_once() {
    item_scenario(true).await;
}

#[tokio::test]
async fn invalidation_unlocks_never_escrowed_character_once() {
    character_invalidation(false).await;
}
#[tokio::test]
async fn invalidation_returns_escrowed_character_once() {
    character_invalidation(true).await;
}

#[tokio::test]
async fn invalidation_unlocks_character_after_rejected_escrow_bind() {
    invalidation_after_refused_escrow_bind("CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
}

#[tokio::test]
async fn invalidation_unlocks_character_after_expired_escrow_bind() {
    invalidation_after_refused_escrow_bind("CHAR_CMD_INTENT_EXPIRED").await;
}

async fn invalidation_after_refused_escrow_bind(code: &str) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(false).await;
    let listing = uid("30");
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, listing, Command::Bind).await;
    approve(&db, listing).await;
    reject(&db, &bind, code).await;
    assert_invalidation_unlock(&db, 1).await;
    db.end().await;
}

async fn assert_invalidation_unlock(db: &BrowserFixture, lock_revision: u64) {
    let listing = uid("30");
    for _ in 0..2 {
        db.runtime.tick().await.unwrap();
        assert_eq!(listing_state(db).await, crate::State::Draft);
        assert_eq!(
            count(db, listing, Command::Unlock).await,
            1,
            "approved invalidation must stage an unlock when escrow cannot commit"
        );
        assert_eq!(count(db, listing, Command::Bind).await, 1);
    }
    let unlock = newest_intent(db, listing, Command::Unlock).await;
    assert_eq!(
        number(&unlock.value, "expected_lock_revision").unwrap(),
        lock_revision
    );
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let facts = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"{}","current_account_id":"{}"}}"#,
        uid("31"),
        uid("32"),
        lock_revision + 1,
        uid("11")
    );
    committed_fact(db.runtime.authority(), &mut tx, &unlock, &facts, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(listing_state(db).await, crate::State::Cancelled);
    assert_eq!(count(db, listing, Command::Unlock).await, 1);
}

async fn locked_character_fixture(escrowed: bool) -> BrowserFixture {
    let mut db = BrowserFixture::new().await;
    let config = std::sync::Arc::make_mut(&mut db.runtime.config);
    config.authority.config.escrow_account_id = Some(uid("40"));
    config.escrow_account = Some(uid("40"));
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let listing = uid("30");
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'character','auction','coins',100,false,'draft',now()-interval '1 second')").bind(listing).bind(uid("11")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$2,$3,$3,7,$4,1,true,'character-browser')").bind(listing).bind(uid("12")).bind(uid("31")).bind([1_u8;32].as_slice()).execute(&mut *tx).await.unwrap();
    let strval = |value: Uuid| Value::String(value.to_string());
    let lock = character_intent(
        Command::Lock,
        101,
        now,
        vec![
            ("character_id", strval(uid("31"))),
            ("expected_character_revision", Value::String("7".into())),
            ("eligibility_revision", Value::String("4".into())),
            ("snapshot_schema_version", Value::Unsigned(1)),
        ],
    );
    let data = format!(
        "{{\"character_id\":\"{}\",\"lock_id\":\"{}\",\"lock_revision\":\"1\",\"eligibility_revision\":\"4\",\"signed_snapshot\":{}}}",
        uid("31"),
        uid("32"),
        vector("locked_character_snapshot")
    );
    committed_fact(a, &mut tx, &lock, &data, now).await;

    let operation = id(text(&lock.value, "operation_id").unwrap()).unwrap();
    super::super::authority::register_writer(&mut tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,3,3)").bind(operation).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE bazaar_game_issuer SET source_revision=101")
        .execute(&mut *tx)
        .await
        .unwrap();
    character_status(&db, &mut tx, "locked", operation, ("1", "1"), now).await;
    if escrowed {
        escrow_character(&db, &mut tx, listing, now).await;
        sqlx::query("UPDATE bazaar_game_issuer SET source_revision=103")
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    db
}

async fn character_invalidation(escrowed: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(escrowed).await;
    let listing = uid("30");
    let a = db.runtime.authority();
    approve(&db, listing).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    if escrowed {
        assert_eq!(count(&db, listing, Command::Unlock).await, 0);
        let returning = newest_intent(&db, listing, Command::Bind).await;
        assert_eq!(
            text(&returning.value, "purpose").unwrap(),
            "BAZAAR_ESCROW_RETURN"
        );
        let transfer = id(text(&returning.value, "transfer_operation_id").unwrap()).unwrap();
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        let facts = format!(
            r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"3","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
            uid("31"),
            uid("32"),
            text(&returning.value, "transfer_binding_digest").unwrap()
        );
        committed_fact(a, &mut tx, &returning, &facts, now).await;
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
        let mut tx = db.pool.begin().await.unwrap();
        let now = runtime::now(&mut tx).await.unwrap();
        character_transfer_receipt(&mut tx, transfer, (uid("40"), uid("11")), now).await;
        character_status(&db, &mut tx, "RETURNED", transfer, ("3", "3"), now).await;
        tx.commit().await.unwrap();
        db.runtime.tick().await.unwrap();
    }
    assert_eq!(listing_state(&db).await, crate::State::Draft);
    assert_eq!(
        count(&db, listing, Command::Unlock).await,
        1,
        "seller-owned character needs one unlock"
    );
    assert_eq!(
        count(&db, listing, Command::Bind).await,
        if escrowed { 2 } else { 0 }
    );
    let unlock = newest_intent(&db, listing, Command::Unlock).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let facts = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"{}","current_account_id":"{}"}}"#,
        uid("31"),
        uid("32"),
        if escrowed { 4 } else { 2 },
        uid("11")
    );
    committed_fact(a, &mut tx, &unlock, &facts, now).await;
    tx.commit().await.unwrap();
    for _ in 0..2 {
        db.runtime.tick().await.unwrap();
        let returns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_ESCROW_RETURN'",
        ).bind(listing).fetch_one(&db.pool).await.unwrap();
        assert_eq!(returns, i64::from(escrowed));
        assert_eq!(listing_state(&db).await, crate::State::Cancelled);
        assert_eq!(count(&db, listing, Command::Unlock).await, 1);
        assert_eq!(
            count(&db, listing, Command::Bind).await,
            if escrowed { 2 } else { 0 }
        );
    }
    db.end().await;
}

#[tokio::test]
async fn r10_refused_escrow_bind_unlocks() {
    refused_escrow_bind_unlocks(false).await;
}

#[tokio::test]
async fn refused_cancelled_escrow_bind_rejection_unlocks() {
    refused_escrow_bind_unlocks(true).await;
}

async fn refused_escrow_bind_unlocks(cancelled: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(false).await;
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    reject(&db, &bind, "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
    if cancelled {
        mark_refused_character(&db, true).await;
    }
    db.runtime.tick().await.unwrap();
    assert_eq!(
        count(&db, uid("30"), Command::Unlock).await,
        1,
        "refused escrow bind must unlock"
    );
    let unlock = newest_intent(&db, uid("30"), Command::Unlock).await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let facts = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"2","current_account_id":"{}"}}"#,
        uid("31"),
        uid("32"),
        uid("11")
    );
    committed_fact(db.runtime.authority(), &mut tx, &unlock, &facts, now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(listing_state(&db).await, crate::State::Cancelled);
    assert_eq!(count(&db, uid("30"), Command::Unlock).await, 1);
    db.end().await;
}

#[tokio::test]
async fn suspended_expired_item_stays_in_escrow() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let listing = uid("20");
    sqlx::query("UPDATE bazaar_listings SET ends_at=now()-interval '1 second' WHERE id=$1")
        .bind(listing)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,1,$2,'fixture moderation hold')")
        .bind(listing).bind(uid("11")).execute(&db.pool).await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(count(&db, listing, Command::Return).await, 0);
    db.end().await;
}

#[tokio::test]
async fn cancelled_deposit_returns_without_sale_policy() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::deposited_item(false).await;
    sqlx::query("UPDATE bazaar_authority_assets SET cancel_requested=true")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM bazaar_game_projections WHERE kind='POLICY'")
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(count(&db, uid("20"), Command::Return).await, 1);
    db.end().await;
}

#[tokio::test]
async fn invalidation_publishes_committed_escrow_bind() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(false).await;
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    let transfer = commit_escrow_bind(&db, &bind).await;
    approve(&db, uid("30")).await;
    db.runtime.tick().await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM character_commands WHERE id=$1)"
        )
        .bind(transfer)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    );
    db.end().await;
}

#[tokio::test]
async fn invalidation_waits_for_then_unlocks_character_after_rejected_escrow_transfer() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(false).await;
    let listing = uid("30");
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, listing, Command::Bind).await;
    let transfer = commit_escrow_bind(&db, &bind).await;
    approve(&db, listing).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    assert_eq!(count(&db, listing, Command::Unlock).await, 0);
    assert_eq!(listing_state(&db).await, crate::State::Draft);

    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let receipt = format!(
        r#"{{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"TransferCharacterOwnership","operation_id":"{transfer}","outcome":"REJECTED","result_code":"CHAR_CMD_CHARACTER_IN_SESSION","recovery_generation":"3","decided_at":"{now}"}}"#
    );
    let updated =
        sqlx::query("UPDATE character_commands SET state='TERMINAL',receipt_json=$2 WHERE id=$1 AND state='PENDING'")
            .bind(transfer)
            .bind(receipt)
            .execute(&mut *tx)
            .await
            .unwrap();
    assert_eq!(updated.rows_affected(), 1);
    let bind_operation = id(text(&bind.value, "operation_id").unwrap()).unwrap();
    character_status(&db, &mut tx, "locked", bind_operation, ("2", "2"), now).await;
    tx.commit().await.unwrap();
    assert_invalidation_unlock(&db, 2).await;
    db.end().await;
}

async fn commit_escrow_bind(db: &BrowserFixture, bind: &PreparedIntent) -> Uuid {
    let transfer = id(text(&bind.value, "transfer_operation_id").unwrap()).unwrap();
    let facts = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"2","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
        uid("31"),
        uid("32"),
        text(&bind.value, "transfer_binding_digest").unwrap()
    );
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    committed_fact(db.runtime.authority(), &mut tx, bind, &facts, now).await;
    tx.commit().await.unwrap();
    transfer
}

#[tokio::test]
async fn stale_return_bind_uses_corrected_revision() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(true).await;
    sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
        .bind(uid("30"))
        .execute(&db.pool)
        .await
        .unwrap();
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_eq!(
        text(&bind.value, "purpose").unwrap(),
        "BAZAAR_ESCROW_RETURN"
    );
    reject(&db, &bind, "CHAR_CMD_BAZAAR_STALE_REVISION").await;
    let mut tx = db.pool.begin().await.unwrap();
    let now = runtime::now(&mut tx).await.unwrap();
    let escrow:Uuid=sqlx::query_scalar("SELECT operation_id FROM bazaar_current_transfers WHERE listing_id=$1 AND purpose='BAZAAR_LISTING_ESCROW'").bind(uid("30")).fetch_one(&mut *tx).await.unwrap();
    character_status(&db, &mut tx, "locked", escrow, ("3", "3"), now).await;
    tx.commit().await.unwrap();
    db.runtime.tick().await.unwrap();
    let new = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_ne!(
        bind.value.get("operation_id"),
        new.value.get("operation_id")
    );
    assert_eq!(text(&new.value, "expected_lock_revision").unwrap(), "3");
    db.end().await;
}

#[tokio::test]
async fn admitted_lock_survives_key_retirement() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = delivered_character_sale("coins").await;
    let keys = &mut std::sync::Arc::make_mut(&mut db.runtime.config)
        .authority
        .config
        .snapshot_keys;
    let (kid, key) = keys.pop().unwrap();
    keys.push((
        "replacement".into(),
        ed25519_dalek::SigningKey::from_bytes(&[42; 32]).verifying_key(),
    ));
    sqlx::query(
        "INSERT INTO bazaar_authority_snapshot_keys(kid,public_key,revoked) VALUES($1,$2,true)",
    )
    .bind(kid)
    .bind(key.as_bytes().as_slice())
    .execute(&db.pool)
    .await
    .unwrap();
    assert_eq!(count(&db, uid("30"), Command::Unlock).await, 0);
    db.runtime.tick().await.unwrap();
    assert_eq!(count(&db, uid("30"), Command::Unlock).await, 1);
    db.end().await;
}

#[tokio::test]
async fn refused_item_deposit_closes_draft() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
    let listing = uid("70");
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) SELECT $1,owner,kind,form,currency,price,accept_offers,ends_at FROM bazaar_listings WHERE id=$2").bind(listing).bind(uid("20")).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) SELECT $1,$1,world_id,seller_character_id,$2,expected_revision,session_hash,quantity,publication_consent,'refused' FROM bazaar_authority_assets WHERE listing_id=$3").bind(listing).bind(uid("71")).bind(uid("20")).execute(&db.pool).await.unwrap();
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
    assert_eq!(
        sqlx::query_scalar::<_, crate::State>("SELECT state FROM bazaar_listings WHERE id=$1")
            .bind(listing)
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        crate::State::Cancelled
    );
    db.end().await;
}

#[tokio::test]
async fn coin_bid_rejects_foreign_destination() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = BrowserFixture::new().await;
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
    assert!(matches!(
        db.runtime
            .bid(account, uid("20"), 10000, uid("21"), Uuid::now_v7())
            .await,
        Err(Error::Denied)
    ));
    db.end().await;
}

#[tokio::test]
async fn refused_draft_with_redecided_unpublished_bind_reports_conflict() {
    redecided_unpublished_bind(false).await;
}

#[tokio::test]
async fn refused_cancelled_with_redecided_unpublished_bind_reports_conflict() {
    redecided_unpublished_bind(true).await;
}

async fn redecided_unpublished_bind(cancelled: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let mut db = locked_character_fixture(false).await;
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    let refusal = "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE";
    reject(&db, &bind, refusal).await;
    mark_refused_character(&db, cancelled).await;
    super::lifecycle_table_tests::set_resync_fence(&mut db, 3, 4);
    let a = db.runtime.authority();
    let mut tx = db.pool.begin().await.unwrap();
    recovery_tests::replay(a, &mut tx, uid("20")).await;
    recovery_tests::replay_except(a, &mut tx, uid("30"), Some(Command::Bind)).await;
    let now = runtime::now(&mut tx).await.unwrap();
    let transfer = text(&bind.value, "transfer_operation_id").unwrap();
    let facts = format!(
        r#"{{"character_id":"{}","lock_id":"{}","lock_revision":"2","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
        uid("31"),
        uid("32"),
        text(&bind.value, "transfer_binding_digest").unwrap()
    );
    committed_fact(a, &mut tx, &bind, &facts, now).await;
    tx.commit().await.unwrap();

    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let (reports, published): (i64, bool) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM bazaar_audit WHERE row_id=$1
                 AND after_row->>'action'='refused_unpublished_bind_conflict'
                 AND after_row->>'transfer_operation_id'=$2),
                EXISTS(SELECT 1 FROM character_commands WHERE id=$2::uuid)",
    )
    .bind(uid("30"))
    .bind(transfer)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!((reports, published), (1, false));
    assert_eq!(count(&db, uid("30"), Command::Unlock).await, 0);
    assert_eq!(
        listing_state(&db).await,
        if cancelled {
            crate::State::Cancelled
        } else {
            crate::State::Draft
        }
    );
    db.end().await;
}

async fn mark_refused_character(db: &BrowserFixture, cancelled: bool) {
    let refusal = "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE";
    sqlx::query("UPDATE bazaar_authority_assets SET refusal_code=$2 WHERE listing_id=$1")
        .bind(uid("30"))
        .bind(refusal)
        .execute(&db.pool)
        .await
        .unwrap();
    if cancelled {
        sqlx::query("UPDATE bazaar_listings SET state='cancelled' WHERE id=$1")
            .bind(uid("30"))
            .execute(&db.pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn refused_draft_with_committed_cc1_stages_return() {
    refused_with_committed_cc1(false).await;
}

#[tokio::test]
async fn refused_cancelled_with_committed_cc1_stages_return() {
    refused_with_committed_cc1(true).await;
}

async fn refused_with_committed_cc1(cancelled: bool) {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(true).await;
    mark_refused_character(&db, cancelled).await;
    db.runtime.tick().await.unwrap();
    db.runtime.tick().await.unwrap();
    let returning = newest_intent(&db, uid("30"), Command::Bind).await;
    assert_eq!(
        text(&returning.value, "purpose").unwrap(),
        "BAZAAR_ESCROW_RETURN"
    );
    assert_eq!(count(&db, uid("30"), Command::Unlock).await, 0);
    let returns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_all_transfers WHERE listing_id=$1 AND purpose='BAZAAR_ESCROW_RETURN'",
    ).bind(uid("30")).fetch_one(&db.pool).await.unwrap();
    assert_eq!(returns, 1);
    db.end().await;
}
