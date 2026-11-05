//! Recruitment board. PORTAL_PARTY_ENABLED=true enables reads; mutations additionally
//! require a trusted dependency adapter. The unavailable default never grants access.
//! Routes: /{pl|en}/party; the staff queue lives at /admin/{pl|en}/party.
//! Browser origins use OTERYN_PUBLIC_ORIGIN, as in the main server.
//! No game command is sent: contact is exclusively through the character's in-game name.
use axum::{
    Form, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

mod party_store;
#[cfg(test)]
mod party_tests;
mod party_view;

pub type CheckFuture<'a, T> = Pin<Box<dyn Future<Output = Option<T>> + Send + 'a>>;
/// Trusted current identity and LCFA data, never decoded from user input.
#[derive(Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub world: Uuid,
    pub name: String,
    pub level: i32,
    pub profession: String,
}
/// Narrow seam for T44/T33c. Implementations must reject stale, ambiguous, disabled
/// or unavailable identity/ownership. Moderator checks require staff MFA, RBAC
/// ManageSupportReports and the current T10b authorization policy.
pub trait Authority: Send + Sync + std::fmt::Debug {
    fn active(&self, account: Uuid) -> CheckFuture<'_, ()>;
    fn character(&self, account: Uuid, character: Uuid) -> CheckFuture<'_, Character>;
    fn moderator(&self, account: Uuid) -> CheckFuture<'_, ()>;
    /// T33c abuse restrictions must explicitly allow new reports.
    fn reporting_allowed(&self, account: Uuid) -> CheckFuture<'_, ()>;
}
#[derive(Debug)]
struct Unavailable;
impl Authority for Unavailable {
    fn reporting_allowed(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
    fn active(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
    fn character(&self, _: Uuid, _: Uuid) -> CheckFuture<'_, Character> {
        Box::pin(async { None })
    }
    fn moderator(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
}
#[derive(Clone, Debug)]
pub struct Board {
    db: PgPool,
    authority: Arc<dyn Authority>,
}
impl Board {
    pub fn new(db: PgPool, authority: Arc<dyn Authority>) -> Self {
        Self { db, authority }
    }
}
pub fn configured_router(db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("PORTAL_PARTY_ENABLED").as_deref() == Ok("true");
    let board = Board::new(db.clone(), Arc::new(Unavailable));
    let public = router(board.clone(), enabled);
    if !enabled {
        return Ok(public);
    }
    let staff = oteryn_admin::web::configured_guard(
        staff_routes(board),
        db,
        oteryn_admin::rbac::Permission::ManageSupportReports,
    )?;
    Ok(public.merge(staff))
}
/// Wraps the recruitment queue in T10b's host, MFA, current RBAC and CSRF guard.
pub fn moderation_router<
    I: oteryn_admin::web::StaffSession,
    A: oteryn_admin::rbac::AccountStateReader + 'static,
>(
    board: Board,
    config: oteryn_admin::web::PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    oteryn_admin::web::guarded(
        staff_routes(board.clone()),
        board.db,
        config,
        identity,
        accounts,
        oteryn_admin::rbac::Permission::ManageSupportReports,
    )
}
fn staff_routes(board: Board) -> Router {
    Router::new()
        .route("/admin/{lang}/party", get(moderation))
        .route("/admin/{lang}/party/{id}/moderate", post(moderate))
        .with_state(board)
        .layer(axum::middleware::from_fn(party_view::headers))
        .layer(axum::extract::DefaultBodyLimit::max(8192))
}
pub fn router(board: Board, enabled: bool) -> Router {
    if !enabled {
        return Router::new();
    }
    let mut session = oteryn_web::session::SessionLayer::new(board.db.clone());
    session.csrf.trusted_origins = vec![
        std::env::var("OTERYN_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into()),
    ];
    Router::new()
        .route("/{lang}/party", get(page).post(create))
        .route("/{lang}/party/{id}/report", post(report))
        .route("/{lang}/party/{id}/remove", post(remove))
        .with_state(board)
        .layer(axum::middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .layer(axum::middleware::from_fn(party_view::headers))
        .layer(axum::extract::DefaultBodyLimit::max(8192))
}
#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub world: Option<Uuid>,
    pub purpose: Option<String>,
    pub level: Option<i32>,
    pub profession: Option<String>,
}
fn profession(value: &str) -> bool {
    matches!(
        value,
        "any" | "knight" | "paladin" | "sorcerer" | "druid" | "monk"
    )
}
impl Filter {
    fn valid(&self) -> bool {
        self.world.is_none_or(|id| !id.is_nil())
            && self.purpose.as_deref().is_none_or(purpose)
            && self.level.is_none_or(|n| (1..=100000).contains(&n))
            && self.profession.as_deref().is_none_or(profession)
    }
}
fn purpose(value: &str) -> bool {
    matches!(value, "hunt" | "boss" | "quest")
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub character: Uuid,
    pub purpose: String,
    pub min_level: i32,
    pub max_level: i32,
    pub profession: String,
    pub starts_at: i64,
    pub expires_at: i64,
    pub note: String,
    #[serde(rename = "_csrf")]
    pub csrf: Option<String>,
}
impl Draft {
    fn valid(&self, now: i64) -> bool {
        !self.character.is_nil()
            && purpose(&self.purpose)
            && profession(&self.profession)
            && (1..=100000).contains(&self.min_level)
            && (self.min_level..=100000).contains(&self.max_level)
            && self.starts_at >= now
            && self.expires_at > self.starts_at
            && self.expires_at <= now.saturating_add(7 * 86400)
            && text(&self.note, 0, 500)
    }
}
fn text(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.trim().chars().count())
        && value.chars().count() <= max
        && !value.chars().any(|c| c.is_control())
}
#[derive(Debug)]
pub enum Error {
    Denied,
    Invalid,
    Missing,
    Limited,
    Storage(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage(e)
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Denied => StatusCode::FORBIDDEN,
            Self::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Limited => StatusCode::TOO_MANY_REQUESTS,
            Self::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}
async fn account(
    board: &Board,
    session: &oteryn_web::session::BrowserSession,
) -> Result<Uuid, Error> {
    let id = session
        .account_id()
        .await
        .filter(|id| !id.is_nil())
        .ok_or(Error::Denied)?;
    board.authority.active(id).await.ok_or(Error::Denied)?;
    Ok(id)
}
fn language(lang: &str) -> Result<(), Error> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(Error::Missing)
    }
}
async fn page(
    State(board): State<Board>,
    Path(lang): Path<String>,
    Query(filter): Query<Filter>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
) -> Result<Response, Error> {
    language(&lang)?;
    let rows = board.list(&filter).await?;
    let token = session.csrf_token().await?;
    party_view::page(
        &lang,
        &token,
        rows,
        false,
        account(&board, &session).await.is_ok(),
    )
}
async fn create(
    State(board): State<Board>,
    Path(lang): Path<String>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Form(draft): Form<Draft>,
) -> Result<Response, Error> {
    language(&lang)?;
    let owner = account(&board, &session).await?;
    board.create(owner, draft).await?;
    Ok(Redirect::to(&format!("/{lang}/party")).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reason {
    reason: String,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn report(
    State(board): State<Board>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Form(input): Form<Reason>,
) -> Result<Response, Error> {
    language(&lang)?;
    board
        .report(account(&board, &session).await?, id, &input.reason)
        .await?;
    Ok(Redirect::to(&format!("/{lang}/party")).into_response())
}
async fn remove(
    State(board): State<Board>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
) -> Result<Response, Error> {
    language(&lang)?;
    board.remove(account(&board, &session).await?, id).await?;
    Ok(Redirect::to(&format!("/{lang}/party")).into_response())
}
async fn moderation(
    State(board): State<Board>,
    Path(lang): Path<String>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
) -> Result<Response, Error> {
    language(&lang)?;
    let actor = account(&board, &session).await?;
    board
        .authority
        .moderator(actor)
        .await
        .ok_or(Error::Denied)?;
    let rows = board.reports().await?;
    party_view::page(&lang, &session.csrf_token().await?, rows, true, false)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    action: String,
    reason: String,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn moderate(
    State(board): State<Board>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Form(input): Form<Decision>,
) -> Result<Response, Error> {
    language(&lang)?;
    board
        .moderate(
            account(&board, &session).await?,
            id,
            &input.action,
            &input.reason,
        )
        .await?;
    Ok(Redirect::to(&format!("/admin/{lang}/party")).into_response())
}
