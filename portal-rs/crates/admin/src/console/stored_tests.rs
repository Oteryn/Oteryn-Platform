use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use ed25519_dalek::{Signer, SigningKey};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

#[derive(Clone)]
struct Browser(Uuid);
impl web::StaffSession for Browser {
    async fn resolve(&self, _: &BrowserSession) -> Option<web::StaffIdentity> {
        Some(web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
fn missing() -> std::io::Error {
    std::io::Error::other("missing browser evidence")
}
fn agent_request(
    operation: &str,
    body: Value,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let mut request = Request::post(format!("/internal/v1/admin-control/{operation}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))?;
    request.extensions_mut().insert(
        crate::control_http::VerifiedAgent::from_verified_certificate_cn(
            "oteryn-admin-agent/agent-a",
        )
        .map_err(|_| missing())?,
    );
    request.extensions_mut().insert(
        oteryn_internal_api::guard::VerifiedClient::from_verified_tls13_http11(
            oteryn_internal_api::guard::Identity::san("agent-a.internal")?,
        ),
    );
    Ok(request)
}
fn signed_result(
    key: &SigningKey,
    request: &signer::RequestEnvelope,
    value: Value,
) -> Result<signer::ResultEnvelope, Box<dyn std::error::Error>> {
    let mut result = signer::ResultEnvelope {
        request_id: request.request_id,
        admin_operation_id: request.admin_operation_id,
        status: signer::Status::Succeeded,
        error: None,
        result: Json(value),
        dry_run_preview: None,
        game_ledger_id: None,
        audit_event_ids: vec![],
        effective_revision: None,
        progress: None,
        completed_at: Some(OffsetDateTime::now_utc().format(&Rfc3339)?),
        agent_id: "agent-a".into(),
        agent_build: "test-build".into(),
        signature: String::new(),
    };
    result.signature =
        URL_SAFE_NO_PAD.encode(key.sign(&signer::adm_x1_signing_bytes(&result)?).to_bytes());
    Ok(result)
}
async fn answer(
    internal: &Router,
    key: &SigningKey,
    request: signer::RequestEnvelope,
    value: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = signed_result(key, &request, value)?;
    let schema: Value = serde_json::from_str(include_str!(
        "../control/fixtures/admin-control-v1/result.schema.json"
    ))?;
    assert!(jsonschema::validator_for(&schema)?.is_valid(&serde_json::to_value(&result)?));
    let response = internal
        .clone()
        .oneshot(agent_request("results", serde_json::to_value(&result)?)?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}
async fn browser_get(
    app: &Router,
    cookie: &str,
    path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = browser_response(app, cookie, path).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 200000).await?.to_vec(),
    )?)
}
async fn browser_response(
    app: &Router,
    cookie: &str,
    path: &str,
) -> Result<axum::response::Response, Box<dyn std::error::Error>> {
    Ok(app
        .clone()
        .oneshot(
            Request::get(path)
                .header("sec-fetch-site", "same-origin")
                .header("host", "staff.test")
                .header("cookie", cookie)
                .body(Body::empty())?,
        )
        .await?)
}
async fn poll(
    internal: &Router,
) -> Result<Vec<signer::RequestEnvelope>, Box<dyn std::error::Error>> {
    let response = internal
        .clone()
        .oneshot(agent_request(
            "channel",
            json!({"agent_id":"agent-a","owner_epoch":1,"wait_seconds":0,"limit":32}),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let queue: Vec<signer::RequestEnvelope> =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await?)?;
    Ok(queue)
}
async fn speculative_gets_leave_queue_empty(
    reads: &Router,
    internal: &Router,
    cookie: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for path in [
        "/admin/console/players?character=c-1",
        "/admin/console/audit",
        "/admin/console/access",
    ] {
        for (header, value) in [
            ("sec-purpose", "prefetch;prerender"),
            ("purpose", "prerender"),
        ] {
            let response = reads
                .clone()
                .oneshot(
                    Request::get(path)
                        .header("host", "staff.test")
                        .header("cookie", cookie)
                        .header("sec-fetch-site", "same-origin")
                        .header(header, value)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(
                poll(internal).await?.is_empty(),
                "{path} queued a speculative query"
            );
        }
    }
    Ok(())
}
async fn concurrent_identical_searches_share_pending_query(
    reads: &Router,
    internal: &Router,
    cookie: &str,
    pool: &PgPool,
    actor: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let form = browser_get(reads, cookie, "/admin/console/players").await?;
    let csrf = form
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or_else(missing)?;
    let submit = || -> Result<_, axum::http::Error> {
        Ok(reads.clone().oneshot(
            Request::post("/admin/console/players")
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&q=Knight&lang=en")))?,
        ))
    };
    let mut gate = pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(actor)
        .execute(&mut *gate)
        .await?;
    let first = tokio::spawn(submit()?);
    let second = tokio::spawn(submit()?);
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("WITH RECURSIVE blocked(pid) AS (SELECT pg_backend_pid() UNION SELECT a.pid FROM pg_stat_activity a JOIN blocked b ON b.pid=ANY(pg_blocking_pids(a.pid))) SELECT count(*)-1 FROM blocked")
                .fetch_one(&mut *gate).await?;
            if waiting >= 2 { return Ok::<_,sqlx::Error>(()); }
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *gate).await?;
            tokio::task::yield_now().await;
        }
    }).await??;
    gate.commit().await?;
    let (first, second) = (first.await?, second.await?);
    assert_eq!(first?.status(), StatusCode::SEE_OTHER);
    assert_eq!(second?.status(), StatusCode::SEE_OTHER);
    let submitted = poll(internal).await?;
    assert_eq!(
        submitted.len(),
        1,
        "identical searches must share one pending query"
    );
    assert_eq!(submitted[0].operation, "player.search");
    Ok(())
}
async fn exercise_reads(
    internal: &Router,
    channel: StoredChannel,
    pool: PgPool,
    actor: Uuid,
    cookie: &str,
    agent_key: &SigningKey,
    check_correlation: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = super::super::access::StoredMetadata::new(
        pool.clone(),
        super::super::access::Flags { console: true },
        web::PortalIdentity::configured(pool.clone()),
        control::Config { enabled: true },
    )
    .with_channel(channel.clone());
    let reads = super::super::access::router(
        pool.clone(),
        super::super::access::Flags { console: true },
        web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(actor),
        web::PortalIdentity::configured(pool.clone()),
        backend,
    );
    for path in ["/admin/console/audit", "/admin/console/access"] {
        reads
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", "staff.test")
                    .header("cookie", cookie)
                    .header("sec-fetch-site", "cross-site")
                    .body(Body::empty())?,
            )
            .await?;
        assert!(
            poll(internal).await?.is_empty(),
            "{path} queued a cross-site query"
        );
    }
    for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
        let response = reads
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/admin/console/players?q=Knight")
                    .header("host", "staff.test")
                    .header("cookie", cookie)
                    .header("sec-fetch-site", "same-origin")
                    .header("sec-purpose", "prefetch")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            poll(internal).await?.is_empty(),
            "speculative player search queued a request"
        );
    }
    speculative_gets_leave_queue_empty(&reads, internal, cookie).await?;
    concurrent_identical_searches_share_pending_query(&reads, internal, cookie, &pool, actor)
        .await?;
    let path = "/admin/console/players?lang=en&character=c-1";
    browser_get(&reads, cookie, path).await?;
    let requests = poll(internal).await?;
    assert_eq!(requests.len(), 1);
    for request in requests {
        assert_eq!(request.operation, "player.character.get");
        answer(internal,agent_key,request,json!({"character_id":"c-1","world_id":"world-a","name":"Verified Knight","level":42,"vocation":"knight","skills":[],"position":{"x":1,"y":2,"z":3},"online":true,"premium":true,"guild":null,"house":null,"bank_balance":"123","last_logins":[],"sanctions":[]})).await?;
    }
    assert!(
        browser_get(&reads, cookie, path)
            .await?
            .contains("Verified Knight")
    );
    let requests = poll(internal).await?;
    assert_eq!(requests.len(), 2);
    for request in requests {
        let value = match request.operation.as_str() {
            "player.inventory.get" => {
                json!({"items":[{"instance_id":"verified-item","type_id":42,"parent_id":null,"location":"bag","count":"2"}],"stash_balances":[{"type_id":99,"count":"9007199254740993"}],"next_cursor":null})
            }
            "sanction.list" => {
                json!({"sanctions":[{"sanction_id":"verified-sanction","account_ref":null,"character_id":"c-1","kind":"warning","active":true,"starts_at":"2026-11-15T00:00:00Z","expires_at":null}],"next_cursor":null})
            }
            _ => return Err(missing().into()),
        };
        answer(internal, agent_key, request, value).await?;
    }
    let html = browser_get(&reads, cookie, path).await?;
    assert!(html.contains("verified-item") && html.contains("verified-sanction"));
    assert!(html.contains("Stash · 99 · 9007199254740993"));
    if check_correlation {
        for (character, code, status, message) in [
            (
                "c-2",
                "ADM_PRECONDITION_FAILED",
                StatusCode::CONFLICT,
                "The character must be offline.",
            ),
            (
                "c-3",
                "ADM_INVALID_PARAMS",
                StatusCode::BAD_REQUEST,
                "The selected target or filters are incompatible",
            ),
        ] {
            let path = format!("/admin/console/players?lang=en&character={character}");
            browser_get(&reads, cookie, &path).await?;
            let requests = poll(internal).await?;
            assert_eq!(requests.len(), 1);
            let request = requests.first().ok_or_else(missing)?;
            assert_eq!(request.operation, "player.character.get");
            let mut result = signed_result(agent_key, request, Value::Null)?;
            result.status = signer::Status::Rejected;
            result.error = Some(signer::ResultError {
                code: code.into(),
                message: "agent detail".into(),
                retryable: false,
            });
            result.signature = URL_SAFE_NO_PAD.encode(
                agent_key
                    .sign(&signer::adm_x1_signing_bytes(&result)?)
                    .to_bytes(),
            );
            let response = internal
                .clone()
                .oneshot(agent_request("results", serde_json::to_value(result)?)?)
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let response = browser_response(&reads, cookie, &path).await?;
            assert_eq!(response.status(), status);
            let body = String::from_utf8(to_bytes(response.into_body(), 200000).await?.to_vec())?;
            assert!(body.contains(message));
            assert!(!body.contains("agent detail"));
        }
    }
    browser_get(&reads, cookie, "/admin/console/access?lang=en").await?;
    browser_get(&reads, cookie, "/admin/console/audit?lang=en").await?;
    let mut requests = poll(internal).await?;
    assert_eq!(requests.len(), 2);
    if check_correlation {
        for request in &requests {
            let value = match request.operation.as_str() {
                "admin.policy.get" => json!({"operations":[{
                    "operation":"legacy-policy","min_risk_class":"R9",
                    "allowed_permissions":["game.world.read"],"worlds":["world-a"]
                }]}),
                "admin.ledger.query" => json!({"entries":[{
                    "request_id":request.request_id,"admin_operation_id":request.admin_operation_id,
                    "admin_id":actor,"ledger_id":"legacy-ledger","operation":"world.list",
                    "risk_class":"R9","status":"succeeded",
                    "target":{"world_id":"world-a","node_id":null,"character_id":null,"account_ref":null},
                    "started_at":"2026-11-15T00:00:00Z","finished_at":"2026-11-15T00:00:01Z","error_code":null
                }],"next_cursor":null}),
                _ => return Err(missing().into()),
            };
            let result = signed_result(agent_key, request, value)?;
            channel
                .store
                .record_result(
                    &control::NewResult {
                        request_id: request.request_id,
                        agent_id: result.agent_id.clone(),
                        status: control::Status::Succeeded,
                        game_ledger_id: None,
                        error_code: None,
                        payload: serde_json::to_value(result)?,
                    },
                    1,
                )
                .await?;
        }
        for (path, marker) in [
            ("/admin/console/access?lang=en", "legacy-policy"),
            ("/admin/console/audit?lang=en", "legacy-ledger"),
        ] {
            let html = browser_get(&reads, cookie, path).await?;
            assert!(html.contains("Unavailable"));
            assert!(!html.contains(marker));
        }
        let replacement = poll(internal).await?;
        assert_eq!(replacement.len(), 2);
        for fresh in &replacement {
            let old = requests
                .iter()
                .find(|r| r.operation == fresh.operation)
                .ok_or_else(missing)?;
            assert_ne!(fresh.request_id, old.request_id);
        }
        requests = replacement;
    }
    let query_id = requests
        .iter()
        .find(|r| r.operation == "admin.ledger.query")
        .ok_or_else(missing)?
        .request_id;
    for request in requests {
        let value = match request.operation.as_str() {
            "admin.policy.get" => {
                json!({"operations":[{"operation":"world.list","min_risk_class":"R0","allowed_permissions":["game.world.read"],"worlds":["world-a"]}]})
            }
            "admin.ledger.query" => {
                let ledger_request_id = Uuid::now_v7();
                json!({"entries":[{"request_id":ledger_request_id,"admin_operation_id":ledger_request_id,"admin_id":actor,"ledger_id":"verified-ledger","operation":"world.list","risk_class":"R0","status":"succeeded","target":{"world_id":"world-a","node_id":null,"character_id":null,"account_ref":null},"started_at":"2026-11-15T00:00:00Z","finished_at":"2026-11-15T00:00:01Z","error_code":null}],"next_cursor":null})
            }
            _ => return Err(missing().into()),
        };
        answer(internal, agent_key, request, value).await?;
    }
    assert!(
        browser_get(&reads, cookie, "/admin/console/access?lang=en")
            .await?
            .contains("game.world.read")
    );
    assert!(
        browser_get(&reads, cookie, "/admin/console/audit?lang=en")
            .await?
            .contains("verified-ledger")
    );
    if check_correlation {
        let audited: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audit_log WHERE request_id=$1 AND action='admin.ledger.query' AND operation='admin.ledger.query' AND correlation_id=$1)").bind(query_id).fetch_one(&pool).await?;
        assert!(audited, "ledger read must reference its verified query");
    }
    browser_get(&reads, cookie, "/admin/console/approvals?lang=en").await?;
    Ok(())
}
async fn stored_console_browser_query_verified_result_and_ownership(
    check_lifecycle: bool,
    check_correlation: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("a2_console_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'console@example.test','test-only','active',now(),'en')").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,unnest(ARRAY['game.world.read','game.player.read','game.sanction.read','game.audit.read','game.ops.read','game.node.read','game.db.read']) FROM roles WHERE key='platform_admin' ON CONFLICT DO NOTHING").execute(&pool).await?;
    let agent_key = SigningKey::from_bytes(&[23; 32]);
    let validity = signer::Validity {
        not_before: 0,
        not_after: i64::MAX,
    };
    let binding = crate::control_http::AgentBinding {
        allowed_worlds: vec!["world-a".into()],
        allowed_nodes: vec![],
        agent_id: "agent-a".into(),
        identity: oteryn_internal_api::guard::Identity::san("agent-a.internal")?,
        result_key: Some((agent_key.verifying_key(), validity)),
    };
    let mut ring = signer::KeyRing::new(true);
    ring.add(signer::AdminKey::new(
        "platform-admin-signer-1".into(),
        SigningKey::from_bytes(&[17; 32]),
        validity,
    )?)?;
    let channel = StoredChannel {
        pool: pool.clone(),
        store: control::Store::new(pool.clone(), control::Config { enabled: true }),
        signer: Some((Arc::new(ring), "platform-admin-signer-1".into())),
        bindings: vec![
            binding.clone(),
            crate::control_http::AgentBinding {
                agent_id: "agent-b".into(),
                ..binding.clone()
            },
        ],
    };
    let advertised: Vec<_> = [
        "world.list",
        "world.get",
        "ops.overview.get",
        "node.list",
        "db.status",
        "db.migrations.list",
        "player.search",
        "player.character.get",
        "player.inventory.get",
        "sanction.list",
        "admin.ledger.query",
        "admin.policy.get",
    ]
    .into_iter()
    .map(|operation| json!({"operation":operation,"version":1}))
    .collect();
    let internal = crate::control_http::router(pool.clone(), true, vec![binding])?;
    let response = internal.clone().oneshot(agent_request("hello", json!({"agent_id":"agent-a","build_revision":"test-build","catalog_digest":URL_SAFE_NO_PAD.encode([1;32]),
        "operations":advertised,"worlds":["world-a"],"nodes":[],"owner_epoch":1,
        "clock":OffsetDateTime::now_utc().format(&Rfc3339)?}))?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let app = web::guarded(
        super::super::configured_routes(pool.clone(), true, channel.clone()),
        pool.clone(),
        web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(actor),
        web::PortalIdentity::configured(pool.clone()),
        Permission::Access,
    );
    let response = app
        .clone()
        .oneshot(
            Request::get("/console/worlds?agent=agent-a&lang=en")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get("set-cookie")
        .ok_or_else(missing)?
        .to_str()?
        .split(';')
        .next()
        .ok_or_else(missing)?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or_else(missing)?;
    let token_hash = Sha256::digest(b"test-mfa-grant").to_vec();
    sqlx::query("UPDATE web_sessions SET account_id=$1,data=jsonb_build_object('mfa_grant','test-mfa-grant')").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '1 hour')")
        .bind(token_hash).bind(actor).bind(Uuid::now_v7()).bind(vec![0_u8;32]).execute(&pool).await?;
    crate::test_staff::browser(&pool, actor, &cookie).await?;
    let response = crate::test_staff::race_account_before_fence(
        &pool,
        actor,
        app.clone().oneshot(
            Request::post("/console/worlds")
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&agent=agent-a&operation=world.list&lang=en"
                )))?,
        ),
    )
    .await
    .map_err(|e| e as Box<dyn std::error::Error>)??;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .ok_or_else(missing)?
        .to_str()?
        .to_owned();
    let response = internal
        .clone()
        .oneshot(agent_request(
            "channel",
            json!({"agent_id":"agent-a","owner_epoch":1,"wait_seconds":0,"limit":32}),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let queued: Value = serde_json::from_slice(&to_bytes(response.into_body(), 100000).await?)?;
    let request: signer::RequestEnvelope = serde_json::from_value(queued[0].clone())?;
    channel
        .signer
        .as_ref()
        .ok_or_else(missing)?
        .0
        .verify(&request, OffsetDateTime::now_utc().unix_timestamp())?;
    let mut result = signer::ResultEnvelope {
        request_id: request.request_id,
        admin_operation_id: request.admin_operation_id,
        status: signer::Status::Succeeded,
        error: None,
        result: Json(
            json!({"worlds":[{"world_id":"world-a","name":"World A","state":"online","map_revision":"42","content_revision":"1","ruleset_revision":"1","policy_revision":"1","channels":[]}],"next_cursor":null}),
        ),
        dry_run_preview: None,
        game_ledger_id: None,
        audit_event_ids: vec![],
        effective_revision: Some("revision-1".into()),
        progress: None,
        completed_at: Some(OffsetDateTime::now_utc().format(&Rfc3339)?),
        agent_id: "agent-a".into(),
        agent_build: "test-build".into(),
        signature: String::new(),
    };
    result.signature = URL_SAFE_NO_PAD.encode(
        agent_key
            .sign(&signer::adm_x1_signing_bytes(&result)?)
            .to_bytes(),
    );
    let response = internal
        .clone()
        .oneshot(agent_request("results", serde_json::to_value(&result)?)?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(
            Request::get(&location)
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    assert!(html.contains("World A") && html.contains("42") && html.contains("revision-1"));
    assert!(
        channel
            .read(
                Uuid::now_v7(),
                "agent-a",
                Operation::Worlds,
                None,
                Some(request.request_id)
            )
            .await?
            .result
            .is_none()
    );
    exercise_reads(
        &internal,
        channel.clone(),
        pool.clone(),
        actor,
        &cookie,
        &agent_key,
        check_correlation,
    )
    .await?;
    if check_correlation {
        let detail = app
            .clone()
            .oneshot(
                Request::post("/console/worlds")
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "_csrf={csrf}&agent=agent-a&operation=world.get&target=world-a&lang=en"
                    )))?,
            )
            .await?;
        assert_eq!(detail.status(), StatusCode::SEE_OTHER);
        let path = detail.headers()["location"].to_str()?.to_owned();
        let response = internal
            .clone()
            .oneshot(agent_request(
                "channel",
                json!({
                    "agent_id":"agent-a","owner_epoch":1,"wait_seconds":0,"limit":32
                }),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let detail = serde_json::from_slice::<Vec<signer::RequestEnvelope>>(
            &to_bytes(response.into_body(), 100_000).await?,
        )?
        .into_iter()
        .find(|request| request.operation == "world.get")
        .ok_or_else(missing)?;
        let result = signed_result(
            &agent_key,
            &detail,
            json!({
                "world_id":"world-b","name":"Wrong world", "state":"ready","channels":[],
                "ruleset_revision":"1", "content_revision":"1","map_revision":"1","policy_revision":"1"
            }),
        )?;
        // Simulate a correctly signed receipt persisted before target matching.
        channel
            .store
            .record_result(
                &control::NewResult {
                    request_id: detail.request_id,
                    agent_id: result.agent_id.clone(),
                    status: control::Status::Succeeded,
                    game_ledger_id: None,
                    error_code: None,
                    payload: serde_json::to_value(result)?,
                },
                1,
            )
            .await?;
        let html = browser_get(&app, &cookie, &path).await?;
        assert!(!html.contains("Wrong world"));
        assert!(html.contains("Unavailable"));
        assert!(html.contains("Request fresh data"));
    }
    if check_lifecycle {
        let world_target = serde_json::to_value(&request.target)?;
        let mut scoped = channel.clone();
        scoped.bindings[0].allowed_worlds.clear();
        assert!(matches!(
            scoped
                .read(
                    actor,
                    "agent-a",
                    Operation::Worlds,
                    None,
                    Some(request.request_id)
                )
                .await,
            Err(Error::Unavailable)
        ));
        for (operation, page) in [
            (Operation::Overview, "dashboard"),
            (Operation::Worlds, "worlds"),
            (Operation::Nodes, "nodes"),
            (Operation::Database, "database"),
        ] {
            let target = if matches!(operation, Operation::Database) {
                serde_json::to_value(control::Target::default())?
            } else {
                world_target.clone()
            };
            sqlx::query("UPDATE admin_control_requests SET operation=$1, target=$3, params=$4 WHERE request_id=$2")
                .bind(operation.name())
                .bind(request.request_id)
                .bind(target)
                .bind(StoredChannel::ops_params(operation))
                .execute(&pool)
                .await?;
            for (status, label) in [
                ("queued", "Awaiting a stored result"),
                ("delivered", "Awaiting a stored result"),
                ("accepted", "Awaiting a stored result"),
                ("in_progress", "In progress"),
                ("failed", "Query failed"),
                ("rejected", "Query rejected"),
                ("cancelled", "Query cancelled"),
                ("expired", "Query expired"),
            ] {
                sqlx::query("UPDATE admin_control_requests SET status=$1,issued_at=clock_timestamp()-interval '120 seconds',expires_at=CASE WHEN $1='expired' THEN clock_timestamp()-interval '1 second' ELSE clock_timestamp()+interval '60 seconds' END WHERE request_id=$2")
            .bind(status).bind(request.request_id).execute(&pool).await?;
                let path = format!(
                    "/console/{page}?agent=agent-a&lang=en&request={}",
                    request.request_id
                );
                let html = browser_get(&app, &cookie, &path).await?;
                assert!(html.contains(label), "{status} must display {label}");
                let expected = match status {
                    "queued" | "delivered" | "accepted" => "pending",
                    "in_progress" => "in-progress",
                    other => other,
                };
                let view = channel
                    .read(actor, "agent-a", operation, None, Some(request.request_id))
                    .await?;
                assert_eq!(view.status.key(), expected);
                let foreign = channel
                    .read(
                        Uuid::now_v7(),
                        "agent-a",
                        operation,
                        None,
                        Some(request.request_id),
                    )
                    .await?;
                assert!(matches!(foreign.status, QueryStatus::Pending));
            }
            for status in ["queued", "delivered", "accepted", "in_progress"] {
                sqlx::query("UPDATE admin_control_requests SET status=$1,expires_at=clock_timestamp()-interval '1 second' WHERE request_id=$2").bind(status).bind(request.request_id).execute(&pool).await?;
                let path = format!(
                    "/console/{page}?agent=agent-a&lang=en&request={}",
                    request.request_id
                );
                assert!(
                    browser_get(&app, &cookie, &path)
                        .await?
                        .contains("Query expired")
                );
            }
        }
        sqlx::query("UPDATE admin_control_requests SET operation='world.list',status='succeeded',expires_at=clock_timestamp()+interval '60 seconds',target=$2,params=$3 WHERE request_id=$1").bind(request.request_id).bind(&world_target).bind(StoredChannel::ops_params(Operation::Worlds)).execute(&pool).await?;
    }
    let response = app
        .clone()
        .oneshot(
            Request::post("/console/worlds")
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&agent=agent-a&operation=world.list&lang=en"
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    sqlx::query("UPDATE admin_control_agents SET owner_epoch=2 WHERE agent_id='agent-a'")
        .execute(&pool)
        .await?;
    assert!(
        channel
            .read(
                actor,
                "agent-a",
                Operation::Worlds,
                None,
                Some(request.request_id)
            )
            .await?
            .result
            .is_none()
    );
    sqlx::query("INSERT INTO admin_control_agents(agent_id, build, catalog_digest, advertised_operations, served_worlds, served_nodes, owner_epoch) SELECT 'agent-b',build,catalog_digest,advertised_operations,served_worlds,served_nodes,3 FROM admin_control_agents WHERE agent_id='agent-a'").execute(&pool).await?;
    sqlx::query("UPDATE admin_control_requests SET status='expired' WHERE request_id=$1")
        .bind(request.request_id)
        .execute(&pool)
        .await?;
    let moved = channel
        .read(
            actor,
            "agent-b",
            Operation::Worlds,
            None,
            Some(request.request_id),
        )
        .await?;
    assert_eq!(moved.status.key(), "expired");
    let claimed = channel.store.claim_pending("agent-b", 3, 32).await?;
    let handoff = claimed
        .iter()
        .find(|r| r.operation == "world.list")
        .ok_or_else(missing)?;
    result.request_id = handoff.request_id;
    result.admin_operation_id = handoff.admin_operation_id;
    result.agent_id = "agent-b".into();
    result.completed_at = Some(OffsetDateTime::now_utc().format(&Rfc3339)?);
    result.signature = URL_SAFE_NO_PAD.encode(
        agent_key
            .sign(&signer::adm_x1_signing_bytes(&result)?)
            .to_bytes(),
    );
    channel
        .store
        .record_result(
            &control::NewResult {
                request_id: handoff.request_id,
                agent_id: result.agent_id.clone(),
                status: control::Status::Succeeded,
                game_ledger_id: None,
                error_code: None,
                payload: serde_json::to_value(result)?,
            },
            3,
        )
        .await?;
    let moved = channel
        .read(
            actor,
            "agent-b",
            Operation::Worlds,
            None,
            Some(handoff.request_id),
        )
        .await?;
    assert_eq!(
        moved.result.ok_or_else(missing)?.request_id,
        handoff.request_id
    );
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    Ok(())
}

#[tokio::test]
async fn a2_stored_console_browser_query_verified_result_and_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let result = stored_console_browser_query_verified_result_and_ownership(false, false).await;
    eprintln!(
        "a2_stored_console_browser_query_verified_result_and_ownership duration: {:?}",
        started.elapsed()
    );
    result
}

#[tokio::test]
async fn a3_configured_console_displays_every_lifecycle_outcome()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let result = stored_console_browser_query_verified_result_and_ownership(true, false).await;
    eprintln!(
        "a3_configured_console_displays_every_lifecycle_outcome duration: {:?}",
        started.elapsed()
    );
    result
}

#[tokio::test]
async fn a3_stored_ledger_read_audit_references_source_query()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let result = stored_console_browser_query_verified_result_and_ownership(false, true).await;
    eprintln!(
        "a3_stored_ledger_read_audit_references_source_query duration: {:?}",
        started.elapsed()
    );
    result
}

#[tokio::test]
async fn game_staff_access_respects_console_and_platform_permissions()
-> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("game_staff_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search_path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;

    let flags = access::Flags { console: true };
    let control = control::Config { enabled: true };
    for (role, expected_console, expected_platform) in [
        ("game_master_senior", StatusCode::OK, StatusCode::FORBIDDEN),
        ("platform_admin", StatusCode::OK, StatusCode::OK),
    ] {
        let actor: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES($1,'test-only','active',now(),'en') RETURNING id")
            .bind(format!("{role}@example.test"))
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            sqlx::query(
                "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2"
            )
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?
            .rows_affected(),
            1
        );
        let panel = web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        };
        let accounts = web::PortalIdentity::configured(pool.clone());
        let channel = StoredChannel {
            pool: pool.clone(),
            store: control::Store::new(pool.clone(), control),
            signer: None,
            bindings: vec![],
        };
        let backend = access::StoredMetadata::new(pool.clone(), flags, accounts.clone(), control)
            .with_channel(channel.clone());
        let app = web::guarded(
            super::super::configured_routes(pool.clone(), true, channel),
            pool.clone(),
            panel.clone(),
            Browser(actor),
            accounts.clone(),
            Permission::Access,
        )
        .merge(access::router(
            pool.clone(),
            flags,
            panel.clone(),
            Browser(actor),
            accounts.clone(),
            backend,
        ))
        .merge(web::router(pool.clone(), panel, Browser(actor), accounts));

        for (path, expected) in [
            ("/console/dashboard", expected_console),
            ("/admin/console/players", expected_console),
            ("/admin/console/audit", expected_console),
            ("/admin/roles", expected_platform),
            ("/admin/console/access", expected_platform),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(path)
                        .header("host", "staff.test")
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), expected, "{role} GET {path}");
        }
    }
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
