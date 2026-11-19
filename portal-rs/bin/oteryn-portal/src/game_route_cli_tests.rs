#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use oteryn_gameauth::admission::route_revision;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WORLD: &str = "019a0000-0000-7000-8000-00000000e001";
const CHANNEL: &str = "019a0000-0000-7000-8000-00000000c001";

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("game_route_{}", Uuid::now_v7().simple());
        // Schema identifiers contain only a fixed prefix and generated UUID hex.
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = Uuid::now_v7();
        account(&pool, actor, "active").await?;
        let role = Uuid::now_v7();
        sqlx::query("INSERT INTO roles (id,key,built_in) VALUES ($1,'world_operator',false)")
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO role_permissions (role_id,permission) VALUES ($1,$2)")
            .bind(role)
            .bind(Permission::GameWorldAdmin.as_str())
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO account_roles (account_id,role_id) VALUES ($1,$2)")
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            actor,
        }))
    }
    async fn count(&self, sql: &'static str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await
    }
    async fn audits(&self) -> Result<i64, sqlx::Error> {
        self.count("SELECT count(*) FROM audit_log WHERE action='game.route.published'")
            .await
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}

async fn account(pool: &PgPool, id: Uuid, status: &str) -> TestResult {
    sqlx::query("INSERT INTO accounts (id,email,password_hash,status,locale) VALUES ($1,$2,'test-only-hash',$3,'en')")
        .bind(id).bind(format!("{id}@example.invalid")).bind(status).execute(pool).await?;
    Ok(())
}

fn input() -> Value {
    json!({"worldId": WORLD, "name": "Oteryn", "region": "eu", "channelId": CHANNEL,
           "host": "game.oteryn.com", "port": 7172,
           "tlsServerName": "game.oteryn.com", "version": 1})
}

fn is_bad(result: &Result<Value, PublishError>) -> bool {
    matches!(result, Err(PublishError::Seed(SeedError::BadInput(_))))
}

#[test]
fn arguments_are_exactly_publish_actor_and_one_object() {
    let actor = Uuid::now_v7().to_string();
    let ok = parse_args(&[
        "publish".into(),
        "--actor".into(),
        actor.clone(),
        "{}".into(),
    ])
    .unwrap();
    assert_eq!(ok.0.to_string(), actor);
    for args in [
        vec![],
        vec!["publish".to_owned(), "--actor".to_owned(), actor.clone()],
        vec!["seed".into(), "--actor".into(), actor.clone(), "{}".into()],
        vec!["publish".into(), "--as".into(), actor.clone(), "{}".into()],
        vec![
            "publish".into(),
            "--actor".into(),
            "nope".into(),
            "{}".into(),
        ],
        vec![
            "publish".into(),
            "--actor".into(),
            Uuid::nil().to_string(),
            "{}".into(),
        ],
        vec![
            "publish".into(),
            "--actor".into(),
            actor.clone(),
            "{".into(),
        ],
        vec![
            "publish".into(),
            "--actor".into(),
            actor.clone(),
            "{}".into(),
            "{}".into(),
        ],
    ] {
        assert!(
            matches!(
                parse_args(&args),
                Err(PublishError::Seed(SeedError::BadInput(_)))
            ),
            "{args:?} was accepted"
        );
    }
}

#[tokio::test]
async fn publishes_once_audits_changes_and_rotates() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = execute(&db.pool, db.actor, &input()).await?;
    let expected = route_revision(
        1,
        Uuid::parse_str(WORLD)?,
        Uuid::parse_str(CHANNEL)?,
        "game.oteryn.com",
        7172,
        "game.oteryn.com",
    )?;
    assert_eq!(first["routeRevision"], json!(expected));
    assert_eq!(first["nativeLoginEnabled"], true);
    assert_eq!(first["worldName"], "Oteryn");
    assert_eq!(db.audits().await?, 1);
    // The world, the channel and the route the issuer selects are stored.
    let (name, location): (String, String) =
        sqlx::query_as("SELECT name, location FROM world_status WHERE id=$1")
            .bind(Uuid::parse_str(WORLD)?)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!((name.as_str(), location.as_str()), ("Oteryn", "eu"));
    let stored: String = sqlx::query_scalar(
        "SELECT native_route_revision FROM game_channels WHERE native_login_enabled",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(stored, expected);
    // The audit row names the actor and the channel and carries no secret.
    let (actor, target, before, after): (Option<Uuid>, String, Value, Value) = sqlx::query_as(
        "SELECT actor, target, before, after FROM audit_log WHERE action='game.route.published'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(actor, Some(db.actor));
    assert_eq!(target, format!("game_channels:{WORLD}/{CHANNEL}"));
    assert_eq!(before["routeRevision"], Value::Null);
    assert_eq!(after["routeRevision"], json!(expected));
    // An identical rerun prints the same object and writes nothing.
    assert_eq!(execute(&db.pool, db.actor, &input()).await?, first);
    assert_eq!(db.audits().await?, 1);
    // A renamed world is a change and is audited; the route stays.
    let mut renamed = input();
    renamed["name"] = json!("Oteryn Prime");
    let out = execute(&db.pool, db.actor, &renamed).await?;
    assert_eq!(out["routeRevision"], json!(expected));
    assert_eq!(db.audits().await?, 2);
    // Same version with other values is a conflict and rolls back the world.
    let mut moved = renamed.clone();
    moved["port"] = json!(7173);
    moved["name"] = json!("Other");
    assert!(matches!(
        execute(&db.pool, db.actor, &moved).await,
        Err(PublishError::Seed(SeedError::Conflict(_)))
    ));
    assert_eq!(
        db.count("SELECT count(*) FROM world_status WHERE name='Other'")
            .await?,
        0
    );
    assert_eq!(db.audits().await?, 2);
    // A higher version rotates the record.
    moved["version"] = json!(2);
    let rotated = execute(&db.pool, db.actor, &moved).await?;
    assert!(
        rotated["routeRevision"]
            .as_str()
            .unwrap()
            .starts_with("rt.2.")
    );
    assert_eq!(db.audits().await?, 3);
    assert_eq!(db.count("SELECT count(*) FROM game_channels").await?, 1);
    db.close().await
}

#[tokio::test]
async fn refuses_actors_without_the_permission_and_bad_input() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let stranger = Uuid::now_v7();
    account(&db.pool, stranger, "active").await?;
    let closed = Uuid::now_v7();
    account(&db.pool, closed, "closed").await?;
    sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,role_id FROM account_roles WHERE account_id=$2")
        .bind(closed)
        .bind(db.actor)
        .execute(&db.pool)
        .await?;
    for actor in [stranger, closed, Uuid::now_v7()] {
        assert_eq!(
            execute(&db.pool, actor, &input()).await,
            Err(PublishError::Denied)
        );
    }
    for (key, value) in [
        ("worldId", json!("not-a-uuid")),
        ("name", json!("")),
        ("host", json!("bad host")),
        ("tlsServerName", json!("127.0.0.1")),
        ("port", json!(0)),
        ("version", json!(0)),
        ("nativeLoginEnabled", json!("yes")),
        ("channelId", json!(WORLD)),
        ("routeRevision", json!("rt.1.00")),
    ] {
        let mut bad_input = input();
        bad_input[key] = value;
        assert!(
            is_bad(&execute(&db.pool, db.actor, &bad_input).await),
            "{key} was accepted"
        );
    }
    assert_eq!(db.count("SELECT count(*) FROM world_status").await?, 0);
    assert_eq!(db.count("SELECT count(*) FROM game_channels").await?, 0);
    assert_eq!(db.audits().await?, 0);
    db.close().await
}
