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
use sqlx::{PgPool, Row};
use std::{future::Future, pin::Pin, sync::Arc};

/// Decisions must be current at serving time; errors must never reuse an earlier allow.
#[derive(Debug)]
pub struct CharacterRef {
    pub id: String,
    pub world: String,
    pub name: String,
}
pub trait PublicVisibility: Send + Sync {
    fn is_hidden<'a>(&'a self, character_ref: &'a str) -> VisibilityFuture<'a, bool>;
    fn references<'a>(&'a self, characters: &'a [CharacterRef]) -> VisibilityFuture<'a, Vec<bool>> {
        Box::pin(async move {
            let ids: Vec<_> = characters.iter().map(|c| c.id.clone()).collect();
            self.batch(&ids).await
        })
    }
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
        db.clone(),
        std::env::var("PORTAL_PUBLIC_RANKINGS_ENABLED").as_deref() == Ok("true"),
        Arc::new(crate::privacy::Authority::configured(db)),
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
    skill: Option<String>,
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
    kind: String,
    title: String,
    message: String,
    as_of: String,
    computed_at: String,
    world_label: String,
    vocation_label: String,
    apply: String,
    kind_label: String,
    worlds: Vec<(String, String, bool)>,
    filters_available: bool,
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
            kind: String::new(),
            title: text("rankings-title"),
            message: String::new(),
            as_of: text("rankings-computed-at"),
            computed_at: String::new(),
            world_label: text("rankings-world"),
            vocation_label: text("rankings-vocation"),
            apply: text("rankings-apply"),
            kind_label: text("rankings-category"),
            worlds: vec![],
            filters_available: false,
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
    if !KINDS.iter().any(|(k, _)| *k == kind) {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    }
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
    if !(1..=MAX_ROWS / PAGE_SIZE).contains(&number) || page_number != number.to_string() {
        return failure(lang, StatusCode::BAD_REQUEST, "rankings-bad-query");
    }
    let kind = filters.skill.as_deref().unwrap_or(&kind);
    let Some((_, category)) = KINDS.iter().find(|(k, _)| *k == kind) else {
        return failure(lang, StatusCode::NOT_FOUND, "rankings-not-found");
    };
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
    let mut page = Page::new(lang);
    page.kind = kind.into();
    page.vocation = vocation.into();
    page.page = number;
    page.filters_available = true;
    page.title = format!(
        "{} · {}",
        page.title,
        lang.text(&format!("rankings-{kind}"))
    );
    if kind == "level" {
        page.headings.pop();
    }
    let result = load_selected(&state, lang, &mut page, *category, filters.world.as_deref()).await;
    match result {
        Ok(()) => page.response(StatusCode::OK),
        Err(Unavailable::NotFound) => {
            page.message = lang.text("rankings-not-found");
            page.response(StatusCode::NOT_FOUND)
        }
        Err(reason) => {
            tracing::debug!(?reason, world = %page.world, category, vocation, "rankings unavailable");
            page.message = lang.text("rankings-unavailable");
            page.response(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

async fn load_selected(
    state: &Rankings,
    lang: Language,
    page: &mut Page,
    category: u8,
    selected: Option<&str>,
) -> Result<(), Unavailable> {
    page.worlds = worlds(&state.db, category, &page.vocation).await?;
    let world = match selected {
        Some(id) => page
            .worlds
            .iter()
            .find(|(world, _, _)| world == id)
            .ok_or(Unavailable::NotFound)?,
        None => page
            .worlds
            .iter()
            .find(|(_, _, published)| *published)
            .or_else(|| page.worlds.first())
            .ok_or(Unavailable::MissingWorld)?,
    };
    page.world.clone_from(&world.0);
    load(state, lang, page, category).await
}

async fn worlds(
    db: &PgPool,
    category: u8,
    vocation: &str,
) -> Result<Vec<(String, String, bool)>, Unavailable> {
    let worlds = sqlx::query_as(
        "SELECT w.id::text, w.name, EXISTS (
            SELECT 1 FROM public_snapshots s
            JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch
            WHERE s.family='highscores' AND NOT s.is_watermark
              AND s.subject=w.id::text || ':' || $1 || ':' || $2
         )
         FROM world_status w ORDER BY w.name COLLATE \"C\", w.id LIMIT 257",
    )
    .bind(category.to_string())
    .bind(vocation)
    .fetch_all(db)
    .await?;
    if worlds.len() > 256 {
        return Err(Unavailable::WorldRegistryLimit);
    }
    Ok(worlds)
}

async fn load(
    state: &Rankings,
    lang: Language,
    page: &mut Page,
    category: u8,
) -> Result<(), Unavailable> {
    let privacy_cut = crate::privacy::Cut::read(&state.db)
        .await
        .map_err(Unavailable::from)?;
    let subject = format!("{}:{category}:{}", page.world, page.vocation);
    let cut = RankingCut::read(&state.db, &subject).await?;
    let snapshot = cut.snapshot(&page.world, category, &page.vocation)?;
    let refs: Vec<_> = snapshot
        .rows
        .iter()
        .map(|r| r.reference(&page.world))
        .collect();
    let hidden = state
        .visibility
        .references(&refs)
        .await
        .map_err(|_| Unavailable::Privacy)?;
    if hidden.len() != refs.len() {
        return Err(Unavailable::Privacy);
    }
    let visible: Vec<_> = snapshot
        .rows
        .into_iter()
        .zip(hidden)
        .filter_map(|(row, hidden)| (!hidden).then_some(row))
        .collect();
    cut.validate(&state.db, &subject, &privacy_cut).await?;
    let page_count = visible.len().div_ceil(PAGE_SIZE).max(1);
    if page.page > page_count {
        return Err(Unavailable::NotFound);
    }
    let as_of = cut
        .as_of
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| Unavailable::InvalidSnapshot)?;
    let url = |number| {
        format!(
            "/{}/rankings/{}?world={}&vocation={}&page={number}",
            lang.code(),
            page.kind,
            page.world,
            page.vocation,
        )
    };
    if page.page > 1 {
        page.previous_url = url(page.page - 1);
    }
    if page.page < page_count {
        page.next_url = url(page.page + 1);
    }
    // Producer positions and order survive filtering and pagination, including ties.
    page.rows = visible
        .into_iter()
        .skip((page.page - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .map(|row| {
            let vocation = match row.vocation {
                0 => lang.text("rankings-vocation-none"),
                1..=5 => page.vocations[usize::from(row.vocation)].1.clone(),
                v => format!("{} {v}", page.vocation_label),
            };
            let mut cells = vec![row.rank, row.name, vocation, row.level];
            if page.kind != "level" {
                cells.push(row.value);
            }
            cells
        })
        .collect();
    if page.rows.is_empty() {
        page.message = lang.text("rankings-empty");
    }
    page.computed_at = as_of;
    Ok(())
}

impl Entry {
    fn reference(&self, world: &str) -> CharacterRef {
        CharacterRef {
            id: self.character_id.clone(),
            world: world.into(),
            name: self.name.clone(),
        }
    }
}

const PAGE_SIZE: usize = 50;
const MAX_ROWS: usize = 1000;
const WATERMARK_MAX_AGE: i64 = 29;
const SNAPSHOT_MAX_AGE: i64 = 299;
const CLOCK_UNCERTAINTY: i64 = 1;

#[derive(Debug)]
enum Unavailable {
    WorldRegistryLimit,
    Conflict,
    MissingWatermark,
    StaleWatermark,
    MissingSnapshot,
    StaleSnapshot,
    InvalidSnapshot,
    Privacy,
    Changed,
    Database,
    MissingWorld,
    NotFound,
}

impl From<sqlx::Error> for Unavailable {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RankingCut {
    epoch: String,
    revision: String,
    observed: i64,
    as_of: time::OffsetDateTime,
    payload: serde_json::Value,
}

impl RankingCut {
    async fn read(
        db: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
        subject: &str,
    ) -> Result<Self, Unavailable> {
        let row = sqlx::query(
            "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint AS now,
                    public_projection_conflicted('highscores') AS conflicted,
                    w.watermark AS complete_through, p.highest_epoch::text AS epoch,
                    s.version::text AS revision, s.watermark AS observed, s.payload
             FROM public_projection_state p
             LEFT JOIN public_snapshots w
               ON w.family='highscores' AND w.subject='' AND w.is_watermark AND w.epoch=p.highest_epoch
             LEFT JOIN public_snapshots s
               ON s.family='highscores' AND s.subject=$1 AND NOT s.is_watermark AND s.epoch=p.highest_epoch
             WHERE p.id=1",
        )
        .bind(subject)
        .fetch_one(db)
        .await?;
        if row.try_get("conflicted")? {
            return Err(Unavailable::Conflict);
        }
        let complete_through = row
            .try_get::<Option<i64>, _>("complete_through")?
            .ok_or(Unavailable::MissingWatermark)?;
        let now: i64 = row.try_get("now")?;
        if !(now - WATERMARK_MAX_AGE..=now + CLOCK_UNCERTAINTY).contains(&complete_through) {
            return Err(Unavailable::StaleWatermark);
        }
        let observed = row
            .try_get::<Option<i64>, _>("observed")?
            .ok_or(Unavailable::MissingSnapshot)?;
        let payload: serde_json::Value = row.try_get("payload")?;
        let computed = payload["computed_at"]
            .as_str()
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or(Unavailable::InvalidSnapshot)?;
        if !(now - SNAPSHOT_MAX_AGE..=observed.min(now + CLOCK_UNCERTAINTY)).contains(&computed)
            || observed > complete_through
        {
            return Err(Unavailable::StaleSnapshot);
        }
        Ok(Self {
            epoch: row.try_get("epoch")?,
            revision: row.try_get("revision")?,
            observed,
            as_of: time::OffsetDateTime::from_unix_timestamp(computed)
                .map_err(|_| Unavailable::InvalidSnapshot)?,
            payload,
        })
    }

    fn snapshot(&self, world: &str, category: u8, vocation: &str) -> Result<Snapshot, Unavailable> {
        let snapshot =
            Snapshot::deserialize(&self.payload).map_err(|_| Unavailable::InvalidSnapshot)?;
        if snapshot.world_id != world
            || snapshot.category != category
            || snapshot.filter.to_string() != vocation
            || snapshot.rows.len() > MAX_ROWS
        {
            return Err(Unavailable::InvalidSnapshot);
        }
        Ok(snapshot)
    }

    async fn validate(
        &self,
        db: &PgPool,
        subject: &str,
        privacy_cut: &crate::privacy::Cut,
    ) -> Result<(), Unavailable> {
        let mut tx = crate::privacy::read_fence(db).await?;
        sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR SHARE")
            .fetch_one(&mut *tx)
            .await?;
        if !privacy_cut.validate(&mut tx).await? {
            return Err(Unavailable::Changed);
        }
        // Check age after the locks and privacy lookups, using the database clock.
        if Self::read(&mut *tx, subject).await? != *self {
            return Err(Unavailable::Changed);
        }
        tx.commit().await?;
        Ok(())
    }
}
/// Top visible characters of the first registered world's level ranking.
pub async fn home_top(
    db: &PgPool,
    visibility: &dyn PublicVisibility,
    lang: Language,
    limit: usize,
    profiles: bool,
) -> Result<(Vec<oteryn_ui::HomeRank>, String), VisibilityUnavailable> {
    home_top_inner(db, visibility, lang, limit, profiles)
        .await
        .map_err(|reason| {
            tracing::debug!(?reason, "home rankings unavailable");
            VisibilityUnavailable
        })
}

async fn home_top_inner(
    db: &PgPool,
    visibility: &dyn PublicVisibility,
    lang: Language,
    limit: usize,
    profiles: bool,
) -> Result<(Vec<oteryn_ui::HomeRank>, String), Unavailable> {
    let privacy_cut = crate::privacy::Cut::read(db).await?;
    let worlds = worlds(db, 1, "0").await?;
    let (world, _, _) = worlds
        .iter()
        .find(|(_, _, published)| *published)
        .ok_or(Unavailable::MissingSnapshot)?;
    let subject = format!("{world}:1:0");
    let cut = RankingCut::read(db, &subject).await?;
    let snapshot = cut.snapshot(world, 1, "0")?;
    let as_of = cut
        .as_of
        .format(time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]"
        ))
        .map_err(|_| Unavailable::InvalidSnapshot)?;
    if snapshot.rows.is_empty() {
        visibility
            .references(&[])
            .await
            .map_err(|_| Unavailable::Privacy)?;
    }
    let mut shown = Vec::with_capacity(limit.min(MAX_ROWS));
    // Privacy checks stop once enough visible rows are found.
    for chunk in snapshot.rows.chunks(10) {
        let refs: Vec<_> = chunk.iter().map(|r| r.reference(world)).collect();
        let hidden = visibility
            .references(&refs)
            .await
            .map_err(|_| Unavailable::Privacy)?;
        if hidden.len() != refs.len() {
            return Err(Unavailable::Privacy);
        }
        shown.extend(
            chunk
                .iter()
                .zip(hidden)
                .filter(|(_, hidden)| !hidden)
                .take(limit - shown.len())
                .map(|(row, _)| oteryn_ui::HomeRank {
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
                }),
        );
        if shown.len() == limit {
            break;
        }
    }
    cut.validate(db, &subject, &privacy_cut).await?;
    Ok((shown, as_of))
}
