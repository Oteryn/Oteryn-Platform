//! Browser surface. The configured runtime stays unavailable until T58 is qualified.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

fn text(lang: &str, key: &str) -> String {
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
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
#[template(path = "bazaar/message.html", config = "../web/askama.toml")]
struct Message<'a> {
    lang: &'a str,
    message: String,
}
fn failure(error: Error, lang: &str) -> Response {
    let (status, key) = match error {
        Error::InvalidInput | Error::Overflow => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::NotFound => (StatusCode::NOT_FOUND, "missing"),
        Error::Denied | Error::InsufficientFunds => (StatusCode::FORBIDDEN, "denied"),
        Error::IdempotencyConflict | Error::IllegalTransition => (StatusCode::CONFLICT, "conflict"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    match (Message {
        lang,
        message: text(lang, key),
    })
    .render()
    {
        Ok(html) => (status, [(header::CACHE_CONTROL, "no-store")], Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn language(lang: &str) -> Result<(), Error> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}
/// No environment variable can bypass the absent accepted capability/receipt integration.
/// All routes produce localized unavailable responses, including unsafe requests.
pub fn configured_router() -> Router {
    Router::new()
        .route("/{locale}/bazaar/items", get(unavailable).post(unavailable))
        .route("/{locale}/bazaar/items/new", get(unavailable))
        .route("/{locale}/bazaar/items/{id}", get(unavailable))
        .route("/{locale}/bazaar/items/{id}/{action}", post(unavailable))
}
async fn unavailable(Path(params): Path<std::collections::HashMap<String, String>>) -> Response {
    let lang = params.get("locale").map(String::as_str).unwrap_or("en");
    if let Err(e) = language(lang) {
        return failure(e, "en");
    }
    failure(Error::Disabled, lang)
}
/// Composable qualified runtime. Session middleware supplies authentication and validates
/// synchronizer CSRF tokens on every mutation; identity never comes from a form field.
pub fn router<A: Accounts + 'static, R: Reservations + 'static, G: ItemAuthority + 'static>(
    service: Items<A, R, G>,
    pool: PgPool,
    csrf: oteryn_web::csrf::CsrfConfig,
) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf = csrf;
    let limiter = oteryn_web::ratelimit::RateLimiter::new(pool);
    Router::new()
        .route(
            "/{locale}/bazaar/items",
            get(index::<A, R, G>).post(create::<A, R, G>),
        )
        .route("/{locale}/bazaar/items/new", get(new_listing))
        .route("/{locale}/bazaar/items/{id}", get(detail::<A, R, G>))
        .route(
            "/{locale}/bazaar/items/{id}/{action}",
            post(action::<A, R, G>),
        )
        .with_state(Arc::new(service))
        .layer(axum::middleware::from_fn_with_state(limiter, rate))
        .layer(axum::middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
}
#[derive(Template)]
#[template(path = "bazaar/item.html", config = "../web/askama.toml")]
struct Detail<'a> {
    lang: &'a str,
    item: ItemView,
    csrf: String,
    request: Uuid,
    owner: bool,
    offers: Vec<crate::Offer>,
}
impl Detail<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
async fn detail<A: Accounts, R: Reservations, G: ItemAuthority>(
    State(api): State<Arc<Items<A, R, G>>>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = language(&lang) {
        return failure(e, "en");
    }
    let actor = session.account_id().await;
    let result = async {
        let (item, offers, owner) = if let Some(actor) = actor {
            match api.owner_view(id, actor).await {
                Ok((item, offers)) => (item, offers, true),
                Err(Error::Denied) => (api.view(id).await?, vec![], false),
                Err(e) => return Err(e),
            }
        } else {
            (api.view(id).await?, vec![], false)
        };
        Ok::<_, Error>((item, offers, owner, session.csrf_token().await?))
    }
    .await;
    match result {
        Ok((item, offers, owner, csrf)) => match (Detail {
            lang: &lang,
            item,
            csrf,
            request: Uuid::now_v7(),
            owner,
            offers,
        })
        .render()
        {
            Ok(body) => ([(header::CACHE_CONTROL, "no-store")], Html(body)).into_response(),
            Err(_) => failure(Error::Denied, &lang),
        },
        Err(e) => failure(e, &lang),
    }
}
async fn create<A: Accounts, R: Reservations, G: ItemAuthority>(
    State(api): State<Arc<Items<A, R, G>>>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    input: Result<Form<CreateForm>, axum::extract::rejection::FormRejection>,
) -> Response {
    if let Err(e) = language(&lang) {
        return failure(e, "en");
    }
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::InvalidInput, &lang),
    };
    let Some(actor) = session.account_id().await else {
        return failure(Error::Denied, &lang);
    };
    let input = match input.parse() {
        Ok(input) => input,
        Err(e) => return failure(e, &lang),
    };
    match api.create(actor, input).await {
        Ok(id) => Redirect::to(&format!("/{lang}/bazaar/items/{id}")).into_response(),
        Err(e) => failure(e, &lang),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
    request_id: Uuid,
    amount: Option<i64>,
    offer_id: Option<Uuid>,
}
async fn action<A: Accounts, R: Reservations, G: ItemAuthority>(
    State(api): State<Arc<Items<A, R, G>>>,
    Path((lang, id, action)): Path<(String, Uuid, String)>,
    Extension(session): Extension<BrowserSession>,
    input: Result<Form<Action>, axum::extract::rejection::FormRejection>,
) -> Response {
    if let Err(e) = language(&lang) {
        return failure(e, "en");
    }
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::InvalidInput, &lang),
    };
    let Some(actor) = session.account_id().await else {
        return failure(Error::Denied, &lang);
    };
    if !super::id(input.request_id) {
        return failure(Error::InvalidInput, &lang);
    }
    let result = match action.as_str() {
        "bid" => match input.amount {
            Some(amount) => api
                .bid(id, actor, amount, input.request_id)
                .await
                .map(|_| ()),
            None => Err(Error::InvalidInput),
        },
        "offer" => match input.amount {
            Some(amount) => api
                .offer(id, actor, amount, input.request_id)
                .await
                .map(|_| ()),
            None => Err(Error::InvalidInput),
        },
        "accept" => match input.offer_id {
            Some(offer) => api.accept(id, actor, offer).await,
            None => Err(Error::InvalidInput),
        },
        "reject" => match input.offer_id {
            Some(offer) => api.reject(id, actor, offer).await,
            None => Err(Error::InvalidInput),
        },
        "buy-now" => api.buy_now(id, actor, input.request_id).await,
        "publish" => api.publish(id, actor).await,
        "cancel" => api.cancel(id, actor).await,
        "renew" => api.renew(id, actor, input.request_id).await,
        _ => Err(Error::NotFound),
    };
    match result {
        Ok(()) => Redirect::to(&format!("/{lang}/bazaar/items/{id}")).into_response(),
        Err(e) => failure(e, &lang),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateForm {
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
    request_id: Uuid,
    mode: Mode,
    definition_key: String,
    definition_revision: String,
    world_id: Uuid,
    #[serde(default)]
    seller_character_id: String,
    #[serde(default)]
    item_instance_id: String,
    quantity: u32,
    #[serde(default)]
    unit_price: String,
    #[serde(default)]
    buy_now_unit_price: String,
    #[serde(default)]
    auction_days: String,
    #[serde(default)]
    message: String,
}
fn optional<T: std::str::FromStr>(value: &str) -> Result<Option<T>, Error> {
    if value.is_empty() {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(|_| Error::InvalidInput)
    }
}
impl CreateForm {
    fn parse(self) -> Result<CreateItem, Error> {
        Ok(CreateItem {
            request_id: self.request_id,
            mode: self.mode,
            definition_key: self.definition_key,
            definition_revision: self.definition_revision,
            world_id: self.world_id,
            seller_character_id: optional(&self.seller_character_id)?,
            item_instance_id: optional(&self.item_instance_id)?,
            quantity: self.quantity,
            unit_price: optional(&self.unit_price)?,
            buy_now_unit_price: optional(&self.buy_now_unit_price)?,
            auction_days: optional(&self.auction_days)?,
            message: self.message,
        })
    }
}
#[derive(Template)]
#[template(path = "bazaar/new.html", config = "../web/askama.toml")]
struct New<'a> {
    lang: &'a str,
    csrf: String,
    request: Uuid,
}
impl New<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
async fn new_listing(
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = language(&lang) {
        return failure(e, "en");
    }
    if session.account_id().await.is_none() {
        return failure(Error::Denied, &lang);
    }
    match session.csrf_token().await {
        Ok(csrf) => match (New {
            lang: &lang,
            csrf,
            request: Uuid::now_v7(),
        })
        .render()
        {
            Ok(body) => ([(header::CACHE_CONTROL, "no-store")], Html(body)).into_response(),
            Err(_) => failure(Error::Denied, &lang),
        },
        Err(e) => failure(e.into(), &lang),
    }
}
#[derive(Template)]
#[template(path = "bazaar/list.html", config = "../web/askama.toml")]
struct Index<'a> {
    lang: &'a str,
    items: Vec<ItemView>,
}
impl Index<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
async fn index<A: Accounts, R: Reservations, G: ItemAuthority>(
    State(api): State<Arc<Items<A, R, G>>>,
    Path(lang): Path<String>,
) -> Response {
    if let Err(e) = language(&lang) {
        return failure(e, "en");
    }
    match api.recent().await {
        Ok(items) => match (Index { lang: &lang, items }).render() {
            Ok(body) => ([(header::CACHE_CONTROL, "no-store")], Html(body)).into_response(),
            Err(_) => failure(Error::Denied, &lang),
        },
        Err(e) => failure(e, &lang),
    }
}

async fn rate(
    State(limiter): State<oteryn_web::ratelimit::RateLimiter>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if request.method() != axum::http::Method::POST {
        return next.run(request).await;
    }
    let lang = if request.uri().path().starts_with("/pl/") {
        oteryn_web::Language::Pl
    } else {
        oteryn_web::Language::En
    };
    let Some(session) = request.extensions().get::<BrowserSession>() else {
        return failure(Error::Denied, lang.code());
    };
    let Some(actor) = session.account_id().await else {
        return failure(Error::Denied, lang.code());
    };
    let Some(peer) = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    else {
        return failure(Error::Denied, lang.code());
    };
    let mut keys = oteryn_web::ratelimit::Keys::source(peer.0.ip());
    keys.account = Some(actor.to_string());
    let policy = if request.uri().path().ends_with("/items") {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceListing
    } else {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceBid
    };
    match limiter.check(policy, &[keys]).await {
        Ok(()) => next.run(request).await,
        Err(denial) => denial.response(lang, false),
    }
}
