//! SSR routes. A composing application must supply session authentication and synchronizer CSRF.
use super::*;
use crate::{Accounts, Currency, Error, Reservations};
use askama::Template;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use serde::Deserialize;
use std::{future::Future, sync::Arc};
use uuid::Uuid;

pub trait Market: Send + Sync + 'static {
    fn own_requests(
        &self,
        actor: Uuid,
    ) -> impl Future<Output = Result<Vec<OwnRequest>, Error>> + Send;
    fn cards(&self) -> impl Future<Output = Result<Vec<CharacterCard>, Error>> + Send;
    fn detail(&self, id: Uuid) -> impl Future<Output = Result<CharacterCard, Error>> + Send;
    fn choices(
        &self,
        actor: Uuid,
    ) -> impl Future<Output = Result<Vec<CharacterChoice>, Error>> + Send;
    fn request(
        &self,
        actor: Uuid,
        input: &ListingRequest,
    ) -> impl Future<Output = Result<Uuid, Error>> + Send;
    fn bid(
        &self,
        actor: Uuid,
        id: Uuid,
        amount: &str,
        key: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
impl<A: Accounts + Clone + 'static, R: Reservations + Clone + 'static, G: CharacterEvidence> Market
    for CharacterMarket<A, R, G>
{
    async fn own_requests(&self, actor: Uuid) -> Result<Vec<OwnRequest>, Error> {
        self.own_requests(actor).await
    }
    async fn cards(&self) -> Result<Vec<CharacterCard>, Error> {
        self.cards().await
    }
    async fn detail(&self, id: Uuid) -> Result<CharacterCard, Error> {
        self.detail(id).await
    }
    async fn choices(&self, actor: Uuid) -> Result<Vec<CharacterChoice>, Error> {
        self.choices(actor).await
    }
    async fn request(&self, actor: Uuid, input: &ListingRequest) -> Result<Uuid, Error> {
        self.request(actor, input).await
    }
    async fn bid(&self, actor: Uuid, id: Uuid, amount: &str, key: &str) -> Result<(), Error> {
        self.bid(actor, id, amount, key).await.map(|_| ())
    }
}
pub struct BrowserContext {
    pub actor: Uuid,
    pub csrf: String,
}
impl std::fmt::Debug for BrowserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserContext").finish_non_exhaustive()
    }
}
pub trait BrowserSecurity: Send + Sync + 'static {
    /// Read the server session; for mutations verify synchronizer token and origin before parsing.
    /// Never accept actor IDs, MFA/sudo or CSRF truth supplied by a form or public headers.
    fn authorize(
        &self,
        request: &mut Request,
        mutation: bool,
    ) -> impl Future<Output = Result<BrowserContext, StatusCode>> + Send;
}
struct HttpState<M, S> {
    market: Arc<M>,
    security: Arc<S>,
    enabled: bool,
}
impl<M, S> Clone for HttpState<M, S> {
    fn clone(&self) -> Self {
        Self {
            market: self.market.clone(),
            security: self.security.clone(),
            enabled: self.enabled,
        }
    }
}
pub fn routes<M: Market, S: BrowserSecurity>(market: M, security: S, enabled: bool) -> Router {
    Router::new()
        .route("/{lang}/bazaar", get(index::<M, S>))
        .route("/{lang}/account/bazaar", get(account::<M, S>))
        .route("/{lang}/bazaar/{listing}", get(detail::<M, S>))
        .route(
            "/{lang}/account/bazaar/sell",
            get(sell::<M, S>).post(create::<M, S>),
        )
        .route(
            "/{lang}/account/bazaar/{listing}/bids",
            axum::routing::post(bid::<M, S>),
        )
        .layer(axum::middleware::map_response(|response: Response| async {
            no_store(response)
        }))
        .with_state(HttpState {
            market: Arc::new(market),
            security: Arc::new(security),
            enabled,
        })
}
/// Mount documented refusal pages only when explicitly enabled. Missing accepted
/// custody/signing authority never enables a listing, bid or transfer command.
pub fn configured_router() -> Router {
    unavailable_routes(std::env::var("OTERYN_BAZAAR_CHARACTERS_ENABLED").as_deref() == Ok("true"))
}
fn authority_refusal(lang: &str) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, Html(format!("<!doctype html><html lang=\"{lang}\"><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{0}</title></head><body><main><h1>{0}</h1><p>{1}</p></main></body></html>",text(lang,"authority-title"),text(lang,"authority-unavailable")))).into_response()
}
fn unavailable_routes(enabled: bool) -> Router {
    if !enabled {
        return Router::new();
    }
    let mut routes = Router::new();
    for path in ["bazaar", "account/bazaar", "account/bazaar/sell"] {
        routes = routes.route(
            &format!("/{{locale}}/{path}"),
            get(|Path(lang): Path<String>| async move {
                if locale(&lang, true).is_err() {
                    return StatusCode::NOT_FOUND.into_response();
                }
                authority_refusal(&lang)
            }),
        );
    }
    routes = routes.route(
        "/{locale}/bazaar/{listing}",
        get(|Path((lang, listing)): Path<(String, String)>| async move {
            if locale(&lang, true).is_err() || super::id(&listing).is_err() {
                return StatusCode::NOT_FOUND.into_response();
            }
            authority_refusal(&lang)
        }),
    );
    routes.layer(axum::middleware::map_response(|response: Response| async {
        no_store(response)
    }))
}

fn locale(lang: &str, enabled: bool) -> Result<(), StatusCode> {
    if enabled && matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled => StatusCode::SERVICE_UNAVAILABLE,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}
fn no_store(response: impl IntoResponse) -> Response {
    let mut r = response.into_response();
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        r.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    r
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Filter {
    #[serde(default)]
    item: String,
}
async fn account<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path(lang): Path<String>,
    mut request: Request,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    let c = match s.security.authorize(&mut request, false).await {
        Ok(c) => c,
        Err(e) => return no_store(e),
    };
    match s.market.own_requests(c.actor).await {
        Ok(rows) => {
            let mut p = Page::new(lang, "account", vec![]);
            p.own_requests = rows;
            page(p)
        }
        Err(e) => no_store(status(e)),
    }
}
async fn index<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path(lang): Path<String>,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    match s.market.cards().await {
        Ok(cards) => page(Page::new(lang, "list", cards)),
        Err(e) => no_store(status(e)),
    }
}
async fn detail<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path((lang, id)): Path<(String, String)>,
    Query(filter): Query<Filter>,
    mut request: Request,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    if filter.item.len() > 128 {
        return no_store(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let Ok(id) = super::id(&id) else {
        return no_store(StatusCode::NOT_FOUND);
    };
    match s.market.detail(id).await {
        Ok(card) => {
            let mut p = Page::new(lang, "detail", vec![card]);
            p.item = filter.item;
            if let Ok(c) = s.security.authorize(&mut request, false).await {
                p.csrf = c.csrf;
            }
            page(p)
        }
        Err(e) => no_store(status(e)),
    }
}
async fn sell<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path(lang): Path<String>,
    mut request: Request,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    let context = match s.security.authorize(&mut request, false).await {
        Ok(c) => c,
        Err(e) => return no_store(e),
    };
    match s.market.choices(context.actor).await {
        Ok(choices) => {
            let mut p = Page::new(lang, "sell", vec![]);
            p.csrf = context.csrf;
            p.choices = choices;
            page(p)
        }
        Err(e) => no_store(status(e)),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListingForm {
    character_id: String,
    currency: String,
    starting_bid: String,
    duration_days: u8,
    request_id: String,
    publish_snapshot: Option<String>,
    _csrf: String,
}
async fn create<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path(lang): Path<String>,
    mut request: Request,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    let context = match s.security.authorize(&mut request, true).await {
        Ok(c) => c,
        Err(e) => return no_store(e),
    };
    let bytes = match body(request).await {
        Ok(b) => b,
        Err(e) => return no_store(e),
    };
    let form: ListingForm = match serde_urlencoded::from_bytes(&bytes) {
        Ok(f) => f,
        Err(_) => return no_store(StatusCode::UNPROCESSABLE_ENTITY),
    };
    let input = ListingRequest {
        character_id: form.character_id,
        currency: form.currency,
        starting_bid: form.starting_bid,
        duration_days: form.duration_days,
        request_id: form.request_id,
        publish_snapshot: form.publish_snapshot.as_deref() == Some("yes"),
    };
    match s.market.request(context.actor, &input).await {
        Ok(_) => no_store(Redirect::to(&format!("/{lang}/account/bazaar"))),
        Err(e) => no_store(status(e)),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BidForm {
    amount: String,
    request_id: String,
    _csrf: String,
}
async fn bid<M: Market, S: BrowserSecurity>(
    State(s): State<HttpState<M, S>>,
    Path((lang, id)): Path<(String, String)>,
    mut request: Request,
) -> Response {
    if let Err(e) = locale(&lang, s.enabled) {
        return no_store(e);
    }
    let context = match s.security.authorize(&mut request, true).await {
        Ok(c) => c,
        Err(e) => return no_store(e),
    };
    let Ok(id) = super::id(&id) else {
        return no_store(StatusCode::NOT_FOUND);
    };
    let bytes = match body(request).await {
        Ok(b) => b,
        Err(e) => return no_store(e),
    };
    let form: BidForm = match serde_urlencoded::from_bytes(&bytes) {
        Ok(f) => f,
        Err(_) => return no_store(StatusCode::UNPROCESSABLE_ENTITY),
    };
    match s
        .market
        .bid(context.actor, id, &form.amount, &form.request_id)
        .await
    {
        Ok(()) => no_store(Redirect::to(&format!("/{lang}/bazaar/{id}"))),
        Err(e) => no_store(status(e)),
    }
}
async fn body(mut request: Request) -> Result<Vec<u8>, StatusCode> {
    if !request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.split(';').next() == Some("application/x-www-form-urlencoded"))
    {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    to_bytes(std::mem::replace(request.body_mut(), Body::empty()), 8192)
        .await
        .map(|v| v.to_vec())
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)
}

#[derive(Template)]
#[template(path = "bazaar/characters.html", config = "src/characters/askama.toml")]
struct Page {
    lang: String,
    mode: &'static str,
    cards: Vec<CharacterCard>,
    choices: Vec<CharacterChoice>,
    csrf: String,
    request_id: String,
    item: String,
    own_requests: Vec<OwnRequest>,
}
impl Page {
    fn new(lang: String, mode: &'static str, cards: Vec<CharacterCard>) -> Self {
        Self {
            lang,
            mode,
            cards,
            choices: vec![],
            csrf: String::new(),
            request_id: oteryn_core::new_id().to_string(),
            item: String::new(),
            own_requests: vec![],
        }
    }
    fn text(&self, key: &str) -> String {
        text(&self.lang, key)
    }
    fn date(&self, value: &u64) -> String {
        i64::try_from(*value)
            .ok()
            .and_then(|v| time::OffsetDateTime::from_unix_timestamp(v).ok())
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
    fn currency(&self, currency: &Currency) -> String {
        self.text(match currency {
            Currency::Coins => "coins",
            Currency::Gold => "gold",
        })
    }
    fn vocation(&self, value: &u8) -> String {
        self.text(&format!("vocation-{value}"))
    }
}
pub fn text(lang: &str, key: &str) -> String {
    let (locale, source) = if lang == "en" {
        (
            unic_langid::langid!("en"),
            include_str!("../../../../templates/bazaar/en.ftl"),
        )
    } else {
        (
            unic_langid::langid!("pl"),
            include_str!("../../../../templates/bazaar/pl.ftl"),
        )
    };
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(value) = bundle.get_message(key).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle.format_pattern(value, None, &mut vec![]).into_owned()
}
fn page(p: Page) -> Response {
    match p.render() {
        Ok(body) => no_store(Html(body)),
        Err(_) => no_store(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Real browser adapter: actor and CSRF material come exclusively from the server-side session.
#[derive(Clone, Debug)]
pub struct SessionSecurity {
    pub csrf: oteryn_web::csrf::CsrfConfig,
}
impl BrowserSecurity for SessionSecurity {
    async fn authorize(
        &self,
        request: &mut Request,
        mutation: bool,
    ) -> Result<BrowserContext, StatusCode> {
        let session = request
            .extensions()
            .get::<oteryn_web::session::BrowserSession>()
            .cloned()
            .ok_or(StatusCode::FORBIDDEN)?;
        let actor = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
        if mutation {
            oteryn_web::csrf::verify(&self.csrf, &session, request).await?;
        }
        let csrf = session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(BrowserContext { actor, csrf })
    }
}
/// Compose after qualified T58 and Wallet adapters exist. The session layer applies to these
/// routes even when this router is merged after the shared web router has been constructed.
pub fn browser_routes<M: Market>(
    market: M,
    pool: sqlx::PgPool,
    security: SessionSecurity,
    enabled: bool,
) -> Router {
    let csrf = security.csrf.clone();
    routes(market, security, enabled)
        .route_layer(axum::middleware::from_fn_with_state(
            oteryn_web::session::SessionLayer::new(pool),
            oteryn_web::session::middleware,
        ))
        .layer(axum::Extension(csrf))
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn startup_refuses_without_game_authority_and_stays_off_by_default()
    -> Result<(), Box<dyn std::error::Error>> {
        for lang in ["pl", "en"] {
            let app = unavailable_routes(true);
            for path in [
                format!("/{lang}/bazaar"),
                format!("/{lang}/account/bazaar"),
                format!("/{lang}/bazaar/{}", Uuid::now_v7()),
            ] {
                let response = app
                    .clone()
                    .oneshot(axum::http::Request::get(path).body(Body::empty())?)
                    .await?;
                assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
                assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
                let html =
                    String::from_utf8(to_bytes(response.into_body(), 10000).await?.to_vec())?;
                assert!(html.contains(&format!("lang=\"{lang}\"")));
                assert!(html.contains(&text(lang, "authority-title")));
            }
            let invalid = app
                .oneshot(
                    axum::http::Request::get(format!("/{lang}/bazaar/invalid"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(invalid.status(), StatusCode::NOT_FOUND);
        }
        let composed = unavailable_routes(true)
            .merge(Router::new().route("/{locale}/bazaar/items", get(|| async { StatusCode::OK })));
        let items = composed
            .oneshot(axum::http::Request::get("/pl/bazaar/items").body(Body::empty())?)
            .await?;
        assert_eq!(items.status(), StatusCode::OK);
        let disabled = unavailable_routes(false)
            .oneshot(axum::http::Request::get("/en/bazaar").body(Body::empty())?)
            .await?;
        assert_eq!(disabled.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}
