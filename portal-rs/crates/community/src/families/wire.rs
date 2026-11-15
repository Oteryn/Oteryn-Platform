//! Exact G-4 v2 wire for the skills, achievements, equipment and outfit families.
//! Struct declaration order is the wire member order, including inner records.
use super::Family;
use serde::{Deserialize, Serialize};

pub(super) const EQUIPMENT_SLOTS: [&str; 10] = [
    "Head",
    "Necklace",
    "Container",
    "Armor",
    "RightHand",
    "LeftHand",
    "Legs",
    "Feet",
    "Ring",
    "Ammo",
];
const ACHIEVEMENTS: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Skills {
    pub world_id: String,
    pub character_revision: String,
    pub level: String,
    pub total_experience: String,
    pub magic_level: u16,
    pub fist_level: u16,
    pub club_level: u16,
    pub sword_level: u16,
    pub axe_level: u16,
    pub distance_level: u16,
    pub shielding_level: u16,
    pub fishing_level: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Earned {
    pub key: String,
    pub earned_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Achievements {
    pub world_id: String,
    pub character_revision: String,
    pub catalogue_revision: String,
    pub earned: Vec<Earned>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Slot {
    pub slot: String,
    pub item_key: String,
    pub item_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Equipment {
    pub world_id: String,
    pub character_revision: String,
    pub slots: Vec<Slot>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mount {
    pub mount_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Outfit {
    pub world_id: String,
    pub character_revision: String,
    pub content_revision: String,
    pub outfit_key: String,
    pub colours: Vec<u16>,
    pub addons: u8,
    pub mount: Vec<Mount>,
}

/// One snapshot; exactly the body member of its route's family is present.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<Skills>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievements: Option<Vec<Achievements>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<Vec<Equipment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit: Option<Vec<Outfit>>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: String,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}

type Result<T = ()> = std::result::Result<T, ()>;

fn check(ok: bool) -> Result {
    if ok { Ok(()) } else { Err(()) }
}
pub(super) fn decimal(s: &str, min: u64, max: u64) -> Result<u64> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(());
    }
    s.parse::<u64>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or(())
}
/// Unix seconds, at most 1 s into the future.
pub(super) fn time(s: &str, min: u64, now: u64) -> Result<u64> {
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
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn key(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
}
fn revision_key(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn achievement_key(s: &str) -> bool {
    s.strip_prefix("oteryn:achievement/").is_some_and(|rest| {
        (1..=141).contains(&rest.len())
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    })
}
fn subject(world: &str, revision: &str) -> Result {
    check(id(world))?;
    decimal(revision, 1, u64::MAX).map(|_| ())
}
fn level(value: u16, min: u16) -> Result {
    check((min..=1000).contains(&value))
}

impl Snapshot {
    pub fn validate(&self, family: Family, now: u64) -> Result {
        check(
            self.contract_version == 2
                && self.operation == family.operation()
                && authority(&self.source_authority)
                && id(&self.character_id),
        )?;
        decimal(&self.projection_epoch, 1, u64::MAX)?;
        decimal(&self.projection_revision, 1, u64::MAX)?;
        time(&self.source_observed_at, 0, now)?;
        let bodies = [
            self.skills.is_some(),
            self.achievements.is_some(),
            self.equipment.is_some(),
            self.outfit.is_some(),
        ];
        check(bodies.iter().filter(|b| **b).count() == 1)?;
        match family {
            Family::Skills => {
                let records = self.skills.as_deref().ok_or(())?;
                check(records.len() <= 1)?;
                for r in records {
                    subject(&r.world_id, &r.character_revision)?;
                    decimal(&r.level, 1, u32::MAX.into())?;
                    decimal(&r.total_experience, 0, i64::MAX as u64)?;
                    level(r.magic_level, 0)?;
                    for skill in [
                        r.fist_level,
                        r.club_level,
                        r.sword_level,
                        r.axe_level,
                        r.distance_level,
                        r.shielding_level,
                        r.fishing_level,
                    ] {
                        level(skill, 10)?;
                    }
                }
            }
            Family::Achievements => {
                let records = self.achievements.as_deref().ok_or(())?;
                check(records.len() <= 1)?;
                for r in records {
                    subject(&r.world_id, &r.character_revision)?;
                    // Over capacity refuses the whole snapshot; nothing is truncated.
                    check(revision_key(&r.catalogue_revision) && r.earned.len() <= ACHIEVEMENTS)?;
                    let mut previous: Option<&str> = None;
                    for e in &r.earned {
                        check(
                            achievement_key(&e.key)
                                && previous.is_none_or(|p| p.as_bytes() < e.key.as_bytes()),
                        )?;
                        time(&e.earned_at, 1, now)?;
                        previous = Some(&e.key);
                    }
                }
            }
            Family::Equipment => {
                let records = self.equipment.as_deref().ok_or(())?;
                check(records.len() <= 1)?;
                for r in records {
                    subject(&r.world_id, &r.character_revision)?;
                    check(r.slots.len() <= EQUIPMENT_SLOTS.len())?;
                    let mut previous = None;
                    for s in &r.slots {
                        let position = EQUIPMENT_SLOTS
                            .iter()
                            .position(|known| *known == s.slot)
                            .ok_or(())?;
                        check(
                            previous.is_none_or(|p| p < position)
                                && key(&s.item_key)
                                && revision_key(&s.item_revision),
                        )?;
                        previous = Some(position);
                    }
                }
            }
            Family::Outfit => {
                let records = self.outfit.as_deref().ok_or(())?;
                check(records.len() <= 1)?;
                for r in records {
                    subject(&r.world_id, &r.character_revision)?;
                    check(
                        revision_key(&r.content_revision)
                            && key(&r.outfit_key)
                            && r.colours.len() == 4
                            && r.mount.len() <= 1
                            && r.mount.iter().all(|m| key(&m.mount_key)),
                    )?;
                }
            }
        }
        Ok(())
    }
}
impl Watermark {
    pub fn validate(&self, now: u64) -> Result<Family> {
        check(
            self.contract_version == 2
                && self.operation == "PublishPublicProjectionWatermarkV2"
                && authority(&self.source_authority),
        )?;
        let family = Family::from_name(&self.family).ok_or(())?;
        decimal(&self.projection_epoch, 1, u64::MAX)?;
        let observed = time(&self.observed_at, 0, now)?;
        check(time(&self.complete_through, 0, now)? <= observed)?;
        Ok(family)
    }
}
/// The projection name rule; used for public and owner lookups.
pub(super) fn name(s: &str) -> bool {
    use unicode_normalization::UnicodeNormalization;
    static FORBIDDEN: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> =
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
