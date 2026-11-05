use super::*;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::{cell::RefCell, collections::BTreeMap, fs, path::PathBuf};
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Default)]
struct Memory(RefCell<Option<Zeroizing<Vec<u8>>>>);
impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Memory([redacted])")
    }
}
impl KeyStorage for Memory {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyError> {
        Ok(self.0.borrow().clone())
    }
    fn save(&self, plaintext: &[u8]) -> Result<(), KeyError> {
        *self.0.borrow_mut() = Some(Zeroizing::new(plaintext.to_vec()));
        Ok(())
    }
}
#[derive(Debug, Default)]
struct Trust(BTreeMap<String, TrustedKey>);
impl Trust {
    fn publish(&mut self, key: &PublicKey, now: u64) -> Result<(), KeyError> {
        self.0.insert(
            key.key_id.clone(),
            TrustedKey {
                public_key: trust::decode_key(&key.public_key)?,
                trusted_since: now,
            },
        );
        Ok(())
    }
}
impl SigningTrust for Trust {
    fn trusted_key(
        &self,
        issuer: &str,
        profile: &str,
        purpose: &str,
        kid: &str,
        _: u64,
    ) -> Result<Option<TrustedKey>, KeyError> {
        assert_eq!((issuer, profile, purpose), (ISSUER, PROFILE, "fresh_entry"));
        Ok(self.0.get(kid).copied())
    }
}
fn settings() -> Settings {
    Settings {
        enabled: true,
        key_purpose: "fresh_entry".into(),
        clock_uncertainty_seconds: 2,
    }
}
fn ring() -> Result<Keyring<Memory>, KeyError> {
    Keyring::create(settings(), Memory::default())
}

#[test]
fn rotation_overlap_resigns_old_and_signs_new_then_destroys_retired_seed() -> TestResult {
    let mut ring = ring()?;
    let mut trust = Trust::default();
    let old = ring.generate_next("old")?;
    assert_eq!(ring.activate("old", 100, &trust), Err(KeyError::Untrusted));
    trust.publish(&old, 100)?;
    assert_eq!(ring.activate("old", 106, &trust), Err(KeyError::TooEarly));
    ring.activate("old", 107, &trust)?;
    let original = ring.sign(b"header.payload", 137, 107, &trust)?;
    verify(
        trust.0["old"].public_key,
        b"header.payload",
        original.signature(),
    )?;
    let new = ring.generate_next("new")?;
    trust.publish(&new, 108)?;
    assert_eq!(ring.generate_next("third"), Err(KeyError::InvalidState));
    assert_eq!(ring.activate("new", 114, &trust), Err(KeyError::TooEarly));
    ring.activate("new", 115, &trust)?;
    let current = ring.sign(b"new.payload", 145, 115, &trust)?;
    assert_eq!(current.key_id(), "new");
    verify(
        trust.0["new"].public_key,
        b"new.payload",
        current.signature(),
    )?;
    assert_eq!(
        ring.resign(
            original.key_id(),
            original.signing_input(),
            original.expires_at(),
            116,
            &trust
        )?,
        original.signature()
    );
    assert_eq!(ring.publication_keys()?.len(), 2);
    assert_eq!(
        ring.retire("old", 143, &trust, false),
        Err(KeyError::TooEarly)
    );
    assert_eq!(
        ring.retire("old", 144, &trust, false),
        Err(KeyError::Untrusted)
    );
    trust.0.remove("old");
    ring.retire("old", 144, &trust, false)?;
    assert_eq!(ring.state("old"), Some(KeyState::Retired));
    assert!(ring.record("old")?.seed.is_none());
    assert_eq!(ring.publication_keys()?.len(), 1);
    assert!(
        ring.resign(
            original.key_id(),
            original.signing_input(),
            original.expires_at(),
            120,
            &trust
        )
        .is_err()
    );
    assert_eq!(ring.generate_next("old"), Err(KeyError::InvalidState));
    let storage = ring.storage;
    let mut reopened = Keyring::open(settings(), storage)?;
    assert_eq!(reopened.state("old"), Some(KeyState::Retired));
    reopened.generate_next("third")?;
    Ok(())
}
#[test]
fn disabled_bad_input_and_unknown_state_deny() -> TestResult {
    assert!(matches!(
        Keyring::open(Settings::default(), Memory::default()),
        Err(KeyError::Disabled)
    ));
    let mut ring = ring()?;
    for kid in ["", "bad\n", "bad key", "\"injected"] {
        assert_eq!(ring.generate_next(kid), Err(KeyError::BadInput));
    }
    assert_eq!(ring.generate_next(&"a".repeat(65)), Err(KeyError::BadInput));
    let mut trust = Trust::default();
    trust.publish(&ring.generate_next("key")?, 100)?;
    ring.activate("key", 107, &trust)?;
    for (input, expiry) in [
        (b"".as_slice(), 120),
        (b"x", 106),
        (b"x", 111),
        (b"x", 138),
        (&[255], 120),
    ] {
        assert!(matches!(
            ring.sign(input, expiry, 107, &trust),
            Err(KeyError::BadInput)
        ));
    }
    assert!(ring.sign(&vec![b'a'; 4010], 120, 107, &trust).is_err());
    assert_eq!(
        ring.activate("missing", 110, &trust),
        Err(KeyError::InvalidState)
    );
    assert_eq!(
        ring.retire("missing", 110, &trust, true),
        Err(KeyError::InvalidState)
    );
    assert!(verify([0; 32], b"x", &[0; 63]).is_err());
    Ok(())
}
#[test]
fn revoked_wrong_key_bad_replay_and_future_trust_deny() -> TestResult {
    let mut ring = ring()?;
    let mut trust = Trust::default();
    let key = ring.generate_next("key")?;
    trust.publish(&key, 100)?;
    ring.activate("key", 107, &trust)?;
    let grant = ring.sign(b"header.payload", 130, 107, &trust)?;
    assert_eq!(
        ring.resign(grant.key_id(), &[255], grant.expires_at(), 108, &trust),
        Err(KeyError::BadInput)
    );
    assert!(
        ring.resign(
            grant.key_id(),
            grant.signing_input(),
            grant.expires_at(),
            130,
            &trust
        )
        .is_err()
    );
    trust.0.get_mut("key").ok_or("missing")?.public_key = [0; 32];
    assert!(matches!(
        ring.sign(b"x", 130, 110, &trust),
        Err(KeyError::Untrusted)
    ));
    trust.publish(&key, 200)?;
    assert!(matches!(
        ring.sign(b"x", 130, 110, &trust),
        Err(KeyError::Untrusted)
    ));
    trust.0.remove("key");
    assert!(matches!(
        ring.sign(b"x", 130, 110, &trust),
        Err(KeyError::Untrusted)
    ));
    ring.retire("key", 110, &trust, true)?;
    assert!(
        ring.resign(
            grant.key_id(),
            grant.signing_input(),
            grant.expires_at(),
            111,
            &trust
        )
        .is_err()
    );
    assert!(ring.sign(b"x", 130, 111, &trust).is_err());
    Ok(())
}
#[test]
fn corrupt_snapshot_denies_and_debug_redacts_seed() -> TestResult {
    let mut ring = ring()?;
    ring.generate_next("key")?;
    let seed = ring.record("key")?.seed.as_ref().ok_or("seed")?;
    assert!(!format!("{ring:?}").contains(&format!("{:?}", seed.0)));
    let mut corrupt = ring.state.clone();
    corrupt.records[0].state = KeyState::Retired;
    let storage = Memory::default();
    storage.save(&serde_json::to_vec(&corrupt)?)?;
    assert!(matches!(
        Keyring::open(settings(), storage),
        Err(KeyError::InvalidState)
    ));
    let storage = Memory::default();
    storage.save(br#"{"version":1,"purpose":"fresh_entry","records":[{"state":"unknown"}]}"#)?;
    assert!(Keyring::open(settings(), storage).is_err());
    Ok(())
}
#[test]
fn uncertain_write_poisoning_never_returns_signature() -> TestResult {
    #[derive(Debug, Default)]
    struct Failing {
        memory: Memory,
        writes: std::cell::Cell<u8>,
    }
    impl KeyStorage for Failing {
        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyError> {
            self.memory.load()
        }
        fn save(&self, bytes: &[u8]) -> Result<(), KeyError> {
            self.writes.set(self.writes.get() + 1);
            self.memory.save(bytes)?;
            if self.writes.get() >= 4 {
                Err(KeyError::Unavailable)
            } else {
                Ok(())
            }
        }
    }
    let mut ring = Keyring::create(settings(), Failing::default())?;
    let mut trust = Trust::default();
    trust.publish(&ring.generate_next("key")?, 100)?;
    ring.activate("key", 107, &trust)?;
    assert!(matches!(
        ring.sign(b"header.payload", 120, 107, &trust),
        Err(KeyError::Unavailable)
    ));
    assert_eq!(ring.publication_keys(), Err(KeyError::Unavailable));
    assert_eq!(ring.generate_next("next"), Err(KeyError::Unavailable));
    Ok(())
}
#[test]
fn game_consumer_signature_fixture() -> TestResult {
    // SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
    // crates/gameplay/foundation/src/foundation/fnd04_verifier.rs,
    // signature_validation_only_uses_verifier_fixed_trust_keys. Test-only seed
    // and signing input; fixed signature independently checked with libsodium.
    let input = b"eyJhbGciOiJFZDI1NTE5Iiwia2lkIjoiZnJlc2giLCJ0eXAiOiJ4In0.e30";
    let public = hex("ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c")?;
    let signature = hex(
        "d098fe96ee1c09587a79abdf4dc7bdedf7be93446b76ebdc096c9790e2cf991dbb31f1695987980629e26e0b7ffa5768452f0a01d2d662af4de31a46c4e66507",
    )?;
    let record = Record {
        kid: "fresh".into(),
        public_key: public.as_slice().try_into()?,
        seed: Some(Seed([7; 32])),
        state: KeyState::Active,
        last_expiry: 0,
    };
    assert_eq!(sign(&record, input)?.as_slice(), signature);
    verify(record.public_key, input, &signature)?;
    assert!(verify(record.public_key, b"changed.payload", &signature).is_err());
    Ok(())
}
fn hex(value: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16))
        .collect()
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Result<Self, std::io::Error> {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("portal-keys-{}", Uuid::now_v7()));
        fs::create_dir(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        Ok(Self(path))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn encrypted_storage_roundtrip_tampering_and_permissions() -> TestResult {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = Directory::new()?;
    let wrapping = dir.0.join("wrapping");
    fs::write(&wrapping, [19; 32])?;
    fs::set_permissions(&wrapping, fs::Permissions::from_mode(0o600))?;
    let path = dir.0.join("custody");
    let storage = EncryptedFileStorage::from_key_file(&path, &wrapping)?;
    assert!(storage.load()?.is_none());
    assert!(EncryptedFileStorage::from_key_file(&path, &wrapping).is_err());
    storage.save(b"private test material")?;
    let first = fs::read(&path)?;
    assert!(!first.windows(7).any(|w| w == b"private"));
    assert_eq!(
        storage.load()?.ok_or("load")?.as_slice(),
        b"private test material"
    );
    storage.save(b"private test material")?;
    assert_ne!(fs::read(&path)?, first);
    let mut corrupt = fs::read(&path)?;
    corrupt[15] ^= 1;
    fs::write(&path, corrupt)?;
    assert!(matches!(storage.load(), Err(KeyError::InvalidState)));
    fs::set_permissions(&wrapping, fs::Permissions::from_mode(0o644))?;
    assert!(EncryptedFileStorage::from_key_file(&path, &wrapping).is_err());
    fs::set_permissions(&wrapping, fs::Permissions::from_mode(0o600))?;
    let link = dir.0.join("link");
    symlink(&wrapping, &link)?;
    assert!(EncryptedFileStorage::from_key_file(&path, &link).is_err());
    fs::remove_file(&path)?;
    symlink(&wrapping, &path)?;
    assert!(storage.save(b"x").is_err());
    assert!(storage.load().is_err());
    Ok(())
}

#[test]
fn production_custody_reopens_with_overlap_and_expiry_history() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let dir = Directory::new()?;
    let wrapping = dir.0.join("wrapping");
    fs::write(&wrapping, [21; 32])?;
    fs::set_permissions(&wrapping, fs::Permissions::from_mode(0o600))?;
    let path = dir.0.join("custody");
    let storage = EncryptedFileStorage::from_key_file(&path, &wrapping)?;
    assert!(Keyring::open(settings(), storage).is_err());
    let storage = EncryptedFileStorage::from_key_file(&path, &wrapping)?;
    let mut ring = Keyring::create(settings(), storage)?;
    let mut trust = Trust::default();
    trust.publish(&ring.generate_next("key")?, 100)?;
    ring.activate("key", 107, &trust)?;
    let original = ring.sign(b"header.payload", 130, 107, &trust)?;
    trust.publish(&ring.generate_next("next")?, 108)?;
    ring.activate("next", 115, &trust)?;
    assert_eq!(ring.retire_after("key")?, 137);
    drop(ring);
    let storage = EncryptedFileStorage::from_key_file(&path, &wrapping)?;
    let ring = Keyring::open(settings(), storage)?;
    assert_eq!(ring.state("key"), Some(KeyState::Retiring));
    assert_eq!(ring.state("next"), Some(KeyState::Active));
    assert_eq!(
        ring.resign(
            original.key_id(),
            original.signing_input(),
            original.expires_at(),
            116,
            &trust
        )?,
        original.signature()
    );
    assert_eq!(ring.retire_after("key")?, 137);
    Ok(())
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    directory: Directory,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP signing-key PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("keys_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105220101_t19c_signing_trust.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105151801_t19b_native_observations.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            directory: Directory::new()?,
        }))
    }
    fn registry(&self) -> Result<PublicTrustRegistry, KeyError> {
        PublicTrustRegistry::new(self.pool.clone(), settings(), self.directory.0.clone())
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
    async fn now(&self) -> Result<u64, sqlx::Error> {
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&self.pool)
                .await?;
        Ok(now as u64)
    }
}
#[tokio::test]
async fn postgres_publication_rotation_evidence_and_terminal_revocation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let registry = db.registry()?;
    let mut ring = ring()?;
    let old = ring.generate_next("old")?;
    registry.publish(&old).await?;
    registry.publish(&old).await?;
    sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()-interval '10 seconds'").execute(&db.pool).await?;
    let snapshot = registry.snapshot().await?;
    let now = db.now().await?;
    ring.activate("old", now, &snapshot)?;
    let signed = ring.sign(b"old.payload", now + 20, now, &snapshot)?;
    let new = ring.generate_next("new")?;
    registry.publish(&new).await?;
    assert!(matches!(
        ring.activate("new", now, &registry.snapshot().await?),
        Err(KeyError::TooEarly | KeyError::Untrusted)
    ));
    sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()-interval '10 seconds'").execute(&db.pool).await?;
    let snapshot = registry.snapshot().await?;
    let now = db.now().await?;
    ring.activate("new", now, &snapshot)?;
    assert_eq!(
        ring.resign(
            signed.key_id(),
            signed.signing_input(),
            signed.expires_at(),
            now,
            &snapshot
        )?,
        signed.signature()
    );
    let fresh = ring.sign(b"new.payload", now + 20, now, &snapshot)?;
    verify(
        trust::decode_key(&new.public_key)?,
        b"new.payload",
        fresh.signature(),
    )?;
    // Read through the real evidence producer, not a parallel public key route.
    use oteryn_internal_api::canonical::{self, Value};
    let value = canonical::decode(format!(r#"{{"version":1,"operation":"ReadFreshSigningTrustV1","issuer":"{ISSUER}","profile":"{PROFILE}","key_purpose":"fresh_entry","key_id":"old"}}"#).as_bytes(), 2048, 4)?;
    let request = crate::evidence::EvidenceRequest::decode(&value).map_err(|_| "request decode")?;
    let config = crate::evidence::Settings {
        enabled: true,
        activated: true,
        witness_directory: db.directory.0.clone(),
        source_authority: "platform".into(),
        fresh_account_purpose: "security".into(),
        fresh_account_scope: "admission".into(),
        fresh_key_purpose: "fresh_entry".into(),
        ..Default::default()
    };
    let response = crate::evidence::observe(&db.pool, &config, &request).await;
    let rendered = String::from_utf8(canonical::encode(&response)?)?;
    assert!(rendered.contains(&old.public_key), "{rendered}");
    assert!(rendered.contains("\"trusted\":true"));
    registry.revoke("old").await?;
    registry.revoke("old").await?;
    let snapshot = registry.snapshot().await?;
    assert!(
        ring.resign(
            signed.key_id(),
            signed.signing_input(),
            signed.expires_at(),
            db.now().await?,
            &snapshot
        )
        .is_err()
    );
    ring.retire("old", db.now().await?, &snapshot, true)?;
    assert!(registry.publish(&old).await.is_err());
    let response = crate::evidence::observe(&db.pool, &config, &request).await;
    assert!(String::from_utf8(canonical::encode(&response)?)?.contains("\"trusted\":false"));
    assert!(matches!(response, Value::Object(_)));
    db.close().await
}
#[tokio::test]
async fn postgres_refuses_third_rebinding_bad_input_and_stale_snapshot() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let registry = db.registry()?;
    let mut ring = ring()?;
    let a = ring.generate_next("a")?;
    registry.publish(&a).await?;
    let b = PublicKey {
        key_id: "b".into(),
        public_key: URL_SAFE_NO_PAD
            .encode(SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes()),
    };
    registry.publish(&b).await?;
    let c = PublicKey {
        key_id: "c".into(),
        public_key: URL_SAFE_NO_PAD
            .encode(SigningKey::from_bytes(&[13; 32]).verifying_key().to_bytes()),
    };
    assert_eq!(registry.publish(&c).await, Err(KeyError::InvalidState));
    let rebound = PublicKey {
        key_id: "a".into(),
        public_key: b.public_key.clone(),
    };
    assert!(registry.publish(&rebound).await.is_err());
    assert_eq!(
        registry
            .publish(&PublicKey {
                key_id: "bad\n".into(),
                public_key: a.public_key.clone()
            })
            .await,
        Err(KeyError::BadInput)
    );
    assert_eq!(
        registry
            .publish(&PublicKey {
                key_id: "bad".into(),
                public_key: "bad".into()
            })
            .await,
        Err(KeyError::BadInput)
    );
    let snapshot = registry.snapshot().await?;
    assert!(
        snapshot
            .trusted_key(ISSUER, PROFILE, "fresh_entry", "a", db.now().await? + 6)
            .is_err()
    );
    registry.revoke("a").await?;
    assert!(registry.publish(&a).await.is_err());
    registry.publish(&c).await?;
    db.close().await
}
#[tokio::test]
async fn postgres_witness_ahead_restore_and_missing_state_deny() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let registry = db.registry()?;
    let mut ring = ring()?;
    let key = ring.generate_next("key")?;
    registry.publish(&key).await?;
    let ns = crate::evidence::trust_state_namespace(ISSUER, PROFILE, "fresh_entry")
        .map_err(|_| "namespace")?;
    let revision: i64 =
        sqlx::query("SELECT issuer_revision FROM native_game_signing_trust_profiles")
            .fetch_one(&db.pool)
            .await?
            .try_get("issuer_revision")?;
    let witness =
        crate::evidence::Witness::lock(&db.directory.0, &ns).map_err(|_| "witness lock")?;
    witness
        .advance(&ns, revision + 1)
        .map_err(|_| "witness advance")?;
    drop(witness);
    assert!(registry.snapshot().await.is_err());
    assert!(registry.publish(&key).await.is_err());
    assert!(registry.revoke("key").await.is_err());
    sqlx::query("DELETE FROM native_game_signing_trust_key_versions")
        .execute(&db.pool)
        .await?;
    sqlx::query("DELETE FROM native_game_signing_trust_profiles")
        .execute(&db.pool)
        .await?;
    assert!(registry.publish(&key).await.is_err());
    db.close().await
}
