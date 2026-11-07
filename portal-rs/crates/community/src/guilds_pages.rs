use super::*;
use askama::Template;
use axum::{
    extract::{OriginalUri, RawQuery},
    response::Html,
    routing::get,
};
use std::{future::Future, pin::Pin};
#[derive(Debug)]
pub enum Unavailable {
    Evidence,
    Database,
}
#[derive(Debug, Clone)]
pub struct MemberRef {
    pub character_id: String,
    pub guild_id: String,
    pub world_id: String,
    pub name: String,
}
pub type VisibilityResult<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<bool>, Unavailable>> + Send + 'a>>;
/// True requires current same-epoch, same-world eligible profile evidence by stable
/// CharacterId and current Platform profile/guild visibility. Name is only a
/// consistency check. Unknown evidence must return false or an error.
pub trait PublicVisibility: Send + Sync {
    fn visible_batch<'a>(
        &'a self,
        epoch: &'a str,
        members: &'a [MemberRef],
    ) -> VisibilityResult<'a>;
}
#[cfg(test)]
pub(super) struct FailClosed;
#[cfg(test)]
impl PublicVisibility for FailClosed {
    fn visible_batch<'a>(&'a self, _: &'a str, _: &'a [MemberRef]) -> VisibilityResult<'a> {
        Box::pin(async { Err(Unavailable::Evidence) })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub enabled: bool,
    pub source_authority: String,
}
#[derive(Clone)]
struct Guilds {
    contract_accepted: bool,
    db: PgPool,
    visibility: Arc<dyn PublicVisibility>,
    config: Config,
}
pub fn router(db: PgPool, visibility: Arc<dyn PublicVisibility>, config: Config) -> Router {
    router_with_contract(db, visibility, config, true)
}
pub(crate) fn router_with_contract(
    db: PgPool,
    visibility: Arc<dyn PublicVisibility>,
    config: Config,
    contract_accepted: bool,
) -> Router {
    Router::new()
        .route("/guilds", get(page))
        .route("/guilds/{name}", get(page))
        .route("/pl/guilds", get(page))
        .route("/pl/guilds/{name}", get(page))
        .route("/en/guilds", get(page))
        .route("/en/guilds/{name}", get(page))
        .with_state(Guilds {
            contract_accepted,
            db,
            visibility,
            config,
        })
        .layer(axum::middleware::from_fn(headers))
}
async fn headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "private, no-store"),
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    q: Option<String>,
    page: Option<usize>,
}
struct Member {
    name: String,
    rank: String,
    joined: String,
}
struct Guild {
    name: String,
    href: String,
    state: String,
    founded: String,
    members: Vec<Member>,
}
#[derive(Template)]
#[template(path = "guilds/page.html", config = "src/guilds_askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    detail: bool,
    search: String,
    guilds: Vec<Guild>,
    previous: String,
    next: String,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../../templates/guilds/en.ftl")
    } else {
        include_str!("../../../templates/guilds/pl.ftl")
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
        "search",
        "apply",
        "members",
        "founded",
        "rank",
        "joined",
        "empty",
        "previous",
        "next",
        "forming",
        "active",
        "disbanding",
    ]
    .iter()
    .map(|key| {
        bundle
            .get_message(key)
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
fn date(s: &str) -> Result<String, Unavailable> {
    let n = s.parse::<i64>().map_err(|_| Unavailable::Evidence)?;
    let time = time::OffsetDateTime::from_unix_timestamp(n).map_err(|_| Unavailable::Evidence)?;
    Ok(time.date().to_string())
}
fn encoded(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
async fn load(state: &Guilds, lang: &str, labels: &[String]) -> Result<Vec<Guild>, Unavailable> {
    if !state.contract_accepted {
        return Err(Unavailable::Evidence);
    }
    let wm=sqlx::query("SELECT epoch::text,watermark,payload FROM public_snapshots WHERE family='guild' AND subject='v2' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)").fetch_optional(&state.db).await.map_err(|_|Unavailable::Database)?.ok_or(Unavailable::Evidence)?;
    let watermark_payload: serde_json::Value =
        wm.try_get("payload").map_err(|_| Unavailable::Evidence)?;
    if !wire::authority(&state.config.source_authority)
        || watermark_payload["source_authority"].as_str()
            != Some(state.config.source_authority.as_str())
    {
        return Err(Unavailable::Evidence);
    }
    let epoch: String = wm.try_get("epoch").map_err(|_| Unavailable::Evidence)?;
    let through: i64 = wm.try_get("watermark").map_err(|_| Unavailable::Evidence)?;
    let current = i64::try_from(now()).map_err(|_| Unavailable::Evidence)?;
    if through > current.saturating_add(1) || current.saturating_sub(through).saturating_add(1) > 30
    {
        return Err(Unavailable::Evidence);
    }
    let profile_through: Option<i64> = sqlx::query_scalar("SELECT watermark FROM public_snapshots WHERE family='character_profile' AND subject='' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)").fetch_optional(&state.db).await.map_err(|_|Unavailable::Database)?;
    let profile_through = profile_through.ok_or(Unavailable::Evidence)?;
    if profile_through > current.saturating_add(1)
        || current.saturating_sub(profile_through).saturating_add(1) > 30
    {
        return Err(Unavailable::Evidence);
    }
    let subjects:Vec<String>=sqlx::query_scalar("SELECT subject FROM public_snapshots WHERE family='guild' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) ORDER BY subject").fetch_all(&state.db).await.map_err(|_|Unavailable::Database)?;
    let mut result = vec![];
    for subject in subjects {
        let payload = crate::ingest::latest(&state.db, "guild", &subject)
            .await
            .map_err(|_| Unavailable::Database)?
            .ok_or(Unavailable::Evidence)?;
        if payload["contract_version"] != 2
            || payload["source_authority"].as_str() != Some(state.config.source_authority.as_str())
        {
            return Err(Unavailable::Evidence);
        }
        let guilds: Vec<wire::Guild> =
            serde_json::from_value(payload["guild"].clone()).map_err(|_| Unavailable::Evidence)?;
        if guilds.len() > 1 {
            return Err(Unavailable::Evidence);
        }
        for g in guilds {
            wire::validate(&g, current).map_err(|_| Unavailable::Evidence)?;
            let mut eligible = Vec::new();
            for member in &g.members {
                let profile =
                    crate::ingest::latest(&state.db, "character_profile", &member.character_id)
                        .await
                        .map_err(|_| Unavailable::Database)?;
                let Some(payload) = profile else {
                    continue;
                };
                let Some(profiles) = payload["profile"].as_array() else {
                    return Err(Unavailable::Evidence);
                };
                if profiles.is_empty() {
                    continue;
                }
                if profiles.len() != 1 {
                    return Err(Unavailable::Evidence);
                }
                let profile = &profiles[0];
                let memberships = profile["guild"].as_array().ok_or(Unavailable::Evidence)?;
                if profile["world_id"].as_str() == Some(g.world_id.as_str())
                    && profile["name"].as_str() == Some(member.name.as_str())
                    && profile["guild_visible"].as_bool() == Some(true)
                    && memberships.len() == 1
                    && memberships[0]["guild_id"].as_str() == Some(subject.as_str())
                    && memberships[0]["rank_level"].as_u64() == Some(u64::from(member.rank_level))
                {
                    eligible.push(member);
                }
            }
            let refs: Vec<_> = eligible
                .iter()
                .map(|m| MemberRef {
                    character_id: m.character_id.clone(),
                    guild_id: subject.clone(),
                    world_id: g.world_id.clone(),
                    name: m.name.clone(),
                })
                .collect();
            let visible = state.visibility.visible_batch(&epoch, &refs).await?;
            if visible.len() != refs.len() {
                return Err(Unavailable::Evidence);
            }
            let mut members = vec![];
            for (m, allowed) in eligible.into_iter().zip(visible) {
                if allowed {
                    members.push(Member {
                        name: m.name.clone(),
                        rank: g
                            .ranks
                            .iter()
                            .find(|r| r.level == m.rank_level)
                            .ok_or(Unavailable::Evidence)?
                            .name
                            .clone(),
                        joined: date(&m.joined_at)?,
                    });
                }
            }
            let state_label = match g.state.as_str() {
                "FORMING" => 11,
                "ACTIVE" => 12,
                "DISBANDING" => 13,
                _ => return Err(Unavailable::Evidence),
            };
            result.push(Guild {
                href: format!("/{lang}/guilds/{}", encoded(&g.name)),
                name: g.name,
                state: labels[state_label].clone(),
                founded: date(&g.founded_at)?,
                members,
            });
        }
    }
    let fence: Option<String> =
        sqlx::query_scalar("SELECT highest_epoch::text FROM public_projection_state WHERE id=1")
            .fetch_one(&state.db)
            .await
            .map_err(|_| Unavailable::Database)?;
    if fence.as_deref() != Some(&epoch)
        || i64::try_from(now())
            .map_err(|_| Unavailable::Evidence)?
            .saturating_sub(through.min(profile_through))
            .saturating_add(1)
            > 30
    {
        return Err(Unavailable::Evidence);
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    let mut names = std::collections::HashSet::new();
    if result.iter().any(|g| !names.insert(g.name.to_lowercase())) {
        return Err(Unavailable::Evidence);
    }
    Ok(result)
}
async fn page(
    State(state): State<Guilds>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let en = uri.path().starts_with("/en/");
    let lang = if en { "en" } else { "pl" };
    let labels = labels(en);
    let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or("")) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let search = query.q.unwrap_or_default().trim().to_owned();
    let index = query.page.unwrap_or(1);
    if search.chars().count() > 80
        || search.chars().any(char::is_control)
        || index == 0
        || index > 1000000
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let prefix = if en {
        "/en/guilds"
    } else if uri.path().starts_with("/pl/") {
        "/pl/guilds"
    } else {
        "/guilds"
    };
    let detail = uri.path() != prefix;
    let name = if detail {
        let Some(raw) = uri.path().strip_prefix(&format!("{prefix}/")) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Ok(name) = percent_encoding::percent_decode_str(raw).decode_utf8() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if !wire::name(&name) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        Some(name.into_owned())
    } else {
        None
    };
    let result = if state.config.enabled {
        load(&state, lang, &labels).await
    } else {
        Err(Unavailable::Evidence)
    };
    let unavailable = result.is_err();
    let status = if !state.contract_accepted || matches!(result, Err(Unavailable::Database)) {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    let mut guilds = result.unwrap_or_default();
    if let Some(name) = name {
        guilds.retain(|g| g.name == name);
        if !unavailable && guilds.is_empty() {
            return StatusCode::NOT_FOUND.into_response();
        }
    } else {
        guilds.retain(|g| g.name.to_lowercase().contains(&search.to_lowercase()));
    }
    let count = if detail {
        guilds.first().map_or(0, |g| g.members.len())
    } else {
        guilds.len()
    };
    let start = (index - 1) * 50;
    if detail {
        for g in &mut guilds {
            g.members = std::mem::take(&mut g.members)
                .into_iter()
                .skip(start)
                .take(50)
                .collect();
        }
    } else {
        guilds = guilds.into_iter().skip(start).take(50).collect();
    }
    let link = |p| format!("{}?q={}&page={p}", uri.path(), encoded(&search));
    let previous = if index > 1 {
        link(index - 1)
    } else {
        String::new()
    };
    let next = if start.saturating_add(50) < count {
        link(index + 1)
    } else {
        String::new()
    };
    let page = Page {
        lang,
        labels,
        unavailable,
        detail,
        search,
        guilds,
        previous,
        next,
    };
    match page.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
