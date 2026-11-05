#![allow(clippy::unwrap_used)]
use super::*;
use oteryn_playerdata::progress::{self, Authority, Payload, Subject};
use oteryn_world::{WorldOptions, WorldStore};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn now() -> u64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as u64
}
#[derive(Debug, Clone)]
struct Trust {
    account: Uuid,
    character: Uuid,
    allow: Arc<AtomicBool>,
}
impl Authority for Trust {
    fn source(&self) -> &str {
        "synthetic:journal"
    }
    fn permits(&self, s: Subject<'_>) -> bool {
        self.allow.load(Ordering::SeqCst)
            && s.account == self.account
            && s.character.is_none_or(|c| c == self.character)
            && s.epoch == "1"
            && s.recovery == "1"
    }
    fn catalog_accepts(&self, _: &str, _: &Payload) -> bool {
        self.allow.load(Ordering::SeqCst)
    }
}
fn card(version: Uuid) -> Card {
    Card {
        id: Uuid::now_v7(),
        world_version: version,
        kind: "Creature".into(),
        key: "synthetic:fox".into(),
        race_key: "synthetic:fox".into(),
        race_revision: "definition-r1".into(),
        final_kill_threshold: 10,
        name: Labels {
            pl: "Lis".into(),
            en: "Fox".into(),
        },
        appearance_id: 1,
    }
}
#[test]
fn deterministic_turns_retry_and_bad_input() {
    let c = card(Uuid::now_v7());
    let mut a = Battle::new(vec![c.fighter()], vec![c.fighter()], 17).unwrap();
    let mut b = a.clone();
    a.advance(0, Action::Guard).unwrap();
    let saved = a.clone();
    a.advance(0, Action::Guard).unwrap();
    assert_eq!(a, saved);
    assert!(matches!(a.advance(0, Action::Strike), Err(Error::Conflict)));
    b.advance(0, Action::Guard).unwrap();
    assert_eq!(a, b);
    while a.outcome == Outcome::Active {
        let n = a.turn;
        a.advance(n, Action::Strike).unwrap();
        b.advance(n, Action::Strike).unwrap();
    }
    assert_eq!(a, b);
    assert!(a.turn <= 100);
    assert!(a.advance(a.turn, Action::Strike).is_err());
    for (name, ids) in [
        ("", vec![c.id]),
        ("ok", vec![]),
        ("ok", vec![c.id, c.id]),
        ("bad\nname", vec![c.id]),
        ("ok", vec![Uuid::nil()]),
    ] {
        assert!(valid_deck(name, &ids).is_err());
    }
    assert!(Battle::new(vec![], vec![], 0).is_err());
    assert!(serde_json::from_str::<Action>("\"win\"").is_err());
    assert!(!Flags::default().collection);
}
#[test]
fn terminal_outcomes_and_reserves() {
    let c = card(Uuid::now_v7());
    for (p, e, outcome) in [
        (1, 30, Outcome::Defeat),
        (30, 1, Outcome::Victory),
        (1, 1, Outcome::Draw),
    ] {
        let mut b = Battle::new(vec![c.fighter()], vec![c.fighter()], 1).unwrap();
        b.player[0].health = p;
        b.enemy[0].health = e;
        b.advance(0, Action::Strike).unwrap();
        assert_eq!(b.outcome, outcome);
    }
    let mut b = Battle::new(vec![c.fighter(), c.fighter()], vec![c.fighter()], 1).unwrap();
    b.player[0].health = 1;
    b.advance(0, Action::Strike).unwrap();
    assert_eq!(b.outcome, Outcome::Active);
    assert_eq!(b.player[1].health, 30);
}
struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    service: Collection,
    trust: Trust,
    card: Card,
}
impl Database {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED collection PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("collection_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let version = Uuid::now_v7();
        let card = card(version);
        sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
            .bind(version)
            .bind("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE world_active_version SET version_id=$1")
            .bind(version)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'Creature','synthetic:fox',$2,true)").bind(version).bind(serde_json::json!({"definition":{"identity":{"family":"Creature","key":"synthetic:fox"},"client_projection":"ClientSafe"},"authoring":{"profile":{"details":{"display_name":"Fox","bestiary":{"class":"synthetic"}}}}})).execute(&pool).await.unwrap();
        let world = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
        world.allow("Creature", "synthetic:fox").await.unwrap();
        let trust = Trust {
            account: Uuid::now_v7(),
            character: Uuid::now_v7(),
            allow: Arc::new(AtomicBool::new(true)),
        };
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'synthetic@example.invalid','unused','active',now(),'en')").bind(trust.account).execute(&pool).await.unwrap();
        let progress = progress::ProgressStore::new(
            pool.clone(),
            progress::Flags {
                ingest: true,
                read: true,
            },
            vec!["bestiary".into()],
        );
        // SOURCE: synthetic G-11 revision 2 envelope, no copied game content.
        let payload = progress::Bestiary {
            race_key: card.race_key.clone(),
            race_definition_revision: card.race_revision.clone(),
            kill_count: 10,
            final_kill_threshold: 10,
            charm_points_awarded: 0,
        };
        let record = progress::Record {
            contract_version: 1,
            operation: "PublishGameJournalProgressV1".into(),
            source_authority: trust.source().into(),
            account_id: trust.account,
            character_id: Some(trust.character),
            world_id: Some(Uuid::now_v7()),
            projection_revision: Some("1".into()),
            projection_epoch: "1".into(),
            recovery_generation: "1".into(),
            family: "bestiary".into(),
            source_revision: "1".into(),
            record_id: Uuid::now_v7(),
            entity_key: card.race_key.clone(),
            character_revision: Some("1".into()),
            occurred_at_unix_ms: now().to_string(),
            record_kind: "UPSERT".into(),
            payload: Some(
                serde_json::value::RawValue::from_string(serde_json::to_string(&payload).unwrap())
                    .unwrap(),
            ),
        };
        progress
            .ingest(&serde_json::to_vec(&record).unwrap(), &trust)
            .await
            .unwrap();
        let w = progress::Watermark {
            contract_version: 1,
            operation: "PublishGameJournalProgressWatermarkV1".into(),
            source_authority: trust.source().into(),
            account_id: trust.account,
            projection_epoch: "1".into(),
            recovery_generation: "1".into(),
            family: "bestiary".into(),
            complete_through: "1".into(),
            observed_at_unix_ms: now().to_string(),
            baseline_complete: true,
        };
        progress
            .watermark(&serde_json::to_vec(&w).unwrap(), &trust, now())
            .await
            .unwrap();
        let service = Collection::new(pool.clone(), world, progress, Flags { collection: true });
        let mut bytes = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::webp::WebPEncoder::new_lossless(&mut bytes),
            &[255, 0, 0, 255],
            1,
            1,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
        service
            .publish(
                card.clone(),
                Sprite {
                    bytes,
                    commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    appearance: 1,
                },
            )
            .await
            .unwrap();
        Some(Self {
            admin,
            pool,
            schema,
            service,
            trust,
            card,
        })
    }
    fn deck(&self) -> Deck {
        Deck {
            id: Uuid::now_v7(),
            character: self.trust.character,
            name: "My deck".into(),
            cards: vec![self.card.id],
        }
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
#[tokio::test]
async fn postgres_collection_success_and_concurrent_retry() {
    let Some(db) = Database::new().await else {
        return;
    };
    let t = &db.trust;
    let s = &db.service;
    assert!(s.cards(t.account, t.character, t, now()).await.unwrap()[0].unlocked);
    let deck = db.deck();
    let id = deck.id;
    s.save_deck(t.account, deck, t, now()).await.unwrap();
    let battle = s.start(t.account, id, t, now()).await.unwrap();
    assert!(matches!(
        s.start(t.account, id, t, now()).await,
        Err(Error::Conflict)
    ));
    let (a, b) = tokio::join!(
        s.turn(t.account, battle, 0, Action::Strike, t, now()),
        s.turn(t.account, battle, 0, Action::Strike, t, now())
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(s.battle(t.account, battle, t, now()).await.unwrap().turn, 1);
    assert!(matches!(
        s.turn(t.account, battle, 0, Action::Guard, t, now()).await,
        Err(Error::Conflict)
    ));
    loop {
        let state = s.battle(t.account, battle, t, now()).await.unwrap();
        if state.outcome != Outcome::Active {
            break;
        }
        s.turn(t.account, battle, state.turn, Action::Strike, t, now())
            .await
            .unwrap();
    }
    let next = s.start(t.account, id, t, now()).await.unwrap();
    assert_eq!(
        s.active_battle(t.account, t.character).await.unwrap(),
        Some(next)
    );
    s.delete_deck(t.account, id).await.unwrap();
    assert!(s.decks(t.account).await.unwrap().is_empty());
    assert_eq!(
        s.battle(t.account, next, t, now()).await.unwrap().outcome,
        Outcome::Active
    );
    s.forfeit(t.account, next).await.unwrap();
    assert_eq!(s.active_battle(t.account, t.character).await.unwrap(), None);
    assert!(s.forfeit(t.account, next).await.is_err());
    db.close().await;
}
#[tokio::test]
async fn postgres_collection_refusal_and_bad_input() {
    let Some(db) = Database::new().await else {
        return;
    };
    let t = &db.trust;
    let s = &db.service;
    let deck = db.deck();
    let id = deck.id;
    s.save_deck(t.account, deck, t, now()).await.unwrap();
    let battle = s.start(t.account, id, t, now()).await.unwrap();
    let mut bad = db.deck();
    bad.cards.push(db.card.id);
    assert!(matches!(
        s.save_deck(t.account, bad, t, now()).await,
        Err(Error::BadInput)
    ));
    let mut bad = db.deck();
    bad.cards = vec![Uuid::now_v7()];
    assert!(matches!(
        s.save_deck(t.account, bad, t, now()).await,
        Err(Error::Denied)
    ));
    assert!(s.battle(Uuid::now_v7(), battle, t, now()).await.is_err());
    assert!(
        s.cards(t.account, t.character, t, now() + 86400001)
            .await
            .is_err()
    );
    t.allow.store(false, Ordering::SeqCst);
    assert!(
        s.turn(t.account, battle, 0, Action::Strike, t, now())
            .await
            .is_err()
    );
    t.allow.store(true, Ordering::SeqCst);
    WorldStore::new(db.pool.clone(), WorldOptions { enabled: true })
        .deny("Creature", "synthetic:fox")
        .await
        .unwrap();
    assert!(matches!(
        s.turn(t.account, battle, 0, Action::Strike, t, now()).await,
        Err(Error::Denied)
    ));
    WorldStore::new(db.pool.clone(), WorldOptions { enabled: true })
        .allow("Creature", "synthetic:fox")
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(t.account)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        s.cards(t.account, t.character, t, now()).await,
        Err(Error::Denied)
    ));
    db.close().await;
}
#[tokio::test]
async fn disabled_router_refuses() {
    use tower::ServiceExt;
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let response = configured_router(pool)
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/collection")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}
#[tokio::test]
async fn postgres_browser_collection_deck_battle_csrf_and_bad_input() {
    use axum::{
        Extension, Router,
        body::{Body, to_bytes},
        extract::State,
        http::{Request, StatusCode, header},
        routing::get,
    };
    use oteryn_web::{
        csrf::CsrfConfig,
        session::{self, BrowserSession, PrivilegeChange, SessionLayer},
    };
    use tower::ServiceExt;
    let Some(db) = Database::new().await else {
        return;
    };
    async fn login(
        State(id): State<Uuid>,
        Extension(session): Extension<BrowserSession>,
    ) -> String {
        session::rotate(&session, PrivilegeChange::Login(id))
            .await
            .unwrap();
        session.csrf_token().await.unwrap()
    }
    let app = Router::new()
        .route("/test-login", get(login))
        .route_layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            session::middleware,
        ))
        .with_state(db.trust.account);
    let response = app
        .oneshot(Request::get("/test-login").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let csrf =
        String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap();
    let app = router(
        db.pool.clone(),
        db.service.clone(),
        Flags { collection: true },
        Ownership(Arc::new(db.trust.clone())),
        CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..CsrfConfig::default()
        },
    );
    for (lang, label) in [("en", "Monster collection"), ("pl", "Kolekcja potworów")] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{lang}/collection/{}", db.trust.character))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let html = String::from_utf8(
            to_bytes(response.into_body(), 100000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        if let Ok(directory) = std::env::var("PORTAL_COLLECTION_HTML_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{lang}.html")),
                html.replace(&csrf, "synthetic-test-token"),
            )
            .unwrap();
        }
        assert!(html.contains(label));
        assert!(html.contains(&csrf));
        assert!(html.contains("name=\"first\""));
        assert!(html.contains("initial-scale=1"));
    }
    let path = format!("/en/collection/{}/decks", db.trust.character);
    for (body, status) in [
        (
            format!("name=Deck&first={}", db.card.id),
            StatusCode::FORBIDDEN,
        ),
        (
            format!("name=Deck&first=bad&_csrf={csrf}"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!("name=Deck&first={}&owner=bad&_csrf={csrf}", db.card.id),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!("name=Deck&first={}&second=&third=&_csrf={csrf}", db.card.id),
            StatusCode::SEE_OTHER,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(&path)
                    .header(header::COOKIE, &cookie)
                    .header(header::ORIGIN, "https://portal.example.invalid")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let deck = db.service.decks(db.trust.account).await.unwrap().remove(0);
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/en/collection/decks/{}/battle", deck.id))
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let path = response.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_owned();
    let response = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header(header::COOKIE, &cookie)
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
    if let Ok(directory) = std::env::var("PORTAL_COLLECTION_HTML_DIR") {
        std::fs::write(
            std::path::Path::new(&directory).join("battle.html"),
            html.replace(&csrf, "synthetic-test-token"),
        )
        .unwrap();
    }
    assert!(html.contains("Strike"));
    assert!(!html.contains("seed"));
    let response = app
        .oneshot(
            Request::post(&path)
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("expected=0&action=strike&_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    db.close().await;
}
