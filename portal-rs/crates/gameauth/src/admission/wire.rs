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
    /// §19.2.5, protocol 3 only: the requested channel is not a candidate while
    /// another channel of the World is. The body carries the candidate list.
    ChannelUnavailable,
}
impl Error {
    pub fn status(self) -> StatusCode {
        match self {
            Self::Malformed | Self::UnsupportedVersion => StatusCode::BAD_REQUEST,
            Self::TicketRejected | Self::SecurityDenied => StatusCode::UNAUTHORIZED,
            Self::OfferUnsupported
            | Self::AttemptConflict
            | Self::CharacterConflict
            | Self::GrantExpired
            | Self::ChannelUnavailable => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    pub fn body(self, attempt: Option<&str>) -> Vec<u8> {
        self.body_for(attempt, 2, &[])
    }
    /// The §11.1 body for `protocol` (2 or 3); `channels` is the §19.2.5 member
    /// and is emitted only for a protocol 3 channel refusal.
    pub fn body_for(self, attempt: Option<&str>, protocol: u8, channels: &[Channel]) -> Vec<u8> {
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
            Self::ChannelUnavailable => (
                "NATIVE_LOGIN_CHANNEL_UNAVAILABLE",
                "TEMPORARILY_UNAVAILABLE",
            ),
        };
        let attempt = attempt.map_or(Value::Null, |s| Value::String(s.to_owned()));
        let mut members = vec![
            (
                "protocol_version".into(),
                Value::Unsigned(u64::from(protocol)),
            ),
            (
                "error".into(),
                Value::Object(vec![
                    ("code".into(), Value::String(code.into())),
                    ("public_class".into(), Value::String(class.into())),
                ]),
            ),
            ("attempt_ref".into(), attempt),
        ];
        if protocol == 3 && self == Self::ChannelUnavailable && !channels.is_empty() {
            members.push(("channels".into(), channel_list(channels)));
        }
        canonical::encode(&Value::Object(members)).unwrap_or_default()
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
    /// 2, or 3 when the protocol 3 switch accepted the request (§19.1).
    pub protocol: u8,
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
    /// Protocol 2 only.
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        Self::decode_with(raw, false)
    }
    /// Protocol 2, and protocol 3 when its switch is on (§19.1): the same member
    /// set, with 3 bound into the request digest so a retry cannot change version.
    pub fn decode_with(raw: &[u8], protocol3: bool) -> Result<Self, Error> {
        let value = canonical::decode(raw, MAX_BODY, 5).map_err(|_| Error::Malformed)?;
        let version = number(value.get("protocol_version"))?;
        let protocol = match version {
            2 => 2,
            3 if protocol3 => 3,
            _ => return Err(Error::UnsupportedVersion),
        };
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
        let mut binding = vec![
            Value::String(character_id.to_string()),
            channel_id.map_or(Value::Null, |v| Value::String(v.to_string())),
            offer.clone(),
        ];
        if protocol == 3 {
            binding.push(Value::Unsigned(3));
        }
        let binding = Value::Array(binding);
        let digest =
            Sha256::digest(canonical::encode(&binding).map_err(|_| Error::Malformed)?).into();
        Ok(Self {
            protocol,
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
/// The request's `protocol_version`, when the body decodes.
pub(super) fn version(raw: &[u8]) -> Option<i64> {
    let value = canonical::decode(raw, MAX_BODY, 5).ok()?;
    number(value.get("protocol_version")).ok()
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        if self.expires_at_ms - now > 30_000
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
/// The grant token of a stored or encoded success body.
pub(super) fn token(raw: &[u8]) -> Result<String, Error> {
    let success: Success =
        serde_json::from_slice(raw).map_err(|_| Error::ReconciliationRequired)?;
    Ok(success.grant.token)
}
/// The same success body with another grant token: the signing input for the
/// attempt record (§6.1 stores no signature) or the re-signed token on replay.
pub(super) fn with_token(raw: &[u8], token: &str) -> Result<Vec<u8>, Error> {
    let mut success: Success =
        serde_json::from_slice(raw).map_err(|_| Error::ReconciliationRequired)?;
    success.grant.token = token.into();
    encode_success(&success)
}
/// World, channel and endpoint bound by a stored success body.
pub(super) fn scope(raw: &[u8]) -> Result<(Uuid, Uuid, Endpoint), Error> {
    let success: Success =
        serde_json::from_slice(raw).map_err(|_| Error::ReconciliationRequired)?;
    Ok((success.world_id, success.channel_id, success.endpoint))
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

/// A §19.2.4 `channels` entry: a candidate channel and its Registry key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub channel_id: Uuid,
    pub key: String,
}
impl Channel {
    /// §19.2.2: the key with its first ASCII letter upper-cased.
    pub fn label(&self) -> String {
        let mut label = self.key.clone();
        if let Some(index) = label.find(|c: char| c.is_ascii_lowercase()) {
            label[index..=index].make_ascii_uppercase();
        }
        label
    }
}
/// The Registry channel-key grammar `[a-z0-9][a-z0-9-]{0,63}`.
pub fn channel_key_valid(key: &str) -> bool {
    let bytes = key.as_bytes();
    (1..=64).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}
fn channel_list(channels: &[Channel]) -> Value {
    Value::Array(
        channels
            .iter()
            .map(|c| {
                Value::Object(vec![
                    ("channel_id".into(), Value::String(c.channel_id.to_string())),
                    ("key".into(), Value::String(c.key.clone())),
                    ("label".into(), Value::String(c.label())),
                ])
            })
            .collect(),
    )
}
/// Largest protocol 3 login response (§19.2.4).
pub const MAX_SUCCESS3: usize = 8192;
/// Most `channels` entries a response lists (§19.2.4).
pub const MAX_LISTED_CHANNELS: usize = 16;
/// The §19.3 `recovery` member: the plaintext handle and whole seconds until
/// its absolute expiry. Never persisted.
pub struct Recovery {
    pub handle: String,
    pub expires_in: u64,
}
impl std::fmt::Debug for Recovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Recovery{redacted}")
    }
}
/// The §19.2.4 protocol 3 success body from a protocol 2 shaped core body (the
/// stored attempt shape), in the contract's member order.
pub(super) fn success3(
    core: &[u8],
    automatic: bool,
    channels: &[Channel],
    recovery: Option<&Recovery>,
) -> Result<Vec<u8>, Error> {
    if channels.is_empty() || channels.len() > MAX_LISTED_CHANNELS {
        return Err(Error::Unavailable);
    }
    let value = canonical::decode(core, 6144, 5).map_err(|_| Error::Unavailable)?;
    let member = |name: &str| value.get(name).cloned().ok_or(Error::Unavailable);
    let recovery = recovery.map_or(Value::Null, |r| {
        Value::Object(vec![
            ("handle".into(), Value::String(r.handle.clone())),
            ("expires_in".into(), Value::Unsigned(r.expires_in)),
        ])
    });
    let selection = if automatic { "automatic" } else { "requested" };
    let body = canonical::encode(&Value::Object(vec![
        ("protocol_version".into(), Value::Unsigned(3)),
        ("attempt_ref".into(), member("attempt_ref")?),
        ("world_id".into(), member("world_id")?),
        ("channel_id".into(), member("channel_id")?),
        ("channel_selection".into(), Value::String(selection.into())),
        ("channels".into(), channel_list(channels)),
        ("endpoint".into(), member("endpoint")?),
        ("grant".into(), member("grant")?),
        ("recovery".into(), recovery),
    ]))
    .map_err(|_| Error::Unavailable)?;
    if body.len() > MAX_SUCCESS3 {
        return Err(Error::Unavailable);
    }
    Ok(body)
}
