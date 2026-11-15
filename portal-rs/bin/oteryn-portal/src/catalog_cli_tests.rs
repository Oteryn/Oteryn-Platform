use super::*;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
}
impl Database {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("catalog_cli_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = Uuid::now_v7();
        account(&pool, actor, "active").await?;
        let role = Uuid::now_v7();
        sqlx::query("INSERT INTO roles (id,key,built_in) VALUES ($1,'catalog_operator',false)")
            .bind(role)
            .execute(&pool)
            .await?;
        for permission in [
            Permission::ImportStoreCatalog,
            Permission::ActivateStoreCatalog,
        ] {
            sqlx::query("INSERT INTO role_permissions (role_id,permission) VALUES ($1,$2)")
                .bind(role)
                .bind(permission.as_str())
                .execute(&pool)
                .await?;
        }
        sqlx::query("INSERT INTO account_roles (account_id,role_id) VALUES ($1,$2)")
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        Ok(Self {
            pool,
            admin,
            schema,
            actor,
        })
    }
    async fn counts(&self) -> Result<(i64, i64, i64, i64), sqlx::Error> {
        sqlx::query_as("SELECT (SELECT count(*) FROM store_price_list_imports),(SELECT count(*) FROM store_active_catalog WHERE catalog_revision IS NOT NULL),(SELECT count(*) FROM store_catalog_activations),(SELECT count(*) FROM audit_log)").fetch_one(&self.pool).await
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
async fn account(pool: &PgPool, actor: Uuid, status: &str) -> TestResult {
    sqlx::query("INSERT INTO accounts (id,email,password_hash,status,locale) VALUES ($1,$2,'test-only-hash',$3,'en')")
        .bind(actor).bind(format!("{actor}@example.invalid")).bind(status).execute(pool).await?;
    Ok(())
}
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/store-price-list-v1")
}
fn command(action: &str, value: &str, actor: Uuid) -> Result<Command, &'static str> {
    parse_args(&[
        action.to_owned(),
        value.to_owned(),
        "--actor".to_owned(),
        actor.to_string(),
    ])
}
async fn import(db: &Database) -> Result<String, Box<dyn std::error::Error>> {
    let path = fixtures().join("price-list.valid.json");
    Ok(execute(
        &db.pool,
        &command("import", path.to_str().ok_or("fixture path")?, db.actor)?,
    )
    .await?)
}
async fn activate(db: &Database, revision: &str) -> TestResult {
    assert_eq!(
        execute(&db.pool, &command("activate", revision, db.actor)?).await?,
        revision
    );
    Ok(())
}

#[tokio::test]
async fn catalog_cli_imports_activates_and_reads_active() -> TestResult {
    let db = Database::new().await?;
    let revision = import(&db).await?;
    activate(&db, &revision).await?;
    let store = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    assert_eq!(
        store
            .active()
            .await?
            .ok_or("missing active catalog")?
            .revision(),
        revision
    );
    assert_eq!(db.counts().await?, (1, 1, 1, 2));
    let entries: Vec<(String, Uuid, String, serde_json::Value)> =
        sqlx::query_as("SELECT action,actor,target,after FROM audit_log ORDER BY action")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, "store.catalog.activated");
    assert_eq!(
        entries[0].3,
        json!({"catalog_revision":revision,"previous_revision":null})
    );
    assert_eq!(entries[1].0, "store.catalog.imported");
    let offers: i64 = sqlx::query_scalar("SELECT count(*) FROM store_offers")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        entries[1].3,
        json!({"catalog_revision":revision,"offers":offers})
    );
    for (_, actor, target, _) in entries {
        assert_eq!(actor, db.actor);
        assert_eq!(target, revision);
    }
    db.close().await
}

#[tokio::test]
async fn catalog_cli_rejects_invalid_and_changes_nothing() -> TestResult {
    let db = Database::new().await?;
    let before = db.counts().await?;
    let mut tested = 0;
    for entry in std::fs::read_dir(fixtures())? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("price-list.invalid-") && s.ends_with(".json"))
        {
            assert_eq!(
                execute(
                    &db.pool,
                    &command("import", path.to_str().ok_or("fixture path")?, db.actor)?
                )
                .await,
                Err("InvalidDocument")
            );
            tested += 1;
            assert_eq!(db.counts().await?, before);
        }
    }
    assert!(tested > 0);
    db.close().await
}

#[tokio::test]
async fn catalog_cli_refuses_actor_without_permission() -> TestResult {
    let db = Database::new().await?;
    let revision = import(&db).await?;
    let before = db.counts().await?;
    let active = Uuid::now_v7();
    let closed = Uuid::now_v7();
    account(&db.pool, active, "active").await?;
    account(&db.pool, closed, "closed").await?;
    // Give the closed actor the operator role: account state must still deny.
    sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,role_id FROM account_roles WHERE account_id=$2").bind(closed).bind(db.actor).execute(&db.pool).await?;
    for actor in [active, closed, Uuid::now_v7()] {
        for (action, value) in [
            (
                "import",
                fixtures()
                    .join("price-list.valid.json")
                    .to_str()
                    .ok_or("fixture path")?
                    .to_owned(),
            ),
            ("activate", revision.clone()),
        ] {
            assert_eq!(
                execute(&db.pool, &command(action, &value, actor)?).await,
                Err("Denied")
            );
            assert_eq!(db.counts().await?, before);
        }
    }
    db.close().await
}

#[tokio::test]
async fn catalog_cli_is_idempotent() -> TestResult {
    let db = Database::new().await?;
    let revision = import(&db).await?;
    assert_eq!(import(&db).await?, revision);
    assert_eq!(db.counts().await?, (1, 0, 0, 1));
    activate(&db, &revision).await?;
    activate(&db, &revision).await?;
    assert_eq!(db.counts().await?, (1, 1, 1, 2));
    // Concurrent retries use the same immutable source events for audit identity.
    let (left, right) = tokio::join!(import(&db), import(&db));
    assert_eq!(left?, revision);
    assert_eq!(right?, revision);
    assert_eq!(db.counts().await?, (1, 1, 1, 2));
    assert_eq!(
        execute(&db.pool, &command("activate", "invalid", db.actor)?).await,
        Err("InvalidRevision")
    );
    assert_eq!(
        execute(&db.pool, &command("activate", &"0".repeat(64), db.actor)?).await,
        Err("NotImported")
    );
    assert_eq!(db.counts().await?, (1, 1, 1, 2));
    db.close().await
}

#[tokio::test]
async fn catalog_cli_does_not_audit_unchanged_library_history() -> TestResult {
    let db = Database::new().await?;
    let store = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let raw = std::fs::read(fixtures().join("price-list.valid.json"))?;
    let revision = store.import(&raw, db.actor, &Access::default()).await?;
    store
        .activate(&revision, db.actor, &Access::default())
        .await?;
    assert_eq!(db.counts().await?, (1, 1, 1, 0));
    assert_eq!(import(&db).await?, revision);
    activate(&db, &revision).await?;
    assert_eq!(db.counts().await?, (1, 1, 1, 0));
    db.close().await
}

#[tokio::test]
async fn sw3_06_catalog_activation_fences_set_permissions() -> TestResult {
    use oteryn_admin::rbac::{Authorizer, Change, CheckContext};
    let started = std::time::Instant::now();
    let db = Database::new().await?;
    let revision = import(&db).await?;
    let manager = Uuid::now_v7();
    account(&db.pool, manager, "active").await?;
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='catalog_operator'")
        .fetch_one(&db.pool)
        .await?;
    let manager_role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,key,built_in) VALUES($1,'catalog_manager',false)")
        .bind(manager_role)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)")
        .bind(manager_role)
        .bind(Permission::ManageRoles.as_str())
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(manager)
        .bind(manager_role)
        .execute(&db.pool)
        .await?;
    struct ObservedAccess {
        inner: Access,
        checked: tokio::sync::Notify,
    }
    impl CatalogAccess for ObservedAccess {
        async fn account_state(
            &self,
            actor: Uuid,
            db: &mut PgConnection,
        ) -> Result<AccountState, Error> {
            self.inner.account_state(actor, db).await
        }
        async fn permitted(
            &self,
            actor: Uuid,
            action: Action,
            db: &mut PgConnection,
        ) -> Result<bool, Error> {
            let allowed = self.inner.permitted(actor, action, db).await?;
            self.checked.notify_one();
            Ok(allowed)
        }
    }
    let access = std::sync::Arc::new(ObservedAccess {
        inner: Access::default(),
        checked: tokio::sync::Notify::new(),
    });
    let mut pointer = db.pool.begin().await?;
    sqlx::query("SELECT id FROM store_active_catalog WHERE id=1 FOR UPDATE")
        .execute(&mut *pointer)
        .await?;
    let store = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let actor = db.actor;
    let observed = access.clone();
    let activation =
        tokio::spawn(async move { store.activate(&revision, actor, &*observed).await });
    access.checked.notified().await;
    sqlx::query("UPDATE accounts SET email_verified_at=now() WHERE id=$1")
        .bind(manager)
        .execute(&db.pool)
        .await?;
    let auth = Authorizer::new(
        db.pool.clone(),
        oteryn_admin::web::PortalIdentity::configured_from(db.pool.clone(), &|_| None),
    );
    let mut revocation = tokio::spawn(async move {
        auth.change(
            CheckContext {
                account: manager,
                require_staff_mfa: true,
                mfa_verified: true,
            },
            Uuid::now_v7(),
            Change::SetPermissions {
                role,
                permissions: &[],
            },
        )
        .await
    });
    let completed = tokio::time::timeout(std::time::Duration::from_millis(200), &mut revocation)
        .await
        .ok();
    let blocked = completed.is_none();
    pointer.commit().await?;
    activation.await??;
    match completed {
        Some(result) => {
            result??;
        }
        None => {
            revocation.await??;
        }
    }
    assert!(
        blocked,
        "SetPermissions committed before the already-authorized activation"
    );
    db.close().await?;
    eprintln!(
        "PG sw3_06_catalog_activation_fences_set_permissions {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
