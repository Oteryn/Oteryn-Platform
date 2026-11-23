//! Staff planning records for operations that await a ratified Game contract.
mod http;
mod model;
mod store;

pub(crate) use http::{has_scope, routes};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid planning input")]
    Invalid,
    #[error("planning access denied")]
    Denied,
    #[error("plan not found")]
    NotFound,
    #[error("planning window or state conflict")]
    Conflict,
    #[error("world operations planning unavailable")]
    Unavailable,
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        tracing::warn!(%error, "world operations database operation failed");
        Self::Unavailable
    }
}

impl From<crate::rbac::Error> for Error {
    fn from(error: crate::rbac::Error) -> Self {
        match error {
            crate::rbac::Error::Denied => Self::Denied,
            crate::rbac::Error::Storage(error) => error.into(),
            _ => Self::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests;
