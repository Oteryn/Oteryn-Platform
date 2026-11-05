//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("PORTAL_HIRELINGS_ENABLED"),
        "hirelings",
        "shell-hirelings",
        "panel/hirelings",
        Group::Account,
        false,
    );
}
