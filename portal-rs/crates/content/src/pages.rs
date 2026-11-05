//! Published pages and staff editorial workflows. Unknown publication state is never public.
use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::{PgPool, Row};
mod staff;
#[cfg(test)]
mod tests;
pub use staff::{configured_router, staff_router};

pub(crate) fn reserved(slug: &str) -> bool {
    matches!(
        slug,
        "rules"
            | "terms-of-service"
            | "privacy-policy"
            | "cookie-policy"
            | "server-information"
            | "report-a-bug"
            | "getting-started"
            | "support"
    )
}
pub(crate) fn legal(slug: &str) -> bool {
    matches!(
        slug,
        "terms-of-service" | "privacy-policy" | "cookie-policy"
    )
}
pub(crate) fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 160
        && slug.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
pub(crate) fn text(lang: &str, key: &str) -> String {
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("../../../templates/pages/en.ftl")
    } else {
        include_str!("../../../templates/pages/pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "pages/editorial.html", config = "src/pages/askama.toml")]
struct Page {
    lang: String,
    title: String,
    body: String,
    version: String,
}
pub fn router(pool: PgPool) -> Router {
    let mut app = Router::new();
    for lang in ["pl", "en"] {
        app = app.route(&format!("/{lang}/pages/{{slug}}"), get(custom));
        for (route, slug) in [
            ("rules", "rules"),
            ("legal/terms", "terms-of-service"),
            ("legal/privacy", "privacy-policy"),
            ("legal/cookies", "cookie-policy"),
            ("server-information", "server-information"),
            ("support/report-a-bug", "report-a-bug"),
        ] {
            app = app.route(
                &format!("/{lang}/{route}"),
                get(move |State(pool): State<PgPool>| async move { show(&pool, lang, slug).await }),
            );
        }
    }
    app.with_state(pool)
        .layer(axum::middleware::from_fn(public_headers))
}
async fn custom(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    uri: axum::http::Uri,
) -> Response {
    if !valid_slug(&slug) || reserved(&slug) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = if uri.path().starts_with("/en/") {
        "en"
    } else {
        "pl"
    };
    show(&pool, lang, &slug).await
}
async fn show(pool: &PgPool, lang: &str, slug: &str) -> Response {
    let result = sqlx::query("SELECT e.title,e.body,COALESCE(e.legal_version,'') AS version,COALESCE(e.legal_effective_date::text,'') AS effective FROM pages p JOIN editorial_pages e ON e.page_id=p.id JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en' WHERE p.slug=$1 AND e.locale=$2 AND e.published_at<=now() AND source.published_at<=now() AND (e.locale='en' OR e.source_revision=source.revision) AND (p.slug NOT IN ('terms-of-service','privacy-policy','cookie-policy') OR (e.legal_version IS NOT NULL AND e.legal_effective_date IS NOT NULL))").bind(slug).bind(lang).fetch_optional(pool).await;
    let (status, title, body, version) = match result {
        Ok(Some(row)) => {
            let body: String = row.get("body");
            let mut html = String::new();
            pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&body));
            (
                StatusCode::OK,
                row.get("title"),
                ammonia::clean(&html),
                format!(
                    "{} {}",
                    row.get::<String, _>("version"),
                    row.get::<String, _>("effective")
                ),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            text(lang, "unavailable"),
            String::new(),
            String::new(),
        ),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let mut response = oteryn_web::render(&Page {
        lang: lang.to_owned(),
        title,
        body,
        version,
    });
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

async fn public_headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let lang = if request.uri().path().starts_with("/en/") {
        "en"
    } else {
        "pl"
    };
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        ("content-language", lang),
        ("cache-control", "no-store"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
