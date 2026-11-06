//! Localized SSR routes with server-side sessions and synchronizer-token CSRF.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionKey, SessionLayer};
use serde::Serialize;

pub fn router(pool: PgPool, policy: Policy) -> Router {
    router_with_origin(pool, policy, std::env::var("OTERYN_PUBLIC_ORIGIN").ok())
}

pub(super) fn router_with_origin(pool: PgPool, policy: Policy, origin: Option<String>) -> Router {
    if !policy.enabled || !policy.valid() {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = origin.into_iter().collect();
    Router::new()
        .route("/{lang}/support/reports", get(index).post(store))
        .route("/{lang}/support/reports/create", get(create))
        .route("/{lang}/support/reports/{id}", get(show))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn(private_headers))
        .with_state(Reports::new(pool, policy))
}

/// Only the exact value `true` enables reporting. Invalid policy values deny routes.
pub fn configured_router(pool: PgPool) -> Router {
    let pending_limit =
        std::env::var("OTERYN_REPORTS_PENDING_LIMIT").map_or(5, |v| v.parse().unwrap_or(0));
    let unjustified_limit =
        std::env::var("OTERYN_REPORTS_UNJUSTIFIED_LIMIT").map_or(3, |v| v.parse().unwrap_or(0));
    router(
        pool,
        Policy {
            enabled: std::env::var("OTERYN_REPORTS_ENABLED").as_deref() == Ok("true"),
            pending_limit,
            unjustified_limit,
        },
    )
}

#[derive(Clone, Copy)]
struct Locale(&'static str);
impl Locale {
    fn parse(lang: &str) -> Result<Self, StatusCode> {
        match lang {
            "pl" => Ok(Self("pl")),
            "en" => Ok(Self("en")),
            _ => Err(StatusCode::NOT_FOUND),
        }
    }
    fn text(self, key: &str) -> String {
        let source = if self.0 == "pl" {
            include_str!("pl.ftl")
        } else {
            include_str!("en.ftl")
        };
        let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
            return String::new();
        };
        let Ok(language) = self.0.parse() else {
            return String::new();
        };
        let mut bundle = FluentBundle::new(vec![language]);
        if bundle.add_resource(resource).is_err() {
            return String::new();
        }
        bundle
            .get_message(key)
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct Input {
    request_key: String,
    report_type: String,
    category: String,
    target_reference: String,
    evidence_summary: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
struct Flash {
    input: Input,
    error: String,
}
const FLASH: SessionKey<Option<Flash>> = SessionKey::new("support.reports.flash");

#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ locale.0 }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ self.text("title") }} · Oteryn</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head>
<body class="op-theme"><main class="op-stack"><h1>{{ self.text("title") }}</h1>
<nav class="op-row" aria-label="{{ self.text("navigation") }}"><a class="op-button op-button--secondary" href="/{{ locale.0 }}/support/reports">{{ self.text("history") }}</a><a class="op-button" href="/{{ locale.0 }}/support/reports/create">{{ self.text("new") }}</a></nav>
{% if !error.is_empty() %}<p class="op-alert op-alert--danger" role="alert">{{ self.text(error) }}</p>{% endif %}
{% if form %}<form class="op-panel op-stack" method="post" action="/{{ locale.0 }}/support/reports">
<input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="request_key" value="{{ input.request_key }}">
<div class="op-field"><label for="report_type">{{ self.text("type") }}</label><select class="op-input" id="report_type" name="report_type" required>{% for kind in kinds %}<option value="{{ kind }}" {% if input.report_type == *kind %}selected{% endif %}>{{ self.text(kind) }}</option>{% endfor %}</select></div>
<div class="op-field"><label for="category">{{ self.text("category") }}</label><select class="op-input" id="category" name="category" aria-describedby="category-help" required>{% for kind in kinds %}<optgroup label="{{ self.text(kind) }}">{% for category in self.categories(kind) %}<option value="{{ category }}" {% if input.category == *category && input.report_type == *kind %}selected{% endif %}>{{ self.text(category) }}</option>{% endfor %}</optgroup>{% endfor %}</select><p id="category-help" class="op-hint">{{ self.text("category-help") }}</p></div>
<div class="op-field"><label for="target_reference">{{ self.text("target") }}</label><input class="op-input" id="target_reference" name="target_reference" maxlength="160" required value="{{ input.target_reference }}"></div>
<div class="op-field"><label for="evidence_summary">{{ self.text("evidence") }}</label><textarea class="op-input" id="evidence_summary" name="evidence_summary" maxlength="4000" rows="8">{{ input.evidence_summary.as_deref().unwrap_or_default() }}</textarea></div>
<button class="op-button" type="submit">{{ self.text("submit") }}</button></form>{% else %}
{% if reports.is_empty() %}<p class="op-empty">{{ self.text("empty") }}</p>{% endif %}
{% for report in reports %}<article class="op-panel op-stack"><h2><a href="/{{ locale.0 }}/support/reports/{{ report.id }}">{{ report.target_reference }}</a></h2><p>{{ self.text(&report.report_type) }} · {{ self.text(&report.category) }}</p><p>{{ self.text("status") }}: {{ self.text(&report.status) }}</p><p>{{ self.text("date") }}: {{ report.created_at.date() }}</p>
{% if detail %}<p>{{ report.evidence_summary.as_deref().unwrap_or_default() }}</p>{% if let Some(outcome) = report.public_outcome %}<h3>{{ self.text("outcome") }}</h3><p>{{ outcome }}</p>{% endif %}{% endif %}</article>{% endfor %}
{% if !detail %}<nav class="op-row" aria-label="{{ self.text("pagination") }}">{% if page > 1 %}<a class="op-button op-button--secondary" href="?page={{ page - 1 }}">{{ self.text("previous") }}</a>{% endif %}{% if reports.len() == 20 %}<a class="op-button op-button--secondary" href="?page={{ page + 1 }}">{{ self.text("next") }}</a>{% endif %}</nav>{% endif %}
{% endif %}</main></body></html>"#,
    ext = "html"
)]
struct Page {
    locale: Locale,
    kinds: &'static [&'static str],
    csrf: String,
    input: Input,
    error: String,
    reports: Vec<Report>,
    form: bool,
    detail: bool,
    page: u32,
}
impl Page {
    fn new(locale: Locale) -> Self {
        Self {
            locale,
            kinds: TYPES,
            csrf: String::new(),
            input: Input::default(),
            error: String::new(),
            reports: vec![],
            form: false,
            detail: false,
            page: 1,
        }
    }
    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }
    fn categories(&self, kind: &str) -> &'static [&'static str] {
        categories(kind)
    }
    fn render(&self) -> Result<Response, StatusCode> {
        self.render_html()
            .map(axum::response::Html)
            .map(IntoResponse::into_response)
    }
    fn render_html(&self) -> Result<String, StatusCode> {
        Template::render(self).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}
fn redirect(location: &str) -> Response {
    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}
async fn owner(session: &BrowserSession, locale: Locale) -> Result<AccountId, Response> {
    session
        .account_id()
        .await
        .ok_or_else(|| redirect(&format!("/{}/login", locale.0)))
}
fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Forbidden | Error::UnjustifiedLimit => StatusCode::FORBIDDEN,
        Error::PendingLimit => StatusCode::CONFLICT,
        Error::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        Error::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
#[derive(Deserialize)]
struct Paging {
    #[serde(default = "first_page")]
    page: u32,
}
fn first_page() -> u32 {
    1
}
async fn index(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Query(paging): Query<Paging>,
) -> Result<Response, Response> {
    let locale = Locale::parse(&lang).map_err(IntoResponse::into_response)?;
    let owner = owner(&session, locale).await?;
    let mut page = Page::new(locale);
    page.page = paging.page;
    page.reports = service
        .list(owner, paging.page)
        .await
        .map_err(|e| status(e).into_response())?;
    page.render().map_err(IntoResponse::into_response)
}
async fn create(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, Response> {
    let locale = Locale::parse(&lang).map_err(IntoResponse::into_response)?;
    let owner = owner(&session, locale).await?;
    service
        .list(owner, 1)
        .await
        .map_err(|e| status(e).into_response())?;
    let mut page = Page::new(locale);
    page.form = true;
    page.csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if let Some(flash) = session
        .get(FLASH)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .flatten()
    {
        page.input = flash.input;
        page.error = flash.error;
        session
            .set(FLASH, &None::<Flash>)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    } else {
        page.input.request_key = oteryn_core::new_id().to_string();
        page.input.report_type = "player".to_owned();
        page.input.category = "harassment".to_owned();
    }
    page.render().map_err(IntoResponse::into_response)
}
async fn store(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    input: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, Response> {
    let locale = Locale::parse(&lang).map_err(IntoResponse::into_response)?;
    let owner = owner(&session, locale).await?;
    let input = input.map(|Form(input)| input).unwrap_or_default();
    let request_key = input.request_key.parse().unwrap_or(Uuid::nil());
    let result = {
        service
            .submit(
                owner,
                Submission {
                    request_key,
                    report_type: input.report_type.clone(),
                    category: input.category.clone(),
                    target_reference: input.target_reference.clone(),
                    evidence_summary: input.evidence_summary.clone(),
                },
            )
            .await
    };
    match result {
        Ok(report) => Ok(redirect(&format!(
            "/{}/support/reports/{}",
            locale.0, report.id
        ))),
        Err(Error::RateLimited) => Err((
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            locale.text("rate-limited"),
        )
            .into_response()),
        Err(error @ (Error::Invalid | Error::PendingLimit | Error::UnjustifiedLimit)) => {
            let key = match error {
                Error::PendingLimit => "pending-limit",
                Error::UnjustifiedLimit => "unjustified-limit",
                _ => "invalid",
            };
            // Invalid oversized input must never become unbounded session data.
            let mut input = input;
            input.target_reference = input.target_reference.chars().take(160).collect();
            input.evidence_summary = input
                .evidence_summary
                .map(|s| s.chars().take(2000).collect());
            input.report_type = input.report_type.chars().take(16).collect();
            input.category = input.category.chars().take(32).collect();
            input.request_key = input.request_key.chars().take(64).collect();
            session
                .set(
                    FLASH,
                    &Some(Flash {
                        input,
                        error: key.to_owned(),
                    }),
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
            Ok(redirect(&format!("/{}/support/reports/create", locale.0)))
        }
        Err(e) => Err(status(e).into_response()),
    }
}
async fn show(
    State(service): State<Reports>,
    Path((lang, id)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, Response> {
    let locale = Locale::parse(&lang).map_err(IntoResponse::into_response)?;
    let owner = owner(&session, locale).await?;
    let id = id
        .parse()
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    let mut page = Page::new(locale);
    page.detail = true;
    page.reports = vec![
        service
            .get(owner, id)
            .await
            .map_err(|e| status(e).into_response())?,
    ];
    page.render().map_err(IntoResponse::into_response)
}
async fn private_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (header::CACHE_CONTROL, "private, no-store"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
        (
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
