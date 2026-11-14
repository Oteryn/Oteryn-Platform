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
    /// Client certificate with exactly the given DNS SAN and subject CN (or none).
    fn issue_agent(&self, san: &str, cn: Option<&str>) -> Result<(String, String), rcgen::Error> {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![san.to_owned()])?;
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        params.distinguished_name = rcgen::DistinguishedName::new();
        if let Some(cn) = cn {
            params
                .distinguished_name
                .push(rcgen::DnType::CommonName, cn);
        }
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
        Router::new()
            .route("/internal/v1/test", post(|| async { "accepted" }))
            .route("/internal/v2/test", post(|| async { "accepted v2" })),
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
        .route("/pl/", get(|| async { "public" }))
        .route("/internal/v3/test", post(|| async { "never" }))
        .route("/internal/test", post(|| async { "never" }))
        .route("/internalv2/test", post(|| async { "never" })))
}

struct Fixture {
    _dir: tempfile::TempDir,
    addr: SocketAddr,
    ca_pem: String,
    ca: Authority,
}

async fn listener() -> Result<Fixture, Box<dyn std::error::Error>> {
    listener_with(app()?, None).await
}

async fn listener_with(
    app: Router,
    agents: Option<Arc<AgentIdentities>>,
) -> Result<Fixture, Box<dyn std::error::Error>> {
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
    tokio::spawn(serve(socket, tls, app, agents));
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

#[tokio::test]
async fn v2_prefix_is_served_and_other_prefixes_are_refused() -> TestResult {
    let fixture = listener().await?;
    let (cert, key) = fixture
        .ca
        .issue(&[PEER], ExtendedKeyUsagePurpose::ClientAuth)?;
    let v2 = POST.replace("/internal/v1/test", "/internal/v2/test");
    let response = call(&fixture, Some((&cert, &key)), &v2).await?;
    assert_eq!(status(&response), Some("200"));
    assert!(
        response
            .as_deref()
            .is_some_and(|r| r.ends_with("accepted v2"))
    );
    // The v2 prefix keeps the identity rules: an unbound identity is refused.
    let (other, other_key) = fixture.ca.issue(
        &["other.oteryn.internal"],
        ExtendedKeyUsagePurpose::ClientAuth,
    )?;
    let response = call(&fixture, Some((&other, &other_key)), &v2).await?;
    assert_eq!(status(&response), Some("401"));
    // Routes outside both prefixes exist on the router but stay 404 here.
    for path in ["/internal/v3/test", "/internal/test", "/internalv2/test"] {
        let request = POST.replace("/internal/v1/test", path);
        let response = call(&fixture, Some((&cert, &key)), &request).await?;
        assert_eq!(status(&response), Some("404"), "{path}");
    }
    Ok(())
}

const AGENT_SAN: &str = "agent-a.oteryn.internal";
const AGENT_CN: &str = "oteryn-admin-agent/agent-a";

fn agent_bindings() -> Result<Vec<oteryn_admin::control_http::AgentBinding>, guard::ConfigError> {
    Ok(vec![oteryn_admin::control_http::AgentBinding {
        agent_id: "agent-a".into(),
        identity: Identity::san(AGENT_SAN)?,
        result_key: None,
    }])
}

/// The real admin-control router (enabled) plus a probe that reports the agent.
/// The pool is never reached: every request here stops before the database.
fn admin_app() -> Result<Router, Box<dyn std::error::Error>> {
    let pool = oteryn_db::outbox::PgPoolOptions::new()
        .max_connections(2)
        .connect_lazy("postgres://localhost/unused")?;
    let control = oteryn_admin::control_http::router(pool, true, agent_bindings()?)?;
    Ok(control.route(
        "/internal/v1/agent-probe",
        get(|agent: Option<Extension<VerifiedAgent>>| async move {
            agent.map_or_else(
                || "agent none".to_owned(),
                |a| format!("agent {}", a.agent_id()),
            )
        }),
    ))
}

fn enabled_agents() -> Result<Option<Arc<AgentIdentities>>, guard::ConfigError> {
    Ok(AgentIdentities::new(true, &agent_bindings()?).map(Arc::new))
}

const PROBE: &str =
    "GET /internal/v1/agent-probe HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

/// A hello with an unknown member: past dispatch and the identity guard it is
/// refused as invalid input before any database access.
fn hello() -> String {
    let body = r#"{"agent_id":"agent-a","unexpected":true}"#;
    format!(
        "POST /internal/v1/admin-control/hello HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Client-Cert-CN: {AGENT_CN}\r\nX-SSL-Client-S-DN: CN={AGENT_CN}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

async fn agent_of(
    fixture: &Fixture,
    cert: &(String, String),
) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
    let client = Some((cert.0.as_str(), cert.1.as_str()));
    let probe = call(fixture, client, PROBE).await?;
    let hello = call(fixture, client, &hello()).await?;
    Ok((probe, hello))
}

#[tokio::test]
async fn admin_agent_with_bound_san_and_cn_is_verified() -> TestResult {
    let fixture = listener_with(admin_app()?, enabled_agents()?).await?;
    let cert = fixture.ca.issue_agent(AGENT_SAN, Some(AGENT_CN))?;
    let (probe, hello) = agent_of(&fixture, &cert).await?;
    assert!(
        probe
            .as_deref()
            .is_some_and(|r| r.ends_with("agent agent-a"))
    );
    assert_eq!(status(&hello), Some("400"));
    assert!(
        hello
            .as_deref()
            .is_some_and(|r| r.contains("ADM_INVALID_PARAMS"))
    );
    Ok(())
}

#[tokio::test]
async fn admin_agent_is_refused_without_a_bound_identity() -> TestResult {
    let fixture = listener_with(admin_app()?, enabled_agents()?).await?;
    let cases = [
        // Wrong SAN for the CN: the CN alone never authenticates.
        ("other.oteryn.internal", Some(AGENT_CN)),
        // Unknown agent id.
        (AGENT_SAN, Some("oteryn-admin-agent/agent-b")),
        // Malformed or missing CN.
        (AGENT_SAN, Some("agent-a")),
        (AGENT_SAN, Some("oteryn-admin-agent/")),
        (AGENT_SAN, Some("oteryn-admin-agent/agent a")),
        (AGENT_SAN, Some("OTERYN-ADMIN-AGENT/agent-a")),
        (AGENT_SAN, None),
    ];
    for (san, cn) in cases {
        let cert = fixture.ca.issue_agent(san, cn)?;
        let (probe, hello) = agent_of(&fixture, &cert).await?;
        assert!(
            probe.as_deref().is_some_and(|r| r.ends_with("agent none")),
            "{san} {cn:?}"
        );
        assert_eq!(status(&hello), Some("403"), "{san} {cn:?}");
        assert!(
            hello
                .as_deref()
                .is_some_and(|r| r.contains("ADM_FORBIDDEN"))
        );
    }
    // Missing client certificate: the handshake is refused.
    assert_eq!(call(&fixture, None, PROBE).await?, None);
    assert_eq!(call(&fixture, None, &hello()).await?, None);
    Ok(())
}

#[tokio::test]
async fn admin_agent_is_never_verified_while_the_flag_is_off() -> TestResult {
    assert!(AgentIdentities::new(false, &agent_bindings()?).is_none());
    let fixture = listener_with(admin_app()?, None).await?;
    let cert = fixture.ca.issue_agent(AGENT_SAN, Some(AGENT_CN))?;
    let (probe, hello) = agent_of(&fixture, &cert).await?;
    assert!(probe.as_deref().is_some_and(|r| r.ends_with("agent none")));
    assert_eq!(status(&hello), Some("403"));
    Ok(())
}

#[tokio::test]
async fn public_listener_never_verifies_an_admin_agent() -> TestResult {
    use tower::ServiceExt;
    let request = axum::http::Request::post("/internal/v1/admin-control/hello")
        .header("content-type", "application/json")
        .header("x-client-cert-cn", AGENT_CN)
        .header("x-client-cert-san", AGENT_SAN)
        .header("x-ssl-client-s-dn", format!("CN={AGENT_CN}"))
        .header("x-ssl-client-verify", "SUCCESS")
        .body(axum::body::Body::from(
            r#"{"agent_id":"agent-a","unexpected":true}"#,
        ))?;
    let response = admin_app()?.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let probe = axum::http::Request::get("/internal/v1/agent-probe")
        .header("x-client-cert-cn", AGENT_CN)
        .body(axum::body::Body::empty())?;
    let body = axum::body::to_bytes(admin_app()?.oneshot(probe).await?.into_body(), 1024).await?;
    assert_eq!(&body[..], b"agent none");
    Ok(())
}

#[tokio::test]
async fn spki_bound_admin_agent_is_refused_on_the_san_only_listener() -> TestResult {
    // An agent configured by SPKI is never matched here: the listener proves
    // peers by SAN only, so its CN must not select it (fail closed).
    let bindings = vec![oteryn_admin::control_http::AgentBinding {
        agent_id: "agent-spki".into(),
        identity: Identity::spki_sha256([7; 32]),
        result_key: None,
    }];
    let pool = oteryn_db::outbox::PgPoolOptions::new()
        .max_connections(2)
        .connect_lazy("postgres://localhost/unused")?;
    let control = oteryn_admin::control_http::router(pool, true, bindings.clone())?;
    let app = control.route(
        "/internal/v1/agent-probe",
        get(|agent: Option<Extension<VerifiedAgent>>| async move {
            agent.map_or_else(
                || "agent none".to_owned(),
                |a| format!("agent {}", a.agent_id()),
            )
        }),
    );
    let agents = AgentIdentities::new(true, &bindings).map(Arc::new);
    let fixture = listener_with(app, agents).await?;
    let cert = fixture.ca.issue_agent(
        "agent-spki.oteryn.internal",
        Some("oteryn-admin-agent/agent-spki"),
    )?;
    let client = Some((cert.0.as_str(), cert.1.as_str()));
    let probe = call(&fixture, client, PROBE).await?;
    assert!(probe.as_deref().is_some_and(|r| r.ends_with("agent none")));
    let body = r#"{"agent_id":"agent-spki","unexpected":true}"#;
    let hello = format!(
        "POST /internal/v1/admin-control/hello HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let hello = call(&fixture, client, &hello).await?;
    assert_eq!(status(&hello), Some("403"));
    assert!(
        hello
            .as_deref()
            .is_some_and(|r| r.contains("ADM_FORBIDDEN"))
    );
    Ok(())
}
