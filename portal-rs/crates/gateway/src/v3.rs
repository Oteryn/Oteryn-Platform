//! Protocol 3 wire on `POST /v1/login` (gateway login contract §19.1-§19.2).
//! The request member set is §3.1 with `3`; the success response adds the
//! channel selection, the candidate list and the recovery handle; the channel
//! refusal adds `channels`. Protocol 2 types and bodies are untouched.
use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use std::fmt;

use crate::{
    errors::{Code, Refusal},
    wire::{self, Endpoint, Grant, Request, check_json, exact, uuid_v7},
};

/// Largest protocol 3 login response (§19.2.4).
pub const MAX_RESPONSE_BYTES: usize = 8192;
const MAX_CHANNELS: usize = 16;

/// The raw `protocol_version` is the literal `3`.
pub fn is_v3(body: &[u8]) -> bool {
    serde_json::from_slice::<std::collections::HashMap<String, Box<RawValue>>>(body)
        .ok()
        .and_then(|mut m| m.remove("protocol_version"))
        .is_some_and(|v| v.get() == "3")
}
pub fn parse_request(body: &[u8]) -> Result<Request, Code> {
    wire::parse_versioned(body, 3)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub channel_id: String,
    pub key: String,
    pub label: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub handle: String,
    pub expires_in: i64,
}
impl fmt::Debug for Recovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Recovery{redacted}")
    }
}
/// §19.2.4, in the contract's member order.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Success {
    pub protocol_version: i64,
    pub attempt_ref: String,
    pub world_id: String,
    pub channel_id: String,
    pub channel_selection: String,
    pub channels: Vec<Channel>,
    pub endpoint: Endpoint,
    pub grant: Grant,
    pub recovery: Option<Recovery>,
}
impl fmt::Debug for Success {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Success")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}
/// The Registry channel-key grammar `[a-z0-9][a-z0-9-]{0,63}`.
fn key_valid(key: &str) -> bool {
    let b = key.as_bytes();
    (1..=64).contains(&b.len())
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}
/// §19.2.2: the key with its first ASCII letter upper-cased.
pub fn label(key: &str) -> String {
    let mut label = key.to_owned();
    if let Some(index) = label.find(|c: char| c.is_ascii_lowercase()) {
        label[index..=index].make_ascii_uppercase();
    }
    label
}
/// 1..=16 entries, canonical ascending `channel_id`, exact entry shape.
pub fn channels_valid(channels: &[Channel]) -> bool {
    (1..=MAX_CHANNELS).contains(&channels.len())
        && channels
            .iter()
            .all(|c| uuid_v7(&c.channel_id) && key_valid(&c.key) && c.label == label(&c.key))
        && channels
            .windows(2)
            .all(|pair| pair[0].channel_id < pair[1].channel_id)
}
/// `otr1.` and 43 unpadded base64url characters (§19.3).
pub fn handle_valid(handle: &str) -> bool {
    handle.strip_prefix("otr1.").is_some_and(|rest| {
        rest.len() == 43
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            // 32 bytes leave the last character's low four bits zero.
            && rest
                .bytes()
                .last()
                .is_some_and(|b| b"AEIMQUYcgkosw048".contains(&b))
    })
}
pub fn parse_success(body: &[u8], r: &Request) -> Option<Success> {
    if body.len() > MAX_RESPONSE_BYTES || !check_json(body, 3) {
        return None;
    }
    let v: Value = serde_json::from_slice(body).ok()?;
    if !exact(
        &v,
        &[
            "protocol_version",
            "attempt_ref",
            "world_id",
            "channel_id",
            "channel_selection",
            "channels",
            "endpoint",
            "grant",
            "recovery",
        ],
        &["recovery"],
    ) || !v["recovery"].is_null() && !exact(&v["recovery"], &["handle", "expires_in"], &[])
    {
        return None;
    }
    let s: Success = serde_json::from_slice(body).ok()?;
    let core = wire::Success {
        protocol_version: 2,
        attempt_ref: s.attempt_ref.clone(),
        world_id: s.world_id.clone(),
        channel_id: s.channel_id.clone(),
        endpoint: s.endpoint.clone(),
        grant: s.grant.clone(),
    };
    let selection = if r.channel_id.is_some() {
        "requested"
    } else {
        "automatic"
    };
    let valid = s.protocol_version == 3
        && wire::core_valid(&core, r)
        && s.channel_selection == selection
        && channels_valid(&s.channels)
        && s.recovery
            .as_ref()
            .is_none_or(|h| handle_valid(&h.handle) && (1..=43_200).contains(&h.expires_in));
    valid.then_some(s)
}

/// A protocol 3 refusal: the §11.1 body with `protocol_version: 3`, plus
/// `channels` for `NATIVE_LOGIN_CHANNEL_UNAVAILABLE` only (§19.2.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal3 {
    pub refusal: Refusal,
    pub channels: Vec<Channel>,
}
impl From<Code> for Refusal3 {
    fn from(code: Code) -> Self {
        Self {
            refusal: code.into(),
            channels: Vec::new(),
        }
    }
}
impl From<Refusal> for Refusal3 {
    fn from(refusal: Refusal) -> Self {
        Self {
            refusal,
            channels: Vec::new(),
        }
    }
}
#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    protocol_version: u8,
    error: ErrorDetail,
    attempt_ref: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    channels: Vec<Channel>,
}
#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    public_class: &'static str,
}
pub fn body<'a>(refused: &Refusal3, attempt_ref: Option<&'a str>) -> ErrorBody<'a> {
    let code = refused.refusal.code;
    ErrorBody {
        protocol_version: 3,
        error: ErrorDetail {
            code: code.name(),
            public_class: code.class(),
        },
        attempt_ref,
        channels: if code == Code::ChannelUnavailable {
            refused.channels.clone()
        } else {
            Vec::new()
        },
    }
}
pub fn parse_issuer_error(status: u16, b: &[u8], attempt: &str) -> Option<Refusal3> {
    if !check_json(b, 3) {
        return None;
    }
    let v: Value = serde_json::from_slice(b).ok()?;
    let base = ["protocol_version", "error", "attempt_ref"];
    let name = v.get("error")?.get("code")?.as_str()?;
    let channel = name == Code::ChannelUnavailable.name();
    let shaped = if channel {
        exact(
            &v,
            &["protocol_version", "error", "attempt_ref", "channels"],
            &["attempt_ref"],
        )
    } else {
        exact(&v, &base, &["attempt_ref"])
    };
    if !shaped
        || v["protocol_version"].as_i64()? != 3
        || (!v["attempt_ref"].is_null() && v["attempt_ref"].as_str()? != attempt)
        || !exact(&v["error"], &["code", "public_class"], &[])
    {
        return None;
    }
    let name = match name {
        "NATIVE_LOGIN_TICKET_REJECTED" | "NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED" => {
            Code::AuthenticationRequired.name()
        }
        _ => name,
    };
    let code = Code::ALL
        .into_iter()
        .chain([Code::ChannelUnavailable])
        .find(|c| c.name() == name)?;
    let channels = if channel {
        let channels: Vec<Channel> = serde_json::from_value(v["channels"].clone()).ok()?;
        channels_valid(&channels).then_some(channels)?
    } else {
        Vec::new()
    };
    (code.status() == status && v["error"]["public_class"].as_str()? == code.class()).then(|| {
        Refusal3 {
            refusal: code.into(),
            channels,
        }
    })
}
