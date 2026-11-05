//! Store navigation follows the existing storefront flag.
use oteryn_ui::{Group, Registry};
/// Register the storefront when enabled in this process.
pub fn register(registry: &mut Registry) {
    registry.section(
        std::env::var("OTERYN_SHOP_ENABLED").as_deref() == Ok("true"),
        "shop",
        "shell-shop",
        "shop",
        Group::Play,
        false,
    );
}
