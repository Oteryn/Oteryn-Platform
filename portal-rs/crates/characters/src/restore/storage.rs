use super::{Error, QueueItem, Restore, wire};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) async fn lock(tx: &mut Transaction<'_, Postgres>, exclusive: bool) -> Result<(), Error> {
    let sql = if exclusive {
        "SELECT id FROM character_restore_locks WHERE id=1 FOR UPDATE"
    } else {
        "SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE"
    };
    if sqlx::query(sql).fetch_optional(&mut **tx).await?.is_none() {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(super) async fn alarm(
    pool: &PgPool,
    event: Option<Uuid>,
    operation: Option<Uuid>,
    reason: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO character_restore_queue(restore_event_id,operation_id,account_id,reason) VALUES ($1,$2,(SELECT account_id FROM character_commands WHERE id=$2),$3) ON CONFLICT DO NOTHING")
        .bind(event).bind(operation).bind(reason).execute(pool).await?;
    Ok(())
}
pub(super) async fn queue(pool: &PgPool) -> Result<Vec<QueueItem>, Error> {
    sqlx::query("SELECT id,restore_event_id,operation_id,account_id,reason FROM character_restore_queue WHERE resolved_at IS NULL ORDER BY id LIMIT 1000")
        .fetch_all(pool).await?.into_iter().map(|r| Ok(QueueItem { id:r.try_get("id")?, event_id:r.try_get("restore_event_id")?, operation_id:r.try_get("operation_id")?, account_id:r.try_get("account_id")?, reason:r.try_get("reason")? })).collect()
}
impl Restore {
    pub(super) async fn record(&self, notice: wire::Notice, raw: &[u8]) -> Result<String, Error> {
        let raw = std::str::from_utf8(raw).map_err(|_| Error::BadInput)?;
        let mut tx = self.pool.begin().await?;
        lock(&mut tx, true).await?;
        if let Some(row) = sqlx::query(
            "SELECT notice_json FROM character_restore_notices WHERE restore_event_id=$1",
        )
        .bind(notice.id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if row.try_get::<String, _>("notice_json")? != raw {
                tx.rollback().await?;
                alarm(
                    &self.pool,
                    Some(notice.id),
                    None,
                    "character_authority.restore_notice_conflict",
                )
                .await?;
                return Err(Error::Conflict);
            }
        } else {
            let stale:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_restore_notices WHERE new_recovery_generation >= $1)").bind(&notice.new_generation).fetch_one(&mut *tx).await?;
            if stale {
                tx.rollback().await?;
                alarm(
                    &self.pool,
                    Some(notice.id),
                    None,
                    "character_authority.restore_notice_stale",
                )
                .await?;
                return Err(Error::Conflict);
            }
            // Missing or unreadable history is unknown state: retain it for staff,
            // deny the notice, and never guess a generation from a timestamp.
            let ambiguous=sqlx::query("SELECT c.id FROM character_commands c LEFT JOIN character_restore_receipts r ON r.operation_id=c.id AND r.superseded_at IS NULL WHERE c.state='TERMINAL' AND (r.id IS NULL OR r.recovery_generation IS NULL OR r.receipt_json IS DISTINCT FROM c.receipt_json OR character_restore_generation(r.receipt_json) IS DISTINCT FROM r.recovery_generation) OR (c.state='PENDING' AND r.id IS NOT NULL) UNION SELECT c.id FROM character_restore_reopens reopen JOIN character_commands c ON c.id=reopen.operation_id WHERE reopen.new_receipt_id IS NULL AND (c.state<>'PENDING' OR c.receipt_json IS NOT NULL) UNION SELECT operation_id AS id FROM character_restore_reopens WHERE new_receipt_id IS NULL GROUP BY operation_id HAVING count(DISTINCT superseded_receipt_id)>1").fetch_all(&mut *tx).await?;
            if !ambiguous.is_empty() {
                tx.rollback().await?;
                for row in ambiguous {
                    alarm(
                        &self.pool,
                        Some(notice.id),
                        Some(row.try_get("id")?),
                        "character_authority.restore_state_ambiguous",
                    )
                    .await?;
                }
                return Err(Error::Unavailable);
            }
            sqlx::query("INSERT INTO character_restore_notices(restore_event_id,notice_json,new_recovery_generation) VALUES ($1,$2,$3)").bind(notice.id).bind(raw).bind(&notice.new_generation).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO character_restore_reopens(restore_event_id,operation_id,superseded_receipt_id) SELECT DISTINCT ON (r.operation_id) $1,r.operation_id,r.superseded_receipt_id FROM character_restore_reopens r JOIN character_commands c ON c.id=r.operation_id WHERE r.new_receipt_id IS NULL AND c.state='PENDING' ORDER BY r.operation_id,r.created_at,r.restore_event_id ON CONFLICT DO NOTHING").bind(notice.id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO character_restore_reopens(restore_event_id,operation_id,superseded_receipt_id) SELECT $1,c.id,r.id FROM character_commands c JOIN character_restore_receipts r ON r.operation_id=c.id AND r.superseded_at IS NULL WHERE c.state='TERMINAL' AND r.recovery_generation >= $2 ON CONFLICT DO NOTHING").bind(notice.id).bind(&notice.restored).execute(&mut *tx).await?;
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM character_restore_reopens WHERE restore_event_id=$1",
            )
            .bind(notice.id)
            .fetch_one(&mut *tx)
            .await?;
            if count > 0 && !self.settings.exchange_available {
                tx.rollback().await?;
                alarm(
                    &self.pool,
                    Some(notice.id),
                    None,
                    "character_authority.restore_notice_exchange_unavailable",
                )
                .await?;
                return Err(Error::Unavailable);
            }
            sqlx::query("UPDATE character_commands SET receipt_json=NULL,state='PENDING',updated_at=clock_timestamp() WHERE state='TERMINAL' AND id IN (SELECT operation_id FROM character_restore_reopens WHERE restore_event_id=$1)").bind(notice.id).execute(&mut *tx).await?;
            sqlx::query(
                "UPDATE character_restore_notices SET reopened_count=$2 WHERE restore_event_id=$1",
            )
            .bind(notice.id)
            .bind(count)
            .execute(&mut *tx)
            .await?;
        }
        let row=sqlx::query("SELECT reopened_count,(SELECT count(*) FROM character_restore_reopens WHERE restore_event_id=$1 AND new_receipt_id IS NULL) AS outstanding FROM character_restore_notices WHERE restore_event_id=$1").bind(notice.id).fetch_one(&mut *tx).await?;
        let outstanding: i64 = row.try_get("outstanding")?;
        if outstanding == 0 {
            sqlx::query("UPDATE character_restore_notices SET acknowledged_at=coalesce(acknowledged_at,clock_timestamp()) WHERE restore_event_id=$1").bind(notice.id).execute(&mut *tx).await?;
        }
        let response = wire::status(notice.id, row.try_get("reopened_count")?, outstanding);
        tx.commit().await?;
        Ok(response)
    }
}
