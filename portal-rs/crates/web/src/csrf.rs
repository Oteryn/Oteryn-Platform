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

pub async fn verify(
    config: &CsrfConfig,
    session: &BrowserSession,
    request: &mut Request,
) -> Result<(), StatusCode> {
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
    if let Some(site) = request.headers().get("sec-fetch-site") {
        match site.to_str() {
            Ok("same-origin" | "none") => {}
            _ => return Err(StatusCode::FORBIDDEN),
        }
    }
    if request.headers().get_all(header::ORIGIN).iter().count() > 1 {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        let origin = origin.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
        let uri: axum::http::Uri = origin.parse().map_err(|_| StatusCode::FORBIDDEN)?;
        if !matches!(uri.scheme_str(), Some("https" | "http"))
            || uri.authority().is_none()
            || uri.path() != "/"
            || uri.query().is_some()
            || !config.trusted_origins.iter().any(|v| v == origin)
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    let expected = session
        .existing_csrf_token()
        .await
        .ok_or(StatusCode::FORBIDDEN)?;
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
    Ok(())
}
