#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ACCOUNT: &str = "019a0000-0000-7000-8000-00000000a001";
const SUSPENDED: &str = "019a0000-0000-7000-8000-00000000a002";
const STAFF: &str = "019a0000-0000-7000-8000-00000000a0f1";
const ON: Settings = Settings {
    enabled: true,
    ttl_seconds: Some(120),
};

fn request(operation: &str, identity: &str) -> String {
    json!({
        "identityId": identity, "operationId": operation,
        "targetWorldId": "019a0000-0000-7000-8000-00000000e001",
        "requestedName": "Aldric Vale", "requestedSex": "Female",
        "profileRevision": "profile:1", "rulesetRevision": "ruleset:1",
        "contentRevision": "content:1", "starterTemplateRevision": "starter:1",
    })
    .to_string()
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
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
        let schema = format!("bootstrap_cli_{}", Uuid::now_v7().simple());
        // Schema identifiers contain only a fixed prefix and generated UUID hex.
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(sql)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        for (id, email, status) in [
            (ACCOUNT, "operator-cli@example.invalid", "active"),
            (SUSPENDED, "suspended-cli@example.invalid", "suspended"),
            (STAFF, "staff-cli@example.invalid", "active"),
        ] {
            sqlx::query("INSERT INTO accounts (id, email, email_verified_at, password_hash, status, locale) VALUES ($1::uuid, $2, now(), 'x', $3, 'en')")
                .bind(id)
                .bind(email)
                .bind(status)
                .execute(&pool)
                .await?;
        }
        // The built-in senior game master role holds game.player.manage.
        sqlx::query("INSERT INTO account_roles (account_id, role_id) SELECT $1::uuid, id FROM roles WHERE key='game_master_senior'")
            .bind(STAFF)
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn counts(&self) -> Result<(i64, i64), sqlx::Error> {
        sqlx::query_as("SELECT (SELECT count(*) FROM character_bootstrap_intents), (SELECT count(*) FROM audit_log WHERE action='character.bootstrap_intent.issued')")
            .fetch_one(&self.pool)
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

#[test]
fn arguments_and_exit_codes_are_distinct() {
    let words = |list: &[&str]| list.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    let command = |source, actor: Option<&str>| Command {
        source,
        actor: actor.map(str::to_owned),
    };
    assert_eq!(
        parse_args(&words(&["issue"])),
        Ok(command(Source::Stdin, None))
    );
    assert_eq!(
        parse_args(&words(&["issue", "--actor", STAFF])),
        Ok(command(Source::Stdin, Some(STAFF)))
    );
    let file = || Source::File(PathBuf::from("req.json"));
    assert_eq!(
        parse_args(&words(&["issue", "--input", "req.json"])),
        Ok(command(file(), None))
    );
    for order in [
        words(&["issue", "--actor", STAFF, "--input", "req.json"]),
        words(&["issue", "--input", "req.json", "--actor", STAFF]),
    ] {
        assert_eq!(parse_args(&order), Ok(command(file(), Some(STAFF))));
    }
    for bad_args in [
        words(&[]),
        words(&["read"]),
        words(&["issue", "--input"]),
        words(&["issue", "--input", ""]),
        words(&["issue", "--file", "x"]),
        words(&["issue", "--input", "a", "b"]),
        words(&["issue", "--actor"]),
        words(&["issue", "--actor", STAFF, "--actor", STAFF]),
        words(&["issue", "--input", "a", "--input", "b"]),
        words(&["issue", "--actor", STAFF, "extra"]),
    ] {
        assert_eq!(parse_args(&bad_args).map_err(|e| e.code()), Err(2));
    }
    let codes = [
        CliError::BadInput(String::new()).code(),
        CliError::Conflict.code(),
        CliError::Unavailable("x").code(),
    ];
    assert_eq!(codes, [2, 3, 4]);
    assert_eq!(
        CliError::from(IssueError::Db),
        CliError::Unavailable("database")
    );
    // The actor is a lower-case UUIDv7 and is required.
    assert!(actor(Some(STAFF)).is_ok());
    for refused in [
        None,
        Some(""),
        Some("staff"),
        Some("019A0000-0000-7000-8000-00000000A0F1"),
        Some("019a0000-0000-4000-8000-00000000a0f1"),
        Some("{019a0000-0000-7000-8000-00000000a0f1}"),
    ] {
        assert_eq!(actor(refused).map_err(|e| e.code()), Err(2), "{refused:?}");
    }
}

#[test]
fn flag_is_default_off_and_ttl_has_no_default() {
    for (enabled, ttl, expected) in [
        (None, Some("60"), None),
        (Some("1"), Some("60"), None),
        (Some("true"), None, None),
        (Some("true"), Some("0"), None),
        (Some("true"), Some("-5"), None),
        (Some("true"), Some("301"), None),
        (Some("true"), Some("060"), None),
        (Some("true"), Some("+60"), None),
        (Some("true"), Some("6O"), None),
        (Some("true"), Some("1"), Some(1)),
        (Some("true"), Some("300"), Some(300)),
    ] {
        let settings = Settings::from_values(enabled, ttl);
        assert_eq!(settings.ttl().ok(), expected, "{enabled:?} {ttl:?}");
        if expected.is_none() {
            assert_eq!(settings.ttl().map_err(|e| e.code()), Err(4));
        }
    }
}

#[test]
fn input_is_one_flat_object_of_string_fields() {
    let good = request("019a0000-0000-7000-8000-0000000c0001", ACCOUNT);
    let parsed = parse_input(good.as_bytes(), 60).unwrap();
    assert_eq!(parsed.identity_id, ACCOUNT);
    assert_eq!(parsed.ttl_seconds, 60);
    let mut pretty =
        serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&good).unwrap())
            .unwrap();
    pretty.push('\n');
    assert_eq!(parse_input(pretty.as_bytes(), 60).unwrap(), parsed);

    let object: serde_json::Value = serde_json::from_str(&good).unwrap();
    let mut cases: Vec<String> = vec![
        String::new(),
        "[]".into(),
        "not json".into(),
        format!("{{\"identityId\":\"{ACCOUNT}\",\"identityId\":\"{ACCOUNT}\"}}"),
        "x".repeat(MAX_INPUT_BYTES + 1),
    ];
    for key in FIELDS {
        let mut missing = object.clone();
        missing.as_object_mut().unwrap().remove(key);
        cases.push(missing.to_string());
        let mut number = object.clone();
        number[key] = json!(1);
        cases.push(number.to_string());
        let mut nested = object.clone();
        nested[key] = json!({"x": "y"});
        cases.push(nested.to_string());
    }
    for extra in ["accountId", "ttlSeconds", "account_id"] {
        let mut more = object.clone();
        more[extra] = json!("x");
        cases.push(more.to_string());
    }
    for raw in cases {
        assert_eq!(
            parse_input(raw.as_bytes(), 60).map_err(|e| e.code()),
            Err(2),
            "{raw}"
        );
    }
}

#[tokio::test]
async fn issues_replays_and_refuses_with_distinct_codes() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let operation = "019a0000-0000-7000-8000-0000000c0001";
    let raw = request(operation, ACCOUNT);

    // Disabled or unconfigured: unavailable before any read or write.
    for settings in [
        Settings::from_values(None, Some("60")),
        Settings::from_values(Some("true"), None),
    ] {
        assert_eq!(
            execute(&db.pool, settings, Some(STAFF), raw.as_bytes())
                .await
                .map_err(|e| e.code()),
            Err(4)
        );
    }

    let intent = execute(&db.pool, ON, Some(STAFF), raw.as_bytes()).await?;
    let wire = canonical::decode(intent.as_bytes(), 4096, 3)?;
    assert_eq!(
        wire.get("operation_id"),
        Some(&Value::String(operation.into()))
    );
    assert_eq!(wire.get("account_id"), Some(&Value::String(ACCOUNT.into())));
    assert_eq!(
        wire.get("requested_sex"),
        Some(&Value::String("Female".into()))
    );
    assert_eq!(canonical::encode(&wire)?, intent.as_bytes());
    // Exact retry: identical bytes, even with another configured TTL.
    let later = Settings::from_values(Some("true"), Some("5"));
    assert_eq!(
        execute(&db.pool, later, Some(STAFF), raw.as_bytes()).await?,
        intent
    );

    // Same operation, different binding: conflict.
    let changed = raw.replace("Aldric Vale", "Other Name");
    assert_eq!(
        execute(&db.pool, ON, Some(STAFF), changed.as_bytes()).await,
        Err(CliError::Conflict)
    );
    // Inactive, unknown or malformed Identity and bad fields: bad input.
    for refused in [
        request("019a0000-0000-7000-8000-0000000c0002", SUSPENDED),
        request(
            "019a0000-0000-7000-8000-0000000c0003",
            "019a0000-0000-7000-8000-00000000dead",
        ),
        raw.replace("Female", "female"),
        raw.replace("profile:1", "-profile"),
    ] {
        assert_eq!(
            execute(&db.pool, ON, Some(STAFF), refused.as_bytes())
                .await
                .map_err(|e| e.code()),
            Err(2),
            "{refused}"
        );
    }
    // One intent and exactly one audit row: retries, conflicts and refusals add none.
    assert_eq!(db.counts().await?, (1, 1));
    let (actor_id, target, after): (Option<Uuid>, String, serde_json::Value) = sqlx::query_as(
        "SELECT actor, target, after FROM audit_log WHERE action='character.bootstrap_intent.issued'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(actor_id, Some(Uuid::parse_str(STAFF)?));
    assert_eq!(target, operation);
    assert_eq!(after["identityId"], json!(ACCOUNT));
    assert_eq!(after["ttlSeconds"], json!(120));
    let digest =
        sqlx::query_scalar::<_, String>("SELECT intent_sha256 FROM character_bootstrap_intents")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(after["intentSha256"], json!(digest));
    assert!(!after.to_string().contains("Aldric") && !after.to_string().contains(&intent));

    // Errors carry no intent binding or configuration.
    let body = CliError::Unavailable("database").body().to_string();
    assert!(!body.contains("postgres") && !body.contains(ACCOUNT));
    db.close().await
}

#[tokio::test]
async fn missing_malformed_or_unknown_actor_issues_and_audits_nothing() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let raw = request("019a0000-0000-7000-8000-0000000c0011", ACCOUNT);
    for refused in [
        None,
        Some("not-a-uuid"),
        Some("019A0000-0000-7000-8000-00000000A0F1"),
        Some("019a0000-0000-4000-8000-00000000a0f1"),
        // Unknown, not active, or active without the staff permission.
        Some("019a0000-0000-7000-8000-00000000dead"),
        Some(SUSPENDED),
        Some(ACCOUNT),
    ] {
        assert_eq!(
            execute(&db.pool, ON, refused, raw.as_bytes())
                .await
                .map_err(|e| e.code()),
            Err(2),
            "{refused:?}"
        );
    }
    assert_eq!(db.counts().await?, (0, 0));
    // Disabled issuance stays unavailable whatever the actor.
    assert_eq!(
        execute(
            &db.pool,
            Settings::from_values(None, Some("60")),
            None,
            raw.as_bytes()
        )
        .await
        .map_err(|e| e.code()),
        Err(4)
    );
    db.close().await
}
