//! Account-scoped read-only premium, payment history and printable receipts.
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, RawQuery, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_store::account::{
    PREMIUM_PAGE_SIZE, PremiumEvent, PremiumHistoryQuery, premium_history,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use sqlx::{PgPool, Row};
use std::{future::Future, pin::Pin};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// Implemented only by server-side session adapters, never from request parameters.
pub trait CurrentAccount {
    fn current_account(&self) -> Pin<Box<dyn Future<Output = Option<Uuid>> + Send + '_>>;
}
impl CurrentAccount for BrowserSession {
    fn current_account(&self) -> Pin<Box<dyn Future<Output = Option<Uuid>> + Send + '_>> {
        Box::pin(self.account_id())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Flags {
    pub panel_premium_payments: bool,
}

/// Route construction keeps the feature absent by default, including receipts.
pub fn router(pool: PgPool, flags: Flags) -> Router {
    if !flags.panel_premium_payments {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/panel/premium", get(premium))
        .route("/{lang}/panel/payments", get(history))
        .route("/{lang}/panel/payments/receipts/{id}", get(receipt))
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(private_response))
        .with_state(pool)
}

pub fn configured_router(pool: PgPool) -> Router {
    router(
        pool,
        Flags {
            panel_premium_payments: std::env::var("PANEL_PREMIUM_PAYMENTS").as_deref()
                == Ok("true"),
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
            include_str!("../templates/account/pl.ftl")
        } else {
            include_str!("../templates/account/en.ftl")
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
            .and_then(|message| message.value())
            .map(|pattern| {
                bundle
                    .format_pattern(pattern, None, &mut vec![])
                    .into_owned()
            })
            .unwrap_or_default()
    }
}
async fn identity(session: &impl CurrentAccount, locale: Locale) -> Result<Uuid, Response> {
    session
        .current_account()
        .await
        .ok_or_else(|| Redirect::to(&format!("/{}/login", locale.0)).into_response())
}

#[derive(Template)]
#[template(path = "account/page.html")]
struct Page {
    lang: &'static str,
    title: String,
    premium_label: String,
    payments_label: String,
    status_label: String,
    amount_label: String,
    date_label: String,
    receipt_label: String,
    empty_label: String,
    previous_label: String,
    next_label: String,
    paging_label: String,
    print_label: String,
    payment_reference: String,
    payment_label: String,
    rows: Vec<Entry>,
    previous: Option<u32>,
    next: Option<u32>,
    printable: bool,
}
impl Page {
    fn new(locale: Locale, title: &str) -> Self {
        Self {
            lang: locale.0,
            title: locale.text(title),
            premium_label: locale.text("premium"),
            payments_label: locale.text("payments"),
            status_label: locale.text("status"),
            amount_label: locale.text("amount"),
            date_label: locale.text("date"),
            receipt_label: locale.text("receipt"),
            empty_label: locale.text("empty"),
            previous_label: locale.text("previous"),
            next_label: locale.text("next"),
            paging_label: locale.text("paging"),
            print_label: locale.text("print"),
            payment_reference: locale.text("payment-reference"),
            payment_label: locale.text("payment-entry"),
            rows: vec![],
            previous: None,
            next: None,
            printable: false,
        }
    }
}
async fn private_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (header::CACHE_CONTROL, "private, no-store"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
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
            .insert(name, header::HeaderValue::from_static(value));
    }
    response
}
fn render(page: &Page) -> Response {
    oteryn_web::render(page)
}
fn date(value: OffsetDateTime) -> Result<String, StatusCode> {
    value
        .format(&Rfc3339)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
#[derive(Template)]
#[template(path = "account/premium.html")]
struct PremiumPage {
    locale: Locale,
    status: String,
    available: bool,
    game_unavailable: String,
    end: Option<String>,
    events: Vec<PremiumRow>,
    receipt: Option<PurchaseReceipt>,
    previous: Option<u32>,
    next: Option<u32>,
}

struct PremiumRow {
    event: &'static str,
    date: String,
    end: String,
    revision: i64,
    days: Option<i32>,
    order: Option<Uuid>,
}

struct PurchaseReceipt {
    order: Uuid,
    date: String,
    coins: i64,
}

impl TryFrom<PremiumEvent> for PremiumRow {
    type Error = StatusCode;

    fn try_from(event: PremiumEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event: if event.entitlement.revoked {
                "revocation"
            } else {
                "grant"
            },
            date: date(event.recorded_at)?,
            end: end_date(event.entitlement.until)?,
            revision: event.entitlement.revision,
            days: event.days,
            order: event.purchase.map(|purchase| purchase.order),
        })
    }
}

fn end_date(until: i64) -> Result<String, StatusCode> {
    date(OffsetDateTime::from_unix_timestamp(until).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?)
}

fn premium_query(query: Option<&str>) -> Result<PremiumHistoryQuery, StatusCode> {
    if let Some(id) = query.and_then(|query| query.strip_prefix("receipt=")) {
        return Uuid::parse_str(id)
            .ok()
            .filter(|uuid| uuid.to_string() == id)
            .map(PremiumHistoryQuery::Receipt)
            .ok_or(StatusCode::BAD_REQUEST);
    }
    paging(query).map(PremiumHistoryQuery::Page)
}

async fn premium(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    let account = match identity(&session, locale).await {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let query = premium_query(query.as_deref())?;
    let mut page = PremiumPage {
        locale,
        status: oteryn_ui::text(locale.0, "error-503-title"),
        available: false,
        game_unavailable: oteryn_ui::text(locale.0, "online-unavailable"),
        end: None,
        events: Vec::new(),
        receipt: None,
        previous: None,
        next: None,
    };
    let history = match premium_history(&pool, account, query).await {
        Ok(history) => history,
        Err(_) => {
            return Ok((StatusCode::SERVICE_UNAVAILABLE, oteryn_web::render(&page)).into_response());
        }
    };
    page.available = true;
    let mut events = history.events;
    match query {
        PremiumHistoryQuery::Page(number) => {
            page.status = locale.text(history.current.as_ref().map_or("NONE", |entitlement| {
                entitlement.classify(OffsetDateTime::now_utc().unix_timestamp())
            }));
            page.end = history
                .current
                .map(|entitlement| end_date(entitlement.until))
                .transpose()?;
            if events.len() > PREMIUM_PAGE_SIZE {
                events.truncate(PREMIUM_PAGE_SIZE);
                page.next = next_page(number);
            }
            page.previous = previous_page(number);
        }
        PremiumHistoryQuery::Receipt(_) => {
            let purchase = events
                .first()
                .and_then(|event| event.purchase.as_ref())
                .ok_or(StatusCode::NOT_FOUND)?;
            page.receipt = Some(PurchaseReceipt {
                order: purchase.order,
                date: date(purchase.purchased_at)?,
                coins: purchase.coins,
            });
        }
    }
    page.events = events
        .into_iter()
        .map(PremiumRow::try_from)
        .collect::<Result<_, _>>()?;
    Ok(oteryn_web::render(&page))
}

/// Strict, bounded pages avoid overflow, duplicate parameters and expensive offsets.
const MAX_PAGE: u32 = 10_000;

fn next_page(page: u32) -> Option<u32> {
    (page < MAX_PAGE).then_some(page + 1)
}

fn previous_page(page: u32) -> Option<u32> {
    (page > 1).then_some(page - 1)
}

fn paging(query: Option<&str>) -> Result<u32, StatusCode> {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return Ok(1);
    };
    let value = query.strip_prefix("page=").ok_or(StatusCode::BAD_REQUEST)?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let page: u32 = value.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    if !(1..=MAX_PAGE).contains(&page) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(page)
}
#[derive(Debug)]
struct Entry {
    id: Uuid,
    amount: String,
    status: String,
    date: String,
    receipt: Option<Uuid>,
    payment: Option<Uuid>,
}
async fn entries(
    pool: &PgPool,
    account: Uuid,
    page: u32,
    locale: Locale,
) -> Result<Vec<Entry>, sqlx::Error> {
    let rows = sqlx::query("SELECT p.id,p.amount,p.currency,p.state,p.created_at,i.id AS receipt FROM payment_intents p LEFT JOIN invoices i ON i.intent_id=p.id AND i.account_id=p.account_id AND i.amount=p.amount AND i.currency=p.currency AND p.state IN ('succeeded','refunded','chargeback') WHERE p.account_id=$1 ORDER BY p.created_at DESC,p.id DESC LIMIT 21 OFFSET $2")
        .bind(account).bind(i64::from(page - 1) * 20).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(Entry {
                id: row.try_get("id")?,
                amount: format!(
                    "{} {} {}",
                    row.try_get::<i64, _>("amount")?,
                    row.try_get::<String, _>("currency")?,
                    locale.text("minor")
                ),
                status: locale.text(&row.try_get::<String, _>("state")?),
                date: row.try_get::<OffsetDateTime, _>("created_at")?.to_string(),
                receipt: row.try_get("receipt")?,
                payment: None,
            })
        })
        .collect()
}
async fn history(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    let account = match identity(&session, locale).await {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let number = paging(query.as_deref())?;
    let mut page = Page::new(locale, "payments");
    page.rows = entries(&pool, account, number, locale)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if page.rows.len() > 20 {
        page.rows.truncate(20);
        page.next = next_page(number);
    }
    page.previous = previous_page(number);
    Ok(render(&page))
}
async fn receipt_entry(
    pool: &PgPool,
    account: Uuid,
    id: Uuid,
    locale: Locale,
) -> Result<Option<Entry>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT i.id,i.intent_id,i.amount,i.currency,i.created_at,p.state FROM invoices i
         JOIN payment_intents p ON p.id=i.intent_id AND p.account_id=i.account_id
           AND p.amount=i.amount AND p.currency=i.currency
         WHERE i.id=$1 AND i.account_id=$2 AND p.state IN ('succeeded','refunded','chargeback')",
    )
    .bind(id)
    .bind(account)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(Entry {
            id: row.try_get("id")?,
            amount: format!(
                "{} {} {}",
                row.try_get::<i64, _>("amount")?,
                row.try_get::<String, _>("currency")?,
                locale.text("minor")
            ),
            status: locale.text(&row.try_get::<String, _>("state")?),
            date: row.try_get::<OffsetDateTime, _>("created_at")?.to_string(),
            receipt: None,
            payment: Some(row.try_get("intent_id")?),
        })
    })
    .transpose()
}
async fn receipt(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, id)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    let account = match identity(&session, locale).await {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let id = Uuid::parse_str(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    let entry = receipt_entry(&pool, account, id, locale)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut page = Page::new(locale, "receipt");
    page.printable = true;
    page.rows.push(entry);
    Ok(render(&page))
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
