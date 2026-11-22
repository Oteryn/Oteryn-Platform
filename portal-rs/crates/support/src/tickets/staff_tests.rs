use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
type TestResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[tokio::test]
async fn postgres_configured_staff_support_domain_actions() -> TestResult {
    configured_staff_fixture("postgres_configured_staff_support_domain_actions", None).await
}

#[tokio::test]
async fn postgres_staff_message_pagination() -> TestResult {
    configured_staff_fixture("postgres_staff_message_pagination", Some("messages")).await
}
#[tokio::test]
async fn postgres_staff_moderation_ticket_lock_order() -> TestResult {
    configured_staff_fixture("postgres_staff_moderation_ticket_lock_order", Some("locks")).await
}
#[tokio::test]
async fn postgres_permanent_sanction_expiry_conflict() -> TestResult {
    configured_staff_fixture(
        "postgres_permanent_sanction_expiry_conflict",
        Some("expiry"),
    )
    .await
}

async fn configured_staff_fixture(test_name: &str, scenario: Option<&str>) -> TestResult {
    // Exercise the configured production composition with an isolated, explicit
    // fixture environment; never mutate process-global environment in the suite.
    if std::env::var("OTERYN_LK_STAFF_SUPPORT_FIXTURE").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                &format!("tickets::staff_tests::{test_name}"),
                "--nocapture",
            ])
            .env("OTERYN_LK_STAFF_SUPPORT_FIXTURE", "1")
            .env("OTERYN_ADMIN_PANEL", "true")
            .env("OTERYN_ADMIN_HOST", "staff.test")
            .env("OTERYN_SUPPORT_TICKETS", "true")
            .env("OTERYN_REPORTS_ENABLED", "true")
            .env("OTERYN_MFA_KEY", "07".repeat(32))
            .env("OTERYN_NOTIFY_UNSUBSCRIBE_KEY", "07".repeat(32))
            .env(
                "OTERYN_NOTIFY_UNSUBSCRIBE_URL",
                "https://portal.example.test/unsubscribe",
            )
            .status()?;
        assert!(
            status.success(),
            "configured staff support fixture failed: {status}"
        );
        return Ok(());
    }
    // Acquire in the child that owns the database fixture, never its launcher.
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("staff_support_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(if scenario == Some("appeals") { 8 } else { 1 })
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let staff = Uuid::now_v7();
    let owner = Uuid::now_v7();
    for id in [staff, owner] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'synthetic-hash','active','en',now())")
            .bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
    }
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('support','moderator')").bind(staff).execute(&pool).await?;
    let grant = oteryn_identity::mfa::random_token()?;
    let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true) RETURNING generation").bind(staff).fetch_one(&pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '1 hour')")
        .bind(oteryn_identity::mfa::digest(&grant)).bind(staff).bind(generation).bind(oteryn_identity::mfa::digest("synthetic-hash")).execute(&pool).await?;
    let bearer = oteryn_identity::mfa::random_token()?;
    let csrf = "a".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',$3,$4,(SELECT native_security_generation FROM accounts WHERE id=$2))")
        .bind(oteryn_identity::mfa::digest(&bearer)).bind(staff).bind(&csrf).bind(serde_json::json!({"mfa_grant":grant})).execute(&pool).await?;
    let cookie = format!("__Host-oteryn_session={bearer}");
    let ticket: Uuid = sqlx::query_scalar("INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,$2,'account','Existing ticket') RETURNING id").bind(owner).bind(Uuid::now_v7()).fetch_one(&pool).await?;
    let report: Uuid = sqlx::query_scalar("INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference) VALUES($1,$2,'player','conduct','Existing report') RETURNING id").bind(owner).bind(Uuid::now_v7()).fetch_one(&pool).await?;
    let sanction: Uuid = sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'warning','active','Existing sanction',to_timestamp(1794000000),to_timestamp(1894000000),$2) RETURNING id").bind(owner).bind(staff).fetch_one(&pool).await?;
    sqlx::query("INSERT INTO support_appeal_workflows(sanction_id,owner_id,status,message) VALUES($1,$2,'requested','Please review')").bind(sanction).bind(owner).execute(&pool).await?;
    sqlx::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,'support',true,true)").bind(owner).execute(&pool).await?;
    let host = "staff.test".to_owned();
    let lookup = |key: &str| match key {
        "OTERYN_SUPPORT_TICKETS" | "OTERYN_REPORTS_ENABLED" | "OTERYN_ADMIN_PANEL" => {
            Some("true".to_owned())
        }
        "OTERYN_ADMIN_HOST" => Some(host.clone()),
        // Synthetic test keys; grant checks never decrypt the fixture secret.
        "OTERYN_MFA_KEY" => Some("ab".repeat(32)),
        "OTERYN_NOTIFY_UNSUBSCRIBE_KEY" => Some("k".repeat(32)),
        "OTERYN_NOTIFY_UNSUBSCRIBE_URL" => Some("https://portal.test/unsubscribe".to_owned()),
        _ => None,
    };
    let app =
        configured_staff_router_from(pool.clone(), &lookup).map_err(|error| error.to_string())?;
    if scenario == Some("appeals") {
        browser_appeals(&pool, app.clone(), owner, staff, &cookie, &csrf).await?;
        appeal_decision_race(&pool, app, owner, staff, sanction, &cookie, &csrf).await?;
        pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .execute(&root)
            .await?;
        return Ok(());
    }
    if scenario == Some("locks") {
        let path = schema.clone();
        let concurrent_pool = sqlx::postgres::PgPoolOptions::new().max_connections(8)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)").bind(path).execute(c).await?;
                Ok(())
            })
        }).connect(&url).await?;
        let concurrent_app = configured_staff_router_from(concurrent_pool.clone(), &lookup)
            .map_err(|e| e.to_string())?;
        for enforcement in [false, true] {
            let mut fence = concurrent_pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(71010)")
                .execute(&mut *fence)
                .await?;
            let area = if enforcement {
                "moderation/enforcement"
            } else {
                "moderation/reports"
            };
            let id = if enforcement { sanction } else { report };
            let form = if enforcement {
                format!(
                    "lock_version=1&action=sanction&status=active&target={owner}&category=warning&public_reason=Existing+sanction&moderator_notes=&effective_at=1794000000&expires_at=1894000000"
                )
            } else {
                "lock_version=1&action=moderate&status=reviewing&public_outcome=&moderator_notes="
                    .into()
            };
            let a = concurrent_app.clone();
            let h = host.clone();
            let c = cookie.clone();
            let token = csrf.clone();
            let moderation =
                tokio::spawn(async move { staff_post(a, &h, &c, &token, area, id, &form).await });
            wait_staff_locks(&concurrent_pool, &schema, 1).await?;
            let a = concurrent_app.clone();
            let h = host.clone();
            let c = cookie.clone();
            let token = csrf.clone();
            let ticket_version = 1;
            let reply = tokio::spawn(async move {
                staff_post(
                a,
                &h,
                &c,
                &token,
                "support/tickets",
                ticket,
                &format!(
                    "lock_version={ticket_version}&action=reply&body=Concurrent+reply&internal=true"
                ),
            )
            .await
            });
            wait_staff_locks(&concurrent_pool, &schema, 2).await?;
            fence.commit().await?;
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(5), moderation).await???,
                StatusCode::SEE_OTHER
            );
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(5), reply).await???,
                StatusCode::SEE_OTHER
            );
            // Restore fixture versions for the existing route assertions below.
            sqlx::query("UPDATE tickets SET lock_version=1 WHERE id=$1")
                .bind(ticket)
                .execute(&pool)
                .await?;
            sqlx::query("UPDATE player_reports SET lock_version=1,status='submitted' WHERE id=$1")
                .bind(report)
                .execute(&pool)
                .await?;
            sqlx::query("UPDATE sanctions SET lock_version=1 WHERE id=$1")
                .bind(sanction)
                .execute(&pool)
                .await?;
        }
        concurrent_pool.close().await;
    }
    if scenario == Some("messages") {
        for internal in [false, true] {
            sqlx::query("DELETE FROM ticket_messages WHERE ticket_id=$1")
                .bind(ticket)
                .execute(&pool)
                .await?;
            let mut cursor = Uuid::nil();
            for n in 1..=51 {
                let message = Uuid::now_v7();
                if n == 50 {
                    cursor = message;
                }
                sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,'staff',$4,$5)")
                .bind(message).bind(ticket).bind(staff).bind(format!("Distinct message {n}")).bind(internal && n==51).execute(&pool).await?;
            }
            let mut next = format!("/admin/support/tickets/{ticket}");
            for page in 0..2 {
                let response = app
                    .clone()
                    .oneshot(
                        Request::get(&next)
                            .header("accept-language", "en")
                            .header("host", &host)
                            .header("cookie", &cookie)
                            .body(Body::empty())?,
                    )
                    .await?;
                assert_eq!(response.status(), StatusCode::OK);
                let html =
                    String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
                if page == 0 {
                    assert!(!html.contains("Distinct message 51"));
                    next = format!("/admin/support/tickets/{ticket}?after={cursor}");
                    assert!(
                        html.contains(&format!("href=\"{next}\"")),
                        "missing message continuation"
                    );
                } else {
                    assert!(html.contains("Distinct message 51"));
                    if internal {
                        assert!(html.contains("Internal"));
                    }
                }
            }
        }
    }
    for (area, id, title) in [
        ("support/tickets", ticket, "Existing ticket"),
        ("moderation/reports", report, "Existing report"),
        ("moderation/enforcement", sanction, "Existing sanction"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/admin/{area}/{id}"))
                    .header("host", &host)
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert!(body.contains(title));
    }
    let post = |area: &'static str, id: Uuid, form: String| {
        let app = app.clone();
        let host = host.clone();
        let cookie = cookie.clone();
        let csrf = csrf.clone();
        async move { staff_post(app, &host, &cookie, &csrf, area, id, &form).await }
    };
    assert_eq!(
        post(
            "support/tickets",
            ticket,
            "lock_version=1&action=reply&body=Staff+reply&internal=false".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM tickets WHERE id=$1")
            .bind(ticket)
            .fetch_one(&pool)
            .await?,
        "waiting_user"
    );
    assert_eq!(
        post(
            "support/tickets",
            ticket,
            "lock_version=1&action=ticket_status&status=closed".into()
        )
        .await?,
        StatusCode::CONFLICT
    );
    let before_notes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM notifications WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        post(
            "support/tickets",
            ticket,
            "lock_version=2&action=reply&body=Private+internal+note&internal=true".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?,
        before_notes
    );
    assert_eq!(
        post(
            "support/tickets",
            ticket,
            "lock_version=3&action=ticket_status&status=resolved".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(post("moderation/reports",report,"lock_version=1&action=moderate&status=rejected&public_outcome=Reviewed&moderator_notes=Private".into()).await?,StatusCode::SEE_OTHER);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM player_reports WHERE id=$1 AND rejected_at IS NOT NULL"
        )
        .bind(report)
        .fetch_one(&pool)
        .await?,
        "rejected"
    );
    assert_eq!(post("moderation/enforcement",sanction,format!("lock_version=1&action=sanction&status=active&target={owner}&category=warning&public_reason=Updated&moderator_notes=Private&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::SEE_OTHER);
    assert_eq!(
        post(
            "moderation/enforcement",
            sanction,
            "lock_version=2&action=appeal&appeal_status=reviewing&appeal_outcome=".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    sqlx::query("CREATE FUNCTION reject_appeal_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''audit unavailable''; END'").execute(&pool).await?;
    sqlx::query("CREATE TRIGGER reject_appeal_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_appeal_audit()").execute(&pool).await?;
    let notification_count: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        post(
            "moderation/enforcement",
            sanction,
            "lock_version=4&action=appeal&appeal_status=accepted&appeal_outcome=Accepted".into()
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM sanctions WHERE id=$1")
            .bind(sanction)
            .fetch_one(&pool)
            .await?,
        "active"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM support_appeal_workflows WHERE sanction_id=$1"
        )
        .bind(sanction)
        .fetch_one(&pool)
        .await?,
        "reviewing"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications")
            .fetch_one(&pool)
            .await?,
        notification_count
    );
    sqlx::query("DROP TRIGGER reject_appeal_audit ON audit_log")
        .execute(&pool)
        .await?;
    assert_eq!(
        post(
            "moderation/enforcement",
            sanction,
            "lock_version=4&action=appeal&appeal_status=accepted&appeal_outcome=Accepted".into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM sanctions WHERE id=$1")
            .bind(sanction)
            .fetch_one(&pool)
            .await?,
        "revoked"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM support_appeal_workflows WHERE sanction_id=$1"
        )
        .bind(sanction)
        .fetch_one(&pool)
        .await?,
        "accepted"
    );
    let created = Uuid::now_v7();
    assert_eq!(post("moderation/enforcement",created,format!("lock_version=1&action=sanction&create=true&status=active&target={owner}&category=warning&public_reason=Created&moderator_notes=&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::SEE_OTHER);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sanctions WHERE id=$1")
            .bind(created)
            .fetch_one(&pool)
            .await?,
        1
    );
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?
            >= 4
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM email_outbox WHERE account_id=$1 AND text_body LIKE '%Private%'"
        )
        .bind(owner)
        .fetch_one(&pool)
        .await?,
        0
    );
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM email_outbox WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?
            >= 4
    );
    sqlx::query("CREATE FUNCTION reject_staff_support_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''audit unavailable''; END'").execute(&pool).await?;
    sqlx::query("CREATE TRIGGER reject_staff_support_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_staff_support_audit()").execute(&pool).await?;
    assert_eq!(
        post(
            "moderation/reports",
            report,
            "lock_version=2&action=moderate&status=closed&public_outcome=Closed&moderator_notes="
                .into()
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM player_reports WHERE id=$1")
            .bind(report)
            .fetch_one(&pool)
            .await?,
        "rejected"
    );
    sqlx::query("DROP TRIGGER reject_staff_support_audit ON audit_log")
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await?;
    assert_eq!(post("moderation/enforcement",created,format!("lock_version=1&action=sanction&status=revoked&target={owner}&category=warning&public_reason=Revoked&moderator_notes=&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::SEE_OTHER);
    assert_eq!(
        post(
            "moderation/reports",
            report,
            "lock_version=2&action=moderate&status=closed&public_outcome=Closed&moderator_notes="
                .into()
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    let pending = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'synthetic-hash','pending_verification','en')").bind(pending).bind(format!("{pending}@example.invalid")).execute(&pool).await?;
    let pending_report:Uuid=sqlx::query_scalar("INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference) VALUES($1,$2,'player','conduct','Pending reporter') RETURNING id").bind(pending).bind(Uuid::now_v7()).fetch_one(&pool).await?;
    assert_eq!(post("moderation/reports",pending_report,"lock_version=1&action=moderate&status=rejected&public_outcome=Reviewed&moderator_notes=".into()).await?,StatusCode::SEE_OTHER);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications WHERE account_id=$1")
            .bind(pending)
            .fetch_one(&pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM email_outbox WHERE account_id=$1")
            .bind(pending)
            .fetch_one(&pool)
            .await?,
        0
    );
    let permanent:Uuid=sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,created_by,approved_by) VALUES($1,'suspension','active','Permanent',now(),$2,$1) RETURNING id").bind(owner).bind(staff).fetch_one(&pool).await?;
    sqlx::query("INSERT INTO support_appeal_workflows(sanction_id,owner_id,status,message) VALUES($1,$2,'requested','Please review this decision')").bind(permanent).bind(owner).execute(&pool).await?;
    if scenario == Some("expiry") {
        let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
            .fetch_one(&pool)
            .await?;
        let notifications_before: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications")
            .fetch_one(&pool)
            .await?;
        let emails_before: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox")
            .fetch_one(&pool)
            .await?;
        assert_eq!(post("moderation/enforcement",permanent,format!("lock_version=1&action=sanction&status=expired&target={owner}&category=suspension&public_reason=Permanent&moderator_notes=&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::CONFLICT);
        assert!(sqlx::query_scalar::<_,bool>("SELECT expires_at IS NULL AND status='active' AND lock_version=1 FROM sanctions WHERE id=$1").bind(permanent).fetch_one(&pool).await?);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
                .fetch_one(&pool)
                .await?,
            audit_before
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications")
                .fetch_one(&pool)
                .await?,
            notifications_before
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM email_outbox")
                .fetch_one(&pool)
                .await?,
            emails_before
        );
    }
    let mut owner_tx = pool.begin().await?;
    sqlx::query("SELECT id FROM sanctions WHERE id=$1 FOR UPDATE")
        .bind(permanent)
        .fetch_one(&mut *owner_tx)
        .await?;
    crate::appeals::Appeals { enabled: true }
        .change(
            &mut owner_tx,
            owner,
            permanent,
            1,
            crate::appeals::Action::Acknowledge,
        )
        .await?;
    owner_tx.commit().await?;
    assert_eq!(post("moderation/enforcement",permanent,format!("lock_version=1&action=sanction&status=revoked&target={owner}&category=suspension&public_reason=Revoked&moderator_notes=&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::CONFLICT);
    assert_eq!(post("moderation/enforcement",permanent,format!("lock_version=2&action=sanction&status=revoked&target={owner}&category=suspension&public_reason=Revoked&moderator_notes=&effective_at=1794000000&expires_at=1894000000")).await?,StatusCode::SEE_OTHER);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT expires_at IS NULL AND status='revoked' FROM sanctions WHERE id=$1"
        )
        .bind(permanent)
        .fetch_one(&pool)
        .await?
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!("{test_name} duration: {:?}", started.elapsed());
    Ok(())
}
async fn staff_post(
    app: Router,
    host: &str,
    cookie: &str,
    csrf: &str,
    area: &str,
    id: Uuid,
    form: &str,
) -> std::result::Result<StatusCode, Box<dyn std::error::Error + Send + Sync>> {
    Ok(app
        .oneshot(
            Request::post(format!("/admin/{area}/{id}"))
                .header("host", host)
                .header("cookie", cookie)
                .header("origin", format!("https://{host}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&{form}")))?,
        )
        .await?
        .status())
}

async fn wait_staff_locks(pool: &PgPool, application: &str, count: i64) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND state='active' AND wait_event_type='Lock'")
                .bind(application).fetch_one(pool).await?;
            if waiting >= count { return Ok::<_,sqlx::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await??;
    Ok(())
}

#[derive(Clone)]
struct PausedPreparation {
    backend: StaffSupport,
    prepared: Arc<tokio::sync::Notify>,
    resume: Arc<tokio::sync::Notify>,
}
impl oteryn_admin::support::Backend for PausedPreparation {
    async fn prepare(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: oteryn_admin::rbac::CheckContext,
        id: Uuid,
        command: &oteryn_admin::support::Command,
    ) -> std::result::Result<(), oteryn_admin::support::Error> {
        self.backend.prepare(tx, context, id, command).await?;
        self.prepared.notify_one();
        self.resume.notified().await;
        Ok(())
    }
    async fn list(
        &self,
        area: oteryn_admin::support::Area,
        page: u32,
    ) -> std::result::Result<Vec<oteryn_admin::support::Record>, oteryn_admin::support::Error> {
        self.backend.list(area, page).await
    }
    async fn load(
        &self,
        area: oteryn_admin::support::Area,
        id: Uuid,
        after: Option<Uuid>,
    ) -> std::result::Result<oteryn_admin::support::Record, oteryn_admin::support::Error> {
        self.backend.load(area, id, after).await
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
        id: Uuid,
        version: i64,
        command: &oteryn_admin::support::Command,
    ) -> std::result::Result<(serde_json::Value, serde_json::Value), oteryn_admin::support::Error>
    {
        self.backend.apply(tx, context, id, version, command).await
    }
}
#[tokio::test]
async fn postgres_ticket_preparation_and_player_note_share_account_order() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    use oteryn_admin::{
        players::{Flags, IdentityAccounts, Players},
        rbac::{Authorizer, CheckContext},
        web::PortalIdentity,
    };
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("account_order_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search = schema.clone();
    let pool=sqlx::postgres::PgPoolOptions::new().max_connections(6).after_connect(move |c,_| {
        let search=search.clone(); Box::pin(async move {
            sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)").bind(search).execute(c).await?;Ok(())
        })
    }).connect(&url).await?;
    oteryn_db::migrate(&pool).await?;
    let actor = Uuid::now_v7();
    let owner = Uuid::now_v7();
    for id in [actor, owner] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'synthetic-hash','active','en',now())").bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
    }
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(actor)
    .execute(&pool)
    .await?;
    let ticket:Uuid=sqlx::query_scalar("INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,$2,'account','Identity help') RETURNING id").bind(owner).bind(Uuid::now_v7()).fetch_one(&pool).await?;
    let lookup = |key: &str| match key {
        "OTERYN_SUPPORT_TICKETS" => Some("true".to_owned()),
        "OTERYN_NOTIFY_UNSUBSCRIBE_KEY" => Some("x".repeat(32)),
        "OTERYN_NOTIFY_UNSUBSCRIBE_URL" => Some("https://portal.test/unsubscribe".to_owned()),
        _ => None,
    };
    let prepared = Arc::new(tokio::sync::Notify::new());
    let resume = Arc::new(tokio::sync::Notify::new());
    let backend = PausedPreparation {
        backend: StaffSupport::configured_from(pool.clone(), &lookup).map_err(|e| e.to_string())?,
        prepared: prepared.clone(),
        resume: resume.clone(),
    };
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(&pool, actor).await?),
    };
    let reply_pool = pool.clone();
    let reply = tokio::spawn(async move {
        let auth = Authorizer::new(
            reply_pool.clone(),
            PortalIdentity::configured(reply_pool.clone()),
        );
        oteryn_admin::support::execute(
            &reply_pool,
            &auth,
            &backend,
            context,
            ticket,
            1,
            &oteryn_admin::support::Command::Reply {
                body: "Verified reply".into(),
                internal: false,
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), prepared.notified()).await?;
    let players = Players::new(
        pool.clone(),
        Flags {
            players: true,
            ..Flags::default()
        },
        PortalIdentity::configured(pool.clone()),
        IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts { pool: pool.clone() },
            projection: None,
        },
    );
    let note = tokio::spawn(async move { players.note(context, owner, "Support note").await });
    // Observe the note blocked by prepared ticket locks. Before the fix it holds
    // 71010 while waiting for the actor's SHARE lock, creating the cycle on resume.
    tokio::time::timeout(std::time::Duration::from_secs(2),async {
        loop {
            let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock' AND query LIKE '%FROM accounts%')").bind(&schema).fetch_one(&root).await?;
            if waiting {break;}
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<(),sqlx::Error>(())
    }).await??;
    resume.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(2), reply).await???;
    tokio::time::timeout(std::time::Duration::from_secs(2), note).await???;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM player_notes WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ticket_messages WHERE ticket_id=$1")
            .bind(ticket)
            .fetch_one(&pool)
            .await?,
        1
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!(
        "postgres_ticket_preparation_and_player_note_share_account_order duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_browser_appeals() -> TestResult {
    configured_staff_fixture("postgres_browser_appeals", Some("appeals")).await
}

async fn browser_appeals(
    pool: &PgPool,
    staff_app: Router,
    owner: Uuid,
    staff: Uuid,
    staff_cookie: &str,
    csrf: &str,
) -> TestResult {
    use axum::extract::ConnectInfo;
    use std::net::SocketAddr;

    let host = "staff.test";
    let origin = format!("https://{host}");
    let player_app = crate::appeals::web::router(pool.clone(), &origin, true);
    let bearer = oteryn_identity::mfa::random_token()?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,authorization_generation) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',$3,(SELECT native_security_generation FROM accounts WHERE id=$2))")
        .bind(oteryn_identity::mfa::digest(&bearer)).bind(owner).bind(csrf).execute(pool).await?;
    let owner_cookie = format!("__Host-oteryn_session={bearer}");
    // Existing sanctions can predate workflow registration.
    let sanction: Uuid = sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,moderator_notes,effective_at,expires_at,created_by) VALUES($1,'suspension','active','Restriction','Private note',now()-interval '1 minute',now()+interval '1 hour',$2) RETURNING id")
        .bind(owner).bind(staff).fetch_one(pool).await?;
    let game_features = crate::sanctions::Features {
        game_access: true,
        ..Default::default()
    };
    assert!(matches!(
        crate::sanctions::require_game_access(&mut *pool.acquire().await?, owner, game_features)
            .await,
        Err(crate::sanctions::Error::Denied)
    ));
    let path = format!("/en/support/appeals/{sanction}");

    let get = |cookie: &str| {
        let request = Request::get(&path)
            .header("cookie", cookie)
            .body(Body::empty());
        let app = player_app.clone();
        async move { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(app.oneshot(request?).await?) }
    };
    assert_eq!(get(staff_cookie).await?.status(), StatusCode::NOT_FOUND);
    assert_eq!(get("").await?.status(), StatusCode::FORBIDDEN);
    let page = get(&owner_cookie).await?;
    assert_eq!(page.status(), StatusCode::OK);
    assert!(
        page.headers()["content-security-policy"]
            .to_str()?
            .contains("frame-ancestors 'none'")
    );
    assert_eq!(page.headers()["cache-control"], "no-store");
    let html = String::from_utf8(to_bytes(page.into_body(), 100000).await?.to_vec())?;
    assert!(html.contains("Submit appeal"));
    assert!(!html.contains("Private note"));

    let request = |cookie: &str, lang: &str, id: Uuid, form: &str, source: u8| {
        Request::post(format!("/{lang}/support/appeals/{id}"))
            .header("host", host)
            .header("cookie", cookie)
            .header("origin", &origin)
            .header("content-type", "application/x-www-form-urlencoded")
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, source], 12345))))
            .body(Body::from(form.to_owned()))
    };
    let send = |cookie: &str, form: &str| {
        let request = request(cookie, "en", sanction, form, 1);
        let app = player_app.clone();
        async move { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(app.oneshot(request?).await?) }
    };
    let valid = format!("_csrf={csrf}&version=1&message=Please+review");
    for disabled in [
        "OTERYN_ADMIN_PANEL",
        "OTERYN_ACCOUNT_MFA",
        "OTERYN_ACCOUNT_LOGIN",
    ] {
        let unavailable =
            super::staff::configured_support_router_from(pool.clone(), &origin, &|key| {
                if key == disabled {
                    return Some("false".into());
                }
                match key {
                    "OTERYN_ADMIN_PANEL" | "OTERYN_ACCOUNT_MFA" | "OTERYN_ACCOUNT_LOGIN" => {
                        Some("true".into())
                    }
                    "OTERYN_ADMIN_HOST" => Some(host.into()),
                    "OTERYN_MFA_KEY" => Some("ab".repeat(32)),
                    "OTERYN_NOTIFY_UNSUBSCRIBE_KEY" => Some("k".repeat(32)),
                    "OTERYN_NOTIFY_UNSUBSCRIBE_URL" => {
                        Some("https://portal.test/unsubscribe".into())
                    }
                    _ => None,
                }
            })
            .map_err(|error| error.to_string())?;
        let history = unavailable
            .clone()
            .oneshot(
                Request::get(&path)
                    .header("cookie", &owner_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(history.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(history.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains("Restriction"));
        assert!(!html.contains("Submit appeal"));
        assert_eq!(
            unavailable
                .clone()
                .oneshot(request(&owner_cookie, "en", sanction, &valid, 1)?)
                .await?
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(
            unavailable
                .oneshot(request("", "en", sanction, "", 1)?)
                .await?
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM support_appeal_workflows WHERE sanction_id=$1)"
        )
        .bind(sanction)
        .fetch_one(pool)
        .await?
    );

    let stale_csrf = format!("_csrf={}&version=1&message=Please+review", "b".repeat(64));
    let stale = player_app
        .clone()
        .oneshot(request(&owner_cookie, "pl", sanction, &stale_csrf, 1)?)
        .await?;
    assert_eq!(stale.status(), StatusCode::FORBIDDEN);
    let html = String::from_utf8(to_bytes(stale.into_body(), 100000).await?.to_vec())?;
    assert!(html.contains("Brak dostępu"));

    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(owner)
    .execute(pool)
    .await?;
    for stale in [
        get(&owner_cookie).await?,
        send(&owner_cookie, &valid).await?,
    ] {
        assert_eq!(stale.status(), StatusCode::FORBIDDEN);
        let html = String::from_utf8(to_bytes(stale.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains("Sign in again to use appeals."));
        assert!(!html.contains("Submit appeal"));
    }
    // Bind the fixture to the new generation before exercising the successful flow.
    sqlx::query("UPDATE web_sessions s SET authorization_generation=a.native_security_generation FROM accounts a WHERE a.id=s.account_id AND s.id_hash=$1")
        .bind(oteryn_identity::mfa::digest(&bearer)).execute(pool).await?;

    assert_eq!(
        send(&owner_cookie, "version=1&message=Please+review")
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(staff_cookie, &valid).await?.status(),
        StatusCode::NOT_FOUND
    );
    let invalid = format!("_csrf={csrf}&version=1&message=+++");
    let response = send(&owner_cookie, &invalid).await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    assert!(html.contains("Please check your input"));
    assert_eq!(
        send(&owner_cookie, &valid).await?.status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        send(&owner_cookie, &valid).await?.status(),
        StatusCode::CONFLICT
    );
    // Align with the real fixed-window counter, even if an earlier request crossed the minute.
    let mut limited = false;
    for _ in 0..4 {
        let response = player_app
            .clone()
            .oneshot(request(&owner_cookie, "pl", sanction, &valid, 1)?)
            .await?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry = response.headers()["retry-after"].to_str()?.parse::<u32>()?;
            assert!((1..=60).contains(&retry));
            let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
            assert!(html.contains("Zbyt wiele żądań"));
            limited = true;
            break;
        }
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
    assert!(limited);

    let decide = |cookie: &str, form: String| {
        let app = staff_app.clone();
        let cookie = cookie.to_owned();
        async move {
            staff_post(
                app,
                host,
                &cookie,
                csrf,
                "moderation/enforcement",
                sanction,
                &form,
            )
            .await
        }
    };
    let decision = "lock_version=2&action=appeal&appeal_status=accepted&appeal_outcome=Overturned";
    assert_eq!(
        decide(&owner_cookie, decision.into()).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        decide(
            staff_cookie,
            "lock_version=2&action=appeal&appeal_status=accepted&appeal_outcome=+".into()
        )
        .await?,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        decide(staff_cookie, decision.into()).await?,
        StatusCode::SEE_OTHER
    );
    // Even a fresh lock version cannot reopen or reverse a terminal decision.
    assert_eq!(
        decide(
            staff_cookie,
            "lock_version=4&action=appeal&appeal_status=rejected&appeal_outcome=Different".into()
        )
        .await?,
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM sanctions WHERE id=$1")
            .bind(sanction)
            .fetch_one(pool)
            .await?,
        "revoked"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE target=$1 AND action IN ('support.enforcement_appeal_submitted','support.enforcement_appeal_updated')")
            .bind(sanction.to_string()).fetch_one(pool).await?,
        2
    );
    crate::sanctions::require_game_access(&mut *pool.acquire().await?, owner, game_features)
        .await?;
    let page = get(&owner_cookie).await?;
    assert_eq!(page.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(page.into_body(), 100000).await?.to_vec())?;
    assert!(html.contains("Overturned"));
    assert!(!html.contains("Submit appeal"));

    let fresh: Uuid = sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','Revocation test',now(),now()+interval '1 hour',$2) RETURNING id")
        .bind(owner).bind(staff).fetch_one(pool).await?;
    let application: String = sqlx::query_scalar("SELECT current_setting('application_name')")
        .fetch_one(pool)
        .await?;
    use oteryn_web::ratelimit::{Keys, NamedPolicy, RateLimiter};
    let mut quota_key = Keys::source([127, 0, 0, 7].into());
    quota_key.account = Some(owner.to_string());
    let policy = NamedPolicy::SupportEnforcementAppeal;
    let limiter = RateLimiter::new(pool.clone());
    // Leave enough time in the current fixed window for the row lock to remain relevant.
    tokio::time::timeout(std::time::Duration::from_secs(6), async {
        loop {
            if sqlx::query_scalar::<_, bool>("SELECT extract(second FROM clock_timestamp()) < 55")
                .fetch_one(pool)
                .await?
            {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    limiter.check(policy, &[quota_key]).await?;
    let mut quota_lock = pool.begin().await?;
    sqlx::query("SELECT key_hash FROM web_rate_limits WHERE limiter_name=$1 FOR UPDATE")
        .bind(policy.name())
        .execute(&mut *quota_lock)
        .await?;
    let quota =
        tokio::spawn(
            player_app
                .clone()
                .oneshot(request(&owner_cookie, "en", fresh, &valid, 7)?),
        );
    wait_staff_locks(pool, &application, 1).await?;
    let blocked = tokio::time::timeout(std::time::Duration::from_secs(4), quota).await???;
    assert_eq!(blocked.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(blocked.headers().contains_key("retry-after"));
    quota_lock.rollback().await?;

    for (lang, title) in [
        ("en", "Temporarily unavailable"),
        ("pl", "Usługa chwilowo niedostępna"),
    ] {
        let mut session_lock = pool.begin().await?;
        sqlx::query("SELECT id_hash FROM web_sessions WHERE id_hash=$1 FOR UPDATE")
            .bind(oteryn_identity::mfa::digest(&bearer))
            .execute(&mut *session_lock)
            .await?;
        let refresh_timeout = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            player_app
                .clone()
                .oneshot(request(&owner_cookie, lang, fresh, &valid, 5)?),
        )
        .await??;
        assert_eq!(refresh_timeout.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refresh_timeout.headers()["cache-control"], "no-store");
        assert!(
            refresh_timeout.headers()["content-type"]
                .to_str()?
                .starts_with("text/html")
        );
        let body = String::from_utf8(
            to_bytes(refresh_timeout.into_body(), 100000)
                .await?
                .to_vec(),
        )?;
        assert!(body.contains(title));
        assert!(body.contains(&format!("lang=\"{lang}\"")));
        session_lock.rollback().await?;
    }

    let chrome_app = crate::appeals::web::with_browser_deadline(player_app.clone().layer(
        axum::middleware::from_fn_with_state(
            oteryn_web::chrome::Shell {
                registry: oteryn_ui::Registry::default(),
                sessions: oteryn_web::session::SessionLayer::new(pool.clone()),
            },
            oteryn_web::chrome::middleware,
        ),
    ));
    // Both shared chrome and standalone refresh must refuse while the pool is still exhausted.
    let mut held = Vec::new();
    for _ in 0..pool.options().get_max_connections() {
        held.push(pool.acquire().await?);
    }
    for app in [player_app.clone(), chrome_app.clone()] {
        let pool_timeout = tokio::time::timeout(
            std::time::Duration::from_secs(4),
            app.oneshot(request(&owner_cookie, "en", fresh, &valid, 5)?),
        )
        .await??;
        assert_eq!(pool_timeout.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    drop(held);

    // Spend part of the same budget in shared chrome, then hold the row lock
    // through the remaining time available to session refresh.
    let mut shared_lock = pool.begin().await?;
    sqlx::query("SELECT id_hash FROM web_sessions WHERE id_hash=$1 FOR UPDATE")
        .bind(oteryn_identity::mfa::digest(&bearer))
        .execute(&mut *shared_lock)
        .await?;
    let mut shared_leases = Vec::new();
    for _ in 1..pool.options().get_max_connections() {
        shared_leases.push(pool.acquire().await?);
    }
    let started_at = tokio::time::Instant::now();
    let response =
        tokio::spawn(chrome_app.oneshot(request(&owner_cookie, "en", fresh, &valid, 5)?));
    while started_at.elapsed() < std::time::Duration::from_millis(1800) {
        assert!(
            !response.is_finished(),
            "chrome must wait for the exhausted pool"
        );
        tokio::task::yield_now().await;
    }
    drop(shared_leases.pop());
    let response =
        tokio::time::timeout_at(started_at + std::time::Duration::from_secs(4), response)
            .await???;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    assert!(body.contains("Temporarily unavailable"));
    shared_lock.rollback().await?;
    drop(shared_leases);

    // The submission must roll back if the session expires after the write,
    // while the audit append is blocked.
    let expiring_bearer = oteryn_identity::mfa::random_token()?;
    let expiring_hash = oteryn_identity::mfa::digest(&expiring_bearer);
    let expiring_cookie = format!("__Host-oteryn_session={expiring_bearer}");
    let mut audit_lock = pool.begin().await?;
    sqlx::query("LOCK TABLE audit_log IN SHARE MODE")
        .execute(&mut *audit_lock)
        .await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,authorization_generation) SELECT $1,id,clock_timestamp()+interval '2 seconds',clock_timestamp()+interval '2 seconds','test',$2,native_security_generation FROM accounts WHERE id=$3")
        .bind(&expiring_hash).bind(csrf).bind(owner).execute(pool).await?;
    let pending = tokio::spawn(player_app.clone().oneshot(request(
        &expiring_cookie,
        "en",
        fresh,
        &valid,
        3,
    )?));
    wait_staff_locks(pool, &application, 1).await?;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let expired: bool = sqlx::query_scalar("SELECT absolute_expires_at <= clock_timestamp() FROM web_sessions WHERE id_hash=$1")
                .bind(&expiring_hash).fetch_one(pool).await?;
            if expired {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    }).await??;
    audit_lock.commit().await?;
    assert_eq!(pending.await??.status(), StatusCode::FORBIDDEN);

    let mut sanction_lock = pool.begin().await?;
    sqlx::query("SELECT id FROM sanctions WHERE id=$1 FOR UPDATE")
        .bind(fresh)
        .execute(&mut *sanction_lock)
        .await?;
    let timed_out = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        player_app
            .clone()
            .oneshot(request(&owner_cookie, "en", fresh, &valid, 4)?),
    )
    .await??;
    assert_eq!(timed_out.status(), StatusCode::SERVICE_UNAVAILABLE);
    sanction_lock.rollback().await?;

    // Block deferred work inside COMMIT, after the last application-side session check.
    sqlx::raw_sql(
        "CREATE FUNCTION f4_wait_in_commit() RETURNS trigger LANGUAGE plpgsql AS
         $$ BEGIN PERFORM pg_advisory_xact_lock(84209, 9130); RETURN NEW; END $$;
         CREATE CONSTRAINT TRIGGER f4_wait_in_commit AFTER INSERT ON audit_log
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         WHEN (NEW.action = 'support.enforcement_appeal_submitted')
         EXECUTE FUNCTION f4_wait_in_commit()",
    )
    .execute(pool)
    .await?;
    let mut commit_lock = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(84209, 9130)")
        .execute(&mut *commit_lock)
        .await?;
    let commit_bearer = oteryn_identity::mfa::random_token()?;
    let commit_cookie = format!("__Host-oteryn_session={commit_bearer}");
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,authorization_generation) SELECT $1,id,clock_timestamp()+interval '1700 milliseconds',clock_timestamp()+interval '1700 milliseconds','test',$2,native_security_generation FROM accounts WHERE id=$3")
        .bind(oteryn_identity::mfa::digest(&commit_bearer)).bind(csrf).bind(owner).execute(pool).await?;
    let committing =
        tokio::spawn(
            player_app
                .clone()
                .oneshot(request(&commit_cookie, "en", fresh, &valid, 6)?),
        );
    wait_for_appeal_commit(pool, &application).await?;
    // Wait for the session deadline to abort COMMIT while its blocker remains held.
    // A request that only has the three-second mutation timer cannot finish in this window.
    let expired =
        tokio::time::timeout(std::time::Duration::from_millis(2300), committing).await???;
    assert_eq!(expired.status(), StatusCode::SERVICE_UNAVAILABLE);
    commit_lock.rollback().await?;
    sqlx::raw_sql("DROP TRIGGER f4_wait_in_commit ON audit_log; DROP FUNCTION f4_wait_in_commit()")
        .execute(pool)
        .await?;

    let rejected: Uuid = sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','New evidence test',now(),now()+interval '1 hour',$2) RETURNING id")
        .bind(owner).bind(staff).fetch_one(pool).await?;
    sqlx::query("INSERT INTO support_appeal_workflows(sanction_id,owner_id,status,message,outcome,version) VALUES($1,$2,'rejected','Initial evidence','Previous decision',4)")
        .bind(rejected).bind(owner).execute(pool).await?;
    let rejected_page = player_app
        .clone()
        .oneshot(
            Request::get(format!("/en/support/appeals/{rejected}"))
                .header("cookie", &owner_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(rejected_page.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(rejected_page.into_body(), 100000).await?.to_vec())?;
    assert!(body.contains("Submit appeal"));
    assert!(body.contains("Previous decision"));
    let retry = format!("_csrf={csrf}&version=4&message=New+evidence");
    assert_eq!(
        player_app
            .clone()
            .oneshot(request(&owner_cookie, "en", rejected, &retry, 8)?)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    let stored: (String, String, Option<String>, i64) = sqlx::query_as(
        "SELECT status,message,outcome,version FROM support_appeal_workflows WHERE sanction_id=$1",
    )
    .bind(rejected)
    .fetch_one(pool)
    .await?;
    assert_eq!(stored, ("requested".into(), "New evidence".into(), None, 5));
    let prior: (String, i64) = sqlx::query_as("SELECT before->>'status',(before->>'version')::bigint FROM audit_log WHERE action='support.enforcement_appeal_submitted' AND target=$1")
        .bind(rejected.to_string()).fetch_one(pool).await?;
    assert_eq!(prior, ("rejected".into(), 4));

    let mut lock = lock_appeal_owner(pool, owner).await?;
    let pending = tokio::spawn(player_app.oneshot(request(&owner_cookie, "en", fresh, &valid, 2)?));
    wait_staff_locks(pool, &application, 1).await?;
    sqlx::query("DELETE FROM web_sessions WHERE id_hash=$1")
        .bind(oteryn_identity::mfa::digest(&bearer))
        .execute(&mut *lock)
        .await?;
    lock.commit().await?;
    assert_eq!(pending.await??.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM support_appeal_workflows WHERE sanction_id=$1"
        )
        .bind(fresh)
        .fetch_one(pool)
        .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE target=$1 AND action='support.enforcement_appeal_submitted'")
            .bind(fresh.to_string()).fetch_one(pool).await?, 0
    );
    Ok(())
}

async fn lock_appeal_owner(pool: &PgPool, owner: Uuid) -> Result<Transaction<'static, Postgres>> {
    let mut lock = pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .execute(&mut *lock)
        .await?;
    Ok(lock)
}

async fn appeal_decision_race(
    pool: &PgPool,
    app: Router,
    owner: Uuid,
    staff: Uuid,
    sanction: Uuid,
    cookie: &str,
    csrf: &str,
) -> TestResult {
    let accepted = "lock_version=1&action=appeal&appeal_status=accepted&appeal_outcome=Overturned";
    let rejected = "lock_version=1&action=appeal&appeal_status=rejected&appeal_outcome=Upheld";
    let host = "staff.test";
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=(SELECT id FROM roles WHERE key='moderator')")
        .bind(staff).execute(pool).await?;
    let allowed = app
        .clone()
        .oneshot(
            Request::get("/admin/support/tickets")
                .header("host", host)
                .header("cookie", cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        allowed.status(),
        StatusCode::OK,
        "the support actor has a live MFA staff session"
    );
    assert_eq!(
        staff_post(
            app.clone(),
            host,
            cookie,
            csrf,
            "moderation/enforcement",
            sanction,
            accepted
        )
        .await?,
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'")
        .bind(staff).execute(pool).await?;

    let application: String = sqlx::query_scalar("SELECT current_setting('application_name')")
        .fetch_one(pool)
        .await?;
    sqlx::raw_sql(
        "CREATE FUNCTION f4_wait_in_commit() RETURNS trigger LANGUAGE plpgsql AS
         $$ BEGIN PERFORM pg_advisory_xact_lock(84209, 9130); RETURN NEW; END $$;
         CREATE CONSTRAINT TRIGGER f4_wait_in_commit AFTER INSERT ON audit_log
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         WHEN (NEW.action = 'support.enforcement_appeal_updated')
         EXECUTE FUNCTION f4_wait_in_commit()",
    )
    .execute(pool)
    .await?;
    // Make each authorization component the earliest deadline in turn.
    for (shorten, restore) in [
        (
            "UPDATE web_sessions SET absolute_expires_at=clock_timestamp()+interval '1700 milliseconds' WHERE account_id=$1",
            "UPDATE web_sessions SET absolute_expires_at=clock_timestamp()+interval '1 hour',idle_expires_at=clock_timestamp()+interval '1 hour' WHERE account_id=$1",
        ),
        (
            "UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '1700 milliseconds' WHERE account_id=$1 AND purpose='grant'",
            "UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '1 hour' WHERE account_id=$1 AND purpose='grant'",
        ),
    ] {
        let mut blocker = pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(84209, 9130)")
            .execute(&mut *blocker)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(shorten))
            .bind(staff)
            .execute(pool)
            .await?;
        let staff_app = app.clone();
        let cookie = cookie.to_owned();
        let csrf = csrf.to_owned();
        let pending = tokio::spawn(async move {
            staff_post(
                staff_app,
                host,
                &cookie,
                &csrf,
                "moderation/enforcement",
                sanction,
                accepted,
            )
            .await
        });
        wait_for_appeal_commit(pool, &application).await?;
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_millis(2300), pending).await???,
            StatusCode::SERVICE_UNAVAILABLE
        );
        blocker.rollback().await?;
        sqlx::query(sqlx::AssertSqlSafe(restore))
            .bind(staff)
            .execute(pool)
            .await?;
        let state: (String, String, i64) = sqlx::query_as(
            "SELECT s.status,a.status,a.version FROM sanctions s JOIN support_appeal_workflows a ON a.sanction_id=s.id WHERE s.id=$1")
            .bind(sanction).fetch_one(pool).await?;
        assert_eq!(state, ("active".into(), "requested".into(), 1));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE target=$1 AND action='support.enforcement_appeal_updated'")
            .bind(sanction.to_string()).fetch_one(pool).await?, 0);
    }
    sqlx::raw_sql("DROP TRIGGER f4_wait_in_commit ON audit_log; DROP FUNCTION f4_wait_in_commit()")
        .execute(pool)
        .await?;

    let lock = lock_appeal_owner(pool, owner).await?;
    let start = |form: &'static str| {
        let app = app.clone();
        let cookie = cookie.to_owned();
        let csrf = csrf.to_owned();
        tokio::spawn(async move {
            staff_post(
                app,
                host,
                &cookie,
                &csrf,
                "moderation/enforcement",
                sanction,
                form,
            )
            .await
        })
    };
    let accept = start(accepted);
    let reject = start(rejected);
    wait_staff_locks(pool, &application, 2).await?;
    lock.commit().await?;
    let (accept, reject) = (accept.await??, reject.await??);
    assert!(
        matches!(
            (accept, reject),
            (StatusCode::SEE_OTHER, StatusCode::CONFLICT)
                | (StatusCode::CONFLICT, StatusCode::SEE_OTHER)
        ),
        "opposing decisions returned {accept} and {reject}"
    );
    let (sanction_status, appeal_status): (String, String) = sqlx::query_as(
        "SELECT s.status,a.status FROM sanctions s JOIN support_appeal_workflows a ON a.sanction_id=s.id WHERE s.id=$1"
    ).bind(sanction).fetch_one(pool).await?;
    if accept == StatusCode::SEE_OTHER {
        assert_eq!(
            (sanction_status.as_str(), appeal_status.as_str()),
            ("revoked", "accepted")
        );
    } else {
        assert_eq!(
            (sanction_status.as_str(), appeal_status.as_str()),
            ("active", "rejected")
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE target=$1 AND action='support.enforcement_appeal_updated'")
            .bind(sanction.to_string()).fetch_one(pool).await?, 1
    );
    Ok(())
}

async fn wait_for_appeal_commit(pool: &PgPool, application: &str) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND query='COMMIT' AND wait_event_type='Lock' AND wait_event='advisory')")
                .bind(application).fetch_one(pool).await?;
            if blocked {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    }).await??;
    Ok(())
}
