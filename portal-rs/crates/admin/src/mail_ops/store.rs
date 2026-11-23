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

pub(super) enum Quota {
    Read,
    Change,
    Batch,
}

pub(super) async fn charge(pool: &PgPool, actor: Uuid, quota: Quota) -> Result<(), Error> {
    let (name, limit) = match quota {
        Quota::Read => ("mail-outbox-read", 60_i64),
        Quota::Change => ("mail-outbox-change", 30),
        Quota::Batch => ("mail-outbox-batch", 5),
    };
    if let Some(retry_after) = crate::web::charge_staff_quota(pool, actor, name, limit).await? {
        return Err(Error::Limited(retry_after));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
pub(super) struct Job {
    pub id: Uuid,
    pub state: String,
    pub kind: String,
    pub recipient: Option<String>,
    pub has_recipient: bool,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: time::OffsetDateTime,
    pub queued_at: String,
}

pub(super) async fn list(
    pool: &PgPool,
    filters: &Filters,
    reveal: bool,
) -> Result<Vec<Job>, Error> {
    let (created_at, id) = filters.after.map_or((None, None), |cursor| {
        (Some(cursor.created_at), Some(cursor.id))
    });
    Ok(sqlx::query_as(
        "SELECT id,state,kind,CASE WHEN $1 THEN recipient END AS recipient,
            recipient IS NOT NULL AS has_recipient,attempts,last_error,
            created_at, to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS queued_at
         FROM admin_mail_outbox
         WHERE kind <> 'notify.operational' AND ($2 = '' OR state = $2) AND ($3 = '' OR kind = $3)
           AND (NULLIF($4,'') IS NULL OR created_at >= NULLIF($4,'')::date::timestamp AT TIME ZONE 'UTC')
           AND (NULLIF($5,'') IS NULL OR created_at < (NULLIF($5,'')::date + 1)::timestamp AT TIME ZONE 'UTC')
           AND (NOT $1 OR $6 = '' OR lower(recipient) = $6)
           AND ($7::timestamptz IS NULL OR (created_at,id) < ($7,$8::uuid))
         ORDER BY created_at DESC,id DESC LIMIT $9",
    )
    .bind(reveal)
    .bind(&filters.state)
    .bind(&filters.kind)
    .bind(&filters.from)
    .bind(&filters.to)
    .bind(&filters.recipient)
    .bind(created_at)
    .bind(id)
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
            FROM admin_mail_outbox WHERE kind <> 'notify.operational' AND created_at >= now() - interval '168 hours' AND created_at <= now()
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
    Retry(Uuid, i32),
    RetryKind(&'a str),
    Cancel(Uuid, i32),
}

impl Action<'_> {
    pub(super) fn quota(&self) -> Quota {
        match self {
            Self::RetryKind(_) => Quota::Batch,
            _ => Quota::Change,
        }
    }

    fn permission(&self) -> Permission {
        match self {
            Self::Retry(..) => Permission::MailRetry,
            Self::RetryKind(_) => Permission::MailRetryBatch,
            Self::Cancel(..) => Permission::MailCancel,
        }
    }
}

pub(super) async fn changed(
    pool: &PgPool,
    actor: Uuid,
    request_id: Uuid,
) -> Result<Option<u8>, Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND request_id=$2
         AND action IN ('mail.outbox.retried','mail.outbox.batch_retried','mail.outbox.cancelled')",
    )
    .bind(actor)
    .bind(request_id)
    .fetch_one(pool)
    .await?;
    Ok(u8::try_from(count)
        .ok()
        .filter(|count| (1..=BATCH_LIMIT).contains(count)))
}

pub(super) async fn apply(
    pool: &PgPool,
    actor: CheckContext,
    auth: &MutationAuthorization,
    action: Action<'_>,
    request_id: Uuid,
) -> Result<u8, Error> {
    if request_id.is_nil() {
        return Err(Error::Invalid);
    }
    let (id, attempts, kind, cancel, event, before) = match action {
        Action::Retry(id, attempts) if !id.is_nil() && attempts >= 0 => (
            Some(id),
            Some(attempts),
            None,
            false,
            "mail.outbox.retried",
            "failed",
        ),
        Action::RetryKind(kind) if KINDS.contains(&kind) => (
            None,
            None,
            Some(kind),
            false,
            "mail.outbox.batch_retried",
            "failed",
        ),
        Action::Cancel(id, attempts) if !id.is_nil() && attempts >= 0 => (
            Some(id),
            Some(attempts),
            None,
            true,
            "mail.outbox.cancelled",
            "pending",
        ),
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

    if let Some(kind) = kind {
        let (count, matches): (i64, Option<bool>) = sqlx::query_as(
            "SELECT count(*),bool_and(after->>'kind'=$4) FROM audit_log
             WHERE actor=$1 AND request_id=$2 AND action=$3",
        )
        .bind(actor.account)
        .bind(request_id)
        .bind(event)
        .bind(kind)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(matches) = matches {
            return if matches {
                u8::try_from(count).map_err(|_| Error::Invalid)
            } else {
                Err(Error::Conflict)
            };
        }
    }

    // Pending rows are the only candidates. Lock the actual worker rows before
    // changing them so an in-flight attempt always wins over an operator action.
    let jobs: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "WITH candidates AS (
            SELECT o.id, m.kind FROM outbox o JOIN admin_mail_outbox m ON m.id = o.id
            WHERE o.kind IN ('email.account_exists', 'email.verify', 'email.password-reset', 'notify.email')
              AND m.kind <> 'notify.operational'
              AND o.state = 'pending' AND (o.last_error IS NULL) = $1
              AND ($2::uuid IS NULL OR o.id = $2) AND ($3::text IS NULL OR m.kind = $3)
              AND ($5::integer IS NULL OR o.attempts = $5)
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
    .bind(attempts)
    .fetch_all(&mut *tx)
    .await?;
    if jobs.is_empty() {
        return Err(Error::Conflict);
    }

    let context = AuditContext {
        actor: Some(actor.account),
        request_id,
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
