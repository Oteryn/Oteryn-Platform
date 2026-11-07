//! Dedicated Game-initiated TLS 1.3 / HTTP 1.1 publisher listener. Public headers
//! cannot manufacture a verified peer. No Game listener/pull API is assumed.
use super::{
    admission,
    config::{self, Config},
    runtime, *,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sqlx::PgPool;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use tower::ServiceExt;
#[derive(Clone)]
struct Surface {
    pool: PgPool,
    config: Arc<Config>,
    rate: Arc<Mutex<VecDeque<u64>>>,
}
fn response(status: StatusCode) -> Response {
    if status == StatusCode::OK {
        (
            status,
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_TYPE, "application/json"),
            ],
            "{\"contract_version\":1,\"result\":\"accepted\"}",
        )
            .into_response()
    } else {
        (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
    }
}
async fn ingest(State(s): State<Surface>, request: Request) -> Response {
    if request.version() != axum::http::Version::HTTP_11 || request.uri().query().is_some() {
        return response(StatusCode::BAD_REQUEST);
    }
    let path = request.uri().path().to_owned();
    let clock = match runtime::now() {
        Ok(t) => t,
        Err(_) => return response(StatusCode::SERVICE_UNAVAILABLE),
    };
    let allowed = s
        .rate
        .lock()
        .map(|mut queue| {
            while queue.front().is_some_and(|t| t.saturating_add(60) <= clock) {
                queue.pop_front();
            }
            if queue.len() >= s.config.requests_per_minute {
                return false;
            }
            queue.push_back(clock);
            true
        })
        .unwrap_or(false);
    if !allowed {
        return response(StatusCode::TOO_MANY_REQUESTS);
    }
    let Some(peer) = request.extensions().get::<VerifiedPeer>().cloned() else {
        return response(StatusCode::UNAUTHORIZED);
    };
    if peer.0 != s.config.client_spki_sha256 {
        return response(StatusCode::UNAUTHORIZED);
    }
    let max = match path.as_str() {
        admission::EVENT => 16384,
        admission::SEASON => 262144,
        admission::WATERMARK => 512,
        _ => return response(StatusCode::BAD_REQUEST),
    };
    let Ok(body) = to_bytes(request.into_body(), max).await else {
        return response(StatusCode::BAD_REQUEST);
    };
    let m = match admission::prepare(&s.config, &path, &body, &peer.0) {
        Ok(m) => m,
        Err(Error::Conflict | Error::Superseded) => return response(StatusCode::CONFLICT),
        Err(Error::Unavailable) => return response(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => return response(StatusCode::BAD_REQUEST),
    };
    let Ok(meta) = serde_json::to_vec(&m) else {
        return response(StatusCode::SERVICE_UNAVAILABLE);
    };
    let signature = peer.0.as_bytes();
    match runtime::receive(&s.pool, &s.config, &path, &body, &meta, signature).await {
        Ok(true) => response(StatusCode::OK),
        Ok(false) => response(StatusCode::SERVICE_UNAVAILABLE),
        Err(Error::Superseded) => response(StatusCode::CONFLICT),
        Err(Error::Conflict) => response(StatusCode::CONFLICT),
        Err(Error::Invalid) => response(StatusCode::BAD_REQUEST),
        Err(_) => response(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// None means the single feature flag is off or deployment trust is incomplete;
/// pages remain mounted/refusing when enabled, never switched to fake authority.
pub async fn start_private_listener(pool: PgPool) -> Result<Option<JoinHandle<()>>> {
    if !config::enabled() {
        return Ok(None);
    }
    let Ok(c) = Config::from_env() else {
        return Ok(None);
    };
    start(pool, c).await.map(Some)
}
async fn start(pool: PgPool, c: Config) -> Result<JoinHandle<()>> {
    let connection_limit = c.connections()?;
    let load = |p: &std::path::Path| std::fs::read(p).map_err(|_| Error::Unavailable);
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(load(&c.ca)?))
        .map_err(|_| Error::Invalid)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()
    .map_err(|_| Error::Invalid)?;
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| Error::Invalid)?
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![CertificateDer::from(load(&c.certificate)?)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(load(&c.private_key)?)),
        )
        .map_err(|_| Error::Invalid)?;
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let listener = tokio::net::TcpListener::bind(&c.bind)
        .await
        .map_err(|_| Error::Unavailable)?;
    let surface = Surface {
        pool,
        config: Arc::new(c),
        rate: Arc::new(Mutex::new(VecDeque::new())),
    };
    let mut app = Router::new();
    for path in [admission::EVENT, admission::SEASON, admission::WATERMARK] {
        app = app.route(path, post(ingest));
    }
    let app = app
        .fallback(|| async { response(StatusCode::BAD_REQUEST) })
        .method_not_allowed_fallback(|| async { response(StatusCode::BAD_REQUEST) })
        .with_state(surface.clone());
    let limit = Arc::new(tokio::sync::Semaphore::new(connection_limit));
    Ok(tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let Ok(permit) = limit.clone().try_acquire_owned() else {
                continue;
            };
            let acceptor = acceptor.clone();
            let app = app.clone();
            let expected = surface.config.client_spki_sha256.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let Ok(Ok(tls)) =
                    tokio::time::timeout(std::time::Duration::from_secs(2), acceptor.accept(tcp))
                        .await
                else {
                    return;
                };
                let (_, connection) = tls.get_ref();
                if connection.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
                    || connection.alpn_protocol() != Some(b"http/1.1".as_slice())
                {
                    return;
                }
                let Some(peer) = connection
                    .peer_certificates()
                    .and_then(|c| c.first())
                    .and_then(|cert| certificate_identity(cert.as_ref()).ok())
                else {
                    return;
                };
                // Only this listener can inject VerifiedPeer, after WebPKI verifies
                // chain, validity and clientAuth EKU. Headers never establish it.
                let authorized = peer == expected;
                let io = hyper_util::rt::TokioIo::new(tls);
                let service = hyper::service::service_fn(
                    move |mut request: hyper::Request<hyper::body::Incoming>| {
                        let peer = peer.clone();
                        let app = app.clone();
                        async move {
                            if !authorized {
                                return Ok(response(StatusCode::UNAUTHORIZED));
                            }
                            request.extensions_mut().insert(VerifiedPeer(peer));
                            app.oneshot(request.map(Body::new)).await
                        }
                    },
                );
                let mut builder = hyper::server::conn::http1::Builder::new();
                builder.keep_alive(false).max_buf_size(65536);
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    builder.serve_connection(io, service),
                )
                .await;
            });
        }
    }))
}

#[derive(Clone)]
struct VerifiedPeer(String);
fn certificate_identity(der: &[u8]) -> Result<String> {
    use x509_parser::prelude::FromDer;
    let (rest, certificate) =
        x509_parser::certificate::X509Certificate::from_der(der).map_err(|_| Error::Invalid)?;
    schema::ensure(rest.is_empty())?;
    Ok(format!(
        "sha256:{}",
        schema::hash(certificate.public_key().raw)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::tests::{Db, real_config, wire, write_recovery};

    #[tokio::test]
    async fn game_json_without_headers_is_admitted_only_for_verified_pinned_peer()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let Some(db) = Db::new().await? else {
            return Ok(());
        };
        let dir = tempfile::tempdir()?;
        let config = real_config(dir.path());
        write_recovery(&config, "1")?;
        let peer = config.client_spki_sha256.clone();
        let app = Router::new()
            .route(admission::EVENT, post(ingest))
            .with_state(Surface {
                pool: db.pool.clone(),
                config: Arc::new(config.clone()),
                rate: Arc::new(Mutex::new(VecDeque::new())),
            });
        let raw = wire("settlement")?;
        let request = |identity: Option<&str>| -> std::result::Result<_, axum::http::Error> {
            let mut req = axum::http::Request::builder()
                .method("POST")
                .uri(admission::EVENT)
                .version(axum::http::Version::HTTP_11)
                .header("content-type", "application/json")
                .body(Body::from(raw.clone()))?;
            if let Some(identity) = identity {
                req.extensions_mut()
                    .insert(VerifiedPeer(identity.to_owned()));
            }
            Ok(req)
        };
        for identity in [None, Some("sha256:another-purpose")] {
            let reply = app.clone().oneshot(request(identity)?).await?;
            assert_eq!(reply.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(reply.headers()[header::CACHE_CONTROL], "no-store");
            assert!(to_bytes(reply.into_body(), 256).await?.is_empty());
        }
        // Initial 503 asks Game to retry retained bytes until the worker commits.
        assert_eq!(
            app.clone().oneshot(request(Some(&peer))?).await?.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        runtime::FeedJob {
            pool: db.pool.clone(),
        }
        .run_config(config)
        .await?;
        let reply = app.oneshot(request(Some(&peer))?).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(reply.headers()[header::CACHE_CONTROL], "no-store");
        assert!(to_bytes(reply.into_body(), 256).await?.len() <= 256);
        db.close().await
    }
}
