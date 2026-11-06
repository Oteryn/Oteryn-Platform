//! Browser boundary supplied by the web composition root, avoiding a crate cycle.
use crate::{
    accounts::AccountId,
    recovery_key::{Enabled, Error, RecoveryKeys, SecurityReset},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use serde::Deserialize;
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use unic_langid::langid;

pub type RateFuture = Pin<Box<dyn Future<Output = Result<(), Response>> + Send>>;
/// Only the composition root constructs this from the public browser-session API.
/// The shared session middleware must enforce CSRF before invoking these handlers.
#[derive(Clone)]
pub struct BrowserContext {
    pub account: Option<AccountId>,
    pub chrome: oteryn_ui::Chrome,
    pub csrf: String,
    pub notice: String,
    pub flash: Arc<dyn Fn(&'static str) -> RateFuture + Send + Sync>,
    pub language: &'static str,
    pub rate: Arc<dyn Fn(String) -> RateFuture + Send + Sync>,
}
#[derive(Clone, Copy, Debug)]
pub struct ResetAdapter(pub SecurityReset);

pub fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool) -> Router<S> {
    let mut router = Router::new();
    for prefix in ["", "/pl", "/en"] {
        router = router
            .route(&format!("{prefix}/recovery-key"), get(show).post(recover))
            .route(
                &format!("{prefix}/account/recovery-key"),
                get(manage).post(generate).delete(revoke),
            )
            .route(
                &format!("{prefix}/account/recovery-key/confirm"),
                get(confirm_show).post(confirm),
            )
            .route(
                &format!("{prefix}/account/recovery-key/revoke"),
                post(revoke),
            );
    }
    router.layer(Extension(RecoveryKeys::new(pool)))
}
fn service(
    mut service: RecoveryKeys,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
) -> Result<RecoveryKeys, StatusCode> {
    service.enabled = enabled.map(|v| v.0).unwrap_or_default();
    if !service.enabled.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(adapter) = adapter {
        service.reset = adapter.0.0;
    }
    Ok(service)
}
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Input {
    pub email: String,
    pub recovery_key: String,
    pub current_password: String,
    pub password: String,
    pub password_confirmation: String,
}
// Credential-bearing input uses redacted Debug output.
#[derive(Template)]
#[template(
    config = "../ui/askama.toml",
    source = r##"{% extends "layout.html" %}{% block title %}{{ self.text("title") }}{% endblock %}{% block content %}<section class="op-card op-stack" data-theme="dark"><h1>{{ self.text("title") }}</h1><p role="status">{{ self.text(message) }}</p>
{% if mode == "confirm" %}{% if !raw.is_empty() %}<p class="op-code">{{ raw }}</p>{% endif %}{% endif %}
<form class="op-stack" method="post" action="/{{ context.language }}/{% if mode != "recover" %}account/{% endif %}recovery-key{% if mode == "confirm" %}/confirm{% endif %}"><input type="hidden" name="_csrf" value="{{ context.csrf }}">
{% if mode == "recover" %}<label class="op-field" for="email">{{ self.text("email") }}<input class="op-input" id="email" name="email" type="email" autocomplete="email" maxlength="254" required></label>{% endif %}
{% if mode == "recover" || mode == "confirm" %}<label class="op-field" for="key">{{ self.text("key") }}<input class="op-input" id="key" name="recovery_key" autocomplete="off" maxlength="160" spellcheck="false" required></label>{% endif %}
{% if mode == "recover" %}<label class="op-field" for="password">{{ self.text("password") }}<input class="op-input" id="password" name="password" type="password" autocomplete="new-password" minlength="10" maxlength="256" required></label><label class="op-field" for="confirmation">{{ self.text("confirmation") }}<input class="op-input" id="confirmation" name="password_confirmation" type="password" autocomplete="new-password" minlength="10" maxlength="256" required></label>{% else %}<label class="op-field" for="current">{{ self.text("current") }}<input class="op-input" id="current" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required></label>{% endif %}
<button class="op-button op-button--primary" type="submit">{% if mode == "recover" %}{{ self.text("recover") }}{% else if mode == "confirm" %}{{ self.text("confirm") }}{% else %}{{ self.text("generate") }}{% endif %}</button>
{% if mode == "manage" %}<button class="op-button" type="submit" formaction="/{{ context.language }}/account/recovery-key/revoke">{{ self.text("revoke") }}</button>{% endif %}</form></section>{% endblock %}"##,
    ext = "html"
)]
struct Page<'a> {
    chrome: oteryn_ui::Chrome,
    context: &'a BrowserContext,
    mode: &'a str,
    raw: &'a str,
    message: &'a str,
}
impl Page<'_> {
    fn text(&self, key: &str) -> String {
        let (locale, source) = if self.context.language == "en" {
            (langid!("en"), include_str!("recovery_key_en.ftl"))
        } else {
            (langid!("pl"), include_str!("recovery_key_pl.ftl"))
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
}
fn page(
    context: &BrowserContext,
    mode: &str,
    raw: &str,
    message: &str,
    status: StatusCode,
) -> Result<Response, StatusCode> {
    let message = if matches!(
        context.notice.as_str(),
        "invalid" | "rejected" | "recovered"
    ) {
        context.notice.as_str()
    } else {
        message
    };
    let body = Page {
        chrome: context.chrome.clone(),
        context,
        mode,
        raw,
        message,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        status,
        [
            (header::CACHE_CONTROL, "no-store, private"),
            (header::PRAGMA, "no-cache"),
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
        ],
        body,
    )
        .into_response())
}
fn redirect(path: &str) -> Result<Response, StatusCode> {
    let location =
        axum::http::HeaderValue::from_str(path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::FOUND,
        [
            (header::LOCATION, location),
            (
                header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store, private"),
            ),
        ],
    )
        .into_response())
}
async fn failure(
    error: Error,
    context: &BrowserContext,
    mode: &str,
) -> Result<Response, StatusCode> {
    let key = match error {
        Error::Disabled => return Err(StatusCode::NOT_FOUND),
        Error::Unavailable => return Err(StatusCode::SERVICE_UNAVAILABLE),
        Error::InvalidInput => "invalid",
        Error::Rejected => "rejected",
    };
    if let Err(response) = (context.flash)(key).await {
        return Ok(response);
    }
    let path = match mode {
        "recover" => "recovery-key",
        "confirm" => "account/recovery-key/confirm",
        _ => "account/recovery-key",
    };
    redirect(&format!("/{}/{path}", context.language))
}
async fn confirm_show(
    Extension(value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
) -> Result<Response, StatusCode> {
    service(value, enabled, adapter)?;
    context.account.ok_or(StatusCode::FORBIDDEN)?;
    page(&context, "confirm", "", "save", StatusCode::OK)
}
async fn show(
    Extension(service_value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
) -> Result<Response, StatusCode> {
    service(service_value, enabled, adapter)?;
    page(&context, "recover", "", "hint", StatusCode::OK)
}
async fn manage(
    Extension(service_value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
) -> Result<Response, StatusCode> {
    service(service_value, enabled, adapter)?;
    context.account.ok_or(StatusCode::FORBIDDEN)?;
    page(&context, "manage", "", "manage-hint", StatusCode::OK)
}
async fn generate(
    Extension(value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let service = service(value, enabled, adapter)?;
    let id = context.account.ok_or(StatusCode::FORBIDDEN)?;
    if let Err(response) = (context.rate)(String::new()).await {
        return Ok(response);
    }
    match service.generate(id, &input.current_password).await {
        Ok(raw) => page(&context, "confirm", &raw, "save", StatusCode::OK),
        Err(error) => failure(error, &context, "manage").await,
    }
}
async fn confirm(
    Extension(value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let service = service(value, enabled, adapter)?;
    let id = context.account.ok_or(StatusCode::FORBIDDEN)?;
    if let Err(response) = (context.rate)(String::new()).await {
        return Ok(response);
    }
    match service
        .confirm(id, &input.current_password, &input.recovery_key)
        .await
    {
        Ok(()) => page(&context, "manage", "", "confirmed", StatusCode::OK),
        Err(error) => failure(error, &context, "confirm").await,
    }
}
async fn revoke(
    Extension(value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let service = service(value, enabled, adapter)?;
    let id = context.account.ok_or(StatusCode::FORBIDDEN)?;
    if let Err(response) = (context.rate)(String::new()).await {
        return Ok(response);
    }
    match service.revoke(id, &input.current_password).await {
        Ok(()) => redirect(&format!("/{}/account/recovery-key", context.language)),
        Err(error) => failure(error, &context, "manage").await,
    }
}
async fn recover(
    Extension(value): Extension<RecoveryKeys>,
    Extension(context): Extension<BrowserContext>,
    enabled: Option<Extension<Enabled>>,
    adapter: Option<Extension<ResetAdapter>>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let service = service(value, enabled, adapter)?;
    // Use a bounded nonempty value so malformed email cannot bypass charging.
    let email = if input.email.trim().is_empty() || input.email.len() > 254 {
        "invalid".to_owned()
    } else {
        input.email.clone()
    };
    if let Err(response) = (context.rate)(email).await {
        return Ok(response);
    }
    match service
        .recover(
            &input.email,
            &input.recovery_key,
            &input.password,
            &input.password_confirmation,
        )
        .await
    {
        Ok(_) => {
            if let Err(response) = (context.flash)("recovered").await {
                return Ok(response);
            }
            redirect(&format!("/{}/login", context.language))
        }
        Err(error) => failure(error, &context, "recover").await,
    }
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for BrowserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserContext").finish_non_exhaustive()
    }
}
