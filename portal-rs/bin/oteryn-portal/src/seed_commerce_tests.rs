use super::*;
use crate::seed::commerce::SEED_ACTOR;
use std::{path::PathBuf, sync::Arc};

/// Revision of `crates/store/tests/fixtures/game-runtime-catalogue.json`, the
/// same golden the store's purchase tests quote (`purchase_tests.rs`).
const GAME_RUNTIME_REVISION: &str =
    "451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c";

const PLAYER: &str = "019a0000-0000-7000-8000-00000000a001";
const OPERATION: &str = "019a0000-0000-7000-8000-0000000c0001";

fn game_runtime_catalogue() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/store/tests/fixtures/game-runtime-catalogue.json")
}
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
    // The catalogue RBAC and audit fences are database-wide advisory locks.
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let file = price_lists().join("price-list.valid.json");
    let input = json!({"file": file.to_string_lossy()});
    let out = db.twice("store-catalog", input).await?;
    // The S-5 revision is SHA-256 of the file's canonical bytes, computed here.
    let revision = format!("{:x}", Sha256::digest(std::fs::read(&file)?));
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
    // A Game runtime catalogue activates under the revision Game quotes.
    let runtime = json!({"file": game_runtime_catalogue().to_string_lossy()});
    let out = db.twice("store-catalog", runtime).await?;
    assert_eq!(
        out,
        json!({"entity": "store-catalog", "catalogRevision": GAME_RUNTIME_REVISION,
               "offers": 24, "active": true})
    );
    let active: Option<String> =
        sqlx::query_scalar("SELECT catalog_revision FROM store_active_catalog WHERE id=1")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(active.as_deref(), Some(GAME_RUNTIME_REVISION));
    // The rerun of the runtime catalogue added no second activation or audit row.
    let activations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM store_catalog_activations WHERE catalog_revision=$1",
    )
    .bind(GAME_RUNTIME_REVISION)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(activations, 1);
    let audit: Vec<(String, Uuid)> =
        sqlx::query_as("SELECT action, actor FROM audit_log WHERE target=$1 ORDER BY action")
            .bind(GAME_RUNTIME_REVISION)
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(
        audit,
        vec![
            ("store.catalog.activated".to_owned(), SEED_ACTOR),
            ("store.catalog.imported".to_owned(), SEED_ACTOR),
        ]
    );
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 4);
    db.close().await
}

#[tokio::test]
async fn store_catalog_refuses_bad_input_and_changes_nothing() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let audit: Vec<(String, Uuid, String, Value, Value, Uuid)> = sqlx::query_as(
        "SELECT action, actor, target, before, after, request_id FROM audit_log ORDER BY request_id",
    )
    .fetch_all(&db.pool)
    .await?;
    let second = Uuid::parse_str("019a0000-0000-7000-8000-0000000c0002")?;
    let credited = |before: i64, amount: i64, request: Uuid| {
        (
            "seed.wallet.credited".to_owned(),
            SEED_ACTOR,
            PLAYER.to_owned(),
            json!({"available": before, "reserved": 0}),
            json!({"available": before + amount, "reserved": 0, "amount": amount}),
            request,
        )
    };
    assert_eq!(
        audit,
        vec![
            credited(0, 25, Uuid::parse_str(OPERATION)?),
            credited(25, 5, second)
        ]
    );
    assert!(is_conflict(
        db.seed(
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 26, "operationId": OPERATION})
        )
        .await
    ));
    // The conflict changed nothing.
    assert_eq!(wire.balance(account).await?.available, 30);
    assert_eq!(db.count("SELECT count(*) FROM wallet_operations").await?, 2);
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 2);
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
    // The e-mail bound runs before the lookup, not only as a lookup miss.
    for email in [
        format!("{}@example.invalid", "a".repeat(250)),
        " player1@example.invalid".to_owned(),
    ] {
        assert_eq!(
            db.seed(
                "coin-credit",
                json!({"email": email, "amount": 25, "operationId": OPERATION}),
            )
            .await,
            Err(SeedError::BadInput(
                "email must be 1..=254 characters, trimmed".into()
            ))
        );
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

/// Settings that carry the testing authority the commerce seeds require.
fn testing_settings() -> Settings {
    Settings {
        testing_seed: Some(oteryn_store::premium::repository::TestingSeed::assume_testing()),
        ..Settings::default()
    }
}

#[tokio::test]
async fn commerce_seeds_refuse_without_testing_authority() -> TestResult {
    // Every refusal happens before the database is touched.
    let pool = PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
    for (entity, input) in [
        (
            "store-catalog",
            json!({"file": "/nonexistent/catalogue.json"}),
        ),
        (
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION}),
        ),
        ("loyalty", json!({"accountId": PLAYER, "premiumDays": 7})),
    ] {
        assert_eq!(
            execute(&pool, &Settings::default(), entity, &input).await,
            Err(SeedError::Unavailable(
                "seed runs only with OTERYN_ENV=testing"
            )),
            "{entity} ran without testing authority"
        );
    }
    Ok(())
}

// rustix has no `mkfifoat` on Apple targets.
#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn store_catalog_refuses_a_fifo_without_blocking() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let fifo = scratch.path().join("catalogue.fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::from_raw_mode(0o600),
    )?;
    // A FIFO swapped in after the path check: the open itself returns at once
    // and the descriptor is not a regular file.
    let (opened, done) = std::sync::mpsc::channel();
    let path = fifo.clone();
    std::thread::spawn(move || {
        let _ = opened.send(
            crate::seed::commerce::open_nonblocking(&path)
                .and_then(|file| file.metadata())
                .map(|meta| meta.is_file()),
        );
    });
    let is_file = done
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("opening a FIFO still blocked after 10 s")?;
    assert!(!is_file);
    let input = json!({"file": fifo.to_string_lossy()});
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(async {
            let pool = PgPoolOptions::new()
                .connect_lazy("postgresql://localhost/unused")
                .unwrap();
            execute(&pool, &testing_settings(), "store-catalog", &input).await
        });
        let _ = send.send(result);
    });
    let result = receive
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("store-catalog still blocked on a FIFO after 10 s");
    assert!(is_bad(result));
    Ok(())
}

#[tokio::test]
async fn coin_credit_reports_an_eligibility_read_failure_as_unavailable() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    // The eligibility read fails as a database error, not as an ineligible account.
    sqlx::query(
        "ALTER FUNCTION account_value_eligible(uuid) RENAME TO account_value_eligible_missing",
    )
    .execute(&db.pool)
    .await?;
    let result = db
        .seed(
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION}),
        )
        .await;
    assert_eq!(result, Err(SeedError::Unavailable("account eligibility")));
    assert_eq!(db.count("SELECT count(*) FROM wallet_operations").await?, 0);
    db.close().await
}

#[tokio::test]
async fn coin_credit_refuses_an_ineligible_account() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    player(&db).await?;
    sqlx::query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
        .bind(Uuid::parse_str(PLAYER)?)
        .execute(&db.pool)
        .await?;
    let result = db
        .seed(
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION}),
        )
        .await;
    assert_eq!(
        result,
        Err(SeedError::BadInput(
            "account is not eligible for value".into()
        ))
    );
    assert_eq!(db.count("SELECT count(*) FROM wallet_operations").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 0);
    db.close().await
}

#[tokio::test]
async fn the_seed_actor_is_never_an_account() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let actor = SEED_ACTOR.to_string();
    assert!(is_bad(
        db.seed(
            "account",
            json!({"email": "actor@example.invalid", "password": PASSWORD, "accountId": actor}),
        )
        .await
    ));
    assert_eq!(db.count("SELECT count(*) FROM accounts").await?, 0);
    // An account row that already holds the actor id stops every commerce seed.
    player(&db).await?;
    let hash = oteryn_identity::password::hash(PASSWORD)?;
    sqlx::query("INSERT INTO accounts (id, email, email_verified_at, password_hash, status, locale, native_security_generation) VALUES ($1, 'actor@example.invalid', now(), $2, 'active', 'en', 1)")
        .bind(SEED_ACTOR)
        .bind(hash)
        .execute(&db.pool)
        .await?;
    // Resolving that row by e-mail, with its own password, is refused too.
    assert_eq!(
        db.seed(
            "account",
            json!({"email": "actor@example.invalid", "password": PASSWORD}),
        )
        .await,
        Err(SeedError::Conflict(
            "email belongs to the commerce seed actor id".into()
        ))
    );
    let file = price_lists().join("price-list.valid.json");
    for (entity, input) in [
        ("store-catalog", json!({"file": file.to_string_lossy()})),
        (
            "coin-credit",
            json!({"accountId": PLAYER, "amount": 25, "operationId": OPERATION}),
        ),
        ("loyalty", json!({"accountId": PLAYER, "premiumDays": 7})),
    ] {
        let result = db.seed(entity, input).await;
        assert!(is_conflict(result.clone()), "{entity}: {result:?}");
    }
    assert_eq!(
        db.count("SELECT count(*) FROM store_price_list_imports")
            .await?,
        0
    );
    assert_eq!(db.count("SELECT count(*) FROM wallet_operations").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM premium_ledger").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM audit_log").await?, 0);
    db.close().await
}
