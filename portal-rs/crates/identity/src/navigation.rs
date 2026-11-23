//! Account section navigation, using existing feature flags.
use oteryn_ui::{Entry, Group, Registry};

pub fn mfa_enabled() -> bool {
    std::env::var("OTERYN_ACCOUNT_MFA").as_deref() == Ok("true")
        && std::env::var("OTERYN_ACCOUNT_LOGIN").as_deref() == Ok("true")
}

/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    registry.section(
        std::env::var("OTERYN_ACCOUNT_OVERVIEW_ENABLED").as_deref() == Ok("true"),
        "account-overview",
        "shell-account-overview",
        "/account",
        Group::Account,
        false,
    );
    register_security(
        registry,
        std::env::var("OTERYN_ACCOUNT_CREDENTIALS").as_deref() == Ok("true"),
        mfa_enabled(),
    );
    registry.section(
        std::env::var("OTERYN_SECURITY_WIZARD_ENABLED").as_deref() == Ok("true"),
        "account-wizard",
        "shell-account-wizard",
        "account/security/wizard",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_ACCOUNT_PASSKEYS").as_deref() == Ok("true"),
        "account-passkeys",
        "shell-account-passkeys",
        "account/passkeys",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_ACCOUNT_RECOVERY_KEYS").as_deref() == Ok("true"),
        "account-recovery-key",
        "shell-account-recovery-key",
        "account/recovery-key",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_ACCOUNT_SESSIONS").as_deref() == Ok("true"),
        "account-sessions",
        "shell-account-sessions",
        "account/sessions",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_ACCOUNT_PRIVACY_ENABLED").as_deref() == Ok("true"),
        "account-privacy",
        "shell-account-privacy",
        "account/privacy",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_KEY").is_ok_and(|k| !k.is_empty()),
        "account-notifications",
        "shell-account-notifications",
        "account/notifications",
        Group::Account,
        false,
    );
}

fn register_security(registry: &mut Registry, credentials: bool, mfa: bool) {
    registry.section(
        credentials,
        "account-security",
        "shell-account-security",
        "password/change",
        Group::Account,
        false,
    );
    registry.register(
        mfa && !credentials,
        Entry {
            section: "account-security",
            label: "shell-account-security",
            path: "mfa",
            group: Group::Account,
            footer: false,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_security_navigation_follows_enabled_features() {
        for (credentials, mfa, target) in [
            (true, true, Some("password/change")),
            (true, false, Some("password/change")),
            (false, true, Some("mfa")),
            (false, false, None),
        ] {
            let mut registry = Registry::default();
            register_security(&mut registry, credentials, mfa);
            for lang in ["en", "pl"] {
                let expected = target.map(|path| format!("/{lang}/{path}"));
                let chrome = registry.chrome(
                    lang,
                    expected.as_deref().unwrap_or("/"),
                    oteryn_ui::Account::default(),
                );
                assert_eq!(chrome.entries.len(), usize::from(target.is_some()));
                assert_eq!(
                    chrome.entries.first().map(|link| &link.url),
                    expected.as_ref()
                );
                assert!(
                    chrome
                        .entries
                        .iter()
                        .all(|link| link.current && !link.label.is_empty())
                );
            }
        }
    }
}
