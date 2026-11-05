//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("OTERYN_PLAYERDATA_HUNTS_READ"),
        "hunts",
        "shell-hunts",
        "account/hunts",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_JOURNAL_REPORTS_ENABLED"),
        "reports",
        "shell-reports",
        "account/journal/reports",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED")
            && on("OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED"),
        "logins",
        "shell-logins",
        "account/game-logins",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_PLAYERDATA_PLACES_AGGREGATE"),
        "places",
        "shell-places",
        "atlas/places",
        Group::World,
        false,
    );
}
