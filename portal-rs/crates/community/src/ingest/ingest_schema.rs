use super::{FAMILIES, Family, Publication};
use oteryn_internal_api::canonical::Value;
use unicode_normalization::UnicodeNormalization;
type Result<T> = std::result::Result<T, ()>;
fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value> {
    v.get(k).ok_or(())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    match field(v, k)? {
        Value::String(s) => Ok(s),
        _ => Err(()),
    }
}
fn keys(v: &Value, expected: &[&str]) -> Result<()> {
    match v {
        Value::Object(m)
            if m.len() == expected.len()
                && m.iter().all(|(k, _)| expected.contains(&k.as_str())) =>
        {
            Ok(())
        }
        _ => Err(()),
    }
}
fn list<'a>(v: &'a Value, k: &str, min: usize, max: usize) -> Result<&'a [Value]> {
    match field(v, k)? {
        Value::Array(a) if (min..=max).contains(&a.len()) => Ok(a),
        _ => Err(()),
    }
}
fn integer(v: &Value, k: &str, min: u64, max: u64) -> Result<u64> {
    let n = match field(v, k)? {
        Value::Unsigned(n) => *n,
        Value::Signed(n) => u64::try_from(*n).map_err(|_| ())?,
        _ => return Err(()),
    };
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(())
    }
}
fn decimal<'a>(v: &'a Value, k: &str, zero: bool) -> Result<&'a str> {
    let s = text(v, k)?;
    if s == "0" && zero {
        return Ok(s);
    }
    if s.starts_with('0')
        || s.is_empty()
        || !s.bytes().all(|b| b.is_ascii_digit())
        || s.parse::<u64>().is_err()
    {
        Err(())
    } else {
        Ok(s)
    }
}
fn time(v: &Value, k: &str, zero: bool) -> Result<i64> {
    decimal(v, k, zero)?.parse().map_err(|_| ())
}
fn uuid<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    let s = text(v, k)?;
    let b = s.as_bytes();
    if b.len() != 36 {
        return Err(());
    }
    for (i, c) in b.iter().enumerate() {
        if [8, 13, 18, 23].contains(&i) {
            if *c != b'-' {
                return Err(());
            }
        } else if !c.is_ascii_digit() && !(b'a'..=b'f').contains(c) {
            return Err(());
        }
    }
    if b[14] != b'7' || !b"89ab".contains(&b[19]) {
        Err(())
    } else {
        Ok(s)
    }
}
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn content<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    let s = text(v, k)?;
    if !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
    {
        Ok(s)
    } else {
        Err(())
    }
}
fn name_value(v: &Value) -> Result<&str> {
    let Value::String(s) = v else { return Err(()) };
    static FORBIDDEN: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> =
        std::sync::OnceLock::new();
    let forbidden = FORBIDDEN
        .get_or_init(|| regex::Regex::new(r"[\p{Cc}\p{Cf}]"))
        .as_ref()
        .map_err(|_| ())?;
    if s.is_empty()
        || s.len() > 64
        || [".", ".."].contains(&s.as_str())
        || s.contains(['"', '\\', '/'])
        || forbidden.is_match(s)
        || s.nfc().collect::<String>() != *s
    {
        Err(())
    } else {
        Ok(s)
    }
}
fn name<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    name_value(field(v, k)?)
}
fn boolean(v: &Value, k: &str) -> Result<()> {
    if matches!(field(v, k)?, Value::Bool(_)) {
        Ok(())
    } else {
        Err(())
    }
}
fn choice<'a>(v: &'a Value, k: &str, values: &[&str]) -> Result<&'a str> {
    let s = text(v, k)?;
    if values.contains(&s) { Ok(s) } else { Err(()) }
}
pub(super) fn publication(family: Option<Family>, v: &Value, now: u64) -> Result<Publication> {
    integer(v, "contract_version", 1, 1)?;
    let source = text(v, "source_authority")?;
    if !authority(source) {
        return Err(());
    }
    let epoch = decimal(v, "projection_epoch", false)?.to_owned();
    let mut p = Publication {
        family: String::new(),
        subject: String::new(),
        authority: source.into(),
        epoch,
        revision: "1".into(),
        observed: 0,
        watermark: family.is_none(),
        payload: serde_json::Value::Null,
    };
    let Some(f) = family else {
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
        choice(v, "operation", &["PublishPublicProjectionWatermarkV1"])?;
        let f = text(v, "family")?;
        if !FAMILIES.iter().any(|x| x.name == f) {
            return Err(());
        }
        let complete = time(v, "complete_through", true)?;
        let observed = time(v, "observed_at", true)?;
        if complete > observed || observed as u64 > now.saturating_add(1) {
            return Err(());
        }
        p.family = f.into();
        p.observed = complete;
        return Ok(p);
    };
    choice(v, "operation", &[f.operation])?;
    p.family = f.name.into();
    p.revision = decimal(v, "projection_revision", false)?.into();
    p.observed = time(v, "source_observed_at", true)?;
    if p.observed as u64 > now.saturating_add(1) {
        return Err(());
    }
    let extras: &[&str] = match f.name {
        "highscores" => {
            let world = uuid(v, "world_id")?;
            let category = integer(v, "category", 1, 11)?;
            let filter = integer(v, "filter", 0, 5)?;
            time(v, "computed_at", false)?;
            let mut previous = "";
            let mut seen = std::collections::HashSet::new();
            for (i, r) in list(v, "rows", 0, 1000)?.iter().enumerate() {
                keys(
                    r,
                    &["rank", "character_id", "name", "vocation", "level", "value"],
                )?;
                let rank = decimal(r, "rank", false)?;
                if rank != (i + 1).to_string() && rank != previous {
                    return Err(());
                }
                previous = rank;
                if !seen.insert(uuid(r, "character_id")?) {
                    return Err(());
                }
                name(r, "name")?;
                integer(r, "vocation", 0, 255)?;
                decimal(r, "level", false)?;
                decimal(r, "value", true)?;
            }
            p.subject = format!("{world}:{category}:{filter}");
            &["world_id", "category", "filter", "computed_at", "rows"]
        }
        "character_profile" => {
            p.subject = uuid(v, "character_id")?.into();
            for r in list(v, "profile", 0, 1)? {
                keys(
                    r,
                    &[
                        "world_id",
                        "name",
                        "vocation",
                        "level",
                        "guild",
                        "guild_visible",
                    ],
                )?;
                uuid(r, "world_id")?;
                name(r, "name")?;
                integer(r, "vocation", 0, 255)?;
                decimal(r, "level", false)?;
                boolean(r, "guild_visible")?;
                for g in list(r, "guild", 0, 1)? {
                    keys(g, &["guild_id", "rank_level"])?;
                    uuid(g, "guild_id")?;
                    integer(g, "rank_level", 1, 20)?;
                }
            }
            &["character_id", "profile"]
        }
        "character_deaths" => {
            p.subject = uuid(v, "character_id")?.into();
            let mut previous = i64::MAX;
            for r in list(v, "deaths", 0, 20)? {
                keys(r, &["occurred_at", "level", "pvp", "killers"])?;
                let t = time(r, "occurred_at", false)?;
                if t > previous {
                    return Err(());
                }
                previous = t;
                decimal(r, "level", false)?;
                boolean(r, "pvp")?;
                for k in list(r, "killers", 0, 16)? {
                    name_value(k)?;
                }
            }
            &["character_id", "deaths"]
        }
        "guild" => {
            p.subject = uuid(v, "guild_id")?.into();
            for g in list(v, "guild", 0, 1)? {
                keys(
                    g,
                    &[
                        "world_id",
                        "name",
                        "state",
                        "founded_at",
                        "ranks",
                        "members",
                    ],
                )?;
                uuid(g, "world_id")?;
                name(g, "name")?;
                choice(g, "state", &["FORMING", "ACTIVE", "DISBANDING"])?;
                time(g, "founded_at", false)?;
                let mut levels = Vec::new();
                for r in list(g, "ranks", 3, 20)? {
                    keys(r, &["level", "name"])?;
                    let n = integer(r, "level", 1, 20)?;
                    if levels.last().is_some_and(|last| n <= *last) {
                        return Err(());
                    }
                    levels.push(n);
                    name(r, "name")?;
                }
                let mut previous = None;
                for m in list(g, "members", 0, 2000)? {
                    keys(m, &["name", "rank_level", "joined_at"])?;
                    let n = name(m, "name")?;
                    let rank = integer(m, "rank_level", 1, 20)?;
                    if !levels.contains(&rank) || previous.is_some_and(|last| (rank, n) <= last) {
                        return Err(());
                    }
                    previous = Some((rank, n));
                    time(m, "joined_at", false)?;
                }
            }
            &["guild_id", "guild"]
        }
        "house" => {
            p.subject = format!("{}:{}", uuid(v, "world_id")?, content(v, "house_key")?);
            for h in list(v, "house", 0, 1)? {
                let status = choice(
                    h,
                    "status",
                    &[
                        "VACANT",
                        "AUCTION",
                        "OWNED",
                        "MOVE_OUT_PENDING",
                        "UNAVAILABLE",
                    ],
                )?;
                let mut fields = vec![
                    "name",
                    "kind",
                    "town",
                    "size_sqm",
                    "beds",
                    "rent_gold",
                    "status",
                ];
                if status == "AUCTION" {
                    fields.extend(["current_bid", "auction_ends_at"]);
                    decimal(h, "current_bid", true)?;
                    time(h, "auction_ends_at", false)?;
                }
                if ["OWNED", "MOVE_OUT_PENDING"].contains(&status) {
                    fields.extend(["owner_kind", "owner_name", "paid_until"]);
                    choice(h, "owner_kind", &["CHARACTER", "GUILD"])?;
                    name(h, "owner_name")?;
                    time(h, "paid_until", false)?;
                }
                if status == "MOVE_OUT_PENDING" {
                    fields.push("move_out_at");
                    time(h, "move_out_at", false)?;
                }
                keys(h, &fields)?;
                name(h, "name")?;
                choice(h, "kind", &["private_house", "shop", "guildhall"])?;
                content(h, "town")?;
                decimal(h, "size_sqm", false)?;
                decimal(h, "beds", true)?;
                decimal(h, "rent_gold", false)?;
            }
            &["world_id", "house_key", "house"]
        }
        "world_online" => {
            p.subject = uuid(v, "world_id")?.into();
            let mut previous = None;
            for r in list(v, "characters", 0, 4096)? {
                keys(r, &["name", "vocation", "level"])?;
                let n = name(r, "name")?;
                if previous.is_some_and(|last| n <= last) {
                    return Err(());
                }
                previous = Some(n);
                integer(r, "vocation", 0, 255)?;
                decimal(r, "level", false)?;
            }
            &["world_id", "characters"]
        }
        _ => return Err(()),
    };
    let mut all = vec![
        "contract_version",
        "operation",
        "source_authority",
        "projection_epoch",
        "projection_revision",
        "source_observed_at",
    ];
    all.extend(extras);
    keys(v, &all)?;
    let mut content = serde_json::Map::new();
    for k in extras {
        content.insert(
            (*k).into(),
            serde_json::to_value(field(v, k)?).map_err(|_| ())?,
        );
    }
    p.payload = serde_json::Value::Object(content);
    Ok(p)
}
