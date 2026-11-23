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
    static_dir: std::path::PathBuf,
    options: Options,
    computation: std::sync::Arc<tokio::sync::Semaphore>,
}
pub fn configured_router(pool: PgPool) -> Router {
    let flags = crate::config::Flags::configured();
    with_static(
        pool,
        Options {
            enabled: flags.delivery,
        },
        WorldOptions {
            enabled: flags.world,
        },
        std::env::var_os("OTERYN_STATIC_DIR").map_or_else(|| "static".into(), Into::into),
    )
}
pub fn router(pool: PgPool, options: Options, world: WorldOptions) -> Router {
    with_static(pool, options, world, "static".into())
}
fn with_static(
    pool: PgPool,
    options: Options,
    world: WorldOptions,
    static_dir: std::path::PathBuf,
) -> Router {
    let mut app = Router::new().route("/{lang}/atlas/delivery", get(page));
    if options.enabled {
        app = app.route("/atlas/delivery.css", get(css));
    }
    app.with_state(Service {
        store: WorldStore::new(pool, world),
        static_dir,
        options,
        computation: std::sync::Arc::new(tokio::sync::Semaphore::new(2)),
    })
}
async fn css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        "body{font:1rem/1.6 system-ui;background:#101820;color:#f4f6f8;margin:0}img{max-width:100%;height:auto}main{max-width:65rem;margin:auto;padding:1rem;overflow-wrap:anywhere}a{color:#80d4ff}label{display:block}input,select,button{font:inherit;max-width:100%;box-sizing:border-box;padding:.6rem;margin:.25rem 0;min-height:44px}nav{display:flex;gap:1rem;flex-wrap:wrap}article{border:1px solid #91a0b0;padding:1rem;margin:1rem 0}a:focus-visible,input:focus-visible,select:focus-visible,button:focus-visible{outline:3px solid #ffd66b;outline-offset:3px}",
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
    let title = escape(&oteryn_ui::text(&lang, "shell-delivery"));
    let mut body = format!(
        "<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title><link rel=\"stylesheet\" href=\"/atlas/delivery.css\"><main><h1>{title}</h1>"
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
    let permit = match service.computation.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return finish(StatusCode::SERVICE_UNAVAILABLE, body),
    };
    let snapshot = match load(&service.store).await {
        Ok(s) => s,
        Err(_) => {
            body += &format!("<p>{}</p>", t.text("unavailable"));
            return finish(StatusCode::SERVICE_UNAVAILABLE, body);
        }
    };
    if filters
        .version
        .is_some_and(|version| version != snapshot.version)
    {
        body += &format!("<p>{}</p>", t.text("version-missing"));
        return finish(StatusCode::NOT_FOUND, body);
    }
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
            "<a href=\"?tab={value}&amp;version={}{}\">{}</a>",
            snapshot.version,
            filters
                .item
                .as_ref()
                .map(|i| format!("&amp;item={}", encode(i)))
                .unwrap_or_default(),
            t.text(key)
        );
    }
    body += &format!(
        "</nav><form method=\"get\"><input type=\"hidden\" name=\"version\" value=\"{}\">",
        snapshot.version
    );
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
            filters.count.map(|v| v.to_string()).unwrap_or_default(),
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
            if r.reward {
                body += &format!(" · {}", t.text("reward"));
            }
            match &r.item {
                Some(item) => {
                    body += &format!(
                        " <a href=\"?item={}&amp;version={}{}\">{}</a>",
                        encode(item),
                        snapshot.version,
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
    if let Some(item) = filters.item.as_deref() {
        let sprite = if let Some(mapping) = snapshot
            .get(sprite::KIND, item)
            .and_then(|data| sprite::Mapping::deserialize(data).ok())
            && snapshot.get("Item", item).is_some()
            && let Ok(Some(report)) = service.store.import_report(snapshot.version).await
        {
            sprite::url(&service.static_dir, &report.source_commit, &mapping).await
        } else {
            None
        };
        let creature_only =
            filters.tab.as_deref() == Some("creatures") || filters.creature.is_some();
        let options = service.options;
        let calculation = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            snapshot.discover(options, &filters)
        })
        .await;
        let data = match calculation {
            Ok(Ok(d)) => d,
            Ok(Err(_)) | Err(_) => {
                body += &t.text("unavailable");
                return finish(StatusCode::NOT_FOUND, body);
            }
        };
        body += &format!("<h2>{}</h2>", escape(&data.item));
        if let Some(url) = sprite {
            body += &format!(
                "<p><img src=\"{}\" alt=\"{}\"></p>",
                escape(&url),
                t.text("sprite")
            );
        } else {
            body += &format!("<p>{}: {}</p>", t.text("sprite"), t.text("unavailable"));
        }
        if !creature_only {
            body += &format!("<section><h2>{}</h2>", t.text("shops"));
            if data.shops.is_empty() {
                body += &format!("<p>{}</p>", t.text("unavailable"));
            }
            for offer in data.shops {
                let direction = match offer.direction {
                    shops::Direction::NpcSellOffer => "npc-sells",
                    shops::Direction::NpcBuyOffer => "npc-buys",
                };
                body += &format!(
                    "<article><h3>{}: {}</h3><p>{}: {} {}; {}: {}</p></article>",
                    t.text(direction),
                    escape(&offer.npc),
                    t.text("unit-price"),
                    offer.unit_price,
                    escape(&offer.currency),
                    t.text("subtype"),
                    offer.item_subtype
                );
            }
            body += "</section>";
        }
        body += &format!("<h2>{}</h2>", t.text("creatures"));
        if data.sources.is_empty() {
            body += &format!("<p>{}</p>", t.text("unavailable"));
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
                    "<li><a href=\"/{lang}/atlas?x={}&amp;y={}&amp;z={}&amp;version={}\">{} ({}, {}, {})</a></li>",
                    p.x,
                    p.y,
                    p.floor,
                    data.version,
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

#[cfg(test)]
mod computation_tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test(flavor = "current_thread")]
    async fn delivery_admission_is_bounded_and_heartbeat_remains_responsive()
    -> Result<(), Box<dyn std::error::Error>> {
        let computation = std::sync::Arc::new(tokio::sync::Semaphore::new(2));
        let pool = oteryn_db::lazy_pool("postgres://localhost/unreachable")?;
        let service = Service {
            store: WorldStore::new(pool, WorldOptions { enabled: true }),
            options: Options { enabled: true },
            computation: computation.clone(),
            static_dir: "static".into(),
        };
        let app = Router::new()
            .route("/{lang}/atlas/delivery", get(page))
            .route("/heartbeat", get(|| async { StatusCode::OK }))
            .with_state(service);
        let first = computation.clone().try_acquire_owned()?;
        let second = computation.clone().try_acquire_owned()?;
        let busy = app
            .clone()
            .oneshot(
                Request::get("/en/atlas/delivery?item=item%3Aore&count=100000")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(busy.status(), StatusCode::SERVICE_UNAVAILABLE);
        drop(first);
        drop(second);
        let mut snapshot = super::super::tests::snapshot();
        for i in 0..1000 {
            snapshot.entities.insert(
                ("Creature".into(), format!("source-{i}")),
                serde_json::json!({"loot":"loot:rat","level":1}),
            );
        }
        let permit = computation.clone().try_acquire_owned()?;
        let calculation = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            snapshot.discover(
                Options { enabled: true },
                &Filters {
                    item: Some("item:ore".into()),
                    count: Some(100_000),
                    ..Default::default()
                },
            )
        });
        let heartbeat = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            app.oneshot(Request::get("/heartbeat").body(Body::empty())?),
        )
        .await??;
        assert_eq!(heartbeat.status(), StatusCode::OK);
        let result = calculation.await?.map_err(|_| "discovery failed")?;
        assert_eq!(result.sources.len(), 1001);
        assert!(result.sources.iter().all(|s| s.estimate.is_none()));
        assert_eq!(computation.available_permits(), 2);
        Ok(())
    }
}
