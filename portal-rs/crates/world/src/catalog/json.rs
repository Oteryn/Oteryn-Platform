use serde::{Deserialize, Deserializer, de};
use serde_json::Value;
use std::{collections::HashSet, fmt};

/// Reject duplicate members at every depth before any map can discard them.
pub(super) struct Unique(pub Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded JSON with unique members and signed integers")
            }
            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Unique, M::Error> {
                let mut result = serde_json::Map::new();
                let mut seen = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if key.len() > 2048 || result.len() >= 4096 {
                        return Err(de::Error::custom("CATALOG_CAPACITY"));
                    }
                    if key.is_empty() || !seen.insert(key.clone()) {
                        return Err(de::Error::custom("invalid JSON object"));
                    }
                    result.insert(key, map.next_value::<Unique>()?.0);
                }
                Ok(Unique(Value::Object(result)))
            }
            fn visit_seq<S: de::SeqAccess<'de>>(self, mut seq: S) -> Result<Unique, S::Error> {
                let mut result = Vec::new();
                while let Some(value) = seq.next_element::<Unique>()? {
                    if result.len() >= 1_000_000 {
                        return Err(de::Error::custom("CATALOG_CAPACITY"));
                    }
                    result.push(value.0);
                }
                Ok(Unique(Value::Array(result)))
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<Unique, E> {
                if s.len() > 2048 {
                    return Err(E::custom("CATALOG_CAPACITY"));
                }
                Ok(Unique(Value::String(s.into())))
            }
            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Unique, E> {
                Ok(Unique(n.into()))
            }
            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Unique, E> {
                let n = i64::try_from(n).map_err(|_| E::custom("signed integer limit"))?;
                Ok(Unique(n.into()))
            }
            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Unique, E> {
                Ok(Unique(b.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
        }
        d.deserialize_any(Visitor)
    }
}
/// Explicit ordering is independent of serde_json's feature-unified map backend.
pub(super) fn canonical(value: &Value) -> Result<Vec<u8>, super::Error> {
    fn write(value: &Value, output: &mut Vec<u8>) -> Result<(), super::Error> {
        match value {
            Value::Object(map) => {
                output.push(b'{');
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for (i, key) in keys.into_iter().enumerate() {
                    if i > 0 {
                        output.push(b',');
                    }
                    output.extend(serde_json::to_vec(key).map_err(|_| super::Error::InvalidInput)?);
                    output.push(b':');
                    write(&map[key], output)?;
                }
                output.push(b'}');
            }
            Value::Array(array) => {
                output.push(b'[');
                for (i, item) in array.iter().enumerate() {
                    if i > 0 {
                        output.push(b',');
                    }
                    write(item, output)?;
                }
                output.push(b']');
            }
            _ => output.extend(serde_json::to_vec(value).map_err(|_| super::Error::InvalidInput)?),
        }
        Ok(())
    }
    let mut output = Vec::new();
    write(value, &mut output)?;
    Ok(output)
}
