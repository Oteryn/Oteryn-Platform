//! Account-owner MFA changes. The account lock serializes proofs, credentials and revocation.
use super::{Mfa, State, Tx, error, now};
use crate::{AccountId, sessions};
use oteryn_notify::EmailTemplate;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagementOperation {
    Replace,
    ConfirmReplacement,
    Rotate,
    Disable,
}

pub enum ManagementOutcome {
    ReplacementStarted,
    Codes(Vec<String>),
    Disabled,
}

async fn live_session(
    conn: &mut PgConnection,
    actor: &sessions::Actor,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT s.session_id FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.account_id=$2 AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp() AND (s.authorization_generation IS NULL OR s.authorization_generation=a.native_security_generation) FOR UPDATE OF s")
        .bind(&actor.hash).bind(actor.account).fetch_optional(conn).await
}

async fn pending(
    conn: &mut PgConnection,
    session_id: Uuid,
    generation: Uuid,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_scalar("SELECT secret FROM account_mfa_replacements WHERE session_id=$1 AND factor_generation=$2 AND expires_at>clock_timestamp()")
        .bind(session_id).bind(generation).fetch_optional(conn).await
}

impl Mfa {
    pub async fn replacement_secret(
        &self,
        actor: &sessions::Actor,
    ) -> Result<Option<String>, sqlx::Error> {
        let secret: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT r.secret FROM account_mfa_replacements r
             JOIN web_sessions s ON s.session_id=r.session_id
             JOIN accounts a ON a.id=s.account_id
             JOIN account_mfa_totp m ON m.account_id=a.id AND m.generation=r.factor_generation
             WHERE s.id_hash=$1 AND a.id=$2 AND a.status='active' AND a.email_verified_at IS NOT NULL
             AND m.confirmed AND r.expires_at>clock_timestamp()
             AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp()
             AND (s.authorization_generation IS NULL OR s.authorization_generation=a.native_security_generation)",
        )
        .bind(&actor.hash)
        .bind(actor.account)
        .fetch_optional(&self.pool)
        .await?;
        secret
            .map(|value| {
                Ok(totp_rs::Secret::Raw(self.decrypt(actor.account, &value)?)
                    .to_encoded()
                    .to_string())
            })
            .transpose()
    }

    async fn manage_proof(
        &self,
        tx: &mut Tx<'_>,
        id: AccountId,
        state: &State,
        code: &str,
    ) -> Result<bool, sqlx::Error> {
        if let Some(step) = self.step(id, state, code, now()?)? {
            sqlx::query(
                "UPDATE account_mfa_totp SET last_step=$2,updated_at=now() WHERE account_id=$1",
            )
            .bind(id)
            .bind(step)
            .execute(&mut **tx)
            .await?;
            return Ok(true);
        }
        Self::consume_recovery(tx, id, code.trim()).await
    }

    /// The current password and one unused factor proof authorize one operation.
    /// Replacement confirmation uses the short-lived proof bound to this browser.
    pub async fn manage(
        &self,
        actor: &sessions::Actor,
        operation: ManagementOperation,
        password: &str,
        code: &str,
    ) -> Result<Option<ManagementOutcome>, sqlx::Error> {
        use ManagementOperation::*;
        let id = actor.account;
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        let Some(hash) = Self::lock(&mut tx, id).await? else {
            return Ok(None);
        };
        let Some(state) = Self::state(&mut tx, id).await?.filter(|s| s.confirmed) else {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        };
        if operation != ConfirmReplacement && !Self::password(password, &hash).await? {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }
        let Some(session_id) = live_session(&mut tx, actor).await? else {
            return Ok(None);
        };
        if operation != ConfirmReplacement && !self.manage_proof(&mut tx, id, &state, code).await? {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }

        let (outcome, event, notification) = match operation {
            Replace => {
                let secret = self.encrypt(id, &Self::new_secret()?)?;
                sqlx::query("INSERT INTO account_mfa_replacements(session_id,factor_generation,secret) VALUES($1,$2,$3) ON CONFLICT(session_id) DO UPDATE SET factor_generation=$2,secret=$3,expires_at=EXCLUDED.expires_at")
                    .bind(session_id).bind(state.generation).bind(secret).execute(&mut *tx).await?;
                Self::audit(&mut tx, Some(id), "mfa.replacement_started").await?;
                tx.commit().await?;
                return Ok(Some(ManagementOutcome::ReplacementStarted));
            }
            ConfirmReplacement => {
                let Some(secret) = pending(&mut tx, session_id, state.generation).await? else {
                    Self::refuse(tx, Some(id)).await?;
                    return Ok(None);
                };
                let replacement = State {
                    secret,
                    last_step: -1,
                    ..state
                };
                let Some(step) = self.step(id, &replacement, code, now()?)? else {
                    Self::refuse(tx, Some(id)).await?;
                    return Ok(None);
                };
                sqlx::query("UPDATE account_mfa_totp SET secret=$2,last_step=$3,generation=uuidv7(),updated_at=now() WHERE account_id=$1")
                    .bind(id).bind(replacement.secret).bind(step).execute(&mut *tx).await?;
                (
                    ManagementOutcome::Codes(Self::replace_recovery_codes(&mut tx, id).await?),
                    "mfa.replaced",
                    EmailTemplate::MfaReplaced,
                )
            }
            Rotate => {
                // A code-only change does not invoke the TOTP credential trigger.
                sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1")
                    .bind(id).execute(&mut *tx).await?;
                (
                    ManagementOutcome::Codes(Self::replace_recovery_codes(&mut tx, id).await?),
                    "mfa.recovery_codes_rotated",
                    EmailTemplate::MfaRecoveryCodesRotated,
                )
            }
            Disable => {
                for statement in [
                    "UPDATE accounts SET login_mfa_required=false,updated_at=now() WHERE id=$1",
                    "DELETE FROM account_mfa_totp WHERE account_id=$1",
                    "DELETE FROM account_recovery_codes WHERE account_id=$1",
                ] {
                    sqlx::query(statement).bind(id).execute(&mut *tx).await?;
                }
                (
                    ManagementOutcome::Disabled,
                    "mfa.disabled",
                    EmailTemplate::MfaDisabled,
                )
            }
        };
        sessions::revoke_all_authorizations(&mut tx, id).await?;
        Self::audit(&mut tx, Some(id), event).await?;
        sessions::security_notifications(self.pool.clone())
            .map_err(|_| error())?
            .enqueue(&mut tx, id, &notification)
            .await
            .map_err(|_| error())?;
        tx.commit().await?;
        Ok(Some(outcome))
    }
}
