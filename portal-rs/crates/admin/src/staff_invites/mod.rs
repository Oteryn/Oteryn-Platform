//! Staff invitation issuance and one-time redemption under the RBAC fence.
mod web;
pub use web::router;

use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, AccountStateReader, Authorizer, CheckContext, Error, Permission},
};
use oteryn_identity::{
    accounts::normalize_email,
    reset::{DeliveryKey, valid_email},
    verify::{new_token, token_hash},
};
use oteryn_web::session::{CommittedRotation, reissue_in_transaction};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub const MAIL_KIND: &str = "email.staff-invite";

#[derive(Deserialize)]
struct Delivery {
    invite: Uuid,
    ciphertext: Vec<u8>,
}

pub enum DeliveryDecision {
    Send {
        email: String,
        locale: String,
        token: String,
    },
    AwaitingApproval,
    Closed,
}

impl std::fmt::Debug for DeliveryDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Send { .. } => "Send([redacted])",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::Closed => "Closed",
        })
    }
}

/// Decide from the current role definitions and the exact delivery credential.
/// An invite that is still awaiting approval keeps its encrypted outbox job.
pub async fn delivery(
    pool: &PgPool,
    key: &DeliveryKey,
    payload: &str,
) -> Result<DeliveryDecision, &'static str> {
    let job: Delivery = serde_json::from_str(payload).map_err(|_| "invalid staff invite job")?;
    let token = key
        .open(&job.ciphertext)
        .ok_or("invalid staff invite ciphertext")?;
    let address: Option<(String, String, bool)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT i.email,i.locale,{awaiting} FROM staff_invites i WHERE i.id=$1 AND i.token_hash=$2
         AND i.expires_at>clock_timestamp() AND i.accepted_at IS NULL AND i.revoked_at IS NULL",
        awaiting = awaiting_approval_sql(),
    )))
    .bind(job.invite)
    .bind(token_hash(&token))
    .fetch_optional(pool)
    .await
    .map_err(|_| "staff invite delivery unavailable")?;
    Ok(match address {
        Some((_, _, true)) => DeliveryDecision::AwaitingApproval,
        Some((email, locale, false)) => DeliveryDecision::Send {
            email,
            locale,
            token,
        },
        None => DeliveryDecision::Closed,
    })
}

pub fn mail_text(locale: &str) -> (String, String) {
    (
        crate::web::text(locale, "invite-mail-subject"),
        crate::web::text(locale, "invite-mail-body"),
    )
}

const MANAGE: [Permission; 2] = [Permission::Access, Permission::ManageRoles];

// Membership implies possession of the role's permissions. Require membership
// in each selected role, including an explicit built-in platform_admin grant.
const DELEGABLE_ROLES: &str =
    "SELECT r.id,r.key FROM account_roles ar JOIN roles r ON r.id=ar.role_id
     WHERE ar.account_id=$1 AND r.key<>'owner' AND (r.key<>'platform_admin' OR r.built_in)
     ORDER BY r.key";

async fn check_roles(tx: &mut PgConnection, actor: Uuid, roles: &[Uuid]) -> Result<(), Error> {
    let held: Vec<(Uuid, String)> = sqlx::query_as(DELEGABLE_ROLES)
        .bind(actor)
        .fetch_all(tx)
        .await?;
    if roles
        .iter()
        .all(|id| held.iter().any(|(held_id, _)| held_id == id))
    {
        Ok(())
    } else {
        Err(Error::Denied)
    }
}

#[derive(sqlx::FromRow)]
struct Pending {
    id: Uuid,
    email: String,
    invited_by: Uuid,
    role_ids: Vec<Uuid>,
    requires_approval: bool,
    approved_by: Option<Uuid>,
}

const PENDING: &str = "SELECT id,email,invited_by,role_ids,requires_approval,approved_by
    FROM staff_invites WHERE accepted_at IS NULL AND revoked_at IS NULL
    AND expires_at>clock_timestamp()";

#[derive(sqlx::FromRow)]
struct Listing {
    id: Uuid,
    email: String,
    roles: Vec<String>,
    awaiting_approval: bool,
}

fn awaiting_approval_sql() -> String {
    format!(
        "(i.approved_by IS NULL AND (i.requires_approval OR
          EXISTS (SELECT 1 FROM roles r WHERE r.id=ANY(i.role_ids) AND {})))",
        rbac::PROTECTED_ROLE
    )
}

async fn pending_for(
    pool: &PgPool,
    issuer: Uuid,
    before: Option<Uuid>,
) -> Result<Vec<Listing>, sqlx::Error> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT i.id,i.email,ARRAY(SELECT r.key FROM roles r WHERE r.id=ANY(i.role_ids) ORDER BY r.key) AS roles,
         {awaiting} AS awaiting_approval
         FROM staff_invites i WHERE i.invited_by=$1 AND i.accepted_at IS NULL AND i.revoked_at IS NULL
         AND i.expires_at>clock_timestamp() AND ($2::uuid IS NULL OR i.id<$2)
         ORDER BY i.id DESC LIMIT 51",
        awaiting = awaiting_approval_sql(),
    ))).bind(issuer).bind(before).fetch_all(pool).await
}

struct Invites<A> {
    pool: PgPool,
    auth: Authorizer<A>,
}

impl<A: AccountStateReader> Invites<A> {
    async fn create(
        &self,
        actor: CheckContext,
        key: &DeliveryKey,
        email: &str,
        mut roles: Vec<Uuid>,
        locale: &str,
        request_key: Uuid,
    ) -> Result<Uuid, Error> {
        let email = normalize_email(email);
        roles.sort_unstable();
        roles.dedup();
        if request_key.is_nil() || !valid_email(&email) || !(1..=32).contains(&roles.len()) {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        rbac::authorize_mutation(&mut tx, &[], &self.auth, actor, &MANAGE).await?;
        if let Some((id, original_email, original_roles)) = sqlx::query_as::<
            _,
            (Uuid, String, Vec<Uuid>),
        >(
            "SELECT id,email,role_ids FROM staff_invites WHERE invited_by=$1 AND request_key=$2",
        )
        .bind(actor.account)
        .bind(request_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            if original_email != email || original_roles != roles {
                return Err(Error::InvalidInput);
            }
            tx.commit().await?;
            return Ok(id);
        }
        check_roles(&mut tx, actor.account, &roles).await?;
        let requires_approval = rbac::requires_grant_approval(&mut tx, &roles).await?;
        let token = new_token()?;
        let ciphertext = key.seal(&token)?;
        let id = Uuid::now_v7();
        let (created, expires): (time::OffsetDateTime, time::OffsetDateTime) = sqlx::query_as(
            "INSERT INTO staff_invites(id,email,role_ids,invited_by,token_hash,locale,created_at,expires_at,request_key,requires_approval)
             VALUES ($1,$2,$3,$4,$5,$6,statement_timestamp(),statement_timestamp()+interval '48 hours',$7,$8)
             RETURNING created_at,expires_at",
        )
        .bind(id).bind(email).bind(&roles).bind(actor.account).bind(token_hash(&token))
        .bind(locale).bind(request_key).bind(requires_approval).fetch_one(&mut *tx).await?;
        oteryn_db::outbox::enqueue(
            &mut tx,
            MAIL_KIND,
            &json!({"invite":id,"ciphertext":ciphertext}).to_string(),
            &id.to_string(),
            if requires_approval { expires } else { created },
            5,
        )
        .await?;
        record(
            &mut tx,
            actor.account,
            id,
            "staff.invite.created",
            json!({"roles":roles,"requires_approval":requires_approval}),
        )
        .await?;
        self.auth.session_in(&mut tx, actor).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn approve(&self, actor: CheckContext, id: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        let invite: Pending = sqlx::query_as(sqlx::AssertSqlSafe(format!("{PENDING} AND id=$1")))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Denied)?;
        rbac::authorize_mutation(&mut tx, &[invite.invited_by], &self.auth, actor, &MANAGE).await?;
        self.check_approver(&mut tx, actor.account, &invite).await?;
        self.check_manager(&mut tx, invite.invited_by, &invite.role_ids)
            .await?;
        let updated = sqlx::query(
            "UPDATE staff_invites SET requires_approval=true,approved_by=$2 WHERE id=$1
             AND approved_by IS NULL AND revoked_at IS NULL AND accepted_at IS NULL
             AND expires_at>clock_timestamp()",
        )
        .bind(id)
        .bind(actor.account)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(Error::Denied);
        }
        sqlx::query("UPDATE outbox SET run_after=clock_timestamp() WHERE kind=$1 AND idempotency_key=$2 AND state='pending'")
            .bind(MAIL_KIND).bind(id.to_string()).execute(&mut *tx).await?;
        record(
            &mut tx,
            actor.account,
            id,
            "staff.invite.approved",
            json!({"roles":invite.role_ids}),
        )
        .await?;
        self.auth.session_in(&mut tx, actor).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn check_approver(
        &self,
        conn: &mut PgConnection,
        actor: Uuid,
        invite: &Pending,
    ) -> Result<(), Error> {
        if actor == invite.invited_by
            || invite.approved_by.is_some()
            || (!invite.requires_approval
                && !rbac::requires_grant_approval(conn, &invite.role_ids).await?)
        {
            return Err(Error::Denied);
        }
        let recipient: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND lower(email)=$2)",
        )
        .bind(actor)
        .bind(&invite.email)
        .fetch_one(&mut *conn)
        .await?;
        if recipient {
            return Err(Error::Denied);
        }
        self.check_manager(conn, actor, &invite.role_ids).await
    }

    async fn check_manager(
        &self,
        conn: &mut PgConnection,
        actor: Uuid,
        roles: &[Uuid],
    ) -> Result<(), Error> {
        for permission in MANAGE {
            self.auth.check_subject_in(conn, actor, permission).await?;
        }
        check_roles(conn, actor, roles).await
    }

    async fn revoke(&self, actor: CheckContext, id: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        rbac::authorize_mutation(&mut tx, &[], &self.auth, actor, &MANAGE).await?;
        let result = sqlx::query(
            "UPDATE staff_invites SET revoked_at=clock_timestamp() WHERE id=$1 AND invited_by=$2
             AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at>clock_timestamp()",
        )
        .bind(id)
        .bind(actor.account)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(Error::Denied);
        }
        record(
            &mut tx,
            actor.account,
            id,
            "staff.invite.revoked",
            json!({}),
        )
        .await?;
        self.auth.session_in(&mut tx, actor).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn accept(&self, actor: CheckContext, token: &str) -> Result<CommittedRotation, Error> {
        if !valid_token(token) {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        let digest = token_hash(token);
        let invite: Pending =
            sqlx::query_as(sqlx::AssertSqlSafe(format!("{PENDING} AND token_hash=$1")))
                .bind(&digest)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::Denied)?;
        if invite.invited_by == actor.account {
            return Err(Error::Denied);
        }
        let mut accounts = vec![invite.invited_by, actor.account];
        accounts.extend(invite.approved_by);
        rbac::lock_accounts_and_mutation(&mut tx, &accounts).await?;
        self.auth.session_in(&mut tx, actor).await?;
        self.check_manager(&mut tx, invite.invited_by, &invite.role_ids)
            .await?;
        if invite.requires_approval
            || rbac::requires_grant_approval(&mut tx, &invite.role_ids).await?
        {
            let approver = invite
                .approved_by
                .filter(|id| *id != actor.account)
                .ok_or(Error::Denied)?;
            self.check_manager(&mut tx, approver, &invite.role_ids)
                .await?;
        }
        let id = invite.id;
        let roles = invite.role_ids;
        let result = sqlx::query(
            "UPDATE staff_invites i SET accepted_at=clock_timestamp(),accepted_by=a.id FROM accounts a
             WHERE i.id=$1 AND a.id=$2 AND lower(a.email)=i.email AND a.email_verified_at IS NOT NULL
             AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>clock_timestamp()",
        )
        .bind(id)
        .bind(actor.account)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(Error::Denied);
        }
        sqlx::query(
            "INSERT INTO account_roles(account_id,role_id)
             SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING",
        )
        .bind(actor.account)
        .bind(&roles)
        .execute(&mut *tx)
        .await?;
        // Only the session that presented a current MFA grant may keep access.
        let proof = actor.session_proof.ok_or(Error::Denied)?;
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1 AND id_hash<>$2")
            .bind(actor.account)
            .bind(proof.session_hash.as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1 AND token_hash<>$2 AND purpose='grant'")
            .bind(actor.account)
            .bind(proof.grant_hash.as_slice())
            .execute(&mut *tx)
            .await?;
        record(
            &mut tx,
            actor.account,
            id,
            "staff.invite.accepted",
            json!({"roles":roles}),
        )
        .await?;
        self.auth.session_in(&mut tx, actor).await?;
        let rotation =
            reissue_in_transaction(&mut tx, actor.account, &proof.session_hash, None).await?;
        tx.commit().await?;
        Ok(rotation)
    }
}

fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: Uuid,
    id: Uuid,
    action: &str,
    after: serde_json::Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before: json!(null),
            after,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
