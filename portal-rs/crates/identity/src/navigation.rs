//! Account navigation uses the same configuration as the account routes.
use oteryn_ui::{Account, Group, NavLink, Registry};

fn enabled(key: &str) -> bool {
    std::env::var(key).as_deref() == Ok("true")
}

pub fn overview_enabled() -> bool {
    enabled("OTERYN_ACCOUNT_OVERVIEW_ENABLED")
}

pub fn sudo_enabled() -> bool {
    enabled("OTERYN_ACCOUNT_CREDENTIALS")
}

pub fn export_action() -> Option<&'static str> {
    (crate::export::ExportConfig::configured().enabled && sudo_enabled())
        .then_some("account/privacy/export?start=true")
}

pub fn closure_enabled() -> bool {
    enabled("PORTAL_ACCOUNT_CLOSURE_ENABLED")
        && crate::termination::Config::from_env().is_ok_and(|config| config.enabled)
}

pub fn mfa_enabled() -> bool {
    enabled("OTERYN_ACCOUNT_MFA") && enabled("OTERYN_ACCOUNT_LOGIN")
}

fn email_change_enabled() -> bool {
    enabled("OTERYN_ACCOUNT_EMAIL_CHANGE")
        && std::env::var("OTERYN_PUBLIC_ORIGIN")
            .is_ok_and(|origin| crate::email_change::valid_origin(&origin))
        && std::env::var("OTERYN_MFA_KEY").is_ok_and(|key| crate::mfa::valid_token(&key))
}

fn action_sections(registry: &mut Registry, pending_closure: bool) {
    for (enabled, section, label, path) in [
        (
            sudo_enabled(),
            "account-security",
            "shell-account-security",
            "password/change",
        ),
        (mfa_enabled(), "account-mfa", "shell-account-mfa", "mfa"),
        (
            mfa_enabled(),
            "account-remembered-devices",
            "shell-account-remembered-devices",
            "mfa#remembered-devices",
        ),
        (
            email_change_enabled(),
            "account-email-change",
            "shell-account-email-change",
            "account/security/email",
        ),
        (
            enabled("OTERYN_ACCOUNT_PASSKEYS"),
            "account-passkeys",
            "shell-account-passkeys",
            "account/passkeys",
        ),
        (
            enabled("OTERYN_ACCOUNT_SESSIONS"),
            "account-sessions",
            "shell-account-sessions",
            "account/sessions",
        ),
        (
            enabled("OTERYN_ACCOUNT_PRIVACY_ENABLED"),
            "account-privacy",
            "shell-account-privacy",
            "account/privacy",
        ),
        (
            pending_closure || closure_enabled(),
            "account-closure",
            if pending_closure {
                "shell-account-closure-pending"
            } else {
                "shell-account-closure"
            },
            "account/privacy/delete",
        ),
    ] {
        registry.section(enabled, section, label, path, Group::Account, false);
    }
    if let Some(path) = export_action() {
        registry.section(
            true,
            "account-export",
            "shell-account-export",
            path,
            Group::Account,
            false,
        );
    }
}

pub fn register(registry: &mut Registry) {
    registry.section(
        overview_enabled(),
        "account-overview",
        "shell-account-overview",
        "/account",
        Group::Account,
        false,
    );
    action_sections(registry, false);
    registry.section(
        enabled("OTERYN_SECURITY_WIZARD_ENABLED"),
        "account-wizard",
        "shell-account-wizard",
        "account/security/wizard",
        Group::Account,
        false,
    );
    registry.section(
        enabled("OTERYN_ACCOUNT_RECOVERY_KEYS"),
        "account-recovery-key",
        "shell-account-recovery-key",
        "account/recovery-key",
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

pub fn overview_actions(
    lang: &str,
    passkeys_eligible: bool,
    pending_closure: bool,
) -> Vec<NavLink> {
    let mut registry = Registry::default();
    action_sections(&mut registry, pending_closure);
    registry
        .chrome(
            lang,
            "/account",
            Account::SignedIn {
                csrf: String::new(),
                passkeys_eligible,
            },
        )
        .entries
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SudoReturn {
    #[default]
    Security,
    Export,
    Closure,
}

impl SudoReturn {
    pub fn path(self, lang: &str) -> String {
        let path = match self {
            Self::Security => "account/security",
            Self::Export => "account/privacy/export",
            Self::Closure => "account/privacy/delete",
        };
        format!("/{lang}/{path}")
    }

    pub fn query(self) -> &'static str {
        match self {
            Self::Security => "",
            Self::Export => "?return_to=export",
            Self::Closure => "?return_to=closure",
        }
    }

    pub fn sudo_path(self, lang: &str) -> String {
        format!("/{lang}/account/sudo{}", self.query())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudo_return_targets_are_localized() {
        assert_eq!(SudoReturn::default().path("pl"), "/pl/account/security");
        assert_eq!(SudoReturn::Export.path("en"), "/en/account/privacy/export");
        assert_eq!(SudoReturn::Closure.path("pl"), "/pl/account/privacy/delete");
    }

    #[test]
    fn sudo_return_targets_accept_only_named_actions() {
        use serde::Deserialize;
        use serde::de::value::{Error, StrDeserializer};
        let parse = |value| SudoReturn::deserialize(StrDeserializer::<Error>::new(value));
        assert!(matches!(parse("export"), Ok(SudoReturn::Export)));
        assert!(matches!(parse("closure"), Ok(SudoReturn::Closure)));
        for value in [
            "",
            "unknown",
            "/account",
            "//example.invalid",
            "https://example.invalid",
        ] {
            assert!(parse(value).is_err());
        }
    }
}
