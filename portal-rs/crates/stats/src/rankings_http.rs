use super::*;
use askama::Template;
use axum::{
    Router,
    extract::{Path, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use serde::Deserialize;

struct Unavailable;
impl PublicCatalog for Unavailable {
    fn characters(&self, _: Kind) -> CatalogFuture<'_> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
/// A current privacy/catalog adapter is required before enabling public statistics.
pub fn configured_router(db: PgPool) -> Router {
    router(Rankings {
        db,
        enabled: std::env::var("PORTAL_GROWTH_RANKINGS_ENABLED").as_deref() == Ok("true"),
        catalog: Arc::new(Unavailable),
    })
}
pub fn router(state: Rankings) -> Router {
    Router::new()
        .route("/{lang}/stats/rankings/{kind}", get(handle))
        .with_state(state)
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Option<String>,
    vocation: Option<String>,
    pvp: Option<String>,
    page: Option<String>,
}
fn parse(raw: &str) -> Result<Filters, Error> {
    if raw.len() > 2048
        || raw.as_bytes().iter().enumerate().any(|(i, b)| {
            *b == b'%'
                && (i + 2 >= raw.len()
                    || !raw.as_bytes()[i + 1].is_ascii_hexdigit()
                    || !raw.as_bytes()[i + 2].is_ascii_hexdigit())
        })
    {
        return Err(Error::Invalid);
    }
    let q: Query = serde_urlencoded::from_str(raw).map_err(|_| Error::Invalid)?;
    let world = q
        .world
        .filter(|v| !v.is_empty())
        .map(|s| {
            Uuid::parse_str(&s)
                .ok()
                .filter(|u| u.to_string() == s)
                .ok_or(Error::Invalid)
        })
        .transpose()?;
    let vocation = q
        .vocation
        .filter(|v| !v.is_empty())
        .map(|s| match s.as_str() {
            "0" => Ok(0),
            "1" => Ok(1),
            "2" => Ok(2),
            "3" => Ok(3),
            "4" => Ok(4),
            "5" => Ok(5),
            _ => Err(Error::Invalid),
        })
        .transpose()?;
    let pvp = match q.pvp.as_deref().unwrap_or("") {
        "" => None,
        "open" => Some(Pvp::Open),
        "optional" => Some(Pvp::Optional),
        "hardcore" => Some(Pvp::Hardcore),
        _ => return Err(Error::Invalid),
    };
    let page = q.page.as_deref().unwrap_or("1");
    let number = page.parse::<usize>().map_err(|_| Error::Invalid)?;
    if number == 0 || number > 20_000 || number.to_string() != page {
        return Err(Error::Invalid);
    }
    Ok(Filters {
        world,
        vocation,
        pvp,
        page: number,
    })
}
struct Text(FluentBundle<FluentResource>);
impl Text {
    fn new(lang: &str) -> Option<Self> {
        let source = if lang == "pl" {
            include_str!("../../../templates/stats/rankings/pl.ftl")
        } else {
            include_str!("../../../templates/stats/rankings/en.ftl")
        };
        let resource = FluentResource::try_new(source.to_owned()).ok()?;
        let mut bundle = FluentBundle::new_concurrent(vec![lang.parse().ok()?]);
        bundle.add_resource(resource).ok()?;
        Some(Self(bundle))
    }
    fn get(&self, key: &str) -> String {
        self.0
            .get_message(key)
            .and_then(|m| m.value())
            .map(|v| self.0.format_pattern(v, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}
#[derive(Template)]
#[template(path = "stats/rankings/index.html", config = "../web/askama.toml")]
struct Page {
    lang: String,
    title: String,
    message: String,
    hint: String,
    labels: Vec<String>,
    kinds: Vec<(String, String)>,
    rows: Vec<(usize, String, String, String)>,
    worlds: Vec<(String, String)>,
    vocations: Vec<String>,
    world: String,
    vocation: String,
    pvp: String,
    page: usize,
    previous: String,
    next: String,
}
async fn handle(
    State(state): State<Rankings>,
    Path((lang, kind)): Path<(String, String)>,
    RawQuery(raw): RawQuery,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(text) = Text::new(&lang) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut page = Page {
        lang,
        title: text.get("title"),
        message: String::new(),
        hint: text.get("hint"),
        labels: [
            "world", "vocation", "pvp", "apply", "rank", "name", "value", "date", "page", "all",
            "open", "optional", "hardcore",
        ]
        .map(|k| text.get(k))
        .to_vec(),
        kinds: Kind::ALL
            .iter()
            .map(|(_, k)| (k.to_string(), text.get(k)))
            .collect(),
        rows: vec![],
        worlds: vec![],
        vocations: (0..=5)
            .map(|v| text.get(&format!("vocation-{v}")))
            .collect(),
        world: String::new(),
        vocation: String::new(),
        pvp: String::new(),
        page: 1,
        previous: text.get("previous"),
        next: String::new(),
    };
    let result = async {
        if !state.enabled {
            return Err(Error::Disabled);
        }
        let k = Kind::ALL
            .iter()
            .find(|(_, s)| *s == kind)
            .map(|(k, _)| *k)
            .ok_or(Error::NotFound)?;
        let filters = parse(raw.as_deref().unwrap_or(""))?;
        let ranking = state
            .load(k, time::OffsetDateTime::now_utc().date(), &filters)
            .await?;
        page.worlds = ranking.worlds;
        page.world = filters.world.map(|v| v.to_string()).unwrap_or_default();
        page.vocation = filters.vocation.map(|v| v.to_string()).unwrap_or_default();
        page.pvp = match filters.pvp {
            Some(Pvp::Open) => "open",
            Some(Pvp::Optional) => "optional",
            Some(Pvp::Hardcore) => "hardcore",
            None => "",
        }
        .into();
        page.page = ranking.page;
        if ranking.page * 50 < ranking.total {
            page.next = text.get("next");
        }
        let event = matches!(k, Kind::NameChanges | Kind::Transfers | Kind::Sold);
        page.rows = ranking
            .rows
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                (
                    (ranking.page - 1) * 50 + i + 1,
                    e.character.name,
                    if event { text.get(&kind) } else { e.value },
                    e.day.to_string(),
                )
            })
            .collect();
        if page.rows.is_empty() {
            page.message = text.get("empty");
        }
        Ok(())
    }
    .await;
    let status = match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            page.message = text.get(match e {
                Error::Disabled | Error::NotFound => "not-found",
                Error::Invalid => "invalid",
                Error::Unavailable => "unavailable",
            });
            match e {
                Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
                Error::Invalid => StatusCode::BAD_REQUEST,
                Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            }
        }
    };
    let body = page.render().unwrap_or_else(|_| text.get("unavailable"));
    (
        status,
        [
            (header::CACHE_CONTROL, "no-store, private"),
            (header::CONTENT_LANGUAGE, page.lang.as_str()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            ),
            (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
            (
                header::STRICT_TRANSPORT_SECURITY,
                "max-age=63072000; includeSubDomains",
            ),
        ],
        Html(body),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_validation_and_translations() {
        for raw in [
            "page=0",
            "page=01",
            "page=1&page=2",
            "world=bad",
            "vocation=6",
            "pvp=unknown",
            "x=1",
            "world=%",
            "page=20001",
        ] {
            assert!(parse(raw).is_err(), "{raw}");
        }
        assert!(parse("vocation=0&pvp=open&page=2").is_ok());
        for lang in ["pl", "en"] {
            let Some(text) = Text::new(lang) else {
                unreachable!()
            };
            for (_, key) in Kind::ALL {
                assert!(!text.get(key).is_empty());
            }
            assert!(!text.get("unavailable").is_empty());
        }
    }
}
