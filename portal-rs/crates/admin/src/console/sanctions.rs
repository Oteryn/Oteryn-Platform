//! Platform-only case reviews. A local approval carries no execution authority.
use super::access::{language, tr};
use crate::{
    audit::{self, AuditContext, Entry},
    control::catalog::StaffOperation,
    rbac::{self, AccountStateReader, Authorizer, CheckContext, Permission},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use std::sync::Arc;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const PATH: &str = "/admin/console/sanctions";

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum Error {
    #[error("invalid review input")]
    BadInput,
    #[error("review access denied")]
    Denied,
    #[error("review state has changed or expired")]
    Conflict,
    #[error("review submission limit reached")]
    Limited,
    #[error("review service unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl From<rbac::Error> for Error {
    fn from(error: rbac::Error) -> Self {
        match error {
            rbac::Error::Storage(_) => Self::Unavailable,
            _ => Self::Denied,
        }
    }
}
impl Error {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadInput => StatusCode::BAD_REQUEST,
            Self::Denied => StatusCode::FORBIDDEN,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Limited => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::BadInput => "sanctions-invalid",
            Self::Denied => "sanctions-denied",
            Self::Conflict => "sanctions-conflict",
            Self::Limited => "sanctions-limited",
            Self::Unavailable => "unavailable",
        }
    }

    fn render(&self, lang: &str, back: String) -> Response {
        let mut response = oteryn_web::render(&Refusal {
            lang,
            back,
            message: tr(lang, self.message()),
            title: tr(lang, "sanction-reviews"),
        });
        *response.status_mut() = self.status();
        response
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.status(), [(header::CACHE_CONTROL, "no-store")]).into_response()
    }
}

#[derive(Template)]
#[template(
    path = "sanction_refusal.html",
    config = "src/console/templates/askama.toml"
)]
struct Refusal<'a> {
    lang: &'a str,
    back: String,
    message: String,
    title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
enum Kind {
    Ban,
    Mute,
    Jail,
}
impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::Ban => "ban",
            Self::Mute => "mute",
            Self::Jail => "jail",
        }
    }

    fn permission(self) -> Permission {
        match self {
            Self::Ban => Permission::for_staff_operation(StaffOperation::Ban),
            Self::Mute | Self::Jail => Permission::GameSanctionWrite,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
enum Reason {
    Conduct,
    Harassment,
    Cheating,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
enum LiftReason {
    Appeal,
    Correction,
    Review,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Apply {
    _csrf: String,
    lang: String,
    target: Uuid,
    report: Uuid,
    kind: Kind,
    reason: Reason,
    expires: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Approve,
    Lift,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Advance {
    _csrf: String,
    lang: String,
    revision: u8,
    action: Action,
    reason: Option<LiftReason>,
}

fn expiry(kind: Kind, value: &str, now: OffsetDateTime) -> Result<Option<OffsetDateTime>, Error> {
    if kind == Kind::Ban && value.is_empty() {
        return Ok(None);
    }
    if value.len() != 16 {
        return Err(Error::BadInput);
    }
    let at =
        OffsetDateTime::parse(&format!("{value}:00Z"), &Rfc3339).map_err(|_| Error::BadInput)?;
    if at <= now || at > now + Duration::days(30) {
        return Err(Error::BadInput);
    }
    Ok(Some(at))
}

fn valid_id(id: Uuid) -> Result<(), Error> {
    (id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
        .then_some(())
        .ok_or(Error::BadInput)
}

fn transition(
    stage: u8,
    revision: u8,
    action: Action,
    requester: Uuid,
    actor: Uuid,
    eligible: bool,
) -> Result<u8, Error> {
    if stage != revision || !eligible {
        return Err(Error::Conflict);
    }
    match (stage, action) {
        (1 | 3, Action::Approve) if requester == actor => Err(Error::Denied),
        (1 | 3, Action::Approve) | (2, Action::Lift) => Ok(stage + 1),
        _ => Err(Error::Conflict),
    }
}

#[derive(Clone, sqlx::FromRow)]
struct Target {
    id: Uuid,
    world_id: Uuid,
    account_id: Uuid,
    character_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct Review {
    id: Uuid,
    target_id: Uuid,
    report_id: Uuid,
    kind: Kind,
    reason: String,
    expires_at: Option<OffsetDateTime>,
    requested_by: Uuid,
    created_at: OffsetDateTime,
    apply_approved_by: Option<Uuid>,
    apply_approved_at: Option<OffsetDateTime>,
    lift_requested_by: Option<Uuid>,
    lift_requested_at: Option<OffsetDateTime>,
    lift_reason: Option<String>,
    lift_approved_by: Option<Uuid>,
    lift_approved_at: Option<OffsetDateTime>,
}

impl Review {
    fn stage(&self) -> u8 {
        if self.lift_approved_by.is_some() {
            4
        } else if self.lift_requested_by.is_some() {
            3
        } else if self.apply_approved_by.is_some() {
            2
        } else {
            1
        }
    }

    fn requester(&self) -> Uuid {
        self.lift_requested_by.unwrap_or(self.requested_by)
    }

    fn eligible(&self, now: OffsetDateTime) -> bool {
        self.expires_at.is_none_or(|expiry| now < expiry)
            && match self.stage() {
                1 => now < self.created_at + Duration::minutes(30),
                3 => self
                    .lift_requested_at
                    .is_some_and(|at| now < at + Duration::minutes(30)),
                _ => true,
            }
    }
}

struct Service<A> {
    pool: PgPool,
    auth: Authorizer<A>,
}

pub(super) fn routes<A: AccountStateReader + 'static>(pool: PgPool, accounts: A) -> Router {
    let auth = Authorizer::new(pool.clone(), accounts).with_game_flags(rbac::GamePermissionFlags {
        enabled: true,
        deploy: false,
    });
    Router::new()
        .route(PATH, get(page::<A>).post(apply::<A>))
        .route(
            "/admin/console/sanctions/{id}",
            axum::routing::post(advance::<A>),
        )
        .with_state(Arc::new(Service { pool, auth }))
}

fn locale(lang: Option<&str>) -> Result<&'static str, Error> {
    language(lang).map_err(|_| Error::BadInput)
}

async fn lock_scope(conn: &mut PgConnection, staff: Uuid, target: Uuid) -> Result<Target, Error> {
    sqlx::query_as(
        "SELECT t.* FROM admin_sanction_review_targets t
         JOIN admin_sanction_review_grants g ON g.target_id=t.id
         WHERE t.id=$1 AND g.staff_id=$2 FOR SHARE OF t,g",
    )
    .bind(target)
    .bind(staff)
    .fetch_optional(conn)
    .await?
    .ok_or(Error::Denied)
}

async fn recent_confirmation(conn: &mut PgConnection, actor: CheckContext) -> Result<bool, Error> {
    let proof = actor.session_proof.ok_or(Error::Denied)?;
    // Identity issues a 15-minute confirmation. Local reviews use its first five minutes.
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM web_sessions
         WHERE id_hash=$1 AND account_id=$2 AND sudo_until>clock_timestamp()+interval '10 minutes'
         AND sudo_until<=clock_timestamp()+interval '15 minutes')",
    )
    .bind(proof.session_hash.as_slice())
    .bind(actor.account)
    .fetch_one(conn)
    .await?)
}

async fn clock(conn: &mut PgConnection) -> Result<OffsetDateTime, Error> {
    Ok(sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(conn)
        .await?)
}

impl<A: AccountStateReader> Service<A> {
    async fn authorize(
        &self,
        conn: &mut PgConnection,
        actor: CheckContext,
        kind: Kind,
    ) -> Result<(), Error> {
        for permission in [
            Permission::Access,
            Permission::GameSanctionRead,
            kind.permission(),
        ] {
            self.auth.check_in(conn, actor, permission).await?;
        }
        if !recent_confirmation(conn, actor).await? {
            return Err(Error::Denied);
        }
        Ok(())
    }

    async fn propose(&self, actor: CheckContext, input: Apply) -> Result<Uuid, Error> {
        valid_id(input.target)?;
        valid_id(input.report)?;
        let mut tx = self.pool.begin().await?;
        rbac::lock_accounts_resources_and_mutation_with::<Error, _>(
            &mut tx,
            &[actor.account],
            |conn| Box::pin(async move {
                let target = lock_scope(&mut *conn, actor.account, input.target).await?;
                if target.account_id == actor.account {
                    return Err(Error::Denied);
                }
                // The report is a case reference. Target authority comes from the grant.
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM player_reports WHERE id=$1 AND report_type='player'
                     AND target_reference IN ($2, $3) AND status IN ('submitted','reviewing','actioned')
                     FOR SHARE",
                )
                .bind(input.report)
                .bind(target.account_id.to_string())
                .bind(target.character_id.to_string())
                .fetch_optional(conn)
                .await?
                .ok_or(Error::Denied)?;
                Ok(())
            }),
        )
        .await?;
        self.authorize(&mut tx, actor, input.kind).await?;
        let now = clock(&mut tx).await?;
        let expires = expiry(input.kind, &input.expires, now)?;
        let recent: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM admin_sanction_reviews
             WHERE requested_by=$1 AND created_at>$2::timestamptz - interval '1 hour'",
        )
        .bind(actor.account)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        if recent >= 20 {
            return Err(Error::Limited);
        }
        let id = sqlx::query_scalar(
            "INSERT INTO admin_sanction_reviews
             (target_id,report_id,kind,reason,expires_at,requested_by)
             VALUES($1,$2,$3,$4,$5,$6)
             ON CONFLICT (target_id,report_id,kind) DO NOTHING RETURNING id",
        )
        .bind(input.target)
        .bind(input.report)
        .bind(input.kind)
        .bind(input.reason)
        .bind(expires)
        .bind(actor.account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Conflict)?;
        record(
            &mut tx,
            actor.account,
            id,
            input.kind,
            "sanctions.apply_review_requested",
            None,
            1,
        )
        .await?;
        self.authorize(&mut tx, actor, input.kind).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn advance(&self, actor: CheckContext, id: Uuid, input: Advance) -> Result<(), Error> {
        valid_id(id)?;
        if !(1..=4).contains(&input.revision)
            || (input.action == Action::Lift) != input.reason.is_some()
        {
            return Err(Error::BadInput);
        }
        let mut tx = self.pool.begin().await?;
        // Find the immutable target and expected requester without locking any
        // domain row. Acquire actor and requester locks in canonical order.
        let (target_id, requester): (Uuid, Uuid) = sqlx::query_as(
            "SELECT r.target_id,coalesce(r.lift_requested_by,r.requested_by)
             FROM admin_sanction_reviews r JOIN admin_sanction_review_grants g ON g.target_id=r.target_id
             WHERE r.id=$1 AND g.staff_id=$2",
        )
        .bind(id)
        .bind(actor.account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Denied)?;
        rbac::lock_accounts_resources_and_mutation_with::<Error, _>(
            &mut tx,
            &[actor.account, requester],
            |conn| {
                Box::pin(async move {
                    let target = lock_scope(&mut *conn, actor.account, target_id).await?;
                    if target.account_id == actor.account {
                        return Err(Error::Denied);
                    }
                    if input.action == Action::Approve {
                        lock_scope(&mut *conn, requester, target_id).await?;
                    }
                    sqlx::query("SELECT id FROM admin_sanction_reviews WHERE id=$1 FOR UPDATE")
                        .bind(id)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            },
        )
        .await?;
        let row: Review = sqlx::query_as("SELECT * FROM admin_sanction_reviews WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        self.authorize(&mut tx, actor, row.kind).await?;
        if row.requester() != requester {
            return Err(Error::Conflict);
        }
        let now = clock(&mut tx).await?;
        let next = transition(
            row.stage(),
            input.revision,
            input.action,
            requester,
            actor.account,
            row.eligible(now),
        )?;
        if input.action == Action::Approve {
            for permission in [
                Permission::Access,
                Permission::GameSanctionRead,
                row.kind.permission(),
            ] {
                self.auth
                    .check_subject_in(&mut tx, requester, permission)
                    .await?;
            }
        }
        let (action, statement) = match next {
            2 => (
                "sanctions.apply_review_approved",
                "UPDATE admin_sanction_reviews SET apply_approved_by=$2,apply_approved_at=$3 WHERE id=$1",
            ),
            3 => (
                "sanctions.lift_review_requested",
                "UPDATE admin_sanction_reviews SET lift_requested_by=$2,lift_requested_at=$3,lift_reason=$4 WHERE id=$1",
            ),
            4 => (
                "sanctions.lift_review_approved",
                "UPDATE admin_sanction_reviews SET lift_approved_by=$2,lift_approved_at=$3 WHERE id=$1",
            ),
            _ => return Err(Error::Conflict),
        };
        let change = sqlx::query(statement)
            .bind(id)
            .bind(actor.account)
            .bind(now);
        if let Some(reason) = input.reason {
            change.bind(reason).execute(&mut *tx).await?;
        } else {
            change.execute(&mut *tx).await?;
        }
        record(
            &mut tx,
            actor.account,
            id,
            row.kind,
            action,
            Some(row.stage()),
            next,
        )
        .await?;
        self.authorize(&mut tx, actor, row.kind).await?;
        if !row.eligible(clock(&mut tx).await?) {
            return Err(Error::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    id: Uuid,
    kind: Kind,
    action: &str,
    before: Option<u8>,
    after: u8,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before: before.map_or(
                serde_json::Value::Null,
                |revision| json!({"revision": revision}),
            ),
            after: json!({"kind": kind.key(), "revision": after, "authority": "platform_review"}),
        },
    )
    .await?;
    Ok(())
}

fn review_url(lang: &str, id: Uuid) -> String {
    format!("{PATH}?lang={lang}&id={id}")
}

async fn apply<A: AccountStateReader>(
    State(service): State<Arc<Service<A>>>,
    Extension(actor): Extension<CheckContext>,
    Form(input): Form<Apply>,
) -> Result<Response, Error> {
    let lang = locale(Some(&input.lang))?;
    Ok(match service.propose(actor, input).await {
        Ok(id) => Redirect::to(&review_url(lang, id)).into_response(),
        Err(error) => error.render(lang, format!("{PATH}?lang={lang}")),
    })
}

async fn advance<A: AccountStateReader>(
    State(service): State<Arc<Service<A>>>,
    Extension(actor): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(input): Form<Advance>,
) -> Result<Response, Error> {
    let lang = locale(Some(&input.lang))?;
    Ok(match service.advance(actor, id, input).await {
        Ok(()) => Redirect::to(&review_url(lang, id)).into_response(),
        Err(error) => error.render(lang, review_url(lang, id)),
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filter {
    lang: Option<String>,
    id: Option<Uuid>,
    before: Option<Uuid>,
}

struct ReviewView {
    id: Uuid,
    target: Target,
    report_id: Uuid,
    kind: &'static str,
    reason: String,
    expires: String,
    revision: u8,
    status: &'static str,
    approve: bool,
    lift: bool,
    events: Vec<Event>,
    lift_reason: Option<String>,
}

struct Event {
    action: &'static str,
    actor: Uuid,
    at: String,
}

fn stamp(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}

impl ReviewView {
    fn new(
        row: Review,
        target: Target,
        actor: Uuid,
        now: OffsetDateTime,
        can_change: bool,
    ) -> Self {
        let revision = row.stage();
        let eligible = row.eligible(now);
        let status = match revision {
            4 => "sanctions-lift-approved",
            _ if !eligible => "sanctions-expired",
            3 => "sanctions-lift-pending",
            2 => "sanctions-apply-approved",
            _ => "sanctions-apply-pending",
        };
        let kind = match row.kind {
            Kind::Ban => "sanctions-ban",
            Kind::Mute => "sanctions-mute",
            Kind::Jail => "sanctions-jail",
        };
        let events = [
            (
                Some(row.requested_by),
                Some(row.created_at),
                "sanctions-apply-requested",
            ),
            (
                row.apply_approved_by,
                row.apply_approved_at,
                "sanctions-apply-approved",
            ),
            (
                row.lift_requested_by,
                row.lift_requested_at,
                "sanctions-lift-pending",
            ),
            (
                row.lift_approved_by,
                row.lift_approved_at,
                "sanctions-lift-approved",
            ),
        ]
        .into_iter()
        .filter_map(|(actor, at, action)| {
            Some(Event {
                action,
                actor: actor?,
                at: stamp(at?),
            })
        })
        .collect();
        Self {
            id: row.id,
            target,
            report_id: row.report_id,
            kind,
            reason: format!("sanctions-{}", row.reason),
            expires: row.expires_at.map(stamp).unwrap_or_default(),
            revision,
            status,
            approve: can_change
                && eligible
                && matches!(revision, 1 | 3)
                && actor != row.requester(),
            lift: can_change && eligible && revision == 2,
            events,
            lift_reason: row
                .lift_reason
                .map(|reason| format!("sanctions-lift-{reason}")),
        }
    }
}

#[derive(Template)]
#[template(
    path = "sanction_review.html",
    config = "src/console/templates/askama.toml"
)]
struct Page {
    lang: &'static str,
    targets: Vec<Target>,
    reviews: Vec<ReviewView>,
    detail: bool,
    can_write: bool,
    can_ban: bool,
    can_read_dossier: bool,
    confirmed: bool,
    csrf: String,
    next: Option<Uuid>,
}
impl Page {
    fn text(&self, key: &str) -> String {
        tr(self.lang, key)
    }
}

async fn page<A: AccountStateReader>(
    State(service): State<Arc<Service<A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(filter): Query<Filter>,
) -> Result<Response, Error> {
    let lang = locale(filter.lang.as_deref())?;
    if filter.id.is_some() && filter.before.is_some() {
        return Err(Error::BadInput);
    }
    for id in [filter.id, filter.before].into_iter().flatten() {
        valid_id(id)?;
    }
    service
        .auth
        .check(actor, Permission::GameSanctionRead)
        .await?;
    let can_write = service
        .auth
        .check(actor, Permission::GameSanctionWrite)
        .await
        .is_ok();
    let can_ban = service
        .auth
        .check(actor, Kind::Ban.permission())
        .await
        .is_ok();
    let can_read_dossier = service
        .auth
        .check(actor, Permission::GamePlayerRead)
        .await
        .is_ok();
    let csrf = if can_write || can_ban {
        session.csrf_token().await.map_err(|_| Error::Unavailable)?
    } else {
        String::new()
    };
    let mut tx = service.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let confirmed = (can_write || can_ban) && recent_confirmation(&mut tx, actor).await?;
    let now = clock(&mut tx).await?;
    let targets = if filter.id.is_none() && (can_write || can_ban) {
        sqlx::query_as(
            "SELECT t.* FROM admin_sanction_review_targets t
             JOIN admin_sanction_review_grants g ON g.target_id=t.id
             WHERE g.staff_id=$1 AND t.account_id<>$1 ORDER BY t.id LIMIT 100",
        )
        .bind(actor.account)
        .fetch_all(&mut *tx)
        .await?
    } else {
        Vec::new()
    };
    let mut rows = sqlx::query_as::<_, Review>(
        "SELECT r.* FROM admin_sanction_review_grants g
         JOIN admin_sanction_reviews r ON r.target_id=g.target_id
         JOIN admin_sanction_review_targets t ON t.id=g.target_id
         WHERE g.staff_id=$1 AND t.account_id<>$1 AND ($2::uuid IS NULL OR r.id=$2)
         AND ($3::uuid IS NULL OR r.id<$3) ORDER BY r.id DESC LIMIT 51",
    )
    .bind(actor.account)
    .bind(filter.id)
    .bind(filter.before)
    .fetch_all(&mut *tx)
    .await?;
    if filter.id.is_some() && rows.is_empty() {
        return Err(Error::Denied);
    }
    let next = if rows.len() > 50 {
        rows.truncate(50);
        rows.last().map(|row| row.id)
    } else {
        None
    };
    let mut reviews = Vec::with_capacity(rows.len());
    let target_ids: Vec<_> = rows.iter().map(|row| row.target_id).collect();
    let reviewed_targets: std::collections::HashMap<_, _> =
        sqlx::query_as::<_, Target>("SELECT * FROM admin_sanction_review_targets WHERE id=ANY($1)")
            .bind(&target_ids)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|target| (target.id, target))
            .collect();
    for row in rows {
        let target = reviewed_targets
            .get(&row.target_id)
            .ok_or(Error::Unavailable)?
            .clone();
        let can_change = confirmed
            && if row.kind == Kind::Ban {
                can_ban
            } else {
                can_write
            };
        reviews.push(ReviewView::new(row, target, actor.account, now, can_change));
    }
    tx.commit().await?;
    let mut response = oteryn_web::render(&Page {
        lang,
        targets,
        reviews,
        detail: filter.id.is_some(),
        can_write,
        can_ban,
        can_read_dossier,
        confirmed,
        csrf,
        next,
    });
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    Ok(response)
}

#[cfg(test)]
#[path = "sanctions_tests.rs"]
mod tests;
