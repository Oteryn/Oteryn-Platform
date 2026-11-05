use super::{Config, Error, read, schema, store};
use askama::Template;
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{OriginalUri, RawQuery, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{Next, from_fn, from_fn_with_state},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use sqlx::PgPool;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;
#[derive(Clone)]
struct Surface {
    db: PgPool,
    config: Config,
}
#[derive(Clone)]
struct Ingest {
    surface: Surface,
    watermark: bool,
    rate: Arc<Mutex<Rate>>,
}
#[derive(Default)]
struct Rate {
    snapshots: VecDeque<i64>,
    watermarks: VecDeque<i64>,
}
fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
/// Only the private TLS adapter supplies VerifiedClient. Headers never authenticate ingest.
pub fn router(db: PgPool, identities: Arc<IdentityMap>, config: Config) -> Router {
    let mut router = Router::new();
    let rate = Arc::new(Mutex::new(Rate::default()));
    for watermark in [false, true] {
        let path = if watermark {
            "watermark"
        } else {
            "world-daily"
        };
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), post(ingest))
            .with_state(Ingest {
                surface: Surface {
                    db: db.clone(),
                    config: config.clone(),
                },
                watermark,
                rate: rate.clone(),
            });
        router = router.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: config.ingest_enabled && config.valid(),
                group: "game-public".into(),
                max_body_bytes: if watermark { 512 } else { 131_072 },
                // The shared decoder uses an exclusive depth bound: five containers.
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        ));
    }
    router.layer(from_fn(ingest_headers))
}
async fn ingest_headers(request: Request, next: Next) -> Response {
    let mut r = next.run(request).await;
    if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
        r = guard::failure(StatusCode::BAD_REQUEST);
    }
    no_store(&mut r);
    r
}
async fn ingest(State(state): State<Ingest>, headers: HeaderMap, body: Bytes) -> Response {
    if headers
        .get(header::CONTENT_LENGTH)
        .is_some_and(|v| v.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let now = now();
    let publication = match schema::decode(&body, state.watermark, now) {
        Ok(p) => p,
        Err(_) => return guard::failure(StatusCode::BAD_REQUEST),
    };
    let authority = match &publication {
        schema::Publication::Snapshot(s) => &s.source_authority,
        schema::Publication::Watermark(w) => &w.source_authority,
    };
    if authority != &state.surface.config.source_authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    let allowed = {
        let Ok(mut rate) = state.rate.lock() else {
            return guard::failure(StatusCode::SERVICE_UNAVAILABLE);
        };
        let (times, limit) = if state.watermark {
            (&mut rate.watermarks, 60)
        } else {
            (&mut rate.snapshots, 120)
        };
        while times.front().is_some_and(|t| *t <= now - 60) {
            times.pop_front();
        }
        if times.len() >= limit {
            false
        } else {
            times.push_back(now);
            true
        }
    };
    if !allowed {
        let mut r = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        r.headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        return r;
    }
    match store::apply(&state.surface.db, &state.surface.config, &publication).await {
        Ok(result) => {
            #[derive(serde::Serialize)]
            struct Ack {
                contract_version: u8,
                result: &'static str,
            }
            axum::Json(Ack {
                contract_version: 2,
                result,
            })
            .into_response()
        }
        Err(e) => guard::failure(status(e)),
    }
}
fn status(e: Error) -> StatusCode {
    match e {
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::Conflict => StatusCode::CONFLICT,
    }
}
/// Extend existing Today and public headers; disabled serving passes responses through unchanged.
pub fn mount(app: Router, db: PgPool, config: Config) -> Router {
    let state = Surface { db, config };
    app.merge(
        Router::new()
            .route("/today/data", get(data))
            .route("/{lang}/today/data", get(data))
            .route("/today/widget", get(widget))
            .route("/{lang}/today/widget", get(widget))
            .with_state(state.clone()),
    )
    .layer(from_fn_with_state(state, extend))
}
pub fn configured_mount(app: Router, db: PgPool) -> Router {
    let config = Config::configured();
    let identities = std::env::var("PORTAL_PUBLIC_PROJECTIONS_SAN")
        .ok()
        .and_then(|s| guard::Identity::san(&s).ok())
        .and_then(|identity| {
            IdentityMap::new(vec![guard::Binding {
                identity,
                group: "game-public".into(),
            }])
            .ok()
        })
        .unwrap_or_default();
    mount(
        app.merge(router(db.clone(), Arc::new(identities), config.clone())),
        db,
        config,
    )
}
#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Options {
    world: Option<String>,
    lang: Option<String>,
}
fn options(raw: Option<&str>, config: &Config) -> Result<(Uuid, Option<String>), Error> {
    let q: Options =
        serde_urlencoded::from_str(raw.unwrap_or_default()).map_err(|_| Error::BadInput)?;
    if q.lang.as_deref().is_some_and(|s| !matches!(s, "pl" | "en")) {
        return Err(Error::BadInput);
    }
    let world = match q.world {
        Some(s) => schema::world_id(&s)?,
        None if config.worlds.len() == 1 => *config.worlds.first().ok_or(Error::Unavailable)?,
        None => return Err(Error::Unavailable),
    };
    if !config.worlds.contains(&world) {
        return Err(Error::Unavailable);
    }
    Ok((world, q.lang))
}
fn language(path: &str, selected: Option<&str>) -> Result<&'static str, Error> {
    match path.split('/').nth(1) {
        Some("en") => Ok("en"),
        Some("pl") => Ok("pl"),
        Some("today") => Ok(if selected == Some("en") { "en" } else { "pl" }),
        _ => Err(Error::BadInput),
    }
}
fn request_language(path: &str, raw: Option<&str>) -> &'static str {
    let selected = serde_urlencoded::from_str::<Vec<(String, String)>>(raw.unwrap_or_default())
        .ok()
        .and_then(|pairs| {
            let mut values = pairs
                .iter()
                .filter(|(key, _)| key == "lang")
                .map(|(_, value)| value);
            let first = values.next()?.clone();
            values.next().is_none().then_some(first)
        });
    language(path, selected.as_deref()).unwrap_or("pl")
}
async fn load(
    state: &Surface,
    path: &str,
    raw: Option<&str>,
) -> Result<(super::DailyView, &'static str), Error> {
    if !state.config.public_enabled {
        return Err(Error::Unavailable);
    }
    let (world, selected) = options(raw, &state.config)?;
    let lang = language(path, selected.as_deref())?;
    Ok((
        read(&state.db, &state.config, world, lang, now()).await?,
        lang,
    ))
}
async fn data(
    State(state): State<Surface>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let mut r = match load(&state, uri.path(), raw.as_deref()).await {
        Ok((v, _)) => axum::Json(v).into_response(),
        Err(e) => (
            status(e),
            axum::Json(serde_json::json!({"status":"unavailable"})),
        )
            .into_response(),
    };
    no_store(&mut r);
    r
}
async fn widget(
    State(state): State<Surface>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let lang = request_language(uri.path(), raw.as_deref());
    let result = load(&state, uri.path(), raw.as_deref()).await;
    let code = result
        .as_ref()
        .map_or_else(|e| status(*e), |_| StatusCode::OK);
    let (view, lang) = result.map(|(v, l)| (Some(v), l)).unwrap_or((None, lang));
    let mut r = (code, Html(render_widget(view.as_ref(), lang))).into_response();
    no_store(&mut r);
    r
}
fn public_html_path(path: &str) -> bool {
    ["/pl/", "/en/"].iter().any(|prefix| {
        path.strip_prefix(prefix).is_some_and(|p| {
            p.is_empty()
                || p == "today"
                || [
                    "wiki", "atlas", "news", "events", "stats", "rankings", "online", "worlds",
                ]
                .iter()
                .any(|s| p == *s || p.starts_with(&format!("{s}/")))
        })
    }) || path == "/today"
}
async fn extend(State(state): State<Surface>, request: Request, next: Next) -> Response {
    if !state.config.public_enabled
        || request.method() != axum::http::Method::GET
        || !public_html_path(request.uri().path())
    {
        return next.run(request).await;
    }
    let uri = request.uri().clone();
    let today = matches!(uri.path(), "/today" | "/pl/today" | "/en/today");
    let response = next.run(request).await;
    if !response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|v| v.as_bytes().starts_with(b"text/html"))
    {
        return response;
    }
    let result = load(&state, uri.path(), if today { uri.query() } else { None }).await;
    let lang = request_language(uri.path(), uri.query());
    let view = result.as_ref().ok().map(|(v, _)| v);
    let lang = result.as_ref().map_or(lang, |(_, l)| *l);
    let (mut parts, body) = response.into_parts();
    let bytes = match to_bytes(body, 2_097_152).await {
        Ok(b) => b,
        Err(_) => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    let mut html = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    // The new source participates in the existing page's overall availability.
    if today
        && view.is_some()
        && let Some(main) = html.find("<main")
    {
        let marker = "<p role=\"status\">";
        if let Some(start) = html[main..].find(marker) {
            let start = main + start + marker.len();
            if let Some(end) = html[start..].find("</p>") {
                html.replace_range(start..start + end, &label(lang, "partial"));
            }
        }
    }
    let widget = render_widget(view, lang);
    if let Some(i) = html.find("</header>") {
        html.insert_str(i, &widget);
    } else if today && let Some(i) = html.find("<body>") {
        html.insert_str(i + 6, &format!("<header>{widget}</header>"));
    }
    if today && let Some(i) = html.find("</main>") {
        html.insert_str(i, &render_section(view, lang));
    }
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.remove(header::ETAG);
    let mut r = Response::from_parts(parts, Body::from(html));
    no_store(&mut r);
    r
}
fn no_store(r: &mut Response) {
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        r.headers_mut()
            .entry(axum::http::HeaderName::from_static(name))
            .or_insert(HeaderValue::from_static(value));
    }
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store, max-age=0"),
    );
    r.headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    r.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
}
fn label(lang: &str, key: &str) -> String {
    let (locale, source) = if lang == "en" {
        (
            unic_langid::langid!("en"),
            include_str!("../../../../templates/today/en.ftl"),
        )
    } else {
        (
            unic_langid::langid!("pl"),
            include_str!("../../../../templates/today/pl.ftl"),
        )
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "today/widget.html", config = "src/today.askama.toml")]
struct Widget {
    title: String,
    creature_label: String,
    boss_label: String,
    creature: String,
    boss: String,
    unavailable: String,
    available: bool,
}
fn render_widget(view: Option<&super::DailyView>, lang: &str) -> String {
    Widget {
        title: label(lang, "title"),
        creature_label: label(lang, "creature"),
        boss_label: label(lang, "boss"),
        creature: view.map(|v| v.current.creature.clone()).unwrap_or_default(),
        boss: view.map(|v| v.current.boss.clone()).unwrap_or_default(),
        unavailable: label(lang, "unavailable"),
        available: view.is_some(),
    }
    .render()
    .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "today/section.html", config = "src/today.askama.toml")]
struct Section {
    title: String,
    unavailable: String,
    available: bool,
    world: String,
    current: String,
    history_label: String,
    coverage: String,
    history: Vec<String>,
    raids_label: String,
    raid_note: String,
    raids: Vec<String>,
    empty: String,
}
fn date(seconds: i64) -> String {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|d| d.format(&Rfc3339).ok())
        .unwrap_or_default()
}
fn boost(b: &super::Boost, lang: &str) -> String {
    format!(
        "{} — {}: {}; {}: {} ({} – {})",
        label(lang, "boost"),
        label(lang, "creature"),
        b.creature,
        label(lang, "boss"),
        b.boss,
        date(b.from),
        date(b.until)
    )
}
fn render_section(view: Option<&super::DailyView>, lang: &str) -> String {
    Section {
        title: label(lang, "title"),
        unavailable: label(lang, "unavailable"),
        available: view.is_some(),
        world: view.map(|v| v.world.clone()).unwrap_or_default(),
        current: view.map(|v| boost(&v.current, lang)).unwrap_or_default(),
        history_label: label(lang, "history"),
        coverage: view
            .map(|v| {
                format!(
                    "{} {}",
                    label(lang, "coverage"),
                    date(v.history_complete_from)
                )
            })
            .unwrap_or_default(),
        history: view
            .map(|v| v.history.iter().map(|h| boost(h, lang)).collect())
            .unwrap_or_default(),
        raids_label: label(lang, "raids"),
        raid_note: label(lang, "raid-note"),
        raids: view
            .map(|v| {
                v.raids
                    .iter()
                    .map(|r| {
                        format!(
                            "{} — {} ({})",
                            r.name,
                            date(r.scheduled_at),
                            label(lang, &r.state)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        empty: label(lang, "empty"),
    }
    .render()
    .unwrap_or_default()
}
