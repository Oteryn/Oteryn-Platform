use super::{
    CalculationError, experience_to_level, loot_split, skill_training_time, stamina_regeneration,
};
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_world::{Entity, WorldOptions, WorldStore};
use serde::Deserialize;
use sqlx::PgPool;

/// Explicit references to allow-listed records, never public user-selected paths.
#[derive(Clone, Debug)]
pub struct WorldKey {
    pub kind: String,
    pub key: String,
}
/// Policy references remain absent until T14 imports the authoritative records.
#[derive(Clone, Debug, Default)]
pub struct CalculatorOptions {
    pub enabled: bool,
    pub experience_table: Option<WorldKey>,
    pub training_rates: Option<WorldKey>,
}
#[derive(Clone, Debug)]
struct Calculators {
    world: WorldStore,
    options: CalculatorOptions,
}

pub fn router<S: Clone + Send + Sync + 'static>(
    world: WorldStore,
    options: CalculatorOptions,
) -> Router<S> {
    Router::new()
        .route("/pl/wiki/calculators", get(page))
        .route("/en/wiki/calculators", get(page))
        .with_state(Calculators { world, options })
}
/// Both flags accept only `true`. Optional references use `kind|key`.
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    fn reference(variable: &str) -> Option<WorldKey> {
        let value = std::env::var(variable).ok()?;
        let (kind, key) = value.split_once('|')?;
        if kind.is_empty() || key.is_empty() || key.contains('|') || value.len() > 512 {
            return None;
        }
        Some(WorldKey {
            kind: kind.to_owned(),
            key: key.to_owned(),
        })
    }
    router(
        WorldStore::new(
            db,
            WorldOptions {
                enabled: std::env::var("PORTAL_WORLD_ENABLED").as_deref() == Ok("true"),
            },
        ),
        CalculatorOptions {
            enabled: std::env::var("PORTAL_CALCULATORS_ENABLED").as_deref() == Ok("true"),
            experience_table: reference("PORTAL_CALCULATORS_EXPERIENCE_SOURCE"),
            training_rates: reference("PORTAL_CALCULATORS_TRAINING_SOURCE"),
        },
    )
}
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Input {
    calculator: Option<String>,
    current: Option<String>,
    target: Option<String>,
    offline: Option<String>,
    remaining: Option<String>,
    family: Option<String>,
    dummy: Option<String>,
    total: Option<String>,
    members: Option<String>,
}
fn number(input: &Option<String>) -> Result<u64, CalculationError> {
    let text = input.as_deref().ok_or(CalculationError::BadInput)?;
    if text.is_empty() || text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CalculationError::BadInput);
    }
    text.parse().map_err(|_| CalculationError::BadInput)
}
impl Calculators {
    async fn entity(&self, source: &Option<WorldKey>) -> Result<Entity, CalculationError> {
        let source = source.as_ref().ok_or(CalculationError::Unavailable)?;
        self.world
            .get_entity(&source.kind, &source.key)
            .await
            .map_err(|_| CalculationError::Unavailable)?
            .ok_or(CalculationError::Unavailable)
    }
    async fn calculate(&self, input: &Input) -> Result<(Vec<u64>, String), CalculationError> {
        match input.calculator.as_deref() {
            Some("experience") => {
                let current = number(&input.current)?;
                let target = u32::try_from(number(&input.target)?)
                    .map_err(|_| CalculationError::BadInput)?;
                let entity = self.entity(&self.options.experience_table).await?;
                Ok((
                    vec![experience_to_level(&entity.data, current, target)?],
                    entity.version_id.to_string(),
                ))
            }
            Some("stamina") => Ok((
                vec![stamina_regeneration(
                    number(&input.current)?,
                    number(&input.offline)?,
                )?],
                String::new(),
            )),
            Some("training") => {
                let remaining = number(&input.remaining)?;
                let family = usize::try_from(number(&input.family)?)
                    .map_err(|_| CalculationError::BadInput)?;
                let dummy = input
                    .dummy
                    .as_deref()
                    .filter(|v| !v.is_empty() && v.len() <= 512)
                    .ok_or(CalculationError::BadInput)?;
                let entity = self.entity(&self.options.training_rates).await?;
                let (seconds, charges) =
                    skill_training_time(&entity.data, remaining, family, dummy)?;
                Ok((vec![seconds, charges], entity.version_id.to_string()))
            }
            Some("loot") => {
                let split = loot_split(number(&input.total)?, number(&input.members)?)?;
                Ok((vec![split.per_member, split.remainder], String::new()))
            }
            _ => Err(CalculationError::BadInput),
        }
    }
}
#[derive(Template)]
#[template(
    path = "wiki/calculators/page.html",
    config = "src/calculators.askama.toml"
)]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    input: Input,
    result: Vec<u64>,
    result_label: String,
    version: String,
    error: String,
}
const IDS: &[&str] = &[
    "title",
    "skip",
    "experience",
    "current",
    "target",
    "calculate",
    "stamina",
    "minutes",
    "offline",
    "training",
    "remaining",
    "family",
    "dummy",
    "loot",
    "total",
    "members",
    "unavailable",
    "invalid",
    "experience-result",
    "stamina-result",
    "training-result",
    "loot-result",
    "version",
    "estimate",
    "training-help",
    "loot-help",
    "language",
];
fn labels(lang: &str) -> Option<Vec<String>> {
    let source = if lang == "pl" {
        include_str!("../../../../templates/wiki/calculators/pl.ftl")
    } else {
        include_str!("../../../../templates/wiki/calculators/en.ftl")
    };
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let locale = lang.parse::<unic_langid::LanguageIdentifier>().ok()?;
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle.add_resource(resource).ok()?;
    IDS.iter()
        .map(|id| {
            let pattern = bundle.get_message(id)?.value()?;
            let mut errors = Vec::new();
            let text = bundle
                .format_pattern(pattern, None, &mut errors)
                .into_owned();
            errors.is_empty().then_some(text)
        })
        .collect()
}
async fn page(
    State(state): State<Calculators>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<Input>, QueryRejection>,
) -> Response {
    if !state.options.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = if uri.path().starts_with("/pl/") {
        "pl"
    } else {
        "en"
    };
    let Some(labels) = labels(lang) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut status = StatusCode::OK;
    let mut page = Page {
        lang,
        labels,
        input: Input::default(),
        result: vec![],
        result_label: String::new(),
        version: String::new(),
        error: String::new(),
    };
    if uri.query().is_some_and(|query| query.len() > 2048) {
        status = StatusCode::BAD_REQUEST;
        page.error = page.labels[17].clone();
    } else {
        match query {
            Ok(Query(input)) => {
                if uri.query().is_some_and(|query| !query.is_empty()) {
                    match state.calculate(&input).await {
                        Ok((result, version)) => {
                            page.result_label = page.labels[match input.calculator.as_deref() {
                                Some("experience") => 18,
                                Some("stamina") => 19,
                                Some("training") => 20,
                                _ => 21,
                            }]
                            .clone();
                            page.result = result;
                            page.version = version;
                        }
                        Err(CalculationError::BadInput) => {
                            status = StatusCode::BAD_REQUEST;
                            page.error = page.labels[17].clone();
                        }
                        Err(CalculationError::Unavailable) => {
                            status = StatusCode::SERVICE_UNAVAILABLE;
                            page.error = page.labels[16].clone();
                        }
                    }
                }
                page.input = input;
            }
            Err(_) => {
                status = StatusCode::BAD_REQUEST;
                page.error = page.labels[17].clone();
            }
        }
    }
    match page.render() {
        Ok(html) => (status, [
            (header::CONTENT_LANGUAGE, lang), (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
        ], Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
