#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use sqlx::postgres::PgPoolOptions;
const VALID: &[u8] =
    include_bytes!("../../../../docs/contracts/fixtures/store-price-list-v1/price-list.valid.json");

#[test]
fn every_fixture_and_laravel_revision() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/store-price-list-v1");
    let mut count = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if name == "price-list.valid.json" || name.starts_with("price-list.invalid-") {
            let result = Catalog::parse(&std::fs::read(entry.path()).unwrap());
            assert_eq!(
                result.is_ok(),
                name == "price-list.valid.json",
                "{name}: {result:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 8);
    let catalog = Catalog::parse(VALID).unwrap();
    assert_eq!(
        catalog.revision(),
        "5d531f49ad6cd22e93d37e8cc394c496334ce197ac8a1f505cea1b6891a55294"
    );
    assert_eq!(catalog.canonical_bytes(), VALID);
}
#[test]
fn canonical_and_domain_boundaries() {
    let text = std::str::from_utf8(VALID).unwrap();
    for bad in [
        format!("{text}\n"),
        text.replace("\"contract_version\":1", "\"contract_version\":1.0"),
        text.replace("\"quantity\":5", "\"quantity\":0"),
        text.replace("test-supply", "Test-supply"),
        text.replace("oteryn.test_supply", "oteryn..supply"),
        text.replace("\"product_version\":\"1\"", "\"product_version\":\"01\""),
        text.replace("\"max_quantity\":10", "\"max_quantity\":101"),
        text.replace(
            "\"contract_version\":1",
            "\"contract_version\":1,\"contract_version\":1",
        ),
        // Owner D1: `COINS` only, and the Game offer_key pattern.
        text.replace("COINS", "TRANSFERABLE_COINS"),
        text.replace("COINS", "GOLD"),
        text.replace("\"offer_key\":\"test-supply\"", "\"offer_key\":\":store\""),
        text.replace("\"offer_key\":\"test-supply\"", "\"offer_key\":\"_store\""),
        text.replace(
            "\"offer_key\":\"test-supply\"",
            "\"offer_key\":\"store/sword\"",
        ),
        text.replace("\"offer_key\":\"test-supply\"", "\"offer_key\":\"\""),
        text.replace(
            "\"offer_key\":\"test-supply\"",
            &format!("\"offer_key\":\"a{}\"", "b".repeat(128)),
        ),
    ] {
        assert!(Catalog::parse(bad.as_bytes()).is_err(), "{bad}");
    }
    for key in [
        "store:exercise-sword".to_owned(),
        "0.a_b:c-d".to_owned(),
        format!("a{}", "b".repeat(127)),
    ] {
        let good = text.replace(
            "\"offer_key\":\"test-supply\"",
            &format!("\"offer_key\":\"{key}\""),
        );
        assert_eq!(
            Catalog::parse(good.as_bytes()).unwrap().offers()[0].offer_key,
            key
        );
    }
    assert!(Catalog::parse(&[]).is_err());
    assert!(Catalog::parse(&vec![b' '; MAX_BYTES + 1]).is_err());
    let premium = text.replace("ITEM", "PREMIUM_TIME");
    assert!(Catalog::parse(premium.as_bytes()).unwrap().offers()[0].available());
    for (from, to) in [("ITEM", "COSMETIC"), ("true", "false")] {
        assert!(
            !Catalog::parse(text.replace(from, to).as_bytes())
                .unwrap()
                .offers()[0]
                .available()
        );
    }
    let max = text
        .replace(
            "\"product_version\":\"1\"",
            "\"product_version\":\"18446744073709551615\"",
        )
        .replace(
            "\"unit_price\":\"25\"",
            "\"unit_price\":\"9223372036854775807\"",
        );
    assert!(Catalog::parse(max.as_bytes()).is_ok());
}

const GAME_RUNTIME: &str = include_str!("../tests/fixtures/game-runtime-catalogue.json");

#[test]
fn runtime_digest_covers_nested_metadata_and_disabled_offers() {
    let catalogue = Catalog::parse(GAME_RUNTIME.as_bytes()).unwrap();
    assert_eq!(catalogue.offers().len(), 24);
    assert!(
        !catalogue
            .offers()
            .iter()
            .any(|o| o.offer_key == "store:prey-list-expansion")
    );
    assert_eq!(
        Catalog::parse(catalogue.canonical_bytes())
            .unwrap()
            .revision(),
        catalogue.revision()
    );

    // Independent D2(b) golden using sorted keys and raw UTF-8, including U+2028.
    let metadata = GAME_RUNTIME.replace(
        r#""source": {"#,
        r#""source": {"z":["Łódź\u2028雪",{"n":18446744073709551615,"a":-9223372036854775808}],"#,
    );
    let with_metadata = Catalog::parse(metadata.as_bytes()).unwrap();
    assert_eq!(
        with_metadata.revision(),
        "f8c279c3c2a6dbed13d67d9e88e8967f656b280528c0a9f13a59fa5acb5129eb"
    );
    assert_eq!(with_metadata.offers().len(), catalogue.offers().len());

    let disabled_price = GAME_RUNTIME.replace(r#""price": null"#, r#""price": 1"#);
    let with_disabled_price = Catalog::parse(disabled_price.as_bytes()).unwrap();
    assert_ne!(with_disabled_price.revision(), catalogue.revision());
    assert_eq!(with_disabled_price.offers().len(), catalogue.offers().len());
}

#[test]
fn runtime_catalogue_rejects_invalid_source_and_fulfilment() {
    for (from, to) in [
        (r#""source": {"#, r#""source": {"nested":[1.0],"#),
        (r#""source": {"#, r#""source": {"nested":{"x":1,"x":2},"#),
        (
            r#""source": {"#,
            r#""catalog_revision":"injected","source": {"#,
        ),
        (r#""price": 25"#, r#""price": 9223372036854775808"#),
        (r#""price": 25"#, r#""price": null"#),
        (
            r#""currency": "TIBIA_COIN""#,
            r#""currency": "TRANSFERABLE_TIBIA_COIN""#,
        ),
        (
            r#""key": "store:exercise-sword""#,
            r#""key": "store:exercise-bow""#,
        ),
        (
            r#""key": "oteryn:item.tibia.i28552""#,
            r#""key": "contains a space""#,
        ),
        (
            r#""revision": "definition-r1""#,
            r#""revision": "definition-r01""#,
        ),
        (
            r#""unlock_key": "charm-expansion""#,
            r#""unlock_key": "unknown-effect""#,
        ),
    ] {
        let input = GAME_RUNTIME.replacen(from, to, 1);
        assert_ne!(input, GAME_RUNTIME, "{from}");
        assert!(
            matches!(
                Catalog::parse(input.as_bytes()),
                Err(Error::InvalidDocument)
            ),
            "{to}"
        );
    }
}

/// Regression (owner D1): a Game offer key keeps `:`; the import used to refuse it,
/// so a purchase quoting it could only answer `OFFER_UNAVAILABLE`.
fn game_key_premium() -> String {
    std::str::from_utf8(VALID)
        .unwrap()
        .replace(
            "\"offer_key\":\"test-supply\"",
            "\"offer_key\":\"store:exercise-sword\"",
        )
        .replace("ITEM", "PREMIUM_TIME")
}
#[test]
fn game_colon_offer_key_is_available() {
    let catalog = Catalog::parse(game_key_premium().as_bytes()).unwrap();
    let offer = &catalog.offers()[0];
    assert_eq!(offer.offer_key, "store:exercise-sword");
    assert!(offer.available());
}
#[test]
fn price_list_schema_matches_store_wire_d1() {
    let schema = |set: &str, name: &str| -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/contracts/fixtures")
            .join(set)
            .join(name);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    let price_list = schema("store-price-list-v1", "price-list.schema.json");
    let wire = schema("store-wire-v1", "purchase-request.schema.json");
    for (offer, request, expected) in [
        (
            "/properties/offers/items/properties/offer_key/pattern",
            "/properties/offer_key/pattern",
            serde_json::json!("^[a-z0-9][a-z0-9_.:-]{0,127}$"),
        ),
        (
            "/properties/offers/items/properties/currency/enum",
            "/properties/currency/enum",
            serde_json::json!(["COINS"]),
        ),
    ] {
        assert_eq!(price_list.pointer(offer), Some(&expected), "{offer}");
        assert_eq!(wire.pointer(request), Some(&expected), "{request}");
    }
}

struct Access {
    state: AccountState,
    allow: bool,
}
impl CatalogAccess for Access {
    async fn account_state(&self, _: Uuid, _: &mut PgConnection) -> Result<AccountState, Error> {
        Ok(self.state)
    }
    async fn permitted(&self, _: Uuid, _: Action, _: &mut PgConnection) -> Result<bool, Error> {
        Ok(self.allow)
    }
}
fn actor() -> Uuid {
    Uuid::now_v7()
}
fn access() -> Access {
    Access {
        state: AccountState::Active,
        allow: true,
    }
}
struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED store catalog DB test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        // Schema identifiers contain only a fixed prefix and locally generated UUID hex.
        let schema = format!("catalog_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(&search)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105160200_t28_catalog.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261112100004_store_offers_coins_only.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        Some(Self {
            admin,
            pool,
            schema,
        })
    }
    fn store(&self) -> CatalogStore {
        CatalogStore::new(self.pool.clone(), CatalogFlags { enabled: true })
    }
    async fn finish(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
#[tokio::test]
async fn db_import_is_idempotent_and_immutable() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let store = db.store();
    let actor = actor();
    let permission = access();
    let (a, b) = tokio::join!(
        store.import(VALID, actor, &permission),
        store.import(VALID, actor, &permission)
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM store_price_list_imports")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let offers: i64 = sqlx::query_scalar("SELECT count(*) FROM store_offers")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(offers, 1);
    assert!(store.active().await.unwrap().is_none());
    assert!(
        sqlx::query("DELETE FROM store_price_list_imports")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE store_offers SET unit_price=1")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.finish().await;
}
#[tokio::test]
async fn db_import_keeps_game_colon_offer_key_available() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let store = db.store();
    let actor = actor();
    let document = game_key_premium();
    let revision = store
        .import(document.as_bytes(), actor, &access())
        .await
        .unwrap();
    store.activate(&revision, actor, &access()).await.unwrap();
    let key: String = sqlx::query_scalar("SELECT offer_key FROM store_offers")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(key, "store:exercise-sword");
    let active = store.active().await.unwrap().unwrap();
    assert_eq!(active.offers()[0].offer_key, "store:exercise-sword");
    assert!(active.offers()[0].available());
    // A file quoted in another currency is refused before anything is written.
    let transferable = document.replace("COINS", "TRANSFERABLE_COINS");
    assert!(matches!(
        store
            .import(transferable.as_bytes(), actor, &access())
            .await,
        Err(Error::InvalidDocument)
    ));
    let imports: i64 = sqlx::query_scalar("SELECT count(*) FROM store_price_list_imports")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(imports, 1);
    db.finish().await;
}
#[tokio::test]
async fn db_refusals_and_bad_input_leave_no_changes() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let store = db.store();
    let actor = actor();
    let disabled = CatalogStore::new(db.pool.clone(), CatalogFlags::default());
    assert!(matches!(
        disabled.import(VALID, actor, &access()).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled.activate(&"a".repeat(64), actor, &access()).await,
        Err(Error::Disabled)
    ));
    for state in [AccountState::Unknown, AccountState::Denied] {
        assert!(matches!(
            store
                .import(VALID, actor, &Access { state, allow: true })
                .await,
            Err(Error::Denied)
        ));
    }
    assert!(matches!(
        store
            .import(
                VALID,
                actor,
                &Access {
                    state: AccountState::Active,
                    allow: false
                }
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        store.import(b"{}", actor, &access()).await,
        Err(Error::InvalidDocument)
    ));
    assert!(matches!(
        store.activate("bad", actor, &access()).await,
        Err(Error::InvalidRevision)
    ));
    assert!(matches!(
        store.activate(&"a".repeat(64), actor, &access()).await,
        Err(Error::NotImported)
    ));
    let revision = store.import(VALID, actor, &access()).await.unwrap();
    assert!(matches!(
        store
            .activate(
                &revision,
                actor,
                &Access {
                    state: AccountState::Unknown,
                    allow: true
                }
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        store
            .activate(
                &revision,
                actor,
                &Access {
                    state: AccountState::Active,
                    allow: false
                }
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(store.active().await.unwrap().is_none());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM store_catalog_activations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.finish().await;
}
#[tokio::test]
async fn db_concurrent_activation_has_serial_history() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let store = db.store();
    let actor = actor();
    let first = store.import(VALID, actor, &access()).await.unwrap();
    let second = store
        .import(
            &std::str::from_utf8(VALID)
                .unwrap()
                .replace("\"25\"", "\"26\"")
                .into_bytes(),
            actor,
            &access(),
        )
        .await
        .unwrap();
    let permission = access();
    let (a, b) = tokio::join!(
        store.activate(&first, actor, &permission),
        store.activate(&second, actor, &permission)
    );
    a.unwrap();
    b.unwrap();
    let rows = sqlx::query("SELECT previous_revision,catalog_revision,actor_id FROM store_catalog_activations ORDER BY activation_sequence").fetch_all(&db.pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<Option<String>, _>("previous_revision"), None);
    assert_eq!(
        rows[1].get::<String, _>("previous_revision"),
        rows[0].get::<String, _>("catalog_revision")
    );
    let active = store.active().await.unwrap().unwrap();
    assert_eq!(
        active.revision(),
        rows[1].get::<String, _>("catalog_revision")
    );
    assert_eq!(rows[1].get::<Uuid, _>("actor_id"), actor);
    store
        .activate(active.revision(), actor, &access())
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM store_catalog_activations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    db.finish().await;
}
#[tokio::test]
async fn db_activation_history_failure_rolls_back_pointer() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let store = db.store();
    let actor = actor();
    let first = store.import(VALID, actor, &access()).await.unwrap();
    store.activate(&first, actor, &access()).await.unwrap();
    let second = store
        .import(
            &std::str::from_utf8(VALID)
                .unwrap()
                .replace("\"25\"", "\"27\"")
                .into_bytes(),
            actor,
            &access(),
        )
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_pointer() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END; $$; CREATE TRIGGER reject_pointer BEFORE UPDATE ON store_active_catalog FOR EACH ROW EXECUTE FUNCTION reject_pointer();").execute(&db.pool).await.unwrap();
    assert!(matches!(
        store.activate(&second, actor, &access()).await,
        Err(Error::Database(_))
    ));
    assert_eq!(store.active().await.unwrap().unwrap().revision(), first);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM store_catalog_activations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.finish().await;
}

#[tokio::test]
async fn lk_catalog_account_before_pointer() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = Database::new()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    sqlx::query("CREATE TABLE accounts(id uuid PRIMARY KEY)")
        .execute(&db.pool)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts VALUES($1)")
        .bind(actor)
        .execute(&db.pool)
        .await
        .unwrap();
    let revision = db.store().import(VALID, actor, &access()).await.unwrap();
    struct LockedAccess(tokio::sync::Notify);
    impl CatalogAccess for LockedAccess {
        async fn account_state(
            &self,
            actor: Uuid,
            db: &mut PgConnection,
        ) -> Result<AccountState, Error> {
            self.0.notify_one();
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
                .bind(actor)
                .execute(db)
                .await?;
            Ok(AccountState::Active)
        }
        async fn permitted(&self, _: Uuid, _: Action, _: &mut PgConnection) -> Result<bool, Error> {
            Ok(true)
        }
    }
    let access = std::sync::Arc::new(LockedAccess(tokio::sync::Notify::new()));
    let mut purchase = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(actor)
        .execute(&mut *purchase)
        .await
        .unwrap();
    let store = db.store();
    let other = access.clone();
    let activation = tokio::spawn(async move { store.activate(&revision, actor, &*other).await });
    access.0.notified().await;
    // Activation has reached its account check. A purchaser already owning that
    // account must still be able to lock the pointer and release the account.
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        sqlx::query("SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR SHARE")
            .execute(&mut *purchase),
    )
    .await
    .expect("catalog/account cycle")
    .unwrap();
    purchase.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), activation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    db.finish().await;
    eprintln!(
        "lk_catalog_account_before_pointer duration: {:?}",
        started.elapsed()
    );
}

/// Takes the actor's account row as the production adapter does, without
/// arming a budget of its own: the catalog transaction must bound the wait.
struct SharedAccountAccess;
impl CatalogAccess for SharedAccountAccess {
    async fn account_state(
        &self,
        actor: Uuid,
        db: &mut PgConnection,
    ) -> Result<AccountState, Error> {
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
            .bind(actor)
            .execute(db)
            .await?;
        Ok(AccountState::Active)
    }
    async fn permitted(&self, _: Uuid, _: Action, _: &mut PgConnection) -> Result<bool, Error> {
        Ok(true)
    }
}

/// Holds the actor's account row `FOR UPDATE` on a separate connection.
async fn held_account(db: &Database, actor: Uuid) -> sqlx::PgConnection {
    use sqlx::Connection;
    sqlx::query("CREATE TABLE accounts(id uuid PRIMARY KEY)")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO accounts VALUES($1)")
        .bind(actor)
        .execute(&db.pool)
        .await
        .unwrap();
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let mut blocker = sqlx::PgConnection::connect(&url).await.unwrap();
    sqlx::query("BEGIN").execute(&mut blocker).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM {schema}.accounts WHERE id=$1 FOR UPDATE"
    )))
    .bind(actor)
    .execute(&mut blocker)
    .await
    .unwrap();
    blocker
}

async fn release(mut blocker: sqlx::PgConnection) {
    use sqlx::Connection;
    sqlx::query("ROLLBACK").execute(&mut blocker).await.unwrap();
    blocker.close().await.unwrap();
}

#[tokio::test]
async fn postgres_import_held_account_lock_sheds_within_mutation_budget() {
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let actor = Uuid::now_v7();
    let blocker = held_account(&db, actor).await;
    let store = db.store();
    let started = Instant::now();
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        store.import(VALID, actor, &SharedAccountAccess),
    )
    .await;
    let elapsed = started.elapsed();
    release(blocker).await;
    db.finish().await;
    let observed = observed.expect("import still waiting at 10 s on a held account lock");
    assert!(matches!(observed, Err(Error::Database(_))), "{observed:?}");
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
}

#[tokio::test]
async fn postgres_activate_held_account_lock_sheds_within_mutation_budget() {
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let actor = Uuid::now_v7();
    let revision = db.store().import(VALID, actor, &access()).await.unwrap();
    let blocker = held_account(&db, actor).await;
    let store = db.store();
    let started = Instant::now();
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        store.activate(&revision, actor, &SharedAccountAccess),
    )
    .await;
    let elapsed = started.elapsed();
    release(blocker).await;
    db.finish().await;
    let observed = observed.expect("activate still waiting at 10 s on a held account lock");
    assert!(matches!(observed, Err(Error::Database(_))), "{observed:?}");
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
}
