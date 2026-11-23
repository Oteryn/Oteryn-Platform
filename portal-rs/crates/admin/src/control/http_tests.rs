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
    request_for("staff.warning", Kind::Command)
}
/// A signed request for a catalogued operation; queries reuse the request id.
fn request_for(operation: &str, kind: Kind) -> NewRequest {
    let now = OffsetDateTime::now_utc();
    let request_id = Uuid::now_v7();
    let mut input = NewRequest {
        request_id,
        admin_operation_id: if kind == Kind::Query {
            request_id
        } else {
            Uuid::now_v7()
        },
        operation: operation.into(),
        operation_version: 1,
        kind,
        target: if catalog::GLOBAL_READS.contains(&operation) {
            Target::default()
        } else {
            Target {
                world_id: Some("world-a".into()),
                node_id: None,
                character_id: None,
                account_ref: None,
            }
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
        canonical_envelope: b"{}".to_vec(),
        signature: vec![4; 64],
    };
    let mut envelope: signer::RequestEnvelope = serde_json::from_value(json!({
        "contract":"oteryn.admin-control.v1", "request_id":input.request_id,
        "admin_operation_id":input.admin_operation_id,"operation":input.operation,
        "operation_version":1,"kind":input.kind.as_str(),"target":input.target,
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
    app_with_scope(pool, enabled, vec!["world-a".into()], vec![])
}

fn app_with_scope(
    pool: PgPool,
    enabled: bool,
    allowed_worlds: Vec<String>,
    allowed_nodes: Vec<String>,
) -> Router {
    router(
        pool,
        enabled,
        vec![AgentBinding {
            allowed_worlds,
            allowed_nodes,
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
/// Game agent hello framing, transcribed from Oteryn-Game
/// `crates/admin/agent/src/channel.rs` L120-129 at
/// 3d4ea6ed913359c2e5aafb7ab6b890a894ff32fa (framing ratified 2026-11-19).
fn game_hello(epoch: i64, operations: Value) -> Value {
    json!({"agent_id":"agent-a","build_revision":"test-build","catalog_digest":URL_SAFE_NO_PAD.encode([1;32]),"operations":operations,"worlds":["world-a"],"nodes":[],"owner_epoch":epoch,"clock":"2026-11-05T23:00:00Z"})
}
fn hello_body(epoch: i64) -> Value {
    game_hello(
        epoch,
        json!([{"operation":"staff.warning","version":1},{"operation":"admin.policy.get","version":1}]),
    )
}
/// Game poll framing (`channel.rs` L329 at the same commit).
fn game_poll(epoch: i64, wait_seconds: u8) -> Value {
    json!({"agent_id":"agent-a","owner_epoch":epoch,"wait_seconds":wait_seconds,"limit":32})
}
/// Game heartbeat framing (`channel.rs` L131 and L322 at the same commit).
fn game_heartbeat(epoch: i64) -> Value {
    json!({"agent_id":"agent-a","owner_epoch":epoch,"clock":"2026-11-05T23:00:10Z"})
}
/// A bare result envelope in Game's `Response` shape, signed by the configured agent key.
fn signed_result(r: &NewRequest, status: &str, result: Value, error: Value) -> Value {
    use ed25519_dalek::Signer;
    let mut envelope = json!({"request_id":r.request_id,"admin_operation_id":r.admin_operation_id,"status":status,"error":error,"result":result,"dry_run_preview":null,"game_ledger_id":"ledger-a","audit_event_ids":[],"effective_revision":null,"progress":null,"completed_at":"2026-11-05T23:00:01Z","agent_id":"agent-a","agent_build":"test-build","signature":""});
    let parsed: signer::ResultEnvelope = serde_json::from_value(envelope.clone()).unwrap();
    envelope["signature"] = json!(
        URL_SAFE_NO_PAD.encode(
            ed25519_dalek::SigningKey::from_bytes(&[9; 32])
                .sign(&signer::adm_x1_signing_bytes(&parsed).unwrap())
                .to_bytes()
        )
    );
    envelope
}
async fn json_body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 2 * MIB).await.unwrap()).unwrap()
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
#[test]
fn binding_requires_both_scope_arrays() {
    let mut binding =
        json!({"cn":"oteryn-admin-agent/agent-a","identity":{"san":"agent-a.internal"}});
    assert!(serde_json::from_value::<ConfiguredAgent>(binding.clone()).is_err());
    binding["allowed_worlds"] = json!(["world-a"]);
    assert!(serde_json::from_value::<ConfiguredAgent>(binding.clone()).is_err());
    binding["allowed_nodes"] = json!([]);
    assert!(serde_json::from_value::<ConfiguredAgent>(binding).is_ok());
}

#[tokio::test]
async fn hello_refuses_out_of_scope_claims() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open().await.unwrap();
    let app = app(db.pool.clone(), true);
    for field in ["worlds", "nodes"] {
        let mut body = hello_body(9);
        body[field] = json!(["outside"]);
        assert_eq!(
            post(app.clone(), "hello", body, true).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_control_agents")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
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
    let response = post(app.clone(), "channel", game_poll(2, 0), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let value = json_body(response).await;
    assert_eq!(value.as_array().unwrap().len(), 1);
    let signed: signer::RequestEnvelope = serde_json::from_value(value[0].clone()).unwrap();
    assert_eq!(
        signer::adm_x1_signing_bytes(&signed).unwrap(),
        r.canonical_envelope
    );
    assert_eq!(value[0]["signature"], URL_SAFE_NO_PAD.encode(&r.signature));
    for status in ["accepted", "in_progress", "succeeded"] {
        let result = signed_result(&r, status, json!({}), Value::Null);
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
    let mut snapshot = game_heartbeat(2);
    snapshot["health"] = json!({"healthy":true});
    assert_eq!(
        post(app.clone(), "heartbeat", snapshot, true)
            .await
            .status(),
        StatusCode::OK
    );
    let rollup =
        json!({"agent_id":"agent-a","owner_epoch":2,"key":"online","payload":{"healthy":true}});
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
    assert!(json_body(response).await.as_array().unwrap().is_empty());
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
            allowed_worlds: vec!["world-a".into()],
            allowed_nodes: vec![],
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
    let response = post(routes, "results", json!(envelope), true).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), MIB).await.unwrap()).unwrap();
    assert_eq!(value["error"]["code"], "ADM_BAD_SIGNATURE");
}

#[test]
fn agent_identity_requires_flag_exact_cn_and_bound_san() {
    let bindings = [
        AgentBinding {
            allowed_worlds: vec!["world-a".into()],
            allowed_nodes: vec![],
            agent_id: "agent-a".into(),
            identity: Identity::san("agent-a.oteryn.internal").unwrap(),
            result_key: None,
        },
        AgentBinding {
            allowed_worlds: vec!["world-a".into()],
            allowed_nodes: vec![],
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
    let result = signed_result(&r, "accepted", json!({}), Value::Null);
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

#[tokio::test]
async fn game_wire_bad_input_is_refused_before_storage() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let refused = |path: &'static str, body: Value| {
        let app = app(pool.clone(), true);
        async move { post(app, path, body, true).await }
    };
    // The agent never supplies an operation kind; the trusted catalogue does.
    let with_kind = game_hello(
        1,
        json!([{"operation":"admin.policy.get","version":1,"kind":"query"}]),
    );
    assert_eq!(
        refused("hello", with_kind).await.status(),
        StatusCode::BAD_REQUEST
    );
    let legacy = json!({"agent_id":"agent-a","build":"test-build","catalog_digest":URL_SAFE_NO_PAD.encode([1;32]),"advertised_operations":[{"operation":"admin.policy.get","operation_version":1,"kind":"query"}],"served_worlds":["world-a"],"served_nodes":[],"owner_epoch":1,"clock":"2026-11-05T23:00:00Z"});
    assert_eq!(
        refused("hello", legacy).await.status(),
        StatusCode::BAD_REQUEST
    );
    for operations in [
        json!([{"operation":"unknown.operation","version":1}]),
        json!([{"operation":"admin.agent.list","version":1}]),
        json!([{"operation":"analytics.catalog","version":1}]),
        json!([{"operation":"admin.policy.get","version":2}]),
        json!([{"operation":"staff.warning","version":0}]),
    ] {
        let response = refused("hello", game_hello(1, operations)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            json_body(response).await["error"]["code"],
            "ADM_UNKNOWN_OPERATION"
        );
    }
    for limit in [0, 33] {
        let mut poll = game_poll(1, 0);
        poll["limit"] = json!(limit);
        assert_eq!(
            refused("channel", poll).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        refused("channel", game_poll(1, 26)).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut clock = game_heartbeat(1);
    clock["clock"] = json!("not-a-clock");
    assert_eq!(
        refused("heartbeat", clock).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut missing = game_heartbeat(1);
    missing.as_object_mut().unwrap().remove("clock");
    assert_eq!(
        refused("heartbeat", missing).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut legacy = game_heartbeat(1);
    legacy["payload"] = json!({"healthy":true});
    assert_eq!(
        refused("heartbeat", legacy).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut scalar = game_heartbeat(1);
    scalar["health"] = json!(true);
    assert_eq!(
        refused("heartbeat", scalar).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut wrong = game_heartbeat(1);
    wrong["agent_id"] = json!("agent-b");
    assert_eq!(
        refused("heartbeat", wrong).await.status(),
        StatusCode::FORBIDDEN
    );
    // The former wrapped result framing is no longer accepted.
    let r = request();
    let wrapped =
        json!({"owner_epoch":1,"envelope":signed_result(&r, "succeeded", json!({}), Value::Null)});
    assert_eq!(
        refused("results", wrapped).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut other = signed_result(&r, "succeeded", json!({}), Value::Null);
    other["agent_id"] = json!("agent-b");
    assert_eq!(
        refused("results", other).await.status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn postgres_game_hello_registers_catalogue_kinds() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let app = app(db.pool.clone(), true);
    let unknown = game_hello(
        2,
        json!([{"operation":"admin.policy.get","version":1},{"operation":"unknown.operation","version":1}]),
    );
    assert_eq!(
        post(app.clone(), "hello", unknown, true).await.status(),
        StatusCode::BAD_REQUEST
    );
    let count = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM admin_control_agents")
            .fetch_one(&db.pool)
            .await
            .unwrap()
    };
    assert_eq!(count().await, 0);
    let operations: Vec<_> = catalog::GAME_OPERATIONS
        .iter()
        .filter(|op| op.phase == 2)
        .map(|op| json!({"operation":op.name,"version":1}))
        .collect();
    assert!(!operations.is_empty());
    let response = post(app.clone(), "hello", game_hello(2, json!(operations)), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let advertised: Value = sqlx::query_scalar(
        "SELECT advertised_operations FROM admin_control_agents WHERE agent_id='agent-a'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(advertised, json!([]));
    let unadvertised = request_for("db.status", Kind::Query);
    db.store().enqueue(&unadvertised).await.unwrap();
    let response = post(app.clone(), "channel", game_poll(2, 0), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, json!([]));

    let mut operations = operations;
    operations.extend(
        catalog::REGISTERED_READS
            .iter()
            .map(|operation| json!({"operation":operation,"version":1})),
    );
    let response = post(app.clone(), "hello", game_hello(2, json!(operations)), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, json!({"status":"registered"}));
    let row = sqlx::query("SELECT build,catalog_digest,advertised_operations,served_worlds,served_nodes,owner_epoch FROM admin_control_agents WHERE agent_id='agent-a'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    use sqlx::Row;
    assert_eq!(row.get::<String, _>("build"), "test-build");
    assert_eq!(row.get::<Vec<u8>, _>("catalog_digest"), vec![1; 32]);
    assert_eq!(row.get::<i64, _>("owner_epoch"), 2);
    assert_eq!(
        row.get::<Vec<String>, _>("served_worlds"),
        vec!["world-a".to_owned()]
    );
    assert!(row.get::<Vec<String>, _>("served_nodes").is_empty());
    assert_eq!(
        row.get::<Value, _>("advertised_operations"),
        json!(
            catalog::REGISTERED_READS
                .iter()
                .map(|operation| {
                    json!({"operation":operation,"operation_version":1,"kind":"query"})
                })
                .collect::<Vec<_>>()
        )
    );
    let mut outside = game_hello(3, json!([{"operation":"admin.policy.get","version":1}]));
    outside["worlds"] = json!(["outside"]);
    assert_eq!(
        post(app, "hello", outside, true).await.status(),
        StatusCode::FORBIDDEN
    );
    let epoch: i64 = sqlx::query_scalar("SELECT owner_epoch FROM admin_control_agents")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(epoch, 2);
    assert_eq!(count().await, 1);
    db.close().await;
}

#[tokio::test]
async fn postgres_agent_channel_follows_configured_scope() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let world_router = app(db.pool.clone(), true);
    let node_router = app_with_scope(db.pool.clone(), true, vec![], vec!["node-a".into()]);
    let advertised = json!([{"operation":"ops.overview.get","version":1},{"operation":"db.status","version":1},{"operation":"db.migrations.list","version":1}]);
    let mut node_hello = game_hello(1, advertised.clone());
    node_hello["nodes"] = json!(["node-a"]);
    assert_eq!(
        post(node_router.clone(), "hello", node_hello.clone(), true)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            world_router.clone(),
            "hello",
            game_hello(1, advertised),
            true
        )
        .await
        .status(),
        StatusCode::OK
    );

    let delivered = request_for("ops.overview.get", Kind::Query);
    db.store().enqueue(&delivered).await.unwrap();
    let response = post(world_router, "channel", game_poll(1, 0), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_body(response).await[0]["request_id"],
        json!(delivered.request_id)
    );

    let queued = request_for("ops.overview.get", Kind::Query);
    db.store().enqueue(&queued).await.unwrap();
    assert_eq!(
        post(node_router.clone(), "channel", game_poll(1, 0), true)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let result = signed_result(
        &delivered,
        "succeeded",
        json!({"operations":[]}),
        Value::Null,
    );
    assert_eq!(
        post(node_router.clone(), "results", result, true)
            .await
            .status(),
        StatusCode::CONFLICT
    );

    node_hello["owner_epoch"] = json!(2);
    node_hello["worlds"] = json!([]);
    assert_eq!(
        post(node_router.clone(), "hello", node_hello, true)
            .await
            .status(),
        StatusCode::OK
    );
    let response = post(node_router, "channel", game_poll(2, 0), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, json!([]));
    let status: String =
        sqlx::query_scalar("SELECT status FROM admin_control_requests WHERE request_id=$1")
            .bind(queued.request_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(status, "queued");
    let global_app = app_with_scope(db.pool.clone(), true, vec![], vec![]);
    let mut global_hello = game_hello(
        3,
        json!([{"operation":"db.status","version":1},{"operation":"db.migrations.list","version":1},{"operation":"ops.overview.get","version":1}]),
    );
    global_hello["worlds"] = json!([]);
    assert_eq!(
        post(global_app.clone(), "hello", global_hello, true)
            .await
            .status(),
        StatusCode::OK
    );
    for operation in ["db.status", "db.migrations.list"] {
        let input = request_for(operation, Kind::Query);
        let mut scoped = input.clone();
        scoped.target.world_id = Some("world-a".into());
        assert!(matches!(
            db.store().enqueue(&scoped).await,
            Err(Error::InvalidInput)
        ));
        db.store().enqueue(&input).await.unwrap();
    }
    let response = post(global_app, "channel", game_poll(3, 0), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let batch: Vec<signer::RequestEnvelope> =
        serde_json::from_value(json_body(response).await).unwrap();
    assert_eq!(batch.len(), 2);
    for envelope in batch {
        assert!(["db.status", "db.migrations.list"].contains(&envelope.operation.as_str()));
        assert!(envelope.target.world_id.is_none() && envelope.target.node_id.is_none());
    }
    db.close().await;
}

async fn last_seen(db: &Database) -> OffsetDateTime {
    sqlx::query_scalar("SELECT last_seen_at FROM admin_control_agents WHERE agent_id='agent-a'")
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
async fn health_rows(db: &Database) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM admin_control_health")
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn postgres_game_poll_result_and_heartbeat() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let app = app(db.pool.clone(), true);
    assert_eq!(
        post(
            app.clone(),
            "hello",
            game_hello(2, json!([{"operation":"admin.policy.get","version":1}])),
            true
        )
        .await
        .status(),
        StatusCode::OK
    );
    let ok = request_for("admin.policy.get", Kind::Query);
    let refused = request_for("admin.policy.get", Kind::Query);
    db.store().enqueue(&ok).await.unwrap();
    db.store().enqueue(&refused).await.unwrap();
    // A smaller limit is honoured.
    let mut single = game_poll(2, 0);
    single["limit"] = json!(1);
    let response = post(app.clone(), "channel", single, true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let first = json_body(response).await;
    assert_eq!(first.as_array().unwrap().len(), 1);
    // A request is ready, so Game's 25-second long poll returns at once.
    let response = post(app.clone(), "channel", game_poll(2, 25), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rest = json_body(response).await;
    let batch: Vec<&Value> = first
        .as_array()
        .unwrap()
        .iter()
        .chain(rest.as_array().unwrap())
        .collect();
    assert_eq!(batch.len(), 2);
    assert_ne!(batch[0]["request_id"], batch[1]["request_id"]);
    for value in batch {
        let signed: signer::RequestEnvelope = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(signed.operation, "admin.policy.get");
    }
    let result = signed_result(&ok, "succeeded", json!({"operations":[]}), Value::Null);
    for _ in 0..2 {
        let response = post(app.clone(), "results", result.clone(), true).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, json!({"status":"recorded"}));
    }
    let stored = db.store().results(ok.request_id, 32).await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].status, Status::Succeeded);
    assert_eq!(
        db.store().get(ok.request_id).await.unwrap().unwrap().status,
        Status::Succeeded
    );
    // Game's refusal shape: null result, closed error detail.
    let rejection = signed_result(
        &refused,
        "rejected",
        Value::Null,
        json!({"code":"ADM_FORBIDDEN","message":"AdmForbidden","retryable":false}),
    );
    let response = post(app.clone(), "results", rejection, true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        db.store()
            .get(refused.request_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        Status::Rejected
    );
    let mut tampered = signed_result(&ok, "failed", json!({}), Value::Null);
    tampered["agent_build"] = json!("tampered");
    assert_eq!(
        post(app.clone(), "results", tampered, true).await.status(),
        StatusCode::FORBIDDEN
    );
    // Liveness-only heartbeat: advances last_seen_at and writes no health row.
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=clock_timestamp()-interval '20 seconds' WHERE agent_id='agent-a'")
        .execute(&db.pool)
        .await
        .unwrap();
    let before = last_seen(&db).await;
    let response = post(app.clone(), "heartbeat", game_heartbeat(2), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, json!({"status":"recorded"}));
    assert!(last_seen(&db).await > before + Duration::seconds(10));
    assert_eq!(health_rows(&db).await, 0);
    // A stale epoch is refused before liveness moves.
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=$1 WHERE agent_id='agent-a'")
        .bind(before)
        .execute(&db.pool)
        .await
        .unwrap();
    let response = post(app.clone(), "heartbeat", game_heartbeat(1), true).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "ADM_AGENT_SUPERSEDED"
    );
    assert_eq!(last_seen(&db).await, before);
    // An optional health snapshot is still stored when supplied.
    let mut snapshot = game_heartbeat(2);
    snapshot["health"] = json!({"nodes":[]});
    assert_eq!(
        post(app, "heartbeat", snapshot, true).await.status(),
        StatusCode::OK
    );
    assert_eq!(health_rows(&db).await, 1);
    db.close().await;
}

#[tokio::test]
async fn postgres_result_is_bound_to_its_delivery_epoch() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open()
        .await
        .expect("PORTAL_TEST_DATABASE_URL required");
    let app = app(db.pool.clone(), true);
    let hello = game_hello(2, json!([{"operation":"admin.policy.get","version":1}]));
    assert_eq!(
        post(app.clone(), "hello", hello, true).await.status(),
        StatusCode::OK
    );
    let r = request_for("admin.policy.get", Kind::Query);
    db.store().enqueue(&r).await.unwrap();
    let response = post(app.clone(), "channel", game_poll(2, 0), true).await;
    assert_eq!(json_body(response).await.as_array().unwrap().len(), 1);
    // The same agent restarts under a newer epoch; the epoch-2 delivery is stale.
    let hello = game_hello(3, json!([{"operation":"admin.policy.get","version":1}]));
    assert_eq!(
        post(app.clone(), "hello", hello, true).await.status(),
        StatusCode::OK
    );
    let result = signed_result(&r, "succeeded", json!({"operations":[]}), Value::Null);
    let response = post(app.clone(), "results", result, true).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(response).await["error"]["code"],
        "ADM_AGENT_SUPERSEDED"
    );
    assert!(
        db.store()
            .results(r.request_id, 32)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.store().get(r.request_id).await.unwrap().unwrap().status,
        Status::Delivered
    );
    // A request delivered under the current epoch is still accepted.
    let current = request_for("admin.policy.get", Kind::Query);
    db.store().enqueue(&current).await.unwrap();
    let response = post(app.clone(), "channel", game_poll(3, 0), true).await;
    assert_eq!(json_body(response).await.as_array().unwrap().len(), 1);
    let result = signed_result(&current, "succeeded", json!({"operations":[]}), Value::Null);
    assert_eq!(
        post(app.clone(), "results", result, true).await.status(),
        StatusCode::OK
    );
    // A request never delivered has no delivery session to bind to.
    let queued = request_for("admin.policy.get", Kind::Query);
    db.store().enqueue(&queued).await.unwrap();
    let result = signed_result(&queued, "succeeded", json!({"operations":[]}), Value::Null);
    assert_eq!(
        post(app, "results", result, true).await.status(),
        StatusCode::CONFLICT
    );
    assert!(
        db.store()
            .results(queued.request_id, 32)
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await;
}

#[tokio::test]
async fn postgres_adm_results_validate_signed_payload_against_stored_operation() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open().await.expect("PostgreSQL required");
    let app = app(db.pool.clone(), true);
    assert_eq!(
        post(
            app.clone(),
            "hello",
            game_hello(
                1,
                json!([
                    {"operation":"node.list","version":1},
                    {"operation":"ops.overview.get","version":1}
                ])
            ),
            true,
        )
        .await
        .status(),
        StatusCode::OK
    );
    let request = request_for("node.list", Kind::Query);
    db.store().enqueue(&request).await.unwrap();
    let claimed = post(app.clone(), "channel", game_poll(1, 0), true).await;
    assert_eq!(claimed.status(), StatusCode::OK);
    assert_eq!(
        json_body(claimed).await[0]["request_id"],
        request.request_id.to_string()
    );
    let payload = json!({"nodes":[{
        "node_id":"node-test", "host":"node.internal", "build":"test-build",
        "uptime_seconds":12, "worlds":[], "ready":true, "cpu_percent":18.5, "memory_bytes":"1024"
    }]});
    let result = signed_result(&request, "succeeded", payload.clone(), Value::Null);
    assert_eq!(
        post(app.clone(), "results", result, true).await.status(),
        StatusCode::OK
    );
    let stored = db.store().results(request.request_id, 1).await.unwrap();
    assert_eq!(stored[0].payload["result"], payload);

    let overview = request_for("ops.overview.get", Kind::Query);
    db.store().enqueue(&overview).await.unwrap();
    let claimed = post(app.clone(), "channel", game_poll(1, 0), true).await;
    assert_eq!(claimed.status(), StatusCode::OK);
    assert_eq!(
        json_body(claimed).await[0]["request_id"],
        overview.request_id.to_string()
    );
    let mut counts = json!({
        "agents":[], "worlds":[], "nodes":[], "players_online":18.5,
        "db_available":true, "alerts":[]
    });
    let mut overflow = counts.clone();
    overflow["players_online"] = json!(u64::from(u32::MAX) + 1);
    for invalid in [counts.clone(), overflow, payload, Value::Null] {
        let result = signed_result(&overview, "succeeded", invalid, Value::Null);
        let refused = post(app.clone(), "results", result, true).await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            json_body(refused).await["error"]["code"],
            "ADM_INVALID_PARAMS"
        );
        assert!(
            db.store()
                .results(overview.request_id, 1)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            db.store()
                .get(overview.request_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            Status::Delivered
        );
    }
    counts["players_online"] = json!(u32::MAX);
    let result = signed_result(&overview, "succeeded", counts.clone(), Value::Null);
    assert_eq!(
        post(app, "results", result, true).await.status(),
        StatusCode::OK
    );
    let stored = db.store().results(overview.request_id, 1).await.unwrap();
    assert_eq!(stored[0].payload["result"], counts);
    db.close().await;
}

#[tokio::test]
async fn exact_legacy_read_retry_is_acknowledged_only_by_its_delivery_owner() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::open().await.expect("PostgreSQL required");
    let app = app(db.pool.clone(), true);
    assert_eq!(
        post(app.clone(), "hello", hello_body(1), true)
            .await
            .status(),
        StatusCode::OK
    );
    let request = request_for("admin.policy.get", Kind::Query);
    db.store().enqueue(&request).await.unwrap();
    let claimed = post(app.clone(), "channel", game_poll(1, 0), true).await;
    assert_eq!(claimed.status(), StatusCode::OK);
    assert_eq!(
        json_body(claimed).await[0]["request_id"],
        request.request_id.to_string()
    );
    // Simulate a receipt persisted before operation schemas were enforced.
    let payload = signed_result(&request, "succeeded", json!({}), Value::Null);
    db.store()
        .record_result(
            &NewResult {
                request_id: request.request_id,
                agent_id: "agent-a".into(),
                status: Status::Succeeded,
                game_ledger_id: Some("ledger-a".into()),
                error_code: None,
                payload: payload.clone(),
            },
            1,
        )
        .await
        .unwrap();
    let audit_count = || {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE request_id=$1")
            .bind(request.request_id)
            .fetch_one(&db.pool)
    };
    let before = audit_count().await.unwrap();
    assert_eq!(
        post(app.clone(), "results", payload.clone(), true)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(app.clone(), "hello", hello_body(2), true)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post(app, "results", payload, true).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        db.store()
            .results(request.request_id, 32)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(audit_count().await.unwrap(), before);
    db.close().await;
}
