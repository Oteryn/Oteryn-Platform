use axum::http::StatusCode;
use oteryn_internal_api::canonical::{self, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MAX_BODY: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed,
    UnsupportedVersion,
    OfferUnsupported,
    TicketRejected,
    SecurityDenied,
    AttemptConflict,
    CharacterConflict,
    RouteUnavailable,
    RateLimited,
    GrantExpired,
    ReconciliationRequired,
    Unavailable,
}
impl Error {
    pub fn status(self) -> StatusCode {
        match self {
            Self::Malformed | Self::UnsupportedVersion => StatusCode::BAD_REQUEST,
            Self::TicketRejected | Self::SecurityDenied => StatusCode::UNAUTHORIZED,
            Self::OfferUnsupported
            | Self::AttemptConflict
            | Self::CharacterConflict
            | Self::GrantExpired => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    pub fn body(self, attempt: Option<&str>) -> Vec<u8> {
        let (code, class) = match self {
            Self::Malformed => ("NATIVE_LOGIN_REQUEST_MALFORMED", "RETRY_LOGIN"),
            Self::UnsupportedVersion => {
                ("NATIVE_LOGIN_UNSUPPORTED_VERSION", "CLIENT_UPDATE_REQUIRED")
            }
            Self::OfferUnsupported => ("NATIVE_LOGIN_OFFER_UNSUPPORTED", "CLIENT_UPDATE_REQUIRED"),
            Self::TicketRejected | Self::SecurityDenied => (
                "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
                "AUTHENTICATION_REQUIRED",
            ),
            Self::AttemptConflict => ("NATIVE_LOGIN_ATTEMPT_CONFLICT", "RETRY_LOGIN"),
            Self::CharacterConflict => ("NATIVE_LOGIN_CHARACTER_CONFLICT", "SESSION_UNAVAILABLE"),
            Self::RouteUnavailable => ("NATIVE_LOGIN_ROUTE_UNAVAILABLE", "TEMPORARILY_UNAVAILABLE"),
            Self::RateLimited => ("NATIVE_LOGIN_RATE_LIMITED", "TEMPORARILY_UNAVAILABLE"),
            Self::GrantExpired => ("NATIVE_LOGIN_GRANT_EXPIRED", "RETRY_LOGIN"),
            Self::ReconciliationRequired => (
                "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
                "TEMPORARILY_UNAVAILABLE",
            ),
            Self::Unavailable => ("NATIVE_LOGIN_UNAVAILABLE", "TEMPORARILY_UNAVAILABLE"),
        };
        let attempt = attempt.map_or(Value::Null, |s| Value::String(s.to_owned()));
        canonical::encode(&Value::Object(vec![
            ("protocol_version".into(), Value::Unsigned(2)),
            (
                "error".into(),
                Value::Object(vec![
                    ("code".into(), Value::String(code.into())),
                    ("public_class".into(), Value::String(class.into())),
                ]),
            ),
            ("attempt_ref".into(), attempt),
        ]))
        .unwrap_or_default()
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native admission refused: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone)]
pub struct AdmissionRequest {
    pub attempt_ref: Uuid,
    pub character_id: Uuid,
    pub channel_id: Option<Uuid>,
    ticket_hash: [u8; 32],
    digest: [u8; 32],
    ticket: String,
    pub offer: Value,
}
impl std::fmt::Debug for AdmissionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdmissionRequest")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}
impl AdmissionRequest {
    /// Bearer material is exposed only to the ticket authority and never persisted.
    pub fn ticket(&self) -> &str {
        &self.ticket
    }
    pub(super) fn ticket_hash(&self) -> &[u8] {
        &self.ticket_hash
    }
    pub(super) fn digest(&self) -> &[u8] {
        &self.digest
    }
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        let value = canonical::decode(raw, MAX_BODY, 5).map_err(|_| Error::Malformed)?;
        let version = number(value.get("protocol_version"))?;
        if version != 2 {
            return Err(Error::UnsupportedVersion);
        }
        keys(
            &value,
            &[
                "protocol_version",
                "game_login_ticket",
                "attempt_ref",
                "character_id",
                "channel_id",
                "offer",
            ],
        )?;
        let ticket = string(value.get("game_login_ticket"))?;
        visible(ticket, 256)?;
        let attempt_ref = uuid(string(value.get("attempt_ref"))?)?;
        let character_id = uuid(string(value.get("character_id"))?)?;
        let channel_id = match value.get("channel_id") {
            Some(Value::Null) => None,
            v => Some(uuid(string(v)?)?),
        };
        let offer = value.get("offer").ok_or(Error::Malformed)?;
        keys(offer, &["client_build", "client_platform", "transports"])?;
        visible(string(offer.get("client_build"))?, 64)?;
        if string(offer.get("client_platform"))? != "windows" {
            return Err(Error::Malformed);
        }
        let Some(Value::Array(transports)) = offer.get("transports") else {
            return Err(Error::Malformed);
        };
        if !(1..=4).contains(&transports.len()) {
            return Err(Error::Malformed);
        }
        let mut supported = false;
        for transport in transports {
            keys(transport, &["protocol_major", "transport_profile", "alpn"])?;
            let major = number(transport.get("protocol_major"))?;
            let profile = number(transport.get("transport_profile"))?;
            if !(0..=65535).contains(&major) || !(0..=65535).contains(&profile) {
                return Err(Error::Malformed);
            }
            let alpn = string(transport.get("alpn"))?;
            visible(alpn, 255)?;
            supported |= major == 1 && profile == 1 && alpn == "oteryn-game/1";
        }
        if !supported {
            return Err(Error::OfferUnsupported);
        }
        // The validated shape contains ASCII strings and integers only. Sorting
        // object keys recursively is the contract's JCS offer serialization.
        fn sorted(value: &Value) -> Value {
            match value {
                Value::Object(members) => {
                    let mut members: Vec<_> = members
                        .iter()
                        .map(|(k, v)| (k.clone(), sorted(v)))
                        .collect();
                    members.sort_by(|a, b| a.0.cmp(&b.0));
                    Value::Object(members)
                }
                Value::Array(values) => Value::Array(values.iter().map(sorted).collect()),
                v => v.clone(),
            }
        }
        let offer = sorted(offer);
        let binding = Value::Array(vec![
            Value::String(character_id.to_string()),
            channel_id.map_or(Value::Null, |v| Value::String(v.to_string())),
            offer.clone(),
        ]);
        let digest =
            Sha256::digest(canonical::encode(&binding).map_err(|_| Error::Malformed)?).into();
        Ok(Self {
            attempt_ref,
            character_id,
            channel_id,
            ticket_hash: Sha256::digest(ticket.as_bytes()).into(),
            digest,
            ticket: ticket.into(),
            offer,
        })
    }
}
pub(super) fn attempt(raw: &[u8]) -> Option<String> {
    let value = canonical::decode(raw, MAX_BODY, 5).ok()?;
    let text = string(value.get("attempt_ref")).ok()?;
    uuid(text).ok().map(|_| text.into())
}
fn keys(value: &Value, names: &[&str]) -> Result<(), Error> {
    match value {
        Value::Object(members)
            if members.len() == names.len()
                && names.iter().all(|name| value.get(name).is_some()) =>
        {
            Ok(())
        }
        _ => Err(Error::Malformed),
    }
}
fn string(value: Option<&Value>) -> Result<&str, Error> {
    match value {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::Malformed),
    }
}
fn number(value: Option<&Value>) -> Result<i64, Error> {
    match value {
        Some(Value::Signed(n)) => Ok(*n),
        Some(Value::Unsigned(n)) => i64::try_from(*n).map_err(|_| Error::Malformed),
        _ => Err(Error::Malformed),
    }
}
fn visible(text: &str, max: usize) -> Result<(), Error> {
    if text.is_empty() || text.len() > max || !text.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        Err(Error::Malformed)
    } else {
        Ok(())
    }
}
pub(super) fn uuid(text: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(text).map_err(|_| Error::Malformed)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != text
    {
        return Err(Error::Malformed);
    }
    Ok(id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub tls_server_name: String,
    pub alpn: String,
    pub protocol_major: u16,
    pub transport_profile: u16,
}
/// Produced by the trusted route/grant authority, never from request routing claims.
#[derive(Clone)]
pub struct Material {
    pub world_id: Uuid,
    pub channel_id: Uuid,
    pub endpoint: Endpoint,
    pub token: String,
    pub expires_at_ms: i64,
}
impl std::fmt::Debug for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Material{redacted}")
    }
}
#[derive(Serialize, Deserialize)]
struct Grant {
    profile: String,
    token: String,
    valid_for_seconds: i64,
}
#[derive(Serialize, Deserialize)]
struct Success {
    protocol_version: u8,
    attempt_ref: Uuid,
    world_id: Uuid,
    channel_id: Uuid,
    endpoint: Endpoint,
    grant: Grant,
}
impl Material {
    pub(super) fn encode(&self, request: &AdmissionRequest, now: i64) -> Result<Vec<u8>, Error> {
        if self.expires_at_ms - now < 1000 {
            return Err(Error::GrantExpired);
        }
        if self.expires_at_ms - now > 20_000
            || self.endpoint.port == 0
            || self.endpoint.alpn != "oteryn-game/1"
            || self.endpoint.protocol_major != 1
            || self.endpoint.transport_profile != 1
            || self.token.is_empty()
            || self.token.len() > 4096
            || !self.token.is_ascii()
            || uuid(&self.world_id.to_string()).is_err()
            || uuid(&self.channel_id.to_string()).is_err()
        {
            return Err(Error::Unavailable);
        }
        if request.channel_id.is_some_and(|id| id != self.channel_id) {
            return Err(Error::RouteUnavailable);
        }
        let success = Success {
            protocol_version: 2,
            attempt_ref: request.attempt_ref,
            world_id: self.world_id,
            channel_id: self.channel_id,
            endpoint: self.endpoint.clone(),
            grant: Grant {
                profile: "oteryn-pre-admission-v1".into(),
                token: self.token.clone(),
                valid_for_seconds: (self.expires_at_ms - now) / 1000,
            },
        };
        encode_success(&success)
    }
}
fn encode_success(success: &Success) -> Result<Vec<u8>, Error> {
    let raw = serde_json::to_vec(success).map_err(|_| Error::Unavailable)?;
    let value = canonical::decode(&raw, 6144, 5).map_err(|_| Error::Unavailable)?;
    canonical::encode(&value).map_err(|_| Error::Unavailable)
}
pub(super) fn replay(raw: &[u8], expires: i64, now: i64) -> Result<Vec<u8>, Error> {
    if expires - now < 1000 {
        return Err(Error::GrantExpired);
    }
    let mut success: Success =
        serde_json::from_slice(raw).map_err(|_| Error::ReconciliationRequired)?;
    success.grant.valid_for_seconds = (expires - now) / 1000;
    encode_success(&success)
}
