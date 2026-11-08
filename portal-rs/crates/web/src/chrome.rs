//! Read the existing browser session for presentation without adding a CSRF gate.
use crate::session::SessionLayer;
use axum::{
    body::Body,
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use oteryn_ui::Registry;

/// Startup state: enabled navigation and the existing browser-session configuration.
#[derive(Clone, Debug)]
pub struct Shell {
    pub registry: Registry,
    pub sessions: SessionLayer,
}
/// Install as the outermost application middleware so every module can extract Chrome.
/// Session validation and mutation authorization remain in their existing route layers.
pub async fn middleware(State(shell): State<Shell>, mut request: Request, next: Next) -> Response {
    let lang = crate::surface::path_language(request.uri().path())
        .unwrap_or_else(|| crate::surface::negotiate(request.headers()))
        .code();
    let account = crate::session::chrome_account(&shell.sessions, &mut request).await;
    let path = request.uri().path();
    let local = path
        .strip_prefix("/pl/")
        .or_else(|| path.strip_prefix("/en/"))
        .unwrap_or(path.trim_start_matches('/'));
    if request.method() == axum::http::Method::GET
        && matches!(account, oteryn_ui::Account::Anonymous)
        && (local == "account"
            || local.starts_with("account/")
            || local.starts_with("panel/")
            || local.starts_with("support/tickets")
            || local.starts_with("support/reports")
            || matches!(local, "pass" | "atlas/places"))
    {
        use axum::response::IntoResponse;
        return axum::response::Redirect::to(&format!("/{lang}/login")).into_response();
    }
    let chrome = shell.registry.chrome(lang, request.uri().path(), account);
    request.extensions_mut().insert(chrome.clone());
    let browser = request.method() == axum::http::Method::GET
        && !request.uri().path().starts_with("/api/")
        && !request.uri().path().starts_with("/internal/")
        && request
            .headers()
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("text/html"));
    let mut response = next.run(request).await;
    if browser && (response.status().is_client_error() || response.status().is_server_error()) {
        let html = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/html"));
        if !html {
            let mut replacement = oteryn_ui::error(&chrome, response.status());
            for name in [
                header::ALLOW,
                header::RETRY_AFTER,
                header::CONTENT_SECURITY_POLICY,
                header::STRICT_TRANSPORT_SECURITY,
                header::X_CONTENT_TYPE_OPTIONS,
                header::REFERRER_POLICY,
                header::CONTENT_LANGUAGE,
                header::VARY,
                axum::http::HeaderName::from_static("permissions-policy"),
            ] {
                if let Some(value) = response.headers().get(&name) {
                    replacement.headers_mut().insert(name, value.clone());
                }
            }
            for value in response.headers().get_all(header::SET_COOKIE) {
                replacement
                    .headers_mut()
                    .append(header::SET_COOKIE, value.clone());
            }
            for (name, value) in [
                (
                    "content-security-policy",
                    "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; img-src 'self' data:; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
                ),
                (
                    "strict-transport-security",
                    "max-age=63072000; includeSubDomains",
                ),
                ("x-content-type-options", "nosniff"),
                ("referrer-policy", "strict-origin-when-cross-origin"),
                (
                    "permissions-policy",
                    "camera=(), microphone=(), geolocation=()",
                ),
            ] {
                replacement
                    .headers_mut()
                    .entry(axum::http::HeaderName::from_static(name))
                    .or_insert(axum::http::HeaderValue::from_static(value));
            }
            replacement.headers_mut().insert(
                header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            response = replacement;
        }
    }
    let mut response = wrap_document(response, &chrome).await;
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"))
        && let Some(policy) = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok())
        && let Ok(value) = shell_asset_policy(policy).parse()
    {
        response
            .headers_mut()
            .insert(header::CONTENT_SECURITY_POLICY, value);
    }
    response
}

// The shell requires only local fonts and images. Preserve every existing explicit
// directive, including stricter image policies, and never permit scripts or inline styles.
fn shell_asset_policy(policy: &str) -> String {
    let mut result = policy.to_owned();
    for name in ["font-src", "img-src"] {
        if !policy
            .split(';')
            .any(|value| value.trim_start().starts_with(&format!("{name} ")))
        {
            result.push_str(&format!("; {name} 'self'"));
        }
    }
    result
}

/// Largest module document the shell adapter buffers; larger bodies pass through unchanged.
const WRAP_LIMIT: usize = 4 * 1024 * 1024;

/// Give a module page that renders its own HTML document the shared header, footer and theme.
/// Pages already built on the shared layout, non-HTML, encoded and oversized bodies pass through.
async fn wrap_document(response: Response, chrome: &oteryn_ui::Chrome) -> Response {
    let html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if !html || response.headers().contains_key(header::CONTENT_ENCODING) {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, WRAP_LIMIT).await {
        Ok(bytes) => bytes,
        Err(_) => {
            parts.status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
            parts.headers.remove(header::CONTENT_LENGTH);
            return Response::from_parts(parts, Body::empty());
        }
    };
    let wrapped = std::str::from_utf8(&bytes).ok().and_then(|page| {
        let (top, bottom) = oteryn_ui::shell_parts(chrome)?;
        inject(page, &top, &bottom)
    });
    match wrapped {
        Some(page) => {
            parts.headers.remove(header::CONTENT_LENGTH);
            Response::from_parts(parts, Body::from(page))
        }
        None => Response::from_parts(parts, Body::from(bytes)),
    }
}

/// Insert the shell into a complete HTML document. Returns None when the page already has
/// the shell or is not a full document (fragments, widgets).
pub(crate) fn inject(page: &str, header: &str, footer: &str) -> Option<String> {
    if page.contains(oteryn_ui::SHELL_MARKER) {
        return None;
    }
    let lower = page.to_ascii_lowercase();
    let html_open = lower.find("<html")?;
    if !lower.contains("<body") {
        let start = BODY_STARTS.iter().filter_map(|tag| lower.find(tag)).min()?;
        let end = lower.rfind("</html>").unwrap_or(page.len());
        if end < start {
            return None;
        }
        let explicit = format!(
            "{}<body>{}</body>{}",
            &page[..start],
            &page[start..end],
            &page[end..]
        );
        return inject(&explicit, header, footer);
    }
    let body_open = lower.find("<body")?;
    let body_end = body_open + lower[body_open..].find('>')?;
    let body_close = lower.rfind("</body>").unwrap_or(page.len());
    if body_close < body_end {
        return None;
    }
    let html_end = html_open + lower[html_open..].find('>')?;
    let html_tag = without_attribute(&page[html_open..html_end], "data-theme");
    let body_tag = &page[body_open..body_end];
    let body_tag = match body_tag.to_ascii_lowercase().find("class=\"") {
        Some(at) => format!(
            "{}class=\"{} {}",
            &body_tag[..at],
            oteryn_ui::SHELL_MARKER,
            &body_tag[at + 7..]
        ),
        None => format!("{body_tag} class=\"{}\"", oteryn_ui::SHELL_MARKER),
    };
    let head = &page[html_end + 1..body_open];
    let mut styles = String::new();
    if !lower.contains("/static/css/components.css") {
        styles.push_str("<link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\">");
    }
    if !lower.contains("/static/css/home.css") {
        styles.push_str("<link rel=\"stylesheet\" href=\"/static/css/home.css\">");
    }
    let head = match head.to_ascii_lowercase().find("</head>") {
        Some(at) => format!("{}{styles}{}", &head[..at], &head[at..]),
        None => format!("{styles}{head}"),
    };
    let content = content_landmark(&without_module_skip(&page[body_end + 1..body_close]));
    Some(format!(
        "{}{html_tag} data-theme=\"dark\">{head}{body_tag}>{header}<div class=\"op-shell__page\">{}</div>{footer}{}",
        &page[..html_open],
        content,
        &page[body_close..]
    ))
}

// Older standalone documents have a main landmark but no skip-link target.
fn content_landmark(content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    let Some(start) = lower.find("<main") else {
        return content.to_owned();
    };
    let Some(end) = lower[start..].find('>') else {
        return content.to_owned();
    };
    let end = start + end;
    let id = if lower[start..end].contains(" id=") {
        ""
    } else {
        " id=\"main\""
    };
    let focus = if lower[start..end].contains("tabindex=") {
        ""
    } else {
        " tabindex=\"-1\""
    };
    format!("{}{id}{focus}{}", &content[..end], &content[end..])
}

// Standalone modules keep their skip link when rendered alone. The composed shell
// supplies that same landmark, so retain exactly one when adapting a document.
fn without_module_skip(content: &str) -> String {
    let marker = "<a class=\"op-skip\" href=\"#main\">";
    let Some(start) = content.find(marker) else {
        return content.to_owned();
    };
    let Some(end) = content[start..].find("</a>") else {
        return content.to_owned();
    };
    format!("{}{}", &content[..start], &content[start + end + 4..])
}

/// Elements that open an implied HTML5 body when a document omits the body tag.
const BODY_STARTS: [&str; 9] = [
    "<header", "<main", "<nav", "<section", "<article", "<div", "<h1", "<p", "<form",
];

fn without_attribute(tag: &str, name: &str) -> String {
    let lower = tag.to_ascii_lowercase();
    let Some(at) = lower.find(&format!(" {name}=\"")) else {
        return tag.to_owned();
    };
    let value = at + name.len() + 3;
    match tag[value..].find('"') {
        Some(end) => format!("{}{}", &tag[..at], &tag[value + end + 1..]),
        None => tag.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{content_landmark, inject, shell_asset_policy};

    #[test]
    fn skip_target_is_focusable_without_duplicate_landmark_ids() {
        assert_eq!(
            content_landmark("<main class=\"panel\"><h1>Account</h1></main>"),
            "<main class=\"panel\" id=\"main\" tabindex=\"-1\"><h1>Account</h1></main>"
        );
        let existing = "<main id=\"main\" tabindex=\"-1\">Content</main>";
        assert_eq!(content_landmark(existing), existing);
    }

    #[test]
    fn adapted_policy_permits_local_shell_assets_and_preserves_restrictions() {
        let policy = shell_asset_policy(
            "default-src 'none'; style-src 'self'; form-action 'self'; frame-ancestors 'none'",
        );
        assert_eq!(
            policy,
            "default-src 'none'; style-src 'self'; form-action 'self'; frame-ancestors 'none'; font-src 'self'; img-src 'self'"
        );
        let strict = "default-src 'none'; img-src 'none'; font-src 'self'";
        assert_eq!(shell_asset_policy(strict), strict);
        assert!(!policy.contains("unsafe-inline"));
        assert!(!policy.contains("script-src"));
    }

    #[test]
    fn wraps_standalone_documents_once() -> Result<(), &'static str> {
        let page = "<!doctype html><html lang=\"pl\" data-theme=\"light\"><head><title>x</title></head><body class=\"page\"><main>tresc</main></body></html>";
        let out = inject(page, "<header>H</header>", "<footer>F</footer>")
            .ok_or("standalone page must be wrapped")?;
        assert!(out.contains("<html lang=\"pl\" data-theme=\"dark\">"));
        assert!(out.contains(
            "components.css\"><link rel=\"stylesheet\" href=\"/static/css/home.css\"></head>"
        ));
        assert!(out.contains("<body class=\"op-theme op-shell page\"><header>H</header><div class=\"op-shell__page\"><main id=\"main\" tabindex=\"-1\">tresc</main></div><footer>F</footer></body></html>"));
        assert!(inject(&out, "<header>H</header>", "<footer>F</footer>").is_none());
        Ok(())
    }

    #[test]
    fn composed_document_has_only_the_shared_skip_link() -> Result<(), &'static str> {
        let skip = "<a class=\"op-skip\" href=\"#main\">Skip</a>";
        let page =
            format!("<html><body>{skip}<main id=\"main\"><h1>Account</h1></main></body></html>");
        let out = inject(&page, skip, "F").ok_or("document must be wrapped")?;
        assert_eq!(out.matches("class=\"op-skip\"").count(), 1);
        assert!(out.contains("<main id=\"main\" tabindex=\"-1\"><h1>Account</h1></main>"));
        Ok(())
    }

    #[test]
    fn leaves_fragments_alone() -> Result<(), &'static str> {
        assert!(inject("<section>widget</section>", "h", "f").is_none());
        let page = "<html><body><p>a</p></body></html>";
        let out = inject(page, "H", "F").ok_or("bare document must be wrapped")?;
        assert!(out.contains("<body class=\"op-theme op-shell\">H<div class=\"op-shell__page\"><p>a</p></div>F</body>"));
        let implied = "<!doctype html><html lang=\"pl\"><meta charset=\"utf-8\"><title>t</title><main>m</main></html>";
        let out =
            inject(implied, "H", "F").ok_or("document with an implied body must be wrapped")?;
        assert!(out.find("components.css") < out.find("<body"));
        assert!(out.contains("<html lang=\"pl\" data-theme=\"dark\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\">"));
        assert!(out.contains("<body class=\"op-theme op-shell\">H<div class=\"op-shell__page\"><main id=\"main\" tabindex=\"-1\">m</main></div>F</body></html>"));
        Ok(())
    }
}

#[cfg(test)]
mod response_tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn public_refusals_are_documents_and_api_statuses_stay_intact()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool =
            sqlx::postgres::PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
        let app = Router::new()
            .route(
                "/{lang}/cold",
                get(|| async {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        [
                            (header::RETRY_AFTER, "17"),
                            (header::STRICT_TRANSPORT_SECURITY, "max-age=7"),
                            (header::REFERRER_POLICY, "no-referrer"),
                            (header::SET_COOKIE, "probe=value; Secure; HttpOnly"),
                        ],
                        "",
                    )
                }),
            )
            .route("/api/refused", get(|| async { StatusCode::FORBIDDEN }))
            .route(
                "/{lang}/account/privacy",
                get(|| async { StatusCode::FORBIDDEN }).post(|| async { StatusCode::FORBIDDEN }),
            )
            .layer(axum::middleware::from_fn_with_state(
                Shell {
                    registry: Registry::default(),
                    sessions: SessionLayer::new(pool),
                },
                middleware,
            ));
        for lang in ["pl", "en"] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("/{lang}/cold"))
                        .header(header::ACCEPT, "text/html")
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(
                response.headers()[header::CONTENT_TYPE]
                    .to_str()?
                    .starts_with("text/html")
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert_eq!(
                response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                "nosniff"
            );
            assert_eq!(
                response.headers()[header::STRICT_TRANSPORT_SECURITY],
                "max-age=7"
            );
            assert_eq!(response.headers()[header::RETRY_AFTER], "17");
            assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
            assert_eq!(
                response.headers()[header::SET_COOKIE],
                "probe=value; Secure; HttpOnly"
            );
            assert!(
                response.headers()[header::CONTENT_SECURITY_POLICY]
                    .to_str()?
                    .contains("frame-ancestors 'none'")
            );
            let html = String::from_utf8(
                axum::body::to_bytes(response.into_body(), 100000)
                    .await?
                    .to_vec(),
            )?;
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(html.contains("class=\"hx-head\""));
            assert!(html.contains("class=\"hx-foot\""));
            assert!(html.contains("/static/css/home.css"));
            let response = app
                .clone()
                .oneshot(Request::get(format!("/{lang}/account/privacy")).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                response.headers()[header::LOCATION],
                format!("/{lang}/login")
            );
            let response = app
                .clone()
                .oneshot(Request::post(format!("/{lang}/account/privacy")).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        let response = app
            .oneshot(
                Request::get("/api/refused")
                    .header(header::ACCEPT, "application/json")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(!response.headers().contains_key(header::CONTENT_TYPE));
        Ok(())
    }
}
