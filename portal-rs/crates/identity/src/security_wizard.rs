//! Read-only account security assessment (plan §5.5, T53).
//!
//! No wizard route exists in the Laravel reference or the published contracts.
//! T05b, T06 and T08b are absent from the base. Their adapter must return verified,
//! current facts; missing evidence never earns credit. Password age is informative,
//! not a reason to demand periodic password changes. This is not an authorization grant.

use std::{future::Future, pin::Pin, sync::Arc};

use axum::{
    Extension, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::{PgPool, types::time::OffsetDateTime};

use crate::{AccountId, accounts::Accounts};

/// Installed by the browser-session adapter, never from client input.
#[derive(Clone, Copy, Debug)]
pub struct Viewer(pub AccountId);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Evidence {
    Complete,
    Missing,
    #[default]
    Unknown,
}

/// Only non-secret facts cross this seam. A used/revoked key or an unconfirmed
/// backup address is Missing, not Complete. A pending TOTP enrollment is Missing.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub mfa: Evidence,
    pub recovery_key: Evidence,
    pub backup_email: Evidence,
    pub password_changed_at: Option<OffsetDateTime>,
}

pub type FactsFuture<'a> = Pin<Box<dyn Future<Output = Result<Facts, sqlx::Error>> + Send + 'a>>;

/// Implement with the public APIs of the owning identity modules when available.
pub trait FactsSource: Send + Sync {
    fn load(&self, account: AccountId) -> FactsFuture<'_>;
}

/// Explicit fallback until the dependency tasks supply authoritative APIs.
#[derive(Debug)]
pub struct UnavailableFacts;
impl FactsSource for UnavailableFacts {
    fn load(&self, _: AccountId) -> FactsFuture<'_> {
        Box::pin(async { Ok(Facts::default()) })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assessment {
    /// Email, MFA, recovery key, confirmed backup email, in that order.
    pub steps: [Evidence; 4],
    /// Number of positively verified protections; never a permission decision.
    pub completed: usize,
    pub password_changed_at: Option<OffsetDateTime>,
}

pub fn assess(
    email_verified_at: Option<OffsetDateTime>,
    facts: Facts,
    now: OffsetDateTime,
) -> Result<Assessment, StatusCode> {
    if email_verified_at.is_some_and(|date| date > now)
        || facts.password_changed_at.is_some_and(|date| date > now)
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let steps = [
        if email_verified_at.is_some() {
            Evidence::Complete
        } else {
            Evidence::Missing
        },
        facts.mfa,
        facts.recovery_key,
        facts.backup_email,
    ];
    Ok(Assessment {
        completed: steps.iter().filter(|s| **s == Evidence::Complete).count(),
        steps,
        password_changed_at: facts.password_changed_at,
    })
}

#[derive(Clone)]
pub struct Wizard {
    pub pool: PgPool,
    pub enabled: bool,
    pub facts: Arc<dyn FactsSource>,
}

impl std::fmt::Debug for Wizard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wizard")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// Off unless the exact configuration value is true. Never infers dependency facts
/// from account updated_at, password hashes, pending enrollments or address presence.
pub fn configured(pool: PgPool) -> Router {
    router(Wizard {
        pool: pool.clone(),
        enabled: std::env::var("OTERYN_SECURITY_WIZARD_ENABLED").is_ok_and(|v| v == "true"),
        facts: Arc::new(RecoveryFacts(pool.clone())),
    })
}

/// Reads only the non-secret state owned by the recovery-key module.
#[derive(Debug)]
struct RecoveryFacts(PgPool);
impl FactsSource for RecoveryFacts {
    fn load(&self, account: AccountId) -> FactsFuture<'_> {
        Box::pin(async move {
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_recovery_keys WHERE account_id=$1 AND confirmed_at IS NOT NULL AND revoked_at IS NULL AND used_at IS NULL)")
                .bind(account).fetch_one(&self.0).await?;
            Ok(Facts {
                recovery_key: if active {
                    Evidence::Complete
                } else {
                    Evidence::Missing
                },
                ..Facts::default()
            })
        })
    }
}

pub fn router(wizard: Wizard) -> Router {
    Router::new()
        .route("/{lang}/account/security/wizard", get(page))
        .with_state(wizard)
}

async fn page(
    State(wizard): State<Wizard>,
    Path(lang): Path<String>,
    viewer: Option<Extension<Viewer>>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Response {
    let response = async {
        if !wizard.enabled {
            return StatusCode::NOT_FOUND.into_response();
        }
        if lang != "pl" && lang != "en" {
            return StatusCode::NOT_FOUND.into_response();
        }
        let Some(Extension(Viewer(id))) = viewer else {
            return StatusCode::UNAUTHORIZED.into_response();
        };
        let account = match (Accounts { pool: wizard.pool }).find_by_id(id).await {
            Ok(Some(account))
                if matches!(account.status.as_str(), "active" | "pending_verification") =>
            {
                account
            }
            Ok(_) => return StatusCode::FORBIDDEN.into_response(),
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        let facts = match wizard.facts.load(id).await {
            Ok(facts) => facts,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        match assess(account.email_verified_at, facts, OffsetDateTime::now_utc()) {
            Ok(assessment) => match render(&lang, &assessment, chrome.map(|value| value.0)) {
                Ok(html) => Html(html).into_response(),
                Err(status) => status.into_response(),
            },
            Err(status) => status.into_response(),
        }
    }
    .await;
    let mut response = response;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(
            "default-src 'none'; style-src 'self'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("no-referrer"),
    );
    response
}

fn render(
    lang: &str,
    assessment: &Assessment,
    chrome: Option<oteryn_ui::Chrome>,
) -> Result<String, StatusCode> {
    let source = match lang {
        "pl" => include_str!("security_wizard_pl.ftl"),
        "en" => include_str!("security_wizard_en.ftl"),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let locale = lang
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    let resource = FluentResource::try_new(source.to_owned())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let text = |id: &str| -> Result<String, StatusCode> {
        let pattern = bundle
            .get_message(id)
            .and_then(|m| m.value())
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, None, &mut errors);
        if !errors.is_empty() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Ok(escape(&value))
    };
    let title = text("title")?;
    let mut html = format!(
        "<section class=\"op-card op-stack\" data-theme=\"dark\"><h1>{title}</h1><p>{}: {}/4</p><p>{}</p><ul>",
        text("score")?,
        assessment.completed,
        text("explanation")?
    );
    for (id, state) in ["email", "mfa", "key", "backup"]
        .into_iter()
        .zip(assessment.steps)
    {
        let status = match state {
            Evidence::Complete => "complete",
            Evidence::Missing => "missing",
            Evidence::Unknown => "unknown",
        };
        if id == "key" {
            html.push_str(&format!(
                "<li><a href=\"/{lang}/account/recovery-key\">{}</a>: {}</li>",
                text(id)?,
                text(status)?
            ));
        } else {
            html.push_str(&format!("<li>{}: {}</li>", text(id)?, text(status)?));
        }
    }
    let date = assessment
        .password_changed_at
        .map(|date| date.date().to_string())
        .unwrap_or(text("unknown")?);
    html.push_str(&format!("</ul><p>{}: {date}</p><p>{}</p><p><a href=\"/{lang}/account/security\">{}</a></p></section>", text("password")?, text("password-help")?, text("manage")?));
    use askama::Template;
    WizardPage {
        chrome: chrome.unwrap_or_else(|| {
            oteryn_ui::Registry::default().chrome(
                lang,
                &format!("/{lang}/account/security/wizard"),
                oteryn_ui::Account::Anonymous,
            )
        }),
        title,
        content: html,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(askama::Template)]
#[template(
    config = "../ui/askama.toml",
    source = "{% extends \"layout.html\" %}{% block title %}{{ title }}{% endblock %}{% block content %}{{ content|safe }}{% endblock %}",
    ext = "html"
)]
struct WizardPage {
    chrome: oteryn_ui::Chrome,
    title: String,
    content: String,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
#[path = "security_wizard_tests.rs"]
mod tests;
