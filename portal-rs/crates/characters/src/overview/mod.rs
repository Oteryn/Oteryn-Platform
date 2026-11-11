//! Private account overview. Missing producer facts remain explicitly unavailable.
use crate::lcfa::{AccountId, AccountState, AccountStateLookup, Projection, ReadState, Settings};
mod wallet;
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

/// Identity account state for LCFA reads: only `active` reads. Callers that
/// also need sanctions apply them before and after the read (player API).
#[derive(Debug, Clone)]
pub struct Accounts(pub PgPool);
impl AccountStateLookup for Accounts {
    fn state(&self, account: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async move {
            match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1")
                .bind(account.as_uuid())
                .fetch_optional(&self.0)
                .await
            {
                Ok(Some(status)) if status == "active" => AccountState::Active,
                Ok(_) => AccountState::Denied,
                Err(_) => AccountState::Unavailable,
            }
        })
    }
}

#[derive(Template)]
#[template(path = "account/overview.html", config = "src/overview/askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    facts: Vec<(String, String)>,
    characters_label: String,
    state: String,
    characters: Vec<(String, String, String, String)>,
    world: String,
    availability: String,
    level: String,
    profession: String,
    unavailable: String,
    shortcuts: String,
    security: String,
    premium: String,
    balance: String,
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("../../../../templates/account/overview.en.ftl")
    } else {
        include_str!("../../../../templates/account/overview.pl.ftl")
    };
    let Ok(locale) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.into()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
/// Called only with the account resolved by the server-side browser session.
/// No account identifier is accepted from the URL or request body.
pub async fn show(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    enabled: bool,
    settings: Settings,
) -> Response {
    let mut response = load(pool, account, lang, enabled, settings).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-robots-tag"),
        header::HeaderValue::from_static("noindex, nofollow"),
    );
    response
}
async fn load(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    enabled: bool,
    settings: Settings,
) -> Response {
    if !enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = match lang {
        "pl" => "pl",
        "en" => "en",
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let Some(account) = account else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let Ok(id) = AccountId::parse(&account.to_string()) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let record = sqlx::query_as::<_, (String, String)>(
        "SELECT status, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') || ' UTC' FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_optional(pool)
    .await;
    let (status, created) = match record {
        Ok(Some(row)) if row.0 == "active" => row,
        Ok(_) => return StatusCode::FORBIDDEN.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let projection = Projection::new(pool.clone(), settings, Arc::new(Accounts(pool.clone())));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|v| i64::try_from(v.as_secs()).ok());
    let view = match now {
        Some(now) => projection.list_characters(id, now).await.ok(),
        None => None,
    };
    if view.as_ref().is_some_and(|v| v.state == ReadState::Denied) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let read_state = view.as_ref().map(|view| view.state);
    let t = |key| text(lang, key);
    let mut facts = vec![(t("created"), created), (t("status"), t(&status))];
    for key in ["web-login", "game-login", "loyalty", "title", "termination"] {
        facts.push((t(key), t("unavailable")));
    }
    let limits = configured_limits();
    facts.push((
        t("limit"),
        limits
            .map(|(active, total)| format!("{active} / {total}"))
            .unwrap_or_else(|| t("unavailable")),
    ));
    facts.push((
        t("balance"),
        wallet::balance(pool, account)
            .await
            .map(|value| value.to_string())
            .unwrap_or_else(|| t("unavailable")),
    ));
    let characters = view
        .filter(|v| v.state == ReadState::Ready)
        .map(|v| {
            v.characters
                .into_iter()
                .map(|c| {
                    let availability =
                        t(if c.availability == crate::lcfa::Availability::Available {
                            "available"
                        } else {
                            "unavailable"
                        });
                    (
                        segment(&c.name),
                        c.name,
                        c.world_id.to_string(),
                        availability,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let page = Page {
        lang,
        title: t("overview"),
        facts,
        characters_label: t("characters"),
        state: t(match read_state {
            Some(ReadState::Ready) => "linked",
            Some(ReadState::Missing) => "missing",
            Some(ReadState::Stale) => "stale",
            Some(ReadState::Invalid) => "invalid",
            _ => "unavailable",
        }),
        characters,
        world: t("world"),
        availability: t("availability"),
        level: t("level"),
        profession: t("profession"),
        unavailable: t("unavailable"),
        shortcuts: t("shortcuts"),
        security: t("security"),
        premium: t("premium"),
        balance: t("balance"),
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests;

/// Projection settings retain the LCFA environment and authority restrictions.
pub fn configured_projection() -> Settings {
    Settings {
        enabled: std::env::var("OTERYN_LCFA_ENABLED").as_deref() == Ok("true"),
        environment: match std::env::var("OTERYN_ENV").as_deref() {
            Ok("testing") => oteryn_core::Environment::Testing,
            Ok("preprod") => oteryn_core::Environment::Preprod,
            _ => oteryn_core::Environment::Production,
        },
        source_authority: std::env::var("OTERYN_LCFA_SOURCE_AUTHORITY").unwrap_or_default(),
        ..Settings::default()
    }
}

/// Missing, inconsistent or malformed limits never become an unlimited allowance.
pub fn configured_limits() -> Option<(u16, u16)> {
    parse_limits(
        &std::env::var("OTERYN_ACTIVE_CHARACTER_LIMIT").ok()?,
        &std::env::var("OTERYN_TOTAL_CHARACTER_LIMIT").ok()?,
    )
}
fn parse_limits(active: &str, total: &str) -> Option<(u16, u16)> {
    let active = active.parse::<u16>().ok()?;
    let total = total.parse::<u16>().ok()?;
    (active > 0 && total >= active).then_some((active, total))
}
