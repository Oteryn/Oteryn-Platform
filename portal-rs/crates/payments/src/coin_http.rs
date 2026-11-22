//! Routes of OTERYN_COIN_PURCHASE_V1 and the hosted sandbox checkout page.
//! Without a configured service (the default) every route answers 503.
use std::sync::Arc;

use askama::Template;
use axum::{
    Extension, Form, Json, Router,
    body::{Body, to_bytes},
    extract::{Path, RawQuery, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{any, get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_ui::Chrome;
use oteryn_web::session::{BrowserSession, SessionKey, SessionLayer};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::coin::{
    CHECKOUT_PATH, CoinOrderRequest, MAX_REQUEST_BYTES, OrderState, canonical_id, checkout_path,
};
use crate::coin_store::{CoinError, CoinPurchases};

const API: &str = "/api/v1/native-account/coin-purchases";
/// The session's post-login return target, shared with the OAuth authorize flow.
const RETURN_KEY: SessionKey<Option<String>> = SessionKey::new("oauth_return");
type Service = Option<Arc<CoinPurchases>>;

pub fn router(pool: PgPool, service: Service) -> Router {
    let api = Router::new()
        .route(&format!("{API}/catalogue"), get(catalogue))
        .route(&format!("{API}/balance"), get(balance))
        .route(&format!("{API}/orders"), post(create))
        .route(&format!("{API}/requests/{{request_id}}"), get(status))
        .with_state(service.clone());
    let path = format!("{CHECKOUT_PATH}{{order_id}}");
    let page = match service {
        Some(service) => Router::new()
            .route(&path, get(page).post(act))
            .route_layer(middleware::from_fn_with_state(
                SessionLayer::new(pool),
                oteryn_web::session::middleware,
            ))
            .with_state(service),
        None => Router::new().route(&path, any(unavailable)),
    };
    api.merge(page).layer(middleware::from_fn(private))
}

async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
        .headers_mut()
        .entry(header::REFERRER_POLICY)
        .or_insert(HeaderValue::from_static("no-referrer"));
    response
}

async fn unavailable() -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

fn refusal(error: CoinError) -> Response {
    let status = match error {
        CoinError::Invalid => StatusCode::BAD_REQUEST,
        CoinError::Unauthorized => {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer")],
            )
                .into_response();
        }
        CoinError::Forbidden => StatusCode::FORBIDDEN,
        CoinError::NotFound => StatusCode::NOT_FOUND,
        CoinError::Conflict => StatusCode::CONFLICT,
        CoinError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    status.into_response()
}
fn reply<T: Serialize>(result: Result<T, CoinError>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => refusal(error),
    }
}

/// Exactly one `Authorization: Bearer <token>` header.
async fn payer(
    service: &Service,
    headers: &HeaderMap,
) -> Result<(Arc<CoinPurchases>, Uuid), CoinError> {
    let service = service.clone().ok_or(CoinError::Unavailable)?;
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next().ok_or(CoinError::Unauthorized)?;
    if values.next().is_some() {
        return Err(CoinError::Unauthorized);
    }
    let token = value
        .to_str()
        .ok()
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(CoinError::Unauthorized)?;
    let payer = service.payer(token).await?;
    Ok((service, payer))
}

/// Absent query, or exactly `character_id=<canonical UUIDv7>`.
fn character_query(query: Option<&str>) -> Result<Option<Uuid>, CoinError> {
    match query {
        None | Some("") => Ok(None),
        Some(query) => query
            .strip_prefix("character_id=")
            .and_then(canonical_id)
            .map(Some)
            .ok_or(CoinError::Invalid),
    }
}

async fn catalogue(
    State(service): State<Service>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    reply(
        async {
            let (service, payer) = payer(&service, &headers).await?;
            let character = character_query(query.as_deref())?;
            service.catalogue(payer, character).await
        }
        .await,
    )
}

async fn balance(
    State(service): State<Service>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    reply(
        async {
            let (service, payer) = payer(&service, &headers).await?;
            if query.is_some_and(|query| !query.is_empty()) {
                return Err(CoinError::Invalid);
            }
            service.balance(payer).await
        }
        .await,
    )
}

async fn create(State(service): State<Service>, headers: HeaderMap, body: Body) -> Response {
    reply(
        async {
            let (service, payer) = payer(&service, &headers).await?;
            let body = to_bytes(body, MAX_REQUEST_BYTES)
                .await
                .map_err(|_| CoinError::Invalid)?;
            let request = CoinOrderRequest::decode(&body).ok_or(CoinError::Invalid)?;
            service.create_order(payer, &request).await
        }
        .await,
    )
}

async fn status(
    State(service): State<Service>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    reply(
        async {
            let (service, payer) = payer(&service, &headers).await?;
            let request = canonical_id(&request_id).ok_or(CoinError::Invalid)?;
            service.order(payer, request).await
        }
        .await,
    )
}

#[derive(Template)]
#[template(path = "coin/checkout.html")]
struct CheckoutPage {
    lang: String,
    labels: FluentBundle<FluentResource>,
    csrf: String,
    order_id: Uuid,
    coins: i64,
    price: String,
    state: OrderState,
}

impl CheckoutPage {
    fn labels(lang: &str) -> FluentBundle<FluentResource> {
        let source = if lang == "en" {
            include_str!("../templates/coin/en.ftl")
        } else {
            include_str!("../templates/coin/pl.ftl")
        };
        let mut bundle = FluentBundle::new(vec![lang.parse().unwrap_or_default()]);
        if let Ok(resource) = FluentResource::try_new(source.to_owned()) {
            let _ = bundle.add_resource(resource);
        }
        bundle
    }

    fn text(&self, key: &str) -> String {
        self.labels
            .get_message(&format!("coin-checkout-{key}"))
            .and_then(|message| message.value())
            .map(|value| {
                self.labels
                    .format_pattern(value, None, &mut Vec::new())
                    .into_owned()
            })
            .unwrap_or_default()
    }

    fn awaiting(&self) -> bool {
        self.state == OrderState::AwaitingPayment
    }

    fn response(&self) -> Response {
        let mut response = oteryn_web::render(self);
        if response.status() == StatusCode::OK {
            // Keep the Origin header on same-origin form submissions for CSRF validation.
            response.headers_mut().insert(
                header::REFERRER_POLICY,
                HeaderValue::from_static("same-origin"),
            );
        }
        response
    }
}

#[derive(Deserialize)]
struct Decision {
    action: String,
}

async fn page(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(order): Path<String>,
    chrome: Option<Extension<Chrome>>,
) -> Response {
    let chrome = chrome.map(|Extension(chrome)| chrome).unwrap_or_default();
    let Some(order) = canonical_id(&order) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(account) = session.account_id().await else {
        // Return here after login. The stored target is a same-origin path built
        // from the canonical order ID only; the login hook accepts nothing else.
        let target = Some(checkout_path(order));
        if session.set(RETURN_KEY, &target).await.is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        return Redirect::to("/pl/login").into_response();
    };
    let order = match service.checkout(account, order).await {
        Ok(order) => order,
        Err(error) => return refusal(error),
    };
    let Ok(csrf) = session.csrf_token().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let price = order.price.display_for_locale(&chrome.lang);
    CheckoutPage {
        labels: CheckoutPage::labels(&chrome.lang),
        lang: chrome.lang,
        csrf,
        order_id: order.order_id,
        coins: order.coins,
        price,
        state: order.state,
    }
    .response()
}

/// CSRF is verified by the session layer before this runs.
async fn act(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(order): Path<String>,
    chrome: Option<Extension<Chrome>>,
    Form(decision): Form<Decision>,
) -> Response {
    let chrome = chrome.map(|Extension(chrome)| chrome).unwrap_or_default();
    let Some(account) = session.account_id().await else {
        return Redirect::to("/pl/login").into_response();
    };
    let Some(order) = canonical_id(&order) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let paid = match decision.action.as_str() {
        "pay" => true,
        "fail" => false,
        _ => return oteryn_ui::error(&chrome, StatusCode::BAD_REQUEST),
    };
    match service.sandbox_complete(account, order, paid).await {
        Ok(_) => Redirect::to(&format!("{CHECKOUT_PATH}{order}")).into_response(),
        Err(error) => refusal(error),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn checkout_uses_the_requested_locale_for_copy_and_order_state() {
        for (lang, title, status, pay, price) in [
            (
                "pl",
                "Zakup monet w trybie testowym",
                "Oczekuje na płatność",
                "Zakończ płatność testową",
                "10,00 PLN",
            ),
            (
                "en",
                "Sandbox coin checkout",
                "Awaiting payment",
                "Complete test payment",
                "10.00 PLN",
            ),
        ] {
            let page = CheckoutPage {
                lang: lang.into(),
                labels: CheckoutPage::labels(lang),
                csrf: String::new(),
                order_id: Uuid::nil(),
                coins: 12,
                price: crate::coin::Price {
                    minor_units: 1000,
                    currency: "PLN".into(),
                    exponent: 2,
                }
                .display_for_locale(lang),
                state: OrderState::AwaitingPayment,
            };
            assert_eq!(
                page.response().headers()[header::REFERRER_POLICY],
                "same-origin"
            );
            let html = page.render().unwrap();
            assert!(html.contains(&format!("<html lang=\"{lang}\">")));
            assert!(html.contains(&format!("<title>{title} · Oteryn</title>")));
            assert!(html.contains(&format!("<dd>{status}</dd>")));
            assert!(html.contains(&format!("<dd>{price}</dd>")));
            assert!(html.contains(pay));
            for state in [
                OrderState::AwaitingPayment,
                OrderState::Processing,
                OrderState::Paid,
                OrderState::Failed,
                OrderState::Refunded,
                OrderState::Chargeback,
            ] {
                assert!(
                    !page.text(state.as_str()).is_empty(),
                    "{lang}: missing {state:?}"
                );
            }
        }
    }
}
