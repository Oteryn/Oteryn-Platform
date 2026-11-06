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
use base64::{Engine, engine::general_purpose::STANDARD};
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
    if request.version() != axum::http::Version::HTTP_11 {
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
    let Some(meta) = request
        .headers()
        .get("x-oteryn-league-admission")
        .and_then(|h| h.to_str().ok())
        .filter(|s| s.len() <= 44000)
        .and_then(|s| STANDARD.decode(s).ok())
    else {
        return response(StatusCode::UNAUTHORIZED);
    };
    let Some(signature) = request
        .headers()
        .get("x-oteryn-league-signature")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| config::unhex(s).ok())
        .filter(|s| s.len() == 64)
    else {
        return response(StatusCode::UNAUTHORIZED);
    };
    let max = match path.as_str() {
        admission::EVENT => 16384,
        admission::SEASON | admission::CUT => 262144,
        admission::WATERMARK => 512,
        _ => return response(StatusCode::NOT_FOUND),
    };
    let Ok(body) = to_bytes(request.into_body(), max).await else {
        return response(StatusCode::BAD_REQUEST);
    };
    match admission::authenticate(&s.config, &path, &meta, &body, &signature) {
        Ok(_) => {}
        Err(Error::Superseded) => {
            return (
                StatusCode::OK,
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::CONTENT_TYPE, "application/json"),
                ],
                "{\"contract_version\":1,\"result\":\"superseded\"}",
            )
                .into_response();
        }
        Err(Error::Unavailable) => return response(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => return response(StatusCode::UNAUTHORIZED),
    }
    match runtime::receive(&s.pool, &s.config, &path, &body, &meta, &signature).await {
        Ok(true) => response(StatusCode::OK),
        Ok(false) => response(StatusCode::SERVICE_UNAVAILABLE),
        Err(Error::Superseded) => response(StatusCode::OK),
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
    for path in [
        admission::EVENT,
        admission::SEASON,
        admission::WATERMARK,
        admission::CUT,
    ] {
        app = app.route(path, post(ingest));
    }
    let app = app.with_state(surface.clone());
    let limit = Arc::new(tokio::sync::Semaphore::new(connection_limit));
    Ok(tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let Ok(permit) = limit.clone().try_acquire_owned() else {
                continue;
            };
            let acceptor = acceptor.clone();
            let app = app.clone();
            let expected = surface.config.peer_certificate_sha256.clone();
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
                    || !connection
                        .peer_certificates()
                        .and_then(|c| c.first())
                        .is_some_and(|cert| schema::hash(cert.as_ref()) == expected)
                {
                    return;
                }
                let io = hyper_util::rt::TokioIo::new(tls);
                let service = hyper::service::service_fn(
                    move |request: hyper::Request<hyper::body::Incoming>| {
                        let app = app.clone();
                        async move { app.oneshot(request.map(Body::new)).await }
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
