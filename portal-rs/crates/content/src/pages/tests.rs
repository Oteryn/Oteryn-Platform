use super::staff::{LegalEdition, PageForm, SaveError, persist, publish_legal_edition};
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_admin::rbac::Error;
use oteryn_identity::consents::ConsentConfig;
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
async fn pages_database() -> Result<Option<(PgPool, PgPool, String)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL pages test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(None);
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
    Ok(Some((root, pool, schema)))
}
async fn drop_pages_schema(root: PgPool, pool: PgPool, schema: &str) -> Result<(), sqlx::Error> {
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}

#[tokio::test]
async fn relabeled_placeholder_pages_show_localized_draft_notice()
-> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL is required for the legal renderer test")?;
    for profile in [None, Some("production"), Some("preprod")] {
        assert!(matches!(
            legal::install(&pool, profile).await,
            Err(oteryn_admin::rbac::Error::InvalidInput)
        ));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pages")
            .fetch_one(&pool)
            .await?,
        0
    );
    assert_eq!(legal::install(&pool, Some("testing")).await?, 3);
    let app = router(pool.clone());
    for (english, polish, visible) in [
        ("KRS 0000000000", "KRS 0000000000", "KRS 0000000000"),
        (
            "Tax ID (NIP) (000) 000-00-00",
            "NIP (000) 000-00-00",
            "(000) 000-00-00",
        ),
        (
            "Tax ID (NIP) <strong>000</strong>&#8209;000&#x2011;00‑00",
            "NIP 000‑000‑00‑00",
            "‑00",
        ),
        (
            "Service terms.\n\n![Tax ID (NIP) 000-000-00-00](/missing-image.png)",
            "Warunki świadczenia usług.\n\n<img src='/missing-image.png' alt='NIP 000-000-00-00'>",
            "000-000-00-00",
        ),
        (
            "Tax ID (NIP) PL 123-456-78-03",
            "NIP 123-456-78-03",
            "123-456-78-03",
        ),
        (
            "Operator: {{operator}}",
            "Dane operatora: {{operator}}",
            "{{operator}}",
        ),
    ] {
        sqlx::query(
            "UPDATE editorial_pages SET legal_version='relabeled-v1',
             body=CASE WHEN locale='pl' THEN $1 ELSE $2 END",
        )
        .bind(polish)
        .bind(english)
        .execute(&pool)
        .await?;
        for (locale, notice) in [
            ("en", "Draft — operator data will be replaced."),
            ("pl", "Wersja robocza — dane operatora zostaną zastąpione."),
        ] {
            for path in ["terms", "privacy", "cookies"] {
                let response = app
                    .clone()
                    .oneshot(Request::get(format!("/{locale}/legal/{path}")).body(Body::empty())?)
                    .await?;
                assert_eq!(response.status(), StatusCode::OK);
                let html =
                    String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
                assert!(html.contains("role=\"note\""));
                assert!(html.contains(notice));
                assert!(
                    html.contains(visible),
                    "operator identifier stays in the rendered page"
                );
            }
        }
    }
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn postgres_installer_preserves_preexisting_draft_and_history_and_audits_once()
-> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL is required for the legal installer test")?;
    let draft: Uuid = sqlx::query_scalar(
        "INSERT INTO pages(id,slug) VALUES (uuidv7(),'terms-of-service') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO editorial_pages(page_id,locale,title,body)
         VALUES ($1,'en','Staff draft','Staff draft body')",
    )
    .bind(draft)
    .execute(&pool)
    .await?;
    let history: Uuid = sqlx::query_scalar(
        "INSERT INTO pages(id,slug) VALUES (uuidv7(),'privacy-policy') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let snapshot = serde_json::json!({"title":"Archived privacy notice","body":"Prior staff content","effective":"2026-11-05"});
    sqlx::query(
        "INSERT INTO page_legal_versions(page_id,locale,version,content) VALUES ($1,'pl','staff-v1',$2)",
    )
    .bind(history)
    .bind(&snapshot)
    .execute(&pool)
    .await?;

    for installed in [1, 0, 0] {
        assert_eq!(legal::install(&pool, Some("testing")).await?, installed);
        let editions: Vec<(String, String, String, bool)> = sqlx::query_as(
            "SELECT locale,title,body,published_at IS NULL FROM editorial_pages
             WHERE page_id IN ($1,$2)",
        )
        .bind(draft)
        .bind(history)
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            editions,
            [(
                "en".to_owned(),
                "Staff draft".to_owned(),
                "Staff draft body".to_owned(),
                true
            )]
        );
        let versions: Vec<(Uuid, String, String, serde_json::Value)> = sqlx::query_as(
            "SELECT page_id,locale,version,content FROM page_legal_versions WHERE page_id IN ($1,$2)",
        )
        .bind(draft)
        .bind(history)
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            versions,
            [(
                history,
                "pl".to_owned(),
                "staff-v1".to_owned(),
                snapshot.clone()
            )]
        );
        let audited: Vec<String> = sqlx::query_scalar(
            "SELECT target FROM audit_log WHERE action='cms.legal.defaults_installed'",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(audited, ["cookie-policy"]);
    }
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}
#[tokio::test]
async fn postgres_pages_publication_and_legal_history() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let Some((root, pool, schema)) = pages_database().await? else {
        return Ok(());
    };
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
            if matches!(path, "help" | "support/report-a-bug") {
                assert!(html.contains(&format!("href=\"/{lang}/support/tickets/create\"")));
                assert!(html.contains(&format!("href=\"/{lang}/forgot-password\"")));
            } else {
                assert!(html.contains(message));
            }
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(!html.contains("Changed legal text"));
        }
    }

    // A bare slug is represented by a page with no locale content.
    sqlx::query("INSERT INTO pages(id,slug) VALUES($1,'cookie-policy')")
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await?;
    assert_eq!(legal::install(&pool, Some("testing")).await?, 3);
    assert_eq!(legal::install(&pool, Some("testing")).await?, 0);
    let audited: Vec<String> = sqlx::query_scalar(
        "SELECT target FROM audit_log WHERE action='cms.legal.defaults_installed' ORDER BY target",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        audited,
        ["cookie-policy", "privacy-policy", "terms-of-service"]
    );
    for (lang, note, titles) in [
        (
            "en",
            "Draft — operator details are placeholders.",
            ["Terms of Service", "Privacy Policy", "Cookie Policy"],
        ),
        (
            "pl",
            "Wersja robocza — dane operatora są tymczasowe.",
            [
                "Regulamin Oteryn",
                "Polityka prywatności",
                "Polityka plików cookie",
            ],
        ),
    ] {
        for (path, title) in ["terms", "privacy", "cookies"].into_iter().zip(titles) {
            let response = app
                .clone()
                .oneshot(Request::get(format!("/{lang}/legal/{path}")).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
            assert!(html.contains(&format!("<h1>{title}</h1>")));
            assert!(html.contains(&format!("<strong>{note}</strong>")));
            assert!(html.contains("2026-11-22"));
            assert!(html.contains("0000000000"));
            assert!(!html.contains("{{operator}}"));
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
    form.final_approved = "yes".into();
    for (locale, body) in [
        ("en", "Tax ID (NIP) **000**&#8209;000&#x2011;00‑00"),
        ("pl", "NIP 000‑000‑00‑00"),
        ("en", "KRS 0000000000"),
        ("en", "Tax ID (NIP) (000) 000-00-00"),
        ("pl", "NIP (000) 000-00-00"),
        (
            "en",
            "Service terms.\n\n![Tax ID (NIP) 000-000-00-00](/missing-image.png)",
        ),
        (
            "pl",
            "Warunki świadczenia usług.\n\n<img src='/missing-image.png' alt='NIP 000-000-00-00'>",
        ),
        ("en", "Tax ID (NIP) PL 123-456-78-03"),
        ("pl", "NIP 123-456-78-03"),
        ("en", "Operator: {{operator}}"),
        ("pl", "Dane operatora: {{operator}}"),
        ("en", "<!-- Terms draft -->"),
        ("pl", "<!-- Projekt regulaminu -->"),
        ("en", "&#xfe0f;"),
        ("pl", "&#x034f;"),
    ] {
        form.body = body.into();
        assert!(matches!(
            persist_current(&pool, &auth, ctx, "terms-of-service", locale, &form).await,
            Err(oteryn_admin::rbac::Error::InvalidInput)
        ));
    }
    form.body = "Changed legal text. Tax ID (NIP) 123-456-78-02".into();
    persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await?;
    form.final_approved.clear();
    persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await?;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT content @> '{\"final_approved\":true}'::jsonb
         FROM page_legal_versions WHERE locale='en' AND version='test-v2'",
        )
        .fetch_one(&pool)
        .await?
    );
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM page_legal_versions")
        .fetch_one(&pool)
        .await?;
    assert_eq!(versions, 8);
    assert_eq!(legal::install(&pool, Some("testing")).await?, 0);
    let edited: String = sqlx::query_scalar(
        "SELECT e.body FROM editorial_pages e JOIN pages p ON p.id=e.page_id
         WHERE p.slug='terms-of-service' AND e.locale='en'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(edited, form.body);
    form.body.push_str(" Another change.");
    assert!(matches!(
        persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await,
        Err(oteryn_admin::rbac::Error::InvalidInput)
    ));
    form.body = edited;
    form.legal_version = "test-v3".into();
    persist_current(&pool, &auth, ctx, "terms-of-service", "en", &form).await?;
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT content @> '{\"final_approved\":true}'::jsonb
             FROM page_legal_versions WHERE locale='en' AND version='test-v3'",
        )
        .fetch_one(&pool)
        .await?
    );
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
    drop_pages_schema(root, pool, &schema).await?;
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

async fn legal_operator(pool: &PgPool, role: &str) -> Result<Uuid, sqlx::Error> {
    let actor = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
         VALUES($1,$2,'editor-test-password','active','en',now())",
    )
    .bind(actor)
    .bind(format!("{actor}@example.invalid"))
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
    )
    .bind(actor)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(actor)
}

fn legal_edition(slug: &str, locale: &str, version: &str) -> LegalEdition {
    LegalEdition {
        slug: slug.into(),
        locale: locale.into(),
        title: if locale == "en" {
            "Legal edition"
        } else {
            "Dokument prawny"
        }
        .into(),
        body: if locale == "en" {
            "Published service documents."
        } else {
            "Opublikowane dokumenty serwisu."
        }
        .into(),
        version: version.into(),
        effective: "2026-11-23".into(),
    }
}

#[tokio::test]
async fn operator_publishes_bilingual_signup_editions() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let actor = legal_operator(&pool, "content_editor").await?;
    let terms = "terms-test-edition";
    let privacy = "privacy-test-edition";
    let consents = ConsentConfig::published(terms.into(), privacy.into(), false);
    assert!(consents.check_publication(&pool, "en").await.is_err());
    for (slug, version) in [("terms-of-service", terms), ("privacy-policy", privacy)] {
        for locale in ["en", "pl"] {
            publish_legal_edition(&pool, actor, legal_edition(slug, locale, version)).await?;
        }
    }
    // The check-signup-legal role runs this final-only check for both locales.
    let mut tx = pool.begin().await?;
    for locale in ["en", "pl"] {
        consents.check_publication(&mut *tx, locale).await?;
    }
    tx.commit().await?;
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn operator_refuses_polish_before_english() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let actor = legal_operator(&pool, "content_editor").await?;
    let edition = legal_edition("terms-of-service", "pl", "terms-test-edition");
    assert!(matches!(
        publish_legal_edition(&pool, actor, edition).await,
        Err(SaveError::MissingSource)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pages")
            .fetch_one(&pool)
            .await?,
        0
    );
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn operator_refuses_placeholder_text() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let actor = legal_operator(&pool, "content_editor").await?;
    for body in [
        "Operator: {{operator}}",
        "Legal text.\n\n<!-- {{operator}} -->",
        "Tax ID (NIP) 000-000-00-00",
    ] {
        let mut edition = legal_edition("terms-of-service", "en", "terms-test-edition");
        edition.body = body.into();
        assert!(matches!(
            publish_legal_edition(&pool, actor, edition).await,
            Err(SaveError::InvalidForm)
        ));
    }
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn operator_requires_an_account_and_both_grants() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let without_pages = legal_operator(&pool, "moderator").await?;
    sqlx::query("INSERT INTO roles(key) VALUES('test_page_only')")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'cms.pages.manage' FROM roles WHERE key='test_page_only'")
        .execute(&pool).await?;
    let without_access = legal_operator(&pool, "test_page_only").await?;
    for actor in [without_pages, without_access, Uuid::now_v7()] {
        let edition = legal_edition("terms-of-service", "en", "terms-test-edition");
        assert!(matches!(
            publish_legal_edition(&pool, actor, edition).await,
            Err(SaveError::Authorization(Error::Denied))
        ));
    }
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn operator_cannot_rewrite_an_immutable_edition() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let actor = legal_operator(&pool, "content_editor").await?;
    let version = "terms-test-edition";
    let edition = legal_edition("terms-of-service", "en", version);
    let original_body = edition.body.clone();
    publish_legal_edition(&pool, actor, edition).await?;
    let mut changed = legal_edition("terms-of-service", "en", version);
    changed.body.push_str(" Revised text.");
    assert!(matches!(
        publish_legal_edition(&pool, actor, changed).await,
        Err(SaveError::ImmutableVersion)
    ));
    let (body, snapshot): (String, String) = sqlx::query_as(
        "SELECT e.body, v.content->>'body' FROM editorial_pages e JOIN page_legal_versions v
         ON v.page_id=e.page_id AND v.locale=e.locale AND v.version=e.legal_version",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(body, original_body);
    assert_eq!(snapshot, original_body);
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn operator_save_audits_actor_and_publication_time() -> Result<(), Box<dyn std::error::Error>>
{
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("PostgreSQL required")?;
    let actor = legal_operator(&pool, "content_editor").await?;
    let edition = legal_edition("cookie-policy", "en", "cookies-test-edition");
    publish_legal_edition(&pool, actor, edition).await?;
    let audit: (Option<Uuid>, bool, bool) = sqlx::query_as(
        "SELECT a.actor, a.after @> '{\"locale\":\"en\",\"final_approved\":true}'::jsonb,
                e.published_at = a.created_at AND e.published_at = (a.after->>'published_at')::timestamptz
         FROM audit_log a JOIN pages p ON p.slug=a.target JOIN editorial_pages e ON e.page_id=p.id
         WHERE a.action='cms.page.saved' AND a.target='cookie-policy'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(audit, (Some(actor), true, true));
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn paired_page_window_requires_current_editions_and_editor_permission()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let (root, pool, schema) = pages_database().await?.ok_or("test database required")?;
    let editor = legal_operator(&pool, "content_editor").await?;
    let reader = legal_operator(&pool, "security_admin").await?;
    let context = |account| oteryn_admin::rbac::CheckContext {
        require_staff_mfa: false,
        ..oteryn_admin::rbac::CheckContext::new(account)
    };
    let auth = oteryn_admin::rbac::MutationAuthorization::new(std::sync::Arc::new(
        oteryn_admin::rbac::Authorizer::new(pool.clone(), Identity(editor)),
    ));
    let slug = "scheduled-launch";
    let draft = PageForm {
        title: "Launch".into(),
        body: "Launch information".into(),
        action: "save".into(),
        ..PageForm::default()
    };
    persist_current(&pool, &auth, context(editor), slug, "en", &draft).await?;
    let revision = || async {
        sqlx::query_as::<_, (String, String)>(
            "SELECT en.updated_at::text,coalesce(pl.updated_at::text,'')
             FROM pages p JOIN editorial_pages en ON en.page_id=p.id AND en.locale='en'
             LEFT JOIN editorial_pages pl ON pl.page_id=p.id AND pl.locale='pl'
             WHERE p.slug=$1",
        )
        .bind(slug)
        .fetch_one(&pool)
        .await
    };
    let mut form = PageForm {
        schedule_revision: revision().await?.0,
        schedule_at: "2030-05-10T12:00:00+02:00".into(),
        schedule_until: "2030-05-10T13:00:00+02:00".into(),
        action: "schedule".into(),
        ..PageForm::default()
    };
    assert!(matches!(
        persist(&pool, &auth, context(editor), slug, "en", &form).await,
        Err(Error::InvalidInput)
    ));
    persist_current(&pool, &auth, context(editor), slug, "pl", &draft).await?;
    (form.schedule_revision, form.translation_revision) = revision().await?;
    assert!(matches!(
        persist(&pool, &auth, context(reader), slug, "en", &form).await,
        Err(Error::Denied)
    ));
    let mut reversed = form.clone();
    reversed.schedule_until = "2030-05-10T09:59:00Z".into();
    assert!(matches!(
        persist(&pool, &auth, context(editor), slug, "en", &reversed).await,
        Err(Error::InvalidInput)
    ));
    persist(&pool, &auth, context(editor), slug, "en", &form).await?;
    assert!(matches!(
        persist(&pool, &auth, context(editor), slug, "en", &form).await,
        Err(Error::InvalidInput)
    ));
    let scheduled: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM editorial_pages
         WHERE published_at='2030-05-10T10:00:00Z' AND expires_at='2030-05-10T11:00:00Z'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(scheduled, 2);
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='cms.page.scheduled' AND target=$1",
    )
    .bind(slug)
    .fetch_one(&pool)
    .await?;
    assert_eq!(audits, 1);
    drop_pages_schema(root, pool, &schema).await?;
    Ok(())
}
