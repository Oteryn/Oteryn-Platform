//! Premium wire in contract member order. Requests are bounded at 256 bytes;
//! contract section 5 bounds the sixteen-member snapshot at 1024 bytes.
use super::{Entitlement, Error, MAX_REVISION, http::hex};
use oteryn_internal_api::canonical::{self, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub const MAX_RESPONSE_BYTES: usize = 1024;
fn text(value: impl Into<String>) -> Value {
    Value::String(value.into())
}
fn timestamp(value: i64) -> Result<Value, Error> {
    let date = OffsetDateTime::from_unix_timestamp(value).map_err(|_| Error::Unavailable)?;
    if !(0..=9999).contains(&date.year()) {
        return Err(Error::Unavailable);
    }
    Ok(text(date.format(&Rfc3339).map_err(|_| Error::Unavailable)?))
}
pub fn snapshot(
    entitlement: Option<&Entitlement>,
    issued: i64,
    authority: i64,
    account: Uuid,
    nonce: &str,
    revision: &str,
) -> Result<Vec<u8>, Error> {
    if !hex(nonce, 32)
        || !hex(revision, 40)
        || !(1..=MAX_REVISION).contains(&authority)
        || account.get_version_num() != 7
        || account.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    if let Some(entitlement) = entitlement {
        entitlement.validate()?;
    }
    let state = entitlement.map_or("NONE", |e| e.classify(issued));
    let mut cutoff = issued.checked_add(3600).ok_or(Error::Unavailable)?;
    if let Some(entitlement) =
        entitlement.filter(|_| matches!(state, "ACTIVE" | "NOT_YET_EFFECTIVE"))
    {
        cutoff = cutoff.min(entitlement.until);
    }
    if cutoff <= issued {
        return Err(Error::Unavailable);
    }
    let refresh = issued + 2 * (cutoff - issued) / 3;
    let value = Value::Object(vec![
        ("schema".into(), text("oteryn.premium_snapshot.v1")),
        ("producer_revision".into(), text(revision)),
        (
            "producer_profile".into(),
            text("oteryn.entitlement.profile_b.v1"),
        ),
        ("nonce".into(), text(nonce)),
        ("account_id".into(), text(account.to_string())),
        ("product_id".into(), text("oteryn.premium_time")),
        ("product_version".into(), Value::Signed(1)),
        (
            "entitlement_id".into(),
            entitlement.map_or(Value::Null, |e| text(e.id.to_string())),
        ),
        ("entitlement_state".into(), text(state)),
        (
            "lifecycle_revision".into(),
            Value::Signed(entitlement.map_or(0, |e| e.revision)),
        ),
        ("authority_revision".into(), Value::Signed(authority)),
        (
            "effective_from".into(),
            entitlement
                .map(|e| timestamp(e.from))
                .transpose()?
                .unwrap_or(Value::Null),
        ),
        (
            "effective_until".into(),
            entitlement
                .map(|e| timestamp(e.until))
                .transpose()?
                .unwrap_or(Value::Null),
        ),
        ("authority_issued_at".into(), timestamp(issued)?),
        ("authority_valid_until".into(), timestamp(cutoff)?),
        ("refresh_after".into(), timestamp(refresh)?),
    ]);
    let bytes = canonical::encode(&value).map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
