use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Death {
    pub death_occurrence_id: String,
    pub occurred_at: String,
    pub level: String,
    pub pvp: bool,
    pub killers: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub character_id: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub source_observed_at: String,
    pub deaths: Vec<Death>,
}
/// The accepted V2 set is separate from T24's closed V1 watermark enum.
#[derive(Debug, Serialize, Deserialize)]
pub enum V2Family {
    #[serde(rename = "character_deaths")]
    Deaths,
    #[serde(rename = "guild")]
    Guild,
    #[serde(rename = "house")]
    House,
    #[serde(rename = "character_skills")]
    Skills,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: V2Family,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}
pub(super) fn decimal(s: &str, min: u64, max: u64) -> Result<u64, ()> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(());
    }
    s.parse::<u64>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or(())
}
pub(super) fn time(s: &str, min: u64, now: u64) -> Result<u64, ()> {
    decimal(s, min, now.saturating_add(1).min(i64::MAX as u64))
}
pub(super) fn id(s: &str) -> bool {
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
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(super) fn name(s: &str) -> bool {
    static FORBIDDEN: std::sync::OnceLock<Result<regex::Regex, regex::Error>> =
        std::sync::OnceLock::new();
    !s.is_empty()
        && s.len() <= 64
        && ![".", ".."].contains(&s)
        && !s.contains(['"', '\\', '/'])
        && s.nfc().collect::<String>() == s
        && FORBIDDEN
            .get_or_init(|| regex::Regex::new(r"[\p{Cc}\p{Cf}]"))
            .as_ref()
            .is_ok_and(|r| !r.is_match(s))
}
impl Snapshot {
    pub fn validate(&self, now: u64) -> Result<(), ()> {
        if self.contract_version != 2
            || self.operation != "PublishCharacterDeathsV2"
            || !authority(&self.source_authority)
            || !id(&self.character_id)
            || self.deaths.len() > 20
        {
            return Err(());
        }
        decimal(&self.projection_epoch, 1, u64::MAX)?;
        decimal(&self.projection_revision, 1, u64::MAX)?;
        let observed = time(&self.source_observed_at, 0, now)?;
        let mut previous = None;
        let mut ids = std::collections::HashSet::new();
        for death in &self.deaths {
            let occurred = time(&death.occurred_at, 1, now)?;
            let key = (occurred, death.death_occurrence_id.as_str());
            if !id(&death.death_occurrence_id)
                || !ids.insert(&death.death_occurrence_id)
                || previous.is_some_and(|p| key >= p)
                || occurred > observed
                || death.killers.len() > 16
                || death.killers.iter().any(|k| !name(k))
            {
                return Err(());
            }
            decimal(&death.level, 1, u32::MAX.into())?;
            previous = Some(key);
        }
        Ok(())
    }
}
impl Watermark {
    pub fn validate(&self, now: u64) -> Result<(), ()> {
        if self.contract_version != 2
            || self.operation != "PublishPublicProjectionWatermarkV2"
            || !authority(&self.source_authority)
            || !matches!(self.family, V2Family::Deaths)
        {
            return Err(());
        }
        decimal(&self.projection_epoch, 1, u64::MAX)?;
        let observed = time(&self.observed_at, 0, now)?;
        if time(&self.complete_through, 0, now)? > observed {
            return Err(());
        }
        Ok(())
    }
}
