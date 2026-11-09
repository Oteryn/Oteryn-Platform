//! Local operator publication. OS access and database credentials are operational prerequisites.
use crate::{AccountState, Channel, Downloads, Error, FeatureFlags, PublishAccess, Upload};
use async_trait::async_trait;
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::ExitCode,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct Command {
    pub channel: Channel,
    pub actor: Uuid,
    pub dir: PathBuf,
}
impl Command {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Error> {
        let mut args = args.into_iter();
        let (mut channel, mut actor, mut dir) = (None, None, None);
        while let Some(arg) = args.next() {
            let value = args.next().ok_or(Error::Invalid("command arguments"))?;
            match arg.as_str() {
                "--channel" if channel.is_none() => channel = Some(Channel::parse(&value)?),
                "--actor" if actor.is_none() => {
                    actor = Some(Uuid::parse_str(&value).map_err(|_| Error::Invalid("actor UUID"))?)
                }
                "--dir" if dir.is_none() => dir = Some(PathBuf::from(value)),
                _ => return Err(Error::Invalid("command arguments")),
            }
        }
        let actor = actor.ok_or(Error::Invalid("actor UUID"))?;
        if actor.get_version_num() != 7 {
            return Err(Error::Invalid("actor UUID"));
        }
        Ok(Self {
            channel: channel.ok_or(Error::Invalid("channel"))?,
            actor,
            dir: dir.ok_or(Error::Invalid("bundle directory"))?,
        })
    }

    pub async fn execute(
        &self,
        pool: PgPool,
        flags: FeatureFlags,
        root: &Path,
    ) -> Result<Uuid, Error> {
        if !flags.downloads {
            return Err(Error::Disabled);
        }
        let root = read_bounded(root, 1024 * 1024)?;
        let mut metadata = BTreeMap::new();
        let mut total = 0;
        for entry in std::fs::read_dir(self.dir.join("metadata"))
            .map_err(|_| Error::Invalid("metadata directory"))?
        {
            let entry = entry.map_err(|_| Error::Invalid("metadata file"))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| Error::Invalid("metadata filename"))?;
            if metadata.len() >= 32 {
                return Err(Error::Invalid("metadata count"));
            }
            let bytes = read_bounded(&entry.path(), 1024 * 1024)?;
            total += bytes.len();
            if total > 8 * 1024 * 1024 {
                return Err(Error::Invalid("bundle size"));
            }
            metadata.insert(name, bytes);
        }
        let upload = Upload {
            channel: self.channel,
            metadata,
            policy: read_bounded(&self.dir.join("policy-v1.json"), 256 * 1024)?,
        };
        let access = DatabaseAccess(pool.clone());
        Downloads::new(pool, flags, BTreeMap::from([(self.channel, root)]))
            .publish(&access, self.actor, upload)
            .await
    }
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    let info = std::fs::symlink_metadata(path).map_err(|_| Error::Invalid("public file"))?;
    if !info.is_file() || info.len() > limit as u64 {
        return Err(Error::Invalid("public file size or type"));
    }
    let file = std::fs::File::open(path).map_err(|_| Error::Invalid("public file"))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Invalid("public file"))?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(Error::Invalid("public file size"));
    }
    Ok(bytes)
}
struct DatabaseAccess(PgPool);
#[async_trait]
impl PublishAccess for DatabaseAccess {
    async fn account_state(&self, actor: Uuid) -> AccountState {
        match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id = $1")
            .bind(actor)
            .fetch_optional(&self.0)
            .await
        {
            Ok(Some(status)) if status == "active" => AccountState::Active,
            _ => AccountState::Unknown,
        }
    }
    async fn may_publish(&self, actor: Uuid, _: Channel) -> Option<bool> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp ON rp.role_id = ar.role_id WHERE ar.account_id = $1 AND rp.permission = 'downloads.manage')")
            .bind(actor).fetch_one(&self.0).await.ok()
    }
}
pub async fn run() -> ExitCode {
    match configured().await {
        Ok(id) => {
            println!("Published {id}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Publication refused: {error}");
            ExitCode::FAILURE
        }
    }
}
async fn configured() -> Result<Uuid, Error> {
    let command = Command::parse(std::env::args().skip(2))?;
    let flags = FeatureFlags {
        downloads: std::env::var("OTERYN_DOWNLOADS_ENABLED").is_ok_and(|v| v == "true"),
    };
    if !flags.downloads {
        return Err(Error::Disabled);
    }
    let key = match command.channel {
        Channel::Stable => "OTERYN_DOWNLOADS_ROOT_STABLE",
        Channel::Beta => "OTERYN_DOWNLOADS_ROOT_BETA",
    };
    let root = std::env::var_os(key).ok_or(Error::Invalid("pinned public root configuration"))?;
    let url = std::env::var("OTERYN_DATABASE_URL")
        .map_err(|_| Error::Invalid("database configuration"))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;
    command.execute(pool, flags, Path::new(&root)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_files_are_bounded() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("public-root-{}", Uuid::now_v7()));
        std::fs::write(&path, b"{}")?;
        assert_eq!(read_bounded(&path, 2)?, b"{}");
        assert!(matches!(read_bounded(&path, 1), Err(Error::Invalid(_))));
        std::fs::write(&path, b"")?;
        assert!(matches!(read_bounded(&path, 2), Err(Error::Invalid(_))));
        std::fs::remove_file(path)?;
        Ok(())
    }
}
