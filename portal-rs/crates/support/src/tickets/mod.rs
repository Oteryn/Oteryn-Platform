//! Account-owned support tickets. Account locks serialize every limit-changing operation.
use oteryn_admin::{
    audit,
    rbac::{self, Authorizer, CheckContext, Permission},
    web::PortalIdentity,
};
use oteryn_notify::{EmailTemplate, Notifications};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) mod staff;
mod web;
pub use staff::{StaffSupport, configured_staff_router, configured_staff_router_from};
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
    #[error("request key is bound to a different ticket payload")]
    RequestKeyConflict,
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
enum Acknowledge {
    Never,
    DisplayedPage,
    Reply(Uuid),
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
                return Err(Error::RequestKeyConflict);
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
    ) -> Result<Uuid> {
        Ok(sqlx::query_scalar("INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,internal,body) VALUES($1,$2,$3,$4,$5) RETURNING id")
            .bind(id).bind(actor).bind(if staff {"staff"} else {"user"}).bind(internal).bind(body.trim()).fetch_one(&mut **tx).await?)
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
        self.change_direct(actor, staff, (id, version), change, None)
            .await
    }

    async fn change_direct(
        &self,
        actor: Uuid,
        staff: Option<CheckContext>,
        binding: (Uuid, i64),
        change: Change<'_>,
        reason: Option<&str>,
    ) -> Result<Ticket> {
        self.validate_change(actor, staff, binding.1, change)?;
        let mut tx = self.pool.begin().await?;
        let (ticket, _, _) = self
            .change_in(
                &mut tx,
                actor,
                staff,
                binding,
                change,
                MutationSource::Direct(reason),
            )
            .await?;
        if let Some(context) = staff {
            self.check_staff_permissions(&mut tx, context).await?;
        }
        tx.commit().await?;
        Ok(ticket)
    }
    async fn check_staff_permissions(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
    ) -> Result<()> {
        let auth = Authorizer::new(
            self.pool.clone(),
            PortalIdentity::configured(self.pool.clone()),
        );
        for permission in [Permission::Access, Permission::ManageSupportTickets] {
            auth.check_in(tx, context, permission)
                .await
                .map_err(|error| match error {
                    rbac::Error::Storage(error) => Error::Database(error),
                    _ => Error::Denied,
                })?;
        }
        Ok(())
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
        source: MutationSource<'_>,
    ) -> Result<(Ticket, String, Option<Uuid>)> {
        let (id, version) = binding;
        let is_staff = self.validate_change(actor, staff, version, change)?;
        // Always acquire account locks before ticket locks, including reopen operations.
        let actor_state = oteryn_identity::accounts::Accounts::find_by_id_in(tx, actor)
            .await?
            .ok_or(Error::Denied)?;
        if actor_state.status != "active" {
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
        // Include the author's implicit FK lock and RBAC's account SHARE lock.
        // Crossed staff replies must acquire the same pair in UUID order.
        if let Some(context) = staff {
            let auth = Authorizer::new(
                self.pool.clone(),
                PortalIdentity::configured(self.pool.clone()),
            );
            rbac::authorize_mutation_with_resources(
                tx,
                &[actor, owner],
                move |connection| {
                    Box::pin(async move {
                        sqlx::query("SELECT id FROM tickets WHERE id=$1 FOR UPDATE")
                            .bind(id)
                            .execute(connection)
                            .await?;
                        Ok(())
                    })
                },
                &auth,
                context,
                &[Permission::Access, Permission::ManageSupportTickets],
            )
            .await
            .map_err(|error| match error {
                rbac::Error::Storage(error) => Error::Database(error),
                _ => Error::Denied,
            })?;
        } else {
            for account in if actor < owner {
                [actor, owner]
            } else {
                [owner, actor]
            } {
                self.lock_account(tx, account).await?;
            }
        }
        let mut ticket: Ticket = sqlx::query_as(
            "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE",
        )
        .bind(id)
        .bind(is_staff)
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
        if let Some(context) = staff {
            self.check_staff_permissions(tx, context).await?;
        }
        if !STATUSES.contains(&ticket.status.as_str()) {
            return Err(Error::Denied);
        }
        if ticket.lock_version != version {
            return Err(Error::Conflict);
        }
        let before = ticket.status.clone();
        let (status, notify, message_id) = match change {
            Change::Reply { body, internal } => {
                if (is_staff && !active(&ticket.status))
                    || (!is_staff && !matches!(ticket.status.as_str(), "open" | "waiting_user"))
                {
                    return Err(Error::Conflict);
                }
                let message_id = Self::message(tx, id, actor, body, is_staff, internal).await?;
                (
                    if internal {
                        ticket.status.clone()
                    } else if is_staff {
                        "waiting_user".into()
                    } else {
                        "waiting_staff".into()
                    },
                    is_staff && !internal,
                    Some(message_id),
                )
            }
            Change::Status(status) => {
                if !is_staff && status == "open" && ticket.status != "closed" {
                    return Err(Error::Conflict);
                }
                if active(status) && !active(&ticket.status) {
                    self.capacity(tx, ticket.account_id).await?;
                }
                (status.to_owned(), is_staff, None)
            }
        };
        let reply = message_id.is_some();
        ticket = sqlx::query_as("UPDATE tickets SET status=$2,lock_version=lock_version+1,unread_reply=CASE WHEN $3 THEN true ELSE unread_reply END,last_message_at=CASE WHEN $4 THEN now() ELSE last_message_at END,closed_at=CASE WHEN $2 IN ('resolved','closed') THEN now() ELSE NULL END,updated_at=now() WHERE id=$1 RETURNING *")
            .bind(id).bind(status).bind(notify && reply).bind(reply).fetch_one(&mut **tx).await?;
        if let MutationSource::Direct(reason) = source
            && is_staff
        {
            let mut after = ticket.audit_change(message_id);
            if let Some(reason) = reason {
                use sha2::{Digest, Sha256};
                after["reason_sha256"] =
                    serde_json::json!(format!("{:x}", Sha256::digest(reason.trim().as_bytes())));
            }
            audit::append(
                tx,
                audit::Entry {
                    context: audit::AuditContext {
                        actor: Some(actor),
                        request_id: Uuid::now_v7(),
                    },
                    action: match change {
                        Change::Reply { internal: true, .. } => {
                            "support.ticket_internal_note_added"
                        }
                        Change::Reply { .. } => "support.ticket_replied",
                        Change::Status(_) => "support.ticket_status_changed",
                    },
                    target: &id.to_string(),
                    before: serde_json::json!({"status":before,"lock_version":version}),
                    after,
                },
            )
            .await
            .map_err(|_| Error::Denied)?;
        }
        if notify {
            let owner_locale =
                oteryn_identity::accounts::Accounts::find_by_id_in(tx, ticket.account_id)
                    .await?
                    .ok_or(Error::Denied)?
                    .locale;
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
            if matches!(source, MutationSource::Direct(_)) {
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
        Ok((ticket, before, message_id))
    }
    pub async fn list(&self, account: Uuid, staff: bool, page: u32) -> Result<Vec<Ticket>> {
        self.enabled()?;
        sqlx::query_as("SELECT * FROM tickets WHERE ($1 OR account_id=$2) ORDER BY last_message_at DESC,id DESC LIMIT 20 OFFSET $3")
            .bind(staff).bind(account).bind(i64::from(page)*20).fetch_all(&self.pool).await.map_err(Into::into)
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
        self.load_page(account, staff, id, after, Acknowledge::DisplayedPage)
            .await
            .map(|(ticket, messages, _)| (ticket, messages))
    }

    async fn load_page(
        &self,
        account: Uuid,
        staff: bool,
        id: Uuid,
        after: Option<Uuid>,
        acknowledge: Acknowledge,
    ) -> Result<(Ticket, Vec<Message>, Option<Uuid>)> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let ticket: Ticket = sqlx::query_as(
            "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE",
        )
        .bind(id)
        .bind(staff)
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let messages: Vec<Message> = sqlx::query_as(
            "SELECT * FROM ticket_messages WHERE ticket_id=$1 AND ($2 OR NOT internal) AND ($3::uuid IS NULL OR id>$3) ORDER BY id LIMIT 50",
        )
        .bind(id)
        .bind(staff)
        .bind(after)
        .fetch_all(&mut *tx)
        .await?;
        let mut read_reply = None;
        if ticket.unread_reply && !staff {
            // The ticket lock fences concurrent replies. A durable message ID
            // identifies the newest public reply; only its displayed page may
            // acknowledge it. Owner mutations establish no viewing boundary.
            let boundary: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ticket_messages WHERE ticket_id=$1 AND author_kind='staff' AND NOT internal ORDER BY id DESC LIMIT 1")
                .bind(id).fetch_optional(&mut *tx).await?;
            read_reply = boundary.filter(|id| messages.iter().any(|m| m.id == *id));
            if let Some(boundary) = read_reply
                && match acknowledge {
                    Acknowledge::Never => false,
                    Acknowledge::DisplayedPage => true,
                    Acknowledge::Reply(reply) => reply == boundary,
                }
            {
                sqlx::query("UPDATE tickets SET unread_reply=false WHERE id=$1 AND account_id=$2 AND NOT EXISTS (SELECT 1 FROM ticket_messages WHERE ticket_id=$1 AND author_kind='staff' AND NOT internal AND id>$3)")
                    .bind(id).bind(account).bind(boundary).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok((ticket, messages, read_reply))
    }
}
impl Ticket {
    fn audit_change(&self, message_id: Option<Uuid>) -> serde_json::Value {
        let mut after = serde_json::json!({"status": self.status, "lock_version": self.lock_version});
        if let Some(id) = message_id {
            after["message_id"] = serde_json::json!(id);
        }
        after
    }
}

pub(crate) enum MutationSource<'a> {
    Direct(Option<&'a str>),
    Queue,
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
}
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            sqlx::query("DELETE FROM tickets WHERE id IN (SELECT id FROM tickets WHERE status IN ('resolved','closed') AND closed_at < now()-interval '180 days' LIMIT 1000 FOR UPDATE SKIP LOCKED)").execute(&self.pool).await.map(|_|()).map_err(|e|e.to_string())
        })
    }
}

#[cfg(test)]
mod staff_tests;
