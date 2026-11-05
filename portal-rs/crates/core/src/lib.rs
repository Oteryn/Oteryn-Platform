//! Shared configuration, errors and identifiers for the Oteryn portal.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Deployment environment. Features that touch the game or money stay off unless enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Local development.
    Local,
    /// Automated tests.
    Testing,
    /// Pre-production on `oteryn-vps-2`.
    Preprod,
    /// Production.
    Production,
}

impl Environment {
    fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw {
            "local" => Ok(Self::Local),
            "testing" => Ok(Self::Testing),
            "preprod" => Ok(Self::Preprod),
            "production" => Ok(Self::Production),
            other => Err(ConfigError::Invalid {
                key: "OTERYN_ENV",
                value: other.to_owned(),
            }),
        }
    }
}

/// Process configuration read from the environment. Secrets are never logged.
#[derive(Clone)]
pub struct Config {
    /// Environment the process runs in.
    pub environment: Environment,
    /// Public listener address.
    pub bind: SocketAddr,
    /// PostgreSQL connection string.
    database_url: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("environment", &self.environment)
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .finish()
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is missing.
    #[error("missing environment variable {0}")]
    Missing(&'static str),
    /// A variable has an invalid value.
    #[error("invalid value for {key}: {value}")]
    Invalid {
        /// Variable name.
        key: &'static str,
        /// Rejected value (never a secret).
        value: String,
    },
}

impl Config {
    /// Reads configuration through `lookup`, so tests do not touch the process environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let environment =
            Environment::parse(&lookup("OTERYN_ENV").unwrap_or_else(|| "local".to_owned()))?;
        let bind_raw = lookup("OTERYN_BIND").unwrap_or_else(|| "127.0.0.1:8080".to_owned());
        let bind = bind_raw.parse().map_err(|_| ConfigError::Invalid {
            key: "OTERYN_BIND",
            value: bind_raw.clone(),
        })?;
        let database_url =
            lookup("OTERYN_DATABASE_URL").ok_or(ConfigError::Missing("OTERYN_DATABASE_URL"))?;
        Ok(Self {
            environment,
            bind,
            database_url,
        })
    }

    /// Reads configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// PostgreSQL connection string. Do not log it.
    pub fn database_url(&self) -> &str {
        &self.database_url
    }
}

/// New time-ordered identifier (UUIDv7), used for every primary key.
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn config_reads_defaults_and_redacts_the_database_url() -> Result<(), ConfigError> {
        let config = Config::from_lookup(lookup(&[(
            "OTERYN_DATABASE_URL",
            "postgres://u:secret@h/db",
        )]))?;
        assert_eq!(config.environment, Environment::Local);
        assert_eq!(config.bind.to_string(), "127.0.0.1:8080");
        assert!(!format!("{config:?}").contains("secret"));
        Ok(())
    }

    #[test]
    fn config_rejects_missing_database_and_unknown_environment() {
        assert!(matches!(
            Config::from_lookup(lookup(&[])),
            Err(ConfigError::Missing(_))
        ));
        assert!(matches!(
            Config::from_lookup(lookup(&[
                ("OTERYN_DATABASE_URL", "x"),
                ("OTERYN_ENV", "prod")
            ])),
            Err(ConfigError::Invalid { .. })
        ));
    }

    #[test]
    fn ids_are_version_seven_and_ordered() {
        let first = new_id();
        let second = new_id();
        assert_eq!(first.get_version_num(), 7);
        assert!(first < second);
    }
}
