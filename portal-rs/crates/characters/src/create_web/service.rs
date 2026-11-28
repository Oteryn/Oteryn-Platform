use super::names;
use crate::commands::{self, AccountState, AccountStateReader, Commands, Intent, Sex, Vocation};
use crate::{lcfa, overview};
use oteryn_identity::accounts::Accounts;
use sqlx::{Acquire, PgConnection, PgPool, Row};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub commands: commands::Settings,
}
impl Settings {
    pub fn configured() -> Self {
        let flag = |key| std::env::var(key).as_deref() == Ok("true");
        Self {
            enabled: flag("OTERYN_CHARACTER_CREATE_WEB_ENABLED"),
            commands: commands::Settings {
                enabled: flag("OTERYN_CHARACTER_COMMANDS_ENABLED"),
                create_enabled: flag("OTERYN_CHARACTER_COMMAND_CREATE_ENABLED"),
                source_authority: std::env::var("OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY")
                    .unwrap_or_default(),
                ..Default::default()
            },
        }
    }

    pub(crate) fn mounted(&self) -> bool {
        self.enabled && self.commands.enabled && self.commands.create_enabled
    }

    pub fn available(&self) -> bool {
        self.mounted() && self.commands.available()
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid creation input")]
    Invalid,
    #[error("reserved name")]
    Reserved,
    #[error("duplicate character name")]
    Duplicate,
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
    /// Required: there is no default sex or vocation (#1937).
    pub sex: Sex,
    pub vocation: Vocation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Reconciling,
    Created(Uuid),
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
        let mut conn = self.pool.acquire().await?;
        self.eligible_in(&mut conn, account).await
    }
    async fn eligible_in(&self, conn: &mut PgConnection, account: Uuid) -> Result<(), Error> {
        if account.get_version_num() != 7 {
            return Err(Error::Denied);
        }
        let current = Accounts::find_by_id_in(conn, account).await?;
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
        let mut conn = self.pool.acquire().await?;
        self.limit_in(&mut conn, account, surface).await
    }
    async fn limit_in(
        &self,
        conn: &mut PgConnection,
        account: Uuid,
        surface: &str,
    ) -> Result<(), Error> {
        let maximum = if surface == "create" { 5 } else { 60 };
        let count: i32 = sqlx::query_scalar("INSERT INTO character_create_web_limits (account_id,surface,window_start,requests) VALUES ($1,$2,clock_timestamp(),1) ON CONFLICT (account_id,surface) DO UPDATE SET window_start=CASE WHEN character_create_web_limits.window_start <= clock_timestamp()-interval '1 minute' THEN clock_timestamp() ELSE character_create_web_limits.window_start END, requests=CASE WHEN character_create_web_limits.window_start <= clock_timestamp()-interval '1 minute' THEN 1 ELSE LEAST(character_create_web_limits.requests+1,1000000) END RETURNING requests")
            .bind(account).bind(surface).fetch_one(conn).await?;
        if count > maximum {
            return Err(Error::Limited);
        }
        Ok(())
    }
    pub async fn worlds(&self) -> Result<Vec<World>, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        let mut conn = self.pool.acquire().await?;
        self.worlds_in(&mut conn).await
    }
    async fn worlds_in(&self, conn: &mut PgConnection) -> Result<Vec<World>, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        // Registry metadata is advisory. Game rechecks world acceptance at execution.
        sqlx::query("SELECT id,name,pvp_type,location FROM world_status ORDER BY name,id LIMIT 256")
            .fetch_all(conn)
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
        let mut conn = self.pool.acquire().await?;
        self.reserved_in(&mut conn, name).await
    }
    async fn reserved_in(&self, conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reserved_names WHERE name=lower($1) AND expires_at>clock_timestamp())")
            .bind(name).fetch_one(conn).await?)
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
        let roster = self.current_roster(account).await;
        // Serialize with identity changes. Keep this lock until the command is durable.
        // All checks and command issuance share this transaction.
        let mut tx = self.pool.begin().await?;
        let found: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR NO KEY UPDATE")
                .bind(account)
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_none() {
            return Err(Error::Denied);
        }
        self.eligible_in(&mut tx, account).await?;
        let intent = Intent::Create {
            account_id: account,
            world_id: input.world,
            name: name.clone(),
            sex: input.sex,
            vocation: input.vocation,
        };
        let key = format!("web-create:{account}:{}", input.id);
        if let Some(status) = self
            .commands
            .replay_in(&mut tx, input.id, &key, &intent)
            .await?
        {
            return Ok(status.operation_id);
        }
        self.limit_in(&mut tx, account, "create").await?;
        // Issuance may fail, but an admitted attempt must remain charged. A
        // savepoint keeps admission durable without acquiring a nested connection.
        let mut issuance = tx.begin().await?;
        let result = async {
            let tx = &mut issuance;
            if !self
                .worlds_in(tx)
                .await?
                .iter()
                .any(|world| world.id == input.world)
            {
                return Err(Error::Invalid);
            }
            if self.reserved_in(tx, &name).await? {
                return Err(Error::Reserved);
            }
            let duplicate_request: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM character_commands
                    WHERE account_id=$1 AND command='CreateCharacter' AND state='PENDING'
                    AND binding_json::jsonb->>'target_world_id'=$2
                    AND binding_json::jsonb->>'requested_name'=$3
                )",
            )
            .bind(account)
            .bind(input.world.to_string())
            .bind(&name)
            .fetch_one(&mut **tx)
            .await?;
            if duplicate_request
                || roster.iter().flatten().any(|character| {
                    character.world_id.as_uuid() == input.world
                        && character.name.eq_ignore_ascii_case(&name)
                })
            {
                return Err(Error::Duplicate);
            }
            let status = self
                .commands
                .enqueue_in(tx, &Eligible(account), input.id, &key, intent)
                .await?;
            Ok(status.operation_id)
        }
        .await;
        if result.is_ok() {
            issuance.commit().await?;
        } else {
            issuance.rollback().await?;
        }
        tx.commit().await?;
        result
    }
    pub(super) async fn current_roster(
        &self,
        account: Uuid,
    ) -> Option<Vec<lcfa::CharacterSummary>> {
        roster(&self.pool, account, overview::configured_projection()).await
    }

    pub async fn status(&self, account: Uuid, operation: Uuid) -> Result<Outcome, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        self.eligible(account).await?;
        self.limit(account, "read").await?;
        let (receipt, current): (Option<String>, bool) = sqlx::query_as(
            r#"SELECT receipt_json, starts_with(intent_json,'{"contract_version":2,')
             FROM character_commands
             WHERE id=$1 AND account_id=$2 AND command='CreateCharacter'"#,
        )
        .bind(operation)
        .bind(account)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::NotFound)?;
        let Some(receipt) = receipt else {
            return Ok(if current {
                Outcome::Pending
            } else {
                Outcome::Reconciling
            });
        };
        let receipt: serde_json::Value =
            serde_json::from_str(&receipt).map_err(|_| Error::Unavailable)?;
        match (receipt["outcome"].as_str(), receipt["result_code"].as_str()) {
            (Some("COMMITTED"), Some("CHAR_CMD_OK")) => {
                let id = receipt["character_id"]
                    .as_str()
                    .and_then(|id| lcfa::CharacterId::parse(id).ok())
                    .ok_or(Error::Unavailable)?;
                Ok(Outcome::Created(id.as_uuid()))
            }
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

/// Uses the same live, account-scoped projection as the account overview.
async fn roster(
    pool: &PgPool,
    account: Uuid,
    settings: lcfa::Settings,
) -> Option<Vec<lcfa::CharacterSummary>> {
    let account = lcfa::AccountId::parse(&account.to_string()).ok()?;
    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()?;
    let projection = lcfa::Projection::new(
        pool.clone(),
        settings,
        Arc::new(overview::Accounts(pool.clone())),
    );
    projection
        .list_characters(account, now)
        .await
        .ok()
        .filter(|view| view.state == lcfa::ReadState::Ready)
        .map(|view| view.characters)
}

pub(crate) async fn recent_requests(
    pool: &PgPool,
    account: Uuid,
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, intent_json::jsonb->>'requested_name'
         FROM character_commands WHERE account_id=$1 AND command='CreateCharacter'
         ORDER BY source_revision DESC LIMIT 10",
    )
    .bind(account)
    .fetch_all(pool)
    .await
}
