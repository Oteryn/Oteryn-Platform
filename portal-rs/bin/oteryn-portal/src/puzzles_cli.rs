use oteryn_admin::{audit, rbac};
use oteryn_minigames::puzzles::{Error, Flags, Publication, Puzzles};
use oteryn_world::{WorldOptions, WorldStore};
use serde_json::{Value, json};
use std::{path::Path, process::ExitCode};
use uuid::Uuid;

const USAGE: &str = "Usage: oteryn-portal admin-cli puzzles publish <file> --actor <uuid>";

fn publication_error(error: Error) -> &'static str {
    match error {
        Error::Invalid => "InvalidDocument",
        Error::Conflict => "Conflict",
        Error::Disabled => "Disabled",
        Error::Unavailable | Error::NotFound => "Unavailable",
        Error::Forbidden => "Denied",
        Error::Database(_) => "Database",
    }
}

async fn run(args: &[String]) -> Result<Value, &'static str> {
    let [group, command, file, flag, actor] = args else {
        return Err(USAGE);
    };
    if group != "puzzles" || command != "publish" || flag != "--actor" {
        return Err(USAGE);
    }
    let actor: Uuid = actor.parse().map_err(|_| "Denied")?;
    if actor.is_nil() {
        return Err("Denied");
    }
    if std::env::var("OTERYN_DAILY_PUZZLES_ENABLED").as_deref() != Ok("true")
        || std::env::var("OTERYN_WORLD_ENABLED").as_deref() != Ok("true")
    {
        return Err("Disabled");
    }
    let config = oteryn_core::Config::from_env().map_err(|_| "Configuration")?;
    let pool = oteryn_db::lazy_pool(config.database_url()).map_err(|_| "Database")?;
    let mut tx = pool.begin().await.map_err(|_| "Database")?;
    // World catalogue writers lock the active pointer before the RBAC fence.
    rbac::lock_accounts_resources_and_mutation(&mut tx, &[actor], |connection| {
        Box::pin(async move {
            sqlx::query("SELECT version_id FROM world_active_version WHERE singleton FOR SHARE")
                .execute(connection)
                .await?;
            Ok(())
        })
    })
    .await
    .map_err(|_| "Database")?;
    let permitted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM accounts a JOIN account_roles ar ON ar.account_id=a.id
           JOIN role_permissions rp ON rp.role_id=ar.role_id
           WHERE a.id=$1 AND a.status='active' AND rp.permission=$2)",
    )
    .bind(actor)
    .bind(rbac::Permission::PublishDailyPuzzles.as_str())
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "Database")?;
    if !permitted {
        return Err("Denied");
    }

    let file = Path::new(file);
    let publication = Publication::read(file).map_err(publication_error)?;
    let root = file.parent().ok_or("InvalidDocument")?;
    let world = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
    let service = Puzzles::new(
        pool,
        world,
        Flags {
            daily_puzzles: true,
        },
    );
    let puzzle_ids = service
        .activate_publication(&mut tx, &publication, root)
        .await
        .map_err(publication_error)?;
    let id = publication.publication_id;
    let replayed = puzzle_ids.is_none();
    if let Some(ids) = puzzle_ids {
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(actor),
                    request_id: id,
                },
                action: "puzzles.publish",
                target: &id.to_string(),
                before: Value::Null,
                after: json!({"publicationId": id, "puzzleIds": ids}),
            },
        )
        .await
        .map_err(|_| "Database")?;
    }
    tx.commit().await.map_err(|_| "Database")?;
    Ok(json!({"publicationId": id, "replayed": replayed}))
}

pub async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(2).collect();
    match run(&args).await {
        Ok(result) => {
            println!("{result}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", json!({"error": error}));
            ExitCode::from(2)
        }
    }
}
