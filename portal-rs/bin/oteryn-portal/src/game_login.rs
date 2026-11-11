//! Production composition of the native game login path (plan §6, T17, T18,
//! T19, T34): OAuth access token -> ticket -> native admission, the client's
//! character list and the LCFA ingestion on the internal mTLS listener.
//!
//! Every account check goes through the sanctions guard, so a suspended,
//! closed or unknown account gets no ticket, no character list and no
//! admission. Each surface keeps its own default-off flag:
//! `PORTAL_GAME_TICKETS_ENABLED` (tickets and native characters),
//! `PORTAL_NATIVE_ADMISSIONS_ENABLED` (admission), `OTERYN_LCFA_ENABLED` (the
//! character projection) and `PORTAL_LCFA_INTERNAL_IDENTITY` (LCFA ingestion on
//! the internal listener).
use axum::Router;
use oteryn_characters::{lcfa, overview};
use oteryn_gameauth::{admission, devices::PlatformAuthority, oauth, tickets};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap};
use oteryn_support::sanctions::{AdmissionAccounts, Features, GuardedAdmission, GuardedTickets};
use sqlx::PgPool;
use std::sync::Arc;

/// DNS SAN of the Game LCFA producer certificate on the internal listener.
pub const LCFA_IDENTITY: &str = "PORTAL_LCFA_INTERNAL_IDENTITY";

/// Enforcement is not optional on this path: game access always runs the
/// sanctions check; the surfaces themselves stay behind their own flags.
const FEATURES: Features = Features {
    sanctions: false,
    game_access: true,
};

/// The T16 OAuth verifier (native client allowlist, generations, consumption)
/// behind the suspension guard.
pub fn ticket_authority(config: oauth::Config) -> Arc<dyn tickets::OAuthAuthority> {
    Arc::new(GuardedTickets::new(
        PlatformAuthority {
            oauth: config,
            projection_authority: String::new(),
        },
        FEATURES,
    ))
}

/// Ticket redemption and the character check behind the suspension guard.
/// Without a live LCFA source the issuer answers `NATIVE_LOGIN_ROUTE_UNAVAILABLE`.
pub fn admission_authority(
    projection: &lcfa::Settings,
) -> GuardedAdmission<admission::NativeTickets> {
    GuardedAdmission::new(
        admission::NativeTickets {
            projection_authority: if projection.enabled {
                projection.source_authority.clone()
            } else {
                String::new()
            },
        },
        FEATURES,
    )
}

pub fn admission_accounts() -> AdmissionAccounts {
    AdmissionAccounts(FEATURES)
}

/// `POST /api/v1/game-auth/tickets` and `GET /api/v1/game-auth/native-characters`.
pub fn player_router(pool: PgPool) -> Router {
    oteryn_player_api::configured_router(
        pool.clone(),
        ticket_authority(oauth::Config::from_env()),
        Arc::new(overview::Accounts(pool)),
    )
}

/// Loopback `native-admissions` listener for the login gateway.
pub async fn start_admission_listener(
    pool: PgPool,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    admission::start_private_listener(
        pool,
        Arc::new(admission_accounts()),
        Arc::new(admission_authority(&overview::configured_projection())),
    )
    .await
}

/// LCFA snapshot and watermark ingestion for the internal mTLS listener, keyed
/// to its own certificate identity; `None` when the identity is not set.
pub fn lcfa_router(
    pool: PgPool,
    identity: Option<String>,
    settings: lcfa::Settings,
) -> Result<Option<Router>, Box<dyn std::error::Error>> {
    let Some(identity) = identity.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let identities = IdentityMap::new(vec![Binding {
        identity: Identity::san(&identity).map_err(|_| format!("invalid {LCFA_IDENTITY}"))?,
        group: "native-account-characters".into(),
    }])
    .map_err(|_| format!("invalid {LCFA_IDENTITY}"))?;
    let projection =
        lcfa::Projection::new(pool.clone(), settings, Arc::new(overview::Accounts(pool)));
    Ok(Some(oteryn_web::internal_character_router(
        projection,
        Arc::new(identities),
    )))
}

#[cfg(test)]
#[path = "game_login_tests.rs"]
mod tests;
