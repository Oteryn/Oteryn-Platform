//! Immutable daily puzzles. An editorial caller authorizes publication; there is
//! deliberately no public publishing endpoint. A UTC day is complete only when
//! all three puzzles are answered correctly. Each puzzle permits one attempt.
mod assets;
mod http;
mod store;

pub use assets::Media;
pub use http::{configured_router, router};
pub use store::Puzzles;

use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub daily_puzzles: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("daily puzzles are disabled")]
    Disabled,
    #[error("invalid puzzle input")]
    Invalid,
    #[error("puzzle source is unavailable")]
    Unavailable,
    #[error("account is not eligible")]
    Forbidden,
    #[error("puzzle is not available today")]
    NotFound,
    #[error("puzzle already exists")]
    Conflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Monster,
    Atlas,
    Quiz,
}
impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Monster => "monster",
            Self::Atlas => "atlas",
            Self::Quiz => "quiz",
        }
    }
}

/// Plain editorial text; rendering always escapes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub pl: String,
    pub en: String,
}
impl Text {
    fn validate(&self) -> Result<(), Error> {
        for value in [&self.pl, &self.en] {
            if value.trim().is_empty() || value.len() > 500 || value.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
    fn localized(&self, language: &str) -> &str {
        if language == "pl" { &self.pl } else { &self.en }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    pub kind: String,
    pub key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub label: Text,
    pub source: EntityRef,
}
/// Stored only on the server. Correct choice and entity keys never reach HTML.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub kind: Kind,
    pub world_version: Uuid,
    pub question: Text,
    /// Accessible equivalent clue that does not name the solution.
    pub clue: Text,
    pub choices: Vec<Choice>,
    pub correct_choice: u8,
}
impl std::fmt::Debug for Definition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Definition")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl Definition {
    fn validate(&self) -> Result<(), Error> {
        self.question.validate()?;
        self.clue.validate()?;
        if !(2..=6).contains(&self.choices.len())
            || usize::from(self.correct_choice) >= self.choices.len()
            || self.world_version.get_version_num() != 7
        {
            return Err(Error::Invalid);
        }
        for (index, choice) in self.choices.iter().enumerate() {
            choice.label.validate()?;
            for value in [&choice.source.kind, &choice.source.key] {
                if value.is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
                    return Err(Error::Invalid);
                }
            }
            if self.choices[..index].iter().any(|other| {
                (other.source.kind == choice.source.kind && other.source.key == choice.source.key)
                    || other
                        .label
                        .pl
                        .trim()
                        .eq_ignore_ascii_case(choice.label.pl.trim())
                    || other
                        .label
                        .en
                        .trim()
                        .eq_ignore_ascii_case(choice.label.en.trim())
            }) {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub choice: u8,
    pub correct: bool,
    pub replayed: bool,
}
#[derive(Debug, Clone)]
pub struct Daily {
    pub id: Uuid,
    pub day: Date,
    pub kind: Kind,
    pub question: Text,
    pub clue: Text,
    pub choices: Vec<Text>,
    pub has_image: bool,
    pub answer: Option<Answer>,
}

/// Badges derive from the durable best streak; no transferable reward or game effect.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub current: i32,
    pub best: i32,
}
impl Progress {
    pub fn badges(&self) -> Vec<i32> {
        [3, 7, 30]
            .into_iter()
            .filter(|days| self.best >= *days)
            .collect()
    }
}

#[cfg(test)]
mod tests;
