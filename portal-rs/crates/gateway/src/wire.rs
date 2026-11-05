//! Exact protocol 2 wire. The gateway validates routing material but never interprets claims.
use std::{collections::HashSet, fmt, net::IpAddr};

use serde::{
    Deserialize, Serialize,
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, value::RawValue};

use crate::errors::Code;

pub const MAX_REQUEST_BYTES: usize = 2048;
pub const MAX_RESPONSE_BYTES: usize = 6144;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transport {
    pub protocol_major: i64,
    pub transport_profile: i64,
    pub alpn: String,
}
impl Transport {
    fn supported(&self) -> bool {
        self.protocol_major == 1 && self.transport_profile == 1 && self.alpn == "oteryn-game/1"
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub client_build: String,
    pub client_platform: String,
    pub transports: Vec<Transport>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol_version: i64,
    pub game_login_ticket: String,
    pub attempt_ref: String,
    pub character_id: String,
    pub channel_id: Option<String>,
    pub offer: Offer,
}
impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub host: String,
    pub port: i64,
    pub tls_server_name: String,
    pub alpn: String,
    pub protocol_major: i64,
    pub transport_profile: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub profile: String,
    pub token: String,
    pub valid_for_seconds: i64,
}
impl fmt::Debug for Grant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Grant{redacted}")
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Success {
    pub protocol_version: i64,
    pub attempt_ref: String,
    pub world_id: String,
    pub channel_id: String,
    pub endpoint: Endpoint,
    pub grant: Grant,
}
impl fmt::Debug for Success {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Success")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}

pub fn uuid_v7(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'7',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}
fn visible(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

/// The Go outer router uses the last duplicate member and the literal integer spelling.
pub fn is_native(body: &[u8]) -> bool {
    serde_json::from_slice::<std::collections::HashMap<String, Box<RawValue>>>(body)
        .ok()
        .and_then(|mut m| m.remove("protocol_version"))
        .is_some_and(|v| v.get() == "2")
}
pub fn attempt_echo(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > MAX_REQUEST_BYTES {
        return None;
    }
    let v: Value = serde_json::from_slice(body).ok()?;
    let s = v.get("attempt_ref")?.as_str()?;
    uuid_v7(s).then(|| s.to_owned())
}

pub fn parse_request(body: &[u8]) -> Result<Request, Code> {
    let malformed = Code::RequestMalformed;
    if body.is_empty() || body.len() > MAX_REQUEST_BYTES || !check_json(body, 4) {
        return Err(malformed);
    }
    // Check the top-level member set before version, as the reference parser does.
    let v: Value = serde_json::from_slice(body).map_err(|_| malformed)?;
    if !exact(
        &v,
        &[
            "protocol_version",
            "game_login_ticket",
            "attempt_ref",
            "character_id",
            "channel_id",
            "offer",
        ],
        &["channel_id"],
    ) {
        return Err(malformed);
    }
    let version = v["protocol_version"].as_i64().ok_or(malformed)?;
    if version != 2 {
        return Err(Code::UnsupportedVersion);
    }
    let r: Request = serde_json::from_slice(body).map_err(|_| malformed)?;
    if !visible(&r.game_login_ticket, 256)
        || !uuid_v7(&r.attempt_ref)
        || !uuid_v7(&r.character_id)
        || r.channel_id.as_deref().is_some_and(|s| !uuid_v7(s))
        || !visible(&r.offer.client_build, 64)
        || r.offer.client_platform != "windows"
        || !(1..=4).contains(&r.offer.transports.len())
    {
        return Err(malformed);
    }
    if r.offer.transports.iter().any(|t| {
        !visible(&t.alpn, 255)
            || !(0..=65535).contains(&t.protocol_major)
            || !(0..=65535).contains(&t.transport_profile)
    }) {
        return Err(malformed);
    }
    if !r.offer.transports.iter().any(Transport::supported) {
        return Err(Code::OfferUnsupported);
    }
    Ok(r)
}

/// Compact contract order, without HTML escaping or a trailing newline.
pub fn encode_request(r: &Request) -> Result<Vec<u8>, Code> {
    let b = encode(r).map_err(|_| Code::RequestMalformed)?;
    if b.len() > MAX_REQUEST_BYTES {
        return Err(Code::RequestMalformed);
    }
    Ok(b)
}
pub(crate) fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>, serde_json::Error> {
    // Go always escapes the two JavaScript line separators, even with EscapeHTML(false).
    Ok(serde_json::to_string(v)?
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .into_bytes())
}
pub fn parse_success(body: &[u8], r: &Request) -> Option<Success> {
    if body.len() > MAX_RESPONSE_BYTES || !check_json(body, 3) {
        return None;
    }
    let s: Success = serde_json::from_slice(body).ok()?;
    let e = &s.endpoint;
    let t = Transport {
        protocol_major: e.protocol_major,
        transport_profile: e.transport_profile,
        alpn: e.alpn.clone(),
    };
    let token_parts: Vec<_> = s.grant.token.split('.').collect();
    let valid = s.protocol_version == 2
        && s.attempt_ref == r.attempt_ref
        && uuid_v7(&s.world_id)
        && uuid_v7(&s.channel_id)
        && r.channel_id.as_ref().is_none_or(|c| c == &s.channel_id)
        && (e.host.parse::<IpAddr>().is_ok() || dns_name(&e.host))
        && (1..=65535).contains(&e.port)
        && dns_name(&e.tls_server_name)
        && !e
            .tls_server_name
            .rsplit('.')
            .next()?
            .bytes()
            .all(|b| b.is_ascii_digit())
        && t.supported()
        && s.grant.profile == "oteryn-pre-admission-v1"
        && s.grant.token.len() <= 4096
        && token_parts.len() == 3
        && token_parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
        && (1..=30).contains(&s.grant.valid_for_seconds);
    valid.then_some(s)
}
fn dns_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && s.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && l.as_bytes()[0].is_ascii_alphanumeric()
                && l.as_bytes()[l.len() - 1].is_ascii_alphanumeric()
                && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}
pub(crate) fn exact(v: &Value, names: &[&str], nullable: &[&str]) -> bool {
    v.as_object().is_some_and(|m| {
        m.len() == names.len()
            && names.iter().all(|n| {
                m.get(*n)
                    .is_some_and(|v| !v.is_null() || nullable.contains(n))
            })
    })
}

// Walk without constructing unbounded containers. Decoded keys catch escaped duplicates too.
struct Check(usize);
impl<'de> DeserializeSeed<'de> for Check {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Check {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded JSON without duplicate members")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<(), M::Error> {
        if self.0 == 0 {
            return Err(de::Error::custom("JSON depth"));
        }
        let mut keys = HashSet::new();
        while let Some(k) = m.next_key::<String>()? {
            if !keys.insert(k) {
                return Err(de::Error::custom("duplicate JSON member"));
            }
            m.next_value_seed(Check(self.0 - 1))?;
        }
        Ok(())
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut s: S) -> Result<(), S::Error> {
        if self.0 == 0 {
            return Err(de::Error::custom("JSON depth"));
        }
        while s.next_element_seed(Check(self.0 - 1))?.is_some() {}
        Ok(())
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
}
pub(crate) fn check_json(b: &[u8], depth: usize) -> bool {
    let mut d = serde_json::Deserializer::from_slice(b);
    Check(depth).deserialize(&mut d).is_ok() && d.end().is_ok()
}
