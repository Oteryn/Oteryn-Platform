//! Public status and Platform-authored incident notices for a registry world.
use super::{Worlds, localized};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, Permission},
};
use oteryn_web::{Language, ratelimit::Denial, session::BrowserSession};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) const NOTICE_PERMISSION: Permission = Permission::ManagePortalAnnouncements;
const PAGE_SIZE: usize = 20;
const CHANGES_PER_MINUTE: i64 = 30;
// Each summary can contain 240 four-byte characters, percent-encoded by the browser.
const MAX_FORM_BYTES: usize = 8 * 1024;

pub(super) fn staff_routes(db: PgPool) -> Router {
    Router::new()
        .route(
            "/admin/world-status/{lang}/{world}",
            get(operator).post(publish),
        )
        .route(
            "/admin/world-status/{lang}/{world}/{incident}/resolve",
            post(resolve),
        )
        .layer(DefaultBodyLimit::max(MAX_FORM_BYTES))
        .with_state(db)
}

pub(crate) fn world_id(s: &str) -> Result<Uuid, StatusCode> {
    let id = Uuid::parse_str(s).map_err(|_| StatusCode::BAD_REQUEST)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != s
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(id)
}

fn locale(lang: &str) -> Result<Language, StatusCode> {
    match lang {
        "en" => Ok(Language::En),
        "pl" => Ok(Language::Pl),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    before: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Incident {
    id: Uuid,
    summary: String,
    opened: String,
    resolved: Option<String>,
}

#[derive(Template)]
#[template(
    path = "community/world_uptime.html",
    config = "src/worlds_askama.toml"
)]
struct Page {
    world: Uuid,
    name: String,
    language: Language,
    lang: &'static str,
    incidents: Vec<Incident>,
    before: Option<Uuid>,
    next: Option<Uuid>,
    can_publish: bool,
    csrf: Option<String>,
    show_operator_link: bool,
    show_online_link: bool,
}

impl Page {
    fn path(&self, lang: &str) -> String {
        if self.csrf.is_some() {
            format!("/admin/world-status/{lang}/{}", self.world)
        } else {
            format!("/{lang}/worlds/{}/uptime", self.world)
        }
    }

    fn response(&self) -> Response {
        localized(oteryn_web::render(self), self.language)
    }

    fn locale_path(&self, lang: &str) -> String {
        let path = self.path(lang);
        match self.before {
            Some(before) => format!("{path}?before={before}"),
            None => path,
        }
    }
}

async fn history_page(
    db: &PgPool,
    language: Language,
    world: Uuid,
    history: History,
) -> Result<Page, StatusCode> {
    let before = history.before.as_deref().map(world_id).transpose()?;
    let name = sqlx::query_scalar("SELECT name FROM world_status WHERE id=$1")
        .bind(world)
        .fetch_optional(db)
        .await
        .map_err(unavailable)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut incidents = sqlx::query_as::<_, Incident>(
        "SELECT id, CASE WHEN $2='en' THEN summary_en ELSE summary_pl END AS summary,
                to_char(opened_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS opened,
                to_char(resolved_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS resolved
         FROM world_incidents
         WHERE world_id=$1 AND ($3::uuid IS NULL OR id < $3)
         ORDER BY id DESC LIMIT $4",
    )
    .bind(world)
    .bind(language.code())
    .bind(before)
    .bind((PAGE_SIZE + 1) as i64)
    .fetch_all(db)
    .await
    .map_err(unavailable)?;
    let next = if incidents.len() > PAGE_SIZE {
        incidents.pop();
        incidents.last().map(|incident| incident.id)
    } else {
        None
    };
    let can_publish =
        before.is_none() && incidents.iter().all(|incident| incident.resolved.is_some());
    Ok(Page {
        world,
        name,
        language,
        lang: language.code(),
        incidents,
        before,
        next,
        can_publish,
        csrf: None,
        show_operator_link: false,
        show_online_link: false,
    })
}

pub(super) async fn public(
    State(state): State<Worlds>,
    Path((lang, world)): Path<(String, String)>,
    Query(history): Query<History>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let mut page = history_page(&state.db, locale(&lang)?, world_id(&world)?, history).await?;
    page.show_online_link = state.show_online_link;
    page.show_operator_link = state.staff_host.as_ref().is_some_and(|host| {
        let values = headers.get_all(header::HOST);
        values.iter().count() == 1
            && values
                .iter()
                .next()
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.eq_ignore_ascii_case(host))
    });
    Ok(page.response())
}

async fn operator(
    State(db): State<PgPool>,
    Path((lang, world)): Path<(String, String)>,
    Query(history): Query<History>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let mut page = history_page(&db, locale(&lang)?, world_id(&world)?, history).await?;
    page.csrf = Some(session.csrf_token().await.map_err(unavailable)?);
    Ok(page.response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Notice {
    #[serde(rename = "_csrf")]
    _csrf: String,
    summary_en: String,
    summary_pl: String,
}

impl Notice {
    fn summaries(&self) -> Result<(&str, &str), StatusCode> {
        let en = self.summary_en.trim();
        let pl = self.summary_pl.trim();
        if [en, pl]
            .iter()
            .any(|s| !(1..=240).contains(&s.chars().count()) || s.chars().any(char::is_control))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok((en, pl))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolve {
    #[serde(rename = "_csrf")]
    _csrf: String,
}

async fn publish(
    State(db): State<PgPool>,
    Path((lang, world)): Path<(String, String)>,
    Extension(auth): Extension<MutationAuthorization>,
    Extension(context): Extension<CheckContext>,
    Form(notice): Form<Notice>,
) -> Result<Redirect, ChangeError> {
    let language = locale(&lang)?;
    charge_change(&db, context.account)
        .await
        .map_err(|error| ChangeError::Quota(error, language))?;
    let world = world_id(&world)?;
    let (en, pl) = notice.summaries()?;
    change(&db, &auth, context, world, Change::Open { en, pl }).await?;
    Ok(operator_redirect(language, world))
}

async fn resolve(
    State(db): State<PgPool>,
    Path((lang, world, incident)): Path<(String, String, String)>,
    Extension(auth): Extension<MutationAuthorization>,
    Extension(context): Extension<CheckContext>,
    Form(_): Form<Resolve>,
) -> Result<Redirect, ChangeError> {
    let language = locale(&lang)?;
    charge_change(&db, context.account)
        .await
        .map_err(|error| ChangeError::Quota(error, language))?;
    let world = world_id(&world)?;
    let incident = world_id(&incident)?;
    change(&db, &auth, context, world, Change::Resolve(incident)).await?;
    Ok(operator_redirect(language, world))
}

enum ChangeError {
    Status(StatusCode),
    Quota(Denial, Language),
}

impl From<StatusCode> for ChangeError {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl IntoResponse for ChangeError {
    fn into_response(self) -> Response {
        match self {
            Self::Status(status) => status.into_response(),
            Self::Quota(error, language) => error.response(language, false),
        }
    }
}

async fn charge_change(db: &PgPool, account: Uuid) -> Result<(), Denial> {
    let (count, retry_after): (i64, i64) = sqlx::query_as(
        "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         VALUES('world-incident-change',sha256(uuid_send($1)),date_trunc('minute',statement_timestamp()),1)
         ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE
         SET count=least(web_rate_limits.count+1,$2+1)
         RETURNING count,greatest(1,ceil(extract(epoch FROM
           window_start + interval '1 minute' - statement_timestamp())))::bigint",
    )
    .bind(account)
    .bind(CHANGES_PER_MINUTE)
    .fetch_one(db)
    .await
    .map_err(|_| Denial::Unavailable)?;
    if count > CHANGES_PER_MINUTE {
        return Err(Denial::Limited { retry_after });
    }
    Ok(())
}

fn operator_redirect(language: Language, world: Uuid) -> Redirect {
    Redirect::to(&format!("/admin/world-status/{}/{world}", language.code()))
}

enum Change<'a> {
    Open { en: &'a str, pl: &'a str },
    Resolve(Uuid),
}

async fn change(
    db: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    world: Uuid,
    change: Change<'_>,
) -> Result<(), StatusCode> {
    let mut tx = db.begin().await.map_err(unavailable)?;
    oteryn_db::mutation_budget(&mut tx)
        .await
        .map_err(unavailable)?;
    rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM world_status WHERE id=$1 FOR NO KEY UPDATE")
                    .bind(world)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        auth,
        context,
        &[Permission::Access, NOTICE_PERMISSION],
    )
    .await
    .map_err(|error| match error {
        rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let known: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM world_status WHERE id=$1)")
        .bind(world)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
    if !known {
        return Err(StatusCode::NOT_FOUND);
    }

    let (id, action, before, after) = match change {
        Change::Open { en, pl } => {
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO world_incidents(world_id, summary_en, summary_pl) VALUES ($1,$2,$3)
                 ON CONFLICT (world_id) WHERE resolved_at IS NULL DO NOTHING RETURNING id",
            )
            .bind(world)
            .bind(en)
            .bind(pl)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::CONFLICT)?;
            (
                id,
                "portal.world_incident.published",
                json!({}),
                json!({"world":world,"state":"open","summary_en":en,"summary_pl":pl}),
            )
        }
        Change::Resolve(id) => {
            let resolved: Option<bool> = sqlx::query_scalar(
                "SELECT resolved_at IS NOT NULL FROM world_incidents WHERE world_id=$1 AND id=$2",
            )
            .bind(world)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
            if resolved.ok_or(StatusCode::NOT_FOUND)? {
                return Err(StatusCode::CONFLICT);
            }
            sqlx::query(
                "UPDATE world_incidents SET resolved_at=greatest(clock_timestamp(), opened_at)
                 WHERE world_id=$1 AND id=$2",
            )
            .bind(world)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            (
                id,
                "portal.world_incident.resolved",
                json!({"world":world,"state":"open"}),
                json!({"world":world,"state":"resolved"}),
            )
        }
    };
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)
}

fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

#[cfg(test)]
mod tests;
