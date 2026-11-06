//! Owner-accepted policy and deployment trust are separate inputs.
use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

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
    pub peer_certificate_sha256: String,
    pub keys: BTreeMap<String, [u8; 32]>,
    pub recovery_file: PathBuf,
    pub requests_per_minute: usize,
    pub max_envelope_bytes: usize,
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
pub(super) fn public_keys(raw: &str) -> Result<BTreeMap<String, String>> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique public key IDs")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate key ID"));
                }
            }
            Ok(values)
        }
    }
    let mut decoder = serde_json::Deserializer::from_str(raw);
    let values = serde::de::Deserializer::deserialize_map(&mut decoder, Unique)
        .map_err(|_| Error::Invalid)?;
    decoder.end().map_err(|_| Error::Invalid)?;
    Ok(values)
}
impl Config {
    pub fn from_env() -> Result<Self> {
        let raw = public_keys(&value("OTERYN_LEAGUE_PUBLIC_KEYS")?)?;
        let keys = raw
            .into_iter()
            .map(|(id, key)| Ok((id, unhex(&key)?.try_into().map_err(|_| Error::Invalid)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        schema::ensure(
            !keys.is_empty()
                && keys.len() <= 16
                && keys
                    .keys()
                    .all(|k| !k.is_empty() && k.len() <= 128 && k.is_ascii()),
        )?;
        let config = Self {
            authority: value("OTERYN_LEAGUE_SOURCE_AUTHORITY")?,
            audience: value("OTERYN_LEAGUE_BASE_URL")?,
            bind: value("OTERYN_LEAGUE_BIND")?,
            ca: value("OTERYN_LEAGUE_CLIENT_CA_DER")?.into(),
            certificate: value("OTERYN_LEAGUE_CERTIFICATE_DER")?.into(),
            private_key: value("OTERYN_LEAGUE_PRIVATE_KEY_DER")?.into(),
            peer_certificate_sha256: value("OTERYN_LEAGUE_PEER_CERTIFICATE_SHA256")?,
            keys,
            recovery_file: value("OTERYN_LEAGUE_RECOVERY_FILE")?.into(),
            max_envelope_bytes: std::env::var("OTERYN_LEAGUE_MAX_ENVELOPE_BYTES")
                .ok()
                .map(|s| s.parse().map_err(|_| Error::Invalid))
                .transpose()?
                .unwrap_or(4096),
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
                && unhex(&config.peer_certificate_sha256)?.len() == 32
                && (1..=1200).contains(&config.requests_per_minute)
                && (1..=4096).contains(&config.max_envelope_bytes),
        )?;
        config.connections()?;
        config.batch_limit()?;
        config.subject_limit()?;
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
    pub(super) fn subject_limit(&self) -> Result<usize> {
        Self::cap("OTERYN_LEAGUE_SUBJECT_LIMIT", 1000)
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
