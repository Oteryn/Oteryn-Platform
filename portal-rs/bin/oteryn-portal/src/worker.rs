//! Lease-backed outbox worker and interval cleanup schedules.
use oteryn_db::outbox::{self, Job, PgPool};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration};

pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
pub type DeliveryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DeliveryOutcome, String>> + Send + 'a>>;

pub enum DeliveryOutcome {
    Complete,
    Deferred,
}

/// Domain handlers validate payloads and deduplicate effects using the stable job ID.
/// Delivery is at least once: a process can stop after an effect but before acknowledgement.
pub trait Handler: std::fmt::Debug + Send + Sync {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a>;

    fn deliver<'a>(&'a self, job: &'a Job) -> DeliveryFuture<'a> {
        Box::pin(async move { self.run(job).await.map(|()| DeliveryOutcome::Complete) })
    }
}

pub use oteryn_db::outbox::Cleanup;
#[derive(Debug)]
struct CleanupHandler(Arc<dyn Cleanup>);
impl Handler for CleanupHandler {
    fn run<'a>(&'a self, _job: &'a Job) -> HandlerFuture<'a> {
        self.0.clean()
    }
}
/// Prunes expired native ticket budget windows, one bounded batch per run.
#[derive(Debug)]
struct TicketBudgetCleanup(oteryn_gameauth::tickets::BudgetRetention);
impl Cleanup for TicketBudgetCleanup {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async move { self.0.prune().await.map(|_| ()).map_err(|e| e.to_string()) })
    }
}
/// OTERYN_NATIVE_LOGIN_CHALLENGE_V1 counters and challenges past retention.
#[derive(Debug)]
struct NativeChallengeCleanup(oteryn_gameauth::oauth::challenge::Retention);
impl Cleanup for NativeChallengeCleanup {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async move {
            self.0
                .prune()
                .await
                .map(|_| ())
                .map_err(|e| format!("native login challenge retention failed: {e:?}"))
        })
    }
}
/// Deletes obsolete verification history in bounded batches, retaining the
/// entire resend-rate window even when a token was consumed immediately.
#[derive(Debug)]
struct VerificationTokenCleanup(PgPool);
const TOKEN_CLEANUP_SQL: &str = "WITH obsolete AS (
    SELECT token_tid FROM (
        SELECT ctid AS token_tid, token_hash, updated_at, greatest((created_at AT TIME ZONE 'UTC') + interval '60 seconds',
                  CASE WHEN used_at IS NULL THEN expires_at AT TIME ZONE 'UTC' END) AS eligible_at
        FROM account_email_tokens WHERE purpose = 'verify_email'
    ) tokens
    WHERE eligible_at <= statement_timestamp() AT TIME ZONE 'UTC'
      AND updated_at <= statement_timestamp() - interval '60 seconds'
    ORDER BY eligible_at, token_hash LIMIT 1000 FOR UPDATE SKIP LOCKED
) DELETE FROM account_email_tokens WHERE ctid = ANY (ARRAY(SELECT token_tid FROM obsolete))";

impl Cleanup for VerificationTokenCleanup {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async move {
            sqlx::query(TOKEN_CLEANUP_SQL)
                .execute(&self.0)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    }
}
/// Placeholder until the owning domain supplies its retention implementation.
#[derive(Debug)]
pub struct NoopCleanup;
impl Cleanup for NoopCleanup {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug)]
pub struct Registry {
    disabled: Vec<String>,
    handlers: BTreeMap<String, Arc<dyn Handler>>,
    schedules: BTreeMap<String, i32>,
    execution_budget: Duration,
    heartbeat_interval: Duration,
    shutdown_budget: Duration,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            disabled: Vec::new(),
            handlers: BTreeMap::new(),
            schedules: BTreeMap::new(),
            execution_budget: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(15),
            shutdown_budget: Duration::from_secs(10),
        }
    }
}
impl Registry {
    /// Duplicate registrations are refused instead of silently changing ownership.
    pub fn register(&mut self, kind: &str, handler: Arc<dyn Handler>) -> Result<(), String> {
        if kind.trim().is_empty() || kind.len() > 128 || self.handlers.contains_key(kind) {
            return Err("invalid or duplicate job kind".into());
        }
        self.disabled.retain(|disabled| disabled != kind);
        self.handlers.insert(kind.into(), handler);
        Ok(())
    }
    pub fn cleanup(
        &mut self,
        kind: &str,
        seconds: i32,
        cleanup: Arc<dyn Cleanup>,
    ) -> Result<(), String> {
        if seconds <= 0 {
            return Err("cleanup interval must be positive".into());
        }
        self.register(kind, Arc::new(CleanupHandler(cleanup)))?;
        self.schedules.insert(kind.into(), seconds);
        Ok(())
    }
    async fn schedule(&self, pool: &PgPool) -> Result<(), outbox::Error> {
        for (kind, seconds) in &self.schedules {
            outbox::schedule_with_deadline(pool, kind, *seconds, self.execution_budget).await?;
        }
        Ok(())
    }
    /// Batch jobs start together and renew leases while handlers are in flight.
    pub(crate) async fn batch(&self, pool: &PgPool) -> Result<usize, Box<dyn std::error::Error>> {
        // Preserve connection hooks (including tenant/search_path settings), but
        // reserve independent capacity before handlers can occupy the shared pool.
        let leases = pool
            .options()
            .clone()
            .max_connections(2)
            .min_connections(0)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with((*pool.connect_options()).clone())
            .await?;
        let jobs = outbox::claim_excluding_with_deadline(
            pool,
            16,
            60,
            self.execution_budget,
            &self.disabled,
        )
        .await?;
        let count = jobs.len();
        let mut tasks = tokio::task::JoinSet::new();
        for job in jobs {
            let pool = pool.clone();
            let leases = leases.clone();
            let handler = self.handlers.get(&job.kind).cloned();
            let budget = self.execution_budget;
            let heartbeat_interval = self.heartbeat_interval;
            tasks.spawn(async move {
                let id = job.id;
                // Covers the handler, lease renewals and acknowledgement. Dropping
                // transactional work queues rollback; no acknowledgement is made
                // on timeout, so lease expiry and domain idempotency own retry.
                match tokio::time::timeout(budget, execute(&pool, &leases, job, handler, heartbeat_interval)).await {
                    Ok(result) => result,
                    Err(_) => {
                        tracing::warn!(%id, "outbox execution deadline exceeded; lease left for retry");
                        Ok(())
                    }
                }
            });
        }
        // Drain in-flight work even if another job loses its lease or a database call fails.
        let mut failure = None;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => failure = Some(error.to_string()),
                Err(error) => failure = Some(error.to_string()),
            }
        }
        if let Some(error) = failure {
            return Err(error.into());
        }
        Ok(count)
    }
}
async fn execute(
    pool: &PgPool,
    leases: &PgPool,
    job: Job,
    handler: Option<Arc<dyn Handler>>,
    heartbeat_interval: Duration,
) -> Result<(), outbox::Error> {
    execute_observed(pool, leases, job, handler, heartbeat_interval, || {}).await
}

// The observer marks polling a renewal, before connection acquisition. Tests
// can synchronize pool exhaustion without relying on a timer or a global hook.
async fn execute_observed(
    pool: &PgPool,
    leases: &PgPool,
    job: Job,
    handler: Option<Arc<dyn Handler>>,
    heartbeat_interval: Duration,
    renewal_started: impl Fn(),
) -> Result<(), outbox::Error> {
    let Some(handler) = handler else {
        outbox::finish(
            pool,
            &job,
            Some(&format!("unknown job kind: {}", job.kind)),
            true,
        )
        .await?;
        return Ok(());
    };
    let future = handler.deliver(&job);
    tokio::pin!(future);
    let mut heartbeat = tokio::time::interval(heartbeat_interval);
    let result = loop {
        tokio::select! {
            result = &mut future => break result,
            _ = heartbeat.tick() => {
                let renewal = async {
                    renewal_started();
                    tokio::time::timeout(Duration::from_secs(5), outbox::renew(leases, &job, 60)).await
                };
                tokio::pin!(renewal);
                tokio::select! {
                    result = &mut future => break result,
                    renewed = &mut renewal => {
                        let owned = renewed.map_err(|_| outbox::Error::Protocol(
                            "outbox renewal deadline exceeded; lease left for retry".into()))??;
                        if !owned {
                            return Err(outbox::Error::Protocol("outbox lease lost".into()));
                        }
                    }
                }
            }
        }
    };
    let acknowledged = match result {
        Ok(DeliveryOutcome::Complete) => outbox::finish(pool, &job, None, false).await?,
        Err(error) => outbox::finish(pool, &job, Some(&error), false).await?,
        Ok(DeliveryOutcome::Deferred) => {
            // The handler has finished and no further acknowledgement can use
            // this claim. Release the live lease and return its attempt together.
            sqlx::query(
                "UPDATE outbox SET state='pending', attempts=attempts-1,
                    run_after=clock_timestamp()+interval '1 minute', leased_until=NULL,
                    last_error=NULL, updated_at=clock_timestamp()
                 WHERE id=$1 AND attempts=$2 AND state='running' AND leased_until>clock_timestamp()",
            )
            .bind(job.id)
            .bind(job.attempts)
            .execute(pool)
            .await?
            .rows_affected()
                == 1
        }
    };
    if !acknowledged {
        return Err(outbox::Error::Protocol(
            "outbox lease lost before acknowledgement".into(),
        ));
    }
    Ok(())
}
async fn run_until(
    pool: PgPool,
    registry: Registry,
    shutdown: impl Future<Output = ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => return Ok(()),
            result = registry.schedule(&pool) => result?,
        }
        let batch = registry.batch(&pool);
        tokio::pin!(batch);
        let count = tokio::select! {
            biased;
            () = &mut shutdown => {
                tracing::info!(name: "outbox_worker_drain_started", "outbox worker draining in-flight batch");
                if let Ok(result) = tokio::time::timeout(registry.shutdown_budget, &mut batch).await {
                    result?;
                } else {
                    // Dropping batch drops its JoinSet, aborting every remaining
                    // delivery. Their unacknowledged leases remain retryable.
                    tracing::warn!("outbox shutdown drain deadline exceeded; cancelling batch");
                }
                return Ok(());
            }
            result = &mut batch => result?,
        };
        if count == 0 {
            tokio::select! {
                () = &mut shutdown => return Ok(()),
                () = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
        }
    }
}
fn register_journal_cleanup(registry: &mut Registry, pool: &PgPool) -> Result<(), String> {
    registry.cleanup(
        "cleanup.journal-payloads",
        10,
        Arc::new(oteryn_playerdata::retention::Retention(pool.clone())),
    )
}
fn register_security_notifications(
    registry: &mut Registry,
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(delivery) = crate::sessions_http::delivery(pool)? {
        registry.notifications(delivery)?;
    }
    Ok(())
}
pub(crate) async fn configured(pool: PgPool) -> Result<Registry, Box<dyn std::error::Error>> {
    let mut registry = Registry::default();
    if std::env::var("OTERYN_ADMIN_COMMERCE").as_deref() == Ok("true")
        && std::env::var("OTERYN_REFUNDS_ENABLED").as_deref() == Ok("true")
        && let Some(adapter) = oteryn_admin::commerce::configured_payment_adapter(pool.clone())?
    {
        registry.commerce_refunds(oteryn_admin::commerce::RefundDelivery {
            pool: pool.clone(),
            adapter,
        })?;
    }
    if std::env::var("OTERYN_PAYMENTS_ENABLED").as_deref() == Ok("true")
        && std::env::var("OTERYN_WALLET_ENABLED").as_deref() == Ok("true")
    {
        crate::wallet_topup::register(&mut registry, pool.clone())?;
    }
    if let Some(runtime) = oteryn_bazaar::game::runtime::Runtime::configured(pool.clone()).await? {
        registry.cleanup("bazaar.authority", 1, Arc::new(runtime))?;
    }
    registry.cleanup(
        "league.retention",
        1,
        Arc::new(oteryn_minigames::league::runtime::Retention { pool: pool.clone() }),
    )?;
    if oteryn_minigames::league::config::enabled() {
        registry.cleanup(
            "league.ingest",
            1,
            Arc::new(oteryn_minigames::league::runtime::FeedJob { pool: pool.clone() }),
        )?;
    }
    registry.cleanup(
        "cleanup.world-kills",
        3600,
        Arc::new(oteryn_stats::world::kills::Retention(pool.clone())),
    )?;
    registry.cleanup(
        "community.today.retention",
        3600,
        Arc::new(oteryn_community::today::Retention(pool.clone())),
    )?;
    registry.cleanup(
        "cleanup.game-logins",
        60,
        Arc::new(oteryn_playerdata::logins::Retention(pool.clone())),
    )?;
    register_journal_cleanup(&mut registry, &pool)?;
    crate::reset_delivery::register(&mut registry, pool.clone())?;
    crate::registration_delivery::register(&mut registry, pool.clone())?;
    if std::env::var("PORTAL_REFERRALS_ENABLED").as_deref() == Ok("true") {
        registry.cleanup(
            "referrals.process",
            60,
            Arc::new(oteryn_community::referrals::campaign::Worker::configured(
                pool.clone(),
            )?),
        )?;
    }
    let closure = crate::account_closure::configured(pool.clone())?;
    tracing::debug!(
        game_enabled = closure.game_enabled(),
        "configured account closure worker"
    );
    registry.cleanup(
        "identity.account-closure-audit",
        3600,
        Arc::new(oteryn_identity::termination::game_closure::AuditRetention(
            pool.clone(),
        )),
    )?;
    registry.cleanup(
        "identity.finalize-terminations",
        closure.worker_interval_seconds(),
        Arc::new(closure.terminations),
    )?;
    register_security_notifications(&mut registry, pool.clone())?;
    registry.cleanup(
        "cleanup.deaths",
        3600,
        Arc::new(oteryn_community::deaths::Cleanup { pool: pool.clone() }),
    )?;
    if std::env::var("PORTAL_STATS_ENABLED").as_deref() == Ok("true") {
        registry.cleanup(
            "stats.poll",
            60,
            Arc::new(oteryn_stats::worker::StatisticsJob {
                pool: pool.clone(),
                enabled: true,
            }),
        )?;
    }
    registry.cleanup(
        "cleanup.sessions",
        3600,
        Arc::new(oteryn_web::session::SessionCleanup { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "cleanup.rate-limits",
        60,
        Arc::new(oteryn_web::ratelimit::RateLimitCleanup { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "cleanup.game-ticket-budgets",
        60,
        Arc::new(TicketBudgetCleanup(
            oteryn_gameauth::tickets::BudgetRetention { pool: pool.clone() },
        )),
    )?;
    registry.cleanup(
        "cleanup.native-login-challenge",
        60,
        Arc::new(NativeChallengeCleanup(
            oteryn_gameauth::oauth::challenge::Retention { pool: pool.clone() },
        )),
    )?;
    registry.cleanup(
        "cleanup.passkeys",
        300,
        Arc::new(oteryn_identity::passkeys::PasskeyCleanup { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "cleanup.mfa",
        3600,
        Arc::new(oteryn_identity::mfa::MfaCleanup { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "cleanup.hunting-places",
        3600,
        Arc::new(oteryn_playerdata::places::PlacesRetention { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "cleanup.tokens",
        3600,
        Arc::new(VerificationTokenCleanup(pool.clone())),
    )?;
    registry.cleanup("cleanup.support", 3600, Arc::new(NoopCleanup))?;
    registry.cleanup(
        "cleanup.support-tickets",
        3600,
        Arc::new(oteryn_support::tickets::Retention { pool: pool.clone() }),
    )?;
    if let Some(worker) = oteryn_identity::email_change::Worker::configured(pool.clone())? {
        registry.cleanup("identity.email-change", 60, Arc::new(worker))?;
    }
    if oteryn_admin::control::Config::from_env().enabled {
        registry.cleanup(
            "admin.control.expire",
            10,
            Arc::new(oteryn_admin::control::ExpiryJob(
                oteryn_admin::control::Store::new(
                    pool.clone(),
                    oteryn_admin::control::Config::from_env(),
                ),
            )),
        )?;
    }
    registry.cleanup(
        "stats.v2-retention",
        3600,
        Arc::new(oteryn_stats::online::Retention { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "stats.death-retention",
        3600,
        Arc::new(oteryn_stats::worker::DeathRetention { pool: pool.clone() }),
    )?;
    registry.cleanup(
        "stats.world-sample-retention",
        3600,
        Arc::new(oteryn_stats::worker::WorldSampleRetention { pool: pool.clone() }),
    )?;
    // Known consumers without an enabled handler must keep their queued work.
    // Unknown kinds still reach execute() and are permanently refused.
    for kind in [
        "commerce.refund",
        "wallet.top_up",
        "email.password-reset",
        "email.verify",
        "email.account_exists",
        oteryn_notify::EMAIL_JOB,
        "bazaar.authority",
        "league.ingest",
        "identity.email-change",
        "admin.control.expire",
        "referrals.process",
        "stats.poll",
    ] {
        if !registry.handlers.contains_key(kind) {
            registry.disabled.push(kind.into());
        }
    }
    Ok(registry)
}

pub async fn run(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let registry = configured(pool.clone()).await?;
    #[cfg(unix)]
    let shutdown = {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        async move {
            tokio::select! { _ = terminate.recv() => {}, _ = tokio::signal::ctrl_c() => {}, }
        }
    };
    #[cfg(not(unix))]
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    tracing::info!("outbox worker started");
    run_until(pool, registry, shutdown).await
}
impl Handler for oteryn_notify::Delivery {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            self.notifications
                .deliver(job, self.sender.as_ref())
                .await
                .map_err(|_| "email delivery failed".to_owned())
        })
    }
}
impl Registry {
    /// Identity supplies account state before email delivery can be enabled.
    #[allow(dead_code)]
    pub fn notifications(&mut self, delivery: oteryn_notify::Delivery) -> Result<(), String> {
        self.register(oteryn_notify::EMAIL_JOB, Arc::new(delivery))
    }
}

impl Handler for oteryn_admin::commerce::RefundDelivery {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            self.run(job)
                .await
                .map_err(|_| "refund delivery refused".to_owned())
        })
    }
}
impl Registry {
    /// Commerce supplies verified domain authority before refund dispatch is enabled.
    pub fn commerce_refunds(
        &mut self,
        delivery: oteryn_admin::commerce::RefundDelivery,
    ) -> Result<(), String> {
        self.register("commerce.refund", Arc::new(delivery))
    }
}

#[cfg(test)]
mod tests {
    include!("worker_tests.rs");
    include!("flag_boundary_tests.rs");

    #[tokio::test]
    async fn idle_token_cleanup_avoids_history_scan() -> TestResult {
        let Some(db) = Database::new().await? else {
            return Ok(());
        };
        let account: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES ('plan@example.test','test-only','en') RETURNING id").fetch_one(&db.pool).await?;
        query("INSERT INTO account_email_tokens(token_hash,account_id,purpose,expires_at,created_at)
               SELECT sha256(i::text::bytea),$1,'verify_email',now()+interval '1 day',now()-interval '1 hour'
               FROM generate_series(1,20000) i").bind(account).execute(&db.pool).await?;
        query("ANALYZE account_email_tokens")
            .execute(&db.pool)
            .await?;
        let plan: Vec<String> = query_scalar(outbox::AssertSqlSafe(format!(
            "EXPLAIN ANALYZE {TOKEN_CLEANUP_SQL}"
        )))
        .fetch_all(&db.pool)
        .await?;
        db.close().await?;
        assert!(
            !plan
                .iter()
                .any(|line| line.contains("Seq Scan on account_email_tokens")
                    || line.contains("Rows Removed by Filter: 20000")),
            "idle cleanup scanned token history: {plan:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn completed_schedule_slots_are_pruned() -> TestResult {
        let Some(db) = Database::new().await? else {
            return Ok(());
        };
        outbox::schedule(&db.pool, "league.retention", 1).await?;
        let jobs = outbox::claim(&db.pool, 1, 60).await?;
        assert_eq!(jobs.len(), 1);
        assert!(outbox::finish(&db.pool, &jobs[0], None, false).await?);
        tokio::time::sleep(Duration::from_secs(1)).await;
        outbox::schedule(&db.pool, "league.retention", 1).await?;
        let counts = outbox::counts(&db.pool).await?;
        db.close().await?;
        assert_eq!(counts.pending, 1);
        assert_eq!(counts.done, 0, "obsolete completed schedule slot retained");
        Ok(())
    }
}
