//! Private mutual-TLS listener for `/internal/v1` on the `serve` role (plan §5.3).
//!
//! The route guards in `oteryn-internal-api` trust only a `VerifiedClient`
//! extension. This listener is the one adapter that inserts it on the main
//! router: after a TLS 1.3 handshake whose client certificate chains to the
//! configured CA, from the leaf certificate's single DNS subject alternative
//! name. The public listener never inserts it, and no header can.
//!
//! Configuration (all four or none; a partial set refuses to start):
//! `OTERYN_INTERNAL_TLS_BIND`, `OTERYN_INTERNAL_TLS_CERT_FILE`,
//! `OTERYN_INTERNAL_TLS_KEY_FILE`, `OTERYN_INTERNAL_TLS_CLIENT_CA_FILE`.
use axum::{
    Router,
    extract::{ConnectInfo, Request},
    http::{StatusCode, Version},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use oteryn_internal_api::guard::{Identity, VerifiedClient};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use std::{io::BufReader, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

const KEYS: [&str; 4] = [
    "OTERYN_INTERNAL_TLS_BIND",
    "OTERYN_INTERNAL_TLS_CERT_FILE",
    "OTERYN_INTERNAL_TLS_KEY_FILE",
    "OTERYN_INTERNAL_TLS_CLIENT_CA_FILE",
];
const PREFIX: &str = "/internal/v1/";
const MAX_CONNECTIONS: usize = 64;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

/// Listener settings read from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind: SocketAddr,
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
    pub client_ca_file: PathBuf,
}

impl Config {
    /// `Ok(None)` when no key is set; an error when only some are set or a
    /// value is invalid, so a half-configured deployment never starts silently.
    pub fn from_env() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let values: Vec<Option<String>> = KEYS
            .iter()
            .map(|key| lookup(key).filter(|value| !value.trim().is_empty()))
            .collect();
        if values.iter().all(Option::is_none) {
            return Ok(None);
        }
        let mut present = Vec::with_capacity(KEYS.len());
        for (key, value) in KEYS.iter().zip(values) {
            present
                .push(value.ok_or_else(|| format!("missing internal TLS configuration: {key}"))?);
        }
        let bind = present[0]
            .parse()
            .map_err(|_| "OTERYN_INTERNAL_TLS_BIND must be an address:port")?;
        Ok(Some(Self {
            bind,
            cert_file: PathBuf::from(&present[1]),
            key_file: PathBuf::from(&present[2]),
            client_ca_file: PathBuf::from(&present[3]),
        }))
    }

    /// TLS 1.3 only, client certificate required and chained to the CA, ALPN http/1.1.
    pub fn server_config(&self) -> Result<rustls::ServerConfig, Box<dyn std::error::Error>> {
        let mut roots = rustls::RootCertStore::empty();
        for certificate in certificates(&self.client_ca_file)? {
            roots.add(certificate)?;
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider.clone(),
        )
        .build()?;
        let chain = certificates(&self.cert_file)?;
        let key = PrivateKeyDer::from_pem_file(&self.key_file)?;
        let mut tls = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_client_cert_verifier(verifier)
            .with_single_cert(chain, key)?;
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(tls)
    }
}

fn certificates(
    path: &PathBuf,
) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let certs = CertificateDer::pem_reader_iter(&mut reader).collect::<Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        return Err(format!("no certificate in {}", path.display()).into());
    }
    Ok(certs)
}

/// Identity of a verified leaf certificate: its only DNS SAN. Zero or several
/// DNS names are ambiguous and refused.
pub fn identity(der: &[u8]) -> Option<Identity> {
    use x509_parser::prelude::{FromDer, GeneralName, X509Certificate};
    let (rest, certificate) = X509Certificate::from_der(der).ok()?;
    if !rest.is_empty() {
        return None;
    }
    let names = certificate.subject_alternative_name().ok()??;
    let mut dns = names
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::DNSName(name) => Some(*name),
            _ => None,
        });
    let first = dns.next()?;
    if dns.next().is_some() {
        return None;
    }
    Identity::san(first).ok()
}

/// Bind and serve `app` on the private listener; `None` when not configured.
pub async fn start(
    app: Router,
) -> Result<Option<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
    let Some(config) = Config::from_env()? else {
        return Ok(None);
    };
    let tls = config.server_config()?;
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(bind = %config.bind, "internal mTLS listener");
    Ok(Some(tokio::spawn(serve(listener, tls, app))))
}

/// Accept loop. Each connection carries exactly one verified identity.
pub async fn serve(listener: tokio::net::TcpListener, tls: rustls::ServerConfig, app: Router) {
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let slots = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    loop {
        let Ok((socket, peer)) = listener.accept().await else {
            continue;
        };
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let acceptor = acceptor.clone();
        let app = app.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let Ok(Ok(stream)) =
                tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(socket)).await
            else {
                return;
            };
            let session = stream.get_ref().1;
            if session.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
                || session
                    .alpn_protocol()
                    .is_some_and(|protocol| protocol != b"http/1.1")
            {
                return;
            }
            let Some(identity) = session
                .peer_certificates()
                .and_then(|certs| certs.first())
                .and_then(|leaf| identity(leaf.as_ref()))
            else {
                return;
            };
            let service = hyper_util::service::TowerToHyperService::new(scoped(
                app,
                VerifiedClient::from_verified_tls13_http11(identity),
                peer,
            ));
            let connection = hyper::server::conn::http1::Builder::new()
                .max_buf_size(65536)
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service);
            let _ = tokio::time::timeout(CONNECTION_TIMEOUT, connection).await;
        });
    }
}

/// Only `/internal/v1/` over HTTP/1.1; everything else is 404 on this port.
fn scoped(app: Router, client: VerifiedClient, peer: SocketAddr) -> Router {
    app.layer(middleware::from_fn(
        move |mut request: Request, next: Next| {
            let client = client.clone();
            async move {
                if request.version() != Version::HTTP_11 {
                    return StatusCode::BAD_REQUEST.into_response();
                }
                if !request.uri().path().starts_with(PREFIX) {
                    return StatusCode::NOT_FOUND.into_response();
                }
                request.extensions_mut().insert(client);
                request.extensions_mut().insert(ConnectInfo(peer));
                let response: Response = next.run(request).await;
                response
            }
        },
    ))
}

#[cfg(test)]
#[path = "internal_tls_tests.rs"]
mod tests;
