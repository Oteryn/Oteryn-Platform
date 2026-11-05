use super::*;
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde_json::json;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct Events {
    pool: PgPool,
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
        if !matches!(locale, "pl" | "en") || slug.is_some_and(|s| !valid_slug(s)) {
            return Err(Error::Invalid);
        }
        Ok(sqlx::query_as("SELECT e.id,t.title,t.slug,t.summary,t.body,e.starts_at,e.ends_at FROM events e JOIN event_translations t ON t.event_id=e.id WHERE e.status='published' AND e.published_by IS NOT NULL AND t.locale=$1 AND ($2::text IS NULL OR t.slug=$2) ORDER BY e.featured DESC,e.starts_at,e.id LIMIT 200")
            .bind(locale).bind(slug).fetch_all(&self.pool).await?)
    }
    pub async fn list(&self) -> Result<Vec<Event>, Error> {
        Ok(sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version FROM events ORDER BY updated_at DESC,id DESC LIMIT 200").fetch_all(&self.pool).await?)
    }
    pub async fn load(&self, id: Uuid) -> Result<(Event, Translation, Option<Translation>), Error> {
        let event = sqlx::query_as(
            "SELECT id,status,starts_at,ends_at,featured,lock_version FROM events WHERE id=$1",
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
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        auth.check(context, Permission::ManageEvents).await?;
        draft.validate()?;
        let id = existing.map_or_else(Uuid::now_v7, |v| v.0);
        let before = if let Some((_, version)) = existing {
            let row: (String, i64) =
                sqlx::query_as("SELECT status,lock_version FROM events WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or(Error::NotFound)?;
            if version < 1 || row.1 != version || Status::parse(&row.0).is_none() {
                return Err(Error::Conflict);
            }
            sqlx::query("UPDATE events SET status='draft',starts_at=$2,ends_at=$3,featured=$4,updated_by=$5,published_by=NULL,lock_version=lock_version+1,updated_at=now() WHERE id=$1")
                .bind(id).bind(draft.starts_at).bind(draft.ends_at).bind(draft.featured).bind(context.account).execute(&mut *tx).await?;
            json!({"status":row.0,"version":row.1})
        } else {
            sqlx::query("INSERT INTO events (id,starts_at,ends_at,featured,created_by,updated_by) VALUES ($1,$2,$3,$4,$5,$5)").bind(id).bind(draft.starts_at).bind(draft.ends_at).bind(draft.featured).bind(context.account).execute(&mut *tx).await?;
            json!(null)
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
                after: json!({"status":"draft"}),
            },
        )
        .await?;
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
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        for permission in [Permission::ManageEvents, Permission::PublishEvents] {
            auth.check(context, permission).await?;
        }
        let target = Status::parse(target).ok_or(Error::Invalid)?;
        let current: Event=sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version FROM events WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
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
        sqlx::query("UPDATE events SET status=$2,updated_by=$3,published_by=$4,lock_version=lock_version+1,updated_at=now() WHERE id=$1").bind(id).bind(target.as_str()).bind(context.account).bind(if target==Status::Draft {None} else {Some(context.account)}).execute(&mut *tx).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: "events.status_changed",
                target: &id.to_string(),
                before: json!({"status":current.status,"version":version}),
                after: json!({"status":target.as_str(),"version":version+1}),
            },
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
