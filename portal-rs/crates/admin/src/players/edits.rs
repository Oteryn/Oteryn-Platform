use crate::{
    rbac::Permission,
    staff_actions::{AuditChange, AuditValue, Error, Snapshot},
};
use oteryn_identity::{
    accounts::{address_available, normalize_email},
    email_change::{DenySecurity, Service, text},
    reset::valid_email,
    verify::token_hash,
};
use sqlx::{PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub(super) struct Settings {
    pub status: bool,
    pub email_origin: Option<String>,
}

impl Settings {
    pub fn configured(pool: PgPool, lookup: &dyn Fn(&str) -> Option<String>) -> Self {
        let enabled = |key| lookup(key).as_deref() == Some("true");
        let security_edits = enabled("OTERYN_ADMIN_ACCOUNT_SECURITY_EDITS_ENABLED");
        let email = Service {
            pool,
            enabled: security_edits && enabled("OTERYN_ACCOUNT_EMAIL_CHANGE"),
            security: Arc::new(DenySecurity),
            origin: lookup("OTERYN_PUBLIC_ORIGIN").unwrap_or_default(),
        };
        Self {
            status: security_edits,
            email_origin: email.ready().then_some(email.origin),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
    Note,
    Flag,
    Status,
    Email,
}

impl Field {
    pub fn parse(action: &str) -> Option<Self> {
        match action {
            "note" => Some(Self::Note),
            "review-flag" => Some(Self::Flag),
            "account-status" => Some(Self::Status),
            "account-email" => Some(Self::Email),
            _ => None,
        }
    }

    pub fn permission(self) -> Permission {
        match self {
            Self::Note => Permission::ManageSupportTickets,
            Self::Flag => Permission::EditAccountFlags,
            Self::Status => Permission::EditAccountStatus,
            Self::Email => Permission::EditAccountEmail,
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Note => "players.note.added",
            Self::Flag => "players.flags.changed",
            Self::Status => "players.status.changed",
            Self::Email => "players.email.change_requested",
        }
    }

    pub fn validate(self, value: &str, settings: &Settings) -> Result<Edit, Error> {
        match (self, value.trim()) {
            (Self::Note, note) => {
                super::valid_text(note, 4000).map_err(|_| Error::Invalid)?;
                Ok(Edit::Note(note.to_owned()))
            }
            (Self::Flag, "yes") => Ok(Edit::Flag(true)),
            (Self::Flag, "no") => Ok(Edit::Flag(false)),
            (Self::Status, _) if !settings.status => Err(Error::Forbidden),
            (Self::Status, "active") => Ok(Edit::Status("active")),
            (Self::Status, "suspended") => Ok(Edit::Status("suspended")),
            (Self::Email, address) => {
                let origin = settings.email_origin.clone().ok_or(Error::Forbidden)?;
                if !valid_email(address) {
                    return Err(Error::Invalid);
                }
                Ok(Edit::Email {
                    address: normalize_email(address),
                    origin,
                })
            }
            _ => Err(Error::Invalid),
        }
    }
}

pub(super) enum Edit {
    Note(String),
    Flag(bool),
    Status(&'static str),
    Email { address: String, origin: String },
}

impl Edit {
    pub async fn apply(
        self,
        db: &mut PgConnection,
        actor: Uuid,
        target: Uuid,
    ) -> Result<Option<AuditChange>, Error> {
        let (status, verified, protected) =
            target_state(db, target).await?.ok_or(Error::Invalid)?;

        let (before, after) = match self {
            Self::Note(note) => {
                let id: Uuid = sqlx::query_scalar(
                    "INSERT INTO player_notes(account_id,author,body) VALUES($1,$2,$3) RETURNING id",
                )
                .bind(target)
                .bind(actor)
                .bind(note)
                .fetch_one(&mut *db)
                .await?;
                (Snapshot::new(), [("note", AuditValue::Id(id))].into())
            }
            Self::Flag(enabled) => {
                let old = needs_review(db, target).await?;
                if old == enabled {
                    return Ok(None);
                }
                sqlx::query(
                    "INSERT INTO staff_account_flags(account_id, needs_review) VALUES($1,$2)
                     ON CONFLICT (account_id) DO UPDATE SET needs_review=excluded.needs_review",
                )
                .bind(target)
                .bind(enabled)
                .execute(&mut *db)
                .await?;
                (
                    [("needs_review", AuditValue::Flag(old))].into(),
                    [("needs_review", AuditValue::Flag(enabled))].into(),
                )
            }
            Self::Status(next) => {
                if protected || actor == target || !verified {
                    return Err(Error::Forbidden);
                }
                let previous = match status.as_str() {
                    "active" => "active",
                    "suspended" => "suspended",
                    _ => return Err(Error::Conflict),
                };
                if previous == next {
                    return Ok(None);
                }
                sqlx::query("UPDATE accounts SET status=$2, updated_at=now() WHERE id=$1")
                    .bind(target)
                    .bind(next)
                    .execute(&mut *db)
                    .await?;
                sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                    .bind(target)
                    .execute(&mut *db)
                    .await?;
                (
                    [("status", AuditValue::Label(previous))].into(),
                    [("status", AuditValue::Label(next))].into(),
                )
            }
            Self::Email { address, origin } => {
                if protected || actor == target || !verified || status != "active" {
                    return Err(Error::Forbidden);
                }
                let request = queue_email_change(db, target, &address, &origin).await?;
                (
                    [("pending", AuditValue::Flag(false))].into(),
                    [
                        ("pending", AuditValue::Flag(true)),
                        ("request", AuditValue::Id(request)),
                    ]
                    .into(),
                )
            }
        };
        Ok(Some(AuditChange {
            target,
            before,
            after,
        }))
    }
}

pub(super) async fn target_state(
    db: &mut PgConnection,
    target: Uuid,
) -> Result<Option<(String, bool, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT status, email_verified_at IS NOT NULL,
            EXISTS (SELECT 1 FROM account_roles WHERE account_id=$1)
            OR EXISTS (SELECT 1 FROM account_terminations WHERE account_id=$1
                       AND cancelled_at IS NULL AND finalized_at IS NULL)
         FROM accounts WHERE id=$1",
    )
    .bind(target)
    .fetch_optional(&mut *db)
    .await
}

pub(super) async fn needs_review(db: &mut PgConnection, target: Uuid) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar("SELECT needs_review FROM staff_account_flags WHERE account_id=$1")
            .bind(target)
            .fetch_optional(db)
            .await?
            .unwrap_or(false),
    )
}

async fn queue_email_change(
    db: &mut PgConnection,
    target: Uuid,
    address: &str,
    origin: &str,
) -> Result<Uuid, Error> {
    // The staff action already holds the account row. Try the identity address
    // lock without waiting to avoid a lock inversion with the owner flow.
    let claimed: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1,0))")
            .bind(address)
            .fetch_one(&mut *db)
            .await?;
    if !claimed || !address_available(db, Some(target), address).await? {
        return Err(Error::Conflict);
    }
    let (old, locale): (String, String) = sqlx::query_as(
        "SELECT email, locale FROM accounts WHERE id=$1
         AND email_change_blocked_until <= clock_timestamp()
         AND NOT EXISTS (
            SELECT 1 FROM account_email_changes WHERE account_id=$1 AND kind='primary'
            AND cancelled_at IS NULL AND reverted_at IS NULL
            AND ((applied_at IS NULL AND expires_at > clock_timestamp())
                 OR (applied_at IS NOT NULL AND revert_until > clock_timestamp()))
         )",
    )
    .bind(target)
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::Conflict)?;
    if normalize_email(&old) == address {
        return Err(Error::Invalid);
    }

    sqlx::query(
        "UPDATE account_email_changes SET cancelled_at=now(), updated_at=now()
         WHERE account_id=$1 AND kind='primary' AND applied_at IS NULL
         AND cancelled_at IS NULL AND reverted_at IS NULL AND expires_at <= clock_timestamp()",
    )
    .bind(target)
    .execute(&mut *db)
    .await?;
    let (confirmation, revert): (String, String) = sqlx::query_as(
        "SELECT replace(gen_random_uuid()::text || gen_random_uuid()::text, '-', ''),
                replace(gen_random_uuid()::text || gen_random_uuid()::text, '-', '')",
    )
    .fetch_one(&mut *db)
    .await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO account_email_changes
            (account_id, kind, old_email, new_email, confirmation_hash, revert_hash,
             immediate, apply_after, expires_at, revert_until, initiated_by_staff)
         VALUES ($1, 'primary', $2, $3, $4, $5, false, now()+interval '7 days',
                 now()+interval '8 days', now()+interval '30 days', true)
         RETURNING id",
    )
    .bind(target)
    .bind(&old)
    .bind(address)
    .bind(token_hash(&confirmation))
    .bind(token_hash(&revert))
    .fetch_one(&mut *db)
    .await?;
    let body = format!(
        "{}\n\n{address}\n\n{origin}/{locale}/account/security/email",
        text(&locale, "assisted")?
    );
    sqlx::query(
        "INSERT INTO account_email_change_mail(change_id,recipient,subject,body)
         VALUES($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(&old)
    .bind(text(&locale, "title")?)
    .bind(body)
    .execute(&mut *db)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_edit_validation_is_typed_and_fails_closed() {
        let disabled = Settings::default();
        let enabled = Settings {
            status: true,
            email_origin: Some("https://example.test".into()),
        };
        assert!(matches!(
            Field::Status.validate("active", &disabled),
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            Field::Email.validate("new@example.test", &disabled),
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            Field::Status.validate("closed", &enabled),
            Err(Error::Invalid)
        ));
        assert!(matches!(
            Field::Email.validate("invalid", &enabled),
            Err(Error::Invalid)
        ));
        assert!(matches!(
            Field::Flag.validate("true", &enabled),
            Err(Error::Invalid)
        ));
        assert!(matches!(
            Field::Note.validate("  ", &enabled),
            Err(Error::Invalid)
        ));
        assert!(matches!(
            Field::Status.validate("suspended", &enabled),
            Ok(Edit::Status("suspended"))
        ));
        assert!(
            matches!(Field::Email.validate(" NEW@example.test ", &enabled), Ok(Edit::Email { address, .. }) if address == "new@example.test")
        );
        assert!(matches!(
            Field::Flag.validate("yes", &enabled),
            Ok(Edit::Flag(true))
        ));
        assert!(
            matches!(Field::Note.validate("  review details  ", &enabled), Ok(Edit::Note(value)) if value == "review details")
        );
    }
}
