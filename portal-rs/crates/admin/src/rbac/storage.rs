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

impl Change<'_> {
    pub(crate) async fn revoke_browser_credentials(
        &self,
        connection: &mut sqlx::PgConnection,
        context: CheckContext,
    ) -> Result<(), Error> {
        let (account, role) = match self {
            Self::CreateRole { .. } => return Ok(()),
            Self::SetPermissions { role, .. } => (None, Some(*role)),
            Self::Grant { account, .. } | Self::Revoke { account, .. } => (Some(*account), None),
        };
        let expiry = super::SessionExpiry::capture(connection, context).await?;
        sqlx::query("WITH affected AS (SELECT $1::uuid AS id WHERE $1 IS NOT NULL UNION ALL SELECT account_id FROM account_roles WHERE role_id=$2),
            revoked_sessions AS (DELETE FROM web_sessions WHERE account_id IN (SELECT id FROM affected))
            DELETE FROM account_mfa_tokens WHERE account_id IN (SELECT id FROM affected)")
            .bind(account).bind(role).execute(&mut *connection).await?;
        expiry.check(connection).await
    }
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
        self.change_impl(context, request_id, change, false, "role_maintenance")
            .await
    }

    /// Browser role edits cannot bypass privileged-grant approval or alter the
    /// roles responsible for administrator survival.
    pub async fn change_from_panel(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
    ) -> Result<Option<Uuid>, Error> {
        self.change_from_panel_with_reason(
            context,
            request_id,
            change,
            crate::staff_actions::ROLE_REASON,
        )
        .await
    }

    pub(crate) async fn change_from_panel_with_reason(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
        reason: &'static str,
    ) -> Result<Option<Uuid>, Error> {
        self.change_impl(context, request_id, change, true, reason)
            .await
    }

    /// Grant an ordinary role under the staff-action transaction's existing locks.
    pub(crate) async fn grant_from_staff_action(
        &self,
        connection: &mut sqlx::PgConnection,
        context: CheckContext,
        account: Uuid,
        role: Uuid,
    ) -> Result<bool, Error> {
        self.check_in(connection, context, Permission::ManageRoles)
            .await?;
        ensure_panel_role_grant(connection, role).await?;
        if !grant_membership(connection, &self.accounts, context.account, account, role).await? {
            return Ok(false);
        }
        Change::Grant { account, role }
            .revoke_browser_credentials(connection, context)
            .await?;
        Ok(true)
    }

    async fn change_impl(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
        panel: bool,
        reason: &'static str,
    ) -> Result<Option<Uuid>, Error> {
        if request_id.is_nil() {
            return Err(Error::InvalidInput);
        }
        self.validate_account(context).await?;
        let mut tx = self.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        let target = match &change {
            Change::Grant { account, .. } | Change::Revoke { account, .. } => *account,
            _ => context.account,
        };
        super::authorize_mutation(
            &mut tx,
            &[context.account, target],
            self,
            context,
            &[Permission::ManageRoles],
        )
        .await?;
        self.check_in(&mut tx, context, Permission::ManageRoles)
            .await?;
        if panel {
            self.check_in(&mut tx, context, Permission::Access).await?;
            match &change {
                Change::CreateRole { permissions, .. }
                | Change::SetPermissions { permissions, .. }
                    if permissions.contains(&Permission::ManageRoles) =>
                {
                    return Err(Error::Denied);
                }
                _ => {}
            }
            let protected_role = match &change {
                Change::SetPermissions { role, .. } | Change::Grant { role, .. } => Some(*role),
                _ => None,
            };
            if let Some(role) = protected_role {
                ensure_panel_role_grant(&mut tx, role).await?;
            }
        }
        let mut require_recent = false;
        let (action, target, before, after, result) = match change {
            Change::CreateRole { key, permissions } => {
                validate_key(key)?;
                validate_permissions(permissions)?;
                for permission in permissions {
                    Self::check_permission(&mut tx, context.account, *permission).await?;
                }
                restrict_analytics(key, false, permissions)?;
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
                let (key, built_in): (String, bool) =
                    sqlx::query_as("SELECT key,built_in FROM roles WHERE id=$1")
                        .bind(role)
                        .fetch_one(&mut *tx)
                        .await?;
                restrict_analytics(&key, built_in, permissions)?;
                let before: Vec<String> = sqlx::query_scalar(
                    "SELECT permission FROM role_permissions WHERE role_id = $1 ORDER BY permission")
                    .bind(role).fetch_all(&mut *tx).await?;
                let after = permission_ids(permissions);
                for permission in permissions {
                    if !before.iter().any(|value| value == permission.as_str()) {
                        Self::check_permission(&mut tx, context.account, *permission).await?;
                    }
                }
                // Nobody widens a role they hold themselves; narrowing stays allowed.
                if after.iter().any(|p| !before.iter().any(|b| b == p)) {
                    let (held, assigned): (bool, bool) = sqlx::query_as(
                        "SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2),
                                EXISTS(SELECT 1 FROM account_roles WHERE role_id=$2)",
                    )
                    .bind(context.account)
                    .bind(role)
                    .fetch_one(&mut *tx)
                    .await?;
                    if held {
                        return Err(Error::Denied);
                    }
                    if assigned {
                        self.check_in(&mut tx, context, Permission::GrantRoles)
                            .await?;
                        require_recent = true;
                        check_recent_password(&mut tx, context).await?;
                    }
                }
                if before == after {
                    self.check_in(&mut tx, context, Permission::ManageRoles)
                        .await?;
                    if panel {
                        self.check_in(&mut tx, context, Permission::Access).await?;
                    }
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
                let grant = matches!(change, Change::Grant { .. });
                let affected = if grant {
                    u64::from(
                        grant_membership(&mut tx, &self.accounts, context.account, account, role)
                            .await?,
                    )
                } else {
                    require_role(&mut tx, role).await?;
                    if !oteryn_db::protected_role_survives(&mut tx, account, Some(role)).await? {
                        return Err(Error::Denied);
                    }
                    sqlx::query("DELETE FROM account_roles WHERE account_id = $1 AND role_id = $2")
                        .bind(account)
                        .bind(role)
                        .execute(&mut *tx)
                        .await?
                        .rows_affected()
                };
                if affected == 0 {
                    self.check_in(&mut tx, context, Permission::ManageRoles)
                        .await?;
                    if panel {
                        self.check_in(&mut tx, context, Permission::Access).await?;
                    }
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
        audit::append_with_reason(
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
            Some(reason),
        )
        .await?;
        self.check_in(&mut tx, context, Permission::ManageRoles)
            .await?;
        if panel {
            self.check_in(&mut tx, context, Permission::Access).await?;
        }
        if require_recent {
            check_recent_password(&mut tx, context).await?;
        }
        change.revoke_browser_credentials(&mut tx, context).await?;
        tx.commit().await?;
        Ok(Some(result))
    }
}

async fn check_recent_password(
    connection: &mut sqlx::PgConnection,
    context: CheckContext,
) -> Result<(), Error> {
    crate::staff_actions::check_step_up(
        connection,
        context,
        crate::staff_actions::StepUp::RecentPassword,
    )
    .await
    .map_err(|error| match error {
        crate::staff_actions::Error::StepUp => Error::Denied,
        _ => Error::Storage(sqlx::Error::Protocol(
            "password confirmation unavailable".into(),
        )),
    })
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

async fn require_role(connection: &mut sqlx::PgConnection, role: Uuid) -> Result<(), Error> {
    if role.is_nil() {
        return Err(Error::InvalidInput);
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1)")
        .bind(role)
        .fetch_one(connection)
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

fn restrict_analytics(key: &str, built_in: bool, permissions: &[Permission]) -> Result<(), Error> {
    if (key != "platform_admin" || !built_in)
        && permissions.iter().any(|p| {
            matches!(
                p,
                Permission::AdminAnalyticsView | Permission::AdminAnalyticsExport
            )
        })
    {
        return Err(Error::Denied);
    }
    Ok(())
}

/// Restricted roles remain subject to the existing four-eyes workflow.
async fn ensure_panel_role_grant(
    connection: &mut sqlx::PgConnection,
    role: Uuid,
) -> Result<(), Error> {
    let protected: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM roles r WHERE r.id=$1 AND (r.key IN ('owner','platform_admin') OR EXISTS(SELECT 1 FROM role_permissions rp WHERE rp.role_id=r.id AND rp.permission='admin.roles.manage')))",
    )
    .bind(role)
    .fetch_one(connection)
    .await?;
    if protected {
        Err(Error::Denied)
    } else {
        Ok(())
    }
}

async fn grant_membership<A: AccountStateReader>(
    connection: &mut sqlx::PgConnection,
    accounts: &A,
    actor: Uuid,
    account: Uuid,
    role: Uuid,
) -> Result<bool, Error> {
    require_role(connection, role).await?;
    // A different manager must make the grant, and the recipient must be active.
    if account == actor
        || !matches!(
            accounts.state_in(connection, account).await,
            Ok(AccountState::Active)
        )
    {
        return Err(Error::Denied);
    }
    let permissions: Vec<String> =
        sqlx::query_scalar("SELECT permission FROM role_permissions WHERE role_id=$1")
            .bind(role)
            .fetch_all(&mut *connection)
            .await?;
    for permission in permissions {
        let permission = permission.parse().map_err(|_| Error::Denied)?;
        Authorizer::<A>::check_permission(connection, actor, permission).await?;
    }
    let result = sqlx::query(
        "INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(account)
    .bind(role)
    .execute(connection)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .and_then(|error| error.constraint())
            == Some("role_grant_assisted_email_reversible")
        {
            Error::Denied
        } else {
            error.into()
        }
    })?;
    Ok(result.rows_affected() == 1)
}
