use super::{
    Error,
    http::{Surface, no_store, status},
    presentation::{Labels, timestamp},
    selection::{self, Options, WorldChoice},
    view::read_current,
};
use askama::Template;
use axum::{
    Router,
    extract::{RawQuery, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_admin::{rbac::Permission, web::configured_guard};

pub(super) fn router(state: Surface) -> Result<Router, Box<dyn std::error::Error>> {
    configured_guard(
        Router::new()
            .route("/admin/today", get(page))
            .with_state(state.clone()),
        state.db,
        Permission::GameWorldRead,
    )
}

#[derive(Template)]
#[template(path = "today/operator.html", config = "src/today.askama.toml")]
struct Operator {
    lang: &'static str,
    labels: Labels,
    ingest_enabled: bool,
    public_enabled: bool,
    configured: bool,
    worlds: Vec<WorldChoice>,
    complete_through: Option<String>,
}

impl Operator {
    fn text(&self, key: &str) -> String {
        self.labels.text(key)
    }

    fn switch(&self, enabled: &bool) -> String {
        self.text(if *enabled {
            "operator-enabled"
        } else {
            "operator-disabled"
        })
    }
}

async fn page(
    State(state): State<Surface>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Response {
    let mut response = match load(&state, &headers, raw.as_deref()).await {
        Ok(page) => oteryn_web::render(&page),
        Err(error) => status(error).into_response(),
    };
    no_store(&mut response);
    response
}

async fn load(state: &Surface, headers: &HeaderMap, raw: Option<&str>) -> Result<Operator, Error> {
    let options = Options::parse(raw)?;
    if options.date.is_some() {
        return Err(Error::BadInput);
    }
    let default_lang = if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    };
    let lang = match options.lang.as_deref() {
        Some("en") => "en",
        Some("pl") => "pl",
        _ => default_lang,
    };
    let selected = options.world(&state.config).ok();
    let worlds = selection::choices(&state.db, &state.config, selected).await;
    let complete_through = match selected {
        Some(world) => read_current(&state.db, &state.config, world, lang)
            .await
            .ok()
            .map(|view| timestamp(view.complete_through())),
        None => None,
    };
    Ok(Operator {
        lang,
        labels: Labels::new(lang),
        ingest_enabled: state.config.ingest_enabled,
        public_enabled: state.config.public_enabled,
        configured: state.config.valid() && worlds.len() == state.config.worlds.len(),
        worlds,
        complete_through,
    })
}
