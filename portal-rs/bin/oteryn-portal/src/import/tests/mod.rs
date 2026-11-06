#![allow(clippy::unwrap_used)]
use super::*;
use sqlx::Executor;
type TestResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
struct Fixture(Dataset);
impl Source for Fixture {
    async fn snapshot(self, tables: &BTreeSet<&str>) -> Result<Dataset> {
        Ok(self
            .0
            .into_iter()
            .filter(|(t, _)| tables.contains(t.as_str()))
            .collect())
    }
}
fn fixture() -> Dataset {
    serde_json::from_str(include_str!("legacy.json")).unwrap()
}
struct Database {
    db: PgConnection,
    schema: String,
}
impl Database {
    async fn open() -> std::result::Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL import test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let mut db = PgConnection::connect(&url).await?;
        let schema = format!("import_{}", Uuid::now_v7().simple());
        db.execute(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .await?;
        db.execute(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
            .await?;
        oteryn_db::MIGRATOR.run(&mut db).await?;
        Ok(Some(Self { db, schema }))
    }
    async fn close(mut self) -> TestResult {
        self.db.execute("SET search_path TO public").await?;
        self.db
            .execute(sqlx::AssertSqlSafe(format!(
                "DROP SCHEMA {} CASCADE",
                self.schema
            )))
            .await?;
        Ok(())
    }
}
#[test]
fn arguments_refuse_missing_unknown_and_duplicate_input() {
    let parse = |args: &[&str]| Options::parse(args.iter().map(|s| s.to_string()));
    assert!(parse(&["identity", "--source-url", "mysql://fixture"]).is_ok());
    assert!(
        !parse(&["all", "--source-url", "mysql://fixture"])
            .unwrap()
            .apply
    );
    for args in [
        vec![],
        vec!["invalid"],
        vec!["identity", "--source-url", "postgres://fixture"],
        vec![
            "identity",
            "--source-url",
            "mysql://fixture",
            "--apply",
            "--apply",
        ],
        vec!["identity", "--source-url", "mysql://fixture", "--unknown"],
    ] {
        assert!(parse(&args).is_err());
    }
}
#[tokio::test]
async fn fixture_source_has_only_requested_tables() {
    let tables = BTreeSet::from(["identities"]);
    let data = Fixture(fixture()).snapshot(&tables).await.unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data["identities"].len(), 2);
}
#[test]
fn encrypted_credentials_authenticate_before_decryption() {
    use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let key = [5u8; 32];
    let iv = [7u8; 16];
    let value = cbc::Encryptor::<aes::Aes256>::new(&key.into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(b"synthetic TOTP seed");
    let iv = STANDARD.encode(iv);
    let value = STANDARD.encode(value);
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    mac.update(iv.as_bytes());
    mac.update(value.as_bytes());
    let mac = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let envelope =
        STANDARD.encode(serde_json::to_vec(&json!({"iv":iv,"value":value,"mac":mac})).unwrap());
    let key = format!("base64:{}", STANDARD.encode(key));
    assert_eq!(
        identity::decrypt_with_key(&envelope, &key).unwrap(),
        b"synthetic TOTP seed"
    );
    assert!(
        identity::decrypt_with_key(&envelope, &format!("base64:{}", STANDARD.encode([6; 32])))
            .is_err()
    );
    assert!(identity::decrypt_with_key("bad", &key).is_err());
}
#[tokio::test]
async fn postgres_import_all_areas_dry_run_apply_and_idempotence() -> TestResult {
    let Some(mut database) = Database::open().await? else {
        return Ok(());
    };
    let data = fixture();
    for area in AREAS {
        let report = import_area(&mut database.db, &data, area, false).await?;
        if area == "identity" {
            assert!(report.problems.is_empty(), "{report:?}");
        }
        // Later areas depend on previously committed identity maps, so apply after each dry run.
        assert!(report.problems.is_empty(), "{report:?}");
        let report = import_area(&mut database.db, &data, area, true).await?;
        assert!(report.problems.is_empty(), "{report:?}");
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_import_map")
        .fetch_one(&mut database.db)
        .await?;
    for area in AREAS {
        let report = import_area(&mut database.db, &data, area, true).await?;
        assert!(report.problems.is_empty(), "{report:?}");
        assert_eq!(report.mapped, 0);
    }
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_import_map")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(before, after);
    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM accounts WHERE email='first@example.test'")
            .fetch_one(&mut database.db)
            .await?;
    assert_eq!(hash, text(&data["identities"][0], "password")?);
    assert!(oteryn_identity::password::verify(
        "synthetic import password",
        &hash
    ));
    let verified: bool = sqlx::query_scalar(
        "SELECT email_verified_at IS NOT NULL FROM accounts WHERE email='first@example.test'",
    )
    .fetch_one(&mut database.db)
    .await?;
    assert!(verified);
    let balance: i64 =
        sqlx::query_scalar("SELECT sum(available_delta)::bigint FROM wallet_entries")
            .fetch_one(&mut database.db)
            .await?;
    assert_eq!(balance, 150);
    let internal: bool = sqlx::query_scalar(
        "SELECT internal FROM ticket_messages WHERE body='Synthetic internal message'",
    )
    .fetch_one(&mut database.db)
    .await?;
    assert!(internal);
    database.close().await
}
#[tokio::test]
async fn postgres_import_dry_run_writes_nothing_and_bad_input_rolls_back() -> TestResult {
    let Some(mut database) = Database::open().await? else {
        return Ok(());
    };
    let mut data = fixture();
    let report = import_area(&mut database.db, &data, "identity", false).await?;
    assert!(report.problems.is_empty(), "{report:?}");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_import_map")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    data.get_mut("identities").unwrap()[1].insert("email".into(), json!("bad"));
    // Force a unique-email conflict after the first account was inserted.
    let first = data["identities"][0]["email"].clone();
    data.get_mut("identities").unwrap()[1].insert("email".into(), first);
    let report = import_area(&mut database.db, &data, "identity", true).await?;
    assert!(!report.problems.is_empty());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_import_map")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    database.close().await
}
#[tokio::test]
async fn postgres_import_refuses_auctions_balance_mismatch_and_missing_maps() -> TestResult {
    let Some(mut database) = Database::open().await? else {
        return Ok(());
    };
    let mut data = fixture();
    data.get_mut("character_auctions").unwrap().push(
        json!({"id":"1","status":"active"})
            .as_object()
            .unwrap()
            .clone(),
    );
    let report = import_area(&mut database.db, &data, "commerce", true).await?;
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.contains("open or unresolved"))
    );
    data.get_mut("character_auctions").unwrap().clear();
    data.get_mut("wallet_accounts").unwrap()[0].insert("available_balance".into(), json!(999));
    let report = import_area(&mut database.db, &data, "commerce", true).await?;
    assert!(report.problems.iter().any(|p| p.contains("does not match")));
    let report = import_area(&mut database.db, &data, "support", false).await?;
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.contains("unmapped foreign key"))
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_import_map")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    database.close().await
}
#[tokio::test]
async fn postgres_import_refuses_unrepresentable_security_and_privacy() -> TestResult {
    let Some(mut database) = Database::open().await? else {
        return Ok(());
    };
    let mut data = fixture();
    data.get_mut("identity_recovery_keys").unwrap().push(
        json!({"identity_id":"1","key_hash":"synthetic"})
            .as_object()
            .unwrap()
            .clone(),
    );
    let report = import_area(&mut database.db, &data, "identity", true).await?;
    assert!(report.problems.iter().any(|p| p.contains("recovery keys")));
    data.get_mut("identity_recovery_keys").unwrap().clear();
    data.get_mut("identities").unwrap()[1]
        .insert("password".into(), json!("$2y$10$synthetic.fixture.only"));
    let report = import_area(&mut database.db, &data, "identity", false).await?;
    assert!(report.problems.iter().any(|p| p.contains("bcrypt")));
    data = fixture();

    let report = import_area(&mut database.db, &data, "identity", true).await?;
    assert!(report.problems.is_empty(), "{report:?}");
    data.get_mut("character_profile_preferences").unwrap()[0]
        .insert("show_deaths".into(), json!(false));
    let report = import_area(&mut database.db, &data, "characters", true).await?;
    assert!(report.problems.iter().any(|p| p.contains("exceed")));

    data.get_mut("news_posts").unwrap()[0].insert("slug".into(), json!("INVALID SLUG"));
    let report = import_area(&mut database.db, &data, "content", false).await?;
    assert!(report.problems.iter().any(|p| p.contains("destination")));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM news")
        .fetch_one(&mut database.db)
        .await?;
    assert_eq!(count, 0);
    database.close().await
}

#[test]
fn totp_seed_is_preserved_and_bound_to_the_canonical_account() {
    use aes_gcm::{
        Aes256Gcm, KeyInit, Nonce,
        aead::{Aead, Payload},
    };
    let account = Uuid::now_v7();
    let seed = vec![9u8; 20];
    let encrypted = identity::encrypt_secret(account, &seed, &"05".repeat(32)).unwrap();
    let cipher = Aes256Gcm::new_from_slice(&[5; 32]).unwrap();
    let nonce = Nonce::from(<[u8; 12]>::try_from(&encrypted[..12]).unwrap());
    let decoded = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &encrypted[12..],
                aad: account.as_bytes(),
            },
        )
        .unwrap();
    assert_eq!(seed, decoded);
    assert!(
        cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &encrypted[12..],
                    aad: Uuid::now_v7().as_bytes()
                }
            )
            .is_err()
    );
    assert!(identity::encrypt_secret(account, &seed, "invalid").is_err());
}
