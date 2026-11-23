//! Read-only account closure status derived from the lifecycle and retained Game evidence.
use super::{AccountId, Error, Result, Terminations};
use sqlx::{PgPool, types::time::OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformState {
    Scheduled,
    Pending,
    Cancelled,
    Finalized,
}

impl PlatformState {
    pub fn message(self) -> &'static str {
        match self {
            Self::Scheduled => "closure-scheduled",
            Self::Pending => "closure-pending",
            Self::Cancelled => "closure-cancelled",
            Self::Finalized => "closure-finalized",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    NotRequired,
    Unavailable,
    CloseReceipt,
    Refused,
    Review,
    Confirmed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClosureStatus {
    pub id: Uuid,
    pub scheduled_for: OffsetDateTime,
    pub platform: PlatformState,
    pub game: GameState,
    pub close_issued: bool,
}

impl ClosureStatus {
    pub fn game_message(&self) -> Option<&'static str> {
        match (self.platform, self.game) {
            (_, GameState::NotRequired) => None,
            (PlatformState::Cancelled, GameState::Unavailable) => Some("restoration-pending"),
            (_, GameState::Unavailable) => Some("closure-game-unavailable"),
            (_, GameState::CloseReceipt) => Some("closure-game-close-received"),
            (_, GameState::Refused) => Some("closure-game-refused"),
            (_, GameState::Review) => Some("closure-game-review"),
            (PlatformState::Cancelled, GameState::Confirmed) => Some("closure-game-restored"),
            (_, GameState::Confirmed) => Some("closure-game-finalized"),
        }
    }

    pub fn deadline_utc(&self) -> String {
        let deadline = self
            .scheduled_for
            .to_offset(sqlx::types::time::UtcOffset::UTC);
        format!(
            "{} {:02}:{:02} UTC",
            deadline.date(),
            deadline.hour(),
            deadline.minute()
        )
    }
}

#[derive(sqlx::FromRow)]
struct Evidence {
    id: Uuid,
    scheduled_for: OffsetDateTime,
    due: bool,
    cancelled: bool,
    finalized: bool,
    close_issued: bool,
    conflict: bool,
    outcome: Option<String>,
    terminal_confirmed: bool,
}

impl From<Evidence> for ClosureStatus {
    fn from(e: Evidence) -> Self {
        let platform = if e.finalized {
            PlatformState::Finalized
        } else if e.cancelled {
            PlatformState::Cancelled
        } else if e.due {
            PlatformState::Pending
        } else {
            PlatformState::Scheduled
        };
        let game = match (platform, e.close_issued, e.conflict, e.outcome.as_deref()) {
            (PlatformState::Scheduled | PlatformState::Cancelled, false, _, _) => {
                GameState::NotRequired
            }
            (_, _, true, _) => GameState::Review,
            (_, _, _, Some("REJECTED")) => GameState::Refused,
            (PlatformState::Cancelled | PlatformState::Finalized, _, _, Some("COMMITTED"))
                if e.terminal_confirmed =>
            {
                GameState::Confirmed
            }
            (PlatformState::Pending, _, _, Some("COMMITTED")) => GameState::CloseReceipt,
            _ => GameState::Unavailable,
        };
        Self {
            id: e.id,
            scheduled_for: e.scheduled_for,
            platform,
            game,
            close_issued: e.close_issued,
        }
    }
}

impl Terminations {
    /// Call with the account ID from the authenticated player's session.
    pub async fn latest_status(&self, owner: AccountId) -> Result<Option<ClosureStatus>> {
        read(&self.pool, Some(owner), None).await
    }
}

/// Call after the staff host, session, MFA and audit permission checks.
pub async fn operator_status(pool: &PgPool, reference: &str) -> Result<Option<ClosureStatus>> {
    let id = Uuid::parse_str(reference).map_err(|_| Error::InvalidInput)?;
    if id.to_string() != reference || id.get_variant() != uuid::Variant::RFC4122 {
        return Err(Error::InvalidInput);
    }
    read(pool, None, Some(id)).await
}

async fn read(
    pool: &PgPool,
    owner: Option<AccountId>,
    reference: Option<Uuid>,
) -> Result<Option<ClosureStatus>> {
    // A committed resolution is displayed as confirmed only when the retained
    // snapshot from the current epoch corroborates the exact resolution receipt.
    let evidence: Option<Evidence> = sqlx::query_as(
        "SELECT t.id, t.scheduled_for, t.scheduled_for <= now() AS due,
                t.cancelled_at IS NOT NULL AS cancelled, t.finalized_at IS NOT NULL AS finalized,
                w.close_operation_id IS NOT NULL AS close_issued,
                COALESCE(w.invalid OR (w.close_operation_id IS NOT NULL AND (s.invalid OR f.invalid)), false) AS conflict,
                attempt.receipt->>'outcome' AS outcome,
                COALESCE(f.highest_epoch = s.epoch
                    AND attempt.receipt->'result_data'->>'closure_state' =
                        CASE WHEN t.cancelled_at IS NOT NULL THEN 'CANCELLED'
                             WHEN t.finalized_at IS NOT NULL THEN 'FINALIZED' END
                    AND record.data->>'closure_state' = attempt.receipt->'result_data'->>'closure_state'
                    AND record.data->>'termination_id' = t.id::text
                    AND record.data->>'operation_id' = attempt.operation_id::text
                    AND record.data->>'lifecycle_generation' = attempt.receipt->'result_data'->>'lifecycle_generation'
                    AND record.data->>'proof_sha256' = attempt.receipt->'result_data'->>'proof_sha256'
                    AND record.data->>'receipt_digest' = encode(attempt.receipt_sha256,'hex'), false) AS terminal_confirmed
         FROM account_terminations t
         LEFT JOIN plat3_closure_work w ON w.termination_id=t.id AND w.account_id=t.account_id
         LEFT JOIN plat3_closure_feed f ON f.id=1
         LEFT JOIN plat3_closure_snapshots s ON s.account_id=t.account_id
         LEFT JOIN LATERAL (
             SELECT c.operation_id, convert_from(r.receipt,'UTF8')::jsonb AS receipt, r.receipt_sha256
             FROM plat3_closure_commands c
             LEFT JOIN plat3_closure_receipts r USING (operation_id)
             WHERE c.termination_id=w.termination_id
               AND ((w.resolution IS NULL AND c.operation_id=w.close_operation_id)
                    OR (w.resolution IS NOT NULL AND c.command='ResolveAccountClosure'))
             ORDER BY c.source_revision DESC LIMIT 1
         ) attempt ON true
         LEFT JOIN LATERAL (
             SELECT convert_from(s.snapshot,'UTF8')::jsonb->'closures'->0 AS data
         ) record ON true
         WHERE (t.account_id=$1 AND $2::uuid IS NULL)
            OR (t.id=$2 AND $1::uuid IS NULL)
         ORDER BY t.created_at DESC, t.id DESC LIMIT 1",
    )
    .bind(owner)
    .bind(reference)
    .fetch_optional(pool)
    .await?;
    Ok(evidence.map(Into::into))
}
