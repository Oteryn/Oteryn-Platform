#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

#[derive(Clone)]
struct Staff(Uuid);
impl web::StaffSession for Staff {
    async fn resolve(&self, _: &BrowserSession) -> Option<web::StaffIdentity> {
        Some(web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
impl AccountStateReader for Staff {
    async fn state(&self, _: Uuid) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        Ok(rbac::AccountState::Active)
    }
}
struct Grants(bool);
impl Permissions for Grants {
    async fn check(&self, _: CheckContext, _: Operation) -> Result<(), Error> {
        if self.0 { Ok(()) } else { Err(Error::Denied) }
    }
}
#[derive(Clone)]
struct Fixture {
    mode: u8,
    calls: Arc<AtomicUsize>,
}
impl Channel for Fixture {
    async fn read(
        &self,
        actor: Uuid,
        agent: &str,
        op: Operation,
        target: Option<&str>,
        request: Option<Uuid>,
    ) -> Result<AgentView, Error> {
        let now = SystemTime::now();
        Ok(AgentView {
            owner_epoch: 1,
            status: if self.mode == 7 {
                QueryStatus::Expired
            } else {
                QueryStatus::Succeeded
            },
            last_contact: if self.mode == 1 {
                now - Duration::from_secs(30)
            } else {
                now
            },
            current_owner: self.mode != 3,
            operations: if self.mode == 2 {
                vec![]
            } else {
                vec![(op, 1)]
            },
            result: Some(CachedResult {
                owner_epoch: if self.mode == 6 { 2 } else { 1 },
                request_id: request.unwrap_or_else(Uuid::now_v7),
                actor: if self.mode == 4 {
                    Uuid::now_v7()
                } else {
                    actor
                },
                agent: agent.into(),
                operation: op,
                target: target.map(str::to_owned),
                observed_at: now - Duration::from_secs(if self.mode == 5 { 90 } else { 1 }),
                revision: "fixture-revision".into(),
                records: vec![Record {
                    name: "<script>alert(1)</script>".into(),
                    fields: vec![Field {
                        label: FieldLabel::Health,
                        value: "fixture-healthy".into(),
                    }],
                }],
            }),
        })
    }
    async fn enqueue(
        &self,
        _: CheckContext,
        _: &BrowserSession,
        _: &str,
        _: Operation,
        _: Option<&str>,
    ) -> Result<Uuid, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Uuid::now_v7())
    }
}
fn fixture(mode: u8) -> Fixture {
    Fixture {
        mode,
        calls: Arc::new(AtomicUsize::new(0)),
    }
}
fn app(pool: PgPool, actor: Uuid, fixture: Fixture, allowed: bool, enabled: bool) -> Router {
    router(
        Console {
            enabled,
            permissions: Grants(allowed),
            channel: fixture,
            pool,
        },
        web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Staff(actor),
        Staff(actor),
    )
}
async fn get(app: Router, path: &str) -> Response {
    app.oneshot(
        Request::get(path)
            .header("host", "staff.test")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
async fn body(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
struct Db {
    root: PgPool,
    pool: PgPool,
    schema: String,
}
impl Db {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("adm_p2d_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        for migration in oteryn_db::MIGRATOR.iter() {
            sqlx::raw_sql(AssertSqlSafe(migration.sql.as_ref()))
                .execute(&pool)
                .await
                .unwrap();
        }
        Some(Self { root, pool, schema })
    }
    async fn staff(&self) -> Uuid {
        let actor = Uuid::now_v7();
        let role: Uuid =
            sqlx::query_scalar("INSERT INTO roles(key) VALUES('console_fixture') RETURNING id")
                .fetch_one(&self.pool)
                .await
                .unwrap();
        sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.access')")
            .bind(role)
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
            .bind(actor)
            .bind(role)
            .execute(&self.pool)
            .await
            .unwrap();
        actor
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
}
#[test]
fn capabilities_connection_and_input_fail_closed() {
    let now = SystemTime::now();
    let mut view = AgentView {
        owner_epoch: 1,
        status: QueryStatus::Succeeded,
        last_contact: now,
        current_owner: true,
        operations: vec![(Operation::Overview, 1)],
        result: None,
    };
    assert!(connected(&view, Operation::Overview, now));
    for age in [30, 31] {
        view.last_contact = now - Duration::from_secs(age);
        assert!(!connected(&view, Operation::Overview, now));
    }
    view.last_contact = now + Duration::from_secs(1);
    assert!(!connected(&view, Operation::Overview, now));
    view.last_contact = now;
    view.operations.push((Operation::Overview, 2));
    assert!(!connected(&view, Operation::Overview, now));
    for agent in ["", "a/b", "a&x=y", "<script>", &"x".repeat(129)] {
        assert!(
            Selection {
                agent: Some(agent.into()),
                ..Selection::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Selection {
            agent: Some("agent-1".into()),
            target: Some("world_1".into()),
            ..Selection::default()
        }
        .validate()
        .is_ok()
    );
    for op in [
        Operation::Overview,
        Operation::Worlds,
        Operation::World,
        Operation::Nodes,
        Operation::Node,
        Operation::Database,
        Operation::Migrations,
    ] {
        assert!(op.permission().starts_with("game."));
        for lang in ["pl", "en"] {
            assert!(!text(lang, op.title()).is_empty());
        }
    }
}
#[tokio::test]
async fn postgres_pages_permissions_unavailable_and_happy_render() {
    let Some(db) = Db::new().await else {
        return;
    };
    let actor = db.staff().await;
    let fixture = fixture(0);
    for path in ["dashboard", "worlds", "nodes", "database"] {
        let url = format!("/console/{path}?agent=agent-1&lang=en");
        let response = get(
            app(db.pool.clone(), actor, fixture.clone(), false, true),
            &url,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = get(
            app(db.pool.clone(), actor, fixture.clone(), true, false),
            &url,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        for lang in ["pl", "en"] {
            let response = get(
                app(db.pool.clone(), actor, fixture.clone(), true, true),
                &format!("/console/{path}?agent=agent-1&lang={lang}"),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let html = body(response).await;
            std::fs::write(format!("/tmp/adm-p2d-{path}-{lang}.html"), &html).unwrap();
            assert!(html.contains("fixture-healthy"));
            assert!(html.contains("fixture-revision"));
            assert!(html.contains("&#60;script&#62;"));
            assert!(!html.contains("<script>"));
            assert!(html.contains("width=device-width"));
            assert!(html.contains(r#"<label for="agent""#));
            assert!(html.contains(&format!(r#"lang="{lang}""#)));
        }
        for mode in 1..=7 {
            if mode == 5 {
                continue;
            }
            let response = get(
                app(
                    db.pool.clone(),
                    actor,
                    super::tests::fixture(mode),
                    true,
                    true,
                ),
                &url,
            )
            .await;
            let html = body(response).await;
            assert!(!html.contains("fixture-healthy"));
            assert!(html.contains(if mode == 7 {
                "Query expired"
            } else {
                "Unavailable"
            }));
        }
    }
    for path in ["worlds", "nodes"] {
        assert!(
            body(
                get(
                    app(db.pool.clone(), actor, fixture.clone(), true, true),
                    &format!("/console/{path}?agent=agent-1&target=world-1&lang=en")
                )
                .await
            )
            .await
            .contains("fixture-healthy")
        );
    }
    let stale = body(
        get(
            app(db.pool.clone(), actor, super::tests::fixture(5), true, true),
            "/console/worlds?agent=agent-1&lang=en",
        )
        .await,
    )
    .await;
    assert!(stale.contains("Stale cached data"));
    let missing = body(
        get(
            app(db.pool.clone(), actor, fixture.clone(), true, true),
            "/console/worlds?lang=en",
        )
        .await,
    )
    .await;
    assert!(missing.contains("Unavailable"));
    for path in [
        "/console/worlds?agent=bad%2Fid",
        "/console/worlds?lang=xx",
        "/console/dashboard?target=world-1",
        "/console/nodes?request=bad",
        "/console/database?unknown=1",
    ] {
        assert_eq!(
            get(
                app(db.pool.clone(), actor, fixture.clone(), true, true),
                path
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        fixture.calls.load(Ordering::SeqCst),
        0,
        "render must never enqueue or contact Game"
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_query_csrf_permission_and_local_polling() {
    let Some(db) = Db::new().await else {
        return;
    };
    let actor = db.staff().await;
    let fixture = fixture(0);
    let application = app(db.pool.clone(), actor, fixture.clone(), true, true);
    let response = get(application.clone(), "/console/worlds?agent=agent-1&lang=en").await;
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = body(response).await;
    let csrf = html
        .split(r#"name="_csrf" value=""#)
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    for (allowed, token, operation, status) in [
        (true, "wrong", "world.list", StatusCode::FORBIDDEN),
        (false, csrf, "world.list", StatusCode::FORBIDDEN),
        (true, csrf, "node.list", StatusCode::BAD_REQUEST),
        (true, csrf, "world.list", StatusCode::SEE_OTHER),
    ] {
        let response = app(db.pool.clone(), actor, fixture.clone(), allowed, true)
            .oneshot(
                Request::post("/console/worlds")
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "operation={operation}&agent=agent-1&lang=en&_csrf={token}"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status == StatusCode::SEE_OTHER {
            let location = response.headers()[header::LOCATION].to_str().unwrap();
            assert!(location.contains("request="));
            assert!(
                body(get(application.clone(), location).await)
                    .await
                    .contains("fixture-healthy")
            );
        }
    }
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    db.close().await;
}
#[tokio::test]
async fn postgres_platform_metrics_and_closed_permission_catalog() {
    let Some(db) = Db::new().await else {
        return;
    };
    let actor = db.staff().await;
    let auth = PgPermissions {
        pool: db.pool.clone(),
        accounts: Staff(actor),
    };
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    for op in [
        Operation::Overview,
        Operation::Worlds,
        Operation::Nodes,
        Operation::Database,
    ] {
        assert!(matches!(auth.check(context, op).await, Err(Error::Denied)));
    }
    let account:Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES('fixture@example.invalid','fixture','en') RETURNING id").fetch_one(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO security_events(account_id,kind) VALUES($1,'login_succeeded'),($1,'all_sessions_revoked')").bind(account).execute(&db.pool).await.unwrap();
    let intent = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state,created_at) VALUES($1,$2,'fixture','receipt',12345,'PLN','succeeded',now()-interval '2 days')").bind(intent).bind(account).execute(&db.pool).await.unwrap();
    for _ in 0..2 {
        sqlx::query("INSERT INTO payment_events(id,provider,provider_event_id,intent_id,state) VALUES($1,'fixture',$2,$3,'succeeded')").bind(Uuid::now_v7()).bind(Uuid::now_v7().to_string()).bind(intent).execute(&db.pool).await.unwrap();
    }
    let section = dashboard::section(&db.pool, "en").await;
    assert_eq!(section.source, "Platform");
    assert_eq!(section.state, "Fresh");
    let fields = &section.records[0].fields;
    assert!(fields.contains(&("Registrations".into(), "1".into())));
    assert!(fields.contains(&("Successful logins".into(), "1".into())));
    assert!(fields.contains(&("Revenue (minor units) (PLN)".into(), "12345".into())));
    sqlx::query("INSERT INTO payment_events(id,provider,provider_event_id,intent_id,state,created_at) VALUES($1,'fixture',$2,$3,'succeeded',now()-interval '2 days')").bind(Uuid::now_v7()).bind(Uuid::now_v7().to_string()).bind(intent).execute(&db.pool).await.unwrap();
    let section = dashboard::section(&db.pool, "en").await;
    assert!(section.records[0].fields.contains(&(
        "Revenue (minor units)".into(),
        "No successful payments today".into()
    )));
    db.close().await;
}
