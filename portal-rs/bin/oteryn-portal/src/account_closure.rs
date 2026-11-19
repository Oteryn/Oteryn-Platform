//! Production composition shared by serve and worker. Game pulls the private CC
//! surface; Platform never guesses a Game push URL or performs I/O under locks.
use axum::Router;
use oteryn_identity::termination::{
    self,
    game_closure::{self, runtime},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap};
use sqlx::PgPool;
use std::sync::Arc;
pub struct Runtime {
    pub terminations: termination::Terminations,
    game: Option<Arc<game_closure::GameBoundary>>,
    config: runtime::Config,
}
impl Runtime {
    pub fn game_enabled(&self) -> bool {
        self.game.is_some()
    }
    pub fn worker_interval_seconds(&self) -> i32 {
        if self.game.is_some() {
            self.config.worker_interval_seconds
        } else {
            60
        }
    }
}
pub fn configured(pool: PgPool) -> Result<Runtime, Box<dyn std::error::Error>> {
    let termination = termination::Config::from_env()?;
    let config = runtime::Config::from_env()?;
    // Continue receiving Game proof for accepted closures while intake is off.
    let game = config.boundary(
        Arc::new(crate::account_lifecycle_http::TerminationBoundary),
        true,
    )?;
    let boundary: Arc<dyn termination::Boundary> = game.as_ref().map_or_else(
        || {
            Arc::new(crate::account_lifecycle_http::TerminationBoundary)
                as Arc<dyn termination::Boundary>
        },
        |game| game.clone(),
    );
    let terminations = termination::Terminations::new(pool, termination, boundary)?;
    Ok(Runtime {
        terminations,
        game,
        config,
    })
}
fn required(key: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::var(key).map_err(|_| format!("missing account closure configuration: {key}").into())
}
pub struct Ingress {
    pub router: Router,
    pins: [[u8; 32]; 2],
    identities: [Identity; 2],
}
impl Ingress {
    fn new(router: Router, pins: [[u8; 32]; 2], identities: [Identity; 2]) -> Self {
        Self {
            router,
            pins,
            identities,
        }
    }

    pub fn identity(&self, leaf_sha256: &[u8; 32]) -> Option<Identity> {
        self.pins
            .iter()
            .zip(&self.identities)
            .find_map(|(pin, identity)| (pin == leaf_sha256).then(|| identity.clone()))
    }
}

pub fn private_ingress(
    pool: PgPool,
    runtime: &Runtime,
    environment: oteryn_core::Environment,
) -> Result<Option<Ingress>, Box<dyn std::error::Error>> {
    let Some(game) = &runtime.game else {
        return Ok(None);
    };
    let url: axum::http::Uri = required("PORTAL_ACCOUNT_CLOSURE_BASE_URL")?.parse()?;
    if url.scheme_str() != Some("https")
        || url.authority().is_none()
        || url.path() != "/"
        || url.query().is_some()
    {
        return Err("account closure base URL must be an HTTPS origin".into());
    }
    let pin = |key| -> Result<[u8; 32], Box<dyn std::error::Error>> {
        runtime::decode_hex(&required(key)?)?
            .try_into()
            .map_err(|_| format!("{key} must be a SHA-256 certificate pin").into())
    };
    let command_pin = pin("PORTAL_ACCOUNT_CLOSURE_CLIENT_CERT_SHA256")?;
    let lcfa_pin = pin("PORTAL_ACCOUNT_CLOSURE_LCFA_CLIENT_CERT_SHA256")?;
    if command_pin == lcfa_pin {
        return Err("closure identities must be distinct SHA-256 certificate pins".into());
    }
    let (purpose_ids, identities) = purpose_identities()?;
    let app = routes(
        pool,
        game.clone(),
        &runtime.config.key_id,
        identities,
        closure_projection(environment),
    );
    #[cfg(test)]
    crate::record_route_registrations("closure", &app)?;
    Ok(Some(Ingress::new(
        app,
        [command_pin, lcfa_pin],
        purpose_ids,
    )))
}

fn closure_projection(environment: oteryn_core::Environment) -> oteryn_characters::lcfa::Settings {
    oteryn_characters::lcfa::Settings {
        environment,
        ..oteryn_characters::overview::configured_projection()
    }
}
#[test]
fn r7_closure_uses_feed_authority() -> Result<(), Box<dyn std::error::Error>> {
    const TEST: &str = "account_closure::r7_closure_uses_feed_authority";
    if std::env::var_os("R7_CLOSURE_TEST_CHILD").is_some() {
        assert_eq!(
            closure_projection(oteryn_core::Environment::Testing).source_authority,
            "main-feed"
        );
    } else {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", TEST])
            .env("R7_CLOSURE_TEST_CHILD", "1")
            .env("OTERYN_LCFA_SOURCE_AUTHORITY", "main-feed")
            .env("PORTAL_ACCOUNT_CLOSURE_SOURCE_AUTHORITY", "closure-only")
            .status()?;
        assert!(status.success());
    }
    Ok(())
}

fn purpose_identities() -> Result<([Identity; 2], Arc<IdentityMap>), Box<dyn std::error::Error>> {
    let character = Identity::san("closure-character-authority")?;
    let portfolio = Identity::san("closure-portfolio-authority")?;
    let identities = Arc::new(IdentityMap::new(vec![
        Binding {
            identity: character.clone(),
            group: game_closure::http::GROUP.into(),
        },
        Binding {
            identity: portfolio.clone(),
            group: "native-account-characters".into(),
        },
    ])?);
    Ok(([character, portfolio], identities))
}

fn routes(
    pool: PgPool,
    game: Arc<game_closure::GameBoundary>,
    key_id: &str,
    identities: Arc<IdentityMap>,
    settings: oteryn_characters::lcfa::Settings,
) -> axum::Router {
    let closure = game_closure::http::router(pool.clone(), game, key_id.into(), identities.clone());
    let lcfa = oteryn_characters::lcfa::Projection::new(
        pool,
        settings,
        Arc::new(oteryn_characters::lcfa::DenyAccounts),
    );
    closure.merge(oteryn_characters::lcfa::router(lcfa, identities))
}

#[cfg(test)]
mod accept_tests {
    use oteryn_gameauth::accept::accept_with_retry;

    #[tokio::test]
    async fn recoverable_accept_error_is_recorded_and_next_connection_is_served()
    -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let client = tokio::net::TcpStream::connect(listener.local_addr()?).await?;
        let mut first = true;
        let mut recorded = Vec::new();
        let (mut socket, _) = accept_with_retry(
            || {
                let fail = std::mem::take(&mut first);
                let listener = &listener;
                async move {
                    if fail {
                        Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted))
                    } else {
                        listener.accept().await
                    }
                }
            },
            |error| recorded.push(error.kind()),
        )
        .await?;
        assert_eq!(recorded, [std::io::ErrorKind::ConnectionAborted]);
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        socket.write_all(b"served").await?;
        let mut bytes = [0; 6];
        let mut client = client;
        client.read_exact(&mut bytes).await?;
        assert_eq!(&bytes, b"served");
        Ok(())
    }

    #[tokio::test]
    async fn sustained_accept_failure_returns_to_supervisor() {
        let mut recorded = 0;
        let result = accept_with_retry(
            || async { Err::<(), _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied)) },
            |_| recorded += 1,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(recorded, 8);
    }
}

#[cfg(test)]
#[path = "account_closure_tls_tests.rs"]
mod tls_tests;
