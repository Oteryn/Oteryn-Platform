//! Accounts, passwords, 2FA, passkeys, recovery, e-mail verification and privacy (design T03–T09).

#[cfg(test)]
#[path = "../../../test-support/rate_clock.rs"]
mod rate_clock;

pub mod accounts;
pub mod consents;
pub mod password;
pub mod register;
pub mod verify;
pub use accounts::AccountId;
pub mod abuse;
pub mod breach;

pub mod credentials;
pub mod email_change;
pub mod export;
pub mod login;
pub mod mfa;
pub mod passkeys;
pub mod reset;
pub mod security_wizard;
pub mod termination;

pub mod privacy;
pub mod sessions;

/// Shared portal navigation registration.
pub mod navigation;

pub mod recovery_key;
pub mod recovery_key_http;

/// Account security revocation stream (internal mTLS) and the staff kick seam.
pub mod security_revocations;

#[cfg(test)]
mod oauth_mfa_web_tests;

#[cfg(test)]
mod native_recovery_test_support;

pub mod mutation_actor;
pub mod newsletter_mail;
pub mod notification_preferences;
