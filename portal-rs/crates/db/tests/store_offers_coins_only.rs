//! Migration 20261112100004 (Store price list D1, COINS only) stops on an existing
//! offer in another currency instead of converting or deleting it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oteryn_db::MIGRATOR;
use sqlx::{PgPool, postgres::PgPoolOptions};

const VERSION: i64 = 20261112100004;
const REVISION: &str = "5d531f49ad6cd22e93d37e8cc394c496334ce197ac8a1f505cea1b6891a55294";

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    /// A fresh schema migrated up to the version just before `VERSION`.
    async fn before() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!(
                "SKIPPED store_offers COINS migration test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        // The identifier is a fixed prefix plus server-generated UUID hex, never input.
        let schema: String = sqlx::query_scalar(
            "SELECT 'coins_migration_' || replace(gen_random_uuid()::text, '-', '')",
        )
        .fetch_one(&admin)
        .await
        .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(2)
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
        let previous = MIGRATOR
            .iter()
            .map(|m| m.version)
            .filter(|v| *v < VERSION)
            .max()
            .unwrap();
        MIGRATOR.run_to(previous, &pool).await.unwrap();
        assert!(!applied(&pool, VERSION).await);
        sqlx::query(
            "INSERT INTO store_price_list_imports (catalog_revision, canonical_bytes, imported_by) VALUES ($1, '\\x7b7d', gen_random_uuid())",
        )
        .bind(REVISION)
        .execute(&pool)
        .await
        .unwrap();
        Some(Self {
            admin,
            pool,
            schema,
        })
    }
    async fn offer(&self, key: &str, currency: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO store_offers (catalog_revision, offer_key, currency, unit_price, max_quantity, purchasable, canonical_bytes) VALUES ($1, $2, $3, 25, 10, true, '\\x7b7d')")
            .bind(REVISION)
            .bind(key)
            .bind(currency)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
    async fn offers(&self) -> Vec<(String, String)> {
        sqlx::query_as("SELECT offer_key, currency FROM store_offers ORDER BY offer_key")
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }
    async fn currency_check(&self) -> String {
        sqlx::query_scalar("SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid = 'store_offers'::regclass AND conname = 'store_offers_currency_check'")
            .fetch_one(&self.pool)
            .await
            .unwrap()
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
async fn applied(pool: &PgPool, version: i64) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = $1)")
        .bind(version)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn transferable_offer_stops_the_migration_and_changes_nothing() {
    let Some(db) = Database::before().await else {
        return;
    };
    db.offer("test-supply", "COINS").await.unwrap();
    db.offer("test-transferable", "TRANSFERABLE_COINS")
        .await
        .unwrap();
    let before = db.currency_check().await;
    assert!(before.contains("TRANSFERABLE_COINS"), "{before}");

    // sqlx keeps its session advisory lock when a migration fails; release it on
    // this connection so no other migrator on the test server waits on it.
    let mut connection = db.pool.acquire().await.unwrap();
    let error = MIGRATOR
        .run(&mut *connection)
        .await
        .unwrap_err()
        .to_string();
    sqlx::query("SELECT pg_advisory_unlock_all()")
        .execute(&mut *connection)
        .await
        .unwrap();
    drop(connection);
    assert!(
        error.contains("store_offers holds offers not quoted in COINS"),
        "{error}"
    );
    assert!(!applied(&db.pool, VERSION).await);
    assert_eq!(db.currency_check().await, before);
    assert_eq!(
        db.offers().await,
        vec![
            ("test-supply".to_owned(), "COINS".to_owned()),
            (
                "test-transferable".to_owned(),
                "TRANSFERABLE_COINS".to_owned()
            ),
        ]
    );
    db.finish().await;
}

#[tokio::test]
async fn coins_only_offers_migrate_and_later_transferable_is_refused() {
    let Some(db) = Database::before().await else {
        return;
    };
    db.offer("store:exercise-sword", "COINS").await.unwrap();

    MIGRATOR.run(&db.pool).await.unwrap();
    assert!(applied(&db.pool, VERSION).await);
    assert_eq!(
        db.currency_check().await,
        "CHECK ((currency = 'COINS'::text))"
    );
    assert_eq!(
        db.offers().await,
        vec![("store:exercise-sword".to_owned(), "COINS".to_owned())]
    );
    let refused = db
        .offer("test-transferable", "TRANSFERABLE_COINS")
        .await
        .unwrap_err();
    assert_eq!(
        refused
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("23514")
    );
    db.offer("test-supply", "COINS").await.unwrap();
    db.finish().await;
}
