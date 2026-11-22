use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_playerdata::authority::CollectionEvidence;
use oteryn_playerdata::progress::{Authority, Payload, ProgressStore, Subject};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use oteryn_world::{WorldOptions, WorldStore};
use sqlx::PgPool;
use std::sync::Arc;

/// T72 must provide a freshly revalidated authority; no header can supply it.
#[derive(Clone)]
pub struct Ownership(pub Arc<dyn Authority + Send + Sync>);
impl std::fmt::Debug for Ownership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Ownership")
    }
}
impl Authority for Ownership {
    fn lock_mutation<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: Uuid,
        character: Uuid,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = oteryn_playerdata::progress::Result<()>> + Send + 'a>,
    > {
        self.0.lock_mutation(tx, account, character)
    }

    fn permits(&self, subject: Subject<'_>) -> bool {
        self.0.permits(subject)
    }
    fn source(&self) -> &str {
        self.0.source()
    }
    fn catalog_accepts(&self, family: &str, payload: &Payload) -> bool {
        self.0.catalog_accepts(family, payload)
    }
}
struct Unavailable;
impl Authority for Unavailable {
    fn permits(&self, _: Subject<'_>) -> bool {
        false
    }
    fn source(&self) -> &str {
        ""
    }
    fn catalog_accepts(&self, _: &str, _: &Payload) -> bool {
        false
    }
}
#[derive(Debug, Clone)]
struct HttpState {
    service: Collection,
    authority: Ownership,
    evidence: Option<EvidenceSources>,
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub(super) struct EvidenceSources {
    pub(super) inventory: String,
    pub(super) progress: String,
}
/// Does not register routes when disabled. All browser mutations retain the
/// shared synchronizer-token CSRF and current authenticated session middleware.
pub fn router(
    pool: PgPool,
    service: Collection,
    flags: Flags,
    authority: Ownership,
    csrf: CsrfConfig,
) -> Router {
    router_with_evidence(pool, service, flags, authority, csrf, None)
}

pub(super) fn router_with_evidence(
    pool: PgPool,
    service: Collection,
    flags: Flags,
    authority: Ownership,
    csrf: CsrfConfig,
    evidence: Option<EvidenceSources>,
) -> Router {
    if !flags.collection {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf = csrf;
    Router::new()
        .route("/{lang}/collection", get(index))
        .route("/{lang}/collection/{character}", get(page))
        .route("/{lang}/collection/{character}/decks", post(save))
        .route(
            "/{lang}/collection/{character}/cards/{id}/image",
            get(image),
        )
        .route("/{lang}/collection/decks/{id}/battle", post(start))
        .route(
            "/{lang}/collection/battles/{id}",
            get(battle_page).post(turn),
        )
        .route("/{lang}/collection/battles/{id}/forfeit", post(forfeit))
        .route("/{lang}/collection/decks/{id}/delete", post(delete_deck))
        .route_layer(middleware::from_fn(protect))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(4096))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
        .layer(middleware::from_fn(response_policy))
        .layer(axum::middleware::from_fn(crate::browser::deadline))
        .with_state(HttpState {
            service,
            authority,
            evidence,
            pool,
        })
}
pub fn configured_router(pool: PgPool) -> Router {
    let flags = Flags {
        collection: std::env::var("OTERYN_COLLECTION_ENABLED").as_deref() == Ok("true"),
    };
    let world = WorldStore::new(
        pool.clone(),
        WorldOptions {
            enabled: std::env::var("OTERYN_WORLD_ENABLED").as_deref() == Ok("true"),
        },
    );
    // Card unlocks and battle mutations still need accepted catalogue mappings.
    // The journal evidence view reads timestamps only.
    let progress = ProgressStore::new(pool.clone(), Default::default(), Vec::new());
    let service = Collection::new(pool.clone(), world, progress, flags);
    let evidence = (std::env::var("OTERYN_COLLECTION_JOURNAL_READ_ENABLED").as_deref()
        == Ok("true"))
    .then(|| EvidenceSources {
        inventory: std::env::var(oteryn_playerdata::journal::ITEMS_SOURCE).unwrap_or_default(),
        progress: std::env::var(oteryn_playerdata::journal::PROGRESS_SOURCE).unwrap_or_default(),
    });
    router_with_evidence(
        pool,
        service,
        flags,
        Ownership(Arc::new(Unavailable)),
        CsrfConfig {
            trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
                .ok()
                .into_iter()
                .collect(),
            ..CsrfConfig::default()
        },
        evidence,
    )
}
async fn protect(
    Extension(session): Extension<BrowserSession>,
    request: Request,
    next: Next,
) -> Response {
    let lang = request.uri().path().split('/').nth(1).unwrap_or_default();
    if !matches!(lang, "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if session.account_id().await.is_none() {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    }
    next.run(request).await
}
async fn response_policy(request: Request, next: Next) -> Response {
    let lang = if request.uri().path().starts_with("/pl/") {
        "pl"
    } else {
        "en"
    };
    let mut response = next.run(request).await;
    if response.status().is_client_error() || response.status().is_server_error() {
        let status = response.status();
        response = (status,Html(format!("<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><body><main><h1>{}</h1><a href=\"/{lang}/collection\">{}</a></main></body></html>",text(lang,"title"),text(lang,"unavailable"),text(lang,"title")))).into_response();
    }
    for (key, value) in [
        (header::CACHE_CONTROL, "private, no-store"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
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
            .insert(key, header::HeaderValue::from_static(value));
    }
    response.headers_mut().insert(
        "permissions-policy",
        header::HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    response
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
    };
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let Ok(language) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![language]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle
        .format_pattern(pattern, None, &mut Vec::new())
        .into_owned()
}
fn now() -> Result<u64> {
    u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| Error::Unavailable)
}
fn status(error: Error) -> StatusCode {
    StatusCode::from_u16(error.status()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
}
async fn account(session: &BrowserSession) -> std::result::Result<Uuid, StatusCode> {
    session.account_id().await.ok_or(StatusCode::FORBIDDEN)
}
#[derive(Template)]
#[template(
    path = "collection/evidence.html",
    config = "src/collection/askama.toml"
)]
struct EvidencePage<'a> {
    lang: &'a str,
    characters: Vec<(Uuid, String)>,
    evidence: Option<CollectionEvidence>,
}

impl EvidencePage<'_> {
    fn label(&self, key: &str) -> String {
        text(self.lang, key)
    }

    fn updated(&self, millis: &Option<i64>) -> String {
        millis
            .and_then(|ms| {
                time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000).ok()
            })
            .and_then(|time| {
                time.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            })
            .unwrap_or_else(|| self.label("unknown-update"))
    }

    fn html(&self) -> std::result::Result<Html<String>, StatusCode> {
        self.render()
            .map(Html)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    }
}

fn evidence_status(error: oteryn_playerdata::store::Error) -> StatusCode {
    use oteryn_playerdata::store::Error;
    match error {
        Error::Invalid => StatusCode::BAD_REQUEST,
        Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn index(
    State(s): State<HttpState>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> std::result::Result<Html<String>, StatusCode> {
    let characters = if s.evidence.is_some() {
        let account = account(&session).await?;
        let tx = crate::browser::begin(&s.pool)
            .await
            .map_err(|e| status(e.into()))?;
        match CollectionEvidence::characters(tx, account).await {
            Ok(characters) => characters,
            Err(oteryn_playerdata::store::Error::Unavailable) => Vec::new(),
            Err(error) => return Err(evidence_status(error)),
        }
    } else {
        Vec::new()
    };
    EvidencePage {
        lang: &lang,
        characters,
        evidence: None,
    }
    .html()
}
struct CardView {
    id: Uuid,
    name: String,
    unlocked: bool,
    state: String,
}
struct DeckView {
    id: Uuid,
    name: String,
}
#[derive(Template)]
#[template(path = "collection/index.html", config = "src/collection/askama.toml")]
struct Page {
    active: Option<Uuid>,
    resume: String,
    delete: String,
    forfeit: String,
    lang: String,
    character: Uuid,
    csrf: String,
    title: String,
    rules: String,
    cards_label: String,
    decks_label: String,
    name_label: String,
    choose: String,
    save: String,
    fight: String,
    skip: String,
    empty: String,
    cards: Vec<CardView>,
    decks: Vec<DeckView>,
}
async fn page(
    State(s): State<HttpState>,
    Path((lang, character)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> std::result::Result<Html<String>, StatusCode> {
    let account = account(&session).await?;
    if let Some(sources) = &s.evidence {
        let tx = crate::browser::begin(&s.pool)
            .await
            .map_err(|e| status(e.into()))?;
        let evidence = CollectionEvidence::read(
            tx,
            account,
            character,
            &sources.inventory,
            &sources.progress,
        )
        .await
        .map_err(evidence_status)?;
        return EvidencePage {
            lang: &lang,
            characters: Vec::new(),
            evidence: Some(evidence),
        }
        .html();
    }
    let cards = s
        .service
        .cards(account, character, &s.authority, now().map_err(status)?)
        .await
        .map_err(status)?;
    let decks = s
        .service
        .decks(account)
        .await
        .map_err(status)?
        .into_iter()
        .filter(|d| d.character == character)
        .map(|d| DeckView {
            id: d.id,
            name: d.name,
        })
        .collect();
    let cards = cards
        .into_iter()
        .map(|c| CardView {
            id: c.card.id,
            name: if lang == "pl" {
                c.card.name.pl
            } else {
                c.card.name.en
            },
            unlocked: c.unlocked,
            state: text(&lang, if c.unlocked { "unlocked" } else { "locked" }),
        })
        .collect();
    let page = Page {
        active: s
            .service
            .active_battle(account, character)
            .await
            .map_err(status)?,
        resume: text(&lang, "resume"),
        delete: text(&lang, "delete"),
        forfeit: text(&lang, "forfeit"),
        character,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        title: text(&lang, "title"),
        rules: text(&lang, "rules"),
        cards_label: text(&lang, "cards"),
        decks_label: text(&lang, "decks"),
        name_label: text(&lang, "name"),
        choose: text(&lang, "choose"),
        save: text(&lang, "save"),
        fight: text(&lang, "fight"),
        skip: text(&lang, "skip"),
        empty: text(&lang, "empty"),
        lang,
        cards,
        decks,
    };
    Ok(Html(
        page.render().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeckForm {
    _csrf: String,
    name: String,
    first: Uuid,
    second: Option<String>,
    third: Option<String>,
    id: Option<Uuid>,
}
async fn save(
    State(s): State<HttpState>,
    Path((lang, character)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(f): Form<DeckForm>,
) -> std::result::Result<Redirect, StatusCode> {
    s.service
        .save_deck(
            account(&session).await?,
            Deck {
                id: f.id.unwrap_or_else(Uuid::now_v7),
                character,
                name: f.name,
                cards: [Some(f.first), optional_id(f.second)?, optional_id(f.third)?]
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            &s.authority,
            now().map_err(status)?,
        )
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/collection/{character}")))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartForm {
    _csrf: String,
}
async fn start(
    State(s): State<HttpState>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(_): Form<StartForm>,
) -> std::result::Result<Redirect, StatusCode> {
    let id = s
        .service
        .start(
            account(&session).await?,
            id,
            &s.authority,
            now().map_err(status)?,
        )
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/collection/battles/{id}")))
}
#[derive(Template)]
#[template(path = "collection/battle.html", config = "src/collection/askama.toml")]
struct BattlePage {
    character: Uuid,
    lang: String,
    id: Uuid,
    csrf: String,
    title: String,
    round: String,
    turn: u32,
    outcome: String,
    active: bool,
    player_label: String,
    enemy_label: String,
    strike: String,
    guard: String,
    back: String,
    player: Vec<u16>,
    enemy: Vec<u16>,
}
async fn battle_page(
    State(s): State<HttpState>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> std::result::Result<Html<String>, StatusCode> {
    let state = s
        .service
        .battle(
            account(&session).await?,
            id,
            &s.authority,
            now().map_err(status)?,
        )
        .await
        .map_err(status)?;
    let key = match state.outcome {
        Outcome::Active => "active",
        Outcome::Victory => "victory",
        Outcome::Defeat => "defeat",
        Outcome::Draw => "draw",
    };
    let page = BattlePage {
        character: s
            .service
            .battle_character(account(&session).await?, id)
            .await
            .map_err(status)?,
        id,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        title: text(&lang, "fight"),
        round: text(&lang, "round"),
        turn: state.turn,
        outcome: text(&lang, key),
        active: state.outcome == Outcome::Active,
        player_label: text(&lang, "player"),
        enemy_label: text(&lang, "enemy"),
        strike: text(&lang, "strike"),
        guard: text(&lang, "guard"),
        back: text(&lang, "title"),
        lang,
        player: state.player.iter().map(|f| f.health).collect(),
        enemy: state.enemy.iter().map(|f| f.health).collect(),
    };
    Ok(Html(
        page.render().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnForm {
    _csrf: String,
    expected: u32,
    action: Action,
}
async fn turn(
    State(s): State<HttpState>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(f): Form<TurnForm>,
) -> std::result::Result<Redirect, StatusCode> {
    s.service
        .turn(
            account(&session).await?,
            id,
            f.expected,
            f.action,
            &s.authority,
            now().map_err(status)?,
        )
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/collection/battles/{id}")))
}
async fn image(
    State(s): State<HttpState>,
    Path((_, character, id)): Path<(String, Uuid, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> std::result::Result<Response, StatusCode> {
    let bytes = s
        .service
        .sprite(
            account(&session).await?,
            character,
            id,
            &s.authority,
            now().map_err(status)?,
        )
        .await
        .map_err(status)?;
    Ok(([(header::CONTENT_TYPE, "image/webp")], bytes).into_response())
}

fn optional_id(value: Option<String>) -> std::result::Result<Option<Uuid>, StatusCode> {
    value
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::parse_str(&s).map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()
}

async fn forfeit(
    State(s): State<HttpState>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(_): Form<StartForm>,
) -> std::result::Result<Redirect, StatusCode> {
    let character = s
        .service
        .forfeit(account(&session).await?, id)
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/collection/{character}")))
}

async fn delete_deck(
    State(s): State<HttpState>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(_): Form<StartForm>,
) -> std::result::Result<Redirect, StatusCode> {
    let character = s
        .service
        .delete_deck(account(&session).await?, id)
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/collection/{character}")))
}
