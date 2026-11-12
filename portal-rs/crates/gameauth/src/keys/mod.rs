//! Fresh-admission signing-key custody (gateway contract §9). No grant semantics
//! from the proposed admission amendment are implemented here.
//!
//! Run only in the private issuer process. Persist the encrypted custody file
//! outside the application database and its restore/backup domain. Public keys
//! from `publication_keys` are inputs to the native signing-trust publisher, not
//! a substitute for ReadFreshSigningTrustV1 or its retained revision witnesses.
//! File custody enforces one writer. Other storage implementations must provide
//! the same exclusion. Never restore an older custody snapshot.
mod storage;
mod trust;
pub use storage::{EncryptedFileStorage, KeyStorage};
pub use trust::{PublicTrustRegistry, TrustSnapshot};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const ISSUER: &str = "urn:oteryn:platform:game-admission";
pub const PROFILE: &str = "oteryn-pre-admission-v1";

/// Deliberately contains no underlying I/O, configuration or secret values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    Disabled,
    BadInput,
    Unavailable,
    Untrusted,
    InvalidState,
    TooEarly,
}
impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signing-key operation refused: {self:?}")
    }
}
impl std::error::Error for KeyError {}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub key_purpose: String,
    pub clock_uncertainty_seconds: u8,
}

/// Read the authoritative, non-revoked fixed-scope trust set on every operation.
/// Implementations must fail on ambiguous state and enforce the evidence contract
/// retained revision witnesses. `trusted_since` is the publication time, not read time.
pub trait SigningTrust: fmt::Debug + Send + Sync {
    fn trusted_key(
        &self,
        issuer: &str,
        profile: &str,
        purpose: &str,
        kid: &str,
        now: u64,
    ) -> Result<Option<TrustedKey>, KeyError>;
}
#[derive(Debug, Clone, Copy)]
pub struct TrustedKey {
    pub public_key: [u8; 32],
    pub trusted_since: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyState {
    Active,
    Next,
    Retiring,
    Retired,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicKey {
    pub key_id: String,
    /// Canonical unpadded base64url, exactly 32 decoded bytes.
    pub public_key: String,
}
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct Seed([u8; 32]);
impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Seed([redacted])")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    kid: String,
    public_key: [u8; 32],
    seed: Option<Seed>,
    state: KeyState,
    last_expiry: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: u8,
    purpose: String,
    records: Vec<Record>,
}

/// Ephemeral signing result. Persist only the key id, exact signing input and
/// expiry with the admission attempt, never this signature or a complete token.
#[derive(Clone)]
pub struct SignedInput {
    key_id: String,
    input: Vec<u8>,
    signature: Vec<u8>,
    expires_at: u64,
}
impl fmt::Debug for SignedInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedInput")
            .field("key_id", &self.key_id)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}
impl SignedInput {
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn signing_input(&self) -> &[u8] {
        &self.input
    }
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Debug)]
pub struct Keyring<S> {
    settings: Settings,
    storage: S,
    state: State,
    poisoned: bool,
}
impl<S: KeyStorage> Keyring<S> {
    /// Explicit first-time provisioning. Never use this to recover lost custody.
    pub fn create(settings: Settings, storage: S) -> Result<Self, KeyError> {
        check_settings(&settings)?;
        if storage.load()?.is_some() {
            return Err(KeyError::InvalidState);
        }
        let state = State {
            version: 1,
            purpose: settings.key_purpose.clone(),
            records: vec![],
        };
        let mut ring = Self {
            settings,
            storage,
            state,
            poisoned: false,
        };
        ring.commit(ring.state.clone())?;
        Ok(ring)
    }
    /// Existing custody must be present, authenticated and internally consistent.
    pub fn open(settings: Settings, storage: S) -> Result<Self, KeyError> {
        check_settings(&settings)?;
        let bytes = storage.load()?.ok_or(KeyError::InvalidState)?;
        let state = serde_json::from_slice(&bytes).map_err(|_| KeyError::InvalidState)?;
        validate(&state, &settings)?;
        Ok(Self {
            settings,
            storage,
            state,
            poisoned: false,
        })
    }
    /// Generate inside issuer custody. Persist before revealing public material.
    /// A never-used id remains reserved even after emergency retirement.
    pub fn generate_next(&mut self, kid: &str) -> Result<PublicKey, KeyError> {
        self.enabled()?;
        if !valid_id(kid) {
            return Err(KeyError::BadInput);
        }
        if self.state.records.iter().any(|r| r.kid == kid)
            || self
                .state
                .records
                .iter()
                .any(|r| matches!(r.state, KeyState::Next | KeyState::Retiring))
            || self
                .state
                .records
                .iter()
                .filter(|r| r.state != KeyState::Retired)
                .count()
                >= 2
        {
            return Err(KeyError::InvalidState);
        }
        let mut seed = Seed([0; 32]);
        getrandom::fill(&mut seed.0).map_err(|_| KeyError::Unavailable)?;
        let public_key = SigningKey::from_bytes(&seed.0).verifying_key().to_bytes();
        let mut next = self.state.clone();
        next.records.push(Record {
            kid: kid.into(),
            public_key,
            seed: Some(seed),
            state: KeyState::Next,
            last_expiry: 0,
        });
        self.commit(next)?;
        Ok(PublicKey {
            key_id: kid.into(),
            public_key: URL_SAFE_NO_PAD.encode(public_key),
        })
    }
    /// Includes Next for publication. Never includes retired ids or private bytes.
    pub fn publication_keys(&self) -> Result<Vec<PublicKey>, KeyError> {
        self.enabled()?;
        Ok(self
            .state
            .records
            .iter()
            .filter(|r| r.state != KeyState::Retired)
            .map(|r| PublicKey {
                key_id: r.kid.clone(),
                public_key: URL_SAFE_NO_PAD.encode(r.public_key),
            })
            .collect())
    }
    /// Key id of the current signing key, placed in the grant header before signing.
    pub fn active_key_id(&self) -> Option<&str> {
        self.state
            .records
            .iter()
            .find(|r| r.state == KeyState::Active)
            .map(|r| r.kid.as_str())
    }
    pub fn state(&self, kid: &str) -> Option<KeyState> {
        self.state
            .records
            .iter()
            .find(|r| r.kid == kid)
            .map(|r| r.state)
    }
    /// Publish separately through the trust authority before calling this method.
    pub fn activate(
        &mut self,
        kid: &str,
        now: u64,
        trust: &dyn SigningTrust,
    ) -> Result<(), KeyError> {
        self.enabled()?;
        let record = self.record(kid)?;
        if record.state != KeyState::Next {
            return Err(KeyError::InvalidState);
        }
        let published = self.check_trust(record, now, trust)?;
        if now.saturating_sub(published.trusted_since) < self.overlap() {
            return Err(KeyError::TooEarly);
        }
        // A rotation cannot hide an already revoked or ambiguous current key.
        for active in self
            .state
            .records
            .iter()
            .filter(|r| r.state == KeyState::Active)
        {
            self.check_trust(active, now, trust)?;
        }
        let mut next = self.state.clone();
        for r in &mut next.records {
            if r.state == KeyState::Active {
                r.state = KeyState::Retiring;
            }
            if r.kid == kid {
                r.state = KeyState::Active;
            }
        }
        self.commit(next)
    }
    /// Low-level pure Ed25519 over the exact validated JWS signing input. The
    /// admission authority owns canonical header/claim validation and commit order.
    pub fn sign(
        &mut self,
        input: &[u8],
        expires_at: u64,
        now: u64,
        trust: &dyn SigningTrust,
    ) -> Result<SignedInput, KeyError> {
        self.enabled()?;
        let ttl = expires_at.checked_sub(now).ok_or(KeyError::BadInput)?;
        if !(5..=30).contains(&ttl)
            || input.is_empty()
            || input.len() + 87 > 4096
            || !input.is_ascii()
        {
            return Err(KeyError::BadInput);
        }
        let record = self
            .state
            .records
            .iter()
            .find(|r| r.state == KeyState::Active)
            .ok_or(KeyError::InvalidState)?;
        self.check_trust(record, now, trust)?;
        let signature = sign(record, input)?;
        let result = SignedInput {
            key_id: record.kid.clone(),
            input: input.into(),
            signature: signature.to_vec(),
            expires_at,
        };
        let mut next = self.state.clone();
        let r = next
            .records
            .iter_mut()
            .find(|r| r.kid == result.key_id)
            .ok_or(KeyError::InvalidState)?;
        r.last_expiry = r.last_expiry.max(expires_at);
        // Durable expiry tracking is required before returning a usable signature.
        self.commit(next)?;
        Ok(result)
    }
    /// Re-sign exact bytes from a committed admission attempt. Before calling,
    /// the admission authority must revalidate canonical claims, expiry, ticket,
    /// account and route bindings (§6.2); this low-level key service does not
    /// authorize admission. No stored signature or complete token is required.
    pub fn resign(
        &self,
        key_id: &str,
        input: &[u8],
        expires_at: u64,
        now: u64,
        trust: &dyn SigningTrust,
    ) -> Result<Vec<u8>, KeyError> {
        self.enabled()?;
        let record = self.record(key_id)?;
        if !matches!(record.state, KeyState::Active | KeyState::Retiring)
            || expires_at <= now
            || expires_at > record.last_expiry
        {
            return Err(KeyError::InvalidState);
        }
        if input.is_empty() || !input.is_ascii() || input.len() + 87 > 4096 {
            return Err(KeyError::BadInput);
        }
        self.check_trust(record, now, trust)?;
        Ok(sign(record, input)?.to_vec())
    }
    /// Earliest safe public revocation time for a normal rotation.
    pub fn retire_after(&self, kid: &str) -> Result<u64, KeyError> {
        self.enabled()?;
        let record = self.record(kid)?;
        if record.state != KeyState::Retiring {
            return Err(KeyError::InvalidState);
        }
        record
            .last_expiry
            .checked_add(self.overlap())
            .ok_or(KeyError::InvalidState)
    }
    /// Normal retirement is permitted only after every tracked grant has retired.
    /// The caller must revoke public trust before this method destroys the seed.
    pub fn retire(
        &mut self,
        kid: &str,
        now: u64,
        trust: &dyn SigningTrust,
        emergency: bool,
    ) -> Result<(), KeyError> {
        self.enabled()?;
        let record = self.record(kid)?;
        if record.state == KeyState::Retired {
            return Err(KeyError::InvalidState);
        }
        if !emergency
            && (record.state != KeyState::Retiring
                || now
                    < record
                        .last_expiry
                        .checked_add(self.overlap())
                        .ok_or(KeyError::InvalidState)?)
        {
            return Err(KeyError::TooEarly);
        }
        if trust
            .trusted_key(ISSUER, PROFILE, &self.settings.key_purpose, kid, now)?
            .is_some()
        {
            return Err(KeyError::Untrusted);
        }
        let mut next = self.state.clone();
        let r = next
            .records
            .iter_mut()
            .find(|r| r.kid == kid)
            .ok_or(KeyError::InvalidState)?;
        r.state = KeyState::Retired;
        r.seed = None;
        self.commit(next)
    }
    fn overlap(&self) -> u64 {
        5 + u64::from(self.settings.clock_uncertainty_seconds)
    }
    fn enabled(&self) -> Result<(), KeyError> {
        if self.poisoned {
            return Err(KeyError::Unavailable);
        }
        check_settings(&self.settings)
    }
    fn record(&self, kid: &str) -> Result<&Record, KeyError> {
        self.state
            .records
            .iter()
            .find(|r| r.kid == kid)
            .ok_or(KeyError::InvalidState)
    }
    fn check_trust(
        &self,
        record: &Record,
        now: u64,
        trust: &dyn SigningTrust,
    ) -> Result<TrustedKey, KeyError> {
        let t = trust
            .trusted_key(
                ISSUER,
                PROFILE,
                &self.settings.key_purpose,
                &record.kid,
                now,
            )?
            .ok_or(KeyError::Untrusted)?;
        if t.public_key != record.public_key || t.trusted_since > now {
            return Err(KeyError::Untrusted);
        }
        Ok(t)
    }
    fn commit(&mut self, next: State) -> Result<(), KeyError> {
        validate(&next, &self.settings)?;
        let bytes = Zeroizing::new(serde_json::to_vec(&next).map_err(|_| KeyError::Unavailable)?);
        if self.storage.save(&bytes).is_err() {
            // An I/O failure may have committed a snapshot. Require a fresh load.
            self.poisoned = true;
            return Err(KeyError::Unavailable);
        }
        self.state = next;
        Ok(())
    }
}
fn sign(record: &Record, input: &[u8]) -> Result<[u8; 64], KeyError> {
    let seed = record.seed.as_ref().ok_or(KeyError::InvalidState)?;
    Ok(SigningKey::from_bytes(&seed.0).sign(input).to_bytes())
}
pub fn verify(public_key: [u8; 32], input: &[u8], signature: &[u8]) -> Result<(), KeyError> {
    let key = VerifyingKey::from_bytes(&public_key).map_err(|_| KeyError::BadInput)?;
    let signature = Signature::from_slice(signature).map_err(|_| KeyError::BadInput)?;
    key.verify_strict(input, &signature)
        .map_err(|_| KeyError::Untrusted)
}
fn valid_id(kid: &str) -> bool {
    (1..=64).contains(&kid.len())
        && kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn check_settings(settings: &Settings) -> Result<(), KeyError> {
    if !settings.enabled {
        return Err(KeyError::Disabled);
    }
    if settings.clock_uncertainty_seconds > 5
        || settings.key_purpose.is_empty()
        || settings.key_purpose.len() > 256
        || !settings
            .key_purpose
            .bytes()
            .all(|b| (0x20..=0x7e).contains(&b))
    {
        return Err(KeyError::BadInput);
    }
    Ok(())
}
fn validate(state: &State, settings: &Settings) -> Result<(), KeyError> {
    let live = state
        .records
        .iter()
        .filter(|r| r.state != KeyState::Retired)
        .count();
    let count = |s| state.records.iter().filter(|r| r.state == s).count();
    if state.version != 1
        || state.purpose != settings.key_purpose
        || live > 2
        || count(KeyState::Active) > 1
        || count(KeyState::Next) > 1
        || count(KeyState::Retiring) > 1
        || (count(KeyState::Next) > 0 && count(KeyState::Retiring) > 0)
    {
        return Err(KeyError::InvalidState);
    }
    for (index, r) in state.records.iter().enumerate() {
        if !valid_id(&r.kid)
            || state.records[..index]
                .iter()
                .any(|old| old.kid == r.kid || old.public_key == r.public_key)
            || (r.state == KeyState::Retired) != r.seed.is_none()
        {
            return Err(KeyError::InvalidState);
        }
        if let Some(seed) = &r.seed
            && SigningKey::from_bytes(&seed.0).verifying_key().to_bytes() != r.public_key
        {
            return Err(KeyError::InvalidState);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
