//! Private watch subscriptions. Canonical identities never become public lookup keys.
use oteryn_notify::{EmailTemplate, Locale, Notifications};
use sqlx::PgPool;
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

pub mod http;
#[cfg(test)]
mod tests;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type PolicyFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Character,
    Guild,
}
impl TargetKind {
    fn key(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Guild => "guild",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Online,
    LevelUp,
    Death,
    GuildChange,
}
impl EventKind {
    fn key(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::LevelUp => "level_up",
            Self::Death => "death",
            Self::GuildChange => "guild_change",
        }
    }
}
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subscription {
    pub target_kind: TargetKind,
    pub target_id: Uuid,
    pub event_kind: EventKind,
}
/// Only accepted projection ingestion may supply event identities; never HTTP callers.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub id: Uuid,
    pub character: Uuid,
    pub guild: Option<Uuid>,
    pub kind: EventKind,
}
/// T66 seam: decisions must use current independent privacy and source evidence.
/// Unknown, stale, hidden, ambiguous and unaccepted guild identity states return false.
/// `allowed` also verifies active recipient state, source event identity, guild membership and
/// the character's status/deaths/guild switch for the particular event field.
/// Both event-time disclosure and current disclosure must be allowed. A replay
/// must not disclose an event that was private when it occurred.
/// Implementations must serialize privacy changes with the supplied transaction.
pub trait Policy: std::fmt::Debug + Send + Sync {
    /// Validate active account, token scope, expiry and revocation on every request.
    fn owner<'a>(&'a self, bearer: &'a str) -> PolicyFuture<'a, Option<Uuid>>;
    fn allowed<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
        subscription: Subscription,
        event: Option<Event>,
    ) -> PolicyFuture<'a, bool>;
    fn locale(&self, owner: Uuid) -> PolicyFuture<'_, Locale>;
}
#[derive(Debug)]
pub struct Watches {
    pub pool: PgPool,
    pub enabled: bool,
    pub policy: Arc<dyn Policy>,
}
fn invalid() -> Box<dyn std::error::Error + Send + Sync> {
    "watch unavailable".into()
}
impl Watches {
    fn check(&self, owner: Uuid, s: Subscription) -> Result<()> {
        if !self.enabled || owner.is_nil() || s.target_id.is_nil() {
            return Err(invalid());
        }
        Ok(())
    }
    pub async fn subscribe(&self, owner: Uuid, s: Subscription) -> Result<()> {
        self.check(owner, s)?;
        let mut tx = self.pool.begin().await?;
        // Serialize per owner so concurrent requests cannot bypass the subscription cap.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,70))")
            .bind(owner.to_string())
            .execute(&mut *tx)
            .await?;
        if !self.policy.allowed(&mut tx, owner, s, None).await? {
            return Err(invalid());
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM stats_watches WHERE account_id=$1")
                .bind(owner)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 100 {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stats_watches WHERE account_id=$1 AND target_kind=$2 AND target_id=$3 AND event_kind=$4)")
                .bind(owner).bind(s.target_kind.key()).bind(s.target_id).bind(s.event_kind.key()).fetch_one(&mut *tx).await?;
            if !exists {
                return Err(invalid());
            }
        }
        sqlx::query("INSERT INTO stats_watches(account_id,target_kind,target_id,event_kind) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(owner).bind(s.target_kind.key()).bind(s.target_id).bind(s.event_kind.key()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Removal does not require the target to remain visible.
    pub async fn unsubscribe(&self, owner: Uuid, s: Subscription) -> Result<()> {
        self.check(owner, s)?;
        sqlx::query("DELETE FROM stats_watches WHERE account_id=$1 AND target_kind=$2 AND target_id=$3 AND event_kind=$4")
            .bind(owner).bind(s.target_kind.key()).bind(s.target_id).bind(s.event_kind.key()).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn list(&self, owner: Uuid) -> Result<Vec<Subscription>> {
        if !self.enabled || owner.is_nil() {
            return Err(invalid());
        }
        let mut tx = self.pool.begin().await?;
        let rows: Vec<(String,Uuid,String)> = sqlx::query_as("SELECT target_kind,target_id,event_kind FROM stats_watches WHERE account_id=$1 ORDER BY created_at,target_id,event_kind LIMIT 100")
            .bind(owner).fetch_all(&mut *tx).await?;
        let mut result = Vec::new();
        for (target, id, kind) in rows {
            let s: Subscription = serde_json::from_value(
                serde_json::json!({"target_kind":target,"target_id":id,"event_kind":kind}),
            )?;
            if self.policy.allowed(&mut tx, owner, s, None).await? {
                result.push(s);
            }
        }
        tx.commit().await?;
        Ok(result)
    }
    /// Retry-safe event fanout. No subject name, ID, event details or link is copied
    /// into durable notifications; their content remains safe after a privacy change.
    pub async fn dispatch(&self, notify: &Notifications, event: Event) -> Result<u64> {
        if !self.enabled
            || event.id.is_nil()
            || event.character.is_nil()
            || event.guild.is_some_and(|id| id.is_nil())
        {
            return Err(invalid());
        }
        let mut tx = self.pool.begin().await?;
        let rows: Vec<(Uuid,String,Uuid)> = sqlx::query_as("SELECT account_id,target_kind,target_id FROM stats_watches WHERE event_kind=$1 AND ((target_kind='character' AND target_id=$2) OR (target_kind='guild' AND target_id=$3)) ORDER BY account_id,target_kind,target_id FOR UPDATE")
            .bind(event.kind.key()).bind(event.character).bind(event.guild).fetch_all(&mut *tx).await?;
        let mut sent = 0;
        for (owner, target, id) in rows {
            let s = Subscription {
                target_kind: if target == "character" {
                    TargetKind::Character
                } else {
                    TargetKind::Guild
                },
                target_id: id,
                event_kind: event.kind,
            };
            if !self.policy.allowed(&mut tx, owner, s, Some(event)).await? {
                continue;
            }
            let inserted = sqlx::query("INSERT INTO stats_watch_deliveries(account_id,event_id) VALUES($1,$2) ON CONFLICT DO NOTHING")
                .bind(owner).bind(event.id).execute(&mut *tx).await?;
            if inserted.rows_affected() == 0 {
                continue;
            }
            let (title, body) = message(self.policy.locale(owner).await?)?;
            notify
                .enqueue(&mut tx, owner, &EmailTemplate::News { title, body })
                .await?;
            sent += 1;
        }
        tx.commit().await?;
        Ok(sent)
    }
}
fn message(locale: Locale) -> Result<(String, String)> {
    use fluent_bundle::{FluentResource, concurrent::FluentBundle};
    let (language, source) = match locale {
        Locale::Pl => (
            "pl",
            "title = Obserwowane\nbody = Sprawdź aktualnie publiczne informacje na liście obserwowanych.",
        ),
        Locale::En => (
            "en",
            "title = Watch list\nbody = Check the currently public information in your watch list.",
        ),
    };
    let mut bundle = FluentBundle::new_concurrent(vec![language.parse()?]);
    let resource = FluentResource::try_new(source.into()).map_err(|_| invalid())?;
    bundle.add_resource(resource).map_err(|_| invalid())?;
    let render = |key| -> Result<String> {
        let pattern = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or_else(invalid)?;
        let mut errors = Vec::new();
        let text = bundle
            .format_pattern(pattern, None, &mut errors)
            .into_owned();
        if !errors.is_empty() {
            return Err(invalid());
        }
        Ok(text)
    };
    Ok((render("title")?, render("body")?))
}
