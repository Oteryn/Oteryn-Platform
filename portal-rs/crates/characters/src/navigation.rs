//! Account section navigation, using existing feature flags.
use crate::create_web::Settings;
use oteryn_ui::{Group, Registry};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    register_from(registry, |key| std::env::var(key).ok());
}

fn register_from(registry: &mut Registry, lookup: impl Fn(&str) -> Option<String>) {
    registry.section(
        Settings::configured(lookup).mounted(),
        "create-character",
        "shell-create-character",
        "account/characters/new",
        Group::Account,
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_creation_link_requires_the_router_flags() {
        let flags = [
            "OTERYN_CHARACTER_CREATE_WEB_ENABLED",
            "OTERYN_CHARACTER_COMMANDS_ENABLED",
            "OTERYN_CHARACTER_COMMAND_CREATE_ENABLED",
        ];
        for disabled in flags.into_iter().map(Some).chain([None]) {
            let mut registry = Registry::default();
            register_from(&mut registry, |key| {
                (flags.contains(&key) && disabled != Some(key)).then(|| "true".into())
            });
            let chrome = registry.chrome("en", "/en/", oteryn_ui::Account::Anonymous);
            assert_eq!(
                chrome.link("create-character").is_some(),
                disabled.is_none()
            );
        }
    }
}
