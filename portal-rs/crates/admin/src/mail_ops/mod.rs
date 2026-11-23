//! Staff operations for the delivery worker's mail jobs.
mod store;

use crate::{
    rbac::{self, CheckContext, MutationAuthorization, Permission},
    web::{Panel, StaffSession, language, message, messages, text},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use oteryn_web::session::BrowserSession;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

const PATH: &str = "/admin/mail-outbox";
const STATES: &[&str] = &["pending", "running", "sent", "failed", "dead", "cancelled"];
const KINDS: &[&str] = &[
    "email.account_exists",
    "email.verify",
    "email.password-reset",
    "notify.new-device",
    "notify.password-changed",
    "notify.news",
    "notify.support",
    "notify.security",
    "notify.email",
];

fn state_key(state: &str) -> &str {
    match state {
        "pending" => "mail-pending",
        "running" => "mail-running",
        "sent" => "mail-sent",
        "failed" => "mail-failed",
        "dead" => "mail-dead",
        "cancelled" => "mail-cancelled",
        _ => "mail-unavailable",
    }
}

fn kind_key(kind: &str) -> &str {
    match kind {
        "email.account_exists" => "mail-account-exists",
        "email.verify" => "mail-verify",
        "email.password-reset" => "mail-reset",
        "notify.new-device" => "mail-new-device",
        "notify.password-changed" => "mail-password-changed",
        "notify.news" => "mail-news",
        "notify.support" => "mail-support",
        "notify.security" => "mail-security",
        _ => "mail-notification",
    }
}

fn error_key(error: &str) -> &str {
    match error {
        "email delivery failed" | "verification delivery failed" | "reset delivery failed" => {
            "mail-error-delivery"
        }
        "invalid verification payload"
        | "invalid account-exists payload"
        | "invalid reset payload"
        | "invalid reset ciphertext" => "mail-error-payload",
        "invalid verification locale"
        | "invalid account-exists locale"
        | "invalid reset locale" => "mail-error-locale",
        "invalid verification recipient"
        | "invalid account-exists recipient"
        | "invalid reset recipient" => "mail-error-recipient",
        "verification message unavailable"
        | "account-exists message unavailable"
        | "invalid reset message" => "mail-error-message",
        "verification delivery unavailable"
        | "account-exists delivery unavailable"
        | "reset delivery unavailable" => "mail-error-unavailable",
        "reset audit unavailable" => "mail-error-audit",
        "lease expired on final attempt" => "mail-error-lease",
        _ => "mail-error-recorded",
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid mail outbox request")]
    Invalid,
    #[error("no eligible mail jobs")]
    Conflict,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

impl Error {
    fn response(&self, lang: &str) -> Response {
        let (status, key) = match self {
            Self::Invalid | Self::Authorization(rbac::Error::InvalidInput) => {
                (StatusCode::BAD_REQUEST, "mail-invalid")
            }
            Self::Conflict => (StatusCode::CONFLICT, "mail-conflict"),
            Self::Authorization(rbac::Error::Denied) => (StatusCode::FORBIDDEN, "mail-denied"),
            _ => (StatusCode::SERVICE_UNAVAILABLE, "mail-unavailable"),
        };
        (status, text(lang, key)).into_response()
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Filters {
    #[serde(skip_serializing_if = "String::is_empty")]
    state: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    from: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    to: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recipient: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<u8>,
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes.iter().enumerate().any(|(i, byte)| {
            if matches!(i, 4 | 7) {
                *byte != b'-'
            } else {
                !byte.is_ascii_digit()
            }
        })
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[..4].parse::<i32>(),
        value[5..7].parse::<u8>(),
        value[8..].parse::<u8>(),
    ) else {
        return false;
    };
    year > 0
        && time::Month::try_from(month)
            .is_ok_and(|month| time::Date::from_calendar_date(year, month, day).is_ok())
}

impl Filters {
    fn validate(&mut self) -> Result<(), Error> {
        self.recipient = self.recipient.trim().to_ascii_lowercase();
        if (!self.state.is_empty() && !STATES.contains(&self.state.as_str()))
            || (!self.kind.is_empty() && !KINDS.contains(&self.kind.as_str()))
            || (!self.from.is_empty() && !valid_date(&self.from))
            || (!self.to.is_empty() && !valid_date(&self.to))
            || (!self.from.is_empty() && !self.to.is_empty() && self.from > self.to)
            || (!self.recipient.is_empty() && !oteryn_identity::reset::valid_email(&self.recipient))
            || self.after.is_some_and(|id| id.is_nil())
            || self
                .changed
                .is_some_and(|count| !(1..=store::BATCH_LIMIT).contains(&count))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }

    fn query(&self) -> Result<String, Error> {
        serde_urlencoded::to_string(self).map_err(|_| Error::Invalid)
    }
}

#[derive(Template)]
#[template(path = "admin/mail_outbox.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    labels: fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    filters: Filters,
    jobs: Vec<store::Job>,
    counts: Vec<store::Count>,
    next: Option<String>,
    return_query: String,
    csrf: String,
    retry: bool,
    retry_batch: bool,
    cancel: bool,
}

impl Page {
    fn tr(&self, key: &str) -> String {
        message(&self.labels, key)
    }

    fn state_label(&self, state: &str) -> String {
        self.tr(state_key(state))
    }

    fn kind_label(&self, kind: &str) -> String {
        self.tr(kind_key(kind))
    }

    fn error_label(&self, error: &str) -> String {
        self.tr(error_key(error))
    }

    fn states(&self) -> &'static [&'static str] {
        STATES
    }

    fn kinds(&self) -> &'static [&'static str] {
        KINDS
    }

    fn batch_limit(&self) -> u8 {
        store::BATCH_LIMIT
    }
}

async fn has<A: rbac::AccountStateReader>(
    auth: &rbac::Authorizer<A>,
    actor: CheckContext,
    permission: Permission,
) -> Result<bool, Error> {
    match auth.check(actor, permission).await {
        Ok(()) => Ok(true),
        Err(rbac::Error::Denied) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn page<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(mut filters): Query<Filters>,
) -> Response {
    let lang = language(&headers);
    let result = async {
        filters.validate()?;
        let reveal = has(&panel.auth, actor, Permission::MailRecipient).await?;
        let retry = has(&panel.auth, actor, Permission::MailRetry).await?;
        let retry_batch = has(&panel.auth, actor, Permission::MailRetryBatch).await?;
        let cancel = has(&panel.auth, actor, Permission::MailCancel).await?;
        let mut jobs = store::list(&panel.pool, &filters, reveal).await?;
        let counts = store::counts(&panel.pool).await?;
        let changed = filters.changed.take();
        let after = filters.after.take();
        let return_query = filters.query()?;
        filters.after = after;
        let next = if jobs.len() > store::PAGE_SIZE {
            jobs.pop();
            filters.after = jobs.last().map(|job| job.id);
            Some(format!("{PATH}?{}", filters.query()?))
        } else {
            None
        };
        filters.changed = changed;
        let csrf = session.csrf_token().await?;
        Ok(Page {
            lang,
            labels: messages(lang).ok_or(Error::Invalid)?,
            filters,
            jobs,
            counts,
            next,
            return_query,
            csrf,
            retry,
            retry_batch,
            cancel,
        })
    }
    .await;
    match result {
        Ok(page) => oteryn_web::render(&page),
        Err(error) => Error::response(&error, lang),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobForm {
    id: Uuid,
    #[serde(default)]
    filters: String,
    #[serde(default)]
    _csrf: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BatchForm {
    kind: String,
    #[serde(default)]
    filters: String,
    #[serde(default)]
    _csrf: Option<String>,
}

pub(super) async fn retry<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Form(form): Form<JobForm>,
) -> Response {
    change(
        &panel.pool,
        actor,
        &auth,
        store::Action::Retry(form.id),
        &form.filters,
        &headers,
    )
    .await
}

pub(super) async fn cancel<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Form(form): Form<JobForm>,
) -> Response {
    change(
        &panel.pool,
        actor,
        &auth,
        store::Action::Cancel(form.id),
        &form.filters,
        &headers,
    )
    .await
}

pub(super) async fn retry_kind<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Form(form): Form<BatchForm>,
) -> Response {
    change(
        &panel.pool,
        actor,
        &auth,
        store::Action::RetryKind(&form.kind),
        &form.filters,
        &headers,
    )
    .await
}

async fn change(
    pool: &sqlx::PgPool,
    actor: CheckContext,
    auth: &MutationAuthorization,
    action: store::Action<'_>,
    filters: &str,
    headers: &HeaderMap,
) -> Response {
    let result = async {
        let mut filters =
            serde_urlencoded::from_str::<Filters>(filters).map_err(|_| Error::Invalid)?;
        filters.validate()?;
        filters.changed = Some(store::apply(pool, actor, auth, action).await?);
        Ok(Redirect::to(&format!("{PATH}?{}", filters.query()?)))
    }
    .await;
    match result {
        Ok(redirect) => redirect.into_response(),
        Err(error) => Error::response(&error, language(headers)),
    }
}

#[cfg(test)]
mod tests;
