use super::*;
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use time::{Date, PrimitiveDateTime};

async fn translation_snapshot(
    db: &mut PgConnection,
    event: Uuid,
) -> Result<Map<String, Value>, sqlx::Error> {
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT locale, slug, title, summary, body FROM event_translations WHERE event_id=$1",
    )
    .bind(event)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(locale, slug, title, summary, body)| {
            let fingerprint = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
            (
                locale,
                json!({
                    "slug": slug,
                    "title_sha256": fingerprint(&title),
                    "summary_sha256": fingerprint(&summary),
                    "body_sha256": fingerprint(&body),
                }),
            )
        })
        .collect())
}

#[derive(Debug, Clone)]
pub struct Events {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub(super) struct ZonedEvent {
    #[sqlx(flatten)]
    pub event: PublicEvent,
    pub local_start: PrimitiveDateTime,
    pub local_end: PrimitiveDateTime,
}

#[derive(sqlx::FromRow)]
pub(super) struct CalendarEvent {
    pub title: String,
    pub slug: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub local_start: PrimitiveDateTime,
    pub local_end: PrimitiveDateTime,
    pub last_day: Date,
}

#[derive(sqlx::FromRow)]
pub(super) struct ExportEvent {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Events {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn public(
        &self,
        locale: &str,
        slug: Option<&str>,
    ) -> Result<Vec<PublicEvent>, Error> {
        Ok(self
            .public_page(locale, slug, 0, 200, "UTC")
            .await?
            .into_iter()
            .map(|row| row.event)
            .collect())
    }
    pub(super) async fn public_page(
        &self,
        locale: &str,
        slug: Option<&str>,
        offset: i64,
        limit: i64,
        zone: &str,
    ) -> Result<Vec<ZonedEvent>, Error> {
        if !matches!(locale, "pl" | "en") || slug.is_some_and(|s| !valid_slug(s)) {
            return Err(Error::Invalid);
        }
        Ok(sqlx::query_as(
            "SELECT e.id,t.title,t.slug,t.summary,t.body,e.starts_at,e.ends_at,
                    e.starts_at AT TIME ZONE $5 AS local_start,
                    e.ends_at AT TIME ZONE $5 AS local_end
             FROM events e JOIN event_translations t ON t.event_id=e.id
             WHERE e.status='published' AND e.published_by IS NOT NULL
               AND t.locale=$1 AND ($2::text IS NULL OR t.slug=$2)
             ORDER BY (e.ends_at>clock_timestamp()) DESC,e.featured DESC,e.starts_at,e.id
             LIMIT $4 OFFSET $3",
        )
        .bind(locale)
        .bind(slug)
        .bind(offset)
        .bind(limit)
        .bind(zone)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(super) async fn time_zones(&self, name: Option<&str>) -> Result<Vec<String>, Error> {
        // Use the same tzdata for the selector and for conversion. Require a
        // geographic name (or UTC) so PostgreSQL does not resolve abbreviations.
        Ok(sqlx::query_scalar(
            "SELECT name FROM pg_timezone_names
             WHERE ($1::text IS NULL OR name=$1)
               AND (name='UTC' OR name ~ '^[A-Za-z0-9_+-]+(/[A-Za-z0-9_+-]+)+$')
               AND name NOT LIKE 'posix/%' AND name NOT LIKE 'right/%'
             ORDER BY (name='UTC') DESC, name",
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(super) async fn local_today(&self, zone: &str) -> Result<Date, Error> {
        Ok(
            sqlx::query_scalar("SELECT (CURRENT_TIMESTAMP AT TIME ZONE $1)::date")
                .bind(zone)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub(super) async fn calendar_events(
        &self,
        locale: &str,
        zone: &str,
        first: Date,
        next: Date,
    ) -> Result<Vec<CalendarEvent>, Error> {
        // Pad the UTC window so both occurrences of a repeated local hour
        // remain candidates. The calendar clips the rows to the local month.
        Ok(sqlx::query_as(
            "SELECT t.title,t.slug,e.starts_at,e.ends_at,
                    e.starts_at AT TIME ZONE $2 AS local_start,
                    e.ends_at AT TIME ZONE $2 AS local_end,
                    ((e.ends_at - interval '1 microsecond') AT TIME ZONE $2)::date AS last_day
             FROM events e JOIN event_translations t ON t.event_id=e.id
             WHERE e.status='published' AND e.published_by IS NOT NULL AND t.locale=$1
               AND e.starts_at < (($4::date + 1)::timestamp AT TIME ZONE 'UTC')
               AND e.ends_at > (($3::date - 1)::timestamp AT TIME ZONE 'UTC')
             ORDER BY e.starts_at,e.id",
        )
        .bind(locale)
        .bind(zone)
        .bind(first)
        .bind(next)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(super) async fn export_events(
        &self,
        locale: &str,
        slug: Option<&str>,
    ) -> Result<Vec<ExportEvent>, Error> {
        Ok(sqlx::query_as(
            "SELECT e.id,t.title,t.summary,e.starts_at,e.ends_at,e.updated_at
             FROM events e JOIN event_translations t ON t.event_id=e.id
             WHERE e.status='published' AND e.published_by IS NOT NULL AND t.locale=$1
               AND (($2::text IS NOT NULL AND t.slug=$2)
                    OR ($2::text IS NULL AND e.ends_at>CURRENT_TIMESTAMP))
             ORDER BY e.starts_at,e.id",
        )
        .bind(locale)
        .bind(slug)
        .fetch_all(&self.pool)
        .await?)
    }
    pub(crate) async fn digest(
        &self,
        locale: &str,
        at: time::OffsetDateTime,
    ) -> Result<Vec<PublicEvent>, Error> {
        Ok(sqlx::query_as("SELECT e.id,t.title,t.slug,t.summary,t.body,e.starts_at,e.ends_at FROM events e JOIN event_translations t ON t.event_id=e.id WHERE e.status='published' AND e.published_by IS NOT NULL AND t.locale=$1 AND e.ends_at>$2 ORDER BY (e.starts_at>$2),e.featured DESC,e.starts_at,e.id LIMIT 1")
            .bind(locale).bind(at).fetch_all(&self.pool).await?)
    }
    pub async fn list(&self) -> Result<Vec<Event>, Error> {
        self.list_page(0).await
    }
    pub(crate) async fn list_page(&self, offset: i64) -> Result<Vec<Event>, Error> {
        Ok(sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version,published_by FROM events ORDER BY updated_at DESC,id DESC LIMIT 200 OFFSET $1").bind(offset).fetch_all(&self.pool).await?)
    }
    pub async fn load(&self, id: Uuid) -> Result<(Event, Translation, Option<Translation>), Error> {
        let event = sqlx::query_as(
            "SELECT id,status,starts_at,ends_at,featured,lock_version,published_by FROM events WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::NotFound)?;
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT locale,title,slug,summary,body FROM event_translations WHERE event_id=$1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let mut en = None;
        let mut pl = None;
        for (locale, title, slug, summary, body) in rows {
            let t = Translation {
                title,
                slug,
                summary,
                body,
            };
            match locale.as_str() {
                "en" => en = Some(t),
                "pl" => pl = Some(t),
                _ => return Err(Error::Conflict),
            }
        }
        Ok((event, en.ok_or(Error::Conflict)?, pl))
    }
    pub async fn save<A: AccountStateReader>(
        &self,
        auth: &Authorizer<A>,
        context: CheckContext,
        existing: Option<(Uuid, i64)>,
        mut draft: Draft,
    ) -> Result<Uuid, Error> {
        let mut tx = self.pool.begin().await?;
        oteryn_admin::rbac::authorize_mutation_with_resources(
            &mut tx,
            &[context.account],
            move |connection| {
                Box::pin(async move {
                    if let Some((id, _)) = existing {
                        sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE")
                            .bind(id)
                            .execute(connection)
                            .await?;
                    }
                    Ok(())
                })
            },
            auth,
            context,
            &[Permission::Access, Permission::ManageEvents],
        )
        .await?;
        auth.check_in(&mut tx, context, Permission::Access).await?;
        auth.check_in(&mut tx, context, Permission::ManageEvents)
            .await?;
        draft.validate()?;
        let reason = audit_reason(&draft.reason)?;
        let id = existing.map_or_else(Uuid::now_v7, |v| v.0);
        let mut permissions = vec![Permission::Access, Permission::ManageEvents];
        let (before, published_by): (_, Option<Uuid>) = if let Some((_, version)) = existing {
            let row: Event =
                sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version,published_by FROM events WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or(Error::NotFound)?;
            if version < 1 || row.lock_version != version || Status::parse(&row.status).is_none() {
                return Err(Error::Conflict);
            }
            if row.status != Status::Draft.as_str() {
                auth.check_in(&mut tx, context, Permission::PublishEvents)
                    .await?;
                permissions.push(Permission::PublishEvents);
            }
            let translations = translation_snapshot(&mut tx, id).await?;
            let published_by = sqlx::query_scalar("UPDATE events SET status='draft',starts_at=$2,ends_at=$3,featured=$4,updated_by=$5,published_by=NULL,lock_version=lock_version+1,updated_at=now() WHERE id=$1 RETURNING published_by")
                .bind(id).bind(draft.starts_at).bind(draft.ends_at).bind(draft.featured).bind(context.account).fetch_one(&mut *tx).await?;
            (
                json!({
                    "published_by": row.published_by,
                    "status": row.status, "version": version, "featured": row.featured,
                    "starts_at": row.starts_at.unix_timestamp(), "ends_at": row.ends_at.unix_timestamp(),
                    "translations": translations,
                }),
                published_by,
            )
        } else {
            let published_by = sqlx::query_scalar("INSERT INTO events (id,starts_at,ends_at,featured,created_by,updated_by) VALUES ($1,$2,$3,$4,$5,$5) RETURNING published_by").bind(id).bind(draft.starts_at).bind(draft.ends_at).bind(draft.featured).bind(context.account).fetch_one(&mut *tx).await?;
            (json!(null), published_by)
        };
        sqlx::query("DELETE FROM event_translations WHERE event_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for (locale, t) in [("en", Some(draft.en)), ("pl", draft.pl)] {
            if let Some(t) = t {
                sqlx::query("INSERT INTO event_translations (event_id,locale,title,slug,summary,body) VALUES ($1,$2,$3,$4,$5,$6)").bind(id).bind(locale).bind(t.title).bind(t.slug).bind(t.summary).bind(t.body).execute(&mut *tx).await.map_err(constraint)?;
            }
        }
        let translations = translation_snapshot(&mut tx, id).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: if existing.is_some() {
                    "events.event_updated"
                } else {
                    "events.event_created"
                },
                target: &id.to_string(),
                before,
                after: json!({
                    "status": "draft", "version": existing.map_or(1, |(_, version)| version + 1),
                    "starts_at": draft.starts_at.unix_timestamp(), "ends_at": draft.ends_at.unix_timestamp(),
                    "featured": draft.featured, "reason": reason, "translations": translations, "published_by": published_by,
                }),
            },
        )
        .await?;
        crate::revalidate(auth, &mut tx, context, &permissions).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn transition<A: AccountStateReader>(
        &self,
        auth: &Authorizer<A>,
        context: CheckContext,
        id: Uuid,
        version: i64,
        target: &str,
        reason: &str,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        oteryn_admin::rbac::authorize_mutation_with_resources(
            &mut tx,
            &[context.account],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE")
                        .bind(id)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            auth,
            context,
            &[
                Permission::Access,
                Permission::ManageEvents,
                Permission::PublishEvents,
            ],
        )
        .await?;
        auth.check_in(&mut tx, context, Permission::Access).await?;
        for permission in [Permission::ManageEvents, Permission::PublishEvents] {
            auth.check_in(&mut tx, context, permission).await?;
        }
        let target = Status::parse(target).ok_or(Error::Invalid)?;
        let reason = audit_reason(reason)?;
        let current: Event=sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version,published_by FROM events WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if version < 1
            || version != current.lock_version
            || Status::parse(&current.status).is_none()
        {
            return Err(Error::Conflict);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if target == Status::Scheduled && current.starts_at <= now {
            return Err(Error::Conflict);
        }
        if target != Status::Draft {
            let en: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM event_translations WHERE event_id=$1 AND locale='en')",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if !en {
                return Err(Error::Conflict);
            }
        }
        let published_by: Option<Uuid> = sqlx::query_scalar("UPDATE events SET status=$2,updated_by=$3,published_by=$4,lock_version=lock_version+1,updated_at=now() WHERE id=$1 RETURNING published_by").bind(id).bind(target.as_str()).bind(context.account).bind(if target==Status::Draft {None} else {Some(context.account)}).fetch_one(&mut *tx).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: "events.status_changed",
                target: &id.to_string(),
                before: json!({"status":current.status,"version":version,"published_by":current.published_by}),
                after: json!({"status":target.as_str(),"version":version+1,"reason":reason,"published_by":published_by}),
            },
        )
        .await?;
        crate::revalidate(
            auth,
            &mut tx,
            context,
            &[
                Permission::Access,
                Permission::ManageEvents,
                Permission::PublishEvents,
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn constraint(error: sqlx::Error) -> Error {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation() || e.is_check_violation())
    {
        Error::Invalid
    } else {
        Error::Storage(error)
    }
}
