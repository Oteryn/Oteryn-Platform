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
/// or unavailable identity/ownership. Moderator checks gate queue reads behind
/// the staff guard. Mutations require its transaction-local RBAC/session proof.
pub trait Authority: Send + Sync + std::fmt::Debug {
    fn active(&self, account: Uuid) -> CheckFuture<'_, ()>;
    fn character(&self, account: Uuid, character: Uuid) -> CheckFuture<'_, Character>;
    fn moderator(&self, account: Uuid) -> CheckFuture<'_, ()>;
    /// T33c abuse restrictions must explicitly allow new reports.
    fn reporting_allowed(&self, account: Uuid) -> CheckFuture<'_, ()>;
    /// Mutation proofs must use this connection and retain all fences through commit.
    fn active_in<'a>(
        &'a self,
        _tx: &'a mut sqlx::PgConnection,
        _account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async { None })
    }

    fn reporting_allowed_in<'a>(
        &'a self,
        _tx: &'a mut sqlx::PgConnection,
        _account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async { None })
    }

    fn character_in<'a>(
        &'a self,
        _tx: &'a mut sqlx::PgConnection,
        _account: Uuid,
        _character: Uuid,
    ) -> CheckFuture<'a, Character> {
        Box::pin(async { None })
    }

    fn moderator_in<'a>(
        &'a self,
        _tx: &'a mut sqlx::PgConnection,
        _account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async { None })
    }
}
#[derive(Debug)]
struct ProductionAuthority {
    db: PgPool,
}
impl Authority for ProductionAuthority {
    fn active(&self, account: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async move {
            let mut tx = self.db.begin().await.ok()?;
            self.active_in(&mut tx, account).await?;
            tx.commit().await.ok()?;
            Some(())
        })
    }

    fn reporting_allowed(&self, account: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async move {
            let mut tx = self.db.begin().await.ok()?;
            self.reporting_allowed_in(&mut tx, account).await?;
            tx.commit().await.ok()?;
            Some(())
        })
    }

    fn character(&self, account: Uuid, character: Uuid) -> CheckFuture<'_, Character> {
        Box::pin(async move {
            let mut tx = self.db.begin().await.ok()?;
            let proof = self.character_in(&mut tx, account, character).await?;
            tx.commit().await.ok()?;
            Some(proof)
        })
    }

    fn moderator(&self, account: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async move {
            let mut tx = self.db.begin().await.ok()?;
            self.moderator_in(&mut tx, account).await?;
            tx.commit().await.ok()?;
            Some(())
        })
    }

    fn active_in<'a>(
        &'a self,
        tx: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async move {
            // Account eligibility port: shared budget, then the account row in
            // shared mode before any party resource (hierarchy level 3).
            oteryn_db::mutation_budget(&mut *tx).await.ok()?;
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .execute(&mut *tx)
                .await
                .ok()?;
            let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a WHERE a.id=$1 AND a.status='active' AND a.email_verified_at IS NOT NULL AND NOT EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.status='active' AND s.category IN ('restriction','suspension') AND s.effective_at<=clock_timestamp() AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())))")
                .bind(account).fetch_one(&mut *tx).await.ok()?;
            allowed.then_some(())
        })
    }
    fn reporting_allowed_in<'a>(
        &'a self,
        tx: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async move {
            self.active_in(tx, account).await?;
            sqlx::query("LOCK TABLE player_reports IN SHARE MODE")
                .execute(&mut *tx)
                .await
                .ok()?;
            let rejected: i64 = sqlx::query_scalar("SELECT count(*) FROM player_reports WHERE reporter_account_id=$1 AND rejected_at>=clock_timestamp()-interval '30 days'")
                .bind(account).fetch_one(&mut *tx).await.ok()?;
            (rejected < 3).then_some(())
        })
    }
    fn character_in<'a>(
        &'a self,
        tx: &'a mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
    ) -> CheckFuture<'a, Character> {
        Box::pin(async move {
            // Account (level 3) precedes the shared issuance fence taken by
            // the ownership read (level 4).
            self.active_in(&mut *tx, account).await?;
            let oteryn_playerdata::authority::Ownership::Owner(cut) =
                oteryn_playerdata::authority::ownership(&mut *tx, account, character)
                    .await
                    .ok()?
            else {
                return None;
            };
            let name: String = sqlx::query_scalar("SELECT name FROM account_characters WHERE account_id=$1 AND character_id=$2 AND world_id=$3 AND availability='AVAILABLE'")
                .bind(account).bind(character).bind(cut.world).fetch_optional(&mut *tx).await.ok()??;
            sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE")
                .execute(&mut *tx)
                .await
                .ok()?;
            let profile: serde_json::Value = sqlx::query_scalar("SELECT s.payload->'profile'->0 FROM public_snapshots s JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch WHERE s.family='character_profile' AND NOT public_projection_conflicted('character_profile') AND s.subject=$1 AND NOT s.is_watermark AND jsonb_array_length(s.payload->'profile')=1 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM clock_timestamp()))::bigint-29 AND floor(extract(epoch FROM clock_timestamp()))::bigint+1)")
                .bind(character.to_string()).fetch_optional(&mut *tx).await.ok()??;
            if profile["name"].as_str()? != name
                || profile["world_id"].as_str()? != cut.world.to_string()
            {
                return None;
            }
            let level = profile["level"].as_str()?.parse().ok()?;
            let profession = match profile["vocation"].as_u64()? {
                1 | 5 => "sorcerer",
                2 | 6 => "druid",
                3 | 7 => "paladin",
                4 | 8 => "knight",
                9 | 10 => "monk",
                _ => return None,
            };
            Some(Character {
                id: character,
                world: cut.world,
                name,
                level,
                profession: profession.into(),
            })
        })
    }
    fn moderator_in<'a>(
        &'a self,
        tx: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        Box::pin(async move {
            self.active_in(tx, account).await?;
            // Queue reads run behind the staff MFA guard. Mutation authority
            // comes from that guard's context/proof on the caller transaction.
            let identity = oteryn_admin::web::PortalIdentity::configured(self.db.clone());
            let auth = oteryn_admin::rbac::Authorizer::new(self.db.clone(), identity);
            let mut context = oteryn_admin::rbac::CheckContext::new(account);
            context.mfa_verified = true;
            for permission in [
                oteryn_admin::rbac::Permission::Access,
                oteryn_admin::rbac::Permission::ManageSupportReports,
            ] {
                auth.check(context, permission).await.ok()?;
            }
            Some(())
        })
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
fn production_board(db: PgPool) -> Board {
    Board::new(db.clone(), Arc::new(ProductionAuthority { db }))
}
pub fn configured_router(db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("PORTAL_PARTY_ENABLED").as_deref() == Ok("true");
    let board = production_board(db.clone());
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
impl From<oteryn_admin::rbac::Error> for Error {
    fn from(error: oteryn_admin::rbac::Error) -> Self {
        match error {
            oteryn_admin::rbac::Error::Storage(error) => Self::Storage(error),
            _ => Self::Denied,
        }
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
    Extension(context): Extension<oteryn_admin::rbac::CheckContext>,
    Extension(authority): Extension<oteryn_admin::rbac::MutationAuthorization>,
    Form(input): Form<Decision>,
) -> Result<Response, Error> {
    language(&lang)?;
    board
        .moderate(context, &authority, id, &input.action, &input.reason)
        .await?;
    Ok(Redirect::to(&format!("/admin/{lang}/party")).into_response())
}

#[cfg(test)]
#[derive(Debug)]
struct Unavailable;
#[cfg(test)]
impl Authority for Unavailable {
    fn active(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
    fn reporting_allowed(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
    fn character(&self, _: Uuid, _: Uuid) -> CheckFuture<'_, Character> {
        Box::pin(async { None })
    }
    fn moderator(&self, _: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async { None })
    }
}
