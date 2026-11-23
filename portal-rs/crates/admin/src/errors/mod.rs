//! Staff review of grouped portal errors.
use crate::{
    audit,
    rbac::{self, Authorizer, CheckContext, MutationAuthorization, Permission},
    web,
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::{observability::RequestId, session::BrowserSession};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, types::time::OffsetDateTime};
use uuid::Uuid;

pub(super) fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(web::configured_guard(
        Router::new()
            .route("/admin/errors", get(list))
            .route("/admin/errors/{id}", get(detail))
            .with_state(pool.clone()),
        pool.clone(),
        Permission::ViewPortalErrors,
    )?
    .merge(web::configured_guard_all(
        Router::new()
            .route("/admin/errors/{id}/resolve", post(resolve))
            .with_state(pool.clone()),
        pool.clone(),
        &[
            Permission::ViewPortalErrors,
            Permission::ResolvePortalErrors,
        ],
    )?)
    .merge(web::configured_guard_all(
        Router::new()
            .route("/admin/errors/{id}/ignore", post(ignore))
            .with_state(pool.clone()),
        pool,
        &[Permission::ViewPortalErrors, Permission::IgnorePortalErrors],
    )?))
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid error filter or review")]
    Invalid,
    #[error("error group not found")]
    Missing,
    #[error("the error group changed")]
    Conflict,
    #[error("unable to load error group")]
    Storage(#[from] sqlx::Error),
    #[error("unable to authorize error review")]
    Authorization(#[from] rbac::Error),
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Authorization(rbac::Error::Denied) => StatusCode::FORBIDDEN,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ReviewStatus {
    All,
    Open,
    Resolved,
    Ignored,
}
impl ReviewStatus {
    fn filter(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            _ => Some(self.code()),
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    status: Option<ReviewStatus>,
    #[serde(default)]
    route: String,
    before: Option<String>,
}
impl Filters {
    fn validate(&self) -> Result<(), Error> {
        if self.route.encode_utf16().count() > 200 || self.route.chars().any(char::is_control) {
            return Err(Error::Invalid);
        }
        Ok(())
    }

    fn cursor(&self) -> Result<(Option<OffsetDateTime>, Option<Uuid>), Error> {
        let Some(cursor) = &self.before else {
            return Ok((None, None));
        };
        let (time, id) = cursor.split_once(',').ok_or(Error::Invalid)?;
        if cursor.len() > 80 {
            return Err(Error::Invalid);
        }
        let time = OffsetDateTime::parse(time, &time::format_description::well_known::Rfc3339)
            .map_err(|_| Error::Invalid)?;
        let id = Uuid::parse_str(id).map_err(|_| Error::Invalid)?;
        Ok((Some(time), Some(id)))
    }
}

#[derive(sqlx::FromRow)]
struct Group {
    id: Uuid,
    route: String,
    error_kind: String,
    top_frame: Option<String>,
    status: String,
    occurrence_count: i64,
    first_seen: OffsetDateTime,
    last_seen: OffsetDateTime,
    last_request_id: Uuid,
}
impl Group {
    fn first_seen(&self) -> String {
        utc(self.first_seen)
    }

    fn last_seen(&self) -> String {
        utc(self.last_seen)
    }
}

fn utc(value: OffsetDateTime) -> String {
    value
        .to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| value.unix_timestamp().to_string())
}

const PAGE_SIZE: usize = 30;

#[derive(Template)]
#[template(path = "admin/errors/list.html", config = "src/web_askama.toml")]
struct List {
    lang: &'static str,
    filters: Filters,
    groups: Vec<Group>,
    next: Option<String>,
}
impl List {
    fn text(&self, key: &str) -> String {
        web::text(self.lang, key)
    }

    fn selected(&self, status: &str) -> bool {
        self.filters
            .status
            .is_some_and(|selected| selected.code() == status)
    }

    fn status_text(&self, status: &str) -> String {
        status_text(self.lang, status)
    }
}

async fn list(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filters): Query<Filters>,
) -> Result<Response, Error> {
    filters.validate()?;
    let (before, id) = filters.cursor()?;
    let mut groups = sqlx::query_as::<_, Group>(
        "SELECT id, route, error_kind, top_frame, status, occurrence_count, first_seen, last_seen, last_request_id
           FROM portal_errors
          WHERE ($1::text IS NULL OR status = $1)
            AND ($2 = '' OR strpos(route, $2) > 0)
            AND ($3::timestamptz IS NULL OR (last_seen, id) < ($3, $4::uuid))
          ORDER BY last_seen DESC, id DESC LIMIT $5",
    )
    .bind(filters.status.and_then(ReviewStatus::filter))
    .bind(filters.route.trim())
    .bind(before)
    .bind(id)
    .bind(PAGE_SIZE as i64 + 1)
    .fetch_all(&pool)
    .await?;
    let next = if groups.len() > PAGE_SIZE {
        groups.truncate(PAGE_SIZE);
        groups.last().and_then(|group| {
            let mut args = vec![
                ("before", format!("{},{}", group.last_seen(), group.id)),
                ("route", filters.route.clone()),
            ];
            if let Some(status) = filters.status {
                args.push(("status", status.code().to_owned()));
            }
            serde_urlencoded::to_string(args).ok()
        })
    } else {
        None
    };
    Ok(oteryn_web::render(&List {
        lang: web::language(&headers),
        filters,
        groups,
        next,
    }))
}

#[derive(Template)]
#[template(path = "admin/errors/detail.html", config = "src/web_askama.toml")]
struct Detail {
    lang: &'static str,
    group: Group,
    resolve: bool,
    ignore: bool,
    csrf: String,
}
impl Detail {
    fn text(&self, key: &str) -> String {
        web::text(self.lang, key)
    }

    fn status_text(&self, status: &str) -> String {
        status_text(self.lang, status)
    }
}

fn status_text(lang: &str, status: &str) -> String {
    web::text(lang, &format!("errors-{status}"))
}

async fn detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let group = sqlx::query_as::<_, Group>(
        "SELECT id, route, error_kind, top_frame, status, occurrence_count, first_seen, last_seen, last_request_id
         FROM portal_errors WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::Missing)?;
    let authorizer = Authorizer::new(pool.clone(), web::PortalIdentity::configured(pool));
    let resolve = group.status == "open"
        && allowed(&authorizer, context, Permission::ResolvePortalErrors).await?;
    let ignore = group.status == "open"
        && allowed(&authorizer, context, Permission::IgnorePortalErrors).await?;
    let csrf = if resolve || ignore {
        session.csrf_token().await?
    } else {
        String::new()
    };
    Ok(oteryn_web::render(&Detail {
        lang: web::language(&headers),
        group,
        resolve,
        ignore,
        csrf,
    }))
}

async fn allowed(
    auth: &Authorizer<web::PortalIdentity>,
    context: CheckContext,
    permission: Permission,
) -> Result<bool, Error> {
    match auth.check(context, permission).await {
        Ok(()) => Ok(true),
        Err(rbac::Error::Denied) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    count: i64,
    _csrf: String,
}

#[derive(Clone, Copy)]
enum Action {
    Resolve,
    Ignore,
}
impl Action {
    fn permission(self) -> Permission {
        match self {
            Self::Resolve => Permission::ResolvePortalErrors,
            Self::Ignore => Permission::IgnorePortalErrors,
        }
    }

    fn status(self) -> ReviewStatus {
        match self {
            Self::Resolve => ReviewStatus::Resolved,
            Self::Ignore => ReviewStatus::Ignored,
        }
    }
}

async fn resolve(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    request: Option<Extension<RequestId>>,
    Form(review): Form<Review>,
) -> Result<Redirect, Error> {
    review_group(
        &pool,
        id,
        context,
        &auth,
        request.map(|r| r.0),
        review.count,
        Action::Resolve,
    )
    .await
}

async fn ignore(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    request: Option<Extension<RequestId>>,
    Form(review): Form<Review>,
) -> Result<Redirect, Error> {
    review_group(
        &pool,
        id,
        context,
        &auth,
        request.map(|r| r.0),
        review.count,
        Action::Ignore,
    )
    .await
}

async fn review_group(
    pool: &PgPool,
    id: Uuid,
    context: CheckContext,
    auth: &MutationAuthorization,
    request: Option<RequestId>,
    count: i64,
    action: Action,
) -> Result<Redirect, Error> {
    if count < 1 || id.is_nil() {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    rbac::authorize_mutation_with_resources_domain::<Error, _>(
        &mut tx,
        &[],
        |connection| {
            Box::pin(async move {
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM portal_errors WHERE id = $1 FOR UPDATE",
                )
                .bind(id)
                .fetch_optional(connection)
                .await?
                .ok_or(Error::Missing)?;
                Ok(())
            })
        },
        auth,
        context,
        &[
            Permission::Access,
            Permission::ViewPortalErrors,
            action.permission(),
        ],
    )
    .await?;
    let status = action.status().code();
    let updated = sqlx::query(
        "UPDATE portal_errors SET status = $2
         WHERE id = $1 AND status = 'open' AND occurrence_count = $3",
    )
    .bind(id)
    .bind(status)
    .bind(count)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(Error::Conflict);
    }
    audit::append(
        &mut tx,
        audit::Entry {
            context: audit::AuditContext {
                actor: Some(context.account),
                request_id: request.map_or_else(Uuid::now_v7, |request| request.0),
            },
            action: action.permission().as_str(),
            target: &id.to_string(),
            before: json!({"status": ReviewStatus::Open.code(), "occurrence_count": count}),
            after: json!({"status": status, "occurrence_count": count}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Redirect::to(&format!("/admin/errors/{id}")))
}

#[cfg(test)]
mod tests;
