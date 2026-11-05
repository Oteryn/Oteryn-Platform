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
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
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
    async fn close(self) -> TestResult {
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
struct Auth;
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
struct Verifier;
impl TotpVerifier for Verifier {
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
        *s.captured.lock().map_err(|_| Refusal::Unavailable)? = Some(session);
        Ok::<_, Refusal>(())
    }
    .await
    .map_or(StatusCode::INTERNAL_SERVER_ERROR, |()| StatusCode::OK)
}
async fn session(
    pool: &PgPool,
    age: i64,
) -> Result<(BrowserSession, String), Box<dyn std::error::Error>> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let captured = Arc::new(Mutex::new(None));
    let app = Router::new()
        .route("/seed", get(seed))
        .with_state(Seed {
            id: Uuid::now_v7(),
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
fn reason() -> Reason {
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
            Request::post("/admin/control/step-up")
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
            Request::post("/admin/control/step-up")
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
            Request::post("/admin/control/step-up")
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
    let id = s
        .record_preview(
            &requester,
            &intent(),
            &"b".repeat(64),
            json!({"effect":"<script>test</script>","reference":"x".repeat(500)}),
            reason(),
        )
        .await?;
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
    let i = intent();
    let d = "b".repeat(64);
    let evidence = s
        .control_evidence(
            &requester,
            &Submission {
                intent: &i,
                reason: Some(reason()),
                preview: Some(id),
                params_preview_digest: Some(&d),
                dry_run: false,
            },
        )
        .await?;
    assert_eq!(evidence.approvals.len(), 1);
    db.close().await
}
