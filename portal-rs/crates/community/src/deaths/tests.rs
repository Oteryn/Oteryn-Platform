#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_internal_api::guard::{Binding, Identity, VerifiedClient};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const SUBJECT: &str = "01934f10-7c04-7001-805b-3b1122334401";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const AUTHORITY: &str = "oteryn:game:primary";
const BEREN: &str = "01934f10-7c04-7001-805b-3b1122334402";
const MIRA: &str = "01934f10-7c04-7001-805b-3b1122334403";
const DRAGON: &str = "oteryn:monster.dragon";
// SOURCE: Oteryn-Game 3732e96aa2874204dcc97504f13681cf04a65c6e, copied byte for byte
// (docs/contracts/fixtures/public-game-families-v2/README.md, contract revision 4).
const GAME_PLAYER: &[u8] = include_bytes!(
    "../../../../../docs/contracts/fixtures/public-game-families-v2/character_deaths_player_v2.json"
);
const GAME_CREATURE: &[u8] = include_bytes!(
    "../../../../../docs/contracts/fixtures/public-game-families-v2/character_deaths_creature_v2.json"
);
// Revision 4 golden line; the same bytes as the Game player fixture.
const GOLDEN: &[u8] = include_bytes!(
    "../../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-deaths.json"
);
fn player(character_id: &str, name: &str) -> wire::Killer {
    wire::Killer::Player {
        character_id: character_id.into(),
        name: name.into(),
    }
}
fn creature(creature_key: &str, name: &str) -> wire::Killer {
    wire::Killer::Creature {
        creature_key: creature_key.into(),
        name: name.into(),
    }
}
// SOURCE: docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md,
// revision 4 golden deaths payloads (Game fixtures), with a creature and a second
// player added. Dynamic times exercise serve-time bounds.
fn snapshot() -> wire::Snapshot {
    wire::Snapshot {
        contract_version: 2,
        operation: "PublishCharacterDeathsV2".into(),
        source_authority: AUTHORITY.into(),
        character_id: SUBJECT.into(),
        projection_epoch: "1".into(),
        projection_revision: "1".into(),
        source_observed_at: now().to_string(),
        deaths: vec![wire::Death {
            death_occurrence_id: "01934f10-7c05-7001-805b-3b1122334401".into(),
            occurred_at: (now() - 10).to_string(),
            level: "413".into(),
            pvp: true,
            killers: vec![
                player(BEREN, "Beren"),
                creature(DRAGON, "Dragon"),
                player(MIRA, "Mira"),
            ],
        }],
    }
}
fn watermark() -> wire::Watermark {
    wire::Watermark {
        contract_version: 2,
        operation: "PublishPublicProjectionWatermarkV2".into(),
        source_authority: AUTHORITY.into(),
        family: V2Family::Deaths,
        projection_epoch: "1".into(),
        complete_through: now().to_string(),
        observed_at: now().to_string(),
    }
}
fn app(db: PgPool, enabled: bool) -> Router {
    ingest_router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san("deaths-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        IngestConfig {
            requests_per_minute: 6000,
            enabled,
            source_authority: AUTHORITY.into(),
        },
    )
}
async fn post(app: Router, path: &str, body: Vec<u8>, identity: &str) -> Response {
    app.oneshot(
        Request::post(format!("/internal/v2/game-public/{path}"))
            .header("content-type", "application/json")
            .extension(VerifiedClient::from_verified_tls13_http11(
                Identity::san(identity).unwrap(),
            ))
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
}
async fn get(app: Router, path: &str) -> Response {
    app.oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
async fn text(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
#[test]
fn wire_success_and_bad_input() {
    let good = snapshot();
    let body = serde_json::to_vec(&good).unwrap();
    let value = canonical::decode(&body, 32768, 6).unwrap();
    assert!(
        decode::<wire::Snapshot>(&value, &body)
            .unwrap()
            .validate(now())
            .is_ok()
    );
    assert!(watermark().validate(now()).is_ok());
    for field in [
        "contract_version",
        "operation",
        "character_id",
        "projection_epoch",
        "projection_revision",
        "source_observed_at",
    ] {
        let mut v = serde_json::to_value(&good).unwrap();
        v[field] = serde_json::json!(null);
        assert!(serde_json::from_value::<wire::Snapshot>(v).is_err());
    }
    for bad in ["0", "01", "-1", "18446744073709551616", "1.0"] {
        let mut s = snapshot();
        s.projection_epoch = bad.into();
        assert!(s.validate(now()).is_err());
    }
    for bad in ["0", "4294967296", "01"] {
        let mut s = snapshot();
        s.deaths[0].level = bad.into();
        assert!(s.validate(now()).is_err());
    }
    for bad in ["bad/id", "..", "bad\nname", "a\u{200b}b", "e\u{301}"] {
        let mut s = snapshot();
        for killer in [player(BEREN, bad), creature(DRAGON, bad)] {
            s.deaths[0].killers = vec![killer];
            assert!(s.validate(now()).is_err());
        }
    }
    for bad in [
        player("01934f10-7c04-4001-805b-3b1122334402", "Beren"),
        player("01934F10-7C04-7001-805B-3B1122334402", "Beren"),
        player("Beren", "Beren"),
        creature("player", "Dragon"),
        creature("unknown", "Dragon"),
        creature("", "Dragon"),
        creature("oteryn:Monster.dragon", "Dragon"),
        creature(&"a".repeat(129), "Dragon"),
    ] {
        let mut s = snapshot();
        s.deaths[0].killers = vec![bad];
        assert!(s.validate(now()).is_err());
    }
    let mut s = snapshot();
    s.deaths[0].occurred_at = (now() + 2).to_string();
    assert!(s.validate(now()).is_err());
    let mut s = snapshot();
    s.deaths = vec![s.deaths[0].clone(); 21];
    assert!(s.validate(now()).is_err());
    let mut s = snapshot();
    s.deaths[0].killers = vec![creature(DRAGON, "Dragon"); 16];
    assert!(s.validate(now()).is_ok());
    s.deaths[0].killers.push(creature(DRAGON, "Dragon"));
    assert!(s.validate(now()).is_err());
    let mut s = snapshot();
    let mut second = s.deaths[0].clone();
    second.death_occurrence_id = "01934f10-7c05-7001-805b-3b1122334400".into();
    s.deaths.push(second);
    assert!(s.validate(now()).is_ok());
    s.deaths.swap(0, 1);
    assert!(s.validate(now()).is_err());
    for family in ["character_activity", "character_equipment", "unknown"] {
        assert!(serde_json::from_str::<V2Family>(&format!("\"{family}\"")).is_err());
    }
    let mut w = watermark();
    w.family = V2Family::Guild;
    assert!(w.validate(now()).is_err());
    let mut w = watermark();
    w.complete_through = (now() + 2).to_string();
    assert!(w.validate(now()).is_err());
    let bad = String::from_utf8(body).unwrap().replace(
        "\"contract_version\":2,\"operation\":\"PublishCharacterDeathsV2\"",
        "\"operation\":\"PublishCharacterDeathsV2\",\"contract_version\":2",
    );
    let v = canonical::decode(bad.as_bytes(), 32768, 6).unwrap();
    assert!(decode::<wire::Snapshot>(&v, bad.as_bytes()).is_err());
}
#[tokio::test]
async fn transport_refusal_and_bad_input() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let good = serde_json::to_vec(&snapshot())?;
    assert_eq!(
        post(
            app(db.clone(), false),
            "character-deaths",
            good.clone(),
            "deaths-test"
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post(app(db.clone(), true), "character-deaths", good, "other")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for bad in [
        b"{}".to_vec(),
        vec![b'x'; 32769],
        b"{\"contract_version\":2,\"contract_version\":2}".to_vec(),
    ] {
        let r = post(
            app(db.clone(), true),
            "character-deaths",
            bad,
            "deaths-test",
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert!(text(r).await.is_empty());
    }
    let r = get(
        router(db.clone(), Arc::new(FailClosed), Config::default()),
        "/en/deaths",
    )
    .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(r.headers()["cache-control"], "private, no-store");
    let r = get(
        router(
            db,
            Arc::new(FailClosed),
            Config {
                enabled: true,
                source_authority: AUTHORITY.into(),
            },
        ),
        "/en/deaths?unexpected=true",
    )
    .await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
struct Database {
    admin: PgPool,
    db: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping deaths PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("deaths_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let options = admin
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", schema.as_str())]);
        let db = PgPool::connect_with(options).await?;
        for migration in [
            include_str!("../../../../migrations/20261105180000_t24_public_snapshots.sql"),
            include_str!(
                "../../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"
            ),
            include_str!("../../../../migrations/20261105190000_t25_world_registry.sql"),
            include_str!("../../../../migrations/20261105231101_t24d_deaths.sql"),
            include_str!("../../../../migrations/20261105231103_t24f_houses.sql"),
            include_str!("../../../../migrations/20261115110006_fix_06_projection_read_bounds.sql"),
            include_str!("../../../../migrations/20261115110016_fix_w2a_ingest_budget.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&db).await?;
        }
        sqlx::raw_sql("CREATE TABLE community_privacy_revision(id integer PRIMARY KEY,revision bigint NOT NULL DEFAULT 0); INSERT INTO community_privacy_revision(id) VALUES(1)").execute(&db).await?;
        Ok(Some(Self { admin, db, schema }))
    }
    async fn close(self) -> TestResult {
        self.db.close().await;
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
#[tokio::test]
async fn postgres_replay_conflict_suppression_and_epoch_fence() -> TestResult {
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    let mut s = snapshot();
    let r = post(
        app(d.db.clone(), true),
        "character-deaths",
        serde_json::to_vec(&s)?,
        "deaths-test",
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        text(r).await,
        "{\"contract_version\":2,\"result\":\"accepted\"}"
    );
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "accepted");
    s.deaths[0].level = "414".into();
    assert!(matches!(
        storage::snapshot(&d.db, &s).await,
        Err(storage::Error::Conflict)
    ));
    s.projection_revision = "2".into();
    assert!(matches!(
        storage::snapshot(&d.db, &s).await,
        Err(storage::Error::Conflict)
    ));
    s.deaths.clear();
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "accepted");
    let count: i64 =
        sqlx::query_scalar("SELECT jsonb_array_length(deaths)::bigint FROM public_deaths_v2")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(count, 0);
    s.projection_revision = "1".into();
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "superseded");
    let mut w = watermark();
    assert!(matches!(
        storage::watermark(&d.db, &w).await,
        Err(storage::Error::Conflict)
    ));
    // Only explicit authoritative reconciliation in this isolated fixture
    // releases the fence before testing independent epoch ordering.
    sqlx::query("DELETE FROM public_projection_conflicts WHERE family='character_deaths'")
        .execute(&d.db)
        .await?;
    assert_eq!(storage::watermark(&d.db, &w).await.unwrap(), "accepted");
    assert_eq!(storage::watermark(&d.db, &w).await.unwrap(), "superseded");
    w.projection_epoch = "2".into();
    assert_eq!(storage::watermark(&d.db, &w).await.unwrap(), "accepted");
    assert_eq!(
        storage::snapshot(&d.db, &snapshot()).await.unwrap(),
        "superseded"
    );
    let epoch: String =
        sqlx::query_scalar("SELECT highest_epoch::text FROM public_projection_state")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(epoch, "2");
    sqlx::query("UPDATE community_ingest_budgets SET admitted_at=array_fill(extract(epoch FROM clock_timestamp())::double precision,ARRAY[6000]) WHERE route='v2/character-deaths'")
        .execute(&d.db)
        .await?;
    let r = post(
        app(d.db.clone(), true),
        "character-deaths",
        serde_json::to_vec(&snapshot())?,
        "deaths-test",
    )
    .await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.headers()["retry-after"], "60");
    assert!(text(r).await.is_empty());
    d.close().await
}
struct Policy {
    visible: bool,
    killers: &'static [&'static str],
}
impl Privacy for Policy {
    fn ready(&self) -> PrivacyFuture<'_> {
        Box::pin(async { Ok(true) })
    }
    fn allows_deaths<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async { Ok(self.visible) })
    }
    fn allows_killer<'a>(&'a self, id: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async move { Ok(self.killers.contains(&id)) })
    }
}
async fn seed(db: &PgPool) -> TestResult {
    storage::snapshot(db, &snapshot()).await.unwrap();
    storage::watermark(db, &watermark()).await.unwrap();
    sqlx::query("INSERT INTO world_status (id,name) VALUES ($1::uuid,'Example')")
        .bind(WORLD)
        .execute(db)
        .await?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: Identity::san("deaths-test")?,
        group: "game-public".into(),
    }])?);
    let profiles = crate::ingest::router(
        db.clone(),
        identities,
        crate::ingest::Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let observed = now();
    let profile = format!(
        r#"{{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"{AUTHORITY}","character_id":"{SUBJECT}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{observed}","profile":[{{"world_id":"{WORLD}","name":"Aldric","vocation":4,"level":"413","guild":[],"guild_visible":false}}]}}"#
    );
    let watermark = format!(
        r#"{{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"{AUTHORITY}","family":"character_profile","projection_epoch":"1","complete_through":"{observed}","observed_at":"{observed}"}}"#
    );
    for (path, body) in [("character-profile", profile), ("watermark", watermark)] {
        let response = profiles
            .clone()
            .oneshot(
                Request::post(format!("/internal/v1/game-public/{path}"))
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                        "deaths-test",
                    )?))
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }
    Ok(())
}
fn pages(db: PgPool, visible: bool) -> Router {
    pages_with(db, visible, &[])
}
fn pages_with(db: PgPool, visible: bool, killers: &'static [&'static str]) -> Router {
    router(
        db,
        Arc::new(Policy { visible, killers }),
        Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    )
}
#[tokio::test]
async fn death_filters_refuse_invalid_windows_and_render_controls_during_outage() -> TestResult {
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@localhost:1/unused")?;
    let app = web::router_with_qualification(
        db,
        Arc::new(FailClosed),
        Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
        false,
    );
    let response = get(app.clone(), "/en/deaths?start=2099-01-01&end=2099-01-01").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = get(app, "/en/deaths").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = text(response).await;
    assert!(body.contains("name=\"start\""));
    assert!(body.contains("Death records are currently unavailable."));
    Ok(())
}

#[tokio::test]
async fn postgres_public_pages_privacy_freshness_names_and_window() -> TestResult {
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    seed(&d.db).await?;
    for path in [
        "/en/deaths?world=Example",
        "/pl/deaths",
        "/en/characters/Aldric/deaths",
    ] {
        let r = get(pages(d.db.clone(), true), path).await;
        assert_eq!(r.status(), StatusCode::OK);
        let body = text(r).await;
        if let Ok(dir) = std::env::var("PORTAL_DEATHS_TEST_RENDER_DIR") {
            std::fs::create_dir_all(&dir)?;
            std::fs::write(
                std::path::Path::new(&dir).join(if path.starts_with("/pl/") {
                    "pl.html"
                } else {
                    "en.html"
                }),
                &body,
            )?;
        }
        assert!(body.contains("Aldric"));
        assert!(body.contains(r#"<article class="op-card op-stack">"#));
        assert!(!body.contains("Beren"));
        assert!(!body.contains("Mira"));
        assert!(!body.contains(SUBJECT));
        assert!(!body.contains(WORLD));
        assert!(!body.contains("01934f10-7c05"));
    }
    sqlx::query(
        "UPDATE public_deaths_v2 SET deaths=jsonb_set(deaths,'{0,occurred_at}',to_jsonb($1::text))",
    )
    .bind((now() - 30 * 86400 - 1).to_string())
    .execute(&d.db)
    .await?;
    let r = get(pages(d.db.clone(), true), "/en/deaths").await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = text(r).await;
    assert!(!body.contains(r#"<article class="op-card op-stack">"#));
    assert!(!body.contains("Beren"));
    sqlx::query("UPDATE public_deaths_v2 SET deaths=$1")
        .bind(serde_json::to_value(snapshot().deaths)?)
        .execute(&d.db)
        .await?;
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/characters/Unknown/deaths")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let r = get(pages(d.db.clone(), false), "/en/deaths").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert!(!text(r).await.contains("Aldric"));
    assert_eq!(
        get(pages(d.db.clone(), false), "/en/characters/Aldric/deaths")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let r = get(
        router(
            d.db.clone(),
            Arc::new(FailClosed),
            Config {
                enabled: true,
                source_authority: AUTHORITY.into(),
            },
        ),
        "/en/deaths",
    )
    .await;
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!text(r).await.contains("Aldric"));
    sqlx::query("UPDATE public_deaths_v2_watermark SET complete_through=$1")
        .bind(now() as i64 - 30)
        .execute(&d.db)
        .await?;
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/deaths").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE public_deaths_v2_watermark SET complete_through=$1")
        .bind(now() as i64)
        .execute(&d.db)
        .await?;
    let mut s = snapshot();
    s.projection_revision = "2".into();
    s.deaths.clear();
    storage::snapshot(&d.db, &s).await.unwrap();
    let r = get(pages(d.db.clone(), true), "/en/characters/Aldric/deaths").await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = text(r).await;
    assert!(body.contains("No visible recorded deaths"));
    assert!(!body.contains("Beren"));
    sqlx::query("INSERT INTO public_snapshots (family,subject,epoch,version,watermark,payload) SELECT family,'01934f10-7c04-7001-805b-3b1122334402',epoch,version,watermark,jsonb_set(payload,'{character_id}','\"01934f10-7c04-7001-805b-3b1122334402\"') FROM public_snapshots WHERE subject=$1").bind(SUBJECT).execute(&d.db).await?;
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/characters/Aldric/deaths")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/deaths").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{profile}','[]') WHERE NOT is_watermark").execute(&d.db).await?;
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/characters/Aldric/deaths")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    d.close().await
}
#[tokio::test]
async fn postgres_retention_and_immutable_replay_after_purge() -> TestResult {
    use oteryn_db::outbox::Cleanup as _;
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    let mut s = snapshot();
    s.deaths[0].occurred_at = (now() - 30 * 86400 - 1).to_string();
    storage::snapshot(&d.db, &s).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT jsonb_array_length(deaths)::bigint FROM public_deaths_v2")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(count, 0);
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "accepted");
    sqlx::query("UPDATE public_deaths_v2 SET deaths=$1")
        .bind(serde_json::to_value(&s.deaths)?)
        .execute(&d.db)
        .await?;
    Cleanup { pool: d.db.clone() }.clean().await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT jsonb_array_length(deaths)::bigint FROM public_deaths_v2")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(count, 0);
    s.projection_revision = "2".into();
    s.deaths[0].level = "414".into();
    assert!(matches!(
        storage::snapshot(&d.db, &s).await,
        Err(storage::Error::Conflict)
    ));
    d.close().await
}
#[tokio::test]
async fn postgres_suppression_waits_for_inflight_profile_read() -> TestResult {
    struct Blocking {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    impl Privacy for Blocking {
        fn ready(&self) -> PrivacyFuture<'_> {
            Box::pin(async { Ok(true) })
        }
        fn allows_deaths<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
            Box::pin(async {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(true)
            })
        }
    }
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    seed(&d.db).await?;
    let privacy = Arc::new(Blocking {
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    let app = router(
        d.db.clone(),
        privacy.clone(),
        Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let read = tokio::spawn(get(app, "/en/deaths"));
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        privacy.entered.notified(),
    )
    .await?;
    let db = d.db.clone();
    let mut suppression = snapshot();
    suppression.projection_revision = "2".into();
    suppression.deaths.clear();
    let mut write = tokio::spawn(async move { storage::snapshot(&db, &suppression).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut write)
            .await
            .is_err()
    );
    privacy.release.notify_one();
    let response = read.await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = text(response).await;
    assert!(body.contains("Aldric"));
    assert!(body.contains(r#"<article class="op-card op-stack">"#));
    assert!(!body.contains("Beren"));
    assert_eq!(write.await?.unwrap(), "accepted");
    let response = get(pages(d.db.clone(), true), "/en/deaths").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = text(response).await;
    assert!(!body.contains(r#"<article class="op-card op-stack">"#));
    assert!(!body.contains("Beren"));
    d.close().await
}

#[tokio::test]
async fn audit06_deaths_replay_excludes_observation_time() -> TestResult {
    let d = Database::open().await?.ok_or("PostgreSQL required")?;
    let a = app(d.db.clone(), true);
    let mut s = snapshot();
    let response = post(
        a.clone(),
        "character-deaths",
        serde_json::to_vec(&s)?,
        "deaths-test",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let before: (Vec<u8>, serde_json::Value, Option<i64>) =
        sqlx::query_as("SELECT digest,deaths,observed FROM public_deaths_v2")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(before.2, Some(s.source_observed_at.parse::<i64>()?));
    s.source_observed_at = (s.source_observed_at.parse::<u64>()? - 1).to_string();
    let response = post(
        a.clone(),
        "character-deaths",
        serde_json::to_vec(&s)?,
        "deaths-test",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let after: (Vec<u8>, serde_json::Value, Option<i64>) =
        sqlx::query_as("SELECT digest,deaths,observed FROM public_deaths_v2")
            .fetch_one(&d.db)
            .await?;
    assert_eq!(before, after);
    s.deaths[0].level = "999".into();
    assert_eq!(
        post(
            a,
            "character-deaths",
            serde_json::to_vec(&s)?,
            "deaths-test"
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    d.close().await
}

#[tokio::test]
async fn audit06_deaths_two_connection_pool_with_nested_privacy() -> TestResult {
    struct Resolving {
        db: PgPool,
        barrier: tokio::sync::Barrier,
    }
    impl Privacy for Resolving {
        fn ready(&self) -> PrivacyFuture<'_> {
            Box::pin(async { Ok(true) })
        }
        fn allows_deaths<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
            Box::pin(async {
                self.barrier.wait().await;
                sqlx::query("SELECT pg_sleep(0.1)")
                    .execute(&self.db)
                    .await
                    .map_err(|_| Unavailable)?;
                Ok(true)
            })
        }
    }
    let d = Database::open().await?.ok_or("PostgreSQL required")?;
    seed(&d.db).await?;
    let small = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_with(d.db.connect_options().as_ref().clone())
        .await?;
    let a = router(
        small.clone(),
        Arc::new(Resolving {
            db: small.clone(),
            barrier: tokio::sync::Barrier::new(2),
        }),
        Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(get(a.clone(), "/en/deaths"), get(a, "/pl/deaths"))
    })
    .await?;
    assert_eq!(a.status(), StatusCode::OK);
    assert_eq!(b.status(), StatusCode::OK);
    small.close().await;
    d.close().await
}

#[tokio::test]
async fn audit_w2_c06_deaths_detail_ignores_unrelated_profiles() -> TestResult {
    let started = std::time::Instant::now();
    let d = Database::open().await?.ok_or("PostgreSQL required")?;
    seed(&d.db).await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) SELECT 'character_profile',n::text,false,1,1,0,jsonb_build_object('profile',jsonb_build_array(jsonb_build_object('name','Unrelated'||n))) FROM generate_series(1,10001) n").execute(&d.db).await?;
    let response = get(pages(d.db.clone(), true), "/en/characters/Aldric/deaths").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(text(response).await.contains("413"));
    d.close().await?;
    eprintln!(
        "audit_w2_c06_deaths_detail_ignores_unrelated_profiles {:?}",
        started.elapsed()
    );
    Ok(())
}
#[test]
fn game_killer_fixtures_decode_and_validate() {
    for body in [GAME_PLAYER, GAME_CREATURE, GOLDEN] {
        let value = canonical::decode(body, 32768, 6).unwrap();
        let decoded = decode::<wire::Snapshot>(&value, body).unwrap();
        assert!(decoded.validate(now()).is_ok());
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), body);
    }
}
#[tokio::test]
async fn bad_killers_are_refused_with_400() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let fixture = std::str::from_utf8(GAME_PLAYER)?;
    let object = format!(r#"{{"kind":"player","character_id":"{BEREN}","name":"Beren"}}"#);
    assert!(fixture.contains(&object));
    let swap = |to: &str| fixture.replace(&object, to);
    let mut bad = vec![
        swap(&object.replace(r#""kind":"player""#, r#""kind":"npc""#)),
        swap(&object.replace(r#""kind":"player""#, r#""kind":"Player""#)),
        swap(&object.replace(r#""name":"Beren""#, r#""name":"Beren","extra":"x""#)),
        swap(&format!(
            r#"{{"kind":"player","name":"Beren","character_id":"{BEREN}"}}"#
        )),
        swap(&format!(
            r#"{{"character_id":"{BEREN}","kind":"player","name":"Beren"}}"#
        )),
        swap(&format!(r#"{{"character_id":"{BEREN}","name":"Beren"}}"#)),
        swap(&format!(
            r#"{{"kind":"creature","character_id":"{BEREN}","name":"Beren"}}"#
        )),
        swap(r#"{"kind":"creature","creature_key":"player","name":"Beren"}"#),
        swap(r#""Beren""#),
        swap(&vec![object.as_str(); 17].join(",")),
    ];
    bad.push(std::str::from_utf8(GAME_CREATURE)?.replace(
        r#""creature_key":"oteryn:monster.dragon""#,
        r#""creature_key":"unknown""#,
    ));
    for body in bad {
        let r = post(
            app(db.clone(), true),
            "character-deaths",
            body.clone().into_bytes(),
            "deaths-test",
        )
        .await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "{body}");
        assert!(text(r).await.is_empty());
    }
    Ok(())
}
#[tokio::test]
async fn postgres_game_killer_fixtures_are_accepted() -> TestResult {
    for body in [GAME_PLAYER, GAME_CREATURE] {
        let Some(d) = Database::open().await? else {
            return Ok(());
        };
        for _ in 0..2 {
            // The second post is an exact, idempotent replay.
            let r = post(
                app(d.db.clone(), true),
                "character-deaths",
                body.to_vec(),
                "deaths-test",
            )
            .await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(
                text(r).await,
                "{\"contract_version\":2,\"result\":\"accepted\"}"
            );
        }
        let fenced: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public_deaths_v2_occurrences WHERE subject=$1 AND occurrence='01934f10-7c08-7001-805b-3b1122334401'",
        )
        .bind(SUBJECT)
        .fetch_one(&d.db)
        .await?;
        assert_eq!(fenced, 1);
        d.close().await?;
    }
    // Fixture times are outside the 30-day window, so typed persistence is
    // checked with a current snapshot of the same shape.
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    assert_eq!(
        storage::snapshot(&d.db, &snapshot()).await.unwrap(),
        "accepted"
    );
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT deaths->0->'killers' FROM public_deaths_v2 WHERE subject=$1")
            .bind(SUBJECT)
            .fetch_one(&d.db)
            .await?;
    assert_eq!(
        stored,
        serde_json::json!([
            {"kind":"player","character_id":BEREN,"name":"Beren"},
            {"kind":"creature","creature_key":DRAGON,"name":"Dragon"},
            {"kind":"player","character_id":MIRA,"name":"Mira"}
        ])
    );
    d.close().await
}
#[tokio::test]
async fn postgres_killer_visibility_is_enforced_at_serving() -> TestResult {
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    seed(&d.db).await?;
    let date = time::OffsetDateTime::from_unix_timestamp(now() as i64 - 10)?.date();
    let dated_path = format!("/en/deaths?world=Example&start={date}&end={date}");
    for path in [
        dated_path.as_str(),
        "/pl/deaths",
        "/en/characters/Aldric/deaths",
    ] {
        // Beren is currently visible, Mira is hidden: Mira's whole object is dropped.
        let r = get(pages_with(d.db.clone(), true, &[BEREN]), path).await;
        assert_eq!(r.status(), StatusCode::OK);
        let body = text(r).await;
        assert!(body.contains("Beren, Dragon"), "{body}");
        assert!(!body.contains("Mira"));
        for internal in [BEREN, MIRA, DRAGON, SUBJECT, "character_id", "creature_key"] {
            assert!(!body.contains(internal), "{internal}");
        }
        // Without a current allow, no player killer is shown; creatures still are.
        let r = get(pages(d.db.clone(), true), path).await;
        assert_eq!(r.status(), StatusCode::OK);
        let body = text(r).await;
        assert!(body.contains("Dragon"));
        assert!(!body.contains("Beren"));
        assert!(!body.contains("Mira"));
    }
    assert_eq!(
        get(pages(d.db.clone(), true), "/en/deaths?world=Unknown")
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    // A hidden victim hides its deaths, killers included.
    let r = get(pages_with(d.db.clone(), false, &[BEREN]), "/en/deaths").await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = text(r).await;
    assert!(!body.contains("Beren") && !body.contains("Dragon"));
    // A retained revision 3 name-only row still serves, without any label.
    sqlx::query(
        "UPDATE public_deaths_v2 SET deaths=jsonb_set(deaths,'{0,killers}','[\"Beren\",\"Mira\"]')",
    )
    .execute(&d.db)
    .await?;
    let r = get(pages_with(d.db.clone(), true, &[BEREN, MIRA]), "/en/deaths").await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = text(r).await;
    assert!(body.contains("Aldric"));
    assert!(body.contains("No public killer recorded."));
    assert!(!body.contains("Beren") && !body.contains("Mira"));
    d.close().await
}
/// Revision 3 name-only serialization, in its struct member order.
#[derive(serde::Serialize)]
struct NameOnlyDeath<'a> {
    death_occurrence_id: &'a str,
    occurred_at: &'a str,
    level: &'a str,
    pvp: bool,
    killers: Vec<&'a str>,
}
#[tokio::test]
async fn postgres_name_only_rows_take_typed_identity_without_guessing() -> TestResult {
    use sha2::{Digest, Sha256};
    let Some(d) = Database::open().await? else {
        return Ok(());
    };
    let s = snapshot();
    let death = &s.deaths[0];
    let names = |names: Vec<&'static str>| NameOnlyDeath {
        death_occurrence_id: &death.death_occurrence_id,
        occurred_at: &death.occurred_at,
        level: &death.level,
        pvp: death.pvp,
        killers: names,
    };
    let legacy = names(vec!["Beren", "Dragon", "Mira"]);
    let mut identity = serde_json::to_value(&s)?;
    identity
        .as_object_mut()
        .unwrap()
        .remove("source_observed_at");
    identity["deaths"] = serde_json::to_value(vec![&legacy])?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&d.db)
        .await?;
    sqlx::query("INSERT INTO public_deaths_v2(subject,authority,epoch,revision,digest,deaths,observed) VALUES($1,$2,1,1,$3,$4,$5)")
        .bind(SUBJECT)
        .bind(AUTHORITY)
        .bind(Sha256::digest(serde_json::to_vec(&identity)?).to_vec())
        .bind(serde_json::to_value(vec![&legacy])?)
        .bind(i64::try_from(now())?)
        .execute(&d.db)
        .await?;
    sqlx::query(
        "INSERT INTO public_deaths_v2_occurrences(subject,occurrence,digest) VALUES($1,$2,$3)",
    )
    .bind(SUBJECT)
    .bind(&death.death_occurrence_id)
    .bind(Sha256::digest(serde_json::to_vec(&legacy)?).to_vec())
    .execute(&d.db)
    .await?;
    // The same revision, now typed by the producer, replaces the name-only body.
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "accepted");
    assert_eq!(storage::snapshot(&d.db, &s).await.unwrap(), "accepted");
    let (fence, killer): (Vec<u8>, Option<String>) = sqlx::query_as(
        "SELECT o.digest, d.deaths->0->'killers'->0->>'character_id' FROM public_deaths_v2 d JOIN public_deaths_v2_occurrences o USING (subject) WHERE d.subject=$1",
    )
    .bind(SUBJECT)
    .fetch_one(&d.db)
    .await?;
    assert_eq!(fence, Sha256::digest(serde_json::to_vec(death)?).to_vec());
    assert_eq!(killer.as_deref(), Some(BEREN));
    // A name-only fence whose names differ is a conflict, never a match.
    let other = "01934f10-7c04-7001-805b-3b1122334499";
    sqlx::query(
        "INSERT INTO public_deaths_v2_occurrences(subject,occurrence,digest) VALUES($1,$2,$3)",
    )
    .bind(other)
    .bind(&death.death_occurrence_id)
    .bind(
        Sha256::digest(serde_json::to_vec(&names(vec![
            "Beren", "Dragon", "Other",
        ]))?)
        .to_vec(),
    )
    .execute(&d.db)
    .await?;
    let mut t = snapshot();
    t.character_id = other.into();
    assert!(matches!(
        storage::snapshot(&d.db, &t).await,
        Err(storage::Error::Conflict)
    ));
    d.close().await
}
