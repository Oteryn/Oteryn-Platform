//! Shared breached-password check for every operation that sets a new password
//! (registration, password change, password reset, recovery-key recovery).
//!
//! The corpus is the Have I Been Pwned range API (owner decision 2026-11-21) with a
//! 30-day PostgreSQL cache in `registration_password_ranges`. Only rows marked
//! `complete = true` decide a lookup; operator imports that kept a subset of the
//! range stay `complete = false` and are ignored. Only the five-character SHA-1
//! prefix leaves the process (k-anonymity); the password, the full hash and the
//! suffix list are never logged or sent. A failed fetch is `Unavailable`, never
//! "not breached". SHA-1 is only the lookup format; password storage is Argon2id.
//!
//! Callers run the check before opening a transaction: no database transaction or
//! connection is held across the HTTP request.
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};

/// Default range endpoint; the five-character prefix is appended.
pub const DEFAULT_URL: &str = "https://api.pwnedpasswords.com/range/";
/// Environment override for the range endpoint (tests point it at a local mock).
pub const URL_ENV: &str = "OTERYN_PWNED_PASSWORDS_URL";
/// Total budget for connect, TLS, request and body.
pub const TIMEOUT: Duration = Duration::from_secs(3);
/// Largest accepted response body.
pub const MAX_BODY: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreachError {
    /// The password appears in the breach corpus.
    Breached,
    /// The corpus could not be consulted; the password must not be accepted.
    Unavailable,
}
impl std::fmt::Display for BreachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Breached => "password appears in known data breaches",
            Self::Unavailable => "breached-password check unavailable",
        })
    }
}
impl std::error::Error for BreachError {}

/// Range source plus cache. Debug output never shows the pool.
#[derive(Clone)]
pub struct Checker {
    pool: PgPool,
    base: String,
    timeout: Duration,
}
impl Checker {
    /// Production checker: `OTERYN_PWNED_PASSWORDS_URL` or the HIBP default, 3 s budget.
    pub fn from_env(pool: PgPool) -> Self {
        let base = std::env::var(URL_ENV)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_URL.to_owned());
        Self::new(pool, base, TIMEOUT)
    }
    pub fn new(pool: PgPool, base: impl Into<String>, timeout: Duration) -> Self {
        Self {
            pool,
            base: base.into(),
            timeout,
        }
    }
}
impl std::fmt::Debug for Checker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checker")
            .field("base", &self.base)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// Uppercase SHA-1 hex of `password`, the range API's lookup key (prefix = first five).
pub fn sha1_hex(password: &str) -> String {
    use std::fmt::Write;
    Sha1::digest(password.as_bytes())
        .iter()
        .fold(String::with_capacity(40), |mut out, byte| {
            let _ = write!(out, "{byte:02X}");
            out
        })
}
fn upper_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

/// Decide whether `password` may be set. Never returns `Ok` without a complete range.
pub async fn check(checker: &Checker, password: &str) -> Result<(), BreachError> {
    let digest = sha1_hex(password);
    let (prefix, suffix) = digest.split_at(5);
    let suffixes = match cached(&checker.pool, prefix).await {
        Some(suffixes) => suffixes,
        None => {
            let suffixes = fetch(checker, prefix).await?;
            // A failed upsert still decides from the fetched, complete range.
            let _ = store(&checker.pool, prefix, &suffixes).await;
            suffixes
        }
    };
    if suffixes.iter().any(|s| s == suffix) {
        Err(BreachError::Breached)
    } else {
        Ok(())
    }
}

/// A fresh, complete, well-formed cached range, or `None` to fetch.
async fn cached(pool: &PgPool, prefix: &str) -> Option<Vec<String>> {
    let suffixes: Vec<String> = sqlx::query_scalar(
        "SELECT suffixes FROM registration_password_ranges WHERE prefix=$1 AND complete AND expires_at>now()",
    )
    .bind(prefix)
    .fetch_optional(pool)
    .await
    .ok()??;
    suffixes
        .iter()
        .all(|s| upper_hex(s, 35))
        .then_some(suffixes)
}

async fn store(pool: &PgPool, prefix: &str, suffixes: &[String]) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES($1,$2,now()+interval '30 days',true,pg_current_xact_id())
         ON CONFLICT(prefix) DO UPDATE SET suffixes=EXCLUDED.suffixes,expires_at=EXCLUDED.expires_at,complete=true,complete_xact=pg_current_xact_id(),updated_at=now()",
    )
    .bind(prefix)
    .bind(suffixes)
    .execute(pool)
    .await?;
    Ok(())
}

/// Strict range parser: every line is `SUFFIX:COUNT` (35 uppercase hex, decimal count),
/// an optional trailing `\r` is stripped, count-0 padding lines are dropped. An empty,
/// oversized, non-UTF-8 or malformed body is `Unavailable`.
pub fn parse(body: &[u8]) -> Result<Vec<String>, BreachError> {
    if body.is_empty() || body.len() > MAX_BODY {
        return Err(BreachError::Unavailable);
    }
    let text = std::str::from_utf8(body).map_err(|_| BreachError::Unavailable)?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let mut suffixes = Vec::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let (suffix, count) = line.split_once(':').ok_or(BreachError::Unavailable)?;
        if !upper_hex(suffix, 35) || count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(BreachError::Unavailable);
        }
        if count.bytes().any(|b| b != b'0') {
            suffixes.push(suffix.to_owned());
        }
    }
    Ok(suffixes)
}

async fn fetch(checker: &Checker, prefix: &str) -> Result<Vec<String>, BreachError> {
    let url = url::Url::parse(&format!("{}{prefix}", checker.base))
        .map_err(|_| BreachError::Unavailable)?;
    let local = url
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(BreachError::Unavailable);
    }
    let body = tokio::time::timeout(checker.timeout, exchange(&url))
        .await
        .map_err(|_| BreachError::Unavailable)??;
    parse(&body)
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// One GET without redirects, proxies, cookies or ambient credentials.
async fn exchange(url: &url::Url) -> Result<Vec<u8>, BreachError> {
    let unavailable = |_| BreachError::Unavailable;
    let host = url
        .host_str()
        .ok_or(BreachError::Unavailable)?
        .trim_matches(['[', ']']);
    let port = url
        .port_or_known_default()
        .ok_or(BreachError::Unavailable)?;
    let tcp = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(|_| BreachError::Unavailable)?;
    let io: Box<dyn Io> = if url.scheme() == "https" {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| BreachError::Unavailable)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(host.to_owned())
            .map_err(|_| BreachError::Unavailable)?;
        Box::new(
            tokio_rustls::TlsConnector::from(Arc::new(config))
                .connect(name, tcp)
                .await
                .map_err(|_| BreachError::Unavailable)?,
        )
    } else {
        Box::new(tcp)
    };
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(io))
        .await
        .map_err(unavailable)?;
    // Abort the connection task on success, refusal or cancellation/timeout.
    struct Connection(tokio::task::JoinHandle<()>);
    impl Drop for Connection {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _connection = Connection(tokio::spawn(async move {
        let _ = connection.await;
    }));
    let request = hyper::Request::builder()
        .method("GET")
        .uri(url.path())
        .header(
            "host",
            &url[url::Position::BeforeHost..url::Position::AfterPort],
        )
        .header("user-agent", "oteryn-portal")
        .header("add-padding", "true")
        .header("accept", "text/plain")
        .header("accept-encoding", "identity")
        .body(Empty::<axum::body::Bytes>::new())
        .map_err(|_| BreachError::Unavailable)?;
    let response = sender.send_request(request).await.map_err(unavailable)?;
    if response.status() != 200
        || response
            .headers()
            .get("content-encoding")
            .is_some_and(|v| v != "identity")
        || response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|n| n > MAX_BODY as u64)
    {
        return Err(BreachError::Unavailable);
    }
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(unavailable)?;
        if let Some(data) = frame.data_ref() {
            if data.len() > MAX_BODY.saturating_sub(bytes.len()) {
                return Err(BreachError::Unavailable);
            }
            bytes.extend_from_slice(data);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "breach_tests.rs"]
pub(crate) mod tests;
