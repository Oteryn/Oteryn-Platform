use super::private;
use askama::Template;
use axum::{
    Extension, Router,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_ui::{
    Chrome,
    panels::{Entry, Panel, Permission},
};
use oteryn_web::session::{self, BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const FLAG: &str = "OTERYN_PANEL_STORE_ORDERS_ENABLED";
const PAGE_SIZE: usize = 25;

fn enabled() -> bool {
    std::env::var(FLAG).as_deref() == Ok("true")
}

pub fn register(registry: &mut oteryn_ui::panels::Registry) {
    registry.register(
        enabled(),
        Entry {
            panel: Panel::Player,
            route: "account/store-orders",
            label: "commerce-orders",
            permission: Permission::AccountOwner,
        },
    );
}

pub fn mount(pool: PgPool) -> Router {
    if !enabled() {
        return Router::new();
    }
    routes(pool)
}

pub(super) fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/{lang}/account/store-orders", get(history))
        .route("/{lang}/account/store-orders/{id}", get(detail))
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(pool)
}

#[derive(sqlx::FromRow)]
struct Order {
    order_id: Uuid,
    offer_key: String,
    quantity: i32,
    total_price: i64,
    placed_at: String,
}

#[derive(sqlx::FromRow)]
struct Unit {
    product_id: String,
    quantity: i32,
    state: String,
    updated_at: String,
    receipt: Option<String>,
}

const HISTORY: &str = "SELECT o.order_id,l.offer_key,l.quantity,o.total_price,
    to_char(o.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS placed_at
    FROM store_orders o JOIN store_order_lines l USING(order_id)
    WHERE o.account_id=$1 AND ($2::uuid IS NULL OR (o.created_at,o.order_id) <
        (SELECT created_at,order_id FROM store_orders WHERE order_id=$2 AND account_id=$1))
    ORDER BY o.created_at DESC,o.order_id DESC LIMIT $3";

const DETAIL: &str = "SELECT o.order_id,l.offer_key,l.quantity,o.total_price,
    to_char(o.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS placed_at
    FROM store_orders o JOIN store_order_lines l USING(order_id)
    WHERE o.account_id=$1 AND o.order_id=$2";

const UNITS: &str = "SELECT u.product_id,u.quantity,u.state,
    to_char(GREATEST(u.updated_at, r.updated_at) AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS updated_at,
    r.outcome AS receipt
    FROM store_orders o JOIN store_order_lines l USING(order_id)
    JOIN store_fulfilment_units u USING(line_id)
    LEFT JOIN LATERAL (
        SELECT r.outcome,r.updated_at FROM store_operations s JOIN store_receipts r USING(operation_id)
        WHERE s.entitlement_id=u.entitlement_id AND r.superseded_by_restore_event_id IS NULL
        ORDER BY s.source_revision DESC LIMIT 1
    ) r ON true
    WHERE o.account_id=$1 AND u.account_id=$1 AND o.order_id=$2 ORDER BY u.unit_index";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Paging {
    before: Option<Uuid>,
}

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"<!doctype html><html lang="{{ chrome.lang }}"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{ chrome.text("commerce-orders") }}</title></head><body>
<main class="op-container op-stack">
<h1>{{ chrome.text("commerce-orders") }}</h1>
{% if detail %}<a href="/{{ chrome.lang }}/account/store-orders">{{ chrome.text("commerce-orders") }}</a>{% endif %}
{% for order in orders %}
<article class="op-card">
<h2>{% if detail %}{{ order.offer_key }}{% else %}<a href="/{{ chrome.lang }}/account/store-orders/{{ order.order_id }}">{{ order.offer_key }}</a>{% endif %}</h2>
<p>{{ order.quantity }} × · {{ order.total_price }} COINS</p>
<p>{{ order.placed_at }} UTC</p>
</article>
{% else %}<p>{{ chrome.text("commerce-no-orders") }}</p>{% endfor %}
{% if detail %}
<section class="op-stack"><h2>{{ chrome.text("commerce-delivery") }}</h2>
<p>{{ chrome.text("commerce-delivery-unavailable") }}</p>
{% for unit in units %}
<article class="op-card"><h3>{{ unit.product_id }} × {{ unit.quantity }}</h3>
<p>{{ chrome.text(unit.state_key()) }}</p>
{% if let Some(receipt) = unit.receipt_key() %}<p>{{ chrome.text(receipt) }}</p>{% endif %}
<p>{{ chrome.text("commerce-updated") }} {{ unit.updated_at }} UTC</p></article>
{% else %}<p>{{ chrome.text("commerce-no-deliveries") }}</p>{% endfor %}
</section>
{% endif %}
{% if let Some(next) = next %}
<a href="?before={{ next }}">{{ chrome.text("commerce-older") }}</a>
{% endif %}
</main></body></html>"#
)]
struct Page {
    chrome: Chrome,
    orders: Vec<Order>,
    units: Vec<Unit>,
    detail: bool,
    next: Option<Uuid>,
}

impl Unit {
    fn state_key(&self) -> &'static str {
        match self.state.as_str() {
            "DELIVERY_PENDING" => "commerce-state-delivery-pending",
            "DELIVERED" => "commerce-state-delivered",
            "CLAIM_PENDING" => "commerce-state-claim-pending",
            "CLAIMED" => "commerce-state-claimed",
            "REVOKED" => "commerce-state-revoked",
            "RECLAIMED" => "commerce-state-reclaimed",
            "MANUAL_REVIEW" => "commerce-state-manual-review",
            "REVERSAL_PARTIAL" => "commerce-state-reversal-partial",
            _ => "commerce-state-unknown",
        }
    }

    fn receipt_key(&self) -> Option<&'static str> {
        match self.receipt.as_deref() {
            Some("COMMITTED") => Some("commerce-receipt-committed"),
            Some("REJECTED") => Some("commerce-receipt-rejected"),
            _ => None,
        }
    }
}

async fn history(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    filter: Result<Query<Paging>, QueryRejection>,
) -> Response {
    let (mut tx, account) = match order_reader(&pool, &session).await {
        Ok(reader) => reader,
        Err(StatusCode::SEE_OTHER) => return login(&chrome),
        Err(status) => return oteryn_ui::error(&chrome, status),
    };
    let Ok(Query(filter)) = filter else {
        return oteryn_ui::error(&chrome, StatusCode::UNPROCESSABLE_ENTITY);
    };
    if !matches!(lang.as_str(), "pl" | "en") {
        return oteryn_ui::error(&chrome, StatusCode::NOT_FOUND);
    }
    let mut orders = match sqlx::query_as(HISTORY)
        .bind(account)
        .bind(filter.before)
        .bind(PAGE_SIZE as i64 + 1)
        .fetch_all(&mut *tx)
        .await
    {
        Ok(orders) => orders,
        Err(_) => return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE),
    };
    let next = if orders.len() > PAGE_SIZE {
        orders.pop();
        orders.last().map(|order: &Order| order.order_id)
    } else {
        None
    };
    if tx.commit().await.is_err() {
        return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE);
    }
    oteryn_web::render(&Page {
        chrome,
        orders,
        units: vec![],
        detail: false,
        next,
    })
}

async fn detail(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    path: Result<Path<(String, Uuid)>, PathRejection>,
) -> Response {
    let (mut tx, account) = match order_reader(&pool, &session).await {
        Ok(reader) => reader,
        Err(StatusCode::SEE_OTHER) => return login(&chrome),
        Err(status) => return oteryn_ui::error(&chrome, status),
    };
    let Ok(Path((lang, id))) = path else {
        return oteryn_ui::error(&chrome, StatusCode::UNPROCESSABLE_ENTITY);
    };
    if !matches!(lang.as_str(), "pl" | "en") {
        return oteryn_ui::error(&chrome, StatusCode::NOT_FOUND);
    }
    let order = match sqlx::query_as(DETAIL)
        .bind(account)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(order)) => order,
        Ok(None) => return oteryn_ui::error(&chrome, StatusCode::NOT_FOUND),
        Err(_) => return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE),
    };
    let units = match sqlx::query_as(UNITS)
        .bind(account)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
    {
        Ok(units) => units,
        Err(_) => return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE),
    };
    if tx.commit().await.is_err() {
        return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE);
    }
    oteryn_web::render(&Page {
        chrome,
        orders: vec![order],
        units,
        detail: true,
        next: None,
    })
}

async fn order_reader<'a>(
    pool: &'a PgPool,
    session: &BrowserSession,
) -> Result<(Transaction<'a, Postgres>, Uuid), StatusCode> {
    let (Some(account), Some(session_hash)) =
        (session.account_id().await, session.identity_hash().await)
    else {
        return Err(StatusCode::SEE_OTHER);
    };
    let unavailable = |_| StatusCode::SERVICE_UNAVAILABLE;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query(
        "SELECT set_config('lock_timeout','1s',true),set_config('statement_timeout','2500ms',true)",
    )
    .execute(&mut *tx)
    .await
    .map_err(unavailable)?;
    if !oteryn_wallet::value_eligible_in(&mut tx, account)
        .await
        .map_err(unavailable)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
    // Match session rotation's account-before-session locking order.
    let live: Option<Uuid> = sqlx::query_scalar(
        "SELECT account_id FROM web_sessions WHERE id_hash=$1 AND account_id=$2
         AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()
         FOR SHARE",
    )
    .bind(session_hash)
    .bind(account)
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if live.is_none() {
        return Err(StatusCode::SEE_OTHER);
    }
    Ok((tx, account))
}

fn login(chrome: &Chrome) -> Response {
    Redirect::to(&format!("/{}/login", chrome.lang)).into_response()
}
