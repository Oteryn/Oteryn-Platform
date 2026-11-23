use super::Error;
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationCheck, Permission},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

const VERSION: &str = "oteryn-audit-evidence-v1";
const RETENTION_DAYS: i32 = 90;
const MAX_ENTRIES: i32 = 25_000;

#[derive(Deserialize, Serialize)]
struct Record {
    id: Uuid,
    actor: Option<Uuid>,
    actor_kind: String,
    action: String,
    target: String,
    before_digest: String,
    after_digest: String,
    request_id: Uuid,
    created_at: String,
    admin_operation_id: Option<Uuid>,
    correlation_id: Option<Uuid>,
    ip: Option<String>,
    row_digest: String,
}

impl Record {
    fn validate(&self) -> Result<(), Error> {
        if time::OffsetDateTime::parse(
            &self.created_at,
            &time::format_description::well_known::Rfc3339,
        )
        .is_err()
            || self.id.is_nil()
            || self.request_id.is_nil()
            || self.actor.is_some_and(|id| id.is_nil())
            || self.admin_operation_id.is_some_and(|id| id.is_nil())
            || self.correlation_id.is_some_and(|id| id.is_nil())
            || (self.admin_operation_id.is_some() && self.actor.is_none())
            || self.action.trim().is_empty()
            || self.target.trim().is_empty()
            || self
                .ip
                .as_ref()
                .is_some_and(|ip| ip.parse::<std::net::IpAddr>().is_err())
        {
            return Err(Error::Integrity);
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct Link {
    sequence: usize,
    previous_hash: String,
    hash: String,
    record: Record,
}

#[derive(Serialize)]
struct Evidence<'a> {
    version: &'static str,
    day: &'a str,
    head: String,
    entries: Vec<Link>,
}

pub(super) async fn window(pool: &PgPool) -> Result<(String, String), sqlx::Error> {
    sqlx::query_as(
        "SELECT to_char(today - $1, 'YYYY-MM-DD'), to_char(today - 1, 'YYYY-MM-DD')
         FROM (SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date AS today) AS utc",
    )
    .bind(RETENTION_DAYS)
    .fetch_one(pool)
    .await
}

async fn lock_day(connection: &mut PgConnection, day: &str) -> Result<(), Error> {
    let eligible: bool = sqlx::query_scalar(
        "SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date - $1::text::date BETWEEN 1 AND $2",
    )
    .bind(day)
    .bind(RETENTION_DAYS)
    .fetch_one(&mut *connection)
    .await?;
    if !eligible {
        return Err(Error::OutsideWindow);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(20261124, $1::text::date - DATE '2000-01-01')")
        .bind(day)
        .execute(connection)
        .await?;
    Ok(())
}

async fn load(connection: &mut PgConnection, day: &str) -> Result<Vec<Record>, Error> {
    let rows: Vec<(Option<sqlx::types::Json<Record>>, Option<bool>)> = sqlx::query_as(
        "WITH entries AS (
             SELECT entry FROM audit_log AS entry
             WHERE coalesce(evidence_day, (created_at AT TIME ZONE 'UTC')::date) = $1::text::date
         ), records AS (
             SELECT id, digest FROM audit_evidence_records WHERE day = $1::text::date
         )
         SELECT CASE WHEN (entry).id IS NOT NULL AND records.id IS NOT NULL THEN jsonb_build_object(
             'id', (entry).id, 'actor', (entry).actor,
             'actor_kind', CASE WHEN (entry).actor IS NULL THEN 'system' ELSE 'account' END,
             'action', (entry).action, 'target', (entry).target,
             'before_digest', encode(sha256(convert_to((entry).before::text, 'UTF8')), 'hex'),
             'after_digest', encode(sha256(convert_to((entry).after::text, 'UTF8')), 'hex'),
             'request_id', (entry).request_id, 'created_at', to_char(
                 (entry).created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),
             'admin_operation_id', (entry).admin_operation_id,
             'correlation_id', (entry).correlation_id, 'ip', (entry).ip,
             'row_digest', encode(records.digest, 'hex')
         ) END, records.digest = audit_evidence_digest(entry)
         FROM entries FULL JOIN records ON (entry).id = records.id
         ORDER BY coalesce((entry).id, records.id)
         LIMIT $2",
    )
    .bind(day)
    .bind(MAX_ENTRIES + 1)
    .fetch_all(connection)
    .await?;
    if rows.len() > MAX_ENTRIES as usize {
        return Err(Error::Limit);
    }
    rows.into_iter()
        .map(|(record, valid)| {
            let (Some(record), Some(true)) = (record, valid) else {
                return Err(Error::Integrity);
            };
            record.0.validate()?;
            Ok(record.0)
        })
        .collect()
}

fn chain(day: &str, records: Vec<Record>) -> Result<(Evidence<'_>, [u8; 32]), Error> {
    let mut digest: [u8; 32] = Sha256::digest(format!("{VERSION}\n{day}")).into();
    let mut entries = Vec::with_capacity(records.len());
    for record in records {
        let previous_hash = format!("{:x}", sha2::digest::Output::<Sha256>::from(digest));
        let mut hash = Sha256::new();
        hash.update(previous_hash.as_bytes());
        hash.update(b"\n");
        hash.update(serde_json_canonicalizer::to_vec(&record)?);
        let next = hash.finalize();
        digest = next.into();
        entries.push(Link {
            sequence: entries.len() + 1,
            previous_hash,
            hash: format!("{next:x}"),
            record,
        });
    }
    Ok((
        Evidence {
            version: VERSION,
            day,
            head: format!("{:x}", sha2::digest::Output::<Sha256>::from(digest)),
            entries,
        },
        digest,
    ))
}

pub(super) async fn export(
    pool: &PgPool,
    authority: &dyn MutationCheck,
    actor: CheckContext,
    day: &str,
) -> Result<Vec<u8>, Error> {
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    const PERMISSIONS: &[Permission] = &[Permission::Access, Permission::ViewAudit];
    for &permission in PERMISSIONS {
        authority.check(&mut tx, actor, permission).await?;
    }
    let locked_day = day.to_owned();
    rbac::authorize_mutation_with_resources_domain(
        &mut tx,
        &[],
        |connection| Box::pin(async move { lock_day(connection, &locked_day).await }),
        authority,
        actor,
        PERMISSIONS,
    )
    .await?;
    let (evidence, digest) = chain(day, load(&mut tx, day).await?)?;
    let count = i32::try_from(evidence.entries.len()).map_err(|_| Error::Limit)?;
    let stored: Option<(Vec<u8>, i32)> = sqlx::query_as(
        "SELECT head, entry_count FROM audit_evidence_seals WHERE day = $1::text::date",
    )
    .bind(day)
    .fetch_optional(&mut *tx)
    .await?;
    match stored {
        Some((head, entry_count)) if head != digest || entry_count != count => {
            return Err(Error::Integrity);
        }
        Some(_) => {}
        None => {
            sqlx::query(
                "INSERT INTO audit_evidence_seals (day, head, entry_count)
                 VALUES ($1::text::date, $2, $3)",
            )
            .bind(day)
            .bind(digest.as_slice())
            .bind(count)
            .execute(&mut *tx)
            .await?;
        }
    }
    let request_id = Uuid::now_v7();
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor.account),
                request_id,
            },
            action: "audit.evidence.exported",
            target: day,
            before: serde_json::Value::Null,
            after: serde_json::json!({"head": evidence.head, "entry_count": count}),
        },
    )
    .await?;
    for &permission in PERMISSIONS {
        authority.check(&mut tx, actor, permission).await?;
    }
    let body = serde_json::to_vec(&evidence)?;
    tx.commit().await?;
    tracing::info!(
        target: "oteryn_admin::audit_anchor",
        version = VERSION, day, head = evidence.head, entry_count = count, %request_id,
        "Platform audit evidence anchor",
    );
    Ok(body)
}
