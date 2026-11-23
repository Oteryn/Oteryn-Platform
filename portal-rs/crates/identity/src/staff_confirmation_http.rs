//! Password confirmation on the separately guarded staff panel.
use super::{configured_service, rate_limited, show_sudo, sudo};
use axum::{Extension, Router, routing::get};
use sqlx::PgPool;

#[derive(Clone, Copy)]
pub(super) struct StaffConfirmation;

/// Mount these routes through the admin host, MFA, and grant-permission guard.
pub fn routes(pool: PgPool) -> Router {
    let Some(service) = configured_service(pool) else {
        return Router::new();
    };
    rate_limited(
        Router::new()
            .route("/pl/admin/roles/confirm", get(show_sudo).post(sudo))
            .route("/en/admin/roles/confirm", get(show_sudo).post(sudo))
            .layer(Extension(StaffConfirmation)),
        service,
    )
}
