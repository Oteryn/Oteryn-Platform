use super::{Action, Appeals, Error, Status};
use crate::reports::web::{Locale, private_headers};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

const PAGE_SIZE: u32 = 20;
const MAX_PAGE: u32 = 10_000;
const APPEALS: Appeals = Appeals { enabled: true };

pub(crate) fn router(pool: PgPool, origin: Option<String>) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = origin.into_iter().collect();
    Router::new()
        .route("/{lang}/account/sanctions", get(index))
        .route("/{lang}/account/sanctions/{id}", get(show).post(change))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn(private_headers))
        .with_state(pool)
}

#[derive(sqlx::FromRow)]
struct Sanction {
    id: Uuid,
    category: String,
    status: String,
    public_reason: String,
    effective_at: OffsetDateTime,
    expires_at: Option<OffsetDateTime>,
    acknowledged: bool,
    appeal_status: String,
    message: Option<String>,
    outcome: Option<String>,
    version: i64,
}

const HISTORY: &str = "SELECT s.id, s.category,
    CASE WHEN s.status='active' AND s.expires_at <= now() THEN 'expired'
         WHEN s.status='active' AND s.effective_at > now() THEN 'scheduled'
         ELSE s.status END AS status,
    s.public_reason, s.effective_at, s.expires_at,
    coalesce(a.acknowledged, s.acknowledged_at IS NOT NULL) AS acknowledged,
    coalesce(a.status, 'none') AS appeal_status, a.message, a.outcome,
    coalesce(a.version, 1) AS version
    FROM sanctions s LEFT JOIN support_appeal_workflows a
        ON a.sanction_id=s.id AND a.owner_id=s.account_id
    WHERE ";

const OWNER_SCOPE: &str = "s.account_id=$1 AND s.status<>'pending'
    AND (s.expires_at IS NOT NULL OR s.approved_by IS NOT NULL)";

async fn account(db: &mut PgConnection, owner: Uuid) -> Result<(), Error> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(owner)
            .fetch_optional(db)
            .await?;
    match status.as_deref() {
        Some("active" | "pending_verification" | "suspended") => Ok(()),
        _ => Err(Error::NotFound),
    }
}

async fn history(pool: &PgPool, owner: Uuid, page: u32) -> Result<Vec<Sanction>, Error> {
    if !(1..=MAX_PAGE).contains(&page) {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    account(&mut tx, owner).await?;
    let items = sqlx::query_as(AssertSqlSafe(format!(
        "{HISTORY}{OWNER_SCOPE} ORDER BY s.created_at DESC, s.id DESC LIMIT $2 OFFSET $3"
    )))
    .bind(owner)
    .bind(i64::from(PAGE_SIZE + 1))
    .bind(i64::from(page - 1) * i64::from(PAGE_SIZE))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(items)
}

async fn detail(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<Sanction, Error> {
    let mut tx = pool.begin().await?;
    account(&mut tx, owner).await?;
    let item = sqlx::query_as(AssertSqlSafe(format!("{HISTORY}{OWNER_SCOPE} AND s.id=$2")))
        .bind(owner)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    tx.commit().await?;
    Ok(item)
}

async fn update(
    pool: &PgPool,
    owner: Uuid,
    id: Uuid,
    version: i64,
    action: PlayerAction,
) -> Result<(), Error> {
    if id.is_nil() || version < 1 {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    account(&mut tx, owner).await?;
    let exists = sqlx::query_scalar::<_, Uuid>(AssertSqlSafe(format!(
        "SELECT s.id FROM sanctions s WHERE {OWNER_SCOPE} AND s.id=$2 FOR UPDATE"
    )))
    .bind(owner)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !exists {
        return Err(Error::NotFound);
    }
    // Older sanctions can predate workflow registration. Adopt them under the
    // sanction lock and carry forward any existing acknowledgement.
    sqlx::query(
        "INSERT INTO support_appeal_workflows (sanction_id,owner_id,acknowledged_at)
        SELECT id,account_id,acknowledged_at FROM sanctions WHERE id=$1 AND account_id=$2
        ON CONFLICT (sanction_id) DO NOTHING",
    )
    .bind(id)
    .bind(owner)
    .execute(&mut *tx)
    .await?;
    let action = match &action {
        PlayerAction::Acknowledge => Action::Acknowledge,
        PlayerAction::Submit { message } => Action::Submit(message),
    };
    APPEALS.change(&mut tx, owner, id, version, action).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
struct Paging {
    page: Option<u32>,
}

#[derive(Deserialize)]
struct Input {
    version: i64,
    #[serde(flatten)]
    action: PlayerAction,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum PlayerAction {
    Acknowledge,
    Submit { message: String },
}

#[derive(Template)]
#[template(path = "support/sanctions.html", config = "src/appeals/askama.toml")]
struct Page {
    locale: Locale,
    items: Vec<Sanction>,
    detail: bool,
    csrf: String,
    previous: Option<u32>,
    next: Option<u32>,
}
impl Page {
    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }
    fn can_submit(item: &Sanction) -> bool {
        matches!(
            Status::parse(&item.appeal_status),
            Ok(Status::None | Status::Rejected)
        )
    }
    fn render(&self) -> Response {
        match Template::render(self) {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

async fn identity(lang: &str, session: &BrowserSession) -> Result<(Locale, Uuid), Response> {
    let locale = Locale::parse(lang).map_err(IntoResponse::into_response)?;
    let owner = session
        .account_id()
        .await
        .ok_or_else(|| Redirect::to(&format!("/{lang}/login")).into_response())?;
    Ok((locale, owner))
}

fn refusal(locale: Locale, error: Error) -> Response {
    let code = StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE);
    let key = match error {
        Error::Invalid => "sanctions-invalid",
        Error::Conflict => "sanctions-conflict",
        Error::Database(_) => "sanctions-unavailable",
        Error::Disabled | Error::NotFound => "sanctions-not-found",
    };
    (code, locale.text(key)).into_response()
}

async fn index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Query(paging): Query<Paging>,
) -> Result<Response, Response> {
    let (locale, owner) = identity(&lang, &session).await?;
    let page = paging.page.unwrap_or(1);
    let mut items = history(&pool, owner, page)
        .await
        .map_err(|e| refusal(locale, e))?;
    let next = (items.len() > PAGE_SIZE as usize && page < MAX_PAGE).then_some(page + 1);
    items.truncate(PAGE_SIZE as usize);
    Ok(Page {
        locale,
        items,
        detail: false,
        csrf: String::new(),
        previous: (page > 1).then_some(page - 1),
        next,
    }
    .render())
}

async fn show(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, Response> {
    let (locale, owner) = identity(&lang, &session).await?;
    let item = detail(&pool, owner, id)
        .await
        .map_err(|e| refusal(locale, e))?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|e| refusal(locale, Error::Database(e)))?;
    Ok(Page {
        locale,
        items: vec![item],
        detail: true,
        csrf,
        previous: None,
        next: None,
    }
    .render())
}

async fn change(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(input): Form<Input>,
) -> Result<Response, Response> {
    let (locale, owner) = identity(&lang, &session).await?;
    update(&pool, owner, id, input.version, input.action)
        .await
        .map_err(|e| refusal(locale, e))?;
    Ok(Redirect::to(&format!("/{lang}/account/sanctions/{id}")).into_response())
}

#[cfg(test)]
mod tests;
