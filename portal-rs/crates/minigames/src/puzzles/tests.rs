#![allow(clippy::unwrap_used)]
use super::*;
#[path = "../../../../test-support/clock.rs"]
mod test_clock;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode, header},
    routing::get,
};
use image::{ExtendedColorType, ImageEncoder, codecs::webp::WebPEncoder};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{self, BrowserSession, PrivilegeChange, SessionLayer},
};
use oteryn_world::{WorldOptions, WorldStore};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const POOL_CONNECTIONS: u32 = 6;

// All records and images in this module are synthetic; no game fixtures are copied.
fn text(value: &str) -> Text {
    Text {
        pl: format!("PL {value}"),
        en: value.into(),
    }
}
fn definition(version: Uuid, kind: Kind) -> Definition {
    Definition {
        kind,
        world_version: version,
        question: text("Which creature lives here?"),
        clue: text("A small creature with pointed ears."),
        choices: ["fox", "wolf"]
            .into_iter()
            .map(|key| Choice {
                label: text(key),
                source: EntityRef {
                    kind: "synthetic-creature".into(),
                    key: key.into(),
                },
            })
            .collect(),
        correct_choice: 1,
    }
}
fn webp() -> Vec<u8> {
    let mut bytes = Vec::new();
    WebPEncoder::new_lossless(&mut bytes)
        .write_image(
            &[20, 50, 100, 255].repeat(256 * 256),
            256,
            256,
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    bytes
}
fn media(version: Uuid, kind: Kind) -> Option<Media> {
    match kind {
        Kind::Quiz => None,
        Kind::Monster => Some(Media {
            kind,
            version: COMMIT.into(),
            bytes: webp(),
        }),
        Kind::Atlas => Some(Media {
            kind,
            version: version.to_string(),
            bytes: webp(),
        }),
    }
}
struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    world: WorldStore,
    service: Puzzles,
    version: Uuid,
    account: Uuid,
    today: Date,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let url = match std::env::var("PORTAL_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => {
                eprintln!("SKIPPED PostgreSQL puzzle test: PORTAL_TEST_DATABASE_URL is unset");
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("puzzles_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = format!("{schema},pg_catalog");
        let pool = PgPoolOptions::new()
            .max_connections(POOL_CONNECTIONS)
            .after_connect(move |conn, _| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search_path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let instant = time::OffsetDateTime::from_unix_timestamp(1_790_035_199)?;
        test_clock::install(&pool, instant).await?;
        let version = Uuid::now_v7();
        sqlx::query("INSERT INTO world_versions (id,source_commit,status) VALUES ($1,$2,'ready')")
            .bind(version)
            .bind(COMMIT)
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE world_active_version SET version_id=$1")
            .bind(version)
            .execute(&pool)
            .await?;
        for key in ["fox", "wolf"] {
            sqlx::query("INSERT INTO world_entities (version_id,kind,key,data,publishable) VALUES ($1,'synthetic-creature',$2,'{}',true)").bind(version).bind(key).execute(&pool).await?;
        }
        let world = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
        world.allow("synthetic-creature", "fox").await?;
        world.allow("synthetic-creature", "wolf").await?;
        let service = Puzzles::new(
            pool.clone(),
            world.clone(),
            Flags {
                daily_puzzles: true,
            },
        );
        let account = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts (id,email,password_hash,status,email_verified_at,locale) VALUES ($1,'synthetic@example.invalid','unused','active',now(),'en')").bind(account).execute(&pool).await?;
        let today = instant.date();
        Ok(Some(Self {
            admin,
            pool,
            schema,
            world,
            service,
            version,
            account,
            today,
        }))
    }
    async fn publish(&self, kind: Kind) -> Result<Uuid, Error> {
        self.service
            .publish(
                self.today,
                definition(self.version, kind),
                media(self.version, kind),
            )
            .await
    }
    async fn seed(&self, day: Date, kind: Kind) -> Result<Uuid, Error> {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO puzzles_daily (id,day,kind,world_version,definition,image) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(id).bind(day).bind(kind.key()).bind(self.version).bind(sqlx::types::Json(definition(self.version,kind)))
            .bind(media(self.version,kind).map(|media|media.bytes)).execute(&self.pool).await?;
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
    fn router(&self) -> Router {
        router(
            self.pool.clone(),
            self.world.clone(),
            Flags {
                daily_puzzles: true,
            },
            CsrfConfig {
                trusted_origins: vec!["https://portal.example.invalid".into()],
                ..CsrfConfig::default()
            },
        )
    }
    async fn login(&self) -> Result<(String, String), Box<dyn std::error::Error>> {
        // Test-only authentication adapter exercises the production session API.
        async fn login(
            State(id): State<Uuid>,
            Extension(session): Extension<BrowserSession>,
        ) -> Result<String, StatusCode> {
            session::rotate(&session, PrivilegeChange::Login(id))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        let app = Router::new()
            .route("/test-login", get(login))
            .route_layer(axum::middleware::from_fn_with_state(
                SessionLayer::new(self.pool.clone()),
                session::middleware,
            ))
            .with_state(self.account);
        let response = app
            .oneshot(Request::get("/test-login").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let csrf = String::from_utf8(to_bytes(response.into_body(), 4096).await?.to_vec())?;
        Ok((cookie, csrf))
    }
}

#[test]
fn definitions_refuse_ambiguous_or_bad_input_and_badges_are_durable() {
    let valid = definition(Uuid::now_v7(), Kind::Quiz);
    assert!(valid.validate().is_ok());
    for change in 0..6 {
        let mut bad = valid.clone();
        match change {
            0 => bad.correct_choice = 6,
            1 => bad.choices[1] = bad.choices[0].clone(),
            2 => bad.clue.pl.clear(),
            3 => bad.choices.truncate(1),
            4 => bad.choices[0].source.key = "\n".into(),
            _ => bad.world_version = Uuid::nil(),
        }
        assert!(matches!(bad.validate(), Err(Error::Invalid)));
    }
    assert_eq!(
        Progress {
            current: 0,
            best: 7
        }
        .badges(),
        vec![3, 7]
    );
    assert!(Progress::default().badges().is_empty());
}

#[test]
fn media_are_verified_and_monster_pixels_are_silhouettes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = webp();
    std::fs::write(dir.path().join("sprite.webp"), &bytes)?;
    let request = oteryn_world::sprites::Request {
        kind: oteryn_world::sprites::Kind::Creature,
        appearance_id: 1,
        pattern: None,
    };
    let mut manifest = oteryn_world::sprites::Manifest {
        format: 1,
        game_commit: COMMIT.into(),
        sources: Default::default(),
        thumbnails: vec![oteryn_world::sprites::Thumbnail {
            request,
            pattern: [0, 0, 0],
            file: "sprite.webp".into(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            width: 256,
            height: 256,
        }],
    };
    let media = Media::monster(dir.path(), &manifest, request)?;
    let decoded = image::load_from_memory(&media.bytes)?.to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0).0, [240, 240, 240, 255]);
    manifest.thumbnails[0].sha256 = "bad".into();
    assert!(Media::monster(dir.path(), &manifest, request).is_err());
    manifest.thumbnails[0].file = "../sprite.webp".into();
    assert!(matches!(
        Media::monster(dir.path(), &manifest, request),
        Err(Error::Invalid)
    ));
    let tile = oteryn_atlas::tiles::Tile {
        path: "sprite.webp".into(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
    };
    let mut atlas = oteryn_atlas::tiles::Manifest {
        format_version: 1,
        world_version: Uuid::now_v7().to_string(),
        floor: 7,
        tile_size: 256,
        bounds: oteryn_atlas::tiles::Bounds {
            min_x: 0,
            min_y: 0,
            max_x: 256,
            max_y: 256,
        },
        levels: vec![],
        tiles: vec![tile],
    };
    assert!(Media::atlas(dir.path(), &atlas, "sprite.webp").is_ok());
    atlas.tiles.clear();
    assert!(Media::atlas(dir.path(), &atlas, "sprite.webp").is_err());
    Ok(())
}

#[tokio::test]
async fn postgres_publication_is_immutable_and_validates_media_and_sources() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.publish(Kind::Quiz).await?;
    assert!(matches!(db.publish(Kind::Quiz).await, Err(Error::Conflict)));
    assert!(matches!(
        db.service
            .publish(db.today, definition(db.version, Kind::Monster), None)
            .await,
        Err(Error::Invalid)
    ));
    let mut stale = media(db.version, Kind::Monster).unwrap();
    stale.version = "b".repeat(40);
    assert!(matches!(
        db.service
            .publish(db.today, definition(db.version, Kind::Monster), Some(stale))
            .await,
        Err(Error::Unavailable)
    ));
    let old = db.today.previous_day().unwrap();
    assert!(matches!(
        db.service
            .publish(old, definition(db.version, Kind::Quiz), None)
            .await,
        Err(Error::Conflict)
    ));
    assert!(
        sqlx::query("UPDATE puzzles_daily SET definition='{}' WHERE id=$1")
            .bind(id)
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM puzzles_daily WHERE id=$1")
            .bind(id)
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert_eq!(db.service.today(db.account).await?.len(), 1);
    db.close().await
}

#[tokio::test]
async fn postgres_one_attempt_replay_wrong_answer_and_account_isolation() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.publish(Kind::Quiz).await?;
    assert!(matches!(
        db.service.answer(db.account, id, 8).await,
        Err(Error::Invalid)
    ));
    assert!(db.service.today(db.account).await?[0].answer.is_none());
    let wrong = db.service.answer(db.account, id, 0).await?;
    assert!(!wrong.correct);
    assert!(db.service.answer(db.account, id, 0).await?.replayed);
    assert!(matches!(
        db.service.answer(db.account, id, 1).await,
        Err(Error::Conflict)
    ));
    let other:Uuid=sqlx::query_scalar("INSERT INTO accounts (email,password_hash,status,email_verified_at,locale) VALUES ('other@example.invalid','unused','active',now(),'pl') RETURNING id").fetch_one(&db.pool).await?;
    assert!(db.service.today(other).await?[0].answer.is_none());
    assert!(db.service.answer(other, id, 1).await?.correct);
    assert_eq!(db.service.progress(db.account).await?.current, 0);
    assert!(matches!(
        db.service.answer(Uuid::now_v7(), id, 1).await,
        Err(Error::Forbidden)
    ));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(db.account)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        db.service.today(db.account).await,
        Err(Error::Forbidden)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_concurrent_answers_complete_once_and_award_streak_badges() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO puzzles_streaks (account_id,last_completed,current_streak,best_streak) VALUES ($1,$2-1,6,6)").bind(db.account).bind(db.today).execute(&db.pool).await?;
    let monster = db.publish(Kind::Monster).await?;
    let atlas = db.publish(Kind::Atlas).await?;
    let quiz = db.publish(Kind::Quiz).await?;
    let (a, b, c, d) = tokio::join!(
        db.service.answer(db.account, monster, 1),
        db.service.answer(db.account, atlas, 1),
        db.service.answer(db.account, quiz, 1),
        db.service.answer(db.account, quiz, 1)
    );
    assert!(a?.correct && b?.correct && c?.correct && d?.correct);
    assert_eq!(
        db.service.progress(db.account).await?,
        Progress {
            current: 7,
            best: 7
        }
    );
    assert_eq!(db.service.progress(db.account).await?.badges(), vec![3, 7]);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 3);
    // As many first answers as pool connections, one per account, must all
    // finish through the full answer path: the public reads before the
    // transaction release their connections, so nothing starves, and distinct
    // accounts never queue on each other's account row.
    let mut accounts = std::collections::HashSet::from([db.account]);
    for index in 1..POOL_CONNECTIONS {
        let account = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts (id,email,password_hash,status,email_verified_at,locale) VALUES ($1,$2,'unused','active',now(),'en')")
            .bind(account).bind(format!("synthetic{index}@example.invalid")).execute(&db.pool).await?;
        accounts.insert(account);
    }
    assert_eq!(accounts.len(), POOL_CONNECTIONS as usize);
    let others: Vec<Uuid> = accounts
        .iter()
        .copied()
        .filter(|account| *account != db.account)
        .collect();
    let mut requests = tokio::task::JoinSet::new();
    for account in others.iter().copied().chain([db.account]) {
        let service = db.service.clone();
        // The other accounts answer for the first time; this one replays.
        requests.spawn(async move { (account, service.answer(account, quiz, 1).await) });
    }
    let mut finished = std::collections::HashSet::new();
    while let Some(joined) = requests.join_next().await {
        let (account, answer) = joined?;
        let answer = answer?;
        assert!(answer.correct);
        assert_eq!(answer.replayed, account == db.account);
        assert!(finished.insert(account));
    }
    assert_eq!(finished, accounts);
    // One account beyond the pool sheds load at the W3-04 bounds instead of
    // queueing: pool admission (250 ms) and the account row lock (500 ms).
    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let service = db.service.clone();
        let account = db.account;
        requests.spawn(async move { service.answer(account, quiz, 1).await });
    }
    let mut replayed = 0;
    while let Some(answer) = requests.join_next().await {
        match answer? {
            Ok(answer) => {
                assert!(answer.replayed);
                replayed += 1;
            }
            Err(Error::Database(sqlx::Error::PoolTimedOut)) => {}
            Err(Error::Database(sqlx::Error::Database(error)))
                if error.code().as_deref() == Some("55P03") => {}
            Err(error) => return Err(error.into()),
        }
    }
    assert!(replayed >= 1);
    assert!(db.service.answer(db.account, quiz, 1).await?.replayed);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers WHERE account_id=$1")
        .bind(db.account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 3);
    assert_eq!(db.service.progress(db.account).await?.current, 7);
    db.close().await
}

#[tokio::test]
async fn postgres_gap_resets_streak_without_removing_badges_and_days_expire() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO puzzles_streaks (account_id,last_completed,current_streak,best_streak) VALUES ($1,$2-2,7,30)").bind(db.account).bind(db.today).execute(&db.pool).await?;
    assert_eq!(
        db.service.progress(db.account).await?,
        Progress {
            current: 0,
            best: 30
        }
    );
    let past = db
        .seed(db.today.previous_day().unwrap(), Kind::Quiz)
        .await?;
    let future = db.seed(db.today.next_day().unwrap(), Kind::Quiz).await?;
    for id in [past, future] {
        assert!(matches!(
            db.service.answer(db.account, id, 1).await,
            Err(Error::NotFound)
        ));
    }
    for kind in [Kind::Monster, Kind::Atlas, Kind::Quiz] {
        let id = db.publish(kind).await?;
        db.service.answer(db.account, id, 1).await?;
    }
    assert_eq!(
        db.service.progress(db.account).await?,
        Progress {
            current: 1,
            best: 30
        }
    );
    assert_eq!(
        db.service.progress(db.account).await?.badges(),
        vec![3, 7, 30]
    );
    assert_eq!(db.service.today(db.account).await?.len(), 3);
    db.close().await
}

#[tokio::test]
async fn postgres_hidden_inactive_or_disabled_sources_fail_closed() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.publish(Kind::Monster).await?;
    db.world.deny("synthetic-creature", "wolf").await?;
    assert!(matches!(
        db.service.today(db.account).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        db.service.image(db.account, id).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        db.service.answer(db.account, id, 1).await,
        Err(Error::Unavailable)
    ));
    db.world.allow("synthetic-creature", "wolf").await?;
    assert!(!db.service.image(db.account, id).await?.is_empty());
    sqlx::query("UPDATE world_active_version SET previous_version_id=version_id,version_id=NULL")
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        db.service.today(db.account).await,
        Err(Error::Unavailable)
    ));
    let disabled = Puzzles::new(db.pool.clone(), db.world.clone(), Flags::default());
    assert!(matches!(
        disabled.today(db.account).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled.answer(db.account, id, 1).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled
            .publish(db.today, definition(db.version, Kind::Quiz), None)
            .await,
        Err(Error::Disabled)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_browser_flow_localization_csrf_bad_input_and_replay() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.publish(Kind::Quiz).await?;
    let anonymous = db
        .router()
        .oneshot(Request::get("/en/puzzles").body(Body::empty())?)
        .await?;
    assert_eq!(anonymous.status(), StatusCode::SEE_OTHER);
    assert_eq!(anonymous.headers()[header::LOCATION], "/en/login");
    let (cookie, csrf) = db.login().await?;
    for (lang, title) in [("en", "Daily puzzles"), ("pl", "Codzienne zagadki")] {
        let response = db
            .router()
            .oneshot(
                Request::get(format!("/{lang}/puzzles"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(html.contains(title));
        assert!(html.contains("<label for=\"choice-"));
        assert!(!html.contains("synthetic-creature"));
        assert!(!html.contains("correct_choice"));
        assert!(html.contains(&csrf));
    }
    for (body, origin, expected) in [
        (
            "choice=1".to_owned(),
            "https://portal.example.invalid",
            StatusCode::FORBIDDEN,
        ),
        (
            format!("choice=1&_csrf={csrf}"),
            "https://foreign.example.invalid",
            StatusCode::FORBIDDEN,
        ),
        (
            format!("choice=8&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("choice=1&choice=0&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("choice=bad&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("choice=1&account_id={}&_csrf={csrf}", Uuid::now_v7()),
            "https://portal.example.invalid",
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("choice=1&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::SEE_OTHER,
        ),
        (
            format!("choice=1&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::SEE_OTHER,
        ),
        (
            format!("choice=0&_csrf={csrf}"),
            "https://portal.example.invalid",
            StatusCode::CONFLICT,
        ),
    ] {
        let response = db
            .router()
            .oneshot(
                Request::post(format!("/en/puzzles/{id}/answer"))
                    .header(header::COOKIE, &cookie)
                    .header(header::ORIGIN, origin)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    let response = db
        .router()
        .oneshot(
            Request::get("/en/puzzles")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Correct answer."));
    assert!(!html.contains("<form"));
    db.close().await
}

#[tokio::test]
async fn disabled_router_never_connects_to_database() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let app = router(
        pool.clone(),
        WorldStore::new(pool, WorldOptions::default()),
        Flags::default(),
        CsrfConfig::default(),
    );
    for path in ["/pl/puzzles", "/en/puzzles/invalid/image"] {
        assert_eq!(
            app.clone()
                .oneshot(Request::get(path).body(Body::empty())?)
                .await?
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    Ok(())
}

#[tokio::test]
async fn postgres_answer_rechecks_locked_account() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.publish(Kind::Quiz).await?;
    let mut suspension = db.pool.begin().await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(db.account)
        .execute(&mut *suspension)
        .await?;
    {
        let answer = db.service.answer(db.account, id, 0);
        tokio::pin!(answer);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    result = &mut answer => panic!("answer passed suspension: {result:?}"),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
                }
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='accounts'::regclass))")
                    .fetch_one(&db.pool).await?;
                if waiting { break; }
            }
            Ok::<(), sqlx::Error>(())
        }).await??;
        suspension.commit().await?;
        assert!(matches!(answer.await, Err(Error::Forbidden)));
    }
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM puzzles_answers WHERE account_id=$1),(SELECT count(*) FROM puzzles_streaks WHERE account_id=$1)").bind(db.account).fetch_one(&db.pool).await?;
    assert_eq!(counts, (0, 0));
    db.close().await?;
    eprintln!(
        "postgres_answer_rechecks_locked_account duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_answer_holds_account_lock_while_waiting_for_streak() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.publish(Kind::Quiz).await?;
    sqlx::query("INSERT INTO puzzles_streaks(account_id) VALUES($1)")
        .bind(db.account)
        .execute(&db.pool)
        .await?;
    let mut streak = db.pool.begin().await?;
    sqlx::query("SELECT account_id FROM puzzles_streaks WHERE account_id=$1 FOR UPDATE")
        .bind(db.account)
        .fetch_one(&mut *streak)
        .await?;
    {
        let answer = db.service.answer(db.account, id, 0);
        tokio::pin!(answer);
        // Poll the answer until PostgreSQL reports its domain-row lock wait.
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    result = &mut answer => panic!("answer did not wait: {result:?}"),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {}
                }
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND a.query LIKE 'SELECT account_id FROM puzzles_streaks%' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='puzzles_streaks'::regclass))")
                    .fetch_one(&db.pool).await?;
                if waiting { break; }
            }
            Ok::<(), sqlx::Error>(())
        }).await??;
        let suspension = sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(db.account)
            .execute(&db.pool);
        tokio::pin!(suspension);
        tokio::select! {
            result = &mut suspension => panic!("suspension bypassed account lock: {result:?}"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
        // Account-before-domain ordering serializes suspension after the answer.
        streak.commit().await?;
        answer.await?;
        suspension.await?;
    }
    assert!(matches!(
        db.service.answer(db.account, id, 0).await,
        Err(Error::Forbidden)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers WHERE account_id=$1")
        .bind(db.account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await?;
    eprintln!(
        "postgres_answer_holds_account_lock_while_waiting_for_streak duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_controlled_midnight_changes_available_puzzles() -> TestResult {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let old = db.publish(Kind::Monster).await?;
    let tomorrow = db.today.next_day().ok_or("date overflow")?;
    let next = db.seed(tomorrow, Kind::Quiz).await?;
    assert_eq!(db.service.today(db.account).await?[0].id, old);
    test_clock::set(&db.pool, tomorrow.midnight().assume_utc()).await?;
    assert_eq!(db.service.today(db.account).await?[0].id, next);
    assert!(matches!(
        db.service.answer(db.account, old, 1).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db.service.image(db.account, old).await,
        Err(Error::NotFound)
    ));
    assert!(db.service.answer(db.account, next, 1).await?.correct);
    db.close().await
}

#[tokio::test]
async fn postgres_answer_rechecks_source_after_account_wait() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.publish(Kind::Quiz).await?;
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(db.account)
        .execute(&mut *blocker)
        .await?;
    let service = db.service.clone();
    let account = db.account;
    let answer = tokio::spawn(async move { service.answer(account, id, 1).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='accounts'::regclass))").fetch_one(&db.pool).await?;
            if waiting { break; }
            tokio::task::yield_now().await;
        }
        Ok::<_, sqlx::Error>(())
    }).await??;
    db.world.deny("synthetic-creature", "wolf").await?;
    blocker.commit().await?;
    assert!(matches!(answer.await?, Err(Error::Unavailable)));
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM puzzles_answers),(SELECT count(*) FROM puzzles_streaks)",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(counts, (0, 0));
    db.close().await?;
    eprintln!(
        "postgres_answer_rechecks_source_after_account_wait duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_browser_lock_waits_release_saturated_pool() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.publish(Kind::Quiz).await?;
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(db.account)
        .execute(&mut *blocker)
        .await?;
    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let service = db.service.clone();
        let account = db.account;
        requests.spawn(async move { service.answer(account, id, 1).await });
    }
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        while let Some(result) = requests.join_next().await {
            assert!(result?.is_err());
        }
        Ok::<_, tokio::task::JoinError>(())
    })
    .await??;
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_millis(250),
            sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&db.pool)
        )
        .await??,
        1
    );
    blocker.rollback().await?;
    db.close().await?;
    eprintln!(
        "postgres_browser_lock_waits_release_saturated_pool duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_account_lock_wait_sheds_as_unavailable() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.publish(Kind::Quiz).await?;
    let (cookie, csrf) = db.login().await?;
    let post = || -> Result<Request<Body>, axum::http::Error> {
        Request::post(format!("/en/puzzles/{id}/answer"))
            .header(header::COOKIE, &cookie)
            .header(header::ORIGIN, "https://portal.example.invalid")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("choice=1&_csrf={csrf}")))
    };
    // Hold the account row longer than the 500 ms browser lock bound (W3-04).
    let mut holder = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(db.account)
        .execute(&mut *holder)
        .await?;
    match db.service.answer(db.account, id, 1).await {
        Err(Error::Database(sqlx::Error::Database(error)))
            if error.code().as_deref() == Some("55P03") => {}
        other => return Err(format!("expected lock timeout, got {other:?}").into()),
    }
    let shed = db.router().oneshot(post()?).await?;
    assert_eq!(shed.status(), StatusCode::SERVICE_UNAVAILABLE);
    holder.rollback().await?;
    let answered = db.router().oneshot(post()?).await?;
    assert_eq!(answered.status(), StatusCode::SEE_OTHER);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await
}

#[tokio::test]
async fn postgres_answer_retains_visibility_fence_through_commit() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.publish(Kind::Quiz).await?;
    sqlx::query("INSERT INTO puzzles_streaks(account_id) VALUES($1)")
        .bind(db.account)
        .execute(&db.pool)
        .await?;
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT account_id FROM puzzles_streaks WHERE account_id=$1 FOR UPDATE")
        .bind(db.account)
        .execute(&mut *blocker)
        .await?;
    let service = db.service.clone();
    let account = db.account;
    let answer = tokio::spawn(async move { service.answer(account, id, 1).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND a.query LIKE 'SELECT account_id FROM puzzles_streaks%' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='puzzles_streaks'::regclass))").fetch_one(&db.pool).await?;
            if waiting { return Ok::<_,sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    let world = db.world.clone();
    let hide = tokio::spawn(async move { world.deny("synthetic-creature", "wolf").await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.wait_event_type='Lock' AND a.query LIKE 'INSERT INTO world_visibility%' AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.relation='world_visibility'::regclass))").fetch_one(&db.pool).await?;
            if waiting { return Ok::<_,sqlx::Error>(()); }
            assert!(!hide.is_finished(), "visibility changed before answer committed");
            tokio::task::yield_now().await;
        }
    }).await??;
    blocker.commit().await?;
    assert!(answer.await??.correct);
    hide.await??;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    assert!(matches!(
        db.service.answer(db.account, id, 1).await,
        Err(Error::Unavailable)
    ));
    db.close().await?;
    eprintln!(
        "postgres_answer_retains_visibility_fence_through_commit duration={:?}",
        started.elapsed()
    );
    Ok(())
}
