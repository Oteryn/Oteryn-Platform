use super::{Cohort, Error, Flags, PlacesStore, UnresolvedSource};
use askama::Template;
use axum::{
    Extension, Form, Json, Router,
    extract::{Path, RawQuery, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn configured_router(pool: sqlx::PgPool) -> Router {
    router(PlacesStore::new(
        pool,
        Flags {
            aggregate: std::env::var("OTERYN_PLAYERDATA_PLACES_AGGREGATE").as_deref() == Ok("true"),
            ingest: false,
        },
        Arc::new(UnresolvedSource),
    ))
    .layer(Extension(oteryn_web::csrf::CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..Default::default()
    }))
}
pub fn router(store: PlacesStore) -> Router {
    if store.enabled().is_err() {
        return Router::new()
            .route(
                "/{lang}/atlas/places",
                get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .route(
                "/{lang}/atlas/places/participation",
                get(|| async { StatusCode::SERVICE_UNAVAILABLE })
                    .post(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .layer(middleware::from_fn(private));
    }
    Router::new()
        .route("/{lang}/atlas/places", get(page))
        .route(
            "/{lang}/atlas/places/participation",
            get(preference).post(update),
        )
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(store.pool().clone()),
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(store)
}
async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}
impl From<Error> for StatusCode {
    fn from(value: Error) -> Self {
        match value {
            Error::Disabled | Error::Database(_) => Self::SERVICE_UNAVAILABLE,
            Error::Denied => Self::FORBIDDEN,
            Error::Invalid => Self::BAD_REQUEST,
            Error::Conflict => Self::CONFLICT,
        }
    }
}
#[derive(Debug, Clone, Copy)]
struct Locale(&'static str);
impl Locale {
    fn parse(lang: &str) -> Result<Self, StatusCode> {
        match lang {
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
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}
#[derive(Template)]
#[template(
    source = r###"<!doctype html>
<html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/atlas/places.css"></head>
<body><main><a href="/{{ lang }}/atlas">{{ back }}</a><h1>{{ title }}</h1><p>{{ help }}</p><p>{{ privacy }}</p>
<form method="post" action="/{{ lang }}/atlas/places/participation"><fieldset><legend>{{ participation }}</legend><input type="hidden" name="_csrf" value="{{ csrf }}"><label><input type="radio" name="participating" value="true" {% if participating %}checked{% endif %}>{{ enabled }}</label><label><input type="radio" name="participating" value="false" {% if !participating %}checked{% endif %}>{{ excluded }}</label><button type="submit">{{ save }}</button></fieldset></form>
{% if rows.is_empty() %}<p role="status">{{ empty }}</p>{% endif %}
<ul class="places">{% for row in rows %}<li><h2>{{ row.place_key }}</h2><dl><dt>{{ day }}</dt><dd>{{ row.utc_day }}</dd><dt>{{ world }}</dt><dd>{{ row.world_id }}</dd><dt>{{ level }}</dt><dd>{{ row.level_min }}–{{ row.level_max }}</dd><dt>{{ vocation }}</dt><dd>{{ row.vocation_key }}</dd><dt>{{ experience }}</dt><dd>{{ row.experience_per_hour }}</dd><dt>{{ profit }}</dt><dd>{{ row.profit_per_hour }}</dd><dt>{{ count }}</dt><dd>{{ row.sessions }}</dd><dt>{{ provenance }}</dt><dd>{{ row.catalog_revision }} / {{ row.price_revision }}</dd></dl></li>{% endfor %}</ul></main></body></html>
"###,
    ext = "html"
)]
struct Page {
    lang: &'static str,
    rows: Vec<Cohort>,
    participating: bool,
    csrf: String,
    title: String,
    help: String,
    privacy: String,
    enabled: String,
    excluded: String,
    save: String,
    level: String,
    vocation: String,
    day: String,
    experience: String,
    profit: String,
    count: String,
    empty: String,
    back: String,
    participation: String,
    provenance: String,
    world: String,
}
fn no_filters(query: Option<String>) -> Result<(), StatusCode> {
    if query.is_some() {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}
async fn owner(session: &BrowserSession) -> Result<uuid::Uuid, StatusCode> {
    session.account_id().await.ok_or(StatusCode::FORBIDDEN)
}
#[derive(Serialize)]
struct Preference {
    participating: bool,
}
async fn preference(
    State(store): State<PlacesStore>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
    Extension(session): Extension<BrowserSession>,
) -> Result<Json<Preference>, StatusCode> {
    Locale::parse(&lang)?;
    no_filters(query)?;
    let account = owner(&session).await?;
    Ok(Json(Preference {
        participating: store.preference(account, account).await?,
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    _csrf: String,
    participating: String,
}
async fn update(
    State(store): State<PlacesStore>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<Update>,
) -> Result<Redirect, StatusCode> {
    Locale::parse(&lang)?;
    no_filters(query)?;
    let account = owner(&session).await?;
    let participating = match form.participating.as_str() {
        "true" => true,
        "false" => false,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    store
        .set_preference(account, account, participating)
        .await?;
    Ok(Redirect::to(&format!("/{lang}/atlas/places")))
}
async fn page(
    State(store): State<PlacesStore>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
    Extension(session): Extension<BrowserSession>,
) -> Result<Html<String>, StatusCode> {
    let locale = Locale::parse(&lang)?;
    no_filters(query)?;
    let account = owner(&session).await?;
    let participating = store.preference(account, account).await?;
    let rows = store.daily(account).await?;
    let view = Page {
        lang: locale.0,
        rows,
        participating,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        title: locale.text("title"),
        help: locale.text("help"),
        privacy: locale.text("privacy"),
        enabled: locale.text("enabled"),
        excluded: locale.text("excluded"),
        save: locale.text("save"),
        level: locale.text("level"),
        vocation: locale.text("vocation"),
        day: locale.text("day"),
        experience: locale.text("experience"),
        profit: locale.text("profit"),
        count: locale.text("count"),
        empty: locale.text("empty"),
        back: locale.text("back"),
        participation: locale.text("participation"),
        provenance: locale.text("provenance"),
        world: locale.text("world"),
    };
    Ok(Html(
        view.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
