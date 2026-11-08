//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("PANEL_PREMIUM_PAYMENTS"),
        "premium",
        "shell-premium",
        "panel/premium",
        Group::Account,
        false,
    );
    registry.section(
        on("PANEL_PREMIUM_PAYMENTS"),
        "payments",
        "shell-payments",
        "panel/payments",
        Group::Account,
        false,
    );
}
