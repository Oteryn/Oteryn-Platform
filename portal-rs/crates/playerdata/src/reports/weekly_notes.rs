//! Owner-authored notes on a completed UTC week.
use super::{Error, Reports, Result, Week, validate_note_body};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(super) struct WeeklyNote {
    id: Uuid,
    week_start: i64,
    pub body: String,
}

impl Reports {
    pub(super) async fn weekly_note(
        &self,
        account: Uuid,
        week: Week,
    ) -> Result<Option<WeeklyNote>> {
        Ok(sqlx::query_as(
            "SELECT id,week_start,body FROM pd_weekly_notes WHERE account_id=$1 AND week_start=$2",
        )
        .bind(account)
        .bind(week.start)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(super) async fn save_weekly_note(
        &self,
        account: Uuid,
        week: Week,
        body: &str,
    ) -> Result<()> {
        validate_note_body(body)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR SHARE",
        )
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Denied)?;

        if body.trim().is_empty() {
            sqlx::query("DELETE FROM pd_weekly_notes WHERE account_id=$1 AND week_start=$2")
                .bind(account)
                .bind(week.start)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO pd_weekly_notes(id,account_id,week_start,body) VALUES($1,$2,$3,$4)
                 ON CONFLICT(account_id,week_start) DO UPDATE SET body=excluded.body",
            )
            .bind(Uuid::now_v7())
            .bind(account)
            .bind(week.start)
            .bind(body)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(super) async fn export_weekly_notes(
        &self,
        account: Uuid,
        id: Option<Uuid>,
    ) -> Result<Vec<WeeklyNote>> {
        if let Some(id) = id {
            if id.get_version_num() != 7 {
                return Err(Error::Invalid);
            }
            let note = sqlx::query_as(
                "SELECT id,week_start,body FROM pd_weekly_notes WHERE account_id=$1 AND id=$2",
            )
            .bind(account)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::Denied)?;
            return Ok(vec![note]);
        }
        let notes = sqlx::query_as(
            "SELECT id,week_start,body FROM pd_weekly_notes WHERE account_id=$1
             ORDER BY week_start DESC LIMIT 10001",
        )
        .bind(account)
        .fetch_all(&self.pool)
        .await?;
        if notes.len() > 10000 {
            return Err(Error::Unavailable);
        }
        Ok(notes)
    }
}
