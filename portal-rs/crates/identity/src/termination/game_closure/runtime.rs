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
            || (enabled && recovery_generation.is_none())
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
            self.public_key,
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
        .as_chunks::<2>()
        .0
        .iter()
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
        // The finalized event and the financial disposition commit together.
        // A later verified reversal remains a reconciliation obligation, but
        // cannot revoke the disposition Game needs to finish the closure.
        let finalized = phase == "FINALIZE"
            && sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM account_termination_events WHERE termination_id=$1 AND kind='finalized')",
            )
            .bind(t.id)
            .fetch_one(&mut *tx)
            .await?;
        let financial_obligations_clear =
            finalized || financial_obligations_clear(&mut tx, account).await?;
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

/// Shared financial check for local termination and Game lifecycle attestation.
/// The caller holds the account lock.
pub async fn financial_obligations_clear(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
) -> Result<bool> {
    // Restore/publication fences precede wallet locks, as in the Game lifecycle read.
    sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE")
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::UnknownState)?;
    sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    Ok(sqlx::query_scalar("SELECT NOT (
        EXISTS(SELECT 1 FROM wallet_reservations WHERE account_id=$1 AND status='active')
        OR EXISTS(SELECT 1 FROM coin_purchase_holds WHERE account_id=$1)
        OR EXISTS(SELECT 1 FROM coin_outstanding_debts WHERE account_id=$1 AND owed>0)
        OR EXISTS(SELECT 1 FROM wallet_incoming_capacity WHERE account_id=$1)
        OR EXISTS(SELECT 1 FROM bazaar_coin_holds WHERE account_id=$1 AND amount>0)
        OR EXISTS(SELECT 1 FROM bazaar_listings WHERE (owner=$1 OR leader=$1) AND state IN ('draft','active','matched','awaiting_delivery'))
        OR EXISTS(SELECT 1 FROM bazaar_settlements s WHERE (s.buyer=$1 OR s.seller=$1)
            AND s.completed_at IS NULL AND NOT EXISTS (
                SELECT 1 FROM bazaar_authority_compensations c JOIN bazaar_listings l ON l.id=c.listing_id
                WHERE c.listing_id=s.listing_id AND l.state='cancelled'))
        OR EXISTS(SELECT 1 FROM bazaar_authority_gold_bids WHERE buyer=$1 AND phase IN ('pending','releasing'))
        OR EXISTS(SELECT 1 FROM bazaar_authority_gold_bids b
            LEFT JOIN bazaar_game_outbox o ON o.operation_id=b.release_operation_id
                AND o.listing_id=b.listing_id AND o.command='ReleaseBazaarGold'
            WHERE b.phase='releasing' AND (
                (o.intent::jsonb->>'buyer_account_id')::uuid=$1
                OR (o.intent::jsonb->>'buyer_account_id' IS NULL AND EXISTS (
                    SELECT 1 FROM bazaar_authority_gold_bids prior
                    WHERE prior.listing_id=b.listing_id AND prior.buyer=$1))))
        OR EXISTS(SELECT 1 FROM bazaar_authority_gold_sales s WHERE (s.buyer=$1 OR s.seller=$1)
            AND s.completed_at IS NULL AND NOT EXISTS (
                SELECT 1 FROM bazaar_authority_compensations c JOIN bazaar_listings l ON l.id=c.listing_id
                WHERE c.listing_id=s.listing_id AND l.state='cancelled'))
        OR EXISTS(
            SELECT 1 FROM payment_intents p
            JOIN LATERAL (
                SELECT account_id,coins,NULL::uuid AS uncredited_native_order FROM store_web_topups
                WHERE p.id IN (payment_intent_id,intended_payment_intent_id)
                UNION ALL
                SELECT payer_account_id,coins,CASE WHEN receipt_id IS NULL THEN id END
                FROM coin_purchase_orders WHERE payment_intent_id=p.id
            ) checkout ON p.account_id=$1 OR checkout.account_id=$1
            WHERE p.state IN ('succeeded','refunded','chargeback') AND (
                checkout.account_id<>p.account_id OR NOT (
                    EXISTS (SELECT 1 FROM wallet_operations o JOIN wallet_entries e ON e.operation_id=o.id
                        WHERE o.id=p.id AND o.account_id=p.account_id AND o.kind='top_up'
                          AND o.amount=checkout.coins AND e.account_id=p.account_id
                          AND e.available_delta=checkout.coins AND e.reserved_delta=0)
                    OR (p.state IN ('refunded','chargeback') AND EXISTS (
                        SELECT 1 FROM payment_events e
                        WHERE e.intent_id=p.id AND e.provider=p.provider AND e.state=p.state))
                    OR (p.state='succeeded' AND EXISTS (
                        SELECT 1 FROM coin_purchase_reversals r JOIN payment_events e
                          ON (e.provider,e.provider_event_id)=(r.provider,r.provider_event_id)
                        WHERE r.order_id=checkout.uncredited_native_order
                          AND r.payer_account_id=p.account_id AND r.coins=checkout.coins
                          AND r.outcome='fenced' AND r.debited=0
                          AND e.intent_id=p.id AND e.provider=p.provider
                          AND (r.kind,e.state) IN (('refund','refunded'),('chargeback','chargeback'))))
                )
            )
        )
    )").bind(account).fetch_one(&mut **tx).await?)
}
