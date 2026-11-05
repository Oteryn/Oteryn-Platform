#![allow(clippy::unwrap_used)]
use super::super::access::{AccessSnapshot, AgentStatus, Flags, PendingBackend, Policy};
use super::*;
use askama::Template;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
struct Fake {
    allowed: bool,
    advertised: bool,
    snapshot: Option<Snapshot>,
}
impl Backend for Fake {
    async fn permitted(&self, _: CheckContext, _: &'static str) -> bool {
        self.allowed
    }
    async fn advertised(&self, _: &str) -> bool {
        self.advertised
    }
    async fn players(
        &self,
        _: CheckContext,
        _: &Input,
        _: &'static str,
    ) -> Result<Option<Snapshot>, Error> {
        assert!(
            self.allowed && self.advertised,
            "unavailable reads must never reach the backend"
        );
        Ok(self.snapshot.clone())
    }
    async fn ledger(
        &self,
        actor: CheckContext,
        _: &super::super::audit::Filter,
    ) -> Result<Option<Vec<super::super::audit::LedgerRow>>, Error> {
        Ok(self.snapshot.as_ref().map(|snapshot| {
            vec![super::super::audit::LedgerRow {
                request_id: snapshot.request_id,
                ledger_id: "test-ledger".into(),
                admin: actor.account.to_string(),
                operation: "player.character.get".into(),
                target: "c-1".into(),
                status: "succeeded".into(),
                observed_at: snapshot.observed_at.clone(),
            }]
        }))
    }
    async fn access(&self) -> Result<AccessSnapshot, Error> {
        Ok(AccessSnapshot {
            signers: None,
            agents: Some(vec![AgentStatus {
                agent_id: "test-agent".into(),
                operations: vec![],
                observed_at: "2026-11-05T19:00:00Z".into(),
            }]),
        })
    }
    async fn policy(&self, _: CheckContext) -> Result<Option<Vec<Policy>>, Error> {
        Ok(self.advertised.then(|| {
            vec![Policy {
                operation: "world.retire".into(),
                minimum_risk: "R4".into(),
                permission: "game.world.admin".into(),
            }]
        }))
    }
    async fn approvals(
        &self,
        actor: CheckContext,
    ) -> Result<Option<Vec<super::super::approvals::Pending>>, Error> {
        Ok(self.advertised.then(|| {
            vec![super::super::approvals::Pending {
                request_id: Uuid::now_v7(),
                requester: actor.account,
                operation: "world.retire".into(),
                target: "test-world".into(),
                expires_at: "2026-11-05T20:00:00Z".into(),
                preview_digest: "test-digest".into(),
                preview: vec!["safe preview".into()],
            }]
        }))
    }
}
fn actor() -> CheckContext {
    CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
    }
}
fn lazy() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap()
}
fn fake(allowed: bool, advertised: bool) -> Fake {
    Fake {
        allowed,
        advertised,
        snapshot: None,
    }
}
#[tokio::test]
async fn permission_and_disabled_flag_deny_without_storage() {
    let state = Console {
        pool: lazy(),
        flags: Flags { console: true },
        backend: fake(false, true),
    };
    assert!(matches!(
        view(&state, actor(), Input::default()).await,
        Err(Error::Denied)
    ));
    let state = Console {
        pool: lazy(),
        flags: Flags::default(),
        backend: fake(true, true),
    };
    assert!(matches!(
        view(&state, actor(), Input::default()).await,
        Err(Error::Denied)
    ));
    assert!(!PendingBackend.permitted(actor(), "game.player.read").await);
}
#[tokio::test]
async fn unavailable_is_explicit_and_bad_input_refused() {
    let state = Console {
        pool: lazy(),
        flags: Flags { console: true },
        backend: fake(true, false),
    };
    let page = view(
        &state,
        actor(),
        Input {
            lang: Some("en".into()),
            q: Some("Test".into()),
            character: None,
        },
    )
    .await
    .unwrap();
    let html = page.render().unwrap();
    assert!(html.contains("Unavailable"));
    assert!(!html.contains("masked"));
    for q in ["x", "<script>", "a\nb"] {
        assert!(
            Input {
                q: Some(q.into()),
                ..Input::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Input {
            lang: Some("xx".into()),
            ..Input::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Input {
            q: Some("Test".into()),
            character: Some("c-1".into()),
            ..Input::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        super::super::audit::Filter {
            target: Some("\n".into()),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn translations_and_escaping_are_complete() {
    for lang in ["pl", "en"] {
        let mut page = Page::new(lang, "access");
        page.section("signers", Some(vec!["<script>alert(1)</script>".into()]));
        let html = page.render().unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("script"));
        assert!(html.contains("alert(1)"));
        assert!(!page.title.is_empty());
    }
}
#[tokio::test]
async fn postgres_masked_view_writes_exactly_one_read_audit()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL console read audit: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = Database::open(&url).await?;
    let pool = db.pool.clone();
    let request_id = Uuid::now_v7();
    let actor = actor();
    let snapshot = Snapshot {
        request_id,
        observed_at: "2026-11-05T19:00:00Z".into(),
        characters: Some(vec![Character {
            id: "c-1".into(),
            name: "Test Knight".into(),
            world: "test-world".into(),
            level: 42,
            dossier: None,
        }]),
        inventory: Some(vec![]),
        sanctions: Some(vec![]),
    };
    let state = Console {
        pool: pool.clone(),
        flags: Flags { console: true },
        backend: Fake {
            snapshot: Some(snapshot),
            ..fake(true, true)
        },
    };
    let page = view(
        &state,
        actor,
        Input {
            lang: Some("en".into()),
            character: Some("c-1".into()),
            q: None,
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let html = page.render()?;
    assert!(html.contains("Test Knight"));
    assert!(html.contains("Email: masked"));
    assert!(html.contains("IP: masked"));
    assert!(html.contains("Payment details: masked"));
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE request_id=$1 AND actor=$2 AND action='console.players.view'").bind(request_id).bind(actor.account).fetch_one(&pool).await?;
    assert_eq!(count, 1);
    let denied = Console {
        pool: pool.clone(),
        flags: Flags { console: true },
        backend: fake(false, true),
    };
    assert!(matches!(
        view(&denied, actor, Input::default()).await,
        Err(Error::Denied)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE request_id=$1")
        .bind(request_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await?;
    Ok(())
}

struct Database {
    pool: sqlx::PgPool,
    root: sqlx::PgPool,
    schema: String,
}
impl Database {
    async fn open(url: &str) -> Result<Self, sqlx::Error> {
        let root = PgPoolOptions::new().max_connections(2).connect(url).await?;
        let schema = format!("console_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(3)
            .after_connect(move |connection, _| {
                let query = search_path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(query))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(url)
            .await?;
        oteryn_db::migrate(&pool)
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        Ok(Self { pool, root, schema })
    }
    async fn close(self) -> Result<(), sqlx::Error> {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}
#[derive(Clone)]
struct Browser(Uuid);
impl crate::web::StaffSession for Browser {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<crate::web::StaffIdentity> {
        Some(crate::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
#[derive(Clone)]
struct Accounts;
impl crate::rbac::AccountStateReader for Accounts {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<crate::rbac::AccountState, crate::rbac::AccountStateUnavailable> {
        Ok(crate::rbac::AccountState::Active)
    }
}
#[tokio::test]
async fn postgres_http_four_pages_filters_host_and_read_only()
-> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL console HTTP: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let db = Database::open(&url).await?;
    let actor = actor();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute(&db.pool).await?;
    let app = super::super::access::router(
        db.pool.clone(),
        Flags { console: true },
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(actor.account),
        Accounts,
        fake(true, false),
    );
    for area in ["players", "audit", "access", "approvals"] {
        let path = format!("/admin/console/{area}?lang=en");
        let response = app
            .clone()
            .oneshot(
                Request::get(&path)
                    .header("host", "staff.test")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "{area}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains("Unavailable"), "{area}");
        assert!(html.contains("width=device-width"));
        assert!(!html.contains("method=\"post\""));
        // Preserve a real rendered artifact for narrow viewport/accessibility checks.
        if let Ok(dir) = std::env::var("PORTAL_CONSOLE_RENDER_DIR") {
            std::fs::write(format!("{dir}/{area}.html"), html)?;
        }
    }
    for (path, host, status) in [
        (
            "/admin/console/players?q=%3Cscript%3E",
            "staff.test",
            StatusCode::BAD_REQUEST,
        ),
        (
            "/admin/console/audit?request_id=&admin=&operation=&target=",
            "staff.test",
            StatusCode::OK,
        ),
        (
            "/admin/console/audit?admin=bad",
            "staff.test",
            StatusCode::BAD_REQUEST,
        ),
        ("/admin/console/access", "wrong.test", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", host)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), status, "{path}");
    }
    let seed_id = Uuid::now_v7();
    record_read(&db.pool, actor, seed_id, "c-1", "player.character.get")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let available = super::super::access::router(
        db.pool.clone(),
        Flags { console: true },
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(actor.account),
        Accounts,
        Fake {
            snapshot: Some(Snapshot {
                request_id: seed_id,
                observed_at: "2026-11-05T19:00:00Z".into(),
                characters: None,
                inventory: None,
                sanctions: None,
            }),
            ..fake(true, true)
        },
    );
    for (path, expected) in [
        (
            format!(
                "/admin/console/audit?lang=en&request_id={seed_id}&admin={}&operation=player.character.get&target=c-1",
                actor.account
            ),
            "test-ledger",
        ),
        ("/admin/console/access?lang=en".into(), "game.world.admin"),
        ("/admin/console/approvals?lang=en".into(), "safe preview"),
    ] {
        let response = available
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", "staff.test")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains(expected));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='console.audit.view'",
    )
    .bind(actor.account)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 1);
    let refused = super::super::access::router(
        db.pool.clone(),
        Flags { console: true },
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(actor.account),
        Accounts,
        fake(false, false),
    );
    let response = refused
        .oneshot(
            Request::get("/admin/console/players")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await?;
    Ok(())
}
