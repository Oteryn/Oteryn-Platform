//! Bounded transient-source recovery. Every attempt retains the exact configured
//! source fences; malformed/integrity failures never turn into a retry bypass.
use super::{CatalogStore, Error, Result, Source};
use sqlx::PgPool;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    initial_seconds: u64,
    max_seconds: u64,
    max_attempts: u32,
}
impl RetryPolicy {
    /// Intervals are seconds, attempts include the first immediate attempt.
    pub fn new(initial_seconds: u64, max_seconds: u64, max_attempts: u32) -> Result<Self> {
        if !(1..=300).contains(&initial_seconds)
            || !(initial_seconds..=3600).contains(&max_seconds)
            || !(1..=64).contains(&max_attempts)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            initial_seconds,
            max_seconds,
            max_attempts,
        })
    }
    pub(super) fn configured() -> Result<Self> {
        let read = |name, default| match std::env::var(name) {
            Ok(value) => value.parse::<u64>().map_err(|_| Error::InvalidInput),
            Err(std::env::VarError::NotPresent) => Ok(default),
            Err(_) => Err(Error::InvalidInput),
        };
        Self::new(
            read("OTERYN_WORLD_CATALOG_RETRY_INITIAL_SECONDS", 5)?,
            read("OTERYN_WORLD_CATALOG_RETRY_MAX_SECONDS", 300)?,
            u32::try_from(read("OTERYN_WORLD_CATALOG_RETRY_MAX_ATTEMPTS", 12)?)
                .map_err(|_| Error::InvalidInput)?,
        )
    }
    fn delay(self, retry: u32, entropy: u64) -> Duration {
        let multiplier = 1u64.checked_shl(retry).unwrap_or(u64::MAX);
        let cap_ms = self
            .initial_seconds
            .saturating_mul(multiplier)
            .min(self.max_seconds)
            * 1000;
        let floor_ms = cap_ms * 3 / 4;
        Duration::from_millis(floor_ms + entropy % (cap_ms - floor_ms + 1))
    }
}
pub(super) async fn read_source(db: &PgPool, source: &Source, publish: bool) -> Result<String> {
    // A persisted publication is already verified source evidence. All six fences
    // and the independently configured byte SHA must match; no producer I/O here.
    if publish
        && CatalogStore::new(db.clone(), Some(source.clone()))
            .active_version_id()
            .await?
            .is_some()
    {
        return Ok(source.fence.snapshot_id.clone());
    }
    let candidate = source.fetch().await?;
    candidate.persist_inactive(db).await?;
    if publish {
        candidate.publish(db).await?;
    }
    Ok(candidate.snapshot_id().into())
}
pub(super) async fn run(
    db: &PgPool,
    source: &Source,
    publish: bool,
    policy: RetryPolicy,
    ready: &AtomicBool,
) -> Result<String> {
    for attempt in 0..policy.max_attempts {
        let result =
            tokio::time::timeout(Duration::from_secs(60), read_source(db, source, publish))
                .await
                .unwrap_or(Err(Error::Unavailable));
        match result {
            Ok(snapshot) => {
                ready.store(true, Ordering::Release);
                return Ok(snapshot);
            }
            Err(Error::Unavailable) if attempt + 1 < policy.max_attempts => {
                // UUIDv7's random low bits provide independent process jitter.
                let entropy = Uuid::now_v7().as_u128() as u64;
                tokio::time::sleep(policy.delay(attempt, entropy)).await;
            }
            Err(error) => return Err(error),
        }
    }
    Err(Error::Unavailable)
}
/// Trusted service entry point for an explicitly pinned source/policy. The serve
/// wrapper obtains those values from configuration; neither path waits at startup.
pub fn refresh_source_in_background(
    db: PgPool,
    source: Source,
    publish: bool,
    policy: RetryPolicy,
    ready: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<Result<Option<String>>> {
    ready.store(false, Ordering::Release);
    tokio::spawn(async move { run(&db, &source, publish, policy, &ready).await.map(Some) })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn retry_bounds_exponential_growth_and_capped_jitter() {
        for (initial, cap, attempts) in [
            (0, 300, 12),
            (301, 301, 12),
            (5, 4, 12),
            (5, 3601, 12),
            (5, 300, 0),
            (5, 300, 65),
        ] {
            assert!(RetryPolicy::new(initial, cap, attempts).is_err());
        }
        let policy = RetryPolicy::new(5, 300, 12).expect("valid test policy");
        for retry in 0..64 {
            let cap = 5u64
                .saturating_mul(1u64.checked_shl(retry).unwrap_or(u64::MAX))
                .min(300)
                * 1000;
            for entropy in [0, 1, 123456789, u64::MAX] {
                let delay = policy.delay(retry, entropy).as_millis();
                assert!(delay >= u128::from(cap * 3 / 4) && delay <= u128::from(cap));
            }
        }
        assert_eq!(policy.delay(0, 0), Duration::from_millis(3750));
        assert_eq!(policy.delay(1, 0), Duration::from_millis(7500));
        assert_ne!(policy.delay(12, 0), policy.delay(12, 1000));
    }
}
