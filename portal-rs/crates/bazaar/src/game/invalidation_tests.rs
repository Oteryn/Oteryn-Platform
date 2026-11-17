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
