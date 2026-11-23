use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationCheck, Permission},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{FromRow, PgConnection, PgPool};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const MAX_OPEN_INCIDENTS: i64 = 50;
const MAX_MAINTENANCE_WINDOWS: i64 = 100;
const MAX_TIMELINE_ENTRIES: i16 = 100;
pub const HISTORY_PAGE_SIZE: usize = 20;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid status input")]
    Invalid,
    #[error("status record was not found")]
    Missing,
    #[error("status record is closed or the publication limit was reached")]
    Conflict,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, Deserialize)]
pub struct Message {
    message_pl: String,
    message_en: String,
}

impl Message {
    fn validate(&mut self) -> Result<(), Error> {
        validate_text(&mut self.message_pl, 2000)?;
        validate_text(&mut self.message_en, 2000)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Minor,
    Major,
}

impl Severity {
    fn code(self) -> &'static str {
        match self {
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateIncident {
    scope: String,
    severity: Severity,
    title_pl: String,
    title_en: String,
    #[serde(flatten)]
    message: Message,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleMaintenance {
    scope: String,
    starts_at: String,
    ends_at: String,
    #[serde(flatten)]
    message: Message,
}

#[derive(Clone, Copy, Debug)]
pub enum Transition {
    Update,
    Resolve,
}

pub enum Command {
    Create(CreateIncident),
    Transition(Uuid, Transition, Message),
    Schedule(ScheduleMaintenance),
}

impl Command {
    pub fn permission(&self) -> Permission {
        match self {
            Self::Create(_) => Permission::StatusIncidentCreate,
            Self::Transition(_, Transition::Update, _) => Permission::StatusIncidentUpdate,
            Self::Transition(_, Transition::Resolve, _) => Permission::StatusIncidentResolve,
            Self::Schedule(_) => Permission::StatusMaintenanceSchedule,
        }
    }

    fn validate(&mut self) -> Result<(), Error> {
        match self {
            Self::Create(form) => {
                validate_text(&mut form.title_pl, 120)?;
                validate_text(&mut form.title_en, 120)?;
                form.message.validate()
            }
            Self::Transition(_, _, message) => message.validate(),
            Self::Schedule(form) => form.message.validate(),
        }
    }

    fn world(&self) -> Result<Option<Uuid>, Error> {
        let scope = match self {
            Self::Create(form) => &form.scope,
            Self::Schedule(form) => &form.scope,
            Self::Transition(..) => return Ok(None),
        };
        if scope.is_empty() {
            Ok(None)
        } else {
            let id = Uuid::parse_str(scope).map_err(|_| Error::Invalid)?;
            if id.is_nil() {
                return Err(Error::Invalid);
            }
            Ok(Some(id))
        }
    }

    fn incident(&self) -> Option<Uuid> {
        match self {
            Self::Transition(id, ..) => Some(*id),
            _ => None,
        }
    }
}

fn validate_text(text: &mut String, max: usize) -> Result<(), Error> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max
        || trimmed.chars().any(|c| {
            (c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
                || matches!(c, '\u{fffe}' | '\u{ffff}')
        })
    {
        return Err(Error::Invalid);
    }
    if trimmed.len() != text.len() {
        *text = trimmed.to_owned();
    }
    Ok(())
}

fn utc_minute(input: &str) -> Result<OffsetDateTime, Error> {
    if input.len() != 16 || input.as_bytes().get(10) != Some(&b'T') {
        return Err(Error::Invalid);
    }
    OffsetDateTime::parse(&format!("{input}:00Z"), &Rfc3339).map_err(|_| Error::Invalid)
}

fn maintenance_window(
    form: &ScheduleMaintenance,
    now: OffsetDateTime,
) -> Result<(OffsetDateTime, OffsetDateTime), Error> {
    let start = utc_minute(&form.starts_at)?;
    let end = utc_minute(&form.ends_at)?;
    // Allow a publication during the current minute.
    if now - start >= Duration::minutes(1)
        || start > now + Duration::days(365)
        || end <= start
        || end - start > Duration::days(30)
    {
        return Err(Error::Invalid);
    }
    Ok((start, end))
}

pub async fn mutate(
    db: &PgPool,
    context: CheckContext,
    authority: &dyn MutationCheck,
    mut command: Command,
) -> Result<Uuid, Error> {
    command.validate()?;
    let world = command.world()?;
    let incident = command.incident();
    let permission = command.permission();
    let mut tx = db.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    rbac::authorize_mutation_with_resources_domain(
        &mut tx,
        &[],
        move |connection| {
            Box::pin(async move {
                if let Some(world) = world {
                    sqlx::query_scalar::<_, Uuid>(
                        "SELECT id FROM world_status WHERE id=$1 FOR KEY SHARE",
                    )
                    .bind(world)
                    .fetch_optional(&mut *connection)
                    .await?
                    .ok_or(Error::Invalid)?;
                }
                if let Some(incident) = incident {
                    sqlx::query_scalar::<_, Uuid>(
                        "SELECT id FROM status_incidents WHERE id=$1 FOR UPDATE",
                    )
                    .bind(incident)
                    .fetch_optional(connection)
                    .await?
                    .ok_or(Error::Missing)?;
                }
                Ok::<(), Error>(())
            })
        },
        authority,
        context,
        &[Permission::Access, permission],
    )
    .await?;
    let (id, before, after) = match command {
        Command::Create(form) => create_incident(&mut tx, world, form).await?,
        Command::Transition(id, transition, message) => {
            transition_incident(&mut tx, id, transition, message).await?
        }
        Command::Schedule(form) => schedule_maintenance(&mut tx, world, form).await?,
    };
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: permission.as_str(),
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

async fn create_incident(
    db: &mut PgConnection,
    world: Option<Uuid>,
    form: CreateIncident,
) -> Result<(Uuid, Value, Value), Error> {
    let active: i64 =
        sqlx::query_scalar("SELECT count(*) FROM status_incidents WHERE resolved_at IS NULL")
            .fetch_one(&mut *db)
            .await?;
    if active >= MAX_OPEN_INCIDENTS {
        return Err(Error::Conflict);
    }
    let (id, opened): (Uuid, OffsetDateTime) = sqlx::query_as(
        "INSERT INTO status_incidents (world_id,severity,title_pl,title_en)
         VALUES ($1,$2,$3,$4) RETURNING id,opened_at",
    )
    .bind(world)
    .bind(form.severity.code())
    .bind(&form.title_pl)
    .bind(&form.title_en)
    .fetch_one(&mut *db)
    .await?;
    insert_update(db, id, 1, opened, "opened", &form.message).await?;
    Ok((
        id,
        Value::Null,
        json!({
            "world_id":world, "severity":form.severity.code(), "sequence":1, "state":"open"
        }),
    ))
}

async fn transition_incident(
    db: &mut PgConnection,
    id: Uuid,
    transition: Transition,
    message: Message,
) -> Result<(Uuid, Value, Value), Error> {
    let (closed, previous, timestamp): (bool, i16, OffsetDateTime) = sqlx::query_as(
        "SELECT i.resolved_at IS NOT NULL, u.sequence, greatest(clock_timestamp(), u.posted_at)
         FROM status_incidents i JOIN LATERAL (
           SELECT sequence, posted_at FROM status_incident_updates
           WHERE incident_id=i.id ORDER BY sequence DESC LIMIT 1
         ) u ON true WHERE i.id=$1",
    )
    .bind(id)
    .fetch_one(&mut *db)
    .await?;
    let sequence = previous + 1;
    let (kind, state) = match transition {
        Transition::Update if sequence < MAX_TIMELINE_ENTRIES => ("updated", "open"),
        Transition::Resolve if sequence <= MAX_TIMELINE_ENTRIES => ("resolved", "resolved"),
        _ => return Err(Error::Conflict),
    };
    if closed {
        return Err(Error::Conflict);
    }
    if matches!(transition, Transition::Resolve) {
        sqlx::query("UPDATE status_incidents SET resolved_at=$2 WHERE id=$1")
            .bind(id)
            .bind(timestamp)
            .execute(&mut *db)
            .await?;
    }
    insert_update(db, id, sequence, timestamp, kind, &message).await?;
    Ok((
        id,
        json!({"state":"open", "sequence":previous}),
        json!({"state":state, "sequence":sequence}),
    ))
}

async fn insert_update(
    db: &mut PgConnection,
    id: Uuid,
    sequence: i16,
    at: OffsetDateTime,
    kind: &str,
    message: &Message,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO status_incident_updates
         (incident_id,sequence,posted_at,kind,message_pl,message_en)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(sequence)
    .bind(at)
    .bind(kind)
    .bind(&message.message_pl)
    .bind(&message.message_en)
    .execute(db)
    .await?;
    Ok(())
}

async fn schedule_maintenance(
    db: &mut PgConnection,
    world: Option<Uuid>,
    form: ScheduleMaintenance,
) -> Result<(Uuid, Value, Value), Error> {
    let now = clock(db).await?;
    let (start, end) = maintenance_window(&form, now)?;
    let scheduled: i64 =
        sqlx::query_scalar("SELECT count(*) FROM status_maintenance WHERE ends_at>$1")
            .bind(now)
            .fetch_one(&mut *db)
            .await?;
    if scheduled >= MAX_MAINTENANCE_WINDOWS {
        return Err(Error::Conflict);
    }
    let id = sqlx::query_scalar(
        "INSERT INTO status_maintenance (world_id,starts_at,ends_at,message_pl,message_en)
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(world)
    .bind(start)
    .bind(end)
    .bind(&form.message.message_pl)
    .bind(&form.message.message_en)
    .fetch_one(db)
    .await?;
    Ok((
        id,
        Value::Null,
        json!({"world_id":world, "starts_at":iso(start), "ends_at":iso(end)}),
    ))
}

pub async fn clock(db: &mut PgConnection) -> Result<OffsetDateTime, sqlx::Error> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(db)
        .await
}

pub fn iso(time: impl std::borrow::Borrow<OffsetDateTime>) -> String {
    time.borrow().format(&Rfc3339).unwrap_or_default()
}

#[derive(FromRow)]
pub struct Component {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub state: String,
    pub uptime: f64,
}

pub async fn components(
    db: &mut PgConnection,
    now: OffsetDateTime,
) -> Result<Vec<Component>, sqlx::Error> {
    // Clip each incident to the reporting window and union overlaps before summing.
    sqlx::query_as(
        "WITH components AS (
           SELECT NULL::uuid AS id, NULL::text AS name
           UNION ALL SELECT id, name FROM world_status
         ), incident_time AS (
           SELECT world_id,
                  bool_or(resolved_at IS NULL AND severity='major') AS major,
                  bool_or(resolved_at IS NULL) AS active,
                  range_agg(tstzrange(greatest(opened_at,$2), least(coalesce(resolved_at,$1),$1))) AS ranges
           FROM status_incidents
           WHERE opened_at <= $1 AND (resolved_at IS NULL OR resolved_at > $2)
           GROUP BY world_id
         )
         SELECT c.id, c.name,
           CASE WHEN i.major THEN 'major'
                WHEN EXISTS (SELECT FROM status_maintenance m
                  WHERE c.id IS NOT NULL AND (m.world_id IS NULL OR m.world_id=c.id)
                    AND m.starts_at <= $1 AND m.ends_at > $1) THEN 'maintenance'
                WHEN i.active THEN 'minor' ELSE 'operational' END AS state,
           (100 * (1 - coalesce((SELECT sum(extract(epoch FROM upper(r)-lower(r)))
                   FROM unnest(i.ranges) r),0) / 7776000))::double precision AS uptime
         FROM components c LEFT JOIN incident_time i ON i.world_id IS NOT DISTINCT FROM c.id
         ORDER BY c.name COLLATE \"C\" NULLS FIRST, c.id",
    )
    .bind(now)
    .bind(now - Duration::days(90))
    .fetch_all(db)
    .await
}

#[derive(FromRow)]
pub struct Incident {
    pub id: Uuid,
    pub world: Option<String>,
    pub title: String,
    pub severity: String,
    pub opened_at: OffsetDateTime,
    pub resolved_at: Option<OffsetDateTime>,
    pub posted_at: OffsetDateTime,
    pub message: String,
    pub kind: String,
}

#[derive(Clone, Copy)]
pub enum Incidents {
    Open,
    History(i64),
    Feed,
    Detail(Uuid),
}

/// Read metadata and the latest update. The detail route loads the complete timeline.
pub async fn incidents(
    db: &mut PgConnection,
    lang: &str,
    now: OffsetDateTime,
    filter: Incidents,
) -> Result<Vec<Incident>, sqlx::Error> {
    let (mode, id, offset, limit) = match filter {
        Incidents::Open => ("open", None, 0, MAX_OPEN_INCIDENTS),
        Incidents::History(offset) => ("history", None, offset, (HISTORY_PAGE_SIZE + 1) as i64),
        Incidents::Feed => ("feed", None, 0, 50),
        Incidents::Detail(id) => ("detail", Some(id), 0, 1),
    };
    sqlx::query_as(
        "SELECT i.id, w.name AS world, i.severity, i.opened_at, i.resolved_at,
                CASE WHEN $1='pl' THEN i.title_pl ELSE i.title_en END AS title,
                u.posted_at, u.kind, u.message
         FROM status_incidents i LEFT JOIN world_status w ON w.id=i.world_id
         JOIN LATERAL (
           SELECT posted_at, kind, CASE WHEN $1='pl' THEN message_pl ELSE message_en END AS message
           FROM status_incident_updates WHERE incident_id=i.id ORDER BY sequence DESC LIMIT 1
         ) u ON true
         WHERE CASE $2
           WHEN 'open' THEN i.resolved_at IS NULL
           WHEN 'history' THEN i.resolved_at > $3
           WHEN 'feed' THEN i.resolved_at IS NULL OR i.resolved_at > $3
           ELSE i.id=$4 END
         ORDER BY CASE WHEN $2='history' THEN i.resolved_at ELSE u.posted_at END DESC, i.id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(lang)
    .bind(mode)
    .bind(now - Duration::days(90))
    .bind(id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

#[derive(FromRow)]
pub struct Update {
    pub posted_at: OffsetDateTime,
    pub kind: String,
    pub message: String,
}

pub async fn timeline(
    db: &mut PgConnection,
    id: Uuid,
    lang: &str,
) -> Result<Vec<Update>, sqlx::Error> {
    sqlx::query_as(
        "SELECT posted_at,kind,CASE WHEN $2='pl' THEN message_pl ELSE message_en END AS message
         FROM status_incident_updates WHERE incident_id=$1 ORDER BY sequence DESC",
    )
    .bind(id)
    .bind(lang)
    .fetch_all(db)
    .await
}

#[derive(FromRow)]
pub struct Maintenance {
    pub id: Uuid,
    pub world: Option<String>,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub message: String,
    pub active: bool,
}

pub async fn maintenance(
    db: &mut PgConnection,
    lang: &str,
    now: OffsetDateTime,
    banner: bool,
) -> Result<Vec<Maintenance>, sqlx::Error> {
    sqlx::query_as(
        "SELECT m.id,w.name AS world,m.starts_at,m.ends_at,m.starts_at <= $2 AS active,
                CASE WHEN $1='pl' THEN m.message_pl ELSE m.message_en END AS message
         FROM status_maintenance m LEFT JOIN world_status w ON w.id=m.world_id
         WHERE m.ends_at > $2 AND (NOT $3 OR m.starts_at <= $2 + interval '24 hours')
         ORDER BY m.starts_at, m.id",
    )
    .bind(lang)
    .bind(now)
    .bind(banner)
    .fetch_all(db)
    .await
}
