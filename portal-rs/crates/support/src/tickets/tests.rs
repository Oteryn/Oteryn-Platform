use super::*;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
type TestResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> std::result::Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL support test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("support_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = format!("SET search_path TO {schema}");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(path)).execute(conn).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        for migration in [
            include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
            include_str!("../../../../migrations/20261105144852_t02_web_sessions.sql"),
            include_str!("../../../../migrations/20261105120000_t37_outbox.sql"),
            include_str!("../../../../migrations/20261105130000_t36_notifications.sql"),
            include_str!("../../../../migrations/20261105160300_t04a_rate_limits.sql"),
            include_str!("../../../../migrations/20261105170000_t10_rbac_audit.sql"),
            include_str!("../../../../migrations/20261105220218_t33_tickets.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn account(&self, locale: &str) -> std::result::Result<Uuid, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let account = oteryn_identity::accounts::Accounts::create(
            &mut conn,
            &format!("{}@example.test", Uuid::now_v7()),
            "unused-test-hash",
            locale,
        )
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
        oteryn_identity::accounts::Accounts::mark_verified(&mut conn, account.id).await?;
        Ok(account.id)
    }
    fn tickets(&self, limit: u32) -> oteryn_notify::Result<Tickets> {
        Ok(Tickets {
            pool: self.pool.clone(),
            notifications: Arc::new(Notifications {
                pool: self.pool.clone(),
                accounts: Arc::new(web::NotifyAccounts(self.pool.clone())),
                unsubscribe: oteryn_notify::Unsubscribe::new(
                    vec![7; 32],
                    "https://portal.example.test/unsubscribe".into(),
                )?,
            }),
            enabled: true,
            open_limit: limit,
        })
    }
    async fn finish(self) -> TestResult {
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
    async fn cookie(&self, account: Uuid) -> std::result::Result<(String, String), sqlx::Error> {
        let token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
        let csrf = "a".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind(&csrf).execute(&self.pool).await?;
        Ok((
            format!("{}={token}", oteryn_web::session::COOKIE_NAME),
            csrf,
        ))
    }
}
fn staff(id: Uuid) -> Option<CheckContext> {
    Some(CheckContext {
        account: id,
        require_staff_mfa: true,
        mfa_verified: true,
    })
}

#[tokio::test]
async fn postgres_creation_idempotency_limit_and_validation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("en").await?;
    let t = db.tickets(1)?;
    let key = Uuid::now_v7();
    for (category, subject, body) in [
        ("unknown", "subject", "body"),
        ("account", " ", "body"),
        ("account", "subject", ""),
    ] {
        assert!(matches!(
            t.create(account, Uuid::now_v7(), category, subject, body)
                .await,
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        t.create(account, Uuid::nil(), "other", "subject", "body")
            .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        t.create(account, Uuid::now_v7(), "other", &"x".repeat(161), "body")
            .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        t.create(
            account,
            Uuid::now_v7(),
            "other",
            "subject",
            &"x".repeat(8001)
        )
        .await,
        Err(Error::Invalid)
    ));
    let ticket = t
        .create(account, key, "technical", " Subject ", " Body ")
        .await?;
    assert_eq!(ticket.subject, "Subject");
    assert_eq!(ticket.lock_version, 1);
    assert_eq!(
        t.create(account, key, "technical", "different", "different")
            .await?
            .id,
        ticket.id
    );
    assert!(matches!(
        t.create(account, Uuid::now_v7(), "other", "subject", "body")
            .await,
        Err(Error::Conflict)
    ));
    let (_, messages) = t.show(account, false, ticket.id).await?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].body, "Body");
    let mut off = t.clone();
    off.enabled = false;
    assert!(matches!(
        off.create(account, key, "other", "subject", "body").await,
        Err(Error::Denied)
    ));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        t.create(account, key, "other", "subject", "body").await,
        Err(Error::Denied)
    ));
    db.finish().await
}
#[tokio::test]
async fn postgres_concurrent_creation_and_reopen_share_limit() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("en").await?;
    let t = db.tickets(1)?;
    let (a, b) = tokio::join!(
        t.create(account, Uuid::now_v7(), "other", "A", "body"),
        t.create(account, Uuid::now_v7(), "other", "B", "body")
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let ticket = a.or(b)?;
    let closed = t
        .change(account, None, ticket.id, 1, Change::Status("closed"))
        .await?;
    let (a, b) = tokio::join!(
        t.create(account, Uuid::now_v7(), "other", "C", "body"),
        t.change(
            account,
            None,
            ticket.id,
            closed.lock_version,
            Change::Status("open")
        )
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tickets WHERE account_id=$1 AND status NOT IN ('resolved','closed')",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 1);
    db.finish().await
}
#[tokio::test]
async fn postgres_replies_status_ownership_and_internal_privacy() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("pl").await?;
    let actor = db.account("en").await?;
    let t = db.tickets(10)?;
    let ticket = t
        .create(owner, Uuid::now_v7(), "account", "Help", "body")
        .await?;
    assert!(matches!(
        t.show(actor, false, ticket.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        t.change(actor, None, ticket.id, 1, Change::Status("closed"))
            .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        t.change(owner, None, ticket.id, 1, Change::Status("unknown"))
            .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        t.change(
            owner,
            None,
            ticket.id,
            1,
            Change::Reply {
                body: " ",
                internal: false
            }
        )
        .await,
        Err(Error::Invalid)
    ));
    let note = t
        .change(
            actor,
            staff(actor),
            ticket.id,
            1,
            Change::Reply {
                body: "private",
                internal: true,
            },
        )
        .await?;
    assert_eq!(note.status, "open");
    assert_eq!(t.show(owner, false, ticket.id).await?.1.len(), 1);
    assert_eq!(t.show(actor, true, ticket.id).await?.1.len(), 2);
    let reply = t
        .change(
            actor,
            staff(actor),
            ticket.id,
            2,
            Change::Reply {
                body: "public",
                internal: false,
            },
        )
        .await?;
    assert_eq!(reply.status, "waiting_user");
    assert!(reply.unread_reply);
    assert!(matches!(
        t.change(
            owner,
            None,
            ticket.id,
            2,
            Change::Reply {
                body: "stale",
                internal: false
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    t.show(owner, false, ticket.id).await?;
    assert!(!t.list(owner, false, 0).await?[0].unread_reply);
    let reply = t
        .change(
            owner,
            None,
            ticket.id,
            3,
            Change::Reply {
                body: "answer",
                internal: false,
            },
        )
        .await?;
    assert_eq!(reply.status, "waiting_staff");
    assert!(matches!(
        t.change(
            owner,
            None,
            ticket.id,
            4,
            Change::Reply {
                body: "repeat",
                internal: false
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    t.change(
        actor,
        staff(actor),
        ticket.id,
        4,
        Change::Status("resolved"),
    )
    .await?;
    assert!(matches!(
        t.change(
            actor,
            staff(actor),
            ticket.id,
            5,
            Change::Reply {
                body: "late",
                internal: false
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        t.change(owner, None, ticket.id, 5, Change::Status("open"))
            .await,
        Err(Error::Conflict)
    ));
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(audits, 3);
    let notices: Vec<String> =
        sqlx::query_scalar("SELECT body FROM notifications ORDER BY created_at,id")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(notices.len(), 2);
    assert!(notices[0].contains("Pomoc odpowiedziała"));
    assert!(!notices.iter().any(|s| s.contains("private")));
    db.finish().await
}
#[tokio::test]
async fn postgres_notification_failure_rolls_back_and_preferences_apply() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let actor = db.account("en").await?;
    let mut t = db.tickets(10)?;
    let ticket = t
        .create(owner, Uuid::now_v7(), "payment", "Help", "body")
        .await?;
    t.notifications = Arc::new(Notifications {
        pool: db.pool.clone(),
        accounts: Arc::new(oteryn_notify::DenyAccounts),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![7; 32],
            "https://portal.example.test/unsubscribe".into(),
        )?,
    });
    assert!(matches!(
        t.change(
            actor,
            staff(actor),
            ticket.id,
            1,
            Change::Reply {
                body: "answer",
                internal: false
            }
        )
        .await,
        Err(Error::Notification)
    ));
    assert_eq!(t.list(owner, false, 0).await?[0].lock_version, 1);
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(audits, 0);
    let t = db.tickets(10)?;
    sqlx::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,'support',true,false)").bind(owner).execute(&db.pool).await?;
    t.change(
        actor,
        staff(actor),
        ticket.id,
        1,
        Change::Status("waiting_user"),
    )
    .await?;
    let emails: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(emails, 1);
    let notices: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(notices, 0);
    db.finish().await
}
#[tokio::test]
async fn postgres_retention_only_removes_old_terminal_tickets() -> TestResult {
    use oteryn_db::outbox::Cleanup;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let t = db.tickets(10)?;
    let old = t
        .create(owner, Uuid::now_v7(), "other", "old", "body")
        .await?;
    let live = t
        .create(owner, Uuid::now_v7(), "other", "live", "body")
        .await?;
    t.change(owner, None, old.id, 1, Change::Status("closed"))
        .await?;
    sqlx::query("UPDATE tickets SET closed_at=now()-interval '181 days' WHERE id=$1")
        .bind(old.id)
        .execute(&db.pool)
        .await?;
    Retention {
        pool: db.pool.clone(),
        enabled: false,
    }
    .clean()
    .await
    .map_err(std::io::Error::other)?;
    assert!(t.show(owner, false, old.id).await.is_ok());
    Retention {
        pool: db.pool.clone(),
        enabled: true,
    }
    .clean()
    .await
    .map_err(std::io::Error::other)?;
    assert!(matches!(
        t.show(owner, false, old.id).await,
        Err(Error::NotFound)
    ));
    assert!(t.show(owner, false, live.id).await.is_ok());
    let messages: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ticket_messages WHERE ticket_id=$1")
            .bind(old.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(messages, 0);
    db.finish().await
}
#[tokio::test]
async fn postgres_player_routes_session_csrf_bad_input_and_success() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let t = db.tickets(1)?;
    let app = router(t.clone(), "http://localhost:3000");
    let (cookie, csrf) = db.cookie(owner).await?;
    let response = app
        .clone()
        .oneshot(Request::get("/en/support/tickets").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            Request::get("/pl/support/tickets/create")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Utwórz zgłoszenie"));
    assert!(html.contains("name=\"_csrf\""));
    for (body, status) in [
        ("body=x".to_owned(), StatusCode::FORBIDDEN),
        (
            format!(
                "_csrf={csrf}&request_key={}&category=unknown&subject=x&body=x",
                Uuid::now_v7()
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!(
                "_csrf={csrf}&request_key={}&category=other&subject=x&body=x",
                Uuid::now_v7()
            ),
            StatusCode::FOUND,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/en/support/tickets")
                    .header("cookie", &cookie)
                    .header("origin", "http://localhost:3000")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .extension(ConnectInfo(
                        "127.0.0.1:3210".parse::<std::net::SocketAddr>()?,
                    ))
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let draft_key = Uuid::now_v7();
    let response=app.clone().oneshot(Request::post("/en/support/tickets").header("cookie",&cookie).header("content-type","application/x-www-form-urlencoded").extension(ConnectInfo("127.0.0.1:3210".parse::<std::net::SocketAddr>()?)).body(Body::from(format!("_csrf={csrf}&request_key={draft_key}&category=account&subject=Saved+draft&body=Preserved+message")))?).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = app
        .clone()
        .oneshot(
            Request::get("/en/support/tickets/create")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Saved draft"));
    assert!(html.contains("Preserved message"));
    assert!(html.contains(&draft_key.to_string()));
    assert!(html.contains("role=\"alert\""));
    let ticket = t.list(owner, false, 0).await?.remove(0);
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/en/support/tickets/{}", ticket.id))
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .oneshot(
            Request::put(format!("/en/support/tickets/{}/status", ticket.id))
                .header("cookie", &cookie)
                .header("origin", "http://localhost:3000")
                .header("content-type", "application/x-www-form-urlencoded")
                .extension(ConnectInfo(
                    "127.0.0.1:3210".parse::<std::net::SocketAddr>()?,
                ))
                .body(Body::from(format!(
                    "_csrf={csrf}&lock_version=1&status=closed"
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(t.list(owner, false, 0).await?[0].status, "closed");
    db.finish().await
}
#[test]
fn all_user_text_is_localized_and_html_is_escaped() -> TestResult {
    for language in ["pl", "en"] {
        for key in CATEGORIES.iter().chain(STATUSES).copied().chain([
            "notification-reply",
            "notification-status",
            "error",
        ]) {
            assert!(!text::text(language, key).is_empty());
        }
        let html = web::test_render(language)?;
        assert!(!html.contains("<script>"));
        assert!(html.contains("width=device-width"));
        assert!(html.contains("op-input"));
    }
    Ok(())
}

#[derive(Clone)]
struct StaffAdapter {
    mfa: bool,
    state: oteryn_admin::rbac::AccountState,
}
impl oteryn_admin::web::StaffSession for StaffAdapter {
    async fn resolve(
        &self,
        session: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        session
            .account_id()
            .await
            .map(|account| oteryn_admin::web::StaffIdentity {
                account,
                mfa_satisfied: self.mfa,
            })
    }
}
impl oteryn_admin::rbac::AccountStateReader for StaffAdapter {
    async fn state(
        &self,
        _: Uuid,
    ) -> std::result::Result<
        oteryn_admin::rbac::AccountState,
        oteryn_admin::rbac::AccountStateUnavailable,
    > {
        Ok(self.state)
    }
}
#[tokio::test]
async fn postgres_staff_routes_require_host_mfa_permission_and_csrf() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let actor = db.account("en").await?;
    let t = db.tickets(10)?;
    let ticket = t
        .create(owner, Uuid::now_v7(), "other", "Help", "body")
        .await?;
    let (cookie, csrf) = db.cookie(actor).await?;
    let routes = |mfa, state| {
        let adapter = StaffAdapter { mfa, state };
        staff_router(
            t.clone(),
            oteryn_admin::web::PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            adapter.clone(),
            adapter,
        )
    };
    let path = format!("/en/admin/support/tickets/{}", ticket.id);
    for (mfa, state, host, expected) in [
        (
            true,
            oteryn_admin::rbac::AccountState::Active,
            "wrong.test",
            StatusCode::NOT_FOUND,
        ),
        (
            false,
            oteryn_admin::rbac::AccountState::Active,
            "staff.test",
            StatusCode::FORBIDDEN,
        ),
        (
            true,
            oteryn_admin::rbac::AccountState::Unknown,
            "staff.test",
            StatusCode::FORBIDDEN,
        ),
        (
            true,
            oteryn_admin::rbac::AccountState::Active,
            "staff.test",
            StatusCode::FORBIDDEN,
        ),
    ] {
        let response = routes(mfa, state)
            .oneshot(
                Request::get(&path)
                    .header("host", host)
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await?;
    let app = routes(true, oteryn_admin::rbac::AccountState::Active);
    let response = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    for (body, expected) in [
        ("body=answer&lock_version=1".into(), StatusCode::FORBIDDEN),
        (
            format!("_csrf={csrf}&body=answer&lock_version=0"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!("_csrf={csrf}&body=answer&lock_version=1"),
            StatusCode::FOUND,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(format!("{path}/reply"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .extension(ConnectInfo(
                        "127.0.0.1:3210".parse::<std::net::SocketAddr>()?,
                    ))
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    assert_eq!(t.list(owner, false, 0).await?[0].status, "waiting_user");
    db.finish().await
}
