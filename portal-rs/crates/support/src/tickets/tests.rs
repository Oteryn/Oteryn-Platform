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
        Self::with_connections(8).await
    }
    async fn with_connections(
        maximum: u32,
    ) -> std::result::Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
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
            .max_connections(maximum)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(path))
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("SELECT set_config('application_name', current_schema(), false)")
                        .execute(conn)
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
        let cookie = format!("{}={token}", oteryn_web::session::COOKIE_NAME);
        crate::test_staff::browser(&self.pool, account, &cookie).await?;
        Ok((cookie, csrf))
    }
}
async fn staff(pool: &PgPool, id: Uuid) -> Option<CheckContext> {
    Some(CheckContext {
        account: id,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: crate::test_staff::proof(pool, id).await.ok(),
    })
}

#[tokio::test]
async fn postgres_creation_idempotency_limit_and_validation() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    assert!(matches!(
        t.create(account, key, "technical", "different", "different")
            .await,
        Err(Error::RequestKeyConflict)
    ));
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("pl").await?;
    let actor = db.account("en").await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await?;
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
            staff(&db.pool, actor).await,
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
            staff(&db.pool, actor).await,
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
        staff(&db.pool, actor).await,
        ticket.id,
        4,
        Change::Status("resolved"),
    )
    .await?;
    assert!(matches!(
        t.change(
            actor,
            staff(&db.pool, actor).await,
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let actor = db.account("en").await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await?;
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
            staff(&db.pool, actor).await,
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
        staff(&db.pool, actor).await,
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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

#[tokio::test]
async fn postgres_message_pages_and_payload_binding() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("en").await?;
    let service = db.tickets(10)?;
    let key = Uuid::now_v7();
    let ticket = service
        .create(account, key, "technical", " Subject ", " Body ")
        .await?;
    assert_eq!(
        service
            .create(account, key, "technical", "Subject", "Body")
            .await?
            .id,
        ticket.id
    );
    sqlx::query("INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,internal,body) SELECT $1,$2,'user',false,'Message '||n FROM generate_series(1,105) n").bind(ticket.id).bind(account).execute(&db.pool).await?;
    let mut after = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let (_, messages) = service.show_page(account, false, ticket.id, after).await?;
        assert!(messages.len() <= 50);
        if messages.is_empty() {
            break;
        }
        for message in &messages {
            assert!(seen.insert(message.id));
        }
        after = messages.last().map(|m| m.id);
    }
    assert_eq!(seen.len(), 106);
    for (category, subject, body) in [
        ("other", "Subject", "Body"),
        ("technical", "Changed", "Body"),
        ("technical", "Subject", "Changed"),
    ] {
        assert!(matches!(
            service.create(account, key, category, subject, body).await,
            Err(Error::RequestKeyConflict)
        ));
    }

    db.finish().await
}

#[tokio::test]
async fn postgres_ticket_status_with_exhausted_pool() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("en").await?;
    let tickets = db.tickets(4)?;
    let ticket = tickets
        .create(account, Uuid::now_v7(), "other", "subject", "body")
        .await?;
    // Leave precisely one connection for the mutation's transaction.
    let mut held = Vec::new();
    for _ in 0..7 {
        held.push(db.pool.acquire().await?);
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tickets.change(
            account,
            None,
            ticket.id,
            ticket.lock_version,
            Change::Status("closed"),
        ),
    )
    .await??;
    drop(held);
    db.finish().await?;
    eprintln!(
        "postgres_ticket_status_with_exhausted_pool duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_notification_transaction_with_exhausted_pool() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let Some(db) = Database::with_connections(1).await? else {
        return Ok(());
    };
    let account = db.account("en").await?;
    let tickets = db.tickets(4)?;
    let mut tx = db.pool.begin().await?;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tickets
            .notifications
            .enqueue(&mut tx, account, &EmailTemplate::PasswordChanged)
            .await?;
        tickets
            .notifications
            .preferences(&mut tx, account, oteryn_notify::Category::News, true, true)
            .await?;
        let token = tickets.notifications.unsubscribe.token(
            account,
            oteryn_notify::Category::News,
            time::OffsetDateTime::now_utc().unix_timestamp() + 60,
        )?;
        tickets.notifications.unsubscribe(&mut tx, &token).await?;
        oteryn_notify::Result::Ok(())
    })
    .await??;
    tx.commit().await?;
    db.finish().await?;
    eprintln!(
        "postgres_notification_transaction_with_exhausted_pool duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn staff_waiting_mutation_rechecks_authority(
    name: &str,
    method: &str,
    action: &str,
    revocation_kind: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let actor = db.account("en").await?;
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('support_agent') RETURNING id")
            .fetch_one(&db.pool)
            .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access'),($1,'support.tickets.manage')").bind(role).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(&db.pool)
        .await?;
    let tickets = db.tickets(10)?;
    let ticket = tickets
        .create(owner, Uuid::now_v7(), "other", "Help", "body")
        .await?;
    let (cookie, csrf) = db.cookie(actor).await?;
    let adapter = StaffAdapter {
        mfa: true,
        state: oteryn_admin::rbac::AccountState::Active,
    };
    let app = staff_router(
        tickets,
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        adapter.clone(),
        adapter,
    );
    let mut owner_lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .fetch_one(&mut *owner_lock)
        .await?;
    let (surface, fields) = match action {
        "reply" => ("reply", "body=answer&internal=false"),
        "note" => ("reply", "body=private&internal=true"),
        _ => ("status", "status=closed"),
    };
    let request = Request::builder()
        .method(method)
        .uri(format!("/en/admin/support/tickets/{}/{surface}", ticket.id))
        .header("host", "staff.test")
        .header("cookie", cookie)
        .header("content-type", "application/x-www-form-urlencoded")
        .extension(ConnectInfo(
            "127.0.0.1:3210".parse::<std::net::SocketAddr>()?,
        ))
        .body(Body::from(format!("_csrf={csrf}&lock_version=1&{fields}")))?;
    let pending = tokio::spawn(async move { app.oneshot(request).await });
    // Observe the request waiting at the owner's account lock, after its
    // shared guard and pre-lock actor read have completed.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock' AND query='SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE')").bind(&db.schema).fetch_one(&db.pool).await?;
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<(), sqlx::Error>(())
    }).await??;
    let mut revocation = db.pool.begin().await?;
    oteryn_admin::rbac::lock_mutation(&mut revocation).await?;
    if revocation_kind == "suspend" {
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(actor)
            .execute(&mut *revocation)
            .await?;
    } else if revocation_kind == "role" {
        sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
            .bind(actor)
            .execute(&mut *revocation)
            .await?;
    }
    if matches!(revocation_kind, "admin.access" | "support.tickets.manage") {
        sqlx::query("DELETE FROM role_permissions WHERE role_id=$1 AND permission=$2")
            .bind(role)
            .bind(revocation_kind)
            .execute(&mut *revocation)
            .await?;
    }
    revocation.commit().await?;
    owner_lock.commit().await?;
    let response = tokio::time::timeout(std::time::Duration::from_secs(3), pending).await??;
    let response = match response {
        Ok(response) => response,
        Err(error) => match error {},
    };
    assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name}");
    let unchanged: (String, i64, bool) =
        sqlx::query_as("SELECT status,lock_version,unread_reply FROM tickets WHERE id=$1")
            .bind(ticket.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(unchanged, ("open".into(), 1, false));
    let messages: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ticket_messages WHERE ticket_id=$1")
            .bind(ticket.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(messages, 1);
    for table in ["audit_log", "notifications", "email_outbox", "outbox"] {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(count, 0, "{name}: {table}");
    }
    db.finish().await?;
    eprintln!("{name} duration={:?}", started.elapsed());
    Ok(())
}
macro_rules! waiting_staff_test {
    ($name:ident, $method:literal, $action:literal, $revocation:literal) => {
        #[tokio::test]
        async fn $name() -> TestResult {
            let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
            staff_waiting_mutation_rechecks_authority(
                stringify!($name),
                $method,
                $action,
                $revocation,
            )
            .await
        }
    };
}
waiting_staff_test!(postgres_staff_reply_rechecks_role, "POST", "reply", "role");
waiting_staff_test!(postgres_staff_note_rechecks_role, "POST", "note", "role");
waiting_staff_test!(
    postgres_staff_post_status_rechecks_role,
    "POST",
    "status",
    "role"
);
waiting_staff_test!(
    postgres_staff_put_status_rechecks_role,
    "PUT",
    "status",
    "role"
);
waiting_staff_test!(
    postgres_staff_reply_rechecks_disablement,
    "POST",
    "reply",
    "suspend"
);
waiting_staff_test!(
    postgres_staff_note_rechecks_disablement,
    "POST",
    "note",
    "suspend"
);
waiting_staff_test!(
    postgres_staff_post_status_rechecks_disablement,
    "POST",
    "status",
    "suspend"
);
waiting_staff_test!(
    postgres_staff_put_status_rechecks_disablement,
    "PUT",
    "status",
    "suspend"
);

waiting_staff_test!(
    postgres_staff_reply_rechecks_admin_access,
    "POST",
    "reply",
    "admin.access"
);
waiting_staff_test!(
    postgres_staff_reply_rechecks_ticket_permission,
    "POST",
    "reply",
    "support.tickets.manage"
);

#[tokio::test]
async fn postgres_pagination_ticket_boundary() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let account = db.account("en").await?;
    sqlx::query("INSERT INTO tickets(account_id,request_key,category,subject,last_message_at) SELECT $1,uuidv7(),'technical','Boundary ticket '||n,now()-n*interval '1 second' FROM generate_series(1,200021) n").bind(account).execute(&db.pool).await?;
    let tickets = db.tickets(3)?;
    for staff in [false, true] {
        let previous = tickets.list(account, staff, 10000).await?;
        let last = tickets.list(account, staff, 10001).await?;
        assert_eq!(previous.len(), 20);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].subject, "Boundary ticket 200021");
        assert!(previous.iter().all(|row| row.id != last[0].id));
        assert!(tickets.list(account, staff, 10002).await?.is_empty());
    }
    let (cookie, _) = db.cookie(account).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(account).execute(&db.pool).await?;
    let adapter = StaffAdapter {
        mfa: true,
        state: oteryn_admin::rbac::AccountState::Active,
    };
    let staff_app = staff_router(
        tickets.clone(),
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        adapter.clone(),
        adapter,
    );
    let app = router(tickets, "http://localhost:3000");
    for (number, has_next) in [(10000, true), (10001, false), (10002, false)] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/en/support/tickets?page={number}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert_eq!(html.contains(&format!("?page={}\"", number + 1)), has_next);
    }
    for (number, has_next) in [(10000, true), (10001, false)] {
        let response = staff_app
            .clone()
            .oneshot(
                Request::get(format!("/en/admin/support/tickets?page={number}"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert_eq!(html.contains(&format!("?page={}\"", number + 1)), has_next);
    }
    db.finish().await?;
    eprintln!(
        "postgres_pagination_ticket_boundary duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn lk_crossed_staff_replies_finish() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Database::with_connections(2)
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let a = db.account("en").await?;
    let b = db.account("en").await?;
    for actor in [a, b] {
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await?;
    }
    let proof_a = crate::test_staff::proof(&db.pool, a).await?;
    let proof_b = crate::test_staff::proof(&db.pool, b).await?;
    let tickets = db.tickets(100)?;
    for _ in 0..16 {
        let ta = tickets
            .create(a, Uuid::now_v7(), "other", "subject", "body")
            .await?;
        let tb = tickets
            .create(b, Uuid::now_v7(), "other", "subject", "body")
            .await?;
        let barrier = tokio::sync::Barrier::new(2);
        let reply = |actor, id| {
            let barrier = &barrier;
            let tickets = &tickets;
            async move {
                barrier.wait().await;
                tickets
                    .change(
                        actor,
                        Some(CheckContext {
                            account: actor,
                            require_staff_mfa: true,
                            mfa_verified: true,
                            session_proof: Some(if actor == a { proof_a } else { proof_b }),
                        }),
                        id,
                        1,
                        Change::Reply {
                            body: "staff reply",
                            internal: false,
                        },
                    )
                    .await
            }
        };
        let (left, right) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(reply(a, tb.id), reply(b, ta.id))
        })
        .await?;
        left?;
        right?;
    }
    db.finish().await?;
    eprintln!(
        "lk_crossed_staff_replies_finish duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_read_acknowledgement_requires_displayed_reply() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let owner = db.account("en").await?;
    let staff = db.account("en").await?;
    let tickets = db.tickets(10)?;
    let id:Uuid=sqlx::query_scalar("INSERT INTO tickets(account_id,request_key,category,subject,unread_reply) VALUES($1,$2,'account','Read boundary',true) RETURNING id").bind(owner).bind(Uuid::now_v7()).fetch_one(&db.pool).await?;
    for n in 0..51 {
        sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,$4,$5,false)").bind(Uuid::now_v7()).bind(id).bind(if n==50 {staff} else {owner}).bind(if n==50 {"staff"} else {"user"}).bind(format!("Message {n}")).execute(&db.pool).await?;
    }
    let (cookie, _) = db.cookie(owner).await?;
    let cursor: Uuid = sqlx::query_scalar(
        "SELECT id FROM ticket_messages WHERE ticket_id=$1 ORDER BY id OFFSET 49 LIMIT 1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    let response = router(tickets.clone(), "http://localhost:3000")
        .oneshot(
            Request::head(format!("/en/support/tickets/{id}?after={cursor}"))
                .header("cookie", cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let (_, first) = tickets.show(owner, false, id).await?;
    let unread = || {
        sqlx::query_scalar::<_, bool>("SELECT unread_reply FROM tickets WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
    };
    assert!(unread().await?);
    // Owner mutations establish no viewing boundary.
    tickets
        .change(
            owner,
            None,
            id,
            1,
            Change::Reply {
                body: "Owner reply",
                internal: false,
            },
        )
        .await?;
    assert!(unread().await?);
    tickets
        .change(owner, None, id, 2, Change::Status("closed"))
        .await?;
    assert!(unread().await?);
    let (_, last) = tickets
        .show_page(owner, false, id, first.last().map(|m| m.id))
        .await?;
    assert_eq!(last.len(), 2);
    assert!(!unread().await?);
    // A later reply must survive an acknowledgement of the older page.
    sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,'staff','New reply',false)").bind(Uuid::now_v7()).bind(id).bind(staff).execute(&db.pool).await?;
    sqlx::query("UPDATE tickets SET unread_reply=true WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    tickets.show(owner, false, id).await?;
    assert!(unread().await?);
    db.finish().await?;
    eprintln!(
        "postgres_read_acknowledgement_requires_displayed_reply duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_a3_support_refuses_session_revoked_during_account_wait() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let actor = db.account("en").await?;
    let owner = db.account("en").await?;
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(actor)
    .execute(&db.pool)
    .await?;
    let tickets = db.tickets(10)?;
    let ticket = tickets
        .create(
            owner,
            Uuid::now_v7(),
            "other",
            "Session check",
            "Initial message",
        )
        .await?;
    let context = staff(&db.pool, actor)
        .await
        .ok_or("missing staff context")?;
    let result = crate::test_staff::race_session_revocation(&db.pool, actor, async move {
        tickets
            .change(
                actor,
                Some(context),
                ticket.id,
                1,
                Change::Reply {
                    body: "Privileged reply",
                    internal: false,
                },
            )
            .await
    })
    .await?;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ticket_messages WHERE ticket_id=$1")
            .bind(ticket.id)
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.finish().await?;
    eprintln!(
        "postgres_a3_support_refuses_session_revoked_during_account_wait duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn audit_w3_02_changed_payload_draft_recovers_on_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let owner = db.account("en").await?;
    let service = db.tickets(5)?;
    let app = router(service.clone(), "http://localhost:3000");
    let (cookie, csrf) = db.cookie(owner).await?;
    let post = |key: Uuid, subject: &str, body: &str| {
        let request = Request::post("/en/support/tickets")
            .header("cookie", &cookie)
            .header("origin", "http://localhost:3000")
            .header("content-type", "application/x-www-form-urlencoded")
            .extension(ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                3210,
            ))))
            .body(Body::from(format!(
                "_csrf={csrf}&request_key={key}&category=account&subject={subject}&body={body}"
            )));
        let app = app.clone();
        async move { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(app.oneshot(request?).await?) }
    };
    let key = Uuid::now_v7();
    assert_eq!(
        post(key, "Original", "Original+message").await?.status(),
        StatusCode::FOUND
    );
    let response = post(key, "Edited", "Edited+message").await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()["location"], "/en/support/tickets/create");
    let head = app
        .clone()
        .oneshot(
            Request::head("/en/support/tickets/create")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(head.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(
            Request::get("/en/support/tickets/create")
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Edited"));
    assert!(html.contains("Edited message"));
    let displayed = html
        .split("name=\"request_key\" value=\"")
        .nth(1)
        .ok_or("request key field")?
        .split('"')
        .next()
        .ok_or("key value")?;
    let fresh = Uuid::parse_str(displayed)?;
    assert_ne!(fresh, key);
    let response = post(fresh, "Edited", "Edited+message").await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_ne!(response.headers()["location"], "/en/support/tickets/create");
    let tickets = service.list(owner, false, 0).await?;
    assert_eq!(tickets.len(), 2);
    assert!(tickets.iter().any(|t| t.subject == "Edited"));
    db.finish().await?;
    eprintln!(
        "audit_w3_02_changed_payload_draft_recovers_on_postgres duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
