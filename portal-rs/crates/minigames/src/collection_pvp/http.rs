use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

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
        None,
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
    next_tournament: Option<Uuid>,
    tournaments_before: bool,
    rankings: Vec<Ranking>,
    offers: Vec<OfferView>,
    receipts: Vec<Uuid>,
    next_offer: Option<Uuid>,
    next_receipt: Option<Uuid>,
    offers_before: bool,
    receipts_before: bool,
    battles: Vec<BattleView>,
    next_battle: Option<Uuid>,
    battles_before: bool,
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
struct PageQuery {
    standings: Option<Uuid>,
    battles_before: Option<Uuid>,
    tournaments_before: Option<Uuid>,
    offers_before: Option<Uuid>,
    receipts_before: Option<Uuid>,
}

async fn page<C: Collection, P: SetPayment>(
    State(s): State<Arc<Service<C, P>>>,
    Path(lang): Path<String>,
    Query(query): Query<PageQuery>,
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
        let available = s.collection.available()
            && (!s.collection.catalogue_required()
                || !card_names(&mut tx, &lang, None, None, 1).await?.is_empty());
        let journal_available = s.collection.journal_available();
        let cards = if s.flags.pvp && available {
            card_names(&mut tx, &lang, Some(actor), None, i64::MAX).await?
                .into_iter().map(|(id, name)| CardView { id, name, quantity: 1 }).collect()
        } else {
            Vec::new()
        };
        let (offers, next_offer) = if s.flags.sets && available {
            offer_page(&mut tx, &lang, actor, query.offers_before).await?
        } else {
            (Vec::new(), None)
        };
        let mut receipts: Vec<Uuid> = if s.flags.sets {
            sqlx::query_scalar("SELECT DISTINCT set_id FROM collection_pvp_purchases
                 WHERE account_id=$1 AND ($2::uuid IS NULL OR set_id<$2)
                 ORDER BY set_id DESC LIMIT 51")
                .bind(actor).bind(query.receipts_before)
                .fetch_all(&mut *tx).await?
        } else {
            Vec::new()
        };
        let next_receipt = if receipts.len() > 50 {
            receipts.pop();
            receipts.last().copied()
        } else {
            None
        };
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
        let mut next_tournament = None;
        let mut next_battle = None;
        if s.flags.pvp && available {
            let mut rows = s.tournament_page(actor, query.tournaments_before, 51).await?;
            if rows.len() > 50 {
                rows.pop();
                next_tournament = rows.last().map(|row| row.id);
            }
            tournaments = rows.into_iter().map(|row| TournamentView {
                id: row.id,
                can_join: row.status == "registration" && !row.joined && row.entrants < i64::from(row.capacity),
                can_start: row.status == "registration" && row.organizer == actor && row.entrants >= 2,
                status: row.status,
                entrants: row.entrants,
                capacity: row.capacity,
                joined: row.joined,
            }).collect();
            rankings = s.visible_rankings(actor, query.standings).await?;
            let mut rows: Vec<Battle> = sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id
                FROM collection_pvp_battles WHERE $1 IN(left_account,right_account)
                AND ($2::uuid IS NULL OR id<$2) ORDER BY id DESC LIMIT 51")
                .bind(actor).bind(query.battles_before).fetch_all(&s.pool).await?;
            if rows.len() > 50 {
                rows.pop();
                next_battle = rows.last().map(|row| row.id);
            }
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
        Page {
            lang: &lang, actor, available, pvp: s.flags.pvp, sets: s.flags.sets,
            journal_available, collection_home: std::env::var("OTERYN_COLLECTION_ENABLED").as_deref() == Ok("true"), csrf: session.csrf_token().await?, standings: query.standings,
            tournaments, next_tournament, rankings, offers, receipts, battles, next_battle, decks, cards,
            tournaments_before: query.tournaments_before.is_some(), battles_before: query.battles_before.is_some(),
            offers_before: query.offers_before.is_some(), receipts_before: query.receipts_before.is_some(), next_offer, next_receipt,
        }.render().map_err(|_| Error::Unavailable)
    }.await;
    match result {
        Ok(html) => Html(html).into_response(),
        Err(error) => failure(error),
    }
}

async fn card_names(
    tx: &mut Transaction<'_, Postgres>,
    lang: &str,
    owner: Option<Uuid>,
    ids: Option<&[Uuid]>,
    limit: i64,
) -> Result<BTreeMap<Uuid, String>> {
    let rows = sqlx::query_as(
        "SELECT c.id,c.definition->'name'->>$1 AS name FROM collection_cards c
         JOIN world_active_version a ON a.version_id::text=c.definition->>'world_version'
         JOIN world_entities e ON e.version_id=a.version_id AND e.kind=c.definition->>'kind'
             AND e.key=c.definition->>'key' AND e.publishable
         JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed
         WHERE c.definition->'name'->>$1 IS NOT NULL
         AND ($2::uuid IS NULL OR EXISTS(SELECT 1 FROM collection_pvp_purchases p
                                       WHERE p.account_id=$2 AND p.cards ? c.id::text))
         AND ($3::uuid[] IS NULL OR c.id=ANY($3)) ORDER BY c.id LIMIT $4",
    )
    .bind(lang)
    .bind(owner)
    .bind(ids)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().collect())
}

#[derive(sqlx::FromRow)]
struct OfferRow {
    id: Uuid,
    version: i64,
    price: i64,
    cards: Value,
    owned: bool,
}

async fn offer_page(
    tx: &mut Transaction<'_, Postgres>,
    lang: &str,
    actor: Uuid,
    before: Option<Uuid>,
) -> Result<(Vec<OfferView>, Option<Uuid>)> {
    let mut rows: Vec<OfferRow> = sqlx::query_as(
        "SELECT s.id,s.version,s.price,s.cards,
            EXISTS(SELECT 1 FROM collection_pvp_purchases p WHERE p.account_id=$1 AND p.set_id=s.id) AS owned
         FROM collection_pvp_sets s WHERE s.published AND ($2::uuid IS NULL OR s.id<$2)
         ORDER BY s.id DESC LIMIT 11",
    )
    .bind(actor)
    .bind(before)
    .fetch_all(&mut **tx)
    .await?;
    let next_offer = if rows.len() > 10 {
        rows.pop();
        rows.last().map(|row| row.id)
    } else {
        None
    };
    let mut offers = Vec::new();
    for row in rows {
        if validate_cards(&row.cards).is_err() {
            continue;
        }
        let Some(contents) = row.cards.as_object().and_then(|cards| {
            cards
                .iter()
                .map(|(key, quantity)| {
                    let id: Uuid = key.parse().ok()?;
                    (id.to_string() == *key).then_some((id, quantity.as_u64()?))
                })
                .collect::<Option<Vec<_>>>()
        }) else {
            continue;
        };
        offers.push((row, contents));
    }
    let ids: Vec<Uuid> = offers
        .iter()
        .flat_map(|(_, cards)| cards.iter().map(|(id, _)| *id))
        .collect();
    let names = card_names(tx, lang, None, Some(&ids), i64::MAX).await?;
    let offers = offers
        .into_iter()
        .filter_map(|(row, cards)| {
            let cards = cards
                .into_iter()
                .map(|(id, quantity)| {
                    Some(CardView {
                        id,
                        quantity,
                        name: names.get(&id)?.clone(),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(OfferView {
                id: row.id,
                version: row.version,
                price: row.price,
                request: Uuid::now_v7(),
                owned: row.owned,
                cards,
            })
        })
        .collect();
    Ok((offers, next_offer))
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
        return failure(Error::Refused);
    };
    let c = command;
    let result = async {
        charge_action(&s.pool, actor).await?;
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
        Err(error) => failure(error),
    }
}

async fn charge_action(pool: &PgPool, actor: Uuid) -> Result<()> {
    let mut tx = crate::browser::begin(pool).await?;
    let key = Sha256::digest(actor.as_bytes()).to_vec();
    let (count, retry_after): (i64, i64) = sqlx::query_as(
        "WITH clock AS (SELECT statement_timestamp() AS instant),
         charged AS (
             INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
             SELECT 'collection-pvp-action',$1,date_trunc('minute',instant),1 FROM clock
             ON CONFLICT(limiter_name,key_hash,window_start)
                 DO UPDATE SET count=least(web_rate_limits.count,60)+1
             RETURNING count,window_start
         )
         SELECT count,ceil(extract(epoch FROM window_start + interval '1 minute' - instant))::bigint
         FROM charged CROSS JOIN clock",
    )
    .bind(key)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if count > 60 {
        return Err(Error::Limited(retry_after));
    }
    Ok(())
}

fn failure(error: Error) -> Response {
    let status = match error {
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Refused => StatusCode::FORBIDDEN,
        Error::Limited(_) => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    let mut response = status.into_response();
    if let Error::Limited(seconds) = error
        && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
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
        let key = match response.status() {
            StatusCode::SERVICE_UNAVAILABLE => "unavailable",
            StatusCode::TOO_MANY_REQUESTS => "try-later",
            _ => "error",
        };
        let (parts, body) = text(lang, key).into_response().into_parts();
        response.headers_mut().remove(header::CONTENT_LENGTH);
        response.headers_mut().extend(parts.headers);
        *response.body_mut() = body;
    }
    for (name, value) in [
        ("cache-control", "private, no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "same-origin"),
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
