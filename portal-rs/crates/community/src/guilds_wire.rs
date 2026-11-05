use super::*;
use unicode_normalization::UnicodeNormalization;
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    contract_version: u8,
    operation: String,
    source_authority: String,
    guild_id: String,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    pub guild: Vec<Guild>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Guild {
    pub world_id: String,
    pub name: String,
    pub state: String,
    pub founded_at: String,
    pub ranks: Vec<Rank>,
    pub members: Vec<Member>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Rank {
    pub level: u8,
    pub name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub character_id: String,
    pub name: String,
    pub rank_level: u8,
    pub joined_at: String,
}
/// Separate v2 vocabulary; the v1 family set remains closed.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum V2Family {
    CharacterDeaths,
    Guild,
    House,
    CharacterSkills,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Watermark {
    contract_version: u8,
    operation: String,
    source_authority: String,
    family: V2Family,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn decimal(s: &str, zero: bool) -> Result<u64, ()> {
    if s.is_empty() || (s.starts_with('0') && s != "0") || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    let n = s.parse::<u64>().map_err(|_| ())?;
    if !zero && n == 0 { Err(()) } else { Ok(n) }
}
fn time(s: &str, zero: bool) -> Result<i64, ()> {
    i64::try_from(decimal(s, zero)?).map_err(|_| ())
}
fn id(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b[14] == b'7'
        && b"89ab".contains(&b[19])
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || (b'a'..=b'f').contains(c)
            }
        })
}
pub(super) fn name(s: &str) -> bool {
    static FORBIDDEN: std::sync::OnceLock<Result<regex::Regex, regex::Error>> =
        std::sync::OnceLock::new();
    let forbidden = FORBIDDEN.get_or_init(|| regex::Regex::new(r"[\p{Cc}\p{Cf}]"));
    !s.is_empty()
        && s.len() <= 64
        && ![".", ".."].contains(&s)
        && !s.contains(['"', '\\', '/'])
        && s.nfc().collect::<String>() == s
        && forbidden.as_ref().is_ok_and(|r| !r.is_match(s))
}
pub(super) fn validate(g: &Guild, observed: i64) -> Result<(), ()> {
    if !id(&g.world_id)
        || !name(&g.name)
        || !["FORMING", "ACTIVE", "DISBANDING"].contains(&g.state.as_str())
        || time(&g.founded_at, false)? > observed
        || !(3..=20).contains(&g.ranks.len())
        || g.members.len() > 2000
    {
        return Err(());
    }
    let mut last = 0;
    for r in &g.ranks {
        if !(1..=20).contains(&r.level) || r.level <= last || !name(&r.name) {
            return Err(());
        }
        last = r.level;
    }
    let mut previous = None;
    let mut ids = std::collections::HashSet::new();
    for m in &g.members {
        let key = (m.rank_level, m.character_id.as_str());
        if !id(&m.character_id)
            || !name(&m.name)
            || !ids.insert(&m.character_id)
            || !g.ranks.iter().any(|r| r.level == m.rank_level)
            || time(&m.joined_at, false)? < time(&g.founded_at, false)?
            || time(&m.joined_at, false)? > observed
            || previous.is_some_and(|p| p >= key)
        {
            return Err(());
        }
        previous = Some(key);
    }
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned + Serialize>(
    value: &canonical::Value,
    body: &[u8],
) -> Result<T, ()> {
    if canonical::encode(value).map_err(|_| ())? != body {
        return Err(());
    }
    let parsed: T = serde_json::from_slice(body).map_err(|_| ())?;
    if serde_json::to_vec(&parsed).map_err(|_| ())? != body {
        return Err(());
    }
    Ok(parsed)
}
pub(super) fn parse(
    value: &canonical::Value,
    body: &[u8],
    watermark: bool,
    now: u64,
) -> Result<Publication, ()> {
    if watermark {
        let w: Watermark = decode(value, body)?;
        let observed = time(&w.observed_at, true)?;
        let complete = time(&w.complete_through, true)?;
        if w.contract_version != 2
            || w.operation != "PublishPublicProjectionWatermarkV2"
            || w.family != V2Family::Guild
            || !authority(&w.source_authority)
            || complete > observed
            || observed as u64 > now.saturating_add(1)
        {
            return Err(());
        }
        decimal(&w.projection_epoch, false)?;
        return Ok(Publication {
            authority: w.source_authority.clone(),
            subject: "v2".into(),
            epoch: w.projection_epoch,
            revision: "1".into(),
            observed: complete,
            watermark: true,
            payload: serde_json::json!({"contract_version":2,"source_authority":w.source_authority}),
        });
    }
    let s: Snapshot = decode(value, body)?;
    let observed = time(&s.source_observed_at, true)?;
    if s.contract_version != 2
        || s.operation != "PublishGuildV2"
        || !authority(&s.source_authority)
        || !id(&s.guild_id)
        || s.guild.len() > 1
        || observed as u64 > now.saturating_add(1)
    {
        return Err(());
    }
    decimal(&s.projection_epoch, false)?;
    decimal(&s.projection_revision, false)?;
    for g in &s.guild {
        validate(g, observed)?;
    }
    Ok(Publication {
        authority: s.source_authority.clone(),
        subject: s.guild_id.clone(),
        epoch: s.projection_epoch,
        revision: s.projection_revision,
        observed,
        watermark: false,
        payload: serde_json::json!({"contract_version":2,"source_authority":s.source_authority,"guild_id":s.guild_id,"guild":s.guild}),
    })
}
