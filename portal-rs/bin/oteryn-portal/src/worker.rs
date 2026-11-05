//! Lease-backed outbox worker and interval cleanup schedules.
use oteryn_db::outbox::{self, Job, PgPool};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration};

pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Domain handlers validate payloads and deduplicate effects using the stable job ID.
/// Delivery is at least once: a process can stop after an effect but before acknowledgement.
pub trait Handler: std::fmt::Debug + Send + Sync {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a>;
}

pub use oteryn_db::outbox::Cleanup;
#[derive(Debug)]
struct CleanupHandler(Arc<dyn Cleanup>);
impl Handler for CleanupHandler {
    fn run<'a>(&'a self, _job: &'a Job) -> HandlerFuture<'a> {
        self.0.clean()
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

#[derive(Debug, Default)]
pub struct Registry {
    handlers: BTreeMap<String, Arc<dyn Handler>>,
    schedules: BTreeMap<String, i32>,
}
impl Registry {
    /// Duplicate registrations are refused instead of silently changing ownership.
    pub fn register(&mut self, kind: &str, handler: Arc<dyn Handler>) -> Result<(), String> {
        if kind.trim().is_empty() || kind.len() > 128 || self.handlers.contains_key(kind) {
            return Err("invalid or duplicate job kind".into());
        }
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
            outbox::schedule(pool, kind, *seconds).await?;
        }
        Ok(())
    }
    /// Batch jobs start together and renew leases while handlers are in flight.
    async fn batch(&self, pool: &PgPool) -> Result<usize, Box<dyn std::error::Error>> {
        let jobs = outbox::claim(pool, 16, 60).await?;
        let count = jobs.len();
        let mut tasks = tokio::task::JoinSet::new();
        for job in jobs {
            let pool = pool.clone();
            let handler = self.handlers.get(&job.kind).cloned();
            tasks.spawn(async move { execute(&pool, job, handler).await });
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
    job: Job,
    handler: Option<Arc<dyn Handler>>,
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
    let future = handler.run(&job);
    tokio::pin!(future);
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    let result = loop {
        tokio::select! {
            result = &mut future => break result,
            _ = heartbeat.tick() => {
                if !outbox::renew(pool, &job, 60).await? {
                    return Err(outbox::Error::Protocol("outbox lease lost".into()));
                }
            }
        }
    };
    if !outbox::finish(pool, &job, result.as_ref().err().map(String::as_str), false).await? {
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
            () = &mut shutdown => { batch.await?; return Ok(()); }
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
pub async fn run(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::default();
    registry.cleanup(
        "cleanup.game-logins",
        3600,
        Arc::new(oteryn_playerdata::logins::Retention(pool.clone())),
    )?;
    crate::reset_delivery::register(&mut registry, pool.clone())?;
    registry.cleanup(
        "identity.finalize-terminations",
        60,
        Arc::new(oteryn_identity::termination::Terminations::new(
            pool.clone(),
            oteryn_identity::termination::Config::from_env()?,
            Arc::new(oteryn_identity::termination::UnavailableBoundary),
        )?),
    )?;
    if let Some(delivery) = crate::sessions_http::delivery(pool.clone())? {
        registry.notifications(delivery)?;
    }
    registry.cleanup(
        "stats.poll",
        60,
        Arc::new(oteryn_stats::worker::StatisticsJob {
            pool: pool.clone(),
            enabled: std::env::var("PORTAL_STATS_ENABLED").as_deref() == Ok("true"),
        }),
    )?;
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
        "cleanup.passkeys",
        300,
        Arc::new(oteryn_identity::passkeys::PasskeyCleanup { pool: pool.clone() }),
    )?;
    for kind in ["cleanup.tokens", "cleanup.support"] {
        registry.cleanup(kind, 3600, Arc::new(NoopCleanup))?;
    }
    registry.cleanup(
        "cleanup.support-tickets",
        3600,
        Arc::new(oteryn_support::tickets::Retention {
            pool: pool.clone(),
            enabled: std::env::var("OTERYN_SUPPORT_TICKETS").as_deref() == Ok("true"),
        }),
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
#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;

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
    #[allow(dead_code)]
    pub fn commerce_refunds(
        &mut self,
        delivery: oteryn_admin::commerce::RefundDelivery,
    ) -> Result<(), String> {
        self.register("commerce.refund", Arc::new(delivery))
    }
}
