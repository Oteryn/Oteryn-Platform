//! Owner-accepted policy and deployment trust are separate inputs.
use super::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const INGRESS_PATHS: [&str; 3] = [admission::EVENT, admission::SEASON, admission::WATERMARK];

/// The TLS adapter inserts this only after a verified TLS 1.3 / HTTP 1.1 exchange.
#[derive(Clone, Debug)]
pub struct VerifiedPublisher([u8; 32]);

impl VerifiedPublisher {
    pub fn from_verified_tls13_http11(spki: [u8; 32]) -> Self {
        Self(spki)
    }
}

pub fn enabled() -> bool {
    std::env::var("OTERYN_LEAGUE_ENABLED").as_deref() == Ok("true")
}
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Policy {
    accepted_sources: bool,
    accepted_season_rules: bool,
    accepted_final_freeze: bool,
    accepted_pki: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            accepted_sources: true,
            accepted_season_rules: true,
            accepted_final_freeze: true,
            accepted_pki: true,
        }
    }
}
pub fn owner_flags(on: bool) -> Flags {
    owner_flags_from(on, std::env::var("OTERYN_LEAGUE_POLICY"))
}

pub(super) fn owner_flags_from(
    on: bool,
    policy: std::result::Result<String, std::env::VarError>,
) -> Flags {
    let policy = match policy {
        Ok(s) => serde_json::from_str::<Policy>(&s).ok(),
        Err(std::env::VarError::NotPresent) => Some(Policy::default()),
        Err(_) => None,
    };
    let mut flags = Flags {
        enabled: on,
        ingest: on,
        public: on,
        invalid_policy: policy.is_none(),
        ..Flags::default()
    };
    if let Some(p) = policy {
        flags.accepted_sources = p.accepted_sources;
        flags.accepted_season_rules = p.accepted_season_rules;
        flags.accepted_final_freeze = p.accepted_final_freeze;
        flags.accepted_pki = p.accepted_pki;
    }
    flags
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub source_authority: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub authority: String,
    pub audience: String,
    pub bind: String,
    pub ca: PathBuf,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub client_spki_sha256: String,
    pub recovery_file: PathBuf,
    pub requests_per_minute: usize,
}
fn value(key: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or(Error::Unavailable)
}
pub(super) fn unhex(s: &str) -> Result<Vec<u8>> {
    schema::ensure(
        s.len().is_multiple_of(2)
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )?;
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::Invalid))
        .collect()
}
impl Config {
    pub fn from_env() -> Result<Self> {
        let config = Self {
            authority: value("OTERYN_LEAGUE_SOURCE_AUTHORITY")?,
            audience: value("OTERYN_LEAGUE_BASE_URL")?,
            // The standalone transport API still owns these settings. The
            // portal's shared listener supplies its own address and trust.
            bind: std::env::var("OTERYN_LEAGUE_BIND").unwrap_or_else(|_| "127.0.0.1:0".into()),
            ca: std::env::var_os("OTERYN_LEAGUE_CLIENT_CA_DER")
                .unwrap_or_default()
                .into(),
            certificate: std::env::var_os("OTERYN_LEAGUE_CERTIFICATE_DER")
                .unwrap_or_default()
                .into(),
            private_key: std::env::var_os("OTERYN_LEAGUE_PRIVATE_KEY_DER")
                .unwrap_or_default()
                .into(),
            client_spki_sha256: normalize_spki(&value("OTERYN_LEAGUE_CLIENT_SPKI_SHA256")?)?,
            recovery_file: value("OTERYN_LEAGUE_RECOVERY_FILE")?.into(),
            requests_per_minute: std::env::var("OTERYN_LEAGUE_REQUESTS_PER_MINUTE")
                .ok()
                .map(|s| s.parse().map_err(|_| Error::Invalid))
                .transpose()?
                .unwrap_or(120),
        };
        schema::ensure(
            config.audience.starts_with("https://")
                && !config.audience.ends_with('/')
                && config.audience.len() <= 512
                && config.authority == "oteryn:game:primary"
                && (1..=1200).contains(&config.requests_per_minute),
        )?;
        config.connections()?;
        config.batch_limit()?;
        let audience: axum::http::Uri = config.audience.parse().map_err(|_| Error::Invalid)?;
        schema::ensure(
            audience.scheme_str() == Some("https")
                && audience.authority().is_some()
                && audience.query().is_none()
                && audience.path() == "/"
                && !config.audience.contains(['@', '#']),
        )?;
        config.recovery()?;
        Ok(config)
    }
    pub fn client_spki_pin(&self) -> Result<[u8; 32]> {
        let hex = self
            .client_spki_sha256
            .strip_prefix("sha256:")
            .ok_or(Error::Invalid)?;
        unhex(hex)?.try_into().map_err(|_| Error::Invalid)
    }

    pub fn shared_private_router(self, pool: sqlx::PgPool) -> Result<axum::Router> {
        shared_ingress::router(pool, self)
    }

    fn cap(name: &str, default: usize) -> Result<usize> {
        let n = match std::env::var(name) {
            Ok(s) => s.parse::<usize>().map_err(|_| Error::Invalid)?,
            Err(std::env::VarError::NotPresent) => default,
            Err(_) => return Err(Error::Invalid),
        };
        schema::ensure(n > 0 && n <= default)?;
        Ok(n)
    }
    pub(super) fn connections(&self) -> Result<usize> {
        Self::cap("OTERYN_LEAGUE_CONNECTION_LIMIT", 16)
    }
    pub(super) fn batch_limit(&self) -> Result<usize> {
        Self::cap("OTERYN_LEAGUE_BATCH_LIMIT", 64)
    }
    /// This operator-owned file must be on independent nonrollback storage. Both
    /// processes reread it for every admission/use; database state cannot mint it.
    pub fn recovery(&self) -> Result<Recovery> {
        let raw = std::fs::read(&self.recovery_file).map_err(|_| Error::Unavailable)?;
        schema::ensure(raw.len() <= 1024)?;
        let r: Recovery = serde_json::from_slice(&raw).map_err(|_| Error::Invalid)?;
        schema::ensure(r.source_authority == self.authority)?;
        schema::number(&r.projection_epoch, false)?;
        schema::number(&r.recovery_generation, false)?;
        Ok(r)
    }
}

pub(super) fn normalize_spki(raw: &str) -> Result<String> {
    let hex = raw.strip_prefix("sha256:").unwrap_or(raw);
    schema::ensure(unhex(hex)?.len() == 32)?;
    Ok(format!("sha256:{hex}"))
}

mod shared_ingress {
    use super::*;
    use axum::{
        Router,
        body::to_bytes,
        extract::{Request, State},
        http::{StatusCode, Version, header},
        response::{IntoResponse, Response},
        routing::post,
    };
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    #[derive(Clone)]
    struct Ingress {
        config: Arc<Config>,
        pool: sqlx::PgPool,
        spki: [u8; 32],
        requests: Arc<Mutex<VecDeque<u64>>>,
    }

    pub(super) fn router(pool: sqlx::PgPool, config: Config) -> Result<Router> {
        let spki = config.client_spki_pin()?;
        let mut routes = Router::new();
        for path in INGRESS_PATHS {
            routes = routes.route(path, post(ingest));
        }
        Ok(routes
            .method_not_allowed_fallback(|| async { response(StatusCode::BAD_REQUEST) })
            .with_state(Ingress {
                config: Arc::new(config),
                pool,
                spki,
                requests: Arc::default(),
            }))
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

    async fn ingest(State(ingress): State<Ingress>, request: Request) -> Response {
        if !request
            .extensions()
            .get::<VerifiedPublisher>()
            .is_some_and(|peer| peer.0 == ingress.spki)
        {
            return response(StatusCode::UNAUTHORIZED);
        }
        if request.version() != Version::HTTP_11 || request.uri().query().is_some() {
            return response(StatusCode::BAD_REQUEST);
        }
        let clock = match runtime::now() {
            Ok(clock) => clock,
            Err(_) => return response(StatusCode::SERVICE_UNAVAILABLE),
        };
        let allowed = ingress.requests.lock().is_ok_and(|mut queue| {
            while queue.front().is_some_and(|t| t.saturating_add(60) <= clock) {
                queue.pop_front();
            }
            if queue.len() >= ingress.config.requests_per_minute {
                return false;
            }
            queue.push_back(clock);
            true
        });
        if !allowed {
            return response(StatusCode::TOO_MANY_REQUESTS);
        }
        let path = request.uri().path().to_owned();
        let limit = match path.as_str() {
            admission::EVENT => 16384,
            admission::SEASON => 262144,
            admission::WATERMARK => 512,
            _ => return response(StatusCode::BAD_REQUEST),
        };
        let Ok(body) = to_bytes(request.into_body(), limit).await else {
            return response(StatusCode::BAD_REQUEST);
        };
        let identity = &ingress.config.client_spki_sha256;
        let result = async {
            let meta = admission::prepare(&ingress.config, &path, &body, identity)?;
            let meta = serde_json::to_vec(&meta).map_err(|_| Error::Unavailable)?;
            runtime::receive(
                &ingress.pool,
                &ingress.config,
                &path,
                &body,
                &meta,
                identity.as_bytes(),
            )
            .await
        }
        .await;
        response(match result {
            Ok(true) => StatusCode::OK,
            Err(Error::Superseded | Error::Conflict) => StatusCode::CONFLICT,
            Err(Error::Invalid) => StatusCode::BAD_REQUEST,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })
    }
}
