//! Admission and response policy for the fully composed public listener.

use axum::{
    Router,
    body::Body,
    extract::Request,
    handler::Handler,
    http::{HeaderMap, HeaderValue, Method, StatusCode, header, uri::Authority},
    middleware::{self, Next},
    response::Response,
    routing::{MethodRouter, get},
};

use crate::surface::{self, HSTS, PERMISSIONS, REFERRER};

const MAX_FIELD_BYTES: usize = 8 * 1024;
const MAX_HEADER_BYTES: usize = 12 * 1024;
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";

/// Attach after the last public route has been registered so early refusals,
/// redirects and routes supplied by other crates all receive the common policy.
pub fn secure_public(router: Router) -> Router {
    router.layer(middleware::from_fn(policy))
}

/// Register an HTML GET that must negotiate before reading one-time session state.
pub fn html_get<H, T, S>(handler: H) -> MethodRouter<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    get(handler).layer(middleware::from_fn(require_html))
}

async fn require_html(request: Request, next: Next) -> Response {
    if accepts_html(request.headers()) {
        next.run(request).await
    } else {
        let mut response = refusal(
            StatusCode::NOT_ACCEPTABLE,
            request.uri().path(),
            language(&request),
        );
        response.headers_mut().insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        );
        response
    }
}

async fn policy(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let static_asset = request.uri().path().starts_with("/static/");
    let head = request.method() == Method::HEAD;
    let reads_document = request.method() == Method::GET || head;
    let accepts_html = !reads_document || accepts_html(headers);

    let mut response = if oversized(headers) {
        refusal(
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            request.uri().path(),
            language(&request),
        )
    } else if malformed_host(&request) {
        refusal(
            StatusCode::BAD_REQUEST,
            request.uri().path(),
            language(&request),
        )
    } else if accepts_html {
        next.run(request).await
    } else {
        // The public listener also serves JSON, downloads and other protocols.
        // Stateful HTML GETs also negotiate at registration via html_get.
        let path = request.uri().path().to_owned();
        let lang = language(&request);
        let response = next.run(request).await;
        if response.status().is_success() && is_html(response.headers()) {
            refusal(StatusCode::NOT_ACCEPTABLE, &path, lang)
        } else {
            response
        }
    };

    let html = is_html(response.headers());
    let status = response.status();
    let headers = response.headers_mut();
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert(HeaderValue::from_static(CSP));
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static(HSTS),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers
        .entry(header::REFERRER_POLICY)
        .or_insert(HeaderValue::from_static(REFERRER));
    headers.insert("permissions-policy", HeaderValue::from_static(PERMISSIONS));

    let no_store = headers
        .get_all(header::CACHE_CONTROL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|directive| directive.trim().eq_ignore_ascii_case("no-store"));
    if !no_store
        && ((html && !static_asset) || !(status.is_success() || status == StatusCode::NOT_MODIFIED))
    {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    } else {
        headers
            .entry(header::CACHE_CONTROL)
            .or_insert(HeaderValue::from_static("no-store"));
    }
    if head {
        *response.body_mut() = Body::empty();
    }
    response
}

fn oversized(headers: &HeaderMap) -> bool {
    headers
        .iter()
        .any(|(_, value)| value.len() > MAX_FIELD_BYTES)
        || headers
            .get_all(header::COOKIE)
            .iter()
            .map(|value| value.len() + 2)
            .sum::<usize>()
            > MAX_FIELD_BYTES + 2
        || headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.len() + 4)
            .sum::<usize>()
            > MAX_HEADER_BYTES
}

fn malformed_host(request: &Request) -> bool {
    let mut values = request.headers().get_all(header::HOST).iter();
    let authority = request.uri().authority();
    let host = match values.next() {
        Some(value) => match value.to_str().unwrap_or_default().parse::<Authority>() {
            Ok(host) => host,
            Err(_) => return true,
        },
        None => match authority {
            Some(authority) => authority.clone(),
            None => return false,
        },
    };
    let name = host.host();
    let valid_name = if let Some(ip) = name.strip_prefix('[').and_then(|ip| ip.strip_suffix(']')) {
        ip.parse::<std::net::Ipv6Addr>().is_ok()
    } else {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    };
    values.next().is_some()
        || host.as_str().contains('@')
        || !valid_name
        || (name.len() < host.as_str().len() && host.port_u16().is_none())
        || authority
            .is_some_and(|authority| !authority.as_str().eq_ignore_ascii_case(host.as_str()))
}

fn accepts_html(headers: &HeaderMap) -> bool {
    !headers.contains_key(header::ACCEPT)
        || headers
            .get_all(header::ACCEPT)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .filter_map(|value| {
                let mut parts = value.split(';').map(str::trim);
                let media = parts.next()?;
                let specificity = if media.eq_ignore_ascii_case("text/html") {
                    2
                } else if media.eq_ignore_ascii_case("text/*") {
                    1
                } else if media == "*/*" {
                    0
                } else {
                    return None;
                };
                let acceptable = parts
                    .filter_map(|part| part.split_once('='))
                    .find(|(name, _)| name.trim().eq_ignore_ascii_case("q"))
                    .is_none_or(|(_, value)| {
                        value
                            .trim()
                            .parse::<f32>()
                            .is_ok_and(|q| q > 0.0 && q <= 1.0)
                    });
                Some((specificity, acceptable))
            })
            .max()
            .is_some_and(|(_, acceptable)| acceptable)
}

fn is_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.eq_ignore_ascii_case("text/html"))
}

fn language(request: &Request) -> surface::Language {
    surface::path_language(request.uri().path())
        .unwrap_or_else(|| surface::negotiate(request.headers()))
}

fn refusal(status: StatusCode, path: &str, lang: surface::Language) -> Response {
    let chrome =
        oteryn_ui::Registry::default().chrome(lang.code(), path, oteryn_ui::Account::Anonymous);
    let mut response = oteryn_ui::error(&chrome, status);
    response.headers_mut().insert(
        header::CONTENT_LANGUAGE,
        HeaderValue::from_static(lang.code()),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn late_mounted_page_and_admission_refusals_share_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let app = secure_public(surface::module_routes().with_state(crate::AppState {
            db: oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?,
        }));
        let oversized = "a".repeat(MAX_FIELD_BYTES + 1);
        let split_cookie = "x=".to_owned() + &"a".repeat(MAX_FIELD_BYTES / 2);
        let requests = [
            (
                Request::get("/en/")
                    .header(header::HOST, "localhost:8443")
                    .header(header::ACCEPT, "text/*"),
                StatusCode::OK,
                "en",
                None,
            ),
            (
                Request::get("/pl/").header(header::HOST, "localhost:bad"),
                StatusCode::BAD_REQUEST,
                "pl",
                Some("Nieprawidłowe żądanie"),
            ),
            (
                Request::get("/en/").header(header::ACCEPT, "*/*;q=1, text/html;q=0"),
                StatusCode::NOT_ACCEPTABLE,
                "en",
                Some("Response format not available"),
            ),
            (
                Request::get("/pl/").header("x-size", oversized),
                StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                "pl",
                Some("Nagłówki żądania są za duże"),
            ),
            (
                Request::get("/en/")
                    .header(header::COOKIE, &split_cookie)
                    .header(header::COOKIE, &split_cookie),
                StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                "en",
                Some("Request headers are too large"),
            ),
        ];
        for (request, status, language, title) in requests {
            let response = app.clone().oneshot(request.body(Body::empty())?).await?;
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::STRICT_TRANSPORT_SECURITY], HSTS);
            assert_eq!(response.headers()["permissions-policy"], PERMISSIONS);
            assert_eq!(response.headers()[header::REFERRER_POLICY], REFERRER);
            assert_eq!(
                response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                "nosniff"
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert!(
                response.headers()[header::CONTENT_SECURITY_POLICY]
                    .to_str()?
                    .contains("frame-ancestors 'none'")
            );
            if let Some(title) = title {
                assert_eq!(response.headers()[header::CONTENT_LANGUAGE], language);
                let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
                assert!(std::str::from_utf8(&body)?.contains(title));
            }
        }
        Ok(())
    }
    #[tokio::test]
    async fn published_icon_keeps_public_cache_and_denies_framing()
    -> Result<(), Box<dyn std::error::Error>> {
        let response = secure_public(crate::favicon::router())
            .oneshot(
                Request::get("/favicon.ico")
                    .header(header::ACCEPT, "image/svg+xml")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/svg+xml");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=86400"
        );
        assert!(
            response.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()?
                .ends_with("frame-ancestors 'none'")
        );
        Ok(())
    }
}
