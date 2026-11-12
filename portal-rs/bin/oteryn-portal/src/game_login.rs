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
//!
//! Admission grants (T19c) are issued only with
//! `PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED=true` and `OTERYN_ENV` `testing` or
//! `preprod`; production never loads the signing key. The key
//! comes from the encrypted custody file `PORTAL_NATIVE_ADMISSION_KEY_CUSTODY_FILE`
//! wrapped by `PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE`, trusted in the
//! registry scope of `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` with the
//! witness directory `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`; the TTL
//! is `PORTAL_NATIVE_ADMISSION_GRANT_TTL_SECONDS` (5..=30, default 20) and the
//! route is selected from the `GAME_AUTH_NATIVE_RUNTIME_STATUS_*` read model.
//!
//! Protocol 3 (contract §19) is a separate default-off switch,
//! `PORTAL_NATIVE_PROTOCOL3_ENABLED`, in the same environments. Recovery
//! handles additionally need `PORTAL_NATIVE_RECOVERY_ENABLED` and
//! `OTERYN_ENV=testing` (owner decision U21). The §19.4 `native-recoveries`
//! issuer then signs with its own §19.5 custody,
//! `PORTAL_NATIVE_RECOVERY_KEY_CUSTODY_FILE` wrapped by
//! `PORTAL_NATIVE_RECOVERY_KEY_WRAPPING_KEY_FILE`, trusted in the
//! `existing_actor_recovery` scope of `urn:oteryn:platform:game-recovery`.
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

/// Ticket redemption, the character check and grant issuance behind the
/// suspension guard. Without a live LCFA source the issuer answers
/// `NATIVE_LOGIN_ROUTE_UNAVAILABLE`; without permitted issuance and a signer,
/// `NATIVE_LOGIN_UNAVAILABLE`.
pub fn admission_authority(
    projection: &lcfa::Settings,
    issuance: admission::Issuance,
) -> GuardedAdmission<admission::NativeTickets> {
    GuardedAdmission::new(
        admission::NativeTickets {
            projection_authority: if projection.enabled {
                projection.source_authority.clone()
            } else {
                String::new()
            },
            issuance,
        },
        FEATURES,
    )
}

/// Issuance policy from the environment with its signer when permitted. A
/// signer that fails to load leaves issuance unavailable.
pub fn issuance(pool: PgPool, lookup: impl Fn(&str) -> Option<String>) -> admission::Issuance {
    let mut issuance = admission::Issuance::configured(&lookup);
    issuance.signer =
        admission::configured_signer(pool, &issuance, lookup).unwrap_or_else(|error| {
            tracing::warn!(%error, "native admission grant signer unavailable");
            None
        });
    issuance
}

/// The protocol 3 policy over the issuance's runtime read model (§19.1).
pub fn protocol3(
    issuance: &admission::Issuance,
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<admission::Protocol3> {
    admission::Protocol3::configured(lookup, issuance.runtime.clone())
}

/// The §19.4 recovery issuer, composed only while protocol 3 recovery handles
/// are on (testing only, U21) and issuance is permitted. A recovery signer that
/// fails to load leaves `native-recoveries` unavailable; it never falls back to
/// the fresh admission key.
pub fn recovery_issuer(
    pool: PgPool,
    issuance: &admission::Issuance,
    protocol3: Option<&admission::Protocol3>,
    projection: &lcfa::Settings,
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<admission::RecoveryIssuer> {
    if !matches!(
        protocol3.map(|settings| settings.handles),
        Some(admission::Handles::On { .. })
    ) {
        return None;
    }
    let clock_uncertainty_seconds = admission::clock_uncertainty(&lookup).ok()?;
    let signer =
        admission::configured_recovery_signer(pool, issuance, lookup).unwrap_or_else(|error| {
            tracing::warn!(%error, "native recovery grant signer unavailable");
            None
        });
    Some(admission::RecoveryIssuer {
        signer,
        projection_authority: if projection.enabled {
            projection.source_authority.clone()
        } else {
            String::new()
        },
        ttl_seconds: issuance.ttl_seconds,
        clock_uncertainty_seconds,
    })
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
    let issuance = issuance(pool.clone(), |key| std::env::var(key).ok());
    let signer = issuance.signer.clone();
    let protocol3 = protocol3(&issuance, |key| std::env::var(key).ok());
    let projection = overview::configured_projection();
    let recovery = recovery_issuer(
        pool.clone(),
        &issuance,
        protocol3.as_ref(),
        &projection,
        |key| std::env::var(key).ok(),
    );
    admission::start_private_listener(
        pool,
        Arc::new(admission_accounts()),
        Arc::new(admission_authority(&projection, issuance)),
        signer,
        protocol3,
        recovery,
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
