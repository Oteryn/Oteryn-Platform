#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use oteryn_internal_api::canonical::{self, Value as Wire};
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    witness: tempfile::TempDir,
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
        let schema = format!("seed_{}", Uuid::now_v7().simple());
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
        Ok(Some(Self {
            pool,
            admin,
            schema,
            witness: tempfile::tempdir()?,
        }))
    }
    fn settings(&self) -> Settings {
        Settings {
            witness_directory: Some(self.witness.path().to_path_buf()),
            mfa: Some(Mfa::new(self.pool.clone(), [7; 32])),
        }
    }
    async fn seed(&self, entity: &str, input: Value) -> Result<Value, SeedError> {
        execute(&self.pool, &self.settings(), entity, &input).await
    }
    /// Runs the command twice and requires the same output both times.
    async fn twice(&self, entity: &str, input: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let first = self.seed(entity, input.clone()).await?;
        let second = self.seed(entity, input).await?;
        assert_eq!(first, second, "{entity} is not idempotent");
        Ok(first)
    }
    async fn count(&self, sql: &'static str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await
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

fn is_bad(result: Result<Value, SeedError>) -> bool {
    matches!(result, Err(SeedError::BadInput(_)))
}
fn is_conflict(result: Result<Value, SeedError>) -> bool {
    matches!(result, Err(SeedError::Conflict(_)))
}

const PASSWORD: &str = "correct horse battery";
const WORLD: &str = "019a0000-0000-7000-8000-00000000e001";

async fn account(db: &Database, email: &str) -> Result<String, Box<dyn std::error::Error>> {
    let out = db
        .seed("account", json!({"email": email, "password": PASSWORD}))
        .await?;
    Ok(out["accountId"].as_str().unwrap().to_owned())
}

#[tokio::test]
async fn account_is_idempotent_hashed_and_refuses_bad_input() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = "019a0000-0000-7000-8000-00000000a001";
    let input = json!({"email": "Player1@example.invalid", "password": PASSWORD, "accountId": id});
    let out = db.twice("account", input).await?;
    assert_eq!(
        out,
        json!({"entity": "account", "accountId": id, "email": "player1@example.invalid",
               "status": "active", "nativeSecurityGeneration": 1})
    );
    assert_eq!(db.count("SELECT count(*) FROM accounts").await?, 1);
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert!(hash.starts_with("$argon2id$") && oteryn_identity::password::verify(PASSWORD, &hash));
    // Without accountId the existing row is found by e-mail.
    let again = db
        .seed(
            "account",
            json!({"email": "player1@example.invalid", "password": PASSWORD}),
        )
        .await?;
    assert_eq!(again, out);
    // A generated id is UUIDv7.
    let generated = account(&db, "player2@example.invalid").await?;
    assert_eq!(Uuid::parse_str(&generated)?.get_version_num(), 7);

    assert!(is_conflict(
        db.seed(
            "account",
            json!({"email": "player1@example.invalid", "password": "another password"})
        )
        .await
    ));
    assert!(is_conflict(
        db.seed(
            "account",
            json!({"email": "player1@example.invalid", "password": PASSWORD,
                                   "accountId": "019a0000-0000-7000-8000-00000000a002"})
        )
        .await
    ));
    assert!(is_conflict(
        db.seed(
            "account",
            json!({"email": "player3@example.invalid", "password": PASSWORD, "accountId": id})
        )
        .await
    ));
    for input in [
        json!({"email": "player@example.com", "password": PASSWORD}),
        json!({"email": "@example.invalid", "password": PASSWORD}),
        json!({"email": "a b@example.invalid", "password": PASSWORD}),
        json!({"email": "short@example.invalid", "password": "short"}),
        json!({"email": "x@example.invalid", "password": PASSWORD, "accountId": "5f0c6a52-2f8e-4c39-9a4d-0d3a1d0f0b11"}),
        json!({"email": "x@example.invalid", "password": PASSWORD, "admin": true}),
        json!({"email": "x@example.invalid", "password": PASSWORD, "locale": "de"}),
        json!({"password": PASSWORD}),
        json!(["not", "an", "object"]),
    ] {
        assert!(is_bad(db.seed("account", input.clone()).await), "{input}");
    }
    assert_eq!(db.count("SELECT count(*) FROM accounts").await?, 2);
    db.close().await
}

#[tokio::test]
async fn world_and_native_client_are_idempotent_and_refuse_bad_input() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let out = db
        .twice(
            "world",
            json!({"worldId": WORLD, "slug": "entry", "name": "Entry", "region": "local",
                   "host": "127.0.0.1", "port": 7171}),
        )
        .await?;
    assert_eq!(out["worldId"], WORLD);
    assert_eq!(out["notStored"], json!(["host", "port", "slug"]));
    let row = sqlx::query("SELECT name, location FROM world_status")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(row.try_get::<String, _>("name")?, "Entry");
    assert_eq!(row.try_get::<String, _>("location")?, "local");
    // A rename is an upsert on the same id, never a second world.
    db.seed("world", json!({"worldId": WORLD, "name": "Entry Two"}))
        .await?;
    assert_eq!(db.count("SELECT count(*) FROM world_status").await?, 1);
    for input in [
        json!({"name": "Entry"}),
        json!({"worldId": "5f0c6a52-2f8e-4c39-9a4d-0d3a1d0f0b11", "name": "Entry"}),
        json!({"worldId": WORLD, "name": " "}),
        json!({"worldId": WORLD, "name": "Entry", "pvp": "open"}),
        json!({"worldId": WORLD, "name": 7}),
    ] {
        assert!(is_bad(db.seed("world", input.clone()).await), "{input}");
    }

    let client = db.twice("oauth-native-client", json!({})).await?;
    assert_eq!(client["clientId"], DEFAULT_NATIVE_CLIENT);
    let explicit = "019a0000-0000-7000-8000-00000000c001";
    db.twice("oauth-native-client", json!({"clientId": explicit}))
        .await?;
    sqlx::query("UPDATE oauth_native_clients SET enabled=false")
        .execute(&db.pool)
        .await?;
    db.seed("oauth-native-client", json!({"clientId": explicit}))
        .await?;
    assert_eq!(
        db.count("SELECT count(*) FROM oauth_native_clients WHERE enabled AND first_party AND public_client AND native_ticket AND client_id='019a0000-0000-7000-8000-00000000c001'")
            .await?,
        1
    );
    assert_eq!(
        db.count("SELECT count(*) FROM oauth_native_clients")
            .await?,
        2
    );
    assert!(is_bad(
        db.seed("oauth-native-client", json!({"clientId": "client"}))
            .await
    ));
    assert!(is_bad(
        db.seed("oauth-native-client", json!({"secret": "x"})).await
    ));
    db.close().await
}

const FRESH_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const RECOVERY_KEY: &str = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

#[tokio::test]
async fn signing_trust_publishes_both_scopes_once() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let fresh = db
        .twice(
            "signing-trust",
            json!({"purpose": "fresh_admission", "keyId": "fresh-1", "publicKeyHex": FRESH_KEY}),
        )
        .await?;
    assert_eq!(fresh["issuer"], keys::ISSUER);
    assert_eq!(fresh["profile"], keys::PROFILE);
    assert_eq!(fresh["issuerRevision"], 1);
    assert_eq!(
        fresh["publicKey"],
        URL_SAFE_NO_PAD.encode(hex32(FRESH_KEY).unwrap())
    );
    let recovery = db
        .twice(
            "signing-trust",
            json!({"issuer": RECOVERY_ISSUER, "purpose": RECOVERY_PURPOSE,
                   "keyId": "recovery-1", "publicKeyHex": RECOVERY_KEY}),
        )
        .await?;
    assert_eq!(recovery["profile"], RECOVERY_PROFILE);
    assert_eq!(recovery["issuerRevision"], 1);
    assert_eq!(
        db.count("SELECT count(*) FROM native_game_signing_trust_key_versions")
            .await?,
        2
    );
    assert_eq!(
        db.count("SELECT count(*) FROM native_game_signing_trust_profiles")
            .await?,
        2
    );
    // The retained witness equals the relational revision for each scope.
    for (issuer, profile, purpose) in [
        (keys::ISSUER, keys::PROFILE, "fresh_admission"),
        (RECOVERY_ISSUER, RECOVERY_PROFILE, RECOVERY_PURPOSE),
    ] {
        let namespace = trust_state_namespace(issuer, profile, purpose).unwrap();
        let witness = Witness::lock(db.witness.path(), &namespace).unwrap();
        assert_eq!(witness.floor(&namespace).unwrap(), Some(1));
    }

    // A kid keeps its key, a key keeps its kid, and a kid stays in one scope.
    assert!(is_conflict(
        db.seed(
            "signing-trust",
            json!({"purpose": "fresh_admission", "keyId": "fresh-1", "publicKeyHex": RECOVERY_KEY})
        )
        .await
    ));
    assert!(is_conflict(
        db.seed(
            "signing-trust",
            json!({"issuer": RECOVERY_ISSUER, "purpose": RECOVERY_PURPOSE,
                                         "keyId": "fresh-1", "publicKeyHex": FRESH_KEY})
        )
        .await
    ));
    assert!(is_conflict(
        db.seed(
            "signing-trust",
            json!({"issuer": RECOVERY_ISSUER, "purpose": RECOVERY_PURPOSE,
                                         "keyId": "recovery-2", "publicKeyHex": RECOVERY_KEY})
        )
        .await
    ));
    for input in [
        json!({"purpose": "fresh_admission", "keyId": "k", "publicKeyHex": "abcd"}),
        json!({"purpose": "fresh_admission", "keyId": "k", "publicKeyHex": FRESH_KEY.replace('d', "g")}),
        json!({"purpose": "fresh_admission", "keyId": "bad kid", "publicKeyHex": FRESH_KEY}),
        json!({"issuer": "urn:other", "purpose": "x", "keyId": "k", "publicKeyHex": FRESH_KEY}),
        json!({"issuer": RECOVERY_ISSUER, "purpose": "fresh_admission", "keyId": "k", "publicKeyHex": FRESH_KEY}),
        json!({"purpose": "fresh_admission", "keyId": "k", "publicKeyHex": FRESH_KEY, "privateKey": "x"}),
    ] {
        assert!(
            is_bad(db.seed("signing-trust", input.clone()).await),
            "{input}"
        );
    }
    let unconfigured = execute(
        &db.pool,
        &Settings::default(),
        "signing-trust",
        &json!({"purpose": "fresh_admission", "keyId": "fresh-1", "publicKeyHex": FRESH_KEY}),
    )
    .await;
    assert!(matches!(unconfigured, Err(SeedError::Unavailable(_))));
    db.close().await
}

#[tokio::test]
async fn bootstrap_intent_is_issued_once_and_readable() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = account(&db, "bootstrap@example.invalid").await?;
    let operation = "019a0000-0000-7000-8000-0000000b0001";
    let input = json!({
        "accountId": account, "operationId": operation, "targetWorldId": WORLD,
        "requestedName": "Aldric Vale", "requestedSex": "Female",
        "profileRevision": "profile:1", "rulesetRevision": "ruleset:1",
        "contentRevision": "content:1", "starterTemplateRevision": "starter:1",
    });
    let out = db.twice("bootstrap-intent", input.clone()).await?;
    assert_eq!(out["operationId"], operation);
    assert_eq!(out["sourceRevision"], "1");
    let issued: i64 = out["issuedAtSource"].as_str().unwrap().parse()?;
    let expires: i64 = out["expiresAtSource"].as_str().unwrap().parse()?;
    assert_eq!(expires - issued, 300);
    assert_eq!(
        db.count("SELECT count(*) FROM character_bootstrap_intents")
            .await?,
        1
    );

    // The internal read route's own reader accepts the stored intent byte for byte.
    let reader = oteryn_characters::bootstrap::Reader::new(
        db.pool.clone(),
        oteryn_characters::bootstrap::Settings { enabled: true },
        std::sync::Arc::new(oteryn_characters::overview::Accounts(db.pool.clone())),
    );
    let raw = reader.current(operation, issued).await?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        out["intentSha256"].as_str().unwrap()
    );
    let stored = canonical::decode(&raw, 4096, 3)?;
    assert_eq!(
        stored.get("requested_sex"),
        Some(&Wire::String("Female".into()))
    );

    // A second operation advances the issuer revision.
    let mut next = input.clone();
    next["operationId"] = json!("019a0000-0000-7000-8000-0000000b0002");
    next["ttlSeconds"] = json!(60);
    let second = db.seed("bootstrap-intent", next).await?;
    assert_eq!(second["sourceRevision"], "2");

    let mut changed = input.clone();
    changed["requestedName"] = json!("Other Name");
    assert!(is_conflict(db.seed("bootstrap-intent", changed).await));
    for (key, value) in [
        ("requestedSex", json!("female")),
        ("requestedName", json!("Al1")),
        ("requestedName", json!("A")),
        ("requestedName", json!("Two  Spaces")),
        ("accountId", json!("019a0000-0000-7000-8000-00000000dead")),
        ("operationId", json!("not-a-uuid")),
        ("profileRevision", json!(":leading")),
        ("ttlSeconds", json!(301)),
        ("ttlSeconds", json!(0)),
        ("extra", json!(1)),
    ] {
        let mut bad_input = input.clone();
        bad_input["operationId"] = json!("019a0000-0000-7000-8000-0000000b0003");
        bad_input[key] = value;
        assert!(
            is_bad(db.seed("bootstrap-intent", bad_input).await),
            "{key}"
        );
    }
    assert_eq!(
        db.count("SELECT count(*) FROM character_bootstrap_intents")
            .await?,
        2
    );
    // The testing seed needs no actor and writes no operator audit row.
    assert_eq!(
        db.count("SELECT count(*) FROM audit_log WHERE action='character.bootstrap_intent.issued'")
            .await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn account_mfa_enrols_once_and_the_secret_logs_in() -> TestResult {
    use totp_rs::{Algorithm, Secret, TOTP};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = account(&db, "mfa@example.invalid").await?;
    let id = Uuid::parse_str(&account)?;
    let input = json!({"accountId": account, "password": PASSWORD});

    for (key, value) in [
        ("password", json!("wrong horse battery")),
        ("accountId", json!("019a0000-0000-7000-8000-00000000dead")),
        ("accountId", json!("not-a-uuid")),
        ("extra", json!(1)),
    ] {
        let mut bad_input = input.clone();
        bad_input[key] = value;
        assert!(is_bad(db.seed("account-mfa", bad_input).await), "{key}");
    }
    let unconfigured = execute(&db.pool, &Settings::default(), "account-mfa", &input).await;
    assert!(matches!(unconfigured, Err(SeedError::Unavailable(_))));

    let out = db.seed("account-mfa", input.clone()).await?;
    assert_eq!(out["accountId"], account);
    assert_eq!(out["recoveryCodes"].as_array().unwrap().len(), 8);
    let mfa = db.settings().mfa.unwrap();
    assert!(mfa.enrolled(id).await?);
    let required: bool = sqlx::query_scalar("SELECT login_mfa_required FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(required);
    // The returned secret produces a code the portal accepts at the current step.
    let secret = Secret::Encoded(out["totpSecret"].as_str().unwrap().to_owned()).to_bytes()?;
    let code = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret)?.generate_current()?;
    assert!(mfa.verify_totp(id, &code).await?);

    // The secret exists only at enrolment: a rerun is a conflict, never a re-key.
    assert!(is_conflict(db.seed("account-mfa", input).await));
    db.close().await
}

#[tokio::test]
async fn every_seed_action_refuses_outside_testing() -> TestResult {
    let args = [
        "account-mfa".to_owned(),
        r#"{"accountId":"019a0000-0000-7000-8000-00000000a001","password":"x"}"#.to_owned(),
    ];
    for environment in [
        None,
        Some("production"),
        Some("preproduction"),
        Some("Testing"),
    ] {
        assert_eq!(
            run_in(environment, &args).await,
            Err(SeedError::Unavailable(
                "seed runs only with OTERYN_ENV=testing"
            ))
        );
    }
    Ok(())
}

#[tokio::test]
async fn pending_and_unknown_entities_are_reported() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for entity in ["channel", "route"] {
        assert!(matches!(
            db.seed(entity, json!({})).await,
            Err(SeedError::Pending(_))
        ));
    }
    assert!(matches!(
        db.seed("native-evidence", json!({})).await,
        Err(SeedError::NotApplicable(_))
    ));
    assert!(is_bad(db.seed("drop-database", json!({})).await));
    db.close().await
}

#[test]
fn hex_keys_decode_exactly() {
    assert_eq!(hex32(&"00".repeat(32)), Ok([0; 32]));
    assert_eq!(hex32(&"FF".repeat(32)), Ok([255; 32]));
    assert!(hex32(&"0".repeat(63)).is_err());
    assert!(hex32(&"zz".repeat(32)).is_err());
}
