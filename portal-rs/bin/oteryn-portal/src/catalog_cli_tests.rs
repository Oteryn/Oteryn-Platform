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
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Database::new().await?;
    let store = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let raw = std::fs::read(fixtures().join("price-list.valid.json"))?;
    let revision = store.import(&raw, db.actor, &Access).await?;
    store.activate(&revision, db.actor, &Access).await?;
    assert_eq!(db.counts().await?, (1, 1, 1, 0));
    assert_eq!(import(&db).await?, revision);
    activate(&db, &revision).await?;
    assert_eq!(db.counts().await?, (1, 1, 1, 0));
    db.close().await
}

#[tokio::test]
async fn delta_catalog_permission_read_retains_rbac_fence() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Database::new().await?;
    let mut issuance = db.pool.begin().await?;
    let access = Access;
    assert!(
        access
            .permitted(db.actor, Action::Activate, &mut issuance)
            .await?
    );
    assert!(
        access
            .permitted(db.actor, Action::Import, &mut issuance)
            .await?
    );
    let mut revocation = db.pool.begin().await?;
    let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(71010)")
        .fetch_one(&mut *revocation)
        .await?;
    revocation.rollback().await?;
    issuance.rollback().await?;
    db.close().await?;
    eprintln!(
        "delta_catalog_permission_read_retains_rbac_fence duration={:?}",
        started.elapsed()
    );
    assert!(
        !free,
        "catalog permission read did not retain the role-writer fence"
    );
    Ok(())
}

async fn staff_session_proof(
    pool: &PgPool,
    actor: Uuid,
) -> Result<oteryn_admin::rbac::SessionProof, sqlx::Error> {
    use sha2::{Digest, Sha256};
    let password: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(actor)
        .fetch_one(pool)
        .await?;
    let grant = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let session = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) ON CONFLICT(account_id) DO UPDATE SET confirmed=true RETURNING generation")
        .bind(actor).bind(Sha256::digest(Uuid::now_v7().as_bytes()).to_vec()).fetch_one(pool).await?;
    let grant_hash: [u8; 32] = Sha256::digest(grant.as_bytes()).into();
    let session_hash: [u8; 32] = Sha256::digest(session.as_bytes()).into();
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '1 hour')")
        .bind(grant_hash.as_slice()).bind(actor).bind(generation).bind(Sha256::digest(password.as_bytes()).to_vec()).execute(pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation) SELECT $1,id,now()+interval '1 hour',now()+interval '1 day','staff-fixture',$2,jsonb_build_object('mfa_grant',$3::text),native_security_generation FROM accounts WHERE id=$4")
        .bind(session_hash.as_slice()).bind(format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()))).bind(grant).bind(actor).execute(pool).await?;
    Ok(oteryn_admin::rbac::SessionProof {
        session_hash,
        grant_hash,
    })
}

struct RbacAccounts(PgPool);
impl oteryn_admin::rbac::AccountStateReader for RbacAccounts {
    async fn state(
        &self,
        account: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        let mut connection = self
            .0
            .acquire()
            .await
            .map_err(|_| oteryn_admin::rbac::AccountStateUnavailable)?;
        self.state_in(&mut connection, account).await
    }
    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        use oteryn_admin::rbac::{AccountState, AccountStateUnavailable};
        let active: Option<bool> =
            sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_optional(connection)
                .await
                .map_err(|_| AccountStateUnavailable)?;
        Ok(if active == Some(true) {
            AccountState::Active
        } else {
            AccountState::Disabled
        })
    }
}

#[tokio::test]
async fn delta_catalog_activation_serializes_permission_revocation() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    use oteryn_admin::rbac::{Authorizer, Change, CheckContext};
    let started = std::time::Instant::now();
    let db = Database::new().await?;
    let revision = import(&db).await?;
    let manager = Uuid::now_v7();
    account(&db.pool, manager, "active").await?;
    sqlx::query("UPDATE accounts SET email_verified_at=now() WHERE id=$1")
        .bind(manager)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
        .bind(manager).execute(&db.pool).await?;
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='catalog_operator'")
        .fetch_one(&db.pool)
        .await?;
    let proof = staff_session_proof(&db.pool, manager).await?;
    let mut blocker = db.pool.begin().await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("SELECT id FROM store_active_catalog WHERE id=1 FOR UPDATE")
        .execute(&mut *blocker)
        .await?;
    let pool = db.pool.clone();
    let activation_command = command("activate", &revision, db.actor)?;
    let activation = tokio::spawn(async move { execute(&pool, &activation_command).await });
    let activation_pid = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
            )
            .bind(blocker_pid)
            .fetch_optional(&db.admin)
            .await?;
            if let Some(pid) = pid {
                return Ok::<_, sqlx::Error>(pid);
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    let pool = db.pool.clone();
    let revocation = tokio::spawn(async move {
        let auth = Authorizer::new(pool.clone(), RbacAccounts(pool));
        let mut context = CheckContext::new(manager);
        context.mfa_verified = true;
        context.session_proof = Some(proof);
        auth.change(
            context,
            Uuid::now_v7(),
            Change::SetPermissions {
                role,
                permissions: &[Permission::ImportStoreCatalog],
            },
        )
        .await
    });
    let serialized = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if revocation.is_finished() { return Ok::<_, sqlx::Error>(false); }
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND query='SELECT pg_advisory_xact_lock(71010)')")
                .bind(activation_pid).fetch_one(&db.admin).await?;
            if waiting { return Ok(true); }
            tokio::task::yield_now().await;
        }
    }).await??;
    blocker.rollback().await?;
    activation.await??;
    revocation.await??;
    assert_eq!(
        execute(&db.pool, &command("activate", &revision, db.actor)?).await,
        Err("Denied")
    );
    db.close().await?;
    eprintln!(
        "delta_catalog_activation_serializes_permission_revocation duration={:?}",
        started.elapsed()
    );
    assert!(
        serialized,
        "permission revocation committed ahead of activation waiting on the catalog row"
    );
    Ok(())
}

#[tokio::test]
async fn drc_catalog_audit_budget() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Database::new().await?;
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(281510004)")
        .execute(&mut *blocker)
        .await?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), import(&db)).await;
    assert!(
        result.is_ok(),
        "catalog audit lock exceeded the shared three-second budget"
    );
    assert!(result?.is_err());
    blocker.rollback().await?;
    import(&db).await?;
    db.close().await?;
    Ok(())
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
        inner: Access,
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
    let proof = staff_session_proof(&db.pool, manager).await?;
    let mut revocation = tokio::spawn(async move {
        auth.change(
            CheckContext {
                account: manager,
                require_staff_mfa: true,
                mfa_verified: true,
                session_proof: Some(proof),
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

#[tokio::test]
async fn failed_audit_rolls_back_import() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Database::new().await?;
    sqlx::query("ALTER TABLE audit_log ADD CHECK (action <> 'store.catalog.imported')")
        .execute(&db.pool)
        .await?;
    assert!(import(&db).await.is_err());
    let counts = db.counts().await?;
    let offers: i64 = sqlx::query_scalar("SELECT count(*) FROM store_offers")
        .fetch_one(&db.pool)
        .await?;
    db.close().await?;
    assert_eq!(
        (counts, offers),
        ((0, 0, 0, 0), 0),
        "import survived audit failure"
    );
    Ok(())
}

#[tokio::test]
async fn failed_audit_rolls_back_activation() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = Database::new().await?;
    let revision = import(&db).await?;
    sqlx::query("ALTER TABLE audit_log ADD CHECK (action <> 'store.catalog.activated')")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        execute(&db.pool, &command("activate", &revision, db.actor)?).await,
        Err("Database")
    );
    let counts = db.counts().await?;
    db.close().await?;
    assert_eq!(counts, (1, 0, 0, 1), "activation survived audit failure");
    Ok(())
}
