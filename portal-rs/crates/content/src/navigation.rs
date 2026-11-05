//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    registry.section(true, "news", "shell-news", "news", Group::Community, true);
    registry.section(
        true,
        "events",
        "shell-events",
        "events",
        Group::Community,
        false,
    );
    registry.section(
        true,
        "support",
        "shell-support",
        "support/report-a-bug",
        Group::Community,
        true,
    );
    registry.section(
        true,
        "privacy",
        "shell-privacy",
        "legal/privacy",
        Group::Account,
        true,
    );
    registry.section(
        true,
        "terms",
        "shell-terms",
        "legal/terms",
        Group::Account,
        true,
    );
    registry.section(
        true,
        "today",
        "shell-today",
        "today",
        Group::Community,
        false,
    );
    registry.section(true, "rules", "shell-rules", "rules", Group::World, false);
    registry.section(
        true,
        "server",
        "shell-server",
        "server-information",
        Group::World,
        false,
    );
}
