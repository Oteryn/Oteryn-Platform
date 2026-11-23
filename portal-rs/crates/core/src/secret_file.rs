//! Admission checks for files that carry credentials.

use std::{fs::File, io, path::Path};

/// A credential file could not be opened under the file policy.
#[derive(Debug, thiserror::Error)]
#[error("{key}: {source}")]
pub struct SecretFileError {
    key: &'static str,
    source: io::Error,
}

impl SecretFileError {
    /// Identifies the I/O failure without exposing the configured path.
    pub fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }
}

/// Opens a regular credential file owned by the effective process UID with mode 0400 or 0600.
/// Callers must read the returned handle, which is the same descriptor whose metadata was checked.
pub fn open(key: &'static str, path: impl AsRef<Path>) -> Result<File, SecretFileError> {
    open_private(path.as_ref()).map_err(|source| SecretFileError { key, source })
}

#[cfg(unix)]
fn open_private(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};

    let file = File::from(rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )?);
    validate_metadata(&file.metadata()?, rustix::process::geteuid().as_raw())?;
    Ok(file)
}

#[cfg(unix)]
fn validate_metadata(metadata: &std::fs::Metadata, uid: u32) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    if !metadata.is_file()
        || metadata.uid() != uid
        || !matches!(metadata.mode() & 0o7777, 0o400 | 0o600)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "expected a regular file owned by the process with mode 0400 or 0600",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn open_private(_: &Path) -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "credential file ownership and mode cannot be verified on this platform",
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::{PermissionsExt, symlink},
    };

    const KEY: &str = "TEST_CREDENTIAL_FILE";

    #[test]
    fn credential_file_modes() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(b"fixture bytes")?;
        for mode in [0o400, 0o600] {
            fs::set_permissions(file.path(), fs::Permissions::from_mode(mode))?;
            let mut bytes = Vec::new();
            open(KEY, file.path())?.read_to_end(&mut bytes)?;
            assert_eq!(bytes, b"fixture bytes");
        }
        for mode in [0o640, 0o644, 0o666, 0o700, 0o4600] {
            fs::set_permissions(file.path(), fs::Permissions::from_mode(mode))?;
            let error = open(KEY, file.path()).err().ok_or("mode was admitted")?;
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            let message = error.to_string();
            assert!(message.contains(KEY));
            assert!(!message.contains("fixture bytes"));
        }
        Ok(())
    }

    #[test]
    fn credential_file_path_types() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = tempfile::NamedTempFile::new_in(&dir)?;
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o600))?;
        let link = dir.path().join("link");
        symlink(file.path(), &link)?;
        let link_error = open(KEY, link).err().ok_or("symlink was admitted")?;
        assert!(link_error.to_string().contains(KEY));
        let directory_error = open(KEY, dir.path())
            .err()
            .ok_or("directory was admitted")?;
        assert_eq!(directory_error.kind(), io::ErrorKind::PermissionDenied);
        let missing = open(KEY, dir.path().join("absent"))
            .err()
            .ok_or("missing file was admitted")?;
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert!(missing.to_string().contains(KEY));
        Ok(())
    }

    #[test]
    fn credential_file_owner() -> io::Result<()> {
        let file = tempfile::NamedTempFile::new()?;
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o600))?;
        let uid = rustix::process::geteuid().as_raw();
        assert!(matches!(
            validate_metadata(&file.as_file().metadata()?, uid ^ 1),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied
        ));
        Ok(())
    }
}
