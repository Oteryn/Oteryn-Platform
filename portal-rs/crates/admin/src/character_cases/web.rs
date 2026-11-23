use super::{Error, Operation, REVIEW, Review, Submission, decide, load, submit};
use crate::{
    rbac::{CheckContext, MutationAuthorization},
    web::{self, Panel, StaffSession},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        Query, State,
        rejection::{FormRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_characters::commands::{Sex, Vocation};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

const PATH: &str = "/admin/character-cases";
const PAGE_SIZE: usize = 30;
pub(super) const WRITES_PER_MINUTE: i64 = 30;

pub(crate) fn routes<I: StaffSession, A: crate::rbac::AccountStateReader + 'static>()
-> Router<Arc<Panel<I, A>>> {
    Router::new()
        .route(PATH, get(page::<I, A>).post(create::<I, A>))
        .route("/admin/character-cases/review", post(review::<I, A>))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    id: Option<Uuid>,
    after: Option<Uuid>,
    operation: Option<Operation>,
    ticket: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseId {
    id: Uuid,
}

#[derive(sqlx::FromRow)]
struct ListEntry {
    id: Uuid,
    ticket_id: Uuid,
    operation: String,
    state: String,
}

#[derive(Template)]
#[template(path = "admin/character_cases.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    texts: fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    view: View,
    unavailable: String,
}

enum View {
    Queue {
        entries: Vec<ListEntry>,
        ticket: Option<Uuid>,
        can_submit: bool,
        next: Option<Uuid>,
    },
    Intake {
        operation: Operation,
        ticket: Option<Uuid>,
        id: Uuid,
        csrf: String,
    },
    Detail {
        case: Box<super::Case>,
        review_csrf: Option<String>,
    },
    Failure {
        key: &'static str,
    },
}

impl Page {
    fn text(&self, key: &str) -> String {
        web::message(&self.texts, key)
    }

    fn label(&self, key: &str) -> String {
        self.text(&format!("character-case-{key}"))
    }

    fn operations(&self) -> [Operation; 5] {
        Operation::ALL
    }

    fn sexes(&self) -> [Sex; 2] {
        Sex::ALL
    }

    fn vocations(&self) -> [Vocation; 5] {
        Vocation::ALL
    }
}

fn render(lang: &'static str, view: View) -> Response {
    let Some(texts) = web::messages(lang) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    oteryn_web::render(&Page {
        lang,
        texts,
        view,
        unavailable: crate::console::access::tr(lang, "unavailable"),
    })
}

fn failure(lang: &'static str, error: Error) -> Response {
    let (status, key) = match error {
        Error::Invalid => (StatusCode::BAD_REQUEST, "invalid"),
        Error::Denied => (StatusCode::FORBIDDEN, "denied"),
        Error::Missing => (StatusCode::NOT_FOUND, "missing"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::Limited(retry_after) => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                render(lang, View::Failure { key: "limited" }),
            )
                .into_response();
            if let Ok(delay) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, delay);
            }
            return response;
        }
        Error::Storage(_) => (StatusCode::SERVICE_UNAVAILABLE, "storage"),
    };
    // Preserve an SSR return path even when a case changes between view and submit.
    (status, render(lang, View::Failure { key })).into_response()
}

async fn page<I: StaffSession, A: crate::rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    filter: Result<Query<Filters>, QueryRejection>,
) -> Response {
    let lang = web::language(&headers);
    let Ok(Query(filter)) = filter else {
        return failure(lang, Error::Invalid);
    };
    match view(&panel, actor, session, filter).await {
        Ok(view) => render(lang, view),
        Err(error) => failure(lang, error),
    }
}

async fn view<I, A: crate::rbac::AccountStateReader>(
    panel: &Panel<I, A>,
    actor: CheckContext,
    session: BrowserSession,
    filter: Filters,
) -> Result<View, Error> {
    let ticket = filter.ticket;
    match (filter.id, filter.operation, filter.after, ticket) {
        (Some(id), None, None, None) if !id.is_nil() => {
            let case = load(&panel.pool, actor.account, id).await?;
            let review_csrf = if case.reviewer == actor.account {
                if !can_review(panel, actor).await? {
                    return Err(Error::Missing);
                }
                if case.decision.is_none() && open_ticket(&panel.pool, Some(case.ticket_id)).await?
                {
                    Some(session.csrf_token().await?)
                } else {
                    None
                }
            } else {
                None
            };
            Ok(View::Detail {
                case: Box::new(case),
                review_csrf,
            })
        }
        (None, Some(operation), None, _) => {
            if !open_ticket(&panel.pool, ticket).await? {
                return Err(Error::Conflict);
            }
            Ok(View::Intake {
                operation,
                ticket,
                id: Uuid::now_v7(),
                csrf: session.csrf_token().await?,
            })
        }
        (None, None, after, _) => {
            let can_submit = open_ticket(&panel.pool, ticket).await?;
            let mut entries: Vec<ListEntry> = sqlx::query_as(
                "SELECT id, ticket_id, operation, COALESCE(decision, 'pending') AS state
                 FROM staff_character_cases
                 WHERE (requester=$1 OR ($3 AND reviewer=$1)) AND ($2::uuid IS NULL OR id<$2)
                   AND ($4::uuid IS NULL OR ticket_id=$4)
                 ORDER BY id DESC LIMIT $5",
            )
            .bind(actor.account)
            .bind(after)
            .bind(can_review(panel, actor).await?)
            .bind(ticket)
            .bind((PAGE_SIZE + 1) as i64)
            .fetch_all(&panel.pool)
            .await?;
            let next = if entries.len() > PAGE_SIZE {
                entries.truncate(PAGE_SIZE);
                entries.last().map(|entry| entry.id)
            } else {
                None
            };
            Ok(View::Queue {
                entries,
                ticket,
                can_submit,
                next,
            })
        }
        _ => Err(Error::Invalid),
    }
}

async fn open_ticket(pool: &PgPool, ticket: Option<Uuid>) -> Result<bool, Error> {
    match ticket {
        Some(id) if id.is_nil() => Err(Error::Invalid),
        Some(id) => {
            let (_, status) = super::ticket_details(pool, id).await?;
            Ok(super::ticket_is_open(&status))
        }
        None => Ok(true),
    }
}

pub(super) async fn charge_write(pool: &PgPool, actor: Uuid) -> Result<(), Error> {
    let (count, retry_after): (i64, i64) = sqlx::query_as(
        "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         VALUES('staff-character-case-write',sha256(uuid_send($1)),date_trunc('minute',statement_timestamp()),1)
         ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE
         SET count=least(web_rate_limits.count+1,$2+1)
         RETURNING count,greatest(1,ceil(extract(epoch FROM
           window_start + interval '1 minute' - statement_timestamp())))::bigint",
    )
    .bind(actor)
    .bind(WRITES_PER_MINUTE)
    .fetch_one(pool)
    .await?;
    if count > WRITES_PER_MINUTE {
        return Err(Error::Limited(retry_after));
    }
    Ok(())
}

async fn can_review<I, A: crate::rbac::AccountStateReader>(
    panel: &Panel<I, A>,
    actor: CheckContext,
) -> Result<bool, Error> {
    match panel.auth.check(actor, REVIEW).await {
        Ok(()) => Ok(true),
        Err(crate::rbac::Error::Denied) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn create<I: StaffSession, A: crate::rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    headers: HeaderMap,
    form: Result<Form<Submission>, FormRejection>,
) -> Response {
    let lang = web::language(&headers);
    if let Err(error) = charge_write(&panel.pool, actor.account).await {
        return failure(lang, error);
    }
    let Ok(Form(form)) = form else {
        return failure(lang, Error::Invalid);
    };
    match submit(&panel.pool, &panel.auth, &authority, actor, form).await {
        Ok(id) => Redirect::to(&format!("{PATH}?id={id}")).into_response(),
        Err(error) => failure(lang, error),
    }
}

async fn review<I: StaffSession, A: crate::rbac::AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    headers: HeaderMap,
    case: Result<Query<CaseId>, QueryRejection>,
    form: Result<Form<Review>, FormRejection>,
) -> Response {
    let lang = web::language(&headers);
    if let Err(error) = charge_write(&panel.pool, actor.account).await {
        return failure(lang, error);
    }
    let Ok(Form(form)) = form else {
        return failure(lang, Error::Invalid);
    };
    let Ok(Query(case)) = case else {
        return failure(lang, Error::Invalid);
    };
    match decide(&panel.pool, &panel.auth, &authority, actor, case.id, form).await {
        Ok(()) => Redirect::to(&format!("{PATH}?id={}", case.id)).into_response(),
        Err(error) => failure(lang, error),
    }
}
