//! Protocol 3 recovery wire (gateway login contract §19.4, §19.7, §19.8):
//! `POST /v1/recover` relays to `native-recoveries`, `POST /v1/recover/revoke`
//! to `native-recoveries/revoke`. The gateway validates shapes and routing
//! evidence only, holds no state and never interprets the grant. Handles and
//! grants never reach a log or a `Debug` output (§19.10).
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use crate::{
    errors::Code,
    v3::handle_valid,
    wire::{self, Offer, check_json, check_offer, exact, uuid_v7},
};

pub const RECOVER_PATH: &str = "/internal/v1/game-auth/native-recoveries";
pub const REVOKE_PATH: &str = "/internal/v1/game-auth/native-recoveries/revoke";
/// Recovery request and response limits (§19.9).
pub const MAX_REQUEST_BYTES: usize = 2048;
pub const MAX_RESPONSE_BYTES: usize = 6144;
const PROFILE: &str = "oteryn-reauth-recovery-v1";

/// §19.8 public codes and the protocol 2 codes protocol 3 reuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverCode {
    Malformed,
    UnsupportedVersion,
    OfferUnsupported,
    AuthenticationRequired,
    AttemptConflict,
    AttemptPending,
    CharacterConflict,
    WorldStale,
    RouteUnavailable,
    GrantExpired,
    ReconciliationRequired,
    RateLimited,
    Unavailable,
}
impl RecoverCode {
    pub const ALL: [Self; 13] = [
        Self::Malformed,
        Self::UnsupportedVersion,
        Self::OfferUnsupported,
        Self::AuthenticationRequired,
        Self::AttemptConflict,
        Self::AttemptPending,
        Self::CharacterConflict,
        Self::WorldStale,
        Self::RouteUnavailable,
        Self::GrantExpired,
        Self::ReconciliationRequired,
        Self::RateLimited,
        Self::Unavailable,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Malformed => "NATIVE_RECOVERY_REQUEST_MALFORMED",
            Self::UnsupportedVersion => "NATIVE_LOGIN_UNSUPPORTED_VERSION",
            Self::OfferUnsupported => "NATIVE_LOGIN_OFFER_UNSUPPORTED",
            Self::AuthenticationRequired => "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
            Self::AttemptConflict => "NATIVE_RECOVERY_ATTEMPT_CONFLICT",
            Self::AttemptPending => "NATIVE_RECOVERY_ATTEMPT_PENDING",
            Self::CharacterConflict => "NATIVE_RECOVERY_CHARACTER_CONFLICT",
            Self::WorldStale => "NATIVE_RECOVERY_WORLD_STALE",
            Self::RouteUnavailable => "NATIVE_RECOVERY_ROUTE_UNAVAILABLE",
            Self::GrantExpired => "NATIVE_RECOVERY_GRANT_EXPIRED",
            Self::ReconciliationRequired => "RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED",
            Self::RateLimited => "NATIVE_LOGIN_RATE_LIMITED",
            Self::Unavailable => "NATIVE_LOGIN_UNAVAILABLE",
        }
    }
    pub fn status(self) -> u16 {
        match self {
            Self::Malformed | Self::UnsupportedVersion => 400,
            Self::AuthenticationRequired => 401,
            Self::OfferUnsupported
            | Self::AttemptConflict
            | Self::AttemptPending
            | Self::CharacterConflict
            | Self::WorldStale
            | Self::GrantExpired => 409,
            Self::RateLimited => 429,
            Self::RouteUnavailable | Self::ReconciliationRequired | Self::Unavailable => 503,
        }
    }
    pub fn class(self) -> &'static str {
        match self {
            Self::Malformed | Self::AttemptConflict | Self::WorldStale | Self::GrantExpired => {
                "RETRY_LOGIN"
            }
            Self::UnsupportedVersion | Self::OfferUnsupported => "CLIENT_UPDATE_REQUIRED",
            Self::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            Self::CharacterConflict => "SESSION_UNAVAILABLE",
            Self::AttemptPending
            | Self::RouteUnavailable
            | Self::ReconciliationRequired
            | Self::RateLimited
            | Self::Unavailable => "TEMPORARILY_UNAVAILABLE",
        }
    }
    /// Whether the refusal carries `Retry-After` (§10, §19.4 step 3).
    pub fn retries_after(self) -> bool {
        matches!(self, Self::RateLimited | Self::AttemptPending)
    }
}
/// The gateway's own transport and parsing outcomes on the recovery routes.
impl From<Code> for RecoverCode {
    fn from(code: Code) -> Self {
        match code {
            Code::RequestMalformed => Self::Malformed,
            Code::UnsupportedVersion => Self::UnsupportedVersion,
            Code::OfferUnsupported => Self::OfferUnsupported,
            Code::RateLimited => Self::RateLimited,
            Code::ReconciliationRequired => Self::ReconciliationRequired,
            _ => Self::Unavailable,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoverRefusal {
    pub code: RecoverCode,
    pub retry_after: u64,
}
impl From<RecoverCode> for RecoverRefusal {
    fn from(code: RecoverCode) -> Self {
        Self {
            code,
            retry_after: 0,
        }
    }
}
impl From<Code> for RecoverRefusal {
    fn from(code: Code) -> Self {
        RecoverCode::from(code).into()
    }
}

/// §19.4 request with §19.15 binding off: `device_proof` absent.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoverRequest {
    pub protocol_version: i64,
    pub recovery_handle: String,
    pub attempt_ref: String,
    pub character_id: String,
    pub offer: Offer,
}
impl fmt::Debug for RecoverRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoverRequest")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}
/// §19.7 request.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRequest {
    pub protocol_version: i64,
    pub recovery_handle: String,
}
impl fmt::Debug for RevokeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RevokeRequest{redacted}")
    }
}
/// The §3.1 parsing rules: bounded body, no duplicate members, exact member
/// set before the version, then `3` only.
fn front(body: &[u8], members: &[&str]) -> Result<(), RecoverCode> {
    let malformed = RecoverCode::Malformed;
    if body.is_empty() || body.len() > MAX_REQUEST_BYTES || !check_json(body, 4) {
        return Err(malformed);
    }
    let v: Value = serde_json::from_slice(body).map_err(|_| malformed)?;
    if !exact(&v, members, &[]) {
        return Err(malformed);
    }
    if v["protocol_version"].as_i64().ok_or(malformed)? != 3 {
        return Err(RecoverCode::UnsupportedVersion);
    }
    Ok(())
}
pub fn parse_request(body: &[u8]) -> Result<RecoverRequest, RecoverCode> {
    front(
        body,
        &[
            "protocol_version",
            "recovery_handle",
            "attempt_ref",
            "character_id",
            "offer",
        ],
    )?;
    let r: RecoverRequest = serde_json::from_slice(body).map_err(|_| RecoverCode::Malformed)?;
    if !handle_valid(&r.recovery_handle) || !uuid_v7(&r.attempt_ref) || !uuid_v7(&r.character_id) {
        return Err(RecoverCode::Malformed);
    }
    check_offer(&r.offer)?;
    Ok(r)
}
pub fn parse_revoke(body: &[u8]) -> Result<RevokeRequest, RecoverCode> {
    front(body, &["protocol_version", "recovery_handle"])?;
    let r: RevokeRequest = serde_json::from_slice(body).map_err(|_| RecoverCode::Malformed)?;
    if !handle_valid(&r.recovery_handle) {
        return Err(RecoverCode::Malformed);
    }
    Ok(r)
}
/// Compact contract order, as the public request.
pub fn encode_request(r: &RecoverRequest) -> Result<Vec<u8>, RecoverCode> {
    encode_bounded(r)
}
pub fn encode_revoke(r: &RevokeRequest) -> Result<Vec<u8>, RecoverCode> {
    encode_bounded(r)
}
fn encode_bounded(value: &impl Serialize) -> Result<Vec<u8>, RecoverCode> {
    wire::encode(value)
        .ok()
        .filter(|b| b.len() <= MAX_REQUEST_BYTES)
        .ok_or(RecoverCode::Malformed)
}

/// The §19.4 success: the §3.2 member set with a recovery grant.
pub fn parse_success(body: &[u8], r: &RecoverRequest) -> Option<wire::Success> {
    if body.len() > MAX_RESPONSE_BYTES || !check_json(body, 3) {
        return None;
    }
    let s: wire::Success = serde_json::from_slice(body).ok()?;
    (s.protocol_version == 3 && wire::evidence_valid(&s, &r.attempt_ref, PROFILE)).then_some(s)
}
/// Exactly `{"protocol_version":3,"revoked":true}` (§19.7).
pub fn revoked(body: &[u8]) -> bool {
    body == br#"{"protocol_version":3,"revoked":true}"#
}
#[derive(Debug, Serialize)]
pub struct Revoked {
    protocol_version: u8,
    revoked: bool,
}
pub const REVOKED: Revoked = Revoked {
    protocol_version: 3,
    revoked: true,
};

#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    protocol_version: u8,
    error: ErrorDetail,
    attempt_ref: Option<&'a str>,
}
#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    public_class: &'static str,
}
/// The §11.1 body with `protocol_version: 3` (§19.2.5).
pub fn body(code: RecoverCode, attempt_ref: Option<&str>) -> ErrorBody<'_> {
    ErrorBody {
        protocol_version: 3,
        error: ErrorDetail {
            code: code.name(),
            public_class: code.class(),
        },
        attempt_ref,
    }
}
/// An issuer refusal: the protocol 3 §11.1 body whose echo is `attempt` (or
/// `null`), with the status of its code. `SECURITY_TERMINAL` internal codes
/// collapse to `NATIVE_LOGIN_AUTHENTICATION_REQUIRED` (§19.8).
pub fn parse_issuer_error(status: u16, b: &[u8], attempt: Option<&str>) -> Option<RecoverCode> {
    if !check_json(b, 3) {
        return None;
    }
    let v: Value = serde_json::from_slice(b).ok()?;
    if !exact(
        &v,
        &["protocol_version", "error", "attempt_ref"],
        &["attempt_ref"],
    ) || v["protocol_version"].as_i64()? != 3
        || !exact(&v["error"], &["code", "public_class"], &[])
    {
        return None;
    }
    let echo = v["attempt_ref"].as_str();
    if echo.is_some() && echo != attempt {
        return None;
    }
    let name = match v["error"]["code"].as_str()? {
        "NATIVE_RECOVERY_HANDLE_REJECTED"
        | "NATIVE_RECOVERY_DEVICE_PROOF_REJECTED"
        | "NATIVE_RECOVERY_ACCOUNT_SECURITY_DENIED" => RecoverCode::AuthenticationRequired.name(),
        name => name,
    };
    let code = RecoverCode::ALL.into_iter().find(|c| c.name() == name)?;
    (code.status() == status && v["error"]["public_class"].as_str()? == code.class())
        .then_some(code)
}
