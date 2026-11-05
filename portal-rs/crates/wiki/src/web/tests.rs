#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::Request,
};
use oteryn_admin::rbac::{AccountState, AccountStateUnavailable};
use oteryn_admin::web::StaffIdentity;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

#[derive(Clone, Debug)]
struct Identity {
    actor: Uuid,
    state: AccountState,
    mfa: bool,
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
struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
    identity: Identity,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("t13_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(5)
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
        let identity = Identity {
            actor: Uuid::now_v7(),
            state: AccountState::Active,
            mfa: true,
        };
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(identity.actor).execute(&pool).await.unwrap();
        Some(Self {
            pool,
            root,
            schema,
            identity,
        })
    }
    fn context(&self) -> CheckContext {
        CheckContext {
            account: self.identity.actor,
            require_staff_mfa: true,
            mfa_verified: true,
        }
    }
    fn repo(&self) -> Repository<Identity> {
        Repository::new(self.pool.clone(), self.identity.clone())
    }
    fn app(&self) -> Router {
        public_router(self.pool.clone()).merge(staff_router(
            self.pool.clone(),
            PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            self.identity.clone(),
            self.identity.clone(),
        ))
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
fn input() -> Vec<Translation> {
    ["pl", "en"]
        .into_iter()
        .map(|locale| Translation {
            locale: locale.into(),
            title: format!("Guide {locale}"),
            slug: "first-guide".into(),
            summary: "Editorial summary".into(),
            markdown: "## Hunting\n\nA **dragon** guide.".into(),
        })
        .collect()
}
async fn body(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 2_000_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
async fn get(app: Router, path: &str) -> Response {
    app.oneshot(
        Request::get(path)
            .header("host", "staff.test")
            .extension(ConnectInfo(
                "127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
async fn post(app: Router, path: &str, cookie: &str, token: &str, form: &str) -> Response {
    app.oneshot(
        Request::post(path)
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", cookie)
            .header("x-csrf-token", token)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form.to_owned()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[test]
fn rules_rendering_and_localized_forms() {
    for lang in ["pl", "en"] {
        let html = Edit {
            lang,
            id: String::new(),
            version: 0,
            status: "draft".into(),
            values: input(),
            csrf: "test".into(),
            existing: false,
        }
        .render()
        .unwrap();
        assert!(html.contains("width=device-width"));
        assert!(html.contains("for=\"pl_markdown\""));
        assert!(html.contains("for=\"en_title\""));
        assert!(html.contains("op-input"));
        std::fs::write(format!("/tmp/t13-edit-{lang}.html"), &html).unwrap();
        for key in ["wiki", "save", "denied", "invalid", "draft", "conflict"] {
            assert!(!text(lang, key).is_empty());
        }
    }
    assert!(domain::translations(&input(), true).is_ok());
    assert!(domain::translations(&input()[..1], true).is_err());
    let mut bad = input();
    bad[0].slug = "../bad".into();
    assert!(domain::translations(&bad, false).is_err());
    for markdown in [
        "<script>alert(1)</script>",
        "<!-- comment -->",
        "[x](javascript:alert(1))",
        "[x](DATA :text/html,evil)",
    ] {
        let mut bad = input();
        bad[0].markdown = markdown.into();
        assert!(bad[0].validate(false).is_err());
    }
    let html = domain::render(
        "## Good\n\n**safe** [link](https://example.test) <img src=x onerror=alert(1)> [bad](javascript:alert(1))\n\n<script>alert(1)</script>\n\n![image](https://example.test/image)",
    );
    assert!(html.contains("<strong>safe</strong>"));
    assert!(html.contains("<h2>Good</h2>"));
    for forbidden in ["<script", "<img", "onerror", "href=\"javascript:"] {
        assert!(!html.contains(forbidden), "{html}");
    }
    for destination in [
        "http://example.test",
        "//example.test",
        "https://user:pass@example.test",
        "relative",
        "javascript:alert(1)",
    ] {
        let rendered = domain::render(&format!("[bad]({destination})"));
        assert!(!rendered.contains("href="), "{rendered}");
    }
    for destination in [
        "https://example.test/path",
        "/wiki/articles/first-guide",
        "#heading",
    ] {
        assert!(domain::render(&format!("[good]({destination})")).contains("href="));
    }
    assert!(Action::Publish.target("draft").is_err());
    assert!(Action::Submit.target("unknown").is_err());
    for (q, p) in [("x", 1), ("dragon", 0), ("dragon", 1001)] {
        assert!(domain::search(q, p).is_err());
    }
    assert_eq!(
        domain::search("  dragon  guide  ", 1).unwrap(),
        "dragon guide"
    );
}

#[tokio::test]
async fn database_editorial_lifecycle_and_restore() {
    let Some(db) = Database::open().await else {
        return;
    };
    let repo = db.repo();
    let c = db.context();
    let public = PublicRepository::new(db.pool.clone());
    let id = repo
        .create(c, &input(), Some("oteryn:item.test"), "Initial guide")
        .await
        .unwrap();
    assert!(matches!(
        public.article("en", "first-guide").await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        repo.transition(c, id, 1, Action::Publish).await,
        Err(Error::Conflict)
    ));
    repo.transition(c, id, 1, Action::Submit).await.unwrap();
    repo.transition(c, id, 2, Action::Publish).await.unwrap();
    assert_eq!(
        public.article("en", "first-guide").await.unwrap().title,
        "Guide en"
    );
    assert_eq!(
        public.search("pl", "dragon", 1, false).await.unwrap().len(),
        1
    );
    assert!(matches!(
        repo.edit(c, id, 3, &input(), "").await,
        Err(Error::Conflict)
    ));
    repo.transition(c, id, 3, Action::Unpublish).await.unwrap();
    assert!(
        public
            .search("en", "dragon", 1, false)
            .await
            .unwrap()
            .is_empty()
    );
    let mut changed = input();
    changed[0].markdown = "New revision".into();
    repo.edit(c, id, 4, &changed, "Updated").await.unwrap();
    repo.transition(c, id, 5, Action::Submit).await.unwrap();
    repo.restore(c, id, 6, 1).await.unwrap();
    let (guide, values) = repo.get(c, id).await.unwrap();
    assert_eq!(guide.status, "draft");
    assert_eq!(values[0].markdown, input()[0].markdown);
    assert!(matches!(
        repo.transition(c, id, 7, Action::Publish).await,
        Err(Error::Conflict)
    ));
    assert_eq!(repo.history(c, id).await.unwrap().len(), 6);
    repo.transition(c, id, 7, Action::Submit).await.unwrap();
    repo.transition(c, id, 8, Action::Publish).await.unwrap();
    repo.transition(c, id, 9, Action::Archive).await.unwrap();
    assert!(matches!(
        public.article("pl", "first-guide").await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        repo.transition(c, id, 10, Action::Submit).await,
        Err(Error::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE target=$1")
        .bind(id.to_string())
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 10);
    db.close().await;
}
#[tokio::test]
async fn database_refusals_bad_input_and_concurrent_edits() {
    let Some(db) = Database::open().await else {
        return;
    };
    let repo = db.repo();
    let c = db.context();
    let mut unknown = db.identity.clone();
    unknown.state = AccountState::Ambiguous;
    assert!(
        Repository::new(db.pool.clone(), unknown)
            .create(c, &input(), None, "")
            .await
            .is_err()
    );
    let mut no_mfa = c;
    no_mfa.mfa_verified = false;
    assert!(repo.create(no_mfa, &input(), None, "").await.is_err());
    no_mfa = c;
    no_mfa.require_staff_mfa = false;
    assert!(repo.create(no_mfa, &input(), None, "").await.is_err());
    let editor = Uuid::now_v7();
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('wiki_editor_test') RETURNING id")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    for permission in ["admin.access", "wiki.access", "wiki.articles.manage"] {
        sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)")
            .bind(role)
            .bind(permission)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)")
        .bind(editor)
        .bind(role)
        .execute(&db.pool)
        .await
        .unwrap();
    let editor_context = CheckContext {
        account: editor,
        ..c
    };
    assert!(
        repo.authorize(editor_context, Permission::ManageWikiArticles)
            .await
            .is_ok()
    );
    assert!(
        repo.authorize(editor_context, Permission::PublishWiki)
            .await
            .is_err()
    );
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='wiki.access.denied'",
    )
    .bind(editor)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
    let mut stranger = c;
    stranger.account = Uuid::now_v7();
    assert!(repo.create(stranger, &input(), None, "").await.is_err());
    let mut bad = input();
    bad[0].markdown = "<b>raw</b>".into();
    assert!(matches!(
        repo.create(c, &bad, None, "").await,
        Err(Error::Invalid)
    ));
    let id = repo.create(c, &input()[..1], None, "").await.unwrap();
    repo.transition(c, id, 1, Action::Submit).await.unwrap();
    assert!(matches!(
        repo.transition(c, id, 2, Action::Publish).await,
        Err(Error::Invalid)
    ));
    let values = input();
    let (a, b) = tokio::join!(
        repo.edit(c, id, 2, &values, "A"),
        repo.edit(c, id, 2, &values, "B")
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        repo.create(c, &input(), None, "").await,
        Err(Error::Conflict)
    ));
    assert!(repo.restore(c, id, 3, 999).await.is_err());
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(c.account)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.edit(c, id, 3, &input(), "").await.is_err());
    db.close().await;
}
#[tokio::test]
async fn database_history_immutable_and_audit_atomic() {
    let Some(db) = Database::open().await else {
        return;
    };
    let repo = db.repo();
    let c = db.context();
    let id = repo.create(c, &input(), None, "").await.unwrap();
    repo.transition(c, id, 1, Action::Submit).await.unwrap();
    for sql in [
        "UPDATE wiki_revisions SET title='changed'",
        "DELETE FROM wiki_revisions",
        "UPDATE wiki_reviews SET action='publish'",
        "DELETE FROM wiki_reviews",
        "TRUNCATE wiki_revisions",
        "TRUNCATE wiki_reviews",
    ] {
        assert!(
            sqlx::raw_sql(AssertSqlSafe(sql))
                .execute(&db.pool)
                .await
                .is_err(),
            "{sql}"
        );
    }
    sqlx::raw_sql("CREATE FUNCTION fail_wiki_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'audit unavailable'; END; $$; CREATE TRIGGER fail_wiki_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION fail_wiki_audit();").execute(&db.pool).await.unwrap();
    assert!(repo.transition(c, id, 2, Action::Publish).await.is_err());
    assert_eq!(repo.get(c, id).await.unwrap().0.status, "in_review");
    assert!(repo.edit(c, id, 2, &input(), "").await.is_err());
    assert_eq!(repo.history(c, id).await.unwrap().len(), 2);
    db.close().await;
}
#[tokio::test]
async fn database_browser_create_review_publish_and_csrf() {
    let Some(db) = Database::open().await else {
        return;
    };
    let app = db.app();
    let response = get(app.clone(), "/admin/wiki/articles/create").await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = body(response).await;
    let token = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let form = "pl_title=Poradnik&pl_slug=poradnik&pl_summary=Opis&pl_markdown=Dragon&en_title=Guide&en_slug=guide&en_summary=Summary&en_markdown=Dragon";
    assert_eq!(
        post(app.clone(), "/admin/wiki/articles", &cookie, "wrong", form)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = post(app.clone(), "/admin/wiki/articles", &cookie, token, form).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let edit = response.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_owned();
    let id = edit.split('/').nth(4).unwrap();
    assert_eq!(
        get(app.clone(), "/en/wiki/articles/guide").await.status(),
        StatusCode::NOT_FOUND
    );
    for (action, version) in [("submit-review", 1), ("publish", 2)] {
        assert_eq!(
            post(
                app.clone(),
                &format!("/admin/wiki/articles/{id}/{action}"),
                &cookie,
                token,
                &format!("lock_version={version}")
            )
            .await
            .status(),
            StatusCode::SEE_OTHER
        );
    }
    let response = get(app.clone(), "/en/wiki/articles/guide").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("content-security-policy"));
    assert!(body(response).await.contains("Dragon"));
    assert_eq!(
        get(app.clone(), "/pl/wiki/search?q=Dragon").await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get(app.clone(), "/en/wiki/search?q=dragon&page=bad")
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(
            app.clone(),
            &format!("/admin/wiki/articles/{id}/publish"),
            &cookie,
            token,
            "lock_version=bad"
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        get(app.clone(), "/en/wiki/search?q=x").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(
            app.clone(),
            &format!("/admin/wiki/articles/{id}/unpublish"),
            &cookie,
            token,
            "lock_version=1"
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        get(app.clone(), &format!("/admin/wiki/articles/{id}/revisions"))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        get(
            app.clone(),
            &format!("/admin/wiki/articles/{id}/preview/pl")
        )
        .await
        .status(),
        StatusCode::OK
    );
    db.close().await;
}
#[tokio::test]
async fn database_public_search_locale_pagination_and_throttling() {
    let Some(db) = Database::open().await else {
        return;
    };
    let repo = db.repo();
    let c = db.context();
    let id = repo.create(c, &input(), None, "").await.unwrap();
    repo.transition(c, id, 1, Action::Submit).await.unwrap();
    repo.transition(c, id, 2, Action::Publish).await.unwrap();
    let public = PublicRepository::new(db.pool.clone());
    assert_eq!(
        public.search("en", "dragon", 1, false).await.unwrap().len(),
        1
    );
    assert!(
        public
            .search("en", "dragon", 2, false)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(public.search("en", "", 1, false).await.unwrap().is_empty());
    assert!(
        public
            .search("en", "' OR true --", 1, false)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(public.article("de", "first-guide").await.is_err());
    let app = db.app();
    for _ in 0..30 {
        assert_eq!(
            get(app.clone(), "/en/wiki/search?q=dragon").await.status(),
            StatusCode::OK
        );
    }
    let response = get(app.clone(), "/en/wiki/search?q=dragon").await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key("retry-after"));
    assert_eq!(
        get(app.clone(), "/pl/wiki/search?q=dragon").await.status(),
        StatusCode::OK
    );
    db.close().await;
}
#[tokio::test]
async fn staff_host_flag_and_unknown_identity_deny_without_database() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let identity = Identity {
        actor: Uuid::now_v7(),
        state: AccountState::Unknown,
        mfa: false,
    };
    let app = staff_router(
        pool.clone(),
        PanelConfig {
            admin_panel: false,
            host: "staff.test".into(),
        },
        identity.clone(),
        identity.clone(),
    );
    assert_eq!(
        get(app, "/admin/wiki").await.status(),
        StatusCode::NOT_FOUND
    );
    let app = staff_router(
        pool,
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        identity.clone(),
        identity,
    );
    assert_eq!(
        get(app, "/admin/wiki").await.status(),
        StatusCode::FORBIDDEN
    );
}
