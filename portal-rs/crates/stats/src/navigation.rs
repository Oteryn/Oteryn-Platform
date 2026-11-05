//! Public statistics navigation.
use oteryn_ui::{Group, Registry};
/// Register public enabled statistics pages.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("PORTAL_WORLD_STATS_ENABLED"),
        "world-stats",
        "shell-world-stats",
        "stats/world",
        Group::World,
        false,
    );
    registry.section(
        on("PORTAL_GUILD_STATS_ENABLED"),
        "guild-stats",
        "shell-guild-stats",
        "stats/guilds",
        Group::Community,
        false,
    );
    registry.section(
        on("PORTAL_GROWTH_RANKINGS_ENABLED"),
        "growth-rankings",
        "shell-growth-rankings",
        "stats/rankings/yesterday",
        Group::Community,
        false,
    );
    registry.section(
        on("PORTAL_CHARACTER_STATS_ENABLED"),
        "compare",
        "shell-compare",
        "stats/compare",
        Group::Community,
        false,
    );
}
