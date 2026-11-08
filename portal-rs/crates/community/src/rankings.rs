//! Public, privacy-filtered presentation of authoritative highscores snapshots.
use askama::Template;
use axum::{
    Router,
    extract::{Path, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::Language;
use serde::Deserialize;
use sqlx::PgPool;
use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc};

/// Decisions must be current at serving time; errors must never reuse an earlier allow.
pub trait PublicVisibility: Send + Sync {
    fn is_hidden<'a>(&'a self, character_ref: &'a str) -> VisibilityFuture<'a, bool>;
    /// Results correspond exactly to the supplied references, in order.
    fn batch<'a>(&'a self, characters: &'a [String]) -> VisibilityFuture<'a, Vec<bool>> {
        Box::pin(async move {
            let mut decisions = Vec::with_capacity(characters.len());
            for character in characters {
                decisions.push(self.is_hidden(character).await?);
            }
            Ok(decisions)
        })
    }
}
pub type VisibilityFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, VisibilityUnavailable>> + Send + 'a>>;
#[derive(Debug)]
pub struct VisibilityUnavailable;
/// Safe binary adapter until the profile privacy authority is available.
#[derive(Debug)]
pub struct UnavailableVisibility;
impl PublicVisibility for UnavailableVisibility {
    fn is_hidden<'a>(&'a self, _: &'a str) -> VisibilityFuture<'a, bool> {
        Box::pin(async { Err(VisibilityUnavailable) })
    }
    fn batch<'a>(&'a self, _: &'a [String]) -> VisibilityFuture<'a, Vec<bool>> {
        Box::pin(async { Err(VisibilityUnavailable) })
    }
}
#[derive(Clone)]
struct Rankings {
    db: PgPool,
    enabled: bool,
    visibility: Arc<dyn PublicVisibility>,
}
/// Off by default; enabling this flag cannot bypass the privacy adapter.
pub fn configured_router(db: PgPool) -> Router {
    router(
        db,
        std::env::var("PORTAL_PUBLIC_RANKINGS_ENABLED").as_deref() == Ok("true"),
        Arc::new(UnavailableVisibility),
    )
}
pub fn router(db: PgPool, enabled: bool, visibility: Arc<dyn PublicVisibility>) -> Router {
    Router::new()
        .route("/{lang}/rankings/{kind}", get(handle))
        .with_state(Rankings {
            db,
            enabled,
            visibility,
        })
}
// Level and experience are two presentations of the same producer ordering.
const KINDS: &[(&str, u8)] = &[
    ("level", 1),
    ("experience", 1),
    ("magic", 2),
    ("fist", 3),
    ("club", 4),
    ("sword", 5),
    ("axe", 6),
    ("distance", 7),
    ("shielding", 8),
    ("fishing", 9),
];
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    world: Option<String>,
    vocation: Option<String>,
    page: Option<String>,
}
#[derive(Deserialize)]
struct Snapshot {
    world_id: String,
    category: u8,
    filter: u8,
    rows: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    rank: String,
    character_id: String,
    name: String,
    vocation: u8,
    level: String,
    value: String,
}
#[derive(Template)]
#[template(path = "pages/rankings.html", config = "../web/askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    message: String,
    as_of: String,
    watermark: String,
    world_label: String,
    vocation_label: String,
    apply: String,
    kind_label: String,
    worlds: Vec<String>,
    world: String,
    vocation: String,
    vocations: Vec<(String, String)>,
    kinds: Vec<(String, String)>,
    headings: Vec<String>,
    rows: Vec<Vec<String>>,
    hint: String,
    previous: String,
    next: String,
    previous_url: String,
    next_url: String,
    page: usize,
    pagination: String,
    skip: String,
}
impl Page {
    fn new(lang: Language) -> Self {
        let text = |key: &str| lang.text(key);
        Self {
            lang: lang.code(),
            title: text("rankings-title"),
            message: String::new(),
            as_of: text("rankings-as-of"),
            watermark: String::new(),
            world_label: text("rankings-world"),
            vocation_label: text("rankings-vocation"),
            apply: text("rankings-apply"),
            kind_label: text("rankings-category"),
            worlds: vec![],
            world: String::new(),
            vocation: "0".into(),
            vocations: (0..=5)
                .map(|v| (v.to_string(), text(&format!("rankings-vocation-{v}"))))
                .collect(),
            kinds: KINDS
                .iter()
                .map(|(k, _)| (k.to_string(), text(&format!("rankings-{k}"))))
                .collect(),
            headings: ["rank", "name", "vocation", "level", "value"]
                .map(|k| text(&format!("rankings-{k}")))
                .to_vec(),
            rows: vec![],
            hint: text("rankings-scroll"),
            previous: text("rankings-previous"),
            next: text("rankings-next"),
            previous_url: String::new(),
            next_url: String::new(),
            page: 1,
            pagination: text("rankings-pagination"),
            skip: text("rankings-skip"),
        }
    }
    fn response(&self, status: StatusCode) -> Response {
        let body = self.render().unwrap_or_else(|_| self.message.clone());
        (
            status,
            [
                (header::CACHE_CONTROL, "no-store, private"),
                (header::CONTENT_LANGUAGE, self.lang),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (header::STRICT_TRANSPORT_SECURITY, "max-age=63072000; includeSubDomains"),
                (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
                (header::CONTENT_SECURITY_POLICY, "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"),
            ],
            Html(body),
        ).into_response()
    }
}
fn failure(lang: Language, status: StatusCode, key: &str) -> Response {
    let mut page = Page::new(lang);
    page.message = lang.text(key);
    page.response(status)
}
fn malformed(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.iter().enumerate().any(|(i, b)| {
        *b == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
    })
}
async fn handle(
    State(state): State<Rankings>,
    Path((locale, kind)): Path<(String, String)>,
    RawQuery(raw): RawQuery,
) -> Response {
    let lang = match locale.as_str() {
        "pl" => Language::Pl,
        "en" => Language::En,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if !state.enabled {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    }
    let Some((_, category)) = KINDS.iter().find(|(k, _)| *k == kind) else {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    };
    let raw = raw.unwrap_or_default();
    let filters = if raw.len() > 2048
        || malformed(&raw)
        || percent_encoding::percent_decode_str(&raw)
            .decode_utf8()
            .is_err()
    {
        None
    } else {
        serde_urlencoded::from_str::<Filters>(&raw).ok()
    };
    let Some(filters) = filters else {
        return failure(lang, StatusCode::BAD_REQUEST, "rankings-bad-query");
    };
    let page_number = filters.page.as_deref().unwrap_or("1");
    let Ok(number) = page_number.parse::<usize>() else {
        return failure(lang, StatusCode::BAD_REQUEST, "rankings-bad-query");
    };
    if number == 0 || page_number != number.to_string() {
        return failure(lang, StatusCode::BAD_REQUEST, "rankings-bad-query");
    }
    let vocation = filters.vocation.as_deref().unwrap_or("0");
    if !matches!(vocation, "0" | "1" | "2" | "3" | "4" | "5") {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    }
    if filters
        .world
        .as_ref()
        .is_some_and(|w| uuid::Uuid::parse_str(w).map_or(true, |id| id.to_string() != *w))
    {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    }
    match load(
        &state,
        lang,
        &kind,
        *category,
        filters.world.as_deref(),
        vocation,
        number,
    )
    .await
    {
        Ok(page) => page.response(StatusCode::OK),
        Err(status) => failure(
            lang,
            status,
            if status == StatusCode::NOT_FOUND {
                "rankings-not-found"
            } else {
                "rankings-unavailable"
            },
        ),
    }
}
async fn load(
    state: &Rankings,
    lang: Language,
    kind: &str,
    category: u8,
    selected: Option<&str>,
    vocation: &str,
    number: usize,
) -> Result<Page, StatusCode> {
    let unavailable = StatusCode::SERVICE_UNAVAILABLE;
    // Read one subject snapshot only; never combine different ranking revisions/worlds.
    let subjects: Vec<String> = sqlx::query_scalar("SELECT subject FROM public_snapshots WHERE family='highscores' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) ORDER BY subject LIMIT 1101").fetch_all(&state.db).await.map_err(|_| unavailable)?;
    if subjects.len() > 1100 {
        return Err(unavailable);
    }
    let worlds: BTreeSet<String> = subjects
        .iter()
        .filter_map(|s| s.split_once(':').map(|(w, _)| w.to_owned()))
        .collect();
    let world = match selected {
        Some(w) if worlds.contains(w) => w,
        Some(_) => return Err(StatusCode::NOT_FOUND),
        None => worlds.first().map(String::as_str).ok_or(unavailable)?,
    };
    // Payload and observation watermark must come from the same accepted snapshot.
    let (payload, watermark): (serde_json::Value, i64) = sqlx::query_as(
        "SELECT payload, watermark FROM public_snapshots WHERE family='highscores' AND subject=$1 AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)",
    ).bind(format!("{world}:{category}:{vocation}")).fetch_optional(&state.db).await.map_err(|_| unavailable)?.ok_or(unavailable)?;
    let snapshot: Snapshot = serde_json::from_value(payload).map_err(|_| unavailable)?;
    if snapshot.world_id != world
        || snapshot.category != category
        || snapshot.filter.to_string() != vocation
        || snapshot.rows.len() > 1000
    {
        return Err(unavailable);
    }
    let refs: Vec<String> = snapshot
        .rows
        .iter()
        .map(|r| r.character_id.clone())
        .collect();
    let hidden = state
        .visibility
        .batch(&refs)
        .await
        .map_err(|_| unavailable)?;
    if hidden.len() != refs.len() {
        return Err(unavailable);
    }
    let visible: Vec<Entry> = snapshot
        .rows
        .into_iter()
        .zip(hidden)
        .filter_map(|(r, hidden)| (!hidden).then_some(r))
        .collect();
    if number > visible.len().div_ceil(50).max(1) {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut page = Page::new(lang);
    page.title = format!("{}: {}", page.title, lang.text(&format!("rankings-{kind}")));
    page.world = world.to_owned();
    page.worlds = worlds.iter().cloned().collect();
    page.vocation = vocation.into();
    page.page = number;
    page.watermark = time::OffsetDateTime::from_unix_timestamp(watermark)
        .ok()
        .and_then(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .ok_or(unavailable)?;
    let url = |n| {
        format!(
            "/{}/rankings/{kind}?world={world}&vocation={vocation}&page={n}",
            lang.code()
        )
    };
    if number > 1 {
        page.previous_url = url(number - 1);
    }
    if number * 50 < visible.len() {
        page.next_url = url(number + 1);
    }
    page.rows = visible
        .into_iter()
        .skip((number - 1) * 50)
        .take(50)
        .map(|r| {
            vec![
                r.rank,
                r.name,
                if r.vocation <= 5 {
                    if r.vocation == 0 {
                        lang.text("rankings-vocation-none")
                    } else {
                        page.vocations[usize::from(r.vocation)].1.clone()
                    }
                } else {
                    format!("{} {}", page.vocation_label, r.vocation)
                },
                r.level,
                r.value,
            ]
        })
        .collect();
    if page.rows.is_empty() {
        page.message = lang.text("rankings-empty");
    }
    Ok(page)
}

/// Top `limit` visible characters of the level ranking on the first world, for
/// the home page. Same snapshot rules as the page; each shown row passes the
/// current privacy decision, and any doubt makes the whole block unavailable.
pub async fn home_top(
    db: &PgPool,
    visibility: &dyn PublicVisibility,
    lang: Language,
    limit: usize,
    profiles: bool,
) -> Result<(Vec<oteryn_ui::HomeRank>, String), VisibilityUnavailable> {
    fn fail(_: impl std::fmt::Display) -> VisibilityUnavailable {
        VisibilityUnavailable
    }
    let world: Option<String> = sqlx::query_scalar("SELECT min(split_part(subject,':',1)) FROM public_snapshots WHERE family='highscores' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)").fetch_one(db).await.map_err(fail)?;
    let world = world.ok_or(VisibilityUnavailable)?;
    let (payload, watermark): (serde_json::Value, i64) = sqlx::query_as(
        "SELECT payload, watermark FROM public_snapshots WHERE family='highscores' AND subject=$1 AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)",
    ).bind(format!("{world}:1:0")).fetch_optional(db).await.map_err(fail)?.ok_or(VisibilityUnavailable)?;
    let snapshot: Snapshot = serde_json::from_value(payload).map_err(fail)?;
    if snapshot.world_id != world
        || snapshot.category != 1
        || snapshot.filter != 0
        || snapshot.rows.len() > 1000
    {
        return Err(VisibilityUnavailable);
    }
    let as_of = time::OffsetDateTime::from_unix_timestamp(watermark)
        .ok()
        .and_then(|t| {
            t.format(time::macros::format_description!(
                "[year]-[month]-[day] [hour]:[minute]"
            ))
            .ok()
        })
        .ok_or(VisibilityUnavailable)?;
    let mut shown = Vec::with_capacity(limit);
    // Bounded scan: privacy checks stop once enough visible rows are found.
    for chunk in snapshot.rows.chunks(10).take(5) {
        let refs: Vec<String> = chunk.iter().map(|r| r.character_id.clone()).collect();
        let hidden = visibility.batch(&refs).await?;
        if hidden.len() != refs.len() {
            return Err(VisibilityUnavailable);
        }
        for (row, hidden) in chunk.iter().zip(hidden) {
            if !hidden && shown.len() < limit {
                shown.push(oteryn_ui::HomeRank {
                    rank: row.rank.clone(),
                    name: row.name.clone(),
                    level: row.level.clone(),
                    url: profiles.then(|| {
                        format!(
                            "/{}/characters/{}",
                            lang.code(),
                            percent_encoding::utf8_percent_encode(
                                &row.name,
                                percent_encoding::NON_ALPHANUMERIC
                            )
                        )
                    }),
                });
            }
        }
        if shown.len() == limit {
            break;
        }
    }
    Ok((shown, as_of))
}
