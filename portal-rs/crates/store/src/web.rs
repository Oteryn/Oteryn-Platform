//! Default-off, private SSR shop. Browser state never supplies the acting account.
use std::{future::Future, pin::Pin, sync::Arc};

use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, RawQuery, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "web_backend.rs"]
mod backend;
pub use backend::{Access, Backend, Buy, Domain, TopUps, configured as configured_backend};

pub type FutureResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Conflict,
    Unavailable,
    Limited(u32),
}
impl From<crate::purchase::Error> for Error {
    fn from(error: crate::purchase::Error) -> Self {
        use crate::purchase::Error as E;
        match error {
            E::Invalid => Self::Invalid,
            E::Conflict => Self::Conflict,
            E::Limited(n) => Self::Limited(n),
            E::Unauthorized | E::UnknownAccount => Self::Denied,
            E::Unavailable => Self::Unavailable,
        }
    }
}
impl From<oteryn_wallet::Error> for Error {
    fn from(error: oteryn_wallet::Error) -> Self {
        use oteryn_wallet::{Error as E, Refusal as R};
        match error {
            E::Refused(
                R::InvalidAmount | R::InvalidIdentifier | R::SameAccount | R::InvalidPage,
            ) => Self::Invalid,
            E::Refused(R::IdempotencyConflict) => Self::Conflict,
            E::Storage(_) | E::Refused(R::Disabled | R::TransferStateUnknown) => Self::Unavailable,
            E::Refused(_) => Self::Denied,
        }
    }
}

/// All flags are false by default. Enabling the surface grants no domain permission.
#[derive(Clone, Copy, Debug, Default)]
pub struct Flags {
    pub shop: bool,
    pub purchases: bool,
    pub transfers: bool,
    pub top_ups: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub key: String,
    pub coins: i64,
    pub minor_units: i64,
    pub currency: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseForm {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    pub operation: Uuid,
    pub character: Uuid,
    pub world: Uuid,
    pub revision: String,
    pub offer: String,
    pub price: String,
    pub quantity: u8,
}
impl std::fmt::Debug for PurchaseForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PurchaseForm")
            .field("operation", &self.operation)
            .finish_non_exhaustive()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    operation: Uuid,
    recipient: Uuid,
    amount: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TopUpForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    operation: Uuid,
    package: String,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Paging {
    before: Option<Uuid>,
}
#[derive(Clone)]
struct Shop {
    pool: PgPool,
    backend: Arc<dyn Backend>,
    flags: Flags,
}

/// Explicit backend injection is required for any authorized mutation. The default
/// configured composition offers reads and refuses missing policy/provider adapters.
pub fn router(
    pool: PgPool,
    flags: Flags,
    backend: Arc<dyn Backend>,
    origins: Vec<String>,
) -> Router {
    if !flags.shop {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = origins;
    let mut routes = Router::new();
    if flags.top_ups && backend.test_checkout() {
        routes = routes.route(
            "/{lang}/shop/test-checkout/{intent}",
            get(checkout_page).post(checkout_post),
        );
    }
    routes
        .route("/{lang}/shop", get(offers).post(buy))
        .route("/{lang}/shop/coins", get(coins).post(top_up))
        .route("/{lang}/shop/history", get(history))
        .route("/{lang}/shop/transfer", get(transfer_page).post(transfer))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(Shop {
            pool,
            backend,
            flags,
        })
}
pub fn configured_router(pool: PgPool) -> Router {
    configured_router_with_topups(pool, None)
}

/// The binary supplies T31 here, avoiding a store/payments dependency cycle.
pub fn configured_router_with_topups(pool: PgPool, topups: Option<Arc<dyn TopUps>>) -> Router {
    let enabled = |name| std::env::var(name).as_deref() == Ok("true");
    let flags = Flags {
        shop: enabled("OTERYN_SHOP_ENABLED"),
        top_ups: topups.is_some() && enabled("OTERYN_SHOP_TOP_UPS_ENABLED"),
        purchases: false, // Authoritative character ownership is not available in the portal DB.
        transfers: std::env::var("OTERYN_ENV").as_deref() == Ok("testing")
            && enabled("OTERYN_SHOP_TEST_PAYMENTS_ENABLED")
            && enabled("OTERYN_SHOP_TRANSFERS_ENABLED"),
    };
    let origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    router(
        pool.clone(),
        flags,
        backend::configured(pool, topups),
        origins,
    )
}
async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, header::HeaderValue::from_static("default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response
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
            include_str!("web_pl.ftl")
        } else {
            include_str!("web_en.ftl")
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
            .map(|p| bundle.format_pattern(p, None, &mut Vec::new()).into_owned())
            .unwrap_or_default()
    }
}
async fn account(shop: &Shop, session: &BrowserSession, locale: Locale) -> Result<Uuid, Response> {
    let Some(account) = session.account_id().await else {
        return Err(Redirect::to(&format!("/{}/login", locale.0)).into_response());
    };
    match shop.backend.active(account).await {
        Ok(true) => Ok(account),
        Ok(false) => Err(failure(locale, Error::Denied)),
        Err(e) => Err(failure(locale, e)),
    }
}
fn failure(locale: Locale, error: Error) -> Response {
    let (status, key) = match error {
        Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::Denied => (StatusCode::FORBIDDEN, "denied"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        Error::Limited(_) => (StatusCode::TOO_MANY_REQUESTS, "limited"),
    };
    let mut page = Page::new(locale, "shop");
    page.message = locale.text(key);
    let mut response = (status, oteryn_web::render(&page)).into_response();
    if let Error::Limited(seconds) = error
        && let Ok(value) = seconds.max(1).to_string().parse()
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}
fn uuid7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn amount(value: &str) -> Result<i64, Error> {
    if value.starts_with('0') || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid);
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or(Error::Invalid)
}

struct OfferView {
    offer_key: String,
    unit_price: String,
    max_quantity: u8,
    operation: Uuid,
}
struct PackageView {
    key: String,
    coins: i64,
    minor_units: i64,
    currency: String,
    operation: Uuid,
}

#[derive(Template)]
#[template(
    source = r##"<!doctype html>
<html lang="{{ lang }}" data-theme="dark">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ title }} · Oteryn</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head>
<body class="op-theme op-stack">
<header class="op-card op-stack"><a href="#main">{{ skip }}</a><nav class="op-tabs" aria-label="{{ shop }}"><a href="/{{ lang }}/shop">{{ shop }}</a><a href="/{{ lang }}/shop/coins">{{ coins }}</a><a href="/{{ lang }}/shop/history">{{ history }}</a><a href="/{{ lang }}/shop/transfer">{{ transfer }}</a></nav></header>
<main id="main" tabindex="-1" class="op-card op-stack"><h1>{{ title }}</h1>
{% if !message.is_empty() %}<p class="op-alert" role="status">{{ message }}</p>{% endif %}
{% if let Some(b) = balance %}<section class="op-panel"><dl><dt>{{ available }}</dt><dd>{{ b.available }}</dd><dt>{{ transferable }}</dt><dd>{{ b.transferable }}</dd><dt>{{ reserved }}</dt><dd>{{ b.reserved }}</dd></dl></section>{% endif %}
{% if section == "checkout" %}
<p class="op-alert">{{ test_banner }}</p>
{% if let Some(c) = checkout %}<dl><dt>{{ package_label }}</dt><dd>{{ c.package.key }}</dd><dt>{{ coins }}</dt><dd>{{ c.package.coins }}</dd><dt>{{ amount }} ({{ minor_units }})</dt><dd>{{ c.package.minor_units }} {{ c.package.currency }}</dd></dl>
{% if enabled %}<form method="post" class="op-stack" action="/{{ lang }}/shop/test-checkout/{{ operation }}"><input type="hidden" name="_csrf" value="{{ csrf }}"><button class="op-button" name="action" value="pay">{{ test_pay }}</button><button class="op-button" name="action" value="reject">{{ test_reject }}</button><button class="op-button" name="action" value="cancel">{{ test_cancel }}</button></form>{% endif %}{% endif %}
<a href="/{{ lang }}/shop/coins">{{ coins }}</a>
{% else if section == "shop" %}
{% for offer in offers %}<article class="op-card op-stack"><h2>{{ offer.offer_key }}</h2><p>{{ price }}: {{ offer.unit_price }}</p>
{% if enabled %}<form method="post" class="op-stack" action="/{{ lang }}/shop"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="operation" value="{{ offer.operation }}"><input type="hidden" name="revision" value="{{ revision }}"><input type="hidden" name="offer" value="{{ offer.offer_key }}"><input type="hidden" name="price" value="{{ offer.unit_price }}"><label class="op-field">{{ quantity }}<input class="op-input" type="number" name="quantity" min="1" max="{{ offer.max_quantity }}" value="1" required></label><label class="op-field">{{ character }}<input class="op-input" name="character" required maxlength="36"></label><label class="op-field">{{ world }}<input class="op-input" name="world" required maxlength="36"></label><p>{{ confirm }}</p><button class="op-button">{{ submit }}</button></form>{% else %}<p>{{ unavailable }}</p>{% endif %}</article>{% else %}<p>{{ empty }}</p>{% endfor %}
{% else if section == "transfer" %}
{% if enabled %}<form method="post" class="op-stack" action="/{{ lang }}/shop/transfer"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="operation" value="{{ operation }}"><label class="op-field">{{ recipient }}<input class="op-input" name="recipient" maxlength="36" required></label><label class="op-field">{{ amount }}<input class="op-input" name="amount" inputmode="numeric" pattern="[1-9][0-9]*" required></label><p>{{ confirm }}</p><button class="op-button">{{ submit }}</button></form>{% else %}<p>{{ unavailable }}</p>{% endif %}
{% else if section == "coins" %}
{% if enabled %}{% for package in packages %}<form method="post" class="op-card op-stack" action="/{{ lang }}/shop/coins"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="operation" value="{{ package.operation }}"><input type="hidden" name="package" value="{{ package.key }}"><h2>{{ package.coins }} {{ available }}</h2><p>{{ package.minor_units }} {{ package.currency }} ({{ minor_units }})</p><p>{{ confirm }}</p><button class="op-button">{{ submit }}</button></form>{% else %}<p>{{ unavailable }}</p>{% endfor %}{% else %}<p>{{ unavailable }}</p>{% endif %}
{% else if section == "history" %}
{% for entry in entries %}<article class="op-card op-stack"><h2>{{ operation_label }}</h2><p>{{ entry.operation_id }}</p><p>{{ date }}: {{ entry.created_at }}</p><p>{{ delta }}: {{ entry.available_delta }} / {{ entry.reserved_delta }}</p></article>{% else %}<p>{{ empty }}</p>{% endfor %}
{% if let Some(id) = before %}<a class="op-button" href="/{{ lang }}/shop/history?before={{ id }}">{{ next }}</a>{% endif %}
{% endif %}
</main></body></html>
"##,
    ext = "html"
)]
struct Page {
    checkout: Option<Checkout>,
    test_banner: String,
    test_pay: String,
    test_reject: String,
    test_cancel: String,
    package_label: String,
    lang: &'static str,
    title: String,
    shop: String,
    coins: String,
    history: String,
    transfer: String,
    skip: String,
    available: String,
    reserved: String,
    transferable: String,
    price: String,
    quantity: String,
    character: String,
    world: String,
    recipient: String,
    amount: String,
    submit: String,
    unavailable: String,
    empty: String,
    next: String,
    date: String,
    delta: String,
    operation_label: String,
    confirm: String,
    minor_units: String,
    message: String,
    section: &'static str,
    balance: Option<oteryn_wallet::Balance>,
    offers: Vec<OfferView>,
    revision: String,
    entries: Vec<oteryn_wallet::HistoryEntry>,
    packages: Vec<PackageView>,
    before: Option<Uuid>,
    csrf: String,
    operation: Uuid,
    enabled: bool,
}
impl Page {
    fn new(l: Locale, section: &'static str) -> Self {
        Self {
            checkout: None,
            test_banner: l.text("test-banner"),
            test_pay: l.text("test-pay"),
            test_reject: l.text("test-reject"),
            test_cancel: l.text("test-cancel"),
            package_label: l.text("package"),
            lang: l.0,
            title: l.text(section),
            shop: l.text("shop"),
            coins: l.text("coins"),
            history: l.text("history"),
            transfer: l.text("transfer"),
            skip: l.text("skip"),
            available: l.text("available"),
            reserved: l.text("reserved"),
            transferable: l.text("transferable"),
            price: l.text("price"),
            quantity: l.text("quantity"),
            character: l.text("character"),
            world: l.text("world"),
            recipient: l.text("recipient"),
            amount: l.text("amount"),
            submit: l.text("submit"),
            unavailable: l.text("unavailable"),
            empty: l.text("empty"),
            next: l.text("next"),
            date: l.text("date"),
            delta: l.text("delta"),
            operation_label: l.text("operation"),
            confirm: l.text("confirm"),
            minor_units: l.text("minor-units"),
            message: String::new(),
            section,
            balance: None,
            offers: Vec::new(),
            revision: String::new(),
            entries: Vec::new(),
            packages: Vec::new(),
            before: None,
            csrf: String::new(),
            operation: Uuid::now_v7(),
            enabled: false,
        }
    }
}
async fn page(
    shop: &Shop,
    session: &BrowserSession,
    lang: &str,
    section: &'static str,
) -> Result<(Uuid, Locale, Page), Response> {
    let locale = Locale::parse(lang).map_err(IntoResponse::into_response)?;
    let account = account(shop, session, locale).await?;
    let mut page = Page::new(locale, section);
    page.balance = Some(
        shop.backend
            .balance(account)
            .await
            .map_err(|e| failure(locale, e))?,
    );
    page.csrf = session
        .csrf_token()
        .await
        .map_err(|_| failure(locale, Error::Unavailable))?;
    Ok((account, locale, page))
}
async fn offers(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Response {
    let (_, l, mut p) = match page(&s, &session, &lang, "shop").await {
        Ok(p) => p,
        Err(r) => return r,
    };
    match s.backend.catalog().await {
        Ok(Some(c)) => {
            p.revision = c.revision().into();
            p.offers = c
                .offers()
                .iter()
                .filter(|o| o.available())
                .map(|o| OfferView {
                    offer_key: o.offer_key.clone(),
                    unit_price: o.unit_price.clone(),
                    max_quantity: o.max_quantity,
                    operation: Uuid::now_v7(),
                })
                .collect();
        }
        Ok(None) => p.message = l.text("unavailable"),
        Err(e) => return failure(l, e),
    }
    p.enabled = s.flags.purchases;
    oteryn_web::render(&p)
}
async fn coins(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    let (account, locale, mut p) = match page(&s, &session, &lang, "coins").await {
        Ok(p) => p,
        Err(r) => return r,
    };
    if s.backend.test_checkout() {
        let status = match serde_urlencoded::from_str::<CheckoutStatus>(
            query.as_deref().unwrap_or_default(),
        ) {
            Ok(status) => status,
            Err(_) => return failure(locale, Error::Invalid),
        };
        if let Some(id) = status.checkout {
            match s.backend.checkout(account, id, None).await {
                Ok(Some(checkout)) => p.message = locale.text(&checkout.status),
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(e) => return failure(locale, e),
            }
        }
    }
    p.packages = s
        .backend
        .packages()
        .into_iter()
        .filter(|p| {
            p.coins > 0
                && p.minor_units > 0
                && p.currency.len() == 3
                && p.currency.bytes().all(|b| b.is_ascii_uppercase())
                && !p.key.is_empty()
        })
        .map(|p| PackageView {
            key: p.key,
            coins: p.coins,
            minor_units: p.minor_units,
            currency: p.currency,
            operation: Uuid::now_v7(),
        })
        .collect();
    p.enabled = s.flags.top_ups;
    oteryn_web::render(&p)
}
async fn transfer_page(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Response {
    let (_, _, mut p) = match page(&s, &session, &lang, "transfer").await {
        Ok(p) => p,
        Err(r) => return r,
    };
    p.enabled = s.flags.transfers;
    oteryn_web::render(&p)
}
async fn history(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    query: Result<Query<Paging>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let (a, l, mut p) = match page(&s, &session, &lang, "history").await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Ok(Query(query)) = query else {
        return failure(l, Error::Invalid);
    };
    if query.before.is_some_and(|id| !uuid7(id)) {
        return failure(l, Error::Invalid);
    }
    p.entries = match s.backend.history(a, query.before).await {
        Ok(e) => e,
        Err(e) => return failure(l, e),
    };
    if p.entries.len() == 25 {
        p.before = p.entries.last().map(|e| e.id);
    }
    oteryn_web::render(&p)
}
async fn mutation(
    s: &Shop,
    session: &BrowserSession,
    lang: &str,
    enabled: bool,
) -> Result<(Uuid, Locale), Response> {
    let l = Locale::parse(lang).map_err(IntoResponse::into_response)?;
    if !enabled {
        return Err(failure(l, Error::Unavailable));
    }
    let a = account(s, session, l).await?;
    // Shared PostgreSQL limiter, keyed by the authenticated account, independent of rollback.
    let attempts: i32 = sqlx::query_scalar("INSERT INTO store_wire_limits(scope,subject,attempts) VALUES('purchase',$1,1) ON CONFLICT(scope,subject) DO UPDATE SET window_start=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN clock_timestamp() ELSE store_wire_limits.window_start END, attempts=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN 1 ELSE LEAST(store_wire_limits.attempts+1,11) END, updated_at=clock_timestamp() RETURNING attempts")
        .bind(format!("web:{a}")).fetch_one(&s.pool).await.map_err(|_| failure(l, Error::Unavailable))?;
    if attempts > 10 {
        return Err(failure(l, Error::Limited(60)));
    }
    Ok((a, l))
}
async fn parse<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Error> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|v| v.split(';').next() != Some("application/x-www-form-urlencoded"))
    {
        return Err(Error::Invalid);
    }
    let body = axum::body::to_bytes(request.into_body(), 8192)
        .await
        .map_err(|_| Error::Invalid)?;
    serde_urlencoded::from_bytes(&body).map_err(|_| Error::Invalid)
}
async fn transfer(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    request: Request,
) -> Response {
    let (a, l) = match mutation(&s, &session, &lang, s.flags.transfers).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let form: TransferForm = match parse(request).await {
        Ok(v) => v,
        Err(e) => return failure(l, e),
    };
    let amount = match amount(&form.amount) {
        Ok(n) => n,
        Err(e) => return failure(l, e),
    };
    if !uuid7(form.operation) || !uuid7(form.recipient) || form.recipient == a {
        return failure(l, Error::Invalid);
    }
    match s
        .backend
        .transfer(a, form.operation, form.recipient, amount)
        .await
    {
        Ok(_) => Redirect::to(&format!("/{}/shop/history", l.0)).into_response(),
        Err(e) => failure(l, e),
    }
}
async fn top_up(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    request: Request,
) -> Response {
    let (a, l) = match mutation(&s, &session, &lang, s.flags.top_ups).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let form: TopUpForm = match parse(request).await {
        Ok(v) => v,
        Err(e) => return failure(l, e),
    };
    if !uuid7(form.operation)
        || !s
            .backend
            .packages()
            .iter()
            .any(|p| p.key == form.package && p.coins > 0 && p.minor_units > 0)
    {
        return failure(l, Error::Invalid);
    }
    match s.backend.top_up(a, form.operation, form.package).await {
        Ok(id) => {
            if s.backend.test_checkout() {
                return Redirect::to(&format!("/{}/shop/test-checkout/{id}", l.0)).into_response();
            }
            let mut p = Page::new(l, "coins");
            p.message = format!("{} {id}", l.text("pending"));
            oteryn_web::render(&p)
        }
        Err(e) => failure(l, e),
    }
}
async fn buy(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    request: Request,
) -> Response {
    let (a, l) = match mutation(&s, &session, &lang, s.flags.purchases).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let form: PurchaseForm = match parse(request).await {
        Ok(v) => v,
        Err(e) => return failure(l, e),
    };
    if !uuid7(form.operation)
        || !uuid7(form.character)
        || !uuid7(form.world)
        || !(1..=100).contains(&form.quantity)
        || amount(&form.price).is_err()
    {
        return failure(l, Error::Invalid);
    }
    match s.backend.purchase(a, form).await {
        Ok(()) => Redirect::to(&format!("/{}/shop/history", l.0)).into_response(),
        Err(e) => failure(l, e),
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;

#[path = "web_checkout.rs"]
mod checkout;
pub use checkout::Checkout;
use checkout::{CheckoutStatus, checkout_page, checkout_post};
