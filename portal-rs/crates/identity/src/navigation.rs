//! Account section navigation, using existing feature flags.
use oteryn_ui::{Group, Registry};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    let flag = |key| std::env::var(key).as_deref() == Ok("true");
    registry.section(
        std::env::var("OTERYN_ACCOUNT_OVERVIEW_ENABLED").as_deref() == Ok("true"),
        "account-overview",
        "shell-account-overview",
        "/account",
        Group::Account,
        false,
    );
    registry.section(
        std::env::var("OTERYN_ACCOUNT_CREDENTIALS").as_deref() == Ok("true"),
        "account-security",
        "shell-account-security",
        "password/change",
        Group::Account,
        false,
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
    registry.section(
        registry.login && flag("OTERYN_ACCOUNT_MFA"),
        "account-mfa",
        "shell-account-mfa",
        "mfa",
        Group::Account,
        false,
    );
    registry.section(
        flag("OTERYN_ACCOUNT_EMAIL_CHANGE")
            && std::env::var("OTERYN_PUBLIC_ORIGIN")
                .is_ok_and(|origin| crate::email_change::valid_origin(&origin))
            && std::env::var("OTERYN_MFA_KEY").is_ok_and(|key| crate::mfa::valid_token(&key)),
        "account-email",
        "shell-account-email",
        "account/security/email",
        Group::Account,
        false,
    );
    registry.section(
        crate::consents::OptionalConfig::configured().enabled,
        "account-consents",
        "shell-account-consents",
        "account/consents",
        Group::Account,
        false,
    );
    registry.section(
        flag("OTERYN_ACCOUNT_CREDENTIALS") && crate::export::ExportConfig::configured().enabled,
        "account-export",
        "shell-account-export",
        "account/privacy/export",
        Group::Account,
        false,
    );
}
