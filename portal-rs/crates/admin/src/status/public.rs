use super::{
    Labels, failure, store,
    store::{Component, Error, HISTORY_PAGE_SIZE, Incident, Incidents, Maintenance, Update, iso},
};
use askama::Template;
use axum::{
    Extension, Form, Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_web::{
    ratelimit::{ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{BrowserSession, SessionKey, SessionLayer},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
struct Site {
    db: PgPool,
    origin: String,
}

pub(super) fn router(db: PgPool) -> Router {
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .filter(|value| {
            value.parse::<Uri>().is_ok_and(|uri| {
                matches!(uri.scheme_str(), Some("http" | "https"))
                    && uri
                        .authority()
                        .is_some_and(|authority| !authority.as_str().contains('@'))
                    && uri.path() == "/"
                    && uri.query().is_none()
            })
        })
        .unwrap_or_default();
    let mut sessions = SessionLayer::new(db.clone());
    if !origin.is_empty() {
        sessions.csrf.trusted_origins = vec![origin.trim_end_matches('/').to_owned()];
    }
    Router::new()
        .route("/{lang}/status/notices", get(notices).post(dismiss))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .route("/{lang}/status", get(page))
        .route("/{lang}/status/incidents/{id}", get(detail))
        .route("/{lang}/status/incidents.atom", get(feed))
        .route_layer(middleware::from_fn(public_budget))
        .route("/static/status/{asset}", get(asset))
        .with_state(Site {
            db,
            origin: origin.trim_end_matches('/').to_owned(),
        })
}

async fn public_budget(
    Extension(limiter): Extension<RateLimiter>,
    source: Result<ClientSource, StatusCode>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Some(lang) = request.uri().path().split('/').nth(1).and_then(language) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let ClientSource(ip) = match source {
        Ok(source) => source,
        Err(status) => return localized(status.into_response(), lang),
    };
    match limiter
        .check(NamedPolicy::PublicGameData, &[Keys::source(ip)])
        .await
    {
        Ok(()) => next.run(request).await,
        Err(error) => {
            let locale = if lang == "pl" {
                oteryn_web::Language::Pl
            } else {
                oteryn_web::Language::En
            };
            localized(error.response(locale, false), lang)
        }
    }
}

async fn asset(Path(asset): Path<String>) -> Response {
    let (content_type, source) = match asset.as_str() {
        "notices.css" => ("text/css", include_str!("notices.css")),
        "status.css" => ("text/css", include_str!("status.css")),
        "status.js" => ("text/javascript", include_str!("status.js")),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        source,
    )
        .into_response()
}

fn language(lang: &str) -> Option<&'static str> {
    match lang {
        "pl" => Some("pl"),
        "en" => Some("en"),
        _ => None,
    }
}

fn localized(mut response: Response, lang: &'static str) -> Response {
    response
        .headers_mut()
        .insert(header::CONTENT_LANGUAGE, HeaderValue::from_static(lang));
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(oteryn_web::chrome::PORTAL_CONTENT_SECURITY_POLICY),
    );
    response
}

async fn snapshot(db: &PgPool) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::raw_sql(
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY;
         SET LOCAL statement_timeout = '3s'",
    )
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default)]
    page: u32,
}

#[derive(Template)]
#[template(path = "status/public.html", config = "src/web_askama.toml")]
struct StatusPage {
    lang: &'static str,
    labels: Labels,
    components: Vec<Component>,
    active: Vec<Incident>,
    maintenance: Vec<Maintenance>,
    history: Vec<Incident>,
    previous: Option<u32>,
    next: Option<u32>,
}

async fn load_page(db: &PgPool, lang: &'static str, page: u32) -> Result<StatusPage, Error> {
    let mut tx = snapshot(db).await?;
    let now = store::clock(&mut tx).await?;
    let components = store::components(&mut tx, now).await?;
    let active = store::incidents(&mut tx, lang, now, Incidents::Open).await?;
    let maintenance = store::maintenance(&mut tx, lang, now, false).await?;
    let offset = i64::from(page) * HISTORY_PAGE_SIZE as i64;
    let mut history = store::incidents(&mut tx, lang, now, Incidents::History(offset)).await?;
    tx.commit().await?;
    if page > 0 && history.is_empty() {
        return Err(Error::MissingPage);
    }
    let next = (history.len() > HISTORY_PAGE_SIZE).then(|| page.saturating_add(1));
    history.truncate(HISTORY_PAGE_SIZE);
    Ok(StatusPage {
        lang,
        labels: Labels(lang),
        components,
        active,
        maintenance,
        history,
        previous: page.checked_sub(1),
        next,
    })
}

async fn page(
    Path(lang): Path<String>,
    State(site): State<Site>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let Some(lang) = language(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    localized(
        match load_page(&site.db, lang, query.page).await {
            Ok(page) => oteryn_web::render(&page),
            Err(error) => failure(error, lang),
        },
        lang,
    )
}

#[derive(Template)]
#[template(path = "status/incident.html", config = "src/web_askama.toml")]
struct Detail {
    lang: &'static str,
    labels: Labels,
    incident: Incident,
    timeline: Vec<Update>,
}

async fn load_detail(db: &PgPool, lang: &'static str, id: Uuid) -> Result<Detail, Error> {
    let mut tx = snapshot(db).await?;
    let now = store::clock(&mut tx).await?;
    let incident = store::incidents(&mut tx, lang, now, Incidents::Detail(id))
        .await?
        .pop()
        .ok_or(Error::Missing)?;
    let timeline = store::timeline(&mut tx, id, lang).await?;
    tx.commit().await?;
    Ok(Detail {
        lang,
        labels: Labels(lang),
        incident,
        timeline,
    })
}

async fn detail(State(site): State<Site>, Path((lang, id)): Path<(String, Uuid)>) -> Response {
    let Some(lang) = language(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    localized(
        match load_detail(&site.db, lang, id).await {
            Ok(page) => oteryn_web::render(&page),
            Err(error) => failure(error, lang),
        },
        lang,
    )
}

#[derive(Template)]
#[template(
    path = "status/atom.xml",
    config = "src/web_askama.toml",
    escape = "html"
)]
struct Atom {
    lang: &'static str,
    labels: Labels,
    origin: String,
    updated: String,
    incidents: Vec<Incident>,
}

async fn load_feed(site: Site, lang: &'static str) -> Result<Atom, Error> {
    let mut tx = snapshot(&site.db).await?;
    let now = store::clock(&mut tx).await?;
    let incidents = store::incidents(&mut tx, lang, now, Incidents::Feed).await?;
    tx.commit().await?;
    let updated = incidents
        .first()
        .map_or(OffsetDateTime::UNIX_EPOCH, |i| i.posted_at);
    Ok(Atom {
        lang,
        labels: Labels(lang),
        origin: site.origin,
        updated: iso(updated),
        incidents,
    })
}

async fn feed(Path(lang): Path<String>, State(site): State<Site>) -> Response {
    let Some(lang) = language(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let result = match load_feed(site, lang).await {
        Ok(feed) => match feed.render() {
            Ok(xml) => (
                [(header::CONTENT_TYPE, "application/atom+xml; charset=utf-8")],
                xml,
            )
                .into_response(),
            Err(error) => {
                tracing::error!(%error, "status feed rendering failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(error) => failure(error, lang),
    };
    localized(result, lang)
}

const DISMISSED: SessionKey<String> = SessionKey::new("status_notice_edition");

#[derive(Serialize)]
struct BannerItem {
    id: Uuid,
    label: String,
    message: String,
    link: String,
    starts: Option<String>,
    ends: Option<String>,
}

#[derive(Serialize)]
struct Banner {
    edition: String,
    csrf: String,
    title: String,
    dismiss: String,
    items: Vec<BannerItem>,
}

async fn load_notices(
    db: &PgPool,
    lang: &'static str,
    session: &BrowserSession,
) -> Result<Option<Banner>, Error> {
    let labels = Labels(lang);
    let mut tx = snapshot(db).await?;
    let now = store::clock(&mut tx).await?;
    let incidents = store::incidents(&mut tx, lang, now, Incidents::Open).await?;
    let maintenance = store::maintenance(&mut tx, lang, now, true).await?;
    tx.commit().await?;

    let mut items: Vec<_> = incidents
        .into_iter()
        .filter(|i| i.severity == "major")
        .map(|i| BannerItem {
            id: i.id,
            label: i.world.unwrap_or_else(|| labels.get("portal")),
            message: i.title,
            link: format!("/{lang}/status/incidents/{}", i.id),
            starts: None,
            ends: None,
        })
        .collect();
    items.extend(maintenance.into_iter().map(|m| BannerItem {
        id: m.id,
        label: m.world.unwrap_or_else(|| labels.get("all-worlds")),
        message: m.message,
        link: format!("/{lang}/status#maintenance-{}", m.id),
        starts: Some(iso(m.starts_at)),
        ends: Some(iso(m.ends_at)),
    }));
    if items.is_empty() {
        return Ok(None);
    }

    // An edition is independent of language and ordering. New notices can be shown
    // even when a viewer dismissed an earlier edition in another tab.
    let mut ids: Vec<_> = items.iter().map(|item| item.id).collect();
    ids.sort_unstable();
    let mut hash = Sha256::new();
    for id in ids {
        hash.update(id.as_bytes());
    }
    let edition = format!("{:x}", hash.finalize());
    if session.get(DISMISSED).await?.as_deref() == Some(&edition) {
        return Ok(None);
    }

    Ok(Some(Banner {
        edition,
        csrf: session.csrf_token().await?,
        title: labels.get("banner-title"),
        dismiss: labels.get("banner-dismiss"),
        items,
    }))
}

async fn notices(
    Path(lang): Path<String>,
    State(site): State<Site>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    let Some(lang) = language(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    localized(
        match load_notices(&site.db, lang, &session).await {
            Ok(Some(banner)) => Json(banner).into_response(),
            Ok(None) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => failure(error, lang),
        },
        lang,
    )
}

#[derive(Deserialize)]
struct Dismiss {
    edition: String,
}

async fn dismiss(
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<Dismiss>,
) -> Response {
    let Some(lang) = language(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if form.edition.len() != 64 || !form.edition.bytes().all(|b| b.is_ascii_hexdigit()) {
        return failure(Error::Invalid, lang);
    }
    match session.set(DISMISSED, &form.edition).await {
        Ok(()) => localized(StatusCode::NO_CONTENT.into_response(), lang),
        Err(error) => failure(error.into(), lang),
    }
}
