//! Account section navigation, using existing feature flags.
use oteryn_ui::{
    Group, Registry,
    panels::{Entry, Panel, Permission},
};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    for (enabled, route, label) in [
        (
            flag("OTERYN_ACCOUNT_OVERVIEW_ENABLED"),
            "/account#characters",
            "panel-characters",
        ),
        (
            creation_enabled(),
            "account/characters/new",
            "shell-create-character",
        ),
    ] {
        registry.panels.register(
            enabled,
            Entry {
                panel: Panel::Player,
                route,
                label,
                permission: Permission::AccountOwner,
            },
        );
    }
    registry.section(
        creation_enabled(),
        "create-character",
        "shell-create-character",
        "account/characters/new",
        Group::Account,
        false,
    );
}

pub(crate) fn flag(key: &str) -> bool {
    std::env::var(key).as_deref() == Ok("true")
}

pub(crate) fn creation_enabled() -> bool {
    flag("OTERYN_CHARACTER_CREATE_WEB_ENABLED")
        && flag("OTERYN_CHARACTER_COMMANDS_ENABLED")
        && flag("OTERYN_CHARACTER_COMMAND_CREATE_ENABLED")
}

/// The existing owner view uses the restricted Game projection configuration.
pub fn outfit_enabled() -> bool {
    flag("PORTAL_PUBLIC_PROJECTIONS_ENABLED")
        && flag("PORTAL_G4_OUTFIT_SERVING_ENABLED")
        && oteryn_core::restricted_environment(|key| std::env::var(key).ok())
        && std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").is_ok_and(|authority| {
            (1..=128).contains(&authority.len())
                && authority
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
        })
}
