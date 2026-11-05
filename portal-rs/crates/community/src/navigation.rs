//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("PORTAL_WORLDS_ENABLED")
            && std::env::var("PORTAL_WORLDS_FRESHNESS_SECONDS")
                .map_or(true, |s| s.parse::<u64>().is_ok_and(|n| n > 0)),
        "worlds",
        "shell-worlds",
        "worlds",
        Group::World,
        false,
    );
    registry.section(
        on("PORTAL_ONLINE_ENABLED"),
        "online",
        "shell-online",
        "online",
        Group::Community,
        false,
    );
    registry.section(
        on("PORTAL_PUBLIC_RANKINGS_ENABLED"),
        "rankings",
        "shell-rankings",
        "rankings/experience",
        Group::Community,
        false,
    );
    registry.section(
        on("PORTAL_PARTY_ENABLED"),
        "party",
        "shell-party",
        "party",
        Group::Community,
        false,
    );
    let staging = matches!(
        std::env::var("APP_ENV").as_deref(),
        Ok("testing" | "preproduction")
    );
    registry.section(
        on("PORTAL_GUILDS_ENABLED") && staging,
        "guilds",
        "shell-guilds",
        "guilds",
        Group::Community,
        false,
    );
    registry.section(
        on("PORTAL_HOUSES_SERVING_ENABLED") && on("PORTAL_PUBLIC_PROJECTIONS_ENABLED") && staging,
        "houses",
        "shell-houses",
        "houses",
        Group::World,
        false,
    );
    registry.section(
        on("PORTAL_DEATHS_ENABLED") && on("PORTAL_PUBLIC_PROJECTIONS_ENABLED") && staging,
        "deaths",
        "shell-deaths",
        "deaths",
        Group::Community,
        false,
    );
}
