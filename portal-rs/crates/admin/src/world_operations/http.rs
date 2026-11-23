use super::{
    Error,
    model::{Action, DraftForm, display_time},
    store,
};
use crate::{
    rbac::{AccountStateReader, CheckContext},
    web::{self, Panel, StaffSession},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::FormRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

const PATH: &str = "/admin/world-operations";

pub(crate) fn routes<I: StaffSession, A: AccountStateReader + 'static>()
-> axum::Router<Arc<Panel<I, A>>> {
    axum::Router::new()
        .route(
            "/admin/world-operations",
            get(index::<I, A>).post(create::<I, A>),
        )
        .route(
            "/admin/world-operations/{id}",
            get(detail::<I, A>).post(review::<I, A>),
        )
        .layer(DefaultBodyLimit::max(8192))
}

pub(crate) async fn has_scope(pool: &PgPool, account: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM world_operation_planning_grants WHERE account_id=$1)",
    )
    .bind(account)
    .fetch_one(pool)
    .await
}

struct Locale {
    lang: &'static str,
    bundle: FluentBundle<FluentResource>,
}

impl Locale {
    fn new(headers: &HeaderMap) -> Result<Self, Error> {
        let lang = web::language(headers);
        let bundle = web::messages(lang).ok_or(Error::Unavailable)?;
        Ok(Self { lang, bundle })
    }

    fn text(&self, key: &str) -> String {
        web::message(&self.bundle, key)
    }

    fn action(&self, action: &str) -> String {
        self.text(&format!("ops-action-{action}"))
    }

    fn unavailable(&self) -> String {
        crate::console::text(self.lang, "unavailable")
    }
}

#[derive(Deserialize)]
struct Filters {
    scope: Option<String>,
    after: Option<Uuid>,
}

#[derive(Template)]
#[template(
    path = "admin/world_operations/index.html",
    config = "src/web_askama.toml"
)]
struct Index {
    l: Locale,
    scopes: Vec<store::Scope>,
    plans: Vec<ListEntry>,
    draft: Option<Draft>,
    next: Option<Uuid>,
    selected: Option<String>,
}

struct ListEntry {
    id: Uuid,
    world: String,
    action: String,
    status: String,
    starts: String,
}

struct Draft {
    csrf: String,
    id: Uuid,
    world: Uuid,
    action: Action,
}

#[derive(Template)]
#[template(
    path = "admin/world_operations/detail.html",
    config = "src/web_askama.toml"
)]
struct Detail {
    l: Locale,
    plan: store::Plan,
    action: Action,
    status: &'static str,
    starts: String,
    ends: String,
    reviewed_at: Option<String>,
    csrf: Option<String>,
}

#[derive(Template)]
#[template(
    path = "admin/world_operations/error.html",
    config = "src/web_askama.toml"
)]
struct Problem {
    l: Locale,
    message: &'static str,
}

fn response(headers: &HeaderMap, result: Result<Response, Error>) -> Response {
    match result {
        Ok(response) => response,
        Err(error) => {
            let (status, message) = match error {
                Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "ops-invalid"),
                Error::Denied => (StatusCode::FORBIDDEN, "ops-denied"),
                Error::NotFound => (StatusCode::NOT_FOUND, "ops-not-found"),
                Error::Conflict => (StatusCode::CONFLICT, "ops-conflict"),
                Error::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "ops-unavailable"),
            };
            let Ok(l) = Locale::new(headers) else {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            };
            let mut page = oteryn_web::render(&Problem { l, message });
            if page.status().is_success() {
                *page.status_mut() = status;
            }
            page
        }
    }
}

async fn index<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(filters): Query<Filters>,
) -> Response {
    let result = async {
        let scopes = store::scopes(&s.pool, context.account).await?;
        if scopes.is_empty() {
            return Err(Error::Denied);
        }
        let draft = if let Some(selection) = &filters.scope {
            let (world, action) = selection.split_once('/').ok_or(Error::Invalid)?;
            let world = world.parse().map_err(|_| Error::Invalid)?;
            let action = Action::from_key(action)?;
            if !scopes
                .iter()
                .any(|scope| scope.world == world && scope.action == action.key())
            {
                return Err(Error::Denied);
            }
            Some(Draft {
                csrf: session.csrf_token().await.map_err(|_| Error::Unavailable)?,
                id: Uuid::now_v7(),
                world,
                action,
            })
        } else {
            None
        };
        let rows = store::plans(&s.pool, context.account, filters.after).await?;
        let next = (rows.len() > store::PAGE_SIZE).then(|| rows[store::PAGE_SIZE - 1].id);
        let now = store::now(&mut *s.pool.acquire().await?).await?;
        let l = Locale::new(&headers)?;
        let plans = rows
            .into_iter()
            .take(store::PAGE_SIZE)
            .map(|plan| {
                let status = l.text(plan.status(now));
                Ok(ListEntry {
                    id: plan.id,
                    world: plan.world_name,
                    action: l.action(&plan.action),
                    status,
                    starts: display_time(plan.starts)?,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(oteryn_web::render(&Index {
            l,
            scopes,
            plans,
            draft,
            next,
            selected: filters.scope,
        }))
    }
    .await;
    response(&headers, result)
}

async fn create<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    headers: HeaderMap,
    form: Result<Form<DraftForm>, FormRejection>,
) -> Response {
    let result = async {
        let Form(form) = form.map_err(|_| Error::Invalid)?;
        let id = store::create(&s.pool, &s.auth, context, form).await?;
        Ok(Redirect::to(&format!("{PATH}/{id}")).into_response())
    }
    .await;
    response(&headers, result)
}

async fn detail<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let result = async {
        let plan = store::plan(&s.pool, context.account, id).await?;
        let action = Action::from_key(&plan.action).map_err(|_| Error::Unavailable)?;
        let now = store::now(&mut *s.pool.acquire().await?).await?;
        let csrf = if plan.can_review
            && super::model::check_review(
                plan.requester,
                context.account,
                plan.starts,
                now,
                plan.reviewer.is_some(),
            )
            .is_ok()
        {
            Some(session.csrf_token().await.map_err(|_| Error::Unavailable)?)
        } else {
            None
        };
        Ok(oteryn_web::render(&Detail {
            l: Locale::new(&headers)?,
            action,
            status: plan.status(now),
            starts: display_time(plan.starts)?,
            ends: display_time(plan.ends)?,
            reviewed_at: plan.reviewed_at.map(display_time).transpose()?,
            plan,
            csrf,
        }))
    }
    .await;
    response(&headers, result)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewForm {
    _csrf: String,
    decision: Decision,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Decision {
    Approve,
    Decline,
}

async fn review<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    form: Result<Form<ReviewForm>, FormRejection>,
) -> Response {
    let result = async {
        let Form(form) = form.map_err(|_| Error::Invalid)?;
        store::review(
            &s.pool,
            &s.auth,
            context,
            id,
            matches!(form.decision, Decision::Approve),
        )
        .await?;
        Ok(Redirect::to(&format!("{PATH}/{id}")).into_response())
    }
    .await;
    response(&headers, result)
}
