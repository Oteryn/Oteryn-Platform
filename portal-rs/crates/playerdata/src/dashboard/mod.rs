//! Owner-only presentation seam until the private journal store is integrated.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(crate) fn denied() -> Box<dyn std::error::Error + Send + Sync> {
    "private dashboard unavailable or access denied".into()
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub dashboard: bool,
    pub reminders: bool,
    pub email: bool,
    pub web_push: bool,
}

/// Supplied by the authenticated session adapter, never browser parameters.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub account: Uuid,
    pub active: bool,
    pub staff_access: bool,
}
impl Actor {
    pub fn authorize(self, owner: Uuid) -> Result<()> {
        if !self.active || self.staff_access || self.account != owner || owner.is_nil() {
            return Err(denied());
        }
        Ok(())
    }
}

/// Receipt cut must match an independently current ownership publication exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cut {
    pub character_revision: u64,
    pub projection_revision: u64,
    pub projection_epoch: u64,
    pub recovery_generation: u64,
}
impl Cut {
    pub fn valid(self) -> bool {
        self.character_revision > 0
            && self.projection_revision > 0
            && self.projection_epoch > 0
            && self.recovery_generation > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub character: Uuid,
    pub cut: Cut,
    pub level: Option<u32>,
    /// Historical observation only; never a claim of current presence.
    pub online_at_observation: Option<bool>,
    pub stamina_minutes: Option<u16>,
    pub premium_until_ms: Option<i64>,
    pub available_bosses: Vec<String>,
    pub last_session_balance: Option<i64>,
    pub observed_at_ms: i64,
}

/// T72 adapter must enforce baseline completeness, family allowlists, source and
/// catalog binding, deletion, transfer suppression, and recovery fences before returning data.
/// None means unknown, including a missing dependency. Never synthesize an empty baseline.
pub trait Source: std::fmt::Debug + Send + Sync {
    fn characters(&self, owner: Uuid, now_ms: i64) -> Result<Option<Vec<Character>>>;
    fn current_cut(&self, owner: Uuid, character: Uuid, now_ms: i64) -> Result<Option<Cut>>;
}
#[derive(Debug)]
pub struct Unavailable;
impl Source for Unavailable {
    fn characters(&self, _: Uuid, _: i64) -> Result<Option<Vec<Character>>> {
        Ok(None)
    }
    fn current_cut(&self, _: Uuid, _: Uuid, _: i64) -> Result<Option<Cut>> {
        Ok(None)
    }
}

pub fn fresh(observed: i64, now: i64) -> bool {
    observed >= 0 && now >= observed && now.saturating_sub(observed) <= 10_000
}

pub fn load(
    flags: Flags,
    actor: Actor,
    owner: Uuid,
    source: &dyn Source,
    now: i64,
) -> Result<Vec<Character>> {
    actor.authorize(owner)?;
    if !flags.dashboard || now < 0 {
        return Err(denied());
    }
    let rows = source.characters(owner, now)?.ok_or_else(denied)?;
    if rows.len() > 256 {
        return Err(denied());
    }
    let mut ids = std::collections::HashSet::new();
    for row in &rows {
        if row.character.is_nil()
            || !ids.insert(row.character)
            || !row.cut.valid()
            || source.current_cut(owner, row.character, now)? != Some(row.cut)
            || !fresh(row.observed_at_ms, now)
            || row.stamina_minutes.is_some_and(|s| s > 2520)
            || row.available_bosses.len() > 256
            || row.level == Some(0)
            || row.premium_until_ms.is_some_and(|t| t < 0)
            || row.available_bosses.iter().any(|key| {
                key.is_empty()
                    || key.len() > 512
                    || !key
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._:/-".contains(&c))
            })
        {
            return Err(denied());
        }
    }
    Ok(rows)
}

/// SSR fragment uses semantic lists that wrap at narrow widths. The host supplies
/// the existing layout/CSS; there are no scripts, interactive controls or inline styles.
/// Reads are authorized again here so unvalidated rows cannot reach this presentation API.
pub fn render(
    flags: Flags,
    actor: Actor,
    owner: Uuid,
    source: &dyn Source,
    now: i64,
    language: &str,
) -> Result<String> {
    use askama::Template;
    use fluent_bundle::{FluentBundle, FluentResource};
    let rows = load(flags, actor, owner, source, now)?;
    let text = match language {
        "pl" => include_str!("pl.ftl"),
        "en" => include_str!("en.ftl"),
        _ => return Err(denied()),
    };
    let mut bundle = FluentBundle::new(vec![language.parse::<unic_langid::LanguageIdentifier>()?]);
    bundle
        .add_resource(FluentResource::try_new(text.to_owned()).map_err(|_| denied())?)
        .map_err(|_| denied())?;
    let tr = |key| -> Result<String> {
        let p = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or_else(denied)?;
        let mut errors = Vec::new();
        let value = bundle.format_pattern(p, None, &mut errors).into_owned();
        if errors.is_empty() {
            Ok(value)
        } else {
            Err(denied())
        }
    };
    #[derive(Template)]
    #[template(
        source = "<section lang=\"{{ language }}\" aria-labelledby=\"account-dashboard\"><h1 id=\"account-dashboard\">{{ title }}</h1><p>{{ historical }}</p><ul>{% for row in rows %}<li><h2>{{ row.character }}</h2><dl><dt>{{ online }}</dt><dd>{% match row.online_at_observation %}{% when Some with (value) %}{% if value %}{{ yes }}{% else %}{{ no }}{% endif %}{% when None %}{{ unavailable }}{% endmatch %}</dd><dt>{{ observed }}</dt><dd>{{ row.observed_at_ms }}</dd><dt>{{ level }}</dt><dd>{% match row.level %}{% when Some with (n) %}{{ n }}{% when None %}{{ unavailable }}{% endmatch %}</dd><dt>{{ stamina }}</dt><dd>{% match row.stamina_minutes %}{% when Some with (n) %}{{ n }} / 2520{% when None %}{{ unavailable }}{% endmatch %}</dd><dt>{{ premium }}</dt><dd>{% match row.premium_until_ms %}{% when Some with (n) %}{{ n }}{% when None %}{{ unavailable }}{% endmatch %}</dd><dt>{{ bosses }}</dt><dd><ul>{% for boss in row.available_bosses %}<li>{{ boss }}</li>{% endfor %}</ul></dd><dt>{{ rent }}</dt><dd>{{ unavailable }}</dd><dt>{{ balance }}</dt><dd>{% match row.last_session_balance %}{% when Some with (n) %}{{ n }}{% when None %}{{ unavailable }}{% endmatch %}</dd></dl></li>{% endfor %}</ul></section>",
        ext = "html"
    )]
    struct View<'a> {
        language: &'a str,
        rows: Vec<Character>,
        title: String,
        historical: String,
        level: String,
        stamina: String,
        premium: String,
        bosses: String,
        rent: String,
        balance: String,
        unavailable: String,
        online: String,
        yes: String,
        no: String,
        observed: String,
    }
    Ok(View {
        language,
        rows,
        title: tr("title")?,
        historical: tr("historical")?,
        level: tr("level")?,
        stamina: tr("stamina")?,
        premium: tr("premium")?,
        bosses: tr("bosses")?,
        rent: tr("rent")?,
        balance: tr("balance")?,
        unavailable: tr("unavailable")?,
        online: tr("online")?,
        yes: tr("yes")?,
        no: tr("no")?,
        observed: tr("observed")?,
    }
    .render()?)
}

#[derive(Debug, Clone)]
struct HttpState {
    flags: Flags,
    source: std::sync::Arc<dyn Source>,
}
/// Host authentication must insert Actor from its validated session, never headers.
/// This surface has no mutation endpoints. Missing session integration denies reads.
pub fn router(flags: Flags, source: std::sync::Arc<dyn Source>) -> axum::Router {
    axum::Router::new()
        .route("/{language}/account/dashboard", axum::routing::get(page))
        .with_state(HttpState { flags, source })
}
/// Runtime registration deliberately has no configurable producer until T72 is available.
pub fn configured_router() -> axum::Router {
    router(Flags::default(), std::sync::Arc::new(Unavailable))
}
async fn page(
    axum::extract::State(state): axum::extract::State<HttpState>,
    axum::extract::Path(language): axum::extract::Path<String>,
    actor: Option<axum::Extension<Actor>>,
) -> axum::response::Response {
    use axum::{
        http::{StatusCode, header},
        response::{Html, IntoResponse},
    };
    let status = if !state.flags.dashboard {
        Some(StatusCode::SERVICE_UNAVAILABLE)
    } else if !matches!(language.as_str(), "pl" | "en") {
        Some(StatusCode::BAD_REQUEST)
    } else if actor.is_none() {
        Some(StatusCode::UNAUTHORIZED)
    } else if actor
        .as_ref()
        .is_some_and(|a| a.0.authorize(a.0.account).is_err())
    {
        Some(StatusCode::FORBIDDEN)
    } else {
        None
    };
    let mut response = if let Some(status) = status {
        status.into_response()
    } else if let Some(axum::Extension(actor)) = actor {
        let now = oteryn_db::outbox::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        match i64::try_from(now).ok().and_then(|now| render(state.flags, actor, actor.account, state.source.as_ref(), now, &language).ok()) {
            Some(html) => Html(format!("<!doctype html><html lang=\"{language}\"><head><meta charset=\"utf-8\"><title>Oteryn</title><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/surface.css\"></head><body><main>{html}</main></body></html>")).into_response(),
            None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response
}
