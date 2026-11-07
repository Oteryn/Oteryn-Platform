use super::{Config, Error, History, Source, Summary, history, list};
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use std::sync::Arc;
use time::{Date, Duration, OffsetDateTime};
#[derive(Clone)]
struct App {
    db: PgPool,
    config: Config,
    source: Arc<dyn Source>,
}
pub fn router<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    config: Config,
    source: Arc<dyn Source>,
) -> Router<S> {
    Router::new()
        .route("/stats/guilds", get(page))
        .route("/pl/stats/guilds", get(page))
        .route("/en/stats/guilds", get(page))
        .with_state(App { db, config, source })
}
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    router(
        db.clone(),
        Config {
            enabled: std::env::var("PORTAL_GUILD_STATS_ENABLED").as_deref() == Ok("true"),
        },
        Arc::new(crate::privacy::GuildSource::configured(db)),
    )
}
#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    guild: Option<String>,
    compare: Option<String>,
    days: Option<u8>,
}
#[derive(Template)]
#[template(path = "stats/guilds/index.html", config = "src/guilds/askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    cards: Vec<Card>,
}
struct Card {
    summary: Summary,
    history: Option<History>,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../../../templates/stats/guilds/en.ftl")
    } else {
        include_str!("../../../../templates/stats/guilds/pl.ftl")
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
        "note",
        "members",
        "level",
        "average",
        "experience",
        "founded",
        "history",
        "growth",
        "guild",
        "compare",
        "submit",
        "days",
        "extras",
    ]
    .iter()
    .map(|key| {
        bundle
            .get_message(&format!("guild-{key}"))
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
async fn render(state: App, uri: axum::http::Uri, raw: Option<String>) -> Response {
    if !state.config.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    if raw.as_ref().is_some_and(|v| v.len() > 1024) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or_default()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let days = query.days.unwrap_or(7);
    if !(1..=31).contains(&days)
        || [&query.guild, &query.compare].iter().any(|v| {
            v.as_ref()
                .is_some_and(|s| s.len() > 128 || s.chars().any(char::is_control))
        })
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let guild = query.guild.filter(|s| !s.is_empty());
    let compare = query.compare.filter(|s| !s.is_empty());
    if compare.is_some() && guild.is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let result = async {
        let mut summaries = list(state.config, state.source.as_ref()).await?;
        if let Some(name) = &guild {
            for selected in std::iter::once(name).chain(compare.iter()) {
                if summaries
                    .iter()
                    .filter(|s| &s.guild.name == selected)
                    .count()
                    != 1
                {
                    return Err(Error::Unavailable);
                }
            }
            summaries.retain(|s| &s.guild.name == name || compare.as_ref() == Some(&s.guild.name));
        }
        let end = OffsetDateTime::now_utc().date();
        let start: Date = end
            .checked_sub(Duration::days(i64::from(days) - 1))
            .ok_or(Error::BadInput)?;
        let mut cards = Vec::new();
        for summary in summaries {
            let evidence = if guild.is_some() {
                Some(
                    history(
                        &state.db,
                        state.config,
                        state.source.as_ref(),
                        summary.guild.id,
                        start,
                        end,
                    )
                    .await?,
                )
            } else {
                None
            };
            cards.push(Card {
                summary,
                history: evidence,
            });
        }
        Ok::<_, Error>(cards)
    }
    .await;
    let en = uri.path().starts_with("/en/");
    let unavailable = result.is_err();
    let page = Page {
        lang: if en { "en" } else { "pl" },
        labels: labels(en),
        unavailable,
        cards: result.unwrap_or_default(),
    };
    match page.render() {
        Ok(body) => (
            if unavailable {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::OK
            },
            Html(body),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn page(
    State(state): State<App>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let mut response = render(state, uri, raw).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        header::HeaderValue::from_static(
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response
}
