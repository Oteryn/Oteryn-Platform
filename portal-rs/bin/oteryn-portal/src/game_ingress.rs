//! Game publishers with certificate pins share the internal TLS transport.
//! Their routers remain separate where a character path has multiple handlers.
use crate::account_closure;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    http::StatusCode,
    response::Response,
};
use oteryn_bazaar::game::runtime::Runtime as Bazaar;
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, Identity, VerifiedClient},
};
use oteryn_minigames::league::config::{self as league, Config as League};
use rustls::pki_types::CertificateDer;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;

const CHARACTER_COMMANDS: &str = "/internal/v1/game-auth/character-commands/";
const CLOSURE: &str = "/internal/v1/game-auth/account-closure";
const BAZAAR_EVENT: &str = "/internal/v1/game-events";
const BAZAAR_RECEIPT_BYTES: usize = 270336;

#[derive(Default)]
pub struct GameIngress {
    closure: Option<account_closure::Ingress>,
    bazaar: Option<BazaarIngress>,
    league: Option<LeagueIngress>,
}

struct BazaarIngress {
    router: Router,
    commands: [u8; 32],
    receipts: [u8; 32],
}

struct LeagueIngress {
    router: Router,
    spki: [u8; 32],
}

#[derive(Clone, Copy)]
pub struct Peer {
    certificate_sha256: [u8; 32],
    spki_sha256: [u8; 32],
    http11_alpn: bool,
}

impl Peer {
    pub fn new(leaf: &CertificateDer<'static>, http11_alpn: bool) -> Option<Self> {
        use x509_parser::prelude::{FromDer, X509Certificate};
        let (rest, cert) = X509Certificate::from_der(leaf.as_ref()).ok()?;
        if !rest.is_empty() {
            return None;
        }
        Some(Self {
            certificate_sha256: Sha256::digest(leaf.as_ref()).into(),
            spki_sha256: Sha256::digest(cert.public_key().raw).into(),
            http11_alpn,
        })
    }
}

impl GameIngress {
    pub fn configured(
        db: PgPool,
        closure: Option<account_closure::Ingress>,
        bazaar: Option<&Bazaar>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bazaar = bazaar
            .map(|runtime| -> Result<_, Box<dyn std::error::Error>> {
                let router = runtime.shared_private_router()?;
                #[cfg(test)]
                crate::record_route_registrations("bazaar", &router)?;
                Ok(BazaarIngress {
                    router,
                    commands: runtime.config.bazaar_spki,
                    receipts: runtime.config.receipt_spki,
                })
            })
            .transpose()?;
        let league = if league::enabled() {
            // Incomplete deployment trust keeps league ingestion unavailable.
            League::from_env()
                .ok()
                .map(|config| -> Result<_, Box<dyn std::error::Error>> {
                    let spki = config.client_spki_pin()?;
                    let router = config.shared_private_router(db)?;
                    #[cfg(test)]
                    crate::record_route_registrations("league", &router)?;
                    Ok(LeagueIngress { router, spki })
                })
                .transpose()?
        } else {
            None
        };
        Ok(Self {
            closure,
            bazaar,
            league,
        })
    }

    #[cfg(test)]
    pub fn closure_only(closure: account_closure::Ingress) -> Self {
        Self {
            closure: Some(closure),
            ..Self::default()
        }
    }

    pub fn is_configured(&self) -> bool {
        self.closure.is_some() || self.bazaar.is_some() || self.league.is_some()
    }

    /// Recognize certificates that authenticate by pin alone and have no DNS SAN.
    pub fn recognizes(&self, peer: &Peer) -> bool {
        self.closure_identity(peer).is_some()
            || peer.http11_alpn
                && (self.bazaar.as_ref().is_some_and(|bazaar| {
                    [bazaar.commands, bazaar.receipts].contains(&peer.spki_sha256)
                }) || self
                    .league
                    .as_ref()
                    .is_some_and(|league| league.spki == peer.spki_sha256))
    }

    fn closure_identity(&self, peer: &Peer) -> Option<Identity> {
        self.closure.as_ref()?.identity(&peer.certificate_sha256)
    }

    pub async fn dispatch(
        &self,
        app: Router,
        peer: Option<&Peer>,
        mut request: Request,
    ) -> Response {
        let command_path = request.uri().path().strip_prefix(CHARACTER_COMMANDS);
        let is_character_command = matches!(command_path, Some("pending" | "read" | "receipt"));
        let is_portfolio = matches!(
            request.uri().path(),
            "/internal/v1/game-auth/native-account-characters"
                | "/internal/v1/game-auth/native-account-characters/watermark"
        );
        let closure_identity = peer.and_then(|peer| self.closure_identity(peer));
        let spki = peer
            .filter(|peer| peer.http11_alpn)
            .map(|peer| peer.spki_sha256);

        let bazaar_receipt_peer = self
            .bazaar
            .as_ref()
            .is_some_and(|bazaar| spki == Some(bazaar.receipts));
        // Game can use one Character certificate for closure and Bazaar receipts.
        // Only a peer proven for both may select between these two handlers using
        // the command in the bounded body. Both retain their own full validation.
        let use_closure_command = if closure_identity.is_some()
            && bazaar_receipt_peer
            && matches!(command_path, Some("read" | "receipt"))
        {
            let limit = if command_path == Some("read") {
                512
            } else {
                BAZAAR_RECEIPT_BYTES
            };
            let (parts, body) = request.into_parts();
            let Ok(body) = to_bytes(body, limit).await else {
                return guard::failure(StatusCode::PAYLOAD_TOO_LARGE);
            };
            let is_closure = is_closure_command(&body, limit);
            request = Request::from_parts(parts, Body::from(body));
            is_closure
        } else {
            true
        };
        let path = request.uri().path();
        let (router, identity) = if let Some(closure) = &self.closure
            && (path == CLOSURE
                || matches!(
                    path.strip_prefix(CLOSURE),
                    Some("/watermark" | "/lifecycle/read")
                )
                || (is_portfolio || is_character_command && use_closure_command)
                    && closure_identity.is_some())
        {
            (&closure.router, closure_identity)
        } else if let Some(bazaar) = &self.bazaar
            && (path.starts_with("/internal/v1/bazaar/")
                || path == BAZAAR_EVENT
                || is_character_command && bazaar_receipt_peer)
        {
            let identity = spki
                .filter(|spki| [bazaar.commands, bazaar.receipts].contains(spki))
                .map(Identity::spki_sha256);
            (&bazaar.router, identity)
        } else if let Some(league) = &self.league
            && league::INGRESS_PATHS.contains(&path)
        {
            if let Some(spki) = spki.filter(|spki| league.spki == *spki) {
                request
                    .extensions_mut()
                    .insert(league::VerifiedPublisher::from_verified_tls13_http11(spki));
            }
            return league
                .router
                .clone()
                .oneshot(request)
                .await
                .unwrap_or_else(|never| match never {});
        } else {
            return app
                .oneshot(request)
                .await
                .unwrap_or_else(|never| match never {});
        };
        // Purpose handlers consume only the proof selected for their own router.
        // In particular a SAN can never impersonate closure's synthetic identity.
        request.extensions_mut().remove::<VerifiedClient>();
        let Some(identity) = identity else {
            return guard::failure(StatusCode::UNAUTHORIZED);
        };
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(identity));
        router
            .clone()
            .oneshot(request)
            .await
            .unwrap_or_else(|never| match never {})
    }
}

fn is_closure_command(body: &[u8], limit: usize) -> bool {
    canonical::decode(body, limit, 12).is_ok_and(|value| {
        matches!(
            value.get("command"),
            Some(Value::String(command))
                if matches!(command.as_str(), "CloseAccount" | "ResolveAccountClosure")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_character_peer_routes_only_closure_commands_to_closure() {
        for (command, closure) in [
            ("CloseAccount", true),
            ("ResolveAccountClosure", true),
            ("TransferCharacterOwnership", false),
            ("LockBazaarCharacter", false),
        ] {
            let body = format!(r#"{{"contract_version":2,"command":"{command}"}}"#);
            assert_eq!(is_closure_command(body.as_bytes(), 512), closure);
        }
        assert!(!is_closure_command(
            br#"{"command":"CloseAccount","command":"LockBazaarCharacter"}"#,
            512,
        ));
    }
}
