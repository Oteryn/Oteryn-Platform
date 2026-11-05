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
// SOURCE: docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md,
// "Valid death with recorded final-blow killer". Dynamic times exercise serve-time bounds.
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
            killers: vec!["Beren".into(), "Mira".into()],
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
        s.deaths[0].killers = vec![bad.into()];
        assert!(s.validate(now()).is_err());
    }
    let mut s = snapshot();
    s.deaths[0].occurred_at = (now() + 2).to_string();
    assert!(s.validate(now()).is_err());
    let mut s = snapshot();
    s.deaths = vec![s.deaths[0].clone(); 21];
    assert!(s.validate(now()).is_err());
    let mut s = snapshot();
    s.deaths[0].killers = vec!["A".into(); 17];
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
        let schema = format!(
            "deaths_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
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
            include_str!("../../../../migrations/20261105190000_t25_world_registry.sql"),
            include_str!("../../../../migrations/20261105231101_t24d_deaths.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&db).await?;
        }
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
    sqlx::query("UPDATE public_deaths_v2_rate_limits SET requests=6000")
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
}
impl Privacy for Policy {
    fn ready(&self) -> PrivacyFuture<'_> {
        Box::pin(async { Ok(true) })
    }
    fn allows_deaths<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async { Ok(self.visible) })
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
    router(
        db,
        Arc::new(Policy { visible }),
        Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    )
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
        assert!(body.contains("Beren, Mira"));
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
    assert!(!text(r).await.contains("Beren"));
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
    assert!(text(response).await.contains("Beren"));
    assert_eq!(write.await?.unwrap(), "accepted");
    let response = get(pages(d.db.clone(), true), "/en/deaths").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!text(response).await.contains("Beren"));
    d.close().await
}
