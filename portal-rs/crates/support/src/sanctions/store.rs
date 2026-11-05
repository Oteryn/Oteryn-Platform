use super::{Decision, Error, Features, Receipt, valid_id};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{self, Authorizer, CheckContext, Permission},
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Accounts(PgPool);
impl rbac::AccountStateReader for Accounts {
    async fn state(
        &self,
        account: Uuid,
    ) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        let status: Option<String> = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(&self.0)
            .await
            .map_err(|_| rbac::AccountStateUnavailable)?;
        Ok(match status.as_deref() {
            Some("active") => rbac::AccountState::Active,
            _ => rbac::AccountState::Unknown,
        })
    }
}
#[derive(Debug, Clone)]
pub struct Service {
    pool: PgPool,
    features: Features,
}
impl Service {
    pub fn new(pool: PgPool, features: Features) -> Self {
        Self { pool, features }
    }

    async fn staff(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        request: Uuid,
    ) -> Result<(), Error> {
        if !self.features.sanctions {
            return Err(Error::Disabled);
        }
        if !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        valid_id(context.account)?;
        valid_id(request)?;
        sqlx::query("SET LOCAL lock_timeout = '2s'")
            .execute(&mut **tx)
            .await?;
        // Same lock as the public RBAC mutation API: permission revocation cannot race approval.
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut **tx)
            .await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(context.account)
                .fetch_optional(&mut **tx)
                .await?;
        if status.as_deref() != Some("active") {
            return Err(Error::Denied);
        }
        let authorizer = Authorizer::new(self.pool.clone(), Accounts(self.pool.clone()));
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            authorizer.check(context, Permission::ManageSupportEnforcement),
        )
        .await
        .map_err(|_| Error::Unavailable)??;
        Ok(())
    }

    pub async fn create(
        &self,
        context: CheckContext,
        request: Uuid,
        decision: Decision,
    ) -> Result<Receipt, Error> {
        decision.validate()?;
        let mut tx = self.pool.begin().await?;
        self.staff(&mut tx, context, request).await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(decision.account)
                .fetch_optional(&mut *tx)
                .await?;
        if !matches!(status.as_deref(), Some("active" | "suspended")) {
            return Err(Error::Denied);
        }
        let pending = decision.expires_at_ms.is_none();
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO sanctions (id,account_id,category,status,public_reason,moderator_notes,effective_at,expires_at,created_by) VALUES ($1,$2,$3,$4,$5,$6,to_timestamp($7::bigint::double precision/1000),to_timestamp($8::bigint::double precision/1000),$9)")
            .bind(id).bind(decision.account).bind(decision.category.as_str())
            .bind(if pending { "pending" } else { "active" })
            .bind(decision.public_reason.trim()).bind(decision.moderator_notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
            .bind(decision.effective_at_ms).bind(decision.expires_at_ms).bind(context.account)
            .execute(&mut *tx).await?;
        record(
            &mut tx,
            context,
            request,
            id,
            "support.enforcement_created",
            Value::Null,
            json!({"account_id": decision.account.to_string(), "category": decision.category.as_str(), "pending": pending, "effective_at_ms": decision.effective_at_ms, "expires_at_ms": decision.expires_at_ms, "lock_version": 1}),
        )
        .await?;
        tx.commit().await?;
        Ok(Receipt {
            id,
            version: 1,
            pending,
        })
    }

    /// A pending permanent decision is immutable. Approval always addresses its exact version.
    pub async fn approve(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
    ) -> Result<Receipt, Error> {
        self.transition(context, request, id, version, true).await
    }
    pub async fn revoke(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
    ) -> Result<Receipt, Error> {
        self.transition(context, request, id, version, false).await
    }
    async fn transition(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
        approve: bool,
    ) -> Result<Receipt, Error> {
        valid_id(id)?;
        if version < 1 || version == i64::MAX {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        self.staff(&mut tx, context, request).await?;
        // Lock target before sanction, matching game-access reads and creation.
        let target: Uuid = sqlx::query_scalar("SELECT account_id FROM sanctions WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let target_status: String =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(target)
                .fetch_one(&mut *tx)
                .await?;
        if !matches!(target_status.as_str(), "active" | "suspended") {
            return Err(Error::Denied);
        }
        let row = sqlx::query(
            "SELECT status,created_by,lock_version FROM sanctions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let current: i64 = row.try_get("lock_version")?;
        if current != version {
            return Err(Error::Conflict);
        }
        let status: String = row.try_get("status")?;
        let creator: Uuid = row.try_get("created_by")?;
        if approve && (status != "pending" || creator == context.account)
            || !approve && !matches!(status.as_str(), "pending" | "active" | "expired")
        {
            return Err(Error::Denied);
        }
        let next = if approve { "active" } else { "revoked" };
        sqlx::query("UPDATE sanctions SET status=$2,approved_by=CASE WHEN $3 THEN $4 ELSE approved_by END,lock_version=lock_version+1,updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).bind(approve).bind(context.account).execute(&mut *tx).await?;
        record(
            &mut tx,
            context,
            request,
            id,
            if approve {
                "support.enforcement_approved"
            } else {
                "support.enforcement_revoked"
            },
            json!({"status": status, "lock_version": version}),
            json!({"status": next, "lock_version": version + 1}),
        )
        .await?;
        tx.commit().await?;
        Ok(Receipt {
            id,
            version: version + 1,
            pending: false,
        })
    }
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    context: CheckContext,
    request: Uuid,
    id: Uuid,
    action: &str,
    before: Value,
    after: Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: request,
            },
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await?;
    Ok(())
}
