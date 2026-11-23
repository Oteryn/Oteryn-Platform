use super::{Config, Day, Error, read};
use crate::{
    online::{self, CoveragePolicy, WorldTotal},
    store::Visibility,
};
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use std::{fmt, sync::Arc};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// An unavailable privacy service denies the entire representation.
pub trait Policy: Visibility + Send + Sync {
    fn available(&self) -> bool;
}
impl Policy for crate::privacy::Authority {
    fn available(&self) -> bool {
        self.enabled() && crate::privacy::GAME_STATISTICS_ACCEPTED
    }
}
#[derive(Clone)]
struct World {
    db: PgPool,
    config: Config,
    policy: Arc<dyn Policy>,
    kills: Option<KillFeed>,
}

/// Accepted feed settings paired with independently qualified retained-set coverage.
/// A live watermark alone cannot supply this coverage.
#[derive(Clone)]
pub struct KillFeed {
    pub config: online::Config,
    pub coverage: Arc<dyn CoveragePolicy>,
}

impl fmt::Debug for KillFeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("KillFeed").finish_non_exhaustive()
    }
}

pub fn router<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    config: Config,
    policy: Arc<dyn Policy>,
) -> Router<S> {
    router_with_kills(db, config, policy, None)
}

pub fn router_with_kills<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    config: Config,
    policy: Arc<dyn Policy>,
    kills: Option<KillFeed>,
) -> Router<S> {
    Router::new()
        .route("/stats/world", get(page))
        .route("/pl/stats/world", get(page))
        .route("/en/stats/world", get(page))
        .with_state(World {
            db,
            config,
            policy,
            kills,
        })
}
/// G-8 revision 2 is proposed. Service configuration supplies no accepted
/// world-kill coverage, even when the testing ingest switch is set.
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    router(
        db.clone(),
        Config {
            enabled: std::env::var("PORTAL_WORLD_STATS_ENABLED").as_deref() == Ok("true"),
        },
        Arc::new(crate::privacy::Authority::configured(db)),
    )
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Uuid,
    start: String,
    end: String,
}
struct KillDay {
    date: Date,
    total: Option<WorldTotal>,
}

#[derive(Template)]
#[template(path = "stats/world/index.html", config = "src/world.askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    observations_available: bool,
    days: Vec<Day>,
    kills: Vec<KillDay>,
    selected: bool,
    worlds: Vec<(Uuid, String, bool)>,
    start: String,
    end: String,
    today: String,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../../../templates/stats/world/en.ftl")
    } else {
        include_str!("../../../../templates/stats/world/pl.ftl")
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    [
        "title",
        "unavailable",
        "census",
        "deaths",
        "online",
        "note",
        "level",
        "day",
        "count",
        "kills",
        "picker",
        "select",
        "start",
        "end",
        "show",
        "monsters",
        "bosses",
        "complete",
        "provisional",
    ]
    .iter()
    .map(|key| {
        bundle
            .get_message(&format!("world-{key}"))
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
fn date(value: &str) -> Result<Date, Error> {
    let parts = value.split('-').collect::<Vec<_>>();
    if !value.bytes().all(|b| b.is_ascii_digit() || b == b'-')
        || parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
    {
        return Err(Error::BadInput);
    }
    let year = parts[0].parse().map_err(|_| Error::BadInput)?;
    let month = parts[1]
        .parse::<u8>()
        .map_err(|_| Error::BadInput)?
        .try_into()
        .map_err(|_| Error::BadInput)?;
    let day = parts[2].parse().map_err(|_| Error::BadInput)?;
    Date::from_calendar_date(year, month, day).map_err(|_| Error::BadInput)
}
async fn page(
    State(state): State<World>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let response = async {
        if !state.config.enabled {
            return StatusCode::NOT_FOUND.into_response();
        }
        let now = OffsetDateTime::now_utc();
        let today = now.date();
        let selection = match raw
            .as_deref()
            .filter(|raw| !raw.is_empty())
            .map(|raw| parse(raw, today))
            .transpose()
        {
            Ok(selection) => selection,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        let worlds = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, name FROM world_status ORDER BY name COLLATE \"C\", id LIMIT 1001",
        )
        .fetch_all(&state.db)
        .await
        .ok()
        .filter(|worlds| worlds.len() <= 1000);
        if let (Some((world, _, _)), Some(worlds)) = (&selection, &worlds)
            && !worlds.iter().any(|(registered, _)| registered == world)
        {
            return StatusCode::BAD_REQUEST.into_response();
        }

        let en = uri.path().starts_with("/en/");
        let mut page = Page {
            lang: if en { "en" } else { "pl" },
            labels: labels(en),
            observations_available: false,
            days: Vec::new(),
            kills: Vec::new(),
            selected: selection.is_some(),
            worlds: worlds
                .unwrap_or_default()
                .into_iter()
                .map(|(id, name)| (id, name, selection.is_some_and(|(world, _, _)| id == world)))
                .collect(),
            start: today.to_string(),
            end: today.to_string(),
            today: today.to_string(),
        };
        if let Some((world, start, end)) = selection {
            page.start = start.to_string();
            page.end = end.to_string();
            if !page.worlds.is_empty() {
                if state.policy.available()
                    && let Ok(report) = read(
                        &state.db,
                        state.config,
                        world,
                        start,
                        end,
                        &PolicyRef(state.policy.as_ref()),
                    )
                    .await
                {
                    page.observations_available = true;
                    page.days = report.days;
                }
                if let Some(feed) = &state.kills {
                    page.kills = daily_kills(
                        &state.db,
                        feed,
                        world,
                        start,
                        end,
                        OffsetDateTime::now_utc(),
                    )
                    .await;
                }
            }
        }
        let status = if page.worlds.is_empty()
            || (page.selected && !page.observations_available && page.kills.is_empty())
        {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        };
        match page.render() {
            Ok(body) => (status, Html(body)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
    .await;
    let mut response = response;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    response
}
fn parse(raw: &str, today: Date) -> Result<(Uuid, Date, Date), Error> {
    let query: Query = serde_urlencoded::from_str(raw).map_err(|_| Error::BadInput)?;
    let (start, end) = (date(&query.start)?, date(&query.end)?);
    if query.world.get_version_num() != 7
        || query.world.get_variant() != uuid::Variant::RFC4122
        || start > end
        || end > today
        || (end - start).whole_days() > 30
    {
        return Err(Error::BadInput);
    }
    Ok((query.world, start, end))
}

async fn daily_kills(
    db: &PgPool,
    feed: &KillFeed,
    world: Uuid,
    start: Date,
    end: Date,
    now: OffsetDateTime,
) -> Vec<KillDay> {
    let dates: Vec<_> = std::iter::successors(Some(start), |date| date.next_day())
        .take_while(|date| *date <= end)
        .collect();
    let days: Vec<_> = dates
        .iter()
        .map(|date| date.midnight().assume_utc().unix_timestamp())
        .collect();
    let Ok(mut totals) = online::world_totals(
        db,
        &feed.config,
        world,
        &days,
        now.unix_timestamp(),
        feed.coverage.as_ref(),
    )
    .await
    else {
        return Vec::new();
    };
    if totals.is_empty() {
        return Vec::new();
    }
    dates
        .into_iter()
        .zip(days)
        .map(|(date, day)| KillDay {
            date,
            total: totals.remove(&day),
        })
        .collect()
}

struct PolicyRef<'a>(&'a dyn Policy);
impl Visibility for PolicyRef<'_> {
    fn current_permissions(&self, id: Uuid) -> crate::store::PermissionFuture<'_> {
        Box::pin(async move { self.0.current_permissions(id).await })
    }
}
