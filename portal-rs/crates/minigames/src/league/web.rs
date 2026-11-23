//! Public league results and private, stake-free predictions.
use super::{predictions::Overview, *};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Request, State, rejection::FormRejection},
    http::{HeaderValue, StatusCode, header},
    middleware::{Next, from_fn, from_fn_with_state},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::{
    csrf::CsrfConfig,
    ratelimit::{self, ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionLayer, SignedInSession},
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;

// The portal pool has 16 connections. Leave capacity for other routes.
const MAX_CONCURRENT_READS: usize = 4;

#[derive(Template)]
#[template(
    path = "minigames/league.html",
    config = "../../templates/minigames/askama.toml"
)]
struct Page {
    lang: String,
    title: String,
    message: String,
    outcomes: String,
    standings: String,
    world: String,
    result: String,
    rank: String,
    guild: String,
    points: String,
    lock_at: String,
    choice_label: String,
    score_label: String,
    choose: String,
    submit: String,
    sign_in_label: String,
    back: String,
    retry: bool,
    csrf: String,
    events: Vec<EventRow>,
    seasons: Vec<SeasonRow>,
}
struct EventRow {
    subject: String,
    source: String,
    world: String,
    state: String,
    deadline: Option<String>,
    result: Option<String>,
    choice: Option<String>,
    score: Option<u8>,
    form: Option<PredictionForm>,
    sign_in: bool,
}
struct PredictionForm {
    epoch: String,
    generation: u64,
    revision: String,
    options: Vec<String>,
}
struct SeasonRow {
    subject: String,
    world: String,
    state: String,
    guilds: Vec<schema::Guild>,
}
fn text(lang: &str, id: &str) -> String {
    let source = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
    };
    let Ok(locale) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    let Ok(resource) = FluentResource::try_new(source.into()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(value) = bundle.get_message(id).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle.format_pattern(value, None, &mut vec![]).into_owned()
}
impl Page {
    fn new(lang: &str, overview: Option<Overview>, session: Option<SignedInSession>) -> Self {
        let mut p = Self {
            lang: lang.into(),
            title: oteryn_ui::text(lang, "shell-league"),
            message: text(lang, "unavailable"),
            outcomes: text(lang, "outcomes"),
            standings: text(lang, "standings"),
            world: text(lang, "world"),
            result: text(lang, "result"),
            rank: text(lang, "rank"),
            guild: text(lang, "guild"),
            points: text(lang, "points"),
            lock_at: text(lang, "lock-at"),
            choice_label: text(lang, "choice"),
            score_label: text(lang, "score"),
            choose: text(lang, "choose"),
            submit: text(lang, "submit"),
            sign_in_label: text(lang, "sign-in"),
            back: text(lang, "back"),
            retry: false,
            csrf: String::new(),
            events: vec![],
            seasons: vec![],
        };
        let Some(overview) = overview else {
            return p;
        };
        let signed_in = session.is_some();
        p.csrf = session.map(|session| session.csrf).unwrap_or_default();
        p.events.extend(
            overview
                .active
                .iter()
                .filter_map(|s| Self::event(lang, &overview, s, signed_in)),
        );
        for s in &overview.decisions {
            match s {
                schema::Snapshot::Event(s) => {
                    p.events.extend(Self::event(lang, &overview, s, signed_in));
                }
                schema::Snapshot::Season(s) => {
                    for e in &s.season {
                        p.seasons.push(SeasonRow {
                            subject: s.season_id.clone(),
                            world: s.world_id.clone(),
                            state: text(lang, &e.state.to_lowercase()),
                            guilds: if e.state == "VOID" {
                                vec![]
                            } else {
                                e.standings.clone()
                            },
                        });
                    }
                }
            }
        }
        p.message = text(
            lang,
            if p.events.is_empty() && p.seasons.is_empty() {
                "empty"
            } else {
                "pending"
            },
        );
        p
    }

    fn event(
        lang: &str,
        overview: &Overview,
        snapshot: &schema::EventSnapshot,
        signed_in: bool,
    ) -> Option<EventRow> {
        let record = snapshot.event.first()?;
        let key = (
            snapshot.source_authority.clone(),
            schema::id(&snapshot.world_id).ok()?,
            schema::id(&snapshot.event_id).ok()?,
        );
        let choice = overview.choices.get(&key);
        let open = predictions::open(record, overview.now);
        let deadline = schema::number(&record.lock_at, false)
            .ok()
            .and_then(|seconds| i64::try_from(seconds).ok())
            .and_then(|seconds| time::OffsetDateTime::from_unix_timestamp(seconds).ok())
            .and_then(|time| {
                time.format(&time::format_description::well_known::Rfc3339)
                    .ok()
            });
        let phase = record.state.to_lowercase();
        let active = matches!(record.state.as_str(), "SCHEDULED" | "LOCKED");
        Some(EventRow {
            subject: snapshot.event_id.clone(),
            source: record.source_key.clone(),
            world: snapshot.world_id.clone(),
            state: text(lang, if active && !open { "locked" } else { &phase }),
            deadline: deadline.filter(|_| active),
            result: (!active).then(|| {
                record
                    .outcome
                    .first()
                    .cloned()
                    .unwrap_or_else(|| text(lang, "void"))
            }),
            choice: choice.cloned(),
            score: choice.and_then(|choice| predictions::final_score(record, choice)),
            form: (open && overview.eligible && choice.is_none()).then(|| PredictionForm {
                epoch: snapshot.projection_epoch.clone(),
                generation: overview.generation,
                revision: snapshot.projection_revision.clone(),
                options: record.options.clone(),
            }),
            sign_in: open && !signed_in,
        })
    }

    fn response(self, status: StatusCode) -> Response {
        match self.render() {
            Ok(body) => (status, Html(body)).into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}

struct Surface<A> {
    league: League<A>,
    read_slots: Semaphore,
    clock: fn() -> i64,
    sample: bool,
}
pub fn router<A: Authority + 'static>(league: League<A>) -> Router {
    router_with_clock(league, || time::OffsetDateTime::now_utc().unix_timestamp())
}

pub(super) fn router_with_clock<A: Authority + 'static>(
    league: League<A>,
    clock: fn() -> i64,
) -> Router {
    public_router(league, clock, false)
}

fn public_router<A: Authority + 'static>(
    league: League<A>,
    clock: fn() -> i64,
    sample: bool,
) -> Router {
    if !league.flags.enabled || !league.flags.public {
        return Router::new();
    }
    let limiter = RateLimiter::new(league.pool.clone());
    let mut sessions = SessionLayer::new(league.pool.clone());
    sessions.csrf = CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..CsrfConfig::default()
    };
    Router::new()
        .route(
            "/{lang}/league",
            get(page::<A>).layer(from_fn_with_state(
                sessions.clone(),
                session::read_only_middleware,
            )),
        )
        .merge(
            Router::new()
                .route("/{lang}/league/prediction", post(submit::<A>))
                .layer(from_fn_with_state(sessions, session::middleware))
                .layer(DefaultBodyLimit::max(4096))
                .layer(tower_http::limit::RequestBodyLimitLayer::new(4096)),
        )
        .layer(from_fn_with_state(limiter, ratelimit::supply))
        .layer(from_fn(response_policy))
        .with_state(Arc::new(Surface {
            league,
            read_slots: Semaphore::new(MAX_CONCURRENT_READS),
            clock,
            sample,
        }))
}
pub fn configured_router(pool: PgPool) -> Router {
    let flags = super::config::owner_flags(super::config::enabled());
    let authority = super::runtime::GameAuthority::configured();
    public_router(
        League::new(pool, flags, authority),
        || time::OffsetDateTime::now_utc().unix_timestamp(),
        crate::staging_samples(),
    )
}

async fn response_policy(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn limit(
    limiter: &RateLimiter,
    source: std::result::Result<Option<ClientSource>, StatusCode>,
) -> std::result::Result<(), StatusCode> {
    // Portal listeners supply ConnectInfo. Direct in-process callers may omit it.
    if let Some(ClientSource(ip)) = source? {
        let keys = [Keys::source(ip)];
        match tokio::time::timeout(
            Duration::from_secs(1),
            limiter.check(NamedPolicy::PublicGameData, &keys),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(denial)) => return Err(denial.status()),
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }
    Ok(())
}

async fn page<A: Authority>(
    Path(lang): Path<String>,
    State(state): State<Arc<Surface<A>>>,
    Extension(limiter): Extension<RateLimiter>,
    source: std::result::Result<Option<ClientSource>, StatusCode>,
    signed_in: Option<Extension<SignedInSession>>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(status) = limit(&limiter, source).await {
        return Page::new(&lang, None, None).response(status);
    }
    let Ok(now) = u64::try_from((state.clock)()) else {
        return Page::new(&lang, None, None).response(StatusCode::SERVICE_UNAVAILABLE);
    };
    let Ok(_slot) = state.read_slots.try_acquire() else {
        return Page::new(&lang, None, None).response(StatusCode::SERVICE_UNAVAILABLE);
    };
    let session = signed_in.map(|Extension(value)| value);
    let mut overview = state
        .league
        .overview(session.as_ref().map(|s| s.account_id))
        .await;
    if state.sample
        && let Ok(samples) = read_samples(&state.league.pool, now).await
    {
        match &mut overview {
            Ok(overview) => overview.decisions.extend(samples),
            Err(_) => {
                overview = Ok(Overview {
                    decisions: samples,
                    active: Vec::new(),
                    choices: Default::default(),
                    eligible: false,
                    now,
                    generation: 0,
                });
            }
        }
    }
    match overview {
        Ok(overview) => {
            let mut page = Page::new(&lang, Some(overview), session);
            if state.sample {
                page.title = text(&lang, "title");
            }
            page.response(StatusCode::OK)
        }
        Err(_) => Page::new(&lang, None, None).response(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn submit<A: Authority>(
    Path(lang): Path<String>,
    State(state): State<Arc<Surface<A>>>,
    Extension(limiter): Extension<RateLimiter>,
    source: std::result::Result<Option<ClientSource>, StatusCode>,
    Extension(session): Extension<BrowserSession>,
    form: std::result::Result<Form<predictions::Form>, FormRejection>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(status) = limit(&limiter, source).await {
        return Page::new(&lang, None, None).response(status);
    }
    let result = match (session.account_id().await, form) {
        (None, _) => Err(Error::Denied),
        (_, Err(_)) => Err(Error::Invalid),
        (Some(account), Ok(Form(form))) => state.league.predict(account, form).await,
    };
    let (status, message) = match result {
        Ok(()) => return Redirect::to(&format!("/{lang}/league")).into_response(),
        Err(Error::Conflict) => (StatusCode::CONFLICT, "refused"),
        Err(Error::Denied) => (StatusCode::FORBIDDEN, "denied"),
        Err(Error::Invalid) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    let mut page = Page::new(&lang, None, None);
    page.message = text(&lang, message);
    page.retry = true;
    page.response(status)
}
// Sample results do not attest to a live Game feed. They cannot use its
// recovery witness or verified audit path.
async fn read_samples(pool: &PgPool, now: u64) -> Result<Vec<schema::Snapshot>> {
    let rows: Vec<Vec<u8>> = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query_scalar(
            "SELECT wire FROM league_subjects WHERE authority='staging-sample'
         AND family='guild_season' AND final_state AND NOT quarantined
         ORDER BY world_id, subject_id LIMIT 16",
        )
        .fetch_all(pool),
    )
    .await
    .map_err(|_| Error::Unavailable)??;
    rows.into_iter()
        .map(|wire| {
            let season: schema::SeasonSnapshot =
                serde_json::from_slice(&wire).map_err(|_| Error::Invalid)?;
            let canonical = serde_json::to_vec(&season).map_err(|_| Error::Invalid)?;
            schema::Snapshot::decode(&canonical, true, now)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overview(decisions: Vec<schema::Snapshot>) -> Overview {
        Overview {
            decisions,
            active: vec![],
            choices: Default::default(),
            eligible: false,
            now: 1790000200,
            generation: 1,
        }
    }

    #[test]
    fn void_season_never_claims_a_ranking() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let fixtures: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/snapshots.json"))?;
        let raw = fixtures["vectors"][5]["wire"]
            .as_str()
            .ok_or("season fixture missing")?;
        let mut snapshot = schema::Snapshot::decode(raw.as_bytes(), true, 1790000200)?;
        if let schema::Snapshot::Season(s) = &mut snapshot {
            s.season[0].state = "VOID".into();
        }
        for lang in ["pl", "en"] {
            let html = Page::new(lang, Some(overview(vec![snapshot.clone()])), None).render()?;
            assert!(html.contains(&format!("<h2>{}</h2>", text(lang, "standings"))));
            assert!(!html.contains("Aldric Guild"));
            assert!(!html.contains("Second Guild"));
        }
        Ok(())
    }
    #[test]
    fn fresh_empty_feed_has_an_empty_state() {
        assert_eq!(
            Page::new("en", Some(overview(vec![])), None).message,
            "No world events or guild standings have been published."
        );
    }
    #[tokio::test]
    async fn prediction_route_is_disabled_by_default()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use axum::{body::Body, http::Request};
        use sqlx::postgres::PgPoolOptions;
        use tower::ServiceExt;
        let pool = PgPoolOptions::new().connect_lazy_with(sqlx::postgres::PgConnectOptions::new());
        let app = router(League::new(pool, Flags::default(), Unavailable));
        let response = app
            .oneshot(Request::post("/en/league/prediction").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    fn responsive_localized_pages() -> std::result::Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            let p = Page::new(lang, None, None);
            assert!(!p.title.is_empty());
            assert!(!p.message.is_empty());
            let html = p.render()?;
            assert!(html.contains("width=device-width"));
            assert!(html.contains("min(100%,22rem)"));
            assert!(!html.contains("<form"));
            assert!(!html.contains("<section"));
            assert!(!html.contains("<h2"));
        }
        Ok(())
    }
}
