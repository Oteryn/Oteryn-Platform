//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Register the bazaar pages. They are always mounted: without the Game runtime they answer
/// with the documented unavailable page, so the sections stay visible like other Game reads.
pub fn register(registry: &mut Registry) {
    registry.section(
        true,
        "bazaar",
        "shell-bazaar",
        "bazaar",
        Group::World,
        false,
    );
    registry.section(
        true,
        "account-bazaar",
        "shell-account-bazaar",
        "account/bazaar",
        Group::Account,
        false,
    );
}
