//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        on("OTERYN_ATLAS_ENABLED")
            && std::env::var_os("OTERYN_ATLAS_DIRECTORY").is_some_and(|p| !p.is_empty()),
        "atlas",
        "shell-atlas",
        "atlas",
        Group::World,
        false,
    );
    registry.section(
        std::env::var("PORTAL_ATLAS_DELIVERY_ENABLED").as_deref() == Ok("1"),
        "delivery",
        "shell-delivery",
        "atlas/delivery",
        Group::World,
        false,
    );
}
