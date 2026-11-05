//! ADM-01 signing primitives. No transport, replay store or execution policy.
//! Callers must validate operation-specific closed schemas before signing.
use super::canonical;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
pub use canonical::Json;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, de};
use sha2::{Digest, Sha256};
use std::{fmt, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("admin signing is disabled")]
    Disabled,
    #[error("ADM_SIGNER_UNTRUSTED")]
    Untrusted,
    #[error("ADM_BAD_SIGNATURE")]
    BadSignature,
    #[error("invalid signing key configuration")]
    KeyConfiguration,
    #[error("invalid envelope")]
    Envelope,
    #[error("canonical JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("key file could not be read")]
    KeyFile,
}

// Requiring a deserializer prevents serde from silently defaulting absent
// nullable members to null, which would change the signed JSON object.
fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

// UUID parsing must not normalize a different JSON string before verification.
fn canonical_uuid<'de, D: Deserializer<'de>>(d: D) -> Result<uuid::Uuid, D::Error> {
    let text = String::deserialize(d)?;
    let id = uuid::Uuid::parse_str(&text).map_err(de::Error::custom)?;
    if id.to_string() != text {
        return Err(de::Error::custom(
            "UUID must use canonical lowercase hyphenated text",
        ));
    }
    Ok(id)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[serde(deserialize_with = "required_nullable")]
    pub world_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub node_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub character_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub account_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub admin_id: String,
    pub roles: Vec<String>,
    pub permission: String,
    pub mfa_at: String,
    #[serde(deserialize_with = "required_nullable")]
    pub step_up_at: Option<String>,
    pub ip_hash: String,
    pub session_ref: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reason {
    pub code: String,
    pub text: String,
    #[serde(deserialize_with = "required_nullable")]
    pub ticket_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub admin_id: String,
    pub approved_at: String,
    pub approval_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Command,
    Query,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub contract: String,
    #[serde(deserialize_with = "canonical_uuid")]
    pub request_id: uuid::Uuid,
    #[serde(deserialize_with = "canonical_uuid")]
    pub admin_operation_id: uuid::Uuid,
    pub operation: String,
    pub operation_version: u32,
    pub kind: Kind,
    pub target: Target,
    pub params: Json,
    pub dry_run: bool,
    pub actor: Actor,
    #[serde(deserialize_with = "required_nullable")]
    pub reason: Option<Reason>,
    pub approvals: Vec<Approval>,
    pub risk_class: RiskClass,
    pub issued_at: String,
    pub expires_at: String,
    pub nonce: String,
    pub signer_key_id: String,
    pub signature: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Accepted,
    InProgress,
    Succeeded,
    Failed,
    Rejected,
    Expired,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResultError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub step: u32,
    pub of: u32,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResultEnvelope {
    #[serde(deserialize_with = "canonical_uuid")]
    pub request_id: uuid::Uuid,
    #[serde(deserialize_with = "canonical_uuid")]
    pub admin_operation_id: uuid::Uuid,
    pub status: Status,
    #[serde(deserialize_with = "required_nullable")]
    pub error: Option<ResultError>,
    pub result: Json,
    #[serde(deserialize_with = "required_nullable")]
    pub dry_run_preview: Option<Json>,
    #[serde(deserialize_with = "required_nullable")]
    pub game_ledger_id: Option<String>,
    pub audit_event_ids: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub effective_revision: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub progress: Option<Progress>,
    #[serde(deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    pub agent_id: String,
    pub agent_build: String,
    pub signature: String,
}

/// Exact signed bytes, also the integration hook for ADM-X1 golden envelopes
/// to be published by Game under crates/admin/contracts. No placeholder fixtures.
pub fn adm_x1_signing_bytes<T: Serialize>(envelope: &T) -> Result<Vec<u8>, Error> {
    let mut value = serde_json::to_value(envelope)?;
    let object = value.as_object_mut().ok_or(Error::Envelope)?;
    if object.remove("signature").is_none() {
        return Err(Error::Envelope);
    }
    Ok(canonical::encode_wire(&value)?)
}

pub fn params_digest(params: &Json) -> Result<[u8; 32], Error> {
    Ok(Sha256::digest(canonical::encode_wire(params)?).into())
}

/// A half-open UTC Unix-seconds validity window: not_before <= now < not_after.
#[derive(Debug, Clone, Copy)]
pub struct Validity {
    pub not_before: i64,
    pub not_after: i64,
}
impl Validity {
    fn contains(self, now: i64) -> bool {
        self.not_before <= now && now < self.not_after
    }
}

pub struct AdminKey {
    id: String,
    key: SigningKey,
    validity: Validity,
}
impl fmt::Debug for AdminKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminKey")
            .field("id", &self.id)
            .field("validity", &self.validity)
            .finish_non_exhaustive()
    }
}
impl AdminKey {
    pub fn new(id: String, key: SigningKey, validity: Validity) -> Result<Self, Error> {
        let suffix = id
            .strip_prefix("platform-admin-signer-")
            .ok_or(Error::KeyConfiguration)?;
        if suffix.is_empty()
            || !suffix.bytes().all(|b| b.is_ascii_digit())
            || validity.not_before >= validity.not_after
        {
            return Err(Error::KeyConfiguration);
        }
        Ok(Self { id, key, validity })
    }
    /// Load a raw 32-byte Ed25519 seed from a path in the named environment variable.
    /// The variable contains only a path; the file is never logged.
    pub fn from_env(variable: &str, id: String, validity: Validity) -> Result<Self, Error> {
        let path = std::env::var_os(variable).ok_or(Error::KeyFile)?;
        Self::from_file(Path::new(&path), id, validity)
    }
    pub fn from_file(path: &Path, id: String, validity: Validity) -> Result<Self, Error> {
        let bytes = zeroize::Zeroizing::new(std::fs::read(path).map_err(|_| Error::KeyFile)?);
        let seed: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| Error::KeyConfiguration)?;
        let seed = zeroize::Zeroizing::new(seed);
        let key = SigningKey::from_bytes(&seed);
        Self::new(id, key, validity)
    }
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
}

/// Signing is disabled by default. Activation is an explicit caller feature flag.
#[derive(Debug, Default)]
pub struct KeyRing {
    enabled: bool,
    keys: Vec<AdminKey>,
}
impl KeyRing {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            keys: Vec::new(),
        }
    }
    /// First key is current, second is next. Selection is explicit, never fallback.
    pub fn add(&mut self, key: AdminKey) -> Result<(), Error> {
        if self.keys.len() >= 2
            || self
                .keys
                .iter()
                .any(|k| k.id == key.id || k.verifying_key() == key.verifying_key())
        {
            return Err(Error::KeyConfiguration);
        }
        self.keys.push(key);
        Ok(())
    }
    pub fn sign(&self, request: &mut RequestEnvelope, key_id: &str, now: i64) -> Result<(), Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let key = self
            .keys
            .iter()
            .find(|k| k.id == key_id && k.validity.contains(now))
            .ok_or(Error::Untrusted)?;
        validate_request(request)?;
        let mut candidate = request.clone();
        candidate.signer_key_id = key_id.to_owned();
        candidate.signature =
            URL_SAFE_NO_PAD.encode(key.key.sign(&adm_x1_signing_bytes(&candidate)?).to_bytes());
        *request = candidate;
        Ok(())
    }
    pub fn verify(&self, request: &RequestEnvelope, now: i64) -> Result<(), Error> {
        let key = self
            .keys
            .iter()
            .find(|k| k.id == request.signer_key_id)
            .ok_or(Error::Untrusted)?;
        validate_request(request)?;
        verify_signature(
            request,
            &request.signature,
            &key.verifying_key(),
            key.validity,
            now,
        )
    }
}
fn validate_request(request: &RequestEnvelope) -> Result<(), Error> {
    let nonce = URL_SAFE_NO_PAD
        .decode(&request.nonce)
        .map_err(|_| Error::Envelope)?;
    if request.contract != "oteryn.admin-control.v1"
        || request.operation_version == 0
        || request.request_id.get_version_num() != 7
        || request.admin_operation_id.get_version_num() != 7
        || nonce.len() != 16
        || !request.params.0.is_object()
        || request.operation.is_empty()
        || (request.kind == Kind::Query && request.admin_operation_id != request.request_id)
    {
        return Err(Error::Envelope);
    }
    Ok(())
}
/// Verify an agent result against a key and window obtained from trusted configuration.
/// This proves authenticity only; callers enforce identity, freshness and operation schema.
pub fn verify_result(
    result: &ResultEnvelope,
    key: &VerifyingKey,
    validity: Validity,
    now: i64,
) -> Result<(), Error> {
    verify_signature(result, &result.signature, key, validity, now)
}
fn verify_signature<T: Serialize>(
    envelope: &T,
    signature: &str,
    key: &VerifyingKey,
    validity: Validity,
    now: i64,
) -> Result<(), Error> {
    if !validity.contains(now) {
        return Err(Error::Untrusted);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| Error::BadSignature)?;
    let signature = Signature::from_slice(&bytes).map_err(|_| Error::BadSignature)?;
    key.verify_strict(&adm_x1_signing_bytes(envelope)?, &signature)
        .map_err(|_| Error::BadSignature)
}

#[cfg(test)]
#[path = "signer_tests.rs"]
mod tests;
