//! Operator role `oteryn-portal game-route publish --actor <uuid> '<json object>'`.
//!
//! Publishes one world, one channel in it and that channel's native route
//! record (gateway contract §7.3) in every environment, production included,
//! as a one-off `docker compose run` on the host; there is no HTTP surface.
//! The test-only `seed` role keeps its `OTERYN_ENV=testing` guard.
//!
//! Input keys: `worldId`, `name`, `region` (default `local`), `channelId`,
//! `host`, `port`, `tlsServerName`, `version`, `nativeLoginEnabled`
//! (default true). The route rules are the `seed route` rules: the revision is
//! computed, never taken from input; a higher `version` rotates the record;
//! the same version with other values, or a lower one, is a conflict.
//!
//! `--actor` is the operator's staff account: active and holding
//! `game.world.admin`. A run that changes anything writes one `audit_log` row
//! (`game.route.published`) in the same transaction; an identical rerun
//! changes nothing, writes nothing and prints the same JSON object.
//!
//! | exit | meaning |
//! |---|---|
//! | 0 | published or already published |
//! | 2 | bad input or a conflicting stored record |
//! | 4 | the actor is not an active account holding `game.world.admin` |
//! | 1 | configuration or database unavailable |
use crate::seed::{Args, SeedError, bad, bounded, ensure_route, ensure_world};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::Permission,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::process::ExitCode;
use uuid::Uuid;

const USAGE: &str = "usage: oteryn-portal game-route publish --actor <uuid> '<json object>'";
const ACTION: &str = "game.route.published";

#[derive(Debug, PartialEq, Eq)]
pub enum PublishError {
    Seed(SeedError),
    Denied,
}
impl From<SeedError> for PublishError {
    fn from(error: SeedError) -> Self {
        Self::Seed(error)
    }
}
impl From<sqlx::Error> for PublishError {
    fn from(error: sqlx::Error) -> Self {
        Self::Seed(error.into())
    }
}
impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Seed(error) => write!(f, "{error}"),
            Self::Denied => f.write_str("denied"),
        }
    }
}
impl std::error::Error for PublishError {}
impl PublishError {
    fn body(&self) -> Value {
        match self {
            Self::Seed(error) => {
                let (kind, reason) = error.parts();
                json!({ "error": kind, "reason": reason })
            }
            Self::Denied => json!({
                "error": "denied",
                "reason": "the actor is not an active account holding game.world.admin",
            }),
        }
    }
    fn exit(&self) -> ExitCode {
        match self {
            Self::Seed(error) => error.exit(),
            Self::Denied => ExitCode::from(4),
        }
    }
}

/// `publish --actor <uuid> <json>` exactly.
pub fn parse_args(args: &[String]) -> Result<(Uuid, Value), PublishError> {
    let [action, flag, actor, input] = args else {
        return Err(bad(USAGE).into());
    };
    if action != "publish" || flag != "--actor" {
        return Err(bad(USAGE).into());
    }
    let actor = Uuid::parse_str(actor)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(|| bad("--actor must be an account UUID"))?;
    let input = serde_json::from_str(input).map_err(|_| bad("arguments are not JSON"))?;
    Ok((actor, input))
}

/// Publish in one transaction; the actor's roles are locked for its duration.
pub async fn execute(pool: &PgPool, actor: Uuid, input: &Value) -> Result<Value, PublishError> {
    let args = Args::new(input)?;
    args.only(&[
        "worldId",
        "name",
        "region",
        "channelId",
        "host",
        "port",
        "tlsServerName",
        "version",
        "nativeLoginEnabled",
    ])?;
    let world = args
        .uuid("worldId", true)?
        .ok_or_else(|| bad("worldId is required"))?;
    let name = args.required("name")?;
    bounded("name", name, 96)?;
    let region = args.text("region")?.unwrap_or("local");
    bounded("region", region, 96)?;
    let mut tx = pool.begin().await?;
    oteryn_admin::rbac::lock_accounts_and_mutation(&mut tx, &[actor]).await?;
    let permitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a JOIN account_roles ar ON ar.account_id=a.id JOIN role_permissions rp ON rp.role_id=ar.role_id WHERE a.id=$1 AND a.status='active' AND rp.permission=$2)")
        .bind(actor)
        .bind(Permission::GameWorldAdmin.as_str())
        .fetch_one(&mut *tx)
        .await?;
    if !permitted {
        return Err(PublishError::Denied);
    }
    let world_changed = ensure_world(&mut tx, world, name, region).await?;
    let (before, after) = ensure_route(&mut tx, &args).await?;
    if world_changed || before != after {
        let target = format!(
            "game_channels:{}/{}",
            after["worldId"].as_str().unwrap_or_default(),
            after["channelId"].as_str().unwrap_or_default()
        );
        let mut recorded = after.clone();
        if let Some(map) = recorded.as_object_mut() {
            map.insert("worldName".into(), json!(name));
            map.insert("region".into(), json!(region));
        }
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(actor),
                    request_id: Uuid::now_v7(),
                },
                action: ACTION,
                target: &target,
                before,
                after: recorded,
            },
        )
        .await
        .map_err(|_| SeedError::Unavailable("audit"))?;
    }
    tx.commit().await?;
    let mut out = after;
    if let Some(map) = out.as_object_mut() {
        map.insert("entity".into(), json!("game-route"));
        map.insert("worldName".into(), json!(name));
        map.insert("region".into(), json!(region));
    }
    Ok(out)
}

async fn run(args: &[String]) -> Result<Value, PublishError> {
    let (actor, input) = parse_args(args)?;
    let config = oteryn_core::Config::from_env()
        .map_err(|_| SeedError::Unavailable("OTERYN_DATABASE_URL is not configured"))?;
    let pool = oteryn_web::session::configured_pool(config.database_url())
        .map_err(|_| SeedError::Unavailable("database"))?;
    let result = execute(&pool, actor, &input).await;
    pool.close().await;
    result
}

/// Entry point; prints exactly one JSON object on stdout.
pub async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    match run(&args).await {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", error.body());
            error.exit()
        }
    }
}

#[cfg(test)]
#[path = "game_route_cli_tests.rs"]
mod tests;
