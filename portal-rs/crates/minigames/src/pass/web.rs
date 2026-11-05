//! Authenticated, private browser surface. Production adapters deny by default.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "pass/page.html", config = "../../templates/pass/askama.toml")]
struct Page {
    lang: String,
    title: String,
    unavailable: String,
    xp_label: String,
    price_label: String,
    buy_label: String,
    claim_label: String,
    free_label: String,
    paid_label: String,
    claimed_label: String,
    journal_label: String,
    portal_label: String,
    challenges_label: String,
    rewards_label: String,
    end_label: String,
    csrf: String,
    operation: Uuid,
    overview: Option<Overview>,
}
fn text(lang: &str, id: &str) -> String {
    let source = if lang == "pl" {
        include_str!("../../../../templates/pass/pl.ftl")
    } else {
        include_str!("../../../../templates/pass/en.ftl")
    };
    let locale = if lang == "pl" { "pl" } else { "en" };
    let Ok(language) = locale.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![language]);
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle
        .format_pattern(pattern, None, &mut vec![])
        .into_owned()
}
impl Page {
    fn new(lang: String, csrf: String, overview: Option<Overview>) -> Self {
        Self {
            title: text(&lang, "title"),
            unavailable: text(&lang, "unavailable"),
            xp_label: text(&lang, "xp"),
            price_label: text(&lang, "price"),
            buy_label: text(&lang, "buy"),
            claim_label: text(&lang, "claim"),
            free_label: text(&lang, "free"),
            paid_label: text(&lang, "paid"),
            claimed_label: text(&lang, "claimed"),
            journal_label: text(&lang, "journal"),
            portal_label: text(&lang, "portal"),
            challenges_label: text(&lang, "challenges"),
            rewards_label: text(&lang, "rewards"),
            end_label: text(&lang, "ends"),
            lang,
            csrf,
            operation: Uuid::now_v7(),
            overview,
        }
    }
    fn response(self, status: StatusCode) -> Response {
        match self.render() {
            Ok(html) => (
                status,
                [(header::CACHE_CONTROL, "private, no-store")],
                Html(html),
            )
                .into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}
/// This router is usable only with trusted server-side adapters. Browser
/// requests never carry account IDs, prices, XP or evidence assertions.
pub fn router<A: Authority + 'static, P: Payment + 'static, R: Rewards + 'static>(
    pool: PgPool,
    passes: Passes<A, P, R>,
    flags: Flags,
    csrf: CsrfConfig,
) -> Router {
    if !flags.enabled {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool);
    sessions.csrf = csrf;
    Router::new()
        .route("/{lang}/pass", get(page::<A, P, R>))
        .route(
            "/{lang}/pass/{season}/buy",
            axum::routing::post(buy::<A, P, R>),
        )
        .route(
            "/{lang}/pass/{season}/claim",
            axum::routing::post(claim::<A, P, R>),
        )
        .layer(axum::middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(axum::middleware::from_fn(response_policy))
        .layer(DefaultBodyLimit::max(4096))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
        .with_state(Arc::new(passes))
}
/// No environment setting can enable missing source or cosmetic authority.
/// Adapters remain unavailable until their owning contracts/APIs are supplied.
pub fn configured_router(pool: PgPool) -> Router {
    let flags = Flags {
        enabled: std::env::var("OTERYN_SEASON_PASS_ENABLED").as_deref() == Ok("true"),
        ..Flags::default()
    };
    let passes = Passes::new(pool.clone(), flags, Unavailable, Unavailable, Unavailable);
    let csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    router(pool, passes, flags, csrf)
}
async fn response_policy(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let lang = if request.uri().path().starts_with("/pl/") {
        "pl"
    } else {
        "en"
    }
    .to_owned();
    let mut response = next.run(request).await;
    if response.status().is_client_error() || response.status().is_server_error() {
        response = Page::new(lang, String::new(), None).response(response.status());
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response
}
fn locale(lang: &str) -> std::result::Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
fn refusal(lang: String, error: Error) -> Response {
    let status = match error {
        Error::Disabled => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    Page::new(lang, String::new(), None).response(status)
}
async fn page<A: Authority, P: Payment, R: Rewards>(
    Path(lang): Path<String>,
    State(passes): State<Arc<Passes<A, P, R>>>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(status) = locale(&lang) {
        return status.into_response();
    }
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let csrf = match session.csrf_token().await {
        Ok(value) => value,
        Err(_) => return refusal(lang, Error::Unavailable),
    };
    match passes.overview(account).await {
        Ok(view) => Page::new(lang, csrf, Some(view)).response(StatusCode::OK),
        Err(error) => refusal(lang, error),
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuyForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    operation: Uuid,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    operation: Uuid,
    reward: Uuid,
}
async fn buy<A: Authority, P: Payment, R: Rewards>(
    Path((lang, season)): Path<(String, Uuid)>,
    State(passes): State<Arc<Passes<A, P, R>>>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<BuyForm>,
) -> Response {
    if let Err(status) = locale(&lang) {
        return status.into_response();
    }
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match passes.buy(account, season, form.operation).await {
        Ok(_) => Redirect::to(&format!("/{lang}/pass")).into_response(),
        Err(error) => refusal(lang, error),
    }
}
async fn claim<A: Authority, P: Payment, R: Rewards>(
    Path((lang, season)): Path<(String, Uuid)>,
    State(passes): State<Arc<Passes<A, P, R>>>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<ClaimForm>,
) -> Response {
    if let Err(status) = locale(&lang) {
        return status.into_response();
    }
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match passes
        .claim(account, season, form.reward, form.operation)
        .await
    {
        Ok(_) => Redirect::to(&format!("/{lang}/pass")).into_response(),
        Err(error) => refusal(lang, error),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_page_has_both_locales_and_mobile_viewport()
    -> std::result::Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            let page = Page::new(lang.into(), String::new(), None);
            assert!(!page.title.is_empty());
            assert!(!page.unavailable.is_empty());
            let html = page.render()?;
            assert!(html.contains("width=device-width"));
            assert!(html.contains("role=\"status\""));
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(!html.contains("<form"));
        }
        Ok(())
    }
}
