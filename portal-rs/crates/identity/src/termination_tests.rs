use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(super) struct Database {
    pub(super) pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    pub(super) async fn new() -> TestResult<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED termination PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("termination_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    pub(super) async fn account(&self) -> TestResult<AccountId> {
        let hash = password::hash("termination test password")?;
        Ok(sqlx::query_scalar("INSERT INTO accounts (email,password_hash,status,locale,email_verified_at,public_account_association,public_status_visible) VALUES ($1,$2,'active','en',now(),true,true) RETURNING id")
            .bind(format!("{}@example.test",Uuid::now_v7())).bind(hash).fetch_one(&self.pool).await?)
    }
    pub(super) async fn due(&self, id: Uuid) -> TestResult {
        sqlx::query("UPDATE account_terminations SET created_at=now()-interval '15 days',scheduled_for=now()-interval '1 day' WHERE id=$1").bind(id).execute(&self.pool).await?;
        Ok(())
    }
    pub(super) async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
/// Dependency adapter fixture that proves actor binding and transactional rollback.
#[derive(Debug, Default)]
struct BoundaryFixture {
    blocked: AtomicBool,
    fail_revoke: AtomicBool,
}
impl Boundary for BoundaryFixture {
    fn authorize<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        proof: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            if proof.0 == account.to_string() {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        })
    }
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            if self.blocked.load(Ordering::SeqCst)
                || !game_closure::runtime::financial_obligations_clear(tx, account).await?
            {
                Err(Error::UnknownState)
            } else {
                Ok(())
            }
        })
    }
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        _: bool,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                .bind(account)
                .execute(&mut **tx)
                .await?;
            if self.fail_revoke.load(Ordering::SeqCst) {
                return Err(Error::UnknownState);
            }
            Ok(())
        })
    }
}
fn service(db: &Database, boundary: Arc<dyn Boundary>) -> Result<Terminations> {
    Terminations::new(
        db.pool.clone(),
        Config {
            enabled: true,
            ..Config::default()
        },
        boundary,
    )
}
async fn request(service: &Terminations, account: AccountId) -> Result<Termination> {
    service
        .request(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
            "TERMINATE",
        )
        .await
}
async fn events(db: &Database) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM account_termination_events")
            .fetch_one(&db.pool)
            .await?,
    )
}
#[test]
fn configuration_defaults_and_bounds() {
    assert!(!Config::default().enabled);
    assert_eq!(Config::default().grace_days, 14);
    for days in [1, 14, 90] {
        assert!(
            Config {
                enabled: true,
                grace_days: days
            }
            .validate()
            .is_ok()
        );
    }
    for days in [-1, 0, 91, i32::MAX] {
        assert!(matches!(
            Config {
                enabled: true,
                grace_days: days
            }
            .validate(),
            Err(Error::InvalidInput)
        ));
    }
    assert!(!format!("{:?}", ActorProof("sensitive".into())).contains("sensitive"));
}
#[tokio::test]
async fn postgres_request_cancel_reschedule_and_finalize() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let service = service(&db, Arc::new(BoundaryFixture::default()))?;
    let first = request(&service, account).await?;
    let days:i64 = sqlx::query_scalar("SELECT extract(epoch FROM (scheduled_for-created_at))::bigint/86400 FROM account_terminations WHERE id=$1").bind(first.id).fetch_one(&db.pool).await?;
    assert_eq!(days, 14);
    assert_eq!(request(&service, account).await?.id, first.id);
    assert!(!service.finalize(account).await?);
    assert!(matches!(
        service
            .cancel(account, &ActorProof(account.to_string()), "wrong")
            .await,
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        service
            .cancel(account, &ActorProof(account.to_string()), "")
            .await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        service
            .cancel(
                account,
                &ActorProof(Uuid::now_v7().to_string()),
                "termination test password"
            )
            .await,
        Err(Error::Unauthorized)
    ));
    db.due(first.id).await?;
    service
        .cancel(
            account,
            &ActorProof(account.to_string()),
            "termination test password",
        )
        .await?;
    assert_eq!(service.run_worker(100).await?, WorkerResult::default());
    assert!(matches!(
        service
            .cancel(
                account,
                &ActorProof(account.to_string()),
                "termination test password"
            )
            .await,
        Err(Error::NoPending)
    ));
    let second = request(&service, account).await?;
    assert_ne!(first.id, second.id);
    db.due(second.id).await?;
    assert_eq!(service.run_worker(100).await?.finalized, 1);
    assert!(!service.finalize(account).await?);
    assert!(matches!(
        service
            .cancel(
                account,
                &ActorProof(account.to_string()),
                "termination test password"
            )
            .await,
        Err(Error::Unavailable)
    ));
    let closed = crate::accounts::Accounts {
        pool: db.pool.clone(),
    }
    .find_by_id(account)
    .await?
    .ok_or("missing account")?;
    assert_eq!(closed.status, "closed");
    assert_eq!(closed.email, format!("terminated+{account}"));
    assert!(!password::verify(
        "termination test password",
        &closed.password_hash
    ));
    assert!(closed.email_verified_at.is_none());
    assert!(!closed.public_account_association && !closed.public_status_visible);
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(generation, 4);
    assert_eq!(events(&db).await?, 4);
    db.close().await
}
#[tokio::test]
async fn postgres_bad_input_and_actor_refusals() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let service = service(&db, Arc::new(BoundaryFixture::default()))?;
    let proof = ActorProof(account.to_string());
    for (pass, confirmation) in [
        ("", "TERMINATE"),
        ("termination test password", "terminate"),
    ] {
        assert!(matches!(
            service.request(account, &proof, pass, confirmation).await,
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(
        service
            .request(account, &proof, &"x".repeat(1025), "TERMINATE")
            .await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        service.request(account, &proof, "wrong", "TERMINATE").await,
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        service
            .request(
                account,
                &ActorProof(Uuid::now_v7().to_string()),
                "termination test password",
                "TERMINATE"
            )
            .await,
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        request(&service, Uuid::now_v7()).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        request(&service, account).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        service.run_worker(0).await,
        Err(Error::InvalidInput)
    ));
    assert_eq!(events(&db).await?, 0);
    db.close().await
}
#[tokio::test]
async fn postgres_unknown_dependencies_and_disabled_flag_deny() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let unavailable = service(&db, Arc::new(UnavailableBoundary))?;
    assert!(matches!(
        request(&unavailable, account).await,
        Err(Error::UnknownState)
    ));
    let disabled = Terminations::new(
        db.pool.clone(),
        Config::default(),
        Arc::new(BoundaryFixture::default()),
    )?;
    assert!(matches!(
        request(&disabled, account).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled
            .cancel(
                account,
                &ActorProof(account.to_string()),
                "termination test password"
            )
            .await,
        Err(Error::Disabled)
    ));
    let ready = service(&db, Arc::new(BoundaryFixture::default()))?;
    let termination = request(&ready, account).await?;
    db.due(termination.id).await?;
    assert_eq!(disabled.run_worker(100).await?, WorkerResult::default());
    assert_eq!(
        unavailable.run_worker(100).await?,
        WorkerResult {
            finalized: 0,
            blocked: 1
        }
    );
    assert_eq!(events(&db).await?, 1);
    db.close().await
}
#[tokio::test]
async fn postgres_blocker_rechecked_at_finalization() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let boundary = Arc::new(BoundaryFixture::default());
    let service = service(&db, boundary.clone())?;
    boundary.blocked.store(true, Ordering::SeqCst);
    assert!(matches!(
        request(&service, account).await,
        Err(Error::UnknownState)
    ));
    boundary.blocked.store(false, Ordering::SeqCst);
    let termination = request(&service, account).await?;
    db.due(termination.id).await?;
    boundary.blocked.store(true, Ordering::SeqCst);
    assert_eq!(service.run_worker(100).await?.blocked, 1);
    assert_eq!(events(&db).await?, 1);
    boundary.blocked.store(false, Ordering::SeqCst);
    assert_eq!(service.run_worker(100).await?.finalized, 1);
    db.close().await
}
#[tokio::test]
async fn postgres_revocation_failure_rolls_back_lifecycle() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let boundary = Arc::new(BoundaryFixture::default());
    let service = service(&db, boundary.clone())?;
    sqlx::query("INSERT INTO web_sessions (id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES ($1,$2,now()+interval '1 hour',now()+interval '1 day','test',$3)")
        .bind(vec![7u8;32]).bind(account).bind("a".repeat(64)).execute(&db.pool).await?;
    boundary.fail_revoke.store(true, Ordering::SeqCst);
    assert!(matches!(
        request(&service, account).await,
        Err(Error::UnknownState)
    ));
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(sessions, 1);
    assert_eq!(events(&db).await?, 0);
    boundary.fail_revoke.store(false, Ordering::SeqCst);
    let termination = request(&service, account).await?;
    db.due(termination.id).await?;
    boundary.fail_revoke.store(true, Ordering::SeqCst);
    assert_eq!(service.run_worker(100).await?.blocked, 1);
    let status: String = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1")
        .bind(account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(status, "active");
    assert_eq!(events(&db).await?, 1);
    db.close().await
}
#[tokio::test]
async fn postgres_concurrent_requests_and_workers_are_idempotent() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let service = service(&db, Arc::new(BoundaryFixture::default()))?;
    let (first, second) = tokio::join!(request(&service, account), request(&service, account));
    let first = first?;
    assert_eq!(first.id, second?.id);
    db.due(first.id).await?;
    let (first, second) = tokio::join!(service.finalize(account), service.finalize(account));
    assert_eq!(u8::from(first?) + u8::from(second?), 1);
    assert_eq!(events(&db).await?, 2);
    db.close().await
}

#[tokio::test]
async fn postgres_cancel_and_worker_race_has_one_terminal_state() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let service = service(&db, Arc::new(BoundaryFixture::default()))?;
    let termination = request(&service, account).await?;
    db.due(termination.id).await?;
    let proof = ActorProof(account.to_string());
    let (cancel, finalize) = tokio::join!(
        service.cancel(account, &proof, "termination test password"),
        service.finalize(account)
    );
    match (cancel, finalize?) {
        (Ok(()), false) | (Err(Error::Unavailable), true) => {}
        _ => return Err("cancellation and finalization both won or both lost".into()),
    }
    let (cancelled, finalized): (bool, bool) = sqlx::query_as("SELECT cancelled_at IS NOT NULL,finalized_at IS NOT NULL FROM account_terminations WHERE id=$1")
        .bind(termination.id).fetch_one(&db.pool).await?;
    assert_ne!(cancelled, finalized);
    assert_eq!(events(&db).await?, 2);
    db.close().await
}

#[tokio::test]
async fn postgres_grace_period_boundaries() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for days in [1, 90] {
        let account = db.account().await?;
        let service = Terminations::new(
            db.pool.clone(),
            Config {
                enabled: true,
                grace_days: days,
            },
            Arc::new(BoundaryFixture::default()),
        )?;
        let termination = request(&service, account).await?;
        let seconds:i64 = sqlx::query_scalar("SELECT extract(epoch FROM (scheduled_for-created_at))::bigint FROM account_terminations WHERE id=$1")
            .bind(termination.id).fetch_one(&db.pool).await?;
        assert_eq!(seconds, i64::from(days) * 86400);
    }
    assert!(
        Terminations::new(
            db.pool.clone(),
            Config {
                enabled: true,
                grace_days: 0
            },
            Arc::new(BoundaryFixture::default())
        )
        .is_err()
    );
    db.close().await
}

#[tokio::test]
async fn postgres_held_account_lock_sheds_request_and_cancel_within_mutation_budget() -> TestResult
{
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let service = service(&db, Arc::new(BoundaryFixture::default()))?;
    let pending = request(&service, account).await?;
    let mut holder = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut *holder)
        .await?;
    let proof = ActorProof(account.to_string());
    let started = std::time::Instant::now();
    let (requested, cancelled) = tokio::join!(
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            request(&service, account),
        ),
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            service.cancel(account, &proof, "termination test password"),
        ),
    );
    let requested = requested.map_err(|_| "termination request waited past the mutation budget")?;
    let cancelled = cancelled.map_err(|_| "termination cancel waited past the mutation budget")?;
    assert!(started.elapsed() < std::time::Duration::from_secs(6));
    assert!(requested.is_err());
    assert!(cancelled.is_err());
    holder.rollback().await?;
    let cancelled_at: bool =
        sqlx::query_scalar("SELECT cancelled_at IS NOT NULL FROM account_terminations WHERE id=$1")
            .bind(pending.id)
            .fetch_one(&db.pool)
            .await?;
    assert!(!cancelled_at);
    assert_eq!(events(&db).await?, 1);
    db.close().await
}

#[tokio::test]
async fn r7_legacy_tombstone_collision() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let account = db.account().await?;
    db.due(request(&s, account).await?.id).await?;
    sqlx::query("INSERT INTO accounts(email,password_hash,locale) VALUES($1,'pending','en')")
        .bind(format!("terminated+{account}@invalid.oteryn"))
        .execute(&db.pool)
        .await?;
    assert!(
        matches!(s.run_worker(1).await, Ok(WorkerResult { finalized: 1, .. })),
        "legacy address blocked finalization"
    );
    db.close().await
}

#[tokio::test]
async fn r7_closed_backup_available() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let account = db.account().await?;
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'backup@example.test',now())").bind(account).execute(&db.pool).await?;
    db.due(request(&s, account).await?.id).await?;
    assert!(s.finalize(account).await?);
    assert!(
        crate::accounts::address_available(
            &mut *db.pool.acquire().await?,
            None,
            "backup@example.test"
        )
        .await?,
        "closed backup still reserved"
    );
    db.close().await
}

#[tokio::test]
async fn r7_worker_rotates_failed_row() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let blocked = db.account().await?;
    db.due(request(&s, blocked).await?.id).await?;
    let healthy = db.account().await?;
    db.due(request(&s, healthy).await?.id).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "ALTER TABLE accounts ADD CHECK (id <> '{blocked}' OR status <> 'closed')"
    )))
    .execute(&db.pool)
    .await?;
    assert!(
        matches!(s.run_worker(1).await, Ok(WorkerResult { blocked: 1, .. })),
        "row error aborted sweep"
    );
    assert_eq!(s.run_worker(1).await?.finalized, 1);
    db.close().await
}

#[tokio::test]
async fn reclaimed_backup_receives_password_reset_for_new_owner() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let old = db.account().await?;
    let email = "reclaimed@example.test";
    sqlx::query(
        "INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,$2,now())",
    )
    .bind(old)
    .bind(email)
    .execute(&db.pool)
    .await?;
    db.due(request(&s, old).await?.id).await?;
    assert!(s.finalize(old).await?);
    let mut tx = db.pool.begin().await?;
    let new = crate::accounts::Accounts::create(&mut tx, email, "unusable", "en")
        .await?
        .ok_or("registration refused")?;
    tx.commit().await?;
    let key = crate::reset::DeliveryKey::parse(&"11".repeat(32)).ok_or("invalid test key")?;
    crate::reset::request(&db.pool, email, &key).await?;
    let delivery: Option<(AccountId,String)> = sqlx::query_as("SELECT r.account_id,r.recovery_email FROM outbox o JOIN account_password_resets r ON r.account_id=(o.payload->>'account_id')::uuid WHERE o.kind='email.password-reset'")
        .fetch_optional(&db.pool).await?;
    assert_eq!(
        delivery,
        Some((new.id, email.to_owned())),
        "reset delivery missing for new owner"
    );
    db.close().await
}

#[tokio::test]
async fn finalization_closes_pending_support_tickets() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let account = db.account().await?;
    sqlx::query("INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,uuidv7(),'technical','Pending question')").bind(account).execute(&db.pool).await?;
    db.due(request(&s, account).await?.id).await?;
    assert!(s.finalize(account).await?);
    let closed: bool = sqlx::query_scalar(
        "SELECT status='closed' AND closed_at IS NOT NULL FROM tickets WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    assert!(closed);
    db.close().await
}

#[tokio::test]
async fn last_admin_cannot_terminate() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let account = db.account().await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(account).execute(&db.pool).await?;
    assert!(matches!(
        request(&s, account).await,
        Err(Error::UnknownState)
    ));
    let other = db.account().await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
        .bind(other).execute(&db.pool).await?;
    db.due(request(&s, account).await?.id).await?;
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(other)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        s.finalize(account).await,
        Err(Error::UnknownState)
    ));
    db.close().await
}

#[tokio::test]
async fn unpaid_browser_credit_blocks_termination() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = service(&db, Arc::new(BoundaryFixture::default()))?;
    let account = db.account().await?;
    let intent = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($2,$1,'test','checkout',100,'PLN','succeeded')").bind(account).bind(intent).execute(&db.pool).await?;
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,intended_payment_intent_id) VALUES(uuidv7(),$1,'test',100,100,'PLN',$2)").bind(account).bind(intent).execute(&db.pool).await?;
    assert!(matches!(
        request(&s, account).await,
        Err(Error::UnknownState)
    ));
    db.close().await
}
