//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(true, "wiki", "shell-wiki", "wiki", Group::World, true);
    registry.section(
        on("PORTAL_CALCULATORS_ENABLED"),
        "calculators",
        "shell-calculators",
        "wiki/calculators",
        Group::World,
        false,
    );
}
