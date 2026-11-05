use super::*;
use crate::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use sqlx::{AssertSqlSafe, PgConnection, PgPool, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tower::ServiceExt;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn snapshot() -> SaleSnapshot {
    SaleSnapshot {
        name: "Aster Vale".into(),
        world: "world.test".into(),
        vocation: 2,
        level: 80,
        captured_at: 1_794_000_000,
        snapshot_hash: "a".repeat(64),
        health: 900,
        health_max: 900,
        mana_max: 500,
        created_at: None,
        blessings: vec![],
        world_transfer_eligible: None,
        mana: 300,
        capacity_centi_oz: 10000,
        speed: 250,
        experience: 100000,
        skills: vec![Skill {
            key: "sword".into(),
            level: 90,
            progress_bps: 5000,
        }],
        items: Some(vec![SaleItem {
            key: "oteryn:item.sword".into(),
            quantity: 1,
        }]),
        store_grants: None,
        outfits: Some(vec![]),
        mounts: None,
        achievements: Some(vec![Progress {
            key: "achievement.first".into(),
            value: 1,
        }]),
        quests: None,
    }
}
fn input(c: Uuid) -> ListingRequest {
    ListingRequest {
        character_id: c.to_string(),
        currency: "coins".into(),
        starting_bid: "100".into(),
        duration_days: 7,
        request_id: Uuid::now_v7().to_string(),
        publish_snapshot: true,
    }
}
#[test]
fn strict_input_and_public_projection_bounds() -> Result {
    let mut r = input(Uuid::now_v7());
    r.validate()?;
    for v in ["0", "01", "-1", "1e3", " 10", "9223372036854775808"] {
        assert!(amount(v, Currency::Coins).is_err());
    }
    assert!(amount("1000000000000", Currency::Gold).is_err());
    assert_eq!(amount("999999999999", Currency::Gold)?, 999999999999);
    r.duration_days = 15;
    assert!(r.validate().is_err());
    r.duration_days = 1;
    r.publish_snapshot = false;
    assert!(r.validate().is_err());
    assert!(id(&Uuid::nil().to_string()).is_err());
    let mut s = snapshot();
    s.validate()?;
    s.skills[0].progress_bps = 10001;
    assert!(s.validate().is_err());
    s.skills[0].progress_bps = 0;
    s.name = "<script>".into();
    assert!(s.validate().is_err());
    let mut v = serde_json::to_value(snapshot())?;
    v["seller_account_id"] = serde_json::json!(Uuid::now_v7().to_string());
    assert!(serde_json::from_value::<SaleSnapshot>(v).is_err());
    assert!(
        !CharacterFlags::default().enabled
            && !CharacterFlags::default().game_commands
            && !CharacterFlags::default().gold
    );
    Ok(())
}
#[test]
fn both_catalogs_have_every_message() {
    for line in include_str!("../../../../templates/bazaar/en.ftl").lines() {
        let key = line.split(" = ").next().unwrap_or_default();
        assert!(!http::text("pl", key).is_empty(), "missing {key}");
        assert!(!http::text("en", key).is_empty(), "missing {key}");
    }
}
#[derive(Clone, Debug)]
struct AccountAdapter;
impl Accounts for AccountAdapter {
    async fn state(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
    ) -> std::result::Result<Option<AccountState>, Error> {
        let active: Option<bool> =
            sqlx::query_scalar("SELECT active FROM test_accounts WHERE id=$1 FOR UPDATE")
                .bind(actor)
                .fetch_optional(db)
                .await?;
        Ok(active.map(|active| AccountState {
            active,
            email_verified: true,
            mfa: true,
            sudo: true,
            bazaar_blocked: false,
        }))
    }
}
#[derive(Clone, Debug)]
struct Funds;
impl Reservations for Funds {
    async fn lock_accounts(
        &self,
        db: &mut PgConnection,
        ids: &[Uuid],
    ) -> std::result::Result<(), Error> {
        for id in ids {
            sqlx::query("SELECT id FROM test_funds WHERE id=$1 FOR UPDATE")
                .bind(id)
                .execute(&mut *db)
                .await?;
        }
        Ok(())
    }
    async fn reserve(
        &self,
        db: &mut PgConnection,
        h: Reservation,
    ) -> std::result::Result<(), Error> {
        let r = sqlx::query("UPDATE test_funds SET held=$2 WHERE id=$1 AND balance >= $2")
            .bind(h.account)
            .bind(h.amount)
            .execute(db)
            .await?;
        if r.rows_affected() == 1 {
            Ok(())
        } else {
            Err(Error::InsufficientFunds)
        }
    }
    async fn release(
        &self,
        db: &mut PgConnection,
        _: Uuid,
        account: Uuid,
    ) -> std::result::Result<(), Error> {
        sqlx::query("UPDATE test_funds SET held=0 WHERE id=$1")
            .bind(account)
            .execute(db)
            .await?;
        Ok(())
    }
    async fn settle(&self, _: &mut PgConnection, _: &Settlement) -> std::result::Result<(), Error> {
        Err(Error::Denied)
    }
    async fn complete(
        &self,
        _: &mut PgConnection,
        _: &Settlement,
    ) -> std::result::Result<(), Error> {
        Err(Error::Denied)
    }
}
#[derive(Clone, Debug)]
struct Game {
    valid: Arc<AtomicBool>,
    owner: Uuid,
    character: Uuid,
}
impl Evidence for Game {
    async fn escrow_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
    ) -> std::result::Result<bool, Error> {
        Ok(self.valid.load(Ordering::SeqCst))
    }
    async fn offer_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
        _: &Offer,
    ) -> std::result::Result<bool, Error> {
        Ok(false)
    }
    async fn delivered(
        &self,
        _: &mut PgConnection,
        _: &Settlement,
    ) -> std::result::Result<bool, Error> {
        Ok(false)
    }
}
impl CharacterEvidence for Game {
    async fn approved(
        &self,
        _: &mut PgConnection,
        l: &Listing,
    ) -> std::result::Result<Option<SaleSnapshot>, Error> {
        Ok((self.valid.load(Ordering::SeqCst) && l.owner == self.owner).then(snapshot))
    }
    async fn choices(
        &self,
        _: &mut PgConnection,
        actor: Uuid,
    ) -> std::result::Result<Vec<CharacterChoice>, Error> {
        Ok(if actor == self.owner {
            vec![CharacterChoice {
                id: self.character,
                name: "Aster Vale".into(),
            }]
        } else {
            vec![]
        })
    }
}
type Api = CharacterMarket<AccountAdapter, Funds, Game>;
struct Harness {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    owner: Uuid,
    buyer: Uuid,
    game: Game,
}
impl Harness {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("t59_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |db, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(format!("SET search_path TO {path}")))
                        .execute(db)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105152830_t57_bazaar_core.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105230102_t59_character_auctions.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql("CREATE TABLE test_accounts(id uuid PRIMARY KEY,active boolean);CREATE TABLE test_funds(id uuid PRIMARY KEY,balance bigint,held bigint)").execute(&pool).await?;
        let owner = Uuid::now_v7();
        let buyer = Uuid::now_v7();
        for actor in [owner, buyer] {
            sqlx::query("INSERT INTO test_accounts VALUES($1,true)")
                .bind(actor)
                .execute(&pool)
                .await?;
            sqlx::query("INSERT INTO test_funds VALUES($1,10000,0)")
                .bind(actor)
                .execute(&pool)
                .await?;
        }
        let game = Game {
            valid: Arc::new(AtomicBool::new(true)),
            owner,
            character: Uuid::now_v7(),
        };
        Ok(Some(Self {
            pool,
            admin,
            schema,
            owner,
            buyer,
            game,
        }))
    }
    fn api(&self, commands: bool) -> Api {
        CharacterMarket::new(
            self.pool.clone(),
            AccountAdapter,
            Funds,
            self.game.clone(),
            CharacterFlags {
                enabled: true,
                gold: false,
                game_commands: commands,
            },
        )
    }
    async fn finish(self) -> Result {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
#[tokio::test]
async fn postgres_stages_intent_without_dispatch_and_replays_atomically() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let api = h.api(false);
    let mut r = input(h.game.character);
    let id = api.request(h.owner, &r).await?;
    assert_eq!(api.request(h.owner, &r).await?, id);
    r.starting_bid = "101".into();
    assert!(matches!(
        api.request(h.owner, &r).await,
        Err(Error::IdempotencyConflict)
    ));
    assert!(matches!(
        api.publish(h.owner, id).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(api.detail(id).await, Err(Error::NotFound)));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_character_intents")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(n, 1);
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(id)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(state, State::Draft);
    h.finish().await
}
#[tokio::test]
async fn postgres_verified_publication_bids_refusals_and_replay() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let api = h.api(true);
    let r = input(h.game.character);
    assert!(matches!(api.request(h.buyer, &r).await, Err(Error::Denied)));
    let id = api.request(h.owner, &r).await?;
    assert!(matches!(api.publish(h.buyer, id).await, Err(Error::Denied)));
    api.publish(h.owner, id).await?;
    assert_eq!(api.cards().await?.len(), 1);
    let key = Uuid::now_v7().to_string();
    let bid = api.bid(h.buyer, id, "500", &key).await?;
    assert_eq!(bid.price, 100);
    assert_eq!(api.bid(h.buyer, id, "500", &key).await?, bid);
    assert!(matches!(
        api.bid(h.owner, id, "600", &Uuid::now_v7().to_string())
            .await,
        Err(Error::IllegalTransition)
    ));
    assert!(matches!(
        api.bid(h.buyer, id, "10001", &Uuid::now_v7().to_string())
            .await,
        Err(Error::InsufficientFunds)
    ));
    assert!(matches!(
        api.bid(h.buyer, id, "01", &Uuid::now_v7().to_string())
            .await,
        Err(Error::InvalidInput)
    ));
    let held: i64 = sqlx::query_scalar("SELECT held FROM test_funds WHERE id=$1")
        .bind(h.buyer)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(held, 500);
    let overwrite =
        sqlx::query("UPDATE bazaar_character_publications SET projection='{}' WHERE listing_id=$1")
            .bind(id)
            .execute(&h.pool)
            .await;
    assert!(overwrite.is_err());
    h.finish().await
}
#[tokio::test]
async fn postgres_unknown_or_revoked_game_evidence_hides_and_denies() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let api = h.api(true);
    let id = api.request(h.owner, &input(h.game.character)).await?;
    h.game.valid.store(false, Ordering::SeqCst);
    assert!(matches!(api.publish(h.owner, id).await, Err(Error::Denied)));
    h.game.valid.store(true, Ordering::SeqCst);
    api.publish(h.owner, id).await?;
    h.game.valid.store(false, Ordering::SeqCst);
    assert!(api.cards().await?.is_empty());
    assert!(
        api.bid(h.buyer, id, "500", &Uuid::now_v7().to_string())
            .await
            .is_err()
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_bids")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(n, 0);
    h.finish().await
}
struct Session {
    actor: Uuid,
}
impl http::BrowserSecurity for Session {
    async fn authorize(
        &self,
        r: &mut axum::extract::Request,
        mutation: bool,
    ) -> std::result::Result<http::BrowserContext, StatusCode> {
        if mutation
            && r.headers()
                .get("x-csrf-token")
                .and_then(|s| s.to_str().ok())
                != Some("test-token")
        {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(http::BrowserContext {
            actor: self.actor,
            csrf: "test-token".into(),
        })
    }
}
#[tokio::test]
async fn postgres_browser_path_renders_safe_tabs_and_enforces_mutations() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let api = h.api(true);
    let id = api.request(h.owner, &input(h.game.character)).await?;
    api.publish(h.owner, id).await?;
    let app = http::routes(api, Session { actor: h.buyer }, true);
    let response = app
        .clone()
        .oneshot(Request::get(format!("/en/bazaar/{id}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let html = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    if let Ok(dir) = std::env::var("PORTAL_T59_RENDER_DIR") {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(std::path::Path::new(&dir).join("detail.html"), &html)?;
        let response = http::routes(h.api(false), Session { actor: h.owner }, true)
            .oneshot(Request::get("/pl/account/bazaar/sell").body(Body::empty())?)
            .await?;
        let body = to_bytes(response.into_body(), 1_000_000).await?;
        std::fs::write(std::path::Path::new(&dir).join("sell.html"), body)?;
    }
    assert!(
        html.contains("Aster Vale")
            && html.contains("id=\"items\"")
            && html.contains("id=\"outfits\"")
    );
    assert!(
        !html.contains("id=\"mounts\"")
            && !html.contains("id=\"store\"")
            && !html.contains("seller_account_id")
            && !html.contains(&h.owner.to_string())
            && !html.contains("leader_max")
    );
    let uri = format!("/en/account/bazaar/{id}/bids");
    let body = format!("amount=500&request_id={}&_csrf=test-token", Uuid::now_v7());
    let response = app
        .clone()
        .oneshot(
            Request::post(&uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body.clone()))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            Request::post(&uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("x-csrf-token", "test-token")
                .body(Body::from("amount=bad&request_id=bad&_csrf=test-token"))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .oneshot(
            Request::post(&uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("x-csrf-token", "test-token")
                .body(Body::from(body))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    h.finish().await
}

#[tokio::test]
async fn postgres_listing_form_redirects_to_owner_status_while_dispatch_is_off() -> Result {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let app = http::routes(h.api(false), Session { actor: h.owner }, true);
    let response = app
        .clone()
        .oneshot(Request::get("/pl/account/bazaar/sell").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let form = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    assert!(
        form.contains("name=\"publish_snapshot\"")
            && form.contains("name=\"_csrf\"")
            && form.contains("for=\"starting\"")
    );
    let key = Uuid::now_v7();
    let body = format!(
        "character_id={}&currency=coins&starting_bid=100&duration_days=7&request_id={key}&publish_snapshot=yes&_csrf=test-token",
        h.game.character
    );
    let response = app
        .clone()
        .oneshot(
            Request::post("/pl/account/bazaar/sell")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("x-csrf-token", "test-token")
                .body(Body::from(body))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()["location"], "/pl/account/bazaar");
    let response = app
        .clone()
        .oneshot(Request::get("/pl/account/bazaar").body(Body::empty())?)
        .await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    assert!(html.contains("Oczekuje na potwierdzenie gry"));
    let public = app
        .oneshot(Request::get("/en/bazaar").body(Body::empty())?)
        .await?;
    let html = String::from_utf8(to_bytes(public.into_body(), 1_000_000).await?.to_vec())?;
    assert!(!html.contains("Aster Vale"));
    h.finish().await
}

#[tokio::test]
async fn postgres_real_session_csrf_and_origin_adapter() -> Result {
    use axum::{Extension, Router, routing::get};
    use oteryn_web::session::{BrowserSession, PrivilegeChange, SessionLayer, middleware, rotate};
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&h.pool)
    .await?;
    let actor = h.owner;
    let login = Router::new()
        .route(
            "/test-session",
            get(
                move |Extension(session): Extension<BrowserSession>| async move {
                    rotate(&session, PrivilegeChange::Login(actor))
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    session
                        .csrf_token()
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                },
            ),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(h.pool.clone()),
            middleware,
        ));
    let app = http::browser_routes(
        h.api(false),
        h.pool.clone(),
        http::SessionSecurity {
            csrf: oteryn_web::csrf::CsrfConfig {
                trusted_origins: vec!["https://portal.test".into()],
                api_allow_list: vec![],
            },
        },
        true,
    )
    .merge(login);
    let response = app
        .clone()
        .oneshot(Request::get("/test-session").body(Body::empty())?)
        .await?;
    let cookie = response.headers()["set-cookie"]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    let csrf = String::from_utf8(to_bytes(response.into_body(), 1024).await?.to_vec())?;
    let body = format!(
        "character_id={}&currency=coins&starting_bid=100&duration_days=7&request_id={}&publish_snapshot=yes&_csrf={csrf}",
        h.game.character,
        Uuid::now_v7()
    );
    let request = |origin: &str, body: String| {
        Request::post("/en/account/bazaar/sell")
            .header("cookie", &cookie)
            .header("origin", origin)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
    };
    assert_eq!(
        app.clone()
            .oneshot(request("https://attacker.test", body.clone())?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "https://portal.test",
                body.replace(&csrf, "wrong")
            )?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request("https://portal.test", body)?)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        app.oneshot(Request::get("/en/account/bazaar/sell").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    h.finish().await
}
