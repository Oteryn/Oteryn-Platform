//! Presentation. Skills are public behind profile eligibility and the T45 privacy
//! floor. Achievements, equipment and outfit are served only to the current owner
//! proven by Character Authority (`oteryn_playerdata::authority::ownership`) and only
//! when the snapshot's `character_revision` meets that ownership cut. No staff
//! permission widens the owner-only views; there is no public view of them.
use super::{Family, now, storage, wire};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{OriginalUri, Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_playerdata::authority::{Ownership, ownership};
use oteryn_web::session::{self, BrowserSession, SessionLayer};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

#[derive(Debug)]
pub struct Unavailable;
pub type PrivacyFuture<'a> = Pin<Box<dyn Future<Output = Result<bool, Unavailable>> + Send + 'a>>;
/// Current monotonic Platform privacy decision for public skills. Missing evidence denies.
pub trait SkillsPrivacy: Send + Sync {
    fn allows_skills<'a>(&'a self, character: &'a str, epoch: &'a str) -> PrivacyFuture<'a>;
}
#[derive(Debug)]
pub struct DenySkills;
impl SkillsPrivacy for DenySkills {
    fn allows_skills<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async { Err(Unavailable) })
    }
}

/// Per-family serving flags; a missing flag is off.
#[derive(Debug, Clone, Default)]
pub struct ServingConfig {
    pub skills: bool,
    pub achievements: bool,
    pub equipment: bool,
    pub outfit: bool,
    pub source_authority: String,
}
impl ServingConfig {
    fn permits(&self, family: Family) -> bool {
        wire::authority(&self.source_authority)
            && match family {
                Family::Skills => self.skills,
                Family::Achievements => self.achievements,
                Family::Equipment => self.equipment,
                Family::Outfit => self.outfit,
            }
    }
}

#[derive(Clone)]
struct Serving {
    db: PgPool,
    privacy: Arc<dyn SkillsPrivacy>,
    config: ServingConfig,
}

pub fn router(
    db: PgPool,
    privacy: Arc<dyn SkillsPrivacy>,
    config: ServingConfig,
    layer: SessionLayer,
) -> Router {
    let mut public = Router::new();
    let mut owner = Router::new();
    for prefix in ["", "/pl", "/en"] {
        public = public.route(&format!("{prefix}/characters/{{name}}/skills"), get(skills));
        for family in [Family::Achievements, Family::Equipment, Family::Outfit] {
            owner = owner.route(
                &format!("{prefix}/account/characters/{{name}}/{}", segment(family)),
                get(owned),
            );
        }
    }
    let owner = owner.route_layer(axum::middleware::from_fn_with_state(
        layer,
        session::middleware,
    ));
    public
        .merge(owner)
        .with_state(Serving {
            db,
            privacy,
            config,
        })
        .layer(axum::middleware::from_fn(policy))
}
fn segment(family: Family) -> &'static str {
    match family {
        Family::Skills => "skills",
        Family::Achievements => "achievements",
        Family::Equipment => "equipment",
        Family::Outfit => "outfit",
    }
}

/// Environment wiring: the T24 prerequisite in testing/preproduction plus one serving
/// flag per family, the T45 privacy adapter and the browser session layer.
pub fn configured_router(db: PgPool) -> Router {
    let on = |key: &str| std::env::var(key).as_deref() == Ok("true");
    let base = on("PORTAL_PUBLIC_PROJECTIONS_ENABLED")
        && matches!(
            std::env::var("APP_ENV").as_deref(),
            Ok("testing" | "preproduction")
        );
    let mut layer = SessionLayer::new(db.clone());
    layer.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    router(
        db.clone(),
        Arc::new(crate::privacy::Authority::configured(db)),
        ServingConfig {
            skills: base && on("PORTAL_G4_SKILLS_SERVING_ENABLED"),
            achievements: base && on("PORTAL_G4_ACHIEVEMENTS_SERVING_ENABLED"),
            equipment: base && on("PORTAL_G4_EQUIPMENT_SERVING_ENABLED"),
            outfit: base && on("PORTAL_G4_OUTFIT_SERVING_ENABLED"),
            source_authority: std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY")
                .unwrap_or_default(),
        },
        layer,
    )
}

async fn policy(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (header::CACHE_CONTROL, "private, no-store"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}

#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><main><h1>{{ title }}</h1><p>{{ name }}</p>{% if rows.is_empty() %}<p>{{ empty }}</p>{% else %}<dl>{% for row in rows %}<dt>{{ row.0 }}</dt><dd>{{ row.1 }}</dd>{% endfor %}</dl>{% endif %}<p>{{ note }}</p></main></body></html>",
    ext = "html"
)]
struct Page {
    lang: &'static str,
    title: String,
    name: String,
    rows: Vec<(String, String)>,
    empty: String,
    note: String,
}
fn label(en: bool, key: &str) -> String {
    let mut bundle = fluent_bundle::FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    let source = if en {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn render(en: bool, family: Family, name: &str, rows: Vec<(String, String)>) -> Response {
    let page = Page {
        lang: if en { "en" } else { "pl" },
        title: label(en, segment(family)),
        name: name.to_owned(),
        rows,
        empty: label(en, "empty"),
        note: label(
            en,
            if family == Family::Skills {
                "public-note"
            } else {
                "owner-note"
            },
        ),
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn date(seconds: &str) -> Result<String, Unavailable> {
    let seconds =
        i64::try_from(wire::decimal(seconds, 0, i64::MAX as u64).map_err(|_| Unavailable)?)
            .map_err(|_| Unavailable)?;
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| Unavailable)?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| Unavailable)
}

enum Refusal {
    NotFound,
    Forbidden,
    Unavailable,
}
impl From<Unavailable> for Refusal {
    fn from(_: Unavailable) -> Self {
        Self::Unavailable
    }
}
impl From<storage::Unavailable> for Refusal {
    fn from(_: storage::Unavailable) -> Self {
        Self::Unavailable
    }
}
impl From<sqlx::Error> for Refusal {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Refusal::NotFound => StatusCode::NOT_FOUND,
            Refusal::Forbidden => StatusCode::FORBIDDEN,
            Refusal::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}

async fn skills(
    State(s): State<Serving>,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
) -> Response {
    if !s.config.permits(Family::Skills) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !wire::name(&name) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let en = uri.path().starts_with("/en/");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        public_skills(&s, &name, en),
    )
    .await
    .unwrap_or(Err(Refusal::Unavailable))
    {
        Ok(rows) => render(en, Family::Skills, &name, rows),
        Err(refusal) => refusal.into_response(),
    }
}
async fn public_skills(
    s: &Serving,
    name: &str,
    en: bool,
) -> Result<Vec<(String, String)>, Refusal> {
    let authority = &s.config.source_authority;
    let privacy_cut = crate::privacy::Cut::read(&s.db).await?;
    let mut db = s.db.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *db)
        .await?;
    let (epoch, through) = storage::fresh(&mut db, Family::Skills, authority, now()).await?;
    // The one current eligible profile of this name on the same epoch, with a fresh
    // profile watermark. Unknown, hidden and ambiguous names share one not-found.
    let profiles: Vec<(String, String)> = sqlx::query_as("SELECT s.subject,s.payload->'profile'->0->>'world_id' FROM public_snapshots s WHERE s.family='character_profile' AND NOT public_projection_conflicted('character_profile') AND NOT s.is_watermark AND s.epoch=$1::numeric AND jsonb_array_length(s.payload->'profile')=1 AND s.payload->'profile'->0->>'name'=$2 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1) LIMIT 2")
        .bind(&epoch)
        .bind(name)
        .fetch_all(&mut *db)
        .await?;
    let [(subject, world)] = profiles.as_slice() else {
        return Err(Refusal::NotFound);
    };
    let character = Uuid::parse_str(subject).map_err(|_| Refusal::Unavailable)?;
    let (body, revision) = storage::body(
        &mut db,
        Family::Skills,
        character,
        authority,
        &epoch,
        through,
    )
    .await?
    .ok_or(Refusal::Unavailable)?;
    let records: Vec<wire::Skills> =
        serde_json::from_value(body.clone()).map_err(|_| Refusal::Unavailable)?;
    let Some(r) = records.first() else {
        return Err(Refusal::NotFound);
    };
    if &r.world_id != world {
        return Err(Refusal::Unavailable);
    }
    db.commit().await?;
    if !s.privacy.allows_skills(subject, &epoch).await? {
        return Err(Refusal::NotFound);
    }
    // Reacquire only after privacy; serialize the final evidence cut with publication.
    let mut tx = s.db.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE")
        .execute(&mut *tx)
        .await?;
    let (current_epoch, current_through) =
        storage::fresh(&mut tx, Family::Skills, authority, now()).await?;
    if current_epoch != epoch
        || storage::body(
            &mut tx,
            Family::Skills,
            character,
            authority,
            &epoch,
            current_through,
        )
        .await?
        .as_ref()
            != Some(&(body.clone(), revision))
    {
        return Err(Refusal::Unavailable);
    }
    let profile_current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots s WHERE s.family='character_profile' AND NOT public_projection_conflicted('character_profile') AND s.subject=$1 AND NOT s.is_watermark AND s.epoch=$2::numeric AND jsonb_array_length(s.payload->'profile')=1 AND s.payload->'profile'->0->>'name'=$3 AND s.payload->'profile'->0->>'world_id'=$4 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM clock_timestamp()))::bigint-29 AND floor(extract(epoch FROM clock_timestamp()))::bigint+1))")
        .bind(subject).bind(&epoch).bind(name).bind(world).fetch_one(&mut *tx).await?;
    if !profile_current {
        return Err(Refusal::Unavailable);
    }
    if !privacy_cut
        .validate(&mut tx)
        .await
        .map_err(|_| Refusal::Unavailable)?
    {
        return Err(Refusal::Unavailable);
    }
    let profile_through: i64 = sqlx::query_scalar("SELECT watermark FROM public_snapshots WHERE family='character_profile' AND is_watermark AND epoch=$1::numeric").bind(&epoch).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    serving_fresh(current_through.min(profile_through))?;
    Ok([
        ("level", r.level.clone()),
        ("experience", r.total_experience.clone()),
        ("magic", r.magic_level.to_string()),
        ("fist", r.fist_level.to_string()),
        ("club", r.club_level.to_string()),
        ("sword", r.sword_level.to_string()),
        ("axe", r.axe_level.to_string()),
        ("distance", r.distance_level.to_string()),
        ("shielding", r.shielding_level.to_string()),
        ("fishing", r.fishing_level.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (label(en, k), v))
    .collect())
}

async fn owned(
    State(s): State<Serving>,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    session: Option<Extension<BrowserSession>>,
) -> Response {
    let path = uri.path();
    let Some(family) = [Family::Achievements, Family::Equipment, Family::Outfit]
        .into_iter()
        .find(|f| path.ends_with(&format!("/{}", segment(*f))))
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !s.config.permits(family) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(Extension(session)) = session else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let Some(actor) = session.account_id().await else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if !wire::name(&name) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let en = path.starts_with("/en/");
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        owner_rows(&s, family, actor, &name, en),
    )
    .await
    .unwrap_or(Err(Refusal::Unavailable))
    {
        Ok(rows) => render(en, family, &name, rows),
        Err(refusal) => refusal.into_response(),
    }
}
async fn owner_rows(
    s: &Serving,
    family: Family,
    actor: Uuid,
    name: &str,
    en: bool,
) -> Result<Vec<(String, String)>, Refusal> {
    let authority = &s.config.source_authority;
    // Only the actor's own projected characters are candidates: another account's
    // character is indistinguishable from a missing one.
    let characters: Vec<Uuid> = sqlx::query_scalar(
        "SELECT character_id FROM account_characters WHERE account_id=$1 AND name=$2 LIMIT 2",
    )
    .bind(actor)
    .bind(name)
    .fetch_all(&s.db)
    .await?;
    let [character] = characters.as_slice() else {
        return Err(Refusal::NotFound);
    };
    // Ownership proof and snapshot read share one transaction and its row locks.
    let mut tx = s.db.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *tx)
        .await?;
    let cut = match ownership(&mut tx, actor, *character).await {
        Ok(Ownership::Owner(cut)) => cut,
        Ok(Ownership::Former | Ownership::Stranger) => return Err(Refusal::Forbidden),
        Err(_) => return Err(Refusal::Unavailable),
    };
    let (epoch, through) = storage::fresh(&mut tx, family, authority, now()).await?;
    let evidence = storage::body(&mut tx, family, *character, authority, &epoch, through)
        .await?
        .ok_or(Refusal::Unavailable)?;
    sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE")
        .execute(&mut *tx)
        .await?;
    let (current_epoch, through) = storage::fresh(&mut tx, family, authority, now()).await?;
    if current_epoch != epoch
        || storage::body(&mut tx, family, *character, authority, &epoch, through)
            .await?
            .as_ref()
            != Some(&evidence)
    {
        return Err(Refusal::Unavailable);
    }
    let (body, _) = evidence;
    tx.commit().await?;
    serving_fresh(through)?;
    // The snapshot must be bound to the current ownership relation: same World and a
    // character revision at or above the ownership-transition floor. An older
    // owner's snapshot is never shown to the new owner.
    let floor = |world: &str, revision: &str| -> Result<(), Refusal> {
        let revision = wire::decimal(revision, 1, u64::MAX).map_err(|_| Refusal::Unavailable)?;
        if Uuid::parse_str(world).ok() != Some(cut.world) || revision < cut.character_revision {
            return Err(Refusal::Unavailable);
        }
        Ok(())
    };
    let parse = |_| Refusal::Unavailable;
    let rows = match family {
        Family::Achievements => {
            let records: Vec<wire::Achievements> = serde_json::from_value(body).map_err(parse)?;
            let r = records.first().ok_or(Refusal::NotFound)?;
            floor(&r.world_id, &r.character_revision)?;
            r.earned
                .iter()
                .map(|e| Ok((e.key.clone(), date(&e.earned_at)?)))
                .collect::<Result<Vec<_>, Unavailable>>()?
        }
        Family::Equipment => {
            let records: Vec<wire::Equipment> = serde_json::from_value(body).map_err(parse)?;
            let r = records.first().ok_or(Refusal::NotFound)?;
            floor(&r.world_id, &r.character_revision)?;
            r.slots
                .iter()
                .map(|slot| (label(en, &slot.slot), slot.item_key.clone()))
                .collect()
        }
        Family::Outfit | Family::Skills => {
            let records: Vec<wire::Outfit> = serde_json::from_value(body).map_err(parse)?;
            let r = records.first().ok_or(Refusal::NotFound)?;
            floor(&r.world_id, &r.character_revision)?;
            let colours = r
                .colours
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            vec![
                (label(en, "outfit-key"), r.outfit_key.clone()),
                (label(en, "colours"), colours),
                (label(en, "addons"), r.addons.to_string()),
            ]
        }
    };
    Ok(rows)
}

fn serving_fresh(through: i64) -> Result<(), Refusal> {
    let through = u64::try_from(through).map_err(|_| Refusal::Unavailable)?;
    let current = now();
    if through > current.saturating_add(1) || current.saturating_sub(through).saturating_add(1) > 30
    {
        return Err(Refusal::Unavailable);
    }
    Ok(())
}
