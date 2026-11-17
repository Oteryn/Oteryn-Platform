#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
// SOURCE: G-4 contract guild vector, adapted with a hidden second member.
const GOLDEN: &str = r#"{"contract_version":2,"operation":"PublishGuildV2","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","rank_level":1,"joined_at":"1789000000"},{"character_id":"01934f10-7c04-7001-805b-3b1122334402","name":"Hidden Member","rank_level":3,"joined_at":"1789000000"}]}]}"#;
fn publication(raw: &str, watermark: bool) -> Publication {
    let value = canonical::decode(raw.as_bytes(), 393216, 6).unwrap();
    wire::parse(&value, raw.as_bytes(), watermark, now()).unwrap()
}
fn app(db: PgPool, enabled: bool) -> Router {
    ingest_router(
        db,
        Arc::new(
            IdentityMap::new(vec![guard::Binding {
                identity: guard::Identity::san("guild-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        IngestConfig {
            enabled,
            source_authority: "oteryn:game:primary".into(),
        },
    )
}
async fn send(app: Router, body: &str, identity: &str) -> Response {
    app.oneshot(
        Request::post("/internal/v2/game-public/guild")
            .header("content-type", "application/json")
            .extension(guard::VerifiedClient::from_verified_tls13_http11(
                guard::Identity::san(identity).unwrap(),
            ))
            .body(Body::from(body.to_owned()))
            .unwrap(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn refusal_and_bad_wire() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        send(app(db.clone(), false), GOLDEN, "guild-test")
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(app(db.clone(), true), GOLDEN, "wrong-test")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for raw in [
        "{}".to_owned(),
        format!("{GOLDEN}\n"),
        GOLDEN.replace("PublishGuildV2", "PublishGuildV1"),
        GOLDEN.replace("\"rank_level\":3", "\"rank_level\":4"),
        GOLDEN.replace("Hidden Member", "Aldric\\n"),
        GOLDEN.replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"01\""),
        GOLDEN.replace(
            "\"contract_version\":2",
            "\"contract_version\":2,\"extra\":0",
        ),
        "x".repeat(393217),
    ] {
        let response = send(app(db.clone(), true), &raw, "guild-test").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(to_bytes(response.into_body(), 1000).await?.is_empty());
    }
    Ok(())
}
#[test]
fn stable_order_and_bounds() {
    let p = publication(GOLDEN, false);
    assert_eq!(p.revision, "5");
    for raw in [
        GOLDEN
            .replace("\"rank_level\":3", "\"rank_level\":1")
            .replace("4402\"", "4400\""),
        GOLDEN.replace("4402\"", "4401\""),
        GOLDEN.replace("1790000000", "9999999999"),
        GOLDEN.replace("\"state\":\"ACTIVE\"", "\"state\":\"UNKNOWN\""),
    ] {
        let v = canonical::decode(raw.as_bytes(), 393216, 6).unwrap();
        assert!(wire::parse(&v, raw.as_bytes(), false, now()).is_err());
    }
}
struct Visible;
impl PublicVisibility for Visible {
    fn visible_batch<'a>(&'a self, _: &'a str, members: &'a [MemberRef]) -> VisibilityResult<'a> {
        Box::pin(async move { Ok(members.iter().map(|m| m.name != "Hidden Member").collect()) })
    }
}
async fn page_body(app: Router, path: &str) -> (StatusCode, String) {
    let r = app
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (
        r.status(),
        String::from_utf8(to_bytes(r.into_body(), 100000).await.unwrap().to_vec()).unwrap(),
    )
}
#[tokio::test]
async fn postgres_guild_ingest_privacy_replay_and_fence() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping guild PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let pool = PgPool::connect(&url).await?;
    let schema = "guild_t24e_test";
    sqlx::query("DROP SCHEMA IF EXISTS guild_t24e_test CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE SCHEMA guild_t24e_test")
        .execute(&pool)
        .await?;
    let options = pool
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", schema)]);
    let db = PgPool::connect_with(options).await?;
    sqlx::raw_sql("CREATE TABLE community_privacy_revision(id integer PRIMARY KEY,revision bigint NOT NULL DEFAULT 0); INSERT INTO community_privacy_revision(id) VALUES(1)").execute(&db).await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261115110016_fix_w2a_ingest_budget.sql"
    ))
    .execute(&db)
    .await?;
    for (id, name, rank) in [
        ("01934f10-7c04-7001-805b-3b1122334401", "Aldric", 1),
        ("01934f10-7c04-7001-805b-3b1122334402", "Hidden Member", 3),
    ] {
        let payload = serde_json::json!({"character_id":id,"profile":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":name,"vocation":4,"level":"412","guild":[{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":rank}],"guild_visible":true}]});
        sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES ('character_profile',$1,1,1,$2,$3)").bind(id).bind(i64::try_from(now())?).bind(payload).execute(&db).await?;
    }
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('character_profile','',true,1,1,$1,'null')").bind(i64::try_from(now())?).execute(&db).await?;
    let p = publication(GOLDEN, false);
    assert_eq!(
        send(app(db.clone(), true), GOLDEN, "guild-test")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(apply(&db, &p).await.unwrap(), "accepted");
    let mut equal = publication(&GOLDEN.replace("1790000000", "1790000001"), false);
    assert_eq!(apply(&db, &equal).await.unwrap(), "accepted");
    let watermark = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"guild","projection_epoch":"1","complete_through":"{}","observed_at":"{}"}}"#,
        now(),
        now()
    );
    apply(&db, &publication(&watermark, true)).await.unwrap();
    let pages = || {
        router(
            db.clone(),
            Arc::new(Visible),
            Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        )
    };
    let (status, body) = page_body(pages(), "/en/guilds").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Visible members: 1"));
    // Ingest holds the projection row FOR UPDATE; the page's read fence sheds
    // as unavailable within its budget instead of waiting for the ingest.
    let mut ingest = db.begin().await?;
    sqlx::query("SELECT 1 FROM public_projection_state WHERE id=1 FOR UPDATE")
        .execute(&mut *ingest)
        .await?;
    let started = std::time::Instant::now();
    let (status, _) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        page_body(pages(), "/en/guilds"),
    )
    // An elapsed timeout fails the test: the page waited on the held lock.
    .await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    ingest.rollback().await?;
    if let Ok(dir) = std::env::var("PORTAL_TEST_ARTIFACT_DIR") {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(std::path::Path::new(&dir).join("guilds.html"), &body)?;
    }
    assert!(!body.contains("Hidden Member"));
    assert!(!body.contains("01934f10"));
    assert!(
        page_body(pages(), "/pl/guilds")
            .await
            .1
            .contains("Widoczni członkowie: 1")
    );
    assert!(
        page_body(pages(), "/en/guilds?q=red")
            .await
            .1
            .contains("Red Rose")
    );
    let (status, body) = page_body(pages(), "/en/guilds/Red%20Rose").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Aldric"));
    assert!(!body.contains("Hidden Member"));
    equal.payload["guild"][0]["name"] = "Other".into();
    assert_eq!(apply(&db, &equal).await, Err(StatusCode::CONFLICT));
    assert_eq!(
        apply(&db, &publication(&watermark, true)).await,
        Err(StatusCode::CONFLICT)
    );
    let (_, quarantined) = page_body(pages(), "/en/guilds").await;
    assert!(!quarantined.contains("Red Rose") && !quarantined.contains("Aldric"));
    // Explicit authoritative reconciliation in this isolated fixture permits the
    // remaining privacy tests; ordinary publication never removes the fence.
    sqlx::query("DELETE FROM public_projection_conflicts WHERE family='guild'")
        .execute(&db)
        .await?;
    apply(&db, &publication(&watermark, true)).await.unwrap();
    sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{profile,0,guild_visible}','false') WHERE family='character_profile' AND subject='01934f10-7c04-7001-805b-3b1122334401'").execute(&db).await?;
    let (_, hidden) = page_body(pages(), "/en/guilds/Red%20Rose").await;
    assert!(!hidden.contains("Aldric"));
    sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{profile,0,guild_visible}','true') WHERE family='character_profile' AND subject='01934f10-7c04-7001-805b-3b1122334401'").execute(&db).await?;
    assert_eq!(
        page_body(pages(), "/en/guilds/Missing").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        page_body(pages(), "/en/guilds?page=0").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        page_body(pages(), "/en/guilds?unknown=1").await.0,
        StatusCode::BAD_REQUEST
    );
    let (_, body) = page_body(
        router(
            db.clone(),
            Arc::new(pages::FailClosed),
            Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        ),
        "/en/guilds",
    )
    .await;
    assert!(body.contains("unavailable"));
    assert!(!body.contains("Red Rose"));
    let mut suppression = publication(GOLDEN, false);
    suppression.revision = "6".into();
    suppression.payload["guild"] = serde_json::json!([]);
    apply(&db, &suppression).await.unwrap();
    assert_eq!(
        page_body(pages(), "/en/guilds/Red%20Rose").await.0,
        StatusCode::NOT_FOUND
    );
    let mut higher = publication(GOLDEN, false);
    higher.epoch = "2".into();
    apply(&db, &higher).await.unwrap();
    assert_eq!(apply(&db, &p).await.unwrap(), "superseded");
    assert!(
        page_body(pages(), "/en/guilds")
            .await
            .1
            .contains("unavailable")
    );
    let stale = watermark
        .replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\"")
        .replace(&now().to_string(), &now().saturating_sub(40).to_string());
    apply(&db, &publication(&stale, true)).await.unwrap();
    assert!(
        page_body(pages(), "/en/guilds")
            .await
            .1
            .contains("unavailable")
    );
    assert_eq!(
        page_body(pages(), "/en/guilds").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await;
    sqlx::query("DROP SCHEMA guild_t24e_test CASCADE")
        .execute(&pool)
        .await?;
    eprintln!("Guild PostgreSQL tests ran: 1");
    Ok(())
}

async fn audit06_guild_scenario(oversized: bool) -> TestResult {
    struct Visible {
        db: PgPool,
        suppress: bool,
    }
    impl PublicVisibility for Visible {
        fn visible_batch<'a>(&'a self, _: &'a str, refs: &'a [MemberRef]) -> VisibilityResult<'a> {
            Box::pin(async move {
                if self.suppress {
                    sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{guild}','[]'),version=version+1 WHERE family='guild' AND NOT is_watermark").execute(&self.db).await.map_err(|_| Unavailable::Database)?;
                }
                Ok(vec![true; refs.len()])
            })
        }
    }
    let d = crate::audit06_test_db::Database::new().await?;
    apply(&d.db, &publication(GOLDEN, false))
        .await
        .map_err(|_| "guild publication failed")?;
    let wm = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"guild","projection_epoch":"1","complete_through":"{}","observed_at":"{}"}}"#,
        now(),
        now()
    );
    apply(&d.db, &publication(&wm, true))
        .await
        .map_err(|_| "watermark failed")?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1,'null')").bind(i64::try_from(now())?).execute(&d.db).await?;
    if oversized {
        sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) SELECT 'guild','unrelated:'||i,1,1,$1,jsonb_build_object('guild',jsonb_build_array(jsonb_build_object('name','Unrelated'))) FROM generate_series(1,10000) i").bind(i64::try_from(now())?).execute(&d.db).await?;
    }
    for suppress in [false, true] {
        let a = router(
            d.db.clone(),
            Arc::new(Visible {
                db: d.db.clone(),
                suppress,
            }),
            Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        );
        let response = a
            .oneshot(Request::get("/en/guilds/Red%20Rose").body(Body::empty())?)
            .await?;
        let body = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        if suppress {
            assert!(body.contains("unavailable"));
            assert!(!body.contains("Red Rose</"));
        } else {
            assert!(body.contains("Red Rose</"));
        }
    }
    d.close().await
}

#[tokio::test]
async fn audit06_guild_detail_bounds_and_same_epoch_suppression() -> TestResult {
    audit06_guild_scenario(true).await
}
#[tokio::test]
async fn audit06_guild_same_epoch_suppression_after_privacy() -> TestResult {
    audit06_guild_scenario(false).await
}

#[tokio::test]
async fn configured_parent_switch_fences_children() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let get = |key: &str| {
        Some(
            match key {
                "APP_ENV" | "OTERYN_ENV" => "testing",
                "PORTAL_PUBLIC_PROJECTIONS_ENABLED" => "false",
                "PORTAL_GUILDS_INGEST_ENABLED"
                | "PORTAL_WORLD_DAILY_PUBLIC_ENABLED"
                | "PORTAL_WORLD_DAILY_INGEST_ENABLED" => "true",
                "PORTAL_PUBLIC_PROJECTIONS_SAN" => "guild-test",
                "PORTAL_PUBLIC_PROJECTIONS_AUTHORITY" => "oteryn:game:primary",
                "PORTAL_WORLD_DAILY_WORLDS" => "01934f10-7c02-7001-805b-3b1122334401",
                _ => return None,
            }
            .to_owned(),
        )
    };
    let app = configured_with(db.clone(), Part::All, get);
    assert_eq!(
        send(app, GOLDEN, "guild-test").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let daily = crate::today::Config::from_lookup(get);
    assert!(!daily.ingest_enabled);
    assert!(!daily.public_enabled);
    Ok(())
}

#[tokio::test]
async fn postgres_configured_guild_parent_off_refuses_publication_and_watermark() -> TestResult {
    let started = std::time::Instant::now();
    let d = crate::audit06_test_db::Database::new().await?;
    let build = |parent: bool| {
        configured_with(d.db.clone(), Part::All, |key| {
            Some(
                match key {
                    "APP_ENV" | "OTERYN_ENV" => "testing",
                    "PORTAL_GUILDS_INGEST_ENABLED" => "true",
                    "PORTAL_PUBLIC_PROJECTIONS_ENABLED" => {
                        if parent {
                            "true"
                        } else {
                            "false"
                        }
                    }
                    "PORTAL_PUBLIC_PROJECTIONS_SAN" => "guild-test",
                    "PORTAL_PUBLIC_PROJECTIONS_AUTHORITY" => "oteryn:game:primary",
                    _ => return None,
                }
                .to_owned(),
            )
        })
    };
    assert_eq!(
        send(build(true), GOLDEN, "guild-test").await.status(),
        StatusCode::OK
    );
    assert_eq!(
        send(build(false), GOLDEN, "guild-test").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let wm = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"guild","projection_epoch":"1","complete_through":"{}","observed_at":"{}"}}"#,
        now(),
        now()
    );
    let request = Request::post(crate::v2_watermark::PATH)
        .header("content-type", "application/json")
        .extension(guard::VerifiedClient::from_verified_tls13_http11(
            guard::Identity::san("guild-test")?,
        ))
        .body(Body::from(wm))?;
    assert_eq!(
        build(false).oneshot(request).await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    d.close().await?;
    eprintln!(
        "PG postgres_configured_guild_parent_off_refuses_publication_and_watermark {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn w3_c06_002_guild_profile_conflict_during_privacy() -> TestResult {
    async fn publish_profile(db: &PgPool, name: &str) -> StatusCode {
        let identities = Arc::new(
            IdentityMap::new(vec![guard::Binding {
                identity: guard::Identity::san("guild-profile-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        );
        let app = crate::ingest::router(
            db.clone(),
            identities,
            crate::ingest::Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        );
        let raw = format!(
            r#"{{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"1","source_observed_at":"{}","profile":[{{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"{name}","vocation":4,"level":"412","guild_visible":true,"guild":[{{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":1}}]}}]}}"#,
            now()
        );
        app.oneshot(
            Request::post("/internal/v1/game-public/character-profile")
                .header("content-type", "application/json")
                .extension(guard::VerifiedClient::from_verified_tls13_http11(
                    guard::Identity::san("guild-profile-test").unwrap(),
                ))
                .body(Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    }
    struct Visible {
        db: PgPool,
        suppress: bool,
    }
    impl PublicVisibility for Visible {
        fn visible_batch<'a>(&'a self, _: &'a str, refs: &'a [MemberRef]) -> VisibilityResult<'a> {
            Box::pin(async move {
                if self.suppress {
                    assert_eq!(
                        publish_profile(&self.db, "Changed Name").await,
                        StatusCode::CONFLICT
                    );
                }
                Ok(vec![true; refs.len()])
            })
        }
    }
    let started = std::time::Instant::now();
    let d = crate::audit06_test_db::Database::new().await?;
    apply(&d.db, &publication(GOLDEN, false))
        .await
        .map_err(|_| "guild publication failed")?;
    let wm = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"guild","projection_epoch":"1","complete_through":"{}","observed_at":"{}"}}"#,
        now(),
        now()
    );
    apply(&d.db, &publication(&wm, true))
        .await
        .map_err(|_| "watermark failed")?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1,'null')").bind(i64::try_from(now())?).execute(&d.db).await?;
    assert_eq!(publish_profile(&d.db, "Aldric").await, StatusCode::OK);
    for suppress in [false, true] {
        let a = router(
            d.db.clone(),
            Arc::new(Visible {
                db: d.db.clone(),
                suppress,
            }),
            Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        );
        let response = a
            .oneshot(Request::get("/en/guilds/Red%20Rose").body(Body::empty())?)
            .await?;
        let status = response.status();
        let body = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        if suppress {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert!(body.contains("unavailable"));
            assert!(!body.contains("Red Rose</"));
        } else {
            assert!(body.contains("Red Rose</") && body.contains("Aldric"));
        }
    }
    d.close().await?;
    eprintln!(
        "w3_c06_002_guild_profile_conflict_during_privacy {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn w3_c06_010_guild_privacy_unavailability_returns_503() -> TestResult {
    struct Refuse;
    impl PublicVisibility for Refuse {
        fn visible_batch<'a>(&'a self, _: &'a str, _: &'a [MemberRef]) -> VisibilityResult<'a> {
            Box::pin(async { Err(Unavailable::Evidence) })
        }
    }
    let started = std::time::Instant::now();
    let d = crate::audit06_test_db::Database::new().await?;
    apply(&d.db, &publication(GOLDEN, false))
        .await
        .map_err(|_| "publish")?;
    let raw = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"guild","projection_epoch":"1","complete_through":"{}","observed_at":"{}"}}"#,
        now(),
        now()
    );
    apply(&d.db, &publication(&raw, true))
        .await
        .map_err(|_| "watermark")?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1,'null')").bind(i64::try_from(now())?).execute(&d.db).await?;
    let pages = || {
        router(
            d.db.clone(),
            Arc::new(Refuse),
            Config {
                enabled: true,
                source_authority: "oteryn:game:primary".into(),
            },
        )
    };
    for path in ["/en/guilds", "/en/guilds/Red%20Rose"] {
        let (status, body) = page_body(pages(), path).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!body.contains("Red Rose</") && !body.contains("Aldric"));
    }
    d.close().await?;
    eprintln!(
        "w3_c06_010_guild_privacy_unavailability_returns_503 {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn guild_migration_preserves_history_and_serves_both_versions() -> TestResult {
    let admin = PgPool::connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?).await?;
    let schema = format!("guild_upgrade_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let db = PgPool::connect_with(
        admin
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", schema.as_str())]),
    )
    .await?;
    sqlx::raw_sql(concat!(
        include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
        include_str!("../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql")
    ))
    .execute(&db)
    .await?;
    let displaced = publication(GOLDEN, false);
    let mut current = displaced.payload.clone();
    let current_id = "01934f10-7c06-7001-805b-3b1122334402";
    current["guild_id"] = current_id.into();
    current["guild"][0]["name"] = "Blue Rose".into();
    let legacy = serde_json::json!({"guild_id": displaced.subject, "guild":[]});
    let history = serde_json::json!({
        "epoch":"1", "version":displaced.revision, "watermark":displaced.observed,
        "payload":displaced.payload
    });
    let ambiguous = serde_json::json!({"contract_version": 3, "guild":[]});
    let timestamp = i64::try_from(now())?;
    sqlx::raw_sql(
        "UPDATE public_projection_state SET highest_epoch=1;
         CREATE TABLE community_privacy_revision(id integer PRIMARY KEY, revision bigint NOT NULL);
         INSERT INTO community_privacy_revision VALUES (1,0);",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload,previous_snapshot) VALUES
         ('guild',$1,false,1,6,$2,$3,$4),
         ('guild',$5,false,1,5,$2,$6,NULL),
         ('guild','unknown',false,1,1,$2,$7,NULL),
         ('guild','',true,1,1,$2,'null',NULL),
         ('guild','v2',true,1,1,$2,'{\"contract_version\":2,\"source_authority\":\"oteryn:game:primary\"}',NULL),
         ('character_profile','',true,1,1,$2,'null',NULL)",
    )
    .bind(&displaced.subject)
    .bind(timestamp)
    .bind(&legacy)
    .bind(&history)
    .bind(current_id)
    .bind(&current)
    .bind(&ambiguous)
    .execute(&db).await?;

    sqlx::raw_sql(include_str!(
        "../../../migrations/20261117200006_r7_com_a.sql"
    ))
    .execute(&db)
    .await?;
    let saved_history: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT previous_snapshot FROM public_snapshots WHERE family='guild_v1' AND subject=$1",
    )
    .bind(&displaced.subject)
    .fetch_one(&db)
    .await?;
    let unknown_family: String =
        sqlx::query_scalar("SELECT family FROM public_snapshots WHERE subject='unknown'")
            .fetch_one(&db)
            .await?;
    let saved_legacy = crate::ingest::latest(&db, "guild_v1", &displaced.subject).await?;
    let restored = crate::ingest::latest(&db, "guild", &displaced.subject).await?;
    let saved_current = crate::ingest::latest(&db, "guild", current_id).await?;
    let pages = router(
        db.clone(),
        Arc::new(Visible),
        Config {
            enabled: true,
            source_authority: "oteryn:game:primary".into(),
        },
    );
    let (red_status, red_page) = page_body(pages.clone(), "/en/guilds/Red%20Rose").await;
    let (blue_status, blue_page) = page_body(pages, "/en/guilds/Blue%20Rose").await;
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;

    assert_eq!(
        saved_history,
        Some(history),
        "displaced V2 history must be preserved"
    );
    assert_eq!(unknown_family, "guild");
    assert_eq!(saved_legacy, Some(legacy));
    assert_eq!(restored, Some(displaced.payload));
    assert_eq!(saved_current, Some(current));
    assert_eq!(red_status, StatusCode::OK);
    assert!(red_page.contains("Red Rose"));
    assert_eq!(blue_status, StatusCode::OK);
    assert!(blue_page.contains("Blue Rose"));
    Ok(())
}
