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
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
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
    equal.payload["guild"][0]["name"] = "Other".into();
    assert_eq!(apply(&db, &equal).await, Err(StatusCode::CONFLICT));
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
    db.close().await;
    sqlx::query("DROP SCHEMA guild_t24e_test CASCADE")
        .execute(&pool)
        .await?;
    eprintln!("Guild PostgreSQL tests ran: 1");
    Ok(())
}
