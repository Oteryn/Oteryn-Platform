#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, header},
};
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

#[derive(Debug)]
struct Trusted {
    owner: Uuid,
    reporter: Uuid,
    staff: Uuid,
    character: Character,
}
impl Authority for Trusted {
    fn reporting_allowed(&self, account: Uuid) -> CheckFuture<'_, ()> {
        self.active(account)
    }
    fn active(&self, account: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async move {
            [self.owner, self.reporter, self.staff]
                .contains(&account)
                .then_some(())
        })
    }
    fn character(&self, account: Uuid, character: Uuid) -> CheckFuture<'_, Character> {
        Box::pin(async move {
            (account == self.owner && character == self.character.id)
                .then(|| self.character.clone())
        })
    }
    fn moderator(&self, account: Uuid) -> CheckFuture<'_, ()> {
        Box::pin(async move { (account == self.staff).then_some(()) })
    }
}
struct Database {
    root: PgPool,
    pool: PgPool,
    schema: String,
    trusted: Arc<Trusted>,
    board: Board,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t83_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let search = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(search))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let trusted = Arc::new(Trusted {
            owner: Uuid::now_v7(),
            reporter: Uuid::now_v7(),
            staff: Uuid::now_v7(),
            character: Character {
                id: Uuid::now_v7(),
                world: Uuid::now_v7(),
                name: "<script>Player</script>".into(),
                level: 120,
                profession: "druid".into(),
            },
        });
        let board = Board::new(pool.clone(), trusted.clone());
        Some(Self {
            root,
            pool,
            schema,
            trusted,
            board,
        })
    }
    async fn draft(&self) -> Draft {
        let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
            .fetch_one(&self.pool)
            .await
            .unwrap();
        Draft {
            character: self.trusted.character.id,
            purpose: "hunt".into(),
            min_level: 80,
            max_level: 180,
            profession: "knight".into(),
            starts_at: now + 300,
            expires_at: now + 3600,
            note: "Team".into(),
            csrf: None,
        }
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
    async fn session(&self, account: Uuid) -> (String, String) {
        let token = "a".repeat(64);
        let csrf = "b".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',$3)")
            .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind(&csrf).execute(&self.pool).await.unwrap();
        (token, csrf)
    }
}
#[test]
fn validates_bounded_filters_and_drafts() {
    let mut draft = Draft {
        character: Uuid::now_v7(),
        purpose: "hunt".into(),
        min_level: 1,
        max_level: 100,
        profession: "any".into(),
        starts_at: 100,
        expires_at: 200,
        note: String::new(),
        csrf: None,
    };
    assert!(draft.valid(100));
    draft.max_level = 0;
    assert!(!draft.valid(100));
    draft.max_level = 100;
    draft.expires_at = 100 + 7 * 86400 + 1;
    assert!(!draft.valid(100));
    assert!(
        !Filter {
            purpose: Some("unknown".into()),
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Filter {
            level: Some(0),
            ..Default::default()
        }
        .valid()
    );
    assert!(!text("\n", 0, 500));
}
#[tokio::test]
async fn unavailable_authority_and_disabled_router_deny() {
    let pool = oteryn_db::lazy_pool("postgres://unused@127.0.0.1:1/unused").unwrap();
    let board = Board::new(pool, Arc::new(Unavailable));
    assert!(board.authority.active(Uuid::now_v7()).await.is_none());
    assert!(matches!(
        board.remove(Uuid::now_v7(), Uuid::now_v7()).await,
        Err(Error::Denied)
    ));
    let response = router(board, false)
        .oneshot(Request::get("/en/party").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn db_listing_filters_ownership_and_expiry() {
    let Some(db) = Database::open().await else {
        return;
    };
    eprintln!("T83 PostgreSQL: listing_filters_ownership_and_expiry");
    let draft = db.draft().await;
    assert!(matches!(
        db.board.create(db.trusted.reporter, draft).await,
        Err(Error::Denied)
    ));
    let mut draft = db.draft().await;
    draft.min_level = -1;
    assert!(matches!(
        db.board.create(db.trusted.owner, draft).await,
        Err(Error::Invalid)
    ));
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    assert_eq!(db.board.list(&Filter::default()).await.unwrap().len(), 1);
    for filter in [
        Filter {
            world: Some(Uuid::now_v7()),
            ..Default::default()
        },
        Filter {
            purpose: Some("boss".into()),
            ..Default::default()
        },
        Filter {
            level: Some(200),
            ..Default::default()
        },
        Filter {
            profession: Some("druid".into()),
            ..Default::default()
        },
    ] {
        assert!(db.board.list(&filter).await.unwrap().is_empty());
    }
    assert_eq!(
        db.board
            .list(&Filter {
                level: Some(100),
                profession: Some("knight".into()),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        db.board.remove(db.trusted.reporter, id).await,
        Err(Error::Missing)
    ));
    sqlx::query("UPDATE pd_party_posts SET starts_at=now()-interval '2 hours',expires_at=now()-interval '1 second' WHERE id=$1").bind(id).execute(&db.pool).await.unwrap();
    assert!(db.board.list(&Filter::default()).await.unwrap().is_empty());
    assert!(matches!(
        db.board.report(db.trusted.reporter, id, "spam").await,
        Err(Error::Missing)
    ));
    assert_eq!(db.board.expire().await.unwrap(), 1);
    db.close().await;
}
#[tokio::test]
async fn db_reports_moderation_audit_and_caps() {
    let Some(db) = Database::open().await else {
        return;
    };
    eprintln!("T83 PostgreSQL: reports_moderation_audit_and_caps");
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    assert!(matches!(
        db.board.report(db.trusted.owner, id, "spam").await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        db.board.report(db.trusted.reporter, id, "").await,
        Err(Error::Invalid)
    ));
    db.board
        .report(db.trusted.reporter, id, "spam")
        .await
        .unwrap();
    db.board
        .report(db.trusted.reporter, id, "replacement")
        .await
        .unwrap();
    assert_eq!(db.board.reports().await.unwrap()[0].reports, "spam");
    assert!(matches!(
        db.board
            .moderate(db.trusted.reporter, id, "remove", "spam")
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        db.board
            .moderate(db.trusted.staff, id, "restore", "reason")
            .await,
        Err(Error::Invalid)
    ));
    db.board
        .moderate(db.trusted.staff, id, "dismiss", "reviewed")
        .await
        .unwrap();
    assert!(db.board.reports().await.unwrap().is_empty());
    assert_eq!(db.board.list(&Filter::default()).await.unwrap().len(), 1);
    db.board
        .moderate(db.trusted.staff, id, "remove", "spam")
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='party.moderate'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    assert!(db.board.list(&Filter::default()).await.unwrap().is_empty());
    for _ in 0..5 {
        db.board
            .create(db.trusted.owner, db.draft().await)
            .await
            .unwrap();
    }
    assert!(matches!(
        db.board.create(db.trusted.owner, db.draft().await).await,
        Err(Error::Limited)
    ));
    db.close().await;
}
#[tokio::test]
async fn db_http_csrf_locales_and_escaped_content() {
    let Some(db) = Database::open().await else {
        return;
    };
    eprintln!("T83 PostgreSQL: http_csrf_locales_and_escaped_content");
    let (token, csrf) = db.session(db.trusted.owner).await;
    let app = router(db.board.clone(), true);
    let draft = db.draft().await;
    let body = format!(
        "character={}&purpose=hunt&min_level=80&max_level=180&profession=knight&starts_at={}&expires_at={}&note=Team",
        draft.character, draft.starts_at, draft.expires_at
    );
    let response = app
        .clone()
        .oneshot(
            Request::post("/en/party")
                .header(
                    header::COOKIE,
                    format!("{}={token}", oteryn_web::session::COOKIE_NAME),
                )
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            Request::post("/en/party")
                .header(
                    header::COOKIE,
                    format!("{}={token}", oteryn_web::session::COOKIE_NAME),
                )
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    for (lang, title) in [("pl", "Szukam drużyny"), ("en", "Looking for a team")] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{lang}/party"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let html = String::from_utf8(
            to_bytes(response.into_body(), 65536)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains(title));
        assert!(html.contains("&#60;script&#62;") || html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("op-input"));
    }
    let response = app
        .clone()
        .oneshot(Request::get("/admin/en/party").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = app
        .oneshot(
            Request::get("/en/party?level=-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    db.close().await;
}

#[derive(Clone)]
struct StaffAdapter {
    mfa: bool,
}
impl oteryn_admin::web::StaffSession for StaffAdapter {
    async fn resolve(
        &self,
        session: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        session
            .account_id()
            .await
            .map(|account| oteryn_admin::web::StaffIdentity {
                account,
                mfa_satisfied: self.mfa,
            })
    }
}
impl oteryn_admin::rbac::AccountStateReader for StaffAdapter {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        Ok(oteryn_admin::rbac::AccountState::Active)
    }
}
#[tokio::test]
async fn db_staff_guard_and_audit_failure_roll_back() {
    let Some(db) = Database::open().await else {
        return;
    };
    eprintln!("T83 PostgreSQL: staff_guard_and_audit_failure_roll_back");
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    db.board
        .report(db.trusted.reporter, id, "spam")
        .await
        .unwrap();
    let (token, csrf) = db.session(db.trusted.staff).await;
    let config = oteryn_admin::web::PanelConfig {
        admin_panel: true,
        host: "staff.example.test".into(),
    };
    let adapter = StaffAdapter { mfa: true };
    let app = moderation_router(
        db.board.clone(),
        config.clone(),
        adapter.clone(),
        adapter.clone(),
    );
    let request = |host: &str| {
        Request::get("/admin/en/party")
            .header(header::HOST, host)
            .header(
                header::COOKIE,
                format!("{}={token}", oteryn_web::session::COOKIE_NAME),
            )
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(request("public.example.test"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.clone()
            .oneshot(request("staff.example.test"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(db.trusted.staff)
    .execute(&db.pool)
    .await
    .unwrap();
    let no_mfa = moderation_router(
        db.board.clone(),
        config,
        StaffAdapter { mfa: false },
        adapter,
    );
    assert_eq!(
        no_mfa
            .oneshot(request("staff.example.test"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(request("staff.example.test"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("spam"));
    // Audit storage failure must roll back both the listing and its report resolution.
    sqlx::raw_sql("CREATE FUNCTION fail_party_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit refusal'; END $$; CREATE TRIGGER fail_party_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION fail_party_audit();").execute(&db.pool).await.unwrap();
    assert!(
        db.board
            .moderate(db.trusted.staff, id, "remove", "spam")
            .await
            .is_err()
    );
    assert_eq!(db.board.list(&Filter::default()).await.unwrap().len(), 1);
    assert_eq!(db.board.reports().await.unwrap().len(), 1);
    sqlx::raw_sql("DROP TRIGGER fail_party_audit ON audit_log; DROP FUNCTION fail_party_audit();")
        .execute(&db.pool)
        .await
        .unwrap();
    let response = app
        .oneshot(
            Request::post(format!("/admin/en/party/{id}/moderate"))
                .header(header::HOST, "staff.example.test")
                .header(
                    header::COOKIE,
                    format!("{}={token}", oteryn_web::session::COOKIE_NAME),
                )
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("action=remove&reason=spam"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(db.board.list(&Filter::default()).await.unwrap().is_empty());
    db.close().await;
}

#[tokio::test]
async fn public_board_does_not_link_to_the_staff_queue() -> Result<(), Box<dyn std::error::Error>> {
    for lang in ["pl", "en"] {
        let response = super::party_view::page(lang, "csrf", Vec::new(), false, false)
            .map_err(|_| "public party template failed to render")?;
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
        let html = String::from_utf8(bytes.to_vec())?;
        assert!(!html.contains(&format!("href=\"/admin/{lang}/party\"")));
    }
    Ok(())
}
