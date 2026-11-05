//! Nontransferable cards and PvE. Progress never grants gameplay or wallet value.
mod battle;
mod http;
mod media;
mod store;
pub use battle::*;
pub use http::{Ownership, configured_router, router};
pub use media::Sprite;
use serde::{Deserialize, Serialize};
pub use store::*;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("collection disabled")]
    Disabled,
    #[error("access denied")]
    Denied,
    #[error("invalid collection input")]
    BadInput,
    #[error("collection source unavailable")]
    Unavailable,
    #[error("collection state conflict")]
    Conflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl Error {
    pub fn status(&self) -> u16 {
        match self {
            Self::Disabled | Self::Unavailable | Self::Database(_) | Self::Json(_) => 503,
            Self::Denied => 403,
            Self::BadInput => 400,
            Self::Conflict => 409,
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub collection: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Labels {
    pub pl: String,
    pub en: String,
}
/// Trusted worker mapping to a public entity and a matching G-11 race definition.
/// This is not a browser publishing payload or a new Game wire contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub id: Uuid,
    pub world_version: Uuid,
    pub kind: String,
    pub key: String,
    pub race_key: String,
    pub race_revision: String,
    pub final_kill_threshold: u32,
    pub name: Labels,
    pub appearance_id: u32,
}
impl Card {
    /// Native creature reader at the task's pinned release. The trusted caller
    /// supplies the accepted creature-to-bestiary and presentation mappings;
    /// missing mappings must never be guessed from display names.
    pub fn from_world(
        entity: &oteryn_world::Entity,
        bestiary: &oteryn_playerdata::progress::Bestiary,
        appearance_id: u32,
        polish_name: String,
    ) -> Result<Self> {
        let name = entity
            .data
            .pointer("/authoring/profile/details/display_name")
            .and_then(serde_json::Value::as_str)
            .ok_or(Error::Unavailable)?;
        let card = Self {
            id: Uuid::now_v7(),
            world_version: entity.version_id,
            kind: entity.kind.clone(),
            key: entity.key.clone(),
            race_key: bestiary.race_key.clone(),
            race_revision: bestiary.race_definition_revision.clone(),
            final_kill_threshold: bestiary.final_kill_threshold,
            name: Labels {
                pl: polish_name,
                en: name.to_owned(),
            },
            appearance_id,
        };
        card.validate()?;
        card.matches_entity(entity)?;
        Ok(card)
    }
    fn matches_entity(&self, entity: &oteryn_world::Entity) -> Result<()> {
        let data = &entity.data;
        if entity.version_id != self.world_version
            || entity.key != self.key
            || entity.kind != self.kind
            || data
                .pointer("/definition/identity/family")
                .and_then(serde_json::Value::as_str)
                != Some("Creature")
            || data
                .pointer("/definition/identity/key")
                .and_then(serde_json::Value::as_str)
                != Some(self.key.as_str())
            || data
                .pointer("/definition/client_projection")
                .and_then(serde_json::Value::as_str)
                != Some("ClientSafe")
            || data
                .pointer("/authoring/profile/details/display_name")
                .and_then(serde_json::Value::as_str)
                != Some(self.name.en.as_str())
            || !data
                .pointer("/authoring/profile/details/bestiary")
                .is_some_and(serde_json::Value::is_object)
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        if self.id.get_version_num() != 7
            || self.world_version.get_version_num() != 7
            || self.final_kill_threshold == 0
            || self.appearance_id == 0
            || [&self.kind, &self.key, &self.race_key, &self.race_revision]
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || [&self.name.pl, &self.name.en]
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 160 || s.chars().any(char::is_control))
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }
    /// Portal-only ruleset v1: every unlocked creature has equal base strength.
    /// Collection diversity never implies a reward or advantage in the game.
    pub fn fighter(&self) -> Fighter {
        Fighter {
            card: self.id,
            health: 30,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedCard {
    pub card: Card,
    pub unlocked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: Uuid,
    pub character: Uuid,
    pub name: String,
    pub cards: Vec<Uuid>,
}
fn valid_deck(name: &str, cards: &[Uuid]) -> Result<()> {
    if name.trim() != name
        || name.is_empty()
        || name.chars().count() > 40
        || name.chars().any(char::is_control)
        || cards.is_empty()
        || cards.len() > 3
        || cards.iter().any(|c| c.get_version_num() != 7)
        || cards
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != cards.len()
    {
        return Err(Error::BadInput);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
