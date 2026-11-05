//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("OTERYN_DAILY_PUZZLES_ENABLED"),
        "puzzles",
        "shell-puzzles",
        "puzzles",
        Group::Play,
        false,
    );
    registry.section(
        on("OTERYN_COLLECTION_ENABLED"),
        "collection",
        "shell-collection",
        "collection",
        Group::Play,
        false,
    );
    registry.section(
        on("OTERYN_SEASON_PASS_ENABLED"),
        "pass",
        "shell-pass",
        "pass",
        Group::Play,
        false,
    );
}
