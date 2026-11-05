use super::super::tests::{Auth, Database, Verifier, reason, session};
use super::*;
use time::{Duration, OffsetDateTime};
type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(in crate::control::controls) fn input(id: Uuid, actor: Uuid, dry: bool) -> store::NewRequest {
    let now = OffsetDateTime::now_utc();
    store::NewRequest {
        request_id: id,
        admin_operation_id: Uuid::now_v7(),
        operation: if dry {
            "world.retire"
        } else {
            "world.configure"
        }
        .into(),
        operation_version: 1,
        kind: store::Kind::Command,
        target: store::Target {
            world_id: Some("test".into()),
            node_id: None,
            character_id: None,
            account_ref: None,
        },
        params: json!({}),
        params_digest: Sha256::digest(b"{}").to_vec(),
        dry_run: dry,
        risk_class: if dry {
            store::RiskClass::R4
        } else {
            store::RiskClass::R3
        },
        actor,
        reason: json!(reason()),
        approvals: vec![],
        issued_at: now,
        expires_at: now + Duration::minutes(4),
        nonce: vec![2; 16],
        signer_key_id: "test".into(),
        canonical_envelope: b"fixture".to_vec(),
        signature: vec![3; 64],
    }
}
pub(in crate::control::controls) async fn bound(
    s: &BrowserSession,
    id: Uuid,
    age: i64,
) -> TestResult {
    let actor = s.account_id().await.ok_or("account")?;
    s.set(
        BOUND_STEP_UP,
        &BoundStepUp {
            request_id: id,
            step: StepUp {
                account: actor,
                step_up_at: OffsetDateTime::now_utc().unix_timestamp() - age,
            },
            grant: s.get(GRANT).await?.ok_or("grant")?,
        },
    )
    .await?;
    Ok(())
}
async fn enqueue(
    db: &Database,
    s: &BrowserSession,
    id: Uuid,
) -> Result<store::Request, Box<dyn std::error::Error>> {
    let service = Controls::new(db.pool.clone(), Auth, Verifier, true);
    let mut request = input(id, s.account_id().await.ok_or("account")?, false);
    let (identity, op) = service.authorized(s, &request.operation).await?;
    let step = s.get(BOUND_STEP_UP).await?.ok_or("step")?.step;
    let evidence = evaluate(
        &op,
        &identity,
        Some(step),
        Some(reason()),
        &"a".repeat(64),
        None,
        None,
        false,
        service.now().await?,
    )?;
    request.canonical_envelope = wire(&request, &evidence)?;
    Ok(
        store::Store::new(db.pool.clone(), store::Config { enabled: true })
            .enqueue(&request)
            .await?,
    )
}
#[tokio::test]
async fn stored_evidence_accepts_bound_proof_refuses_replay_foreign_stale_and_audit_failure()
-> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (s, _) = session(&db.pool, 0).await?;
    let service = Controls::new(db.pool.clone(), Auth, Verifier, true);
    let id = Uuid::now_v7();
    bound(&s, id, 0).await?;
    enqueue(&db, &s, id).await?;
    let store = store::Store::new(db.pool.clone(), store::Config { enabled: true });
    store
        .register_agent(&store::AgentHello {
            agent_id: "claim-test".into(),
            build: "test".into(),
            catalog_digest: vec![1; 32],
            advertised_operations: vec![store::AdvertisedOperation {
                operation: "world.configure".into(),
                operation_version: 1,
                kind: store::Kind::Command,
            }],
            served_worlds: vec!["test".into()],
            served_nodes: vec![],
            owner_epoch: 1,
        })
        .await?;
    assert!(store.claim_pending("claim-test", 1, 1).await?.is_empty());
    let submission = Submission {
        request_id: id,
        preview: None,
    };
    assert!(
        service
            .control_evidence(&s, &submission)
            .await?
            .actor
            .step_up_at
            .is_some()
    );
    assert_eq!(
        service.control_evidence(&s, &submission).await.err(),
        Some(Refusal::Denied)
    );
    let delivered = store.claim_pending("claim-test", 1, 1).await?;
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].request_id, id);
    let other_id = Uuid::now_v7();
    enqueue(&db, &s, other_id).await?;
    assert_eq!(
        service
            .control_evidence(
                &s,
                &Submission {
                    request_id: other_id,
                    preview: None
                }
            )
            .await
            .err(),
        Some(Refusal::StepUpRequired)
    );
    let (foreign, _) = session(&db.pool, 0).await?;
    assert_eq!(
        service.control_evidence(&foreign, &submission).await.err(),
        Some(Refusal::Denied)
    );
    let mut original = s.get(BOUND_STEP_UP).await?.ok_or("proof")?;
    original.request_id = other_id;
    bound(&s, other_id, 300).await?;
    assert_eq!(
        service
            .control_evidence(
                &s,
                &Submission {
                    request_id: other_id,
                    preview: None
                }
            )
            .await
            .err(),
        Some(Refusal::StepUpRequired)
    );
    s.set(BOUND_STEP_UP, &original).await?;
    // Fail the final audit append after ADM-P2C's structured record. Both roll back.
    sqlx::query("CREATE FUNCTION refuse_evidence() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='admin.control.evidence.attached' THEN RAISE EXCEPTION 'test audit failure'; END IF; RETURN NEW; END; $$").execute(&db.pool).await?;
    sqlx::query("CREATE TRIGGER refuse_evidence BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_evidence()").execute(&db.pool).await?;
    assert_eq!(
        service
            .control_evidence(
                &s,
                &Submission {
                    request_id: other_id,
                    preview: None
                }
            )
            .await
            .err(),
        Some(Refusal::Unavailable)
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE request_id=$1 AND admin_operation_id IS NOT NULL",
    )
    .bind(other_id)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 0);
    db.close().await
}

#[tokio::test]
async fn stored_approval_refuses_self_expired_and_rolls_back_with_audit() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (s, _) = session(&db.pool, 0).await?;
    let (other, _) = session(&db.pool, 0).await?;
    let service = Controls::new(db.pool.clone(), Auth, Verifier, true);
    let id = Uuid::now_v7();
    let request = input(id, s.account_id().await.ok_or("account")?, true);
    let stored = store::Store::new(db.pool.clone(), store::Config { enabled: true })
        .enqueue(&request)
        .await?;
    // The fixture represents a previously verified successful dry-run result.
    sqlx::query("UPDATE admin_control_requests SET status='succeeded' WHERE request_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO admin_control_agents(agent_id,build,catalog_digest,advertised_operations,served_worlds,served_nodes,owner_epoch) VALUES('test','test',$1,'[]',ARRAY['test'],'{}',1)").bind(vec![1u8;32]).execute(&db.pool).await?;
    sqlx::query("INSERT INTO admin_control_results(request_id,agent_id,status,payload) VALUES($1,'test','succeeded',$2)").bind(id).bind(json!({"dry_run_preview":{"effect":"archive"}})).execute(&db.pool).await?;
    bound(&s, id, 0).await?;
    service.record_request_preview(&s, id).await?;
    assert_eq!(
        service.approve_request(&s, id).await.err(),
        Some(Refusal::SelfApproval)
    );
    bound(&other, id, 0).await?;
    sqlx::query("CREATE FUNCTION refuse_approval() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='admin.control.approved' THEN RAISE EXCEPTION 'test audit failure'; END IF; RETURN NEW; END; $$").execute(&db.pool).await?;
    sqlx::query("CREATE TRIGGER refuse_approval BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_approval()").execute(&db.pool).await?;
    assert_eq!(
        service.approve_request(&other, id).await.err(),
        Some(Refusal::Unavailable)
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM admin_control_approvals WHERE preview_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    sqlx::query("DROP TRIGGER refuse_approval ON audit_log")
        .execute(&db.pool)
        .await?;
    let approval = service.approve_request(&other, id).await?;
    let at: i64 = sqlx::query_scalar("SELECT approved_at FROM admin_control_approvals WHERE id=$1")
        .bind(approval)
        .fetch_one(&db.pool)
        .await?;
    assert!(at > 0);
    let target: String = sqlx::query_scalar(
        "SELECT target FROM audit_log WHERE action='admin.control.approved' AND request_id=$1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(target, id.to_string());
    assert_eq!(intent(&stored)?.operation, "world.retire");
    let final_id = Uuid::now_v7();
    bound(&s, final_id, 0).await?;
    let mut final_request = input(final_id, s.account_id().await.ok_or("account")?, true);
    final_request.dry_run = false;
    final_request.issued_at = OffsetDateTime::now_utc();
    let preview_digest = hex_digest(&canonical::encode_wire(&json!({"effect":"archive"}))?);
    final_request.params = json!({"preview_digest":preview_digest});
    final_request.params_digest =
        Sha256::digest(canonical::encode_wire(&final_request.params)?).to_vec();
    final_request.approvals = vec![store::Approval {
        approval_id: approval,
        approver: other.account_id().await.ok_or("approver")?,
        approved_at: OffsetDateTime::from_unix_timestamp(at)?,
        preview_digest: unhex(&preview_digest)?,
        decision: store::Decision::Approved,
    }];
    let (identity, operation) = service.authorized(&s, "world.retire").await?;
    let step = s.get(BOUND_STEP_UP).await?.ok_or("step")?.step;
    let approval_state = ApprovalState {
        approval_id: approval,
        admin_id: other.account_id().await.ok_or("approver")?,
        approved_at: at,
        step_up_at: at,
        requester: stored.actor,
        preview_digest: preview_digest.clone(),
        intent_digest: intent(&stored)?.digest,
    };
    let expected = evaluate(
        &operation,
        &identity,
        Some(step),
        Some(reason()),
        &intent(&stored)?.digest,
        Some(&preview_digest),
        Some(&approval_state),
        false,
        service.now().await?,
    )?;
    final_request.canonical_envelope = wire(&final_request, &expected)?;
    store::Store::new(db.pool.clone(), store::Config { enabled: true })
        .enqueue(&final_request)
        .await?;
    let final_evidence = service
        .control_evidence(
            &s,
            &Submission {
                request_id: final_id,
                preview: Some(id),
            },
        )
        .await?;
    assert_eq!(final_evidence.approvals[0].approval_id, approval);
    let attached: Uuid =
        sqlx::query_scalar("SELECT request_id FROM admin_approvals WHERE approval_id=$1")
            .bind(approval)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(attached, final_id);

    // New expired request has no prior approval: expiry alone must deny.
    let expired = Uuid::now_v7();
    let mut request = input(expired, s.account_id().await.ok_or("account")?, true);
    request.issued_at = OffsetDateTime::now_utc() - Duration::minutes(5);
    request.expires_at = OffsetDateTime::now_utc() - Duration::seconds(1);
    store::Store::new(db.pool.clone(), store::Config { enabled: true })
        .enqueue(&request)
        .await?;
    sqlx::query("UPDATE admin_control_requests SET status='succeeded' WHERE request_id=$1")
        .bind(expired)
        .execute(&db.pool)
        .await?;
    bound(&other, expired, 0).await?;
    assert_eq!(
        service.approve_request(&other, expired).await.err(),
        Some(Refusal::Denied)
    );
    db.close().await
}

#[tokio::test]
async fn real_identity_provider_accepts_fresh_request_step_up_and_denies_revoked_grant()
-> TestResult {
    use oteryn_identity::{login::Credentials, mfa::Mfa, password};
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (s, _) = session(&db.pool, 0).await?;
    let actor = s.account_id().await.ok_or("account")?;
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'real-staff@example.test',$2,'active','en',now())").bind(actor).bind(password::hash("test password")?).execute(&db.pool).await?;
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let enrollment = mfa.enroll(actor, "test password").await?.ok_or("enroll")?;
    let secret = totp_rs::Secret::Encoded(enrollment.secret).to_bytes()?;
    let totp = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, secret)?;
    let now = u64::try_from(OffsetDateTime::now_utc().unix_timestamp())?;
    let codes = mfa
        .confirm(actor, "test password", &totp.generate(now))
        .await?
        .ok_or("confirm")?;
    let challenge = mfa
        .begin_login(&Credentials {
            email: "real-staff@example.test".into(),
            password: "test password".into(),
        })
        .await?
        .ok_or("challenge")?;
    let auth = mfa
        .complete(&challenge.token, &codes[0], false)
        .await?
        .ok_or("login")?;
    let (s, _) = super::super::tests::session_for_account(&db.pool, 0, actor).await?;
    s.set(GRANT, &auth.grant).await?;
    let service = Controls::new(db.pool.clone(), Auth, mfa, true);
    let id = Uuid::now_v7();
    assert_eq!(
        service.step_up_for_request(&s, id, &codes[1]).await.err(),
        Some(Refusal::InvalidInput)
    );
    let fresh = totp.generate(now + 30);
    service.step_up_for_request(&s, id, &fresh).await?;
    assert_eq!(
        service
            .step_up_for_request(&s, Uuid::now_v7(), &fresh)
            .await
            .err(),
        Some(Refusal::Denied)
    );
    enqueue(&db, &s, id).await?;
    let submission = Submission {
        request_id: id,
        preview: None,
    };
    assert!(
        service
            .control_evidence(&s, &submission)
            .await?
            .actor
            .step_up_at
            .is_some()
    );
    assert_eq!(
        service.control_evidence(&s, &submission).await.err(),
        Some(Refusal::Denied)
    );
    sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1")
        .bind(actor)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        service
            .bound_step(&mut db.pool.begin().await?, &s, id, actor)
            .await
            .err(),
        Some(Refusal::StepUpRequired)
    );
    db.close().await
}

pub(in crate::control::controls) fn wire(
    request: &store::NewRequest,
    evidence: &Evidence,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(canonical::encode_wire(&json!({
        "contract":"oteryn.admin-control.v1","request_id":request.request_id,"admin_operation_id":request.admin_operation_id,
        "operation":request.operation,"operation_version":request.operation_version,"kind":request.kind,"target":request.target,
        "params":request.params,"dry_run":request.dry_run,"risk_class":request.risk_class,
        "actor":evidence.actor,"approvals":evidence.approvals,"reason":evidence.reason,
        "issued_at":request.issued_at.format(&time::format_description::well_known::Rfc3339)?,
        "expires_at":request.expires_at.format(&time::format_description::well_known::Rfc3339)?,
        "nonce":URL_SAFE_NO_PAD.encode(&request.nonce),"signer_key_id":request.signer_key_id,
    }))?)
}
