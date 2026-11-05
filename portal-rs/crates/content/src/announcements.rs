//! Explicitly published, time bounded editorial announcements.
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid announcement")]
    Invalid,
    #[error("announcement version conflict")]
    Conflict,
    #[error(transparent)]
    Authorization(#[from] oteryn_admin::rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub publication_state: String,
    pub severity: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub title: String,
    pub body: String,
    pub action_label: Option<String>,
    pub action_url: Option<String>,
    pub title_pl: Option<String>,
    pub body_pl: Option<String>,
    pub action_label_pl: Option<String>,
    pub translation_published_at: Option<OffsetDateTime>,
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max
}
/// Only unambiguous first-party paths can become public actions.
pub fn safe_action(s: &str) -> bool {
    s.starts_with('/')
        && !s.starts_with("//")
        && s.len() <= 2048
        && s.bytes().all(|b| {
            b.is_ascii_graphic()
                && !matches!(b, b'\\' | b'<' | b'>' | b'"' | b'\'' | b'%' | b'?' | b'#')
        })
}
impl Input {
    pub fn validate(&self) -> Result<(), Error> {
        let action = match (&self.action_label, &self.action_url) {
            (None, None) => true,
            (Some(label), Some(url)) => bounded(label, 100) && safe_action(url),
            _ => false,
        };
        let translated = self.title_pl.as_deref().is_some_and(|s| bounded(s, 200))
            && self.body_pl.as_deref().is_some_and(|s| bounded(s, 10000))
            && (self.action_url.is_none()
                || self
                    .action_label_pl
                    .as_deref()
                    .is_some_and(|s| bounded(s, 100)));
        if !matches!(self.publication_state.as_str(), "draft" | "published")
            || !matches!(self.severity.as_str(), "info" | "maintenance" | "warning")
            || self.ends_at.is_some_and(|end| end <= self.starts_at)
            || !bounded(&self.title, 200)
            || !bounded(&self.body, 10000)
            || !action
            || self.title_pl.as_deref().is_some_and(|s| !bounded(s, 200))
            || self.body_pl.as_deref().is_some_and(|s| !bounded(s, 10000))
            || self
                .action_label_pl
                .as_deref()
                .is_some_and(|s| !bounded(s, 100))
            || (self.translation_published_at.is_some() && !translated)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Announcement {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub severity: String,
    pub action_label: Option<String>,
    pub action_url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Announcements {
    pool: PgPool,
}
impl Announcements {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn active(
        &self,
        lang: &str,
        at: OffsetDateTime,
        limit: i64,
    ) -> Result<Vec<Announcement>, Error> {
        if !matches!(lang, "pl" | "en") || !(1..=10).contains(&limit) {
            return Err(Error::Invalid);
        }
        Ok(sqlx::query_as("SELECT id,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body,severity,CASE WHEN $1='pl' THEN action_label_pl ELSE action_label END AS action_label,action_url FROM announcements WHERE publication_state='published' AND severity IN ('info','maintenance','warning') AND starts_at <= $2 AND (ends_at IS NULL OR ends_at > $2) AND ($1='en' OR (translation_published_at <= $2 AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0 AND (action_url IS NULL OR length(trim(action_label_pl))>0))) ORDER BY severity DESC,starts_at DESC,id DESC LIMIT $3").bind(lang).bind(at).bind(limit).fetch_all(&self.pool).await?)
    }
    /// An edit uses an optimistic version and records the staff action atomically.
    pub async fn save<A: AccountStateReader>(
        &self,
        auth: &Authorizer<A>,
        context: CheckContext,
        existing: Option<(Uuid, i64)>,
        input: Input,
    ) -> Result<Uuid, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        auth.check(context, Permission::ManagePortalAnnouncements)
            .await?;
        input.validate()?;
        let id = existing.map_or_else(Uuid::now_v7, |v| v.0);
        let (before, previous_publication, previous_source) = if let Some((_, version)) = existing {
            type StoredVersion = (i64, String, Option<OffsetDateTime>, Option<OffsetDateTime>);
            let current: Option<StoredVersion> = sqlx::query_as(
                "SELECT lock_version,publication_state,translation_published_at,translation_source_updated_at FROM announcements WHERE id=$1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
            let Some((actual, state, publication, source)) = current else {
                return Err(Error::Conflict);
            };
            if actual != version || version < 1 || !matches!(state.as_str(), "draft" | "published")
            {
                return Err(Error::Conflict);
            }
            (json!({"version":actual,"state":state}), publication, source)
        } else {
            (json!(null), None, None)
        };
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        // An unchanged publication stamp cannot reapprove a stale translation.
        let source = input.translation_published_at.and_then(|stamp| {
            if existing.is_none() || Some(stamp) != previous_publication {
                Some(now)
            } else {
                previous_source
            }
        });
        sqlx::query("INSERT INTO announcements (id,publication_state,severity,starts_at,ends_at,title,body,action_label,action_url,title_pl,body_pl,action_label_pl,translation_published_at,translation_source_updated_at,created_by,updated_by,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$16,$15,$15,$14) ON CONFLICT (id) DO UPDATE SET publication_state=$2,severity=$3,starts_at=$4,ends_at=$5,title=$6,body=$7,action_label=$8,action_url=$9,title_pl=$10,body_pl=$11,action_label_pl=$12,translation_published_at=$13,translation_source_updated_at=$16,updated_by=$15,updated_at=$14,lock_version=announcements.lock_version+1")
            .bind(id).bind(&input.publication_state).bind(&input.severity).bind(input.starts_at).bind(input.ends_at).bind(&input.title).bind(&input.body).bind(&input.action_label).bind(&input.action_url).bind(&input.title_pl).bind(&input.body_pl).bind(&input.action_label_pl).bind(input.translation_published_at).bind(now).bind(context.account).bind(source).execute(&mut *tx).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: "portal.announcement_saved",
                target: &id.to_string(),
                before,
                after: json!({"state":input.publication_state}),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
}
#[cfg(test)]
#[path = "announcements_tests.rs"]
mod tests;

/// Installs the editorial banner on public HTML responses only.
pub async fn banner(
    axum::extract::State(pool): axum::extract::State<PgPool>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::{
        body::{Body, to_bytes},
        http::{HeaderValue, Method, StatusCode, header},
        response::IntoResponse,
    };
    let path = request.uri().path();
    let eligible = request.method() == Method::GET
        && ![
            "/admin",
            "/account",
            "/api",
            "/internal",
            "/oauth",
            "/static",
        ]
        .iter()
        .any(|p| path.starts_with(p));
    let lang = if path.starts_with("/en/")
        || request
            .uri()
            .query()
            .is_some_and(|q| q.split('&').any(|p| p == "lang=en"))
    {
        "en"
    } else {
        "pl"
    };
    let mut response = next.run(request).await;
    if !eligible
        || !response.status().is_success()
        || !response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|v| v.as_bytes().starts_with(b"text/html"))
    {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, 2 * 1024 * 1024).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Ok(mut html) = String::from_utf8(bytes.to_vec()) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    // Fragments and non-page responses are left intact.
    if let Some(position) = html.find("<main") {
        let ticker = crate::today::render_ticker(&pool, lang, OffsetDateTime::now_utc()).await;
        html.insert_str(position, &ticker);
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, max-age=0"),
        );
    }
    response = axum::response::Response::from_parts(parts, Body::from(html));
    response
}
