//! Shared response policy and module composition for public HTTP routes.

use axum::{
    Router,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};

use crate::AppState;

pub(crate) const HSTS: &str = "max-age=63072000; includeSubDomains";
pub(crate) const PERMISSIONS: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=()";
pub(crate) const REFERRER: &str = "strict-origin-when-cross-origin";

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
    /// Resolve the portal locale using the URL, preference cookie, then Accept-Language.
    pub fn for_request(request: &Request) -> Self {
        document_language(request.uri(), request.headers())
    }

    /// BCP 47 language code for templates and response headers.
    pub fn code(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
    /// Resolve the browser language preference from its cookie or Accept-Language.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        negotiate(headers)
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

/// Resolve the preferred browser language from its locale cookie and headers.
pub fn negotiate(headers: &HeaderMap) -> Language {
    if oteryn_ui::preferred_language(headers) == "en" {
        Language::En
    } else {
        Language::Pl
    }
}

/// Resolve the public document language before constructing portal chrome.
pub fn document_language(uri: &axum::http::Uri, headers: &HeaderMap) -> Language {
    if let Some(lang) = path_language(uri.path()) {
        return lang;
    }
    if (matches!(uri.path(), "/news" | "/today") || uri.path().starts_with("/news/"))
        && let Some(query) = uri.query()
        && let Ok(options) = serde_urlencoded::from_str::<Vec<(String, String)>>(query)
        && let Some((_, lang)) = options.iter().find(|(key, _)| key == "lang")
    {
        return Language::parse(lang).unwrap_or(Language::Pl);
    }
    negotiate(headers)
}

fn message(lang: Language, id: &str) -> String {
    oteryn_ui::text(lang.code(), id)
}

/// Console pages use an explicit language selector instead of localized paths.
pub(crate) fn request_language(request: &Request) -> Language {
    #[derive(serde::Deserialize)]
    struct LocaleQuery {
        lang: Option<String>,
    }

    let path = request.uri().path();
    let console = path == "/console" || path.starts_with("/console/");
    path_language(path)
        .or_else(|| {
            let query = request.uri().query().filter(|_| console)?;
            let selection: LocaleQuery = serde_urlencoded::from_str(query).ok()?;
            Language::parse(selection.lang.as_deref()?)
        })
        .unwrap_or_else(|| document_language(request.uri(), request.headers()))
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
    let lang = request_language(&request);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ErrorRepresentation {
    DefaultHtml,
    Html,
    Json,
    Unacceptable,
}

fn split_media_header(value: &str, delimiter: char) -> impl Iterator<Item = &str> {
    let mut quoted = false;
    let mut escaped = false;
    value.split(move |character| {
        if escaped {
            escaped = false;
        } else if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if !quoted && character == delimiter {
            return true;
        }
        false
    })
}

fn accepts_utf8(value: &str) -> bool {
    let Some(quoted) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return value.eq_ignore_ascii_case("utf-8");
    };
    let mut bytes = quoted.bytes();
    b"utf-8".iter().all(|expected| {
        let actual = match bytes.next() {
            Some(b'\\') => bytes.next(),
            other => other,
        };
        actual.is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
    }) && bytes.next().is_none()
}

pub(crate) fn error_representation(headers: &HeaderMap) -> ErrorRepresentation {
    use ErrorRepresentation::*;
    if !headers.contains_key(header::ACCEPT) {
        return DefaultHtml;
    }

    let mut html = (0, false, 0.0_f32);
    let mut json = (0, false, 0.0_f32);
    'media_range: for item in headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| split_media_header(value, ','))
    {
        let mut parts = split_media_header(item, ';').map(str::trim);
        let range = parts.next().unwrap_or_default();
        let mut quality = 1.0;
        let mut requires_utf8 = false;
        for parameter in parts.filter(|part| !part.is_empty()) {
            let Some((name, value)) = parameter.split_once('=') else {
                continue 'media_range;
            };
            let (name, value) = (name.trim(), value.trim());
            if name.eq_ignore_ascii_case("q") {
                quality = value
                    .parse::<f32>()
                    .ok()
                    .filter(|q| (0.0..=1.0).contains(q))
                    .unwrap_or(0.0);
            } else if name.eq_ignore_ascii_case("charset") && accepts_utf8(value) {
                requires_utf8 = true;
            } else {
                continue 'media_range;
            }
        }

        // HTML emits charset=utf-8. JSON has no media-type parameters.
        // Apply the most specific matching range, including a q=0 exclusion.
        for (exact, range_type, score) in [
            ("text/html", "text/*", &mut html),
            ("application/json", "application/*", &mut json),
        ] {
            if requires_utf8 && exact != "text/html" {
                continue;
            }
            let specificity = if range.eq_ignore_ascii_case(exact) {
                3
            } else if range.eq_ignore_ascii_case(range_type) {
                2
            } else if range == "*/*" {
                1
            } else {
                continue;
            };
            if (specificity, requires_utf8, quality) > *score {
                *score = (specificity, requires_utf8, quality);
            }
        }
    }

    match (html, json) {
        ((_, _, 0.0), (_, _, 0.0)) => Unacceptable,
        ((1, false, h), (1, false, j)) if h == j => DefaultHtml,
        ((hs, hp, h), (js, jp, j)) if (h, hs, hp) > (j, js, jp) => Html,
        _ => Json,
    }
}

pub(crate) fn oauth_protocol_path(path: &str) -> bool {
    matches!(path, "/oauth/authorize" | "/oauth/token")
}

/// Whether the browser shell owns the response for this path.
pub(crate) fn browser_path(path: &str) -> bool {
    !matches!(path, "/api" | "/internal" | "/oauth/token")
        && !path.starts_with("/api/")
        && !path.starts_with("/internal/")
}

async fn policy(mut request: Request, next: Next) -> Response {
    let lang = request_language(&request);
    let chrome = request_chrome(&request, lang);
    let representation = error_representation(request.headers());
    let path = request.uri().path().to_owned();
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
    if !oauth_protocol_path(&path)
        && path != oteryn_gameauth::locator::PATH
        && (matches!(
            status.as_u16(),
            400 | 403 | 404 | 405 | 413 | 422 | 429 | 500 | 503
        ) || (status == StatusCode::UNAUTHORIZED
            && representation == ErrorRepresentation::Html
            && browser_path(&path)))
        && !(status == StatusCode::UNPROCESSABLE_ENTITY
            && response.extensions().get::<crate::RenderedPage>().is_some())
        && !(status == StatusCode::SERVICE_UNAVAILABLE
            && representation != ErrorRepresentation::Json
            && response
                .extensions()
                .get::<crate::search::SourceUnavailable>()
                .is_some())
    {
        let mut replacement = match representation {
            ErrorRepresentation::Json => {
                let error = serde_json::json!({"status": status.as_u16(), "error": message(lang, &format!("error-{}-title", status.as_u16()))});
                (status, axum::Json(error)).into_response()
            }
            ErrorRepresentation::Html | ErrorRepresentation::DefaultHtml => response
                .extensions()
                .get::<oteryn_ui::RefusalMessage>()
                .map_or_else(
                    || oteryn_ui::error(&chrome, status),
                    |message| oteryn_ui::error_with_message(&chrome, status, &message.0),
                ),
            ErrorRepresentation::Unacceptable => status.into_response(),
        };
        // Preserve protocol metadata, but discard internal content and entity headers.
        for name in [
            header::ALLOW,
            header::RETRY_AFTER,
            header::WWW_AUTHENTICATE,
            header::CONTENT_SECURITY_POLICY,
            header::REFERRER_POLICY,
        ] {
            if let Some(value) = response.headers().get(&name) {
                replacement.headers_mut().insert(name, value.clone());
            }
        }
        for name in [header::SET_COOKIE, header::CACHE_CONTROL] {
            for value in response.headers().get_all(&name) {
                replacement.headers_mut().append(&name, value.clone());
            }
        }
        response = replacement;
    }
    let headers = response.headers_mut();
    let csp = format!(
        "default-src 'none'; script-src 'self' 'nonce-{nonce}'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
    );
    if let Ok(value) = HeaderValue::from_str(&csp) {
        headers
            .entry(header::CONTENT_SECURITY_POLICY)
            .or_insert(value);
    }
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
    if path.starts_with("/static/") && status.is_success() {
        let cache = if hashed_name(&path) {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=300, must-revalidate"
        };
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    } else {
        crate::chrome::ensure_no_store(headers);
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
    use axum::body::{Body, to_bytes};
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
    #[test]
    fn error_accept_respects_quality_and_specific_exclusions() {
        use ErrorRepresentation::*;
        for (accept, expected) in [
            ("application/json, text/html;q=0", Json),
            ("text/html;q=0.0", Unacceptable),
            ("TEXT/HTML ; q=0.8, application/json;q=0", Html),
            ("text/html, application/json", Json),
            ("text/html;q=1, application/json;q=0.1", Html),
            ("text/html;q=0.1, application/json;q=1", Json),
            ("text/html;q=0, */*;q=0.8", Json),
            ("application/json;q=0, */*;q=0.8", Html),
            ("text/*, application/json;q=0.1", Html),
            (
                "text/html; charset=iso-8859-1, application/json;q=0.5",
                Json,
            ),
            ("text/html; CHARSET=\"UTF-8\", application/json;q=0.5", Html),
            (
                "text/html; charset=utf-8;q=0, text/html;q=1, application/json;q=0.5",
                Json,
            ),
            (
                "text/html; charset=iso-8859-1;q=0, text/*;q=0.8, application/json;q=0.5",
                Html,
            ),
            ("application/json; charset=utf-8, */*;q=0.5", DefaultHtml),
            (
                "text/html; note=\"value; text/html, text/html\", application/json;q=0.5",
                Json,
            ),
            ("application/xml", Unacceptable),
            ("*/*", DefaultHtml),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT, HeaderValue::from_static(accept));
            assert_eq!(error_representation(&headers), expected);
        }
        assert_eq!(error_representation(&HeaderMap::new()), DefaultHtml);
    }

    #[tokio::test]
    async fn r7_device_keeps_contract_error() -> TestResult {
        let response = app()?
            .oneshot(HttpRequest::post("/api/v1/game-auth/device-sessions").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            html(response).await?,
            r#"{"error":"device_authorization_unavailable"}"#
        );
        Ok(())
    }

    #[tokio::test]
    async fn r7_locator_keeps_empty_body() -> TestResult {
        let response = app()?
            .oneshot(
                HttpRequest::post("/internal/v1/game-auth/native-recovery-locators")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            html(response).await?.is_empty(),
            "locator 503 must be empty"
        );
        Ok(())
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
    async fn explicit_response_policy_survives_surface_rendering() -> TestResult {
        const EXPLICIT_CSP: &str = "default-src 'none'; frame-ancestors 'none'";
        let app = finish(Router::new().route(
            "/en/{status}",
            get(|Path(status): Path<u16>| async move {
                (
                    StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST),
                    [
                        (header::CONTENT_SECURITY_POLICY, EXPLICIT_CSP),
                        (header::REFERRER_POLICY, "no-referrer"),
                    ],
                )
            }),
        ));
        for status in [StatusCode::OK, StatusCode::NOT_FOUND] {
            let response = app
                .clone()
                .oneshot(HttpRequest::get(format!("/en/{}", status.as_u16())).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), status);
            assert_eq!(
                response.headers()[header::CONTENT_SECURITY_POLICY],
                EXPLICIT_CSP
            );
            assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        }
        Ok(())
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
    async fn rendered_form_refusal_keeps_status_and_retry_for_html_only() -> TestResult {
        #[derive(askama::Template)]
        #[template(source = "<a href='/en/mfa'>Retry verification</a>", ext = "html")]
        struct Retry;

        let app = finish(
            Router::new()
                .route(
                    "/en/bad",
                    axum::routing::post(|| async {
                        crate::render_form_refusal(StatusCode::BAD_REQUEST, &Retry)
                    }),
                )
                .route(
                    "/en/forbidden",
                    axum::routing::post(|| async {
                        crate::render_form_refusal(StatusCode::FORBIDDEN, &Retry)
                    }),
                ),
        );
        for (uri, expected) in [
            ("/en/bad", StatusCode::BAD_REQUEST),
            ("/en/forbidden", StatusCode::FORBIDDEN),
        ] {
            for (accept, retry) in [("text/html", true), ("application/json", false)] {
                let response = app
                    .clone()
                    .oneshot(
                        HttpRequest::post(uri)
                            .header(header::ACCEPT, accept)
                            .body(Body::empty())?,
                    )
                    .await?;
                assert_eq!(response.status(), expected);
                assert_headers(&response);
                assert!(
                    response.headers()[header::CONTENT_TYPE]
                        .to_str()?
                        .starts_with(accept)
                );
                assert_eq!(html(response).await?.contains("Retry verification"), retry);
            }
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
            "<header ",
            "<nav ",
            "<main ",
            "<footer ",
            "href=\"#main\"",
            "<h1 id=\"hero-title\">News</h1>",
            "href=\"/pl/\"",
        ] {
            assert!(body.contains(expected), "missing {expected}");
        }
        assert_eq!(body.matches("<script").count(), 1);
        assert!(body.contains("src=\"/static/thais/home.js\""));
        assert!(!body.contains("style="));
        Ok(())
    }

    #[tokio::test]
    async fn console_fallback_uses_explicit_locale_in_chrome_and_response_headers() -> TestResult {
        let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
        let app = app()?.layer(axum::middleware::from_fn_with_state(
            crate::chrome::Shell {
                registry: oteryn_ui::Registry::default(),
                sessions: crate::session::SessionLayer::new(pool),
            },
            crate::chrome::middleware,
        ));
        for (path, lang, title) in [
            ("/console/missing?lang=en", "en", "Page not found"),
            ("/console/missing?lang=pl", "pl", "Nie znaleziono strony"),
            (
                "/console/missing?lang=invalid",
                "pl",
                "Nie znaleziono strony",
            ),
            ("/other?lang=en", "pl", "Nie znaleziono strony"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    HttpRequest::get(path)
                        .header(header::ACCEPT, "text/html")
                        .header(header::COOKIE, "portal_language=pl")
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
            let body = html(response).await?;
            assert!(body.contains(&format!("<html lang=\"{lang}\"")), "{path}");
            assert!(body.contains(title), "{path}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn native_runtime_refusal_remains_empty_for_html_accept() -> TestResult {
        let pool =
            sqlx::postgres::PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
        let app = finish(oteryn_gameauth::runtime::configured_routes(pool, |key| {
            match key {
                "GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED" => Some("true".into()),
                "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(
                    r#"{"runtime.example.test":["01900000-0000-7000-8000-000000000001/01900000-0000-7000-8000-000000000002"]}"#.into(),
                ),
                _ => None,
            }
        }));
        let response = app
            .oneshot(
                HttpRequest::post("/internal/v1/game-auth/native-runtime-status")
                    .header(header::ACCEPT, "text/html")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(html(response).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn unauthorized_browser_page_is_localized_and_keeps_the_challenge() -> TestResult {
        let app = finish(Router::new().route(
            "/account",
            get(|| async {
                (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, "Bearer")],
                    "private detail",
                )
            }),
        ));
        for (lang, title) in [("pl", "Wymagane logowanie"), ("en", "Sign-in required")] {
            let response = app
                .clone()
                .oneshot(
                    HttpRequest::get("/account")
                        .header(header::ACCEPT, "text/html")
                        .header(header::ACCEPT_LANGUAGE, lang)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
            let body = html(response).await?;
            assert!(body.contains("<main "));
            assert!(body.contains(title));
            assert!(!body.contains("private detail"));
        }
        let response = app
            .oneshot(
                HttpRequest::get("/account")
                    .header(header::ACCEPT, "application/json")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
        assert_eq!(html(response).await?, "private detail");
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
                get(|| async {
                    let mut headers = HeaderMap::new();
                    headers.append(
                        header::CACHE_CONTROL,
                        HeaderValue::from_static("private, no-store"),
                    );
                    headers.append(header::CACHE_CONTROL, HeaderValue::from_static("max-age=0"));
                    (StatusCode::SERVICE_UNAVAILABLE, headers, "private failure")
                }),
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
                if status == StatusCode::SERVICE_UNAVAILABLE {
                    assert_eq!(
                        response
                            .headers()
                            .get_all(header::CACHE_CONTROL)
                            .iter()
                            .collect::<Vec<_>>(),
                        ["private, no-store", "max-age=0"],
                    );
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
