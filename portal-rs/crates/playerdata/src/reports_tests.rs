use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
const START: i64 = 1792972800;
const NOW: i64 = START + 604800 + 60;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x019a5000000070008000000000000000 + n)
}
fn snapshot(owner: Uuid) -> Snapshot {
    Snapshot {
        account_id: owner,
        covered_start: START,
        covered_end: START + 604800,
        complete: true,
        sessions: vec![Session {
            id: id(3),
            character_id: id(4),
            character_revision: 2,
            started_at: START + 10,
            ended_at: START + 70,
            duration_seconds: 60,
            experience_gained: "9223372036854775807".into(),
            experience_lost: "1".into(),
        }],
        drops: vec![Drop {
            event_id: id(5),
            session_id: id(3),
            occurred_at: START + 30,
            item_key: "item:ruby".into(),
            quantity: "2".into(),
        }],
    }
}
#[derive(Debug)]
struct Source {
    owner: Uuid,
    deny: std::sync::atomic::AtomicBool,
}
impl JournalSource for Source {
    fn snapshot(&self, account: Uuid, week: Week) -> SourceFuture<'_, Snapshot> {
        Box::pin(async move {
            if account != self.owner || self.deny.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Error::Denied);
            }
            let mut data = snapshot(account);
            let delta = week.start - START;
            data.covered_start = week.start;
            data.covered_end = week.end;
            for session in &mut data.sessions {
                session.started_at += delta;
                session.ended_at += delta;
            }
            for drop in &mut data.drops {
                drop.occurred_at += delta;
            }
            Ok(data)
        })
    }
    fn authorize(&self, account: Uuid, target: Target) -> SourceFuture<'_, ()> {
        Box::pin(async move {
            if account == self.owner
                && ((target.kind == TargetKind::Session && target.id == id(3))
                    || (target.kind == TargetKind::Character && target.id == id(4)))
                && !self.deny.load(std::sync::atomic::Ordering::SeqCst)
            {
                Ok(())
            } else {
                Err(Error::Denied)
            }
        })
    }
}
#[test]
fn weekly_success_and_invalid_evidence() -> TestResult {
    let week = Week::new(START, NOW)?;
    let report = build(id(1), week, snapshot(id(1)))?;
    assert_eq!(report.session_count, 1);
    assert_eq!(report.experience_gained, "9223372036854775807");
    assert_eq!(report.drops.len(), 1);
    assert!(serde_json::to_string(&report)?.contains("\"9223372036854775807\""));
    assert!(csv(&report, false).contains("\r\n"));
    assert!(csv(&report, true).contains("item:ruby"));
    let mut dangerous = build(id(1), week, snapshot(id(1)))?;
    dangerous.drops[0].item_key = "=SUM(1,2)\"".into();
    assert!(csv(&dangerous, true).contains("\"'=SUM(1,2)\"\"\""));
    assert!(build(id(2), week, snapshot(id(1))).is_err());
    let mut data = snapshot(id(1));
    data.complete = false;
    assert!(build(id(1), week, data).is_err());
    let mut data = snapshot(id(1));
    data.sessions[0].experience_gained = "01".into();
    assert!(build(id(1), week, data).is_err());
    let mut data = snapshot(id(1));
    data.sessions.push(data.sessions[0].clone());
    assert!(build(id(1), week, data).is_err());
    let mut data = snapshot(id(1));
    data.drops[0].session_id = id(8);
    assert!(build(id(1), week, data).is_err());
    let mut data = snapshot(id(1));
    data.drops[0].item_key = "=formula".into();
    assert!(build(id(1), week, data).is_err());
    let mut data = snapshot(id(1));
    data.sessions[0].duration_seconds = 0;
    assert_eq!(build(id(1), week, data)?.duration_seconds, "0");
    assert!(Week::new(START + 1, NOW).is_err());
    assert!(Week::new(START, START + 1).is_err());
    assert!(Week::new(START, NOW + 100 * 86400).is_err());
    Ok(())
}
#[test]
fn localized_pages_escape_notes() -> TestResult {
    for locale in [Locale::Pl, Locale::En] {
        let html = render::note(locale, "token", "<script>bad</script>")?;
        assert!(!html.contains("<script>"));
        assert!(html.contains("aria-describedby"));
        assert!(html.contains("width=device-width"));
        assert!(html.contains("for=\"note\""));
        if let Ok(directory) = std::env::var("PORTAL_REPORTS_RENDER_DIR") {
            std::fs::create_dir_all(&directory)?;
            std::fs::write(format!("{directory}/note-{}.html", locale.code()), &html)?;
        }
        let report = build(id(1), Week::new(START, NOW)?, snapshot(id(1)))?;
        assert!(!render::weekly(locale, &report, "token", false)?.contains("604800"));
        if let Ok(directory) = std::env::var("PORTAL_REPORTS_RENDER_DIR") {
            std::fs::write(
                format!("{directory}/week-{}.html", locale.code()),
                render::weekly(locale, &report, "token", false)?,
            )?;
        }
    }
    Ok(())
}
#[tokio::test]
async fn default_off_and_missing_source_deny() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let service = Reports {
        pool,
        source: Arc::new(DenySource),
        flags: Flags::default(),
    };
    assert!(matches!(
        service.weekly(id(1), START, NOW).await,
        Err(Error::Disabled)
    ));
    let response = http::router(service.clone())
        .oneshot(Request::get("/en/account/journal/reports?week=1792972800").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let service = Reports {
        flags: Flags {
            enabled: true,
            weekly_email: false,
        },
        ..service
    };
    assert!(service.weekly(id(1), START, NOW).await.is_err());
    Ok(())
}
struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> std::result::Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T82 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("reports_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        for (account, email) in [
            (id(1), "reports-owner@example.invalid"),
            (id(2), "reports-other@example.invalid"),
        ] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'fixture','active','en',now())")
                .bind(account).bind(email).execute(&pool).await?;
        }
        use sha2::{Digest, Sha256};
        for (account, token) in [(id(1), "a".repeat(64)), (id(2), "b".repeat(64))] {
            sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'fixture',now()+interval '1 hour',now()+interval '1 day')")
                .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind("c".repeat(64)).execute(&pool).await?;
        }
        Ok(Some(Self {
            admin,
            pool,
            schema,
        }))
    }
    fn service(&self) -> (Reports, Arc<Source>) {
        let source = Arc::new(Source {
            owner: id(1),
            deny: false.into(),
        });
        (
            Reports {
                pool: self.pool.clone(),
                source: source.clone(),
                flags: Flags {
                    enabled: true,
                    weekly_email: true,
                },
            },
            source,
        )
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        Ok(())
    }
}
#[tokio::test]
async fn postgres_notes_owner_refusal_bad_input_and_erasure() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (service, source) = db.service();
    let target = Target {
        kind: TargetKind::Session,
        id: id(3),
    };
    service.save_note(id(1), target, "private <text>").await?;
    assert_eq!(
        service.note(id(1), target).await?,
        Some("private <text>".into())
    );
    assert!(service.note(id(2), target).await.is_err());
    assert!(service.save_note(id(2), target, "overwrite").await.is_err());
    assert!(
        service
            .save_note(id(1), target, &"x".repeat(4001))
            .await
            .is_err()
    );
    assert!(service.save_note(id(1), target, "bad\0").await.is_err());
    assert_eq!(service.export_notes(id(1)).await?.len(), 1);
    assert!(service.export_notes(id(2)).await?.is_empty());
    source.deny.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(service.note(id(1), target).await.is_err());
    assert!(service.export_notes(id(1)).await?.is_empty());
    source
        .deny
        .store(false, std::sync::atomic::Ordering::SeqCst);
    service.save_note(id(1), target, "").await?;
    assert!(service.note(id(1), target).await?.is_none());
    service
        .save_note(
            id(1),
            Target {
                kind: TargetKind::Character,
                id: id(4),
            },
            "character note",
        )
        .await?;
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id(1))
        .execute(&db.pool)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_report_notes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[tokio::test]
async fn postgres_http_owner_csrf_and_downloads() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (service, _) = db.service();
    let app = http::router(service);
    let path = format!(
        "/en/account/journal/reports/notes?kind=session&id={}",
        id(3)
    );
    for (token, body, status) in [
        (
            "a",
            format!("csrf={}&body=hello", "c".repeat(64)),
            StatusCode::NO_CONTENT,
        ),
        (
            "b",
            format!("csrf={}&body=stolen", "c".repeat(64)),
            StatusCode::NOT_FOUND,
        ),
        ("a", "csrf=wrong&body=bad".into(), StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(&path)
                    .header(
                        header::COOKIE,
                        format!("__Host-oteryn_session={}", token.repeat(64)),
                    )
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
    }
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(path.replace("/en/", &format!("/{lang}/")))
                    .header(
                        header::COOKIE,
                        format!("__Host-oteryn_session={}", "a".repeat(64)),
                    )
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?
                .contains("hello")
        );
    }
    let response = app
        .clone()
        .oneshot(
            Request::get("/en/account/journal/reports/notes/export")
                .header(
                    header::COOKIE,
                    format!("__Host-oteryn_session={}", "a".repeat(64)),
                )
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::CONTENT_DISPOSITION));
    let current = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    let week = current - (current - 345600).rem_euclid(604800) - 604800;
    for format in ["html", "json", "csv"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/en/account/journal/reports?week={week}&format={format}"
                ))
                .header(
                    header::COOKIE,
                    format!("__Host-oteryn_session={}", "a".repeat(64)),
                )
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        if format != "html" {
            assert!(response.headers().contains_key(header::CONTENT_DISPOSITION));
        }
        let body = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(body.contains("9223372036854775807"));
    }
    let response = app
        .oneshot(Request::get(&path).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await
}
#[derive(Debug)]
struct Accounts;
impl oteryn_notify::AccountState for Accounts {
    fn active_verified(
        &self,
        _: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async {
            Ok(Some(oteryn_notify::Account {
                email: "reports-owner@example.invalid".into(),
                locale: oteryn_notify::Locale::En,
            }))
        })
    }
}
#[tokio::test]
async fn postgres_optional_email_deduplicates_and_denies_unknown_owner() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (service, source) = db.service();
    let notify = oteryn_notify::Notifications {
        pool: db.pool.clone(),
        accounts: Arc::new(Accounts),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![7; 32],
            "https://portal.example.invalid/unsubscribe".into(),
        )
        .map_err(|e| e.to_string())?,
    };
    assert!(
        !service
            .email_week(id(1), START, NOW, Locale::En, &notify)
            .await?
    );
    service.email_preference(id(1), true).await?;
    assert!(
        !service
            .email_week(id(1), START, NOW, Locale::En, &notify)
            .await?
    );
    sqlx::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,'news',true,false)")
        .bind(id(1)).execute(&db.pool).await?;
    assert!(
        service
            .email_week(id(1), START, NOW, Locale::En, &notify)
            .await?
    );
    assert!(
        !service
            .email_week(id(1), START, NOW, Locale::En, &notify)
            .await?
    );
    let (count, body): (i64, String) =
        sqlx::query_as("SELECT count(*) OVER(),text_body FROM email_outbox WHERE account_id=$1")
            .bind(id(1))
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    assert!(!body.contains("9223372036854775807"));
    assert!(!body.contains("item:ruby"));
    source.deny.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(
        service
            .email_week(id(1), START, NOW, Locale::En, &notify)
            .await
            .is_err()
    );
    db.close().await
}
