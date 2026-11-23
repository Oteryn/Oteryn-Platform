use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) fn configured_flags() -> Flags {
    Flags {
        pvp: std::env::var("OTERYN_COLLECTION_PVP_ENABLED").as_deref() == Ok("true"),
        sets: std::env::var("OTERYN_COLLECTION_SETS_ENABLED").as_deref() == Ok("true"),
    }
}

/// Every effect is behind the existing opt-in switches.
pub fn configured_router(pool: PgPool) -> Router {
    use crate::collection;
    use oteryn_world::{WorldOptions, WorldStore};

    let flags = configured_flags();
    let world_enabled = oteryn_world::configured_world_enabled();
    let world = WorldStore::new(
        pool.clone(),
        WorldOptions {
            enabled: world_enabled,
        },
    );
    let progress = oteryn_playerdata::progress::ProgressStore::new(
        pool.clone(),
        Default::default(),
        Vec::new(),
    );
    let catalogue = collection::Collection::new(
        pool.clone(),
        world,
        progress,
        collection::Flags {
            collection: world_enabled,
        },
    );
    let collection = PlatformCollection::new(catalogue, world_enabled);
    let payment = platform::payment(pool.clone(), flags.sets);
    let csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    router(Service::new(pool, flags, collection, payment), csrf)
}
pub fn router<C: Collection + 'static, P: SetPayment + 'static>(
    service: Service<C, P>,
    csrf: CsrfConfig,
) -> Router {
    if !service.flags.pvp && !service.flags.sets {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(service.pool.clone());
    sessions.csrf = csrf;
    Router::new()
        .route(
            "/{lang}/collection-pvp",
            get(page::<C, P>).post(action::<C, P>),
        )
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(8192))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(8192))
        .layer(middleware::from_fn(response_policy))
        .layer(axum::middleware::from_fn(crate::browser::deadline))
        .with_state(Arc::new(service))
}
fn text(lang: &str, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
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
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "collection/pvp.html", config = "src/collection/askama.toml")]
struct Page<'a> {
    lang: &'a str,
    actor: Uuid,
    available: bool,
    journal_available: bool,
    collection_home: bool,
    pvp: bool,
    sets: bool,
    csrf: String,
    standings: Option<Uuid>,
    tournaments: Vec<TournamentView>,
    rankings: Vec<Ranking>,
    offers: Vec<OfferView>,
    receipts: BTreeSet<Uuid>,
    battles: Vec<BattleView>,
    decks: Vec<DeckView>,
    cards: Vec<CardView>,
}

impl Page<'_> {
    fn label(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

#[derive(sqlx::FromRow)]
struct DeckView {
    id: Uuid,
    name: String,
    removable: bool,
}

struct CardView {
    id: Uuid,
    name: String,
    quantity: u64,
}

struct OfferView {
    id: Uuid,
    version: i64,
    price: i64,
    request: Uuid,
    owned: bool,
    cards: Vec<CardView>,
}

struct TournamentView {
    id: Uuid,
    status: String,
    capacity: i32,
    entrants: i64,
    joined: bool,
    can_join: bool,
    can_start: bool,
}

struct BattleView {
    id: Uuid,
    opponent: Uuid,
    status: &'static str,
    incoming: bool,
    revision: i64,
    tournament: Option<Uuid>,
    turn: Option<TurnView>,
}

#[derive(Deserialize)]
struct TurnView {
    round: u32,
    your_health: u16,
    opponent_health: u16,
    waiting: bool,
}

#[derive(Deserialize)]
struct Standings {
    standings: Option<Uuid>,
}

async fn page<C: Collection, P: SetPayment>(
    State(s): State<Arc<Service<C, P>>>,
    Path(lang): Path<String>,
    Query(query): Query<Standings>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(actor) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let result = async {
        let mut tx = crate::browser::begin(&s.pool).await?;
        if !oteryn_wallet::value_eligible_snapshot_in(&mut tx, actor).await? {
            return Err(Error::Refused);
        }
        let catalogue = if s.collection.available() {
            card_names(&mut tx, &lang).await?
        } else {
            BTreeMap::new()
        };
        let available = s.collection.available()
            && (!s.collection.catalogue_required() || !catalogue.is_empty());
        let journal_available = s.collection.journal_available();
        let purchases: Vec<(Uuid, Value)> = if s.flags.sets || (s.flags.pvp && available) {
            sqlx::query_as("SELECT set_id,cards FROM collection_pvp_purchases WHERE account_id=$1")
                .bind(actor)
                .fetch_all(&mut *tx)
                .await?
        } else {
            Vec::new()
        };
        let receipts = purchases.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>();
        let owned_cards = purchases.iter()
            .flat_map(|(_, cards)| cards.as_object().into_iter().flat_map(|cards| cards.keys()))
            .filter_map(|key| key.parse::<Uuid>().ok())
            .collect::<BTreeSet<_>>();
        let cards = owned_cards.into_iter().filter_map(|id| Some(CardView {
            id,
            name: catalogue.get(&id)?.clone(),
            quantity: 1,
        })).collect();
        let decks = if s.flags.pvp && available {
            sqlx::query_as("SELECT id,name,true AS removable FROM collection_pvp_owned_decks WHERE account_id=$1
                UNION ALL SELECT id,name,false AS removable FROM collection_decks WHERE account_id=$1 AND $2
                ORDER BY name,id")
                .bind(actor)
                .bind(journal_available)
                .fetch_all(&mut *tx).await?
        } else {
            Vec::new()
        };
        tx.commit().await?;
        let mut rankings = Vec::new();
        let mut tournaments = Vec::new();
        let mut battles = Vec::new();
        if s.flags.pvp && available {
            tournaments = s.tournaments(actor).await?.into_iter().map(|row| TournamentView {
                id: row.id,
                can_join: row.status == "registration" && !row.joined && row.entrants < i64::from(row.capacity),
                can_start: row.status == "registration" && row.organizer == actor && row.entrants >= 2,
                status: row.status,
                entrants: row.entrants,
                capacity: row.capacity,
                joined: row.joined,
            }).collect();
            if query.standings.is_some_and(|id| !tournaments.iter().any(|row| row.id == id)) {
                return Err(Error::Refused);
            }
            rankings = s.rankings(query.standings).await?;
            let rows: Vec<Battle> = sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id
                FROM collection_pvp_battles WHERE $1 IN(left_account,right_account)
                ORDER BY (status='active') DESC,created_at DESC LIMIT 50")
                .bind(actor).fetch_all(&s.pool).await?;
            battles = rows.into_iter().map(|row| {
                let left = actor == row.left_account;
                let turn = if matches!(row.status.as_str(), "active" | "completed") {
                    s.collection.view(&row.state, if left { Side::Left } else { Side::Right })
                        .ok()
                        .and_then(|view| serde_json::from_value(view).ok())
                } else {
                    None
                };
                BattleView {
                    id: row.id,
                    opponent: if left { row.right_account } else { row.left_account },
                    incoming: !left && row.status == "pending",
                    status: match row.status.as_str() {
                        "pending" => "pending",
                        "declined" => "declined",
                        "active" => "active",
                        "completed" if row.winner == Some(actor) => "won",
                        "completed" if row.winner.is_some() => "lost",
                        "completed" => "draw",
                        _ => "unavailable",
                    },
                    revision: row.revision,
                    tournament: row.tournament_id,
                    turn,
                }
            }).collect();
        }
        let offers = if s.flags.sets && available {
            s.sets().await?.into_iter().filter_map(|row| {
                let cards = row.cards.as_object()?.iter().map(|(key, quantity)| {
                    let id: Uuid = key.parse().ok()?;
                    Some(CardView { id, name: catalogue.get(&id)?.clone(), quantity: quantity.as_u64()? })
                }).collect::<Option<Vec<_>>>()?;
                Some(OfferView {
                    id: row.id, version: row.version, price: row.price, request: Uuid::now_v7(),
                    owned: receipts.contains(&row.id), cards,
                })
            }).collect()
        } else {
            Vec::new()
        };
        Page {
            lang: &lang, actor, available, pvp: s.flags.pvp, sets: s.flags.sets,
            journal_available, collection_home: std::env::var("OTERYN_COLLECTION_ENABLED").as_deref() == Ok("true"), csrf: session.csrf_token().await?, standings: query.standings,
            tournaments, rankings, offers, receipts, battles, decks, cards,
        }.render().map_err(|_| Error::Unavailable)
    }.await;
    match result {
        Ok(html) => Html(html).into_response(),
        Err(error) => failure(&lang, error),
    }
}

async fn card_names(
    tx: &mut Transaction<'_, Postgres>,
    lang: &str,
) -> Result<BTreeMap<Uuid, String>> {
    let rows = sqlx::query_as(
        "SELECT c.id,c.definition->'name'->>$1 AS name FROM collection_cards c
         JOIN world_active_version a ON a.version_id::text=c.definition->>'world_version'
         JOIN world_entities e ON e.version_id=a.version_id AND e.kind=c.definition->>'kind'
             AND e.key=c.definition->>'key' AND e.publishable
         JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed
         WHERE c.definition->'name'->>$1 IS NOT NULL ORDER BY c.id LIMIT 1001",
    )
    .bind(lang)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 1000 {
        return Err(Error::Unavailable);
    }
    Ok(rows.into_iter().collect())
}

#[derive(Deserialize)]
struct Command {
    op: String,
    id: Option<String>,
    opponent: Option<String>,
    deck: Option<String>,
    revision: Option<String>,
    action: Option<String>,
    capacity: Option<String>,
    version: Option<String>,
    price: Option<String>,
    request: Option<String>,
    name: Option<String>,
    card1: Option<String>,
    card2: Option<String>,
    card3: Option<String>,
}
fn parse<T: std::str::FromStr>(value: &Option<String>) -> Result<T> {
    value
        .as_ref()
        .ok_or(Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)
}
async fn action<C: Collection, P: SetPayment>(
    State(s): State<Arc<Service<C, P>>>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Form(command): Form<Command>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Refused);
    };
    let c = command;
    let result = async {
        match c.op.as_str() {
            "challenge" => {
                s.challenge(actor, parse(&c.opponent)?, parse(&c.deck)?)
                    .await?;
            }
            "accept" => s.accept(actor, parse(&c.id)?, parse(&c.deck)?).await?,
            "decline" => s.decline(actor, parse(&c.id)?).await?,
            "play" => {
                s.play(
                    actor,
                    parse(&c.id)?,
                    parse(&c.revision)?,
                    serde_json::from_str(c.action.as_deref().ok_or(Error::Invalid)?)
                        .map_err(|_| Error::Invalid)?,
                )
                .await?
            }
            "create" => {
                s.create_tournament(actor, parse(&c.capacity)?).await?;
            }
            "join" => s.join(actor, parse(&c.id)?, parse(&c.deck)?).await?,
            "start" => s.start(actor, parse(&c.id)?).await?,
            "save-deck" => {
                let cards = [c.card1, c.card2, c.card3]
                    .into_iter()
                    .flatten()
                    .filter(|value| !value.is_empty())
                    .map(|value| value.parse().map_err(|_| Error::Invalid))
                    .collect::<Result<Vec<_>>>()?;
                s.save_deck(actor, c.name.as_deref().ok_or(Error::Invalid)?, &cards)
                    .await?;
            }
            "delete-deck" => s.delete_deck(actor, parse(&c.id)?).await?,
            "buy" => {
                s.buy_set(
                    actor,
                    parse(&c.id)?,
                    parse(&c.version)?,
                    parse(&c.price)?,
                    parse(&c.request)?,
                )
                .await?
            }
            _ => return Err(Error::Invalid),
        }
        Ok(())
    }
    .await;
    match result {
        Ok(()) => Redirect::to(&format!("/{lang}/collection-pvp")).into_response(),
        Err(error) => failure(&lang, error),
    }
}
fn failure(lang: &str, error: Error) -> Response {
    let status = match error {
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Refused => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (
        status,
        [("cache-control", "private, no-store")],
        text(lang, "error"),
    )
        .into_response()
}

async fn response_policy(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let lang = if request.uri().path().starts_with("/pl/") {
        "pl"
    } else {
        "en"
    };
    let mut response = next.run(request).await;
    if response.status().is_client_error() || response.status().is_server_error() {
        let key = if response.status() == StatusCode::SERVICE_UNAVAILABLE {
            "unavailable"
        } else {
            "error"
        };
        response = (response.status(), text(lang, key)).into_response();
    }
    for (name, value) in [
        ("cache-control", "private, no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::header::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
