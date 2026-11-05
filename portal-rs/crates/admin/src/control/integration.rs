//! Stored request evidence. Request locks serialize approval, attachment and delivery.
use super::*;
use crate::control::{canonical, store};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

const BOUND_STEP_UP: SessionKey<BoundStepUp> = SessionKey::new("admin_control_request_step_up");
const GRANT: SessionKey<String> = SessionKey::new(oteryn_identity::mfa::SESSION_GRANT_KEY);

#[derive(Serialize, Deserialize)]
struct BoundStepUp {
    request_id: Uuid,
    step: StepUp,
    grant: String,
}

/// Only a stored request identifier is accepted. All policy inputs come from storage.
#[derive(Debug)]
pub struct Submission {
    pub request_id: Uuid,
    pub preview: Option<Uuid>,
}
pub trait ControlEvidence: Send + Sync {
    fn control_evidence(
        &self,
        session: &BrowserSession,
        submission: &Submission,
    ) -> impl Future<Output = Result<Evidence, Refusal>> + Send;
}

impl<A: Authority, T: TotpVerifier> Controls<A, T> {
    /// A proof is scoped before TOTP verification. It cannot authorize another request.
    pub async fn step_up_for_request(
        &self,
        session: &BrowserSession,
        request_id: Uuid,
        code: &str,
    ) -> Result<StepUp, Refusal> {
        self.enabled()?;
        if request_id.get_version_num() != 7 {
            return Err(Refusal::InvalidInput);
        }
        let account = self.authority.staff(session).await?;
        let grant = session.get(GRANT).await?.ok_or(Refusal::StepUpRequired)?;
        if !self.totp.grant_valid(account, &grant).await? {
            return Err(Refusal::StepUpRequired);
        }
        self.step_up(session, code).await?;
        if !self.totp.grant_valid(account, &grant).await? {
            return Err(Refusal::StepUpRequired);
        }
        let step = session.get(STEP_UP).await?.ok_or(Refusal::StepUpRequired)?;
        session
            .set(
                BOUND_STEP_UP,
                &BoundStepUp {
                    request_id,
                    step,
                    grant,
                },
            )
            .await?;
        Ok(step)
    }

    async fn bound_step(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session: &BrowserSession,
        request: Uuid,
        actor: Uuid,
    ) -> Result<StepUp, Refusal> {
        let proof = session
            .get(BOUND_STEP_UP)
            .await?
            .ok_or(Refusal::StepUpRequired)?;
        if proof.request_id != request
            || proof.step.account != actor
            || session.account_id().await != Some(actor)
            || !recent(proof.step.step_up_at, self.now().await?, 300)
            || session.get(GRANT).await?.as_ref() != Some(&proof.grant)
            || !self
                .totp
                .grant_valid_in_transaction(tx, actor, &proof.grant)
                .await?
        {
            return Err(Refusal::StepUpRequired);
        }
        Ok(proof.step)
    }

    /// Called by verified result ingestion. Receipt material is loaded from the
    /// successful stored dry-run result, never accepted from a browser form.
    pub async fn record_request_preview(
        &self,
        session: &BrowserSession,
        request_id: Uuid,
    ) -> Result<Uuid, Refusal> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let request = store::Store::lock_control_request(&mut tx, request_id, true).await?;
        let (identity, op) = self.authorized(session, &request.operation).await?;
        if request.actor != identity.admin_id || !request.dry_run || op.risk != RiskClass::R4 {
            return Err(Refusal::Denied);
        }
        let reason = reason(&request)?;
        let step = self
            .bound_step(&mut tx, session, request_id, identity.admin_id)
            .await?;
        let intent = intent(&request)?;
        evaluate(
            &op,
            &identity,
            Some(step),
            reason.clone(),
            &intent.digest,
            None,
            None,
            true,
            self.now().await?,
        )?;
        let payloads: Vec<Value> = sqlx::query_scalar(
            "SELECT payload FROM admin_control_results WHERE request_id=$1 AND status='succeeded'",
        )
        .bind(request_id)
        .fetch_all(&mut *tx)
        .await?;
        let [payload] = payloads.as_slice() else {
            return Err(Refusal::DryRunRequired);
        };
        let preview = payload
            .get("dry_run_preview")
            .filter(|v| !v.is_null() && v.to_string().len() <= 65536)
            .ok_or(Refusal::DryRunRequired)?;
        let preview_digest =
            hex_digest(&canonical::encode_wire(preview).map_err(|_| Refusal::DryRunRequired)?);
        sqlx::query("INSERT INTO admin_control_previews(id,requester,operation,permission,intent_digest,preview_digest,preview,reason) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(request_id).bind(identity.admin_id).bind(&op.name).bind(&op.permission).bind(intent.digest).bind(&preview_digest).bind(preview).bind(json!(reason)).execute(&mut *tx).await?;
        append(
            &mut tx,
            &request,
            identity.admin_id,
            "admin.control.preview.stored",
            json!({"preview_digest":preview_digest}),
        )
        .await?;
        tx.commit().await?;
        Ok(request_id)
    }

    pub async fn approve_request(
        &self,
        session: &BrowserSession,
        request_id: Uuid,
    ) -> Result<Uuid, Refusal> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let request = store::Store::lock_control_request(&mut tx, request_id, true).await?;
        let p = self.preview(request_id).await?;
        let (identity, op) = self.authorized(session, &request.operation).await?;
        if identity.admin_id == request.actor {
            return Err(Refusal::SelfApproval);
        }
        if p.id != request_id
            || p.requester != request.actor
            || p.operation != request.operation
            || p.permission != op.permission
            || op.risk != RiskClass::R4
            || !request.dry_run
        {
            return Err(Refusal::Denied);
        }
        let step = self
            .bound_step(&mut tx, session, request_id, identity.admin_id)
            .await?;
        let now = self.now().await?;
        store::Store::lock_control_request(&mut tx, request_id, true).await?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO admin_control_approvals(id,preview_id,admin_id,step_up_at,approved_at) VALUES($1,$2,$3,$4,$5)")
            .bind(id).bind(request_id).bind(identity.admin_id).bind(step.step_up_at).bind(now).execute(&mut *tx).await?;
        append(&mut tx, &request, identity.admin_id, "admin.control.approved", json!({"approval_id":id,"step_up_at":step.step_up_at,"preview_digest":p.preview_digest})).await?;
        tx.commit().await?;
        Ok(id)
    }
}

impl<A: Authority, T: TotpVerifier> ControlEvidence for Controls<A, T> {
    async fn control_evidence(
        &self,
        session: &BrowserSession,
        submission: &Submission,
    ) -> Result<Evidence, Refusal> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let request =
            store::Store::lock_control_request(&mut tx, submission.request_id, false).await?;
        let (identity, op) = self.authorized(session, &request.operation).await?;
        if request.actor != identity.admin_id
            || request.risk_class.as_str() != format!("{:?}", op.risk)
        {
            return Err(Refusal::Denied);
        }
        let step = if op.risk >= RiskClass::R3 {
            Some(
                self.bound_step(&mut tx, session, request.request_id, identity.admin_id)
                    .await?,
            )
        } else {
            None
        };
        if step
            .is_some_and(|proof| !recent(proof.step_up_at, request.issued_at.unix_timestamp(), 300))
        {
            return Err(Refusal::StepUpRequired);
        }
        let intent = intent(&request)?;
        let preview_digest = request.params.get("preview_digest").and_then(Value::as_str);
        let reason = reason(&request)?;
        let approval = if op.risk == RiskClass::R4 && !request.dry_run {
            let preview_id = submission.preview.ok_or(Refusal::DryRunRequired)?;
            // Lock the referenced dry run too: expiry and approval snapshots are
            // checked within the same transaction as the final evidence append.
            let dry = store::Store::lock_control_request(&mut tx, preview_id, true).await?;
            let p = self.preview(preview_id).await?;
            if !dry.dry_run
                || dry.actor != identity.admin_id
                || p.requester != identity.admin_id
                || p.operation != op.name
                || p.permission != op.permission
                || Some(&p.reason) != reason.as_ref()
            {
                return Err(Refusal::Denied);
            }
            let r = sqlx::query("SELECT id,admin_id,approved_at,step_up_at FROM admin_control_approvals WHERE preview_id=$1")
                .bind(preview_id).fetch_optional(&mut *tx).await?.ok_or(Refusal::ApprovalRequired)?;
            let a = ApprovalState {
                approval_id: r.try_get("id")?,
                admin_id: r.try_get("admin_id")?,
                approved_at: r.try_get("approved_at")?,
                step_up_at: r.try_get("step_up_at")?,
                requester: p.requester,
                preview_digest: p.preview_digest,
                intent_digest: sqlx::query_scalar(
                    "SELECT intent_digest FROM admin_control_previews WHERE id=$1",
                )
                .bind(preview_id)
                .fetch_one(&mut *tx)
                .await?,
            };
            self.authority.approver(a.admin_id, &op).await?;
            let normalized: Vec<store::Approval> = sqlx::query_as("SELECT approval_id,approver,approved_at,preview_digest,decision FROM admin_approvals WHERE request_id=$1")
                .bind(request.request_id).fetch_all(&mut *tx).await?;
            if serde_json::to_value(&normalized).map_err(|_| Refusal::Unavailable)?
                != request.approvals
            {
                return Err(Refusal::Denied);
            }
            if normalized.len() != 1
                || normalized[0].approval_id != a.approval_id
                || normalized[0].approver != a.admin_id
                || normalized[0].approved_at.unix_timestamp() != a.approved_at
                || normalized[0].decision != store::Decision::Approved
                || normalized[0].preview_digest != unhex(&a.preview_digest)?
            {
                return Err(Refusal::Denied);
            }
            Some(a)
        } else {
            if request.approvals != json!([]) {
                return Err(Refusal::Denied);
            }
            None
        };
        let evidence = evaluate(
            &op,
            &identity,
            step,
            reason,
            &intent.digest,
            preview_digest,
            approval.as_ref(),
            request.dry_run,
            self.now().await?,
        )?;
        // Signed bytes remain immutable. Refuse any mismatch with freshly verified
        // evidence rather than changing an envelope after it was signed.
        validate_wire(&request, &evidence)?;
        store::Store::lock_control_request(&mut tx, request.request_id, false).await?;
        store::Store::attach_control_evidence(&mut tx, &request, &evidence).await?;
        tx.commit().await?;
        Ok(evidence)
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn validate_wire(request: &store::Request, evidence: &Evidence) -> Result<(), Refusal> {
    let wire: Value =
        serde_json::from_slice(&request.canonical_envelope).map_err(|_| Refusal::Denied)?;
    for (field, at) in [
        ("issued_at", request.issued_at),
        ("expires_at", request.expires_at),
    ] {
        let text = wire
            .get(field)
            .and_then(Value::as_str)
            .ok_or(Refusal::Denied)?;
        let parsed =
            time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
                .map_err(|_| Refusal::Denied)?;
        if parsed.unix_timestamp_nanos() / 1000 != at.unix_timestamp_nanos() / 1000 {
            return Err(Refusal::Denied);
        }
    }
    let expected = json!({
        "contract":"oteryn.admin-control.v1", "request_id":request.request_id,
        "admin_operation_id":request.admin_operation_id,"operation":request.operation,
        "operation_version":request.operation_version,"kind":request.kind,"target":request.target,
        "params":request.params,"dry_run":request.dry_run,"risk_class":request.risk_class,
        "actor":evidence.actor,"approvals":evidence.approvals,"reason":evidence.reason,
        "issued_at":wire["issued_at"],"expires_at":wire["expires_at"],
        "nonce":URL_SAFE_NO_PAD.encode(&request.nonce),"signer_key_id":request.signer_key_id,
    });
    if canonical::encode_wire(&expected).map_err(|_| Refusal::Denied)? != request.canonical_envelope
        || Sha256::digest(canonical::encode_wire(&request.params).map_err(|_| Refusal::Denied)?)
            .to_vec()
            != request.params_digest
    {
        return Err(Refusal::Denied);
    }
    Ok(())
}

fn reason(request: &store::Request) -> Result<Option<Reason>, Refusal> {
    if request.reason == json!({}) {
        return Ok(None);
    }
    serde_json::from_value(request.reason.clone())
        .map(Some)
        .map_err(|_| Refusal::InvalidInput)
}
fn intent(request: &store::Request) -> Result<Intent, Refusal> {
    let mut params = request.params.clone();
    params
        .as_object_mut()
        .ok_or(Refusal::InvalidInput)?
        .remove("preview_digest");
    // The dry run and execution have distinct transport/idempotency ids. Their
    // immutable operation/version, target and parameters must agree exactly.
    let bytes = canonical::encode_wire(&json!({"operation":request.operation,"operation_version":request.operation_version,"kind":request.kind,"target":request.target,"params":params})).map_err(|_| Refusal::InvalidInput)?;
    Ok(Intent {
        operation: request.operation.clone(),
        digest: Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    })
}
fn unhex(value: &str) -> Result<Vec<u8>, Refusal> {
    if !digest(value) {
        return Err(Refusal::InvalidInput);
    }
    (0..32)
        .map(|i| {
            u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| Refusal::InvalidInput)
        })
        .collect()
}
async fn append(
    tx: &mut Transaction<'_, Postgres>,
    request: &store::Request,
    actor: Uuid,
    action: &str,
    after: Value,
) -> Result<(), Refusal> {
    audit::append(
        tx,
        audit::Entry {
            context: audit::AuditContext {
                actor: Some(actor),
                request_id: request.request_id,
            },
            action,
            target: &request.request_id.to_string(),
            before: Value::Null,
            after,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "integration_tests.rs"]
pub(super) mod tests;
