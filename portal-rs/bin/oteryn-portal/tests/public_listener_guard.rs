//! The public listener must refuse every `/internal/v1` and `/internal/v2`
//! route, even with every feature flag on and every peer identity configured.
//!
//! Only the private mTLS adapter (`internal_tls.rs`) inserts the verified peer
//! extension; on the public port a route must answer 401 (or 403/404/503 where
//! the module refuses that way), and no header a proxy might set, such as
//! `X-Forwarded-Client-Cert`, may change that. The real binary is started, so
//! the router is exactly the one the public listener serves. The native device
//! TLS listener serves the same `public_app` (server-only TLS, no client
//! certificate) and is checked the same way.
//!
//! The database is unreachable on purpose: a handler that ran would need it and
//! answer differently than the guard. The evidence route writes its high-water
//! files to a directory that must stay empty.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustls::pki_types::{CertificateDer, ServerName, pem::PemObject};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WORLD: &str = "01936f3a-1c2b-7d4e-8f5a-0b1c2d3e4f50";
const CHANNEL: &str = "01936f3a-1c2b-7d4e-9f5a-0b1c2d3e4f51";
const ACCOUNT: &str = "01936f3a-1c2b-7d4e-af5a-0b1c2d3e4f52";

const COINS_SAN: &str = "coins.game.internal";
const RUNTIME_SAN: &str = "runtime.game.internal";
const SCOPES_SAN: &str = "scopes.game.internal";
const EVIDENCE_SAN: &str = "evidence.game.internal";
const PUBLIC_SAN: &str = "public.game.internal";
const ANALYTICS_SAN: &str = "game-admin-analytics";
const CONTROL_SAN: &str = "control.game.internal";

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Response {
    status: u16,
    head: String,
    body: Vec<u8>,
}

/// Plain HTTP to the public listener, or TLS 1.3 without a client
/// certificate to the native device listener.
enum Transport {
    Public(u16),
    Native(u16, Arc<rustls::ClientConfig>),
}
trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

impl Transport {
    fn connect(&self) -> Box<dyn Stream> {
        let port = match self {
            Self::Public(port) | Self::Native(port, _) => *port,
        };
        let tcp = TcpStream::connect(("127.0.0.1", port)).expect("connect to listener");
        tcp.set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        match self {
            Self::Public(_) => Box::new(tcp),
            Self::Native(_, config) => {
                let name = ServerName::try_from("localhost").expect("server name");
                let client = rustls::ClientConnection::new(config.clone(), name)
                    .expect("TLS client connection");
                Box::new(rustls::StreamOwned::new(client, tcp))
            }
        }
    }
    fn label(&self) -> &'static str {
        match self {
            Self::Public(_) => "public",
            Self::Native(..) => "native",
        }
    }
}

/// `None` when no complete response arrives in time: a handler that got past
/// the guard waits on the unreachable database instead of refusing.
fn send(
    transport: &Transport,
    method: &str,
    path: &str,
    headers: &[(&str, String)],
    body: &str,
) -> Option<Response> {
    let mut stream = transport.connect();
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
    if method == "POST" {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = Vec::new();
    // A TLS peer may close without close_notify; keep what arrived.
    let _ = stream.read_to_end(&mut raw);
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status = head.split(' ').nth(1).and_then(|code| code.parse().ok())?;
    Some(Response {
        status,
        head,
        body: raw[split + 4..].to_vec(),
    })
}

/// Every header name a proxy or a client could use to claim a certificate
/// identity, plus the signature headers the closure listener reads.
fn spoofed(san: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "X-Client-Cert",
            format!("-----BEGIN CERTIFICATE----- {san}"),
        ),
        (
            "X-Forwarded-Client-Cert",
            format!("By=spiffe://portal;Hash=00;Subject=\"CN={san}\";URI=;DNS={san}"),
        ),
        ("X-SSL-Client-S-DN", format!("CN={san}")),
        ("X-SSL-Client-SAN", san.to_owned()),
        ("X-SSL-Client-Verify", "SUCCESS".to_owned()),
        ("X-SSL-Client-Cert", san.to_owned()),
        ("SSL-Client-Cert", san.to_owned()),
        ("X-Client-Verify", "SUCCESS".to_owned()),
        ("X-Client-SAN", san.to_owned()),
        ("X-Client-DN", format!("CN={san}")),
        ("X-Client-Identity", san.to_owned()),
        ("X-Peer-Identity", san.to_owned()),
        ("X-Verified-Client", san.to_owned()),
        ("X-Forwarded-For", "127.0.0.1".to_owned()),
        ("X-Real-IP", "127.0.0.1".to_owned()),
        ("Forwarded", "for=127.0.0.1;proto=https".to_owned()),
        ("X-Forwarded-Proto", "https".to_owned()),
        ("Authorization", format!("Bearer {san}")),
        ("X-Oteryn-Key-Id", san.to_owned()),
        ("X-Oteryn-Signature", "00".repeat(64)),
    ]
}

struct Route {
    method: &'static str,
    path: String,
    body: &'static str,
    san: &'static str,
    /// The refusal the module gives with its flag on and peers configured.
    expected: u16,
}

fn post(path: &str, san: &'static str, expected: u16) -> Route {
    Route {
        method: "POST",
        path: path.to_owned(),
        body: "{}",
        san,
        expected,
    }
}

/// Every `/internal/*` route merged into the public router (main.rs before
/// `let public_app`, and the gameauth routers in `oteryn_web::router`).
fn public_internal_routes() -> Vec<Route> {
    let mut routes = Vec::new();
    for path in [
        "transfers",
        "market/offers",
        "market/settlements",
        "market/cancellations",
    ] {
        routes.push(post(&format!("/internal/v1/coins/{path}"), COINS_SAN, 401));
    }
    for path in [
        format!("operations/{ACCOUNT}"),
        format!("accounts/{ACCOUNT}/balance"),
        format!("accounts/{ACCOUNT}/history"),
    ] {
        routes.push(Route {
            method: "GET",
            path: format!("/internal/v1/coins/{path}"),
            body: "",
            san: COINS_SAN,
            expected: 401,
        });
    }
    routes.push(post(
        "/internal/v1/game-auth/native-runtime-status",
        RUNTIME_SAN,
        401,
    ));
    routes.push(post(
        "/internal/v1/game-auth/native-evidence",
        EVIDENCE_SAN,
        401,
    ));
    for path in ["native-scope-assignments", "native-scope-revocations"] {
        routes.push(post(
            &format!("/internal/v1/game-auth/{path}"),
            SCOPES_SAN,
            401,
        ));
    }
    for path in [
        "highscores",
        "character-profile",
        "character-deaths",
        "guild",
        "house",
        "world-online",
        "watermark",
    ] {
        routes.push(post(
            &format!("/internal/v1/game-public/{path}"),
            PUBLIC_SAN,
            401,
        ));
    }
    for path in ["character-deaths", "guild", "house", "world-daily"] {
        routes.push(post(
            &format!("/internal/v2/game-public/{path}"),
            PUBLIC_SAN,
            401,
        ));
    }
    // The shared V2 watermark dispatches by family to each module's guard.
    for body in [
        "{}",
        r#"{"family":"character_deaths"}"#,
        r#"{"family":"guild"}"#,
        r#"{"family":"house"}"#,
        r#"{"family":"world_daily"}"#,
    ] {
        routes.push(Route {
            method: "POST",
            path: "/internal/v2/game-public/watermark".to_owned(),
            body,
            san: PUBLIC_SAN,
            expected: 401,
        });
    }
    for path in ["page", "commit", "watermark"] {
        routes.push(post(
            &format!("/internal/v1/admin-analytics/{path}"),
            ANALYTICS_SAN,
            401,
        ));
    }
    // Admin control dispatches on a VerifiedAgent extension that no listener
    // inserts on the public port; it refuses with its own 403 envelope.
    for path in [
        "hello",
        "channel",
        "results",
        "rollups",
        "heartbeat",
        "stream",
    ] {
        routes.push(post(
            &format!("/internal/v1/admin-control/{path}"),
            CONTROL_SAN,
            403,
        ));
    }
    // Internal-only routes merged after `public_app` must not exist here.
    for path in [
        "/internal/v1/identity/security-revocations/stream/read",
        "/internal/v1/game-auth/native-account-characters",
        "/internal/v1/game-auth/character-commands/pending",
        "/internal/v2/game-auth/character-services/state",
        "/internal/v1/game-auth/native-admissions",
        "/internal/v1/bazaar/status",
        "/internal/v1/game-events",
    ] {
        routes.push(post(path, PUBLIC_SAN, 404));
    }
    routes
}

fn environment(port: u16, evidence_dir: &std::path::Path) -> Vec<(&'static str, String)> {
    let scope = format!("[\"{WORLD}/{CHANNEL}\"]");
    vec![
        ("OTERYN_ENV", "testing".to_owned()),
        ("APP_ENV", "testing".to_owned()),
        ("OTERYN_BIND", format!("127.0.0.1:{port}")),
        ("OTERYN_PUBLIC_ORIGIN", format!("http://127.0.0.1:{port}")),
        // Lazy pool; the guard must answer before any handler needs it.
        (
            "OTERYN_DATABASE_URL",
            "postgres://guard-test@127.0.0.1:1/guard_test".to_owned(),
        ),
        // Coin wire.
        ("PORTAL_COIN_TRANSFERS_ENABLED", "true".to_owned()),
        ("PORTAL_COIN_MARKET_ENABLED", "true".to_owned()),
        (
            "PORTAL_GAME_COINS_IDENTITIES",
            format!("{{\"{COINS_SAN}\":[]}}"),
        ),
        // Gameauth runtime status, scopes and evidence.
        ("GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED", "true".to_owned()),
        (
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES",
            format!("{{\"{RUNTIME_SAN}\":{scope}}}"),
        ),
        (
            "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED",
            "true".to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES",
            format!("{{\"{SCOPES_SAN}\":{scope}}}"),
        ),
        ("GAME_AUTH_NATIVE_EVIDENCE_ENABLED", "true".to_owned()),
        ("GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED", "true".to_owned()),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY",
            EVIDENCE_SAN.to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY",
            evidence_dir.display().to_string(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY",
            "game.test".to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE",
            "account".to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE",
            "scope".to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE",
            "key".to_owned(),
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS",
            "0".to_owned(),
        ),
        // Public projections (v1 ingest and the v2 families).
        ("PORTAL_PUBLIC_PROJECTIONS_ENABLED", "true".to_owned()),
        ("PORTAL_PUBLIC_PROJECTIONS_SAN", PUBLIC_SAN.to_owned()),
        (
            "PORTAL_PUBLIC_PROJECTIONS_AUTHORITY",
            "game.test".to_owned(),
        ),
        ("PORTAL_DEATHS_V2_INGEST_ENABLED", "true".to_owned()),
        ("PORTAL_DEATHS_ENABLED", "true".to_owned()),
        ("PORTAL_GUILDS_INGEST_ENABLED", "true".to_owned()),
        ("PORTAL_GUILDS_ENABLED", "true".to_owned()),
        ("PORTAL_HOUSES_INGEST_ENABLED", "true".to_owned()),
        ("PORTAL_HOUSES_SERVING_ENABLED", "true".to_owned()),
        ("PORTAL_WORLD_DAILY_INGEST_ENABLED", "true".to_owned()),
        ("PORTAL_WORLD_DAILY_PUBLIC_ENABLED", "true".to_owned()),
        ("PORTAL_WORLD_DAILY_WORLDS", WORLD.to_owned()),
        // Admin analytics ingest.
        ("OTERYN_ADMIN_ANALYTICS_INGEST", "true".to_owned()),
        ("OTERYN_ADMIN_ANALYTICS_PRESENT", "true".to_owned()),
        ("OTERYN_ADMIN_ANALYTICS_AUTHORITY", "game.test".to_owned()),
        ("OTERYN_ADMIN_ANALYTICS_PRIVACY", "privacy.test".to_owned()),
        ("OTERYN_ADMIN_ANALYTICS_PEER_SAN", ANALYTICS_SAN.to_owned()),
        // Admin control.
        ("PORTAL_ADMIN_CONTROL_ENABLED", "true".to_owned()),
        (
            "PORTAL_ADMIN_CONTROL_AGENTS_JSON",
            format!(
                "[{{\"cn\":\"oteryn-admin-agent/agent-1\",\"identity\":{{\"san\":\"{CONTROL_SAN}\"}},\"result_key\":null}}]"
            ),
        ),
    ]
}

/// A CA and a `localhost` server certificate for the native listener.
struct NativeTls {
    dir: tempfile::TempDir,
    client: Arc<rustls::ClientConfig>,
}

fn native_tls() -> NativeTls {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
    let ca_key = KeyPair::generate().expect("CA key");
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("CA params");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key).expect("CA certificate");
    let key = KeyPair::generate().expect("server key");
    let leaf = CertificateParams::new(vec!["localhost".to_owned()])
        .expect("server params")
        .signed_by(&key, &ca, &ca_key)
        .expect("server certificate");
    let dir = tempfile::tempdir().expect("TLS dir");
    std::fs::write(dir.path().join("cert.pem"), leaf.pem()).expect("write cert");
    std::fs::write(dir.path().join("key.pem"), key.serialize_pem()).expect("write key");
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_slice(ca.pem().as_bytes()).expect("CA der"))
        .expect("trust CA");
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut client = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("TLS 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.alpn_protocols = vec![b"http/1.1".to_vec()];
    NativeTls {
        dir,
        client: Arc::new(client),
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("free port")
        .port()
}

fn start(
    port: u16,
    native_port: u16,
    native: &NativeTls,
    evidence_dir: &std::path::Path,
) -> Server {
    let mut command = Command::new(env!("CARGO_BIN_EXE_oteryn-portal"));
    command
        .arg("serve")
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default());
    for (key, value) in environment(port, evidence_dir) {
        command.env(key, value);
    }
    command
        .env("OTERYN_NATIVE_TLS_BIND", format!("127.0.0.1:{native_port}"))
        .env(
            "OTERYN_NATIVE_TLS_CERT_FILE",
            native.dir.path().join("cert.pem"),
        )
        .env(
            "OTERYN_NATIVE_TLS_KEY_FILE",
            native.dir.path().join("key.pem"),
        );
    let mut server = Server(
        command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn oteryn-portal"),
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = server.0.try_wait().expect("poll server") {
            let mut stderr = String::new();
            if let Some(mut pipe) = server.0.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            panic!("server exited early ({status}): {stderr}");
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok()
            && TcpStream::connect(("127.0.0.1", native_port)).is_ok()
        {
            return server;
        }
        assert!(
            Instant::now() < deadline,
            "server did not listen within 30 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn public_listener_refuses_every_internal_route_with_or_without_spoofed_identity() {
    let (port, native_port) = (free_port(), free_port());
    let native = native_tls();
    let evidence_dir = tempfile::tempdir().expect("evidence dir");
    let _server = start(port, native_port, &native, evidence_dir.path());
    let transports = [
        Transport::Public(port),
        Transport::Native(native_port, native.client.clone()),
    ];

    let routes = public_internal_routes();
    let mut failures = Vec::new();
    let mut sent = 0;
    for transport in &transports {
        // The listener is live and serves the public router.
        let health = send(transport, "GET", "/healthz", &[], "").expect("health response");
        assert_eq!(health.status, 200, "{}: {}", transport.label(), health.head);
        for route in &routes {
            for (label, headers) in [("plain", Vec::new()), ("spoofed", spoofed(route.san))] {
                sent += 1;
                let where_ = format!(
                    "{} {label} {} {}",
                    transport.label(),
                    route.method,
                    route.path
                );
                let Some(response) =
                    send(transport, route.method, &route.path, &headers, route.body)
                else {
                    failures.push(format!(
                        "{where_}: no response; the request passed the guard"
                    ));
                    continue;
                };
                let refused = matches!(response.status, 401 | 403 | 404 | 503);
                if !refused || response.status != route.expected {
                    failures.push(format!(
                        "{where_} body={} -> {} (expected {}) {}",
                        route.body,
                        response.status,
                        route.expected,
                        String::from_utf8_lossy(&response.body)
                    ));
                }
                // Guard refusals carry no handler output; 401 has an empty body.
                if response.status == 401 && !response.body.is_empty() {
                    failures.push(format!("{where_}: 401 carried a body"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {sent} requests not refused as expected:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(sent, routes.len() * 4);

    // No handler side effect: the evidence high-water store stayed empty.
    let written: Vec<_> = std::fs::read_dir(evidence_dir.path())
        .expect("read evidence dir")
        .collect();
    assert!(written.is_empty(), "evidence handler wrote {written:?}");
}
