#![allow(clippy::unwrap_used)]

use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn input() -> Submission {
    Submission {
        request_key: Uuid::now_v7(),
        report_type: "player".into(),
        category: "harassment".into(),
        target_reference: "  Example player  ".into(),
        evidence_summary: Some("  Evidence  ".into()),
    }
}
fn policy() -> Policy {
    Policy {
        enabled: true,
        ..Policy::default()
    }
}

#[test]
fn categories_and_input_are_bounded_and_deny_unknown_values() {
    for kind in TYPES {
        for category in categories(kind) {
            let mut submission = input();
            submission.report_type = (*kind).into();
            submission.category = (*category).into();
            assert!(submission.validate().is_ok());
        }
    }
    let mut bad = input();
    bad.report_type = "unknown".into();
    assert!(bad.validate().is_err());
    bad = input();
    bad.category = "privacy".into();
    assert!(bad.validate().is_err());
    bad = input();
    bad.request_key = Uuid::nil();
    assert!(bad.validate().is_err());
    bad = input();
    bad.target_reference = " \n ".into();
    assert!(bad.validate().is_err());
    bad = input();
    bad.target_reference = "ą".repeat(160);
    assert!(bad.validate().is_ok());
    bad.target_reference.push('ą');
    assert!(bad.validate().is_err());
    bad = input();
    bad.evidence_summary = Some("a".repeat(4001));
    assert!(bad.validate().is_err());
    bad = input();
    bad.target_reference = "bad\0target".into();
    assert!(bad.validate().is_err());
    assert!(!Policy::default().enabled);
    assert!(
        !Policy {
            pending_limit: 21,
            ..policy()
        }
        .valid()
    );
    assert!(
        !Policy {
            unjustified_limit: 0,
            ..policy()
        }
        .valid()
    );
}

#[tokio::test]
async fn disabled_router_and_service_do_not_connect() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let service = Reports::new(pool.clone(), Policy::default());
    assert!(matches!(
        service.submit(Uuid::now_v7(), input()).await,
        Err(Error::Disabled)
    ));
    let response = router(pool, Policy::default())
        .oneshot(Request::get("/en/support/reports").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

struct Database {
    root: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let root = PgPool::connect(&url).await?;
        let schema = format!("reports_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(search_path))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        // Only the tables in this mechanic's public dependency seam are needed.
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105155256_t03_accounts.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105144852_t02_web_sessions.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105220234_t33c_reports.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self { root, pool, schema }))
    }
    async fn account(&self, status: &str) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO accounts (email,password_hash,locale,status) VALUES ($1,'fixture','en',$2) RETURNING id")
            .bind(format!("{}@example.invalid", Uuid::now_v7())).bind(status).fetch_one(&self.pool).await
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}

#[tokio::test]
async fn postgres_success_idempotency_privacy_and_history() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let other = db.account("active").await?;
    let service = Reports::new(db.pool.clone(), policy());
    let original = input();
    let report = service.submit(owner, original.clone()).await?;
    assert_eq!(report.target_reference, "Example player");
    assert_eq!(report.evidence_summary.as_deref(), Some("Evidence"));
    assert_eq!(report.id.get_version_num(), 7);
    let mut replay = original;
    replay.target_reference = "different".into();
    assert_eq!(service.submit(owner, replay).await?.id, report.id);
    assert!(matches!(
        service.get(other, report.id).await,
        Err(Error::NotFound)
    ));
    assert!(service.list(other, 1).await?.is_empty());
    assert!(matches!(service.list(owner, 0).await, Err(Error::Invalid)));
    sqlx::query("UPDATE player_reports SET moderator_notes='private note', public_outcome='Public outcome', status='actioned', created_at=now()-interval '31 days' WHERE id=$1").bind(report.id).execute(&db.pool).await?;
    assert!(service.list(owner, 1).await?.is_empty());
    assert_eq!(
        service
            .get(owner, report.id)
            .await?
            .public_outcome
            .as_deref(),
        Some("Public outcome")
    );
    let guild_owner = db.account("pending_verification").await?;
    let mut guild = input();
    guild.report_type = "guild".into();
    guild.category = "impersonation".into();
    service.submit(guild_owner, guild).await?;
    sqlx::query("INSERT INTO player_reports (reporter_account_id,request_key,report_type,category,target_reference,status,public_outcome) SELECT $1,uuidv7(),'content','spam','Reference ' || n,'closed','Reviewed' FROM generate_series(1,21) n")
        .bind(owner).execute(&db.pool).await?;
    assert_eq!(service.list(owner, 1).await?.len(), 20);
    assert_eq!(service.list(owner, 2).await?.len(), 1);
    db.close().await
}

#[tokio::test]
async fn postgres_refusal_bad_input_and_rejection_block_survives_closure() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let suspended = db.account("suspended").await?;
    let service = Reports::new(db.pool.clone(), policy());
    assert!(matches!(
        service.submit(suspended, input()).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.submit(Uuid::now_v7(), input()).await,
        Err(Error::Forbidden)
    ));
    let mut invalid = input();
    invalid.category = "privacy".into();
    assert!(matches!(
        service.submit(owner, invalid).await,
        Err(Error::Invalid)
    ));
    sqlx::query("DELETE FROM player_report_submission_limits WHERE account_id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await?;
    for _ in 0..3 {
        let report = service.submit(owner, input()).await?;
        sqlx::query(
            "UPDATE player_reports SET status='rejected',public_outcome='Unjustified' WHERE id=$1",
        )
        .bind(report.id)
        .execute(&db.pool)
        .await?;
        sqlx::query("UPDATE player_reports SET status='closed',rejected_at=NULL WHERE id=$1")
            .bind(report.id)
            .execute(&db.pool)
            .await?;
    }
    sqlx::query("DELETE FROM player_report_submission_limits WHERE account_id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        service.submit(owner, input()).await,
        Err(Error::UnjustifiedLimit)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_concurrent_pending_limit_and_replays() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let service = Reports::new(
        db.pool.clone(),
        Policy {
            pending_limit: 1,
            ..policy()
        },
    );
    let (a, b) = tokio::join!(
        service.submit(owner, input()),
        service.submit(owner, input())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(Error::PendingLimit)
    ));
    let other = db.account("active").await?;
    let request = input();
    let (a, b) = tokio::join!(
        service.submit(other, request.clone()),
        service.submit(other, request)
    );
    assert_eq!(a?.id, b?.id);
    db.close().await
}

#[tokio::test]
async fn postgres_rate_limit_is_persistent_and_resets() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let service = Reports::new(
        db.pool.clone(),
        Policy {
            pending_limit: 20,
            ..policy()
        },
    );
    for _ in 0..3 {
        service.submit(owner, input()).await?;
    }
    let second_process = Reports::new(db.pool.clone(), policy());
    assert!(matches!(
        second_process.submit(owner, input()).await,
        Err(Error::RateLimited)
    ));
    sqlx::query("UPDATE player_report_submission_limits SET window_started_at=now()-interval '61 seconds' WHERE account_id=$1").bind(owner).execute(&db.pool).await?;
    second_process.submit(owner, input()).await?;
    let bad_owner = db.account("active").await?;
    for _ in 0..3 {
        let mut invalid = input();
        invalid.target_reference = " ".into();
        assert!(matches!(
            service.submit(bad_owner, invalid).await,
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        second_process.submit(bad_owner, input()).await,
        Err(Error::RateLimited)
    ));
    db.close().await
}

async fn session(db: &Database, owner: Uuid, digit: char) -> Result<(String, String), sqlx::Error> {
    let token = digit.to_string().repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions (id_hash,csrf_token,account_id,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,'fixture',now()+interval '1 hour',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(&csrf).bind(owner).execute(&db.pool).await?;
    Ok((
        format!("{}={token}", oteryn_web::session::COOKIE_NAME),
        csrf,
    ))
}
async fn body(response: axum::response::Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 65536).await?.to_vec(),
    )?)
}
fn get(path: &str, cookie: &str) -> Request<Body> {
    Request::get(path)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}
fn post(path: &str, cookie: &str, csrf: &str, values: &[(&str, String)]) -> Request<Body> {
    Request::post(path)
        .header(header::COOKIE, cookie)
        .header("x-csrf-token", csrf)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(serde_urlencoded::to_string(values).unwrap()))
        .unwrap()
}
fn values() -> Vec<(&'static str, String)> {
    vec![
        ("request_key", Uuid::now_v7().to_string()),
        ("report_type", "content".into()),
        ("category", "privacy".into()),
        ("target_reference", "<script>alert(1)</script>".into()),
        ("evidence_summary", "Evidence".into()),
    ]
}

#[tokio::test]
async fn postgres_browser_flow_csrf_localization_private_output_and_bad_input() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let other = db.account("active").await?;
    let (cookie, csrf) = session(&db, owner, 'a').await?;
    let (other_cookie, _) = session(&db, other, 'c').await?;
    let app: Router = router(db.pool.clone(), policy());
    let response = app
        .clone()
        .oneshot(Request::get("/en/support/reports").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    for (lang, title) in [("pl", "Zgłoszenia"), ("en", "Reports")] {
        let response = app
            .clone()
            .oneshot(get(&format!("/{lang}/support/reports/create"), &cookie))
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let html = body(response).await?;
        assert!(html.contains(title));
        assert!(html.contains("name=\"_csrf\""));
        assert!(html.contains("for=\"evidence_summary\""));
        assert!(html.contains("width=device-width"));
        std::fs::write(format!("/tmp/reports-{lang}.html"), html)?;
    }
    let values = values();
    let response = app
        .clone()
        .oneshot(post("/en/support/reports", &cookie, "wrong", &values))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let mut cross_site = post("/en/support/reports", &cookie, &csrf, &values);
    cross_site
        .headers_mut()
        .insert(header::ORIGIN, "https://untrusted.invalid".parse()?);
    assert_eq!(
        app.clone().oneshot(cross_site).await?.status(),
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(post("/en/support/reports", &cookie, &csrf, &values))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response.headers()[header::LOCATION].to_str()?.to_owned();
    sqlx::query("UPDATE player_reports SET moderator_notes='Hidden moderation note',public_outcome='Public outcome',status='actioned' WHERE reporter_account_id=$1").bind(owner).execute(&db.pool).await?;
    let response = app.clone().oneshot(get(&location, &cookie)).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = body(response).await?;
    assert!(html.contains("&#60;script&#62;") || html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("Hidden moderation note"));
    assert!(html.contains("Public outcome"));
    assert_eq!(
        app.clone()
            .oneshot(get(&location, &other_cookie))
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let mut invalid = values.clone();
    invalid[2].1 = "cheating".into();
    let response = app
        .clone()
        .oneshot(post("/en/support/reports", &cookie, &csrf, &invalid))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let html = body(
        app.clone()
            .oneshot(get("/en/support/reports/create", &cookie))
            .await?,
    )
    .await?;
    assert!(html.contains("Check the report type"));
    assert!(html.contains("role=\"alert\""));
    assert_eq!(
        app.clone()
            .oneshot(get("/de/support/reports", &cookie))
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let malformed = post(
        "/en/support/reports",
        &cookie,
        &csrf,
        &[("request_key", "not-a-uuid".into())],
    );
    assert_eq!(
        app.clone().oneshot(malformed).await?.status(),
        StatusCode::FOUND
    );
    let response = app
        .clone()
        .oneshot(post("/en/support/reports", &cookie, &csrf, &values))
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    db.close().await
}

#[tokio::test]
async fn postgres_browser_configured_origin_still_requires_csrf() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let owner = db.account("active").await?;
    let (cookie, csrf) = session(&db, owner, 'a').await?;
    let origin = "https://portal.local.test";
    let app = super::web::router_with_origin(db.pool.clone(), policy(), Some(origin.into()));
    let fields = values();
    let response = app
        .clone()
        .oneshot(get("/en/support/reports/create", &cookie))
        .await?;
    assert_eq!(response.headers()[header::REFERRER_POLICY], "same-origin");
    for (source, token, status) in [
        (origin, "", StatusCode::FORBIDDEN),
        ("null", csrf.as_str(), StatusCode::FORBIDDEN),
        (
            "https://untrusted.invalid",
            csrf.as_str(),
            StatusCode::FORBIDDEN,
        ),
        (origin, csrf.as_str(), StatusCode::FOUND),
    ] {
        let mut request = post("/en/support/reports", &cookie, token, &fields);
        request
            .headers_mut()
            .insert(header::ORIGIN, source.parse()?);
        assert_eq!(app.clone().oneshot(request).await?.status(), status);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM player_reports WHERE reporter_account_id=$1")
            .bind(owner)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    db.close().await
}
