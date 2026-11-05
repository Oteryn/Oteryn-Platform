use super::{Error, Result, decimal, id, key, revision, timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bestiary {
    pub race_key: String,
    pub race_definition_revision: String,
    pub kill_count: u32,
    pub final_kill_threshold: u32,
    pub charm_points_awarded: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boss {
    pub boss_key: String,
    pub definition_revision: String,
    pub kill_count: String,
    pub available_at_unix_ms: Option<String>,
    pub last_death_occurrence_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub track_key: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quest {
    pub quest_key: String,
    pub pinned_content_revision: String,
    pub definition_hash: String,
    pub completed: bool,
    pub tracks: Vec<Track>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Achievement {
    pub achievement_key: String,
    pub definition_revision: String,
    pub progress: Option<u32>,
    pub target: Option<u32>,
    pub granted_at_unix_ms: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillChange {
    pub skill: String,
    pub level_before: u16,
    pub progress_before: String,
    pub level_after: u16,
    pub progress_after: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillHistory {
    pub build_occurrence_id: String,
    pub cause: String,
    pub policy_revision: String,
    pub skills: Vec<SkillChange>,
}
/// Family selected by the envelope, never by untrusted shape inference.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Payload {
    Bestiary(Bestiary),
    Boss(Boss),
    Quest(Quest),
    Achievement(Achievement),
    Skills(SkillHistory),
}
fn signed(v: &str) -> bool {
    v.parse::<i64>().is_ok_and(|n| n.to_string() == v)
}
fn sorted<T>(items: &[T], get: impl Fn(&T) -> &str) -> bool {
    items.windows(2).all(|w| get(&w[0]) < get(&w[1]))
}
pub fn skill(v: &str) -> bool {
    matches!(
        v,
        "magic" | "fist" | "club" | "sword" | "axe" | "distance" | "shielding" | "fishing"
    )
}
impl Payload {
    pub fn decode(family: &str, value: serde_json::Value, account: bool) -> Result<Self> {
        macro_rules! decode {
            ($ty:ty, $variant:ident) => {
                Self::$variant(serde_json::from_value::<$ty>(value).map_err(|_| Error::BadInput)?)
            };
        }
        let payload = match family {
            "bestiary" => decode!(Bestiary, Bestiary),
            "boss_cooldown" => decode!(Boss, Boss),
            "quest" => decode!(Quest, Quest),
            "achievement" => decode!(Achievement, Achievement),
            "skill_history" => decode!(SkillHistory, Skills),
            _ => return Err(Error::BadInput),
        };
        let valid = match &payload {
            Self::Bestiary(p) => {
                !account
                    && key(&p.race_key)
                    && revision(&p.race_definition_revision)
                    && p.final_kill_threshold > 0
            }
            Self::Boss(p) => {
                !account
                    && key(&p.boss_key)
                    && revision(&p.definition_revision)
                    && decimal(&p.kill_count, false).is_some()
                    && p.available_at_unix_ms.as_deref().is_none_or(timestamp)
                    && p.last_death_occurrence_id.as_deref().is_none_or(id)
            }
            Self::Quest(p) => {
                !account
                    && key(&p.quest_key)
                    && revision(&p.pinned_content_revision)
                    && p.definition_hash.len() == 64
                    && p.definition_hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    && p.tracks.len() <= 256
                    && sorted(&p.tracks, |t| &t.track_key)
                    && p.tracks
                        .iter()
                        .all(|t| key(&t.track_key) && signed(&t.value))
            }
            Self::Achievement(p) => {
                let name = p.achievement_key.strip_prefix("oteryn:achievement/");
                let valid_name = name.is_some_and(|n| {
                    !n.is_empty()
                        && n.split('_').all(|part| {
                            !part.is_empty()
                                && part
                                    .bytes()
                                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                        })
                });
                valid_name
                    && p.achievement_key.len() <= 160
                    && revision(&p.definition_revision)
                    && if account {
                        p.progress.is_none()
                            && p.target.is_none()
                            && p.granted_at_unix_ms.as_deref().is_some_and(timestamp)
                    } else {
                        p.granted_at_unix_ms.is_none()
                            && matches!((p.progress,p.target), (Some(a),Some(b)) if a <= b && (1..=100000).contains(&b))
                    }
            }
            Self::Skills(p) => {
                !account
                    && id(&p.build_occurrence_id)
                    && revision(&p.policy_revision)
                    && matches!(
                        p.cause.as_str(),
                        "training"
                            | "vocation_choice"
                            | "promotion"
                            | "offline_settlement"
                            | "logout"
                            | "death"
                    )
                    && (1..=8).contains(&p.skills.len())
                    && sorted(&p.skills, |s| &s.skill)
                    && p.skills.iter().all(|s| {
                        let min = if s.skill == "magic" { 0 } else { 10 };
                        skill(&s.skill)
                            && (min..=1000).contains(&s.level_before)
                            && (min..=1000).contains(&s.level_after)
                            && [&s.progress_before, &s.progress_after]
                                .iter()
                                .all(|v| decimal(v, false).is_some_and(|n| n <= i64::MAX as u64))
                            && (s.level_before != s.level_after
                                || s.progress_before != s.progress_after)
                    })
            }
        };
        if valid {
            Ok(payload)
        } else {
            Err(Error::BadInput)
        }
    }
    pub fn entity(&self) -> &str {
        match self {
            Self::Bestiary(p) => &p.race_key,
            Self::Boss(p) => &p.boss_key,
            Self::Quest(p) => &p.quest_key,
            Self::Achievement(p) => &p.achievement_key,
            Self::Skills(p) => &p.build_occurrence_id,
        }
    }
}
/// Deadlines are observations only; unknown deadlines never mean ready.
impl Boss {
    pub fn cooldown_remaining_ms(&self, now: u64) -> Option<u64> {
        self.available_at_unix_ms
            .as_deref()
            .and_then(|v| decimal(v, false))
            .map(|v| v.saturating_sub(now))
    }
}
