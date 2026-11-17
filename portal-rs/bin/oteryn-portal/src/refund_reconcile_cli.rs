//! Operator command `oteryn-portal commerce-refunds-reconcile [--dry-run] --actor <uuid>`
//! (Issue #1922). Revives unsettled commerce refunds whose outbox job is dead
//! through `oteryn_admin::commerce::reconcile_dead_refunds`; `--dry-run` only
//! lists them. Prints one line per refund on stdout; errors are one JSON object
//! on stderr and never carry configuration values or the database URL.
use oteryn_admin::commerce::{self, DeadRefund};
use serde_json::json;
use sqlx::PgPool;
use std::process::ExitCode;
use uuid::Uuid;

const USAGE: &str = "usage: oteryn-portal commerce-refunds-reconcile [--dry-run] --actor <uuid>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub dry_run: bool,
    pub actor: Uuid,
}

/// `args` are the words after `commerce-refunds-reconcile`: `--actor <id>`
/// exactly once and `--dry-run` at most once, in any order.
pub fn parse_args(args: &[String]) -> Result<Command, &'static str> {
    let mut dry_run = false;
    let mut actor = None;
    let mut rest = args;
    loop {
        match rest {
            [] => break,
            [flag, tail @ ..] if flag == "--dry-run" && !dry_run => {
                dry_run = true;
                rest = tail;
            }
            [flag, value, tail @ ..] if flag == "--actor" && actor.is_none() => {
                let id = value.parse::<Uuid>().map_err(|_| "Denied")?;
                if id.is_nil() {
                    return Err("Denied");
                }
                actor = Some(id);
                rest = tail;
            }
            _ => return Err(USAGE),
        }
    }
    Ok(Command {
        dry_run,
        actor: actor.ok_or(USAGE)?,
    })
}

/// One line per refund: ids, the row's prior state and attempt, and the action.
pub fn line(refund: &DeadRefund, dry_run: bool) -> String {
    format!(
        "operation={} job={} state={} attempt={} action={}",
        refund.operation,
        refund.job,
        refund.state,
        refund
            .attempt
            .map_or_else(|| "-".to_owned(), |id| id.to_string()),
        if dry_run { "would_revive" } else { "revived" }
    )
}

fn error_code(error: &commerce::Error) -> &'static str {
    match error {
        commerce::Error::Denied => "Denied",
        commerce::Error::Storage(_) => "Database",
        _ => "Unavailable",
    }
}

pub async fn execute(pool: &PgPool, command: &Command) -> Result<Vec<String>, &'static str> {
    let refunds = commerce::reconcile_dead_refunds(pool, command.actor, command.dry_run)
        .await
        .map_err(|e| error_code(&e))?;
    Ok(refunds
        .iter()
        .map(|refund| line(refund, command.dry_run))
        .collect())
}

async fn run(args: &[String]) -> Result<Vec<String>, &'static str> {
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
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("{}", json!({"error": code}));
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "refund_reconcile_cli_tests.rs"]
mod tests;
