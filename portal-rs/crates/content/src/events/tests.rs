#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_admin::{
    rbac::{AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, CheckContext},
    web::{PanelConfig, StaffIdentity, StaffSession},
};
use oteryn_web::session::BrowserSession;
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

#[derive(Clone)]
struct Identity {
    actor: Uuid,
    mfa: bool,
    state: AccountState,
}
impl StaffSession for Identity {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.actor,
            mfa_satisfied: self.mfa,
        })
    }
}
impl AccountStateReader for Identity {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(self.state)
    }
}
fn context(actor: Uuid) -> CheckContext {
    CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    }
}
fn draft(slug: &str) -> Draft {
    Draft {
        starts_at: OffsetDateTime::now_utc() + time::Duration::days(1),
        ends_at: OffsetDateTime::now_utc() + time::Duration::days(2),
        featured: false,
        en: Translation {
            title: "<script>unsafe</script>".into(),
            slug: slug.into(),
            summary: "Summary".into(),
            body: "<img src=x onerror=alert(1)>".into(),
        },
        pl: None,
    }
}
#[test]
fn states_and_invalid_input_fail_closed() {
    for state in [
        "draft",
        "scheduled",
        "archived",
        "active",
        "Published",
        " published",
        "unknown",
        "",
    ] {
        assert!(!Status::visible(state));
    }
    assert!(Status::visible("published"));
    for slug in [
        "",
        "-event",
        "event-",
        "two--parts",
        "Event",
        "../event",
        "a/b",
    ] {
        assert!(!valid_slug(slug));
    }
    assert!(valid_slug("event-12"));
    let mut d = draft("valid");
    d.ends_at = d.starts_at;
    assert!(d.validate().is_err());
    let mut d = draft("valid");
    d.en.title = " ".into();
    assert!(d.validate().is_err());
    let mut d = draft("valid");
    d.pl = Some(Translation {
        title: "Title".into(),
        slug: "slug".into(),
        summary: "".into(),
        body: "Body".into(),
    });
    assert!(d.validate().is_err());
}
struct Database {
    root: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("t11c_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(8)
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
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self { root, pool, schema })
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
    async fn author(&self) -> (Uuid, Authorizer<Identity>, Identity) {
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&self.pool).await.unwrap();
        let identity = Identity {
            actor,
            mfa: true,
            state: AccountState::Active,
        };
        (
            actor,
            Authorizer::new(self.pool.clone(), identity.clone()),
            identity,
        )
    }
}
#[tokio::test]
async fn postgres_publication_versions_validation_and_refusals() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (actor, auth, _) = db.author().await;
    let c = context(actor);
    let events = Events::new(db.pool.clone());
    let id = events.save(&auth, c, None, draft("event")).await.unwrap();
    assert!(events.public("en", None).await.unwrap().is_empty());
    events
        .transition(&auth, c, id, 1, "scheduled")
        .await
        .unwrap();
    assert!(events.public("en", None).await.unwrap().is_empty());
    assert!(matches!(
        events.transition(&auth, c, id, 1, "published").await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        events.transition(&auth, c, id, 2, "unknown").await,
        Err(Error::Invalid)
    ));
    events
        .transition(&auth, c, id, 2, "published")
        .await
        .unwrap();
    assert_eq!(events.public("en", Some("event")).await.unwrap().len(), 1);
    assert!(events.public("pl", Some("event")).await.unwrap().is_empty());
    events
        .transition(&auth, c, id, 3, "archived")
        .await
        .unwrap();
    assert!(events.public("en", None).await.unwrap().is_empty());
    events
        .transition(&auth, c, id, 4, "published")
        .await
        .unwrap();
    events
        .save(&auth, c, Some((id, 5)), draft("event"))
        .await
        .unwrap();
    assert!(events.public("en", None).await.unwrap().is_empty());
    assert!(matches!(
        events.save(&auth, c, Some((id, 5)), draft("event")).await,
        Err(Error::Conflict)
    ));
    let mut bad = draft("bad");
    bad.ends_at = bad.starts_at;
    assert!(matches!(
        events.save(&auth, c, None, bad).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        events.save(&auth, c, None, draft("event")).await,
        Err(Error::Invalid)
    ));
    let denied = Authorizer::new(
        db.pool.clone(),
        Identity {
            actor,
            mfa: true,
            state: AccountState::Ambiguous,
        },
    );
    assert!(matches!(
        events.transition(&denied, c, id, 6, "published").await,
        Err(Error::Authorization(_))
    ));
    let editor = Uuid::now_v7();
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('event_editor') RETURNING id")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'events.manage')")
        .bind(role)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(editor)
        .bind(role)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        events
            .transition(&auth, context(editor), id, 6, "published")
            .await,
        Err(Error::Authorization(_))
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE target=$1")
        .bind(id.to_string())
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 6);
    // Failed translation uniqueness must roll back the event insert as well.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.close().await;
}
async fn request(app: Router, path: &str) -> axum::response::Response {
    app.oneshot(
        Request::get(path)
            .header("host", "staff.test")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
fn staff(pool: PgPool, identity: Identity) -> Router {
    web::admin_router(
        pool,
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        identity.clone(),
        identity,
    )
}
#[tokio::test]
async fn postgres_http_public_visibility_staff_form_and_csrf() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (actor, auth, identity) = db.author().await;
    let events = Events::new(db.pool.clone());
    let id = events
        .save(&auth, context(actor), None, draft("event"))
        .await
        .unwrap();
    assert_eq!(
        request(web::public_router(db.pool.clone()), "/en/events/event")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    events
        .transition(&auth, context(actor), id, 1, "published")
        .await
        .unwrap();
    let r = request(web::public_router(db.pool.clone()), "/en/events/event").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["cache-control"], "no-store");
    let html = String::from_utf8(to_bytes(r.into_body(), 100000).await.unwrap().to_vec()).unwrap();
    std::fs::write("/tmp/t11c-public.html", &html).unwrap();
    assert!(html.contains("unsafe"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    let app = staff(db.pool.clone(), identity.clone());
    let r = request(app.clone(), &format!("/admin/events/{id}/edit")).await;
    assert_eq!(r.status(), StatusCode::OK);
    let cookie = r.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(to_bytes(r.into_body(), 100000).await.unwrap().to_vec()).unwrap();
    std::fs::write("/tmp/t11c-form.html", &html).unwrap();
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(html.contains("name=\"en_body\""));
    assert!(html.contains("name=\"status\""));
    let r = app
        .clone()
        .oneshot(
            Request::post(format!("/admin/events/{id}/status"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("status=archived&lock_version=2"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = app
        .clone()
        .oneshot(
            Request::post(format!("/admin/events/{id}/status"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&status=archived&lock_version=2"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        request(web::public_router(db.pool.clone()), "/en/events/event")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    for (title, expected) in [
        ("", StatusCode::UNPROCESSABLE_ENTITY),
        ("Updated", StatusCode::SEE_OTHER),
    ] {
        let body = format!(
            "_csrf={csrf}&lock_version=3&starts_at=2027-01-01T12%3A00&ends_at=2027-01-02T12%3A00&en_title={title}&en_slug=event&en_summary=Summary&en_body=Body"
        );
        let r = app
            .clone()
            .oneshot(
                Request::post(format!("/admin/events/{id}"))
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
    }
    assert_eq!(events.load(id).await.unwrap().0.status, "draft");
    let mut no_mfa = identity.clone();
    no_mfa.mfa = false;
    assert_eq!(
        request(staff(db.pool.clone(), no_mfa), "/admin/events")
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    for language in ["en", "pl"] {
        assert_eq!(
            request(
                web::public_router(db.pool.clone()),
                &format!("/{language}/events")
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        request(web::public_router(db.pool.clone()), "/de/events")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    db.close().await;
}
