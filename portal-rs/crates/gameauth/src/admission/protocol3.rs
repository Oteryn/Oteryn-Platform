//! Gateway login contract §19 (N4P-4) on the fresh issuer: the protocol 3
//! switch, the per-account play generation fence and recovery handle issuance
//! (§19.3). Testing/preproduction only and default off; handles additionally
//! require `OTERYN_ENV=testing` until §19.15 device binding is accepted beyond it
//! (owner decision U21).
use super::{AdmissionRequest, AuthorizedAccount, Error, grant, wire};
use crate::runtime;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

/// Default-off protocol 3 switch on the issuer (§19.1).
pub const PROTOCOL3_ENABLED: &str = "PORTAL_NATIVE_PROTOCOL3_ENABLED";
/// Default-off recovery switch: handle issuance here and `native-recoveries`.
pub const RECOVERY_ENABLED: &str = "PORTAL_NATIVE_RECOVERY_ENABLED";
/// Handle lifetime H in seconds, 3600..=43200, default 43200 (U20).
pub const HANDLE_LIFETIME_SECONDS: &str = "PORTAL_NATIVE_RECOVERY_HANDLE_LIFETIME_SECONDS";
/// Device binding (§19.15), default on; `false` is accepted only in `testing`.
pub const REQUIRE_DEVICE_BINDING: &str = "PORTAL_NATIVE_RECOVERY_REQUIRE_DEVICE_BINDING";
/// The native device-session switch. §19.15.1: with binding on and this off,
/// protocol 3 logins carry `"recovery": null` and recovery refuses.
pub const DEVICES_ENABLED: &str = "OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED";

pub const MAX_HANDLE_LIFETIME_SECONDS: u32 = 43_200;
const HANDLE_PREFIX: &str = "otr1.";

/// Handle issuance policy (§19.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handles {
    /// Recovery is off: protocol 3 responses carry `"recovery": null`.
    Off,
    On {
        lifetime_seconds: u32,
        require_device_binding: bool,
        /// `OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED`. With binding required
        /// and devices off, protocol 3 logins carry `"recovery": null` and
        /// recovery refuses as for an unknown handle (§19.15.1).
        devices_enabled: bool,
    },
    /// A lifetime or binding setting outside the contract: protocol 3 issuance
    /// is refused rather than falling back (§19.3).
    Invalid,
}
#[derive(Debug, Clone)]
pub struct Protocol3 {
    pub runtime: runtime::Settings,
    pub handles: Handles,
}
impl Protocol3 {
    /// `None` keeps protocol 3 unsupported (§19.1). The runtime settings are the
    /// ones the grant issuer selects with.
    pub fn configured(
        lookup: impl Fn(&str) -> Option<String>,
        runtime: runtime::Settings,
    ) -> Option<Self> {
        let environment = lookup("OTERYN_ENV").unwrap_or_default();
        if lookup(PROTOCOL3_ENABLED).as_deref() != Some("true")
            || !matches!(environment.as_str(), "testing" | "preprod")
        {
            return None;
        }
        let handles = if lookup(RECOVERY_ENABLED).as_deref() == Some("true") {
            let lifetime =
                lookup(HANDLE_LIFETIME_SECONDS).map_or(Some(MAX_HANDLE_LIFETIME_SECONDS), |raw| {
                    raw.parse::<u32>().ok().filter(|v| {
                        raw == v.to_string() && (3600..=MAX_HANDLE_LIFETIME_SECONDS).contains(v)
                    })
                });
            let devices = lookup(DEVICES_ENABLED).as_deref() == Some("true");
            let binding = match lookup(REQUIRE_DEVICE_BINDING).as_deref() {
                None | Some("true") => Some(true),
                Some("false") if environment == "testing" => Some(false),
                _ => None,
            };
            match (lifetime, binding) {
                // U21: recovery stays refused beyond testing.
                _ if environment != "testing" => Handles::Off,
                (Some(lifetime_seconds), Some(require_device_binding)) => Handles::On {
                    lifetime_seconds,
                    require_device_binding,
                    devices_enabled: devices,
                },
                _ => Handles::Invalid,
            }
        } else {
            Handles::Off
        };
        Some(Self { runtime, handles })
    }
}

/// SHA-256 of a presented handle, or `None` when it is not `otr1.` plus 43
/// unpadded base64url characters of 32 bytes.
pub fn handle_hash(handle: &str) -> Option<[u8; 32]> {
    let encoded = handle.strip_prefix(HANDLE_PREFIX)?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    (encoded.len() == 43 && bytes.len() == 32 && URL_SAFE_NO_PAD.encode(&bytes) == encoded)
        .then(|| Sha256::digest(handle.as_bytes()).into())
}
fn mint() -> Result<(String, [u8; 32]), Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    let handle = format!("{HANDLE_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes));
    let hash = Sha256::digest(handle.as_bytes()).into();
    Ok((handle, hash))
}
async fn seconds(db: &mut PgConnection) -> Result<i64, Error> {
    sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(db)
        .await
        .map_err(|_| Error::Unavailable)
}

/// §19.3 play generation fence, inside the fresh issuer transaction: advance
/// the account's generation and revoke every handle it issued before.
pub(super) async fn advance(db: &mut PgConnection, account: Uuid) -> Result<i64, Error> {
    let generation: i64 = sqlx::query_scalar("INSERT INTO native_play_generations (account_id, generation) VALUES ($1, 1) ON CONFLICT (account_id) DO UPDATE SET generation = native_play_generations.generation + 1, updated_at = clock_timestamp() RETURNING generation")
        .bind(account).fetch_one(&mut *db).await.map_err(|_| Error::Unavailable)?;
    sqlx::query("UPDATE native_recovery_handles SET revoked_at = floor(extract(epoch FROM clock_timestamp()))::bigint WHERE account_id = $1 AND revoked_at IS NULL")
        .bind(account).execute(db).await.map_err(|_| Error::Unavailable)?;
    Ok(generation)
}

/// What a fresh protocol 3 attempt binds into its handle.
pub(super) struct Binding<'a> {
    pub request: &'a AdmissionRequest,
    pub account: AuthorizedAccount,
    pub world: Uuid,
    pub channel: Uuid,
    pub play_generation: i64,
}
/// §19.3 handle creation in the fresh issuer transaction. `None` is
/// `"recovery": null`: recovery off, or binding required and the redeemed
/// ticket names no live keyed device family.
pub(super) async fn issue(
    db: &mut PgConnection,
    handles: Handles,
    binding: Binding<'_>,
) -> Result<Option<wire::Recovery>, Error> {
    let (lifetime, required) = match handles {
        Handles::Off => return Ok(None),
        Handles::Invalid => return Err(Error::Unavailable),
        Handles::On {
            require_device_binding: true,
            devices_enabled: false,
            ..
        } => return Ok(None),
        Handles::On {
            lifetime_seconds,
            require_device_binding,
            ..
        } => (i64::from(lifetime_seconds), require_device_binding),
    };
    let now = seconds(db).await?;
    let family: Option<Uuid> = sqlx::query_scalar(
        "SELECT device_family_id FROM game_login_tickets WHERE ticket_hash = $1",
    )
    .bind(binding.request.ticket_hash())
    .fetch_optional(&mut *db)
    .await
    .map_err(|_| Error::Unavailable)?
    .flatten();
    let mut device = None;
    if let Some(family) = family {
        let row = sqlx::query("SELECT device_key_thumbprint, absolute_expires_at FROM device_sessions WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND absolute_expires_at > $3 AND idle_expires_at > $3 AND device_key_thumbprint IS NOT NULL FOR SHARE")
            .bind(family).bind(binding.account.id).bind(now)
            .fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?;
        if let Some(row) = row {
            let thumbprint: String = row
                .try_get("device_key_thumbprint")
                .map_err(|_| Error::Unavailable)?;
            let deadline: i64 = row
                .try_get("absolute_expires_at")
                .map_err(|_| Error::Unavailable)?;
            device = Some((family, thumbprint, deadline));
        }
    }
    if required && device.is_none() {
        return Ok(None);
    }
    let expires = device.as_ref().map_or(now + lifetime, |(_, _, deadline)| {
        (*deadline).min(now + lifetime)
    });
    if expires <= now {
        return Ok(None);
    }
    let (handle, hash) = mint()?;
    let security =
        i64::try_from(binding.account.security_generation).map_err(|_| Error::SecurityDenied)?;
    sqlx::query("INSERT INTO native_recovery_handles (handle_hash, account_id, character_id, world_id, channel_id, security_generation, play_generation, attempt_ref, device_family_id, device_key_thumbprint, issued_at, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(&hash[..]).bind(binding.account.id).bind(binding.request.character_id)
        .bind(binding.world).bind(binding.channel).bind(security).bind(binding.play_generation)
        .bind(binding.request.attempt_ref)
        .bind(device.as_ref().map(|d| d.0)).bind(device.as_ref().map(|d| d.1.clone()))
        .bind(now).bind(expires)
        .execute(db).await.map_err(|_| Error::Unavailable)?;
    Ok(Some(wire::Recovery {
        handle,
        expires_in: u64::try_from(expires - now).map_err(|_| Error::Unavailable)?,
    }))
}
/// §19.3 retry of the fresh attempt: the attempt's live handle is replaced by
/// a new one with the same bindings and expiry. A revoked or expired handle is
/// never revived (`"recovery": null`).
pub(super) async fn replace(
    db: &mut PgConnection,
    handles: Handles,
    attempt: Uuid,
) -> Result<Option<wire::Recovery>, Error> {
    match handles {
        Handles::Off => return Ok(None),
        Handles::Invalid => return Err(Error::Unavailable),
        Handles::On { .. } => {}
    }
    let now = seconds(db).await?;
    let Some(old) = sqlx::query("SELECT handle_hash, expires_at FROM native_recovery_handles WHERE attempt_ref = $1 AND revoked_at IS NULL AND expires_at > $2 FOR UPDATE")
        .bind(attempt).bind(now)
        .fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?
    else {
        return Ok(None);
    };
    let old_hash: Vec<u8> = old.try_get("handle_hash").map_err(|_| Error::Unavailable)?;
    let expires: i64 = old.try_get("expires_at").map_err(|_| Error::Unavailable)?;
    let (handle, hash) = mint()?;
    sqlx::query("INSERT INTO native_recovery_handles (handle_hash, account_id, character_id, world_id, channel_id, security_generation, play_generation, attempt_ref, device_family_id, device_key_thumbprint, issued_at, expires_at) SELECT $2, account_id, character_id, world_id, channel_id, security_generation, play_generation, attempt_ref, device_family_id, device_key_thumbprint, issued_at, expires_at FROM native_recovery_handles WHERE handle_hash = $1")
        .bind(&old_hash).bind(&hash[..])
        .execute(&mut *db).await.map_err(|_| Error::Unavailable)?;
    sqlx::query("UPDATE native_recovery_handles SET revoked_at = $2 WHERE handle_hash = $1")
        .bind(&old_hash)
        .bind(now)
        .execute(db)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok(Some(wire::Recovery {
        handle,
        expires_in: u64::try_from(expires - now).map_err(|_| Error::Unavailable)?,
    }))
}
/// The §19.2.4 `channels` list at response time, at most 16 entries.
pub(super) async fn channels(
    db: &mut PgConnection,
    settings: &Protocol3,
    world: Uuid,
) -> Result<Vec<wire::Channel>, Error> {
    let now = seconds(db).await?;
    Ok(grant::candidates(db, &settings.runtime, world, now)
        .await?
        .into_iter()
        .take(wire::MAX_LISTED_CHANNELS)
        .map(|c| wire::Channel {
            channel_id: c.route.channel,
            key: c.key,
        })
        .collect())
}
