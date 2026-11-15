//! Direct public TLS transport for native device sessions. No proxy trust path.
use axum::{
    Router,
    extract::{ConnectInfo, Request},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

/// Evidence created exclusively after this listener's successful TLS handshake.
/// There is deliberately no public constructor or header conversion.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedHttps(());

/// All three settings must be supplied together. No configuration means no listener.
/// Activation of the routes remains a separate, default-off feature gate.
pub async fn start(
    app: Router,
) -> Result<Option<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
    let keys = [
        "OTERYN_NATIVE_TLS_BIND",
        "OTERYN_NATIVE_TLS_CERT_FILE",
        "OTERYN_NATIVE_TLS_KEY_FILE",
    ];
    let values: Vec<_> = keys
        .iter()
        .map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
        .collect();
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let values = values
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("incomplete native TLS configuration")?;
    let bind: SocketAddr = values[0].parse()?;
    let chain =
        CertificateDer::pem_file_iter(PathBuf::from(&values[1]))?.collect::<Result<Vec<_>, _>>()?;
    let key = PrivateKeyDer::from_pem_file(PathBuf::from(&values[2]))?;
    let config = server_config(chain, key)?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    Ok(Some(tokio::spawn(serve(listener, config, app))))
}
fn server_config(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<rustls::ServerConfig, rustls::Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(chain, key)?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}
/// Serve native routes on an already bound listener; request trust is established by TLS.
pub async fn serve(listener: tokio::net::TcpListener, config: rustls::ServerConfig, app: Router) {
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let slots = Arc::new(tokio::sync::Semaphore::new(64));
    loop {
        let (socket, peer) = match accept_with_retry(
            || listener.accept(),
            |error| eprintln!("native TLS accept failed: {error}"),
        )
        .await
        {
            Ok(connection) => connection,
            Err(_) => return,
        };
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let acceptor = acceptor.clone();
        let app = app.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let Ok(Ok(stream)) =
                tokio::time::timeout(Duration::from_secs(5), acceptor.accept(socket)).await
            else {
                return;
            };
            if stream.get_ref().1.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
                || stream
                    .get_ref()
                    .1
                    .alpn_protocol()
                    .is_some_and(|p| p != b"http/1.1")
            {
                return;
            }
            let scoped = app.layer(axum::middleware::from_fn(
                move |mut request: Request, next: axum::middleware::Next| async move {
                    // Both values originate in this accepted socket, never HTTP metadata.
                    request.extensions_mut().insert(VerifiedHttps(()));
                    request.extensions_mut().insert(ConnectInfo(peer));
                    next.run(request).await
                },
            ));
            let service = hyper_util::service::TowerToHyperService::new(scoped);
            let connection = hyper::server::conn::http1::Builder::new()
                .max_buf_size(65536)
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service);
            let _ = tokio::time::timeout(Duration::from_secs(60), connection).await;
        });
    }
}
#[cfg(test)]
#[path = "devices_tls_tests.rs"]
pub(super) mod tests;

/// Retry transient accept failures with capped delay and return sustained failures.
pub async fn accept_with_retry<T, F, Fut, Report>(
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
mod accept_retry_tests {
    #[tokio::test]
    async fn persistent_accept_errors_are_delayed_recorded_and_propagated()
    -> Result<(), Box<dyn std::error::Error>> {
        let started = std::time::Instant::now();
        let mut causes = Vec::new();
        let mut task = tokio::spawn(async move {
            let result = super::accept_with_retry(
                || async {
                    tokio::task::yield_now().await;
                    Err::<(), _>(std::io::Error::from_raw_os_error(24))
                },
                |error| causes.push(error.raw_os_error()),
            )
            .await;
            (result, causes)
        });
        let completed = tokio::time::timeout(std::time::Duration::from_secs(5), &mut task).await;
        task.abort();
        let (result, causes) = completed??;
        assert_eq!(
            result
                .err()
                .ok_or("persistent failure must propagate")?
                .raw_os_error(),
            Some(24)
        );
        assert_eq!(causes, vec![Some(24); 8]);
        assert!(started.elapsed() >= std::time::Duration::from_millis(2575));
        Ok(())
    }
}
