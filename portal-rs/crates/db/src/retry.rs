//! Bounded retries of complete transactions, preserving database SQLSTATEs.
use std::{
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Internal transaction failure; application refusals are never retried.
#[derive(Debug)]
pub enum TransactionError<E> {
    Database(sqlx::Error),
    Application(E),
}
impl<E> From<sqlx::Error> for TransactionError<E> {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
impl<E: From<sqlx::Error>> TransactionError<E> {
    pub fn into_inner(self) -> E {
        match self {
            Self::Database(error) => error.into(),
            Self::Application(error) => error,
        }
    }
}

/// `attempt` must create a fresh transaction and include discovery and commit.
/// Only explicit PostgreSQL serialization/deadlock failures are safe to replay;
/// connection failures have an uncertain commit outcome and are never retried.
pub async fn transaction<T, E, F, Fut>(mut attempt: F) -> Result<T, E>
where
    E: From<sqlx::Error>,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, TransactionError<E>>>,
{
    let mut number = 1;
    loop {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(TransactionError::Database(error))
                if number < 3
                    && error.as_database_error().is_some_and(|db| {
                        matches!(db.code().as_deref(), Some("40001" | "40P01"))
                    }) =>
            {
                let jitter = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |time| u64::from(time.subsec_nanos()) % 11);
                tokio::time::sleep(Duration::from_millis(number * 5 + jitter)).await;
            }
            Err(error) => return Err(error.into_inner()),
        }
        number += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, PartialEq, Eq)]
    enum Error {
        Unavailable,
        Refused,
    }
    impl From<sqlx::Error> for Error {
        fn from(_: sqlx::Error) -> Self {
            Self::Unavailable
        }
    }
    #[tokio::test]
    async fn application_refusals_are_not_retried() {
        let attempts = Cell::new(0);
        let result: Result<(), Error> = transaction(|| {
            attempts.set(attempts.get() + 1);
            std::future::ready(Err(TransactionError::Application(Error::Refused)))
        })
        .await;
        assert_eq!(result, Err(Error::Refused));
        assert_eq!(attempts.get(), 1);
    }
    #[tokio::test]
    async fn connection_errors_are_not_retried() {
        let attempts = Cell::new(0);
        let result: Result<(), Error> = transaction(|| {
            attempts.set(attempts.get() + 1);
            std::future::ready(Err(TransactionError::Database(sqlx::Error::PoolClosed)))
        })
        .await;
        assert_eq!(result, Err(Error::Unavailable));
        assert_eq!(attempts.get(), 1);
    }
}
