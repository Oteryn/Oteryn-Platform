//! Account creation UX. Requires the public router's session and CSRF middleware.
//! T44 can replace the narrow eligibility seam; uncertain account state always denies.
//! Only name and world cross the accepted command boundary. Profession and sex are
//! not offered until Game accepts these fields in the creation intent.
mod http;
mod names;
mod page;
mod service;
pub use http::{BrowserContext, configured_router, router};
pub use names::{NameError, canonicalize};
pub use service::{Creation, Error, Outcome, Settings, Submission, World};

#[cfg(test)]
mod tests;
pub use http::{Flash, SessionAction};
