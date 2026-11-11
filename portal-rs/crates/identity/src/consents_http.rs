//! Browser surface; session ownership and CSRF are supplied by the web router.
use super::{Decision, DecisionError, Kind, OptionalConfig, current, decide};
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;
use unic_langid::langid;

#[derive(Template)]
#[template(
    config = "../ui/askama.toml",
    source = r##"{% extends "layout.html" %}
{% block title %}{{ self.text("title") }}{% endblock %}
{% block content %}<section class="op-card op-stack"><h1>{{ self.text("title") }}</h1><p>{{ self.text("hint") }}</p>
{% for item in items %}<form class="op-stack" method="post" action="/{{ lang }}/account/consents">
<input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="kind" value="{{ item.kind }}"><input type="hidden" name="version" value="{{ item.version }}">
<fieldset class="op-stack"><legend>{{ self.text(item.kind) }}</legend>
<label class="op-field" for="{{ item.kind }}"><input type="checkbox" id="{{ item.kind }}" name="granted" value="true" {% if item.granted %}checked{% endif %}>{{ self.text(item.purpose) }}</label>
<p>{% if item.granted %}{{ self.text("granted") }}{% else %}{{ self.text("refused") }}{% endif %}</p>
{% if item.date.is_empty() %}<p>{{ self.text("no-decision") }}</p>{% else %}<p>{{ self.text("decided") }} <time datetime="{{ item.date }}">{{ item.date }}</time></p>{% endif %}
<button class="op-button op-button--primary" type="submit">{{ self.text("save") }}</button></fieldset></form>{% endfor %}</section>{% endblock %}"##,
    ext = "html"
)]
struct Page<'a> {
    chrome: oteryn_ui::Chrome,
    lang: &'a str,
    csrf: &'a str,
    items: Vec<Item<'a>>,
}
struct Item<'a> {
    kind: &'static str,
    purpose: &'static str,
    version: &'a str,
    granted: bool,
    date: String,
}
impl Page<'_> {
    fn text(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
fn text(lang: &str, key: &str) -> String {
    let (locale, source) = if lang == "en" {
        (langid!("en"), include_str!("consents_en.ftl"))
    } else {
        (langid!("pl"), include_str!("consents_pl.ftl"))
    };
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle
        .format_pattern(pattern, None, &mut vec![])
        .into_owned()
}

fn access(
    config: &OptionalConfig,
    account: Option<uuid::Uuid>,
    lang: &str,
) -> Result<uuid::Uuid, Box<Response>> {
    if !matches!(lang, "pl" | "en") || !config.enabled {
        return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
    }
    account.ok_or_else(|| Box::new(Redirect::to(&format!("/{lang}/login")).into_response()))
}
pub async fn show(
    pool: &PgPool,
    config: &OptionalConfig,
    account: Option<uuid::Uuid>,
    lang: &str,
    csrf: &str,
) -> Response {
    let account = match access(config, account, lang) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let mut items = Vec::new();
    for (kind, purpose) in [
        (Kind::PlaySession, "play_session-purpose"),
        (Kind::PersonalisedAds, "personalised_ads-purpose"),
    ] {
        let decision: Option<Decision> = match current(pool, account, kind).await {
            Ok(value) => value,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        items.push(Item {
            kind: kind.code(),
            purpose,
            version: config.version(kind),
            granted: decision.as_ref().is_some_and(|d| d.granted),
            date: decision.map(|d| d.decided_at).unwrap_or_default(),
        });
    }
    match (Page {
        chrome: oteryn_ui::Chrome {
            lang: lang.into(),
            alternate_lang: if lang == "en" { "pl" } else { "en" }.into(),
            alternate_url: format!(
                "/{}/account/consents",
                if lang == "en" { "pl" } else { "en" }
            ),
            account: oteryn_ui::Account::SignedIn { csrf: csrf.into() },
            ..oteryn_ui::Chrome::default()
        },
        lang,
        csrf,
        items,
    })
    .render()
    {
        Ok(body) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store, private"),
            ],
            body,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub kind: String,
    pub version: String,
    pub granted: Option<String>,
    #[serde(rename = "_csrf")]
    pub csrf: Option<String>,
}
pub async fn submit(
    pool: &PgPool,
    config: &OptionalConfig,
    account: Option<uuid::Uuid>,
    lang: &str,
    input: Input,
) -> Response {
    let account = match access(config, account, lang) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let Some(kind) = Kind::parse(&input.kind) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let granted = match input.granted.as_deref() {
        Some("true") => true,
        None => false,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    match decide(pool, config, account, kind, granted, &input.version).await {
        Ok(()) => Redirect::to(&format!("/{lang}/account/consents")).into_response(),
        Err(DecisionError::OutdatedVersion) => oteryn_ui::error_with_message(
            &oteryn_ui::Chrome {
                lang: lang.into(),
                ..oteryn_ui::Chrome::default()
            },
            StatusCode::CONFLICT,
            &text(lang, "outdated"),
        ),
        Err(error) => match error {
            DecisionError::Disabled => StatusCode::NOT_FOUND,
            DecisionError::InvalidInput => StatusCode::BAD_REQUEST,
            DecisionError::OutdatedVersion => StatusCode::CONFLICT,
            DecisionError::Refused => StatusCode::FORBIDDEN,
            DecisionError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response(),
    }
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input").finish_non_exhaustive()
    }
}
