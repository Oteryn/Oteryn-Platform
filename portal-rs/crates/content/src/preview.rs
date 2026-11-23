//! Read-only staff previews. Each route is inside its module's staff session
//! and permission guard, so access ends with the session or permission.
use askama::Template;
use axum::{
    http::{HeaderValue, StatusCode, header},
    response::Response,
};

pub(crate) fn locale(lang: &str) -> Result<&str, StatusCode> {
    match lang {
        "en" | "pl" => Ok(lang),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

pub(crate) fn complete(title: &str, body: &str) -> bool {
    !title.trim().is_empty() && !body.trim().is_empty()
}

pub(crate) fn notice(lang: &str) -> String {
    crate::pages::text(lang, "preview-notice")
}

pub(crate) fn response(mut response: Response, lang: &str) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_LANGUAGE,
        HeaderValue::from_static(if lang == "en" { "en" } else { "pl" }),
    );
    headers.insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    response
}

pub(crate) fn link(base: &str, lang: &str, revision: i64) -> (String, String, String) {
    (
        format!("{base}/preview/{lang}/{revision}"),
        lang.to_owned(),
        crate::pages::text(lang, "preview-link"),
    )
}

#[derive(Template)]
#[template(path = "content/preview_links.html", config = "src/pages/askama.toml")]
pub(crate) struct Links {
    pub(crate) links: Vec<(String, String, String)>,
}
