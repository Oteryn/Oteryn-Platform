#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use std::collections::BTreeSet;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("skipping admin-control PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("adm_p1a_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |connection, _| {
                let query = search_path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(query))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self { pool, root, schema })
    }
    fn store(&self) -> Store {
        Store::new(self.pool.clone(), Config { enabled: true })
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
}
fn hello(agent: &str, epoch: i64) -> AgentHello {
    AgentHello {
        agent_id: agent.into(),
        build: "test-build".into(),
        catalog_digest: vec![1; 32],
        advertised_operations: vec![AdvertisedOperation {
            operation: "world.save_now".into(),
            operation_version: 1,
            kind: Kind::Command,
        }],
        served_worlds: vec!["world-a".into()],
        served_nodes: vec!["node-a".into()],
        owner_epoch: epoch,
    }
}
fn request() -> NewRequest {
    let now = OffsetDateTime::now_utc();
    NewRequest {
        request_id: Uuid::now_v7(),
        admin_operation_id: Uuid::now_v7(),
        operation: "world.save_now".into(),
        operation_version: 1,
        kind: Kind::Command,
        target: Target {
            world_id: Some("world-a".into()),
            node_id: None,
            character_id: None,
            account_ref: None,
        },
        params: json!({}),
        params_digest: vec![2; 32],
        dry_run: false,
        risk_class: RiskClass::R2,
        actor: Uuid::now_v7(),
        reason: json!({"code":"TEST","text":"Test save"}),
        approvals: vec![],
        issued_at: now - Duration::seconds(1),
        expires_at: now + Duration::minutes(4),
        nonce: vec![3; 16],
        signer_key_id: "test-key".into(),
        canonical_envelope: b"signed test bytes".to_vec(),
        signature: vec![4; 64],
    }
}
fn result(id: Uuid, status: Status) -> NewResult {
    NewResult {
        request_id: id,
        agent_id: "agent-a".into(),
        status,
        game_ledger_id: Some("ledger-1".into()),
        error_code: None,
        payload: json!({"status":status.as_str()}),
    }
}
async fn delivered(store: &Store) -> NewRequest {
    let input = request();
    store.enqueue(&input).await.unwrap();
    let batch = store.claim_pending("agent-a", 1, 32).await.unwrap();
    assert!(batch.iter().any(|r| r.request_id == input.request_id));
    input
}
async fn arrange(store: &Store, status: Status) -> NewRequest {
    if status == Status::Queued {
        let input = request();
        store.enqueue(&input).await.unwrap();
        return input;
    }
    let input = delivered(store).await;
    if status != Status::Delivered {
        if status == Status::InProgress {
            store
                .record_result(&result(input.request_id, Status::Accepted), 1)
                .await
                .unwrap();
        }
        store
            .record_result(&result(input.request_id, status), 1)
            .await
            .unwrap();
    }
    input
}
async fn count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM admin_control_results WHERE request_id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn disabled_and_bad_input_fail_without_a_database() {
    assert!(!Config::default().enabled);
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let disabled = Store::new(pool.clone(), Config::default());
    assert!(matches!(
        disabled.enqueue(&request()).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled.claim_pending("agent-a", 1, 32).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(disabled.expire_due().await, Err(Error::Disabled)));
    assert!(matches!(disabled.agents().await, Err(Error::Disabled)));
    let enabled = Store::new(pool, Config { enabled: true });
    for limit in [0, 33, u32::MAX] {
        assert!(matches!(
            enabled.claim_pending("agent-a", 1, limit).await,
            Err(Error::InvalidInput)
        ));
    }
    let mut ambiguous = hello("agent-a", 1);
    let mut duplicate = ambiguous.advertised_operations[0].clone();
    duplicate.kind = Kind::Query;
    ambiguous.advertised_operations.push(duplicate);
    assert!(matches!(
        enabled.register_agent(&ambiguous).await,
        Err(Error::InvalidInput)
    ));
    let mut bad_result = result(Uuid::now_v7(), Status::Failed);
    bad_result.error_code = Some("unrecognised".into());
    assert!(matches!(
        enabled.record_result(&bad_result, 1).await,
        Err(Error::InvalidInput)
    ));
    let mut bad = request();
    bad.signature.clear();
    assert!(matches!(
        enabled.enqueue(&bad).await,
        Err(Error::InvalidInput)
    ));
    bad = request();
    bad.kind = Kind::Query;
    assert!(matches!(
        enabled.enqueue(&bad).await,
        Err(Error::InvalidInput)
    ));
    bad = request();
    bad.target.world_id = None;
    assert!(matches!(
        enabled.enqueue(&bad).await,
        Err(Error::InvalidInput)
    ));
    bad = request();
    bad.params = json!([]);
    assert!(matches!(
        enabled.enqueue(&bad).await,
        Err(Error::InvalidInput)
    ));
    bad = request();
    bad.expires_at = bad.issued_at;
    assert!(matches!(
        enabled.enqueue(&bad).await,
        Err(Error::InvalidInput)
    ));
    assert!(serde_json::from_value::<Status>(json!("unknown")).is_err());
    assert!(serde_json::from_value::<Target>(json!({"world_id":"world-a","extra":true})).is_err());
    assert!(matches!(
        enabled
            .record_result(&result(Uuid::now_v7(), Status::Queued), 1)
            .await,
        Err(Error::InvalidInput)
    ));
}

#[tokio::test]
async fn pg_enqueue_idempotency_and_console_queries() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    let input = request();
    let (a, b) = tokio::join!(store.enqueue(&input), store.enqueue(&input));
    assert_eq!(a.unwrap().request_id, b.unwrap().request_id);
    let mut retry = input.clone();
    retry.request_id = Uuid::now_v7();
    retry.nonce = vec![9; 16];
    assert_eq!(
        store.enqueue(&retry).await.unwrap().request_id,
        input.request_id
    );
    retry.params_digest = vec![9; 32];
    assert!(matches!(store.enqueue(&retry).await, Err(Error::Conflict)));
    retry = input.clone();
    retry.target.world_id = Some("other-world".into());
    assert!(matches!(store.enqueue(&retry).await, Err(Error::Conflict)));
    retry = input.clone();
    retry.admin_operation_id = Uuid::now_v7();
    assert!(matches!(store.enqueue(&retry).await, Err(Error::Conflict)));
    let saved = store.get(input.request_id).await.unwrap().unwrap();
    assert_eq!(saved.canonical_envelope, input.canonical_envelope);
    assert_eq!(saved.signature, input.signature);
    let mut query = request();
    query.kind = Kind::Query;
    query.admin_operation_id = query.request_id;
    query.operation = "world.get".into();
    store.enqueue(&query).await.unwrap();
    let page = store.list(&Filter::default(), 1).await.unwrap();
    assert_eq!(page.len(), 1);
    let second = store
        .list(
            &Filter {
                before: Some((page[0].created_at, page[0].request_id)),
                ..Filter::default()
            },
            1,
        )
        .await
        .unwrap();
    assert_eq!(second.len(), 1);
    assert_ne!(page[0].request_id, second[0].request_id);
    let filtered = store
        .list(
            &Filter {
                actor: Some(input.actor),
                operation: Some(input.operation.clone()),
                status: Some(Status::Queued),
                before: None,
            },
            100,
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].request_id, input.request_id);
    assert!(store.get(Uuid::now_v7()).await.unwrap().is_none());
    assert!(matches!(
        store.list(&Filter::default(), 101).await,
        Err(Error::InvalidInput)
    ));
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE request_id=$1")
        .bind(input.request_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(audit_count, 1);
    db.close().await;
}

#[tokio::test]
async fn pg_every_result_transition_and_terminal_refusals() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    store.register_agent(&hello("agent-a", 1)).await.unwrap();
    use Status::*;
    let statuses = [
        Queued, Delivered, Accepted, InProgress, Succeeded, Failed, Rejected, Expired, Cancelled,
    ];
    let allowed = [
        (Delivered, Accepted),
        (Delivered, Succeeded),
        (Delivered, Failed),
        (Delivered, Rejected),
        (Delivered, Expired),
        (Delivered, Cancelled),
        (Accepted, InProgress),
        (Accepted, Succeeded),
        (Accepted, Failed),
        (Accepted, Cancelled),
        (InProgress, InProgress),
        (InProgress, Succeeded),
        (InProgress, Failed),
        (InProgress, Cancelled),
    ];
    for from in statuses {
        for to in statuses {
            let input = arrange(&store, from).await;
            let before = count(&db.pool, input.request_id).await;
            let response = store.record_result(&result(input.request_id, to), 1).await;
            if allowed.contains(&(from, to)) {
                let row = response.unwrap();
                assert_eq!(row.status, to);
                assert_eq!(row.game_ledger_id.as_deref(), Some("ledger-1"));
                assert_eq!(count(&db.pool, input.request_id).await, before + 1);
                assert_eq!(
                    store.results(input.request_id, 100).await.unwrap()[0].status,
                    to
                );
                assert_eq!(
                    store.get(input.request_id).await.unwrap().unwrap().status,
                    to
                );
            } else {
                assert!(
                    response.is_err(),
                    "unexpected transition {from:?} -> {to:?}"
                );
                assert_eq!(count(&db.pool, input.request_id).await, before);
                assert_eq!(
                    store.get(input.request_id).await.unwrap().unwrap().status,
                    from
                );
            }
            // A queued fixture must not become a candidate for the next case.
            if from == Queued {
                sqlx::query("UPDATE admin_control_requests SET issued_at=now()-interval '2 minutes',expires_at=now()-interval '1 minute' WHERE request_id=$1")
                    .bind(input.request_id).execute(&db.pool).await.unwrap();
                store.expire_due().await.unwrap();
            }
        }
    }
    db.close().await;
}

#[tokio::test]
async fn pg_expiry_is_terminal_and_does_not_expire_accepted_work() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    store.register_agent(&hello("agent-a", 1)).await.unwrap();
    let delivery = arrange(&store, Status::Delivered).await;
    let accepted = arrange(&store, Status::Accepted).await;
    let mut queued = request();
    queued.issued_at -= Duration::minutes(6);
    queued.expires_at -= Duration::minutes(6);
    store.enqueue(&queued).await.unwrap();
    for id in [delivery.request_id, accepted.request_id] {
        sqlx::query("UPDATE admin_control_requests SET issued_at=now()-interval '2 minutes',expires_at=now()-interval '1 minute' WHERE request_id=$1")
            .bind(id).execute(&db.pool).await.unwrap();
    }
    assert!(matches!(
        store
            .record_result(&result(delivery.request_id, Status::Accepted), 1)
            .await,
        Err(Error::IllegalTransition)
    ));
    let mut lock = db.pool.begin().await.unwrap();
    sqlx::query("SELECT request_id FROM admin_control_requests WHERE request_id=$1 FOR UPDATE")
        .bind(queued.request_id)
        .execute(&mut *lock)
        .await
        .unwrap();
    assert_eq!(store.expire_due().await.unwrap(), 1);
    lock.rollback().await.unwrap();
    assert_eq!(store.expire_due().await.unwrap(), 1);
    assert_eq!(store.expire_due().await.unwrap(), 0);
    assert_eq!(
        store.enqueue(&queued).await.unwrap().status,
        Status::Expired
    );
    assert!(
        store
            .claim_pending("agent-a", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    for id in [queued.request_id, delivery.request_id] {
        assert_eq!(
            store.get(id).await.unwrap().unwrap().status,
            Status::Expired
        );
    }
    assert!(matches!(
        store
            .record_result(&result(delivery.request_id, Status::Succeeded), 1)
            .await,
        Err(Error::IllegalTransition)
    ));
    store
        .record_result(&result(accepted.request_id, Status::Succeeded), 1)
        .await
        .unwrap();
    db.close().await;
}

#[tokio::test]
async fn pg_claim_skips_locked_rows_and_parallel_batches_are_disjoint() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    store.register_agent(&hello("agent-a", 1)).await.unwrap();
    let first = request();
    store.enqueue(&first).await.unwrap();
    let second = request();
    store.enqueue(&second).await.unwrap();
    let mut lock = db.pool.begin().await.unwrap();
    sqlx::query("SELECT request_id FROM admin_control_requests WHERE request_id=$1 FOR UPDATE")
        .bind(first.request_id)
        .execute(&mut *lock)
        .await
        .unwrap();
    let batch = store.claim_pending("agent-a", 1, 32).await.unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].request_id, second.request_id);
    lock.rollback().await.unwrap();
    assert_eq!(
        store.claim_pending("agent-a", 1, 32).await.unwrap()[0].request_id,
        first.request_id
    );
    for _ in 0..65 {
        store.enqueue(&request()).await.unwrap();
    }
    let (left, right) = tokio::join!(
        store.claim_pending("agent-a", 1, 32),
        store.claim_pending("agent-a", 1, 32)
    );
    let left = left.unwrap();
    let right = right.unwrap();
    assert_eq!(left.len(), 32);
    assert_eq!(right.len(), 32);
    let ids: BTreeSet<_> = left.iter().chain(&right).map(|r| r.request_id).collect();
    assert_eq!(ids.len(), 64);
    assert!(
        left.iter()
            .chain(&right)
            .all(|r| r.status == Status::Delivered && r.delivered_at.is_some())
    );
    assert_eq!(
        store.claim_pending("agent-a", 1, 32).await.unwrap().len(),
        1
    );
    assert!(
        store
            .claim_pending("agent-a", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await;
}

#[tokio::test]
async fn pg_advertisements_ownership_epochs_and_connectivity_fail_closed() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    let owner = hello("agent-a", 1);
    store.register_agent(&owner).await.unwrap();
    let good = request();
    store.enqueue(&good).await.unwrap();
    for variant in 0..5 {
        let mut input = request();
        match variant {
            0 => input.operation = "unadvertised.operation".into(),
            1 => input.operation_version = 2,
            2 => {
                input.kind = Kind::Query;
                input.admin_operation_id = input.request_id;
            }
            3 => input.target.world_id = Some("world-b".into()),
            _ => input.target.node_id = Some("node-b".into()),
        }
        store.enqueue(&input).await.unwrap();
    }
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=now()-interval '30 seconds'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(!store.agents().await.unwrap()[0].connected);
    assert!(matches!(
        store.claim_pending("agent-a", 1, 32).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=now()+interval '1 minute'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(!store.agents().await.unwrap()[0].connected);
    assert!(matches!(
        store.claim_pending("agent-a", 1, 32).await,
        Err(Error::Unavailable)
    ));
    store.heartbeat("agent-a", 1).await.unwrap();
    assert!(store.agents().await.unwrap()[0].connected);
    let claimed = store.claim_pending("agent-a", 1, 32).await.unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].request_id, good.request_id);
    let mut wrong = result(good.request_id, Status::Accepted);
    wrong.agent_id = "other".into();
    assert!(matches!(
        store.record_result(&wrong, 1).await,
        Err(Error::Unavailable)
    ));
    store.register_agent(&hello("agent-b", 1)).await.unwrap();
    let queued = request();
    store.enqueue(&queued).await.unwrap();
    assert!(
        store
            .claim_pending("agent-a", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .claim_pending("agent-b", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    store.register_agent(&hello("agent-b", 2)).await.unwrap();
    assert!(
        store
            .claim_pending("agent-a", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        store
            .record_result(&result(good.request_id, Status::Accepted), 1)
            .await,
        Err(Error::Unavailable)
    ));
    // A disconnected higher epoch must not fall back to the old owner.
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=now()-interval '31 seconds' WHERE agent_id='agent-b'").execute(&db.pool).await.unwrap();
    assert!(
        store
            .claim_pending("agent-a", 1, 32)
            .await
            .unwrap()
            .is_empty()
    );
    store.heartbeat("agent-b", 2).await.unwrap();
    assert_eq!(
        store.claim_pending("agent-b", 2, 32).await.unwrap().len(),
        1
    );
    assert!(matches!(
        store.heartbeat("agent-b", 1).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        store.register_agent(&hello("agent-b", 1)).await,
        Err(Error::Unavailable)
    ));
    let mut changed = hello("agent-b", 2);
    changed.served_worlds = vec!["world-b".into()];
    assert!(matches!(
        store.register_agent(&changed).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        store
            .list(
                &Filter {
                    status: Some(Status::Queued),
                    ..Filter::default()
                },
                100
            )
            .await
            .unwrap()
            .len(),
        5
    );
    db.close().await;
}

#[tokio::test]
async fn pg_approvals_require_a_distinct_admin_and_are_transactional() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    let mut input = request();
    input.risk_class = RiskClass::R4;
    let approval = Approval {
        approval_id: Uuid::now_v7(),
        approver: input.actor,
        approved_at: input.issued_at - Duration::seconds(1),
        preview_digest: vec![8; 32],
        decision: Decision::Approved,
    };
    input.approvals.push(approval);
    assert!(matches!(
        store.enqueue(&input).await,
        Err(Error::SelfApproval)
    ));
    assert!(store.get(input.request_id).await.unwrap().is_none());
    input.approvals[0].approver = Uuid::now_v7();
    store.enqueue(&input).await.unwrap();
    let row = sqlx::query(
        "SELECT approver,decision,preview_digest FROM admin_approvals WHERE request_id=$1",
    )
    .bind(input.request_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    use sqlx::Row;
    assert_ne!(row.get::<Uuid, _>("approver"), input.actor);
    assert_eq!(row.get::<String, _>("decision"), "approved");
    assert_eq!(row.get::<Vec<u8>, _>("preview_digest"), vec![8; 32]);
    let mut duplicate = request();
    duplicate.approvals = input.approvals.clone();
    assert!(store.enqueue(&duplicate).await.is_err());
    assert!(store.get(duplicate.request_id).await.unwrap().is_none());
    db.close().await;
}

#[tokio::test]
async fn pg_schema_constraints_append_only_results_and_audit_rollback() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    store.register_agent(&hello("agent-a", 1)).await.unwrap();
    let input = delivered(&store).await;
    store
        .record_result(&result(input.request_id, Status::Accepted), 1)
        .await
        .unwrap();
    for query in [
        "UPDATE admin_control_results SET payload='{}'",
        "DELETE FROM admin_control_results",
        "TRUNCATE admin_control_results",
    ] {
        assert!(
            sqlx::query(AssertSqlSafe(query))
                .execute(&db.pool)
                .await
                .is_err()
        );
    }
    for query in [
        "UPDATE admin_control_requests SET status='unknown'",
        "UPDATE admin_control_requests SET kind='unknown'",
        "UPDATE admin_control_requests SET risk_class='R5'",
        "UPDATE admin_control_requests SET params_digest='x'::bytea",
    ] {
        assert!(
            sqlx::query(AssertSqlSafe(query))
                .execute(&db.pool)
                .await
                .is_err()
        );
    }
    let unclaimed = request();
    store.enqueue(&unclaimed).await.unwrap();
    let mut due = request();
    due.issued_at -= Duration::minutes(6);
    due.expires_at -= Duration::minutes(6);
    store.enqueue(&due).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION refuse_control_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit unavailable'; END; $$;
        CREATE TRIGGER refuse_control_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_control_audit();")
        .execute(&db.pool).await.unwrap();
    assert!(
        store
            .record_result(&result(input.request_id, Status::Succeeded), 1)
            .await
            .is_err()
    );
    assert_eq!(
        store.get(input.request_id).await.unwrap().unwrap().status,
        Status::Accepted
    );
    assert_eq!(count(&db.pool, input.request_id).await, 1);
    assert!(store.claim_pending("agent-a", 1, 32).await.is_err());
    assert_eq!(
        store
            .get(unclaimed.request_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        Status::Queued
    );
    assert!(store.expire_due().await.is_err());
    assert_eq!(
        store.get(due.request_id).await.unwrap().unwrap().status,
        Status::Queued
    );
    let queued = request();
    assert!(store.enqueue(&queued).await.is_err());
    assert!(store.get(queued.request_id).await.unwrap().is_none());
    db.close().await;
}

#[tokio::test]
async fn pg_claim_checks_live_expiry_after_waiting_for_an_agent_lock() {
    let Some(db) = Database::open().await else {
        return;
    };
    let store = db.store();
    store.register_agent(&hello("agent-a", 1)).await.unwrap();
    let input = request();
    store.enqueue(&input).await.unwrap();
    let mut lock = db.pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query("SELECT agent_id FROM admin_control_agents WHERE agent_id='agent-a' FOR UPDATE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let claimant = store.clone();
    let waiting = tokio::spawn(async move { claimant.claim_pending("agent-a", 1, 32).await });
    // Prove the claim transaction has started and is blocked before changing expiry.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity
                WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    sqlx::query(
        "UPDATE admin_control_requests SET expires_at=clock_timestamp() WHERE request_id=$1",
    )
    .bind(input.request_id)
    .execute(&db.pool)
    .await
    .unwrap();
    lock.rollback().await.unwrap();
    assert!(waiting.await.unwrap().unwrap().is_empty());
    assert_eq!(store.expire_due().await.unwrap(), 1);
    db.close().await;
}
