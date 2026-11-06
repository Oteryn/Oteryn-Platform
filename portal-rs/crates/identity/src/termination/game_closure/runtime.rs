//! Owner-accepted defaults and a transaction-bound production recovery fence.
use super::*;

#[derive(Debug, Clone)]
pub struct Config {
    pub enabled: bool,
    pub authority: String,
    pub key_id: String,
    pub public_key: Option<VerifyingKey>,
    pub recovery_generation: Option<u64>,
    pub worker_interval_seconds: i32,
    pub limits: Limits,
    pub final_disposition: FinalDisposition,
}
impl Config {
    pub fn from_env() -> Result<Self> {
        Self::load(|key| std::env::var(key).ok())
    }
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let enabled = match get("PORTAL_ACCOUNT_CLOSURE_ENABLED").as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            _ => return Err(Error::InvalidInput),
        };
        let authority = get("PORTAL_ACCOUNT_CLOSURE_SOURCE_AUTHORITY")
            .unwrap_or_else(|| "oteryn:character-authority:primary".into());
        let key_id = get("PORTAL_ACCOUNT_CLOSURE_KEY_ID").unwrap_or_default();
        let public_key = get("PORTAL_ACCOUNT_CLOSURE_PUBLIC_KEY_HEX")
            .map(|s| {
                let bytes = decode_hex(&s)?;
                let bytes: [u8; 32] = bytes.try_into().map_err(|_| Error::InvalidInput)?;
                VerifyingKey::from_bytes(&bytes).map_err(|_| Error::InvalidInput)
            })
            .transpose()?;
        let recovery_generation = get("PORTAL_ACCOUNT_CLOSURE_RECOVERY_GENERATION")
            .map(|s| wire::number(&s, true))
            .transpose()?;
        let worker_interval_seconds = get("PORTAL_ACCOUNT_CLOSURE_WORKER_INTERVAL_SECONDS")
            .map_or(Ok(5), |s| s.parse().map_err(|_| Error::InvalidInput))?;
        if !(1..=60).contains(&worker_interval_seconds)
            || (enabled
                && (public_key.is_none()
                    || recovery_generation.is_none()
                    || key_id.is_empty()
                    || key_id.len() > 128
                    || !key_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))))
        {
            return Err(Error::InvalidInput);
        }
        let final_disposition =
            match get("PORTAL_ACCOUNT_CLOSURE_FINAL_DISPOSITION_POLICY").as_deref() {
                None | Some("game_verified") => FinalDisposition::GameVerified,
                Some("hold") => FinalDisposition::Hold,
                _ => return Err(Error::InvalidInput),
            };
        let limits = Limits {
            ttl_seconds: get("PORTAL_ACCOUNT_CLOSURE_INTENT_TTL_SECONDS")
                .map_or(Ok(300), |s| s.parse().map_err(|_| Error::InvalidInput))?,
            max_pending: get("PORTAL_ACCOUNT_CLOSURE_MAX_PENDING")
                .map_or(Ok(40), |s| s.parse().map_err(|_| Error::InvalidInput))?,
            stale_seconds: get("PORTAL_ACCOUNT_CLOSURE_STALE_SECONDS")
                .map_or(Ok(30), |s| s.parse().map_err(|_| Error::InvalidInput))?,
        }
        .validate()?;
        Ok(Self {
            enabled,
            authority,
            key_id,
            public_key,
            recovery_generation,
            worker_interval_seconds,
            limits,
            final_disposition,
        })
    }
    /// Owner decision 2026-11-06 accepts v2 and reversible quarantine. There are
    /// no additional activation switches pretending those decisions remain open.
    pub fn boundary(
        &self,
        local: Arc<dyn Boundary>,
        termination_enabled: bool,
    ) -> Result<Option<Arc<GameBoundary>>> {
        if !self.enabled || !termination_enabled {
            return Ok(None);
        }
        let boundary = GameBoundary::new(
            Settings {
                enabled: true,
                v2_qualified: true,
                reversible_quarantine_accepted: true,
                disposition_and_retention_accepted: true,
                conflict_operator_configured: true,
            },
            local,
            Arc::new(DurableRecoveryFence {
                generation: self.recovery_generation.ok_or(Error::InvalidInput)?,
            }),
            self.public_key.ok_or(Error::InvalidInput)?,
            self.authority.clone(),
        )?;
        Ok(Some(Arc::new(
            boundary
                .with_limits(self.limits)?
                .with_final_disposition(self.final_disposition),
        )))
    }
}
pub fn decode_hex(raw: &str) -> Result<Vec<u8>> {
    if !raw.len().is_multiple_of(2)
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidInput);
    }
    raw.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| Error::InvalidInput)?;
            u8::from_str_radix(text, 16).map_err(|_| Error::InvalidInput)
        })
        .collect()
}
/// The configured floor is supplied by the nonrollback deployment reconciler,
/// never inferred from absence of receipts or a session/security generation.
/// Immutable local history prevents an accidental configuration downgrade.
#[derive(Debug)]
pub struct DurableRecoveryFence {
    pub generation: u64,
}
impl RecoveryFence for DurableRecoveryFence {
    fn current<'a>(&'a self, tx: &'a mut Transaction<'_, Postgres>) -> RecoveryFuture<'a> {
        Box::pin(async move {
            if self.generation == 0 {
                return Err(Error::UnknownState);
            }
            sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE")
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(Error::UnknownState)?;
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_restore_notices WHERE acknowledged_at IS NULL OR new_recovery_generation::numeric>$1::numeric) OR EXISTS(SELECT 1 FROM plat3_closure_receipts WHERE recovery_generation::numeric>$1::numeric) OR EXISTS(SELECT 1 FROM plat3_closure_commands WHERE convert_from(intent,'UTF8')::jsonb->>'expected_recovery_generation' IS NULL OR (convert_from(intent,'UTF8')::jsonb->>'expected_recovery_generation')::numeric>$1::numeric)")
                .bind(self.generation.to_string()).fetch_one(&mut **tx).await?;
            if blocked {
                Err(Error::UnknownState)
            } else {
                Ok(self.generation)
            }
        })
    }
}
