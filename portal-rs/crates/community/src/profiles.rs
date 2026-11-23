//! Public name lookup. Projection facts never establish Platform privacy permission.
use askama::Template;
use axum::{
    Router,
    extract::{Path, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::search::{Hit, Provider, segment};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

#[path = "profiles_details.rs"]
pub mod details;

pub type DecisionFuture<'a> = Pin<Box<dyn Future<Output = Result<bool, Unavailable>> + Send + 'a>>;
#[derive(Debug)]
pub struct Unavailable;
/// T45/T09c seam: resolve current ownership and monotonic privacy floors at serve time.
/// True means publicly visible; missing, ambiguous or unreadable policy must error.
/// Implementations must include account-wide hiding and current character hiding.
pub trait Privacy: Send + Sync {
    /// The empty subject is a negative lookup: check authority health and return
    /// false when healthy, or an error when unavailable; never create a mapping.
    fn visible<'a>(&'a self, character_id: &'a str) -> DecisionFuture<'a>;
    /// Missing field authority hides every optional section. Implementations must
    /// combine T45 switches with T09c's current account bounds where applicable.
    fn fields<'a>(
        &'a self,
        _character_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<details::Visibility, Unavailable>> + Send + 'a>> {
        Box::pin(async { Ok(details::Visibility::default()) })
    }
}
#[derive(Debug)]
pub struct FailClosed;
impl Privacy for FailClosed {
    fn visible<'a>(&'a self, _: &'a str) -> DecisionFuture<'a> {
        Box::pin(async { Err(Unavailable) })
    }
}
#[derive(Clone)]
pub struct Directory {
    db: PgPool,
    privacy: Arc<dyn Privacy>,
    details: Arc<dyn details::PublicDetails>,
}
impl std::fmt::Debug for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Directory").finish_non_exhaustive()
    }
}

pub fn enabled() -> bool {
    std::env::var("PORTAL_PUBLIC_PROFILES_ENABLED").as_deref() == Ok("true")
}

impl Directory {
    pub fn configured(db: PgPool) -> Self {
        Self {
            privacy: Arc::new(crate::privacy::Authority::configured(db.clone())),
            db,
            details: Arc::new(details::MissingDetails),
        }
    }

    pub async fn identity_by_name(&self, name: &str) -> Result<Option<Identity>, Unavailable> {
        self.identity(Lookup::Name(name)).await
    }

    pub async fn identity_by_id(&self, id: Uuid) -> Result<Option<Identity>, Unavailable> {
        self.identity(Lookup::Id(id)).await
    }

    async fn identity(&self, lookup: Lookup<'_>) -> Result<Option<Identity>, Unavailable> {
        load(self, lookup, false)
            .await?
            .map(|(id, character, fields)| {
                let id = Uuid::parse_str(&id).map_err(|_| Unavailable)?;
                if id.get_version_num() != 7 {
                    return Err(Unavailable);
                }
                Ok(Identity {
                    id,
                    name: character.name,
                    fields,
                })
            })
            .transpose()
    }
}

#[derive(Debug)]
pub struct Identity {
    pub id: Uuid,
    pub name: String,
    pub fields: details::Visibility,
}

pub fn configured_search(db: PgPool) -> Option<Provider> {
    enabled().then(|| search_provider(Directory::configured(db)))
}

fn search_provider(directory: Directory) -> Provider {
    Arc::new(move |lang, name, page| {
        let directory = directory.clone();
        Box::pin(async move {
            if language(&lang).is_none() || !valid_name(&name) || !(1..=1000).contains(&page) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let identity = directory
                .identity_by_name(&name)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(identity
                .filter(|_| page == 1)
                .map(|character| Hit {
                    href: format!("/{lang}/characters/{}", segment(&character.name)),
                    title: character.name,
                    kind: "characters".into(),
                })
                .into_iter()
                .collect())
        })
    })
}

pub fn configured_router(db: PgPool) -> Router {
    router(
        db.clone(),
        enabled(),
        Arc::new(crate::privacy::Authority::configured(db)),
    )
}
pub fn router(db: PgPool, enabled: bool, privacy: Arc<dyn Privacy>) -> Router {
    router_with_details(db, enabled, privacy, Arc::new(details::MissingDetails))
}
/// Wiring seam for the accepted public-family reader; enabling routes cannot
/// enable a missing privacy or source authority.
pub fn router_with_details(
    db: PgPool,
    enabled: bool,
    privacy: Arc<dyn Privacy>,
    details: Arc<dyn details::PublicDetails>,
) -> Router {
    if !enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/characters", get(search))
        .route("/{lang}/characters/{name}", get(profile))
        .with_state(Directory {
            db,
            privacy,
            details,
        })
}
fn language(locale: &str) -> Option<bool> {
    match locale {
        "en" => Some(true),
        "pl" => Some(false),
        _ => None,
    }
}
fn text(en: bool, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
    let mut bundle = FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    let source = if en {
        include_str!("../../../templates/profiles/en.ftl")
    } else {
        include_str!("../../../templates/profiles/pl.ftl")
    };
    if let Ok(resource) = FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "profiles/page.html", config = "src/profiles.askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    name_label: String,
    search: String,
    message: String,
    name: String,
    level_label: String,
    level: String,
    vocation_label: String,
    vocation: String,
    online_label: String,
    unavailable: String,
    sections: Vec<(String, String)>,
    comment: String,
    portal_search: String,
    compare_label: String,
    compare_href: Option<String>,
}
fn page(en: bool, status: StatusCode, message: &str, character: Option<Character>) -> Response {
    let c = character.unwrap_or_default();
    let lang = if en { "en" } else { "pl" };
    let compare_href = (!c.name.is_empty()
        && std::env::var("PORTAL_CHARACTER_STATS_ENABLED").as_deref() == Ok("true"))
    .then(|| format!("/{lang}/stats/compare?first={}", segment(&c.name)));
    let page = Page {
        lang,
        title: text(en, "title"),
        name_label: text(en, "name"),
        search: text(en, "search"),
        message: text(en, message),
        name: c.name,
        level_label: text(en, "level"),
        level: c.level,
        vocation_label: text(en, "vocation"),
        vocation: c.vocation.to_string(),
        online_label: text(en, "online"),
        unavailable: text(en, "unavailable"),
        sections: c
            .sections
            .into_iter()
            .map(|(key, value)| {
                (
                    text(en, key),
                    value.unwrap_or_else(|| text(en, "unavailable")),
                )
            })
            .collect(),
        comment: c.comment,
        portal_search: text(en, "portal-search"),
        compare_label: text(en, "compare"),
        compare_href,
    };
    match page.render() {
        Ok(body) => (status, [(header::CACHE_CONTROL,"no-store, private"),(header::CONTENT_LANGUAGE,page.lang),(header::X_CONTENT_TYPE_OPTIONS,"nosniff"),(header::CONTENT_SECURITY_POLICY,"default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),(header::REFERRER_POLICY,"no-referrer"),(header::STRICT_TRANSPORT_SECURITY,"max-age=63072000; includeSubDomains"),(axum::http::HeaderName::from_static("permissions-policy"),"camera=(), microphone=(), geolocation=(), payment=()")], Html(body)).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub fn valid_name(name: &str) -> bool {
    crate::ingest::valid_projection_name(name)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    name: String,
}
async fn search(Path(locale): Path<String>, RawQuery(raw): RawQuery) -> Response {
    let Some(en) = language(&locale) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(raw) = raw else {
        return page(en, StatusCode::OK, "", None);
    };
    let query = if raw.len() <= 1024
        && raw.as_bytes().iter().enumerate().all(|(i, byte)| {
            *byte != b'%'
                || raw
                    .as_bytes()
                    .get(i + 1..i + 3)
                    .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit))
        })
        && percent_encoding::percent_decode_str(&raw)
            .decode_utf8()
            .is_ok()
    {
        serde_urlencoded::from_str::<Search>(&raw).ok()
    } else {
        None
    };
    let Some(query) = query.filter(|q| valid_name(&q.name) && valid_name(q.name.trim())) else {
        return page(en, StatusCode::BAD_REQUEST, "invalid", None);
    };
    let name = percent_encoding::utf8_percent_encode(
        query.name.trim(),
        percent_encoding::NON_ALPHANUMERIC,
    );
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, format!("/{locale}/characters/{name}")),
            (header::CACHE_CONTROL, "no-store, private".to_owned()),
        ],
    )
        .into_response()
}
#[derive(Default, serde::Deserialize)]
struct Character {
    name: String,
    level: String,
    vocation: u8,
    #[serde(skip)]
    sections: Vec<(&'static str, Option<String>)>,
    #[serde(skip)]
    comment: String,
}
async fn profile(
    State(state): State<Directory>,
    Path((locale, name)): Path<(String, String)>,
    RawQuery(query): RawQuery,
) -> Response {
    let Some(en) = language(&locale) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_name(&name) || query.is_some() {
        return page(en, StatusCode::BAD_REQUEST, "invalid", None);
    }
    match load(&state, Lookup::Name(&name), true).await {
        Ok(Some((_, character, _))) => page(en, StatusCode::OK, "", Some(character)),
        Ok(None) => page(en, StatusCode::NOT_FOUND, "missing", None),
        Err(_) => page(en, StatusCode::SERVICE_UNAVAILABLE, "unavailable", None),
    }
}
enum Lookup<'a> {
    Name(&'a str),
    Id(Uuid),
}

async fn load(
    state: &Directory,
    lookup: Lookup<'_>,
    include_details: bool,
) -> Result<Option<(String, Character, details::Visibility)>, Unavailable> {
    let (name, subject) = match lookup {
        Lookup::Name(name) => (Some(name), None),
        Lookup::Id(id) => (None, Some(id.to_string())),
    };
    let privacy_cut = crate::privacy::Cut::read(&state.db)
        .await
        .map_err(|_| Unavailable)?;
    // Keep epoch, watermark, world resolution and name handoff in one committed cut.
    let mut tx = state.db.begin().await.map_err(|_| Unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| Unavailable)?;
    let fresh: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='character_profile' AND NOT public_projection_conflicted('character_profile') AND subject='' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").fetch_one(&mut *tx).await.map_err(|_|Unavailable)?;
    if !fresh {
        return Err(Unavailable);
    }
    let rows: Vec<(String, serde_json::Value, String)> = sqlx::query_as(
        "SELECT s.subject,s.payload,s.epoch::text
         FROM public_snapshots s
         JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch
         JOIN public_snapshots w ON w.family=s.family AND w.subject='' AND w.is_watermark
                                 AND w.epoch=s.epoch AND s.watermark <= w.watermark
         WHERE s.family='character_profile' AND NOT s.is_watermark
           AND ($2::text=s.subject OR lower(s.payload->'profile'->0->>'name')=lower($1))
           AND jsonb_array_length(s.payload->'profile')=1
         LIMIT 2",
    )
    .bind(name)
    .bind(subject)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| Unavailable)?;
    tx.commit().await.map_err(|_| Unavailable)?;
    if rows.len() != 1 {
        // Exercise the same authority boundary for negative lookups. An empty
        // subject can never authorize disclosure or become a stored mapping.
        let _ = state.privacy.visible("").await?;
        return Ok(None);
    }
    let (id, payload, epoch) = &rows[0];
    let value = &payload["profile"][0];
    let world = value["world_id"].as_str().ok_or(Unavailable)?;
    let mut character: Character =
        serde_json::from_value(value.clone()).map_err(|_| Unavailable)?;
    if !valid_name(&character.name) {
        return Err(Unavailable);
    }
    // Consult the privacy authority after reading facts; no stored allow or response cache.
    if !state.privacy.visible(id).await? {
        return Ok(None);
    }
    let mut fields = state.privacy.fields(id).await.unwrap_or_default();
    fields.guild &= value["guild_visible"] == true;
    if include_details {
        let subject = details::Subject {
            character_id: id.clone(),
            world_id: world.to_owned(),
            epoch: epoch.clone(),
        };
        let extra = state.details.current(&subject).await.unwrap_or_default();
        add_details(&mut character, value, extra, &fields);
    }
    if !state.privacy.visible(id).await? {
        return Ok(None);
    }
    // External readers may await I/O. Recheck source eligibility at serving time,
    // so an epoch raise or expired watermark during those waits cannot disclose facts.
    let mut final_cut = crate::privacy::read_fence(&state.db)
        .await
        .map_err(|_| Unavailable)?;
    sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR SHARE")
        .execute(&mut *final_cut)
        .await
        .map_err(|_| Unavailable)?;
    let through: Option<i64> = sqlx::query_scalar("SELECT watermark FROM public_snapshots WHERE family='character_profile' AND subject='' AND is_watermark AND epoch=$1::numeric").bind(epoch).fetch_optional(&mut *final_cut).await.map_err(|_| Unavailable)?;
    let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='character_profile' AND NOT public_projection_conflicted('character_profile') AND subject='' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND epoch::text=$1 AND watermark BETWEEN floor(extract(epoch FROM statement_timestamp()))::bigint-29 AND floor(extract(epoch FROM statement_timestamp()))::bigint+1 AND EXISTS(SELECT 1 FROM public_snapshots current_profile WHERE current_profile.family='character_profile' AND NOT public_projection_conflicted('character_profile') AND current_profile.subject=$2 AND NOT current_profile.is_watermark AND current_profile.epoch=public_snapshots.epoch AND current_profile.payload=$3 AND current_profile.watermark<=public_snapshots.watermark))")
        .bind(epoch).bind(id).bind(payload).fetch_one(&mut *final_cut).await.map_err(|_|Unavailable)?;
    if !eligible {
        return Err(Unavailable);
    }
    if !privacy_cut
        .validate(&mut final_cut)
        .await
        .map_err(|_| Unavailable)?
    {
        return Err(Unavailable);
    }
    final_cut.commit().await.map_err(|_| Unavailable)?;
    let current = time::OffsetDateTime::now_utc().unix_timestamp();
    let through = through.ok_or(Unavailable)?;
    if through > current.saturating_add(1) || current.saturating_sub(through).saturating_add(1) > 30
    {
        return Err(Unavailable);
    }
    Ok(Some((id.clone(), character, fields)))
}

fn add_details(
    character: &mut Character,
    value: &serde_json::Value,
    extra: details::Details,
    fields: &details::Visibility,
) {
    if fields.guild && value["guild"].as_array().is_some_and(|g| g.len() == 1) {
        let guild = extra.guild.map(|(guild, rank)| format!("{guild} · {rank}"));
        character.sections.push(("guild", guild));
    }
    if fields.house {
        character.sections.push(("house", extra.house));
    }
    if fields.skills {
        if let Some(skills) = extra.skills.filter(|skills| {
            skills.iter().all(|n| *n <= 1000) && skills[1..].iter().all(|n| *n >= 10)
        }) {
            for (key, number) in [
                "magic",
                "fist",
                "club",
                "sword",
                "axe",
                "distance",
                "shielding",
                "fishing",
            ]
            .into_iter()
            .zip(skills)
            {
                character.sections.push((key, Some(number.to_string())));
            }
        } else {
            character.sections.push(("skills", None));
        }
    }
    if fields.deaths {
        if let Some(deaths) = extra.deaths.filter(|deaths| deaths.len() <= 20) {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            for death in deaths.into_iter().filter(|d| {
                d.occurred_at > 0 && d.occurred_at <= now && d.occurred_at >= now - 30 * 86400
            }) {
                if let Ok(at) = time::OffsetDateTime::from_unix_timestamp(death.occurred_at)
                    && let Ok(at) = at.format(&time::format_description::well_known::Rfc3339)
                {
                    character
                        .sections
                        .push(("death", Some(format!("{at} · {}", death.level))));
                }
            }
        } else {
            character.sections.push(("deaths", None));
        }
    }
    character.comment = fields
        .comment
        .as_ref()
        .filter(|s| s.chars().count() <= 2000)
        .cloned()
        .unwrap_or_default();
}
#[cfg(test)]
#[path = "profiles_tests.rs"]
mod tests;
