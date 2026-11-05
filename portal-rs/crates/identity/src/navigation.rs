//! Account section navigation, using existing feature flags.
use oteryn_ui::{Group, Registry};
/// Register enabled account sections; no route or authentication behavior changes.
pub fn register(registry: &mut Registry) {
    registry.section(
        std::env::var("OTERYN_ACCOUNT_PRIVACY_ENABLED").as_deref() == Ok("true"),
        "account-privacy",
        "shell-account-privacy",
        "account/privacy",
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
        std::env::var("OTERYN_ACCOUNT_SESSIONS").as_deref() == Ok("true"),
        "account-sessions",
        "shell-account-sessions",
        "account/sessions",
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
}
