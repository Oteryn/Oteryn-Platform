use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use oteryn_world::{WorldOptions, WorldStore};
use sqlx::PgPool;

#[derive(Debug, Clone)]
struct HttpState {
    puzzles: Puzzles,
}
/// A disabled feature mounts no routes or middleware.
pub fn router(pool: PgPool, world: WorldStore, flags: Flags, csrf: CsrfConfig) -> Router {
    if !flags.daily_puzzles {
        return Router::new();
    }
    let state = HttpState {
        puzzles: Puzzles::new(pool.clone(), world, flags),
    };
    let mut sessions = SessionLayer::new(pool);
    sessions.csrf = csrf;
    Router::new()
        .route("/{lang}/puzzles", get(page))
        .route("/{lang}/puzzles/{id}/answer", axum::routing::post(answer))
        .route("/{lang}/puzzles/{id}/image", get(image))
        .route_layer(middleware::from_fn(protect))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(4096))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
        .layer(middleware::from_fn(response_policy))
        .with_state(state)
}
pub fn configured_router(pool: PgPool) -> Router {
    let flags = Flags {
        daily_puzzles: std::env::var("OTERYN_DAILY_PUZZLES_ENABLED").as_deref() == Ok("true"),
    };
    let world = WorldStore::new(
        pool.clone(),
        WorldOptions {
            enabled: std::env::var("OTERYN_WORLD_ENABLED").as_deref() == Ok("true"),
        },
    );
    let csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    router(pool, world, flags, csrf)
}
async fn protect(
    Extension(session): Extension<BrowserSession>,
    request: Request,
    next: Next,
) -> Response {
    let lang = request.uri().path().split('/').nth(1).unwrap_or_default();
    if locale(lang).is_err() {
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
        response = (status, axum::response::Html(format!(
            "<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><body><main><h1>{}</h1><a href=\"/{lang}/puzzles\">{}</a></main></body></html>",
            text(lang,"title"),text(lang,"error"),text(lang,"back")
        ))).into_response();
    }
    for (name, value) in [
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
            .insert(name, header::HeaderValue::from_static(value));
    }
    response
}
fn locale(lang: &str) -> Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
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
    bundle
        .get_message(key)
        .and_then(|message| message.value())
        .map(|pattern| {
            bundle
                .format_pattern(pattern, None, &mut vec![])
                .into_owned()
        })
        .unwrap_or_default()
}
fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::BAD_REQUEST,
        Error::Forbidden => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Unavailable | Error::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
async fn account(session: &BrowserSession) -> Result<Uuid, StatusCode> {
    session.account_id().await.ok_or(StatusCode::UNAUTHORIZED)
}
struct Card {
    id: Uuid,
    heading: String,
    question: String,
    clue: String,
    choices: Vec<String>,
    image: bool,
    result: Option<String>,
}
#[derive(Template)]
#[template(
    source = r##"<!doctype html>
<html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"><link rel="stylesheet" href="/static/css/app.css"></head>
<body class="op-theme"><header class="site-header op-stack"><a href="#main">{{ skip }}</a><nav class="op-row"><a href="/{{ lang }}/">{{ home }}</a><a href="/{{ other }}/puzzles" hreflang="{{ other }}" lang="{{ other }}">{{ other_name }}</a></nav></header>
<main id="main" tabindex="-1" class="op-stack"><h1>{{ title }}</h1><p>{{ rules }}</p>
<section class="op-panel op-stack" aria-label="{{ current_label }}"><p>{{ current_label }}: {{ current }}</p><p>{{ best_label }}: {{ best }}</p><h2>{{ badges_label }}</h2>{% if badges.is_empty() %}<p>{{ none }}</p>{% else %}<ul>{% for badge in badges %}<li>{{ badge }}</li>{% endfor %}</ul>{% endif %}</section>
{% if cards.is_empty() %}<p>{{ empty }}</p>{% endif %}
{% for card in cards %}<section class="op-card op-stack" aria-labelledby="heading-{{ card.id }}"><h2 id="heading-{{ card.id }}">{{ card.heading }}</h2><p>{{ card.question }}</p>
{% if card.image %}<figure><img src="/{{ lang }}/puzzles/{{ card.id }}/image" width="256" height="256" alt="{{ card.clue }}"><figcaption>{{ card.clue }}</figcaption></figure>{% else %}<p>{{ card.clue }}</p>{% endif %}
{% match card.result %}{% when Some with (result) %}<p role="status">{{ result }}</p>{% when None %}
<form method="post" action="/{{ lang }}/puzzles/{{ card.id }}/answer" class="op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><div class="op-field"><label for="choice-{{ card.id }}">{{ choose }}</label><select class="op-input" name="choice" id="choice-{{ card.id }}" required><option value="">{{ choose }}</option>{% for choice in card.choices %}<option value="{{ loop.index0 }}">{{ choice }}</option>{% endfor %}</select></div><button class="op-button" type="submit">{{ submit }}</button></form>
{% endmatch %}</section>{% endfor %}</main></body></html>"##,
    ext = "html"
)]
struct Page {
    lang: String,
    other: &'static str,
    other_name: String,
    title: String,
    skip: String,
    home: String,
    rules: String,
    current_label: String,
    best_label: String,
    badges_label: String,
    none: String,
    empty: String,
    choose: String,
    submit: String,
    csrf: String,
    current: i32,
    best: i32,
    badges: Vec<String>,
    cards: Vec<Card>,
}
async fn page(
    State(state): State<HttpState>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Result<Response, StatusCode> {
    locale(&lang)?;
    let account = account(&session).await?;
    let daily = state.puzzles.today(account).await.map_err(status)?;
    let progress = state.puzzles.progress(account).await.map_err(status)?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let cards = daily
        .into_iter()
        .map(|daily| Card {
            id: daily.id,
            heading: text(&lang, daily.kind.key()),
            question: daily.question.localized(&lang).into(),
            clue: daily.clue.localized(&lang).into(),
            choices: daily
                .choices
                .iter()
                .map(|choice| choice.localized(&lang).to_owned())
                .collect(),
            image: daily.has_image,
            result: daily
                .answer
                .map(|answer| text(&lang, if answer.correct { "correct" } else { "wrong" })),
        })
        .collect();
    let page = Page {
        other: if lang == "pl" { "en" } else { "pl" },
        other_name: text(&lang, "other-language"),
        title: text(&lang, "title"),
        skip: text(&lang, "skip"),
        home: text(&lang, "home"),
        rules: text(&lang, "rules"),
        current_label: text(&lang, "current"),
        best_label: text(&lang, "best"),
        badges_label: text(&lang, "badges"),
        none: text(&lang, "none"),
        empty: text(&lang, "empty"),
        choose: text(&lang, "choose"),
        submit: text(&lang, "submit"),
        badges: progress
            .badges()
            .iter()
            .map(|days| text(&lang, &format!("badge-{days}")))
            .collect(),
        current: progress.current,
        best: progress.best,
        csrf,
        cards,
        lang,
    };
    Ok(oteryn_web::render(&page))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    choice: u8,
    #[serde(rename = "_csrf")]
    _csrf: String,
}
async fn answer(
    State(state): State<HttpState>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, id)): Path<(String, Uuid)>,
    form: Result<Form<Submission>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    locale(&lang)?;
    let Form(form) = form.map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .puzzles
        .answer(account(&session).await?, id, form.choice)
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/puzzles")).into_response())
}
async fn image(
    State(state): State<HttpState>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, id)): Path<(String, Uuid)>,
) -> Result<Response, StatusCode> {
    locale(&lang)?;
    let bytes = state
        .puzzles
        .image(account(&session).await?, id)
        .await
        .map_err(status)?;
    Ok(([(header::CONTENT_TYPE, "image/webp")], bytes).into_response())
}
