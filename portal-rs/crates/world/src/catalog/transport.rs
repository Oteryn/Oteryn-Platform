//! Read-only one-shot artifact download. No redirects, proxies or ambient credentials.
use super::{Error, MAX_FILE_BYTES, Result};
use axum::body::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};
trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}
pub(super) fn validate(url: &url::Url) -> Result<()> {
    let local = url
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(super) async fn fetch(url: &url::Url) -> Result<Vec<u8>> {
    validate(url)?;
    tokio::time::timeout(Duration::from_secs(30), exchange(url))
        .await
        .map_err(|_| Error::Unavailable)?
}
async fn exchange(url: &url::Url) -> Result<Vec<u8>> {
    let host = url
        .host_str()
        .ok_or(Error::InvalidInput)?
        .trim_matches(['[', ']']);
    let port = url.port_or_known_default().ok_or(Error::InvalidInput)?;
    let tcp = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(|_| Error::Unavailable)?;
    let io: Box<dyn Io> = if url.scheme() == "https" {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| Error::Unavailable)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(host.to_owned())
            .map_err(|_| Error::InvalidInput)?;
        Box::new(
            tokio_rustls::TlsConnector::from(Arc::new(config))
                .connect(name, tcp)
                .await
                .map_err(|_| Error::Unavailable)?,
        )
    } else {
        Box::new(tcp)
    };
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(io))
        .await
        .map_err(|_| Error::Unavailable)?;
    let connection = tokio::spawn(async move {
        let _ = connection.await;
    });
    // Abort the connection task on success, refusal or cancellation/timeout.
    struct Connection(tokio::task::JoinHandle<()>);
    impl Drop for Connection {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _connection = Connection(connection);
    let mut path = url.path().to_owned();
    if let Some(query) = url.query() {
        path.push('?');
        path.push_str(query);
    }
    let request = hyper::Request::builder()
        .method("GET")
        .uri(path)
        .header(
            "host",
            &url[url::Position::BeforeHost..url::Position::AfterPort],
        )
        .header("accept", "application/json")
        .header("accept-encoding", "identity")
        .body(Full::new(Bytes::new()))
        .map_err(|_| Error::InvalidInput)?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|_| Error::Unavailable)?;
    if response.status() != 200
        || response
            .headers()
            .get("content-encoding")
            .is_some_and(|v| v != "identity")
    {
        return Err(Error::Unavailable);
    }
    if response
        .headers()
        .get("content-length")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|n| n > MAX_FILE_BYTES as u64)
    {
        return Err(Error::CapacityExceeded);
    }
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| Error::Unavailable)?;
        if let Some(data) = frame.data_ref() {
            if data.len() > MAX_FILE_BYTES.saturating_sub(bytes.len()) {
                return Err(Error::CapacityExceeded);
            }
            bytes.extend_from_slice(data);
        }
    }
    Ok(bytes)
}
