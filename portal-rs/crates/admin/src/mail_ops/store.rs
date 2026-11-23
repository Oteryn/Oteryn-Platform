use super::{Error, Filters, KINDS};
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) const PAGE_SIZE: usize = 40;
pub(super) const BATCH_LIMIT: u8 = 50;

#[derive(sqlx::FromRow)]
pub(super) struct Job {
    pub id: Uuid,
    pub state: String,
    pub kind: String,
    pub recipient: Option<String>,
    pub has_recipient: bool,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: String,
}

pub(super) async fn list(
    pool: &PgPool,
    filters: &Filters,
    reveal: bool,
) -> Result<Vec<Job>, Error> {
    Ok(sqlx::query_as(
        "SELECT id,state,kind,CASE WHEN $1 THEN recipient END AS recipient,
            recipient IS NOT NULL AS has_recipient,attempts,last_error,
            to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS created_at
         FROM admin_mail_outbox
         WHERE ($2 = '' OR state = $2) AND ($3 = '' OR kind = $3)
           AND (NULLIF($4,'') IS NULL OR created_at >= NULLIF($4,'')::date::timestamp AT TIME ZONE 'UTC')
           AND (NULLIF($5,'') IS NULL OR created_at < (NULLIF($5,'')::date + 1)::timestamp AT TIME ZONE 'UTC')
           AND ($6 = '' OR lower(recipient) = $6)
           AND ($7::uuid IS NULL OR (created_at,id) < (SELECT created_at,id FROM outbox WHERE id=$7))
         ORDER BY created_at DESC,id DESC LIMIT $8",
    )
    .bind(reveal)
    .bind(&filters.state)
    .bind(&filters.kind)
    .bind(&filters.from)
    .bind(&filters.to)
    .bind(&filters.recipient)
    .bind(filters.after)
    .bind((PAGE_SIZE + 1) as i64)
    .fetch_all(pool)
    .await?)
}

#[derive(sqlx::FromRow)]
pub(super) struct Count {
    pub state: String,
    pub day: i64,
    pub week: i64,
}

pub(super) async fn counts(pool: &PgPool) -> Result<Vec<Count>, Error> {
    Ok(sqlx::query_as(
        "WITH totals AS (
            SELECT state, count(*) FILTER (WHERE created_at >= now() - interval '24 hours') AS day,
                count(*) AS week
            FROM admin_mail_outbox WHERE created_at >= now() - interval '168 hours' AND created_at <= now()
            GROUP BY state
         )
         SELECT states.state, coalesce(day,0) AS day, coalesce(week,0) AS week
         FROM unnest($1::text[]) WITH ORDINALITY AS states(state, position)
         LEFT JOIN totals USING (state) ORDER BY states.position",
    )
    .bind(super::STATES)
    .fetch_all(pool)
    .await?)
}

pub(super) enum Action<'a> {
    Retry(Uuid),
    RetryKind(&'a str),
    Cancel(Uuid),
}

impl Action<'_> {
    fn permission(&self) -> Permission {
        match self {
            Self::Retry(_) => Permission::MailRetry,
            Self::RetryKind(_) => Permission::MailRetryBatch,
            Self::Cancel(_) => Permission::MailCancel,
        }
    }
}

pub(super) async fn apply(
    pool: &PgPool,
    actor: CheckContext,
    auth: &MutationAuthorization,
    action: Action<'_>,
) -> Result<u8, Error> {
    let (id, kind, cancel, event, before) = match action {
        Action::Retry(id) if !id.is_nil() => {
            (Some(id), None, false, "mail.outbox.retried", "failed")
        }
        Action::RetryKind(kind) if KINDS.contains(&kind) => (
            None,
            Some(kind),
            false,
            "mail.outbox.batch_retried",
            "failed",
        ),
        Action::Cancel(id) if !id.is_nil() => {
            (Some(id), None, true, "mail.outbox.cancelled", "pending")
        }
        _ => return Err(Error::Invalid),
    };
    let permissions = [
        Permission::Access,
        Permission::MailView,
        action.permission(),
    ];
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    rbac::authorize_mutation(&mut tx, &[], auth, actor, &permissions).await?;

    // Pending rows are the only candidates. Lock the actual worker rows before
    // changing them so an in-flight attempt always wins over an operator action.
    let jobs: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "WITH candidates AS (
            SELECT o.id, m.kind FROM outbox o JOIN admin_mail_outbox m ON m.id = o.id
            WHERE o.kind IN ('email.account_exists', 'email.verify', 'email.password-reset', 'notify.email')
              AND o.state = 'pending' AND (o.last_error IS NULL) = $1
              AND ($2::uuid IS NULL OR o.id = $2) AND ($3::text IS NULL OR m.kind = $3)
            ORDER BY o.created_at,o.id LIMIT $4 FOR UPDATE OF o SKIP LOCKED
         )
         UPDATE outbox o
         SET state = CASE WHEN $1 THEN 'dead' ELSE 'pending' END,
             mail_cancelled_at = CASE WHEN $1 THEN now() END,
             last_error = NULL, run_after = now(), updated_at = now()
         FROM candidates c WHERE o.id = c.id
         RETURNING o.id,c.kind,o.attempts",
    )
    .bind(cancel)
    .bind(id)
    .bind(kind)
    .bind(i64::from(BATCH_LIMIT))
    .fetch_all(&mut *tx)
    .await?;
    if jobs.is_empty() {
        return Err(Error::Conflict);
    }

    let context = AuditContext {
        actor: Some(actor.account),
        request_id: Uuid::now_v7(),
    };
    let after = if cancel { "cancelled" } else { "pending" };
    for (job, kind, attempts) in &jobs {
        audit::append(
            &mut tx,
            Entry {
                context,
                action: event,
                target: &job.to_string(),
                before: json!({"kind":kind,"state":before,"attempts":attempts}),
                after: json!({"kind":kind,"state":after,"attempts":attempts}),
            },
        )
        .await?;
    }
    let changed = u8::try_from(jobs.len()).map_err(|_| Error::Invalid)?;
    rbac::mutation_budget(&mut tx).await?;
    for permission in permissions {
        auth.check(&mut tx, actor, permission).await?;
    }
    tx.commit().await?;
    Ok(changed)
}
