//! Bound filesystem work before submitting it to Tokio's blocking pool.
//! Permits remain owned by a running worker even if its request is cancelled.
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

static CAPACITY: Semaphore = Semaphore::const_new(8);

pub(crate) async fn run<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ()> {
    let permit = CAPACITY.acquire().await.map_err(|_| ())?;
    // The global semaphore is static, so the borrowed permit can move to the worker.
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|_| ())
}

pub(crate) async fn serial(gate: Arc<Semaphore>) -> Result<OwnedSemaphorePermit, ()> {
    gate.acquire_owned().await.map_err(|_| ())
}
