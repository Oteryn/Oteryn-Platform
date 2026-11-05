//! T82 owner-private weekly reports, annotations and downloads.
//!
//! T73 is absent in the base. JournalSource is the sole integration seam: it must
//! recheck current privacy, retention, transfer-receipt cut, restore generation and
//! contiguous completeness on every call. Cached ownership is never authority.
//! G-9 revision 2 character_revision and entry-to-durable-logout sessions apply.
//! G-10 inventory and G-11 progress are deliberately not treated as session drops.
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

pub mod http;
mod render;

pub type Result<T> = std::result::Result<T, Error>;
pub type SourceFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Denied,
    Invalid,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub weekly_email: bool,
}

/// UTC Monday 00:00 inclusive to next Monday exclusive, only completed weeks.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Week {
    pub start: i64,
    pub end: i64,
}
impl Week {
    pub fn new(start: i64, now: i64) -> Result<Self> {
        let end = start.checked_add(604800).ok_or(Error::Invalid)?;
        if start < 0 || (start - 345600).rem_euclid(604800) != 0 || end > now {
            return Err(Error::Invalid);
        }
        // G-9's shorter proposed 90-day retention wins over the plan's 365 days.
        if start < now.saturating_sub(90 * 86400) {
            return Err(Error::Denied);
        }
        Ok(Self { start, end })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Session,
    Character,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub kind: TargetKind,
    pub id: Uuid,
}
impl Target {
    fn kind(self) -> &'static str {
        match self.kind {
            TargetKind::Session => "session",
            TargetKind::Character => "character",
        }
    }
    fn validate(self) -> Result<()> {
        if self.id.get_version_num() != 7 {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

/// Platform projection, never an ingest wire schema. Decimal counters serialize as
/// strings to preserve exact integers in downstream JSON consumers.
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: Uuid,
    pub character_id: Uuid,
    #[serde(serialize_with = "decimal_revision")]
    pub character_revision: u64,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_seconds: u64,
    pub experience_gained: String,
    pub experience_lost: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Drop {
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub occurred_at: i64,
    pub item_key: String,
    pub quantity: String,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub account_id: Uuid,
    /// Must prove the whole requested interval, including known-empty intervals.
    pub covered_start: i64,
    pub covered_end: i64,
    pub complete: bool,
    pub sessions: Vec<Session>,
    pub drops: Vec<Drop>,
}

/// Trusted Platform adapter, never populated from browser JSON. The adapter must
/// fence serving at the authoritative transfer cut and suppress stale/ambiguous
/// evidence. Staff privilege is not journal access. Session selection uses end
/// time in [week.start, week.end), avoiding splitting or counting sessions twice.
/// Drops belong to selected sessions and are already included in session loot.
pub trait JournalSource: std::fmt::Debug + Send + Sync {
    fn snapshot(&self, account: Uuid, week: Week) -> SourceFuture<'_, Snapshot>;
    fn authorize(&self, account: Uuid, target: Target) -> SourceFuture<'_, ()>;
}
#[derive(Debug)]
pub struct DenySource;
impl JournalSource for DenySource {
    fn snapshot(&self, _: Uuid, _: Week) -> SourceFuture<'_, Snapshot> {
        Box::pin(async { Err(Error::Unavailable) })
    }
    fn authorize(&self, _: Uuid, _: Target) -> SourceFuture<'_, ()> {
        Box::pin(async { Err(Error::Denied) })
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub week: Week,
    pub session_count: usize,
    pub duration_seconds: String,
    pub experience_gained: String,
    pub experience_lost: String,
    pub sessions: Vec<Session>,
    pub drops: Vec<Drop>,
}
fn decimal_revision<S: serde::Serializer>(
    value: &u64,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
fn counter(value: &str) -> Result<u64> {
    if value.is_empty()
        || value.len() > 19
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|v| v.is_ascii_digit())
    {
        return Err(Error::Invalid);
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|v| *v <= i64::MAX as u64)
        .ok_or(Error::Invalid)
}
fn build(account: Uuid, week: Week, mut data: Snapshot) -> Result<Report> {
    if data.account_id != account
        || !data.complete
        || data.covered_start > week.start
        || data.covered_end < week.end
        || data.sessions.len() > 10000
        || data.drops.len() > 50000
    {
        return Err(Error::Denied);
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut duration = 0_u128;
    let mut gained = 0_u128;
    let mut lost = 0_u128;
    for session in &data.sessions {
        if session.id.get_version_num() != 7
            || session.character_id.get_version_num() != 7
            || session.character_revision == 0
            || !ids.insert(session.id)
            || session.started_at < 0
            || session.ended_at < session.started_at
            || session.ended_at < week.start
            || session.ended_at >= week.end
            || session.duration_seconds > (session.ended_at - session.started_at) as u64
        {
            return Err(Error::Invalid);
        }
        duration += u128::from(session.duration_seconds);
        gained += u128::from(counter(&session.experience_gained)?);
        lost += u128::from(counter(&session.experience_lost)?);
    }
    let mut events = std::collections::BTreeSet::new();
    for drop in &data.drops {
        let session = data
            .sessions
            .iter()
            .find(|s| s.id == drop.session_id)
            .ok_or(Error::Invalid)?;
        if drop.event_id.get_version_num() != 7
            || !events.insert(drop.event_id)
            || drop.occurred_at < session.started_at
            || drop.occurred_at > session.ended_at
            || counter(&drop.quantity)? == 0
            || !valid_key(&drop.item_key)
        {
            return Err(Error::Invalid);
        }
    }
    data.sessions.sort_by_key(|s| (s.ended_at, s.id));
    data.drops.sort_by_key(|d| (d.occurred_at, d.event_id));
    Ok(Report {
        schema_version: 1,
        week,
        session_count: data.sessions.len(),
        duration_seconds: duration.to_string(),
        experience_gained: gained.to_string(),
        experience_lost: lost.to_string(),
        sessions: data.sessions,
        drops: data.drops,
    })
}
fn valid_key(key: &str) -> bool {
    let Some((namespace, name)) = key.split_once(':') else {
        return false;
    };
    key.len() <= 512
        && !namespace.is_empty()
        && !name.is_empty()
        && namespace.as_bytes()[0].is_ascii_alphanumeric()
        && namespace
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_./-".contains(&c))
}

#[derive(Debug, Clone)]
pub struct Reports {
    pub pool: PgPool,
    pub source: Arc<dyn JournalSource>,
    pub flags: Flags,
}
impl Reports {
    fn enabled(&self) -> Result<()> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    pub async fn weekly(&self, account: Uuid, start: i64, now: i64) -> Result<Report> {
        self.enabled()?;
        let week = Week::new(start, now)?;
        build(account, week, self.source.snapshot(account, week).await?)
    }
    pub async fn note(&self, account: Uuid, target: Target) -> Result<Option<String>> {
        self.enabled()?;
        target.validate()?;
        self.source.authorize(account, target).await?;
        Ok(sqlx::query_scalar("SELECT body FROM pd_report_notes WHERE account_id=$1 AND target_kind=$2 AND target_id=$3")
            .bind(account).bind(target.kind()).bind(target.id).fetch_optional(&self.pool).await?)
    }
    /// Empty text deletes a note. No markup is interpreted, no copied game text.
    pub async fn save_note(&self, account: Uuid, target: Target, body: &str) -> Result<()> {
        self.enabled()?;
        target.validate()?;
        if body.len() > 4000
            || body
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err(Error::Invalid);
        }
        self.source.authorize(account, target).await?;
        if body.trim().is_empty() {
            sqlx::query("DELETE FROM pd_report_notes WHERE account_id=$1 AND target_kind=$2 AND target_id=$3")
                .bind(account).bind(target.kind()).bind(target.id).execute(&self.pool).await?;
        } else {
            sqlx::query("INSERT INTO pd_report_notes(account_id,target_kind,target_id,body) VALUES($1,$2,$3,$4) ON CONFLICT(account_id,target_kind,target_id) DO UPDATE SET body=excluded.body,updated_at=now()")
                .bind(account).bind(target.kind()).bind(target.id).bind(body).execute(&self.pool).await?;
        }
        Ok(())
    }
    pub async fn email_enabled(&self, account: Uuid) -> Result<bool> {
        self.enabled()?;
        let value: Option<bool> = sqlx::query_scalar(
            "SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1",
        )
        .bind(account)
        .fetch_optional(&self.pool)
        .await?;
        Ok(value == Some(true))
    }
    pub async fn email_preference(&self, account: Uuid, enabled: bool) -> Result<()> {
        self.enabled()?;
        sqlx::query("INSERT INTO pd_report_preferences(account_id,email_enabled) VALUES($1,$2) ON CONFLICT(account_id) DO UPDATE SET email_enabled=$2,updated_at=now()")
            .bind(account).bind(enabled).execute(&self.pool).await?;
        Ok(())
    }
    /// T09 composition seam: caller supplies the authenticated owner. Reauthorize
    /// each target, omitting inaccessible historical annotations after a transfer.
    pub async fn export_notes(&self, account: Uuid) -> Result<Vec<Note>> {
        self.enabled()?;
        let rows: Vec<(String, Uuid, String)> = sqlx::query_as("SELECT target_kind,target_id,body FROM pd_report_notes WHERE account_id=$1 ORDER BY target_kind,target_id LIMIT 10001")
            .bind(account).fetch_all(&self.pool).await?;
        if rows.len() > 10000 {
            return Err(Error::Unavailable);
        }
        let mut result = Vec::new();
        for (kind, id, body) in rows {
            let target = Target {
                kind: if kind == "session" {
                    TargetKind::Session
                } else {
                    TargetKind::Character
                },
                id,
            };
            match self.source.authorize(account, target).await {
                Ok(()) => result.push(Note { target, body }),
                Err(Error::Denied) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(result)
    }
    /// Optional reference-only email. Both the report opt-in and T36's optional
    /// news email preference are required. Queue carries no gameplay or note data.
    /// T36 rechecks account state and preference at delivery. Retries deduplicate.
    pub async fn email_week(
        &self,
        account: Uuid,
        start: i64,
        now: i64,
        locale: render::Locale,
        notify: &oteryn_notify::Notifications,
    ) -> Result<bool> {
        if !self.flags.weekly_email {
            return Err(Error::Disabled);
        }
        self.weekly(account, start, now).await?;
        let mut tx = self.pool.begin().await?;
        let enabled: Option<bool> = sqlx::query_scalar(
            "SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1 FOR UPDATE",
        )
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?;
        if enabled != Some(true) {
            return Ok(false);
        }
        let pref: Option<bool> = sqlx::query_scalar("SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='news' FOR UPDATE")
            .bind(account).fetch_optional(&mut *tx).await?;
        if pref != Some(true) {
            return Ok(false);
        }
        let inserted = sqlx::query("INSERT INTO pd_report_deliveries(account_id,week_start) VALUES($1,$2) ON CONFLICT DO NOTHING")
            .bind(account).bind(start).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Ok(false);
        }
        let template = oteryn_notify::EmailTemplate::News {
            title: render::message(locale, "title")?,
            body: format!(
                "{} /{}/account/journal/reports?week={start}",
                render::message(locale, "email")?,
                locale.code()
            ),
        };
        if notify
            .enqueue(&mut tx, account, &template)
            .await
            .map_err(|_| Error::Unavailable)?
            .is_none()
        {
            return Ok(false);
        }
        tx.commit().await?;
        Ok(true)
    }
}
#[derive(Debug, Serialize)]
pub struct Note {
    pub target: Target,
    pub body: String,
}

pub use render::Locale;

/// RFC 4180 CSV. Every field is quoted, formula-leading cells are neutralized.
pub fn csv(report: &Report, drops: bool) -> String {
    fn row(out: &mut String, fields: &[String]) {
        let cells: Vec<String> = fields
            .iter()
            .map(|value| {
                let prefixed = if value.trim_start().starts_with(['=', '+', '-', '@']) {
                    format!("'{value}")
                } else {
                    value.clone()
                };
                format!("\"{}\"", prefixed.replace('"', "\"\""))
            })
            .collect();
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    let mut out = String::new();
    if drops {
        row(
            &mut out,
            &[
                "event_id",
                "session_id",
                "occurred_at",
                "item_key",
                "quantity",
            ]
            .map(str::to_owned),
        );
        for d in &report.drops {
            row(
                &mut out,
                &[
                    d.event_id.to_string(),
                    d.session_id.to_string(),
                    d.occurred_at.to_string(),
                    d.item_key.clone(),
                    d.quantity.clone(),
                ],
            );
        }
    } else {
        row(
            &mut out,
            &[
                "session_id",
                "character_id",
                "character_revision",
                "started_at",
                "ended_at",
                "duration_seconds",
                "experience_gained",
                "experience_lost",
            ]
            .map(str::to_owned),
        );
        for s in &report.sessions {
            row(
                &mut out,
                &[
                    s.id.to_string(),
                    s.character_id.to_string(),
                    s.character_revision.to_string(),
                    s.started_at.to_string(),
                    s.ended_at.to_string(),
                    s.duration_seconds.to_string(),
                    s.experience_gained.clone(),
                    s.experience_lost.clone(),
                ],
            );
        }
    }
    out
}

#[cfg(test)]
#[path = "reports_tests.rs"]
mod tests;
