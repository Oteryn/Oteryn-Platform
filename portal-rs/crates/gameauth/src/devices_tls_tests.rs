#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

pub(crate) struct Harness {
    pub address: SocketAddr,
    connector: tokio_rustls::TlsConnector,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Harness {
    pub async fn new(app: Router) -> Self {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let config = server_config(
            vec![cert.der().clone()],
            PrivateKeyDer::Pkcs8(key_pair.serialize_der().into()),
        )
        .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.der().clone()).unwrap();
        let client = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(serve(listener, config, app));
        Self {
            address,
            connector,
            task,
        }
    }
    pub async fn raw(&self, request: &str) -> String {
        let tcp = tokio::net::TcpStream::connect(self.address).await.unwrap();
        let mut stream = self
            .connector
            .connect(
                rustls::pki_types::ServerName::try_from("localhost").unwrap(),
                tcp,
            )
            .await
            .unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        String::from_utf8(response).unwrap()
    }
    pub async fn post(
        &self,
        suffix: &str,
        authorization: &str,
        body: &serde_json::Value,
    ) -> (u16, serde_json::Value) {
        let body = body.to_string();
        self.post_body(suffix, authorization, &body).await
    }
    pub async fn post_body(
        &self,
        suffix: &str,
        authorization: &str,
        body: &str,
    ) -> (u16, serde_json::Value) {
        let response = self.raw(&format!("POST /api/v1/game-auth/device-sessions{suffix} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAuthorization: {authorization}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())).await;
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_str(body).unwrap())
    }
}
fn enabled_router() -> Router {
    use crate::devices::*;
    let pool = sqlx::PgPool::connect_lazy("postgres://unused@127.0.0.1:1/unused").unwrap();
    router(Sessions::new(
        pool,
        Settings {
            enabled: true,
            environment: "testing".into(),
            ..Settings::default()
        },
        Arc::new(super::super::UnavailableAuthority),
    ))
}
#[tokio::test]
async fn devices_m2_plain_http_refused() {
    let response = enabled_router()
        .oneshot(
            Request::post("/api/v1/game-auth/device-sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
#[tokio::test]
async fn devices_m2_forwarded_proto_ignored() {
    let app = enabled_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(b"POST https://localhost/api/v1/game-auth/device-sessions HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-Proto: https\r\nX-Forwarded-For: 203.0.113.1\r\nForwarded: proto=https;for=203.0.113.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 503"));
    task.abort();
}
#[tokio::test]
async fn devices_m2_tls_listener_marks_verified() {
    let app = Router::new().route(
        "/probe",
        axum::routing::get(|request: axum::extract::Request| async move {
            assert!(request.extensions().get::<VerifiedHttps>().is_some());
            let peer = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .unwrap()
                .0;
            assert!(peer.ip().is_loopback());
            assert_ne!(peer.port(), 0);
            "verified"
        }),
    );
    let server = Harness::new(app).await;
    let response = server.raw("GET /probe HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-For: 203.0.113.1\r\nX-Forwarded-Proto: http\r\nConnection: close\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("verified"));
    let mut plain = tokio::net::TcpStream::connect(server.address)
        .await
        .unwrap();
    plain
        .write_all(b"GET /probe HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut bytes = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(6), plain.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("verified"));
}
