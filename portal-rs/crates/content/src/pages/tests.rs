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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
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
    oteryn_db::migrate(&pool).await?;
    let slug = format!("page-{}", Uuid::now_v7());
    let mut ctx = oteryn_admin::rbac::CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: None,
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(ctx.account).execute(&pool).await?;
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'editor-test-password','active','en',now())")
        .bind(ctx.account).bind(format!("{}@example.invalid",ctx.account)).execute(&pool).await?;
    ctx.session_proof = Some(crate::test_staff::proof(&pool, ctx.account).await?);
    let auth = oteryn_admin::rbac::MutationAuthorization::new(std::sync::Arc::new(
        oteryn_admin::rbac::Authorizer::new(
            pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
        ),
    ));
    let mut form = PageForm {
        title: "Safe title".into(),
        body: "# Hello\n<script>alert(1)</script>".into(),
        published_at: "2020-01-01T00:00:00Z".into(),
        action: "save".into(),
        ..Default::default()
    };
    let missing = format!("missing-{slug}");
    let delete = PageForm {
        action: "delete".into(),
        ..form.clone()
    };
    assert!(matches!(
        persist(&pool, &auth, ctx, &missing, "en", &delete).await,
        Err(oteryn_admin::rbac::Error::InvalidInput)
    ));
    let missing_artifacts: (bool, bool) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pages WHERE slug=$1), EXISTS(SELECT 1 FROM audit_log WHERE target=$1)",
    )
    .bind(&missing)
    .fetch_one(&pool)
    .await?;
    assert_eq!(missing_artifacts, (false, false));
    let op_pool = pool.clone();
    let op_auth = auth.clone();
    let op_slug = slug.clone();
    let op_form = form.clone();
    crate::test_staff::race_account_before_fence(&pool, ctx.account, async move {
        persist(&op_pool, &op_auth, ctx, &op_slug, "en", &op_form).await
    })
    .await
    .map_err(|e| e as Box<dyn std::error::Error>)??;
    let source_revision: i64 = sqlx::query_scalar("SELECT e.revision FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale='en'").bind(&slug).fetch_one(&pool).await?;
    let mut reviewed = form.clone();
    reviewed.expected_revision = sqlx::query_scalar("SELECT e.updated_at::text FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale='en'").bind(&slug).fetch_one(&pool).await?;
    persist(&pool, &auth, ctx, &slug, "en", &reviewed).await?;
    let publication: (String, String) = sqlx::query_as(
        "SELECT before->>'published_at',after->>'published_at' FROM audit_log
         WHERE action='cms.page.saved' AND target=$1 AND before <> 'null'::jsonb",
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await?;
    assert_eq!(publication.0, publication.1);
    assert!(matches!(
        persist(&pool, &auth, ctx, &slug, "en", &reviewed).await,
        Err(oteryn_admin::rbac::Error::InvalidInput)
    ));
    reviewed.expected_revision = sqlx::query_scalar("SELECT e.updated_at::text FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale='en'").bind(&slug).fetch_one(&pool).await?;
    reviewed.body.push_str(
        "

Updated English source",
    );
    persist(&pool, &auth, ctx, &slug, "en", &reviewed).await?;
    let mut unseen_translation = form.clone();
    unseen_translation.reviewed_source_revision = source_revision;
    assert!(matches!(
        persist(&pool, &auth, ctx, &slug, "pl", &unseen_translation).await,
        Err(oteryn_admin::rbac::Error::InvalidInput)
    ));
    let translated: i64 = sqlx::query_scalar("SELECT count(*) FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale='pl'").bind(&slug).fetch_one(&pool).await?;
    assert_eq!(translated, 0);
    persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
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
    persist_current(&pool, &auth, ctx, &slug, "pl", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::OK
    );
    // Saving the same source, or changing only publication metadata, keeps approval.
    for published in ["2020-01-01T00:00:00Z", "2020-01-02T00:00:00Z"] {
        form.published_at = published.into();
        persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
        for locale in ["en", "pl"] {
            assert_eq!(
                app.clone()
                    .oneshot(Request::get(format!("/{locale}/pages/{slug}")).body(Body::empty())?)
                    .await?
                    .status(),
                StatusCode::OK
            );
            let search = oteryn_web::search::routes(oteryn_web::search::SearchState {
                pool: pool.clone(),
                world: None,
                characters: None,
            });
            let mut req =
                Request::get(format!("/{locale}/search?q=hello&kind=pages")).body(Body::empty())?;
            req.extensions_mut().insert(axum::extract::ConnectInfo(
                "192.0.2.17:1234".parse::<std::net::SocketAddr>()?,
            ));
            let response = search.oneshot(req).await?;
            let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
            assert!(html.contains(&format!("/{locale}/pages/{slug}")));
        }
    }
    form.body = "Changed source".into();
    persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    form.published_at = "2099-01-01T00:00:00Z".into();
    persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(format!("/en/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.clone()
            .oneshot(Request::get("/en/pages/rules").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    form.title.clear();
    assert!(
        persist_current(&pool, &auth, ctx, &slug, "en", &form)
            .await
            .is_err()
    );
    form.title = "Delete title".into();
    form.action = "delete".into();
    persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale='en'").bind(&slug).fetch_one(&pool).await?;
    assert_eq!(remaining, 0);
    let (before, after): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT before,after FROM audit_log WHERE action='cms.page.deleted' AND target=$1",
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await?;
    assert_eq!(before["title"], "Safe title");
    assert_eq!(after, serde_json::Value::Null);
    form.action = "save".into();
    form.published_at = "2020-01-01T00:00:00Z".into();
    persist_current(&pool, &auth, ctx, &slug, "en", &form).await?;
    assert_eq!(
        router(pool.clone())
            .oneshot(Request::get(format!("/pl/pages/{slug}")).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert!(
        persist_current(&pool, &auth, ctx, "orphan-translation", "pl", &form)
            .await
            .is_err()
    );
    persist_current(&pool, &auth, ctx, "support", "en", &form).await?;
    assert_eq!(
        router(pool.clone())
            .oneshot(Request::get("/en/support").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::OK
    );

    form.action = "save".into();
    form.title = "Legal title".into();
    form.body = "Legal text".into();
    form.published_at = "2020-01-01T00:00:00Z".into();
    form.legal_version = "test-v1".into();
    form.legal_effective_date = "2020-01-01".into();
    persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await?;
    form.body = "Changed legal text".into();
    assert!(
        persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form)
            .await
            .is_err()
    );
    form.legal_version = "test-v2".into();
    persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await?;
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM page_legal_versions")
        .fetch_one(&pool)
        .await?;
    assert_eq!(versions, 2);
    use oteryn_admin::web::PanelConfig;
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(ctx.account)
        .execute(&pool)
        .await?;
    let identity = Identity(ctx.account);
    let staff = staff_router(
        pool.clone(),
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        identity.clone(),
        oteryn_admin::web::PortalIdentity::configured(pool.clone()),
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
    crate::test_staff::browser(&pool, ctx.account, &cookie).await?;
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
    let english = browser_page_hidden(staff.clone(), &cookie, "en").await?;
    let revision = page_hidden(&english, "expected_revision")?;
    assert_eq!(
        browser_page_submit(
            staff.clone(),
            &cookie,
            token,
            "en",
            &revision,
            "0",
            "English one"
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        browser_page_submit(
            staff.clone(),
            &cookie,
            token,
            "en",
            &revision,
            "0",
            "Stale English"
        )
        .await?,
        StatusCode::CONFLICT
    );
    let polish = browser_page_hidden(staff.clone(), &cookie, "pl").await?;
    let reviewed_source = page_hidden(&polish, "reviewed_source_revision")?;
    let translation_revision = page_hidden(&polish, "expected_revision")?;
    let english = browser_page_hidden(staff.clone(), &cookie, "en").await?;
    let revision = page_hidden(&english, "expected_revision")?;
    assert_eq!(
        browser_page_submit(
            staff.clone(),
            &cookie,
            token,
            "en",
            &revision,
            "0",
            "English two"
        )
        .await?,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        browser_page_submit(
            staff.clone(),
            &cookie,
            token,
            "pl",
            &translation_revision,
            &reviewed_source,
            "Polska strona"
        )
        .await?,
        StatusCode::CONFLICT
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug='new-page' AND e.locale='pl'").fetch_one(&pool).await?,0);
    assert_eq!(
        app.clone()
            .oneshot(Request::get("/pl/pages/new-page").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    eprintln!(
        "postgres_pages_publication_and_legal_history duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
#[tokio::test]
async fn staff_off_and_wrong_host_refuse() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
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
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        Ok(oteryn_admin::rbac::AccountState::Active)
    }
}

async fn persist_current(
    pool: &PgPool,
    auth: &oteryn_admin::rbac::MutationAuthorization,
    context: oteryn_admin::rbac::CheckContext,
    slug: &str,
    locale: &str,
    form: &PageForm,
) -> Result<(), oteryn_admin::rbac::Error> {
    // Model reopening the editor before each independent lifecycle action.
    let mut form = form.clone();
    form.expected_revision = sqlx::query_scalar("SELECT e.updated_at::text FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale=$2")
        .bind(slug).bind(locale).fetch_optional(pool).await?.unwrap_or_default();
    form.reviewed_source_revision = sqlx::query_scalar("SELECT e.revision FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale='en'")
        .bind(slug).fetch_optional(pool).await?.unwrap_or(0);
    persist(pool, auth, context, slug, locale, &form).await
}

async fn browser_page_hidden(
    app: Router,
    cookie: &str,
    locale: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::get(format!("/admin/pages/new-page/{locale}"))
                .header("host", "staff.test")
                .header("cookie", cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 200000).await?.to_vec(),
    )?)
}
fn page_hidden(html: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(html
        .split(&format!("name=\"{name}\" value=\""))
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("missing form revision")?
        .to_owned())
}
async fn browser_page_submit(
    app: Router,
    cookie: &str,
    csrf: &str,
    locale: &str,
    version: &str,
    source: &str,
    title: &str,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let version = version.replace('+', "%2B").replace(' ', "%20");
    Ok(app.oneshot(Request::post(format!("/admin/pages/new-page/{locale}")).header("host","staff.test").header("origin","https://staff.test").header("cookie",cookie).header("content-type","application/x-www-form-urlencoded")
        .body(Body::from(format!("_csrf={csrf}&expected_revision={version}&reviewed_source_revision={source}&title={title}&body={title}&action=save&published_at=2020-01-01T00:00:00Z")))?).await?.status())
}
