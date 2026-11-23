//! Authenticated, private browser surface. Production adapters deny by default.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer, SignedInSession},
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
    check_purchase: Option<String>,
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
    show_operator: bool,
    show_puzzles_link: bool,
}
pub(super) fn text(lang: &str, id: &str) -> String {
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
            check_purchase: None,
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
            show_operator: false,
            show_puzzles_link: std::env::var("OTERYN_DAILY_PUZZLES_ENABLED").as_deref()
                == Ok("true"),
        }
    }
    fn t(&self, key: &str) -> String {
        text(&self.lang, key)
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
        .route(
            "/{lang}/pass",
            get(page::<A, P, R>).layer(axum::middleware::from_fn_with_state(
                sessions.clone(),
                oteryn_web::session::read_only_middleware,
            )),
        )
        .merge(
            Router::new()
                .route(
                    "/{lang}/pass/{season}/buy",
                    axum::routing::post(buy::<A, P, R>),
                )
                .route(
                    "/{lang}/pass/{season}/claim",
                    axum::routing::post(claim::<A, P, R>),
                )
                .route(
                    "/{lang}/pass/{season}/progress",
                    axum::routing::post(sync_puzzles::<A, P, R>),
                )
                .layer(axum::middleware::from_fn_with_state(
                    sessions,
                    oteryn_web::session::middleware,
                )),
        )
        .layer(axum::middleware::from_fn(response_policy))
        .layer(DefaultBodyLimit::max(4096))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
        .layer(axum::middleware::from_fn(crate::browser::deadline))
        .with_state(Arc::new(passes))
}
/// Platform progress uses the account authority. Spending stays testing-only,
/// and Game delivery stays unavailable until the Store adapter is qualified.
pub fn configured_router(pool: PgPool) -> Router {
    let enabled = std::env::var("OTERYN_SEASON_PASS_ENABLED").as_deref() == Ok("true");
    let testing = std::env::var("OTERYN_ENV").as_deref() == Ok("testing");
    let flags = Flags {
        enabled,
        paid: enabled
            && testing
            && std::env::var("OTERYN_SEASON_PASS_WALLET_ENABLED").as_deref() == Ok("true"),
        rewards: false,
    };
    let csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    let payment = platform::payment(pool.clone(), flags.paid);
    let passes = Passes::new(
        pool.clone(),
        flags,
        platform::AccountAuthority,
        payment,
        Unavailable,
    );
    let operator = operator::Operator::configured(pool.clone(), flags);
    let player = router(pool, passes, flags, csrf.clone());
    if let Some(operator) = operator {
        let staff = operator.router(csrf);
        player.layer(Extension(operator)).merge(staff)
    } else {
        player
    }
}
pub(super) async fn response_policy(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let purchase = purchase_language(&request);
    let lang = if request.uri().path().starts_with("/pl/") {
        "pl"
    } else {
        "en"
    }
    .to_owned();
    let mut response = next.run(request).await;
    let rendered = response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes().starts_with(b"text/html"));
    if !rendered && (response.status().is_client_error() || response.status().is_server_error()) {
        response = if let Some(lang) = purchase.filter(|_| response.status().is_server_error()) {
            uncertain_purchase(lang, response.status())
        } else {
            Page::new(lang, String::new(), None).response(response.status())
        };
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
pub(crate) fn purchase_language(request: &axum::extract::Request) -> Option<&'static str> {
    if request.method() != Method::POST {
        return None;
    }
    let mut segments = request.uri().path().split('/').skip(1);
    let lang = match segments.next()? {
        "pl" => "pl",
        "en" => "en",
        _ => return None,
    };
    match (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) {
        (Some("pass"), Some(_), Some("buy"), None) => Some(lang),
        _ => None,
    }
}

pub(crate) fn uncertain_purchase(lang: &str, status: StatusCode) -> Response {
    let mut page = Page::new(lang.to_owned(), String::new(), None);
    page.unavailable = text(lang, "purchase-unknown");
    page.check_purchase = Some(text(lang, "check-purchase"));
    page.response(status)
}

pub(super) fn locale(lang: &str) -> std::result::Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
pub(super) fn refusal(lang: String, error: Error) -> Response {
    let status = match error {
        Error::Disabled => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    let mut page = Page::new(lang, String::new(), None);
    if matches!(error, Error::Conflict | Error::Invalid | Error::Denied) {
        page.unavailable = page.t(match error {
            Error::Conflict => "conflict",
            Error::Invalid => "invalid",
            _ => "denied",
        });
    }
    page.response(status)
}
async fn page<A: Authority, P: Payment, R: Rewards>(
    Path(lang): Path<String>,
    State(passes): State<Arc<Passes<A, P, R>>>,
    session: Option<Extension<SignedInSession>>,
    operator: Option<Extension<operator::Operator>>,
    headers: HeaderMap,
) -> Response {
    if let Err(status) = locale(&lang) {
        return status.into_response();
    }
    let Some(Extension(session)) = session else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let (overview, status) = match passes.overview(session.account_id).await {
        Ok(view) => (Some(view), StatusCode::OK),
        Err(Error::Unavailable) => (None, StatusCode::SERVICE_UNAVAILABLE),
        Err(error) => return refusal(lang, error),
    };
    let mut page = Page::new(lang, session.csrf, overview);
    if let Some(Extension(operator)) = operator {
        page.show_operator = operator.show_link(&headers, session.account_id).await;
    }
    page.response(status)
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
        Err(Error::Database(_) | Error::Unavailable) => {
            uncertain_purchase(&lang, StatusCode::SERVICE_UNAVAILABLE)
        }
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
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    challenge: Uuid,
}
async fn sync_puzzles<A: Authority, P: Payment, R: Rewards>(
    Path((lang, season)): Path<(String, Uuid)>,
    State(passes): State<Arc<Passes<A, P, R>>>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<ProgressForm>,
) -> Response {
    if let Err(status) = locale(&lang) {
        return status.into_response();
    }
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match passes.sync_puzzles(account, season, form.challenge).await {
        Ok(_) => Redirect::to(&format!("/{lang}/pass")).into_response(),
        Err(error) => refusal(lang, error),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_journal_progress_has_no_numeric_value() -> std::result::Result<(), askama::Error> {
        let view = Overview {
            can_buy: false,
            season: Uuid::now_v7(),
            ends_at: time::OffsetDateTime::now_utc(),
            price: 10,
            progress: Progress { xp: 0, paid: false },
            challenges: vec![ChallengeView {
                definition: Challenge {
                    id: Uuid::now_v7(),
                    source: Source::Journal,
                    metric: "test.metric".into(),
                    target: 2,
                    xp: 5,
                },
                amount: 0,
            }],
            rewards: Vec::new(),
        };
        let html = Page::new("en".into(), String::new(), Some(view)).render()?;
        assert!(html.contains("Game journal challenge"));
        assert!(!html.contains("0 / 2"));
        Ok(())
    }

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
