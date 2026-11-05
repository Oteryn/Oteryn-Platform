//! RFC 8785 encoding with the ADM-01 safe-integer wire restriction.
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use std::fmt;

/// Encode JCS bytes. Large integer values must be supplied as decimal strings.
pub fn encode_wire<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    validate(&value).map_err(<serde_json::Error as serde::ser::Error>::custom)?;
    serde_json_canonicalizer::to_vec(&value)
}

/// RFC 8785 canonicalisation without the ADM wire safe-integer restriction.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json_canonicalizer::to_vec(value)
}

fn validate(value: &Value) -> Result<(), &'static str> {
    match value {
        Value::Number(n) => {
            let f = n.as_f64().ok_or("invalid number")?;
            if !f.is_finite() || (f.fract() == 0.0 && f.abs() > 9_007_199_254_740_992.0) {
                return Err("integers beyond ±2^53 must be decimal strings");
            }
            // Check integer storage before any lossy binary64 conversion.
            if n.as_u64().is_some_and(|v| v > 9_007_199_254_740_992)
                || n.as_i64().is_some_and(|v| v < -9_007_199_254_740_992)
            {
                return Err("integers beyond ±2^53 must be decimal strings");
            }
        }
        Value::Array(a) => {
            for v in a {
                validate(v)?;
            }
        }
        Value::Object(o) => {
            for v in o.values() {
                validate(v)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// JSON payload retaining duplicate detection during deserialization.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Json(pub Value);

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("unique-member I-JSON")
            }
            fn visit_map<M: de::MapAccess<'de>>(self, mut m: M) -> Result<Value, M::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, Json(v))) = m.next_entry::<String, Json>()? {
                    if out.insert(k, v).is_some() {
                        return Err(de::Error::custom("duplicate member"));
                    }
                }
                Ok(Value::Object(out))
            }
            fn visit_seq<S: de::SeqAccess<'de>>(self, mut s: S) -> Result<Value, S::Error> {
                let mut out = Vec::new();
                while let Some(Json(v)) = s.next_element()? {
                    out.push(v);
                }
                Ok(Value::Array(out))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.into()))
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(v.into())
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                Ok(v.into())
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
                serde_json::Number::from_f64(v)
                    .map(Value::Number)
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
        }
        let v = d.deserialize_any(Visitor)?;
        validate(&v).map_err(de::Error::custom)?;
        Ok(Self(v))
    }
}
