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
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("staff_support_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
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
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',$3,$4)")
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
    eprintln!(
        "postgres_configured_staff_support_domain_actions duration: {:?}",
        started.elapsed()
    );
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
