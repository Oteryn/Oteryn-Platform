//! Operator role grants and email-bound staff invitations.
use super::{CheckContext, Error, MutationAuthorization, Permission, lock_accounts_and_mutation};
use crate::audit::{self, AuditContext, Entry};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use oteryn_identity::{accounts::normalize_email, reset::valid_email, verify::token_hash};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub const MAIL_JOB: &str = "email.staff_invitation";
const INVITATION_LIMIT: i64 = 50;

// An administrator role requiring independent approval stays on that workflow.
// For other staff roles the grantor must hold every permission in the role.
const DELEGABLE_ROLES: &str = "SELECT r.id,r.key FROM roles r
    WHERE ($2::uuid IS NULL OR r.id=$2)
      AND r.key NOT IN ('owner','platform_admin')
      AND EXISTS (SELECT FROM role_permissions p WHERE p.role_id=r.id AND p.permission='admin.access')
      AND NOT EXISTS (
          SELECT FROM role_permissions requested WHERE requested.role_id=r.id
            AND (requested.permission='admin.roles.manage' OR NOT EXISTS (
                SELECT FROM account_roles ar JOIN role_permissions held ON held.role_id=ar.role_id
                WHERE ar.account_id=$1 AND held.permission=requested.permission))
      )
    ORDER BY r.key";

#[derive(Debug, thiserror::Error)]
pub enum InviteError {
    #[error(transparent)]
    Rbac(#[from] Error),
    #[error("invitation unavailable")]
    Unavailable,
    #[error("multi-factor enrollment required")]
    Mfa,
    #[error("invitation already pending")]
    Duplicate,
    #[error("invitation limit reached")]
    Limit,
}

impl From<sqlx::Error> for InviteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Rbac(Error::from(error))
    }
}

/// The web and mail processes derive a separate signing key from the configured
/// staff MFA secret. The worker can reconstruct a link from the invitation ID,
/// so neither the invitation table nor the outbox needs a plaintext credential.
pub struct InvitationKey(SigningKey);

impl std::fmt::Debug for InvitationKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InvitationKey")
            .finish_non_exhaustive()
    }
}

impl InvitationKey {
    pub fn configured() -> Option<Self> {
        let secret = std::env::var("OTERYN_MFA_KEY").ok()?;
        Self::from_secret(&secret)
    }

    pub(super) fn from_secret(secret: &str) -> Option<Self> {
        if secret.len() != 64 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn/portal/staff-invitation/v1\0");
        hash.update(secret.to_ascii_lowercase());
        Some(Self(SigningKey::from_bytes(&hash.finalize().into())))
    }

    pub(super) fn token(&self, id: Uuid) -> String {
        URL_SAFE_NO_PAD.encode(self.0.sign(id.as_bytes()).to_bytes())
    }
}

pub(crate) fn valid_token(token: &str) -> bool {
    token.len() == 86
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn email_address(email: &str) -> Result<String, Error> {
    if !valid_email(email) {
        return Err(Error::InvalidInput);
    }
    Ok(normalize_email(email))
}

/// Uses local operator authority and the canonical RBAC writer lock. Existing
/// membership is a successful no-op, including its audit and session effects.
pub async fn operator_grant(pool: &PgPool, email: &str, role: &str) -> Result<bool, Error> {
    let email = email_address(email)?;
    super::storage::validate_key(role)?;
    let mut tx = pool.begin().await?;
    super::mutation_budget(&mut tx).await?;
    let account: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Denied)?;
    lock_accounts_and_mutation(&mut tx, &[account]).await?;
    let eligible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT FROM accounts
        WHERE id=$1 AND lower(email)=$2 AND status='active' AND email_verified_at IS NOT NULL)",
    )
    .bind(account)
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;
    if !eligible {
        return Err(Error::Denied);
    }
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key=$1")
        .bind(role)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::RoleNotFound)?;
    let granted = super::storage::grant(&mut tx, account, role).await?;
    if granted {
        audit::append_operator_cli(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: None,
                    request_id: Uuid::now_v7(),
                },
                action: "rbac.role.granted",
                target: &account.to_string(),
                before: json!({"role": role, "assigned": false}),
                after: json!({"role": role, "assigned": true}),
            },
        )
        .await?;
        revoke_credentials(&mut tx, account, role).await?;
    }
    tx.commit().await?;
    Ok(granted)
}

async fn revoke_credentials(tx: &mut PgConnection, account: Uuid, role: Uuid) -> Result<(), Error> {
    super::Change::Grant { account, role }
        .revoke_browser_credentials(tx, CheckContext::new(account))
        .await
}

pub(crate) async fn delegable_roles(
    pool: &PgPool,
    inviter: Uuid,
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(DELEGABLE_ROLES)
        .bind(inviter)
        .bind(Option::<Uuid>::None)
        .fetch_all(pool)
        .await
}

async fn can_delegate(
    tx: &mut PgConnection,
    inviter: Uuid,
    role: Uuid,
) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_as::<_, (Uuid, String)>(DELEGABLE_ROLES)
        .bind(inviter)
        .bind(role)
        .fetch_optional(tx)
        .await?
        .is_some())
}

pub(crate) struct NewInvitation<'a> {
    pub email: &'a str,
    pub role: Uuid,
    pub days: i32,
    pub locale: &'a str,
}

pub(crate) async fn create(
    pool: &PgPool,
    auth: &MutationAuthorization,
    actor: CheckContext,
    key: &InvitationKey,
    input: NewInvitation<'_>,
) -> Result<Uuid, InviteError> {
    let email = email_address(input.email)?;
    if !(1..=7).contains(&input.days) || input.role.is_nil() || !matches!(input.locale, "pl" | "en")
    {
        return Err(Error::InvalidInput.into());
    }
    let mut tx = pool.begin().await?;
    super::mutation_budget(&mut tx).await?;
    auth.check_in(&mut tx, actor, Permission::CreateStaffInvite)
        .await?;
    let own_email: String = sqlx::query_scalar("SELECT lower(email) FROM accounts WHERE id=$1")
        .bind(actor.account)
        .fetch_one(&mut *tx)
        .await?;
    if own_email == email || !can_delegate(&mut tx, actor.account, input.role).await? {
        return Err(Error::Denied.into());
    }
    let (recent, pending, duplicate): (i64, i64, bool) = sqlx::query_as("SELECT
        count(*) FILTER (WHERE created_at>clock_timestamp()-interval '24 hours'),
        count(*) FILTER (WHERE expires_at>clock_timestamp() AND accepted_at IS NULL AND revoked_at IS NULL),
        count(*) FILTER (WHERE expires_at>clock_timestamp() AND accepted_at IS NULL AND revoked_at IS NULL
            AND email=$2 AND role_id=$3)>0
        FROM staff_invitations WHERE inviter=$1 AND (created_at>clock_timestamp()-interval '24 hours'
            OR (expires_at>clock_timestamp() AND accepted_at IS NULL AND revoked_at IS NULL))")
        .bind(actor.account).bind(&email).bind(input.role).fetch_one(&mut *tx).await?;
    if duplicate {
        return Err(InviteError::Duplicate);
    }
    if recent >= INVITATION_LIMIT || pending >= INVITATION_LIMIT {
        return Err(InviteError::Limit);
    }

    let id = Uuid::now_v7();
    let (created, expires): (OffsetDateTime, OffsetDateTime) = sqlx::query_as(
        "WITH issued AS (SELECT clock_timestamp() AS at)
         INSERT INTO staff_invitations(id,inviter,email,role_id,locale,token_hash,created_at,expires_at)
         SELECT $1,$2,$3,$4,$5,$6,at,at+($7 * interval '24 hours') FROM issued
         RETURNING created_at,expires_at")
        .bind(id).bind(actor.account).bind(email).bind(input.role).bind(input.locale)
        .bind(token_hash(&key.token(id))).bind(input.days).fetch_one(&mut *tx).await?;
    oteryn_db::outbox::enqueue(
        &mut tx,
        MAIL_JOB,
        &json!({"invitation": id}).to_string(),
        &id.to_string(),
        created,
        10,
    )
    .await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor.account),
                request_id: Uuid::now_v7(),
            },
            action: "rbac.invitation.created",
            target: &id.to_string(),
            before: json!(null),
            after: json!({"role": input.role, "expires_at": expires.unix_timestamp()}),
        },
    )
    .await?;
    crate::rbac::MutationCheck::check(auth, &mut tx, actor, Permission::CreateStaffInvite).await?;
    tx.commit().await?;
    Ok(id)
}

pub(crate) type PendingInvitation = (Uuid, String, String, String);

pub(crate) async fn pending(
    pool: &PgPool,
    actor: Uuid,
) -> Result<Vec<PendingInvitation>, sqlx::Error> {
    sqlx::query_as("SELECT i.id,i.email,r.key,to_char(i.expires_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI')
        FROM staff_invitations i JOIN roles r ON r.id=i.role_id
        WHERE i.inviter=$1 AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>clock_timestamp()
        ORDER BY i.created_at DESC,i.id DESC LIMIT $2")
        .bind(actor).bind(INVITATION_LIMIT).fetch_all(pool).await
}

pub(crate) async fn revoke(
    pool: &PgPool,
    auth: &MutationAuthorization,
    actor: CheckContext,
    id: Uuid,
) -> Result<(), InviteError> {
    let mut tx = pool.begin().await?;
    super::mutation_budget(&mut tx).await?;
    auth.check_in(&mut tx, actor, Permission::RevokeStaffInvite)
        .await?;
    let role: Uuid = sqlx::query_scalar("UPDATE staff_invitations SET revoked_at=clock_timestamp()
        WHERE id=$1 AND inviter=$2 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at>clock_timestamp()
        RETURNING role_id")
        .bind(id).bind(actor.account).fetch_optional(&mut *tx).await?.ok_or(InviteError::Unavailable)?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor.account),
                request_id: Uuid::now_v7(),
            },
            action: "rbac.invitation.revoked",
            target: &id.to_string(),
            before: json!({"role": role, "status": "pending"}),
            after: json!({"role": role, "status": "revoked"}),
        },
    )
    .await?;
    crate::rbac::MutationCheck::check(auth, &mut tx, actor, Permission::RevokeStaffInvite).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn preview(
    pool: &PgPool,
    account: Uuid,
    token: &str,
) -> Result<(String, bool), InviteError> {
    if !valid_token(token) {
        return Err(InviteError::Unavailable);
    }
    sqlx::query_as("SELECT r.key, EXISTS(SELECT FROM account_mfa_totp m WHERE m.account_id=a.id AND m.confirmed)
        FROM staff_invitations i JOIN roles r ON r.id=i.role_id JOIN accounts a ON lower(a.email)=i.email
        WHERE i.token_hash=$1 AND a.id=$2 AND a.id<>i.inviter AND a.status='active' AND a.email_verified_at IS NOT NULL
          AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>clock_timestamp()")
        .bind(token_hash(token)).bind(account).fetch_optional(pool).await?.ok_or(InviteError::Unavailable)
}

pub(crate) async fn accept(
    pool: &PgPool,
    account: Uuid,
    session_hash: &[u8],
    token: &str,
) -> Result<(), InviteError> {
    if !valid_token(token) {
        return Err(InviteError::Unavailable);
    }
    let hash = token_hash(token);
    let mut tx = pool.begin().await?;
    super::mutation_budget(&mut tx).await?;
    let inviter: Uuid =
        sqlx::query_scalar("SELECT inviter FROM staff_invitations WHERE token_hash=$1")
            .bind(&hash)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(InviteError::Unavailable)?;
    if inviter == account {
        return Err(InviteError::Unavailable);
    }
    lock_accounts_and_mutation(&mut tx, &[inviter, account]).await?;

    // The session and verified primary email both belong to the same account.
    // Lock the session so sign-out and acceptance have an unambiguous order.
    let current: Option<Uuid> = sqlx::query_scalar("SELECT s.session_id FROM web_sessions s JOIN accounts a ON a.id=s.account_id
        WHERE s.account_id=$1 AND s.id_hash=$2 AND a.status='active' AND a.email_verified_at IS NOT NULL
          AND s.authorization_generation=a.native_security_generation AND s.idle_expires_at>clock_timestamp()
          AND s.absolute_expires_at>clock_timestamp() FOR UPDATE OF s")
        .bind(account).bind(session_hash).fetch_optional(&mut *tx).await?;
    if current.is_none() {
        return Err(InviteError::Unavailable);
    }
    let (id, role): (Uuid, Uuid) = sqlx::query_as("SELECT i.id,i.role_id FROM staff_invitations i
        JOIN accounts a ON a.id=$2 AND lower(a.email)=i.email
        WHERE i.token_hash=$1 AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>clock_timestamp()")
        .bind(&hash).bind(account).fetch_optional(&mut *tx).await?.ok_or(InviteError::Unavailable)?;
    let authorizer = super::Authorizer::new(
        pool.clone(),
        crate::web::PortalIdentity::configured(pool.clone()),
    );
    for permission in [Permission::Access, Permission::CreateStaffInvite] {
        match authorizer
            .check_subject_in(&mut tx, inviter, permission)
            .await
        {
            Ok(()) => {}
            Err(Error::Denied) => return Err(InviteError::Unavailable),
            Err(error) => return Err(error.into()),
        }
    }
    if !can_delegate(&mut tx, inviter, role).await? {
        return Err(InviteError::Unavailable);
    }

    // Every delegable role includes admin.access, whose browser guard requires MFA.
    let enrolled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT FROM account_mfa_totp
        WHERE account_id=$1 AND confirmed)",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await?;
    if !enrolled {
        return Err(InviteError::Mfa);
    }
    let consumed = sqlx::query(
        "UPDATE staff_invitations SET accepted_at=clock_timestamp()
        WHERE id=$1 AND expires_at>clock_timestamp()",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if consumed != 1 {
        return Err(InviteError::Unavailable);
    }
    let granted = super::storage::grant(&mut tx, account, role).await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(account),
                request_id: Uuid::now_v7(),
            },
            action: "rbac.invitation.accepted",
            target: &id.to_string(),
            before: json!({"role": role, "assigned": !granted, "status": "pending"}),
            after: json!({"role": role, "assigned": true, "status": "accepted"}),
        },
    )
    .await?;
    if granted {
        revoke_credentials(&mut tx, account, role).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub struct InvitationMail {
    pub recipient: String,
    pub subject: String,
    pub body: String,
}

impl std::fmt::Debug for InvitationMail {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InvitationMail")
            .finish_non_exhaustive()
    }
}

/// Resolve the recipient and content only while the invitation is pending.
pub async fn mail(
    pool: &PgPool,
    key: &InvitationKey,
    invitation: Uuid,
    origin: &str,
) -> Result<Option<InvitationMail>, sqlx::Error> {
    let token = key.token(invitation);
    let recipient: Option<(String, String)> = sqlx::query_as(
        "SELECT i.email,coalesce(a.locale,i.locale)
        FROM staff_invitations i LEFT JOIN accounts a ON lower(a.email)=i.email
        WHERE i.id=$1 AND i.token_hash=$2 AND i.accepted_at IS NULL AND i.revoked_at IS NULL
          AND i.expires_at>clock_timestamp()",
    )
    .bind(invitation)
    .bind(token_hash(&token))
    .fetch_optional(pool)
    .await?;
    Ok(recipient.map(|(recipient, locale)| InvitationMail {
        recipient,
        subject: crate::web::text(&locale, "invite-mail-subject"),
        body: format!(
            "{}\n\n{origin}/{locale}/staff-invitation?token={token}",
            crate::web::text(&locale, "invite-mail-body")
        ),
    }))
}
