//! Client release channels and TUF metadata (design T35).

mod domain;
mod http;
mod storage;
mod verification;

pub use domain::*;
pub use http::router;
pub use storage::Downloads;

pub mod command;
