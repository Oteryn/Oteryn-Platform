//! Game world data read-only from a pinned game release: entities, links, map and sprites (design T14, T42).
pub mod sprites;

mod import;
mod store;
mod types;

pub use store::WorldStore;
pub use types::{
    Entity, ImportConfig, ImportIssue, ImportMapping, ImportReport, SchemaScope, VersionStatus,
    WorldError, WorldOptions,
};

pub mod admin;

/// Locked native offline catalogue consumer (inactive candidates only).
pub mod catalog;

#[cfg(test)]
#[path = "../tests/support/catalog.rs"]
mod catalog_fixture;

/// Canonical public entity families shared by imports, approvals, wiki and atlas.
/// Unknown application-specific families retain their identity.
pub fn canonical_kind(kind: &str) -> &str {
    match kind {
        "items" => "Item",
        "creatures" => "Creature",
        "npcs" => "NPC",
        "spells" => "Spell",
        "quests" => "Quest",
        "loot" => "Loot",
        "achievements" => "Achievement",
        "areas" => "Area",
        "spawns" => "Spawn.Source",
        _ => kind,
    }
}

#[cfg(test)]
#[path = "../../admin/src/test_staff.rs"]
pub mod test_staff;
