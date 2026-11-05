use super::names;
use crate::commands::{self, AccountState, AccountStateReader, Commands, Intent};
use oteryn_identity::accounts::Accounts;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub commands: commands::Settings,
}
impl Settings {
    pub fn available(&self) -> bool {
        self.enabled
            && self.commands.enabled
            && self.commands.create_enabled
            && !self.commands.source_authority.is_empty()
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid creation input")]
    Invalid,
    #[error("reserved name")]
    Reserved,
    #[error("creation unavailable")]
    Unavailable,
    #[error("account denied")]
    Denied,
    #[error("operation not found")]
    NotFound,
    #[error("rate limit reached")]
    Limited,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl From<commands::Error> for Error {
    fn from(error: commands::Error) -> Self {
        match error {
            commands::Error::BadInput | commands::Error::Conflict => Self::Invalid,
            commands::Error::Denied => Self::Denied,
            commands::Error::NotFound => Self::NotFound,
            _ => Self::Unavailable,
        }
    }
}
#[derive(Debug, Clone)]
pub struct World {
    pub id: Uuid,
    pub name: String,
    pub pvp: String,
    pub location: String,
}
#[derive(Debug, Clone)]
pub struct Submission {
    pub id: Uuid,
    pub world: Uuid,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Created,
    Rejected(&'static str),
}
#[derive(Clone, Debug)]
pub struct Creation {
    pool: PgPool,
    settings: Settings,
    commands: Commands,
}
/// Eligibility is deliberately separate from projections, which never authorize mutations.
struct Eligible(Uuid);
impl AccountStateReader for Eligible {
    async fn state(&self, account: Uuid) -> AccountState {
        if self.0 == account {
            AccountState::Eligible
        } else {
            AccountState::Unknown
        }
    }
}
impl Creation {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self {
            commands: Commands::new(pool.clone(), settings.commands.clone()),
            pool,
            settings,
        }
    }
    pub fn available(&self) -> bool {
        self.settings.available()
    }
    pub async fn eligible(&self, account: Uuid) -> Result<(), Error> {
        if account.get_version_num() != 7 {
            return Err(Error::Denied);
        }
        let current = Accounts {
            pool: self.pool.clone(),
        }
        .find_by_id(account)
        .await?;
        match current {
            Some(current)
                if current.status == "active" || current.status == "pending_verification" =>
            {
                Ok(())
            }
            _ => Err(Error::Denied),
        }
    }
    pub async fn limit(&self, account: Uuid, surface: &str) -> Result<(), Error> {
        let maximum = if surface == "create" { 5 } else { 60 };
        let count: i32 = sqlx::query_scalar("INSERT INTO character_create_web_limits (account_id,surface,window_start,requests) VALUES ($1,$2,clock_timestamp(),1) ON CONFLICT (account_id,surface) DO UPDATE SET window_start=CASE WHEN character_create_web_limits.window_start <= clock_timestamp()-interval '1 minute' THEN clock_timestamp() ELSE character_create_web_limits.window_start END, requests=CASE WHEN character_create_web_limits.window_start <= clock_timestamp()-interval '1 minute' THEN 1 ELSE LEAST(character_create_web_limits.requests+1,1000000) END RETURNING requests")
            .bind(account).bind(surface).fetch_one(&self.pool).await?;
        if count > maximum {
            return Err(Error::Limited);
        }
        Ok(())
    }
    pub async fn worlds(&self) -> Result<Vec<World>, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        // Registry metadata is advisory. Game rechecks world acceptance at execution.
        sqlx::query("SELECT id,name,pvp_type,location FROM world_status ORDER BY name,id LIMIT 256")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                Ok(World {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    pvp: row
                        .try_get::<Option<String>, _>("pvp_type")?
                        .unwrap_or_default(),
                    location: row
                        .try_get::<Option<String>, _>("location")?
                        .unwrap_or_default(),
                })
            })
            .collect()
    }
    pub async fn suggest(&self) -> Result<String, Error> {
        for _ in 0..16 {
            let candidate = names::suggestion();
            if !self.reserved(&candidate).await? {
                return Ok(candidate);
            }
        }
        Err(Error::Unavailable)
    }
    async fn reserved(&self, name: &str) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reserved_names WHERE name=lower($1) AND expires_at>clock_timestamp())")
            .bind(name).fetch_one(&self.pool).await?)
    }
    pub async fn submit(&self, account: Uuid, input: Submission) -> Result<Uuid, Error> {
        let name = names::canonicalize(&input.name).map_err(|e| match e {
            names::NameError::Invalid => Error::Invalid,
            names::NameError::Reserved => Error::Reserved,
        })?;
        if input.id.get_version_num() != 7 || input.world.get_version_num() != 7 {
            return Err(Error::Invalid);
        }
        if !self.available() {
            return Err(Error::Unavailable);
        }
        self.eligible(account).await?;
        self.limit(account, "create").await?;
        // Serialize with identity changes. Keep this lock until the command is durable.
        // The public command API owns its transaction and idempotency lock.
        let mut tx = self.pool.begin().await?;
        let found: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_none() {
            return Err(Error::Denied);
        }
        self.eligible(account).await?;
        if !self
            .worlds()
            .await?
            .iter()
            .any(|world| world.id == input.world)
        {
            return Err(Error::Invalid);
        }
        if self.reserved(&name).await? {
            return Err(Error::Reserved);
        }
        let status = self
            .commands
            .enqueue(
                &Eligible(account),
                input.id,
                &format!("web-create:{account}:{}", input.id),
                Intent::Create {
                    account_id: account,
                    world_id: input.world,
                    name,
                },
            )
            .await?;
        tx.commit().await?;
        Ok(status.operation_id)
    }
    pub async fn status(&self, account: Uuid, operation: Uuid) -> Result<Outcome, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        self.eligible(account).await?;
        self.limit(account, "read").await?;
        let status = self.commands.status(operation).await?;
        let intent: serde_json::Value =
            serde_json::from_str(&status.intent_json).map_err(|_| Error::Unavailable)?;
        if status.command != commands::CREATE
            || intent["account_id"].as_str() != Some(account.to_string().as_str())
        {
            return Err(Error::NotFound);
        }
        let Some(receipt) = status.receipt_json else {
            return Ok(Outcome::Pending);
        };
        let receipt: serde_json::Value =
            serde_json::from_str(&receipt).map_err(|_| Error::Unavailable)?;
        match (receipt["outcome"].as_str(), receipt["result_code"].as_str()) {
            (Some("COMMITTED"), Some("CHAR_CMD_OK")) => Ok(Outcome::Created),
            (Some("REJECTED"), Some(code)) => Ok(Outcome::Rejected(match code {
                "CHAR_CMD_NAME_INVALID" => "name-invalid",
                "CHAR_CMD_NAME_UNAVAILABLE" => "name-unavailable",
                "CHAR_CMD_WORLD_NOT_ACCEPTED" => "world-rejected",
                "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT" => "limit",
                "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE" => "denied",
                "CHAR_CMD_INTENT_EXPIRED" => "expired",
                _ => "rejected",
            })),
            _ => Err(Error::Unavailable),
        }
    }
}
