//! Account creation UX. Requires the public router's session and CSRF middleware.
//! T44 can replace the narrow eligibility seam; uncertain account state always denies.
//! Name, world, sex and vocation cross the command boundary as create intent v2
//! (#1937). In Newhaven the vocation is chosen at creation: the form requires one
//! sex and one vocation, with no default, and refuses the submission otherwise.
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
