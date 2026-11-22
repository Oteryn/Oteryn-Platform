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
use time::{OffsetDateTime, UtcOffset};
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
        expires_at: OffsetDateTime,
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
    let address: Option<(String, String, OffsetDateTime, bool)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT i.email,i.locale,i.expires_at,{awaiting} FROM staff_invites i WHERE i.id=$1 AND i.token_hash=$2
         AND i.expires_at>clock_timestamp() AND i.accepted_at IS NULL AND i.revoked_at IS NULL",
        awaiting = awaiting_approval_sql(),
    )))
    .bind(job.invite)
    .bind(token_hash(&token))
    .fetch_optional(pool)
    .await
    .map_err(|_| "staff invite delivery unavailable")?;
    Ok(match address {
        Some((_, _, _, true)) => DeliveryDecision::AwaitingApproval,
        Some((email, locale, expires_at, false)) => DeliveryDecision::Send {
            email,
            locale,
            token,
            expires_at,
        },
        None => DeliveryDecision::Closed,
    })
}

pub fn mail_text(locale: &str, expires_at: OffsetDateTime) -> (String, String) {
    let expiry = expires_at.to_offset(UtcOffset::UTC);
    let date = if locale == "pl" {
        format!(
            "{:02}.{:02}.{:04}",
            expiry.day(),
            u8::from(expiry.month()),
            expiry.year()
        )
    } else {
        expiry.date().to_string()
    };
    let mut args = fluent_bundle::FluentArgs::new();
    args.set(
        "expiry",
        format!(
            "{date} {:02}:{:02}:{:02} UTC",
            expiry.hour(),
            expiry.minute(),
            expiry.second()
        ),
    );
    (
        crate::web::text(locale, "invite-mail-subject"),
        crate::web::text_with_args(locale, "invite-mail-body", Some(&args)),
    )
}

const MANAGE: [Permission; 2] = [Permission::Access, Permission::ManageRoles];

// Membership implies possession of the role's permissions. Require membership
// in each selected role, including an explicit built-in platform_admin grant.
const DELEGABLE_ROLE: &str = "r.key<>'owner' AND (r.key<>'platform_admin' OR r.built_in)";

fn delegable_roles_sql() -> sqlx::AssertSqlSafe<String> {
    sqlx::AssertSqlSafe(format!(
        "SELECT r.id,r.key FROM account_roles ar JOIN roles r ON r.id=ar.role_id
         WHERE ar.account_id=$1 AND {DELEGABLE_ROLE} ORDER BY r.key"
    ))
}

async fn check_roles(tx: &mut PgConnection, actor: Uuid, roles: &[Uuid]) -> Result<(), Error> {
    let held: Vec<(Uuid, String)> = sqlx::query_as(delegable_roles_sql())
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
    approved_by: Option<Uuid>,
    awaiting_approval: bool,
}

const PENDING: &str = "i.accepted_at IS NULL AND i.revoked_at IS NULL
    AND i.expires_at>clock_timestamp()";

impl Pending {
    async fn by_id(conn: &mut PgConnection, id: Uuid) -> Result<Self, Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT i.id,i.email,i.invited_by,i.role_ids,i.approved_by,{awaiting} AS awaiting_approval
             FROM staff_invites i WHERE {PENDING} AND i.id=$1",
            awaiting = awaiting_approval_sql(),
        )))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(Error::Denied)
    }

    // Account locks precede the RBAC fence. A concurrent reapproval can change
    // the set of accounts we needed to lock, so refuse that in-flight request.
    async fn recheck_locked(&self, conn: &mut PgConnection) -> Result<Self, Error> {
        let current = Self::by_id(conn, self.id).await?;
        if current.approved_by != self.approved_by {
            return Err(Error::Denied);
        }
        Ok(current)
    }
}

#[derive(sqlx::FromRow)]
struct Listing {
    id: Uuid,
    email: String,
    roles: Vec<String>,
    awaiting_approval: bool,
}

// Use the same database snapshot for the invitation and its current
// approver's account, permissions and memberships.
fn awaiting_approval_sql() -> String {
    format!(
        "((i.requires_approval OR
           EXISTS (SELECT 1 FROM roles r WHERE r.id=ANY(i.role_ids) AND {protected_role}))
          AND NOT EXISTS (
            SELECT 1 FROM accounts a WHERE a.id=i.approved_by AND a.id<>i.invited_by
            AND a.status='active' AND a.email_verified_at IS NOT NULL AND lower(a.email)<>i.email
            AND 2=(SELECT count(DISTINCT rp.permission)
                   FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id
                   WHERE ar.account_id=a.id AND rp.permission IN ('{access}','{manage}'))
            AND NOT EXISTS (
              SELECT 1 FROM unnest(i.role_ids) AS requested(id)
              WHERE NOT EXISTS (
                SELECT 1 FROM account_roles ar JOIN roles r ON r.id=ar.role_id
                WHERE ar.account_id=a.id AND r.id=requested.id AND {DELEGABLE_ROLE}
              )
            )
          ))",
        protected_role = rbac::PROTECTED_ROLE,
        access = MANAGE[0].as_str(),
        manage = MANAGE[1].as_str(),
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
         FROM staff_invites i WHERE {PENDING} AND i.invited_by=$1 AND ($2::uuid IS NULL OR i.id<$2)
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
        let observed = Pending::by_id(&mut tx, id).await?;
        let mut participants = vec![observed.invited_by];
        participants.extend(observed.approved_by);
        rbac::authorize_mutation(&mut tx, &participants, &self.auth, actor, &MANAGE).await?;
        let invite = observed.recheck_locked(&mut tx).await?;
        self.check_approver(&mut tx, actor.account, &invite).await?;
        self.check_manager(&mut tx, invite.invited_by, &invite.role_ids)
            .await?;
        let updated = sqlx::query(
            "UPDATE staff_invites SET requires_approval=true,approved_by=$2 WHERE id=$1
             AND revoked_at IS NULL AND accepted_at IS NULL
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
            json!({"roles":invite.role_ids,"previous_approver":invite.approved_by}),
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
        if actor == invite.invited_by || !invite.awaiting_approval {
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
        let (id, inviter, approver): (Uuid, Uuid, Option<Uuid>) =
            sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT i.id,i.invited_by,i.approved_by FROM staff_invites i
                 WHERE {PENDING} AND i.token_hash=$1",
            )))
            .bind(&digest)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Denied)?;
        if inviter == actor.account {
            return Err(Error::Denied);
        }
        let mut accounts = vec![inviter, actor.account];
        accounts.extend(approver);
        rbac::lock_accounts_and_mutation(&mut tx, &accounts).await?;
        self.auth.session_in(&mut tx, actor).await?;
        let invite = Pending::by_id(&mut tx, id).await?;
        if invite.approved_by != approver || invite.awaiting_approval {
            return Err(Error::Denied);
        }
        self.check_manager(&mut tx, invite.invited_by, &invite.role_ids)
            .await?;
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
        // Keep only the accepting session and its grant. Previously remembered
        // devices and pending challenges must not acquire staff access.
        let proof = actor.session_proof.ok_or(Error::Denied)?;
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1 AND id_hash<>$2")
            .bind(actor.account)
            .bind(proof.session_hash.as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1 AND token_hash<>$2")
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
