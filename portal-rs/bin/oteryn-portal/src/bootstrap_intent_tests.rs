#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ACCOUNT: &str = "019a0000-0000-7000-8000-00000000a001";
const SUSPENDED: &str = "019a0000-0000-7000-8000-00000000a002";
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
        ] {
            sqlx::query("INSERT INTO accounts (id, email, email_verified_at, password_hash, status, locale) VALUES ($1::uuid, $2, now(), 'x', $3, 'en')")
                .bind(id)
                .bind(email)
                .bind(status)
                .execute(&pool)
                .await?;
        }
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
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
    assert_eq!(parse_args(&words(&["issue"])), Ok(Source::Stdin));
    assert_eq!(
        parse_args(&words(&["issue", "--input", "req.json"])),
        Ok(Source::File(PathBuf::from("req.json")))
    );
    for bad_args in [
        words(&[]),
        words(&["read"]),
        words(&["issue", "--input"]),
        words(&["issue", "--input", ""]),
        words(&["issue", "--file", "x"]),
        words(&["issue", "--input", "a", "b"]),
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
            execute(&db.pool, settings, raw.as_bytes())
                .await
                .map_err(|e| e.code()),
            Err(4)
        );
    }

    let intent = execute(&db.pool, ON, raw.as_bytes()).await?;
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
    assert_eq!(execute(&db.pool, later, raw.as_bytes()).await?, intent);

    // Same operation, different binding: conflict.
    let changed = raw.replace("Aldric Vale", "Other Name");
    assert_eq!(
        execute(&db.pool, ON, changed.as_bytes()).await,
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
            execute(&db.pool, ON, refused.as_bytes())
                .await
                .map_err(|e| e.code()),
            Err(2),
            "{refused}"
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_bootstrap_intents")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);

    // Errors carry no intent binding or configuration.
    let body = CliError::Unavailable("database").body().to_string();
    assert!(!body.contains("postgres") && !body.contains(ACCOUNT));
    db.close().await
}
