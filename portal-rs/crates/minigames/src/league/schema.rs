//! Strict candidate wire grammar. Re-encoding must preserve every input byte.
use super::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
record!(EventSnapshot {
    contract_version: u8, operation: String, source_authority: String, world_id: String,
    event_id: String, projection_epoch: String, projection_revision: String,
    source_observed_at: String, event: Vec<Event>
});
record!(SeasonSnapshot {
    contract_version: u8, operation: String, source_authority: String, world_id: String,
    season_id: String, projection_epoch: String, projection_revision: String,
    source_observed_at: String, season: Vec<Season>
});
record!(Event {
    kind: String, source_key: String, occurrence_id: String, ruleset_revision: String,
    content_sha256: String, scheduled_start: String, scheduled_end: String, lock_at: String,
    options: Vec<String>, state: String, outcome: Vec<String>, decided_at: Vec<String>,
    correction_of: Vec<String>, reason_code: String, evidence_sha256: Vec<String>
});
record!(Season {
    ruleset_revision: String, starts_at: String, ends_at: String, freeze_at: String,
    state: String, challenges: Vec<Challenge>, standings: Vec<Guild>, computed_at: String,
    source_complete_through: String, correction_of: Vec<String>
});
record!(Challenge {
    challenge_id: String,
    source_key: String,
    scoring_revision: String,
    max_score: String
});
record!(Guild { guild_id: String, guild_name: String, scores: Vec<Score>, total_score: String, rank: String });
record!(Score {
    challenge_id: String,
    score: String
});

pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn id(s: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(s).map_err(|_| Error::Invalid)?;
    ensure(
        id.get_version_num() == 7
            && id.get_variant() == uuid::Variant::RFC4122
            && id.to_string() == s,
    )?;
    Ok(id)
}
pub fn number(s: &str, zero: bool) -> Result<u64> {
    let n = s.parse::<u64>().map_err(|_| Error::Invalid)?;
    ensure((zero || n > 0) && n.to_string() == s)?;
    Ok(n)
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && (s.as_bytes()[0].is_ascii_lowercase() || s.as_bytes()[0].is_ascii_digit())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:/-".contains(&b))
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn hashes(v: &[String]) -> bool {
    v.len() <= 1 && v.iter().all(|s| digest(s))
}
pub(super) fn ensure(ok: bool) -> Result<()> {
    if ok { Ok(()) } else { Err(Error::Invalid) }
}
fn canonical<T: for<'de> Deserialize<'de> + Serialize>(raw: &[u8], max: usize) -> Result<T> {
    ensure(raw.len() <= max && !raw.contains(&b'\\'))?;
    let value: T = serde_json::from_slice(raw).map_err(|_| Error::Invalid)?;
    ensure(serde_json::to_vec(&value).map_err(|_| Error::Invalid)? == raw)?;
    Ok(value)
}
#[derive(Debug, Clone)]
pub enum Snapshot {
    Event(EventSnapshot),
    Season(SeasonSnapshot),
}
impl Snapshot {
    pub fn decode(raw: &[u8], season: bool, now: u64) -> Result<Self> {
        let s = if season {
            let s: SeasonSnapshot = canonical(raw, 262144)?;
            ensure(
                s.contract_version == 1
                    && s.operation == "PublishGuildSeasonV1"
                    && s.season.len() <= 1,
            )?;
            id(&s.season_id)?;
            for r in &s.season {
                r.validate(now)?;
            }
            Self::Season(s)
        } else {
            let s: EventSnapshot = canonical(raw, 16384)?;
            ensure(
                s.contract_version == 1
                    && s.operation == "PublishWorldEventOutcomeV1"
                    && s.event.len() <= 1,
            )?;
            id(&s.event_id)?;
            for r in &s.event {
                r.validate(now)?;
            }
            Self::Event(s)
        };
        id(s.world())?;
        ensure(key(s.authority()) && s.observed()? <= now.saturating_add(1))?;
        number(s.epoch(), false)?;
        number(s.revision(), false)?;
        Ok(s)
    }
    pub fn family(&self) -> &'static str {
        match self {
            Self::Event(_) => "world_event_outcome",
            Self::Season(_) => "guild_season",
        }
    }
    pub fn authority(&self) -> &str {
        match self {
            Self::Event(s) => &s.source_authority,
            Self::Season(s) => &s.source_authority,
        }
    }
    pub fn world(&self) -> &str {
        match self {
            Self::Event(s) => &s.world_id,
            Self::Season(s) => &s.world_id,
        }
    }
    pub fn subject(&self) -> &str {
        match self {
            Self::Event(s) => &s.event_id,
            Self::Season(s) => &s.season_id,
        }
    }
    pub fn epoch(&self) -> &str {
        match self {
            Self::Event(s) => &s.projection_epoch,
            Self::Season(s) => &s.projection_epoch,
        }
    }
    pub fn revision(&self) -> &str {
        match self {
            Self::Event(s) => &s.projection_revision,
            Self::Season(s) => &s.projection_revision,
        }
    }
    pub fn observed(&self) -> Result<u64> {
        number(
            match self {
                Self::Event(s) => &s.source_observed_at,
                Self::Season(s) => &s.source_observed_at,
            },
            false,
        )
    }
    pub fn record(&self) -> Result<Vec<u8>> {
        match self {
            Self::Event(s) => s.event.first().map(serde_json::to_vec),
            Self::Season(s) => s.season.first().map(serde_json::to_vec),
        }
        .transpose()
        .map(|v| v.unwrap_or_default())
        .map_err(|_| Error::Invalid)
    }
    pub fn final_state(&self) -> bool {
        match self {
            Self::Event(s) => s.event.first().is_some_and(|r| r.state == "CLOSED"),
            Self::Season(s) => s
                .season
                .first()
                .is_some_and(|r| matches!(r.state.as_str(), "FROZEN" | "VOID")),
        }
    }
    pub fn correction(&self) -> &[String] {
        match self {
            Self::Event(s) => s.event.first().map(|r| r.correction_of.as_slice()),
            Self::Season(s) => s.season.first().map(|r| r.correction_of.as_slice()),
        }
        .unwrap_or_default()
    }
    /// Transport observation time is excluded from PP content identity.
    pub fn content(&self) -> Result<Vec<u8>> {
        let mut s = self.clone();
        match &mut s {
            Self::Event(s) => s.source_observed_at = "1".into(),
            Self::Season(s) => s.source_observed_at = "1".into(),
        }
        match s {
            Self::Event(s) => serde_json::to_vec(&s),
            Self::Season(s) => serde_json::to_vec(&s),
        }
        .map_err(|_| Error::Invalid)
    }
}
impl Event {
    fn validate(&self, now: u64) -> Result<()> {
        ensure(
            matches!(
                self.kind.as_str(),
                "RAID" | "BOSS" | "DAILY_WORLD_GATE" | "WORLD_CHANGE"
            ) && key(&self.source_key)
                && key(&self.ruleset_revision)
                && digest(&self.content_sha256),
        )?;
        id(&self.occurrence_id)?;
        let start = number(&self.scheduled_start, false)?;
        let end = number(&self.scheduled_end, false)?;
        ensure(
            number(&self.lock_at, false)? <= start
                && start < end
                && (2..=16).contains(&self.options.len())
                && self.options.iter().all(|s| key(s))
                && self.options.windows(2).all(|w| w[0] < w[1]),
        )?;
        ensure(hashes(&self.correction_of) && hashes(&self.evidence_sha256))?;
        if self.state == "SETTLED" || (self.state == "CLOSED" && !self.outcome.is_empty()) {
            ensure(
                number(&self.lock_at, false)? <= now
                    && self.decided_at.first().is_some_and(|t| {
                        number(t, false)
                            .is_ok_and(|n| n >= number(&self.lock_at, false).unwrap_or(u64::MAX))
                    }),
            )?;
        }
        let terminal = matches!(self.state.as_str(), "SETTLED" | "VOID" | "CLOSED");
        ensure(
            matches!(
                self.state.as_str(),
                "SCHEDULED" | "LOCKED" | "SETTLED" | "VOID" | "CLOSED"
            ) && self.decided_at.len() == usize::from(terminal)
                && self.evidence_sha256.len() == usize::from(terminal),
        )?;
        for t in &self.decided_at {
            ensure(number(t, false)? <= now.saturating_add(1))?;
        }
        ensure(self.outcome.len() <= 1 && self.outcome.iter().all(|o| self.options.contains(o)))?;
        ensure(match self.state.as_str() {
            "SCHEDULED" | "LOCKED" => {
                self.outcome.is_empty()
                    && self.correction_of.is_empty()
                    && self.reason_code == "NONE"
            }
            "SETTLED" => {
                self.outcome.len() == 1
                    && ((self.reason_code == "NATIVE_RESULT" && self.correction_of.is_empty())
                        || (self.reason_code == "CORRECTION" && self.correction_of.len() == 1))
            }
            "VOID" => {
                self.outcome.is_empty()
                    && ((matches!(
                        self.reason_code.as_str(),
                        "CANCELLED" | "SOURCE_UNAVAILABLE" | "INVALID_SOURCE"
                    ) && self.correction_of.is_empty())
                        || (self.reason_code == "CORRECTION" && self.correction_of.len() == 1))
            }
            "CLOSED" => self.reason_code == "FINAL_FREEZE",
            _ => false,
        })
    }
}
impl Season {
    fn validate(&self, now: u64) -> Result<()> {
        let start = number(&self.starts_at, false)?;
        let end = number(&self.ends_at, false)?;
        let freeze = number(&self.freeze_at, false)?;
        let computed = number(&self.computed_at, false)?;
        let complete = number(&self.source_complete_through, false)?;
        ensure(
            key(&self.ruleset_revision)
                && start < end
                && end <= freeze
                && computed >= complete
                && computed <= now.saturating_add(1)
                && hashes(&self.correction_of)
                && matches!(self.state.as_str(), "OPEN" | "FROZEN" | "VOID"),
        )?;
        if self.state == "FROZEN" {
            ensure(now >= freeze && computed >= freeze && complete >= end)?;
        }
        ensure((1..=32).contains(&self.challenges.len()) && self.standings.len() <= 1000)?;
        let mut last = None;
        for c in &self.challenges {
            let cid = id(&c.challenge_id)?;
            ensure(last.is_none_or(|p| p < cid) && key(&c.source_key) && key(&c.scoring_revision))?;
            number(&c.max_score, false)?;
            last = Some(cid);
        }
        let mut seen = std::collections::HashSet::new();
        let mut previous: Option<(u64, Vec<u64>, Uuid)> = None;
        for (i, g) in self.standings.iter().enumerate() {
            let gid = id(&g.guild_id)?;
            // Until the PP privacy/NFC adapter is accepted, a conservative ASCII subset.
            ensure(
                seen.insert(gid)
                    && !g.guild_name.is_empty()
                    && g.guild_name.len() <= 64
                    && g.guild_name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b" '-".contains(&b))
                    && g.scores.len() == self.challenges.len()
                    && number(&g.rank, false)? == (i + 1) as u64,
            )?;
            let mut sum = 0u64;
            let mut vector = vec![];
            for (s, c) in g.scores.iter().zip(&self.challenges) {
                let n = number(&s.score, true)?;
                ensure(s.challenge_id == c.challenge_id && n <= number(&c.max_score, false)?)?;
                sum = sum.checked_add(n).ok_or(Error::Invalid)?;
                vector.push(n);
            }
            ensure(sum == number(&g.total_score, true)?)?;
            if let Some((total, scores, uuid)) = previous {
                ensure(
                    total > sum
                        || (total == sum && (scores > vector || (scores == vector && uuid < gid))),
                )?;
            }
            previous = Some((sum, vector, gid));
        }
        Ok(())
    }
}
pub(super) fn hash_bytes_equal(bytes: &[u8], hex: &str) -> bool {
    bytes.len() == 32 && bytes.iter().map(|b| format!("{b:02x}")).collect::<String>() == hex
}
