//! Read-only public surface; no prediction mutation exists.
use super::*;
use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><style>body{margin:auto;padding:1rem;max-width:90rem;font:1rem/1.5 system-ui;color:#eee;background:#151b25}article{padding:1rem;border:1px solid #708090;border-radius:.5rem;overflow-wrap:anywhere}section{display:grid;gap:1rem;grid-template-columns:repeat(auto-fit,minmax(min(100%,22rem),1fr))}table{width:100%;border-collapse:collapse}td,th{text-align:left;padding:.3rem}h1{font-size:clamp(1.5rem,4vw,2.5rem)}</style></head><body><main><h1>{{ title }}</h1><p role=\"status\">{{ message }}</p><h2>{{ outcomes }}</h2><section>{% for row in events %}<article><h3>{{ row.subject }}</h3><p>{{ world }}: {{ row.world }}</p><p>{{ row.state }}</p><p>{{ result }}: {{ row.result }}</p></article>{% endfor %}</section><h2>{{ standings }}</h2><section>{% for row in seasons %}<article><h3>{{ row.subject }}</h3><p>{{ world }}: {{ row.world }}</p><p>{{ row.state }}</p><table><thead><tr><th>{{ rank }}</th><th>{{ guild }}</th><th>{{ points }}</th></tr></thead><tbody>{% for g in row.guilds %}<tr><td>{{ g.rank }}</td><td>{{ g.guild_name }}</td><td>{{ g.total_score }}</td></tr>{% endfor %}</tbody></table></article>{% endfor %}</section></main></body></html>",
    ext = "html"
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
    events: Vec<EventRow>,
    seasons: Vec<SeasonRow>,
}
struct EventRow {
    subject: String,
    world: String,
    state: String,
    result: String,
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
    fn new(lang: &str, snapshots: Option<Vec<schema::Snapshot>>) -> Self {
        let mut p = Self {
            lang: lang.into(),
            title: text(lang, "title"),
            message: text(
                lang,
                if snapshots.is_some() {
                    "pending"
                } else {
                    "unavailable"
                },
            ),
            outcomes: text(lang, "outcomes"),
            standings: text(lang, "standings"),
            world: text(lang, "world"),
            result: text(lang, "result"),
            rank: text(lang, "rank"),
            guild: text(lang, "guild"),
            points: text(lang, "points"),
            events: vec![],
            seasons: vec![],
        };
        for s in snapshots.unwrap_or_default() {
            match s {
                schema::Snapshot::Event(s) => {
                    for e in s.event {
                        p.events.push(EventRow {
                            subject: s.event_id.clone(),
                            world: s.world_id.clone(),
                            state: text(lang, &e.state.to_lowercase()),
                            result: e
                                .outcome
                                .first()
                                .cloned()
                                .unwrap_or_else(|| text(lang, "void")),
                        });
                    }
                }
                schema::Snapshot::Season(s) => {
                    for e in s.season {
                        p.seasons.push(SeasonRow {
                            subject: s.season_id.clone(),
                            world: s.world_id.clone(),
                            state: text(lang, &e.state.to_lowercase()),
                            guilds: if e.state == "VOID" {
                                vec![]
                            } else {
                                e.standings
                            },
                        });
                    }
                }
            }
        }
        p
    }
    fn response(self, status: StatusCode) -> Response {
        match self.render() {
            Ok(body) => (
                status,
                [
                    (header::CACHE_CONTROL, "no-store"),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                ],
                Html(body),
            )
                .into_response(),
            Err(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::CACHE_CONTROL, "no-store")],
            )
                .into_response(),
        }
    }
}
struct Surface<A> {
    league: League<A>,
    rate: Mutex<(i64, u32)>,
    clock: fn() -> i64,
}
pub fn router<A: Authority + 'static>(league: League<A>) -> Router {
    router_with_clock(league, || time::OffsetDateTime::now_utc().unix_timestamp())
}
pub(super) fn router_with_clock<A: Authority + 'static>(
    league: League<A>,
    clock: fn() -> i64,
) -> Router {
    if !league.flags.enabled || !league.flags.public {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/league", get(page::<A>))
        .with_state(Arc::new(Surface {
            league,
            rate: Mutex::new((0, 0)),
            clock,
        }))
}
pub fn configured_router(pool: PgPool) -> Router {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    let flags = Flags {
        enabled: on("OTERYN_LEAGUE_ENABLED"),
        public: on("OTERYN_LEAGUE_PUBLIC_ENABLED"),
        ..Flags::default()
    };
    router(League::new(pool, flags, Unavailable))
}
async fn page<A: Authority>(
    Path(lang): Path<String>,
    State(state): State<Arc<Surface<A>>>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let now = (state.clock)();
    let allowed = state
        .rate
        .lock()
        .map(|mut r| {
            if now < r.0 || now - r.0 >= 60 {
                *r = (now, 0);
            }
            r.1 = r.1.saturating_add(1);
            r.1 <= 120
        })
        .unwrap_or(false);
    if !allowed {
        return Page::new(&lang, None).response(StatusCode::TOO_MANY_REQUESTS);
    }
    let Ok(now) = u64::try_from(now) else {
        return Page::new(&lang, None).response(StatusCode::SERVICE_UNAVAILABLE);
    };
    match state.league.read(now).await {
        Ok(s) => Page::new(&lang, Some(s)).response(StatusCode::OK),
        Err(_) => Page::new(&lang, None).response(StatusCode::SERVICE_UNAVAILABLE),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
            let html = Page::new(lang, Some(vec![snapshot.clone()])).render()?;
            assert!(!html.contains("Aldric Guild"));
            assert!(!html.contains("Second Guild"));
        }
        Ok(())
    }
    #[test]
    fn responsive_localized_pages() -> std::result::Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            let p = Page::new(lang, None);
            assert!(!p.title.is_empty());
            assert!(!p.message.is_empty());
            let html = p.render()?;
            assert!(html.contains("width=device-width"));
            assert!(html.contains("min(100%,22rem)"));
            assert!(!html.contains("<form"));
        }
        Ok(())
    }
}
