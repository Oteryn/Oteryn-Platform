#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_admin::rbac::{AccountState, AccountStateUnavailable};
use tower::ServiceExt;
#[derive(Clone)]
struct Accounts;
impl AccountStateReader for Accounts {
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

    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
fn input() -> Input {
    Input {
        publication_state: "published".into(),
        severity: "warning".into(),
        starts_at: OffsetDateTime::now_utc() - time::Duration::days(1),
        ends_at: None,
        title: "<script>title</script>".into(),
        body: "Details".into(),
        action_label: Some("Read".into()),
        action_url: Some("/news/update".into()),
        title_pl: Some("Tytuł".into()),
        body_pl: Some("Treść".into()),
        action_label_pl: Some("Czytaj".into()),
        translation_published_at: Some(OffsetDateTime::now_utc() - time::Duration::days(1)),
    }
}
#[test]
fn input_and_action_validation() {
    assert!(input().validate().is_ok());
    for url in [
        "//outside.test",
        "https://outside.test",
        "/\\outside",
        "/%2foutside",
        "/bad\npath",
        "/bad\"path",
    ] {
        assert!(!safe_action(url));
    }
    for state in ["Published", "unknown", ""] {
        let mut i = input();
        i.publication_state = state.into();
        assert!(i.validate().is_err());
    }
    let mut i = input();
    i.ends_at = Some(i.starts_at);
    assert!(i.validate().is_err());
    i = input();
    i.body = String::new();
    assert!(i.validate().is_err());
    i = input();
    i.title_pl = None;
    assert!(i.validate().is_err());
}
#[tokio::test]
async fn postgres_announcements_and_today() {
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL announcements test: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t11d_{}", Uuid::now_v7().simple());
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
    let service = Announcements::new(pool.clone());
    let actor = Uuid::now_v7();
    let mut context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: None,
    };
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'announcement-password','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await.unwrap();
    context.session_proof = Some(crate::test_staff::proof(&pool, actor).await.unwrap());
    let auth = Authorizer::new(
        pool.clone(),
        oteryn_admin::web::PortalIdentity::configured(pool.clone()),
    );
    assert!(matches!(
        service.save(&auth, context, None, input()).await,
        Err(Error::Authorization(_))
    ));
    sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(actor).execute(&pool).await.unwrap();
    let op_service = Announcements::new(pool.clone());
    let op_auth = auth.clone();
    let result = crate::test_staff::race_session_revocation(&pool, actor, async move {
        op_service.save(&op_auth, context, None, input()).await
    })
    .await
    .unwrap();
    assert!(matches!(
        result,
        Err(Error::Authorization(oteryn_admin::rbac::Error::Denied))
    ));
    context.session_proof = Some(crate::test_staff::proof(&pool, actor).await.unwrap());
    let id = service.save(&auth, context, None, input()).await.unwrap();
    let at = OffsetDateTime::now_utc();
    assert_eq!(service.active("en", at, 5).await.unwrap().len(), 1);
    assert_eq!(service.active("pl", at, 5).await.unwrap().len(), 1);
    assert!(service.active("de", at, 5).await.is_err());
    assert!(service.active("en", at, 0).await.is_err());
    assert!(matches!(
        service.save(&auth, context, Some((id, 99)), input()).await,
        Err(Error::Conflict)
    ));
    let mut bad = input();
    bad.action_url = Some("//outside.test".into());
    assert!(matches!(
        service.save(&auth, context, None, bad).await,
        Err(Error::Invalid)
    ));
    sqlx::query("UPDATE announcements SET starts_at=$2,ends_at=$2+interval '1 hour' WHERE id=$1")
        .bind(id)
        .bind(at)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(service.active("en", at, 5).await.unwrap().len(), 1);
    assert!(
        service
            .active("en", at + time::Duration::hours(1), 5)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("UPDATE announcements SET translation_source_updated_at=updated_at-interval '1 second' WHERE id=$1").bind(id).execute(&pool).await.unwrap();
    assert!(service.active("pl", at, 5).await.unwrap().is_empty());
    for (slug, start, end, featured) in [
        (
            "future-event",
            at + time::Duration::days(1),
            at + time::Duration::days(2),
            true,
        ),
        (
            "active-event",
            at - time::Duration::hours(1),
            at + time::Duration::hours(1),
            false,
        ),
    ] {
        let event = Uuid::now_v7();
        sqlx::query("INSERT INTO events(id,status,starts_at,ends_at,featured,created_by,updated_by,published_by) VALUES ($1,'published',$2,$3,$4,$5,$5,$5)").bind(event).bind(start).bind(end).bind(featured).bind(actor).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO event_translations(event_id,locale,title,slug,summary,body) VALUES ($1,'en',$2,$2,'Summary','Body')").bind(event).bind(slug).execute(&pool).await.unwrap();
    }
    for n in 0..200 {
        let event = Uuid::now_v7();
        sqlx::query("INSERT INTO events(id,status,starts_at,ends_at,featured,created_by,updated_by,published_by) VALUES($1,'published',$2,$3,true,$4,$4,$4)").bind(event).bind(at-time::Duration::days(3)).bind(at-time::Duration::days(2)).bind(actor).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO event_translations(event_id,locale,title,slug,summary,body) VALUES($1,'en','Expired',$2,'Expired','Expired')").bind(event).bind(format!("expired-{n}")).execute(&pool).await.unwrap();
    }
    let cards = crate::today::cards(&pool, "en", at).await;
    assert_eq!(cards[2].items[0].title, "active-event");
    sqlx::raw_sql("INSERT INTO pages(id,slug) VALUES(uuidv7(),'support'); INSERT INTO editorial_pages(page_id,locale,title,body,published_at) SELECT id,'en','Assistance dragon','Help body',now() FROM pages WHERE slug='support'").execute(&pool).await.unwrap();
    let search = oteryn_web::search::routes(oteryn_web::search::SearchState {
        pool: pool.clone(),
        world: None,
        characters: None,
    });
    let mut request = Request::get("/en/search?q=assistance&kind=pages")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "127.0.0.1:42148".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = search.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let href = html
        .split("href=\"")
        .filter_map(|part| part.split_once('"').map(|(href, _)| href))
        .find(|href| *href == "/en/support")
        .unwrap();
    let response = crate::pages::router(pool.clone())
        .oneshot(Request::get(href).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let ticker = crate::today::render_ticker(&pool, "en", at).await;
    assert!(ticker.contains("&lt;script&gt;title"));
    assert!(!ticker.contains("<script>"));
    let page = axum::Router::new()
        .route(
            "/en/example",
            axum::routing::get(|| async {
                axum::response::Html("<!doctype html><html><body><main>Page</main></body></html>")
            }),
        )
        .layer(axum::middleware::from_fn_with_state(pool.clone(), banner));
    let r = page
        .oneshot(
            Request::builder()
                .uri("/en/example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let html = String::from_utf8(to_bytes(r.into_body(), 100000).await.unwrap().to_vec()).unwrap();
    assert!(html.contains("<aside"));
    assert!(html.contains("Warning: &lt;script&gt;title"));
    assert!(html.find("<aside").unwrap() < html.find("<main").unwrap());
    let router = crate::today::router(pool.clone());
    let r = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/en/today")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["cache-control"], "no-store, max-age=0");
    let body = String::from_utf8(to_bytes(r.into_body(), 100000).await.unwrap().to_vec()).unwrap();
    assert!(body.contains("World status"));
    assert!(body.contains("Some sources are unavailable"));
    assert!(body.contains("&lt;script&gt;title"));
    let r = router
        .oneshot(
            Request::builder()
                .uri("/de/today")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    let mut edit = input();
    // Reusing a publication stamp after editing the source does not approve Polish content.
    edit.translation_published_at =
        sqlx::query_scalar("SELECT translation_published_at FROM announcements WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    edit.title = "Updated source".into();
    service
        .save(&auth, context, Some((id, 1)), edit.clone())
        .await
        .unwrap();
    assert!(
        service
            .active("pl", OffsetDateTime::now_utc(), 5)
            .await
            .unwrap()
            .is_empty()
    );
    edit.translation_published_at = Some(OffsetDateTime::now_utc());
    service
        .save(&auth, context, Some((id, 2)), edit)
        .await
        .unwrap();
    assert_eq!(
        service
            .active("pl", OffsetDateTime::now_utc(), 5)
            .await
            .unwrap()
            .len(),
        1
    );
    sqlx::query("UPDATE announcements SET publication_state='draft' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(service.active("en", at, 5).await.unwrap().is_empty());
    sqlx::query("DROP TABLE news").execute(&pool).await.unwrap();
    let cards = crate::today::cards(&pool, "en", at).await;
    assert_eq!(cards[1].state, crate::today::SourceState::Empty);
    assert_eq!(cards[3].state, crate::today::SourceState::Unavailable);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    eprintln!(
        "postgres_announcements_and_today duration: {:?}",
        started.elapsed()
    );
}

#[derive(Clone)]
struct Staff(Uuid, bool);
impl oteryn_admin::web::StaffSession for Staff {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: self.1,
        })
    }
}
#[tokio::test]
async fn postgres_staff_announcement_dispatch() {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")
        .expect("PORTAL_TEST_DATABASE_URL required for staff announcement dispatch");
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t11d_{}", Uuid::now_v7().simple());
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

    let actor = Uuid::now_v7();
    let app = |enabled, host: &str, mfa| {
        staff_router(
            pool.clone(),
            oteryn_admin::web::PanelConfig {
                admin_panel: enabled,
                host: host.into(),
            },
            Staff(actor, mfa),
            Accounts,
        )
    };
    let get = |path: &str| {
        Request::get(path)
            .header("host", "staff.test")
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(
        app(false, "staff.test", true)
            .oneshot(get("/admin/announcements/create"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app(true, "other.test", true)
            .oneshot(get("/admin/announcements/create"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app(true, "staff.test", true)
            .oneshot(get("/admin/announcements/create"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(actor).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM role_permissions WHERE permission='portal.announcements.manage'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app(true, "staff.test", true)
            .oneshot(get("/admin/announcements/create"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'portal.announcements.manage' FROM roles WHERE key='content_editor'").execute(&pool).await.unwrap();
    assert_eq!(
        app(true, "staff.test", false)
            .oneshot(get("/admin/announcements/create"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = app(true, "staff.test", true)
        .oneshot(get("/admin/announcements/create"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let token = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let send = |path: String, token: &str, version, state: &str, title: &str, translation: &str| {
        Request::post(path).header("host","staff.test").header("origin","https://staff.test").header("cookie",&cookie).header("content-type","application/x-www-form-urlencoded").body(Body::from(format!("_csrf={token}&lock_version={version}&publication_state={state}&severity=info&starts_at=2026-01-01T00:00:00Z&title={title}&body=Body&title_pl=Tytul&body_pl=Tresc&translation_published_at={translation}"))).unwrap()
    };
    let service = Announcements::new(pool.clone());
    assert_eq!(
        app(true, "staff.test", true)
            .oneshot(send(
                "/admin/announcements".into(),
                "bad",
                0,
                "draft",
                "Original",
                ""
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let r = app(true, "staff.test", true)
        .oneshot(send(
            "/admin/announcements".into(),
            token,
            0,
            "draft",
            "Original",
            "",
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let path = r.headers()["location"].to_str().unwrap().to_owned();
    assert!(
        service
            .active("en", OffsetDateTime::now_utc(), 10)
            .await
            .unwrap()
            .is_empty()
    );
    for (version, state, title, translation, status, visible, polish) in [
        (
            1,
            "published",
            "Original",
            "2026-01-01T00:00:00Z",
            StatusCode::SEE_OTHER,
            true,
            true,
        ),
        (1, "draft", "Stale", "", StatusCode::CONFLICT, true, true),
        (
            2,
            "published",
            "Edited",
            "2026-01-01T00:00:00Z",
            StatusCode::SEE_OTHER,
            true,
            false,
        ),
        (
            3,
            "published",
            "Edited",
            "2026-01-02T00:00:00Z",
            StatusCode::SEE_OTHER,
            true,
            true,
        ),
        (
            4,
            "draft",
            "Edited",
            "",
            StatusCode::SEE_OTHER,
            false,
            false,
        ),
    ] {
        assert_eq!(
            app(true, "staff.test", true)
                .oneshot(send(
                    path.clone(),
                    token,
                    version,
                    state,
                    title,
                    translation
                ))
                .await
                .unwrap()
                .status(),
            status
        );
        assert_eq!(
            !service
                .active("en", OffsetDateTime::now_utc(), 10)
                .await
                .unwrap()
                .is_empty(),
            visible
        );
        assert_eq!(
            !service
                .active("pl", OffsetDateTime::now_utc(), 10)
                .await
                .unwrap()
                .is_empty(),
            polish
        );
        let response = crate::today::router(pool.clone())
            .oneshot(Request::get("/en/today").body(Body::empty()).unwrap())
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
        let expected = if version <= 1 { "Original" } else { "Edited" };
        assert_eq!(html.contains(expected), visible);
    }
    let response = app(true, "staff.test", true)
        .oneshot(get(&path))
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
    assert!(html.contains("value=\"5\""));
    assert_eq!(
        app(true, "staff.test", true)
            .oneshot(get("/admin/announcements"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app(true, "staff.test", true)
            .oneshot(send(path, token, 5, "published", "Denied", ""))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    eprintln!(
        "postgres_staff_announcement_dispatch duration: {:?}",
        started.elapsed()
    );
}
