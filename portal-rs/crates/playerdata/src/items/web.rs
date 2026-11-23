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
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
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
        static PL: OnceLock<Option<FluentBundle<FluentResource>>> = OnceLock::new();
        static EN: OnceLock<Option<FluentBundle<FluentResource>>> = OnceLock::new();
        let (bundle, source) = if self.0 == "pl" {
            (&PL, include_str!("pl.ftl"))
        } else {
            (&EN, include_str!("en.ftl"))
        };
        let Some(bundle) = bundle.get_or_init(|| {
            let resource = FluentResource::try_new(source.to_owned()).ok()?;
            let mut bundle = FluentBundle::new_concurrent(vec![self.0.parse().ok()?]);
            bundle.add_resource(resource).ok()?;
            Some(bundle)
        }) else {
            return String::new();
        };
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
#[template(path = "playerdata/items.html", config = "src/hunts.askama.toml")]
struct ItemsPage {
    locale: Locale,
    title: String,
    at: String,
    total: String,
    unpriced: usize,
    query: String,
    depots: Vec<u16>,
    locations: Vec<LocationOption>,
    world: Uuid,
    character: Uuid,
    history_enabled: bool,
    previous: Option<usize>,
    next: Option<usize>,
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
    path = "playerdata/item_history.html",
    config = "src/hunts.askama.toml"
)]
struct HistoryPage {
    locale: Locale,
    title: String,
    start: String,
    through: String,
    events: Vec<EventView>,
    next_cursor: Option<String>,
    items_link: Option<String>,
}

/// Browser adapters resolve the actor from the existing server-side session.
/// All responses, including extraction failures and denials, are no-store.
pub fn router(projection: Projection) -> Router {
    let mut router = Router::new().route("/{lang}/account/journal/items", get(index));
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
    let flags = Flags {
        items: enabled("PLAYERDATA_G10_ITEMS_ENABLED"),
        history: enabled("PLAYERDATA_G10_HISTORY_ENABLED"),
        bazaar_shortcut: enabled("OTERYN_BAZAAR_ENABLED"),
        ..Flags::default()
    };
    let source = std::env::var(crate::journal::ITEMS_SOURCE)
        .ok()
        .filter(|value| !value.is_empty());
    eprintln!(
        "G-10 journal items_read={} history_read={} ingest_requested={} source_configured={} readiness=awaiting_qualified_game_evidence",
        flags.items,
        flags.history,
        enabled(crate::journal::ITEMS_ENABLED),
        source.is_some(),
    );
    let evidence: Arc<dyn Evidence> = match source {
        Some(source) => Arc::new(crate::authority::JournalEvidence { source }),
        None => Arc::new(UnavailableEvidence),
    };
    router(Projection {
        pool,
        flags,
        evidence,
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
#[derive(Template)]
#[template(path = "playerdata/items_index.html", config = "src/hunts.askama.toml")]
struct IndexPage {
    locale: Locale,
    flags: Flags,
    characters: Vec<(Uuid, String, Uuid)>,
    worlds: Vec<(Uuid, String)>,
}

async fn index(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Response {
    let Some(actor) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let locale = match Locale::parse(&lang) {
        Ok(l) => l,
        Err(s) => return s.into_response(),
    };
    let characters = if p.flags.items || p.flags.history {
        match p.pool.begin().await {
            Ok(tx) => crate::authority::CollectionEvidence::owned_characters(tx, actor)
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let worlds = if p.flags.history {
        sqlx::query_as("SELECT id,name FROM world_status ORDER BY name,id LIMIT 256")
            .fetch_all(&p.pool)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    oteryn_web::render(&IndexPage {
        locale,
        flags: p.flags,
        characters,
        worlds,
    })
}

#[derive(Template)]
#[template(
    path = "playerdata/items_unavailable.html",
    config = "src/hunts.askama.toml"
)]
struct UnavailablePage {
    locale: Locale,
    title: String,
}

fn read_error(locale: Locale, title: String, error: Error) -> Response {
    let code = status(error);
    if code == StatusCode::SERVICE_UNAVAILABLE {
        (code, oteryn_web::render(&UnavailablePage { locale, title })).into_response()
    } else {
        code.into_response()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    #[serde(default)]
    q: String,
    #[serde(default)]
    location: Location,
    page: Option<usize>,
}

struct LocationOption {
    value: &'static str,
    label: &'static str,
    selected: bool,
}

const ITEMS_PER_PAGE: usize = 50;

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
    let page = search.page.unwrap_or(1);
    let Some(skip) = page
        .checked_sub(1)
        .and_then(|n| n.checked_mul(ITEMS_PER_PAGE))
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let locations = [
        (Location::All, "all", "items-all"),
        (Location::Equipment, "equipment", "items-equipment"),
        (Location::Depot, "depot", "items-depot"),
        (Location::Inbox, "inbox", "items-inbox"),
        (Location::House, "house", "items-house-location"),
    ]
    .into_iter()
    .map(|(location, value, label)| LocationOption {
        value,
        label,
        selected: search.location == location,
    })
    .collect();
    let title = locale.text("items-title");
    let mut inventory = match p
        .inventory_at(actor, world, character, &search.q, search.location)
        .await
    {
        Ok(inventory) => inventory,
        Err(error) => return read_error(locale, title, error),
    };
    if skip > 0 && skip >= inventory.items.len() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let has_next = skip + ITEMS_PER_PAGE < inventory.items.len();
    inventory.items = inventory
        .items
        .into_iter()
        .skip(skip)
        .take(ITEMS_PER_PAGE)
        .collect();
    let mut view = items_page(locale, inventory, search.q, world, character);
    view.locations = locations;
    view.history_enabled = p.flags.history;
    view.previous = (page > 1).then_some(page - 1);
    view.next = has_next.then_some(page + 1);
    oteryn_web::render(&view)
}

fn items_page(
    locale: Locale,
    v: Inventory,
    query: String,
    world: Uuid,
    character: Uuid,
) -> ItemsPage {
    let unknown = locale.text("items-unknown");
    let count = v.items.len();
    let rows = v
        .items
        .iter()
        .map(|v| {
            let item = &v.item;
            let mut location = match item.root {
                Root::Equipment => vec![locale.text("items-equipment")],
                Root::Depot => vec![locale.text("items-depot")],
                Root::Inbox => vec![locale.text("items-inbox")],
                Root::House => vec![locale.text("items-house-location")],
            };
            for (label, value) in [
                ("items-town", item.town_id.map(|n| n.to_string())),
                ("items-box", item.depot_box.map(|n| n.to_string())),
                ("items-slot", item.equipment_slot.map(|n| n.to_string())),
            ] {
                if let Some(value) = value {
                    location.push(format!("{} {}", locale.text(label), value));
                }
            }
            if let Some(container) = &v.container {
                location.push(format!("{} {}", locale.text("items-container"), container));
            }
            ItemView {
                definition: item.definition_production_key.clone(),
                quantity: item.quantity,
                location: location.join(" · "),
                estimate: v
                    .estimate
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| unknown.clone()),
                source: v.price_source.clone().unwrap_or_else(|| unknown.clone()),
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
        query,
        depots: v.depots,
        empty: v.snapshot_empty,
        no_results: !v.snapshot_empty && count == 0,
        items: rows,
        world,
        character,
        locations: Vec::new(),
        history_enabled: false,
        previous: None,
        next: None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryQuery {
    cursor: Option<String>,
}

async fn bank_history(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, world, family)): Path<(String, Uuid, String)>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    history(p, session, lang, world, None, family, query).await
}
async fn character_history(
    State(p): State<Projection>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, world, family, character)): Path<(String, Uuid, String, Uuid)>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    history(p, session, lang, world, Some(character), family, query).await
}
async fn history(
    p: Projection,
    session: BrowserSession,
    lang: String,
    world: Uuid,
    character: Option<Uuid>,
    family: String,
    query: HistoryQuery,
) -> Response {
    let Some(actor) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let locale = match Locale::parse(&lang) {
        Ok(l) => l,
        Err(s) => return s.into_response(),
    };
    let (family, label) = match family.as_str() {
        "bank" => ("BANK", "items-bank"),
        "market" => ("MARKET", "items-market"),
        "trades" => ("PLAYER_TRADE", "items-trade"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let title = locale.text(label);
    match p
        .history_page(actor, world, character, family, query.cursor.as_deref())
        .await
    {
        Ok(h) => {
            let events = h
                .records
                .into_iter()
                .map(|r| event_view(locale, r))
                .collect();
            oteryn_web::render(&HistoryPage {
                locale,
                title,
                start: utc(h.coverage_start),
                through: utc(h.through),
                events,
                next_cursor: h.next_cursor,
                items_link: character
                    .filter(|_| p.flags.items)
                    .map(|id| format!("/{lang}/panel/items/{world}/{id}")),
            })
        }
        Err(e) => read_error(locale, title, e),
    }
}

#[cfg(test)]
pub(super) fn render_inventory(v: Inventory, lang: &str) -> String {
    Locale::parse(lang)
        .ok()
        .and_then(|locale| {
            items_page(locale, v, String::new(), Uuid::nil(), Uuid::nil())
                .render()
                .ok()
        })
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
