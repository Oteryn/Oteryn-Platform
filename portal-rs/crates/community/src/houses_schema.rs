//! Exact G-4 house wire validation. The task's narrower house-key rule applies.
use oteryn_internal_api::canonical::{self, Value};
use unicode_normalization::UnicodeNormalization;
type Result<T> = std::result::Result<T, ()>;
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    match v.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(()),
    }
}
fn keys(v: &Value, wanted: &[&str]) -> Result<()> {
    match v {
        Value::Object(m) if m.iter().map(|(k, _)| k.as_str()).eq(wanted.iter().copied()) => Ok(()),
        _ => Err(()),
    }
}
pub(super) fn decimal(v: &Value, key: &str, min: u64, max: u64) -> Result<u64> {
    let s = text(v, key)?;
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(());
    }
    let n = s.parse::<u64>().map_err(|_| ())?;
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(())
    }
}
pub(super) fn world(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'7',
            19 => b"89ab".contains(&b),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}
pub(super) fn house_key(s: &str) -> bool {
    s.len() <= 128
        && s.strip_prefix("oteryn:content.house.").is_some_and(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}
fn name(v: &Value, k: &str) -> Result<()> {
    let s = text(v, k)?;
    let forbidden = regex::Regex::new(r"[\p{Cc}\p{Cf}]").map_err(|_| ())?;
    if s.is_empty()
        || s.len() > 64
        || [".", ".."].contains(&s)
        || s.contains(['"', '\\', '/'])
        || forbidden.is_match(s)
        || s.nfc().collect::<String>() != s
    {
        Err(())
    } else {
        Ok(())
    }
}
fn choice(v: &Value, k: &str, options: &[&str]) -> Result<()> {
    if options.contains(&text(v, k)?) {
        Ok(())
    } else {
        Err(())
    }
}
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(super) fn validate(v: &Value, watermark: bool, now: u64) -> Result<()> {
    if !matches!(v.get("contract_version"), Some(Value::Unsigned(2))) {
        return Err(());
    }
    if !authority(text(v, "source_authority")?) {
        return Err(());
    }
    decimal(v, "projection_epoch", 1, u64::MAX)?;
    if watermark {
        keys(
            v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "family",
                "projection_epoch",
                "complete_through",
                "observed_at",
            ],
        )?;
        choice(v, "operation", &["PublishPublicProjectionWatermarkV2"])?;
        choice(v, "family", &["house"])?;
        let observed = decimal(v, "observed_at", 0, i64::MAX as u64)?;
        decimal(v, "complete_through", 0, observed)?;
        if observed > now.saturating_add(1) {
            return Err(());
        }
        return Ok(());
    }
    keys(
        v,
        &[
            "contract_version",
            "operation",
            "source_authority",
            "world_id",
            "house_key",
            "projection_epoch",
            "projection_revision",
            "source_observed_at",
            "house",
        ],
    )?;
    choice(v, "operation", &["PublishHouseV2"])?;
    if !world(text(v, "world_id")?) || !house_key(text(v, "house_key")?) {
        return Err(());
    }
    decimal(v, "projection_revision", 1, u64::MAX)?;
    if decimal(v, "source_observed_at", 0, i64::MAX as u64)? > now.saturating_add(1) {
        return Err(());
    }
    let Some(Value::Array(records)) = v.get("house") else {
        return Err(());
    };
    if records.len() > 1 {
        return Err(());
    }
    for h in records {
        let status = text(h, "status")?;
        let mut fields = vec![
            "name",
            "kind",
            "town",
            "size_sqm",
            "beds",
            "rent_gold",
            "status",
        ];
        match status {
            "VACANT" | "UNAVAILABLE" => {}
            "AUCTION" => {
                fields.extend(["current_bid", "auction_ends_at"]);
                decimal(h, "current_bid", 0, i64::MAX as u64)?;
                decimal(h, "auction_ends_at", 1, i64::MAX as u64)?;
            }
            "OWNED" | "MOVE_OUT_PENDING" => {
                fields.extend(["owner_kind", "owner_name", "paid_until"]);
                choice(h, "owner_kind", &["CHARACTER", "GUILD"])?;
                name(h, "owner_name")?;
                decimal(h, "paid_until", 1, i64::MAX as u64)?;
                if status == "MOVE_OUT_PENDING" {
                    fields.push("move_out_at");
                    decimal(h, "move_out_at", 1, i64::MAX as u64)?;
                }
            }
            _ => return Err(()),
        }
        keys(h, &fields)?;
        name(h, "name")?;
        choice(h, "kind", &["private_house", "shop", "guildhall"])?;
        let town = text(h, "town")?;
        if town.is_empty()
            || town.len() > 128
            || !town
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
        {
            return Err(());
        }
        decimal(h, "size_sqm", 1, u64::MAX)?;
        decimal(h, "beds", 0, u64::MAX)?;
        decimal(h, "rent_gold", 1, i64::MAX as u64)?;
    }
    Ok(())
}
pub(super) fn json(v: &Value) -> Result<serde_json::Value> {
    serde_json::from_slice(&canonical::encode(v).map_err(|_| ())?).map_err(|_| ())
}
