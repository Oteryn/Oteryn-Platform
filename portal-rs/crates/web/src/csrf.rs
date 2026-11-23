//! Synchronizer tokens for browser mutations, with independent origin checks.
use crate::session::BrowserSession;
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{Method, StatusCode, header},
};
use subtle::ConstantTimeEq;

/// Empty API exemptions by default. Entries match exact method and path and require JSON.
/// Origins are deployment-configured serialized origins (scheme, host, optional port).
#[derive(Clone, Debug, Default)]
pub struct CsrfConfig {
    pub trusted_origins: Vec<String>,
    pub api_allow_list: Vec<(Method, String)>,
}

/// Normalize an HTTPS host and optional port for routing and serialized origins.
pub fn https_authority(value: &str) -> Option<String> {
    let authority: axum::http::uri::Authority = value.parse().ok()?;
    if authority.host().is_empty()
        || value.contains('@')
        || (authority.port().is_none() && value != authority.host())
    {
        return None;
    }
    let url = url::Url::parse(&format!("https://{authority}")).ok()?;
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

/// Apply the origin and fetch-metadata checks without reading a body.
pub fn verify_origin(
    headers: &axum::http::HeaderMap,
    trusted_origins: &[String],
) -> Result<(), StatusCode> {
    if let Some(site) = headers.get("sec-fetch-site") {
        match site.to_str() {
            Ok("same-origin" | "none") => {}
            _ => return Err(StatusCode::FORBIDDEN),
        }
    }
    if headers.get_all(header::ORIGIN).iter().count() > 1 {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = origin.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
        let uri: axum::http::Uri = origin.parse().map_err(|_| StatusCode::FORBIDDEN)?;
        if !matches!(uri.scheme_str(), Some("https" | "http"))
            || uri.authority().is_none()
            || uri.path() != "/"
            || uri.query().is_some()
            || !trusted_origins.iter().any(|v| v == origin)
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(())
}

/// Request evidence emitted after a browser mutation passes token and origin verification.
#[derive(Clone, Copy)]
pub struct Verified([u8; 32]);

impl Verified {
    pub fn matches_session(self, session_hash: &[u8; 32]) -> bool {
        bool::from(self.0.ct_eq(session_hash))
    }
}

impl std::fmt::Debug for Verified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Verified").finish_non_exhaustive()
    }
}

/// Opt a browser route into localized CSRF refusal pages.
#[derive(Clone, Copy, Debug)]
pub struct LocalizedRefusal;

pub async fn verify(
    config: &CsrfConfig,
    session: &BrowserSession,
    request: &mut Request,
) -> Result<(), StatusCode> {
    request.extensions_mut().remove::<Verified>();
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return Ok(());
    }
    let json = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(';').next() == Some("application/json"));
    if request.uri().path().starts_with("/api/")
        && json
        && config
            .api_allow_list
            .iter()
            .any(|(method, path)| method == request.method() && path == request.uri().path())
    {
        return Ok(());
    }
    verify_origin(request.headers(), &config.trusted_origins)?;
    let (expected, session_hash) = session.csrf_proof().await.ok_or(StatusCode::FORBIDDEN)?;
    let headers = request.headers().get_all("x-csrf-token");
    if headers.iter().count() > 1 {
        return Err(StatusCode::FORBIDDEN);
    }
    let supplied = if let Some(value) = headers.iter().next() {
        value
            .to_str()
            .map_err(|_| StatusCode::FORBIDDEN)?
            .to_owned()
    } else {
        let form = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(';').next() == Some("application/x-www-form-urlencoded"));
        if !form {
            return Err(StatusCode::FORBIDDEN);
        }
        let body = std::mem::replace(request.body_mut(), Body::empty());
        let bytes = to_bytes(body, crate::MAX_BODY_BYTES)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let fields: Vec<(String, String)> =
            serde_urlencoded::from_bytes(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        *request.body_mut() = Body::from(bytes);
        let mut tokens = fields.iter().filter(|(key, _)| key == "_csrf");
        let token = tokens.next().ok_or(StatusCode::FORBIDDEN)?.1.clone();
        if tokens.next().is_some() {
            return Err(StatusCode::FORBIDDEN);
        }
        token
    };
    if supplied.len() != 64 || !bool::from(expected.as_bytes().ct_eq(supplied.as_bytes())) {
        return Err(StatusCode::FORBIDDEN);
    }
    request.extensions_mut().insert(Verified(session_hash));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::https_authority;

    #[test]
    fn https_authority_matches_browser_serialization() {
        for (configured, browser) in [
            ("Staff.Example.test:443", "staff.example.test"),
            ("[0:0:0:0:0:0:0:1]:443", "[::1]"),
            ("[::ffff:192.0.2.1]:8443", "[::ffff:c000:201]:8443"),
        ] {
            assert_eq!(https_authority(configured).as_deref(), Some(browser));
            assert_eq!(https_authority(browser).as_deref(), Some(browser));
        }
        for invalid in [
            "staff.example.test:",
            "user@staff.example.test",
            "staff.example.test:65536",
        ] {
            assert_eq!(https_authority(invalid), None);
        }
    }
}
