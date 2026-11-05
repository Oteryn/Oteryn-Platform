use super::*;
pub(super) const ISSUER: &str = "OTERYN_PLATFORM_STORE_AUTHORITY";
pub(super) const HEAD: &str = "contract_version operation source_authority";
pub(super) fn decode(raw: &[u8], max: usize) -> Result<Value, Error> {
    let v = canonical::decode(raw, max, 4).map_err(|_| Error::Invalid)?;
    if raw.contains(&b'\\') || canonical::encode(&v).map_err(|_| Error::Invalid)? != raw {
        return Err(Error::Invalid);
    }
    Ok(v)
}
pub(super) fn exact(v: &Value, names: &str) -> Result<(), Error> {
    let Value::Object(fields) = v else {
        return Err(Error::Invalid);
    };
    if fields
        .iter()
        .map(|(k, _)| k.as_str())
        .ne(names.split_whitespace())
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn get<'a>(v: &'a Value, key: &str) -> Result<&'a str, Error> {
    match v.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::Invalid),
    }
}
pub(super) fn integer(v: &Value, key: &str, min: u64, max: u64) -> Result<u64, Error> {
    match v.get(key) {
        Some(Value::Unsigned(n)) if (min..=max).contains(n) => Ok(*n),
        _ => Err(Error::Invalid),
    }
}
pub(super) fn counter(s: &str, zero: bool) -> Result<u64, Error> {
    if s == "0" && zero {
        return Ok(0);
    }
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid);
    }
    s.parse().map_err(|_| Error::Invalid)
}
pub(super) fn uuid(s: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(s).map_err(|_| Error::Invalid)?;
    if id.to_string() != s
        || id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    Ok(id)
}
fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn head(v: &Value, operation: &str) -> Result<(), Error> {
    let Value::Object(fields) = v else {
        return Err(Error::Invalid);
    };
    if fields.len() < 3
        || fields[..3]
            .iter()
            .map(|(k, _)| k.as_str())
            .ne(HEAD.split_whitespace())
        || v.get("contract_version") != Some(&number(1))
        || get(v, "operation")? != operation
    {
        return Err(Error::Invalid);
    }
    let source = get(v, "source_authority")?;
    if !(1..=128).contains(&source.len())
        || !source
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(super) fn key(v: &Value) -> Result<(&str, Uuid), Error> {
    let kind = get(v, "operation_kind")?;
    if get(v, "issuer_authority")? != ISSUER
        || !matches!(
            kind,
            "DELIVER" | "CLAIM" | "REVOKE_UNCLAIMED" | "RECLAIM" | "FLAG"
        )
    {
        return Err(Error::Invalid);
    }
    Ok((kind, uuid(get(v, "operation_id")?)?))
}
pub(super) fn page(v: &Value) -> Result<(u64, Uuid, i64), Error> {
    exact(
        v,
        &format!("{HEAD} after_source_revision after_operation_id max_entries"),
    )?;
    let revision = counter(get(v, "after_source_revision")?, true)?;
    let after = get(v, "after_operation_id")?;
    let id = if after == Uuid::nil().to_string() {
        Uuid::nil()
    } else {
        uuid(after)?
    };
    if (revision == 0) != id.is_nil() {
        return Err(Error::Invalid);
    }
    Ok((revision, id, integer(v, "max_entries", 1, 32)? as i64))
}
pub(super) fn receipt(v: &Value) -> Result<(), Error> {
    let (kind, _) = key(v)?;
    let outcome = get(v, "outcome")?;
    let committed = outcome == "COMMITTED";
    if !committed && outcome != "REJECTED" {
        return Err(Error::Invalid);
    }
    let extra = if committed {
        match kind {
            "DELIVER" => "delivered_quantity",
            "CLAIM" => "character_id world_id minted_quantity minted_units_digest",
            "REVOKE_UNCLAIMED" => "revoked_quantity",
            "RECLAIM" => "character_id world_id reclaimed_quantity shortfall_quantity",
            _ => "",
        }
    } else {
        ""
    };
    exact(
        v,
        &format!(
            "{HEAD} issuer_authority operation_kind operation_id binding_digest outcome result_code account_id entitlement_id product_id product_version {extra} recovery_generation decided_at"
        ),
    )?;
    let code = get(v, "result_code")?;
    let allowed = if committed {
        if kind == "RECLAIM" {
            matches!(code, "STORE_RECLAIMED" | "STORE_RECLAIM_SHORTFALL")
        } else {
            code == "STORE_OK"
        }
    } else {
        match code {
            "STORE_INTENT_INVALID" | "STORE_OPERATION_CONFLICT" | "STORE_UNSUPPORTED" => true,
            "STORE_ACCOUNT_NOT_FOUND" => kind == "DELIVER",
            "STORE_GRANT_UNKNOWN" => matches!(kind, "DELIVER" | "CLAIM"),
            "STORE_LINE_REVOKED" => kind == "CLAIM",
            "STORE_LINE_ALREADY_CLAIMED" => kind == "REVOKE_UNCLAIMED",
            "STORE_LINE_NOT_FOUND" => kind != "DELIVER",
            _ => false,
        }
    };
    if !allowed || !hex(get(v, "binding_digest")?) {
        return Err(Error::Invalid);
    }
    for field in ["account_id", "entitlement_id"] {
        uuid(get(v, field)?)?;
    }
    for field in ["product_version", "recovery_generation", "decided_at"] {
        counter(get(v, field)?, false)?;
    }
    let product = get(v, "product_id")?;
    let parts: Vec<_> = product.split('.').collect();
    if product.len() > 64
        || !(2..=4).contains(&parts.len())
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 32
                || !p.as_bytes()[0].is_ascii_lowercase()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
    {
        return Err(Error::Invalid);
    }
    if committed {
        for field in extra.split_whitespace() {
            match field {
                "character_id" | "world_id" => {
                    uuid(get(v, field)?)?;
                }
                "minted_units_digest" => {
                    if !hex(get(v, field)?) {
                        return Err(Error::Invalid);
                    }
                }
                _ => {
                    integer(v, field, if kind == "RECLAIM" { 0 } else { 1 }, 65535)?;
                }
            }
        }
        if kind == "RECLAIM"
            && (code == "STORE_RECLAIMED") != (integer(v, "shortfall_quantity", 0, 65535)? == 0)
        {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}
pub(super) fn assert_binding(v: &Value, intent: &Value, quantity: i32) -> Result<(), Error> {
    for field in [
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
    ] {
        if v.get(field) != intent.get(field) {
            return Err(Error::Invalid);
        }
    }
    if get(v, "outcome")? == "COMMITTED" {
        let field = match get(v, "operation_kind")? {
            "DELIVER" => Some("delivered_quantity"),
            "CLAIM" => Some("minted_quantity"),
            "REVOKE_UNCLAIMED" => Some("revoked_quantity"),
            _ => None,
        };
        if let Some(field) = field {
            let expected = if field == "revoked_quantity" {
                u64::try_from(quantity).map_err(|_| Error::Unavailable)?
            } else {
                integer(intent, "quantity", 1, 65535)?
            };
            if integer(v, field, 1, 65535)? != expected {
                return Err(Error::Invalid);
            }
        }
    }
    Ok(())
}
