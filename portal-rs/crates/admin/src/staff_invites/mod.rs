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

/// Only live invitations whose digest matches the decrypted delivery credential
/// can be sent. A delayed or retried job for a closed invitation is a no-op.
pub async fn delivery(
    pool: &PgPool,
    key: &DeliveryKey,
    payload: &str,
) -> Result<Option<(String, String, String)>, &'static str> {
    let job: Delivery = serde_json::from_str(payload).map_err(|_| "invalid staff invite job")?;
    let token = key
        .open(&job.ciphertext)
        .ok_or("invalid staff invite ciphertext")?;
    let address: Option<(String, String)> = sqlx::query_as(
        "SELECT email,locale FROM staff_invites WHERE id=$1 AND token_hash=$2
         AND expires_at>clock_timestamp() AND accepted_at IS NULL AND revoked_at IS NULL",
    )
    .bind(job.invite)
    .bind(token_hash(&token))
    .fetch_optional(pool)
    .await
    .map_err(|_| "staff invite delivery unavailable")?;
    Ok(address.map(|(email, locale)| (email, locale, token)))
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
    ) -> Result<Uuid, Error> {
        let email = normalize_email(email);
        roles.sort_unstable();
        roles.dedup();
        if !valid_email(&email) || !(1..=32).contains(&roles.len()) {
            return Err(Error::InvalidInput);
        }
        let token = new_token()?;
        let ciphertext = key.seal(&token)?;
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;
        rbac::authorize_mutation(&mut tx, &[], &self.auth, actor, &MANAGE).await?;
        check_roles(&mut tx, actor.account, &roles).await?;
        sqlx::query(
            "INSERT INTO staff_invites(id,email,role_ids,invited_by,token_hash,locale,created_at,expires_at)
             VALUES ($1,$2,$3,$4,$5,$6,statement_timestamp(),statement_timestamp()+interval '48 hours')",
        )
        .bind(id)
        .bind(email)
        .bind(&roles)
        .bind(actor.account)
        .bind(token_hash(&token))
        .bind(locale)
        .execute(&mut *tx)
        .await?;
        oteryn_db::outbox::enqueue(
            &mut tx,
            MAIL_KIND,
            &json!({"invite":id,"ciphertext":ciphertext}).to_string(),
            &id.to_string(),
            time::OffsetDateTime::now_utc(),
            5,
        )
        .await?;
        record(
            &mut tx,
            actor.account,
            id,
            "staff.invite.created",
            json!({"roles":roles}),
        )
        .await?;
        self.auth.session_in(&mut tx, actor).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn revoke(&self, actor: CheckContext, id: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        rbac::authorize_mutation(&mut tx, &[], &self.auth, actor, &MANAGE).await?;
        let result = sqlx::query(
            "UPDATE staff_invites SET revoked_at=clock_timestamp() WHERE id=$1
             AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at>clock_timestamp()",
        )
        .bind(id)
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

    async fn accept(&self, actor: CheckContext, token: &str) -> Result<(), Error> {
        if !valid_token(token) {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        let Some((id, inviter, roles)) = sqlx::query_as::<_, (Uuid, Uuid, Vec<Uuid>)>(
            "SELECT id,invited_by,role_ids FROM staff_invites WHERE token_hash=$1
             AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at>clock_timestamp()",
        )
        .bind(token_hash(token))
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Err(Error::Denied);
        };
        if inviter == actor.account {
            return Err(Error::Denied);
        }
        rbac::lock_accounts_and_mutation(&mut tx, &[inviter, actor.account]).await?;
        self.auth.session_in(&mut tx, actor).await?;
        for permission in MANAGE {
            self.auth
                .check_subject_in(&mut tx, inviter, permission)
                .await?;
        }
        check_roles(&mut tx, inviter, &roles).await?;
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
        tx.commit().await?;
        Ok(())
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
