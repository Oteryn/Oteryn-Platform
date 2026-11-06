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
    let chrome = shell.registry.chrome(lang, request.uri().path(), account);
    request.extensions_mut().insert(chrome.clone());
    let response = next.run(request).await;
    wrap_document(response, &chrome).await
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
    let styles = if lower.contains("/static/css/components.css") {
        String::new()
    } else {
        "<link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\">".to_owned()
    };
    let head = match head.to_ascii_lowercase().find("</head>") {
        Some(at) => format!("{}{styles}{}", &head[..at], &head[at..]),
        None => format!("{styles}{head}"),
    };
    Some(format!(
        "{}{html_tag} data-theme=\"dark\">{head}{body_tag}>{header}<div class=\"op-shell__page\">{}</div>{footer}{}",
        &page[..html_open],
        &page[body_end + 1..body_close],
        &page[body_close..]
    ))
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
    use super::inject;

    #[test]
    fn wraps_standalone_documents_once() -> Result<(), &'static str> {
        let page = "<!doctype html><html lang=\"pl\" data-theme=\"light\"><head><title>x</title></head><body class=\"page\"><main>tresc</main></body></html>";
        let out = inject(page, "<header>H</header>", "<footer>F</footer>")
            .ok_or("standalone page must be wrapped")?;
        assert!(out.contains("<html lang=\"pl\" data-theme=\"dark\">"));
        assert!(out.contains("components.css\"></head>"));
        assert!(out.contains("<body class=\"op-theme op-shell page\"><header>H</header><div class=\"op-shell__page\"><main>tresc</main></div><footer>F</footer></body></html>"));
        assert!(inject(&out, "<header>H</header>", "<footer>F</footer>").is_none());
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
        assert!(out.contains("<body class=\"op-theme op-shell\">H<div class=\"op-shell__page\"><main>m</main></div>F</body></html>"));
        Ok(())
    }
}
