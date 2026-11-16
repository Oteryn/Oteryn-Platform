//! Signed queries and verified, actor-scoped results over the existing control store.
use super::*;
use crate::control::{self, canonical::Json, signer};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone)]
pub struct StoredChannel {
    pool: PgPool,
    store: control::Store,
    signer: Option<(Arc<signer::KeyRing>, String)>,
    bindings: Vec<crate::control_http::AgentBinding>,
}
struct QuerySpec<'a> {
    operation: &'a str,
    permission: &'a str,
    target: control::Target,
    params: Value,
    risk: control::RiskClass,
}
impl StoredChannel {
    pub fn configured(pool: PgPool) -> Result<Self, Box<dyn std::error::Error>> {
        let config = control::Config::from_env();
        let bindings = if config.enabled {
            crate::control_http::configured_bindings()?
        } else {
            vec![]
        };
        let signer = if config.enabled {
            if let Some(path) = std::env::var_os("PORTAL_ADMIN_CONTROL_SIGNER_KEY_FILE") {
                let id = std::env::var("PORTAL_ADMIN_CONTROL_SIGNER_KEY_ID")?;
                let validity = signer::Validity {
                    not_before: std::env::var("PORTAL_ADMIN_CONTROL_SIGNER_NOT_BEFORE")?.parse()?,
                    not_after: std::env::var("PORTAL_ADMIN_CONTROL_SIGNER_NOT_AFTER")?.parse()?,
                };
                let mut ring = signer::KeyRing::new(true);
                ring.add(signer::AdminKey::from_file(
                    std::path::Path::new(&path),
                    id.clone(),
                    validity,
                )?)?;
                Some((Arc::new(ring), id))
            } else {
                None
            }
        } else {
            None
        };
        Ok(Self {
            store: control::Store::new(pool.clone(), config),
            pool,
            signer,
            bindings,
        })
    }
    async fn owner(
        &self,
        agent: Option<&str>,
        operation: &str,
        target: Option<&str>,
    ) -> Result<control::Agent, Error> {
        let mut candidates = self
            .store
            .agents()
            .await
            .map_err(|_| Error::Unavailable)?
            .into_iter()
            .filter(|a| {
                a.connected
                    && agent.is_none_or(|id| a.agent_id == id)
                    && a.advertised_operations.iter().any(|op| {
                        op.operation == operation
                            && op.operation_version == 1
                            && op.kind == control::Kind::Query
                    })
                    && target.is_none_or(|id| {
                        a.served_worlds
                            .iter()
                            .chain(&a.served_nodes)
                            .any(|v| v == id)
                    })
            });
        let owner = candidates.next().ok_or(Error::Unavailable)?;
        if candidates.next().is_some() {
            return Err(Error::Unavailable);
        }
        // Results require configured verification keys even if transport is authenticated.
        if !self
            .bindings
            .iter()
            .any(|b| b.agent_id == owner.agent_id && b.result_key.is_some())
        {
            return Err(Error::Unavailable);
        }
        Ok(owner)
    }
    fn target(
        agent: &control::Agent,
        target: Option<&str>,
        character: Option<&str>,
    ) -> Result<control::Target, Error> {
        let (world_id, node_id) = if let Some(id) = target {
            if agent.served_worlds.iter().any(|v| v == id) {
                (Some(id.to_owned()), None)
            } else if agent.served_nodes.iter().any(|v| v == id) {
                (None, Some(id.to_owned()))
            } else {
                return Err(Error::Unavailable);
            }
        } else if agent.served_nodes.len() == 1 {
            (None, agent.served_nodes.first().cloned())
        } else if agent.served_worlds.len() == 1 {
            (agent.served_worlds.first().cloned(), None)
        } else {
            return Err(Error::Unavailable);
        };
        Ok(control::Target {
            world_id,
            node_id,
            character_id: character.map(str::to_owned),
            account_ref: None,
        })
    }
    async fn submit_query(
        &self,
        actor: CheckContext,
        session: &BrowserSession,
        agent: &control::Agent,
        query: QuerySpec<'_>,
    ) -> Result<Uuid, Error> {
        if actor.account.is_nil() || !actor.require_staff_mfa || !actor.mfa_verified {
            return Err(Error::Denied);
        }
        let QuerySpec {
            operation,
            permission,
            target,
            params,
            risk,
        } = query;
        let (ring, key_id) = self.signer.as_ref().ok_or(Error::Unavailable)?;
        let session_hash = session.identity_hash().await.ok_or(Error::Denied)?;
        let now = OffsetDateTime::now_utc();
        let expires = now + time::Duration::seconds(60);
        let request_id = Uuid::now_v7();
        let stamp = now.format(&Rfc3339).map_err(|_| Error::Unavailable)?;
        let mut tx = self.pool.begin().await?;
        let auth = rbac::Authorizer::new(
            self.pool.clone(),
            web::PortalIdentity::configured(self.pool.clone()),
        )
        .with_game_flags(rbac::GamePermissionFlags {
            enabled: true,
            deploy: false,
        });
        rbac::authorize_mutation(
            &mut tx,
            &[actor.account],
            &auth,
            actor,
            &[
                Permission::Access,
                permission.parse().map_err(|_| Error::Denied)?,
            ],
        )
        .await
        .map_err(|error| match error {
            rbac::Error::Storage(_) => Error::Unavailable,
            _ => Error::Denied,
        })?;
        control::store::ownership_lock(&mut tx, true)
            .await
            .map_err(|_| Error::Unavailable)?;
        let owns: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_agents a WHERE a.agent_id=$1 AND a.owner_epoch=$2 AND a.last_seen_at>clock_timestamp()-interval '30 seconds' AND a.last_seen_at<=clock_timestamp() AND ($3::text IS NULL OR $3=ANY(a.served_worlds)) AND ($4::text IS NULL OR $4=ANY(a.served_nodes)) AND a.advertised_operations @> $5 AND NOT EXISTS(SELECT 1 FROM admin_control_agents b WHERE b.agent_id<>a.agent_id AND b.owner_epoch>=a.owner_epoch AND ($3=ANY(b.served_worlds) OR $4=ANY(b.served_nodes))))")
            .bind(&agent.agent_id).bind(agent.owner_epoch).bind(&target.world_id).bind(&target.node_id)
            .bind(json!([{"operation":operation,"operation_version":1,"kind":"query"}])).fetch_one(&mut *tx).await?;
        if !owns {
            return Err(Error::Unavailable);
        }
        auth.check_in(&mut tx, actor, Permission::Access)
            .await
            .map_err(|_| Error::Denied)?;
        auth.check_in(
            &mut tx,
            actor,
            permission.parse().map_err(|_| Error::Denied)?,
        )
        .await
        .map_err(|_| Error::Denied)?;
        let grant = session
            .get_in(
                &mut tx,
                oteryn_web::session::SessionKey::<String>::new(
                    oteryn_identity::mfa::SESSION_GRANT_KEY,
                ),
            )
            .await?
            .ok_or(Error::Denied)?;
        let evidence: Option<(OffsetDateTime, String)> = sqlx::query_as("SELECT t.created_at,COALESCE(s.ip::text,'') FROM account_mfa_tokens t JOIN web_sessions s ON s.account_id=t.account_id WHERE t.token_hash=$1 AND t.account_id=$2 AND t.purpose='grant' AND t.created_at<=clock_timestamp() AND t.expires_at>clock_timestamp() AND s.id_hash=$3 AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp() FOR SHARE OF t,s")
            .bind(Sha256::digest(grant.as_bytes()).to_vec()).bind(actor.account).bind(&session_hash).fetch_optional(&mut *tx).await?;
        let (mfa_at, ip) = evidence.ok_or(Error::Denied)?;
        let roles = sqlx::query_scalar("SELECT r.key FROM roles r JOIN account_roles a ON a.role_id=r.id WHERE a.account_id=$1 ORDER BY r.key").bind(actor.account).fetch_all(&mut *tx).await?;
        let mut envelope = signer::RequestEnvelope {
            contract: "oteryn.admin-control.v1".into(),
            request_id,
            admin_operation_id: request_id,
            operation: operation.into(),
            operation_version: 1,
            kind: signer::Kind::Query,
            target: signer::Target {
                world_id: target.world_id.clone(),
                node_id: target.node_id.clone(),
                character_id: target.character_id.clone(),
                account_ref: None,
            },
            params: Json(params.clone()),
            dry_run: false,
            actor: signer::Actor {
                admin_id: actor.account.to_string(),
                roles,
                permission: permission.into(),
                mfa_at: mfa_at.format(&Rfc3339).map_err(|_| Error::Unavailable)?,
                step_up_at: None,
                ip_hash: format!("{:x}", Sha256::digest(ip.as_bytes())),
                session_ref: format!("{:x}", Sha256::digest(&session_hash)),
            },
            reason: None,
            approvals: vec![],
            risk_class: if risk == control::RiskClass::R0 {
                signer::RiskClass::R0
            } else {
                signer::RiskClass::R1
            },
            issued_at: stamp,
            expires_at: expires.format(&Rfc3339).map_err(|_| Error::Unavailable)?,
            nonce: URL_SAFE_NO_PAD.encode(request_id.as_bytes()),
            signer_key_id: key_id.clone(),
            signature: String::new(),
        };
        ring.sign(&mut envelope, key_id, now.unix_timestamp())
            .map_err(|_| Error::Unavailable)?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_control_requests WHERE actor=$1 AND kind='query' AND status IN ('queued','delivered','accepted','in_progress') AND expires_at>clock_timestamp()")
            .bind(actor.account).fetch_one(&mut *tx).await?;
        if count >= 4 {
            return Err(Error::Unavailable);
        }
        self.store
            .enqueue_in(
                &mut tx,
                &control::NewRequest {
                    request_id,
                    admin_operation_id: request_id,
                    operation: operation.into(),
                    operation_version: 1,
                    kind: control::Kind::Query,
                    target,
                    params,
                    params_digest: signer::params_digest(&envelope.params)
                        .map_err(|_| Error::Unavailable)?
                        .to_vec(),
                    dry_run: false,
                    risk_class: risk,
                    actor: actor.account,
                    reason: json!({}),
                    approvals: vec![],
                    issued_at: now,
                    expires_at: expires,
                    nonce: request_id.as_bytes().to_vec(),
                    signer_key_id: key_id.clone(),
                    canonical_envelope: signer::adm_x1_signing_bytes(&envelope)
                        .map_err(|_| Error::Unavailable)?,
                    signature: URL_SAFE_NO_PAD
                        .decode(&envelope.signature)
                        .map_err(|_| Error::Unavailable)?,
                },
            )
            .await
            .map_err(|_| Error::Unavailable)?;
        crate::audit::append(
            &mut tx,
            crate::audit::Entry {
                context: crate::audit::AuditContext {
                    actor: Some(actor.account),
                    request_id,
                },
                action: "console.query.admitted",
                target: &agent.agent_id,
                before: json!(null),
                after: json!({"owner_epoch":agent.owner_epoch}),
            },
        )
        .await
        .map_err(|_| Error::Unavailable)?;
        auth.check_in(&mut tx, actor, Permission::Access)
            .await
            .map_err(|_| Error::Denied)?;
        auth.check_in(
            &mut tx,
            actor,
            permission.parse().map_err(|_| Error::Denied)?,
        )
        .await
        .map_err(|_| Error::Denied)?;
        tx.commit().await?;
        Ok(request_id)
    }
    async fn verified(
        &self,
        actor: Uuid,
        agent: &control::Agent,
        operation: &str,
        target: &control::Target,
        params: &Value,
        request_id: Option<Uuid>,
    ) -> Result<Option<(control::Request, signer::ResultEnvelope)>, Error> {
        let owns: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM admin_control_agents WHERE agent_id<>$1 AND owner_epoch>=$2 AND ($3::text=ANY(served_worlds) OR $4::text=ANY(served_nodes)))")
            .bind(&agent.agent_id).bind(agent.owner_epoch).bind(&target.world_id).bind(&target.node_id).fetch_one(&self.pool).await?;
        if !owns {
            return Err(Error::Unavailable);
        }
        let request: Option<control::Request> = sqlx::query_as("SELECT r.* FROM admin_control_requests r JOIN audit_log a ON a.request_id=r.request_id AND a.action='console.query.admitted' JOIN admin_control_agents g ON g.agent_id=$6 AND g.owner_epoch::text=$7 AND g.last_seen_at>clock_timestamp()-interval '30 seconds' AND g.last_seen_at<=clock_timestamp() AND (r.target->>'world_id' IS NULL OR r.target->>'world_id'=ANY(g.served_worlds)) AND (r.target->>'node_id' IS NULL OR r.target->>'node_id'=ANY(g.served_nodes)) AND NOT EXISTS(SELECT 1 FROM admin_control_agents other WHERE other.agent_id<>g.agent_id AND other.owner_epoch>=g.owner_epoch AND (r.target->>'world_id'=ANY(other.served_worlds) OR r.target->>'node_id'=ANY(other.served_nodes))) WHERE r.actor=$1 AND r.operation=$2 AND r.target=$3 AND r.params=$4 AND ($5::uuid IS NULL OR r.request_id=$5) AND r.expires_at>clock_timestamp() AND r.issued_at<=clock_timestamp() AND a.target=$6 AND a.after->>'owner_epoch'=$7 ORDER BY r.issued_at DESC LIMIT 1")
            .bind(actor).bind(operation).bind(sqlx::types::Json(target)).bind(params).bind(request_id).bind(&agent.agent_id).bind(agent.owner_epoch.to_string()).fetch_optional(&self.pool).await?;
        let Some(request) = request else {
            return Ok(None);
        };
        if request.status != control::Status::Succeeded
            || request.agent_id.as_deref() != Some(&agent.agent_id)
        {
            return Ok(None);
        }
        let rows = self
            .store
            .results(request.request_id, 1)
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let result: signer::ResultEnvelope =
            serde_json::from_value(row.payload.clone()).map_err(|_| Error::Unavailable)?;
        let (key, validity) = self
            .bindings
            .iter()
            .find(|b| b.agent_id == agent.agent_id)
            .and_then(|b| b.result_key)
            .ok_or(Error::Unavailable)?;
        signer::verify_result(
            &result,
            &key,
            validity,
            OffsetDateTime::now_utc().unix_timestamp(),
        )
        .map_err(|_| Error::Unavailable)?;
        let completed = result
            .completed_at
            .as_deref()
            .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
            .ok_or(Error::Unavailable)?;
        if result.request_id != request.request_id
            || result.admin_operation_id != request.admin_operation_id
            || result.agent_id != agent.agent_id
            || result.status != signer::Status::Succeeded
            || completed < request.issued_at
            || completed > request.expires_at
            || completed > OffsetDateTime::now_utc()
            || !bounded_json(&result.result.0, 0)
        {
            return Err(Error::Unavailable);
        }
        Ok(Some((request, result)))
    }
    pub(super) async fn snapshot(
        &self,
        actor: CheckContext,
        session: Option<&BrowserSession>,
        operation: &str,
        permission: &str,
        character: Option<&str>,
        params: Value,
    ) -> Result<Option<(Uuid, String, Value)>, access::Error> {
        let agent = self
            .owner(None, operation, None)
            .await
            .map_err(|_| access::Error::Unavailable)?;
        let target =
            Self::target(&agent, None, character).map_err(|_| access::Error::Unavailable)?;
        if let Some((request, result)) = self
            .verified(actor.account, &agent, operation, &target, &params, None)
            .await
            .map_err(|_| access::Error::Unavailable)?
        {
            return Ok(Some((
                request.request_id,
                result.completed_at.unwrap_or_default(),
                result.result.0,
            )));
        }
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_requests WHERE actor=$1 AND operation=$2 AND target=$3 AND params=$4 AND expires_at>clock_timestamp() AND status IN ('queued','delivered','accepted','in_progress'))")
            .bind(actor.account).bind(operation).bind(sqlx::types::Json(&target)).bind(&params).fetch_one(&self.pool).await?;
        if !pending && let Some(session) = session {
            self.submit_query(
                actor,
                session,
                &agent,
                QuerySpec {
                    operation,
                    permission,
                    target,
                    params,
                    risk: control::RiskClass::R1,
                },
            )
            .await
            .map_err(|_| access::Error::Unavailable)?;
        }
        Ok(None)
    }
}
fn bounded_json(value: &Value, depth: usize) -> bool {
    if depth > 12 {
        return false;
    }
    match value {
        Value::String(s) => s.len() <= 512 && !s.chars().any(char::is_control),
        Value::Array(a) => a.len() <= 100 && a.iter().all(|v| bounded_json(v, depth + 1)),
        Value::Object(o) => {
            o.len() <= 64
                && o.iter()
                    .all(|(k, v)| k.len() <= 80 && bounded_json(v, depth + 1))
        }
        _ => true,
    }
}
impl Channel for StoredChannel {
    async fn read(
        &self,
        actor: Uuid,
        agent: &str,
        operation: Operation,
        target: Option<&str>,
        request: Option<Uuid>,
    ) -> Result<AgentView, Error> {
        let owner = self.owner(Some(agent), operation.name(), target).await?;
        let scope = Self::target(&owner, target, None)?;
        let verified = self
            .verified(actor, &owner, operation.name(), &scope, &json!({}), request)
            .await?;
        let mut view = AgentView {
            owner_epoch: owner.owner_epoch as u64,
            last_contact: SystemTime::UNIX_EPOCH
                + Duration::from_secs(owner.last_seen_at.unix_timestamp().max(0) as u64),
            current_owner: true,
            operations: vec![(operation, 1)],
            result: None,
            status: QueryStatus::Pending,
        };
        if let Some((request, result)) = verified {
            let records = super::projection::records(operation, &result.result.0)
                .map_err(|_| Error::Unavailable)?;
            view.status = QueryStatus::Succeeded;
            view.result = Some(CachedResult {
                owner_epoch: view.owner_epoch,
                request_id: request.request_id,
                actor,
                agent: agent.into(),
                operation,
                target: target.map(str::to_owned),
                observed_at: SystemTime::UNIX_EPOCH
                    + Duration::from_secs(request.issued_at.unix_timestamp().max(0) as u64),
                revision: result.effective_revision.unwrap_or_default(),
                records,
            });
        }
        Ok(view)
    }
    async fn enqueue(
        &self,
        context: CheckContext,
        session: &BrowserSession,
        agent: &str,
        operation: Operation,
        target: Option<&str>,
    ) -> Result<Uuid, Error> {
        let owner = self.owner(Some(agent), operation.name(), target).await?;
        let scope = Self::target(&owner, target, None)?;
        self.submit_query(
            context,
            session,
            &owner,
            QuerySpec {
                operation: operation.name(),
                permission: operation.permission(),
                target: scope,
                params: json!({}),
                risk: control::RiskClass::R0,
            },
        )
        .await
    }
}

#[cfg(test)]
#[path = "stored_tests.rs"]
mod tests;
