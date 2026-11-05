use super::*;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const CSRF: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
fn settings() -> Settings {
    Settings {
        enabled: true,
        commands: crate::commands::Settings {
            enabled: true,
            create_enabled: true,
            source_authority: "oteryn:character-authority:primary".into(),
            ..Default::default()
        },
    }
}
#[test]
fn names_normalize_spaces_case_and_enforce_boundaries() {
    assert_eq!(
        canonicalize("  aLDRIC   sTONE  "),
        Ok("Aldric Stone".into())
    );
    for name in ["Aldric", "Al Bo Cy", "Abcdefghijklmno Abcdefghijklm"] {
        assert!(canonicalize(name).is_ok(), "{name}");
    }
    for name in [
        "",
        "  ",
        "Al",
        "A Bee",
        "Ab Cd Ef Gh",
        "Abcdefghijklmnop",
        "Abcdefghijklmno Abcdefghijklmno",
        "Al\tBo",
        "Aldric\n",
        "Åldric",
        "Aldric1",
        "Ald-ric",
        "<script>",
    ] {
        assert_eq!(canonicalize(name), Err(NameError::Invalid), "{name}");
    }
}
#[test]
fn reserved_tokens_phrases_and_prefixes_are_denied() {
    for name in [
        "Admin",
        "aLdRiC GM",
        "Game Master",
        "Community Manager",
        "Oterynian",
        "Support",
        "System",
        "Staff",
    ] {
        assert_eq!(canonicalize(name), Err(NameError::Reserved), "{name}");
    }
    let retired = String::from_utf8(vec![99, 97, 110, 97, 114, 121]).unwrap_or_default();
    assert_eq!(canonicalize(&retired), Err(NameError::Reserved));
    assert_eq!(
        canonicalize(&format!("{retired}ian")),
        Err(NameError::Reserved)
    );
    assert!(canonicalize("Adminson").is_ok());
    for _ in 0..100 {
        assert!(canonicalize(&super::names::suggestion()).is_ok());
    }
}
async fn html(response: Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 65536).await?.to_vec(),
    )?)
}
#[tokio::test]
async fn pages_escape_input_localize_and_use_accessible_components() -> TestResult {
    for (lang, title) in [("pl", "Utwórz postać"), ("en", "Create character")] {
        let body = html(super::page::form(
            lang,
            CSRF,
            Some(Flash {
                name: "\"><script>bad</script>".into(),
                world: String::new(),
                submission: Uuid::now_v7().to_string(),
                error: "reserved".into(),
            }),
            vec![World {
                id: Uuid::now_v7(),
                name: "<world>".into(),
                pvp: String::new(),
                location: String::new(),
            }],
            super::http::WorldFilters::default(),
        ))
        .await?;
        assert!(body.contains(title));
        assert!(body.contains("script"));
        assert!(!body.contains("<world>"));
        assert!(!body.contains("<script>"));
        for marker in [
            "width=device-width",
            "for=\"name\"",
            "for=\"world\"",
            "aria-describedby=\"rules\"",
            "op-input",
            "op-button",
            "role=\"status\"",
            "tokens.css",
            "components.css",
        ] {
            assert!(body.contains(marker), "{marker}");
        }
        assert!(!body.contains("style="));
    }
    Ok(())
}
#[derive(Clone)]
struct Session(Arc<Mutex<BrowserContext>>);
fn app(pool: PgPool, settings: Settings, account: Option<Uuid>) -> (Router, Session) {
    let session = Session(Arc::new(Mutex::new(BrowserContext {
        account_id: account,
        csrf_token: CSRF.into(),
        flash: None,
    })));
    let app = router(pool, settings, |session: Session, action| async move {
        let mut context = session.0.lock().await;
        match action {
            SessionAction::Peek => {}
            SessionAction::Take => {
                let result = context.clone();
                context.flash = None;
                return Ok(result);
            }
            SessionAction::Store(flash) => context.flash = Some(flash),
        }
        Ok(context.clone())
    })
    .layer(Extension(session.clone()));
    (app, session)
}
fn post(name: &str, world: Uuid, submission: Uuid, csrf: &str) -> Request<Body> {
    let mut request = Request::new(Body::from(format!(
        "name={name}&world={world}&submission={submission}&_csrf={csrf}"
    )));
    *request.method_mut() = axum::http::Method::POST;
    *request.uri_mut() = axum::http::Uri::from_static("/en/account/characters/new");
    request.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    request
}
#[tokio::test]
async fn unauthenticated_disabled_and_csrf_fail_closed_without_storage() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let (anonymous, _) = app(pool.clone(), Settings::default(), None);
    assert_eq!(
        anonymous
            .oneshot(Request::get("/en/account/characters/new").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    let (disabled, _) = app(pool.clone(), Settings::default(), Some(Uuid::now_v7()));
    let response = disabled
        .oneshot(Request::get("/en/account/characters/new").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(html(response).await?.contains("temporarily unavailable"));
    let (enabled, _) = app(pool, settings(), Some(Uuid::now_v7()));
    assert_eq!(
        enabled
            .oneshot(post("Aldric", Uuid::now_v7(), Uuid::now_v7(), "wrong"))
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    Ok(())
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL creation test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("t22c_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |conn, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        for migration in [
            include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
            include_str!("../../../../migrations/20261105160001_t22_character_commands.sql"),
            include_str!("../../../../migrations/20261105190000_t25_world_registry.sql"),
            include_str!("../../../../migrations/20261105220231_t22c_creation_policy.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn account(&self, status: &str) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,$2,'test-only',$3,'en')").bind(id).bind(format!("{id}@example.invalid")).bind(status).execute(&self.pool).await?;
        Ok(id)
    }
    async fn world(&self) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO world_status(id,name) VALUES ($1,'Test World')")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
fn input(world: Uuid) -> Submission {
    Submission {
        id: Uuid::now_v7(),
        world,
        name: " aLDRIC   Stone ".into(),
    }
}
#[tokio::test]
async fn db_success_concurrent_replay_and_changed_intent_conflict() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("active").await?;
    let world = db.world().await?;
    let service = Creation::new(db.pool.clone(), settings());
    let input = input(world);
    let (a, b) = tokio::join!(
        service.submit(account, input.clone()),
        service.submit(account, input.clone())
    );
    let operation = a?;
    assert_eq!(operation, b?);
    let raw: String = sqlx::query_scalar("SELECT intent_json FROM character_commands WHERE id=$1")
        .bind(operation)
        .fetch_one(&db.pool)
        .await?;
    assert!(raw.contains("Aldric Stone"));
    assert!(raw.contains(&account.to_string()));
    let mut changed = input;
    changed.name = "Another Name".into();
    assert!(matches!(
        service.submit(account, changed).await,
        Err(Error::Invalid)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_commands")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await
}
#[tokio::test]
async fn db_refuses_ineligible_unknown_bad_world_and_reserved_names() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = Creation::new(db.pool.clone(), settings());
    let world = db.world().await?;
    for status in ["suspended", "closed"] {
        let account = db.account(status).await?;
        assert!(matches!(
            service.submit(account, input(world)).await,
            Err(Error::Denied)
        ));
    }
    assert!(matches!(
        service.submit(Uuid::now_v7(), input(world)).await,
        Err(Error::Denied)
    ));
    let account = db.account("pending_verification").await?;
    assert!(matches!(
        service.submit(account, input(Uuid::now_v7())).await,
        Err(Error::Invalid)
    ));
    sqlx::query("INSERT INTO reserved_names(name,expires_at) VALUES ('aldric stone',now()+interval '6 months')").execute(&db.pool).await?;
    assert!(matches!(
        service.submit(account, input(world)).await,
        Err(Error::Reserved)
    ));
    sqlx::query("UPDATE reserved_names SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert!(service.submit(account, input(world)).await.is_ok());
    db.close().await
}
#[tokio::test]
async fn db_browser_actor_to_command_flash_and_owner_only_status() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("active").await?;
    let world = db.world().await?;
    sqlx::query("UPDATE world_status SET pvp_type='Open',location='Europe' WHERE id=$1")
        .bind(world)
        .execute(&db.pool)
        .await?;
    let other_world = Uuid::now_v7();
    sqlx::query("INSERT INTO world_status(id,name,pvp_type,location) VALUES ($1,'Second World','Optional','America')").bind(other_world).execute(&db.pool).await?;
    let (app, session) = app(db.pool.clone(), settings(), Some(account));
    let response = app
        .clone()
        .oneshot(Request::get("/en/account/characters/new").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(html(response).await?.contains(&world.to_string()));
    let filtered = app
        .clone()
        .oneshot(
            Request::get("/en/account/characters/new?pvp=Open&location=Europe")
                .body(Body::empty())?,
        )
        .await?;
    let body = html(filtered).await?;
    assert!(body.contains(&world.to_string()));
    assert!(!body.contains(&other_world.to_string()));
    assert!(body.contains("Filter worlds"));
    let invalid = app
        .clone()
        .oneshot(Request::get("/en/account/characters/new?pvp=unknown").body(Body::empty())?)
        .await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let malformed = app
        .clone()
        .oneshot(post("Aldric&world=duplicate", world, Uuid::now_v7(), CSRF))
        .await?;
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let submission = Uuid::now_v7();
    let response = app
        .clone()
        .oneshot(post("Admin", world, submission, CSRF))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = app
        .clone()
        .oneshot(Request::get("/en/account/characters/new").body(Body::empty())?)
        .await?;
    let body = html(response).await?;
    assert!(body.contains("This character name is reserved"));
    assert!(body.contains(&submission.to_string()));
    assert!(body.contains("value=\"Admin\""));
    let response = app
        .clone()
        .oneshot(post("Aldric", world, submission, CSRF))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response.headers()["location"].to_str()?.to_owned();
    let response = app
        .clone()
        .oneshot(Request::get(&location).body(Body::empty())?)
        .await?;
    assert!(html(response).await?.contains("being processed"));
    session.0.lock().await.account_id = Some(db.account("active").await?);
    assert_eq!(
        app.clone()
            .oneshot(Request::get(&location).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    session.0.lock().await.account_id = Some(account);
    assert_eq!(
        app.clone()
            .oneshot(Request::get("/en/account/characters/requests/invalid").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let response = app
        .clone()
        .oneshot(Request::get("/en/account/characters/suggest").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        canonicalize(
            &session
                .0
                .lock()
                .await
                .flash
                .as_ref()
                .ok_or("missing suggestion")?
                .name
        )
        .is_ok()
    );
    db.close().await
}
#[tokio::test]
async fn db_rate_limits_are_shared_and_expire() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("active").await?;
    let a = Creation::new(db.pool.clone(), settings());
    let b = Creation::new(db.pool.clone(), settings());
    for _ in 0..5 {
        a.limit(account, "create").await?;
    }
    assert!(matches!(
        b.limit(account, "create").await,
        Err(Error::Limited)
    ));
    for _ in 0..60 {
        a.limit(account, "read").await?;
    }
    assert!(matches!(
        b.limit(account, "read").await,
        Err(Error::Limited)
    ));
    sqlx::query("UPDATE character_create_web_limits SET window_start=now()-interval '61 seconds'")
        .execute(&db.pool)
        .await?;
    b.limit(account, "create").await?;
    b.limit(account, "read").await?;
    db.close().await
}
#[tokio::test]
async fn db_receipts_decide_outcome_and_expiry_alone_stays_pending() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account("active").await?;
    let world = db.world().await?;
    let service = Creation::new(db.pool.clone(), settings());
    let operation = service.submit(account, input(world)).await?;
    sqlx::query("UPDATE character_commands SET created_at=now()-interval '1 day'")
        .execute(&db.pool)
        .await?;
    assert_eq!(service.status(account, operation).await?, Outcome::Pending);
    let identities = Arc::new(oteryn_internal_api::guard::IdentityMap::new(vec![
        oteryn_internal_api::guard::Binding {
            identity: oteryn_internal_api::guard::Identity::spki_sha256([1; 32]),
            group: crate::commands::GROUP.into(),
        },
    ])?);
    let command_router = crate::commands::router(
        crate::commands::Commands::new(db.pool.clone(), settings().commands),
        identities,
    );
    let publish = |body: String| {
        let mut request = Request::new(Body::from(body));
        request.headers_mut().insert(
            "content-type",
            axum::http::HeaderValue::from_static("application/json"),
        );
        *request.method_mut() = axum::http::Method::POST;
        *request.uri_mut() =
            axum::http::Uri::from_static("/internal/v1/game-auth/character-commands/receipt");
        request.extensions_mut().insert(
            oteryn_internal_api::guard::VerifiedClient::from_verified_tls13_http11(
                oteryn_internal_api::guard::Identity::spki_sha256([1; 32]),
            ),
        );
        request
    };
    // Reuse the existing golden wire fixture; only operation and binding identifiers change.
    let receipt = include_str!("../commands/fixtures/create-receipt.json")
        .replace(
            "0192a000-0000-7000-8000-000000000001",
            &operation.to_string(),
        )
        .replace("0190f2a1-3b4c-7d5e-8f60-718293a4b5c6", &account.to_string())
        .replace("01934f10-7c02-7001-805b-3b1122334401", &world.to_string())
        .replace("Aldric", "Aldric Stone");
    assert_eq!(
        command_router
            .clone()
            .oneshot(publish(receipt))
            .await?
            .status(),
        StatusCode::OK
    );
    assert_eq!(service.status(account, operation).await?, Outcome::Created);
    let rejected = service.submit(account, input(world)).await?;
    let receipt = include_str!("../commands/fixtures/rejected-receipt.json").replace(
        "0192a000-0000-7000-8000-000000000001",
        &rejected.to_string(),
    );
    assert_eq!(
        command_router.oneshot(publish(receipt)).await?.status(),
        StatusCode::OK
    );
    assert_eq!(
        service.status(account, rejected).await?,
        Outcome::Rejected("name-unavailable")
    );
    db.close().await
}
