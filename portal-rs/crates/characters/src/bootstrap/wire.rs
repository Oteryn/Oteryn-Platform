use super::Error;
use oteryn_internal_api::canonical::{self, Value};
use sha2::{Digest, Sha256};
use sqlx::{Row, postgres::PgRow};
use subtle::ConstantTimeEq;
use uuid::Uuid;

const FIELDS: &[&str] = &[
    "contract_version",
    "variant",
    "issuer_authority",
    "issuer_decision_id",
    "source_revision",
    "operation_id",
    "operation",
    "account_id",
    "target_world_id",
    "requested_name",
    "requested_sex",
    "interpretation_context",
    "issued_at_source",
    "expires_at_source",
    "audience",
];
const CONTEXT: &[&str] = &[
    "profile_revision",
    "ruleset_revision",
    "content_revision",
    "starter_template_revision",
];

fn keys(value: &Value, fields: &[&str]) -> Result<(), Error> {
    match value {
        Value::Object(members)
            if members.len() == fields.len()
                && members.iter().all(|(k, _)| fields.contains(&k.as_str())) =>
        {
            Ok(())
        }
        _ => Err(Error::Unavailable),
    }
}
pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, Error> {
    match value.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::Unavailable),
    }
}
pub(super) fn uuid(raw: &str) -> Result<(), Error> {
    let value = Uuid::parse_str(raw).map_err(|_| Error::Unavailable)?;
    if value.to_string() != raw
        || !(1..=8).contains(&value.get_version_num())
        || value.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(super) fn decimal(raw: &str, min: i64) -> Result<i64, Error> {
    let value = raw.parse::<i64>().map_err(|_| Error::Unavailable)?;
    if value < min || value.to_string() != raw {
        return Err(Error::Unavailable);
    }
    Ok(value)
}
/// Game naming policy revision 1: 2..29 ASCII letters in single-spaced words.
pub(super) fn name(value: &str) -> bool {
    (2..=29).contains(&value.len())
        && value
            .split(' ')
            .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()))
}
/// Interpretation revision: 1-128 bytes of `[A-Za-z0-9][A-Za-z0-9._:-]*`.
pub(super) fn revision(value: &str) -> bool {
    value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
pub(super) fn request(value: &Value) -> Result<&str, Error> {
    let parse = || {
        keys(value, &["contract_version", "operation_id"])?;
        if value.get("contract_version") != Some(&Value::Unsigned(3)) {
            return Err(Error::Unavailable);
        }
        let operation = string(value, "operation_id")?;
        uuid(operation)?;
        Ok(operation)
    };
    parse().map_err(|_| Error::BadInput)
}
pub(super) fn stored(row: &PgRow, raw: &[u8]) -> Result<Value, Error> {
    let digest = format!("{:x}", Sha256::digest(raw));
    let stored: String = row.try_get("intent_sha256")?;
    if !bool::from(digest.as_bytes().ct_eq(stored.as_bytes())) {
        return Err(Error::Unavailable);
    }
    let value = canonical::decode(raw, 4096, 3).map_err(|_| Error::Unavailable)?;
    keys(&value, FIELDS)?;
    if value.get("contract_version") != Some(&Value::Unsigned(3)) {
        return Err(Error::Unavailable);
    }
    for (key, expected) in [
        ("variant", "OPERATOR_CONTROL_PLANE_BOOTSTRAP"),
        ("issuer_authority", "OTERYN_PLATFORM_CHARACTER_AUTHORITY"),
        ("operation", "INITIAL_CHARACTER_BOOTSTRAP"),
        ("audience", "OTERYN_GAME_CHARACTER_AUTHORITY"),
    ] {
        if string(&value, key)? != expected {
            return Err(Error::Unavailable);
        }
    }
    for key in ["operation_id", "issuer_decision_id", "target_world_id"] {
        uuid(string(&value, key)?)?;
    }
    crate::lcfa::AccountId::parse(string(&value, "account_id")?).map_err(|_| Error::Unavailable)?;
    if !name(string(&value, "requested_name")?) {
        return Err(Error::Unavailable);
    }
    if !matches!(string(&value, "requested_sex")?, "Female" | "Male") {
        return Err(Error::Unavailable);
    }
    for key in [
        "operation_id",
        "issuer_decision_id",
        "account_id",
        "target_world_id",
        "requested_name",
        "requested_sex",
        "source_revision",
        "issued_at_source",
        "expires_at_source",
    ] {
        if row.try_get::<String, _>(key)? != string(&value, key)? {
            return Err(Error::Unavailable);
        }
    }
    let context = value
        .get("interpretation_context")
        .ok_or(Error::Unavailable)?;
    keys(context, CONTEXT)?;
    for key in CONTEXT {
        let given = string(context, key)?;
        if !revision(given) || row.try_get::<String, _>(*key)? != given {
            return Err(Error::Unavailable);
        }
    }
    decimal(string(&value, "source_revision")?, 1)?;
    let issued = decimal(string(&value, "issued_at_source")?, 0)?;
    let expires = decimal(string(&value, "expires_at_source")?, 0)?;
    if expires <= issued || expires - issued > 300 {
        return Err(Error::Unavailable);
    }
    // Issuance uses compact ordered JSON; never silently repair corrupt storage.
    if canonical::encode(&value).map_err(|_| Error::Unavailable)? != raw {
        return Err(Error::Unavailable);
    }
    Ok(value)
}
