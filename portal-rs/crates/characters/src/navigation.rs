//! Account section navigation, using existing feature flags.
use oteryn_ui::{Group, Registry};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    registry.section(
        std::env::var("OTERYN_CHARACTER_CREATE_WEB_ENABLED").as_deref() == Ok("true"),
        "create-character",
        "shell-create-character",
        "account/characters/new",
        Group::Account,
        false,
    );
}
