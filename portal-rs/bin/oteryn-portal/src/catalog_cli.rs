//! Explicit operator switch for the immutable store catalog.
use oteryn_admin::rbac::Permission;
use oteryn_store::catalog::{
    AccountState, Action, CatalogAccess, CatalogFlags, CatalogStore, Error,
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::process::ExitCode;
use uuid::Uuid;

#[derive(Debug)]
pub struct Access;
impl CatalogAccess for Access {
    async fn account_state(
        &self,
        actor: Uuid,
        connection: &mut PgConnection,
    ) -> Result<AccountState, Error> {
        // Take the canonical lock before FOR SHARE; concurrent issuers must
        // not upgrade competing shared account locks while taking the fence.
        oteryn_admin::rbac::lock_accounts_and_mutation(connection, &[actor]).await?;
        let states: Vec<Option<String>> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(actor)
                .fetch_all(connection)
                .await?;
        Ok(match states.as_slice() {
            [Some(status)] if status == "active" => AccountState::Active,
            [] | [Some(_)] => AccountState::Denied,
            _ => AccountState::Unknown,
        })
    }
    async fn permitted(
        &self,
        actor: Uuid,
        action: Action,
        connection: &mut PgConnection,
    ) -> Result<bool, Error> {
        // Serialize permission reads with role writers for the full library
        // transaction, including waits on the active-catalog pointer/import.
        oteryn_admin::rbac::lock_accounts_and_mutation(connection, &[actor]).await?;
        let permission = match action {
            Action::Import => Permission::ImportStoreCatalog,
            Action::Activate => Permission::ActivateStoreCatalog,
        };
        // Same authoritative, uncached account_roles/role_permissions read path
        // as admin RBAC and bootstrap-intent, on the library's transaction.
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id JOIN roles r ON r.id=ar.role_id WHERE ar.account_id=$1 AND rp.permission=$2)")
            .bind(actor).bind(permission.as_str()).fetch_one(connection).await?)
    }
}

fn error_code(error: &Error) -> &'static str {
    match error {
        Error::Denied => "Denied",
        Error::InvalidDocument => "InvalidDocument",
        Error::Conflict => "Conflict",
        Error::NotImported => "NotImported",
        Error::InvalidRevision => "InvalidRevision",
        Error::Disabled => "Disabled",
        Error::Database(_) => "Database",
    }
}

#[derive(Debug)]
pub struct Command {
    action: Action,
    value: String,
    actor: Uuid,
}
fn parse_args(args: &[String]) -> Result<Command, &'static str> {
    let [action, value, flag, actor] = args else {
        return Err("Usage");
    };
    if flag != "--actor" || value.is_empty() {
        return Err("Usage");
    }
    let action = match action.as_str() {
        "import" => Action::Import,
        "activate" => Action::Activate,
        _ => return Err("Usage"),
    };
    let actor = actor.parse::<Uuid>().map_err(|_| "Denied")?;
    if actor.is_nil() {
        return Err("Denied");
    }
    Ok(Command {
        action,
        value: value.clone(),
        actor,
    })
}

pub async fn execute(pool: &PgPool, command: &Command) -> Result<String, &'static str> {
    let store = CatalogStore::new(pool.clone(), CatalogFlags { enabled: true }).with_audit_log();
    let revision = match command.action {
        Action::Import => {
            let raw = std::fs::read(&command.value).map_err(|_| "ReadFile")?;
            store
                .import(&raw, command.actor, &Access)
                .await
                .map_err(|e| error_code(&e))?
        }
        Action::Activate => {
            store
                .activate(&command.value, command.actor, &Access)
                .await
                .map_err(|e| error_code(&e))?;
            command.value.clone()
        }
    };
    Ok(revision)
}

async fn run(args: &[String]) -> Result<String, &'static str> {
    let command = parse_args(args)?;
    let config = oteryn_core::Config::from_env().map_err(|_| "Configuration")?;
    let pool = oteryn_db::lazy_pool(config.database_url()).map_err(|_| "Database")?;
    let result = execute(&pool, &command).await;
    pool.close().await;
    result
}

pub async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    match run(&args).await {
        Ok(revision) => {
            println!("{revision}");
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("{}", json!({"error": code}));
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "catalog_cli_tests.rs"]
mod tests;
