//! Community discussion boards, with Platform accounts and staff moderation.
mod mail;
mod markdown;
mod model;
mod player;
mod staff;
mod view;

pub use player::configured_router as router;

#[cfg(test)]
mod tests;
