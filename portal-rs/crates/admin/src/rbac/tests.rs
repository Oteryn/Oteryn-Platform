#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::guard::{Require, RequiredPermission};
use super::*;
use axum::{
    extract::FromRequestParts,
    http::{Request, StatusCode},
};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use std::{collections::BTreeSet, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct Accounts(Result<AccountState, AccountStateUnavailable>);
impl AccountStateReader for Accounts {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        self.0
    }
}

fn verified(account: Uuid) -> CheckContext {
    CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
    }
}

#[test]
fn permission_catalog_is_stable_unique_and_covered() {
    assert_eq!(Permission::ALL.len(), 62);
    let ids: BTreeSet<_> = Permission::ALL.iter().map(|p| p.as_str()).collect();
    assert_eq!(ids.len(), Permission::ALL.len());
    for permission in Permission::ALL {
        assert_eq!(
            permission.as_str().parse::<Permission>().unwrap(),
            *permission
        );
        assert!(
            BuiltInRole::ALL
                .iter()
                .any(|role| role.permissions().contains(permission))
        );
    }
    for invalid in [
        "",
        "admin.*",
        "ADMIN.ACCESS",
        "admin.roles.manage ",
        "unknown",
        "game.*",
        "game.db.*",
        "game.node.unknown",
        "GAME.OPS.READ",
        "game.ops.read ",
    ] {
        assert!(invalid.parse::<Permission>().is_err());
    }
}

#[tokio::test]
async fn account_and_mfa_fail_closed_without_database_access() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let id = Uuid::now_v7();
    for state in [
        Ok(AccountState::Unknown),
        Ok(AccountState::Ambiguous),
        Ok(AccountState::Disabled),
        Err(AccountStateUnavailable),
    ] {
        let auth = Authorizer::new(pool.clone(), Accounts(state));
        assert!(matches!(
            auth.check(verified(id), Permission::Access).await,
            Err(Error::Denied)
        ));
    }
    let auth = Authorizer::new(pool, Accounts(Ok(AccountState::Active)));
    assert!(matches!(
        auth.check(CheckContext::new(id), Permission::Access).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        auth.check(verified(Uuid::nil()), Permission::Access).await,
        Err(Error::Denied)
    ));
}

struct AuditView;
impl RequiredPermission for AuditView {
    const PERMISSION: Permission = Permission::ViewAudit;
}

#[tokio::test]
async fn guard_rejects_missing_or_unverified_identity() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let state = Arc::new(Authorizer::new(pool, Accounts(Ok(AccountState::Active))));
    let (mut parts, _) = Request::new(()).into_parts();
    type Guard = Require<AuditView, Accounts>;
    assert_eq!(
        Guard::from_request_parts(&mut parts, &state).await.err(),
        Some(StatusCode::UNAUTHORIZED)
    );
    parts.extensions.insert(CheckContext::new(Uuid::now_v7()));
    assert_eq!(
        Guard::from_request_parts(&mut parts, &state).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
}

struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t10_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let query = search_path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(query))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self { pool, root, schema })
    }

    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }

    fn auth(&self) -> Authorizer<Accounts> {
        Authorizer::new(self.pool.clone(), Accounts(Ok(AccountState::Active)))
    }

    async fn role(&self, key: &str) -> Uuid {
        sqlx::query_scalar("SELECT id FROM roles WHERE key = $1")
            .bind(key)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    // Trusted fixture bootstrap; production bootstrap/import is a separate workflow.
    async fn manager(&self) -> CheckContext {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2)")
            .bind(id)
            .bind(self.role("security_admin").await)
            .execute(&self.pool)
            .await
            .unwrap();
        verified(id)
    }

    async fn audit_count(&self, request: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE request_id = $1")
            .bind(request)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn db_deny_by_default_and_refuse_unprivileged_mutation() {
    let Some(db) = Database::open().await else {
        return;
    };
    let context = verified(Uuid::now_v7());
    let auth = db.auth();
    assert!(matches!(
        auth.check(context, Permission::Access).await,
        Err(Error::Denied)
    ));
    let request = Uuid::now_v7();
    assert!(matches!(
        auth.change(
            context,
            request,
            Change::CreateRole {
                key: "forbidden",
                permissions: &[]
            }
        )
        .await,
        Err(Error::Denied)
    ));
    assert_eq!(db.audit_count(request).await, 0);
    db.close().await;
}

#[tokio::test]
async fn db_grant_revoke_and_guard_use_current_permissions() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let account = Uuid::now_v7();
    let role = db.role("security_admin").await;
    let auth = Arc::new(db.auth());
    let request = Uuid::now_v7();
    auth.change(manager, request, Change::Grant { account, role })
        .await
        .unwrap();
    assert_eq!(db.audit_count(request).await, 1);
    auth.check(verified(account), Permission::ViewAudit)
        .await
        .unwrap();
    let (mut parts, _) = Request::new(()).into_parts();
    parts.extensions.insert(verified(account));
    type Guard = Require<AuditView, Accounts>;
    let guard = Guard::from_request_parts(&mut parts, &auth).await.unwrap();
    assert_eq!(guard.context().account, account);
    assert!(
        auth.change(manager, request, Change::Grant { account, role })
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(db.audit_count(request).await, 1);
    let revoke_request = Uuid::now_v7();
    auth.change(manager, revoke_request, Change::Revoke { account, role })
        .await
        .unwrap();
    assert_eq!(db.audit_count(revoke_request).await, 1);
    assert!(matches!(
        auth.check(verified(account), Permission::ViewAudit).await,
        Err(Error::Denied)
    ));
    assert_eq!(
        Guard::from_request_parts(&mut parts, &auth).await.err(),
        Some(StatusCode::FORBIDDEN)
    );
    db.close().await;
}

#[tokio::test]
async fn db_custom_role_permissions_and_bad_input() {
    let Some(db) = Database::open().await else {
        return;
    };
    let auth = db.auth();
    let manager = db.manager().await;
    for key in ["", "Bad", "bad key", "bad;sql", "_bad"] {
        assert!(matches!(
            auth.change(
                manager,
                Uuid::now_v7(),
                Change::CreateRole {
                    key,
                    permissions: &[]
                }
            )
            .await,
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(
        auth.change(
            manager,
            Uuid::now_v7(),
            Change::CreateRole {
                key: "duplicate",
                permissions: &[Permission::Access, Permission::Access]
            }
        )
        .await,
        Err(Error::InvalidInput)
    ));
    let request = Uuid::now_v7();
    let role = auth
        .change(
            manager,
            request,
            Change::CreateRole {
                key: "audit_reader",
                permissions: &[Permission::ViewAudit],
            },
        )
        .await
        .unwrap()
        .unwrap();
    let account = Uuid::now_v7();
    auth.change(manager, Uuid::now_v7(), Change::Grant { account, role })
        .await
        .unwrap();
    auth.check(verified(account), Permission::ViewAudit)
        .await
        .unwrap();
    assert!(matches!(
        auth.check(verified(account), Permission::ManageRoles).await,
        Err(Error::Denied)
    ));
    auth.change(
        manager,
        Uuid::now_v7(),
        Change::SetPermissions {
            role,
            permissions: &[],
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        auth.check(verified(account), Permission::ViewAudit).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        auth.change(
            manager,
            Uuid::now_v7(),
            Change::Grant {
                account,
                role: Uuid::now_v7()
            }
        )
        .await,
        Err(Error::RoleNotFound)
    ));
    assert!(matches!(
        auth.change(manager, Uuid::nil(), Change::Revoke { account, role })
            .await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        auth.change(
            manager,
            Uuid::now_v7(),
            Change::Grant {
                account: Uuid::nil(),
                role
            }
        )
        .await,
        Err(Error::InvalidInput)
    ));
    assert_eq!(db.audit_count(request).await, 1);
    db.close().await;
}

#[tokio::test]
async fn db_audit_change_commits_atomically_with_actor_and_diff() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let account = Uuid::now_v7();
    let role = db.role("support").await;
    let request = Uuid::now_v7();
    db.auth()
        .change(manager, request, Change::Grant { account, role })
        .await
        .unwrap();
    let (actor, action, target, before, after): (
        Uuid,
        String,
        String,
        serde_json::Value,
        serde_json::Value,
    ) = sqlx::query_as(
        "SELECT actor, action, target, before, after FROM audit_log WHERE request_id = $1",
    )
    .bind(request)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(actor, manager.account);
    assert_eq!(action, "rbac.role.granted");
    assert_eq!(target, account.to_string());
    assert_eq!(before, serde_json::json!({"role": role, "assigned": false}));
    assert_eq!(after, serde_json::json!({"role": role, "assigned": true}));
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM account_roles WHERE account_id = $1 AND role_id = $2)",
    )
    .bind(account)
    .bind(role)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(assigned);
    db.close().await;
}

#[tokio::test]
async fn db_audit_failure_rolls_back_role_change() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let account = Uuid::now_v7();
    let role = db.role("support").await;
    sqlx::raw_sql("CREATE FUNCTION reject_audit_insert() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END; $$; CREATE TRIGGER reject_insert BEFORE INSERT ON audit_log FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_insert();")
        .execute(&db.pool).await.unwrap();
    let request = Uuid::now_v7();
    assert!(matches!(
        db.auth()
            .change(manager, request, Change::Grant { account, role })
            .await,
        Err(Error::Storage(_))
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_roles WHERE account_id = $1")
        .bind(account)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(db.audit_count(request).await, 0);
    db.close().await;
}

#[tokio::test]
async fn db_audit_update_delete_and_truncate_are_rejected() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let request = Uuid::now_v7();
    db.auth()
        .change(
            manager,
            request,
            Change::CreateRole {
                key: "immutable_audit",
                permissions: &[],
            },
        )
        .await
        .unwrap();
    for query in [
        "UPDATE audit_log SET action = 'tampered'",
        "DELETE FROM audit_log",
        "TRUNCATE audit_log",
    ] {
        let error = sqlx::query(AssertSqlSafe(query))
            .execute(&db.pool)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("42501")
        );
    }
    assert_eq!(db.audit_count(request).await, 1);
    db.close().await;
}

#[tokio::test]
async fn db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants() {
    let Some(db) = Database::open().await else {
        return;
    };
    for role in BuiltInRole::ALL {
        let stored: BTreeSet<String> =
            sqlx::query_scalar("SELECT permission FROM role_permissions WHERE role_id = $1")
                .bind(db.role(role.key()).await)
                .fetch_all(&db.pool)
                .await
                .unwrap()
                .into_iter()
                .collect();
        let expected: BTreeSet<_> = role
            .permissions()
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect();
        assert_eq!(stored, expected);
    }
    let manager = db.manager().await;
    for state in [
        AccountState::Disabled,
        AccountState::Unknown,
        AccountState::Ambiguous,
    ] {
        let auth = Authorizer::new(db.pool.clone(), Accounts(Ok(state)));
        assert!(matches!(
            auth.check(manager, Permission::ManageRoles).await,
            Err(Error::Denied)
        ));
    }
    // The trusted context may explicitly waive MFA for non-staff workflows.
    let mut context = manager;
    context.mfa_verified = false;
    assert!(matches!(
        db.auth().check(context, Permission::ManageRoles).await,
        Err(Error::Denied)
    ));
    context.require_staff_mfa = false;
    db.auth()
        .check(context, Permission::ManageRoles)
        .await
        .unwrap();
    db.close().await;
}

#[derive(Debug)]
struct TargetState {
    target: Uuid,
    state: Result<AccountState, AccountStateUnavailable>,
}
impl AccountStateReader for TargetState {
    async fn state(&self, account: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        if account == self.target {
            self.state
        } else {
            Ok(AccountState::Active)
        }
    }
}

#[tokio::test]
async fn db_grants_deny_unknown_targets_and_database_rejects_unknown_permissions() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let target = Uuid::now_v7();
    let role = db.role("support").await;
    for state in [
        Ok(AccountState::Disabled),
        Ok(AccountState::Unknown),
        Ok(AccountState::Ambiguous),
        Err(AccountStateUnavailable),
    ] {
        let auth = Authorizer::new(db.pool.clone(), TargetState { target, state });
        let request = Uuid::now_v7();
        assert!(matches!(
            auth.change(
                manager,
                request,
                Change::Grant {
                    account: target,
                    role
                }
            )
            .await,
            Err(Error::Denied)
        ));
        assert_eq!(db.audit_count(request).await, 0);
    }
    let error =
        sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, 'admin.*')")
            .bind(role)
            .execute(&db.pool)
            .await
            .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23514")
    );
    // Disabled targets can still have their previous grants removed.
    db.auth()
        .change(
            manager,
            Uuid::now_v7(),
            Change::Grant {
                account: target,
                role,
            },
        )
        .await
        .unwrap();
    let auth = Authorizer::new(
        db.pool.clone(),
        TargetState {
            target,
            state: Ok(AccountState::Disabled),
        },
    );
    auth.change(
        manager,
        Uuid::now_v7(),
        Change::Revoke {
            account: target,
            role,
        },
    )
    .await
    .unwrap();
    db.close().await;
}

#[test]
fn game_roles_have_exact_contract_permissions() {
    let cases: &[(BuiltInRole, &str, &[&str])] = &[
        (
            BuiltInRole::GameViewer,
            "game_viewer",
            &[
                "game.ops.read",
                "game.ops.logs.read",
                "game.world.read",
                "game.node.read",
                "game.db.read",
                "game.economy.read",
                "game.sanction.read",
                "game.analytics.read",
                "game.audit.read",
            ],
        ),
        (
            BuiltInRole::GameSupport,
            "game_support",
            &[
                "game.ops.read",
                "game.ops.logs.read",
                "game.world.read",
                "game.node.read",
                "game.db.read",
                "game.economy.read",
                "game.sanction.read",
                "game.analytics.read",
                "game.audit.read",
                "game.player.read",
                "game.player.moderate",
                "game.sanction.write",
            ],
        ),
        (
            BuiltInRole::GameMasterSenior,
            "game_master_senior",
            &[
                "game.ops.read",
                "game.ops.logs.read",
                "game.world.read",
                "game.node.read",
                "game.db.read",
                "game.economy.read",
                "game.sanction.read",
                "game.analytics.read",
                "game.audit.read",
                "game.player.read",
                "game.player.moderate",
                "game.sanction.write",
                "game.player.manage",
                "game.player.value.write",
                "game.sanction.ban",
                "game.world.events",
                "game.economy.moderate",
            ],
        ),
        (
            BuiltInRole::GameOperator,
            "game_operator",
            &[
                "game.ops.read",
                "game.ops.logs.read",
                "game.world.read",
                "game.node.read",
                "game.db.read",
                "game.economy.read",
                "game.sanction.read",
                "game.analytics.read",
                "game.audit.read",
                "game.world.ops",
                "game.world.config",
                "game.node.ops",
                "game.db.backup",
                "game.ops.write",
            ],
        ),
        (
            BuiltInRole::GameAdmin,
            "game_admin",
            &[
                "game.ops.read",
                "game.ops.logs.read",
                "game.world.read",
                "game.node.read",
                "game.db.read",
                "game.economy.read",
                "game.sanction.read",
                "game.analytics.read",
                "game.audit.read",
                "game.world.ops",
                "game.world.config",
                "game.node.ops",
                "game.db.backup",
                "game.ops.write",
                "game.world.admin",
                "game.node.admin",
                "game.db.admin",
                "game.staff.admin",
            ],
        ),
        (
            BuiltInRole::GameDba,
            "game_dba",
            &[
                "game.db.read",
                "game.db.backup",
                "game.db.query",
                "game.analytics.sql",
            ],
        ),
        (
            BuiltInRole::GameAnalyst,
            "game_analyst",
            &[
                "game.analytics.read",
                "game.analytics.player",
                "game.analytics.sql",
                "game.analytics.export",
            ],
        ),
        (
            BuiltInRole::GameSecurity,
            "game_security",
            &[
                "game.analytics.read",
                "game.analytics.player",
                "game.analytics.sql",
                "game.analytics.export",
                "game.analytics.relink",
                "game.audit.read",
                "game.sanction.read",
            ],
        ),
        (
            BuiltInRole::GameRelease,
            "game_release",
            &["game.node.deploy"],
        ),
    ];
    for (role, key, expected) in cases {
        assert_eq!(role.key(), *key);
        let actual: BTreeSet<_> = role.permissions().iter().map(|p| p.as_str()).collect();
        assert_eq!(actual.len(), role.permissions().len());
        assert_eq!(actual, expected.iter().copied().collect());
    }
    for role in BuiltInRole::ALL {
        assert!(role.permissions().iter().all(|p| !p.as_str().contains('*')));
    }
    assert!(
        BuiltInRole::PlatformAdmin
            .permissions()
            .iter()
            .all(|p| !p.as_str().starts_with("game."))
    );
    assert_eq!(
        Permission::ALL
            .iter()
            .filter(|p| p.as_str().starts_with("game."))
            .count(),
        32
    );
}

#[tokio::test]
async fn game_flags_deny_without_database_access() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let auth = Authorizer::new(pool, Accounts(Ok(AccountState::Active)));
    for permission in Permission::ALL
        .iter()
        .filter(|p| p.as_str().starts_with("game."))
    {
        assert!(matches!(
            auth.check(verified(Uuid::now_v7()), *permission).await,
            Err(Error::Denied)
        ));
    }
    for flags in [
        GamePermissionFlags {
            enabled: true,
            deploy: false,
        },
        GamePermissionFlags {
            enabled: false,
            deploy: true,
        },
    ] {
        let auth = auth.clone().with_game_flags(flags);
        assert!(matches!(
            auth.check(verified(Uuid::now_v7()), Permission::GameNodeDeploy)
                .await,
            Err(Error::Denied)
        ));
    }
}

#[tokio::test]
async fn db_game_roles_authorize_only_explicit_grants_and_deploy_stays_dormant() {
    let Some(db) = Database::open().await else {
        return;
    };
    let manager = db.manager().await;
    let auth = db.auth().with_game_flags(GamePermissionFlags {
        enabled: true,
        deploy: false,
    });
    let permissions: Vec<_> = Permission::ALL
        .iter()
        .copied()
        .filter(|p| p.as_str().starts_with("game."))
        .collect();
    for role in BuiltInRole::ALL
        .iter()
        .filter(|r| r.key().starts_with("game_") || **r == BuiltInRole::PlatformAdmin)
    {
        let account = Uuid::now_v7();
        auth.change(
            manager,
            Uuid::now_v7(),
            Change::Grant {
                account,
                role: db.role(role.key()).await,
            },
        )
        .await
        .unwrap();
        for permission in &permissions {
            let result = auth.check(verified(account), *permission).await;
            if role.permissions().contains(permission) && *permission != Permission::GameNodeDeploy
            {
                result.unwrap();
            } else {
                assert!(
                    matches!(result, Err(Error::Denied)),
                    "{}: {}",
                    role.key(),
                    permission.as_str()
                );
            }
        }
        if role.key().starts_with("game_") {
            assert!(matches!(
                auth.check(verified(account), Permission::ManageRoles).await,
                Err(Error::Denied)
            ));
        }
    }
    let role = db.role("game_viewer").await;
    for invalid in ["game.*", "game.db.*", "game.ops.unknown", "game.ops.read "] {
        let error =
            sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)")
                .bind(role)
                .bind(invalid)
                .execute(&db.pool)
                .await
                .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23514")
        );
    }
    db.close().await;
}
