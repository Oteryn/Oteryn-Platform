//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register sections using the existing process enable/mount decisions.
pub fn register(registry: &mut Registry) {
    let flags = crate::config::Flags::configured();
    registry.section(
        flags.viewer && std::env::var_os("OTERYN_ATLAS_DIRECTORY").is_some_and(|p| !p.is_empty()),
        "atlas",
        "shell-atlas",
        "atlas",
        Group::World,
        false,
    );
    registry.section(
        flags.delivery,
        "delivery",
        "shell-delivery",
        "atlas/delivery",
        Group::World,
        false,
    );
}
