//! Production composition shared by serve and worker. Game pulls the private CC
//! surface; Platform never guesses a Game push URL or performs I/O under locks.
use axum::{
    extract::Request,
    middleware::{self, Next},
};
use oteryn_identity::termination::{
    self,
    game_closure::{self, runtime},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{io::BufReader, sync::Arc, time::Duration};
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
    let game = config.boundary(
        Arc::new(crate::account_lifecycle_http::TerminationBoundary),
        termination.enabled,
    )?;
    let boundary: Arc<dyn termination::Boundary> = game.as_ref().map_or_else(
        || Arc::new(termination::UnavailableBoundary) as Arc<dyn termination::Boundary>,
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
fn certificates(path: &str) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let certs =
        CertificateDer::pem_reader_iter(&mut reader).collect::<std::result::Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        return Err("empty closure TLS certificate chain".into());
    }
    Ok(certs)
}
/// Dedicated TLS 1.3 + HTTP/1.1 listener. Certificates are CA-validated AND
/// exact-leaf pinned. A public header cannot populate VerifiedClient.
pub async fn start_listener(
    pool: PgPool,
    runtime: &Runtime,
) -> Result<Option<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
    let Some(game) = &runtime.game else {
        return Ok(None);
    };
    let url: axum::http::Uri = required("PORTAL_ACCOUNT_CLOSURE_BASE_URL")?.parse()?;
    if url.scheme_str() != Some("https") || url.authority().is_none() || url.path() != "/" {
        return Err("account closure base URL must be an HTTPS origin".into());
    }
    let ca = certificates(&required("PORTAL_ACCOUNT_CLOSURE_TLS_CA_FILE")?)?;
    let mut roots = rustls::RootCertStore::empty();
    for certificate in ca {
        roots.add(certificate)?;
    }
    let chain = certificates(&required("PORTAL_ACCOUNT_CLOSURE_TLS_CERT_FILE")?)?;
    let key = PrivateKeyDer::from_pem_file(required("PORTAL_ACCOUNT_CLOSURE_TLS_KEY_FILE")?)?;
    let tls = tls_config(roots, chain, key)?;
    let command_pin = runtime::decode_hex(&required("PORTAL_ACCOUNT_CLOSURE_CLIENT_CERT_SHA256")?)?;
    let lcfa_pin =
        runtime::decode_hex(&required("PORTAL_ACCOUNT_CLOSURE_LCFA_CLIENT_CERT_SHA256")?)?;
    if command_pin.len() != 32 || lcfa_pin.len() != 32 || command_pin == lcfa_pin {
        return Err("closure identities must be distinct SHA-256 certificate pins".into());
    }
    let ([character, portfolio], identities) = purpose_identities()?;
    let app = routes(
        pool,
        game.clone(),
        &runtime.config.key_id,
        identities,
        oteryn_characters::lcfa::Settings {
            enabled: std::env::var("OTERYN_LCFA_ENABLED").as_deref() == Ok("true"),
            environment: oteryn_core::Config::from_env()?.environment,
            source_authority: runtime.config.authority.clone(),
            ..Default::default()
        },
    );
    let bind =
        std::env::var("PORTAL_ACCOUNT_CLOSURE_BIND").unwrap_or_else(|_| "127.0.0.1:8092".into());
    let listener = tokio::net::TcpListener::bind(bind).await?;
    Ok(Some(serve(
        listener,
        tls,
        app,
        [command_pin, lcfa_pin],
        [character, portfolio],
    )))
}

fn tls_config(
    roots: rustls::RootCertStore,
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<rustls::ServerConfig, Box<dyn std::error::Error>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()?;
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(verifier)
        .with_single_cert(chain, key)?;
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(tls)
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

fn serve(
    listener: tokio::net::TcpListener,
    tls: rustls::ServerConfig,
    app: axum::Router,
    [command_pin, lcfa_pin]: [Vec<u8>; 2],
    [character, portfolio]: [Identity; 2],
) -> tokio::task::JoinHandle<()> {
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let slots = Arc::new(tokio::sync::Semaphore::new(16));
    tokio::spawn(async move {
        loop {
            let (socket, _) = match accept_with_retry(
                || listener.accept(),
                |error| eprintln!("account-closure private listener accept failed: {error}"),
            )
            .await
            {
                Ok(connection) => connection,
                Err(_) => return,
            };
            let Ok(permit) = slots.clone().try_acquire_owned() else {
                continue;
            };
            let acceptor = acceptor.clone();
            let app = app.clone();
            let command_pin = command_pin.clone();
            let lcfa_pin = lcfa_pin.clone();
            let character = character.clone();
            let portfolio = portfolio.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let Ok(Ok(tls)) =
                    tokio::time::timeout(Duration::from_secs(2), acceptor.accept(socket)).await
                else {
                    return;
                };
                if tls
                    .get_ref()
                    .1
                    .alpn_protocol()
                    .is_some_and(|protocol| protocol != b"http/1.1")
                {
                    return;
                }
                let Some(cert) = tls
                    .get_ref()
                    .1
                    .peer_certificates()
                    .and_then(|certs| certs.first())
                else {
                    return;
                };
                let pin = Sha256::digest(cert.as_ref()).to_vec();
                let identity = if pin == command_pin {
                    character
                } else if pin == lcfa_pin {
                    portfolio
                } else {
                    return;
                };
                let client = VerifiedClient::from_verified_tls13_http11(identity);
                let app = app.layer(middleware::from_fn(
                    move |mut request: Request, next: Next| {
                        let client = client.clone();
                        async move {
                            if request.version() != axum::http::Version::HTTP_11 {
                                return oteryn_internal_api::guard::failure(
                                    axum::http::StatusCode::BAD_REQUEST,
                                );
                            }
                            request.extensions_mut().insert(client);
                            next.run(request).await
                        }
                    },
                ));
                let service = hyper_util::service::TowerToHyperService::new(app);
                let connection = hyper::server::conn::http1::Builder::new()
                    .keep_alive(false)
                    .max_buf_size(32768)
                    .serve_connection(hyper_util::rt::TokioIo::new(tls), service);
                let _ = tokio::time::timeout(Duration::from_secs(3), connection).await;
            });
        }
    })
}

// Accept failures (including descriptor exhaustion) must not silently remove a
// private surface. Retry with capped backoff; sustained failure completes the
// task so the process supervisor can fail the process.
async fn accept_with_retry<T, F, Fut, Report>(
    mut accept: F,
    mut report: Report,
) -> std::io::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::io::Result<T>>,
    Report: FnMut(&std::io::Error),
{
    let mut delay = std::time::Duration::from_millis(25);
    for attempt in 0..8 {
        match accept().await {
            Ok(connection) => return Ok(connection),
            Err(error) => {
                report(&error);
                if attempt == 7 {
                    return Err(error);
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(std::time::Duration::from_secs(1));
            }
        }
    }
    unreachable!("accept loop returns on its final attempt")
}

#[cfg(test)]
mod accept_tests {
    use super::accept_with_retry;

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
