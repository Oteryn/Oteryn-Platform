use super::{AccountState, AccountStateReader, Authorizer, CheckContext, Error, Permission};
use crate::{
    audit::{self, AuditContext, Entry},
    staff_actions::{self, Action, AuditChange, Guard},
};
use serde_json::{Value, json};
use sqlx::PgConnection;
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
        Self::revoke_affected_browser_credentials(connection, context, account, role).await
    }

    pub(crate) async fn revoke_affected_browser_credentials(
        connection: &mut PgConnection,
        context: CheckContext,
        account: Option<Uuid>,
        role: Option<Uuid>,
    ) -> Result<(), Error> {
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
        self.change_impl(context, request_id, change, false).await
    }

    /// Browser role edits cannot bypass privileged-grant approval or alter the
    /// roles responsible for administrator survival.
    pub async fn change_from_panel(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
    ) -> Result<Option<Uuid>, Error> {
        self.change_impl(context, request_id, change, true).await
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
        grant_membership(connection, &self.accounts, context.account, account, role).await
    }

    pub(crate) async fn create_grant_role(
        &self,
        connection: &mut PgConnection,
        context: CheckContext,
        key: &str,
        permissions: &[Permission],
    ) -> Result<AuditChange, Error> {
        self.check_in(connection, context, Permission::ManageRoles)
            .await?;
        if permissions.contains(&Permission::ManageRoles)
            || !permissions.contains(&Permission::GrantRoles)
        {
            return Err(Error::Denied);
        }
        Ok(AuditChange::RoleCreated {
            role: create_role(connection, key, permissions).await?,
            permissions: permissions.to_vec(),
        })
    }

    /// Choose the required authorization from the durable permission diff while
    /// the writer fence excludes concurrent role edits and assignments.
    pub(crate) async fn edit_permissions_from_panel(
        &self,
        guard: Guard<'_>,
        action: Action<'_>,
        grant_confirmation: bool,
        role: Uuid,
        permissions: Vec<Permission>,
    ) -> Result<(), staff_actions::Error> {
        use staff_actions::Error::Forbidden;
        let actor = guard.actor;
        let mut tx = self.pool.begin().await?;
        super::authorize_mutation(
            &mut tx,
            &[],
            self,
            actor,
            &[Permission::Access, Permission::ManageRoles],
        )
        .await?;
        if permissions.contains(&Permission::ManageRoles) {
            return Err(Forbidden);
        }
        ensure_panel_role_grant(&mut tx, role).await?;
        let Some((before, protected)) =
            prepare_permission_edit(&mut tx, actor, role, &permissions).await?
        else {
            return Ok(());
        };
        let snapshot = AuditChange::RolePermissions {
            role,
            before: before
                .iter()
                .map(|permission| permission.parse())
                .collect::<Result<_, _>>()?,
            after: permissions.clone(),
        };
        if protected {
            if !grant_confirmation || action.reason.is_empty() {
                return Err(Forbidden);
            }
            let reason = staff_actions::validated_reason(&guard, &action)?;
            return staff_actions::run_locked(
                tx,
                guard,
                action,
                reason,
                (permissions, snapshot),
                move |connection, (permissions, snapshot)| {
                    Box::pin(async move {
                        write_permission_edit(connection, role, &permissions).await?;
                        Ok(Some(snapshot))
                    })
                },
            )
            .await;
        }
        write_permission_edit(&mut tx, role, &permissions).await?;
        staff_actions::record(&mut tx, actor, Uuid::now_v7(), snapshot, None).await?;
        self.check_in(&mut tx, actor, Permission::ManageRoles)
            .await?;
        self.check_in(&mut tx, actor, Permission::Access).await?;
        Change::SetPermissions {
            role,
            permissions: &permissions,
        }
        .revoke_browser_credentials(&mut tx, actor)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn change_impl(
        &self,
        context: CheckContext,
        request_id: Uuid,
        change: Change<'_>,
        panel: bool,
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
            match &change {
                Change::CreateRole { permissions, .. }
                    if permissions.contains(&Permission::GrantRoles) =>
                {
                    return Err(Error::Denied);
                }
                Change::Grant { role, .. } => {
                    let delegates_grants: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM role_permissions WHERE role_id=$1 AND permission=$2)",
                    ).bind(role).bind(Permission::GrantRoles.as_str()).fetch_one(&mut *tx).await?;
                    if delegates_grants {
                        return Err(Error::Denied);
                    }
                }
                _ => {}
            }
        }
        let (action, target, before, after, result) = match change {
            Change::CreateRole { key, permissions } => {
                let role = create_role(&mut tx, key, permissions).await?;
                (
                    "rbac.role.created",
                    role.to_string(),
                    Value::Null,
                    json!({"key": key, "permissions": permission_ids(permissions)}),
                    role,
                )
            }
            Change::SetPermissions { role, permissions } => {
                let Some((before, protected)) =
                    prepare_permission_edit(&mut tx, context, role, permissions).await?
                else {
                    self.check_in(&mut tx, context, Permission::ManageRoles)
                        .await?;
                    if panel {
                        self.check_in(&mut tx, context, Permission::Access).await?;
                    }
                    tx.commit().await?;
                    return Ok(None);
                };
                if protected && panel {
                    return Err(Error::Denied);
                }
                write_permission_edit(&mut tx, role, permissions).await?;
                let after = permission_ids(permissions);
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
        self.check_in(&mut tx, context, Permission::ManageRoles)
            .await?;
        if panel {
            self.check_in(&mut tx, context, Permission::Access).await?;
        }
        change.revoke_browser_credentials(&mut tx, context).await?;
        tx.commit().await?;
        Ok(Some(result))
    }
}

async fn create_role(
    connection: &mut PgConnection,
    key: &str,
    permissions: &[Permission],
) -> Result<Uuid, Error> {
    validate_key(key)?;
    validate_permissions(permissions)?;
    restrict_analytics(key, false, permissions)?;
    let role: Uuid = sqlx::query_scalar("INSERT INTO roles (key) VALUES ($1) RETURNING id")
        .bind(key)
        .fetch_one(&mut *connection)
        .await?;
    write_permissions(connection, role, permissions).await?;
    Ok(role)
}

/// Call with the role-writer fence held. Widening an assigned role confers
/// permissions on its holders, so it requires the same controls as a grant.
async fn prepare_permission_edit(
    connection: &mut PgConnection,
    actor: CheckContext,
    role: Uuid,
    permissions: &[Permission],
) -> Result<Option<(Vec<String>, bool)>, Error> {
    validate_permissions(permissions)?;
    require_role(connection, role).await?;
    let (key, built_in): (String, bool) =
        sqlx::query_as("SELECT key,built_in FROM roles WHERE id=$1")
            .bind(role)
            .fetch_one(&mut *connection)
            .await?;
    restrict_analytics(&key, built_in, permissions)?;
    let before: Vec<String> = sqlx::query_scalar(
        "SELECT permission FROM role_permissions WHERE role_id = $1 ORDER BY permission",
    )
    .bind(role)
    .fetch_all(&mut *connection)
    .await?;
    let after = permission_ids(permissions);
    if before == after {
        return Ok(None);
    }
    let additions = after.iter().filter(|p| !before.iter().any(|b| b == *p));
    let protected = if additions.clone().next().is_some() {
        let (assigned, held): (bool, bool) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM account_roles WHERE role_id=$1),
                    EXISTS(SELECT 1 FROM account_roles WHERE role_id=$1 AND account_id=$2)",
        )
        .bind(role)
        .bind(actor.account)
        .fetch_one(&mut *connection)
        .await?;
        if held {
            return Err(Error::Denied);
        }
        assigned
            || additions
                .into_iter()
                .any(|p| *p == Permission::GrantRoles.as_str())
    } else {
        false
    };
    Ok(Some((before, protected)))
}

async fn write_permission_edit(
    connection: &mut PgConnection,
    role: Uuid,
    permissions: &[Permission],
) -> Result<(), Error> {
    sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
        .bind(role)
        .execute(&mut *connection)
        .await?;
    write_permissions(connection, role, permissions).await?;
    sqlx::query("UPDATE roles SET updated_at = now() WHERE id = $1")
        .bind(role)
        .execute(connection)
        .await?;
    Ok(())
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
    connection: &mut PgConnection,
    role: Uuid,
    permissions: &[Permission],
) -> Result<(), Error> {
    for permission in permissions {
        sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)")
            .bind(role)
            .bind(permission.as_str())
            .execute(&mut *connection)
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

async fn grant_membership(
    connection: &mut sqlx::PgConnection,
    accounts: &impl AccountStateReader,
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
    let result = sqlx::query(
        "INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(account)
    .bind(role)
    .execute(connection)
    .await?;
    Ok(result.rows_affected() == 1)
}
