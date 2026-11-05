use super::*;
use oteryn_internal_api::canonical::{self, Value};
pub(super) fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
pub(super) fn text(s: impl ToString) -> Value {
    Value::String(s.to_string())
}
pub(super) fn get<'a>(v: &'a Value, key: &str) -> Result<&'a str, Error> {
    match v.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::BadInput),
    }
}
pub(super) fn uuid(s: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(s).map_err(|_| Error::BadInput)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != s
    {
        return Err(Error::BadInput);
    }
    Ok(id)
}
pub(super) fn counter(s: &str, zero: bool) -> Result<u64, Error> {
    if s.is_empty()
        || !s.bytes().all(|b| b.is_ascii_digit())
        || (s.starts_with('0') && !(zero && s == "0"))
    {
        return Err(Error::BadInput);
    }
    s.parse().map_err(|_| Error::BadInput)
}
pub(super) fn name(s: &str) -> Result<(), Error> {
    if !(2..=29).contains(&s.len())
        || s.split(' ')
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphabetic()))
    {
        return Err(Error::BadInput);
    }
    Ok(())
}
pub(super) fn authority(s: &str) -> Result<(), Error> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
    {
        return Err(Error::BadInput);
    }
    Ok(())
}
pub(super) fn command(s: &str) -> Result<(), Error> {
    if s.is_empty() || s.len() > 64 || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(Error::BadInput);
    }
    Ok(())
}
pub(super) fn exact(v: &Value, keys: &[&str]) -> Result<(), Error> {
    match v {
        Value::Object(fields)
            if fields.len() == keys.len()
                && fields
                    .iter()
                    .zip(keys)
                    .all(|((k, _), expected)| k == expected) =>
        {
            Ok(())
        }
        _ => Err(Error::BadInput),
    }
}
pub(super) fn decode(raw: &[u8], max: usize) -> Result<Value, Error> {
    let v = canonical::decode(raw, max, 4).map_err(|_| Error::BadInput)?;
    if raw.contains(&b'\\') || canonical::encode(&v).map_err(|_| Error::BadInput)? != raw {
        return Err(Error::BadInput);
    }
    Ok(v)
}
pub(super) fn encode(v: &Value) -> Result<String, Error> {
    String::from_utf8(canonical::encode(v).map_err(|_| Error::BadInput)?)
        .map_err(|_| Error::BadInput)
}
pub(super) fn binding(intent: &Intent) -> Value {
    match intent {
        Intent::Create {
            account_id,
            world_id,
            name,
        } => object(vec![
            ("account_id", text(account_id)),
            ("target_world_id", text(world_id)),
            ("requested_name", text(name)),
        ]),
        Intent::Transfer {
            character_id,
            from_account_id,
            to_account_id,
            purpose,
        } => object(vec![
            ("character_id", text(character_id)),
            ("expected_from_account_id", text(from_account_id)),
            ("to_account_id", text(to_account_id)),
            ("purpose", text(purpose.wire())),
        ]),
    }
}
pub(super) fn intent(
    command: &str,
    decision: Uuid,
    revision: i64,
    operation: Uuid,
    binding: Value,
    issued: i64,
    expires: i64,
) -> Result<String, Error> {
    let mut fields = vec![
        ("contract_version".into(), Value::Unsigned(1)),
        ("command".into(), text(command)),
        (
            "variant".into(),
            text(if command == CREATE {
                "PLATFORM_USER_CREATE"
            } else {
                "PLATFORM_BAZAAR_TRANSFER"
            }),
        ),
        ("issuer_authority".into(), text(ISSUER)),
        ("issuer_decision_id".into(), text(decision)),
        ("source_revision".into(), text(revision)),
        ("operation_id".into(), text(operation)),
    ];
    if let Value::Object(binding) = binding {
        fields.extend(binding);
    }
    fields.extend([
        ("issued_at_source".into(), text(issued)),
        ("expires_at_source".into(), text(expires)),
        ("audience".into(), text("OTERYN_GAME_CHARACTER_AUTHORITY")),
    ]);
    let raw = encode(&Value::Object(fields))?;
    if raw.len() > 4096 {
        return Err(Error::BadInput);
    }
    Ok(raw)
}
pub(super) fn head(v: &Value, operation: &str) -> Result<(), Error> {
    if v.get("contract_version") != Some(&Value::Unsigned(1)) || get(v, "operation")? != operation {
        return Err(Error::BadInput);
    }
    authority(get(v, "source_authority")?)
}
/// Validate exact ordered receipt members and the closed command-specific code set.
pub(super) fn receipt(raw: &[u8]) -> Result<Value, Error> {
    let v = decode(raw, 1024)?;
    head(&v, "PublishCharacterCommandReceiptV1")?;
    let cmd = get(&v, "command")?;
    command(cmd)?;
    uuid(get(&v, "operation_id")?)?;
    let outcome = get(&v, "outcome")?;
    let code = get(&v, "result_code")?;
    let mut keys = vec![
        "contract_version",
        "operation",
        "source_authority",
        "command",
        "operation_id",
        "outcome",
        "result_code",
    ];
    let members: &[&str] = if outcome == "COMMITTED" && code == "CHAR_CMD_OK" {
        match cmd {
            CREATE => &[
                "character_id",
                "account_id",
                "world_id",
                "name",
                "character_revision",
                "projection_epoch",
                "projection_revision",
            ],
            TRANSFER => &[
                "character_id",
                "from_account_id",
                "to_account_id",
                "character_revision",
                "projection_epoch",
                "from_projection_revision",
                "to_projection_revision",
            ],
            _ => return Err(Error::BadInput),
        }
    } else if outcome == "REJECTED" && rejection(cmd, code) {
        &[]
    } else {
        return Err(Error::BadInput);
    };
    keys.extend(members);
    keys.extend(["recovery_generation", "decided_at"]);
    exact(&v, &keys)?;
    for key in members {
        let s = get(&v, key)?;
        if key.ends_with("_id") {
            uuid(s)?;
        } else if *key == "name" {
            name(s)?;
        } else {
            counter(s, false)?;
        }
    }
    counter(get(&v, "recovery_generation")?, false)?;
    counter(get(&v, "decided_at")?, false)?;
    Ok(v)
}
fn rejection(cmd: &str, code: &str) -> bool {
    match code {
        "CHAR_CMD_UNSUPPORTED" => true,
        "CHAR_CMD_NAME_INVALID" | "CHAR_CMD_NAME_UNAVAILABLE" | "CHAR_CMD_WORLD_NOT_ACCEPTED" => {
            cmd == CREATE
        }
        "CHAR_CMD_CHARACTER_NOT_FOUND"
        | "CHAR_CMD_OWNER_MISMATCH"
        | "CHAR_CMD_CHARACTER_IN_SESSION"
        | "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"
        | "CHAR_CMD_HOUSING_NOT_PREPARED"
        | "CHAR_CMD_SAME_ACCOUNT" => cmd == TRANSFER,
        "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT"
        | "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"
        | "CHAR_CMD_INTENT_EXPIRED"
        | "CHAR_CMD_INTENT_INVALID"
        | "CHAR_CMD_OPERATION_CONFLICT" => cmd == CREATE || cmd == TRANSFER,
        _ => false,
    }
}
