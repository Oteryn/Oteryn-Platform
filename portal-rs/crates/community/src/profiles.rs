//! Public name lookup. Projection facts never establish Platform privacy permission.
use askama::Template;
use axum::{
    Router,
    extract::{Path, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use unicode_normalization::UnicodeNormalization;

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
struct Profiles {
    db: PgPool,
    privacy: Arc<dyn Privacy>,
    details: Arc<dyn details::PublicDetails>,
}
pub fn configured_router(db: PgPool) -> Router {
    router(
        db.clone(),
        std::env::var("PORTAL_PUBLIC_PROFILES_ENABLED").as_deref() == Ok("true"),
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
        .with_state(Profiles {
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
}
fn page(en: bool, status: StatusCode, message: &str, character: Option<Character>) -> Response {
    let c = character.unwrap_or_default();
    let page = Page {
        lang: if en { "en" } else { "pl" },
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
            .map(|(key, value)| (text(en, &key), value))
            .collect(),
        comment: c.comment,
    };
    match page.render() {
        Ok(body) => (status, [(header::CACHE_CONTROL,"no-store, private"),(header::CONTENT_LANGUAGE,page.lang),(header::X_CONTENT_TYPE_OPTIONS,"nosniff"),(header::CONTENT_SECURITY_POLICY,"default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),(header::REFERRER_POLICY,"no-referrer"),(header::STRICT_TRANSPORT_SECURITY,"max-age=63072000; includeSubDomains"),(axum::http::HeaderName::from_static("permissions-policy"),"camera=(), microphone=(), geolocation=(), payment=()")], Html(body)).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.nfc().eq(name.chars())
        && name.bytes().all(|b| {
            (32..=126).contains(&b) && !matches!(b, b'/' | b'\\' | b'"' | b'%' | b'?' | b'#')
        })
        && !name.chars().all(|c| c == '.')
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
    let query = if raw.len() <= 1024 {
        serde_urlencoded::from_str::<Search>(&raw).ok()
    } else {
        None
    };
    let Some(query) = query.filter(|q| valid_name(q.name.trim())) else {
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
    sections: Vec<(String, String)>,
    #[serde(skip)]
    comment: String,
}
async fn profile(
    State(state): State<Profiles>,
    Path((locale, name)): Path<(String, String)>,
    RawQuery(query): RawQuery,
) -> Response {
    let Some(en) = language(&locale) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_name(&name) || query.is_some() {
        return page(en, StatusCode::BAD_REQUEST, "invalid", None);
    }
    match load(&state, &name).await {
        Ok(Some(character)) => page(en, StatusCode::OK, "", Some(character)),
        Ok(None) => page(en, StatusCode::NOT_FOUND, "missing", None),
        Err(_) => page(en, StatusCode::SERVICE_UNAVAILABLE, "unavailable", None),
    }
}
async fn load(state: &Profiles, name: &str) -> Result<Option<Character>, Unavailable> {
    // Keep epoch, watermark, world resolution and name handoff in one committed cut.
    let mut tx = state.db.begin().await.map_err(|_| Unavailable)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| Unavailable)?;
    let fresh: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='character_profile' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").fetch_one(&mut *tx).await.map_err(|_|Unavailable)?;
    if !fresh {
        return Err(Unavailable);
    }
    let worlds: Vec<String> = sqlx::query_scalar("SELECT DISTINCT payload->'profile'->0->>'world_id' FROM public_snapshots WHERE family='character_profile' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND jsonb_array_length(payload->'profile')=1 LIMIT 2").fetch_all(&mut *tx).await.map_err(|_|Unavailable)?;
    if worlds.len() != 1 {
        return Err(Unavailable);
    }
    let rows: Vec<(String,serde_json::Value,String)> = sqlx::query_as("SELECT subject,payload,epoch::text FROM public_snapshots WHERE family='character_profile' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND lower(payload->'profile'->0->>'name')=lower($1) LIMIT 2").bind(name).fetch_all(&mut *tx).await.map_err(|_|Unavailable)?;
    tx.commit().await.map_err(|_| Unavailable)?;
    if rows.len() != 1 {
        // Exercise the same authority boundary for negative lookups. An empty
        // subject can never authorize disclosure or become a stored mapping.
        let _ = state.privacy.visible("").await?;
        return Ok(None);
    }
    let (id, payload, epoch) = &rows[0];
    let value = &payload["profile"][0];
    if value["world_id"].as_str() != Some(worlds[0].as_str()) {
        return Err(Unavailable);
    }
    let mut character: Character =
        serde_json::from_value(value.clone()).map_err(|_| Unavailable)?;
    let subject = details::Subject {
        character_id: id.clone(),
        world_id: worlds[0].clone(),
        epoch: epoch.clone(),
    };
    // Consult the privacy authority after reading facts; no stored allow or response cache.
    if !state.privacy.visible(id).await? {
        return Ok(None);
    }
    let extra = state.details.current(&subject).await.unwrap_or_default();
    let fields = state.privacy.fields(id).await.unwrap_or_default();
    if fields.guild
        && value["guild_visible"] == true
        && value["guild"].as_array().is_some_and(|g| g.len() == 1)
        && let Some((guild, rank)) = extra.guild
    {
        character
            .sections
            .push(("guild".into(), format!("{guild} · {rank}")));
    }
    if fields.house
        && let Some(house) = extra.house
    {
        character.sections.push(("house".into(), house));
    }
    if fields.skills
        && let Some(skills) = extra.skills
        && skills.iter().all(|n| *n <= 1000)
        && skills[1..].iter().all(|n| *n >= 10)
    {
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
            character.sections.push((key.into(), number.to_string()));
        }
    }
    if fields.deaths
        && let Some(deaths) = extra.deaths
        && deaths.len() <= 20
    {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        for death in deaths.into_iter().filter(|d| {
            d.occurred_at > 0 && d.occurred_at <= now && d.occurred_at >= now - 30 * 86400
        }) {
            if let Ok(at) = time::OffsetDateTime::from_unix_timestamp(death.occurred_at)
                && let Ok(at) = at.format(&time::format_description::well_known::Rfc3339)
            {
                character
                    .sections
                    .push(("death".into(), format!("{at} · {}", death.level)));
            }
        }
    }
    if !state.privacy.visible(id).await? {
        return Ok(None);
    }
    // External readers may await I/O. Recheck source eligibility at serving time,
    // so an epoch raise or expired watermark during those waits cannot disclose facts.
    let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='character_profile' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND epoch::text=$1 AND watermark BETWEEN floor(extract(epoch FROM statement_timestamp()))::bigint-29 AND floor(extract(epoch FROM statement_timestamp()))::bigint+1 AND EXISTS(SELECT 1 FROM public_snapshots current_profile WHERE current_profile.family='character_profile' AND current_profile.subject=$2 AND NOT current_profile.is_watermark AND current_profile.epoch=public_snapshots.epoch AND current_profile.payload=$3))")
        .bind(epoch).bind(id).bind(payload).fetch_one(&state.db).await.map_err(|_|Unavailable)?;
    if !eligible {
        return Err(Unavailable);
    }
    character.comment = fields
        .comment
        .filter(|s| s.chars().count() <= 2000)
        .unwrap_or_default();
    Ok(Some(character))
}
#[cfg(test)]
#[path = "profiles_tests.rs"]
mod tests;
