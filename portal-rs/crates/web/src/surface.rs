//! Shared response policy and module composition for public HTTP routes.

use axum::{
    Router,
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};

use crate::AppState;

/// Per-response script nonce. Handlers may read this request extension.
#[derive(Clone, Debug)]
pub struct CspNonce(pub String);

/// Transport policy. Disable HSTS only for a local HTTP development server.
#[derive(Clone, Copy, Debug)]
pub struct WebOptions {
    pub hsts: bool,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self { hsts: true }
    }
}

/// Compose module route functions before applying the common response policy.
/// Each later module adds one `.merge(module_routes())` here.
pub fn module_router() -> Router<AppState> {
    Router::new()
}

pub(super) fn module_routes() -> Router<AppState> {
    Router::new()
        .route("/pl", get(localized_home))
        .route("/en", get(localized_home))
        .route("/pl/", get(localized_home))
        .route("/en/", get(localized_home))
        .route("/language/{lang}", get(switch_language))
}

/// Build the public surface with explicit transport options.
pub fn router_with_options(state: AppState, static_dir: &str, options: WebOptions) -> Router {
    crate::router(state, static_dir).layer(middleware::from_fn_with_state(options, transport))
}

pub(super) fn finish(router: Router) -> Router {
    router.layer(middleware::from_fn(policy))
}

async fn transport(State(options): State<WebOptions>, request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if !options.hsts {
        response
            .headers_mut()
            .remove(header::STRICT_TRANSPORT_SECURITY);
    }
    response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Supported locale, available as a request extension after language negotiation.
pub enum Language {
    Pl,
    En,
}
impl Language {
    /// BCP 47 language code for templates and response headers.
    pub fn code(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
    /// Translate a message from the shared Fluent catalog.
    pub fn text(self, id: &str) -> String {
        message(self, id)
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pl" => Some(Self::Pl),
            "en" => Some(Self::En),
            _ => None,
        }
    }
}

pub(crate) fn negotiate(headers: &HeaderMap) -> Language {
    for cookie in headers.get_all(header::COOKIE) {
        if let Ok(cookie) = cookie.to_str() {
            for part in cookie.split(';') {
                if let Some(("portal_language", value)) = part.trim().split_once('=')
                    && let Some(lang) = Language::parse(value)
                {
                    return lang;
                }
            }
        }
    }
    let mut candidates = Vec::new();
    if let Some(value) = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
    {
        for (index, item) in value.split(',').enumerate() {
            let mut parts = item.trim().split(';');
            let tag = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
            let quality = match parts.next() {
                None => 1.0,
                Some(q) => q
                    .trim()
                    .strip_prefix("q=")
                    .and_then(|q| q.parse::<f32>().ok())
                    .unwrap_or(0.0),
            };
            if quality > 0.0
                && quality <= 1.0
                && let Some(lang) = Language::parse(tag.split('-').next().unwrap_or_default())
            {
                candidates.push((quality, index, lang));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    candidates.first().map_or(Language::Pl, |item| item.2)
}

fn message(lang: Language, id: &str) -> String {
    oteryn_ui::text(lang.code(), id)
}

fn request_chrome(request: &Request, lang: Language) -> oteryn_ui::Chrome {
    request
        .extensions()
        .get::<oteryn_ui::Chrome>()
        .cloned()
        .unwrap_or_else(|| {
            oteryn_ui::Registry::default().chrome(
                lang.code(),
                request.uri().path(),
                oteryn_ui::Account::Anonymous,
            )
        })
}

/// Future returned by a [`HomeFeed`].
pub type HomeFuture = std::pin::Pin<Box<dyn Future<Output = oteryn_ui::HomeData> + Send>>;
/// Live home-page data, composed by the binary from module read models. The
/// web crate cannot depend on those modules, so they arrive as a request
/// extension; without one the page shows every live block as unavailable.
#[derive(Clone)]
pub struct HomeFeed(pub std::sync::Arc<dyn Fn(Language) -> HomeFuture + Send + Sync>);
impl std::fmt::Debug for HomeFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HomeFeed")
    }
}
impl HomeFeed {
    /// Attach this feed to every home route of `router`.
    pub fn layer(self, router: Router) -> Router {
        router.layer(axum::Extension(self))
    }
}
/// The request's feed future; resolved after the request is no longer borrowed.
pub(crate) fn home_data(request: &Request, lang: Language) -> Option<HomeFuture> {
    request
        .extensions()
        .get::<HomeFeed>()
        .map(|feed| (feed.0)(lang))
}
pub(crate) async fn resolve(data: Option<HomeFuture>) -> oteryn_ui::HomeData {
    match data {
        Some(future) => future.await,
        None => oteryn_ui::HomeData::default(),
    }
}
async fn localized_home(request: Request) -> Response {
    let lang = path_language(request.uri().path()).unwrap_or_else(|| negotiate(request.headers()));
    let chrome = request_chrome(&request, lang);
    let data = home_data(&request, lang);
    drop(request);
    oteryn_ui::home(&chrome, &resolve(data).await)
}
pub(crate) fn path_language(path: &str) -> Option<Language> {
    Language::parse(path.split('/').nth(1).unwrap_or_default())
}
async fn switch_language(Path(lang): Path<String>) -> Response {
    let Some(language) = Language::parse(&lang) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let mut response = Redirect::to(&format!("/{}/", language.code())).into_response();
    if let Ok(cookie) = HeaderValue::from_str(&format!(
        "portal_language={}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Lax; Secure",
        language.code()
    )) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| {
            accept.split(',').any(|item| {
                let mut parts = item.trim().split(';');
                parts.next() == Some("application/json") && !parts.any(|p| p.trim() == "q=0")
            })
        })
}

async fn policy(mut request: Request, next: Next) -> Response {
    let lang = path_language(request.uri().path()).unwrap_or_else(|| negotiate(request.headers()));
    let chrome = request_chrome(&request, lang);
    let json = wants_json(request.headers());
    let path = request.uri().path().to_owned();
    let is_head = request.method() == axum::http::Method::HEAD;
    let mut random = [0_u8; 32];
    let nonce = if getrandom::fill(&mut random).is_ok() {
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    } else {
        tracing::error!("response nonce generation failed");
        String::new()
    };
    request.extensions_mut().insert(CspNonce(nonce.clone()));
    request.extensions_mut().insert(lang);
    let mut response = next.run(request).await;
    if path == "/" && response.status() == StatusCode::OK {
        response = Redirect::to(&format!("/{}/", lang.code())).into_response();
    }
    if nonce.is_empty() {
        response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let status = response.status();
    if matches!(
        status.as_u16(),
        400 | 403 | 404 | 405 | 413 | 422 | 429 | 500 | 503
    ) && !(status == StatusCode::UNPROCESSABLE_ENTITY
        && response.extensions().get::<crate::RenderedPage>().is_some())
    {
        let mut replacement = if json {
            let error = serde_json::json!({"status": status.as_u16(), "error": message(lang, &format!("error-{}-title", status.as_u16()))});
            (status, axum::Json(error)).into_response()
        } else {
            response
                .extensions()
                .get::<oteryn_ui::RefusalMessage>()
                .map_or_else(
                    || oteryn_ui::error(&chrome, status),
                    |message| oteryn_ui::error_with_message(&chrome, status, &message.0),
                )
        };
        // Preserve protocol metadata, but discard internal content and entity headers.
        for name in [header::ALLOW, header::RETRY_AFTER] {
            if let Some(value) = response.headers().get(&name) {
                replacement.headers_mut().insert(name, value.clone());
            }
        }
        response = replacement;
    }
    let headers = response.headers_mut();
    let csp = format!(
        "default-src 'none'; script-src 'self' 'nonce-{nonce}'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
    );
    if let Ok(value) = HeaderValue::from_str(&csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    );
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::CONTENT_LANGUAGE,
        HeaderValue::from_static(lang.code()),
    );
    headers.append(
        header::VARY,
        HeaderValue::from_static("Accept, Accept-Language, Cookie"),
    );
    let cache = if path.starts_with("/static/") && status.is_success() {
        if hashed_name(&path) {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=300, must-revalidate"
        }
    } else {
        "no-store"
    };
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    if is_head {
        *response.body_mut() = Body::empty();
    }
    response
}

fn hashed_name(path: &str) -> bool {
    path.rsplit('/').next().is_some_and(|name| {
        name.split(['.', '-'])
            .any(|part| part.len() >= 8 && part.bytes().all(|b| b.is_ascii_hexdigit()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn app() -> Result<Router, sqlx::Error> {
        let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
        Ok(crate::router(AppState { db: pool }, "../../static"))
    }
    async fn html(response: Response) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from_utf8(
            to_bytes(response.into_body(), crate::MAX_BODY_BYTES * 2)
                .await?
                .to_vec(),
        )?)
    }
    fn assert_headers(response: &Response) {
        for name in [
            "content-security-policy",
            "strict-transport-security",
            "x-content-type-options",
            "referrer-policy",
            "permissions-policy",
            "cross-origin-opener-policy",
        ] {
            assert!(response.headers().contains_key(name), "missing {name}");
        }
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap_or_default();
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(csp.contains("'nonce-"));
        assert!(!csp.contains("unsafe-inline"));
    }

    #[tokio::test]
    async fn html_json_static_have_policy_and_types() -> TestResult {
        for (uri, accept, status, content_type) in [
            ("/en/", "text/html", StatusCode::OK, "text/html"),
            (
                "/missing",
                "application/json",
                StatusCode::NOT_FOUND,
                "application/json",
            ),
            ("/static/css/surface.css", "*/*", StatusCode::OK, "text/css"),
            (
                "/static/missing",
                "application/json",
                StatusCode::NOT_FOUND,
                "application/json",
            ),
        ] {
            let response = app()?
                .oneshot(
                    HttpRequest::get(uri)
                        .header(header::ACCEPT, accept)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), status);
            assert_headers(&response);
            assert!(
                response.headers()[header::CONTENT_TYPE]
                    .to_str()?
                    .starts_with(content_type)
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn localized_layout_is_semantic_and_has_no_inline_scripts() -> TestResult {
        let response = app()?
            .oneshot(HttpRequest::get("/en/").body(Body::empty())?)
            .await?;
        let body = html(response).await?;
        for expected in [
            "<html lang=\"en\" data-theme=\"dark\">",
            "<header class=\"hx-head\">",
            "<nav ",
            "<main ",
            "<footer class=\"hx-foot\">",
            "href=\"#main\"",
            "Welcome to the world of Oteryn!",
            "href=\"/pl/\"",
        ] {
            assert!(body.contains(expected), "missing {expected}");
        }
        assert!(!body.contains("<script"));
        assert!(!body.contains("style="));
        Ok(())
    }

    #[tokio::test]
    async fn errors_hide_details_and_keep_protocol_headers() -> TestResult {
        let failures = Router::new()
            .route(
                "/failure",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "private database detail") }),
            )
            .route(
                "/unavailable",
                get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "private failure") }),
            )
            .route(
                "/limited",
                get(|| async {
                    (
                        StatusCode::TOO_MANY_REQUESTS,
                        [(header::RETRY_AFTER, "30")],
                        "private limiter detail",
                    )
                }),
            );
        let failures = finish(failures);
        for (uri, status, title) in [
            ("/missing", StatusCode::NOT_FOUND, "Page not found"),
            (
                "/unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                "Temporarily unavailable",
            ),
            (
                "/failure",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong",
            ),
            (
                "/limited",
                StatusCode::TOO_MANY_REQUESTS,
                "Too many requests",
            ),
        ] {
            for accept in ["text/html", "application/json"] {
                let response = failures
                    .clone()
                    .oneshot(
                        HttpRequest::get(uri)
                            .header(header::ACCEPT, accept)
                            .header(header::ACCEPT_LANGUAGE, "en")
                            .body(Body::empty())?,
                    )
                    .await?;
                assert_eq!(response.status(), status);
                assert_headers(&response);
                if status == StatusCode::TOO_MANY_REQUESTS {
                    assert_eq!(response.headers()[header::RETRY_AFTER], "30");
                }
                let body = html(response).await?;
                assert!(body.contains(title));
                for detail in [
                    "private database detail",
                    "private failure",
                    "private limiter detail",
                ] {
                    assert!(!body.contains(detail));
                }
                if accept == "text/html" {
                    assert!(body.contains("<main "));
                } else {
                    let value: serde_json::Value = serde_json::from_str(&body)?;
                    assert_eq!(value["status"], status.as_u16());
                }
            }
        }
        let response = app()?
            .oneshot(HttpRequest::post("/en/").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(response.headers().contains_key(header::ALLOW));
        assert!(html(response).await?.contains("Method not allowed"));
        let response = app()?
            .oneshot(
                HttpRequest::post("/en/")
                    .header(header::CONTENT_LENGTH, crate::MAX_BODY_BYTES + 1)
                    .body(Body::from(vec![0; crate::MAX_BODY_BYTES + 1]))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(html(response).await?.contains("Request too large"));
        Ok(())
    }

    #[tokio::test]
    async fn language_cookie_header_fallback_and_switch() -> TestResult {
        for (cookie, accept, expected) in [
            ("portal_language=en", "pl", "/en/"),
            ("portal_language=pl", "en", "/pl/"),
            ("portal_language=de", "en-GB;q=0.9,pl;q=0.4", "/en/"),
            ("", "de,fr", "/pl/"),
            ("", "en;q=0,pl;q=0.5", "/pl/"),
            ("", "en;q=bad", "/pl/"),
        ] {
            let response = app()?
                .oneshot(
                    HttpRequest::get("/")
                        .header(header::COOKIE, cookie)
                        .header(header::ACCEPT_LANGUAGE, accept)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(response.headers()[header::LOCATION], expected);
        }
        let response = app()?
            .oneshot(HttpRequest::get("/language/en").body(Body::empty())?)
            .await?;
        assert_eq!(response.headers()[header::LOCATION], "/en/");
        assert!(
            response.headers()[header::SET_COOKIE]
                .to_str()?
                .contains("portal_language=en;")
        );
        let response = app()?
            .oneshot(HttpRequest::get("/language/de").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        assert!(html(response).await?.contains("Nieprawidłowe żądanie"));
        Ok(())
    }

    #[tokio::test]
    async fn nonces_are_unique_and_available_to_handlers() -> TestResult {
        let app = finish(Router::new().route(
            "/nonce",
            get(|axum::Extension(nonce): axum::Extension<CspNonce>| async move { nonce.0 }),
        ));
        let mut previous = String::new();
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(HttpRequest::get("/nonce").body(Body::empty())?)
                .await?;
            let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()?
                .to_owned();
            let nonce = html(response).await?;
            assert_eq!(nonce.len(), 64);
            assert!(csp.contains(&format!("'nonce-{nonce}'")));
            assert_ne!(previous, nonce);
            previous = nonce;
        }
        Ok(())
    }

    #[tokio::test]
    async fn static_caching_head_and_local_transport() -> TestResult {
        let dir = std::env::temp_dir().join(format!("portal-static-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("app.0123456789abcdef.js"), "'use strict';")?;
        let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
        let app = router_with_options(
            AppState { db: pool },
            dir.to_str().ok_or("invalid path")?,
            WebOptions { hsts: false },
        );
        let response = app
            .oneshot(HttpRequest::head("/static/app.0123456789abcdef.js").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        assert!(
            !response
                .headers()
                .contains_key(header::STRICT_TRANSPORT_SECURITY)
        );
        assert!(html(response).await?.is_empty());
        std::fs::remove_dir_all(dir)?;
        let response = self::app()?
            .oneshot(HttpRequest::get("/static/css/surface.css").body(Body::empty())?)
            .await?;
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=300, must-revalidate"
        );
        assert!(!hashed_name("/static/plain.css"));
        Ok(())
    }

    #[tokio::test]
    async fn readiness_uses_real_postgres_when_configured() -> TestResult {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED database readiness test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let pool = oteryn_db::lazy_pool(&url)?;
        let response = crate::router(AppState { db: pool.clone() }, "../../static")
            .oneshot(HttpRequest::get("/readyz").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_headers(&response);
        pool.close().await;
        Ok(())
    }
}
