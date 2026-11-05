//! SSR owner routes. Authentication is resolved by the shared browser middleware.
use super::store::Repository;
use super::{Error, Filter, Flags, Pricing, Session, Unpriced, compare, split_loot, valuation};
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
    repo: Repository,
    flags: Flags,
    prices: Arc<dyn Pricing>,
}

/// Prices are injected through T74's interface. Until then values stay unavailable.
/// Party capture remains off until its privacy scope has been accepted.
pub fn router(pool: PgPool, flags: Flags, prices: Arc<dyn Pricing>) -> Router {
    let sessions = SessionLayer::new(pool.clone());
    Router::new()
        .route("/{lang}/account/hunts", get(index))
        .route("/{lang}/account/hunts/compare", get(comparison))
        .route("/{lang}/account/hunts/{id}", get(detail))
        .route("/{lang}/account/hunts/{id}/share", get(share).post(share))
        .with_state(Web {
            repo: Repository::new(pool, flags),
            flags,
            prices,
        })
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .route_layer(middleware::from_fn(
            move |request: axum::extract::Request, next: Next| async move {
                if !flags.read || request.uri().path().ends_with("/share") {
                    StatusCode::SERVICE_UNAVAILABLE.into_response()
                } else {
                    next.run(request).await
                }
            },
        ))
        .route("/static/playerdata/hunts.css", get(styles))
        .layer(middleware::from_fn(private_headers))
}

pub fn configured_router(pool: PgPool) -> Router {
    router(
        pool,
        Flags {
            read: std::env::var("OTERYN_PLAYERDATA_HUNTS_READ").is_ok_and(|v| v == "true"),
            party: false,
        },
        Arc::new(Unpriced),
    )
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
}
#[derive(Template)]
#[template(path = "playerdata/hunts.html", config = "src/hunts.askama.toml")]
struct Page {
    lang: String,
    title: String,
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
    share: String,
    split: String,
    cards: Vec<Card>,
}
fn page(lang: &str, cards: Vec<Card>) -> Response {
    let page = Page {
        lang: lang.to_owned(),
        title: text(lang, "title"),
        empty: text(lang, "empty"),
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
        share: text(lang, "share-disabled"),
        split: text(lang, "split-note"),
        cards,
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn card(lang: &str, session: &Session, prices: &dyn Pricing, flags: Flags) -> Card {
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
            add(label, format!("{key}: {amount}"));
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
        add("balance", text(lang, "unavailable"));
    }
    if session.party.is_some() {
        match split_loot(session, flags, prices) {
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
    Card {
        id: session.session_id,
        lines,
    }
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
    match state.repo.list(account, now, &filter).await {
        Ok(sessions) => page(
            &lang,
            sessions
                .iter()
                .map(|s| card(&lang, s, state.prices.as_ref(), state.flags))
                .collect(),
        ),
        Err(error) => status(error).into_response(),
    }
}
async fn detail(
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
    match state.repo.detail(account, now, id).await {
        Ok(s) => page(
            &lang,
            vec![card(&lang, &s, state.prices.as_ref(), state.flags)],
        ),
        Err(error) => status(error).into_response(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    ids: String,
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
    let selection = match query {
        Ok(Query(s)) => s,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let ids: Result<Vec<Uuid>, _> = selection.ids.split(',').map(str::parse).collect();
    let ids = match ids {
        Ok(ids) if (2..=8).contains(&ids.len()) => ids,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let now = match now() {
        Ok(n) => n,
        Err(e) => return e.into_response(),
    };
    let mut sessions = Vec::new();
    for id in ids {
        match state.repo.detail(account, now, id).await {
            Ok(s) => sessions.push(s),
            Err(error) => return status(error).into_response(),
        }
    }
    if let Err(error) = compare(&sessions, state.prices.as_ref()) {
        return status(error).into_response();
    }
    page(
        &lang,
        sessions
            .iter()
            .map(|s| card(&lang, s, state.prices.as_ref(), state.flags))
            .collect(),
    )
}
/// G-9 expressly denies bearer disclosure under v1, regardless of any flag.
async fn share() -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
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
            let response = page(lang, cards);
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
