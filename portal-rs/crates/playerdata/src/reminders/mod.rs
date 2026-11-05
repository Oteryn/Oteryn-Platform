//! Durable opt-in reminder intents; every delivery revalidates authoritative state.
use crate::dashboard::{Actor, Cut, Flags, Result, denied, fresh};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rent,
    Premium,
    Boss,
    Stamina,
    Bazaar,
}
impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::Rent => "rent",
            Self::Premium => "premium",
            Self::Boss => "boss",
            Self::Stamina => "stamina",
            Self::Bazaar => "bazaar",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Email,
    WebPush,
}
impl Channel {
    fn key(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::WebPush => "web_push",
        }
    }
    fn enabled(self, flags: Flags) -> bool {
        match self {
            Self::Email => flags.email,
            Self::WebPush => flags.web_push,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Intent {
    pub owner: Uuid,
    pub character: Option<Uuid>,
    pub cut: Option<Cut>,
    pub kind: Kind,
    pub entity: String,
    pub channel: Channel,
    pub due_at_ms: i64,
}
impl Intent {
    fn validate(&self) -> Result<()> {
        if self.owner.is_nil()
            || self.character.is_some_and(|id| id.is_nil())
            || self.character.is_some() != self.cut.is_some()
            || self.cut.is_some_and(|c| !c.valid())
            || self.due_at_ms < 0
            || self.entity.is_empty()
            || self.entity.len() > 512
            || !self
                .entity
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._:/-".contains(&c))
            || matches!(self.kind, Kind::Boss | Kind::Stamina) && self.character.is_none()
            || matches!(self.kind, Kind::Premium | Kind::Bazaar) && self.character.is_some()
        {
            return Err(denied());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Evidence {
    pub observed_at_ms: i64,
    pub clock_uncertainty_ms: u32,
    pub current_cut: Option<Cut>,
    pub baseline_complete: bool,
    pub active_account: bool,
    pub channel_opt_in: bool,
    pub source_due_at_ms: Option<i64>,
    /// Premium must agree with active, unrevoked commercial authority.
    pub premium_commercial_agrees: bool,
    /// Stamina only notifies on an observed 2520, never a predicted refill.
    pub stamina_minutes: Option<u16>,
}
/// Implement with T72, entitlement and bazaar public APIs. Missing adapters return None.
/// Implementations must check deletion, current source revision, recovery and ownership
/// at dispatch, including account-scoped records. Never trust an intent as authority.
pub trait Authority: std::fmt::Debug + Send + Sync {
    fn revalidate(&self, intent: &Intent, now_ms: i64) -> Result<Option<Evidence>>;
}
#[derive(Debug)]
pub struct Unavailable;
impl Authority for Unavailable {
    fn revalidate(&self, _: &Intent, _: i64) -> Result<Option<Evidence>> {
        Ok(None)
    }
}
fn eligible(flags: Flags, intent: &Intent, authority: &dyn Authority, now: i64) -> Result<bool> {
    intent.validate()?;
    if !flags.reminders || !intent.channel.enabled(flags) || matches!(intent.kind, Kind::Rent) {
        return Ok(false);
    }
    let Some(e) = authority.revalidate(intent, now)? else {
        return Ok(false);
    };
    Ok(e.active_account
        && e.channel_opt_in
        && e.baseline_complete
        && e.clock_uncertainty_ms <= 5000
        && fresh(e.observed_at_ms, now)
        && e.current_cut == intent.cut
        && e.source_due_at_ms == Some(intent.due_at_ms)
        && (!matches!(intent.kind, Kind::Premium) || e.premium_commercial_agrees)
        && (!matches!(intent.kind, Kind::Stamina) || e.stamina_minutes == Some(2520)))
}

#[derive(Debug)]
pub struct Reminders {
    pub pool: PgPool,
    pub flags: Flags,
}
impl Reminders {
    pub async fn schedule(
        &self,
        actor: Actor,
        intent: &Intent,
        authority: &dyn Authority,
        now: i64,
    ) -> Result<Uuid> {
        actor.authorize(intent.owner)?;
        if !eligible(self.flags, intent, authority, now)? {
            return Err(denied());
        }
        let cut = intent.cut.map(serde_json::to_value).transpose()?;
        let id = sqlx::query_scalar("INSERT INTO pd_reminders(id,account_id,character_id,kind,entity_key,channel,due_at_ms,cut) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(account_id,kind,entity_key,channel,due_at_ms) DO UPDATE SET updated_at=pd_reminders.updated_at WHERE pd_reminders.character_id IS NOT DISTINCT FROM EXCLUDED.character_id AND pd_reminders.cut IS NOT DISTINCT FROM EXCLUDED.cut RETURNING id")
            .bind(Uuid::now_v7()).bind(intent.owner).bind(intent.character).bind(intent.kind.key())
            .bind(&intent.entity).bind(intent.channel.key()).bind(intent.due_at_ms).bind(cut)
            .fetch_optional(&self.pool).await?.ok_or_else(denied)?;
        Ok(id)
    }

    /// Delivery adapter must durably enqueue once under this ID, recheck opt-in and
    /// authoritative evidence immediately before external delivery, and discard stale work.
    /// Enqueue in the supplied transaction so the outbox and intent commit atomically.
    /// No SMTP or push credentials are stored by playerdata.
    pub async fn dispatch(
        &self,
        id: Uuid,
        intent: &Intent,
        authority: &dyn Authority,
        delivery: &dyn Delivery,
        now: i64,
    ) -> Result<bool> {
        intent.validate()?;
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,)> = sqlx::query_as("SELECT state FROM pd_reminders WHERE id=$1 AND account_id=$2 AND character_id IS NOT DISTINCT FROM $3 AND kind=$4 AND entity_key=$5 AND channel=$6 AND due_at_ms=$7 AND cut IS NOT DISTINCT FROM $8 FOR UPDATE")
            .bind(id).bind(intent.owner).bind(intent.character).bind(intent.kind.key()).bind(&intent.entity)
            .bind(intent.channel.key()).bind(intent.due_at_ms).bind(intent.cut.map(serde_json::to_value).transpose()?)
            .fetch_optional(&mut *tx).await?;
        if row.as_ref().is_none_or(|r| r.0 != "pending") {
            return Ok(false);
        }
        if !eligible(self.flags, intent, authority, now)? {
            sqlx::query("UPDATE pd_reminders SET state='cancelled',updated_at=now() WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(false);
        }
        if now < intent.due_at_ms {
            return Ok(false);
        }
        delivery.enqueue(&mut tx, id, intent).await?;
        sqlx::query("UPDATE pd_reminders SET state='delivered',updated_at=now() WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
    pub async fn cancel(&self, actor: Actor, owner: Uuid, id: Uuid) -> Result<bool> {
        actor.authorize(owner)?;
        Ok(sqlx::query("UPDATE pd_reminders SET state='cancelled',updated_at=now() WHERE id=$1 AND account_id=$2 AND state='pending'").bind(id).bind(owner).execute(&self.pool).await?.rows_affected() == 1)
    }
}
/// Narrow seam: T36 has no reminder category or web-push API. An absent adapter
/// refuses; reminders must never be sent as news, support, or security messages.
pub trait Delivery: std::fmt::Debug + Send + Sync {
    fn enqueue<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        idempotency_key: Uuid,
        intent: &'a Intent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>;
}
impl Delivery for Unavailable {
    fn enqueue<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
        _: &'a Intent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Err(denied()) })
    }
}

pub fn message(language: &str, kind: Kind) -> Result<String> {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = match language {
        "pl" => include_str!("pl.ftl"),
        "en" => include_str!("en.ftl"),
        _ => return Err(denied()),
    };
    let mut bundle = FluentBundle::new(vec![language.parse::<unic_langid::LanguageIdentifier>()?]);
    bundle
        .add_resource(FluentResource::try_new(source.to_owned()).map_err(|_| denied())?)
        .map_err(|_| denied())?;
    let pattern = bundle
        .get_message(kind.key())
        .and_then(|m| m.value())
        .ok_or_else(denied)?;
    let mut errors = Vec::new();
    let text = bundle
        .format_pattern(pattern, None, &mut errors)
        .into_owned();
    if !errors.is_empty() {
        return Err(denied());
    }
    Ok(text)
}

#[cfg(test)]
mod tests;

/// G-11 premium alone uses microseconds. Round future deadlines up, never early.
/// None stays unknown and cannot authorize a reminder.
pub fn premium_deadline_ms(unix_us: Option<u64>) -> Result<Option<i64>> {
    unix_us
        .map(|us| {
            if us > 253_402_300_799_999_999 {
                return Err(denied());
            }
            let ms = us.div_ceil(1000);
            if ms > 253_402_300_799_999 {
                return Err(denied());
            }
            i64::try_from(ms).map_err(|_| denied())
        })
        .transpose()
}
