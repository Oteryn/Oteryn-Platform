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
use oteryn_ui::Chrome;
use oteryn_web::session::{BrowserSession, SessionKey, SessionLayer};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::account::Locale;
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
    let checkout_page = request.method() == axum::http::Method::GET
        && request.uri().path().starts_with(CHECKOUT_PATH);
    let mut response = next.run(request).await;
    // Chromium sends Origin: null on form submissions from a no-referrer page.
    // Retain the origin of same-origin checkout POSTs for the shared CSRF guard.
    let referrer = if checkout_page && response.status() == StatusCode::OK {
        "same-origin"
    } else {
        "no-referrer"
    };
    for (name, value) in [
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, referrer),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
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
    lang: &'static str,
    title: String,
    note: String,
    order_label: String,
    coins_label: String,
    price_label: String,
    status_label: String,
    pay_label: String,
    fail_label: String,
    done: String,
    csrf: String,
    order_id: Uuid,
    coins: i64,
    price: String,
    state: String,
    awaiting: bool,
}

#[derive(Deserialize)]
struct Decision {
    action: String,
}

fn checkout_language(chrome: Option<Extension<Chrome>>) -> &'static str {
    if chrome.is_some_and(|Extension(chrome)| chrome.lang == "en") {
        "en"
    } else {
        "pl"
    }
}

async fn page(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(order): Path<String>,
    chrome: Option<Extension<Chrome>>,
) -> Response {
    let lang = checkout_language(chrome);
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
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let order = match service.checkout(account, order).await {
        Ok(order) => order,
        Err(error) => return refusal(error),
    };
    let Ok(csrf) = session.csrf_token().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Ok(locale) = Locale::parse(lang) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    oteryn_web::render(&CheckoutPage {
        lang,
        title: locale.text("coin-checkout-title"),
        note: locale.text("coin-checkout-note"),
        order_label: locale.text("coin-checkout-order"),
        coins_label: locale.text("coin-checkout-coins"),
        price_label: locale.text("coin-checkout-price"),
        status_label: locale.text("coin-checkout-status"),
        pay_label: locale.text("coin-checkout-pay"),
        fail_label: locale.text("coin-checkout-fail"),
        done: locale.text("coin-checkout-done"),
        csrf,
        order_id: order.order_id,
        coins: order.coins,
        price: order.price.display(),
        state: locale.text(&format!("coin-state-{}", order.state.as_str())),
        awaiting: order.state == OrderState::AwaitingPayment,
    })
}

/// CSRF is verified by the session layer before this runs.
async fn act(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(order): Path<String>,
    chrome: Option<Extension<Chrome>>,
    Form(decision): Form<Decision>,
) -> Response {
    let Some(account) = session.account_id().await else {
        let lang = checkout_language(chrome);
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let Some(order) = canonical_id(&order) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let paid = match decision.action.as_str() {
        "pay" => true,
        "fail" => false,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    match service.sandbox_complete(account, order, paid).await {
        Ok(_) => Redirect::to(&format!("{CHECKOUT_PATH}{order}")).into_response(),
        Err(error) => refusal(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn checkout_form_retains_the_same_origin_for_csrf() -> Result<(), axum::http::Error> {
        let checkout = format!("{CHECKOUT_PATH}{{order_id}}");
        let api = format!("{API}/catalogue");
        let app = Router::new()
            .route(&checkout, get(|| async { StatusCode::OK }))
            .route(&api, get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(private));
        for (path, policy) in [
            (
                checkout.replace("{order_id}", &Uuid::now_v7().to_string()),
                "same-origin",
            ),
            (api, "no-referrer"),
        ] {
            let request = axum::http::Request::get(path).body(Body::empty())?;
            let Ok(response) = app.clone().oneshot(request).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::REFERRER_POLICY], policy);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
        Ok(())
    }
}
