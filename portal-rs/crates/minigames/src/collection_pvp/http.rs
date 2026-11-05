use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use std::sync::Arc;

/// No routes are exposed unless explicitly enabled. The default adapter denies
/// gameplay and purchasing until authoritative T90 and wallet wiring is supplied.
pub fn configured_router(pool: PgPool) -> Router {
    let flags = Flags {
        pvp: std::env::var("OTERYN_COLLECTION_PVP_ENABLED").as_deref() == Ok("true"),
        sets: std::env::var("OTERYN_COLLECTION_SETS_ENABLED").as_deref() == Ok("true"),
    };
    let csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    router(Service::new(pool, flags, Unavailable, Unavailable), csrf)
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
#[template(
    source = r##"<!doctype html>
<html lang="{{ lang }}" data-theme="dark"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"><link rel="stylesheet" href="/static/css/app.css"></head>
<body class="op-theme"><main class="op-stack"><h1>{{ title }}</h1><p>{{ notice }}</p>
<h2>{{ rankings_label }}</h2><ol>{% for row in rankings %}<li>{{ loop.index }}: {{ row.points }} / {{ row.played }}</li>{% endfor %}</ol>
<h2>{{ sets_label }}</h2>{% for set in sets %}<section class="op-panel op-stack"><h2>{{ set.id }}</h2><p>{{ set.version }} / {{ set.price }}</p><p>{{ set.cards }}</p></section>{% endfor %}
<h2>{{ tournaments_label }}</h2>{% for tournament in tournaments %}<section class="op-panel op-stack"><h3>{{ tournament.id }}</h3><p>{{ tournament.entrants }} / {{ tournament.capacity }}</p></section>{% endfor %}<h2>{{ battles_label }}</h2>{% for battle in battles %}<section class="op-panel op-stack"><h2>{{ battle.id }}</h2><p>{{ battle.revision }}</p></section>{% endfor %}
{% if available %}<form method="post" class="op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><label for="op">{{ operation }}</label><select class="op-input" id="op" name="op">{% for option in options %}<option value="{{ option.0 }}">{{ option.1 }}</option>{% endfor %}</select>
{% for field in fields %}<div class="op-field"><label for="{{ field.0 }}">{{ field.1 }}</label><input class="op-input" id="{{ field.0 }}" name="{{ field.0 }}" maxlength="4096"></div>{% endfor %}
<button class="op-button" type="submit">{{ submit }}</button></form>{% endif %}</main></body></html>
"##,
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    available: bool,
    csrf: String,
    title: String,
    notice: String,
    submit: String,
    operation: String,
    options: Vec<(&'static str, String)>,
    fields: Vec<(&'static str, String)>,
    rankings_label: String,
    sets_label: String,
    tournaments_label: String,
    battles_label: String,
    tournaments: Vec<Tournament>,
    rankings: Vec<Ranking>,
    sets: Vec<PublishedSet>,
    battles: Vec<Battle>,
}
async fn page<C: Collection, P: SetPayment>(
    State(s): State<Arc<Service<C, P>>>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(actor) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let result = async {
        let mut tx = s.pool.begin().await?;
        Service::<C,P>::active(&mut tx,actor).await?;
        tx.commit().await?;
        let rankings = if s.flags.pvp && s.collection.available() {
            s.rankings(None).await?
        } else { vec![] };
        let sets = if s.flags.sets { s.sets().await? } else { vec![] };
        let battles = if s.flags.pvp {
            sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id FROM collection_pvp_battles WHERE $1 IN(left_account,right_account) ORDER BY created_at DESC LIMIT 50")
                .bind(actor).fetch_all(&s.pool).await?
        } else { vec![] };
        let tournaments=if s.flags.pvp && s.collection.available(){s.tournaments(actor).await?}else{vec![]};
        let csrf = session.csrf_token().await?;
        let options = ["challenge","accept","decline","play","create","join","start","buy"]
            .into_iter()
            .filter(|op| if *op=="buy" {s.flags.sets} else {s.flags.pvp})
            .map(|op|(op,text(&lang,op))).collect();
        let fields = ["id","opponent","deck","revision","action","capacity","version","price","request"]
            .into_iter().map(|key|(key,text(&lang,key))).collect();
        Page {
            lang: &lang,
            available: s.collection.available(),
            csrf,
            title: text(&lang,"title"),
            notice: if s.collection.available() {text(&lang,"notice")} else {text(&lang,"unavailable")},
            submit: text(&lang,"submit"),
            operation: text(&lang,"operation"),
            rankings_label: text(&lang,"rankings"),
            sets_label: text(&lang,"sets"),
            tournaments_label: text(&lang,"tournaments"),
            battles_label: text(&lang,"battles"),
            tournaments, options, fields, rankings, sets, battles,
        }.render().map_err(|_|Error::Unavailable)
    }.await;
    match result {
        Ok(html) => ([("cache-control", "private, no-store")], Html(html)).into_response(),
        Err(error) => failure(&lang, error),
    }
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
        response = (response.status(), text(lang, "error")).into_response();
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
