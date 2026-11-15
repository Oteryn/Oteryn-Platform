//! Account-owned support tickets. Account locks serialize every limit-changing operation.
use oteryn_admin::{audit, rbac::CheckContext};
use oteryn_notify::{EmailTemplate, Notifications};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) mod staff;
mod web;
pub use staff::{StaffSupport, configured_staff_router};
pub use web::{configured_router, router, staff_router};
#[cfg(test)]
mod tests;
mod text;

pub const CATEGORIES: &[&str] = &["account", "character", "technical", "payment", "other"];
pub const STATUSES: &[&str] = &[
    "open",
    "waiting_user",
    "waiting_staff",
    "resolved",
    "closed",
];
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input")]
    Invalid,
    #[error("not found")]
    NotFound,
    #[error("access denied")]
    Denied,
    #[error("ticket conflict")]
    Conflict,
    #[error("database unavailable")]
    Database(#[from] sqlx::Error),
    #[error("notification unavailable")]
    Notification,
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Ticket {
    pub id: Uuid,
    pub account_id: Uuid,
    pub category: String,
    pub subject: String,
    pub status: String,
    pub lock_version: i64,
    pub unread_reply: bool,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub body: String,
    pub author_kind: String,
    pub internal: bool,
}
#[derive(Debug, Clone)]
pub struct Tickets {
    pub pool: PgPool,
    pub notifications: Arc<Notifications>,
    pub enabled: bool,
    pub open_limit: u32,
}
fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max && !value.contains('\0')
}
fn active(status: &str) -> bool {
    matches!(status, "open" | "waiting_user" | "waiting_staff")
}
impl Tickets {
    fn enabled(&self) -> Result<()> {
        if !self.enabled || self.open_limit == 0 {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn lock_account(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> Result<String> {
        self.enabled()?;
        let state: Option<(String, String)> =
            sqlx::query_as("SELECT status,locale FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_optional(&mut **tx)
                .await?;
        match state {
            Some((status, locale))
                if status == "active" && matches!(locale.as_str(), "pl" | "en") =>
            {
                Ok(locale)
            }
            _ => Err(Error::Denied),
        }
    }
    async fn capacity(&self, tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tickets WHERE account_id=$1 AND status NOT IN ('resolved','closed')")
            .bind(account).fetch_one(&mut **tx).await?;
        if count >= i64::from(self.open_limit) {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub async fn create(
        &self,
        account: Uuid,
        key: Uuid,
        category: &str,
        subject: &str,
        body: &str,
    ) -> Result<Ticket> {
        self.enabled()?;
        if key.is_nil()
            || !CATEGORIES.contains(&category)
            || !valid_text(subject, 160)
            || !valid_text(body, 8000)
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        self.lock_account(&mut tx, account).await?;
        if let Some(ticket) = sqlx::query_as::<_, Ticket>(
            "SELECT * FROM tickets WHERE account_id=$1 AND request_key=$2",
        )
        .bind(account)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
        {
            let original: String = sqlx::query_scalar(
                "SELECT body FROM ticket_messages WHERE ticket_id=$1 ORDER BY id LIMIT 1",
            )
            .bind(ticket.id)
            .fetch_one(&mut *tx)
            .await?;
            if ticket.category != category
                || ticket.subject != subject.trim()
                || original != body.trim()
            {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(ticket);
        }
        self.capacity(&mut tx, account).await?;
        let ticket: Ticket = sqlx::query_as("INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,$2,$3,$4) RETURNING *")
            .bind(account).bind(key).bind(category).bind(subject.trim()).fetch_one(&mut *tx).await?;
        Self::message(&mut tx, ticket.id, account, body, false, false).await?;
        tx.commit().await?;
        Ok(ticket)
    }
    async fn message(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        actor: Uuid,
        body: &str,
        staff: bool,
        internal: bool,
    ) -> Result<()> {
        sqlx::query("INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,internal,body) VALUES($1,$2,$3,$4,$5)")
            .bind(id).bind(actor).bind(if staff {"staff"} else {"user"}).bind(internal).bind(body.trim()).execute(&mut **tx).await?;
        Ok(())
    }
    fn validate_change(
        &self,
        actor: Uuid,
        staff: Option<CheckContext>,
        version: i64,
        change: Change<'_>,
    ) -> Result<bool> {
        self.enabled()?;
        if version < 1 {
            return Err(Error::Invalid);
        }
        let is_staff =
            staff.is_some_and(|c| c.account == actor && c.mfa_verified && c.require_staff_mfa);
        if staff.is_some() && !is_staff {
            return Err(Error::Denied);
        }
        match change {
            Change::Reply { body, internal }
                if !valid_text(body, 8000) || (internal && !is_staff) =>
            {
                return Err(Error::Invalid);
            }
            Change::Status(status)
                if !STATUSES.contains(&status)
                    || (!is_staff && !matches!(status, "open" | "closed")) =>
            {
                return Err(Error::Invalid);
            }
            _ => {}
        }
        Ok(is_staff)
    }
    /// Staff context must come from the guarded route extension, never form input.
    pub async fn change(
        &self,
        actor: Uuid,
        staff: Option<CheckContext>,
        id: Uuid,
        version: i64,
        change: Change<'_>,
    ) -> Result<Ticket> {
        self.validate_change(actor, staff, version, change)?;
        let mut tx = self.pool.begin().await?;
        let (ticket, _) = self
            .change_in(&mut tx, actor, staff, (id, version), change, true)
            .await?;
        tx.commit().await?;
        Ok(ticket)
    }
    /// Caller retains the authorization/audit transaction. The queue adapter
    /// supplies its audit through the admin execute boundary.
    pub(crate) async fn change_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        staff: Option<CheckContext>,
        binding: (Uuid, i64),
        change: Change<'_>,
        audit_staff: bool,
    ) -> Result<(Ticket, String)> {
        let (id, version) = binding;
        let is_staff = self.validate_change(actor, staff, version, change)?;
        // Always acquire account locks before ticket locks, including reopen operations.
        let actor_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active')",
        )
        .bind(actor)
        .fetch_one(&mut **tx)
        .await?;
        if !actor_active {
            return Err(Error::Denied);
        }
        let owner: Uuid = sqlx::query_scalar(
            "SELECT account_id FROM tickets WHERE id=$1 AND ($2 OR account_id=$3)",
        )
        .bind(id)
        .bind(is_staff)
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
        self.lock_account(tx, owner).await?;
        let mut ticket: Ticket = sqlx::query_as(
            "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE",
        )
        .bind(id)
        .bind(is_staff)
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
        if !STATUSES.contains(&ticket.status.as_str()) {
            return Err(Error::Denied);
        }
        if ticket.lock_version != version {
            return Err(Error::Conflict);
        }
        let before = ticket.status.clone();
        let (status, notify, reply) = match change {
            Change::Reply { body, internal } => {
                if (is_staff && !active(&ticket.status))
                    || (!is_staff && !matches!(ticket.status.as_str(), "open" | "waiting_user"))
                {
                    return Err(Error::Conflict);
                }
                Self::message(tx, id, actor, body, is_staff, internal).await?;
                (
                    if internal {
                        ticket.status.clone()
                    } else if is_staff {
                        "waiting_user".into()
                    } else {
                        "waiting_staff".into()
                    },
                    is_staff && !internal,
                    true,
                )
            }
            Change::Status(status) => {
                if !is_staff && status == "open" && ticket.status != "closed" {
                    return Err(Error::Conflict);
                }
                if active(status) && !active(&ticket.status) {
                    self.capacity(tx, ticket.account_id).await?;
                }
                (status.to_owned(), is_staff, false)
            }
        };
        ticket = sqlx::query_as("UPDATE tickets SET status=$2,lock_version=lock_version+1,unread_reply=CASE WHEN $3 THEN true WHEN NOT $4 THEN false ELSE unread_reply END,last_message_at=CASE WHEN $5 THEN now() ELSE last_message_at END,closed_at=CASE WHEN $2 IN ('resolved','closed') THEN now() ELSE NULL END,updated_at=now() WHERE id=$1 RETURNING *")
            .bind(id).bind(status).bind(notify && reply).bind(is_staff).bind(reply).fetch_one(&mut **tx).await?;
        if is_staff && audit_staff {
            audit::append(tx,audit::Entry {context:audit::AuditContext {actor:Some(actor),request_id:Uuid::now_v7()},action:match change { Change::Reply {internal:true,..}=>"support.ticket_internal_note_added",Change::Reply {..}=>"support.ticket_replied",Change::Status(_)=>"support.ticket_status_changed"},target:&id.to_string(),before:serde_json::json!({"status":before,"lock_version":version}),after:serde_json::json!({"status":ticket.status,"lock_version":ticket.lock_version})}).await.map_err(|_|Error::Denied)?;
        }
        if notify {
            let owner_locale: String =
                sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1")
                    .bind(ticket.account_id)
                    .fetch_one(&mut **tx)
                    .await?;
            let body = format!(
                "{} /{}/support/tickets/{}",
                text::text(
                    &owner_locale,
                    if reply {
                        "notification-reply"
                    } else {
                        "notification-status"
                    }
                ),
                owner_locale,
                id
            );
            if audit_staff {
                self.notifications
                    .enqueue(tx, ticket.account_id, &EmailTemplate::Support { body })
                    .await
                    .map_err(|_| Error::Notification)?;
            } else {
                // Queue mutations use the locked account snapshot, so a caller
                // holding the sole pool connection never re-enters the pool.
                staff::notify(tx, &self.notifications, ticket.account_id, body)
                    .await
                    .map_err(|_| Error::Notification)?;
            }
        }
        Ok((ticket, before))
    }
    pub async fn list(&self, account: Uuid, staff: bool, page: u32) -> Result<Vec<Ticket>> {
        self.enabled()?;
        sqlx::query_as("SELECT * FROM tickets WHERE ($1 OR account_id=$2) ORDER BY last_message_at DESC,id DESC LIMIT 20 OFFSET $3")
            .bind(staff).bind(account).bind(i64::from(page.min(10000))*20).fetch_all(&self.pool).await.map_err(Into::into)
    }
    pub async fn show(
        &self,
        account: Uuid,
        staff: bool,
        id: Uuid,
    ) -> Result<(Ticket, Vec<Message>)> {
        self.show_page(account, staff, id, None).await
    }
    pub async fn show_page(
        &self,
        account: Uuid,
        staff: bool,
        id: Uuid,
        after: Option<Uuid>,
    ) -> Result<(Ticket, Vec<Message>)> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let ticket = sqlx::query_as(
            "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE",
        )
        .bind(id)
        .bind(staff)
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let messages = sqlx::query_as(
            "SELECT * FROM ticket_messages WHERE ticket_id=$1 AND ($2 OR NOT internal) AND ($3::uuid IS NULL OR id>$3) ORDER BY id LIMIT 50",
        )
        .bind(id)
        .bind(staff)
        .bind(after)
        .fetch_all(&mut *tx)
        .await?;
        if !staff {
            sqlx::query("UPDATE tickets SET unread_reply=false WHERE id=$1 AND account_id=$2")
                .bind(id)
                .bind(account)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok((ticket, messages))
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Change<'a> {
    Reply { body: &'a str, internal: bool },
    Status(&'a str),
}

/// Deletes only terminal tickets after the documented 180-day retention window.
#[derive(Debug, Clone)]
pub struct Retention {
    pub pool: PgPool,
    pub enabled: bool,
}
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            if !self.enabled {
                return Ok(());
            }
            sqlx::query("DELETE FROM tickets WHERE id IN (SELECT id FROM tickets WHERE status IN ('resolved','closed') AND closed_at < now()-interval '180 days' LIMIT 1000 FOR UPDATE SKIP LOCKED)").execute(&self.pool).await.map(|_|()).map_err(|e|e.to_string())
        })
    }
}

#[cfg(test)]
mod staff_tests;
