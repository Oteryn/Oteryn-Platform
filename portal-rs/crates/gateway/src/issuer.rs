//! One HTTP/1 exchange per admission. No redirects, retries, cookies or ambient proxy credentials.
use crate::{
    config::{Config, ISSUER_CA_FILE, validate_issuer_url},
    errors::{Code, Refusal, parse_issuer_error, retry_after},
    recover::{self, RecoverCode, RecoverRefusal},
    v3::{self, Refusal3},
    wire::{MAX_RESPONSE_BYTES, Request, Success, encode_request, parse_success},
};
use axum::body::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request as HttpRequest, body::Incoming};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{CertificateDer, pem::PemObject};
use std::{fmt, path::Path, sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};
use x509_parser::prelude::{FromDer, X509Certificate};

pub const ISSUER_PATH: &str = "/internal/v1/game-auth/native-admissions";
#[derive(Clone)]
pub struct IssuerClient {
    base_url: String,
    token: String,
    timeout: Duration,
    ready_timeout: Duration,
    tls: Arc<rustls::ClientConfig>,
    require_https: bool,
}
impl fmt::Debug for IssuerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IssuerClient{redacted}")
    }
}
fn issuer_roots(path: &Path) -> Result<rustls::RootCertStore, String> {
    let invalid = || format!("invalid CA certificate in {ISSUER_CA_FILE}");
    let certificates =
        CertificateDer::pem_file_iter(path).map_err(|_| format!("cannot open {ISSUER_CA_FILE}"))?;
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates {
        let certificate = certificate.map_err(|_| invalid())?;
        let (rest, ca) = X509Certificate::from_der(certificate.as_ref()).map_err(|_| invalid())?;
        if !rest.is_empty()
            || !ca
                .basic_constraints()
                .map_err(|_| invalid())?
                .is_some_and(|bc| bc.value.ca)
            || ca
                .key_usage()
                .map_err(|_| invalid())?
                .is_some_and(|usage| !usage.value.key_cert_sign())
        {
            return Err(invalid());
        }
        roots.add(certificate).map_err(|_| invalid())?;
    }
    if roots.is_empty() {
        return Err(format!("{ISSUER_CA_FILE} contains no PEM certificates"));
    }
    Ok(roots)
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
        Self::build(base_url, token, timeout, ready_timeout, None)
    }

    pub fn from_config(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::build(
            &config.base_url,
            config.read_token()?,
            config.timeout,
            config.ready_timeout,
            config.issuer_ca_file.as_deref(),
        )?)
    }

    fn build(
        base_url: &str,
        token: String,
        timeout: Duration,
        ready_timeout: Duration,
        ca_file: Option<&Path>,
    ) -> Result<Self, String> {
        validate_issuer_url(base_url, ca_file.is_some())?;
        let roots = match ca_file {
            Some(path) => issuer_roots(path)?,
            None => {
                rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned())
            }
        };
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
            require_https: ca_file.is_some(),
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
        if self.require_https && url.scheme() != "https" {
            return Err(Code::Unavailable);
        }
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
            let Ok(mut url) = url::Url::parse(&format!("{}/health", self.base_url)) else {
                return false;
            };
            let origin = url.origin();
            for _ in 0..10 {
                let mut written = false;
                let Ok(response) = self
                    .exchange("GET", url.as_str(), Vec::new(), false, &mut written)
                    .await
                else {
                    return false;
                };
                let (parts, body) = response.into_parts();
                if read_body(body, 4096).await.is_err() {
                    return false;
                }
                if parts.status == 200 {
                    return true;
                }
                if !matches!(parts.status.as_u16(), 301 | 302 | 303 | 307 | 308) {
                    return false;
                }
                let mut locations = parts.headers.get_all("location").iter();
                let Some(location) = locations.next().and_then(|v| v.to_str().ok()) else {
                    return false;
                };
                let Ok(next) = url.join(location) else {
                    return false;
                };
                if locations.next().is_some()
                    || next.origin() != origin
                    || validate_issuer_url(next.as_str(), self.require_https).is_err()
                {
                    return false;
                }
                url = next;
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
    /// §19.4 recovery: `native-recoveries`, with the recovery reconciliation
    /// code for an exchange whose outcome is unknown.
    pub async fn recover(
        &self,
        request: &recover::RecoverRequest,
    ) -> Result<Success, RecoverRefusal> {
        let body = recover::encode_request(request)?;
        let (status, retry, payload) = self
            .call(recover::RECOVER_PATH, body, recover::MAX_RESPONSE_BYTES)
            .await?;
        if status == 200 {
            return recover::parse_success(&payload, request)
                .ok_or_else(|| RecoverCode::ReconciliationRequired.into());
        }
        let code = recover::parse_issuer_error(status, &payload, Some(&request.attempt_ref))
            .ok_or(RecoverCode::Unavailable)?;
        Err(RecoverRefusal {
            code,
            retry_after: if code.retries_after() { retry } else { 0 },
        })
    }
    /// §19.7 revocation. It is idempotent, so an unknown outcome is
    /// `NATIVE_LOGIN_UNAVAILABLE` and the client sends it again.
    pub async fn revoke(&self, request: &recover::RevokeRequest) -> Result<(), RecoverRefusal> {
        let body = recover::encode_revoke(request)?;
        let (status, retry, payload) = self
            .call(recover::REVOKE_PATH, body, 256)
            .await
            .map_err(|_| RecoverCode::Unavailable)?;
        if status == 200 {
            return recover::revoked(&payload)
                .then_some(())
                .ok_or_else(|| RecoverCode::Unavailable.into());
        }
        let code =
            recover::parse_issuer_error(status, &payload, None).ok_or(RecoverCode::Unavailable)?;
        Err(RecoverRefusal {
            code,
            retry_after: if code.retries_after() { retry } else { 0 },
        })
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
