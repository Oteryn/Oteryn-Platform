use super::{Decision, Error, Features, Receipt, valid_id};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{self, Authorizer, CheckContext, Permission},
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Accounts(PgPool);
impl rbac::AccountStateReader for Accounts {
    async fn state(
        &self,
        account: Uuid,
    ) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        let status: Option<String> = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(&self.0)
            .await
            .map_err(|_| rbac::AccountStateUnavailable)?;
        Ok(match status.as_deref() {
            Some("active") => rbac::AccountState::Active,
            _ => rbac::AccountState::Unknown,
        })
    }
}
#[derive(Debug, Clone)]
pub struct Service {
    pool: PgPool,
    features: Features,
}
impl Service {
    pub fn new(pool: PgPool, features: Features) -> Self {
        Self { pool, features }
    }

    async fn staff(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        request: Uuid,
    ) -> Result<(), Error> {
        if !self.features.sanctions {
            return Err(Error::Disabled);
        }
        if !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        valid_id(context.account)?;
        valid_id(request)?;
        sqlx::query("SET LOCAL lock_timeout = '2s'")
            .execute(&mut **tx)
            .await?;
        // Same lock as the public RBAC mutation API: permission revocation cannot race approval.
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut **tx)
            .await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(context.account)
                .fetch_optional(&mut **tx)
                .await?;
        if status.as_deref() != Some("active") {
            return Err(Error::Denied);
        }
        let authorizer = Authorizer::new(self.pool.clone(), Accounts(self.pool.clone()));
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            authorizer.check(context, Permission::ManageSupportEnforcement),
        )
        .await
        .map_err(|_| Error::Unavailable)??;
        Ok(())
    }

    pub async fn create(
        &self,
        context: CheckContext,
        request: Uuid,
        decision: Decision,
    ) -> Result<Receipt, Error> {
        decision.validate()?;
        let mut tx = self.pool.begin().await?;
        self.staff(&mut tx, context, request).await?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(decision.account)
                .fetch_optional(&mut *tx)
                .await?;
        if !matches!(status.as_deref(), Some("active" | "suspended")) {
            return Err(Error::Denied);
        }
        let pending = decision.expires_at_ms.is_none();
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO sanctions (id,account_id,category,status,public_reason,moderator_notes,effective_at,expires_at,created_by) VALUES ($1,$2,$3,$4,$5,$6,to_timestamp($7::bigint::double precision/1000),to_timestamp($8::bigint::double precision/1000),$9)")
            .bind(id).bind(decision.account).bind(decision.category.as_str())
            .bind(if pending { "pending" } else { "active" })
            .bind(decision.public_reason.trim()).bind(decision.moderator_notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
            .bind(decision.effective_at_ms).bind(decision.expires_at_ms).bind(context.account)
            .execute(&mut *tx).await?;
        record(
            &mut tx,
            context,
            request,
            id,
            "support.enforcement_created",
            Value::Null,
            json!({"account_id": decision.account.to_string(), "category": decision.category.as_str(), "pending": pending, "effective_at_ms": decision.effective_at_ms, "expires_at_ms": decision.expires_at_ms, "lock_version": 1}),
        )
        .await?;
        tx.commit().await?;
        Ok(Receipt {
            id,
            version: 1,
            pending,
        })
    }

    /// A pending permanent decision is immutable. Approval always addresses its exact version.
    pub async fn approve(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
    ) -> Result<Receipt, Error> {
        self.transition(context, request, id, version, true).await
    }
    pub async fn revoke(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
    ) -> Result<Receipt, Error> {
        self.transition(context, request, id, version, false).await
    }
    async fn transition(
        &self,
        context: CheckContext,
        request: Uuid,
        id: Uuid,
        version: i64,
        approve: bool,
    ) -> Result<Receipt, Error> {
        valid_id(id)?;
        if version < 1 || version == i64::MAX {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        self.staff(&mut tx, context, request).await?;
        // Lock target before sanction, matching game-access reads and creation.
        let target: Uuid = sqlx::query_scalar("SELECT account_id FROM sanctions WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let target_status: String =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(target)
                .fetch_one(&mut *tx)
                .await?;
        if !matches!(target_status.as_str(), "active" | "suspended") {
            return Err(Error::Denied);
        }
        let row = sqlx::query(
            "SELECT status,created_by,lock_version FROM sanctions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let current: i64 = row.try_get("lock_version")?;
        if current != version {
            return Err(Error::Conflict);
        }
        let status: String = row.try_get("status")?;
        let creator: Uuid = row.try_get("created_by")?;
        if approve && (status != "pending" || creator == context.account)
            || !approve && !matches!(status.as_str(), "pending" | "active" | "expired")
        {
            return Err(Error::Denied);
        }
        let next = if approve { "active" } else { "revoked" };
        sqlx::query("UPDATE sanctions SET status=$2,approved_by=CASE WHEN $3 THEN $4 ELSE approved_by END,lock_version=lock_version+1,updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).bind(approve).bind(context.account).execute(&mut *tx).await?;
        record(
            &mut tx,
            context,
            request,
            id,
            if approve {
                "support.enforcement_approved"
            } else {
                "support.enforcement_revoked"
            },
            json!({"status": status, "lock_version": version}),
            json!({"status": next, "lock_version": version + 1}),
        )
        .await?;
        tx.commit().await?;
        Ok(Receipt {
            id,
            version: version + 1,
            pending: false,
        })
    }
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    context: CheckContext,
    request: Uuid,
    id: Uuid,
    action: &str,
    before: Value,
    after: Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: request,
            },
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await?;
    Ok(())
}

impl Service {
    /// Allocate and record one authorized intent. The caller owns commit/rollback
    /// and must include its decision, audit and outbox in this same transaction.
    /// This low-level API performs no staff authorization or external effects.
    pub async fn allocate_and_record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        intent: &super::SanctionIntent,
    ) -> Result<super::SanctionReceipt, Error> {
        let current = self.lock_revision(tx, intent.account).await?;
        if current == super::MAX_SANCTION_REVISION {
            return Err(Error::RevisionExhausted);
        }
        self.record_locked(tx, intent, current + 1, current).await
    }

    /// Record an explicitly supplied next revision, or replay identical intent.
    /// Gaps, stale revisions and conflicting replays never change stored state.
    pub async fn record_at_revision(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        intent: &super::SanctionIntent,
        revision: i64,
    ) -> Result<super::SanctionReceipt, Error> {
        if !(1..=super::MAX_SANCTION_REVISION).contains(&revision) {
            return Err(Error::InvalidInput);
        }
        let current = self.lock_revision(tx, intent.account).await?;
        self.record_locked(tx, intent, revision, current).await
    }

    async fn lock_revision(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> Result<i64, Error> {
        if !self.features.sanctions {
            return Err(Error::Disabled);
        }
        valid_id(account)?;
        // Match admission's account lock before taking the counter row lock.
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_optional(&mut **tx)
                .await?;
        exists.ok_or(Error::NotFound)?;
        sqlx::query(
            "INSERT INTO support_sanction_counters(account_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(account)
        .execute(&mut **tx)
        .await?;
        Ok(sqlx::query_scalar(
            "SELECT revision FROM support_sanction_counters WHERE account_id=$1 FOR UPDATE",
        )
        .bind(account)
        .fetch_one(&mut **tx)
        .await?)
    }

    async fn record_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        intent: &super::SanctionIntent,
        revision: i64,
        current: i64,
    ) -> Result<super::SanctionReceipt, Error> {
        use super::SanctionAction;
        valid_id(intent.sanction_id)?;
        if let Some(character) = intent.character {
            valid_id(character)?;
        }
        let encoded = serde_json::to_value(intent).map_err(|_| Error::InvalidInput)?;
        let receipt = super::SanctionReceipt {
            sanction_id: intent.sanction_id,
            revision,
        };
        let previous: Option<Value> = sqlx::query_scalar(
            "SELECT intent FROM support_sanction_revisions WHERE account_id=$1 AND revision=$2",
        )
        .bind(intent.account)
        .bind(revision)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(previous) = previous {
            if previous != encoded {
                return Err(Error::RevisionConflict);
            }
            if revision < current {
                return Err(Error::StaleRevision);
            }
            return Ok(receipt);
        }
        if revision <= current {
            return Err(Error::StaleRevision);
        }
        if revision != current + 1 {
            return Err(Error::InvalidInput);
        }
        let (kind, active, expiry) = match intent.action {
            SanctionAction::Ban { expires_at_ms } => ("ban", true, expires_at_ms),
            SanctionAction::Unban => ("ban", false, None),
            SanctionAction::Namelock { expires_at_ms } => ("namelock", true, expires_at_ms),
            SanctionAction::LiftNamelock => ("namelock", false, None),
            SanctionAction::Warning => ("warning", true, None),
            SanctionAction::Notation => ("notation", true, None),
        };
        if kind == "namelock" && intent.character.is_none()
            || matches!(kind, "ban" | "warning") && intent.character.is_some()
        {
            return Err(Error::InvalidInput);
        }
        let decision_at: time::OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut **tx)
            .await?;
        let expires_at = expiry
            .map(|ms| time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000))
            .transpose()
            .map_err(|_| Error::InvalidInput)?;
        if expires_at.is_some_and(|end| end <= decision_at) {
            return Err(Error::InvalidInput);
        }
        let identity = sqlx::query("SELECT account_id,kind,character_id FROM support_sanction_identities WHERE sanction_id=$1")
            .bind(intent.sanction_id).fetch_optional(&mut **tx).await?;
        if let Some(identity) = identity {
            if identity.try_get::<Uuid, _>("account_id")? != intent.account
                || identity.try_get::<String, _>("kind")? != kind
                || identity.try_get::<Option<Uuid>, _>("character_id")? != intent.character
            {
                return Err(Error::Conflict);
            }
        } else {
            if !active {
                return Err(Error::NotFound);
            }
            sqlx::query("INSERT INTO support_sanction_identities(sanction_id,account_id,kind,character_id) VALUES ($1,$2,$3,$4)")
                .bind(intent.sanction_id).bind(intent.account).bind(kind).bind(intent.character)
                .execute(&mut **tx).await?;
        }
        // Game latches revocation per stream identity. A lifted ban or namelock
        // must be issued as a new sanction, even before public metadata exists.
        if active && matches!(kind, "ban" | "namelock") {
            let lifted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM support_sanction_revisions WHERE sanction_id=$1 AND NOT active)")
                .bind(intent.sanction_id).fetch_one(&mut **tx).await?;
            if lifted {
                return Err(Error::Conflict);
            }
        }
        sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(intent.account).bind(revision).bind(intent.sanction_id).bind(encoded).bind(active).bind(decision_at).bind(expires_at)
            .execute(&mut **tx).await?;
        sqlx::query("UPDATE support_sanction_counters SET revision=$2 WHERE account_id=$1")
            .bind(intent.account)
            .bind(revision)
            .execute(&mut **tx)
            .await?;
        Ok(receipt)
    }

    /// Latest state per stable sanction, evaluated using one database statement's
    /// time. Expired restrictions cannot reappear through delayed older revisions.
    pub async fn effective_state(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
        character: Option<Uuid>,
    ) -> Result<super::EffectiveState, Error> {
        if !self.features.sanctions {
            return Err(Error::Disabled);
        }
        valid_id(account)?;
        if let Some(character) = character {
            valid_id(character)?;
        }
        let rows = sqlx::query("WITH latest AS (SELECT DISTINCT ON (sanction_id) sanction_id,revision,active,expires_at FROM support_sanction_revisions WHERE account_id=$1 ORDER BY sanction_id,revision DESC) SELECT c.revision AS account_revision,l.sanction_id,l.revision,l.expires_at,i.kind FROM support_sanction_counters c LEFT JOIN (latest l JOIN support_sanction_identities i ON i.sanction_id=l.sanction_id) ON l.active AND (l.expires_at IS NULL OR l.expires_at > statement_timestamp()) AND (i.kind='ban' OR (i.kind='namelock' AND i.character_id=$2)) WHERE c.account_id=$1 ORDER BY l.revision")
            .bind(account).bind(character).fetch_all(connection).await?;
        let mut state = super::EffectiveState::default();
        for row in rows {
            state.revision = row.try_get("account_revision")?;
            if let Some(sanction_id) = row.try_get::<Option<Uuid>, _>("sanction_id")? {
                let expiry: Option<time::OffsetDateTime> = row.try_get("expires_at")?;
                let entry = super::EffectiveSanction {
                    sanction_id,
                    revision: row.try_get("revision")?,
                    expires_at_ms: expiry.map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64),
                };
                match row.try_get::<String, _>("kind")?.as_str() {
                    "ban" => state.bans.push(entry),
                    "namelock" => state.namelocks.push(entry),
                    _ => return Err(Error::InvalidInput),
                }
            }
        }
        Ok(state)
    }
}
