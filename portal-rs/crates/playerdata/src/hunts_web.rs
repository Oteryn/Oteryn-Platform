//! SSR owner routes. Authentication is resolved by the shared browser middleware.
use super::journal::{Hunt, Journal, PartySegment};
use super::store::Repository;
use super::{Error, Filter, Flags, Pricing, Session, Unpriced, compare, split_party, valuation};
use super::{prices, v7};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Clone)]
struct Web {
    journal: Journal,
    projected: Option<Repository>,
    flags: Flags,
    prices: Arc<dyn Pricing>,
}

/// Mount the owner views over the reconciled projection API for embedded callers.
pub fn router(pool: PgPool, flags: Flags, prices: Arc<dyn Pricing>) -> Router {
    let projected = Some(Repository::new(pool.clone(), flags));
    mount(
        Journal {
            pool,
            source: String::new(),
        },
        projected,
        flags,
        prices,
    )
}

fn mount(
    journal: Journal,
    projected: Option<Repository>,
    flags: Flags,
    prices: Arc<dyn Pricing>,
) -> Router {
    let sessions = SessionLayer::new(journal.pool.clone());
    let mut app = Router::new()
        .route("/{lang}/account/hunts", get(index))
        .route("/{lang}/account/hunts/compare", get(comparison))
        .route("/{lang}/account/hunts/{id}", get(detail))
        .route(
            "/{lang}/account/hunts/{id}/share",
            get(private_link).post(share),
        )
        .with_state(Web {
            journal,
            projected,
            flags,
            prices,
        })
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .route_layer(middleware::from_fn(
            move |request: axum::extract::Request, next: Next| async move {
                if !flags.read
                    || request.method() == axum::http::Method::POST
                        && request.uri().path().ends_with("/share")
                {
                    StatusCode::SERVICE_UNAVAILABLE.into_response()
                } else {
                    next.run(request).await
                }
            },
        ));
    if flags.read {
        app = app.route("/static/playerdata/hunts.css", get(styles));
    }
    app.layer(middleware::from_fn(private_headers))
}

pub fn configured_router(pool: PgPool) -> Router {
    let settings = crate::sessions::Settings::from_lookup(|key| std::env::var(key).ok());
    mount(
        Journal {
            pool,
            source: settings.source,
        },
        None,
        Flags {
            read: std::env::var("OTERYN_PLAYERDATA_HUNTS_READ").is_ok_and(|v| v == "true"),
            party: crate::sessions::party_privacy_accepted(),
        },
        Arc::new(Unpriced),
    )
}
impl Web {
    async fn read(
        &self,
        account: Uuid,
        now: i64,
        filter: &Filter,
        ids: &[Uuid],
    ) -> Result<Vec<Hunt>, Error> {
        let Some(projected) = &self.projected else {
            return self.journal.read(account, now, filter, ids).await;
        };
        let sessions = if ids.is_empty() {
            projected.list(account, now, filter).await?
        } else {
            let mut sessions = Vec::new();
            for id in ids {
                sessions.push(projected.detail(account, now, *id).await?);
            }
            sessions
        };
        Ok(sessions
            .into_iter()
            .map(|session| {
                let parties = session
                    .party
                    .iter()
                    .map(|party| PartySegment {
                        started_at: session.started_at,
                        ended_at: session.ended_at,
                        ledger: party.clone(),
                    })
                    .collect();
                Hunt {
                    session,
                    catalogue: None,
                    parties,
                    drops: None,
                }
            })
            .collect())
    }
}
async fn private_headers(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, header::HeaderValue::from_static("default-src 'none'; style-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("no-referrer"),
    );
    response
}
fn status(error: Error) -> StatusCode {
    match error {
        Error::Unavailable | Error::Storage => StatusCode::SERVICE_UNAVAILABLE,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::BAD_REQUEST,
    }
}
fn now() -> Result<i64, StatusCode> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .as_secs();
    i64::try_from(seconds).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
async fn owner(state: &Web, lang: &str, session: &BrowserSession) -> Result<Uuid, Response> {
    if !state.flags.read {
        return Err(StatusCode::SERVICE_UNAVAILABLE.into_response());
    }
    if !matches!(lang, "pl" | "en") {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    session.account_id().await.ok_or_else(|| {
        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, format!("/{lang}/login"))],
        )
            .into_response()
    })
}
fn text(lang: &str, id: &str) -> String {
    let source = if lang == "pl" {
        include_str!("../../../templates/playerdata/pl.ftl")
    } else {
        include_str!("../../../templates/playerdata/en.ftl")
    };
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let Ok(locale) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle
        .format_pattern(pattern, None, &mut Vec::new())
        .into_owned()
}
#[derive(Debug)]
struct Line {
    label: String,
    value: String,
}
#[derive(Debug)]
struct Card {
    id: Uuid,
    lines: Vec<Line>,
    drops: Vec<String>,
    journal: bool,
}
#[derive(Template)]
#[template(path = "playerdata/hunts.html", config = "src/hunts.askama.toml")]
struct Page {
    lang: String,
    title: String,
    list: bool,
    state: String,
    first: String,
    second: String,
    drop_history: String,
    private_link: String,
    price_label: String,
    price_help: String,
    price_amounts: String,
    back: String,
    empty: String,
    unavailable: String,
    character: String,
    offset: String,
    place: String,
    from: String,
    until: String,
    party: String,
    apply: String,
    all: String,
    yes: String,
    no: String,
    details: String,
    compare: String,
    split: String,
    cards: Vec<Card>,
}
fn page(lang: &str, cards: Vec<Card>, list: bool, state: &str, price_amounts: String) -> Response {
    let page = Page {
        lang: lang.to_owned(),
        title: text(lang, "title"),
        list,
        state: text(lang, state),
        first: text(lang, "compare-first"),
        second: text(lang, "compare-second"),
        drop_history: text(lang, "drop-history"),
        private_link: text(lang, "private-link"),
        price_label: text(lang, "custom-prices"),
        price_help: text(lang, "custom-prices-help"),
        price_amounts,
        back: text(lang, "back"),
        empty: text(lang, "drops-empty"),
        unavailable: text(lang, "derived"),
        character: text(lang, "character"),
        offset: text(lang, "offset"),
        place: text(lang, "place"),
        from: text(lang, "from"),
        until: text(lang, "until"),
        party: text(lang, "party"),
        apply: text(lang, "apply"),
        all: text(lang, "all"),
        yes: text(lang, "yes"),
        no: text(lang, "no"),
        details: text(lang, "details"),
        compare: text(lang, "compare"),
        split: text(lang, "split-note"),
        cards,
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn card(lang: &str, hunt: &Hunt, prices: &dyn Pricing, flags: Flags, full: bool) -> Card {
    let session = &hunt.session;
    let mut lines = Vec::new();
    let mut add = |label: &str, value: String| {
        lines.push(Line {
            label: text(lang, label),
            value,
        })
    };
    add("session", session.session_id.to_string());
    add("character", session.character_id.to_string());
    add("from", session.started_at.to_string());
    add("until", session.ended_at.to_string());
    add("duration", session.duration_seconds.to_string());
    add("experience", session.experience_gained.to_string());
    add("loss", session.experience_lost.to_string());
    add(
        "experience-rate",
        session
            .experience_per_hour()
            .map_or_else(|| text(lang, "unavailable"), |n| n.to_string()),
    );
    for (label, rows) in [
        ("place", &session.places),
        ("loot", &session.loot),
        ("supplies", &session.supplies),
        ("kills", &session.monsters_killed),
    ] {
        for (key, amount) in rows {
            let name = if key.is_empty() {
                text(lang, "unavailable")
            } else {
                key.clone()
            };
            add(label, format!("{name}: {amount}"));
        }
    }
    for (label, rows) in [
        ("dealt", &session.damage_dealt),
        ("taken", &session.damage_taken),
    ] {
        for row in rows {
            let source = match row.source_kind {
                super::Source::SelfActor => "self",
                super::Source::Monster => "monster",
                super::Source::Player => "player",
                super::Source::Environment => "environment",
                super::Source::Unknown => "unknown",
            };
            add(
                label,
                format!(
                    "{} / {} / {}: {}",
                    row.damage_type_key,
                    text(lang, source),
                    row.source_key.as_deref().unwrap_or("—"),
                    row.amount
                ),
            );
        }
    }
    add("healing-done", session.healing_done.to_string());
    add("healing-received", session.healing_received.to_string());
    if let Some(value) = valuation(session, prices) {
        add("loot-value", value.loot.to_string());
        add("supplies-value", value.supplies.to_string());
        add("balance", value.balance.to_string());
        add(
            "profit-rate",
            value
                .profit_per_hour
                .map_or_else(|| text(lang, "unavailable"), |n| n.to_string()),
        );
        add(
            "provenance",
            value.provenance.into_iter().collect::<Vec<_>>().join(", "),
        );
    } else {
        for label in ["loot-value", "supplies-value", "balance", "profit-rate"] {
            add(label, text(lang, "unavailable"));
        }
    }
    if full && flags.party {
        for segment in &hunt.parties {
            let party = &segment.ledger;
            add(
                "party-interval",
                format!("{} – {}", segment.started_at, segment.ended_at),
            );
            for (slot, loot) in &party.members {
                let owner = if *slot == party.subject_slot {
                    text(lang, "your-slot")
                } else {
                    String::new()
                };
                add(
                    "party-slot",
                    format!("{slot} {owner}").trim_end().to_owned(),
                );
                for (key, quantity) in loot {
                    add("loot", format!("{key}: {quantity}"));
                }
            }
            match split_party(party, session.character_id, prices) {
                Ok(transfers) if transfers.is_empty() => add("transfer", text(lang, "settled")),
                Ok(transfers) => {
                    for transfer in transfers {
                        add(
                            "transfer",
                            format!(
                                "{} → {}: {}",
                                transfer.from_slot, transfer.to_slot, transfer.gold
                            ),
                        );
                    }
                }
                Err(_) => add("transfer", text(lang, "unavailable")),
            }
        }
    }
    Card {
        id: session.session_id,
        lines,
        journal: hunt.drops.is_some(),
        drops: hunt
            .drops
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|drop| {
                let unknown = text(lang, "unavailable");
                format!(
                    "{} × {} · {} · {} · {} · {}",
                    drop.quantity,
                    drop.item_key,
                    drop.occurred_at,
                    drop.place_key.as_deref().unwrap_or(&unknown),
                    drop.monster_key.as_deref().unwrap_or(&unknown),
                    drop.rarity_rule_key
                )
            })
            .collect(),
    }
}

fn unavailable_page(lang: &str, list: bool) -> Response {
    page(lang, Vec::new(), list, "unavailable", String::new())
}

async fn index(
    State(state): State<Web>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let account = match owner(&state, &lang, &session).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    let filter = match query {
        Ok(Query(f)) => f,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let now = match now() {
        Ok(n) => n,
        Err(e) => return e.into_response(),
    };
    match state.read(account, now, &filter, &[]).await {
        Ok(hunts) => page(
            &lang,
            hunts
                .iter()
                .map(|hunt| card(&lang, hunt, state.prices.as_ref(), state.flags, false))
                .collect(),
            true,
            if hunts.is_empty() {
                "unavailable"
            } else {
                "applied-prefix"
            },
            String::new(),
        ),
        Err(Error::Unavailable) => unavailable_page(&lang, true),
        Err(error) => status(error).into_response(),
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailQuery {
    #[serde(default)]
    prices: Option<String>,
}

async fn detail(
    State(state): State<Web>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<DetailQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let account = match owner(&state, &lang, &session).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    let raw = match query {
        Ok(Query(q)) => q.prices.unwrap_or_default(),
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let quotes = match prices::parse(&raw) {
        Ok(quotes) => quotes,
        Err(error) => return status(error).into_response(),
    };
    let now = match now() {
        Ok(n) => n,
        Err(e) => return e.into_response(),
    };
    match state.read(account, now, &Filter::default(), &[id]).await {
        Ok(mut hunts) => {
            let [hunt] = hunts.as_mut_slice() else {
                return StatusCode::NOT_FOUND.into_response();
            };
            if !state.flags.party {
                hunt.parties.clear();
            }
            let prices = prices::Prices::for_hunt(
                &state.journal.pool,
                hunt,
                &quotes,
                state.prices.as_ref(),
                &prices::Labels {
                    custom: text(&lang, "custom-provenance"),
                    npc: text(&lang, "npc-provenance"),
                    bazaar: text(&lang, "bazaar-provenance"),
                },
            )
            .await;
            page(
                &lang,
                vec![card(&lang, hunt, &prices, state.flags, true)],
                false,
                "applied-prefix",
                raw,
            )
        }
        Err(error) => status(error).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    ids: Option<String>,
    first: Option<Uuid>,
    second: Option<Uuid>,
}
impl Selection {
    fn ids(self) -> Result<Vec<Uuid>, Error> {
        let ids = match (self.ids, self.first, self.second) {
            (Some(ids), None, None) if ids.len() <= 8 * 37 => ids
                .split(',')
                .map(|id| Uuid::parse_str(id).map_err(|_| Error::Invalid))
                .collect::<Result<Vec<_>, _>>()?,
            (None, Some(first), Some(second)) => vec![first, second],
            _ => return Err(Error::Invalid),
        };
        if !(2..=8).contains(&ids.len()) || ids.iter().any(|id| !v7(*id)) {
            return Err(Error::Invalid);
        }
        Ok(ids)
    }
}
async fn comparison(
    State(state): State<Web>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<Selection>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let account = match owner(&state, &lang, &session).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ids = match query.ok().and_then(|Query(q)| q.ids().ok()) {
        Some(ids) => ids,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let now = match now() {
        Ok(n) => n,
        Err(e) => return e.into_response(),
    };
    let hunts = match state.read(account, now, &Filter::default(), &ids).await {
        Ok(hunts) => hunts,
        Err(error) => return status(error).into_response(),
    };
    let sessions: Vec<Session> = hunts.iter().map(|hunt| hunt.session.clone()).collect();
    if let Err(error) = compare(&sessions, state.prices.as_ref()) {
        return status(error).into_response();
    }
    page(
        &lang,
        hunts
            .iter()
            .map(|hunt| card(&lang, hunt, state.prices.as_ref(), state.flags, false))
            .collect(),
        false,
        "comparison-results",
        String::new(),
    )
}

async fn private_link(
    State(state): State<Web>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    let account = match owner(&state, &lang, &session).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    let now = match now() {
        Ok(n) => n,
        Err(e) => return e.into_response(),
    };
    if state.projected.is_some() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    match state.read(account, now, &Filter::default(), &[id]).await {
        Ok(_) => {
            axum::response::Redirect::to(&format!("/{lang}/account/hunts/{id}")).into_response()
        }
        Err(error) => status(error).into_response(),
    }
}

/// G-9 v1 permits only owner navigation.
async fn share() -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

/// Mount under the existing staff guard. This page reports configuration only.
pub fn status_router() -> Router {
    Router::new()
        .route("/admin/hunt-journal", get(operator_status))
        .layer(middleware::from_fn(private_headers))
}

#[derive(Template)]
#[template(path = "playerdata/hunt_status.html", config = "src/hunts.askama.toml")]
struct OperatorPage<'a> {
    lang: &'a str,
    title: String,
    rows: Vec<Line>,
    note: String,
}

async fn operator_status(chrome: Option<Extension<oteryn_ui::Chrome>>) -> Response {
    let lang = chrome
        .as_ref()
        .map_or("pl", |Extension(chrome)| chrome.lang.as_str());
    let settings = crate::sessions::Settings::from_lookup(|key| std::env::var(key).ok());
    let page = OperatorPage {
        lang,
        title: text(lang, "operator-title"),
        rows: [
            ("operator-ingest", settings.ingest),
            ("operator-replay", settings.commands),
            ("operator-source", !settings.source.is_empty()),
            ("operator-party", crate::sessions::party_privacy_accepted()),
            (
                "operator-peer",
                std::env::var(crate::sessions::IDENTITY).is_ok_and(|s| !s.is_empty()),
            ),
            (
                "title",
                std::env::var("OTERYN_PLAYERDATA_HUNTS_READ").as_deref() == Ok("true"),
            ),
        ]
        .into_iter()
        .map(|(label, enabled)| Line {
            label: text(lang, label),
            value: text(lang, if enabled { "yes" } else { "no" }),
        })
        .collect(),
        note: text(lang, "operator-note"),
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../templates/playerdata/hunts.css"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn localized_pages_escape_untrusted_values() -> Result<(), Box<dyn std::error::Error>> {
        for (lang, title) in [
            ("en", "Hunt session analyzer"),
            ("pl", "Analizator sesji polowania"),
        ] {
            let cards = vec![Card {
                id: Uuid::now_v7(),
                drops: Vec::new(),
                journal: true,
                lines: vec![
                    Line {
                        label: text(lang, "provenance"),
                        value: "<script>alert(1)</script>".into(),
                    },
                    Line {
                        label: text(lang, "place"),
                        value: "place:".to_owned() + &"x".repeat(500),
                    },
                ],
            }];
            let response = page(lang, cards, true, "applied-prefix", String::new());
            let html = String::from_utf8(
                axum::body::to_bytes(response.into_body(), 1024 * 1024)
                    .await?
                    .to_vec(),
            )?;
            assert!(html.contains(title));
            assert!(!html.contains("<script>"));
            assert!(html.contains("&#60;script&#62;") || html.contains("&lt;script&gt;"));
            if let Ok(directory) = std::env::var("PORTAL_HUNT_RENDER_DIR") {
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("hunts-{lang}.html")),
                    html,
                )?;
            }
        }
        Ok(())
    }
}
