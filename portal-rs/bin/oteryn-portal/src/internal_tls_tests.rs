use super::*;
use axum::{
    Extension,
    routing::{get, post},
};
use oteryn_internal_api::guard::{self, Binding, IdentityMap, JsonRule, RoutePolicy};
use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PEER: &str = "game-peer.oteryn.internal";

struct Authority {
    cert: rcgen::Certificate,
    key: KeyPair,
}
impl Authority {
    fn new() -> Result<Self, rcgen::Error> {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let cert = params.self_signed(&key)?;
        Ok(Self { cert, key })
    }
    fn issue(
        &self,
        names: &[&str],
        usage: ExtendedKeyUsagePurpose,
    ) -> Result<(String, String), rcgen::Error> {
        let key = KeyPair::generate()?;
        let mut params =
            CertificateParams::new(names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>())?;
        params.extended_key_usages = vec![usage];
        let cert = params.signed_by(&key, &self.cert, &self.key)?;
        Ok((cert.pem(), key.serialize_pem()))
    }
}

fn lookup(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = values
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

#[test]
fn config_is_all_or_nothing() -> TestResult {
    assert_eq!(Config::from_lookup(lookup(&[]))?, None);
    assert_eq!(
        Config::from_lookup(lookup(&[("OTERYN_INTERNAL_TLS_BIND", "  ")]))?,
        None
    );
    assert!(
        Config::from_lookup(lookup(&[("OTERYN_INTERNAL_TLS_BIND", "127.0.0.1:8543")])).is_err()
    );
    let full = [
        ("OTERYN_INTERNAL_TLS_BIND", "0.0.0.0:8543"),
        ("OTERYN_INTERNAL_TLS_CERT_FILE", "/run/tls/cert.pem"),
        ("OTERYN_INTERNAL_TLS_KEY_FILE", "/run/tls/key.pem"),
        ("OTERYN_INTERNAL_TLS_CLIENT_CA_FILE", "/run/tls/ca.pem"),
    ];
    let config = Config::from_lookup(lookup(&full))?.ok_or("expected config")?;
    assert_eq!(config.bind, "0.0.0.0:8543".parse()?);
    assert_eq!(config.client_ca_file, PathBuf::from("/run/tls/ca.pem"));
    let mut bad = full;
    bad[0].1 = "not-an-address";
    assert!(Config::from_lookup(lookup(&bad)).is_err());
    Ok(())
}

#[test]
fn identity_requires_exactly_one_dns_name() -> TestResult {
    let ca = Authority::new()?;
    let der = |pem: &str| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(CertificateDer::from_pem_slice(pem.as_bytes())?.to_vec())
    };
    let (one, _) = ca.issue(&[PEER], ExtendedKeyUsagePurpose::ClientAuth)?;
    assert!(identity(&der(&one)?).is_some());
    let (two, _) = ca.issue(
        &[PEER, "other.oteryn.internal"],
        ExtendedKeyUsagePurpose::ClientAuth,
    )?;
    assert!(identity(&der(&two)?).is_none());
    let (none, _) = ca.issue(&[], ExtendedKeyUsagePurpose::ClientAuth)?;
    assert!(identity(&der(&none)?).is_none());
    assert!(identity(b"not a certificate").is_none());
    Ok(())
}

/// A real guarded route, as module crates mount them, plus a public page.
fn app() -> Result<Router, guard::ConfigError> {
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: Identity::san(PEER)?,
        group: "test".into(),
    }])?);
    let guarded = guard::protect(
        Router::new().route("/internal/v1/test", post(|| async { "accepted" })),
        identities,
        RoutePolicy {
            enabled: true,
            group: "test".into(),
            max_body_bytes: 1024,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    );
    Ok(guarded
        .route(
            "/internal/v1/peer",
            get(
                |ConnectInfo(peer): ConnectInfo<SocketAddr>,
                 client: Option<Extension<VerifiedClient>>| async move {
                    format!("{} {}", peer.ip(), client.is_some())
                },
            ),
        )
        .route("/pl/", get(|| async { "public" })))
}

struct Fixture {
    _dir: tempfile::TempDir,
    addr: SocketAddr,
    ca_pem: String,
    ca: Authority,
}

async fn listener() -> Result<Fixture, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let ca = Authority::new()?;
    let (cert, key) = ca.issue(&["localhost"], ExtendedKeyUsagePurpose::ServerAuth)?;
    let write = |name: &str, body: &str| -> std::io::Result<PathBuf> {
        let path = dir.path().join(name);
        std::fs::write(&path, body)?;
        Ok(path)
    };
    let ca_pem = ca.cert.pem();
    let config = Config {
        bind: "127.0.0.1:0".parse()?,
        cert_file: write("cert.pem", &cert)?,
        key_file: write("key.pem", &key)?,
        client_ca_file: write("ca.pem", &ca_pem)?,
    };
    let tls = config.server_config()?;
    let socket = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = socket.local_addr()?;
    tokio::spawn(serve(socket, tls, app()?));
    Ok(Fixture {
        _dir: dir,
        addr,
        ca_pem,
        ca,
    })
}

/// Raw HTTP/1.1 over TLS 1.3; `None` when the handshake or exchange is refused.
async fn call(
    fixture: &Fixture,
    client: Option<(&str, &str)>,
    request: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in CertificateDer::pem_slice_iter(fixture.ca_pem.as_bytes()) {
        roots.add(cert?)?;
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots);
    let mut config = match client {
        Some((cert, key)) => builder.with_client_auth_cert(
            CertificateDer::pem_slice_iter(cert.as_bytes()).collect::<Result<Vec<_>, _>>()?,
            PrivateKeyDer::from_pem_slice(key.as_bytes())?,
        )?,
        None => builder.with_no_client_auth(),
    };
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let tcp = tokio::net::TcpStream::connect(fixture.addr).await?;
    let name = rustls::pki_types::ServerName::try_from("localhost")?;
    let Ok(mut tls) = connector.connect(name, tcp).await else {
        return Ok(None);
    };
    if tls.write_all(request.as_bytes()).await.is_err() {
        return Ok(None);
    }
    let mut response = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(5), tls.read_to_end(&mut response)).await?;
    if read.is_err() || response.is_empty() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(response)?))
}

const POST: &str = "POST /internal/v1/test HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}";

fn status(response: &Option<String>) -> Option<&str> {
    response.as_deref()?.split(' ').nth(1)
}

#[tokio::test]
async fn bound_client_certificate_reaches_guarded_route() -> TestResult {
    let fixture = listener().await?;
    let (cert, key) = fixture
        .ca
        .issue(&[PEER], ExtendedKeyUsagePurpose::ClientAuth)?;
    let response = call(&fixture, Some((&cert, &key)), POST).await?;
    assert_eq!(status(&response), Some("200"));
    assert!(response.as_deref().is_some_and(|r| r.ends_with("accepted")));
    let peer = call(
        &fixture,
        Some((&cert, &key)),
        "GET /internal/v1/peer HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    )
    .await?;
    assert!(
        peer.as_deref()
            .is_some_and(|r| r.ends_with("127.0.0.1 true"))
    );
    Ok(())
}

#[tokio::test]
async fn unbound_or_unverified_clients_are_refused() -> TestResult {
    let fixture = listener().await?;
    // Valid chain, identity not bound to the route's group.
    let (other, other_key) = fixture.ca.issue(
        &["other.oteryn.internal"],
        ExtendedKeyUsagePurpose::ClientAuth,
    )?;
    let response = call(&fixture, Some((&other, &other_key)), POST).await?;
    assert_eq!(status(&response), Some("401"));
    // Ambiguous identity: two DNS names, connection is dropped.
    let (two, two_key) = fixture.ca.issue(
        &[PEER, "other.oteryn.internal"],
        ExtendedKeyUsagePurpose::ClientAuth,
    )?;
    assert_eq!(call(&fixture, Some((&two, &two_key)), POST).await?, None);
    // Same name, foreign CA: handshake refused.
    let rogue = Authority::new()?;
    let (forged, forged_key) = rogue.issue(&[PEER], ExtendedKeyUsagePurpose::ClientAuth)?;
    assert_eq!(
        call(&fixture, Some((&forged, &forged_key)), POST).await?,
        None
    );
    // No client certificate at all.
    assert_eq!(call(&fixture, None, POST).await?, None);
    Ok(())
}

#[tokio::test]
async fn only_internal_paths_are_served() -> TestResult {
    let fixture = listener().await?;
    let (cert, key) = fixture
        .ca
        .issue(&[PEER], ExtendedKeyUsagePurpose::ClientAuth)?;
    let response = call(
        &fixture,
        Some((&cert, &key)),
        "GET /pl/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    )
    .await?;
    assert_eq!(status(&response), Some("404"));
    Ok(())
}

#[tokio::test]
async fn public_listener_never_verifies_a_client() -> TestResult {
    use tower::ServiceExt;
    // The same router without this adapter: the guard refuses, whatever the headers say.
    let request = axum::http::Request::post("/internal/v1/test")
        .header("content-type", "application/json")
        .header("x-client-cert-san", PEER)
        .header("x-ssl-client-verify", "SUCCESS")
        .body(axum::body::Body::from("{}"))?;
    let response = app()?.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}
