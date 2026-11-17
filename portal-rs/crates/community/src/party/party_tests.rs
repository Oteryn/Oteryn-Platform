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
    fn active_in<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        self.active(account)
    }

    fn reporting_allowed_in<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        self.reporting_allowed(account)
    }

    fn character_in<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
    ) -> CheckFuture<'a, Character> {
        self.character(account, character)
    }

    fn moderator_in<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        account: Uuid,
    ) -> CheckFuture<'a, ()> {
        self.moderator(account)
    }

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
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')")
            .bind(trusted.owner).bind(trusted.character.id).bind(trusted.character.world).bind(&trusted.character.name).execute(&pool).await.unwrap();
        let board = Board::new(pool.clone(), trusted.clone());
        Some(Self {
            root,
            pool,
            schema,
            trusted,
            board,
        })
    }
    async fn moderate(
        &self,
        actor: Uuid,
        id: Uuid,
        action: &str,
        reason: &str,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'test','active','en',now()) ON CONFLICT DO NOTHING")
            .bind(self.trusted.staff).bind(format!("{}@example.invalid",self.trusted.staff)).execute(&self.pool).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support' ON CONFLICT DO NOTHING")
            .bind(self.trusted.staff).execute(&self.pool).await?;
        let authority = oteryn_admin::rbac::MutationAuthorization::new(Arc::new(
            oteryn_admin::rbac::Authorizer::new(self.pool.clone(), StaffAdapter { mfa: true }),
        ));
        let mut context = oteryn_admin::rbac::CheckContext::new(actor);
        context.mfa_verified = true;
        let rows = self.board.reports().await?;
        let report_ids = rows
            .iter()
            .find(|row| row.id == id)
            .map_or_else(Vec::new, |row| {
                row.report_ids
                    .split(',')
                    .map(|id| id.parse().unwrap())
                    .collect()
            });
        self.board
            .moderate(context, &authority, id, action, reason, &report_ids)
            .await
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
    async fn staff_session(&self, staff: Uuid) -> (String, String) {
        let (token, csrf) = self.session(staff).await;
        let password: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
            .bind(staff)
            .fetch_one(&self.pool)
            .await
            .unwrap();
        let grant = "c".repeat(64);
        let grant_hash = Sha256::digest(grant.as_bytes()).to_vec();
        let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation")
            .bind(staff).bind(vec![1_u8;32]).fetch_one(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '1 hour')")
            .bind(&grant_hash).bind(staff).bind(generation).bind(Sha256::digest(password.as_bytes()).to_vec()).execute(&self.pool).await.unwrap();
        sqlx::query("UPDATE web_sessions SET data=jsonb_build_object('mfa_grant',$1::text),authorization_generation=(SELECT native_security_generation FROM accounts WHERE id=$2) WHERE account_id=$2")
            .bind(grant).bind(staff).execute(&self.pool).await.unwrap();
        (token, csrf)
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
        db.moderate(db.trusted.reporter, id, "remove", "spam").await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        db.moderate(db.trusted.staff, id, "restore", "reason").await,
        Err(Error::Invalid)
    ));
    db.moderate(db.trusted.staff, id, "dismiss", "reviewed")
        .await
        .unwrap();
    assert!(db.board.reports().await.unwrap().is_empty());
    assert_eq!(db.board.list(&Filter::default()).await.unwrap().len(), 1);
    db.moderate(db.trusted.staff, id, "remove", "spam")
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
        db.moderate(db.trusted.staff, id, "remove", "spam")
            .await
            .is_err()
    );
    assert_eq!(db.board.list(&Filter::default()).await.unwrap().len(), 1);
    assert_eq!(db.board.reports().await.unwrap().len(), 1);
    sqlx::raw_sql("DROP TRIGGER fail_party_audit ON audit_log; DROP FUNCTION fail_party_audit();")
        .execute(&db.pool)
        .await
        .unwrap();
    let report_ids = &db.board.reports().await.unwrap()[0].report_ids;
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
                .body(Body::from(format!(
                    "action=remove&reason=spam&report_ids={report_ids}"
                )))
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

#[tokio::test]
async fn audit_w2_c05_production_authority_owner_report_and_staff_flow() {
    let started = std::time::Instant::now();
    let db = Database::open().await.expect("PostgreSQL required");
    let owner = db.trusted.owner;
    let reporter = db.trusted.reporter;
    let staff = db.trusted.staff;
    let character = db.trusted.character.id;
    let world = db.trusted.character.world;
    for account in [owner, reporter, staff] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$2,'test','active',now(),'en')").bind(account).bind(format!("{account}@example.test")).execute(&db.pool).await.unwrap();
    }
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test','1','1',0,decode(repeat('00',32),'hex'))").bind(owner).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE') ON CONFLICT(account_id,character_id) DO UPDATE SET name=excluded.name").bind(owner).bind(character).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($1,$2,$3,1,1,1,1,1)").bind(character).bind(owner).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1 WHERE id=1")
        .execute(&db.pool)
        .await
        .unwrap();
    let profile = serde_json::json!({"profile":[{"world_id":world.to_string(),"name":"Aldric","level":"120","vocation":2}]});
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile',$1,false,1,1,0,$2),('character_profile','',true,1,1,floor(extract(epoch FROM clock_timestamp()))::bigint,'{}')").bind(character.to_string()).bind(profile).execute(&db.pool).await.unwrap();
    let board = production_board(db.pool.clone());
    assert!(
        board
            .authority
            .character(reporter, character)
            .await
            .is_none()
    );
    let app = router(board.clone(), true);
    let (token, csrf) = db.session(owner).await;
    let draft = db.draft().await;
    let body = format!(
        "character={character}&purpose=hunt&min_level=80&max_level=180&profession=knight&starts_at={}&expires_at={}&note=Team",
        draft.starts_at, draft.expires_at
    );
    let response = app
        .clone()
        .oneshot(
            Request::post("/en/party")
                .header(
                    header::COOKIE,
                    format!("{}={token}", oteryn_web::session::COOKIE_NAME),
                )
                .header("x-csrf-token", &csrf)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let id: Uuid = sqlx::query_scalar("SELECT id FROM pd_party_posts WHERE account_id=$1")
        .bind(owner)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM web_sessions")
        .execute(&db.pool)
        .await
        .unwrap();
    let (token, csrf) = db.session(reporter).await;
    let response = app
        .oneshot(
            Request::post(format!("/en/party/{id}/report"))
                .header(
                    header::COOKIE,
                    format!("{}={token}", oteryn_web::session::COOKIE_NAME),
                )
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("reason=spam"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(board.authority.moderator(staff).await.is_none());
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(staff)
    .execute(&db.pool)
    .await
    .unwrap();
    assert!(board.authority.moderator(staff).await.is_some());
    sqlx::query("DELETE FROM web_sessions")
        .execute(&db.pool)
        .await
        .unwrap();
    let (token, csrf) = db.staff_session(staff).await;
    let adapter = StaffAdapter { mfa: true };
    let app = moderation_router(
        board.clone(),
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.example.test".into(),
        },
        adapter,
        oteryn_admin::web::PortalIdentity::configured(db.pool.clone()),
    );
    let report_ids = &db.board.reports().await.unwrap()[0].report_ids;
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
                .body(Body::from(format!(
                    "action=remove&reason=spam&report_ids={report_ids}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(board.list(&Filter::default()).await.unwrap().is_empty());
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(board.authority.active(owner).await.is_none());
    db.close().await;
    eprintln!(
        "audit_w2_c05_production_authority_owner_report_and_staff_flow {:?}",
        started.elapsed()
    );
}

async fn delta_party_revocation(revoke: &str) {
    let started = std::time::Instant::now();
    {
        let db = Database::open().await.unwrap();
        let staff = db.trusted.staff;
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'test-password','active','en',now())")
            .bind(staff).bind(format!("{staff}@example.invalid")).execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'")
            .bind(staff).execute(&db.pool).await.unwrap();
        let id = db
            .board
            .create(db.trusted.owner, db.draft().await)
            .await
            .unwrap();
        db.board
            .report(db.trusted.reporter, id, "spam")
            .await
            .unwrap();
        let (token, csrf) = db.staff_session(staff).await;
        let adapter = StaffAdapter { mfa: true };
        let app = moderation_router(
            db.board.clone(),
            oteryn_admin::web::PanelConfig {
                admin_panel: true,
                host: "staff.example.test".into(),
            },
            adapter.clone(),
            adapter,
        );
        let report_ids = &db.board.reports().await.unwrap()[0].report_ids;
        let mut blocker = db.pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM pd_party_posts WHERE id=$1 FOR UPDATE")
            .bind(id)
            .execute(&mut *blocker)
            .await
            .unwrap();
        let request = Request::post(format!("/admin/en/party/{id}/moderate"))
            .header(header::HOST, "staff.example.test")
            .header(header::ORIGIN, "https://staff.example.test")
            .header(
                header::COOKIE,
                format!("{}={token}", oteryn_web::session::COOKIE_NAME),
            )
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("x-csrf-token", csrf)
            .body(Body::from(format!(
                "action=remove&reason=spam&report_ids={report_ids}"
            )))
            .unwrap();
        let mutation = tokio::spawn(async move { app.oneshot(request).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                assert!(!mutation.is_finished(), "request finished before post wait: {revoke}");
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))")
                    .bind(pid).fetch_one(&db.root).await.unwrap();
                if waiting { break; }
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
        match revoke {
            "session" => {
                sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                    .bind(staff)
                    .execute(&db.pool)
                    .await
                    .unwrap();
            }
            "mfa" => {
                sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1")
                    .bind(staff)
                    .execute(&db.pool)
                    .await
                    .unwrap();
            }
            "permission" => {
                use oteryn_admin::rbac::{Authorizer, Change, CheckContext};
                let manager = Uuid::now_v7();
                sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'test','active','en',now())")
                    .bind(manager).bind(format!("{manager}@example.invalid")).execute(&db.pool).await.unwrap();
                sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
                    .bind(manager).execute(&db.pool).await.unwrap();
                let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='support'")
                    .fetch_one(&db.pool)
                    .await
                    .unwrap();
                let mut context = CheckContext::new(manager);
                context.mfa_verified = true;
                Authorizer::new(db.pool.clone(), StaffAdapter { mfa: true })
                    .change(
                        context,
                        Uuid::now_v7(),
                        Change::SetPermissions {
                            role,
                            permissions: &[],
                        },
                    )
                    .await
                    .unwrap();
            }
            _ => {}
        }
        blocker.rollback().await.unwrap();
        let status = mutation.await.unwrap().unwrap().status();
        let post: String = sqlx::query_scalar("SELECT status FROM pd_party_posts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let reports: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pd_party_reports WHERE post_id=$1 AND status='open'",
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        let audits: i64 =
            sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='party.moderate'")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        db.close().await;
        eprintln!("delta_party {revoke} duration={:?}", started.elapsed());
        if revoke == "none" {
            assert_eq!(status, StatusCode::SEE_OTHER);
            assert_eq!(post, "removed");
            assert_eq!(reports, 0);
            assert_eq!(audits, 1);
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "revoked {revoke}");
            assert_eq!(post, "active");
            assert_eq!(reports, 1);
            assert_eq!(audits, 0);
        }
    }
}

#[tokio::test]
async fn delta_party_revalidates_session_after_post_wait() {
    delta_party_revocation("session").await;
}
#[tokio::test]
async fn delta_party_revalidates_mfa_after_post_wait() {
    delta_party_revocation("mfa").await;
}
#[tokio::test]
async fn delta_party_revalidates_permission_after_post_wait() {
    delta_party_revocation("permission").await;
}

#[tokio::test]
async fn delta_party_valid_session_commits_after_post_wait() {
    delta_party_revocation("none").await;
}

#[tokio::test]
async fn w3_c06_003_party_rechecks_authority_after_mutation_wait() {
    let started = std::time::Instant::now();
    let db = Database::open().await.expect("PostgreSQL required");
    let owner = db.trusted.owner;
    let reporter = db.trusted.reporter;
    let staff = db.trusted.staff;
    let character = db.trusted.character.id;
    let world = db.trusted.character.world;
    for account in [owner, reporter, staff] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$2,'test','active',now(),'en')").bind(account).bind(format!("{account}@example.test")).execute(&db.pool).await.unwrap();
    }
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test','1','1',0,decode(repeat('00',32),'hex'))").bind(owner).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE') ON CONFLICT(account_id,character_id) DO UPDATE SET name=excluded.name").bind(owner).bind(character).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($1,$2,$3,1,1,1,1,1)").bind(character).bind(owner).bind(world).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1 WHERE id=1")
        .execute(&db.pool)
        .await
        .unwrap();
    let profile = serde_json::json!({"profile":[{"world_id":world.to_string(),"name":"Aldric","level":"120","vocation":2}]});
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile',$1,false,1,1,0,$2),('character_profile','',true,1,1,floor(extract(epoch FROM clock_timestamp()))::bigint,'{}')").bind(character.to_string()).bind(profile).execute(&db.pool).await.unwrap();
    let board = production_board(db.pool.clone());
    assert!(
        board
            .authority
            .character(reporter, character)
            .await
            .is_none()
    );

    let draft = db.draft().await;
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
        .bind(owner.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let pending = tokio::spawn(async move { board.create(owner, draft).await });
    wait_party_blocked(&db.pool, pid).await;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test','1','2',0,decode(repeat('00',32),'hex'))").bind(reporter).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE account_characters SET account_id=$1 WHERE character_id=$2")
        .bind(reporter)
        .bind(character)
        .execute(&db.pool)
        .await
        .unwrap();
    blocker.commit().await.unwrap();
    assert!(matches!(pending.await.unwrap(), Err(Error::Denied)));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_posts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
    eprintln!(
        "w3_c06_003_party_rechecks_authority_after_mutation_wait {:?}",
        started.elapsed()
    );
}

async fn wait_party_blocked(pool: &PgPool, pid: i32) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("party mutation must reach lock wait");
}

#[tokio::test]
async fn w3_c06_012_party_uses_post_wait_expiry() {
    let started = std::time::Instant::now();
    let db = Database::open().await.expect("PostgreSQL required");
    let owner = db.trusted.owner;
    let reporter = db.trusted.reporter;
    let mut draft = db.draft().await;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    draft.starts_at = now + 1;
    draft.expires_at = now + 2;
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
        .bind(owner.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let board = db.board.clone();
    let create = tokio::spawn(async move { board.create(owner, draft).await });
    wait_party_blocked(&db.pool, pid).await;
    while time::OffsetDateTime::now_utc().unix_timestamp() <= now + 1 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    blocker.commit().await.unwrap();
    assert!(matches!(create.await.unwrap(), Err(Error::Invalid)));
    let id = db.board.create(owner, db.draft().await).await.unwrap();
    sqlx::query("UPDATE pd_party_posts SET starts_at=clock_timestamp()-interval '1 minute',expires_at=clock_timestamp()+interval '1 second' WHERE id=$1").bind(id).execute(&db.pool).await.unwrap();
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
        .bind(reporter.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let board = db.board.clone();
    let report = tokio::spawn(async move { board.report(reporter, id, "expired").await });
    wait_party_blocked(&db.pool, pid).await;
    loop {
        let expired: bool = sqlx::query_scalar(
            "SELECT expires_at<=clock_timestamp() FROM pd_party_posts WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        if expired {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    blocker.commit().await.unwrap();
    assert!(matches!(report.await.unwrap(), Err(Error::Missing)));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_reports")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
    eprintln!(
        "w3_c06_012_party_uses_post_wait_expiry {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn w3_c06_012_party_admission_timestamps_follow_lock_wait() {
    let started = std::time::Instant::now();
    let db = Database::open().await.expect("PostgreSQL required");
    let owner = db.trusted.owner;
    let reporter = db.trusted.reporter;
    let draft = db.draft().await;
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
        .bind(owner.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let board = db.board.clone();
    let pending = tokio::spawn(async move { board.create(owner, draft).await });
    wait_party_blocked(&db.pool, pid).await;
    let before = time::OffsetDateTime::now_utc().unix_timestamp();
    while time::OffsetDateTime::now_utc().unix_timestamp() == before {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let admitted = time::OffsetDateTime::now_utc().unix_timestamp();
    blocker.commit().await.unwrap();
    let id = pending.await.unwrap().unwrap();
    let current: bool =
        sqlx::query_scalar("SELECT created_at>=to_timestamp($2) FROM pd_party_posts WHERE id=$1")
            .bind(id)
            .bind(admitted as f64)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(current);
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
        .bind(reporter.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let board = db.board.clone();
    let pending = tokio::spawn(async move { board.report(reporter, id, "report").await });
    wait_party_blocked(&db.pool, pid).await;
    let before = time::OffsetDateTime::now_utc().unix_timestamp();
    while time::OffsetDateTime::now_utc().unix_timestamp() == before {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let admitted = time::OffsetDateTime::now_utc().unix_timestamp();
    blocker.commit().await.unwrap();
    pending.await.unwrap().unwrap();
    let current: bool = sqlx::query_scalar(
        "SELECT created_at>=to_timestamp($2) FROM pd_party_reports WHERE post_id=$1",
    )
    .bind(id)
    .bind(admitted as f64)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(current);
    db.close().await;
    eprintln!(
        "w3_c06_012_party_admission_timestamps_follow_lock_wait {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn party_request_lock_wait_is_bounded_by_mutation_budget() {
    let db = Database::open().await.expect("PostgreSQL required");
    let owner = db.trusted.owner;
    let reporter = db.trusted.reporter;
    let id = db.board.create(owner, db.draft().await).await.unwrap();
    for actor in [owner, reporter] {
        let mut blocker = db.pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
            .bind(actor.to_string())
            .execute(&mut *blocker)
            .await
            .unwrap();
        let board = db.board.clone();
        let draft = db.draft().await;
        let started = std::time::Instant::now();
        let pending = tokio::spawn(async move {
            if actor == owner {
                board.create(owner, draft).await.map(|_| ())
            } else {
                board.report(actor, id, "bounded").await
            }
        });
        wait_party_blocked(&db.pool, pid).await;
        // The blocker is never released while the request waits.
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), pending)
            .await
            .expect("party request lock wait must end within the mutation budget")
            .unwrap();
        assert!(
            result.is_err(),
            "a request that never acquires its lock must not succeed"
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(6));
        blocker.rollback().await.unwrap();
    }
    let posts: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_posts")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_reports")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!((posts, reports), (1, 0));
    db.close().await;
}

#[tokio::test]
async fn r7_configured_report_limit() {
    const CHILD: &str = "PARTY_REPORT_LIMIT_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "party::party_tests::r7_configured_report_limit",
                "--exact",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("OTERYN_REPORTS_UNJUSTIFIED_LIMIT", "0")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let db = Database::open().await.unwrap();
    let actor = db.trusted.reporter;
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'reporter@example.test','test','active',now(),'en')").bind(actor).execute(&db.pool).await.unwrap();
    let allowed = production_board(db.pool.clone())
        .authority
        .reporting_allowed(actor)
        .await;
    db.close().await;
    assert!(allowed.is_none(), "zero limit must deny reporting");
}

#[tokio::test]
async fn r7_transferred_character_hidden() {
    let db = Database::open().await.unwrap();
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    assert_eq!(db.board.list(&Filter::default()).await.unwrap()[0].id, id);
    sqlx::query("UPDATE account_characters SET account_id=$1 WHERE character_id=$2")
        .bind(db.trusted.reporter)
        .bind(db.trusted.character.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let visible = db.board.list(&Filter::default()).await.unwrap();
    db.close().await;
    assert!(visible.is_empty(), "transferred character still advertised");
}

#[tokio::test]
async fn r7_queue_shows_all_open_reasons() {
    let db = Database::open().await.unwrap();
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    sqlx::query("INSERT INTO pd_party_reports(post_id, account_id, reason) SELECT $1, uuidv7(), n::text FROM generate_series(1,51) n")
        .bind(id).execute(&db.pool).await.unwrap();
    let rows = db.board.reports().await.unwrap();
    assert_eq!(rows[0].reports.split(" | ").count(), 50);
    assert_eq!(rows[0].report_ids.split(',').count(), 50);
    assert_eq!(rows[0].more_reports, 1);
    let response = party_view::page("en", "csrf", rows, true, false).unwrap();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("<p>1 more</p>"));

    db.moderate(db.trusted.staff, id, "dismiss", "reviewed")
        .await
        .unwrap();
    let rows = db.board.reports().await.unwrap();
    assert_eq!(rows[0].reports.split(" | ").count(), 1);
    assert_eq!(rows[0].more_reports, 0);
    db.moderate(db.trusted.staff, id, "dismiss", "reviewed")
        .await
        .unwrap();
    assert!(db.board.reports().await.unwrap().is_empty());
    let resolved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pd_party_reports WHERE post_id=$1 AND status='resolved'",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    db.close().await;
    assert_eq!(resolved, 51);
}

#[tokio::test]
async fn r7_moderation_preserves_reports_filed_after_page_load() {
    let db = Database::open().await.unwrap();
    let staff = db.trusted.staff;
    let id = db
        .board
        .create(db.trusted.owner, db.draft().await)
        .await
        .unwrap();
    let shown: Uuid = sqlx::query_scalar("INSERT INTO pd_party_reports(post_id, account_id, reason) VALUES($1,$2,'shown reason') RETURNING id")
        .bind(id).bind(db.trusted.reporter).fetch_one(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'staff@example.test','test','active','en',now())").bind(staff).execute(&db.pool).await.unwrap();
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(staff)
    .execute(&db.pool)
    .await
    .unwrap();
    let (token, csrf) = db.staff_session(staff).await;
    let adapter = StaffAdapter { mfa: true };
    let app = moderation_router(
        db.board.clone(),
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.example.test".into(),
        },
        adapter.clone(),
        adapter,
    );
    let cookie = format!("{}={token}", oteryn_web::session::COOKIE_NAME);
    let page = app
        .clone()
        .oneshot(
            Request::get("/admin/en/party")
                .header(header::HOST, "staff.example.test")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let html =
        String::from_utf8(to_bytes(page.into_body(), 65536).await.unwrap().to_vec()).unwrap();
    assert!(html.contains("shown reason"));
    db.board.report(staff, id, "late reason").await.unwrap();
    let mut form = "action=dismiss&reason=reviewed".to_owned();
    let rendered_ids = html
        .split_once("name=\"report_ids\" value=\"")
        .and_then(|(_, tail)| tail.split_once('"').map(|(ids, _)| ids));
    if let Some(ids) = rendered_ids {
        form.push_str("&report_ids=");
        form.push_str(ids);
    }
    let response = app
        .oneshot(
            Request::post(format!("/admin/en/party/{id}/moderate"))
                .header(header::HOST, "staff.example.test")
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let statuses: Vec<(String, String)> = sqlx::query_as(
        "SELECT reason,status FROM pd_party_reports WHERE post_id=$1 ORDER BY reason",
    )
    .bind(id)
    .fetch_all(&db.pool)
    .await
    .unwrap();
    db.close().await;
    assert_eq!(statuses[0].1, "open", "late report was closed unseen");
    assert_eq!(statuses[1].1, "resolved");
    assert_eq!(rendered_ids, Some(shown.to_string().as_str()));
}
