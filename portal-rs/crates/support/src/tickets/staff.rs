//! Production staff queue adapter. Domain writes share the admin authorization/audit transaction.
use super::*;
use oteryn_admin::support::{
    Area, Backend, Command, Error as StaffError, Message as StaffMessage, Record,
};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct StaffSupport {
    pool: PgPool,
    tickets: Option<Tickets>,
    reports: crate::reports::Reports,
    sanctions: crate::sanctions::Service,
    notifications: Option<Arc<Notifications>>,
    reports_enabled: bool,
    sanctions_enabled: bool,
}
impl StaffSupport {
    pub fn configured(pool: PgPool) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        Self::configured_from(pool, &|key| std::env::var(key).ok())
    }
    /// [`Self::configured`] reading its configuration through `lookup`.
    pub fn configured_from(
        pool: PgPool,
        lookup: &dyn Fn(&str) -> Option<String>,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let tickets_enabled = lookup("OTERYN_SUPPORT_TICKETS").as_deref() == Some("true");
        let pending_limit =
            lookup("OTERYN_REPORTS_PENDING_LIMIT").map_or(5, |v| v.parse().unwrap_or(0));
        let unjustified_limit =
            lookup("OTERYN_REPORTS_UNJUSTIFIED_LIMIT").map_or(3, |v| v.parse().unwrap_or(0));
        let reports_enabled = lookup("OTERYN_REPORTS_ENABLED").as_deref() == Some("true")
            && (1..=20).contains(&pending_limit)
            && unjustified_limit > 0;
        let sanctions_enabled = lookup("OTERYN_ADMIN_PANEL").as_deref() == Some("true");
        let notifications = if tickets_enabled || reports_enabled || sanctions_enabled {
            Some(Arc::new(Notifications {
                pool: pool.clone(),
                accounts: Arc::new(web::NotifyAccounts(pool.clone())),
                unsubscribe: oteryn_notify::Unsubscribe::new(
                    lookup("OTERYN_NOTIFY_UNSUBSCRIBE_KEY")
                        .ok_or("missing unsubscribe key")?
                        .into_bytes(),
                    lookup("OTERYN_NOTIFY_UNSUBSCRIBE_URL").ok_or("missing unsubscribe url")?,
                )
                .map_err(|_| "invalid notification configuration")?,
            }))
        } else {
            None
        };
        let tickets = if tickets_enabled {
            Some(Tickets {
                pool: pool.clone(),
                notifications: notifications.clone().ok_or("notifications unavailable")?,
                enabled: true,
                open_limit: 10,
                public_origin: lookup("OTERYN_PUBLIC_ORIGIN").ok_or("missing public origin")?,
            })
        } else {
            None
        };
        Ok(Self {
            reports: crate::reports::Reports::new(
                pool.clone(),
                crate::reports::Policy {
                    enabled: reports_enabled,
                    pending_limit,
                    unjustified_limit,
                },
            ),
            sanctions: crate::sanctions::Service::new(
                pool.clone(),
                crate::sanctions::Features {
                    sanctions: sanctions_enabled,
                    game_access: false,
                },
            ),
            pool,
            tickets,
            notifications,
            reports_enabled,
            sanctions_enabled,
        })
    }
    fn available(&self, area: Area) -> std::result::Result<(), StaffError> {
        if match area {
            Area::Tickets => self.tickets.is_some(),
            Area::Reports => self.reports_enabled,
            Area::Enforcement => self.sanctions_enabled,
        } {
            Ok(())
        } else {
            Err(StaffError::NotFound)
        }
    }
}
/// The same composition used by the portal binary; admin supplies guards, support supplies services.
pub fn configured_staff_router(
    pool: PgPool,
) -> std::result::Result<axum::Router, Box<dyn std::error::Error>> {
    configured_staff_router_from(pool, &|key| std::env::var(key).ok())
}
/// [`configured_staff_router`] reading its configuration through `lookup`.
pub fn configured_staff_router_from(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> std::result::Result<axum::Router, Box<dyn std::error::Error>> {
    oteryn_admin::support::web::configured_router_from(
        pool.clone(),
        StaffSupport::configured_from(pool, lookup)?,
        lookup,
    )
}
impl Backend for StaffSupport {
    async fn prepare(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: oteryn_admin::rbac::CheckContext,
        id: Uuid,
        command: &Command,
    ) -> std::result::Result<(), StaffError> {
        let actor = context.account;
        self.available(command.area())?;
        let owner: Uuid = match command {
            Command::Reply { .. } | Command::TicketStatus { .. } => {
                sqlx::query_scalar("SELECT account_id FROM tickets WHERE id=$1")
                    .bind(id)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(|_| StaffError::Unavailable)?
                    .ok_or(StaffError::NotFound)?
            }
            Command::Moderate { .. } => {
                sqlx::query_scalar("SELECT reporter_account_id FROM player_reports WHERE id=$1")
                    .bind(id)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(|_| StaffError::Unavailable)?
                    .ok_or(StaffError::NotFound)?
            }
            Command::Sanction {
                create: true,
                target,
                ..
            } => *target,
            _ => sqlx::query_scalar("SELECT account_id FROM sanctions WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?
                .ok_or(StaffError::NotFound)?,
        };
        let auth = oteryn_admin::rbac::Authorizer::new(
            self.pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(self.pool.clone()),
        );
        let resource_query = match command {
            Command::Reply { .. } | Command::TicketStatus { .. } => {
                "SELECT id FROM tickets WHERE id=$1 FOR UPDATE"
            }
            Command::Moderate { .. } => "SELECT id FROM player_reports WHERE id=$1 FOR UPDATE",
            Command::Sanction { create: true, .. } => {
                "SELECT id FROM sanctions WHERE false AND id=$1 FOR UPDATE"
            }
            _ => "SELECT id FROM sanctions WHERE id=$1 FOR UPDATE",
        };
        let enforcement = matches!(command.area(), Area::Enforcement);
        oteryn_admin::rbac::authorize_mutation_with_resources(
            tx,
            &[actor, owner],
            move |connection| {
                Box::pin(async move {
                    sqlx::query(resource_query)
                        .bind(id)
                        .execute(&mut *connection)
                        .await?;
                    if enforcement {
                        sqlx::query("SELECT sanction_id FROM support_appeal_workflows WHERE sanction_id=$1 AND owner_id=$2 FOR UPDATE")
                            .bind(id).bind(owner).execute(connection).await?;
                    }
                    Ok(())
                })
            },
            &auth,
            context,
            &[
                oteryn_admin::rbac::Permission::Access,
                command.area().permission(),
            ],
        )
        .await
        .map_err(|error| match error {
            oteryn_admin::rbac::Error::Storage(_) => StaffError::Unavailable,
            _ => StaffError::Denied,
        })?;
        // Include implicit author FK locks and RBAC's actor SHARE lock. Every
        // domain uses the same UUID order before joining the role-writer fence.
        if matches!(
            command,
            Command::Reply { .. } | Command::TicketStatus { .. }
        ) {
            let tickets = self.tickets.as_ref().ok_or(StaffError::NotFound)?;
            for account in if actor < owner {
                [actor, owner]
            } else {
                [owner, actor]
            } {
                tickets
                    .lock_account(tx, account)
                    .await
                    .map_err(ticket_error)?;
            }
        }
        Ok(())
    }

    async fn list(&self, area: Area, page: u32) -> std::result::Result<Vec<Record>, StaffError> {
        self.available(area)?;
        let query = match area {
            Area::Tickets => {
                "SELECT id,lock_version,subject,status FROM tickets ORDER BY last_message_at DESC,id DESC LIMIT 30 OFFSET $1"
            }
            Area::Reports => {
                "SELECT id,lock_version,target_reference,status FROM player_reports ORDER BY created_at DESC,id DESC LIMIT 30 OFFSET $1"
            }
            Area::Enforcement => {
                "SELECT id,lock_version+coalesce((SELECT version FROM support_appeal_workflows a WHERE a.sanction_id=s.id),1)-1,public_reason,status FROM sanctions s WHERE status<>'pending' ORDER BY created_at DESC,id DESC LIMIT 30 OFFSET $1"
            }
        };
        let rows: Vec<(Uuid, i64, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(query))
            .bind(i64::from(page.saturating_sub(1)) * 30)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| StaffError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|(id, version, title, status)| Record {
                id,
                version,
                title,
                status,
                messages: vec![],
                next_message: None,
            })
            .collect())
    }
    async fn load(
        &self,
        area: Area,
        id: Uuid,
        after: Option<Uuid>,
    ) -> std::result::Result<Record, StaffError> {
        self.available(area)?;
        if area == Area::Tickets {
            let tickets = self.tickets.as_ref().ok_or(StaffError::NotFound)?;
            let (t, messages) = tickets
                .show_page(id, true, id, after)
                .await
                .map_err(ticket_error)?;
            let next_message = (messages.len() == 50)
                .then(|| messages.last().map(|m| m.id))
                .flatten();
            return Ok(Record {
                next_message,
                id: t.id,
                version: t.lock_version,
                title: t.subject,
                status: t.status,
                messages: messages
                    .into_iter()
                    .map(|m| StaffMessage {
                        id: Some(m.id),
                        body: m.body,
                        internal: m.internal,
                    })
                    .collect(),
            });
        }
        let query = match area {
            Area::Reports => {
                "SELECT id,lock_version,target_reference AS title,status,evidence_summary AS public_message,moderator_notes AS private_message FROM player_reports WHERE id=$1"
            }
            _ => {
                "SELECT id,lock_version+coalesce((SELECT version FROM support_appeal_workflows a WHERE a.sanction_id=s.id),1)-1 AS lock_version,public_reason AS title,status,(SELECT concat_ws(E'\n',message,outcome) FROM support_appeal_workflows a WHERE a.sanction_id=s.id) AS public_message,moderator_notes AS private_message FROM sanctions s WHERE id=$1 AND status<>'pending'"
            }
        };
        let row = sqlx::query(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StaffError::Unavailable)?
            .ok_or(StaffError::NotFound)?;
        let decode = || -> std::result::Result<Record, sqlx::Error> {
            let mut messages = vec![];
            for (column, internal) in [("public_message", false), ("private_message", true)] {
                if let Some(body) = row.try_get::<Option<String>, _>(column)? {
                    messages.push(StaffMessage {
                        id: None,
                        body,
                        internal,
                    });
                }
            }
            Ok(Record {
                id: row.try_get("id")?,
                version: row.try_get("lock_version")?,
                title: row.try_get("title")?,
                status: row.try_get("status")?,
                messages,
                next_message: None,
            })
        };
        decode().map_err(|_| StaffError::Unavailable)
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        id: Uuid,
        version: i64,
        command: &Command,
    ) -> std::result::Result<(serde_json::Value, serde_json::Value), StaffError> {
        let actor = context.account;
        self.available(command.area())?;
        let notifications = self.notifications.as_ref().ok_or(StaffError::Unavailable)?;
        match command {
            Command::Reply { body, .. } | Command::TicketStatus { status: body } => {
                let tickets = self.tickets.as_ref().ok_or(StaffError::NotFound)?;
                let change = match command {
                    Command::Reply { internal, .. } => Change::Reply {
                        body,
                        internal: *internal,
                    },
                    _ => Change::Status(body),
                };
                let (ticket, before_status) = tickets
                    .change_in(tx, actor, Some(context), (id, version), change, false)
                    .await
                    .map_err(ticket_error)?;
                Ok((
                    serde_json::json!({"lock_version":version,"status":before_status}),
                    serde_json::json!({"lock_version":ticket.lock_version,"status":ticket.status}),
                ))
            }
            Command::Moderate { .. } => {
                self.reports
                    .staff_apply(tx, actor, (id, version), command, notifications)
                    .await
            }
            _ => {
                self.sanctions
                    .staff_apply(tx, context, (id, version), command, notifications)
                    .await
            }
        }
    }
}
fn ticket_error(error: Error) -> StaffError {
    match error {
        Error::Invalid => StaffError::Invalid,
        Error::NotFound => StaffError::NotFound,
        Error::Denied => StaffError::Denied,
        Error::Conflict | Error::RequestKeyConflict => StaffError::Conflict,
        _ => StaffError::Unavailable,
    }
}

/// Support can decide reports from unverified accounts and sanctions on suspended
/// accounts. Unverified addresses receive portal notices only; verified recipients
/// retain the notification service's email preferences and transactional outbox.
pub(crate) async fn notify(
    tx: &mut Transaction<'_, Postgres>,
    notifications: &Notifications,
    recipient: Uuid,
    body: impl FnOnce(&str) -> String,
) -> std::result::Result<(), StaffError> {
    let (email, locale, verified, status): (String, String, bool, String) = sqlx::query_as(
        "SELECT email,locale,email_verified_at IS NOT NULL,status FROM accounts WHERE id=$1",
    )
    .bind(recipient)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StaffError::Unavailable)?;
    if !matches!(
        status.as_str(),
        "active" | "pending_verification" | "suspended" | "closed"
    ) {
        return Err(StaffError::Denied);
    }
    let locale_code = locale.as_str();
    let locale = match locale_code {
        "pl" => oteryn_notify::Locale::Pl,
        "en" => oteryn_notify::Locale::En,
        _ => return Err(StaffError::Denied),
    };
    let template = EmailTemplate::Support {
        body: body(locale_code),
    };
    if !verified || status == "closed" {
        let portal:bool=sqlx::query_scalar("SELECT coalesce((SELECT portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='support'),true)").bind(recipient).fetch_one(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
        if portal {
            let rendered = template
                .render(locale)
                .map_err(|_| StaffError::Unavailable)?;
            sqlx::query("INSERT INTO notifications(account_id,category,subject,body) VALUES($1,'support',$2,$3)").bind(recipient).bind(rendered.subject).bind(rendered.text).execute(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
        }
    } else {
        let snapshot = Notifications {
            pool: notifications.pool.clone(),
            unsubscribe: notifications.unsubscribe.clone(),
            accounts: Arc::new(Recipient {
                id: recipient,
                account: oteryn_notify::Account { email, locale },
            }),
        };
        snapshot
            .enqueue(tx, recipient, &template)
            .await
            .map_err(|_| StaffError::Unavailable)?;
    }
    Ok(())
}
struct Recipient {
    id: Uuid,
    account: oteryn_notify::Account,
}
impl std::fmt::Debug for Recipient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SupportRecipient([redacted])")
    }
}
impl oteryn_notify::AccountState for Recipient {
    fn active_verified(
        &self,
        id: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async move { Ok((id == self.id).then(|| self.account.clone())) })
    }
}
