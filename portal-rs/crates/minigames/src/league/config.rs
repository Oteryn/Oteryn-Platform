//! Owner-accepted policy and deployment trust are separate inputs.
use super::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn enabled() -> bool {
    std::env::var("OTERYN_LEAGUE_ENABLED").as_deref() == Ok("true")
}
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Policy {
    accepted_sources: bool,
    accepted_season_rules: bool,
    accepted_final_freeze: bool,
    accepted_pki: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            accepted_sources: true,
            accepted_season_rules: true,
            accepted_final_freeze: true,
            accepted_pki: true,
        }
    }
}
pub fn owner_flags(on: bool) -> Flags {
    let policy = match std::env::var("OTERYN_LEAGUE_POLICY") {
        Ok(s) => serde_json::from_str::<Policy>(&s).ok(),
        Err(std::env::VarError::NotPresent) => Some(Policy::default()),
        Err(_) => None,
    };
    let mut flags = Flags {
        enabled: on,
        ingest: on,
        public: on,
        ..Flags::default()
    };
    if let Some(p) = policy {
        flags.accepted_sources = p.accepted_sources;
        flags.accepted_season_rules = p.accepted_season_rules;
        flags.accepted_final_freeze = p.accepted_final_freeze;
        flags.accepted_pki = p.accepted_pki;
    }
    flags
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub source_authority: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub authority: String,
    pub audience: String,
    pub bind: String,
    pub ca: PathBuf,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub client_spki_sha256: String,
    pub recovery_file: PathBuf,
    pub requests_per_minute: usize,
}
fn value(key: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or(Error::Unavailable)
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    schema::ensure(
        s.len().is_multiple_of(2)
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )?;
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::Invalid))
        .collect()
}
impl Config {
    pub fn from_env() -> Result<Self> {
        let config = Self {
            authority: value("OTERYN_LEAGUE_SOURCE_AUTHORITY")?,
            audience: value("OTERYN_LEAGUE_BASE_URL")?,
            bind: value("OTERYN_LEAGUE_BIND")?,
            ca: value("OTERYN_LEAGUE_CLIENT_CA_DER")?.into(),
            certificate: value("OTERYN_LEAGUE_CERTIFICATE_DER")?.into(),
            private_key: value("OTERYN_LEAGUE_PRIVATE_KEY_DER")?.into(),
            client_spki_sha256: normalize_spki(&value("OTERYN_LEAGUE_CLIENT_SPKI_SHA256")?)?,
            recovery_file: value("OTERYN_LEAGUE_RECOVERY_FILE")?.into(),
            requests_per_minute: std::env::var("OTERYN_LEAGUE_REQUESTS_PER_MINUTE")
                .ok()
                .map(|s| s.parse().map_err(|_| Error::Invalid))
                .transpose()?
                .unwrap_or(120),
        };
        schema::ensure(
            config.audience.starts_with("https://")
                && !config.audience.ends_with('/')
                && config.audience.len() <= 512
                && config.authority == "oteryn:game:primary"
                && (1..=1200).contains(&config.requests_per_minute),
        )?;
        config.connections()?;
        config.batch_limit()?;
        let audience: axum::http::Uri = config.audience.parse().map_err(|_| Error::Invalid)?;
        schema::ensure(
            audience.scheme_str() == Some("https")
                && audience.authority().is_some()
                && audience.query().is_none()
                && audience.path() == "/"
                && !config.audience.contains(['@', '#']),
        )?;
        config.recovery()?;
        Ok(config)
    }
    fn cap(name: &str, default: usize) -> Result<usize> {
        let n = match std::env::var(name) {
            Ok(s) => s.parse::<usize>().map_err(|_| Error::Invalid)?,
            Err(std::env::VarError::NotPresent) => default,
            Err(_) => return Err(Error::Invalid),
        };
        schema::ensure(n > 0 && n <= default)?;
        Ok(n)
    }
    pub(super) fn connections(&self) -> Result<usize> {
        Self::cap("OTERYN_LEAGUE_CONNECTION_LIMIT", 16)
    }
    pub(super) fn batch_limit(&self) -> Result<usize> {
        Self::cap("OTERYN_LEAGUE_BATCH_LIMIT", 64)
    }
    /// This operator-owned file must be on independent nonrollback storage. Both
    /// processes reread it for every admission/use; database state cannot mint it.
    pub fn recovery(&self) -> Result<Recovery> {
        let raw = std::fs::read(&self.recovery_file).map_err(|_| Error::Unavailable)?;
        schema::ensure(raw.len() <= 1024)?;
        let r: Recovery = serde_json::from_slice(&raw).map_err(|_| Error::Invalid)?;
        schema::ensure(r.source_authority == self.authority)?;
        schema::number(&r.projection_epoch, false)?;
        schema::number(&r.recovery_generation, false)?;
        Ok(r)
    }
}

pub(super) fn normalize_spki(raw: &str) -> Result<String> {
    let hex = raw.strip_prefix("sha256:").unwrap_or(raw);
    schema::ensure(unhex(hex)?.len() == 32)?;
    Ok(format!("sha256:{hex}"))
}
