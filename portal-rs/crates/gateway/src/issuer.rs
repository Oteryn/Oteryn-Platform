//! One HTTP/1 exchange per admission. No redirects, retries, cookies or ambient proxy credentials.
use crate::{
    errors::{Code, Refusal, parse_issuer_error, retry_after},
    v3::{self, Refusal3},
    wire::{MAX_RESPONSE_BYTES, Request, Success, encode_request, parse_success},
};
use axum::body::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request as HttpRequest, body::Incoming};
use hyper_util::rt::TokioIo;
use std::{fmt, sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};

pub const ISSUER_PATH: &str = "/internal/v1/game-auth/native-admissions";
#[derive(Clone)]
pub struct IssuerClient {
    base_url: String,
    token: String,
    timeout: Duration,
    ready_timeout: Duration,
    tls: Arc<rustls::ClientConfig>,
}
impl fmt::Debug for IssuerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IssuerClient{redacted}")
    }
}
trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}
impl IssuerClient {
    pub fn new(
        base_url: &str,
        token: String,
        timeout: Duration,
        ready_timeout: Duration,
    ) -> Result<Self, String> {
        crate::config::validate_url(base_url)?;
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        // Pin the provider: other crates in the workspace enable a second rustls
        // backend, so the process-wide default cannot be inferred.
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            token,
            timeout,
            ready_timeout,
            tls: Arc::new(tls),
        })
    }
    async fn exchange(
        &self,
        method: &str,
        raw_url: &str,
        body: Vec<u8>,
        auth: bool,
        written: &mut bool,
    ) -> Result<hyper::Response<Incoming>, Code> {
        let url = url::Url::parse(raw_url).map_err(|_| Code::Unavailable)?;
        let host = url
            .host_str()
            .ok_or(Code::Unavailable)?
            .trim_matches(['[', ']']);
        let port = url.port_or_known_default().ok_or(Code::Unavailable)?;
        let tcp = tokio::net::TcpStream::connect((host, port))
            .await
            .map_err(|_| Code::Unavailable)?;
        let io: Box<dyn Io> = if url.scheme() == "https" {
            let name = rustls::pki_types::ServerName::try_from(host.to_owned())
                .map_err(|_| Code::Unavailable)?;
            Box::new(
                tokio_rustls::TlsConnector::from(self.tls.clone())
                    .connect(name, tcp)
                    .await
                    .map_err(|_| Code::Unavailable)?,
            )
        } else {
            Box::new(tcp)
        };
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(io))
            .await
            .map_err(|_| Code::Unavailable)?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let mut path = url.path().to_owned();
        if let Some(q) = url.query() {
            path.push('?');
            path.push_str(q);
        }
        let mut builder = HttpRequest::builder().method(method).uri(path).header(
            "Host",
            &url[url::Position::BeforeHost..url::Position::AfterPort],
        );
        if auth {
            builder = builder
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json");
        }
        let request = builder
            .body(Full::new(Bytes::from(body)))
            .map_err(|_| Code::Unavailable)?;
        *written = true;
        sender
            .send_request(request)
            .await
            .map_err(|_| Code::ReconciliationRequired)
    }
    pub async fn ready(&self) -> bool {
        tokio::time::timeout(self.ready_timeout, async {
            let mut url = format!("{}/health", self.base_url);
            for _ in 0..10 {
                let mut written = false;
                let Ok(response) = self
                    .exchange("GET", &url, Vec::new(), false, &mut written)
                    .await
                else {
                    return false;
                };
                if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                    let Some(location) = response
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                    else {
                        return false;
                    };
                    let Ok(next) = url::Url::parse(&url).and_then(|u| u.join(location)) else {
                        return false;
                    };
                    url = next.to_string();
                    continue;
                }
                let ok = response.status() == 200;
                let _ = read_body(response.into_body(), 4096).await;
                return ok;
            }
            false
        })
        .await
        .unwrap_or(false)
    }
    pub async fn admit(&self, request: &Request) -> Result<Success, Refusal> {
        let body = encode_request(request).map_err(Refusal::from)?;
        let mut written = false;
        let result = tokio::time::timeout(self.timeout, async {
            let response = self
                .exchange(
                    "POST",
                    &format!("{}{ISSUER_PATH}", self.base_url),
                    body,
                    true,
                    &mut written,
                )
                .await?;
            let status = response.status().as_u16();
            let retry = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .map(retry_after)
                .unwrap_or(60);
            let failure = if status == 200 {
                Code::ReconciliationRequired
            } else {
                Code::Unavailable
            };
            let limit = if status == 200 {
                MAX_RESPONSE_BYTES
            } else {
                4096
            };
            let payload = read_body(response.into_body(), limit)
                .await
                .map_err(|_| failure)?;
            if status == 200 {
                return parse_success(&payload, request)
                    .ok_or(failure)
                    .map_err(Refusal::from);
            }
            let mut refusal = parse_issuer_error(status, &payload, &request.attempt_ref)
                .ok_or(Code::Unavailable)?;
            if refusal.code == Code::RateLimited {
                refusal.retry_after = retry;
            }
            Err(refusal)
        })
        .await;
        match result {
            Ok(r) => r,
            Err(_) => Err(if written {
                Code::ReconciliationRequired
            } else {
                Code::Unavailable
            }
            .into()),
        }
    }
    /// One authenticated POST to a private issuer operation with the protocol
    /// 2 timeout and reconciliation rules: `(status, Retry-After, body)`.
    pub(crate) async fn call(
        &self,
        path: &str,
        body: Vec<u8>,
        success_limit: usize,
    ) -> Result<(u16, u64, Vec<u8>), Code> {
        let mut written = false;
        let result = tokio::time::timeout(self.timeout, async {
            let response = self
                .exchange(
                    "POST",
                    &format!("{}{path}", self.base_url),
                    body,
                    true,
                    &mut written,
                )
                .await?;
            let status = response.status().as_u16();
            let retry = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .map(retry_after)
                .unwrap_or(60);
            let (failure, limit) = if status == 200 {
                (Code::ReconciliationRequired, success_limit)
            } else {
                (Code::Unavailable, 4096)
            };
            let payload = read_body(response.into_body(), limit)
                .await
                .map_err(|_| failure)?;
            Ok((status, retry, payload))
        })
        .await;
        match result {
            Ok(r) => r,
            Err(_) if written => Err(Code::ReconciliationRequired),
            Err(_) => Err(Code::Unavailable),
        }
    }
    /// Protocol 3 admission (§19.2): same private operation, protocol 3 shapes.
    pub async fn admit3(&self, request: &Request) -> Result<v3::Success, Refusal3> {
        let body = encode_request(request).map_err(Refusal3::from)?;
        let (status, retry, payload) = self
            .call(ISSUER_PATH, body, v3::MAX_RESPONSE_BYTES)
            .await
            .map_err(Refusal3::from)?;
        if status == 200 {
            return v3::parse_success(&payload, request)
                .ok_or_else(|| Code::ReconciliationRequired.into());
        }
        let mut refused = v3::parse_issuer_error(status, &payload, &request.attempt_ref)
            .ok_or_else(|| Refusal3::from(Code::Unavailable))?;
        if refused.refusal.code == Code::RateLimited {
            refused.refusal.retry_after = retry;
        }
        Err(refused)
    }
}
async fn read_body(mut body: Incoming, limit: usize) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| ())?;
        if let Ok(data) = frame.into_data() {
            if bytes.len() + data.len() > limit {
                return Err(());
            }
            bytes.extend_from_slice(&data);
        }
    }
    Ok(bytes)
}
