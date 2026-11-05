use super::KeyError;
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
const AAD: &[u8] = b"oteryn-fresh-admission-custody-v1";
const MAX_BYTES: u64 = 1024 * 1024;

/// Custody boundary: implementations encrypt and authenticate persisted bytes,
/// atomically replace snapshots, and never log plaintext. Failure is distinct
/// from genuinely absent storage. Test implementations stay in memory.
pub trait KeyStorage: fmt::Debug {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyError>;
    fn save(&self, plaintext: &[u8]) -> Result<(), KeyError>;
}
/// AES-256-GCM snapshot with a fresh random nonce per write. The wrapping key is
/// exactly 32 raw bytes in an externally configured owner-only regular file.
/// Paths must be outside the repository/database/backup/image boundary. Use a
/// dedicated owner-only directory and a single issuer writer. Provision and
/// rotate the wrapping key through the deployment's secret custody process.
pub struct EncryptedFileStorage {
    path: PathBuf,
    wrapping_key: Zeroizing<[u8; 32]>,
    _lock: File,
}
impl fmt::Debug for EncryptedFileStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EncryptedFileStorage([redacted])")
    }
}
#[cfg(unix)]
impl EncryptedFileStorage {
    pub fn from_key_file(path: impl Into<PathBuf>, key_path: &Path) -> Result<Self, KeyError> {
        let path = path.into();
        let parent = path.parent().ok_or(KeyError::BadInput)?;
        if !path.is_absolute() || path == key_path || !key_path.is_absolute() {
            return Err(KeyError::BadInput);
        }
        private_metadata(parent, true)?;
        let mut key_file = private_file(key_path)?;
        if key_file
            .metadata()
            .map_err(|_| KeyError::Unavailable)?
            .len()
            != 32
        {
            return Err(KeyError::BadInput);
        }
        let mut wrapping_key = Zeroizing::new([0; 32]);
        key_file
            .read_exact(&mut *wrapping_key)
            .map_err(|_| KeyError::Unavailable)?;
        use std::os::unix::fs::OpenOptionsExt;
        let name = path
            .file_name()
            .ok_or(KeyError::BadInput)?
            .to_str()
            .ok_or(KeyError::BadInput)?;
        let lock_path = parent.join(format!(".{name}.lock"));
        if fs::symlink_metadata(&lock_path).is_ok() {
            private_metadata(&lock_path, false)?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|_| KeyError::Unavailable)?;
        private_metadata(&lock_path, false)?;
        lock.try_lock().map_err(|_| KeyError::Unavailable)?;
        Ok(Self {
            path,
            wrapping_key,
            _lock: lock,
        })
    }
    fn cipher(&self) -> Result<Aes256Gcm, KeyError> {
        Aes256Gcm::new_from_slice(&*self.wrapping_key).map_err(|_| KeyError::Unavailable)
    }
}
#[cfg(unix)]
impl KeyStorage for EncryptedFileStorage {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyError> {
        private_metadata(self.path.parent().ok_or(KeyError::BadInput)?, true)?;
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(KeyError::Unavailable),
            Ok(_) => {}
        }
        let file = private_file(&self.path)?;
        let size = file.metadata().map_err(|_| KeyError::Unavailable)?.len();
        if !(28..=MAX_BYTES).contains(&size) {
            return Err(KeyError::InvalidState);
        }
        let mut envelope = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut envelope)
            .map_err(|_| KeyError::Unavailable)?;
        if envelope.len() as u64 != size {
            return Err(KeyError::InvalidState);
        }
        let plaintext = self
            .cipher()?
            .decrypt(
                &Nonce::from(
                    <[u8; 12]>::try_from(&envelope[..12]).map_err(|_| KeyError::InvalidState)?,
                ),
                Payload {
                    msg: &envelope[12..],
                    aad: AAD,
                },
            )
            .map_err(|_| KeyError::InvalidState)?;
        Ok(Some(Zeroizing::new(plaintext)))
    }
    fn save(&self, plaintext: &[u8]) -> Result<(), KeyError> {
        use std::os::unix::fs::OpenOptionsExt;
        if plaintext.len() as u64 + 28 > MAX_BYTES {
            return Err(KeyError::BadInput);
        }
        let parent = self.path.parent().ok_or(KeyError::BadInput)?;
        private_metadata(parent, true)?;
        if fs::symlink_metadata(&self.path).is_ok() {
            private_metadata(&self.path, false)?;
        }
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| KeyError::Unavailable)?;
        let encrypted = self
            .cipher()?
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plaintext,
                    aad: AAD,
                },
            )
            .map_err(|_| KeyError::Unavailable)?;
        let temporary = parent.join(format!(".custody-{}", uuid::Uuid::now_v7()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|_| KeyError::Unavailable)?;
            file.write_all(&nonce)
                .and_then(|()| file.write_all(&encrypted))
                .and_then(|()| file.sync_all())
                .map_err(|_| KeyError::Unavailable)?;
            fs::rename(&temporary, &self.path).map_err(|_| KeyError::Unavailable)?;
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| KeyError::Unavailable)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}
#[cfg(unix)]
fn private_metadata(path: &Path, directory: bool) -> Result<fs::Metadata, KeyError> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path).map_err(|_| KeyError::Unavailable)?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
        || metadata.mode() & 0o077 != 0
        || metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(KeyError::Unavailable);
    }
    Ok(metadata)
}
#[cfg(unix)]
fn private_file(path: &Path) -> Result<File, KeyError> {
    use std::os::unix::fs::MetadataExt;
    let before = private_metadata(path, false)?;
    let file = File::open(path).map_err(|_| KeyError::Unavailable)?;
    let after = file.metadata().map_err(|_| KeyError::Unavailable)?;
    if before.dev() != after.dev()
        || before.ino() != after.ino()
        || !after.is_file()
        || after.mode() & 0o077 != 0
        || after.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(KeyError::Unavailable);
    }
    Ok(file)
}
