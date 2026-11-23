//! Provision one real recovery signer for the disposable authorization harness.
use oteryn_gameauth::keys::{
    EncryptedFileStorage, KeyError, Keyring, PublicTrustRegistry, RECOVERY_KEY_PURPOSE, Scope,
    Settings,
};
use sqlx::{PgPool, postgres::PgConnectOptions};
use std::{path::PathBuf, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("OTERYN_ENV").as_deref() != Ok("testing") {
        return Err("expected the local testing harness".into());
    }
    let options: PgConnectOptions = std::env::var("OTERYN_DATABASE_URL")?
        .parse()
        .map_err(|_| "invalid harness database URL")?;
    if !matches!(options.get_host(), "127.0.0.1" | "localhost" | "::1")
        || options.get_socket().is_some()
        || !options
            .get_database()
            .is_some_and(|name| name.starts_with("portal_e2e_"))
    {
        return Err("expected the disposable harness database".into());
    }
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("missing private directory")?,
    );
    let witness = PathBuf::from(
        std::env::var_os("GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY")
            .ok_or("expected the harness witness directory")?,
    );
    let settings = Settings {
        enabled: true,
        key_purpose: RECOVERY_KEY_PURPOSE.into(),
        clock_uncertainty_seconds: 0,
    };
    let db = PgPool::connect_with(options).await?;
    let registry =
        PublicTrustRegistry::new_scoped(db.clone(), settings.clone(), Scope::Recovery, witness)?;
    let wrapping = directory.join("recovery-wrapping.key");
    let custody = directory.join("recovery-custody.bin");
    if !wrapping.exists() {
        EncryptedFileStorage::create_key_file(&wrapping)?;
    }
    let exists = custody.exists();
    let storage = EncryptedFileStorage::from_key_file(custody, &wrapping)?;
    let mut keys = if exists {
        Keyring::open_scoped(settings, Scope::Recovery, storage)?
    } else {
        Keyring::create_scoped(settings, Scope::Recovery, storage)?
    };
    let key_id = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid fixture directory")?;
    if keys.active_key_id() == Some(key_id) {
        return Ok(());
    }
    let existing = keys
        .publication_keys()?
        .into_iter()
        .find(|key| key.key_id == key_id);
    let key = match existing {
        Some(key) => key,
        None => keys.generate_next(key_id)?,
    };
    registry.publish(&key).await?;
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut poll = tokio::time::interval(Duration::from_millis(100));
        loop {
            poll.tick().await;
            let snapshot = registry.snapshot().await?;
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&db)
                    .await?;
            match keys.activate(key_id, u64::try_from(now)?, &snapshot) {
                Ok(()) => return Ok::<(), Box<dyn std::error::Error>>(()),
                Err(KeyError::TooEarly | KeyError::Untrusted) => {}
                Err(error) => return Err(error.into()),
            }
        }
    })
    .await??;
    Ok(())
}
