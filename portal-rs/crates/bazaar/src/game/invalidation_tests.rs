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
        (uid("82"), StaffCommand::ApproveInvalidation { id: request }),
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
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = locked_character_fixture(false).await;
    db.runtime.tick().await.unwrap();
    let bind = newest_intent(&db, uid("30"), Command::Bind).await;
    reject(&db, &bind, "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE").await;
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
