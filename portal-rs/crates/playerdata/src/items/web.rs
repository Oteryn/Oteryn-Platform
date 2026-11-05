use super::{model::*, store::*};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct Locale(&'static str);
impl Locale {
    fn parse(s: &str) -> std::result::Result<Self, StatusCode> {
        match s {
            "pl" => Ok(Self("pl")),
            "en" => Ok(Self("en")),
            _ => Err(StatusCode::NOT_FOUND),
        }
    }
    fn text(self, key: &str) -> String {
        let source = if self.0 == "pl" {
            include_str!("pl.ftl")
        } else {
            include_str!("en.ftl")
        };
        let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
            return String::new();
        };
        let Ok(lang) = self.0.parse() else {
            return String::new();
        };
        let mut bundle = FluentBundle::new(vec![lang]);
        if bundle.add_resource(resource).is_err() {
            return String::new();
        }
        bundle
            .get_message(key)
            .and_then(|m| m.value())
            .map(|v| bundle.format_pattern(v, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct ItemView {
    definition: String,
    quantity: u32,
    location: String,
    estimate: String,
    source: String,
    bazaar: bool,
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ locale.0 }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><main class="op-stack op-panel"><h1>{{ title }}</h1><p>{{ locale.text("items-warning") }}</p><p>{{ locale.text("items-house") }}</p><p>{{ locale.text("items-at") }}: {{ at }}</p><p>{{ locale.text("items-estimate") }}: {{ total }}</p><p>{{ locale.text("items-unpriced") }}: {{ unpriced }}</p><form method="get" class="op-stack"><label for="q">{{ locale.text("items-search") }}</label><input class="op-input" id="q" name="q" maxlength="128" value="{{ query }}"><button class="op-button" type="submit">{{ locale.text("items-submit") }}</button></form><p>{{ depots }}</p>{% if empty %}<p>{{ locale.text("items-empty") }}</p>{% endif %}{% if no_results %}<p>{{ locale.text("items-no-results") }}</p>{% endif %}<ul>{% for item in items %}<li><article class="op-panel op-stack"><h2>{{ item.definition }}</h2><dl><dt>{{ locale.text("items-quantity") }}</dt><dd>{{ item.quantity }}</dd><dt>{{ locale.text("items-location") }}</dt><dd>{{ item.location }}</dd><dt>{{ locale.text("items-estimate") }}</dt><dd>{{ item.estimate }}</dd><dt>{{ locale.text("items-source") }}</dt><dd>{{ item.source }}</dd></dl>{% if item.bazaar %}<a href="/{{ locale.0 }}/bazaar">{{ locale.text("items-bazaar") }}</a>{% endif %}</article></li>{% endfor %}</ul></main></body></html>"#,
    ext = "html"
)]
struct ItemsPage {
    locale: Locale,
    title: String,
    at: String,
    total: String,
    unpriced: usize,
    query: String,
    depots: String,
    empty: bool,
    no_results: bool,
    items: Vec<ItemView>,
}

#[derive(Debug)]
struct EventView {
    at: String,
    kind: String,
    details: Vec<(String, String)>,
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ locale.0 }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><main class="op-stack op-panel"><h1>{{ title }}</h1><p>{{ locale.text("items-coverage") }}: {{ start }} – {{ through }}</p><p>{{ locale.text("items-history-note") }}</p><ul>{% for event in events %}<li><article class="op-panel op-stack"><h2>{{ event.kind }}</h2><p>{{ locale.text("items-time") }}: {{ event.at }}</p><dl>{% for (label, value) in event.details %}<dt>{{ label }}</dt><dd>{{ value }}</dd>{% endfor %}</dl></article></li>{% endfor %}</ul></main></body></html>"#,
    ext = "html"
)]
struct HistoryPage {
    locale: Locale,
    title: String,
    start: String,
    through: String,
    events: Vec<EventView>,
}

/// Browser adapters resolve the actor from the existing server-side session.
/// All responses, including extraction failures and denials, are no-store.
pub fn router(projection: Projection) -> Router {
    if !projection.flags.items && !projection.flags.history {
        return Router::new();
    }
    let mut router = Router::new();
    if projection.flags.items {
        router = router.route("/{lang}/panel/items/{world}/{character}", get(items));
    }
    if projection.flags.history {
        router = router
            .route("/{lang}/panel/history/{world}/{family}", get(bank_history))
            .route(
                "/{lang}/panel/history/{world}/{family}/{character}",
                get(character_history),
            );
    }
    router
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(projection.pool.clone()),
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(projection)
}

pub fn configured_router(pool: PgPool) -> Router {
    let enabled = |name: &str| std::env::var(name).as_deref() == Ok("true");
    router(Projection {
        pool,
        flags: Flags {
            items: enabled("PLAYERDATA_G10_ITEMS_ENABLED"),
            history: enabled("PLAYERDATA_G10_HISTORY_ENABLED"),
            // Commands have no accepted Game implementation. Do not activate a shortcut.
            ingest: false,
            bazaar_shortcut: false,
        },
        evidence: Arc::new(UnavailableEvidence),
    })
}
async fn private(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
        .headers_mut()
        .insert(header::VARY, header::HeaderValue::from_static("Cookie"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response
}
fn status(e: Error) -> StatusCode {
    match e {
        Error::Denied => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::BAD_REQUEST,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Unavailable | Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    #[serde(default)]
    q: String,
}
async fn items(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, world, character)): Path<(String, Uuid, Uuid)>,
    Query(search): Query<Search>,
) -> Response {
    let Some(actor) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let locale = match Locale::parse(&lang) {
        Ok(l) => l,
        Err(s) => return s.into_response(),
    };
    match p.inventory(actor, world, character, &search.q).await {
        Ok(inventory) => oteryn_web::render(&items_page(locale, inventory, search.q)),
        Err(e) => status(e).into_response(),
    }
}
fn items_page(locale: Locale, v: Inventory, query: String) -> ItemsPage {
    let unknown = locale.text("items-unknown");
    let count = v.items.len();
    let depots = format!(
        "{}: {}",
        locale.text("items-depot"),
        v.depots
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    let rows = v
        .items
        .into_iter()
        .map(|v| {
            let root = match v.item.root {
                Root::Equipment => "items-equipment",
                Root::Depot => "items-depot",
                Root::Inbox => "items-inbox",
                Root::House => "items-unknown",
            };
            let location = format!(
                "{} {} {} {}",
                locale.text(root),
                v.item.town_id.map(|v| v.to_string()).unwrap_or_default(),
                v.item.depot_box.map(|v| v.to_string()).unwrap_or_default(),
                v.item.parent_item_instance_id.unwrap_or_default()
            );
            ItemView {
                definition: v.item.definition_production_key,
                quantity: v.item.quantity,
                location,
                estimate: v
                    .estimate
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| unknown.clone()),
                source: v.price_source.unwrap_or_else(|| unknown.clone()),
                bazaar: v.bazaar_shortcut,
            }
        })
        .collect();
    ItemsPage {
        locale,
        title: locale.text("items-title"),
        at: utc(v.at_logout),
        total: v.estimated_total.map(|n| n.to_string()).unwrap_or(unknown),
        unpriced: v.unpriced,
        query: query.clone(),
        depots,
        empty: v.estimated_total == Some(0) && count == 0 && query.is_empty(),
        no_results: count == 0 && !query.is_empty(),
        items: rows,
    }
}
async fn bank_history(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, world, family)): Path<(String, Uuid, String)>,
) -> Response {
    history(p, session, lang, world, None, family).await
}
async fn character_history(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, world, family, character)): Path<(String, Uuid, String, Uuid)>,
) -> Response {
    history(p, session, lang, world, Some(character), family).await
}
async fn history(
    p: Projection,
    session: BrowserSession,
    lang: String,
    world: Uuid,
    character: Option<Uuid>,
    family: String,
) -> Response {
    let Some(actor) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let locale = match Locale::parse(&lang) {
        Ok(l) => l,
        Err(s) => return s.into_response(),
    };
    let family = match family.as_str() {
        "bank" => "BANK",
        "market" => "MARKET",
        "trades" => "PLAYER_TRADE",
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    match p.history(actor, world, character, family).await {
        Ok(h) => {
            let events = h
                .records
                .into_iter()
                .map(|r| event_view(locale, r))
                .collect();
            oteryn_web::render(&HistoryPage {
                locale,
                title: locale.text("items-history"),
                start: utc(h.coverage_start),
                through: utc(h.through),
                events,
            })
        }
        Err(e) => status(e).into_response(),
    }
}

#[cfg(test)]
pub(super) fn render_inventory(v: Inventory, lang: &str) -> String {
    Locale::parse(lang)
        .ok()
        .and_then(|locale| items_page(locale, v, String::new()).render().ok())
        .unwrap_or_default()
}

fn utc(seconds: u64) -> String {
    i64::try_from(seconds)
        .ok()
        .and_then(|v| time::OffsetDateTime::from_unix_timestamp(v).ok())
        .and_then(|v| {
            v.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_default()
}
fn event_view(locale: Locale, record: Record) -> EventView {
    let mut details = Vec::new();
    let mut push = |label: &str, value: String| details.push((locale.text(label), value));
    let kind = match record.payload {
        Payload::Bank(b) => {
            push(
                "items-direction",
                locale.text(&format!("items-{}", b.direction)),
            );
            push("items-amount", b.amount);
            push("items-before", b.balance_before);
            push("items-after", b.balance_after);
            locale.text(&format!("items-{}", b.kind))
        }
        Payload::Market(m) => {
            push("items-definition", m.definition_production_key);
            push("items-side", locale.text(&format!("items-{}", m.side)));
            push("items-state", locale.text(&format!("items-{}", m.state)));
            if let Some(end) = m.end_state {
                push("items-terminal", locale.text(&format!("items-{end}")));
            }
            push("items-quantity", m.amount.to_string());
            push("items-remaining", m.remaining.to_string());
            push("items-price", m.piece_price);
            push("items-fee", m.fee_gold);
            push("items-held", m.held_credit_gold);
            locale.text("items-market")
        }
        Payload::Trade(t) => {
            for line in t.given {
                push(
                    "items-given",
                    format!("{} × {}", line.quantity, line.definition_production_key),
                );
            }
            for line in t.received {
                push(
                    "items-received",
                    format!("{} × {}", line.quantity, line.definition_production_key),
                );
            }
            locale.text("items-trade")
        }
        _ => locale.text("items-unknown"),
    };
    EventView {
        at: counter(&record.occurred_at, false)
            .map(utc)
            .unwrap_or_default(),
        kind,
        details,
    }
}
