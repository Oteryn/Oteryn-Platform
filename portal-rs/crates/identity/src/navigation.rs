//! Account section navigation, using existing feature flags.
use oteryn_ui::{Group, Registry};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    let on = |flag| std::env::var(flag).as_deref() == Ok("true");
    registry.section(
        on("OTERYN_ACCOUNT_MFA") && on("OTERYN_ACCOUNT_LOGIN"),
        "account-mfa",
        "shell-mfa",
        "mfa",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_ACCOUNT_CREDENTIALS"),
        "account-sudo",
        "shell-sudo",
        "account/sudo",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_ACCOUNT_EMAIL_CHANGE"),
        "account-email",
        "shell-email",
        "account/security/email",
        Group::Account,
        false,
    );
    registry.section(
        on("OTERYN_ACCOUNT_EXPORT_ENABLED"),
        "account-export",
        "shell-export",
        "account/privacy/export",
        Group::Account,
        false,
    );
    registry.section(
        on("PORTAL_ACCOUNT_TERMINATION_ENABLED"),
        "account-termination",
        "shell-termination",
        "account/privacy/delete",
        Group::Account,
        false,
    );
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
}
