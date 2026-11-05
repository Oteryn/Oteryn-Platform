//! Private native login admission. Ticket, account and grant authorities participate
//! in the same transaction; unavailable dependencies never authorize an admission.
mod http;
mod store;
mod wire;

pub use http::{AdmissionConfig, ServiceTokens, router, start_private_listener};
pub use store::{
    AccountState, AccountStateLookup, AdmissionAuthority, AuthorizedAccount, Future, Unavailable,
};
pub use wire::{AdmissionRequest, Endpoint, Error, Material};

#[cfg(test)]
mod tests;
