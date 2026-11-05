#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;
#[derive(Clone)]
struct Identity(Option<StaffIdentity>);
impl StaffSession for Identity {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        self.0
    }
}
impl AccountStateReader for Identity {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
fn app(pool: PgPool, identity: Option<StaffIdentity>, enabled: bool) -> Router {
    router(
        pool,
        PanelConfig {
            admin_panel: enabled,
            host: "staff.test".into(),
        },
        Identity(identity),
        Identity(identity),
    )
}
async fn request(app: Router, path: &str, host: &str) -> Response {
    app.oneshot(
        Request::get(path)
            .header("host", host)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn host_flag_session_and_mfa_boundaries() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    assert_eq!(
        request(app(pool.clone(), None, true), "/admin", "wrong.test")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(app(pool.clone(), None, false), "/admin", "staff.test")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let response = request(app(pool.clone(), None, true), "/admin", "staff.test").await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/login");
    let response = request(
        app(
            pool,
            Some(StaffIdentity {
                account: Uuid::now_v7(),
                mfa_satisfied: false,
            }),
            true,
        ),
        "/admin",
        "staff.test",
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = String::from_utf8(
        to_bytes(response.into_body(), 10000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("wieloskładnikowe"));
}
#[test]
fn localized_responsive_labelled_template() {
    for lang in ["pl", "en"] {
        let html = Page {
            lang,
            title: text(lang, "title"),
            roles_label: text(lang, "roles"),
            account_label: text(lang, "account"),
            role_label: text(lang, "role"),
            grant: text(lang, "grant"),
            revoke: text(lang, "revoke"),
            csrf: "token".into(),
            roles: vec![(Uuid::now_v7(), "<unsafe>".into())],
            manage: true,
        }
        .render()
        .unwrap();
        assert!(html.contains("width=device-width"));
        assert!(html.contains("op-input"));
        assert!(html.contains("for=\"account\""));
        assert!(html.contains("for=\"role\""));
        assert!(!html.contains("<unsafe>"));
        std::fs::write(format!("/tmp/t10b-{lang}.html"), &html).unwrap();
        assert!(!text(lang, "mfa").is_empty());
    }
}
#[tokio::test]
async fn database_navigation_authorization_csrf_and_owner() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t10b_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
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
    // Isolated fixture executes each script once; main currently has duplicate versions.
    for migration in oteryn_db::MIGRATOR.iter() {
        sqlx::raw_sql(AssertSqlSafe(migration.sql.as_ref()))
            .execute(&pool)
            .await
            .unwrap();
    }
    let actor = Uuid::now_v7();
    let identity = Some(StaffIdentity {
        account: actor,
        mfa_satisfied: true,
    });
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(actor)
    .execute(&pool)
    .await
    .unwrap();
    // Use an explicit minimal staff role independent of catalog defaults.
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('panel_test') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    let response = request(app(pool.clone(), identity, true), "/admin", "staff.test").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!body.contains("href=\"/admin/roles\""));
    assert_eq!(
        request(
            app(pool.clone(), identity, true),
            "/admin/roles",
            "staff.test"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='admin.access.denied' AND actor=$1",
    )
    .bind(actor)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    sqlx::query(
        "INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.roles.manage')",
    )
    .bind(role)
    .execute(&pool)
    .await
    .unwrap();
    let response = request(
        app(pool.clone(), identity, true),
        "/admin/roles",
        "staff.test",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("href=\"/admin/roles\""));
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let target = Uuid::now_v7();
    let ordinary: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='panel_test'")
        .fetch_one(&pool)
        .await
        .unwrap();
    // A grant to a role manager is refused pending four-eyes approval.
    let denied = app(pool.clone(), identity, true)
        .oneshot(
            Request::post("/admin/roles")
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .header("x-csrf-token", csrf)
                .header("origin", "https://staff.test")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "account={target}&role={ordinary}&action=grant"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let ordinary: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('ordinary_staff') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    for (account, action, status) in [
        (target.to_string(), "grant", StatusCode::SEE_OTHER),
        (target.to_string(), "revoke", StatusCode::SEE_OTHER),
        ("invalid".into(), "grant", StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let response = app(pool.clone(), identity, true)
            .oneshot(
                Request::post("/admin/roles")
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("x-csrf-token", csrf)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "account={account}&role={ordinary}&action={action}"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }

    let response = app(pool.clone(), identity, true)
        .oneshot(
            Request::post("/admin/roles")
                .header("host", "staff.test")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("account=bad&action=grant"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let owner: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='platform_admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let auth = Authorizer::new(pool.clone(), Identity(identity));
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let panel = Panel {
        config: PanelConfig::default(),
        identity: Identity(identity),
        auth: Authorizer::new(pool.clone(), Identity(identity)),
        pool: pool.clone(),
    };
    assert!(matches!(
        revoke(&panel, context, actor, owner).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        auth.change(
            context,
            Uuid::now_v7(),
            Change::Grant {
                account: Uuid::nil(),
                role
            }
        )
        .await,
        Err(Error::InvalidInput)
    ));
    let other = Uuid::now_v7();
    auth.change(
        context,
        Uuid::now_v7(),
        Change::Grant {
            account: other,
            role,
        },
    )
    .await
    .unwrap();
    auth.change(
        context,
        Uuid::now_v7(),
        Change::Revoke {
            account: other,
            role,
        },
    )
    .await
    .unwrap();
    // Two competing removals cannot both discard the last full-access holder.
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(other)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        revoke(&panel, context, actor, owner),
        revoke(&panel, context, other, owner)
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let holders: i64 = sqlx::query_scalar("SELECT count(*) FROM account_roles WHERE role_id=$1")
        .bind(owner)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(holders, 1);
    let explicit_owner: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('owner') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(explicit_owner)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        revoke(&panel, context, actor, explicit_owner).await,
        Err(Error::Denied)
    ));
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
