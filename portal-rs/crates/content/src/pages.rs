//! Published pages and staff editorial workflows. Unknown publication state is never public.
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};
pub mod legal;
mod staff;
#[cfg(test)]
mod tests;
pub use staff::{LegalEdition, configured_router, publish_legal_edition, staff_router};

const MAX_SLUG_LENGTH: usize = 160;
const FAQ_PREFIX: &str = "faq-";

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
        && slug.len() <= MAX_SLUG_LENGTH
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
    draft_notice: String,
    help_links: bool,
    tickets_label: String,
    password_label: String,
    faq_label: String,
}
pub fn router(pool: PgPool) -> Router {
    let mut app = Router::new();
    for lang in ["pl", "en"] {
        app = app.route(&format!("/{lang}/pages/{{slug}}"), get(custom));
        app = app
            .route(
                &format!("/{lang}/faq"),
                get(move |State(pool): State<PgPool>, Query(query)| async move {
                    faq_index(&pool, lang, query).await
                }),
            )
            .route(
                &format!("/{lang}/faq/{{slug}}"),
                get(
                    move |State(pool): State<PgPool>, Path(slug): Path<String>| async move {
                        if !valid_slug(&slug) || slug.len() > MAX_SLUG_LENGTH - FAQ_PREFIX.len() {
                            return StatusCode::NOT_FOUND.into_response();
                        }
                        show(&pool, lang, &format!("{FAQ_PREFIX}{slug}")).await
                    },
                ),
            );
        for (route, slug) in [
            ("rules", "rules"),
            ("start", "getting-started"),
            ("help", "support"),
            ("support", "support"),
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

#[derive(Deserialize)]
struct FaqQuery {
    #[serde(default)]
    page: u32,
}

#[derive(Template)]
#[template(path = "pages/faq.html", config = "src/pages/askama.toml")]
struct FaqIndex<'a> {
    lang: &'a str,
    title: String,
    empty: String,
    previous_label: String,
    next_label: String,
    previous: Option<u32>,
    next: Option<u32>,
    entries: Vec<(String, String)>,
}

async fn faq_index(pool: &PgPool, lang: &str, query: FaqQuery) -> Response {
    const PAGE_SIZE: u32 = 25;
    const MAX_PAGE: u32 = 1_000;
    if query.page > MAX_PAGE {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let result = sqlx::query_as::<_, (String, String)>(
        "SELECT substring(p.slug FROM 5), e.title
           FROM pages p
           JOIN editorial_pages e ON e.page_id = p.id
           JOIN editorial_pages source ON source.page_id = p.id AND source.locale = 'en'
          WHERE p.slug LIKE 'faq-%' AND e.locale = $1
            AND e.published_at <= now() AND source.published_at <= now()
            AND (e.locale = 'en' OR e.source_revision = source.revision)
          ORDER BY p.slug LIMIT $2 OFFSET $3",
    )
    .bind(lang)
    .bind(i64::from(PAGE_SIZE + 1))
    .bind(i64::from(query.page) * i64::from(PAGE_SIZE))
    .fetch_all(pool)
    .await;
    let Ok(mut entries) = result else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if query.page != 0 && entries.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let more = entries.len() > PAGE_SIZE as usize && query.page < MAX_PAGE;
    entries.truncate(PAGE_SIZE as usize);
    oteryn_web::render(&FaqIndex {
        lang,
        title: text(lang, "faq-title"),
        empty: text(lang, "faq-empty"),
        previous_label: text(lang, "faq-previous"),
        next_label: text(lang, "faq-next"),
        previous: query.page.checked_sub(1),
        next: more.then_some(query.page + 1),
        entries,
    })
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
    let help_links = matches!(&result, Ok(None)) && matches!(slug, "support" | "report-a-bug");
    let (status, title, body, version, draft_notice) = match result {
        Ok(Some(row)) => {
            let body: String = row.get("body");
            let title: String = row.get("title");
            let version: String = row.get("version");
            let draft_notice = if legal(slug)
                && oteryn_identity::consents::inspect_legal_edition(&version, &title, &body)
                    .needs_operator_update()
            {
                text(lang, "operator-draft")
            } else {
                String::new()
            };
            let mut html = String::new();
            pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&body));
            (
                StatusCode::OK,
                title,
                ammonia::clean(&html),
                if version.is_empty() {
                    String::new()
                } else {
                    format!("{} {}", version, row.get::<String, _>("effective"))
                },
                draft_notice,
            )
        }
        Ok(None) => (
            if reserved(slug) {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            },
            text(
                lang,
                if help_links {
                    slug
                } else if reserved(slug) {
                    "not-published"
                } else {
                    "unavailable"
                },
            ),
            String::new(),
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
        draft_notice,
        help_links,
        tickets_label: text(lang, "help-tickets"),
        password_label: text(lang, "help-password"),
        faq_label: if slug.starts_with(FAQ_PREFIX) {
            text(lang, "faq-back")
        } else {
            String::new()
        },
    });
    if response.status() == StatusCode::OK {
        *response.status_mut() = status;
    }
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
