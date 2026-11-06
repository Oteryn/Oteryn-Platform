use super::*;
use crate::{Evidence, State};
use askama::Template;
use axum::{
    Extension, Form as BrowserForm, Router,
    extract::{Path, State as HttpState},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use sha2::{Digest, Sha256};
#[derive(Debug)]
struct Card {
    id: Uuid,
    name: String,
    kind: String,
    state: String,
    price: i64,
    currency: String,
    owner: bool,
    refused: bool,
    completed: bool,
    cancelled: bool,
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><header><a href=\"/{{ lang }}/bazaar\">{{ title }}</a> <a href=\"/{{ lang }}/account/bazaar\">{{ own_label }}</a> <a href=\"/{{ lang }}/account/bazaar/sell\">{{ sell_label }}</a></header><main><h1>{{ title }}</h1>{% if !message.is_empty() %}<p role=\"status\">{{ message }}</p>{% endif %}{% if create %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/sell\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><label>{{ kind_label }}<select name=\"kind\"><option value=\"character\">{{ character_label }}</option><option value=\"item\">{{ item_label }}</option></select></label><label>{{ world_label }}<input required name=\"world_id\"></label><label>{{ character_label }} ID<input required name=\"seller_character_id\"></label><label>{{ asset_label }}<input required name=\"asset_id\"></label><label>{{ revision_label }}<input name=\"expected_revision\" required value=\"7\"></label><label>{{ quantity_label }}<input name=\"quantity\" required type=\"number\" min=\"1\" value=\"1\"></label><label>{{ definition_label }}<input name=\"definition_key\" value=\"item\"></label><label>{{ version_label }}<input name=\"definition_revision\" value=\"v1\"></label><label>{{ currency_label }}<select name=\"currency\"><option value=\"COINS\">Coins</option><option value=\"GOLD\">Gold</option></select></label><label>{{ price_label }}<input name=\"starting_bid\" required type=\"number\" min=\"1\"></label><label>{{ days_label }}<input name=\"duration_days\" required type=\"number\" min=\"1\" max=\"14\" value=\"1\"></label><label><input type=\"checkbox\" name=\"publish_snapshot\" value=\"true\" required>{{ consent_label }}</label><button type=\"submit\">{{ sell_label }}</button></form>{% endif %}{% for card in cards %}<article data-listing=\"{{ card.id }}\" data-state=\"{{ card.state }}\"><h2><a href=\"/{{ lang }}/bazaar/{{ card.id }}\">{{ card.name }}</a></h2><p>{% if card.kind == \"character\" %}{{ character_label }}{% else %}{{ item_label }}{% endif %} · {{ card.price }} {{ card.currency }}</p><p role=\"status\">{% if card.refused %}{{ refused_label }}{% elif card.cancelled %}{{ cancelled_label }}{% elif card.completed %}{{ complete_label }}{% elif card.state == \"active\" %}{{ active_label }}{% else %}{{ pending_label }}{% endif %}</p>{% if detail && card.owner && !card.completed && !card.refused %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/{{ card.id }}/control\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><button name=\"action\" value=\"resume\">{{ resume_label }}</button><button name=\"action\" value=\"cancel\">{{ cancel_label }}</button></form>{% endif %}{% if detail && card.state == \"active\" && !card.owner %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/{{ card.id }}/bids\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><label>{{ destination_label }}<input name=\"buyer_character_id\" required></label><label>{{ maximum_label }}<input name=\"maximum\" type=\"number\" min=\"1\" required></label><button type=\"submit\">{{ bid_label }}</button></form>{% endif %}</article>{% endfor %}</main></body></html>",
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    own_label: String,
    sell_label: String,
    resume_label: String,
    cancel_label: String,
    kind_label: String,
    character_label: String,
    item_label: String,
    world_label: String,
    asset_label: String,
    revision_label: String,
    quantity_label: String,
    definition_label: String,
    version_label: String,
    price_label: String,
    currency_label: String,
    days_label: String,
    consent_label: String,
    refused_label: String,
    complete_label: String,
    cancelled_label: String,
    active_label: String,
    pending_label: String,
    destination_label: String,
    maximum_label: String,
    bid_label: String,
    message: String,
    csrf: String,
    request: Uuid,
    create: bool,
    detail: bool,
    cards: Vec<Card>,
}
fn translated(lang: &str, key: &str) -> String {
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.into()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|v| bundle.format_pattern(v, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
impl<'a> Page<'a> {
    fn new(lang: &'a str) -> Self {
        Self {
            lang,
            title: translated(lang, "title"),
            own_label: translated(lang, "own"),
            resume_label: translated(lang, "resume"),
            cancel_label: translated(lang, "cancel"),
            sell_label: translated(lang, "sell"),
            kind_label: translated(lang, "kind"),
            character_label: translated(lang, "character"),
            item_label: translated(lang, "item"),
            world_label: translated(lang, "world"),
            asset_label: translated(lang, "asset"),
            revision_label: translated(lang, "revision"),
            quantity_label: translated(lang, "quantity"),
            definition_label: translated(lang, "definition"),
            version_label: translated(lang, "version"),
            currency_label: translated(lang, "currency"),
            price_label: translated(lang, "price"),
            days_label: translated(lang, "days"),
            consent_label: translated(lang, "consent"),
            refused_label: translated(lang, "refused"),
            cancelled_label: translated(lang, "cancelled"),
            complete_label: translated(lang, "complete"),
            active_label: translated(lang, "active"),
            pending_label: translated(lang, "pending"),
            destination_label: translated(lang, "destination"),
            maximum_label: translated(lang, "maximum"),
            bid_label: translated(lang, "bid"),
            message: String::new(),
            csrf: String::new(),
            request: oteryn_core::new_id(),
            create: false,
            detail: false,
            cards: vec![],
        }
    }
}
fn answer(page: Page<'_>, status: StatusCode) -> Response {
    match page.render() {
        Ok(body) => (
            status,
            [(header::CACHE_CONTROL, "no-store, private")],
            Html(body),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn failure(lang: &str, e: Error) -> Response {
    let mut page = Page::new(lang);
    page.message = translated(
        lang,
        if matches!(e, Error::Unavailable | Error::Database(_) | Error::Backlog) {
            "unavailable"
        } else {
            "denied"
        },
    );
    answer(
        page,
        match e {
            Error::Unavailable | Error::Database(_) | Error::Backlog => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Conflict => StatusCode::CONFLICT,
            _ => StatusCode::FORBIDDEN,
        },
    )
}
fn locale(lang: &str) -> Result<(), Error> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}
impl Runtime {
    pub fn browser_router(&self) -> Router {
        let mut sessions = SessionLayer::new(self.pool.clone());
        sessions.csrf.trusted_origins = vec![self.config.public_origin.clone()];
        Router::new()
            .route("/{locale}/bazaar", get(index))
            .route("/{locale}/bazaar/items", get(index))
            .route("/{locale}/account/bazaar", get(own))
            .route("/{locale}/account/bazaar/sell", get(new).post(create))
            .route("/{locale}/bazaar/items/new", get(new))
            .route("/{locale}/bazaar/{listing}", get(detail))
            .route("/{locale}/bazaar/items/{listing}", get(detail))
            .route("/{locale}/account/bazaar/{listing}/bids", post(bid))
            .route("/{locale}/account/bazaar/{listing}/control", post(control))
            .with_state(self.clone())
            .layer(axum::middleware::from_fn_with_state(
                oteryn_web::ratelimit::RateLimiter::new(self.pool.clone()),
                rate,
            ))
            .layer(axum::middleware::from_fn_with_state(
                sessions,
                oteryn_web::session::middleware,
            ))
    }
    async fn cards(
        &self,
        actor: Option<Uuid>,
        listing: Option<Uuid>,
        only_own: bool,
    ) -> Result<Vec<Card>, Error> {
        self.config.current()?;
        let mut tx = self.pool.begin().await?;
        let rows:Vec<Listing>=sqlx::query_as("SELECT l.* FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id WHERE ($1::uuid IS NULL OR l.id=$1) AND (NOT $2 OR l.owner=$3) ORDER BY l.created_at DESC LIMIT 50").bind(listing).bind(only_own).bind(actor).fetch_all(&mut *tx).await?;
        let mut cards = vec![];
        for row in rows {
            let owner = actor == Some(row.owner);
            let refused: bool = sqlx::query_scalar(
                "SELECT refusal_code IS NOT NULL FROM bazaar_authority_assets WHERE listing_id=$1",
            )
            .bind(row.id)
            .fetch_one(&mut *tx)
            .await?;
            let completed = row.state == State::Completed;
            let buyer: bool = if let Some(actor) = actor {
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_settlements WHERE listing_id=$1 AND buyer=$2 UNION ALL SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2)").bind(row.id).bind(actor).fetch_one(&mut *tx).await?
            } else {
                false
            };
            let eligible = row.state == State::Active
                && self
                    .authority()
                    .escrow_ready(&mut tx, &row)
                    .await
                    .map_err(core_error)?;
            if !eligible && !owner && !buyer {
                continue;
            }
            let name = if eligible && row.kind == Kind::Character {
                use crate::characters::CharacterEvidence;
                self.authority()
                    .approved(&mut tx, &row)
                    .await
                    .map_err(core_error)?
                    .ok_or(Error::Unavailable)?
                    .name
            } else if eligible {
                use crate::items::ItemAuthority;
                self.authority()
                    .publication(&mut tx, &row)
                    .await
                    .map_err(core_error)?
                    .ok_or(Error::Unavailable)?
                    .name
            } else {
                translated(
                    "en",
                    if row.kind == Kind::Character {
                        "character"
                    } else {
                        "item"
                    },
                )
            };
            cards.push(Card {
                id: row.id,
                name,
                kind: if row.kind == Kind::Character {
                    "character".into()
                } else {
                    "item".into()
                },
                state: format!("{:?}", row.state).to_ascii_lowercase(),
                price: row.price,
                currency: if row.currency == Currency::Gold {
                    "Gold"
                } else {
                    "Coins"
                }
                .into(),
                owner,
                refused,
                completed,
                cancelled: row.state == State::Cancelled,
            });
        }
        if listing.is_some() && cards.is_empty() {
            return Err(Error::NotFound);
        }
        tx.commit().await?;
        Ok(cards)
    }
}
async fn index(
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let mut page = Page::new(&lang);
    match runtime.cards(session.account_id().await, None, false).await {
        Ok(cards) => page.cards = cards,
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn own(
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let mut page = Page::new(&lang);
    match runtime.cards(Some(actor), None, true).await {
        Ok(cards) => page.cards = cards,
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn detail(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let mut page = Page::new(&lang);
    page.detail = true;
    page.csrf = session.csrf_token().await.unwrap_or_default();
    match runtime
        .cards(session.account_id().await, Some(id), false)
        .await
    {
        Ok(cards) => page.cards = cards,
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn new(Path(lang): Path<String>, Extension(session): Extension<BrowserSession>) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    if session.account_id().await.is_none() {
        return failure(&lang, Error::Denied);
    }
    let mut page = Page::new(&lang);
    page.create = true;
    page.csrf = session.csrf_token().await.unwrap_or_default();
    answer(page, StatusCode::OK)
}
async fn create(
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Extension(session): Extension<BrowserSession>,
    BrowserForm(input): BrowserForm<NewAuction>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let hash = match session_hash(&headers) {
        Ok(hash) => hash,
        Err(e) => return failure(&lang, e),
    };
    match runtime.create(actor, hash, input).await {
        Ok(id) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bid {
    request_id: Uuid,
    maximum: i64,
    buyer_character_id: Uuid,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn bid(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    BrowserForm(input): BrowserForm<Bid>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    match runtime
        .bid(
            actor,
            id,
            input.maximum,
            input.buyer_character_id,
            input.request_id,
        )
        .await
    {
        Ok(()) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}
fn session_hash(headers: &HeaderMap) -> Result<[u8; 32], Error> {
    let mut value = None;
    for header in headers.get_all(header::COOKIE) {
        for cookie in header.to_str().map_err(|_| Error::Denied)?.split(';') {
            if let Some((name, token)) = cookie.trim().split_once('=')
                && name == oteryn_web::session::COOKIE_NAME
            {
                if value.is_some()
                    || token.len() != 64
                    || !token.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Error::Denied);
                }
                value = Some(Sha256::digest(token.as_bytes()).into());
            }
        }
    }
    value.ok_or(Error::Denied)
}
async fn rate(
    HttpState(limiter): HttpState<oteryn_web::ratelimit::RateLimiter>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if request.method() != axum::http::Method::POST {
        return next.run(request).await;
    }
    let Some(session) = request.extensions().get::<BrowserSession>() else {
        return failure("en", Error::Denied);
    };
    let Some(actor) = session.account_id().await else {
        return failure("en", Error::Denied);
    };
    let Some(peer) = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    else {
        return failure("en", Error::Denied);
    };
    let mut keys = oteryn_web::ratelimit::Keys::source(peer.0.ip());
    keys.account = Some(actor.to_string());
    let policy = if request.uri().path().ends_with("/sell") {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceListing
    } else {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceBid
    };
    let lang = if request.uri().path().starts_with("/pl/") {
        oteryn_web::Language::Pl
    } else {
        oteryn_web::Language::En
    };
    match limiter.check(policy, &[keys]).await {
        Ok(()) => next.run(request).await,
        Err(denial) => denial.response(lang, false),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    request_id: Uuid,
    action: String,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn control(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    BrowserForm(input): BrowserForm<Control>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let hash = match session_hash(&headers) {
        Ok(h) => h,
        Err(e) => return failure(&lang, e),
    };
    match runtime
        .control(actor, hash, id, input.request_id, &input.action)
        .await
    {
        Ok(()) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}
