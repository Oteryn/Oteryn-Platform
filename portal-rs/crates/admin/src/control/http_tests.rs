#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use store::*;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
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
fn request() -> NewRequest {
    let now = OffsetDateTime::now_utc();
    let mut input = NewRequest {
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
        reason: json!({"code":"TEST","text":"Test save","ticket_ref":null}),
        approvals: vec![],
        issued_at: now - Duration::seconds(1),
        expires_at: now + Duration::minutes(4),
        nonce: vec![3; 16],
        signer_key_id: "platform-admin-signer-1".into(),
        canonical_envelope: b"{\"operation\":\"world.save_now\"}".to_vec(),
        signature: vec![4; 64],
    };
    let mut envelope: signer::RequestEnvelope = serde_json::from_value(json!({
        "contract":"oteryn.admin-control.v1", "request_id":input.request_id,
        "admin_operation_id":input.admin_operation_id,"operation":input.operation,
        "operation_version":1,"kind":"command","target":input.target,
        "params":{},"dry_run":false,"actor":{"admin_id":input.actor.to_string(),
        "roles":["game_operator"],"permission":"game.world.ops","mfa_at":"2026-11-05T18:00:00Z",
        "step_up_at":null,"ip_hash":"test","session_ref":"test"},"reason":input.reason,
        "approvals":[],"risk_class":"R2","issued_at":input.issued_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "expires_at":input.expires_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "nonce":URL_SAFE_NO_PAD.encode(&input.nonce),"signer_key_id":"platform-admin-signer-1","signature":""
    })).unwrap();
    let mut keys = signer::KeyRing::new(true);
    keys.add(
        signer::AdminKey::new(
            "platform-admin-signer-1".into(),
            ed25519_dalek::SigningKey::from_bytes(&[7; 32]),
            signer::Validity {
                not_before: 0,
                not_after: i64::MAX,
            },
        )
        .unwrap(),
    )
    .unwrap();
    keys.sign(
        &mut envelope,
        "platform-admin-signer-1",
        now.unix_timestamp(),
    )
    .unwrap();
    input.canonical_envelope = signer::adm_x1_signing_bytes(&envelope).unwrap();
    input.signature = URL_SAFE_NO_PAD.decode(&envelope.signature).unwrap();
    input.params_digest = signer::params_digest(&envelope.params).unwrap().to_vec();
    input
}

fn app(pool: PgPool, enabled: bool) -> Router {
    router(
        pool,
        enabled,
        vec![AgentBinding {
            agent_id: "agent-a".into(),
            identity: Identity::san("urn:test:agent-a").unwrap(),
            result_key: Some((
                ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key(),
                signer::Validity {
                    not_before: 0,
                    not_after: i64::MAX,
                },
            )),
        }],
    )
    .unwrap()
}
async fn post(app: Router, path: &str, value: Value, identity: bool) -> Response {
    let mut request = Request::post(format!("/internal/v1/admin-control/{path}"))
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap();
    request
        .extensions_mut()
        .insert(VerifiedAgent::from_verified_certificate_cn("oteryn-admin-agent/agent-a").unwrap());
    request
        .extensions_mut()
        .insert(guard::VerifiedClient::from_verified_tls13_http11(
            Identity::san(if identity {
                "urn:test:agent-a"
            } else {
                "urn:test:unlisted"
            })
            .unwrap(),
        ));
    app.oneshot(request).await.unwrap()
}
fn hello_body(epoch: i64) -> Value {
    json!({"agent_id":"agent-a","build":"test-build","catalog_digest":URL_SAFE_NO_PAD.encode([1;32]),"advertised_operations":[{"operation":"world.save_now","operation_version":1,"kind":"command"}],"served_worlds":["world-a"],"served_nodes":[],"owner_epoch":epoch,"clock":"2026-11-05T23:00:00Z"})
}
#[tokio::test]
async fn transport_refusals_and_closed_input() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    assert_eq!(
        post(app(pool.clone(), false), "hello", hello_body(1), true)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        post(app(pool.clone(), true), "hello", hello_body(1), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut unknown = hello_body(1);
    unknown["unexpected"] = json!(true);
    assert_eq!(
        post(app(pool.clone(), true), "hello", unknown, true)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(
            app(pool.clone(), true),
            "channel",
            json!({"agent_id":"agent-a","owner_epoch":1,"wait_seconds":26}),
            true
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let mut wrong_agent = hello_body(1);
    wrong_agent["agent_id"] = json!("agent-b");
    assert_eq!(
        post(app(pool.clone(), true), "hello", wrong_agent, true)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(post(app(pool.clone(),true),"stream",json!({"agent_id":"agent-a","owner_epoch":1,"request_id":Uuid::now_v7(),"chunks":vec![json!({"index":0,"data":""});65]}),true).await.status(),StatusCode::PAYLOAD_TOO_LARGE);
    let chunk = json!({"agent_id":"agent-a","owner_epoch":1,"request_id":Uuid::now_v7(),"chunks":[{"index":0,"data":URL_SAFE_NO_PAD.encode(vec![0;MIB+1])}]});
    assert_eq!(
        post(app(pool, true), "stream", chunk, true).await.status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
#[tokio::test]
async fn postgres_hello_channel_results_and_snapshots() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::open().await else {
        return;
    };
    let app = app(db.pool.clone(), true);
    assert_eq!(
        post(app.clone(), "hello", hello_body(2), true)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(app.clone(), "hello", hello_body(1), true)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let r = request();
    db.store().enqueue(&r).await.unwrap();
    let poll = json!({"agent_id":"agent-a","owner_epoch":2,"wait_seconds":0});
    let response = post(app.clone(), "channel", poll, true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 2 * MIB).await.unwrap()).unwrap();
    assert_eq!(value["requests"].as_array().unwrap().len(), 1);
    let signed: signer::RequestEnvelope =
        serde_json::from_value(value["requests"][0].clone()).unwrap();
    assert_eq!(
        signer::adm_x1_signing_bytes(&signed).unwrap(),
        r.canonical_envelope
    );
    assert_eq!(
        value["requests"][0]["signature"],
        URL_SAFE_NO_PAD.encode(&r.signature)
    );
    for status in ["accepted", "in_progress", "succeeded"] {
        let mut result = json!({"owner_epoch":2,"envelope":{"request_id":r.request_id,"admin_operation_id":r.admin_operation_id,"status":status,"error":null,"result":{},"dry_run_preview":null,"game_ledger_id":"ledger-a","audit_event_ids":[],"effective_revision":null,"progress":null,"completed_at":null,"agent_id":"agent-a","agent_build":"test-build","signature":"unsigned"}});
        use ed25519_dalek::Signer;
        let envelope: signer::ResultEnvelope =
            serde_json::from_value(result["envelope"].clone()).unwrap();
        result["envelope"]["signature"] = json!(
            URL_SAFE_NO_PAD.encode(
                ed25519_dalek::SigningKey::from_bytes(&[9; 32])
                    .sign(&signer::adm_x1_signing_bytes(&envelope).unwrap())
                    .to_bytes()
            )
        );
        for _ in 0..2 {
            assert_eq!(
                post(app.clone(), "results", result.clone(), true)
                    .await
                    .status(),
                StatusCode::OK
            );
        }
    }
    assert_eq!(db.store().results(r.request_id, 32).await.unwrap().len(), 3);
    assert_eq!(
        db.store().get(r.request_id).await.unwrap().unwrap().status,
        Status::Succeeded
    );
    let snapshot = json!({"agent_id":"agent-a","owner_epoch":2,"payload":{"healthy":true}});
    assert_eq!(
        post(app.clone(), "heartbeat", snapshot.clone(), true)
            .await
            .status(),
        StatusCode::OK
    );
    let mut rollup = snapshot;
    rollup["key"] = json!("online");
    assert_eq!(
        post(app.clone(), "rollups", rollup, true).await.status(),
        StatusCode::OK
    );
    let chunk = json!({"agent_id":"agent-a","owner_epoch":2,"request_id":r.request_id,"chunks":[{"index":0,"data":URL_SAFE_NO_PAD.encode(b"test")}]});
    for _ in 0..2 {
        assert_eq!(
            post(app.clone(), "stream", chunk.clone(), true)
                .await
                .status(),
            StatusCode::OK
        );
    }
    let mut conflict = chunk;
    conflict["chunks"][0]["data"] = json!(URL_SAFE_NO_PAD.encode(b"other"));
    assert_eq!(
        post(app, "stream", conflict, true).await.status(),
        StatusCode::CONFLICT
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM analytics_cache")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.close().await;
}
#[tokio::test]
async fn postgres_expired_request_never_delivered() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::open().await else {
        return;
    };
    let app = app(db.pool.clone(), true);
    assert_eq!(
        post(app.clone(), "hello", hello_body(1), true)
            .await
            .status(),
        StatusCode::OK
    );
    let mut r = request();
    r.issued_at = OffsetDateTime::now_utc() - Duration::minutes(2);
    r.expires_at = OffsetDateTime::now_utc() - Duration::minutes(1);
    db.store().enqueue(&r).await.unwrap();
    let response = post(
        app,
        "channel",
        json!({"agent_id":"agent-a","owner_epoch":1}),
        true,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), MIB).await.unwrap()).unwrap();
    assert!(value["requests"].as_array().unwrap().is_empty());
    assert!(
        db.store()
            .get(r.request_id)
            .await
            .unwrap()
            .unwrap()
            .delivered_at
            .is_none()
    );
    db.close().await;
}

#[tokio::test]
async fn configured_result_key_refuses_tampering_before_storage() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
    let routes = router(
        pool,
        true,
        vec![AgentBinding {
            agent_id: "agent-a".into(),
            identity: Identity::san("urn:test:agent-a").unwrap(),
            result_key: Some((
                key.verifying_key(),
                signer::Validity {
                    not_before: 0,
                    not_after: i64::MAX,
                },
            )),
        }],
    )
    .unwrap();
    let mut envelope:signer::ResultEnvelope=serde_json::from_value(json!({"request_id":Uuid::now_v7(),"admin_operation_id":Uuid::now_v7(),"status":"succeeded","error":null,"result":{},"dry_run_preview":null,"game_ledger_id":null,"audit_event_ids":[],"effective_revision":null,"progress":null,"completed_at":null,"agent_id":"agent-a","agent_build":"test-build","signature":""})).unwrap();
    use ed25519_dalek::Signer;
    envelope.signature = URL_SAFE_NO_PAD.encode(
        key.sign(&signer::adm_x1_signing_bytes(&envelope).unwrap())
            .to_bytes(),
    );
    signer::verify_result(
        &envelope,
        &key.verifying_key(),
        signer::Validity {
            not_before: 0,
            not_after: i64::MAX,
        },
        OffsetDateTime::now_utc().unix_timestamp(),
    )
    .unwrap();
    envelope.agent_build = "tampered".into();
    let response = post(
        routes,
        "results",
        json!({"owner_epoch":1,"envelope":envelope}),
        true,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), MIB).await.unwrap()).unwrap();
    assert_eq!(value["error"]["code"], "ADM_BAD_SIGNATURE");
}

#[test]
fn agent_identity_requires_flag_exact_cn_and_bound_san() {
    let bindings = [
        AgentBinding {
            agent_id: "agent-a".into(),
            identity: Identity::san("agent-a.oteryn.internal").unwrap(),
            result_key: None,
        },
        AgentBinding {
            agent_id: "agent-spki".into(),
            identity: Identity::spki_sha256([7; 32]),
            result_key: None,
        },
    ];
    let san = Identity::san("agent-a.oteryn.internal").unwrap();
    assert!(AgentIdentities::new(false, &bindings).is_none());
    let agents = AgentIdentities::new(true, &bindings).unwrap();
    let agent = agents.verify("oteryn-admin-agent/agent-a", &san).unwrap();
    assert_eq!(agent.agent_id(), "agent-a");
    // The CN selects an entry; another verified identity never satisfies it.
    let other = Identity::san("agent-b.oteryn.internal").unwrap();
    assert!(
        agents
            .verify("oteryn-admin-agent/agent-a", &other)
            .is_none()
    );
    assert!(agents.verify("oteryn-admin-agent/agent-b", &san).is_none());
    for cn in [
        "agent-a",
        "oteryn-admin-agent/",
        "oteryn-admin-agent/agent a",
        "Oteryn-Admin-Agent/agent-a",
        " oteryn-admin-agent/agent-a",
        "oteryn-admin-agent/agent-a/x",
    ] {
        assert!(agents.verify(cn, &san).is_none(), "{cn}");
    }
    // An SPKI-bound agent is not matched by a SAN identity.
    assert!(
        agents
            .verify("oteryn-admin-agent/agent-spki", &san)
            .is_none()
    );
    assert!(
        agents
            .verify(
                "oteryn-admin-agent/agent-spki",
                &Identity::spki_sha256([7; 32])
            )
            .is_some()
    );
    // Duplicate agent ids are ambiguous.
    let duplicate =
        AgentIdentities::new(true, &[bindings[0].clone(), bindings[0].clone()]).unwrap();
    assert!(
        duplicate
            .verify("oteryn-admin-agent/agent-a", &san)
            .is_none()
    );
}

#[tokio::test]
async fn postgres_hello_waits_for_control_before_http_ownership() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&db.root)
        .await
        .unwrap();
    assert!(
        version.starts_with("18."),
        "PostgreSQL 18 required: {version}"
    );
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let schema = db.schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |connection, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)")
                    .bind(schema).execute(connection).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    let mut blocker = db.pool.begin().await.unwrap();
    store::ownership_lock(&mut blocker, false).await.unwrap();
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let app = app(pool.clone(), true);
    let hello = tokio::spawn(async move { post(app, "hello", hello_body(1), true).await.status() });
    // PostgreSQL's wait graph is the barrier: hello has reached the ownership
    // lock and cannot advance until this transaction releases it.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND $2=ANY(pg_blocking_pids(pid)))",
            )
            .bind(&db.schema)
            .bind(blocker_pid)
            .fetch_one(&db.root)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let http_available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock_shared(hashtextextended('admin-control-http',0))",
    )
    .fetch_one(&mut *blocker)
    .await
    .unwrap();
    blocker.rollback().await.unwrap();
    let status = hello.await.unwrap();
    pool.close().await;
    db.close().await;
    assert!(
        http_available,
        "hello acquired HTTP ownership while waiting for control ownership"
    );
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn postgres_hello_uses_one_connection_under_ingress_contention() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let schema = db.schema.clone();
    let pool = PgPoolOptions::new().max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .after_connect(move |c, _| { let schema = schema.clone(); Box::pin(async move {
            sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)").bind(schema).execute(c).await?; Ok(())
        }) }).connect(&url).await.unwrap();
    let app = app(pool.clone(), true);
    // Pause hello's conflict read after ingress is exclusively locked.
    let mut blocker = db.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE admin_control_agents IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let hello_app = app.clone();
    let hello =
        tokio::spawn(async move { post(hello_app, "hello", hello_body(2), true).await.status() });
    wait_control_lock(&db, "SELECT EXISTS(SELECT 1 FROM admin_control_agents%").await;
    let channel = tokio::spawn(async move {
        post(
            app,
            "channel",
            json!({"agent_id":"agent-a","owner_epoch":2,"wait_seconds":0}),
            true,
        )
        .await
        .status()
    });
    wait_control_lock(
        &db,
        "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-http'%",
    )
    .await;
    blocker.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(1), hello)
            .await
            .unwrap()
            .unwrap(),
        StatusCode::OK
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(1), channel)
            .await
            .unwrap()
            .unwrap(),
        StatusCode::OK
    );
    pool.close().await;
    db.close().await;
    eprintln!(
        "postgres_hello_uses_one_connection_under_ingress_contention duration={:?}",
        started.elapsed()
    );
}
async fn wait_control_lock(db: &Database, query: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock' AND query LIKE $2)")
                .bind(&db.schema).bind(query).fetch_one(&db.root).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}

#[tokio::test]
async fn postgres_result_uses_one_connection_under_stream_contention() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use ed25519_dalek::Signer;
    let started = std::time::Instant::now();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let schema = db.schema.clone();
    let pool=PgPoolOptions::new().max_connections(2).acquire_timeout(std::time::Duration::from_secs(2))
        .after_connect(move |c,_| {let schema=schema.clone();Box::pin(async move {
            sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)").bind(schema).execute(c).await?;Ok(())
        })}).connect(&url).await.unwrap();
    let app = app(pool.clone(), true);
    assert_eq!(
        post(app.clone(), "hello", hello_body(2), true)
            .await
            .status(),
        StatusCode::OK
    );
    let r = request();
    db.store().enqueue(&r).await.unwrap();
    assert_eq!(
        post(
            app.clone(),
            "channel",
            json!({"agent_id":"agent-a","owner_epoch":2,"wait_seconds":0}),
            true
        )
        .await
        .status(),
        StatusCode::OK
    );
    let mut result = json!({"owner_epoch":2,"envelope":{"request_id":r.request_id,"admin_operation_id":r.admin_operation_id,"status":"accepted","error":null,"result":{},"dry_run_preview":null,"game_ledger_id":"ledger-a","audit_event_ids":[],"effective_revision":null,"progress":null,"completed_at":null,"agent_id":"agent-a","agent_build":"test-build","signature":"unsigned"}});
    let envelope: signer::ResultEnvelope =
        serde_json::from_value(result["envelope"].clone()).unwrap();
    result["envelope"]["signature"] = json!(
        URL_SAFE_NO_PAD.encode(
            ed25519_dalek::SigningKey::from_bytes(&[9; 32])
                .sign(&signer::adm_x1_signing_bytes(&envelope).unwrap())
                .to_bytes()
        )
    );
    let mut blocker = db.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE admin_control_results IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let result_app = app.clone();
    let result =
        tokio::spawn(async move { post(result_app, "results", result, true).await.status() });
    wait_control_lock(&db, "SELECT EXISTS(SELECT 1 FROM admin_control_results%").await;
    let stream = tokio::spawn(async move {
        post(app,"stream",json!({"agent_id":"agent-a","owner_epoch":2,"request_id":r.request_id,"chunks":[{"index":0,"data":URL_SAFE_NO_PAD.encode(b"test") }]}),true).await.status()
    });
    wait_control_lock(
        &db,
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 17))",
    )
    .await;
    blocker.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(1), result)
            .await
            .unwrap()
            .unwrap(),
        StatusCode::OK
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(1), stream)
            .await
            .unwrap()
            .unwrap(),
        StatusCode::OK
    );
    pool.close().await;
    db.close().await;
    eprintln!(
        "postgres_result_uses_one_connection_under_stream_contention duration={:?}",
        started.elapsed()
    );
}
