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
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let auth = Authorizer::new(pool.clone(), Accounts);
    assert!(matches!(
        service.save(&auth, context, None, input()).await,
        Err(Error::Authorization(_))
    ));
    sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(actor).execute(&pool).await.unwrap();
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
    let cards = crate::today::cards(&pool, "en", at).await;
    assert_eq!(cards[2].items[0].title, "active-event");
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
}
