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
async fn exchange_deadline<F: std::future::Future>(
    duration: Duration,
    exchange: F,
) -> Result<F::Output, tokio::time::error::Elapsed> {
    tokio::time::timeout(duration, exchange).await
}
impl Runtime {
    pub async fn start_private_listener(
        &self,
    ) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
        self.config.current()?;
        let key = PrivateKeyDer::from_pem_reader(oteryn_core::secret_file::open(
            "OTERYN_BAZAAR_TLS_KEY_FILE",
            &self.config.private_key,
        )?)
        .map_err(|_| "invalid PEM in OTERYN_BAZAAR_TLS_KEY_FILE")?;
        let certs = CertificateDer::pem_file_iter(&self.config.certificate)
            .map_err(|_| Error::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
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
            loop {
                let (stream, _) = match accept_with_retry(
                    || listener.accept(),
                    |error| eprintln!("bazaar private listener accept failed: {error}"),
                )
                .await
                {
                    Ok(connection) => connection,
                    Err(_) => return,
                };
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
                    let _ = exchange_deadline(Duration::from_secs(30), async {
                        hyper::server::conn::http1::Builder::new()
                            .header_read_timeout(Duration::from_secs(5))
                            .timer(hyper_util::rt::TokioTimer::new())
                            .serve_connection(TokioIo::new(tls), service)
                            .await
                    })
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
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    async fn sustained_accept_failure_returns_to_supervisor()
    -> Result<(), Box<dyn std::error::Error>> {
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
        let mut recorded = 0;
        let result = accept_with_retry(
            || async { Err::<(), _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied)) },
            |_| recorded += 1,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(recorded, 8);
        Ok(())
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;

    #[tokio::test]
    async fn stalled_body_deadline_releases_connection_permit()
    -> Result<(), Box<dyn std::error::Error>> {
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
        use tokio::io::AsyncWriteExt;
        let permits = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = permits.clone().acquire_owned().await?;
        let (server, mut client) = tokio::io::duplex(4096);
        let task = tokio::spawn(async move {
            let _permit = permit;
            let service = hyper::service::service_fn(
                |request: hyper::Request<hyper::body::Incoming>| async move {
                    let _ = axum::body::to_bytes(Body::new(request.into_body()), 1024).await;
                    Ok::<_, std::convert::Infallible>(hyper::Response::new(Body::empty()))
                },
            );
            exchange_deadline(Duration::from_millis(100), async {
                hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(server), service)
                    .await
            })
            .await
        });
        client
            .write_all(b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\nx")
            .await?;
        assert!(
            tokio::time::timeout(Duration::from_secs(2), task)
                .await??
                .is_err()
        );
        assert_eq!(permits.available_permits(), 1);
        Ok(())
    }
    #[tokio::test]
    async fn postgres_exchange_deadline_cancels_issuer_wait_and_releases_pool()
    -> Result<(), Box<dyn std::error::Error>> {
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let mut blocker = pool.begin().await?;
        sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE")
            .execute(&mut *blocker)
            .await?;
        let result = exchange_deadline(Duration::from_millis(100), async {
            let mut waiting = crate::game::authority_http::begin_exchange(&pool).await?;
            sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE")
                .execute(&mut *waiting)
                .await?;
            waiting.commit().await
        })
        .await;
        assert!(result.is_err());
        let mut connection = tokio::time::timeout(Duration::from_secs(7), pool.acquire()).await??;
        sqlx::query("SELECT 1").execute(&mut *connection).await?;
        blocker.rollback().await?;
        Ok(())
    }
}
