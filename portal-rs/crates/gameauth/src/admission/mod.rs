//! Private native login admission. Ticket, account and grant authorities participate
//! in the same transaction; unavailable dependencies never authorize an admission.
mod grant;
mod http;
mod protocol3;
mod recovery;
mod store;
mod tickets;
mod wire;

pub use grant::{
    CLOCK_UNCERTAINTY_SECONDS, DEFAULT_TTL_SECONDS, GRANT_TTL_SECONDS, GRANTS_ENABLED, GrantSigner,
    Issuance, KEY_CUSTODY_FILE, KEY_PURPOSE, KEY_WRAPPING_FILE, KeyringSigner,
    RECOVERY_KEY_CUSTODY_FILE, RECOVERY_KEY_WRAPPING_FILE, WITNESS_DIRECTORY, clock_uncertainty,
    configured_recovery_signer, configured_signer, header, route_names_valid, route_revision,
};
pub use http::{AdmissionConfig, ServiceTokens, router, start_private_listener};
pub use protocol3::{
    HANDLE_LIFETIME_SECONDS, Handles, MAX_HANDLE_LIFETIME_SECONDS, PROTOCOL3_ENABLED, Protocol3,
    RECOVERY_ENABLED, REQUIRE_DEVICE_BINDING, handle_hash,
};
pub use recovery::{MAX_RESPONSE as MAX_RECOVERY_RESPONSE, RecoveryError, RecoveryIssuer};
pub use store::{
    AccountState, AccountStateLookup, AdmissionAuthority, AuthorizedAccount, Future, Unavailable,
};
pub use tickets::NativeTickets;
pub use wire::{AdmissionRequest, Channel, Endpoint, Error, Material, Recovery, channel_key_valid};

#[cfg(test)]
#[path = "protocol3_tests.rs"]
mod protocol3_tests;
#[cfg(test)]
mod tests;
