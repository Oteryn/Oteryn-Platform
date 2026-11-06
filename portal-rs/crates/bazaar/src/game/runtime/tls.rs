//! TLS 1.3 + HTTP/1.1 only. WebPKI validates the chain and client-auth EKU;
//! the exact leaf SPKI additionally selects one purpose in the shared guard.
use super::{Error, Runtime};
use axum::{Router, body::Body};
use hyper_util::rt::TokioIo;
use oteryn_internal_api::guard::{Identity, VerifiedClient};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
impl Runtime {
    pub async fn start_private_listener(&self) -> Result<tokio::task::JoinHandle<()>, Error> {
        self.config.current()?;
        let certs = CertificateDer::pem_file_iter(&self.config.certificate)
            .map_err(|_| Error::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::Unavailable)?;
        let key = PrivateKeyDer::from_pem_file(&self.config.private_key)
            .map_err(|_| Error::Unavailable)?;
        let mut roots = rustls::RootCertStore::empty();
        for cert in
            CertificateDer::pem_file_iter(&self.config.client_ca).map_err(|_| Error::Unavailable)?
        {
            roots
                .add(cert.map_err(|_| Error::Unavailable)?)
                .map_err(|_| Error::Unavailable)?;
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider.clone(),
        )
        .build()
        .map_err(|_| Error::Unavailable)?;
        let mut tls = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| Error::Unavailable)?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
            .map_err(|_| Error::Unavailable)?;
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
        let listener = tokio::net::TcpListener::bind(self.config.bind)
            .await
            .map_err(|_| Error::Unavailable)?;
        let app: Router = self.private_router()?;
        let runtime = self.clone();
        let connections = Arc::new(tokio::sync::Semaphore::new(64));
        Ok(tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let Ok(permit) = connections.clone().try_acquire_owned() else {
                    continue;
                };
                let acceptor = acceptor.clone();
                let app = app.clone();
                let runtime = runtime.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let Ok(Ok(tls)) =
                        tokio::time::timeout(Duration::from_secs(2), acceptor.accept(stream)).await
                    else {
                        return;
                    };
                    if tls.get_ref().1.alpn_protocol() != Some(b"http/1.1") {
                        return;
                    }
                    let Some(cert) = tls.get_ref().1.peer_certificates().and_then(|c| c.first())
                    else {
                        return;
                    };
                    let Ok(spki) = spki(cert.as_ref()) else {
                        return;
                    };
                    let hash: [u8; 32] = Sha256::digest(spki).into();
                    if hash != runtime.config.bazaar_spki && hash != runtime.config.receipt_spki {
                        return;
                    }
                    let identity =
                        VerifiedClient::from_verified_tls13_http11(Identity::spki_sha256(hash));
                    let service = hyper::service::service_fn(
                        move |request: hyper::Request<hyper::body::Incoming>| {
                            let app = app.clone();
                            let identity = identity.clone();
                            let runtime = runtime.clone();
                            async move {
                                let mut request = request.map(Body::new);
                                request.extensions_mut().insert(identity);
                                if hash == runtime.config.bazaar_spki {
                                    request.extensions_mut().insert(super::transport::AuditPeer);
                                }
                                if runtime.config.current().is_err() {
                                    return Ok::<_, std::convert::Infallible>(
                                        oteryn_internal_api::guard::failure(
                                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                                        ),
                                    );
                                }
                                app.oneshot(request).await
                            }
                        },
                    );
                    let _ = hyper::server::conn::http1::Builder::new()
                        .header_read_timeout(Duration::from_secs(5))
                        .timer(hyper_util::rt::TokioTimer::new())
                        .serve_connection(TokioIo::new(tls), service)
                        .await;
                });
            }
        }))
    }
}
// Extraction only, never certificate validation. WebPKI has already verified DER,
// signatures, chain, time and EKU. Keep full DER SPKI bytes (including its header).
type DerElement<'a> = (u8, &'a [u8], &'a [u8], usize);
fn tlv(raw: &[u8]) -> Result<DerElement<'_>, Error> {
    if raw.len() < 2 {
        return Err(Error::Invalid);
    }
    let count = (raw[1] & 0x7f) as usize;
    let (header, len) = if raw[1] & 0x80 == 0 {
        (2, raw[1] as usize)
    } else {
        if count == 0 || count > 4 || raw.len() < 2 + count || raw[2] == 0 {
            return Err(Error::Invalid);
        }
        let mut len = 0usize;
        for b in &raw[2..2 + count] {
            len = len
                .checked_mul(256)
                .and_then(|n| n.checked_add(*b as usize))
                .ok_or(Error::Invalid)?;
        }
        (2 + count, len)
    };
    let end = header.checked_add(len).ok_or(Error::Invalid)?;
    if end > raw.len() {
        return Err(Error::Invalid);
    }
    Ok((raw[0], &raw[header..end], &raw[end..], end))
}
fn spki(cert: &[u8]) -> Result<&[u8], Error> {
    if cert.len() > 65536 {
        return Err(Error::Invalid);
    }
    let (tag, certificate, rest, _) = tlv(cert)?;
    if tag != 0x30 || !rest.is_empty() {
        return Err(Error::Invalid);
    }
    let (tag, mut tbs, _, _) = tlv(certificate)?;
    if tag != 0x30 {
        return Err(Error::Invalid);
    }
    if tbs.first() == Some(&0xa0) {
        tbs = tlv(tbs)?.2;
    }
    for tag in [0x02, 0x30, 0x30, 0x30, 0x30] {
        let (actual, _, rest, _) = tlv(tbs)?;
        if actual != tag {
            return Err(Error::Invalid);
        }
        tbs = rest;
    }
    let (tag, _, _, end) = tlv(tbs)?;
    if tag != 0x30 {
        return Err(Error::Invalid);
    }
    Ok(&tbs[..end])
}
