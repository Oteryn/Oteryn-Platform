//! Contract JSON, preserving member order rather than sorting keys.
//!
//! StoreWire.php uses JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE,
//! without JSON_PRESERVE_ZERO_FRACTION. Wire contracts use integer JSON numbers;
//! decimals, exponents and numbers above PHP_INT_MAX are rejected rather than
//! rounded through floating point. Contract uint64 values are decimal strings.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    Signed(i64),
    Unsigned(u64),
    Bool(bool),
    Null,
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("ordered JSON with integer numbers and unique members")
            }
            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Value, M::Error> {
                let mut members: Vec<(String, Value)> = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if members.iter().any(|(k, _)| k == &key) {
                        return Err(de::Error::custom("duplicate JSON member"));
                    }
                    members.push((key, value));
                }
                Ok(Value::Object(members))
            }
            fn visit_seq<S: de::SeqAccess<'de>>(self, mut seq: S) -> Result<Value, S::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Value::Array(values))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
                Ok(Value::String(value.to_owned()))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Signed(value))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
                if value > i64::MAX as u64 {
                    return Err(E::custom("JSON integer exceeds PHP_INT_MAX"));
                }
                Ok(Value::Unsigned(value))
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Object(members) => {
                use ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(members.len()))?;
                let mut keys = std::collections::HashSet::new();
                for (key, value) in members {
                    if !keys.insert(key) {
                        return Err(ser::Error::custom("duplicate JSON member"));
                    }
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Array(values) => values.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Signed(value) => serializer.serialize_i64(*value),
            Self::Unsigned(value) => {
                if *value > i64::MAX as u64 {
                    return Err(ser::Error::custom("JSON integer exceeds PHP_INT_MAX"));
                }
                serializer.serialize_u64(*value)
            }
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Null => serializer.serialize_unit(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Size,
    Json,
    Depth,
    Schema,
    NonCanonical,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contract JSON rejected: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Structural shape with members in contract order. Types and domain constraints
/// beyond object/array structure belong to the owning handler.
#[derive(Debug)]
pub enum Shape {
    Object(&'static [(&'static str, Shape)]),
    Array(&'static Shape),
    Scalar,
}

impl Shape {
    pub fn validate(&self, value: &Value) -> Result<(), Error> {
        match (self, value) {
            (Self::Object(fields), Value::Object(members)) if fields.len() == members.len() => {
                for ((expected, shape), (actual, value)) in fields.iter().zip(members) {
                    if expected != actual {
                        return Err(Error::Schema);
                    }
                    shape.validate(value)?;
                }
                Ok(())
            }
            (Self::Array(shape), Value::Array(values)) => {
                values.iter().try_for_each(|value| shape.validate(value))
            }
            (
                Self::Scalar,
                Value::String(_)
                | Value::Signed(_)
                | Value::Unsigned(_)
                | Value::Bool(_)
                | Value::Null,
            ) => Ok(()),
            _ => Err(Error::Schema),
        }
    }
}

/// PHP-compatible compact encoding, including its default escaping of U+2028/2029.
/// No Unicode normalization or lexicographic sorting is performed.
pub fn encode(value: &Value) -> Result<Vec<u8>, Error> {
    serde_json::to_string(value)
        .map(|s| {
            s.replace('\u{2028}', "\\u2028")
                .replace('\u{2029}', "\\u2029")
                .into_bytes()
        })
        .map_err(|_| Error::Json)
}

/// Decode one UTF-8 object. Duplicate members and non-integer JSON numbers fail.
/// `max_depth` follows PHP's json_decode depth: root container consumes one level,
/// so a depth limit of 4 permits at most three nested container levels including root.
pub fn decode(raw: &[u8], max_bytes: usize, max_depth: usize) -> Result<Value, Error> {
    if raw.is_empty() || raw.len() > max_bytes {
        return Err(Error::Size);
    }
    let value: Value = serde_json::from_slice(raw).map_err(|_| Error::Json)?;
    if !matches!(value, Value::Object(_)) {
        return Err(Error::Schema);
    }
    fn depth(value: &Value, level: usize, max: usize) -> Result<(), Error> {
        match value {
            Value::Object(members) => {
                if level >= max {
                    return Err(Error::Depth);
                }
                members
                    .iter()
                    .try_for_each(|(_, v)| depth(v, level + 1, max))
            }
            Value::Array(values) => {
                if level >= max {
                    return Err(Error::Depth);
                }
                values.iter().try_for_each(|v| depth(v, level + 1, max))
            }
            _ => Ok(()),
        }
    }
    depth(&value, 1, max_depth)?;
    Ok(value)
}

/// Store and command-style canonical bodies have no backslash, BOM, whitespace
/// or trailing newline, and exact member order at every schema-defined level.
pub fn decode_canonical(
    raw: &[u8],
    max_bytes: usize,
    max_depth: usize,
    shape: &Shape,
) -> Result<Value, Error> {
    let value = decode(raw, max_bytes, max_depth)?;
    shape.validate(&value)?;
    if raw.contains(&b'\\') || encode(&value)? != raw {
        return Err(Error::NonCanonical);
    }
    Ok(value)
}

/// Encode a bounded canonical wire object. This is the outbound counterpart to
/// `decode_canonical`; the general `encode` also serves escaped schema artifacts.
pub fn encode_canonical(
    value: &Value,
    max_bytes: usize,
    max_depth: usize,
    shape: &Shape,
) -> Result<Vec<u8>, Error> {
    shape.validate(value)?;
    let bytes = encode(value)?;
    decode_canonical(&bytes, max_bytes, max_depth, shape)?;
    Ok(bytes)
}
