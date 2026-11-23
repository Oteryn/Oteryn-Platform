use super::{
    Error,
    model::{self, DraftForm},
};
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct Scope {
    pub(super) world: Uuid,
    pub(super) name: String,
    pub(super) action: String,
}

impl Scope {
    pub(super) fn key(&self) -> String {
        format!("{}/{}", self.world, self.action)
    }
}

pub(super) async fn scopes(pool: &PgPool, actor: Uuid) -> Result<Vec<Scope>, Error> {
    Ok(sqlx::query_as(
        "SELECT w.id AS world,w.name,g.action FROM world_operation_planning_grants g
         JOIN world_status w ON w.id=g.world_id WHERE g.account_id=$1 ORDER BY w.name,w.id,g.action",
    )
    .bind(actor)
    .fetch_all(pool)
    .await?)
}

#[derive(sqlx::FromRow)]
pub(super) struct Plan {
    pub(super) id: Uuid,
    pub(super) world_id: Uuid,
    pub(super) world_name: String,
    pub(super) action: String,
    pub(super) requester: Uuid,
    pub(super) starts: i64,
    pub(super) ends: i64,
    pub(super) ticket: String,
    pub(super) target: Option<Uuid>,
    pub(super) event: Option<Uuid>,
    pub(super) locale: Option<String>,
    pub(super) message: Option<String>,
    pub(super) reviewer: Option<Uuid>,
    pub(super) approved: Option<bool>,
    pub(super) reviewed_at: Option<i64>,
    pub(super) can_review: bool,
}

impl Plan {
    pub(super) fn status(&self, now: i64) -> &'static str {
        match self.approved {
            None if now >= self.starts => "ops-expired",
            None => "ops-pending",
            Some(false) => "ops-declined",
            Some(true) if now >= self.ends => "ops-expired",
            Some(true) => "ops-approved",
        }
    }
}

pub(super) const PAGE_SIZE: usize = 25;

pub(super) async fn plans(
    pool: &PgPool,
    actor: Uuid,
    after: Option<Uuid>,
) -> Result<Vec<Plan>, Error> {
    Ok(sqlx::query_as(
        "SELECT p.*,w.name AS world_name,g.can_review FROM world_operation_plans p
         JOIN world_operation_planning_grants g
           ON g.account_id=$1 AND (g.world_id,g.action)=(p.world_id,p.action)
         JOIN world_status w ON w.id=p.world_id
         WHERE ($2::uuid IS NULL OR p.id<$2) ORDER BY p.id DESC LIMIT $3",
    )
    .bind(actor)
    .bind(after)
    .bind((PAGE_SIZE + 1) as i64)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn plan(pool: &PgPool, actor: Uuid, id: Uuid) -> Result<Plan, Error> {
    sqlx::query_as(
        "SELECT p.*,w.name AS world_name,g.can_review FROM world_operation_plans p
         JOIN world_operation_planning_grants g
           ON g.account_id=$1 AND (g.world_id,g.action)=(p.world_id,p.action)
         JOIN world_status w ON w.id=p.world_id WHERE p.id=$2",
    )
    .bind(actor)
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)
}

pub(super) async fn now(connection: &mut PgConnection) -> Result<i64, Error> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(connection)
            .await?,
    )
}

async fn check_scope(
    connection: &mut PgConnection,
    actor: Uuid,
    world: Uuid,
    action: &str,
    review: bool,
) -> Result<(), Error> {
    let grant: Option<bool> = sqlx::query_scalar(
        "SELECT can_review FROM world_operation_planning_grants
         WHERE account_id=$1 AND world_id=$2 AND action=$3 FOR SHARE",
    )
    .bind(actor)
    .bind(world)
    .bind(action)
    .fetch_optional(connection)
    .await?;
    match grant {
        Some(can_review) if !review || can_review => Ok(()),
        _ => Err(Error::Denied),
    }
}

pub(super) async fn create<A: AccountStateReader>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    context: CheckContext,
    form: DraftForm,
) -> Result<Uuid, Error> {
    let (starts, ends) = form.validate()?;
    let mut tx = pool.begin().await?;
    rbac::lock_accounts_and_mutation(&mut tx, &[context.account]).await?;
    auth.check_in(&mut tx, context, Permission::Access).await?;
    let action = form.action.key();
    check_scope(&mut tx, context.account, form.world, action, false).await?;
    model::check_schedule(starts, now(&mut tx).await?)?;

    let saved = sqlx::query(
        "INSERT INTO world_operation_plans
          (id,world_id,action,requester,starts,ends,ticket,target,event,locale,message)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT (id) DO NOTHING",
    )
    .bind(form.id)
    .bind(form.world)
    .bind(action)
    .bind(context.account)
    .bind(starts)
    .bind(ends)
    .bind(form.ticket.trim())
    .bind(form.target)
    .bind(form.event)
    .bind(form.locale.map(model::MessageLocale::key))
    .bind(form.message.as_deref().map(str::trim))
    .execute(&mut *tx)
    .await?;
    if saved.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "admin.world_operations.proposed",
            target: &form.id.to_string(),
            before: json!(null),
            after: json!({"action": action, "world": form.world, "starts": starts, "ends": ends}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(form.id)
}

pub(super) async fn review<A: AccountStateReader>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    context: CheckContext,
    id: Uuid,
    approve: bool,
) -> Result<(), Error> {
    // Requester and scope are immutable. The decision itself is re-read after locks.
    let selected = plan(pool, context.account, id).await?;
    let mut tx = pool.begin().await?;
    rbac::lock_accounts_and_mutation(&mut tx, &[context.account, selected.requester]).await?;
    auth.check_in(&mut tx, context, Permission::Access).await?;
    check_scope(
        &mut tx,
        context.account,
        selected.world_id,
        &selected.action,
        true,
    )
    .await?;
    auth.check_subject_in(&mut tx, selected.requester, Permission::Access)
        .await?;
    check_scope(
        &mut tx,
        selected.requester,
        selected.world_id,
        &selected.action,
        false,
    )
    .await?;

    let reviewed: bool = sqlx::query_scalar(
        "SELECT reviewer IS NOT NULL FROM world_operation_plans WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let reviewed_at = now(&mut tx).await?;
    model::check_review(
        selected.requester,
        context.account,
        selected.starts,
        reviewed_at,
        reviewed,
    )?;

    sqlx::query(
        "UPDATE world_operation_plans SET reviewer=$2,approved=$3,reviewed_at=$4 WHERE id=$1",
    )
    .bind(id)
    .bind(context.account)
    .bind(approve)
    .bind(reviewed_at)
    .execute(&mut *tx)
    .await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext { actor: Some(context.account), request_id: Uuid::now_v7() },
            action: "admin.world_operations.reviewed",
            target: &id.to_string(),
            before: json!({"approved": null}),
            after: json!({"approved": approve, "reviewed_at": reviewed_at, "world": selected.world_id,
                "action": selected.action, "requester": selected.requester}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
