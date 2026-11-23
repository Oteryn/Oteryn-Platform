use super::{
    Config, Error, operator,
    presentation::{Labels, render_section, render_widget},
    schema,
    selection::{self, Options},
    store,
    view::read_current,
};
use crate::v2_watermark::Part;
use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{OriginalUri, RawQuery, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{Next, from_fn, from_fn_with_state},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;
#[derive(Clone)]
pub(super) struct Surface {
    pub(super) db: PgPool,
    pub(super) config: Config,
}
#[derive(Clone)]
struct Ingest {
    surface: Surface,
    watermark: bool,
}
fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
/// Only the private TLS adapter supplies VerifiedClient. Headers never authenticate ingest.
pub fn router(db: PgPool, identities: Arc<IdentityMap>, config: Config) -> Router {
    routes(db, identities, config, Part::All)
}
fn routes(db: PgPool, identities: Arc<IdentityMap>, config: Config, part: Part) -> Router {
    let mut router = Router::new();
    for watermark in [false, true] {
        let path = if watermark {
            "watermark"
        } else {
            "world-daily"
        };
        if !part.includes(path) {
            continue;
        }
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), post(ingest))
            .with_state(Ingest {
                surface: Surface {
                    db: db.clone(),
                    config: config.clone(),
                },
                watermark,
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
async fn ingest(
    State(state): State<Ingest>,
    Extension(publisher): Extension<guard::PublisherKey>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
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
    let (route, limit) = if state.watermark {
        ("v2/world-daily/watermark", 60)
    } else {
        ("v2/world-daily", 120)
    };
    if let Err(response) =
        crate::ingest::budget::check(&state.surface.db, &publisher, route, limit).await
    {
        return response;
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
pub(super) fn status(e: Error) -> StatusCode {
    match e {
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::Conflict => StatusCode::CONFLICT,
    }
}
/// Add the daily state to Today and eligible public headers.
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
pub fn configured_mount(app: Router, db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let config = Config::configured();
    let operator = operator::router(Surface {
        db: db.clone(),
        config: config.clone(),
    })?;
    Ok(mount(
        app.merge(operator).merge(routes(
            db.clone(),
            Arc::new(configured_identities()),
            config.clone(),
            Part::Payload,
        )),
        db,
        config,
    ))
}
/// The guarded `world_daily` watermark route, mounted only through [`crate::v2_watermark`].
pub fn configured_watermark_router(db: PgPool) -> Router {
    routes(
        db,
        Arc::new(configured_identities()),
        Config::configured(),
        Part::Watermark,
    )
}
fn configured_identities() -> IdentityMap {
    std::env::var("PORTAL_PUBLIC_PROJECTIONS_SAN")
        .ok()
        .and_then(|s| guard::Identity::san(&s).ok())
        .and_then(|identity| {
            IdentityMap::new(vec![guard::Binding {
                identity,
                group: "game-public".into(),
            }])
            .ok()
        })
        .unwrap_or_default()
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
async fn load(state: &Surface, options: &Options, lang: &str) -> Result<super::DailyView, Error> {
    let world = options.world(&state.config)?;
    let view = read_current(&state.db, &state.config, world, lang).await?;
    match options.date {
        Some(date) => view.for_date(date, now()),
        None => Ok(view),
    }
}

async fn requested_view(
    state: &Surface,
    path: &str,
    raw: Option<&str>,
) -> Result<super::DailyView, Error> {
    let options = Options::parse(raw)?;
    let lang = language(path, options.lang.as_deref())?;
    load(state, &options, lang).await
}

#[derive(serde::Serialize)]
struct Data {
    #[serde(flatten)]
    view: super::DailyView,
    upcoming_raids: &'static str,
}
async fn data(
    State(state): State<Surface>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let mut r = match requested_view(&state, uri.path(), raw.as_deref()).await {
        Ok(view) => axum::Json(Data {
            view,
            upcoming_raids: "unavailable",
        })
        .into_response(),
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
    let result = requested_view(&state, uri.path(), raw.as_deref()).await;
    let code = result
        .as_ref()
        .map_or_else(|e| status(*e), |_| StatusCode::OK);
    let mut r = (code, Html(render_widget(result.as_ref().ok(), lang))).into_response();
    no_store(&mut r);
    r
}
fn public_html_path(path: &str) -> bool {
    ["/pl/", "/en/"].iter().any(|prefix| {
        path.strip_prefix(prefix).is_some_and(|p| {
            p == "today"
                || [
                    "wiki", "atlas", "news", "events", "stats", "rankings", "online", "worlds",
                ]
                .iter()
                .any(|s| p == *s || p.starts_with(&format!("{s}/")))
        })
    }) || path == "/today"
}
async fn extend(State(state): State<Surface>, request: Request, next: Next) -> Response {
    let uri = request.uri().clone();
    let today = matches!(uri.path(), "/today" | "/pl/today" | "/en/today");
    if request.method() != axum::http::Method::GET
        || !public_html_path(uri.path())
        || (!today && !state.config.public_enabled)
    {
        return next.run(request).await;
    }
    let response = next.run(request).await;
    if !response.status().is_success()
        || response.headers().contains_key(header::CONTENT_ENCODING)
        || !response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|v| v.as_bytes().starts_with(b"text/html"))
    {
        return response;
    }
    let options = Options::parse(if today { uri.query() } else { None });
    let lang = request_language(uri.path(), uri.query());
    let (mut parts, body) = response.into_parts();
    let bytes = match oteryn_web::decoration::buffer(
        body,
        2_097_152,
        Some(std::time::Duration::from_secs(3)),
    )
    .await
    {
        Ok(bytes) => bytes,
        Err(body) => return Response::from_parts(parts, body),
    };
    let worlds = match options.as_ref().ok().filter(|_| today) {
        Some(options) => {
            let selected = options.world(&state.config).ok();
            selection::choices(&state.db, &state.config, selected).await
        }
        None => Vec::new(),
    };
    // Resolve the fenced projection after buffering and loading the selector.
    let selected = match options.as_ref() {
        Ok(options) => load(&state, options, lang).await.ok(),
        Err(_) => {
            parts.status = StatusCode::BAD_REQUEST;
            None
        }
    };
    let at = now();
    let view = selected.as_ref().filter(|view| view.eligible(at));
    let mut html = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
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
                html.replace_range(start..start + end, &Labels::new(lang).text("partial"));
            }
        }
    }
    if state.config.public_enabled {
        let widget = render_widget(view, lang);
        if let Some(i) = html.find("</header>") {
            html.insert_str(i, &widget);
        } else if today && let Some(i) = html.find("<body>") {
            html.insert_str(i + 6, &format!("<header>{widget}</header>"));
        }
    }
    if today && let Some(i) = html.find("</main>") {
        html.insert_str(
            i,
            &render_section(view, lang, options.as_ref().map_err(|e| *e), worlds, at),
        );
    }
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.remove(header::ETAG);
    let mut r = Response::from_parts(parts, Body::from(html));
    no_store(&mut r);
    r
}
pub(super) fn no_store(r: &mut Response) {
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
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    #[tokio::test]
    async fn daily_queries_validate_before_reading_and_keep_editorial_available() {
        let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unused").unwrap();
        let editorial = Router::new().route(
            "/en/today",
            get(|| async { Html("<main><h1>Editorial</h1></main>") }),
        );
        let app = mount(editorial, db, Config::default());
        for (path, query, code) in [
            ("", "", StatusCode::OK),
            ("", "?date=2024-02-29", StatusCode::OK),
            ("/data", "?date=2024-02-29", StatusCode::SERVICE_UNAVAILABLE),
            ("/widget", "", StatusCode::SERVICE_UNAVAILABLE),
            ("", "?date=2026-02-29", StatusCode::BAD_REQUEST),
            ("/data", "?date=2026-2-01", StatusCode::BAD_REQUEST),
            ("/widget", "?date=", StatusCode::BAD_REQUEST),
            (
                "/data",
                "?date=2026-02-01&date=2026-02-02",
                StatusCode::BAD_REQUEST,
            ),
            ("/widget", "?world=invalid", StatusCode::BAD_REQUEST),
        ] {
            let r = app
                .clone()
                .oneshot(
                    Request::get(format!("/en/today{path}{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(r.status(), code, "{path}{query}");
            if path.is_empty() && code == StatusCode::OK {
                let body = to_bytes(r.into_body(), 16_384).await.unwrap();
                let html = std::str::from_utf8(&body).unwrap();
                assert!(html.contains("Editorial"));
                assert!(html.contains("World daily data is currently unavailable."));
                assert!(html.contains("name=\"date\""));
            }
        }
    }
}
