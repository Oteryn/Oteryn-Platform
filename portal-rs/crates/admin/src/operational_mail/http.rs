use super::{AUDIENCE_LIMIT, Audience, Campaign, Decision, Draft, Error, preview, review};
use crate::{
    rbac::{self, CheckContext, MutationAuthorization, Permission},
    web::{Panel, StaffSession, language, message, messages, text},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::FormRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_notify::{Locale, SmtpSender, Unsubscribe, operational};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const ROOT: &str = "/admin/operational-mail";

pub(crate) fn routes<I: StaffSession, A: rbac::AccountStateReader + 'static>()
-> axum::Router<Arc<Panel<I, A>>> {
    axum::Router::new()
        .route(
            "/admin/operational-mail",
            get(index::<I, A>).post(compose::<I, A>),
        )
        .route(
            "/admin/operational-mail/{id}",
            get(detail::<I, A>).post(decide::<I, A>),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
}

impl Error {
    fn response(&self, lang: &str) -> Response {
        let (status, key) = match self {
            Self::Invalid => (StatusCode::BAD_REQUEST, "operational-mail-invalid"),
            Self::Missing => (StatusCode::NOT_FOUND, "operational-mail-missing"),
            Self::Conflict => (StatusCode::CONFLICT, "operational-mail-conflict"),
            Self::Empty => (StatusCode::CONFLICT, "operational-mail-empty"),
            Self::AudienceLimit => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "operational-mail-audience-limit",
            ),
            Self::RateLimit => (StatusCode::TOO_MANY_REQUESTS, "operational-mail-rate-limit"),
            Self::Authorization(rbac::Error::Denied) => {
                (StatusCode::FORBIDDEN, "operational-mail-denied")
            }
            _ => (
                StatusCode::SERVICE_UNAVAILABLE,
                "operational-mail-unavailable",
            ),
        };
        (status, text(lang, key)).into_response()
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

pub(crate) async fn visible<A: rbac::AccountStateReader>(
    auth: &rbac::Authorizer<A>,
    actor: CheckContext,
) -> Result<bool, Error> {
    Ok(has(auth, actor, Permission::ComposeOperationalMail).await?
        || has(auth, actor, Permission::ReviewOperationalMail).await?)
}

fn configured_delivery() -> Option<Unsubscribe> {
    let unsubscribe = Unsubscribe::configured().ok()?;
    (unsubscribe.operational_ready() && SmtpSender::configured().is_ok()).then_some(unsubscribe)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComposeForm {
    _csrf: String,
    request_id: Uuid,
    audience: Audience,
    subject_pl: String,
    body_pl: String,
    subject_en: String,
    body_en: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewForm {
    _csrf: String,
    decision: Decision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    before: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct Summary {
    id: Uuid,
    audience: String,
    subject_pl: String,
    subject_en: String,
    status: String,
}

#[derive(Default, sqlx::FromRow)]
struct Counts {
    eligible_pl: i64,
    eligible_en: i64,
    delivered: i64,
    skipped: i64,
    waiting: i64,
    failed: i64,
}

#[derive(Template)]
#[template(path = "admin/operational_mail.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    labels: fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    csrf: String,
    compose: bool,
    ready: bool,
    can_review: bool,
    request_id: Uuid,
    campaigns: Vec<Summary>,
    next: Option<Uuid>,
    campaign: Option<Campaign>,
    counts: Counts,
    footer_pl: String,
    footer_en: String,
}

impl Page {
    fn tr(&self, key: &str) -> String {
        message(&self.labels, key)
    }

    fn audience(&self, audience: &str) -> String {
        self.tr(&format!("operational-mail-audience-{audience}"))
    }

    fn status(&self, status: &str) -> String {
        self.tr(&format!("operational-mail-status-{status}"))
    }

    fn subject_limit(&self) -> usize {
        operational::SUBJECT_LIMIT
    }

    fn body_limit(&self) -> usize {
        operational::BODY_LIMIT
    }

    fn audience_limit(&self) -> i64 {
        AUDIENCE_LIMIT
    }
}

struct Access {
    compose: bool,
    review: bool,
}

async fn access<A: rbac::AccountStateReader>(
    auth: &rbac::Authorizer<A>,
    actor: CheckContext,
) -> Result<Access, Error> {
    let compose = has(auth, actor, Permission::ComposeOperationalMail).await?;
    let review = has(auth, actor, Permission::ReviewOperationalMail).await?;
    if !compose && !review {
        return Err(rbac::Error::Denied.into());
    }
    Ok(Access { compose, review })
}

async fn page(session: &BrowserSession, lang: &'static str, access: Access) -> Result<Page, Error> {
    let csrf = session.csrf_token().await?;
    Ok(Page {
        lang,
        labels: messages(lang).ok_or(Error::Unavailable)?,
        csrf,
        compose: access.compose,
        ready: configured_delivery().is_some(),
        can_review: access.review,
        request_id: Uuid::now_v7(),
        campaigns: vec![],
        next: None,
        campaign: None,
        counts: Counts::default(),
        footer_pl: operational::unsubscribe_label(Locale::Pl).map_err(|_| Error::Unavailable)?,
        footer_en: operational::unsubscribe_label(Locale::En).map_err(|_| Error::Unavailable)?,
    })
}

async fn index<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let lang = language(&headers);
    let rendered = async {
        let access = access(&panel.auth, actor).await?;
        let Query(query) = query.map_err(|_| Error::Invalid)?;
        let campaigns = sqlx::query_as(
            "SELECT id,audience,subject_pl,subject_en,
                coalesce(decision,CASE WHEN expires_at<=clock_timestamp() THEN 'expired' ELSE 'pending' END) AS status
             FROM operational_mail_campaigns
             WHERE ($1 OR author=$2) AND ($3::uuid IS NULL OR id<$3)
             ORDER BY id DESC LIMIT 51",
        )
        .bind(access.review).bind(actor.account).bind(query.before)
        .fetch_all(&panel.pool).await?;
        let mut page = page(&session, lang, access).await?;
        page.campaigns = campaigns;
        if page.campaigns.len() > 50 {
            page.campaigns.pop();
            page.next = page.campaigns.last().map(|summary| summary.id);
        }
        Ok(oteryn_web::render(&page))
    }.await;
    rendered.unwrap_or_else(|error: Error| error.response(lang))
}

async fn detail<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    let lang = language(&headers);
    let rendered = async {
        let access = access(&panel.auth, actor).await?;
        let campaign: Campaign = sqlx::query_as(
            "SELECT id,author,audience,subject_pl,body_pl,subject_en,body_en,
                coalesce(decision,CASE WHEN expires_at<=clock_timestamp() THEN 'expired' ELSE 'pending' END) AS status
             FROM operational_mail_campaigns WHERE id=$1 AND ($2 OR author=$3)",
        )
        .bind(id).bind(access.review).bind(actor.account)
        .fetch_optional(&panel.pool).await?.ok_or(Error::Missing)?;
        let counts = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE r.locale='pl') AS eligible_pl,
                    count(*) FILTER (WHERE r.locale='en') AS eligible_en,
                    count(e.delivered_at) AS delivered,
                    count(*) FILTER (WHERE r.mail_id IS NULL OR r.skipped_at IS NOT NULL) AS skipped,
                    count(*) FILTER (WHERE o.state IN ('pending','running')
                         AND e.delivered_at IS NULL AND r.skipped_at IS NULL) AS waiting,
                    count(*) FILTER (WHERE r.mail_id IS NOT NULL AND e.delivered_at IS NULL
                         AND r.skipped_at IS NULL AND (o.state IN ('done','dead') OR o.id IS NULL)) AS failed
             FROM operational_mail_recipients r
             LEFT JOIN email_outbox e ON e.id=r.mail_id
             LEFT JOIN outbox o ON o.id=r.job_id WHERE r.campaign_id=$1",
        )
        .bind(id).fetch_one(&panel.pool).await?;
        let mut page = page(&session, lang, access).await?;
        page.counts = counts;
        page.can_review &= campaign.author != actor.account && campaign.status == "pending";
        page.campaign = Some(campaign);
        Ok(oteryn_web::render(&page))
    }.await;
    rendered.unwrap_or_else(|error: Error| error.response(lang))
}

async fn compose<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    form: Result<Form<ComposeForm>, FormRejection>,
) -> Response {
    let lang = language(&headers);
    let Ok(Form(form)) = form else {
        return Error::Invalid.response(lang);
    };
    let draft = Draft {
        audience: form.audience,
        subject_pl: form.subject_pl,
        body_pl: form.body_pl,
        subject_en: form.subject_en,
        body_en: form.body_en,
    };
    mutation_result(
        preview(&panel.pool, &auth, actor, form.request_id, &draft).await,
        form.request_id,
        lang,
    )
}

async fn decide<I: StaffSession, A: rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    form: Result<Form<ReviewForm>, FormRejection>,
) -> Response {
    let lang = language(&headers);
    let Ok(Form(form)) = form else {
        return Error::Invalid.response(lang);
    };
    let delivery = match form.decision {
        Decision::Approve => configured_delivery(),
        Decision::Reject => None,
    };
    mutation_result(
        review(
            &panel.pool,
            &auth,
            actor,
            id,
            form.decision,
            delivery.as_ref(),
        )
        .await,
        id,
        lang,
    )
}

fn mutation_result(result: Result<(), Error>, id: Uuid, lang: &str) -> Response {
    match result {
        Ok(()) => Redirect::to(&format!("{ROOT}/{id}")).into_response(),
        Err(error) => error.response(lang),
    }
}
