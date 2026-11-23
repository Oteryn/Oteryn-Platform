#[cfg(test)]
#[path = "native_devices_tests.rs"]
mod tests;

// Device families are Platform authorization records. Account and session
// locks keep browser revocation ordered with enrollment and ticket redemption.
use sqlx::PgPool;
use uuid::Uuid;

const PAGE_SIZE: usize = 25;

#[derive(Debug, Clone, Copy, Eq, PartialEq, thiserror::Error)]
#[error("invalid native device family identifier")]
pub struct InvalidFamily;

pub fn parse_family(value: &str) -> Result<Uuid, InvalidFamily> {
    let id = Uuid::parse_str(value).map_err(|_| InvalidFamily)?;
    if id.is_nil() || id.hyphenated().to_string() != value {
        return Err(InvalidFamily);
    }
    Ok(id)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceState {
    Current,
    Expired,
    Revoked,
    Inactive,
}

impl DeviceState {
    pub fn code(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::Inactive => "inactive",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Current => "native-devices-current",
            Self::Expired => "native-devices-expired",
            Self::Revoked => "native-devices-revoked",
            Self::Inactive => "native-devices-inactive",
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct Device {
    pub id: Uuid,
    pub bound: bool,
    pub enrolled: String,
    pub expires: String,
    revoked: bool,
    expired: bool,
    current_generation: bool,
}

impl Device {
    pub fn state(&self) -> DeviceState {
        if self.revoked {
            DeviceState::Revoked
        } else if self.expired {
            DeviceState::Expired
        } else if self.current_generation {
            DeviceState::Current
        } else {
            DeviceState::Inactive
        }
    }

    pub fn can_revoke(&self) -> bool {
        !self.revoked
    }
}

pub async fn list(
    pool: &PgPool,
    owner: Uuid,
    before: Option<Uuid>,
) -> Result<(Vec<Device>, Option<Uuid>), sqlx::Error> {
    let mut rows: Vec<Device> = sqlx::query_as(
        "SELECT d.id, d.device_public_key IS NOT NULL AS bound,
                to_char(d.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS enrolled,
                to_char(to_timestamp(d.idle_expires_at) AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS expires,
                d.revoked_at IS NOT NULL AS revoked,
                d.idle_expires_at <= extract(epoch FROM clock_timestamp()) AS expired,
                d.game_generation = a.game_auth_generation
                    AND d.native_generation = a.native_security_generation AS current_generation
         FROM device_sessions d JOIN accounts a ON a.id=d.account_id
         WHERE d.account_id=$1 AND d.persistence_class='remembered'
           AND a.status='active' AND a.email_verified_at IS NOT NULL
           AND ($2::uuid IS NULL OR d.id < $2)
         ORDER BY d.id DESC LIMIT $3",
    )
    .bind(owner)
    .bind(before)
    .bind((PAGE_SIZE + 1) as i64)
    .fetch_all(pool)
    .await?;
    let more = rows.len() > PAGE_SIZE;
    rows.truncate(PAGE_SIZE);
    let next = rows.last().filter(|_| more).map(|row| row.id);
    Ok((rows, next))
}

#[derive(Debug, Eq, PartialEq)]
pub enum Revocation {
    Denied,
    Confirm,
    Missing,
    Revoked,
}

pub async fn revoke(
    pool: &PgPool,
    owner: Uuid,
    session_hash: &[u8],
    family: Uuid,
) -> Result<Revocation, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let active: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE id=$1 AND status='active'
         AND email_verified_at IS NOT NULL FOR UPDATE",
    )
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?;
    if active.is_none() {
        return Ok(Revocation::Denied);
    }
    let recent: Option<bool> = sqlx::query_scalar(
        "SELECT coalesce(sudo_until > clock_timestamp()
                  AND sudo_until <= clock_timestamp() + interval '15 minutes', false)
         FROM web_sessions
         WHERE id_hash=$1 AND account_id=$2
           AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()
         FOR UPDATE",
    )
    .bind(session_hash)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(recent) = recent else {
        return Ok(Revocation::Denied);
    };
    if !recent {
        return Ok(Revocation::Confirm);
    }

    let existing: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT revoked_at FROM device_sessions
         WHERE id=$1 AND account_id=$2 AND persistence_class='remembered' FOR UPDATE",
    )
    .bind(family)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(revoked_at) = existing else {
        return Ok(Revocation::Missing);
    };
    if revoked_at.is_none() {
        sqlx::query(
            "UPDATE device_sessions
             SET revoked_at=floor(extract(epoch FROM clock_timestamp()))::bigint, updated_at=clock_timestamp()
             WHERE id=$1 AND account_id=$2",
        )
        .bind(family)
        .bind(owner)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO audit_log(actor,action,target,before,after,request_id)
             VALUES($1,'native_device.revoked',$2,'{}','{}',$3)",
        )
        .bind(owner)
        .bind(family.to_string())
        .bind(Uuid::now_v7())
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE game_login_tickets
         SET expires_at=least(expires_at,clock_timestamp()), updated_at=clock_timestamp()
         WHERE device_family_id=$1 AND account_id=$2 AND used_at IS NULL AND expires_at>clock_timestamp()",
    )
    .bind(family)
    .bind(owner)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Revocation::Revoked)
}

pub async fn has_native_client(pool: &PgPool, clients: &[Uuid]) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM oauth_native_clients
         WHERE client_id=ANY($1) AND enabled AND first_party AND public_client AND native_ticket)",
    )
    .bind(clients)
    .fetch_one(pool)
    .await
}
