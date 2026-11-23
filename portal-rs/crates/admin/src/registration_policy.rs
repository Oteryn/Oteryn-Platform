use super::{Panel, StaffSession, language_for as language, text};
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, AccountStateReader, CheckContext, MutationAuthorization, Permission},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{Query, State, rejection::FormRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use oteryn_identity::{
    abuse::policy::{self, Policy, StoredPolicy},
    credentials,
};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession},
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, types::Json};
use std::sync::Arc;
use uuid::Uuid;

pub(super) const PATH: &str = "/admin/registration-policy";
pub(super) const CONFIRM_PATH: &str = "/admin/registration-policy/confirm";
const TARGET: &str = "registration_abuse_policy";
const CHANGE: &str = "registration.policy.changed";
const PERMISSIONS: &[Permission] = &[
    Permission::Access,
    Permission::ViewRegistrationPolicy,
    Permission::ManageRegistrationPolicy,
];

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("registration policy authorization refused")]
    Denied,
    #[error("recent password confirmation required")]
    ConfirmationRequired,
    #[error("registration policy has changed")]
    Stale,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Denied
            | Self::ConfirmationRequired
            | Self::Authorization(rbac::Error::Denied) => StatusCode::FORBIDDEN,
            Self::Stale => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}

#[derive(Default, Deserialize)]
pub(super) struct Selection {
    lang: Option<String>,
}
impl Selection {
    fn lang(&self, headers: &HeaderMap) -> &'static str {
        language(self.lang.as_deref(), headers)
    }
}

#[derive(Deserialize)]
pub(super) struct Settings {
    revision: i64,
    enabled: bool,
    reason_pl: String,
    reason_en: String,
    work_bits: i16,
    work_ttl_seconds: i32,
    challenge_limit: i16,
    attempt_limit: i16,
    rate_window_seconds: i32,
}

impl Settings {
    fn validate(self) -> Result<(i64, Policy), policy::InvalidPolicy> {
        if self.revision < 0 {
            return Err(policy::InvalidPolicy);
        }
        let values = Policy {
            enabled: self.enabled,
            reason_pl: self.reason_pl,
            reason_en: self.reason_en,
            work_bits: self.work_bits,
            work_ttl_seconds: self.work_ttl_seconds,
            challenge_limit: self.challenge_limit,
            attempt_limit: self.attempt_limit,
            rate_window_seconds: self.rate_window_seconds,
        };
        Ok((self.revision, values.validated()?))
    }
}

#[derive(Deserialize)]
pub(super) struct PasswordConfirmation {
    current_password: String,
}

#[derive(sqlx::FromRow)]
struct Corpus {
    complete: i64,
    fresh: i64,
    last_import: Option<String>,
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    actor: Option<Uuid>,
    changed_at: String,
    before: Json<Policy>,
    after: Json<Policy>,
}

struct History {
    actor: Option<Uuid>,
    changed_at: String,
    fields: Vec<(&'static str, String, String)>,
}

impl HistoryRow {
    fn display(self, lang: &str) -> History {
        let state = |open| {
            text(
                lang,
                if open {
                    "registration-open"
                } else {
                    "registration-closed"
                },
            )
        };
        let fields = |values: Policy| {
            [
                ("registration-state", state(values.enabled)),
                ("registration-reason-pl", values.reason_pl),
                ("registration-reason-en", values.reason_en),
                ("registration-work-bits", values.work_bits.to_string()),
                ("registration-work-ttl", values.work_ttl_seconds.to_string()),
                (
                    "registration-challenge-limit",
                    values.challenge_limit.to_string(),
                ),
                (
                    "registration-attempt-limit",
                    values.attempt_limit.to_string(),
                ),
                (
                    "registration-rate-window",
                    values.rate_window_seconds.to_string(),
                ),
            ]
        };
        let fields = fields(self.before.0)
            .into_iter()
            .zip(fields(self.after.0))
            .filter_map(|((key, before), (_, after))| {
                (before != after).then_some((key, before, after))
            })
            .collect();
        History {
            actor: self.actor,
            changed_at: self.changed_at,
            fields,
        }
    }
}

#[derive(Template)]
#[template(
    path = "admin/registration_policy.html",
    config = "src/web_askama.toml"
)]
struct Page {
    lang: &'static str,
    csrf: String,
    policy: StoredPolicy,
    corpus: Corpus,
    changes: Vec<History>,
    may_manage: bool,
    sudo: bool,
    message: Option<&'static str>,
}

impl Page {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

async fn render(
    pool: &PgPool,
    session: &BrowserSession,
    context: CheckContext,
    lang: &'static str,
    may_manage: bool,
    notice: Option<(StatusCode, &'static str)>,
) -> Result<Response, Error> {
    let policy = policy::read(pool).await?;
    let corpus = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE complete) AS complete,
         count(*) FILTER (WHERE complete AND expires_at > now()) AS fresh,
         (SELECT to_char(imported_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS \"UTC\"')
          FROM registration_corpus_import_status WHERE singleton) AS last_import
         FROM registration_password_ranges",
    )
    .fetch_one(pool)
    .await?;
    let changes: Vec<HistoryRow> = sqlx::query_as(
        "SELECT actor,to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS \"UTC\"') AS changed_at,
         before,after FROM audit_log
         WHERE action='registration.policy.changed' AND target='registration_abuse_policy'
         ORDER BY created_at DESC,id DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;
    let sudo = if let (true, Some(proof)) = (may_manage, context.session_proof) {
        credentials::sudo_account(pool, &proof.session_hash).await? == Some(context.account)
    } else {
        false
    };
    let mut response = oteryn_web::render(&Page {
        lang,
        csrf: session.csrf_token().await?,
        policy,
        corpus,
        changes: changes.into_iter().map(|row| row.display(lang)).collect(),
        may_manage,
        sudo,
        message: notice.map(|(_, key)| key),
    });
    if response.status().is_success()
        && let Some((status, _)) = notice
    {
        *response.status_mut() = status;
    }
    Ok(response)
}

pub(super) async fn show<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(selection): Query<Selection>,
) -> Result<Response, Error> {
    let may_manage = match s
        .auth
        .check(context, Permission::ManageRegistrationPolicy)
        .await
    {
        Ok(()) => true,
        Err(rbac::Error::Denied) => false,
        Err(error) => return Err(error.into()),
    };
    render(
        &s.pool,
        &session,
        context,
        selection.lang(&headers),
        may_manage,
        None,
    )
    .await
}

pub(super) async fn confirm<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    Extension(session): Extension<BrowserSession>,
    (Extension(limiter), ClientSource(peer)): (Extension<RateLimiter>, ClientSource),
    (headers, Query(selection)): (HeaderMap, Query<Selection>),
    Form(form): Form<PasswordConfirmation>,
) -> Result<Response, Error> {
    let lang = selection.lang(&headers);
    let mut keys = Keys::source(peer);
    keys.account = Some(context.account.to_string());
    if let Err(denial) = limiter
        .check(NamedPolicy::IdentitySecurityMutation, &[keys])
        .await
    {
        return Ok(denial.response(
            if lang == "en" {
                Language::En
            } else {
                Language::Pl
            },
            false,
        ));
    }
    let proof = context.session_proof.ok_or(Error::Denied)?;
    let mut tx = s.pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    let Some((actor, hash)) = credentials::authenticated(&mut tx, &proof.session_hash).await?
    else {
        return Err(Error::Denied);
    };
    if actor != context.account {
        return Err(Error::Denied);
    }
    let confirmed = credentials::verify(form.current_password, hash).await?;
    rbac::authorize_mutation(&mut tx, &[], &authority, context, PERMISSIONS).await?;
    if !confirmed {
        tx.rollback().await?;
        return render(
            &s.pool,
            &session,
            context,
            lang,
            true,
            Some((
                StatusCode::UNPROCESSABLE_ENTITY,
                "registration-confirmation-refused",
            )),
        )
        .await;
    }
    let until = credentials::record_sudo_confirmation(&mut tx, actor).await?;
    let rotation = session::rotate_in_transaction(&session, &mut tx, until).await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action: "registration.policy.confirmed",
            target: TARGET,
            before: json!(null),
            after: json!({"confirmed": true}),
        },
    )
    .await?;
    tx.commit().await?;
    session::apply_rotation(&session, rotation).await;
    Ok(Redirect::to(&format!("{PATH}?lang={lang}")).into_response())
}

pub(super) async fn save<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(selection): Query<Selection>,
    form: Result<Form<Settings>, FormRejection>,
) -> Result<Response, Error> {
    let lang = selection.lang(&headers);
    let validated = match form {
        Ok(Form(settings)) => settings.validate(),
        Err(error) if error.status() == StatusCode::UNPROCESSABLE_ENTITY => {
            Err(policy::InvalidPolicy)
        }
        Err(error) => return Ok(error.status().into_response()),
    };
    let Ok((revision, policy)) = validated else {
        return render(
            &s.pool,
            &session,
            context,
            lang,
            true,
            Some((StatusCode::UNPROCESSABLE_ENTITY, "registration-invalid")),
        )
        .await;
    };
    match change(&s.pool, &authority, context, revision, policy).await {
        Ok(()) => Ok(Redirect::to(&format!("{PATH}?lang={lang}")).into_response()),
        Err(Error::Stale) => {
            render(
                &s.pool,
                &session,
                context,
                lang,
                true,
                Some((StatusCode::CONFLICT, "registration-stale")),
            )
            .await
        }
        Err(error) => Err(error),
    }
}

async fn change(
    pool: &PgPool,
    authority: &MutationAuthorization,
    context: CheckContext,
    revision: i64,
    values: Policy,
) -> Result<(), Error> {
    let proof = context.session_proof.ok_or(Error::Denied)?;
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    rbac::authorize_mutation_with_resources(
        &mut tx,
        &[],
        |connection| {
            Box::pin(async {
                sqlx::query(
                    "SELECT singleton FROM registration_abuse_policy WHERE singleton FOR UPDATE",
                )
                .fetch_one(connection)
                .await?;
                Ok(())
            })
        },
        authority,
        context,
        PERMISSIONS,
    )
    .await?;
    if credentials::sudo_account(&mut *tx, &proof.session_hash).await? != Some(context.account) {
        return Err(Error::ConfirmationRequired);
    }
    let before = policy::read(&mut *tx).await?;
    if revision != before.revision {
        return Err(Error::Stale);
    }
    if values != before.values {
        policy::update(&mut tx, &values).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: CHANGE,
                target: TARGET,
                before: json!(before.values),
                after: json!(values),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "registration_policy_tests.rs"]
mod tests;
