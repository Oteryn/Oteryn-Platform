//! Owner-only character services. Game remains the lifecycle and name authority.
mod browser;
mod projection;
#[cfg(test)]
mod tests;
mod transport;
pub use crate::commands::services::{Action, Intent};
use crate::commands::{self, Error, services as wire};
pub use browser::{show, submit};
use oteryn_wallet::{AccountStateReader, AuditHook, Ledger, TransferBlockCheck, WalletFlags};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction};
pub use transport::PROJECTION_GROUP;
pub use transport::router;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub schedule_deletion: bool,
    pub cancel_deletion: bool,
    pub rename: bool,
    pub projection_enabled: bool,
    /// Keep exchange enabled to drain outstanding commands when submissions stop.
    pub exchange_enabled: bool,
    pub wallet_enabled: bool,
    pub command_authority: String,
    pub projection_authority: String,
    /// Explicitly qualified clock bound and freshness; absent denies.
    pub clock_uncertainty_seconds: Option<u16>,
    pub freshness_seconds: Option<u16>,
    pub rename_price: Option<Price>,
}
#[derive(Debug, Clone)]
pub struct Price {
    pub product_id: String,
    pub version: i32,
    pub coins: i64,
}
impl Settings {
    fn projection_ready(&self) -> bool {
        self.projection_enabled
            && valid_authority(&self.projection_authority)
            && self.clock_uncertainty_seconds.is_some()
            && self.freshness_seconds.is_some_and(|v| v > 0 && v <= 30)
    }
    fn permits(&self, action: &Action) -> bool {
        self.enabled
            && self.projection_ready()
            && self.exchange_enabled
            && valid_authority(&self.command_authority)
            && match action {
                Action::Schedule => self.schedule_deletion && self.cancel_deletion,
                Action::Cancel { .. } => self.cancel_deletion,
                Action::Rename { .. } => self.rename && self.wallet_enabled,
            }
    }
}
#[derive(Debug, Clone)]
pub struct Services {
    pool: PgPool,
    pub settings: Settings,
}
#[derive(Debug, Clone)]
pub struct Request {
    pub accepted_price: Option<i64>,
    pub request_id: Uuid,
    pub character_id: Uuid,
    pub action: Action,
}
#[derive(Debug, Clone)]
pub struct Authority;
impl AccountStateReader for Authority {
    async fn state(&self, conn: &mut PgConnection, account: Uuid) -> oteryn_wallet::AccountState {
        match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account)
            .fetch_optional(conn)
            .await
        {
            Ok(Some(s)) if s == "active" => oteryn_wallet::AccountState::Active,
            Ok(Some(_)) => oteryn_wallet::AccountState::Inactive,
            _ => oteryn_wallet::AccountState::Unknown,
        }
    }
}
impl TransferBlockCheck for Authority {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> oteryn_wallet::TransferPermission {
        oteryn_wallet::TransferPermission::Unknown
    }
}
impl AuditHook for Authority {
    async fn adjustment(&self, _: &mut PgConnection, _: &oteryn_wallet::AdjustmentAudit) -> bool {
        false
    }
}
type Wallet = Ledger<Authority, Authority, Authority>;
impl Services {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self { pool, settings }
    }
    fn wallet(&self) -> Wallet {
        Ledger::new(
            self.pool.clone(),
            WalletFlags {
                enabled: self.settings.wallet_enabled,
            },
            Authority,
            Authority,
            Authority,
        )
    }
    /// The actor comes exclusively from the authenticated server session. All writes
    /// including coin reservation and command issuance share this transaction.
    pub async fn request(
        &self,
        actor: Uuid,
        request: Request,
    ) -> Result<commands::CommandStatus, Error> {
        if !self.settings.permits(&request.action) {
            return Err(Error::Unavailable);
        }
        wire::identifier(&actor.to_string())?;
        wire::identifier(&request.request_id.to_string())?;
        wire::identifier(&request.character_id.to_string())?;
        let binding = format!(
            "{}:{}",
            action_json(&request.action)?,
            request
                .accepted_price
                .map(|n| n.to_string())
                .unwrap_or_default()
        );
        let mut tx = self.pool.begin().await?;
        restore_lock(&mut tx).await?;
        // Account lock bounds submission rate across different characters.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,47))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        // Same character lock across accounts also serializes old-owner requests.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,46))")
            .bind(request.character_id.to_string())
            .execute(&mut *tx)
            .await?;
        if Authority.state(&mut tx, actor).await != oteryn_wallet::AccountState::Active {
            return Err(Error::Denied);
        }
        let state = self
            .owner_state(&mut tx, actor, request.character_id)
            .await?;
        if let Some(row)=sqlx::query("SELECT o.account_id,o.character_id,o.action_json,c.id,c.command,c.intent_json,c.receipt_json FROM character_service_orders o JOIN character_commands c ON c.id=o.operation_id WHERE o.request_id=$1").bind(request.request_id).fetch_optional(&mut *tx).await? {
            if row.try_get::<Uuid,_>("account_id")?!=actor || row.try_get::<Uuid,_>("character_id")?!=request.character_id || row.try_get::<String,_>("action_json")?!=binding { return Err(Error::Conflict); }
            return Ok(commands::CommandStatus{operation_id:row.try_get("id")?,command:row.try_get("command")?,intent_json:row.try_get("intent_json")?,receipt_json:row.try_get("receipt_json")?});
        }
        let rapid: bool = sqlx::query_scalar("SELECT count(*) >= 10 FROM character_service_orders WHERE account_id=$1 AND created_at>clock_timestamp()-interval '1 minute'").bind(actor).fetch_one(&mut *tx).await?;
        if rapid {
            return Err(Error::BacklogFull);
        }
        state.permits(&request.action, now(&mut tx).await?)?;
        // A committed receipt ahead of the companion snapshot cannot become a fresh precondition.
        let unseen:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_service_orders o JOIN character_commands c ON c.id=o.operation_id WHERE o.character_id=$1 AND c.receipt_json::jsonb->>'outcome'='COMMITTED' AND (c.receipt_json::jsonb->>'character_revision')::numeric>$2::numeric)").bind(request.character_id).bind(state.character_revision.to_string()).fetch_one(&mut *tx).await?;
        if unseen {
            return Err(Error::Unavailable);
        }
        let mut credit = None;
        let mut reservation = None;
        let mut price = None;
        if matches!(request.action, Action::Rename { .. }) {
            let accepted_price = request
                .accepted_price
                .filter(|n| *n >= 0)
                .ok_or(Error::BadInput)?;
            credit=sqlx::query_scalar::<_,Uuid>("SELECT operation_id FROM character_service_orders WHERE account_id=$1 AND character_id=$2 AND financial_state='RECOVERY_REQUIRED' AND credit_used_by IS NULL ORDER BY created_at LIMIT 1 FOR UPDATE")
                .bind(actor).bind(request.character_id).fetch_optional(&mut *tx).await?;
            if credit.is_none() {
                let p = self
                    .settings
                    .rename_price
                    .clone()
                    .ok_or(Error::Unavailable)?;
                if accepted_price != p.coins {
                    return Err(Error::Conflict);
                }
                if !p.valid() {
                    return Err(Error::Unavailable);
                }
                let id = Uuid::now_v7();
                self.wallet()
                    .reserve_in(&mut tx, id, actor, p.coins)
                    .await
                    .map_err(wallet_error)?;
                reservation = Some(id);
                price = Some(p);
            }
        }
        sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
            .execute(&mut *tx)
            .await?;
        let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_commands c WHERE state='PENDING' AND binding_json::jsonb->>'character_id'=$1)").bind(request.character_id.to_string()).fetch_one(&mut *tx).await?;
        if pending {
            return Err(Error::Conflict);
        }
        let intent = Intent {
            character_id: request.character_id,
            account_id: actor,
            expected_character_revision: state.character_revision,
            action: request.action,
        };
        let command = wire::enqueue_in(
            &mut tx,
            Uuid::now_v7(),
            &format!("character-service:{}", request.request_id),
            &intent,
        )
        .await?;
        sqlx::query("INSERT INTO character_service_orders(operation_id,account_id,character_id,request_id,action_json,product_id,product_version,amount,reservation_id,settlement_id,financial_state,credit_from,intent_digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
            .bind(command.operation_id).bind(actor).bind(request.character_id).bind(request.request_id).bind(binding).bind(price.as_ref().map(|p|&p.product_id)).bind(price.as_ref().map(|p|p.version)).bind(price.as_ref().map(|p|p.coins)).bind(reservation).bind(Uuid::now_v7()).bind(if reservation.is_some(){"RESERVED"}else{"FREE"}).bind(credit).bind(Sha256::digest(command.intent_json.as_bytes()).to_vec()).execute(&mut *tx).await?;
        if let Some(credit) = credit {
            sqlx::query("UPDATE character_service_orders SET credit_used_by=$2,updated_at=now() WHERE operation_id=$1").bind(credit).bind(command.operation_id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(command)
    }
    /// Called by the purpose-bound private router. Receipt, settlement and restore
    /// history commit together. Missing receipts never release a reservation.
    pub async fn receive(&self, raw: &[u8]) -> Result<(), Error> {
        if !self.settings.exchange_enabled {
            return Err(Error::Unavailable);
        }
        let v = wire::receipt(raw)?;
        source(&v, &self.settings.command_authority)?;
        let id = wire::identifier(wire::string(&v, "operation_id")?)?;
        let mut tx = self.pool.begin().await?;
        restore_lock(&mut tx).await?;
        let row=sqlx::query("SELECT c.intent_json,c.receipt_json,o.account_id,o.reservation_id,o.settlement_id,o.financial_state,o.credit_from,o.credit_used_by FROM character_commands c JOIN character_service_orders o ON o.operation_id=c.id WHERE c.id=$1 FOR UPDATE OF c,o").bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let intent: String = row.try_get("intent_json")?;
        wire::validate_binding(&v, &intent)?;
        if let Some(existing) = row.try_get::<Option<String>, _>("receipt_json")? {
            return if existing.as_bytes() == raw {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        let generation = wire::number(wire::string(&v, "recovery_generation")?, false)?;
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT max(new_recovery_generation) FROM character_restore_notices",
        )
        .fetch_one(&mut *tx)
        .await?;
        if latest.is_some_and(|g| format!("{generation:020}") < g) {
            return Err(Error::Conflict);
        }
        let committed = wire::string(&v, "outcome")? == "COMMITTED";
        let account: Uuid = row.try_get("account_id")?;
        let financial: String = row.try_get("financial_state")?;
        let mut next = financial.clone();
        if financial == "RESERVED" {
            let reservation: Uuid = row.try_get("reservation_id")?;
            self.wallet()
                .settle_in(
                    &mut tx,
                    row.try_get("settlement_id")?,
                    account,
                    reservation,
                    committed,
                )
                .await
                .map_err(wallet_error)?;
            next = if committed { "SETTLED" } else { "RELEASED" }.into();
        } else if financial == "SETTLED" && !committed {
            // Only the existing restore trigger can reopen a terminal receipt.
            let reopened:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_restore_reopens WHERE operation_id=$1 AND new_receipt_id IS NULL)").bind(id).fetch_one(&mut *tx).await?;
            if !reopened {
                return Err(Error::Conflict);
            }
            next = "RECOVERY_REQUIRED".into();
        } else if financial == "RELEASED" && committed {
            return Err(Error::Conflict);
        }
        if financial == "RECOVERY_REQUIRED" && committed {
            if row.try_get::<Option<Uuid>, _>("credit_used_by")?.is_some() {
                return Err(Error::Conflict);
            }
            next = "SETTLED".into();
        }
        if !committed && let Some(credit) = row.try_get::<Option<Uuid>, _>("credit_from")? {
            sqlx::query("UPDATE character_service_orders SET credit_used_by=NULL,updated_at=now() WHERE operation_id=$1 AND credit_used_by=$2").bind(credit).bind(id).execute(&mut *tx).await?;
        }
        let raw = std::str::from_utf8(raw).map_err(|_| Error::BadInput)?;
        // The existing history trigger enforces recovery generations and immutable replay.
        sqlx::query("UPDATE character_commands SET receipt_json=$2,state='TERMINAL',updated_at=now() WHERE id=$1").bind(id).bind(raw).execute(&mut *tx).await?;
        sqlx::query("UPDATE character_service_orders SET financial_state=$2,updated_at=now() WHERE operation_id=$1").bind(id).bind(next).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
fn wallet_error(e: oteryn_wallet::Error) -> Error {
    match e {
        oteryn_wallet::Error::Refused(oteryn_wallet::Refusal::InsufficientFunds) => Error::Denied,
        _ => Error::Unavailable,
    }
}
async fn restore_lock(tx: &mut Transaction<'_, Postgres>) -> Result<(), Error> {
    if sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE")
        .fetch_optional(&mut **tx)
        .await?
        .is_none()
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
async fn now(tx: &mut Transaction<'_, Postgres>) -> Result<u64, Error> {
    let n: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&mut **tx)
        .await?;
    u64::try_from(n).map_err(|_| Error::Unavailable)
}
fn source(v: &oteryn_internal_api::canonical::Value, expected: &str) -> Result<(), Error> {
    use subtle::ConstantTimeEq;
    if expected.is_empty()
        || !bool::from(
            wire::string(v, "source_authority")?
                .as_bytes()
                .ct_eq(expected.as_bytes()),
        )
    {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
fn action_json(action: &Action) -> Result<String, Error> {
    let v = match action {
        Action::Schedule => serde_json::json!({"action":"schedule"}),
        Action::Cancel {
            deletion_operation_id,
        } => {
            wire::identifier(&deletion_operation_id.to_string())?;
            serde_json::json!({"action":"cancel","deletion_operation_id":deletion_operation_id.to_string()})
        }
        Action::Rename { requested_name } => {
            if !(2..=29).contains(&requested_name.len())
                || requested_name
                    .split(' ')
                    .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphabetic()))
            {
                return Err(Error::BadInput);
            }
            serde_json::json!({"action":"rename","requested_name":requested_name})
        }
    };
    Ok(v.to_string())
}
pub use browser::{configured, recent};

pub use transport::router_with_restore;

fn valid_authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

impl Price {
    fn valid(&self) -> bool {
        self.coins > 0
            && self.version > 0
            && !self.product_id.is_empty()
            && self.product_id.len() <= 64
            && self
                .product_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    }
}
