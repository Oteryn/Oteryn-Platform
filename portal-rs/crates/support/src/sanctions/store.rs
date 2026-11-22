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
    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .fetch_optional(connection)
                .await
                .map_err(|_| rbac::AccountStateUnavailable)?;
        Ok(match status.as_deref() {
            Some("active") => rbac::AccountState::Active,
            _ => rbac::AccountState::Unknown,
        })
    }
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

    fn staff_input(&self, context: CheckContext, request: Uuid) -> Result<(), Error> {
        if !self.features.sanctions {
            return Err(Error::Disabled);
        }
        if !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        valid_id(context.account)?;
        valid_id(request)?;
        Ok(())
    }

    // Preserve admission/refusal order without retaining an actor SHARE lock
    // while waiting for the ordered domain locks. Recheck under RBAC below.
    async fn staff_preflight(&self, context: CheckContext, request: Uuid) -> Result<(), Error> {
        self.staff_input(context, request)?;
        let status: Option<String> = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1")
            .bind(context.account)
            .fetch_optional(&self.pool)
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

    async fn staff(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        request: Uuid,
    ) -> Result<(), Error> {
        self.staff_input(context, request)?;
        rbac::mutation_budget(tx).await?;
        // Same lock as the public RBAC mutation API: permission revocation cannot race approval.
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
            authorizer.check_in(tx, context, Permission::ManageSupportEnforcement),
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
        self.staff_preflight(context, request).await?;
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        let auth = Authorizer::new(self.pool.clone(), Accounts(self.pool.clone()));
        rbac::authorize_mutation(
            &mut tx,
            &[context.account, decision.account],
            &auth,
            context,
            &[Permission::ManageSupportEnforcement],
        )
        .await?;
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
        self.staff(&mut tx, context, request).await?;
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
        self.staff_preflight(context, request).await?;
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        // Lock target before the RBAC fence and sanction, matching ticket writes.
        let target: Uuid = sqlx::query_scalar("SELECT account_id FROM sanctions WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let auth = Authorizer::new(self.pool.clone(), Accounts(self.pool.clone()));
        rbac::authorize_mutation_with_resources(
            &mut tx,
            &[context.account, target],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM sanctions WHERE id=$1 FOR UPDATE")
                        .bind(id)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            &auth,
            context,
            &[Permission::ManageSupportEnforcement],
        )
        .await?;
        self.staff(&mut tx, context, request).await?;
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
        // Revalidate the durable session and grant after the sanction-row wait.
        self.staff(&mut tx, context, request).await?;
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
        self.staff(&mut tx, context, request).await?;
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
        // Expired namelocks also latch revocation when the export is polled.
        if active && matches!(kind, "ban" | "namelock") {
            let lifted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM support_sanction_revisions WHERE sanction_id=$1 AND NOT active) OR ($2 AND COALESCE((SELECT expires_at<=$3 FROM support_sanction_revisions WHERE sanction_id=$1 ORDER BY revision DESC LIMIT 1),false))")
                .bind(intent.sanction_id).bind(kind == "namelock").bind(decision_at).fetch_one(&mut **tx).await?;
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
        let rows = sqlx::query("WITH latest AS (SELECT DISTINCT ON (sanction_id) sanction_id,revision,active,expires_at FROM support_sanction_revisions WHERE account_id=$1 ORDER BY sanction_id,revision DESC) SELECT c.revision AS account_revision,l.sanction_id,l.revision,l.expires_at,i.kind FROM support_sanction_counters c LEFT JOIN (latest l JOIN support_sanction_identities i ON i.sanction_id=l.sanction_id) ON l.active AND (l.expires_at IS NULL OR l.expires_at > statement_timestamp()) AND (i.kind='ban' OR (i.kind='namelock' AND i.character_id=$2)) WHERE c.account_id=$1 ORDER BY l.revision LIMIT 1001")
            .bind(account).bind(character).fetch_all(connection).await?;
        if rows.len() > 1000 {
            return Err(Error::Conflict);
        }
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

impl Service {
    /// Platform moderation only. This seam never enables or emits game commands.
    /// Caller commits the admin authorization, returned audit diff and notifications together.
    pub async fn staff_apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        binding: (Uuid, i64),
        command: &oteryn_admin::support::Command,
        notifications: &oteryn_notify::Notifications,
    ) -> Result<(Value, Value), oteryn_admin::support::Error> {
        let actor = context.account;
        use oteryn_admin::support::{Command, Error as StaffError};
        let (id, version) = binding;
        valid_id(id).map_err(|_| StaffError::Invalid)?;
        if version < 1
            || version
                .checked_add(1 + i64::from(matches!(command, Command::Appeal { .. })))
                .is_none()
        {
            return Err(StaffError::Invalid);
        }

        command.validate()?;
        if !self.features.sanctions {
            return Err(StaffError::NotFound);
        }
        rbac::mutation_budget(tx)
            .await
            .map_err(|_| StaffError::Unavailable)?;
        let target = match command {
            Command::Sanction {
                create: true,
                target,
                ..
            }
            | Command::ApplySanction { target, .. } => *target,
            _ => sqlx::query_scalar("SELECT account_id FROM sanctions WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?
                .ok_or(StaffError::NotFound)?,
        };
        let authorizer = Authorizer::new(
            self.pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(self.pool.clone()),
        );
        rbac::authorize_mutation(
            tx,
            &[actor, target],
            &authorizer,
            context,
            &[Permission::Access, Permission::ManageSupportEnforcement],
        )
        .await
        .map_err(|error| match error {
            rbac::Error::Storage(_) => StaffError::Unavailable,
            _ => StaffError::Denied,
        })?;
        for permission in [Permission::Access, Permission::ManageSupportEnforcement] {
            authorizer
                .check_in(tx, context, permission)
                .await
                .map_err(|error| match error {
                    rbac::Error::Storage(_) => StaffError::Unavailable,
                    _ => StaffError::Denied,
                })?;
        }
        let state: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(target)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?;
        if !matches!(state.as_deref(), Some("active" | "suspended")) {
            return Err(StaffError::Denied);
        }
        let current:Option<(String,i64,Uuid,String,i64,Option<i64>)>=sqlx::query_as("SELECT status,lock_version,account_id,category,(extract(epoch FROM effective_at)*1000)::bigint,(extract(epoch FROM expires_at)*1000)::bigint FROM sanctions WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
        // Bind staff decisions to both the sanction and the appeal they viewed.
        // Player acknowledgements/resubmissions advance only the workflow counter.
        let workflow: Option<(String,i64)> = sqlx::query_as("SELECT status,version FROM support_appeal_workflows WHERE sanction_id=$1 AND owner_id=$2 FOR UPDATE")
            .bind(id).bind(target).fetch_optional(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
        if let Some(row) = &current {
            let shown = row
                .1
                .checked_add(workflow.as_ref().map_or(1, |w| w.1) - 1)
                .ok_or(StaffError::Conflict)?;
            if shown != version {
                return Err(StaffError::Conflict);
            }
        }
        let timed_command;
        let command = if let Command::ApplySanction {
            target,
            category,
            public_reason,
            moderator_notes,
            duration_hours,
        } = command
        {
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(|_| StaffError::Unavailable)?;
            timed_command = Command::Sanction {
                create: true,
                status: "active".into(),
                target: *target,
                category: category.clone(),
                public_reason: public_reason.clone(),
                moderator_notes: moderator_notes.clone(),
                effective_at: now,
                expires_at: now + duration_hours * 3600,
            };
            &timed_command
        } else {
            command
        };
        let before = current.as_ref().map_or(Value::Null,|r|json!({"status":r.0,"lock_version":version,"sanction_version":r.1,"appeal_version":workflow.as_ref().map(|w|w.1),"category":r.3,"effective_at_ms":r.4,"expires_at_ms":r.5}));
        let next = match command {
            Command::Sanction {
                create,
                status,
                target: submitted,
                category,
                public_reason,
                moderator_notes,
                effective_at,
                expires_at,
            } => {
                if *submitted != target {
                    return Err(StaffError::Invalid);
                }
                let decision = Decision {
                    account: target,
                    category: super::Category::parse(category).map_err(|_| StaffError::Invalid)?,
                    public_reason: public_reason.clone(),
                    moderator_notes: Some(moderator_notes.clone()),
                    effective_at_ms: effective_at.checked_mul(1000).ok_or(StaffError::Invalid)?,
                    expires_at_ms: Some(expires_at.checked_mul(1000).ok_or(StaffError::Invalid)?),
                };
                decision.validate().map_err(|_| StaffError::Invalid)?;
                if *create {
                    if current.is_some() {
                        return Err(StaffError::Conflict);
                    }
                    if version != 1
                        || status != "active"
                        || !oteryn_admin::support::valid_sanction_duration(
                            expires_at - effective_at,
                        )
                    {
                        return Err(StaffError::Invalid);
                    }
                    command.transition("absent")?;
                    let inserted = sqlx::query("INSERT INTO sanctions(id,account_id,category,status,public_reason,moderator_notes,effective_at,expires_at,created_by) SELECT $1,$2,$3,'active',$4,nullif(btrim($5),''),to_timestamp($6::bigint::double precision/1000),to_timestamp($7::bigint::double precision/1000),$8
                        WHERE to_timestamp($7::bigint::double precision/1000)>clock_timestamp()
                        ON CONFLICT (id) DO NOTHING")
                        .bind(id).bind(target).bind(category).bind(public_reason.trim()).bind(moderator_notes).bind(decision.effective_at_ms).bind(decision.expires_at_ms).bind(actor).execute(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
                    if inserted.rows_affected() != 1 {
                        return Err(StaffError::Conflict);
                    }
                    crate::appeals::Appeals { enabled: true }
                        .register(tx, id, target)
                        .await
                        .map_err(|_| StaffError::Unavailable)?;
                } else {
                    let row = current.as_ref().ok_or(StaffError::NotFound)?;
                    command.transition(&row.0)?;
                    // Decisions cannot be retargeted, reclassified, rescheduled or revived.
                    // A replacement decision gets a new identity through create.
                    if row.2 != target
                        || row.3 != *category
                        || (status == "active"
                            && row.5.is_some()
                            && (row.4 / 1000 != *effective_at
                                || row.5.map(|time| time / 1000) != Some(*expires_at)))
                    {
                        return Err(StaffError::Invalid);
                    }
                    if (matches!(row.0.as_str(), "revoked" | "expired") && *status != row.0)
                        || (status == "expired"
                            && !sqlx::query_scalar::<_, bool>(
                                "SELECT COALESCE(expires_at<=clock_timestamp(),false) FROM sanctions WHERE id=$1",
                            )
                            .bind(id)
                            .fetch_one(&mut **tx)
                            .await
                            .map_err(|_| StaffError::Unavailable)?)
                    {
                        return Err(StaffError::Conflict);
                    }
                    let updated = sqlx::query(
                        "UPDATE sanctions
                         SET status=$2,public_reason=$3,moderator_notes=nullif(btrim($4),''),
                             lock_version=lock_version+1,updated_at=clock_timestamp()
                         WHERE id=$1 AND ($2 NOT IN ('active','revoked') OR expires_at IS NULL OR expires_at>clock_timestamp())",
                    )
                    .bind(id)
                    .bind(status)
                    .bind(public_reason.trim())
                    .bind(moderator_notes)
                    .execute(&mut **tx)
                    .await
                    .map_err(|_| StaffError::Unavailable)?;
                    if updated.rows_affected() != 1 {
                        return Err(StaffError::Conflict);
                    }
                }
                status.clone()
            }
            Command::RevokeSanction { .. } => {
                let row = current.as_ref().ok_or(StaffError::NotFound)?;
                command.transition(&row.0)?;
                let revoked = sqlx::query(
                    "UPDATE sanctions SET status='revoked',lock_version=lock_version+1,updated_at=clock_timestamp()
                     WHERE id=$1 AND (expires_at IS NULL OR expires_at>clock_timestamp())",
                )
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?;
                if revoked.rows_affected() != 1 {
                    return Err(StaffError::Conflict);
                }
                "revoked".to_owned()
            }
            Command::Appeal {
                appeal_status,
                appeal_outcome,
            } => {
                let row = current.as_ref().ok_or(StaffError::NotFound)?;
                let workflow = workflow.as_ref().ok_or(StaffError::NotFound)?;
                command.transition(&workflow.0)?;
                let status = match appeal_status.as_str() {
                    "reviewing" => crate::appeals::Status::Reviewing,
                    "accepted" => crate::appeals::Status::Accepted,
                    "rejected" => crate::appeals::Status::Rejected,
                    _ => return Err(StaffError::Invalid),
                };
                crate::appeals::Appeals { enabled: true }
                    .change(
                        tx,
                        target,
                        id,
                        workflow.1,
                        crate::appeals::Action::Decide {
                            status,
                            outcome: appeal_outcome,
                        },
                    )
                    .await
                    .map_err(|error| match error {
                        crate::appeals::Error::Conflict => StaffError::Conflict,
                        crate::appeals::Error::Invalid => StaffError::Invalid,
                        _ => StaffError::Unavailable,
                    })?;
                let next = if status == crate::appeals::Status::Accepted {
                    "revoked"
                } else {
                    &row.0
                };
                sqlx::query("UPDATE sanctions SET status=$2,lock_version=lock_version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(next).execute(&mut **tx).await.map_err(|_|StaffError::Unavailable)?;
                next.to_owned()
            }
            _ => return Err(StaffError::Invalid),
        };
        let body = match command {
            Command::Sanction { public_reason, .. } | Command::RevokeSanction { public_reason } => {
                public_reason.trim().to_owned()
            }
            Command::Appeal {
                appeal_status,
                appeal_outcome,
            } => format!("{appeal_status}: {}", appeal_outcome.trim()),
            _ => return Err(StaffError::Invalid),
        };
        crate::tickets::staff::notify(tx, notifications, target, body).await?;
        let (category, effective_at_ms, expires_at_ms) = if let Some(row) = &current {
            (row.3.as_str(), row.4, row.5)
        } else if let Command::Sanction {
            category,
            effective_at,
            expires_at,
            ..
        } = command
        {
            (
                category.as_str(),
                effective_at.checked_mul(1000).ok_or(StaffError::Invalid)?,
                Some(expires_at.checked_mul(1000).ok_or(StaffError::Invalid)?),
            )
        } else {
            return Err(StaffError::Invalid);
        };
        let appeal_status = match command {
            Command::Appeal { appeal_status, .. } => Some(appeal_status.as_str()),
            _ => Some(workflow.as_ref().map_or("none", |w| w.0.as_str())),
        };
        Ok((
            before,
            json!({"status":next,"account_id":target,"category":category,"effective_at_ms":effective_at_ms,"expires_at_ms":expires_at_ms,"appeal_status":appeal_status,"lock_version":if current.is_none(){1}else{version+1+i64::from(matches!(command,Command::Appeal{..}))}}),
        ))
    }
}
