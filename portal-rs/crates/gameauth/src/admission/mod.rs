//! Private native login admission. Ticket, account and grant authorities participate
//! in the same transaction; unavailable dependencies never authorize an admission.
mod grant;
mod http;
mod store;
mod tickets;
mod wire;

pub use grant::{
    CLOCK_UNCERTAINTY_SECONDS, DEFAULT_TTL_SECONDS, GRANT_TTL_SECONDS, GRANTS_ENABLED, GrantSigner,
    Issuance, KEY_CUSTODY_FILE, KEY_PURPOSE, KEY_WRAPPING_FILE, KeyringSigner, WITNESS_DIRECTORY,
    configured_signer, header, route_names_valid, route_revision,
};
pub use http::{AdmissionConfig, ServiceTokens, router, start_private_listener};
pub use store::{
    AccountState, AccountStateLookup, AdmissionAuthority, AuthorizedAccount, Future, Unavailable,
};
pub use tickets::NativeTickets;
pub use wire::{AdmissionRequest, Endpoint, Error, Material};

#[cfg(test)]
mod tests;
