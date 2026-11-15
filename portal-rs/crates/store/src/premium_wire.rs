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

/// Profile-B B1 body bound (wire contract §8.2).
pub const MAX_AUTHORITY_BYTES: usize = 8192;
/// Profile-B B2 page budget (wire contract §1.6).
pub const MAX_CHANGE_PAGE_BYTES: usize = 262144;
/// Maximum lease L of `oteryn.profile_b.lease.v1` (§9.1).
const PROFILE_B_LEASE_SECONDS: i64 = 3600;

/// B1 authority answer in §8.2 member order, one item per configured product in
/// list order, classified at `issued` and cut off per §9.2.
pub fn authority(
    items: &[(
        &super::profile_b::Product,
        Option<&super::profile_b::Record>,
    )],
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
        || items.len() > super::profile_b::MAX_PRODUCTS
    {
        return Err(Error::Invalid);
    }
    let full = issued
        .checked_add(PROFILE_B_LEASE_SECONDS)
        .ok_or(Error::Unavailable)?;
    let mut entitlements = Vec::with_capacity(items.len());
    for (product, record) in items {
        if let Some(record) = record {
            record.validate()?;
        }
        let state = record.map_or("NONE", |r| r.classify(issued));
        let mut cutoff = full;
        if matches!(state, "ACTIVE" | "NOT_YET_EFFECTIVE")
            && let Some(until) = record.and_then(|r| r.until)
        {
            cutoff = cutoff.min(until);
        }
        if cutoff <= issued {
            return Err(Error::Unavailable);
        }
        let refresh = issued + 2 * (cutoff - issued) / 3;
        let optional_time = |value: Option<i64>| value.map(timestamp).transpose();
        entitlements.push(Value::Object(vec![
            ("product_id".into(), text(product.id.as_str())),
            ("product_version".into(), Value::Signed(product.version)),
            (
                "entitlement_id".into(),
                record.map_or(Value::Null, |r| text(r.id.to_string())),
            ),
            ("entitlement_state".into(), text(state)),
            (
                "lifecycle_revision".into(),
                Value::Signed(record.map_or(0, |r| r.revision)),
            ),
            (
                "effective_from".into(),
                optional_time(record.map(|r| r.from))?.unwrap_or(Value::Null),
            ),
            (
                "effective_until".into(),
                optional_time(record.and_then(|r| r.until))?.unwrap_or(Value::Null),
            ),
            ("authority_valid_until".into(), timestamp(cutoff)?),
            ("refresh_after".into(), timestamp(refresh)?),
            ("lease_policy".into(), text(super::profile_b::LEASE_POLICY)),
        ]));
    }
    let value = Value::Object(vec![
        ("schema".into(), text("oteryn.entitlement_authority.v1")),
        ("producer_revision".into(), text(revision)),
        (
            "producer_profile".into(),
            text("oteryn.entitlement.profile_b.v1"),
        ),
        ("nonce".into(), text(nonce)),
        ("account_id".into(), text(account.to_string())),
        ("authority_revision".into(), Value::Signed(authority)),
        ("authority_issued_at".into(), timestamp(issued)?),
        ("entitlements".into(), Value::Array(entitlements)),
    ]);
    let bytes = canonical::encode(&value).map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_AUTHORITY_BYTES || bytes.contains(&b'\\') {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}

/// B2 page in §8.4 member order. Items arrive in ascending position; the page
/// stops before the byte budget would be exceeded and its cursor names the last
/// item returned, or is `null` on an empty page.
pub fn change_page(items: &[(i64, super::profile_b::ChangeItem)]) -> Result<Vec<u8>, Error> {
    // Reserved for the envelope and the longest cursor.
    let mut used = 128_usize;
    let mut values = Vec::with_capacity(items.len());
    let mut last = None;
    for (position, item) in items {
        let value = Value::Object(vec![
            ("account_id".into(), text(item.account_id.to_string())),
            ("product_id".into(), text(item.product_id.as_str())),
            (
                "product_version".into(),
                Value::Signed(item.product_version),
            ),
            (
                "lifecycle_revision".into(),
                Value::Signed(item.lifecycle_revision),
            ),
        ]);
        let size = canonical::encode(&value)
            .map_err(|_| Error::Unavailable)?
            .len();
        if used + size + 1 > MAX_CHANGE_PAGE_BYTES {
            break;
        }
        used += size + 1;
        values.push(value);
        last = Some(*position);
    }
    let bytes = canonical::encode(&Value::Object(vec![
        ("schema".into(), text("oteryn.entitlement_change_page.v1")),
        ("items".into(), Value::Array(values)),
        (
            "next_cursor".into(),
            last.map_or(Value::Null, |n| text(format!("v1:{n}"))),
        ),
    ]))
    .map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_CHANGE_PAGE_BYTES || bytes.contains(&b'\\') {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
