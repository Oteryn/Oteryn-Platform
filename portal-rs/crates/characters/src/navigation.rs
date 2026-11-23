//! Account section navigation, using existing feature flags.
use oteryn_ui::{Group, Registry};
/// Register the character creation link when the browser route is mounted.
pub fn register(registry: &mut Registry) {
    registry.section(
        crate::create_web::Settings::configured().mounted(),
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
    use std::{env, process::Command};

    #[test]
    fn character_creation_link_requires_all_route_flags() -> std::io::Result<()> {
        const EXPECT_LINK: &str = "OTERYN_TEST_EXPECT_CHARACTER_CREATION_LINK";
        if let Ok(expected) = env::var(EXPECT_LINK) {
            let mut registry = Registry::default();
            register(&mut registry);
            let chrome = registry.chrome("en", "/en/", oteryn_ui::Account::Anonymous);
            assert_eq!(
                chrome.link("create-character").is_some(),
                expected == "true"
            );
            return Ok(());
        }

        let flags = [
            "OTERYN_CHARACTER_CREATE_WEB_ENABLED",
            "OTERYN_CHARACTER_COMMANDS_ENABLED",
            "OTERYN_CHARACTER_COMMAND_CREATE_ENABLED",
        ];
        // Isolate the flag combinations from other tests without changing process globals.
        for disabled in flags.into_iter().map(Some).chain([None]) {
            let mut child = Command::new(env::current_exe()?);
            child.args([
                "--exact",
                "navigation::tests::character_creation_link_requires_all_route_flags",
            ]);
            child.env(EXPECT_LINK, disabled.is_none().to_string());
            child.env_remove("OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY");
            for flag in flags {
                child.env(
                    flag,
                    if disabled == Some(flag) {
                        "false"
                    } else {
                        "true"
                    },
                );
            }
            let output = child.output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed;"),
                "disabled flag: {disabled:?}\n{}",
                stdout,
            );
        }
        Ok(())
    }
}
