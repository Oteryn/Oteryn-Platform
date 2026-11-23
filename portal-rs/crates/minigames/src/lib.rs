pub mod collection;
pub mod collection_pvp;
pub mod pass;
pub mod puzzles;

/// Shared portal navigation registration.
pub mod navigation;

pub mod league;

mod browser;

/// Apply after shared browser/session middleware so it is included in the
/// minigame operation budget. Other modules and internal feeds retain their policy.
pub fn with_browser_deadlines(app: axum::Router) -> axum::Router {
    app.layer(axum::middleware::from_fn(browser::operation_deadline))
}

fn staging_samples() -> bool {
    std::env::var("OTERYN_ENV").as_deref() == Ok("testing")
        && std::env::var("PORTAL_STAGING_SAMPLE_DATA").as_deref() == Ok("true")
}
