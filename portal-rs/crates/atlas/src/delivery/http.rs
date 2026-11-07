use super::*;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_world::{WorldOptions, WorldStore};
use sqlx::PgPool;

#[derive(Debug, Clone)]
struct Service {
    store: WorldStore,
    options: Options,
}
pub fn configured_router(pool: PgPool) -> Router {
    let flags = crate::config::Flags::configured();
    router(
        pool,
        Options {
            enabled: flags.delivery,
        },
        WorldOptions {
            enabled: flags.world,
        },
    )
}
pub fn router(pool: PgPool, options: Options, world: WorldOptions) -> Router {
    Router::new()
        .route("/{lang}/atlas/delivery", get(page))
        .route("/atlas/delivery.css", get(css))
        .with_state(Service {
            store: WorldStore::new(pool, world),
            options,
        })
}
async fn css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        "body{font:1rem/1.6 system-ui;background:#101820;color:#f4f6f8;margin:0}main{max-width:65rem;margin:auto;padding:1rem;overflow-wrap:anywhere}a{color:#80d4ff}label{display:block}input,select,button{font:inherit;max-width:100%;box-sizing:border-box;padding:.6rem;margin:.25rem 0;min-height:44px}nav{display:flex;gap:1rem;flex-wrap:wrap}article{border:1px solid #91a0b0;padding:1rem;margin:1rem 0}a:focus-visible,input:focus-visible,select:focus-visible,button:focus-visible{outline:3px solid #ffd66b;outline-offset:3px}",
    )
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
struct Messages(FluentBundle<FluentResource>);
impl Messages {
    fn new(lang: &str) -> Option<Self> {
        let id = lang.parse().ok()?;
        let mut bundle = FluentBundle::new_concurrent(vec![id]);
        let resource = FluentResource::try_new(
            if lang == "pl" {
                include_str!("pl.ftl")
            } else {
                include_str!("en.ftl")
            }
            .into(),
        )
        .ok()?;
        bundle.add_resource(resource).ok()?;
        Some(Self(bundle))
    }
    fn text(&self, key: &str) -> String {
        let Some(pattern) = self.0.get_message(key).and_then(|m| m.value()) else {
            return String::new();
        };
        escape(&self.0.format_pattern(pattern, None, &mut Vec::new()))
    }
}
async fn page(
    State(service): State<Service>,
    Path(lang): Path<String>,
    query: Result<Query<Filters>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if !["pl", "en"].contains(&lang.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(t) = Messages::new(&lang) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut body = format!(
        "<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"/atlas/delivery.css\"><main><h1>{}</h1>",
        t.text("title"),
        t.text("title")
    );
    let finish = |status: StatusCode, body: String| {
        (
            status,
            [(header::CACHE_CONTROL, "no-store")],
            Html(format!("{body}</main></html>")),
        )
            .into_response()
    };
    if !service.options.enabled {
        body += &format!("<p>{}</p>", t.text("unavailable"));
        return finish(StatusCode::NOT_FOUND, body);
    }
    let filters = match query {
        Ok(Query(f)) if f.validate().is_ok() => f,
        _ => {
            body += &t.text("invalid");
            return finish(StatusCode::BAD_REQUEST, body);
        }
    };
    let snapshot = match load(&service.store).await {
        Ok(s) => s,
        Err(_) => {
            body += &t.text("unavailable");
            return finish(StatusCode::SERVICE_UNAVAILABLE, body);
        }
    };
    body += &format!(
        "<p>{}: {}</p><p>{}</p><nav aria-label=\"{}\">",
        t.text("version"),
        snapshot.version,
        t.text("estimate"),
        t.text("tabs")
    );
    for (value, key) in [
        ("all", "all"),
        ("creatures", "creatures"),
        ("quests", "quests"),
    ] {
        body += &format!(
            "<a href=\"?tab={value}{}\">{}</a>",
            filters
                .item
                .as_ref()
                .map(|i| format!("&amp;item={}", encode(i)))
                .unwrap_or_default(),
            t.text(key)
        );
    }
    body += "</nav><form method=\"get\">";
    body += &format!(
        "<label for=\"item\">{}</label><input id=\"item\" name=\"item\" list=\"items\" value=\"{}\"><datalist id=\"items\">",
        t.text("item"),
        escape(filters.item.as_deref().unwrap_or_default())
    );
    let requirements = snapshot.requirements();
    let quest_items: BTreeSet<_> = requirements
        .iter()
        .filter_map(|r| r.item.as_deref())
        .collect();
    let creature_items = filters
        .creature
        .as_ref()
        .map(|c| snapshot.creature_items(c));
    for item in snapshot.items() {
        if filters.tab.as_deref() == Some("quests") && !quest_items.contains(item.as_str()) {
            continue;
        }
        if creature_items
            .as_ref()
            .is_some_and(|items| !items.contains(&item))
        {
            continue;
        }
        body += &format!("<option value=\"{}\"></option>", escape(&item));
    }
    body += "</datalist>";
    if let Some(tab) = &filters.tab {
        body += &format!(
            "<input type=\"hidden\" name=\"tab\" value=\"{}\">",
            escape(tab)
        );
    }
    for (name, key, value, kind) in [
        ("q", "search", filters.q.clone().unwrap_or_default(), "text"),
        (
            "creature",
            "creatures",
            filters.creature.clone().unwrap_or_default(),
            "text",
        ),
        (
            "count",
            "count",
            filters.count.unwrap_or(1).to_string(),
            "number",
        ),
        (
            "level",
            "level",
            filters.level.map(|v| v.to_string()).unwrap_or_default(),
            "number",
        ),
        (
            "floor",
            "floor",
            filters.floor.map(|v| v.to_string()).unwrap_or_default(),
            "number",
        ),
        (
            "city",
            "city",
            filters.city.clone().unwrap_or_default(),
            "text",
        ),
        (
            "rarity",
            "rarity",
            filters.rarity.map(|v| v.to_string()).unwrap_or_default(),
            "number",
        ),
    ] {
        body += &format!(
            "<label for=\"{name}\">{}</label><input id=\"{name}\" name=\"{name}\" type=\"{kind}\" value=\"{}\">",
            t.text(key),
            escape(&value)
        );
    }
    body += &format!("<button>{}</button></form>", t.text("filter"));
    if filters.tab.as_deref() != Some("creatures") {
        body += &format!("<h2>{}</h2><ul>", t.text("quests"));
        for r in &requirements {
            body += "<li>";
            body += &escape(&r.quest);
            body += " / ";
            body += &escape(&r.step);
            match &r.item {
                Some(item) => {
                    body += &format!(
                        " <a href=\"?item={}{}\">{}</a>",
                        encode(item),
                        r.count
                            .map(|n| format!("&amp;count={n}"))
                            .unwrap_or_default(),
                        escape(item)
                    )
                }
                None => body += &format!(" — {}", t.text("unresolved")),
            };
            body += "</li>";
        }
        body += "</ul>";
    }
    if filters.item.is_some() {
        let data = match snapshot.discover(service.options, &filters) {
            Ok(d) => d,
            Err(_) => {
                body += &t.text("unavailable");
                return finish(StatusCode::NOT_FOUND, body);
            }
        };
        if data.sources.is_empty() {
            body += &format!("<p>{}</p>", t.text("empty"));
        }
        for s in data.sources {
            body += &format!(
                "<article><h2><a href=\"/{lang}/wiki/creatures/{}\">{}</a></h2><p>{}: {} / 1000000; {}: {}–{}; {}: {}</p>",
                encode(&s.creature),
                escape(&s.creature),
                t.text("chance"),
                s.probability_ppm,
                t.text("quantity"),
                s.min_count,
                s.max_count,
                t.text("tier"),
                s.tier
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| t.text("unavailable"))
            );
            if let Some(e) = s.estimate {
                body += &format!(
                    "<p>{}: {:.2}; P50: {}; P80: {}; P95: {}</p>",
                    t.text("kills"),
                    e.expected_kills,
                    e.p50_kills,
                    e.p80_kills,
                    e.p95_kills
                );
            } else {
                body += &format!("<p>{}: {}</p>", t.text("kills"), t.text("unavailable"));
            }
            if s.spawns.is_empty() {
                body += &format!("<p>{}: {}</p>", t.text("map"), t.text("unavailable"));
            }
            body += "<ul>";
            for p in s.spawns {
                body += &format!(
                    "<li><a href=\"/{lang}/atlas/{}/{}/{}\">{} ({}, {}, {})</a></li>",
                    p.floor,
                    p.x,
                    p.y,
                    t.text("map"),
                    p.x,
                    p.y,
                    p.floor
                );
            }
            body += "</ul></article>";
        }
    }
    finish(StatusCode::OK, body)
}
