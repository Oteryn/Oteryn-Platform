use super::{
    Error,
    game_wire::{decimal, digest, id, ordered, text},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, VerifyingKey};
use oteryn_internal_api::canonical::{self, Value};
use uuid::Uuid;

/// Expected values come from a current lock decision, never the signed payload.
#[derive(Debug, Clone)]
pub struct SnapshotBinding<'a> {
    pub source_authority: &'a str,
    pub listing: Uuid,
    pub seller: Uuid,
    pub character: Uuid,
    pub world: Uuid,
    pub lock: Uuid,
    pub lock_revision: u64,
    pub character_revision: u64,
    pub eligibility_revision: u64,
    pub projection_epoch: u64,
    pub now: u64,
}
/// Verifies bytes with a provisioned current/retiring key. Callers must recheck
/// trust before publication. This is not ownership or settlement authority.
/// Optional tabs stay unsupported until transfer-survival semantics are qualified.
pub fn verify_snapshot(
    raw: &[u8],
    e: &SnapshotBinding<'_>,
    kid: &str,
    key: &VerifyingKey,
) -> Result<String, Error> {
    let v = canonical::decode(raw, 262144, 12).map_err(|_| Error::Invalid)?;
    if raw.contains(&b'\\') || canonical::encode(&v).map_err(|_| Error::Invalid)? != raw {
        return Err(Error::Invalid);
    }
    ordered(
        &v,
        &["snapshot", "snapshot_hash", "alg", "kid", "signature"],
    )?;
    if text(&v, "alg")? != "Ed25519"
        || text(&v, "kid")? != kid
        || kid.is_empty()
        || kid.len() > 64
        || !kid.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b"._:/-".contains(&b))
        })
    {
        return Err(Error::Denied);
    }
    let s = v.get("snapshot").ok_or(Error::Invalid)?;
    ordered(
        s,
        &[
            "contract_version",
            "issuer_authority",
            "audience",
            "source_authority",
            "listing_id",
            "seller_account_id",
            "character_id",
            "world_id",
            "lock_id",
            "lock_revision",
            "character_revision",
            "eligibility_revision",
            "projection_epoch",
            "captured_at",
            "name",
            "vocation",
            "level",
            "experience",
            "created_at",
            "health",
            "health_max",
            "mana",
            "mana_max",
            "capacity_centi_oz",
            "speed",
            "blessings",
            "skills",
            "world_transfer_eligible",
            "appearance",
            "items",
            "store_grants",
            "outfits",
            "mounts",
            "achievements",
            "quests",
        ],
    )?;
    if s.get("contract_version") != Some(&Value::Unsigned(1))
        || text(s, "issuer_authority")? != "OTERYN_GAME_BAZAAR_AUTHORITY"
        || text(s, "audience")? != "OTERYN_PLATFORM_BAZAAR_AUTHORITY"
        || text(s, "source_authority")? != e.source_authority
    {
        return Err(Error::Denied);
    }
    for (field, want) in [
        ("listing_id", e.listing),
        ("seller_account_id", e.seller),
        ("character_id", e.character),
        ("world_id", e.world),
        ("lock_id", e.lock),
    ] {
        if id(text(s, field)?)? != want {
            return Err(Error::Denied);
        }
    }
    for (field, want) in [
        ("lock_revision", e.lock_revision),
        ("character_revision", e.character_revision),
        ("eligibility_revision", e.eligibility_revision),
        ("projection_epoch", e.projection_epoch),
    ] {
        if want == 0 || decimal(text(s, field)?, false, u64::MAX)? != want {
            return Err(Error::Denied);
        }
    }
    let captured = decimal(text(s, "captured_at")?, false, u64::MAX)?;
    if captured > e.now.saturating_add(1) {
        return Err(Error::Denied);
    }
    let name = text(s, "name")?;
    if !(2..=29).contains(&name.len())
        || name.starts_with(' ')
        || name.ends_with(' ')
        || name.contains("  ")
        || !name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
    {
        return Err(Error::Invalid);
    }
    if !matches!(s.get("vocation"), Some(Value::Unsigned(1..=6))) {
        return Err(Error::Invalid);
    }
    decimal(text(s, "level")?, false, u64::MAX)?;
    for field in [
        "experience",
        "health",
        "health_max",
        "mana",
        "mana_max",
        "capacity_centi_oz",
        "speed",
    ] {
        decimal(text(s, field)?, true, u64::MAX)?;
    }
    for (current, max) in [("health", "health_max"), ("mana", "mana_max")] {
        if decimal(text(s, current)?, true, u64::MAX)? > decimal(text(s, max)?, true, u64::MAX)? {
            return Err(Error::Invalid);
        }
    }
    if s.get("created_at") != Some(&Value::Null)
        && decimal(text(s, "created_at")?, false, u64::MAX)? > captured
    {
        return Err(Error::Invalid);
    }
    for field in [
        "appearance",
        "items",
        "store_grants",
        "outfits",
        "mounts",
        "achievements",
        "quests",
    ] {
        if s.get(field) != Some(&Value::Null) {
            return Err(Error::Unavailable);
        }
    }
    if !matches!(
        s.get("world_transfer_eligible"),
        Some(Value::Null | Value::Bool(_))
    ) {
        return Err(Error::Invalid);
    }
    let Some(Value::Array(blessings)) = s.get("blessings") else {
        return Err(Error::Invalid);
    };
    if !blessings.is_empty() {
        return Err(Error::Unavailable);
    }
    let Some(Value::Array(skills)) = s.get("skills") else {
        return Err(Error::Invalid);
    };
    if skills.len() > 16 {
        return Err(Error::Invalid);
    }
    let mut previous = "";
    for skill in skills {
        ordered(skill, &["key", "level", "progress_bps"])?;
        let name = text(skill, "key")?;
        if name <= previous
            || !matches!(
                name,
                "fist" | "club" | "sword" | "axe" | "distance" | "shielding" | "fishing" | "magic"
            )
        {
            return Err(Error::Invalid);
        }
        previous = name;
        decimal(text(skill, "level")?, true, u64::MAX)?;
        if !matches!(skill.get("progress_bps"), Some(Value::Unsigned(0..=10000))) {
            return Err(Error::Invalid);
        }
    }
    let bytes = canonical::encode(s).map_err(|_| Error::Invalid)?;
    let actual = digest(&bytes);
    if actual != text(&v, "snapshot_hash")? {
        return Err(Error::Denied);
    }
    let signature = text(&v, "signature")?;
    let decoded = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| Error::Invalid)?;
    if signature.len() != 86 || URL_SAFE_NO_PAD.encode(&decoded) != signature {
        return Err(Error::Invalid);
    }
    let signature = Signature::from_slice(&decoded).map_err(|_| Error::Invalid)?;
    let message = format!(
        "oteryn-v2-bazaar-character-snapshot-v1.{}",
        URL_SAFE_NO_PAD.encode(bytes)
    );
    key.verify_strict(message.as_bytes(), &signature)
        .map_err(|_| Error::Denied)?;
    Ok(actual)
}
