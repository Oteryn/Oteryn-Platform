//! Owner-scoped history for confirmed Platform sanctions.
use super::{Action, Appeals, Error, Status};
use crate::sanctions::valid_id;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) const PAGE_SIZE: i64 = 20;
pub(super) const LAST_PAGE: u32 = 10_000;

#[derive(Debug, sqlx::FromRow)]
pub(super) struct Sanction {
    pub id: Uuid,
    pub category: String,
    pub state: String,
    pub public_reason: String,
    pub effective_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub acknowledged_at: Option<OffsetDateTime>,
    pub appeal_status: String,
    pub message: Option<String>,
    pub outcome: Option<String>,
    pub version: i64,
}

const HISTORY: &str = "
    SELECT s.id, s.category, s.public_reason, s.effective_at, s.expires_at,
        CASE
            WHEN s.status = 'active' AND s.effective_at > statement_timestamp() THEN 'scheduled'
            WHEN s.status = 'active' AND s.expires_at <= statement_timestamp() THEN 'expired'
            ELSE s.status
        END AS state,
        COALESCE(a.acknowledged_at, s.acknowledged_at) AS acknowledged_at,
        COALESCE(a.status, 'none') AS appeal_status, a.message, a.outcome,
        s.lock_version + (COALESCE(a.version, 1) - 1) AS version
    FROM sanctions s
    LEFT JOIN support_appeal_workflows a ON a.sanction_id = s.id AND a.owner_id = s.account_id
    WHERE s.account_id = $1 AND s.status <> 'pending' AND ($2::uuid IS NULL OR s.id = $2)
    ORDER BY s.created_at DESC, s.id DESC LIMIT $3 OFFSET $4";

impl Sanction {
    pub fn can_appeal(&self) -> bool {
        Status::parse(&self.appeal_status).is_ok_and(Status::can_submit)
    }
}

pub(super) async fn history(pool: &PgPool, owner: Uuid, page: u32) -> Result<Vec<Sanction>, Error> {
    if !(1..=LAST_PAGE).contains(&page) {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    require_account(&mut tx, owner).await?;
    let history = sqlx::query_as(HISTORY)
        .bind(owner)
        .bind(None::<Uuid>)
        .bind(PAGE_SIZE + 1)
        .bind(i64::from(page - 1) * PAGE_SIZE)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(history)
}

pub(super) async fn detail(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<Sanction, Error> {
    valid_id(id).map_err(|_| Error::Invalid)?;
    let mut tx = pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    require_account(&mut tx, owner).await?;
    let sanction = sqlx::query_as(HISTORY)
        .bind(owner)
        .bind(id)
        .bind(1_i64)
        .bind(0_i64)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    tx.commit().await?;
    Ok(sanction)
}

pub(super) enum PlayerAction<'a> {
    Acknowledge,
    Appeal(&'a str),
}

pub(super) async fn change(
    pool: &PgPool,
    owner: Uuid,
    id: Uuid,
    expected: i64,
    action: PlayerAction<'_>,
) -> Result<(), Error> {
    valid_id(id).map_err(|_| Error::Invalid)?;
    if expected < 1 {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    // Use the same account, sanction, workflow lock order as staff enforcement.
    require_account(&mut tx, owner).await?;
    let (sanction_version, acknowledged_at): (i64, Option<OffsetDateTime>) = sqlx::query_as(
        "SELECT lock_version, acknowledged_at FROM sanctions
         WHERE id = $1 AND account_id = $2 AND status <> 'pending' FOR UPDATE",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound)?;

    // Older decisions can exist without an appeal row. Initialize on the first write.
    sqlx::query(
        "INSERT INTO support_appeal_workflows (sanction_id, owner_id, acknowledged_at)
         VALUES ($1, $2, $3) ON CONFLICT (sanction_id) DO NOTHING",
    )
    .bind(id)
    .bind(owner)
    .bind(acknowledged_at)
    .execute(&mut *tx)
    .await?;
    let (workflow_version, workflow_acknowledged): (i64, bool) = sqlx::query_as(
        "SELECT version, acknowledged FROM support_appeal_workflows
         WHERE sanction_id = $1 AND owner_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound)?;
    if matches!(action, PlayerAction::Acknowledge)
        && (workflow_acknowledged || acknowledged_at.is_some())
    {
        tx.rollback().await?;
        return Ok(());
    }
    if sanction_version.checked_add(workflow_version - 1) != Some(expected) {
        return Err(Error::Conflict);
    }
    Appeals { enabled: true }
        .change(
            &mut tx,
            owner,
            id,
            workflow_version,
            match action {
                PlayerAction::Acknowledge => Action::Acknowledge,
                PlayerAction::Appeal(message) => Action::Submit(message),
            },
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn require_account(db: &mut PgConnection, owner: Uuid) -> Result<(), Error> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM accounts WHERE id = $1 FOR SHARE")
            .bind(owner)
            .fetch_optional(db)
            .await?;
    if matches!(status.as_deref(), Some("active" | "suspended")) {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}

#[cfg(test)]
mod tests;
