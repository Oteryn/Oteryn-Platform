use super::{AccountState, AccountStateReader, Authorizer, CheckContext, Error, Permission};
use crate::audit::{self, AuditContext, Entry};
use serde_json::{Value, json};
use uuid::Uuid;

/// Changes require `admin.roles.manage`. Business workflows must additionally enforce
/// four-eyes approval for administrator grants before invoking this storage API.
#[derive(Debug)]
pub enum Change<'a> {
    CreateRole {
        key: &'a str,
        permissions: &'a [Permission],
    },
    SetPermissions {
        role: Uuid,
        permissions: &'a [Permission],
    },
    Grant {
        account: Uuid,
        role: Uuid,
    },
    Revoke {
        account: Uuid,
        role: Uuid,
    },
}

impl<A: AccountStateReader> Authorizer<A> {
    /// Commits the role change and its audit together. A no-op writes no audit entry.
    /// All role writers serialize, including the authorization check, preventing a
    /// revoked manager from racing a later mutation through this API.
    pub async fn change(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
    ) -> Result<Option<Uuid>, Error> {
        if request_id.is_nil() {
            return Err(Error::InvalidInput);
        }
        self.validate_account(context).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        self.validate_account(context).await?;
        Self::check_permission(&mut tx, context.account, Permission::ManageRoles).await?;
        let (action, target, before, after, result) = match change {
            Change::CreateRole { key, permissions } => {
                validate_key(key)?;
                validate_permissions(permissions)?;
                let role: Uuid =
                    sqlx::query_scalar("INSERT INTO roles (key) VALUES ($1) RETURNING id")
                        .bind(key)
                        .fetch_one(&mut *tx)
                        .await?;
                write_permissions(&mut tx, role, permissions).await?;
                (
                    "rbac.role.created",
                    role.to_string(),
                    Value::Null,
                    json!({"key": key, "permissions": permission_ids(permissions)}),
                    role,
                )
            }
            Change::SetPermissions { role, permissions } => {
                validate_permissions(permissions)?;
                require_role(&mut tx, role).await?;
                let before: Vec<String> = sqlx::query_scalar(
                    "SELECT permission FROM role_permissions WHERE role_id = $1 ORDER BY permission")
                    .bind(role).fetch_all(&mut *tx).await?;
                let after = permission_ids(permissions);
                if before == after {
                    tx.commit().await?;
                    return Ok(None);
                }
                sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
                    .bind(role)
                    .execute(&mut *tx)
                    .await?;
                write_permissions(&mut tx, role, permissions).await?;
                sqlx::query("UPDATE roles SET updated_at = now() WHERE id = $1")
                    .bind(role)
                    .execute(&mut *tx)
                    .await?;
                (
                    "rbac.role.permissions_changed",
                    role.to_string(),
                    json!({"permissions": before}),
                    json!({"permissions": after}),
                    role,
                )
            }
            Change::Grant { account, role } | Change::Revoke { account, role } => {
                if account.is_nil() {
                    return Err(Error::InvalidInput);
                }
                require_role(&mut tx, role).await?;
                let grant = matches!(change, Change::Grant { .. });
                // Revocation must remain possible even for a disabled or deleted account.
                if grant && !matches!(self.accounts.state(account).await, Ok(AccountState::Active))
                {
                    return Err(Error::Denied);
                }
                let affected = if grant {
                    sqlx::query("INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                        .bind(account).bind(role).execute(&mut *tx).await?.rows_affected()
                } else {
                    sqlx::query("DELETE FROM account_roles WHERE account_id = $1 AND role_id = $2")
                        .bind(account)
                        .bind(role)
                        .execute(&mut *tx)
                        .await?
                        .rows_affected()
                };
                if affected == 0 {
                    tx.commit().await?;
                    return Ok(None);
                }
                (
                    if grant {
                        "rbac.role.granted"
                    } else {
                        "rbac.role.revoked"
                    },
                    account.to_string(),
                    json!({"role": role, "assigned": !grant}),
                    json!({"role": role, "assigned": grant}),
                    role,
                )
            }
        };
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id,
                },
                action,
                target: &target,
                before,
                after,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(Some(result))
    }
}

fn validate_key(key: &str) -> Result<(), Error> {
    if key.is_empty()
        || key.len() > 64
        || !key.as_bytes()[0].is_ascii_lowercase()
        || !key
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

fn validate_permissions(permissions: &[Permission]) -> Result<(), Error> {
    let ids = permission_ids(permissions);
    if ids.len() != permissions.len() {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

fn permission_ids(permissions: &[Permission]) -> Vec<&'static str> {
    let mut ids: Vec<_> = permissions.iter().map(|p| p.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

async fn require_role(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    role: Uuid,
) -> Result<(), Error> {
    if role.is_nil() {
        return Err(Error::InvalidInput);
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1)")
        .bind(role)
        .fetch_one(&mut **tx)
        .await?;
    if !exists {
        return Err(Error::RoleNotFound);
    }
    Ok(())
}

async fn write_permissions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    role: Uuid,
    permissions: &[Permission],
) -> Result<(), Error> {
    for permission in permissions {
        sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)")
            .bind(role)
            .bind(permission.as_str())
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
