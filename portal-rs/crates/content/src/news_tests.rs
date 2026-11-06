#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
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
#[tokio::test]
async fn postgres_news_lifecycle() {
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
    for migration in oteryn_db::MIGRATOR.iter() {
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration.sql.as_ref()))
            .execute(&pool)
            .await
            .unwrap();
    }
    let slug = format!("news-{}", Uuid::now_v7());
    let context = CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let make = || Input {
        slug: slug.clone(),
        title: "<script>Title</script>".into(),
        body: "<img src=x onerror=alert(1)>".into(),
        published_at: "2020-01-01T00:00:00Z".into(),
        ..Input::default()
    };
    let _ = save(&pool, context, None, make()).await.unwrap();
    let id: Uuid = sqlx::query_scalar("SELECT id FROM news WHERE slug=$1")
        .bind(&slug)
        .fetch_one(&pool)
        .await
        .unwrap();
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
    let _ = save(&pool, context, Some(id), input).await.unwrap();
    assert!(
        article(&pool, "pl", &slug)
            .await
            .unwrap()
            .0
            .contains("Treść")
    );
    assert_eq!(
        save(&pool, context, None, make()).await.unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut changed = make();
    changed.title = "Changed source".into();
    changed.title_pl = "Tytuł".into();
    changed.body_pl = "Treść".into();
    changed.translation_published_at = "2020-01-01T00:00:00Z".into();
    let _ = save(&pool, context, Some(id), changed).await.unwrap();
    assert_eq!(
        article(&pool, "pl", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let mut invalid = make();
    invalid.published_at = "invalid".into();
    assert_eq!(
        save(&pool, context, Some(id), invalid).await.unwrap_err(),
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
    let _ = delete(State(pool.clone()), Extension(context), Path(id))
        .await
        .unwrap();
    assert_eq!(
        article(&pool, "en", &slug).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        delete(State(pool.clone()), Extension(context), Path(id))
            .await
            .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    let response = public_router(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/en/news?page=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let _ = to_bytes(response.into_body(), 1024).await.unwrap();

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
            Identity(context.account),
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
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
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
