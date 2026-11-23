//! Account-owned goals and calendar reminders. Game observations do not change them.
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use time::{Date, Duration};
use uuid::Uuid;

const LIMIT: i64 = 30;
pub(super) const KINDS: &[&str] = &["level", "skill", "bestiary", "quest", "boss", "stamina"];

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("invalid goal")]
    Invalid,
    #[error("goal limit reached")]
    Limit,
    #[error("active account required")]
    Denied,
    #[error("goal unavailable")]
    Missing,
    #[error("journal storage unavailable")]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, Deserialize)]
pub(super) struct NewGoal {
    pub request_id: Uuid,
    pub kind: String,
    pub title: String,
    pub remind_on: String,
}

#[derive(sqlx::FromRow)]
pub(super) struct Goal {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub remind_on: Option<Date>,
    pub completed: bool,
    pub due: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    Complete,
    Delete,
}

impl NewGoal {
    fn validate(&self, today: Date) -> Result<(&str, Option<Date>), Error> {
        let title = self.title.trim();
        if !super::id(&self.request_id.to_string())
            || !KINDS.contains(&self.kind.as_str())
            || !(1..=120).contains(&title.chars().count())
            || title.chars().any(char::is_control)
        {
            return Err(Error::Invalid);
        }
        if self.remind_on.is_empty() {
            return Ok((title, None));
        }
        let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
            .map_err(|_| Error::Invalid)?;
        let date = Date::parse(&self.remind_on, &format).map_err(|_| Error::Invalid)?;
        if date.to_string() != self.remind_on || date < today || date > today + Duration::days(365)
        {
            return Err(Error::Invalid);
        }
        Ok((title, Some(date)))
    }
}

async fn lock_owner(tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<(), Error> {
    // Serialize per-account limits and lifecycle changes with goal creation.
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM accounts WHERE id=$1 AND status='active'
         AND email_verified_at IS NOT NULL FOR UPDATE",
    )
    .bind(account)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::Denied)?;
    Ok(())
}

pub(super) async fn list(pool: &PgPool, account: Uuid) -> Result<Vec<Goal>, Error> {
    let mut tx = pool.begin().await?;
    lock_owner(&mut tx, account).await?;
    let rows = sqlx::query_as(
        "SELECT id,kind,title,remind_on,completed_at IS NOT NULL AS completed,
         COALESCE(remind_on <= (now() AT TIME ZONE 'UTC')::date
                  AND completed_at IS NULL, false) AS due
         FROM pd_journal_goals WHERE account_id=$1
         ORDER BY completed, due DESC, remind_on NULLS LAST, created_at DESC, id",
    )
    .bind(account)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows)
}

pub(super) async fn create(pool: &PgPool, account: Uuid, form: NewGoal) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    lock_owner(&mut tx, account).await?;
    let today = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'UTC')::date")
        .fetch_one(&mut *tx)
        .await?;
    let (title, date) = form.validate(today)?;
    let replay: Option<bool> = sqlx::query_scalar(
        "SELECT kind=$3 AND title=$4 AND remind_on IS NOT DISTINCT FROM $5
         FROM pd_journal_goals WHERE id=$1 AND account_id=$2",
    )
    .bind(form.request_id)
    .bind(account)
    .bind(&form.kind)
    .bind(title)
    .bind(date)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(matches) = replay {
        return if matches { Ok(()) } else { Err(Error::Invalid) };
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pd_journal_goals WHERE account_id=$1")
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
    if count >= LIMIT {
        return Err(Error::Limit);
    }
    let inserted = sqlx::query(
        "INSERT INTO pd_journal_goals(id,account_id,kind,title,remind_on) VALUES($1,$2,$3,$4,$5)
         ON CONFLICT(account_id,id) DO NOTHING",
    )
    .bind(form.request_id)
    .bind(account)
    .bind(&form.kind)
    .bind(title)
    .bind(date)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(Error::Invalid);
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn change(
    pool: &PgPool,
    account: Uuid,
    goal: Uuid,
    action: Action,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    lock_owner(&mut tx, account).await?;
    let sql = match action {
        Action::Complete => {
            "UPDATE pd_journal_goals SET completed_at=COALESCE(completed_at,now()) WHERE account_id=$1 AND id=$2"
        }
        Action::Delete => "DELETE FROM pd_journal_goals WHERE account_id=$1 AND id=$2",
    };
    if sqlx::query(sql)
        .bind(account)
        .bind(goal)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        != 1
    {
        return Err(Error::Missing);
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests;
