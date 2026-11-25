use super::*;
use crate::seed::commerce::SEED_ACTOR;
use std::{path::PathBuf, sync::Arc};

const PLAYER: &str = "019a0000-0000-7000-8000-00000000a001";
const OPERATION: &str = "019a0000-0000-7000-8000-0000000c0001";

fn price_lists() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/store-price-list-v1")
}
async fn player(db: &Database) -> TestResult {
    db.seed(
        "account",
        json!({"email": "player1@example.invalid", "password": PASSWORD, "accountId": PLAYER}),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn store_catalog_imports_and_activates_once_under_the_seed_actor() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let file = price_lists().join("price-list.valid.json");
    let input = json!({"file": file.to_string_lossy()});
    let out = db.twice("store-catalog", input).await?;
    let revision = out["catalogRevision"].as_str().unwrap().to_owned();
    assert_eq!(
        out,
        json!({"entity": "store-catalog", "catalogRevision": revision, "offers": 1, "active": true})
    );
    let active: Option<String> =
        sqlx::query_scalar("SELECT catalog_revision FROM store_active_catalog WHERE id=1")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(active.as_deref(), Some(revision.as_str()));
    let audit: Vec<(String, Uuid)> =
        sqlx::query_as("SELECT action, actor FROM audit_log ORDER BY action")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(
        audit,
        vec![
            ("store.catalog.activated".to_owned(), SEED_ACTOR),
            ("store.catalog.imported".to_owned(), SEED_ACTOR),
        ]
    );
    assert_eq!(
        db.count("SELECT count(*) FROM store_catalog_activations")
            .await?,
        1
    );
    // The seed actor is not an account and gains no RBAC role.
    assert_eq!(db.count("SELECT count(*) FROM accounts").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM account_roles").await?, 0);
    db.close().await
}

#[tokio::test]
async fn store_catalog_refuses_bad_input_and_changes_nothing() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let invalid = price_lists().join("price-list.invalid-duplicate-offer.json");
    let missing = price_lists().join("absent.json");
    for input in [
        json!({}),
        json!({"file": 7}),
        json!({"file": invalid.to_string_lossy()}),
        json!({"file": missing.to_string_lossy()}),
        json!({"file": price_lists().to_string_lossy()}),
        json!({"file": invalid.to_string_lossy(), "actor": "x"}),
    ] {
        assert!(is_bad(db.seed("store-catalog", input).await));
    }
    assert_eq!(
        db.count("SELECT count(*) FROM store_price_list_imports")
            .await?,
        0
    );
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 0);
    db.close().await
}

#[tokio::test]
async fn coin_credit_is_one_audited_ledger_operation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    let input = json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION});
    let out = db.twice("coin-credit", input).await?;
    assert_eq!(
        out,
        json!({"entity": "coin-credit", "accountId": PLAYER, "operationId": OPERATION, "amount": 25})
    );
    let by_email = db
        .seed(
            "coin-credit",
            json!({"email": "Player1@example.invalid", "amount": 5,
                   "operationId": "019a0000-0000-7000-8000-0000000c0002"}),
        )
        .await?;
    assert_eq!(by_email["accountId"], PLAYER);
    let (available, reserved): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(available_delta),0)::bigint, COALESCE(SUM(reserved_delta),0)::bigint FROM wallet_entries",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!((available, reserved), (30, 0));
    // The coin wire's own ledger and policy see the whole credit as transferable.
    let wire = oteryn_wallet::Ledger::new(
        db.pool.clone(),
        oteryn_wallet::WalletFlags { enabled: true },
        oteryn_wallet::wire::DatabasePolicy,
        oteryn_wallet::wire::DatabasePolicy,
        oteryn_wallet::wire::DatabasePolicy,
    );
    let account = Uuid::parse_str(PLAYER)?;
    wire.coin_account(account).await?;
    assert_eq!(wire.balance(account).await?.available, 30);
    let operations: Vec<(Uuid, String, i64, Option<Uuid>)> =
        sqlx::query_as("SELECT id, kind, amount, reference_id FROM wallet_operations ORDER BY id")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(operations.len(), 2);
    assert!(
        operations
            .iter()
            .all(|(_, kind, _, actor)| kind == "staff_grant" && *actor == Some(SEED_ACTOR))
    );
    assert_eq!(
        db.count("SELECT count(*) FROM audit_log WHERE action='seed.wallet.credited' AND actor='019a0000-0000-7000-8000-000000005eed'")
            .await?,
        2
    );
    assert!(is_conflict(
        db.seed(
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 26, "operationId": OPERATION})
        )
        .await
    ));
    db.close().await
}

#[tokio::test]
async fn coin_credit_refuses_bad_input_and_changes_nothing() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    for input in [
        json!({"accountId": PLAYER, "amount": 0, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": -5, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": "25", "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": 2.5, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": 1_000_000_001_i64, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": 25}),
        json!({"accountId": PLAYER, "amount": 25,
               "operationId": "019a0000-0000-4000-8000-0000000c0001"}),
        json!({"amount": 25, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "email": "player1@example.invalid", "amount": 25,
               "operationId": OPERATION}),
        json!({"accountId": "019a0000-0000-7000-8000-00000000a0ff", "amount": 25,
               "operationId": OPERATION}),
        json!({"email": "nobody@example.invalid", "amount": 25, "operationId": OPERATION}),
        json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION, "kind": "x"}),
    ] {
        assert!(is_bad(db.seed("coin-credit", input).await));
    }
    assert_eq!(db.count("SELECT count(*) FROM wallet_operations").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 0);
    db.close().await
}

#[tokio::test]
async fn loyalty_records_consumed_days_the_stream_publishes() -> TestResult {
    use oteryn_store::loyalty::stream::{Stream, http::DatabaseAccounts};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    let out = db
        .twice("loyalty", json!({"accountId": PLAYER, "premiumDays": 30}))
        .await?;
    assert_eq!(out["entity"], "loyalty");
    assert_eq!(out["accountId"], PLAYER);
    assert_eq!(out["premiumDays"], 30);
    let from = time::OffsetDateTime::parse(
        out["effectiveFrom"].as_str().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )?;
    let until = time::OffsetDateTime::parse(
        out["effectiveUntil"].as_str().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )?;
    assert_eq!(until - from, time::Duration::days(30));
    let by_email = db
        .seed(
            "loyalty",
            json!({"email": "player1@example.invalid", "premiumDays": 30}),
        )
        .await?;
    assert_eq!(by_email, out);
    assert!(is_conflict(
        db.seed("loyalty", json!({"accountId": PLAYER, "premiumDays": 31}))
            .await
    ));
    let page = Stream::new(db.pool.clone(), Arc::new(DatabaseAccounts), true)
        .read(None, 500)
        .await?;
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].account_id.to_string(), PLAYER);
    assert_eq!(page.items[0].spent_premium_days, 30);
    assert_eq!(page.items[0].loyalty_points, 30);
    assert_eq!(
        db.count("SELECT count(*) FROM premium_audit WHERE actor_id='019a0000-0000-7000-8000-000000005eed'")
            .await?,
        1
    );
    db.close().await
}

#[tokio::test]
async fn loyalty_refuses_bad_input_and_changes_nothing() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    for input in [
        json!({"accountId": PLAYER, "premiumDays": 0}),
        json!({"accountId": PLAYER, "premiumDays": 367}),
        json!({"accountId": PLAYER, "premiumDays": "7"}),
        json!({"accountId": PLAYER}),
        json!({"premiumDays": 7}),
        json!({"accountId": "019a0000-0000-7000-8000-00000000a0ff", "premiumDays": 7}),
        json!({"accountId": PLAYER, "premiumDays": 7, "reason": "x"}),
    ] {
        assert!(is_bad(db.seed("loyalty", input).await));
    }
    assert_eq!(db.count("SELECT count(*) FROM premium_ledger").await?, 0);
    db.close().await
}
