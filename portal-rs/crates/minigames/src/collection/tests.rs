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
    second: Option<(Uuid, Uuid)>,
    allow: Arc<AtomicBool>,
}
impl Trust {
    fn owns(&self, account: Uuid, character: Option<Uuid>) -> bool {
        (account == self.account && character.is_none_or(|id| id == self.character))
            || self.second.is_some_and(|(other, subject)| {
                account == other && character.is_none_or(|id| id == subject)
            })
    }
}
impl Authority for Trust {
    fn lock_mutation<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: Uuid,
        character: Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = progress::Result<()>> + Send + 'a>>
    {
        Box::pin(async move {
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .fetch_one(&mut **tx)
                .await?;
            if !self.owns(account, Some(character)) || !self.allow.load(Ordering::SeqCst) {
                return Err(progress::Error::Denied);
            }
            Ok(())
        })
    }
    fn source(&self) -> &str {
        "synthetic:journal"
    }
    fn permits(&self, s: Subject<'_>) -> bool {
        self.allow.load(Ordering::SeqCst)
            && self.owns(s.account, s.character)
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
async fn publish_bestiary(
    progress: &progress::ProgressStore,
    trust: &Trust,
    card: &Card,
    account: Uuid,
    character: Uuid,
) {
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
        account_id: account,
        character_id: Some(character),
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
        .ingest(&serde_json::to_vec(&record).unwrap(), trust)
        .await
        .unwrap();
    let w = progress::Watermark {
        contract_version: 1,
        operation: "PublishGameJournalProgressWatermarkV1".into(),
        source_authority: trust.source().into(),
        account_id: account,
        projection_epoch: "1".into(),
        recovery_generation: "1".into(),
        family: "bestiary".into(),
        complete_through: "1".into(),
        observed_at_unix_ms: now().to_string(),
        baseline_complete: true,
    };
    progress
        .watermark(&serde_json::to_vec(&w).unwrap(), trust, now())
        .await
        .unwrap();
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
            second: None,
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
        publish_bestiary(&progress, &trust, &card, trust.account, trust.character).await;
        let service = Collection::new(
            pool.clone(),
            world,
            progress,
            Flags { collection: true },
            Ownership(Arc::new(trust.clone())),
        );
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
    assert_eq!(
        s.cards(t.account, t.character, now()).await.unwrap()[0].state,
        CardState::Unlocked
    );
    let deck = db.deck();
    let id = deck.id;
    s.save_deck(t.account, deck, now()).await.unwrap();
    let battle = s.start(t.account, id, now()).await.unwrap();
    assert!(matches!(
        s.start(t.account, id, now()).await,
        Err(Error::Conflict)
    ));
    let (a, b) = tokio::join!(
        s.turn(t.account, battle, 0, Action::Strike, now()),
        s.turn(t.account, battle, 0, Action::Strike, now())
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(s.battle(t.account, battle, now()).await.unwrap().turn, 1);
    assert!(matches!(
        s.turn(t.account, battle, 0, Action::Guard, now()).await,
        Err(Error::Conflict)
    ));
    loop {
        let state = s.battle(t.account, battle, now()).await.unwrap();
        if state.outcome != Outcome::Active {
            break;
        }
        s.turn(t.account, battle, state.turn, Action::Strike, now())
            .await
            .unwrap();
    }
    let next = s.start(t.account, id, now()).await.unwrap();
    assert_eq!(
        s.active_battle(t.account, t.character).await.unwrap(),
        Some(next)
    );
    s.delete_deck(t.account, id).await.unwrap();
    assert!(s.decks(t.account).await.unwrap().is_empty());
    assert_eq!(
        s.battle(t.account, next, now()).await.unwrap().outcome,
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
    s.save_deck(t.account, deck, now()).await.unwrap();
    let battle = s.start(t.account, id, now()).await.unwrap();
    let mut bad = db.deck();
    bad.cards.push(db.card.id);
    assert!(matches!(
        s.save_deck(t.account, bad, now()).await,
        Err(Error::BadInput)
    ));
    let mut bad = db.deck();
    bad.cards = vec![Uuid::now_v7()];
    assert!(matches!(
        s.save_deck(t.account, bad, now()).await,
        Err(Error::Denied)
    ));
    assert!(s.battle(Uuid::now_v7(), battle, now()).await.is_err());
    assert!(
        s.cards(t.account, t.character, now() + 86400001)
            .await
            .is_err()
    );
    t.allow.store(false, Ordering::SeqCst);
    assert!(
        s.turn(t.account, battle, 0, Action::Strike, now())
            .await
            .is_err()
    );
    t.allow.store(true, Ordering::SeqCst);
    WorldStore::new(db.pool.clone(), WorldOptions { enabled: true })
        .deny("Creature", "synthetic:fox")
        .await
        .unwrap();
    assert!(matches!(
        s.turn(t.account, battle, 0, Action::Strike, now()).await,
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
        s.cards(t.account, t.character, now()).await,
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
    async fn browser_identity(pool: &sqlx::PgPool, account: Uuid) -> (String, String) {
        let app = Router::new()
            .route("/test-login", get(login))
            .route_layer(axum::middleware::from_fn_with_state(
                SessionLayer::new(pool.clone()),
                session::middleware,
            ))
            .with_state(account);
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
        let csrf = String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec())
            .unwrap();
        (cookie, csrf)
    }
    let (cookie, csrf) = browser_identity(&db.pool, db.trust.account).await;
    // A publisher holding the feed row must hit the PostgreSQL lock timeout
    // before the outer HTTP deadline, while the publisher still holds the lock.
    let mut publisher = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE")
        .fetch_one(&mut *publisher)
        .await
        .unwrap();
    let evidence_app = http::router_with_evidence(
        db.pool.clone(),
        db.service.clone(),
        Flags { collection: true },
        CsrfConfig::default(),
        Some(http::EvidenceSources {
            inventory: "items".into(),
            progress: "progress".into(),
        }),
    );
    for (path, expected) in [
        ("/en/collection".to_owned(), StatusCode::OK),
        (
            format!("/en/collection/{}", db.trust.character),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let response = evidence_app
            .clone()
            .oneshot(
                Request::get(path)
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
    }
    publisher.rollback().await.unwrap();

    let mut accounts = db.pool.begin().await.unwrap();
    let foreign = oteryn_identity::accounts::Accounts::create(
        &mut accounts,
        "collection-foreign@example.invalid",
        "unused",
        "en",
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        oteryn_identity::accounts::Accounts::mark_verified(&mut accounts, foreign.id)
            .await
            .unwrap()
    );
    accounts.commit().await.unwrap();
    let (foreign_cookie, foreign_csrf) = browser_identity(&db.pool, foreign.id).await;
    let foreign_character = Uuid::now_v7();
    let mut trust = db.trust.clone();
    trust.second = Some((foreign.id, foreign_character));
    let progress = progress::ProgressStore::new(
        db.pool.clone(),
        progress::Flags {
            ingest: true,
            read: true,
        },
        vec!["bestiary".into()],
    );
    publish_bestiary(&progress, &trust, &db.card, foreign.id, foreign_character).await;
    let app = router(
        db.pool.clone(),
        db.service
            .clone()
            .with_ownership(Ownership(Arc::new(trust))),
        Flags { collection: true },
        CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..CsrfConfig::default()
        },
    );
    let mut deck_id = Uuid::nil();
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
        let (_, value) = html.split_once("name=\"id\" value=\"").unwrap();
        deck_id = value.split('"').next().unwrap().parse().unwrap();
        assert_eq!(deck_id.get_version_num(), 7);
    }
    let path = format!("/en/collection/{}/decks", db.trust.character);
    let save = format!(
        "name=Deck&first={}&second=&third=&id={deck_id}&_csrf={csrf}",
        db.card.id
    );
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
        (save.clone(), StatusCode::SEE_OTHER),
        (save, StatusCode::SEE_OTHER),
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
    let mut decks = db.service.decks(db.trust.account).await.unwrap();
    assert_eq!(decks.len(), 1);
    let deck = decks.remove(0);
    assert_eq!(deck.id, deck_id);
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
    let snapshot = || async {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT jsonb_build_object(
                'decks',(SELECT jsonb_agg(d ORDER BY d.id) FROM collection_decks d WHERE d.account_id IN ($1,$2)),
                'battles',(SELECT jsonb_agg(b ORDER BY b.id) FROM collection_battles b WHERE b.account_id IN ($1,$2))
            )",
        )
        .bind(db.trust.account)
        .bind(foreign.id)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    };
    let battle = path.rsplit('/').next().unwrap();
    let before = snapshot().await;
    for lang in ["en", "pl"] {
        let root = format!("/{lang}/collection");
        let character = db.trust.character;
        let card = db.card.id;
        let image = format!("{root}/{character}/cards/{card}/image");
        let owner_image = app
            .clone()
            .oneshot(
                Request::get(&image)
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(owner_image.status(), StatusCode::OK);
        assert_eq!(owner_image.headers()[header::CONTENT_TYPE], "image/webp");
        let own_image = app
            .clone()
            .oneshot(
                Request::get(format!("{root}/{foreign_character}/cards/{card}/image"))
                    .header(header::COOKIE, &foreign_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(own_image.status(), StatusCode::OK);
        let unavailable = StatusCode::SERVICE_UNAVAILABLE;
        let denied = StatusCode::FORBIDDEN;
        for (method, path, fields, expected) in [
            (
                "GET",
                format!("{root}/{character}"),
                String::new(),
                unavailable,
            ),
            ("GET", image, String::new(), unavailable),
            (
                "POST",
                format!("{root}/{character}/decks"),
                format!("name=Foreign+character&first={card}"),
                unavailable,
            ),
            (
                "POST",
                format!("{root}/{foreign_character}/decks"),
                format!("name=Foreign+edit&first={card}&id={}", deck.id),
                denied,
            ),
            (
                "POST",
                format!("{root}/decks/{}/delete", deck.id),
                String::new(),
                denied,
            ),
            (
                "POST",
                format!("{root}/decks/{}/battle", deck.id),
                String::new(),
                denied,
            ),
            (
                "GET",
                format!("{root}/battles/{battle}"),
                String::new(),
                denied,
            ),
            (
                "POST",
                format!("{root}/battles/{battle}"),
                "expected=0&action=strike".into(),
                denied,
            ),
            (
                "POST",
                format!("{root}/battles/{battle}/forfeit"),
                String::new(),
                denied,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(&path)
                        .header(header::COOKIE, &foreign_cookie)
                        .header(header::ORIGIN, "https://portal.example.invalid")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .header("x-csrf-token", &foreign_csrf)
                        .body(Body::from(format!("_csrf={foreign_csrf}&{fields}")))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "private, no-store"
            );
            let body = String::from_utf8(
                to_bytes(response.into_body(), 100000)
                    .await
                    .unwrap()
                    .to_vec(),
            )
            .unwrap();
            for private in [
                character.to_string(),
                card.to_string(),
                deck.id.to_string(),
                battle.into(),
            ] {
                assert!(!body.contains(&private), "{method} {path}");
            }
            assert_eq!(snapshot().await, before, "{method} {path}");
        }
    }
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

#[tokio::test]
async fn postgres_only_visible_cards_count_towards_catalogue_limit() {
    let started = std::time::Instant::now();
    let db = Database::new().await.unwrap();
    let retired = Uuid::now_v7();
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
        .bind(retired)
        .bind("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .execute(&db.pool)
        .await
        .unwrap();
    // One statement: the progress stream seeded by the fixture turns stale after ten
    // seconds, and a thousand round trips can exceed that on a loaded database.
    let (ids, definitions): (Vec<Uuid>, Vec<serde_json::Value>) = (0..2000)
        .map(|i| {
            let mut c = db.card.clone();
            c.id = Uuid::now_v7();
            if i < 1000 {
                c.world_version = retired;
            } else {
                c.key = "not-published".into();
            }
            (c.id, serde_json::to_value(c).unwrap())
        })
        .unzip();
    let missing_sprite = ids[1000];
    sqlx::query("INSERT INTO collection_cards(id,definition,sprite) SELECT u.id,u.definition,c.sprite FROM UNNEST($1::uuid[],$2::jsonb[]) AS u(id,definition) CROSS JOIN (SELECT sprite FROM collection_cards WHERE id=$3) AS c")
        .bind(ids)
        .bind(definitions)
        .bind(db.card.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let cards = db
        .service
        .cards(db.trust.account, db.trust.character, now())
        .await
        .unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].card.id, db.card.id);
    assert!(matches!(
        db.service
            .sprite(db.trust.account, db.trust.character, missing_sprite, now())
            .await,
        Err(Error::Denied)
    ));
    db.service
        .save_deck(db.trust.account, db.deck(), now())
        .await
        .unwrap();
    sqlx::query("UPDATE world_active_version SET version_id=NULL WHERE singleton")
        .execute(&db.pool)
        .await
        .unwrap();
    for id in [db.card.id, missing_sprite] {
        assert!(matches!(
            db.service
                .sprite(db.trust.account, db.trust.character, id, now())
                .await,
            Err(Error::Unavailable)
        ));
    }
    db.close().await;
    eprintln!(
        "postgres_only_visible_cards_count_towards_catalogue_limit duration={:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn postgres_collection_rechecks_account_after_wait() {
    let started = std::time::Instant::now();
    let db = Database::new().await.unwrap();
    let account = db.trust.account;
    let deck = db.deck();
    let id = deck.id;
    db.service.save_deck(account, deck, now()).await.unwrap();
    let battle = db.service.start(account, id, now()).await.unwrap();
    for operation in 0..5 {
        let mut suspension = db.pool.begin().await.unwrap();
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(account)
            .execute(&mut *suspension)
            .await
            .unwrap();
        {
            let mutation = async {
                match operation {
                    0 => {
                        let mut replacement = db.deck();
                        replacement.id = id;
                        replacement.name = "Changed".into();
                        db.service.save_deck(account, replacement, now()).await
                    }
                    1 => db.service.start(account, id, now()).await.map(|_| ()),
                    2 => {
                        db.service
                            .turn(account, battle, 0, Action::Strike, now())
                            .await
                    }
                    3 => db.service.forfeit(account, battle).await.map(|_| ()),
                    _ => db.service.delete_deck(account, id).await.map(|_| ()),
                }
            };
            tokio::pin!(mutation);
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    tokio::select! {
                        result = &mut mutation => panic!("operation {operation} passed suspension: {result:?}"),
                        _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
                    }
                    let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='accounts'::regclass))")
                        .fetch_one(&db.pool).await.unwrap();
                    if waiting { break; }
                }
            }).await.unwrap();
            suspension.commit().await.unwrap();
            assert!(
                matches!(mutation.await, Err(Error::Denied)),
                "operation {operation}"
            );
        }
        sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
            .bind(account)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let decks = db.service.decks(account).await.unwrap();
    assert_eq!(decks.len(), 1);
    assert_eq!(decks[0].name, "My deck");
    let state = db.service.battle(account, battle, now()).await.unwrap();
    assert_eq!(state.turn, 0);
    assert_eq!(state.outcome, Outcome::Active);
    db.close().await;
    eprintln!(
        "postgres_collection_rechecks_account_after_wait duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_collection_rechecks_source_and_unlock_after_account_wait() {
    let started = std::time::Instant::now();
    for revoked in ["visibility", "version", "unlock", "ownership"] {
        let db = Database::new().await.unwrap();
        let deck = db.deck();
        let mut blocker = db.pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(db.trust.account)
            .execute(&mut *blocker)
            .await
            .unwrap();
        // The source is withdrawn in the blocking transaction, so the save's
        // unlocked reads still see it and only the recheck under lock can
        // refuse it. Committing releases the account row right after the save
        // starts waiting, well inside the 500 ms browser lock bound (W3-04).
        let withdraw = match revoked {
            "visibility" => Some("UPDATE world_visibility SET allowed=false"),
            "version" => Some("UPDATE world_active_version SET version_id=NULL"),
            "unlock" => Some("UPDATE pd_progress_streams SET baseline_complete=false"),
            _ => None,
        };
        if let Some(withdraw) = withdraw {
            sqlx::query(withdraw).execute(&mut *blocker).await.unwrap();
        }
        let service = db.service.clone();
        let trust = db.trust.clone();
        let save = tokio::spawn(async move { service.save_deck(trust.account, deck, now()).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='accounts'::regclass))").fetch_one(&db.pool).await.unwrap();
                if waiting { break; }
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
        if withdraw.is_none() {
            db.trust.allow.store(false, Ordering::SeqCst);
        }
        blocker.commit().await.unwrap();
        let saved = save.await.unwrap();
        assert!(saved.is_err(), "{revoked}: {saved:?}");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM collection_decks")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        db.close().await;
    }
    eprintln!(
        "postgres_collection_rechecks_source_and_unlock_after_account_wait duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_journal_collection_completes_battle_from_published_progress() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::new().await else {
        return;
    };
    let account = db.trust.account;
    let character = db.trust.character;
    let world = Uuid::now_v7();
    let other = Uuid::now_v7();
    let source = "synthetic:collection";
    let service = Collection::with_journal(
        db.pool.clone(),
        WorldStore::new(db.pool.clone(), WorldOptions { enabled: true }),
        Some(source.to_owned()),
        Flags { collection: true },
    );
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale)
                 VALUES($1,'other@example.invalid','unused','active',now(),'en')",
    )
    .bind(other)
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',
                 source_authority=$1,complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,
                 observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1")
        .bind(source)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,
                 projection_revision,source_observed_at,content_digest)
                 VALUES($1,$2,'1','1',0,decode(repeat('00',32),'hex'))",
    )
    .bind(account)
    .bind(source)
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
                 VALUES($1,$2,$3,'Fox Keeper','AVAILABLE')",
    )
    .bind(account)
    .bind(character)
    .bind(world)
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,
                 from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at)
                 VALUES($1,$2,$3,1,1,1,1,1,now()-interval '1 hour')")
        .bind(character)
        .bind(account)
        .bind(world)
        .execute(&db.pool)
        .await
        .unwrap();

    let deck = db.deck();
    assert!(matches!(
        service.cards(other, character, now()).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        service.save_deck(account, deck.clone(), now()).await,
        Err(Error::Unavailable)
    ));
    let mut invalid = deck.clone();
    invalid.cards.push(db.card.id);
    assert!(matches!(
        service.save_deck(account, invalid, now()).await,
        Err(Error::BadInput)
    ));

    // Isolated reader fixture representing the receiver's committed cut.
    let stream = format!("{}:{source}|36:{account}|1:1|8:bestiary", source.len());
    let subject = format!("36:{character}|36:{world}|1:1|1:1|1:1");
    let observed = now() as i64;
    let payload = serde_json::to_vec(&serde_json::json!({
        "race_key": db.card.race_key,
        "race_definition_revision": db.card.race_revision,
        "kill_count": db.card.final_kill_threshold,
        "final_kill_threshold": db.card.final_kill_threshold,
        "charm_points_awarded": 0,
    }))
    .unwrap();
    sqlx::query("INSERT INTO pd_streams(stream_key,authority,account_id,family,progress,highest_seen,
                stored_through,applied_through,published_through,observed_ms,baseline_complete,watermark_recovery)
                VALUES($1,$2,$3,'bestiary',true,1,1,1,0,$4,true,1)")
        .bind(&stream)
        .bind(source)
        .bind(account.to_string())
        .bind(observed)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,
                 entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms,ownership_revision)
                 VALUES($1,1,$2,'',$3,$4,$5,$6,'UPSERT',$7,$7,decode(repeat('00',32),'hex'),$8,$7+60000,'1')")
        .bind(&stream)
        .bind(source)
        .bind(Uuid::now_v7().to_string())
        .bind(&db.card.race_key)
        .bind(character.to_string())
        .bind(subject)
        .bind(observed)
        .bind(payload)
        .execute(&db.pool)
        .await
        .unwrap();

    let state = service.cards(account, character, now()).await.unwrap()[0].state;
    assert_eq!(state, CardState::Unavailable);
    sqlx::query("UPDATE pd_streams SET published_through=1 WHERE stream_key=$1")
        .bind(&stream)
        .execute(&db.pool)
        .await
        .unwrap();

    let state = service.cards(account, character, now()).await.unwrap()[0].state;
    assert_eq!(state, CardState::Unlocked);
    let id = deck.id;
    service.save_deck(account, deck, now()).await.unwrap();
    let battle = service.start(account, id, now()).await.unwrap();
    assert!(matches!(
        service.start(account, id, now()).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        service
            .turn(account, battle, 1, Action::Strike, now())
            .await,
        Err(Error::Conflict)
    ));
    for turn in 0..100 {
        service
            .turn(account, battle, turn, Action::Strike, now())
            .await
            .unwrap();
        let state = service.battle(account, battle, now()).await.unwrap();
        if state.outcome != Outcome::Active {
            break;
        }
    }
    let state = service.battle(account, battle, now()).await.unwrap();
    assert_ne!(state.outcome, Outcome::Active);

    // A retained expiry marker has no race key. New published evidence can
    // establish a card again without using values from before that marker.
    sqlx::query(
        "INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,
             entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms,ownership_revision)
         SELECT stream_key,2,authority,recovery_generation,uuidv7()::text,
             '',NULL,scope_key,'REMOVE',0,0,digest,NULL,occurred_ms-1,ownership_revision
         FROM pd_records WHERE stream_key=$1 AND source_revision=1",
    )
    .bind(&stream)
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE pd_streams SET stored_through=2,applied_through=2,published_through=2,highest_seen=2 WHERE stream_key=$1")
        .bind(&stream)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        service.cards(account, character, now()).await.unwrap()[0].state,
        CardState::Unavailable
    );
    sqlx::query(
        "INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,
             entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms,ownership_revision)
         SELECT stream_key,3,authority,recovery_generation,uuidv7()::text,
             entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms,ownership_revision
         FROM pd_records WHERE stream_key=$1 AND source_revision=1",
    )
    .bind(&stream)
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE pd_streams SET stored_through=3,applied_through=3,published_through=3,highest_seen=3 WHERE stream_key=$1")
        .bind(&stream)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        service.cards(account, character, now()).await.unwrap()[0].state,
        CardState::Unlocked
    );

    sqlx::query("UPDATE pd_streams SET observed_ms=observed_ms-11000 WHERE stream_key=$1")
        .bind(&stream)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        service.battle(account, battle, now()).await,
        Err(Error::Unavailable)
    ));
    db.close().await;
}
