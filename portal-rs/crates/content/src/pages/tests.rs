use super::staff::{PageForm, persist};
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
use uuid::Uuid;
#[test]
fn input_boundaries() {
    assert!(valid_slug("welcome-2"));
    for slug in ["", "../rules", "two--parts", "<script>", "UPPER"] {
        assert!(!valid_slug(slug));
    }
    assert!(reserved("terms-of-service"));
    assert!(legal("privacy-policy"));
    assert!(!legal("welcome"));
    assert!(!text("pl", "unavailable").is_empty());
    assert!(!text("en", "unavailable").is_empty());
}
#[tokio::test]
async fn postgres_pages_publication_and_legal_history() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL pages test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("pages_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = format!("SET search_path TO {schema}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(path)).execute(c).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    for sql in [
        include_str!("../../../../migrations/20261105144852_t02_web_sessions.sql"),
        include_str!("../../../../migrations/20261105170000_t10_rbac_audit.sql"),
        include_str!("../../../../migrations/20261105220204_t11b_pages.sql"),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&pool)
            .await?;
    }
    let slug = format!("page-{}", Uuid::now_v7());
    let ctx = oteryn_admin::rbac::CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let mut form = PageForm {
        title: "Safe title".into(),
        body: "# Hello\n<script>alert(1)</script>".into(),
        published_at: "2020-01-01T00:00:00Z".into(),
        action: "save".into(),
        ..Default::default()
    };
    persist(&pool, ctx, &slug, "en", &form).await?;
    let app = router(pool.clone());
    for (lang, message) in [
        ("en", "This page has not been published yet."),
        ("pl", "Ta strona nie została jeszcze opublikowana."),
    ] {
        for path in [
            "rules",
            "legal/terms",
            "legal/privacy",
            "legal/cookies",
            "help",
            "support/report-a-bug",
            "server-information",
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(format!("/{lang}/{path}")).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
            assert!(html.contains(message));
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(!html.contains("Changed legal text"));
        }
    }

    let response = app
        .clone()
        .oneshot(Request::get(format!("/en/pages/{slug}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    std::fs::write("/tmp/t11b-public.html", &html)?;
    assert!(html.contains("<h1>Hello</h1>"));
    assert!(!html.contains("<script>"));
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    persist(&pool, ctx, &slug, "pl", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::OK
    );
    form.body = "Changed source".into();
    persist(&pool, ctx, &slug, "en", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    form.published_at = "2099-01-01T00:00:00Z".into();
    persist(&pool, ctx, &slug, "en", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/en/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.oneshot(Request::get("/en/pages/rules").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    form.title.clear();
    assert!(persist(&pool, ctx, &slug, "en", &form).await.is_err());
    form.title = "Delete title".into();
    form.action = "delete".into();
    persist(&pool, ctx, &slug, "en", &form).await?;
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale='en'").bind(&slug).fetch_one(&pool).await?;
    assert_eq!(remaining, 0);
    form.action = "save".into();
    form.title = "Legal title".into();
    form.body = "Legal text".into();
    form.published_at = "2020-01-01T00:00:00Z".into();
    form.legal_version = "test-v1".into();
    form.legal_effective_date = "2020-01-01".into();
    persist(&pool, ctx, "terms-of-service", "en", &form).await?;
    form.body = "Changed legal text".into();
    assert!(
        persist(&pool, ctx, "terms-of-service", "en", &form)
            .await
            .is_err()
    );
    form.legal_version = "test-v2".into();
    persist(&pool, ctx, "terms-of-service", "en", &form).await?;
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM page_legal_versions")
        .fetch_one(&pool)
        .await?;
    assert_eq!(versions, 2);
    use oteryn_admin::web::PanelConfig;
    let identity = Identity(ctx.account);
    let staff = staff_router(
        pool.clone(),
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        identity.clone(),
        identity,
    );
    let response = staff
        .clone()
        .oneshot(
            Request::get("/admin/pages/new-page/en")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(ctx.account).execute(&pool).await?;
    let response = staff
        .clone()
        .oneshot(
            Request::get("/admin/pages/new-page/en")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("invalid cookie")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    std::fs::write("/tmp/t11b-editor.html", &html)?;
    assert!(html.contains("for=\"body\""));
    let token = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .ok_or("missing token")?
        .split('"')
        .next()
        .ok_or("invalid token")?;
    for (csrf, status) in [
        ("bad", StatusCode::FORBIDDEN),
        (token, StatusCode::SEE_OTHER),
    ] {
        let response = staff
            .clone()
            .oneshot(
                Request::post("/admin/pages/new-page/en")
                    .header("host", "staff.test")
                    .header(header::COOKIE, &cookie)
                    .header(header::ORIGIN, "https://staff.test")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "_csrf={csrf}&title=Hello&body=Content&action=save"
                    )))?,
            )
            .await?;
        assert_eq!(response.status(), status);
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
#[tokio::test]
async fn staff_off_and_wrong_host_refuse() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_admin::web::{PanelConfig, PendingAuthentication};
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    for config in [
        PanelConfig::default(),
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
    ] {
        let app = staff_router(
            pool.clone(),
            config,
            PendingAuthentication,
            PendingAuthentication,
        );
        assert_eq!(
            app.oneshot(
                Request::get("/admin/pages")
                    .header("host", "public.test")
                    .body(Body::empty())?
            )
            .await?
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    Ok(())
}

#[derive(Clone)]
struct Identity(Uuid);
impl oteryn_admin::web::StaffSession for Identity {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
impl oteryn_admin::rbac::AccountStateReader for Identity {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        Ok(oteryn_admin::rbac::AccountState::Active)
    }
}
