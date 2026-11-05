//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("OTERYN_SUPPORT_TICKETS"),
        "tickets",
        "shell-tickets",
        "support/tickets",
        Group::Account,
        false,
    );
    let pending =
        std::env::var("OTERYN_REPORTS_PENDING_LIMIT").map_or(Some(5), |v| v.parse::<u8>().ok());
    let unjustified =
        std::env::var("OTERYN_REPORTS_UNJUSTIFIED_LIMIT").map_or(Some(3), |v| v.parse::<u8>().ok());
    registry.section(
        on("OTERYN_REPORTS_ENABLED")
            && pending.is_some_and(|n| (1..=20).contains(&n))
            && unjustified.is_some_and(|n| n > 0),
        "abuse-reports",
        "shell-abuse-reports",
        "support/reports",
        Group::Account,
        false,
    );
}
