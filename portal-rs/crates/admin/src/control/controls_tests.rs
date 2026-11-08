use super::*;
use axum::{
    Extension, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware,
    routing::get,
};
use oteryn_web::session::{PrivilegeChange, SessionLayer};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) struct Database {
    pub(super) pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    pub(super) async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED control PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("control_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    pub(super) async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
#[derive(Clone)]
pub(super) struct Auth;
impl Authority for Auth {
    async fn staff(&self, s: &BrowserSession) -> Result<Uuid, Refusal> {
        s.account_id().await.ok_or(Refusal::Denied)
    }
    async fn authorize(
        &self,
        s: &BrowserSession,
        name: &str,
    ) -> Result<(Identity, Operation), Refusal> {
        let id = self.staff(s).await?;
        let risk = match name {
            "world.configure" => RiskClass::R3,
            "world.retire" => RiskClass::R4,
            _ => return Err(Refusal::Denied),
        };
        Ok((
            Identity {
                admin_id: id,
                roles: vec!["game_admin".into()],
                mfa_at: 1,
                ip_hash: "test-hash".into(),
                session_ref: "test-session".into(),
            },
            Operation {
                name: name.into(),
                permission: "game.world.admin".into(),
                risk,
            },
        ))
    }
    async fn approver(&self, _: Uuid, _: &Operation) -> Result<(), Refusal> {
        Ok(())
    }
}
#[derive(Clone)]
pub(super) struct Verifier;
impl TotpVerifier for Verifier {
    async fn grant_valid_in_transaction(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
        _: &str,
    ) -> Result<bool, Refusal> {
        Ok(true)
    }

    async fn grant_valid(&self, _: Uuid, _: &str) -> Result<bool, Refusal> {
        Ok(true)
    }
    async fn verify_totp(
        &self,
        _: oteryn_identity::AccountId,
        code: &str,
    ) -> Result<bool, Refusal> {
        Ok(code == "123456")
    }
}
#[derive(Clone)]
struct Seed {
    id: Uuid,
    at: i64,
    captured: Arc<Mutex<Option<BrowserSession>>>,
}
async fn seed(State(s): State<Seed>, Extension(session): Extension<BrowserSession>) -> StatusCode {
    async {
        oteryn_web::session::rotate(&session, PrivilegeChange::Login(s.id)).await?;
        session
            .set(
                STEP_UP,
                &StepUp {
                    account: s.id,
                    step_up_at: s.at,
                },
            )
            .await?;
        session
            .set(
                SessionKey::<String>::new(oteryn_identity::mfa::SESSION_GRANT_KEY),
                &"test-grant".to_owned(),
            )
            .await?;
        *s.captured.lock().map_err(|_| Refusal::Unavailable)? = Some(session);
        Ok::<_, Refusal>(())
    }
    .await
    .map_or(StatusCode::INTERNAL_SERVER_ERROR, |()| StatusCode::OK)
}
pub(super) async fn session(
    pool: &PgPool,
    age: i64,
) -> Result<(BrowserSession, String), Box<dyn std::error::Error>> {
    session_for_account(pool, age, Uuid::now_v7()).await
}
pub(super) async fn session_for_account(
    pool: &PgPool,
    age: i64,
    id: Uuid,
) -> Result<(BrowserSession, String), Box<dyn std::error::Error>> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let captured = Arc::new(Mutex::new(None));
    let app = Router::new()
        .route("/seed", get(seed))
        .with_state(Seed {
            id,
            at: now - age,
            captured: captured.clone(),
        })
        .layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ));
    let response = app
        .oneshot(Request::get("/seed").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    let session = captured
        .lock()
        .map_err(|_| "poisoned")?
        .take()
        .ok_or("missing session")?;
    Ok((session, cookie))
}
pub(super) fn reason() -> Reason {
    Reason {
        code: "MAINTENANCE".into(),
        text: "Retire unused test world".into(),
        ticket_ref: Some("T-42".into()),
    }
}
fn intent() -> Intent {
    Intent {
        operation: "world.retire".into(),
        digest: "a".repeat(64),
    }
}
fn controls(db: &Database) -> Controls<Auth, Verifier> {
    Controls::new(db.pool.clone(), Auth, Verifier, true)
}
async fn preview(
    s: &Controls<Auth, Verifier>,
    requester: &BrowserSession,
) -> Result<Uuid, Refusal> {
    s.record_preview(
        requester,
        &intent(),
        &"b".repeat(64),
        json!({"effect":"archive test world"}),
        reason(),
    )
    .await
}
#[tokio::test]
async fn pg_step_up_older_than_five_minutes_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, _) = session(&db.pool, 301).await?;
    let i = Intent {
        operation: "world.configure".into(),
        digest: "a".repeat(64),
    };
    assert_eq!(
        controls(&db)
            .evidence(&session, &i, Some(reason()), None, None, false)
            .await
            .err(),
        Some(Refusal::StepUpRequired)
    );
    assert_eq!(
        controls(&db)
            .evidence(&session, &intent(), Some(reason()), None, None, false)
            .await
            .err(),
        Some(Refusal::StepUpRequired)
    );
    db.close().await
}
#[tokio::test]
async fn pg_self_approval_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, _) = session(&db.pool, 0).await?;
    let s = controls(&db);
    let id = preview(&s, &session).await?;
    assert_eq!(
        s.approve(&session, id).await.err(),
        Some(Refusal::SelfApproval)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_control_approvals")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[tokio::test]
async fn pg_stale_approval_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, _) = session(&db.pool, 0).await?;
    let s = controls(&db);
    let id = preview(&s, &session).await?;
    let at = s.now().await? - 1801;
    sqlx::query("INSERT INTO admin_control_approvals(id,preview_id,admin_id,step_up_at,approved_at) VALUES($1,$2,$3,$4,$4)").bind(Uuid::now_v7()).bind(id).bind(Uuid::now_v7()).bind(at).execute(&db.pool).await?;
    assert_eq!(
        s.evidence(
            &session,
            &intent(),
            Some(reason()),
            Some(id),
            Some(&"b".repeat(64)),
            false
        )
        .await
        .err(),
        Some(Refusal::StaleApproval)
    );
    db.close().await
}
#[tokio::test]
async fn pg_digest_mismatch_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (requester, _) = session(&db.pool, 0).await?;
    let (approver, _) = session(&db.pool, 0).await?;
    let s = controls(&db);
    let id = preview(&s, &requester).await?;
    s.approve(&approver, id).await?;
    assert_eq!(
        s.evidence(
            &requester,
            &intent(),
            Some(reason()),
            Some(id),
            Some(&"c".repeat(64)),
            false
        )
        .await
        .err(),
        Some(Refusal::DigestMismatch)
    );
    let mut changed = intent();
    changed.digest = "c".repeat(64);
    assert_eq!(
        s.evidence(
            &requester,
            &changed,
            Some(reason()),
            Some(id),
            Some(&"b".repeat(64)),
            false
        )
        .await
        .err(),
        Some(Refusal::DigestMismatch)
    );
    db.close().await
}
#[tokio::test]
async fn pg_happy_r3() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, _) = session(&db.pool, 301).await?;
    let s = controls(&db);
    assert_eq!(
        s.step_up(&session, "000000").await.err(),
        Some(Refusal::Denied)
    );
    s.step_up(&session, "123456").await?;
    let i = Intent {
        operation: "world.configure".into(),
        digest: "a".repeat(64),
    };
    let evidence = s
        .evidence(&session, &i, Some(reason()), None, None, false)
        .await?;
    assert!(evidence.actor.step_up_at.is_some());
    assert!(evidence.approvals.is_empty());
    db.close().await
}
#[tokio::test]
async fn pg_happy_r4() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (requester, _) = session(&db.pool, 0).await?;
    let (approver, _) = session(&db.pool, 0).await?;
    let s = controls(&db);
    let id = preview(&s, &requester).await?;
    let approval = s.approve(&approver, id).await?;
    let e = s
        .evidence(
            &requester,
            &intent(),
            Some(reason()),
            Some(id),
            Some(&"b".repeat(64)),
            false,
        )
        .await?;
    assert_eq!(e.approvals.len(), 1);
    assert_eq!(e.approvals[0].approval_id, approval);
    assert_ne!(e.actor.admin_id, e.approvals[0].admin_id);
    let wire = serde_json::to_value(e)?;
    assert!(
        wire["actor"]["step_up_at"]
            .as_str()
            .is_some_and(|v| v.ends_with('Z'))
    );
    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action LIKE 'admin.control.%'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audits, 2);
    assert!(
        sqlx::query("UPDATE admin_control_approvals SET approved_at=1")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await
}
#[tokio::test]
async fn pg_step_up_bad_input_and_account_rate_limit() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, _) = session(&db.pool, 400).await?;
    let s = controls(&db);
    for _ in 0..5 {
        assert_eq!(
            s.step_up(&session, "invalid").await.err(),
            Some(Refusal::InvalidInput)
        );
    }
    assert_eq!(
        s.step_up(&session, "123456").await.err(),
        Some(Refusal::RateLimited)
    );
    let now = s.now().await?;
    assert!(
        session
            .get(STEP_UP)
            .await?
            .is_some_and(|v| !recent(v.step_up_at, now, 300))
    );
    db.close().await
}
#[derive(Clone)]
struct Staff;
impl crate::web::StaffSession for Staff {
    async fn resolve(&self, s: &BrowserSession) -> Option<crate::web::StaffIdentity> {
        Some(crate::web::StaffIdentity {
            account: s.account_id().await?,
            mfa_satisfied: true,
        })
    }
}
impl rbac::AccountStateReader for Staff {
    async fn state(&self, _: Uuid) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        Ok(rbac::AccountState::Active)
    }
}
#[tokio::test]
async fn pg_forms_csrf_locales_and_step_up_redirect() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (session, cookie) = session(&db.pool, 400).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(session.account_id().await).execute(&db.pool).await?;
    let app = http::router(
        controls(&db),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "admin.test".into(),
        },
        Staff,
        Staff,
    );
    for language in ["en", "pl"] {
        let response = app
            .clone()
            .oneshot(
                Request::get("/admin/control/step-up")
                    .header("host", "admin.test")
                    .header("cookie", &cookie)
                    .header("accept-language", language)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 100000)
                .await?
                .to_vec(),
        )?;
        assert!(html.contains("name=\"_csrf\""));
        assert!(html.contains("label for=\"code\""));
        if let Ok(directory) = std::env::var("PORTAL_CONTROL_HTML_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("step-{language}.html")),
                &html,
            )?;
        }

        assert!(html.contains(&format!("lang=\"{language}\"")));
    }
    let response = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/admin/control/step-up?request_id={}",
                Uuid::now_v7()
            ))
            .header("host", "admin.test")
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("code=123456"))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let csrf = session.csrf_token().await?;
    let malformed = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/admin/control/step-up?request_id={}",
                Uuid::now_v7()
            ))
            .header("host", "admin.test")
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("accept-language", "pl")
            .body(Body::from(format!("_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = String::from_utf8(
        axum::body::to_bytes(malformed.into_body(), 100000)
            .await?
            .to_vec(),
    )?;
    assert!(body.contains("Nieprawidłowe dane"));
    let response = app
        .oneshot(
            Request::post(format!(
                "/admin/control/step-up?request_id={}",
                Uuid::now_v7()
            ))
            .header("host", "admin.test")
            .header("cookie", &cookie)
            .header("origin", "https://admin.test")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("code=123456&_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let redirect = http::refusal_response(&axum::http::HeaderMap::new(), Refusal::StepUpRequired);
    assert_eq!(redirect.status(), StatusCode::SEE_OTHER);
    assert_eq!(redirect.headers()["location"], "/admin/control/step-up");
    db.close().await
}
#[test]
fn policy_bad_input_and_boundaries() {
    let actor = Identity {
        admin_id: Uuid::now_v7(),
        roles: vec!["game_admin".into()],
        mfa_at: 900,
        ip_hash: "hash".into(),
        session_ref: "session".into(),
    };
    let op = Operation {
        name: "world.configure".into(),
        permission: "game.world.config".into(),
        risk: RiskClass::R3,
    };
    let step = Some(StepUp {
        account: actor.admin_id,
        step_up_at: 1000,
    });
    assert!(
        evaluate(
            &op,
            &actor,
            step,
            Some(reason()),
            &"a".repeat(64),
            None,
            None,
            false,
            1299
        )
        .is_ok()
    );
    assert_eq!(
        evaluate(
            &op,
            &actor,
            step,
            Some(reason()),
            &"a".repeat(64),
            None,
            None,
            false,
            1300
        )
        .err(),
        Some(Refusal::StepUpRequired)
    );
    assert_eq!(
        evaluate(
            &op,
            &actor,
            step,
            None,
            &"a".repeat(64),
            None,
            None,
            false,
            1000
        )
        .err(),
        Some(Refusal::InvalidInput)
    );
    assert_eq!(
        evaluate(
            &op,
            &actor,
            step,
            Some(reason()),
            "bad",
            None,
            None,
            false,
            1000
        )
        .err(),
        Some(Refusal::InvalidInput)
    );
    assert_eq!(
        evaluate(
            &op,
            &actor,
            step,
            Some(reason()),
            &"a".repeat(64),
            None,
            None,
            false,
            999
        )
        .err(),
        Some(Refusal::StepUpRequired)
    );
}

#[tokio::test]
async fn pg_approval_form_review_csrf_and_submit() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (requester, _) = session(&db.pool, 0).await?;
    let (approver, cookie) = session(&db.pool, 0).await?;
    let s = controls(&db);
    let i = Intent {
        operation: "world.retire".into(),
        digest: {
            use sha2::{Digest, Sha256};
            Sha256::digest(crate::control::canonical::encode_wire(&json!({"operation":"world.retire","operation_version":1,"kind":"command","target":{"world_id":"test","node_id":null,"character_id":null,"account_ref":null},"params":{}}))?).iter().map(|v|format!("{v:02x}")).collect()
        },
    };
    let id = s
        .record_preview(
            &requester,
            &i,
            &"b".repeat(64),
            json!({"effect":"<script>test</script>","reference":"x".repeat(500)}),
            reason(),
        )
        .await?;
    stored_preview_fixture(&db.pool, id, requester.account_id().await.ok_or("account")?).await?;
    super::integration::tests::bound(&approver, id, 0).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(approver.account_id().await).execute(&db.pool).await?;
    let app = http::router(
        controls(&db),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "admin.test".into(),
        },
        Staff,
        Staff,
    );
    let path = format!("/admin/control/approvals/{id}");
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(&path)
                    .header("host", "admin.test")
                    .header("cookie", &cookie)
                    .header("accept-language", lang)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 100000)
                .await?
                .to_vec(),
        )?;
        assert!(html.contains("name=\"_csrf\""));
        assert!(!html.contains("<script>"));
        assert!(html.contains("world.retire"));
        if let Ok(directory) = std::env::var("PORTAL_CONTROL_HTML_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("approval-{lang}.html")),
                html,
            )?;
        }
    }
    let response = app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("host", "admin.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(s.approval(id).await.err(), Some(Refusal::ApprovalRequired));
    let csrf = approver.csrf_token().await?;
    let response = app
        .oneshot(
            Request::post(&path)
                .header("host", "admin.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let d = "b".repeat(64);
    let final_id = Uuid::now_v7();
    s.step_up_for_request(&requester, final_id, "123456")
        .await?;
    let expected = s
        .evidence(&requester, &i, Some(reason()), Some(id), Some(&d), false)
        .await?;
    let mut final_request = super::integration::tests::input(
        final_id,
        requester.account_id().await.ok_or("requester")?,
        true,
    );
    final_request.dry_run = false;
    final_request.params = json!({"preview_digest":d});
    final_request.params_digest = crate::control::signer::params_digest(
        &crate::control::canonical::Json(final_request.params.clone()),
    )?
    .to_vec();
    final_request.approvals = vec![crate::control::Approval {
        approval_id: expected.approvals[0].approval_id,
        approver: expected.approvals[0].admin_id,
        approved_at: time::OffsetDateTime::parse(
            &expected.approvals[0].approved_at,
            &time::format_description::well_known::Rfc3339,
        )?,
        preview_digest: vec![0xbb; 32],
        decision: crate::control::Decision::Approved,
    }];
    final_request.canonical_envelope = super::integration::tests::wire(&final_request, &expected)?;
    crate::control::Store::new(db.pool.clone(), crate::control::Config { enabled: true })
        .enqueue(&final_request)
        .await?;
    let evidence = s
        .control_evidence(
            &requester,
            &Submission {
                request_id: final_id,
                preview: Some(id),
            },
        )
        .await?;
    assert_eq!(evidence.approvals.len(), 1);
    db.close().await
}

async fn stored_preview_fixture(pool: &PgPool, id: Uuid, actor: Uuid) -> TestResult {
    sqlx::query("INSERT INTO admin_control_requests(request_id,admin_operation_id,operation,operation_version,kind,target,params,params_digest,dry_run,risk_class,actor,reason,approvals,issued_at,expires_at,nonce,signer_key_id,canonical_envelope,signature,status) VALUES($1,$2,'world.retire',1,'command','{\"world_id\":\"test\"}','{}',$3,true,'R4',$4,$5,'[]',now()-interval '1 second',now()+interval '5 minutes',$6,'test',$7,$8,'succeeded')")
        .bind(id).bind(Uuid::now_v7()).bind(vec![1u8;32]).bind(actor).bind(json!(reason())).bind(vec![1u8;16]).bind(b"fixture".to_vec()).bind(vec![1u8;64]).execute(pool).await?;
    Ok(())
}

// Preserve the original ADM-P2B policy regressions while the production boundary
// accepts only durable requests. These fixture helpers are never compiled into
// the admin service.
impl<A: Authority, T: TotpVerifier> Controls<A, T> {
    /// Called only by the verified agent-result ingestion path after matching a
    /// successful R4 dry run to the requester and canonical intent. Never mount
    /// this method as a browser endpoint or accept receipt data from a form.
    async fn record_preview(
        &self,
        session: &BrowserSession,
        intent: &Intent,
        preview_digest: &str,
        preview: Value,
        reason: Reason,
    ) -> Result<Uuid, Refusal> {
        self.enabled()?;
        let (identity, op) = self.authorized(session, &intent.operation).await?;
        if op.name != intent.operation
            || op.risk != RiskClass::R4
            || !digest(preview_digest)
            || preview.is_null()
            || preview.to_string().len() > 65536
        {
            return Err(Refusal::InvalidInput);
        }
        evaluate(
            &op,
            &identity,
            session.get(STEP_UP).await?,
            Some(reason.clone()),
            &intent.digest,
            None,
            None,
            true,
            self.now().await?,
        )?;
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO admin_control_previews(id,requester,operation,permission,intent_digest,preview_digest,preview,reason) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(id).bind(identity.admin_id).bind(&op.name).bind(&op.permission).bind(&intent.digest).bind(preview_digest).bind(preview).bind(json!(reason)).execute(&mut *tx).await?;
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(identity.admin_id),
                    request_id: id,
                },
                action: "admin.control.preview.stored",
                target: &id.to_string(),
                before: Value::Null,
                after: json!({"preview_digest":preview_digest}),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
    async fn approve(&self, session: &BrowserSession, preview: Uuid) -> Result<Uuid, Refusal> {
        self.enabled()?;
        let p = self.preview(preview).await?;
        let (identity, op) = self.authorized(session, &p.operation).await?;
        if op.name != p.operation || op.permission != p.permission || op.risk != RiskClass::R4 {
            return Err(Refusal::Denied);
        }
        if identity.admin_id == p.requester {
            return Err(Refusal::SelfApproval);
        }
        let now = self.now().await?;
        let step = session
            .get(STEP_UP)
            .await?
            .filter(|s| s.account == identity.admin_id && recent(s.step_up_at, now, 300))
            .ok_or(Refusal::StepUpRequired)?;
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query("INSERT INTO admin_control_approvals(id,preview_id,admin_id,step_up_at,approved_at) VALUES($1,$2,$3,$4,$5) ON CONFLICT(preview_id) DO NOTHING")
            .bind(id).bind(preview).bind(identity.admin_id).bind(step.step_up_at).bind(now).execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(Refusal::Denied);
        }
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(identity.admin_id),
                    request_id: id,
                },
                action: "admin.control.approved",
                target: &preview.to_string(),
                before: Value::Null,
                after: json!({"approval_id":id,"preview_digest":p.preview_digest}),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
    /// Request builder calls immediately before signing/enqueue, using params'
    /// actual preview_digest and the digest of the actual canonical intent.
    #[allow(clippy::too_many_arguments)]
    async fn evidence(
        &self,
        session: &BrowserSession,
        intent: &Intent,
        reason: Option<Reason>,
        preview: Option<Uuid>,
        params_preview_digest: Option<&str>,
        dry_run: bool,
    ) -> Result<Evidence, Refusal> {
        self.enabled()?;
        let (identity, op) = self.authorized(session, &intent.operation).await?;
        if op.name != intent.operation {
            return Err(Refusal::Denied);
        }
        // Check requester controls first, so expired elevation always leads to
        // step-up before approval/preview processing on a submission.
        let step_up = session.get(STEP_UP).await?;
        evaluate(
            &op,
            &identity,
            step_up,
            reason.clone(),
            &intent.digest,
            None,
            None,
            true,
            self.now().await?,
        )?;
        let approval = if op.risk == RiskClass::R4 && !dry_run {
            let p = self
                .preview(preview.ok_or(Refusal::DryRunRequired)?)
                .await?;
            if p.requester != identity.admin_id
                || p.operation != op.name
                || p.permission != op.permission
                || Some(&p.reason) != reason.as_ref()
            {
                return Err(Refusal::Denied);
            }
            let a = self.approval(p.id).await?;
            self.authority.approver(a.admin_id, &op).await?;
            Some(a)
        } else {
            None
        };
        evaluate(
            &op,
            &identity,
            step_up,
            reason,
            &intent.digest,
            params_preview_digest,
            approval.as_ref(),
            dry_run,
            self.now().await?,
        )
    }
    async fn approval(&self, preview: Uuid) -> Result<ApprovalState, Refusal> {
        let r = sqlx::query("SELECT a.id,a.admin_id,a.approved_at,a.step_up_at,p.requester,p.preview_digest,p.intent_digest FROM admin_control_approvals a JOIN admin_control_previews p ON p.id=a.preview_id WHERE p.id=$1")
            .bind(preview).fetch_optional(&self.pool).await?.ok_or(Refusal::ApprovalRequired)?;
        Ok(ApprovalState {
            approval_id: r.try_get("id")?,
            admin_id: r.try_get("admin_id")?,
            approved_at: r.try_get("approved_at")?,
            step_up_at: r.try_get("step_up_at")?,
            requester: r.try_get("requester")?,
            preview_digest: r.try_get("preview_digest")?,
            intent_digest: r.try_get("intent_digest")?,
        })
    }
}

#[tokio::test]
async fn refusal_documents_have_a_localized_nonempty_title() -> TestResult {
    for (lang, expected) in [
        ("pl", "Odmowa wykonania operacji"),
        ("en", "This action is refused"),
    ] {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(axum::http::header::ACCEPT_LANGUAGE, lang.parse()?);
        let response = http::refusal_response(&headers, Refusal::Denied);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), 65536).await?;
        let html = String::from_utf8(body.to_vec())?;
        assert!(html.contains(&format!("<title>{expected}</title>")));
        assert!(html.contains(&format!("<h1>{expected}</h1>")));
    }
    Ok(())
}
