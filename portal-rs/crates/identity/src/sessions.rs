//! Owner-scoped web session management. Bearer hashes never leave this boundary.
use crate::{AccountId, accounts::Accounts};
use oteryn_notify::{
    Account, AccountState, EmailTemplate, Locale, Notifications, NotifyFuture, Unsubscribe,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction, types::time::OffsetDateTime};
use std::sync::Arc;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn denied() -> Box<dyn std::error::Error + Send + Sync> {
    "session operation refused".into()
}

#[derive(Debug, Clone)]
pub struct NotificationAccounts(pub PgPool);
impl AccountState for NotificationAccounts {
    fn recipient(
        &self,
        id: Uuid,
        category: oteryn_notify::Category,
    ) -> NotifyFuture<'_, Option<Account>> {
        if category != oteryn_notify::Category::Support {
            return self.active_verified(id);
        }
        Box::pin(async move {
            let account = Accounts {
                pool: self.0.clone(),
            }
            .find_by_id(id)
            .await?;
            Ok(account.and_then(|a| {
                if !matches!(a.status.as_str(), "active" | "suspended")
                    || a.email_verified_at.is_none()
                {
                    return None;
                }
                let locale = match a.locale.as_str() {
                    "pl" => Locale::Pl,
                    "en" => Locale::En,
                    _ => return None,
                };
                Some(Account {
                    email: a.email,
                    locale,
                })
            }))
        })
    }

    fn active_verified_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> NotifyFuture<'a, Option<Account>> {
        Box::pin(async move {
            Ok(Accounts::find_by_id_in(tx, id).await?.and_then(|a| {
                if a.status != "active" || a.email_verified_at.is_none() {
                    return None;
                }
                let locale = match a.locale.as_str() {
                    "pl" => Locale::Pl,
                    "en" => Locale::En,
                    _ => return None,
                };
                Some(Account {
                    email: a.email,
                    locale,
                })
            }))
        })
    }
    fn active_verified(&self, id: Uuid) -> NotifyFuture<'_, Option<Account>> {
        Box::pin(async move {
            let account = Accounts {
                pool: self.0.clone(),
            }
            .find_by_id(id)
            .await?;
            Ok(account.and_then(|a| {
                if a.status != "active" || a.email_verified_at.is_none() {
                    return None;
                }
                let locale = match a.locale.as_str() {
                    "pl" => Locale::Pl,
                    "en" => Locale::En,
                    _ => return None,
                };
                Some(Account {
                    email: a.email,
                    locale,
                })
            }))
        })
    }
}
/// This adapter only sends mandatory security messages; it never issues unsubscribe tokens.
pub fn security_notifications(pool: PgPool) -> Result<Notifications> {
    let mut key = vec![0; 32];
    getrandom::fill(&mut key).map_err(|_| denied())?;
    Ok(Notifications {
        accounts: Arc::new(NotificationAccounts(pool.clone())),
        pool,
        unsubscribe: Unsubscribe::new(key, "https://portal.invalid/unsubscribe".into())?,
    })
}

#[derive(Debug, Clone)]
pub struct Sessions {
    pub pool: PgPool,
}
/// Created by the HTTP adapter using verified browser identity and a hashed cookie.
pub struct Actor {
    pub account: AccountId,
    pub hash: Vec<u8>,
}
impl std::fmt::Debug for Actor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Actor([redacted])")
    }
}
#[derive(Debug, sqlx::FromRow)]
pub struct ActiveSession {
    pub session_id: Uuid,
    pub user_agent_summary: String,
    pub ip: Option<String>,
    pub last_seen_at: OffsetDateTime,
    pub current: bool,
}
#[derive(Debug, sqlx::FromRow)]
pub struct SecurityEvent {
    pub kind: String,
    pub device: String,
    pub ip: Option<String>,
    pub created_at: OffsetDateTime,
}
#[derive(Debug, Clone, Copy)]
pub enum Revocation {
    One(Uuid),
    Others,
    All,
}
#[derive(PartialEq, Eq, Debug)]
pub enum Outcome {
    Inactive,
    Revoked,
    SignedOut,
}

impl Sessions {
    /// Serialize login observation and revocation on the account, then recheck the
    /// current session. A cached BrowserSession extension alone is insufficient.
    async fn authorize(&self, tx: &mut Transaction<'_, Postgres>, actor: &Actor) -> Result<Uuid> {
        let active: Option<Uuid> = sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL AND locale IN ('pl','en') FOR UPDATE")
            .bind(actor.account).fetch_optional(&mut **tx).await?;
        if active.is_none() {
            return Err(denied());
        }
        sqlx::query_scalar("SELECT session_id FROM web_sessions WHERE account_id=$1 AND id_hash=$2 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp() FOR UPDATE")
            .bind(actor.account).bind(&actor.hash).fetch_optional(&mut **tx).await?.ok_or_else(denied)
    }
    pub async fn list(&self, actor: &Actor) -> Result<(Vec<ActiveSession>, Vec<SecurityEvent>)> {
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        let sessions = sqlx::query_as("SELECT session_id,user_agent_summary,host(ip) AS ip,last_seen_at,(id_hash=$2) AS current FROM web_sessions WHERE account_id=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp() ORDER BY last_seen_at DESC,session_id DESC LIMIT 50")
            .bind(actor.account).bind(&actor.hash).fetch_all(&mut *tx).await?;
        let history = sqlx::query_as("SELECT kind,device,host(ip) AS ip,created_at FROM security_events WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 50")
            .bind(actor.account).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok((sessions, history))
    }
    pub async fn revoke(&self, actor: &Actor, selection: Revocation) -> Result<Outcome> {
        let mut tx = self.pool.begin().await?;
        let current = self.authorize(&mut tx, actor).await?;
        let (target, others, all, kind) = match selection {
            Revocation::One(id) => (Some(id), false, false, "session_revoked"),
            Revocation::Others => (None, true, false, "other_sessions_revoked"),
            Revocation::All => (None, false, true, "all_sessions_revoked"),
        };
        let (removed, signed_out): (i64, bool) = sqlx::query_as("WITH removed AS (DELETE FROM web_sessions WHERE account_id=$1 AND (session_id=$2 OR ($3 AND session_id<>$4) OR $5) RETURNING session_id=$4 AS current) SELECT count(*),coalesce(bool_or(current),false) FROM removed")
            .bind(actor.account).bind(target).bind(others).bind(current).bind(all).fetch_one(&mut *tx).await?;
        if target.is_some() && removed == 0 {
            return Ok(Outcome::Inactive);
        }
        sqlx::query("INSERT INTO security_events(account_id,session_id,kind) VALUES($1,$2,$3)")
            .bind(actor.account)
            .bind(target)
            .bind(kind)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(if signed_out {
            Outcome::SignedOut
        } else {
            Outcome::Revoked
        })
    }
    /// Observe only an actual successful login response, before releasing its cookie.
    /// Account locking makes first-device detection deterministic under concurrent logins.
    pub async fn record_login(&self, actor: &Actor) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let id = self.authorize(&mut tx, actor).await?;
        let (device, ip): (String, Option<String>) = sqlx::query_as(
            "SELECT user_agent_summary,host(ip) FROM web_sessions WHERE session_id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let inserted = sqlx::query("INSERT INTO security_events(account_id,session_id,kind,device,ip) VALUES($1,$2,'login_succeeded',$3,$4::text::inet) ON CONFLICT DO NOTHING")
            .bind(actor.account).bind(id).bind(&device).bind(ip).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Ok(());
        }
        // A user-agent is a notification heuristic, never an authentication factor.
        // Missing device evidence is always treated as new and is not remembered.
        let new_device = if device.trim().is_empty() {
            true
        } else {
            sqlx::query("INSERT INTO web_known_devices(account_id,fingerprint) VALUES($1,$2) ON CONFLICT(account_id,fingerprint) DO NOTHING")
                .bind(actor.account).bind(Sha256::digest(device.as_bytes()).to_vec())
                .execute(&mut *tx).await?.rows_affected() == 1
        };
        if new_device {
            let (email, locale): (String, String) =
                sqlx::query_as("SELECT email,locale FROM accounts WHERE id=$1")
                    .bind(actor.account)
                    .fetch_one(&mut *tx)
                    .await?;
            let mut notifications = security_notifications(self.pool.clone())?;
            notifications.accounts = Arc::new(LoginAccount {
                id: actor.account,
                account: Account {
                    email,
                    locale: if locale == "pl" {
                        Locale::Pl
                    } else {
                        Locale::En
                    },
                },
            });
            notifications
                .enqueue(&mut tx, actor.account, &EmailTemplate::NewDevice { device })
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;

/// Snapshot read under the account lock; notification enqueue never borrows another connection.
#[derive(Debug)]
struct LoginAccount {
    id: AccountId,
    account: Account,
}
impl AccountState for LoginAccount {
    fn active_verified(&self, id: Uuid) -> NotifyFuture<'_, Option<Account>> {
        Box::pin(async move { Ok((id == self.id).then(|| self.account.clone())) })
    }
}

/// Safe staff projection. The caller must authorize access to the target account.
/// The extra row allows callers to fail closed rather than silently truncate.
pub async fn staff_sessions(
    pool: &PgPool,
    account: AccountId,
) -> std::result::Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM accounts WHERE id=$1")
        .bind(account)
        .fetch_one(pool)
        .await?;
    sqlx::query_as("SELECT kind,expires::text FROM (SELECT 'web' AS kind,least(idle_expires_at,absolute_expires_at) AS expires FROM web_sessions WHERE account_id=$1 UNION ALL SELECT 'oauth',expires_at FROM oauth_token_families WHERE account_id=$1 AND revoked_at IS NULL UNION ALL SELECT 'device',to_timestamp(least(idle_expires_at,absolute_expires_at)::double precision) FROM device_sessions WHERE account_id=$1 AND revoked_at IS NULL UNION ALL SELECT 'game-authorization',expires_at FROM game_login_tickets WHERE account_id=$1 AND used_at IS NULL) AS sessions WHERE expires>clock_timestamp() ORDER BY expires,kind LIMIT 101")
        .bind(account).fetch_all(pool).await
}

/// Complete staff revocation in the caller's authorization/audit transaction.
/// Account locking serializes credential issuance and advances the native fence.
pub async fn staff_revoke_all(
    connection: &mut sqlx::PgConnection,
    account: AccountId,
) -> std::result::Result<(), sqlx::Error> {
    let found = sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1,updated_at=clock_timestamp() WHERE id=$1")
        .bind(account).execute(&mut *connection).await?.rows_affected();
    if found != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    crate::recovery_key::revoke_authorizations(connection, account).await?;
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .execute(&mut *connection)
        .await?;
    sqlx::query("UPDATE game_login_tickets SET used_at=coalesce(used_at,clock_timestamp()),updated_at=clock_timestamp() WHERE account_id=$1").bind(account).execute(&mut *connection).await?;
    sqlx::query("DELETE FROM passkey_ceremonies WHERE account_id=$1")
        .bind(account)
        .execute(&mut *connection)
        .await?;
    sqlx::query("INSERT INTO security_events(account_id,kind) VALUES($1,'all_sessions_revoked')")
        .bind(account)
        .execute(connection)
        .await?;
    Ok(())
}
