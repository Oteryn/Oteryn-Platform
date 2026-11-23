#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, header},
};
use tower::ServiceExt;
#[test]
fn validation_and_escaping() {
    let mut input = Input {
        slug: "valid-news".into(),
        title: "Title".into(),
        body: "Body".into(),
        ..Input::default()
    };
    assert!(input.valid());
    input.slug = "../bad".into();
    assert!(!input.valid());
    assert_eq!(escape("<script>\"&"), "&lt;script&gt;&quot;&amp;");
    assert_eq!(label("pl", "save"), "Zapisz");
    assert_eq!(label("en", "save"), "Save");
}
#[test]
fn tags_have_bounded_canonical_form() {
    assert_eq!(
        parse_tags(" fixes, release-2, fixes ").unwrap(),
        ["fixes", "release-2"]
    );
    assert!(parse_tags(" ").unwrap().is_empty());
    for invalid in ["fixes,,release", "Release", "bad--tag", "a/b", "a_1"] {
        assert_eq!(parse_tags(invalid), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
    assert!(parse_tags(&"a".repeat(MAX_TAG_LENGTH + 1)).is_err());
    assert!(parse_tags(&["a"; MAX_TAGS + 1].join(",")).is_err());
}

#[tokio::test]
async fn postgres_news_lifecycle() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL news test: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t11_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
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
        .await
        .unwrap();
    oteryn_db::MIGRATOR
        .run_to(20261122200000, &pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO news(slug,title,body,published_at)
         VALUES ('undated-legacy-news','Legacy publication','Older content','0001-01-01 00:00:00+00 BC')",
    )
    .execute(&pool)
    .await
    .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let slug = format!("news-{}", Uuid::now_v7());
    let mut context = CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: None,
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(context.account).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'editor-test-password','active','en',now())")
        .bind(context.account).bind(format!("{}@example.invalid",context.account)).execute(&pool).await.unwrap();
    context.session_proof = Some(
        crate::test_staff::proof(&pool, context.account)
            .await
            .unwrap(),
    );
    let auth =
        MutationAuthorization::new(std::sync::Arc::new(oteryn_admin::rbac::Authorizer::new(
            pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
        )));
    let make = || Input {
        slug: slug.clone(),
        title: "<script>Title</script>".into(),
        body: "<img src=x onerror=alert(1)>".into(),
        published_at: "2020-01-01T00:00:00Z".into(),
        ..Input::default()
    };
    let mut invalid_tags = make();
    invalid_tags.tags = Some("release,,fixes".into());
    assert_eq!(
        save(&pool, &auth, context, None, invalid_tags)
            .await
            .unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut release = make();
    release.slug = "release-2".into();
    release.kind = Some(Kind::Changelog);
    release.tags = Some("updates, release-2".into());
    release.title = "Release 2".into();
    release.body = "Fixes 100% of the reported issues".into();
    save(&pool, &auth, context, None, release).await.unwrap();
    let mut legacy_edit = make();
    legacy_edit.slug = "release-2".into();
    legacy_edit.title = "Release 2 revised".into();
    legacy_edit.body = "Fixes 100% of the reported issues".into();
    let release_id: Uuid = sqlx::query_scalar("SELECT id FROM news WHERE slug=$1")
        .bind(&legacy_edit.slug)
        .fetch_one(&pool)
        .await
        .unwrap();
    save_current(&pool, &auth, context, Some(release_id), legacy_edit)
        .await
        .unwrap();
    let (is_changelog, tags): (bool, Vec<String>) =
        sqlx::query_as("SELECT is_changelog,tags FROM news WHERE id=$1")
            .bind(release_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(is_changelog);
    assert_eq!(tags, ["release-2", "updates"]);
    let mut invalid_date = make();
    invalid_date.published_at = "0001-01-01 00:00:00+00 BC".into();
    assert_eq!(
        save(&pool, &auth, context, None, invalid_date)
            .await
            .unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let app = public_router_at(pool.clone(), "https://portal.example.test/")
        .layer(Extension(axum::extract::ConnectInfo(news_test_peer())));
    let (status, html) = get_public(&app, "/en/changelog?q=FIXES%20100%25").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(r#"href="/en/news/release-2""#));
    let (status, html) = get_public(&app, "/en/changelog?q=%25%25").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("Release 2"));
    let (status, html) = get_public(&app, "/pl/changelog").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("Release 2"));
    let (status, html) = get_public(&app, "/en/news").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("/en/news/undated-legacy-news"));
    assert!(!html.contains("<time"));
    assert!(!html.contains("Release 2"));
    let (status, rss) = get_public(&app, "/en/news/rss.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(rss.contains("<link>https://portal.example.test/en/news</link>"));
    assert!(rss.contains("<link>https://portal.example.test/en/news/undated-legacy-news</link>"));
    assert!(!rss.contains("<pubDate>"));
    assert!(!rss.contains("Release 2"));
    for invalid in [
        "/en/news?year=0",
        "/en/news?month=13",
        "/en/news?year=%2B2020",
        "/en/news?tag=Bad",
        "/en/changelog?page=100001",
        "/en/changelog?q=bad%00query",
    ] {
        assert_eq!(
            get_public(&app, invalid).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let op_pool = pool.clone();
    let op_auth = auth.clone();
    let mut input = make();
    input.tags = Some("launch".into());
    crate::test_staff::race_account_before_fence(&pool, context.account, async move {
        save(&op_pool, &op_auth, context, None, input).await
    })
    .await
    .unwrap()
    .unwrap();
    let id: Uuid = sqlx::query_scalar("SELECT id FROM news WHERE slug=$1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (_, html) = get_public(&app, "/en/news?year=2020&month=1&tag=launch").await;
    assert!(html.contains(&slug) && !html.contains("<script>Title</script>"));
    for filter in ["year=2021", "month=2", "tag=missing"] {
        let (status, html) = get_public(&app, &format!("/en/news?{filter}")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!html.contains(&slug));
    }
    let (_, rss) = get_public(&app, "/en/news/rss.xml").await;
    assert!(rss.contains(&format!(
        "<link>https://portal.example.test/en/news/{slug}</link>"
    )));
    assert!(rss.contains("<category>launch</category>"));
    assert!(rss.contains("<pubDate>Wed, 01 Jan 2020 00:00:00 +0000</pubDate>"));
    assert!(rss.contains("<title>&lt;script&gt;Title&lt;/script&gt;</title>"));
    let (status, rss) = get_public(&app, "/pl/news/rss.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!rss.contains(&slug));
    let revision: String = sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut first = make();
    first.expected_revision = revision.clone();
    save(&pool, &auth, context, Some(id), first).await.unwrap();
    let mut stale = make();
    stale.expected_revision = revision;
    stale.title = "Stale editor overwrite".into();
    assert_eq!(
        save(&pool, &auth, context, Some(id), stale)
            .await
            .unwrap_err(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        article(&pool, "pl", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let html = article(&pool, "en", &slug).await.unwrap().0;
    assert!(!html.contains("<script>"));
    assert!(html.contains("Title"));
    assert!(!html.contains("<img"));
    let mut input = make();
    input.title_pl = "Tytuł".into();
    input.body_pl = "Treść".into();
    input.translation_published_at = "2020-01-01T00:00:00Z".into();
    let _ = save_current(&pool, &auth, context, Some(id), input)
        .await
        .unwrap();
    assert!(
        article(&pool, "pl", &slug)
            .await
            .unwrap()
            .0
            .contains("Treść")
    );
    assert_eq!(
        save_current(&pool, &auth, context, None, make())
            .await
            .unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut changed = make();
    changed.title = "Changed source".into();
    changed.title_pl = "Tytuł".into();
    changed.body_pl = "Treść".into();
    changed.translation_published_at = "2020-01-01T00:00:00Z".into();
    let _ = save_current(&pool, &auth, context, Some(id), changed)
        .await
        .unwrap();
    let mut unchanged = make();
    unchanged.title = "Changed source".into();
    unchanged.title_pl = "Tytuł".into();
    unchanged.body_pl = "Treść".into();
    unchanged.translation_published_at = "2020-01-01T00:00:00Z".into();
    let _ = save_current(&pool, &auth, context, Some(id), unchanged)
        .await
        .unwrap();
    assert_eq!(
        article(&pool, "pl", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    // A new approval stamp must win over source-only invalidation.
    let mut renewed = make();
    renewed.title = "Renewed source".into();
    renewed.title_pl = "Tytuł".into();
    renewed.body_pl = "Treść".into();
    renewed.translation_published_at = "2020-01-02T00:00:00Z".into();
    save_current(&pool, &auth, context, Some(id), renewed)
        .await
        .unwrap();
    assert!(article(&pool, "pl", &slug).await.is_ok());
    let mut reused = make();
    reused.title = "Another source change".into();
    reused.title_pl = "Tytuł".into();
    reused.body_pl = "Treść".into();
    reused.translation_published_at = "2020-01-02T00:00:00Z".into();
    save_current(&pool, &auth, context, Some(id), reused)
        .await
        .unwrap();
    assert_eq!(
        article(&pool, "pl", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let mut invalid = make();
    invalid.published_at = "invalid".into();
    assert_eq!(
        save_current(&pool, &auth, context, Some(id), invalid)
            .await
            .unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    sqlx::query(
        "UPDATE news SET translation_source_updated_at=updated_at-interval '1 second' WHERE id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        article(&pool, "pl", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE news SET published_at=now()+interval '1 day' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        article(&pool, "en", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let (source, original): (i64, String) = sqlx::query_as(
        "SELECT id,snapshot->>'body' FROM editorial_revisions
         WHERE kind='news' AND object_key=$1 ORDER BY id LIMIT 1",
    )
    .bind(id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    let taken_slug = format!("taken-{id}");
    let mut renamed = make();
    renamed.slug = taken_slug.clone();
    save_current(&pool, &auth, context, Some(id), renamed)
        .await
        .unwrap();
    save_current(&pool, &auth, context, None, make())
        .await
        .unwrap();
    let current: String = sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let attempted = Edit::Restore {
        source,
        form: revisions::Restore {
            expected_revision: current,
            reviewed_source_revision: 0,
        },
    };
    assert_eq!(
        save_change(&pool, &auth, context, Some(id), attempted)
            .await
            .unwrap_err(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT slug FROM news WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        taken_slug
    );
    sqlx::query("DELETE FROM news WHERE slug=$1")
        .bind(&slug)
        .execute(&pool)
        .await
        .unwrap();
    let expected_revision: String =
        sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let restore = |source| Edit::Restore {
        source,
        form: revisions::Restore {
            expected_revision: expected_revision.clone(),
            reviewed_source_revision: 0,
        },
    };
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM editorial_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        save_change(&pool, &auth, context, Some(id), restore(-1))
            .await
            .unwrap_err(),
        StatusCode::BAD_REQUEST
    );

    let role: Uuid =
        sqlx::query_scalar("DELETE FROM account_roles WHERE account_id=$1 RETURNING role_id")
            .bind(context.account)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(context.account)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        save_change(&pool, &auth, context, Some(id), restore(source))
            .await
            .unwrap_err(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(context.account)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        save_change(&pool, &auth, context, Some(id), restore(source))
            .await
            .unwrap()
            .status(),
        StatusCode::FOUND
    );
    let (editor, restored_from, body, draft): (Uuid, i64, String, bool) = sqlx::query_as(
        "SELECT actor,restored_from,snapshot->>'body',snapshot->>'published_at' IS NULL
         FROM editorial_revisions WHERE kind='news' AND object_key=$1 ORDER BY id DESC LIMIT 1",
    )
    .bind(id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (editor, restored_from, body, draft),
        (context.account, source, original, true)
    );
    assert_eq!(
        save_change(&pool, &auth, context, Some(id), restore(source))
            .await
            .unwrap_err(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM editorial_revisions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before + 1
    );
    let guarded = oteryn_admin::web::guarded(
        admin_router(pool.clone()),
        pool.clone(),
        oteryn_admin::web::PanelConfig::default(),
        oteryn_admin::web::PendingAuthentication,
        oteryn_admin::web::PendingAuthentication,
        Permission::ManageNews,
    );
    let response = guarded
        .oneshot(
            Request::builder()
                .uri("/admin/news")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let _ = delete(
        State(pool.clone()),
        Extension(context),
        Extension(auth.clone()),
        Path(id),
        Form(Revision {
            expected_revision: sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap(),
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        article(&pool, "en", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(id),
            Form(Revision {
                expected_revision: String::new()
            })
        )
        .await
        .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/en/news?page=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()[header::CONTENT_LANGUAGE], "en");
    let body = to_bytes(response.into_body(), 100_000).await.unwrap();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(html.contains(r#"role="alert">Choose a valid result page.</p>"#));

    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(context.account)
        .execute(&pool)
        .await
        .unwrap();
    let mut denied_release = make();
    denied_release.kind = Some(Kind::Changelog);
    assert_eq!(
        save(&pool, &auth, context, None, denied_release)
            .await
            .unwrap_err(),
        StatusCode::FORBIDDEN
    );
    let role: Uuid = sqlx::query_scalar("INSERT INTO roles(key) VALUES ('news_test') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(context.account)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    let guarded = || {
        oteryn_admin::web::guarded(
            admin_router(pool.clone()),
            pool.clone(),
            oteryn_admin::web::PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            Identity(context.account),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
            Permission::ManageNews,
        )
    };
    let request = || {
        Request::builder()
            .uri("/admin/news/create?lang=en")
            .header("host", "staff.test")
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(
        guarded().oneshot(request()).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'cms.news.manage')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    let response = guarded().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
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
    assert!(body.contains("name=\"_csrf\""));
    let token = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    crate::test_staff::browser(&pool, context.account, &cookie)
        .await
        .unwrap();
    let mutation = |token: &str| {
        Request::builder()
            .method("POST")
            .uri("/admin/news")
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", &cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!(
                "_csrf={token}&slug=browser-news&title=Browser&body=Body&published_at="
            )))
            .unwrap()
    };
    assert_eq!(
        guarded().oneshot(mutation("bad")).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        guarded().oneshot(mutation(token)).await.unwrap().status(),
        StatusCode::FOUND
    );
    let browser_id: Uuid = sqlx::query_scalar("SELECT id FROM news WHERE slug='browser-news'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let edit_path = format!("/admin/news/{browser_id}/edit");
    let update_path = format!("/admin/news/{browser_id}");
    let stale = browser_news_revision(guarded(), &cookie, &edit_path).await;
    assert_eq!(
        browser_news_submit(
            guarded(),
            &cookie,
            token,
            &update_path,
            &stale,
            "First editor"
        )
        .await,
        StatusCode::FOUND
    );
    assert_eq!(
        browser_news_submit(
            guarded(),
            &cookie,
            token,
            &update_path,
            &stale,
            "Stale overwrite"
        )
        .await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        browser_news_submit(
            guarded(),
            &cookie,
            token,
            &format!("{update_path}/delete"),
            &stale,
            "Stale delete"
        )
        .await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT title FROM news WHERE id=$1")
            .bind(browser_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "First editor"
    );
    let current = browser_news_revision(guarded(), &cookie, &edit_path).await;
    assert_eq!(
        browser_news_submit(
            guarded(),
            &cookie,
            token,
            &format!("{update_path}/delete"),
            &current,
            "Reviewed delete"
        )
        .await,
        StatusCode::FOUND
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    eprintln!("postgres_news_lifecycle duration: {:?}", started.elapsed());
}

#[derive(Clone)]
struct Identity(Uuid);
impl oteryn_admin::web::StaffSession for Identity {
    async fn resolve(&self, _: &BrowserSession) -> Option<oteryn_admin::web::StaffIdentity> {
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

#[test]
fn news_document_keeps_body_in_the_selected_theme() {
    for lang in ["pl", "en"] {
        let html = page(lang, "<Untrusted title>", "<p>Article</p>".into()).0;
        assert!(html.contains(&format!("lang=\"{lang}\"")));
        assert!(html.contains("</head><body class=\"op-theme\">"));
        assert!(!html.contains("<Untrusted title>"));
        assert!(html.contains("Untrusted title"));
        assert!(html.ends_with("</main></body></html>"));
    }
}
#[test]
fn home_excerpt_and_date() {
    assert_eq!(super::excerpt("a  b\nc", 10), "a b c");
    let long = "słowo ".repeat(40);
    let cut = super::excerpt(&long, 30);
    assert!(cut.ends_with('…') && cut.chars().count() <= 31);
    assert_eq!(super::display_date("pl", "2026-11-07"), "07.11.2026");
    assert_eq!(super::display_date("en", "2026-11-07"), "2026-11-07");
}

#[tokio::test]
async fn postgres_pagination_language_and_caps() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")
        .expect("PORTAL_TEST_DATABASE_URL required for pagination regression");
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t11_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
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
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();

    sqlx::query("INSERT INTO news(slug,title,body,published_at) SELECT 'pagination-'||n,'English news','English body',now() FROM generate_series(1,26) n").execute(&pool).await.unwrap();
    let app =
        public_router(pool.clone()).layer(Extension(axum::extract::ConnectInfo(news_test_peer())));
    let response = app
        .clone()
        .oneshot(Request::get("/news?lang=en").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        html.replace("&#38;", "&amp;")
            .contains(r#"href="?page=2&amp;lang=en" rel="next""#)
    );
    assert!(html.contains("/en/news/pagination-26"));
    assert!(!html.contains("/en/news/pagination-16"));
    let response = app
        .clone()
        .oneshot(
            Request::get("/news?page=2&lang=en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("English news"));
    assert!(html.contains("/en/news/pagination-16"));
    assert!(!html.contains("/en/news/pagination-26"));
    let budget = ratelimit::NamedPolicy::PublicGameData;
    sqlx::query(
        "WITH filled AS (
             UPDATE web_rate_limits SET count=$1 WHERE limiter_name=$2
             RETURNING limiter_name,key_hash,window_start
         )
         INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         SELECT limiter_name,key_hash,window_start+make_interval(secs=>$3),$1
           FROM filled
         ON CONFLICT DO NOTHING",
    )
    .bind(budget.limit())
    .bind(budget.name())
    .bind(budget.window_seconds() as f64)
    .execute(&pool)
    .await
    .unwrap();
    let response = app
        .oneshot(
            Request::get("/en/changelog?q=release")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
    sqlx::query(
        "INSERT INTO news(slug,title,body,published_at)
         VALUES('long-horizon','Long horizon','English body','12000-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let response = admin_listing(
        &pool,
        Options {
            lang: Some("en".into()),
            ..Options::default()
        },
        "en",
        false,
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = to_bytes(response.into_body(), 100000).await.unwrap();
    let html = String::from_utf8_lossy(&html);
    assert!(html.contains("Long horizon"));
    assert!(html.contains("Scheduled"));

    for number in [100000, 4] {
        let q = Options {
            lang: Some("en".into()),
            page: Some(number),
            ..Options::default()
        };
        let peer = ([127, 0, 0, 2], 12345).into();
        let archive = public_router(pool.clone()).layer(Extension(axum::extract::ConnectInfo::<
            std::net::SocketAddr,
        >(peer)));
        let (status, html) = get_public(&archive, &format!("/en/news?page={number}")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!html.contains(r#"rel="next""#));
        assert!(
            !public::listing(&pool, "en", &q)
                .await
                .unwrap()
                .0
                .contains("?page=")
        );
        let response = admin_listing(&pool, q, "en", false).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let html = to_bytes(response.into_body(), 100000).await.unwrap();
        assert!(!String::from_utf8_lossy(&html).contains("?page="));
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    eprintln!(
        "postgres_pagination_language_and_caps duration: {:?}",
        started.elapsed()
    );
}

fn news_test_peer() -> std::net::SocketAddr {
    ([127, 0, 0, 1], 12345).into()
}

async fn get_public(app: &Router, path: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (
        response.status(),
        String::from_utf8(
            to_bytes(response.into_body(), 100_000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap(),
    )
}

async fn save_current(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Option<Uuid>,
    mut input: Input,
) -> Result<Response, StatusCode> {
    if let Some(id) = id {
        input.expected_revision =
            sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
                .bind(id)
                .fetch_one(pool)
                .await
                .map_err(unavailable)?;
    }
    save(pool, auth, context, id, input).await
}

async fn browser_news_revision(app: Router, cookie: &str, path: &str) -> String {
    let response = app
        .oneshot(
            Request::get(path)
                .header("host", "staff.test")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    html.split("name=\"expected_revision\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned()
}
async fn browser_news_submit(
    app: Router,
    cookie: &str,
    csrf: &str,
    path: &str,
    version: &str,
    title: &str,
) -> StatusCode {
    let version = version.replace('+', "%2B").replace(' ', "%20");
    app.oneshot(Request::post(path).header("host","staff.test").header("origin","https://staff.test").header("cookie",cookie).header("content-type","application/x-www-form-urlencoded")
        .body(Body::from(format!("_csrf={csrf}&expected_revision={version}&slug=browser-news&title={title}&body=Reviewed+body&published_at="))).unwrap()).await.unwrap().status()
}

#[tokio::test]
async fn article_discussion_post_requires_browser_csrf() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgresql://localhost/unused")
        .unwrap();
    let response = configured_router(pool)
        .unwrap()
        .oneshot(
            Request::post("/en/news/published/discussion")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("body=Reply"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
