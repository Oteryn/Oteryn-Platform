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

impl GameBoundary {
    /// Independently read under the same account lock used by lifecycle mutations.
    pub async fn read_lifecycle(&self, pool: &PgPool, raw: &[u8]) -> Result<Vec<u8>> {
        self.settings.ready()?;
        let request = wire::LifecycleRequest::parse(raw, &self.authority)?;
        let account = wire::id(&request.account_id)?;
        let termination = wire::id(&request.termination_id)?;
        let mut tx = pool.begin().await?;
        let locked = lock(&mut tx, account).await?.ok_or(Error::NoPending)?;
        if self.recovery(&mut tx).await? != wire::number(&request.recovery_generation, true)? {
            return Err(Error::UnknownState);
        }
        let t: Termination = sqlx::query_as(
            "SELECT * FROM account_terminations WHERE id=$1 AND account_id=$2 FOR UPDATE",
        )
        .bind(termination)
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NoPending)?;
        let (generation, current, resolution): (i64,i64,Option<String>) = sqlx::query_as("SELECT w.generation,l.generation,w.resolution FROM plat3_closure_work w JOIN plat3_closure_lifecycle l USING(account_id) WHERE w.termination_id=$1 AND w.account_id=$2 AND NOT w.invalid FOR UPDATE OF w,l")
            .bind(termination).bind(account).fetch_optional(&mut *tx).await?.ok_or(Error::UnknownState)?;
        let phase = match (
            t.cancelled_at.is_some(),
            t.finalized_at.is_some(),
            resolution.as_deref(),
        ) {
            (false, false, None) if current == generation => "CLOSE",
            (true, false, Some("CANCEL")) if generation.checked_add(1) == Some(current) => "CANCEL",
            (false, true, Some("FINALIZE")) if generation.checked_add(1) == Some(current) => {
                "FINALIZE"
            }
            _ => return Err(Error::UnknownState),
        };
        if !matches!(
            (phase, locked.status.as_str()),
            ("CLOSE" | "CANCEL", "active" | "suspended") | ("FINALIZE", "closed")
        ) {
            return Err(Error::UnknownState);
        }
        // Wallet writers use this lock. Credential and Character state are
        // separate from the current financial attestation.
        sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
            .bind(account)
            .execute(&mut *tx)
            .await?;
        let financial_obligations_clear: bool = sqlx::query_scalar("SELECT NOT (EXISTS(SELECT 1 FROM wallet_reservations WHERE account_id=$1 AND status='active') OR EXISTS(SELECT 1 FROM bazaar_listings WHERE (owner=$1 OR leader=$1) AND state IN ('draft','active','matched','awaiting_delivery')))").bind(account).fetch_one(&mut *tx).await?;
        let observed: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let observed = u64::try_from(observed).map_err(|_| Error::UnknownState)?;
        let revision: i64 =
            sqlx::query_scalar("SELECT nextval('plat3_closure_observation_revision')")
                .fetch_one(&mut *tx)
                .await?;
        let observation = lifecycle_observation(
            request,
            current,
            revision,
            phase,
            financial_obligations_clear,
            observed,
        )?;
        let body = serde_json::to_vec(&observation).map_err(|_| Error::InvalidInput)?;
        if body.len() > 2048 {
            return Err(Error::InvalidInput);
        }
        tx.commit().await?;
        Ok(body)
    }
}

pub(super) fn lifecycle_observation(
    request: wire::LifecycleRequest,
    current: i64,
    revision: i64,
    phase: &str,
    financial_obligations_clear: bool,
    observed: u64,
) -> Result<wire::LifecycleObservation> {
    if current <= 0 || revision <= 0 || observed == 0 {
        return Err(Error::UnknownState);
    }
    let observation = wire::LifecycleObservation {
        contract_version: 2,
        operation: request.operation,
        source_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
        account_id: request.account_id,
        termination_id: request.termination_id,
        lifecycle_generation: current.to_string(),
        source_revision: revision.to_string(),
        phase: phase.into(),
        financial_obligations_clear,
        recovery_generation: request.recovery_generation,
        observed_at: observed.to_string(),
        valid_until: observed
            .checked_add(5)
            .ok_or(Error::UnknownState)?
            .to_string(),
    };
    Ok(observation)
}
