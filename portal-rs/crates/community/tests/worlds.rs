use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use oteryn_community::worlds::{self, Config};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const ID: &str = "0195ad31-14c0-7a12-8c51-3019a3d64509";
#[derive(Debug)]
struct Database {
    admin: PgPool,
    db: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping worlds DB test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        // Identifier consists solely of this static prefix and process-local integers.
        let schema = format!(
            "worlds_{}_{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let admin = PgPool::connect(&url).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let options = admin
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", schema.as_str())]);
        let db = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;
        oteryn_db::migrate(&db).await?;
        Ok(Some(Self { admin, db, schema }))
    }
    fn app(&self, enabled: bool, threshold: u64) -> Router {
        // Mirrors the production composition, including the shared web surface.
        oteryn_web::router(
            oteryn_web::AppState {
                db: self.db.clone(),
            },
            "../../static",
        )
        .merge(worlds::router(
            self.db.clone(),
            Config {
                enabled,
                freshness_seconds: threshold,
            },
        ))
    }
    async fn registry(&self) -> TestResult {
        sqlx::query("INSERT INTO world_status(id,name,pvp_type,location) VALUES ($1::uuid,'Test <World>','Open PvP','Europe')").bind(ID).execute(&self.db).await?;
        Ok(())
    }
    async fn snapshot(&self, age: i64, payload: Value) -> TestResult {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        sqlx::query("UPDATE public_projection_state SET highest_epoch=1 WHERE id=1")
            .execute(&self.db)
            .await?;
        sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES ('world_online',$1,1,1,$2,$3) ON CONFLICT(family,subject,is_watermark) DO UPDATE SET watermark=EXCLUDED.watermark,payload=EXCLUDED.payload")
            .bind(ID).bind(now-age).bind(payload).execute(&self.db).await?;
        Ok(())
    }
    async fn finish(self) -> TestResult {
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
fn roster() -> Value {
    json!({"world_id": ID,"characters":[{"name":"Private roster name","vocation":1,"level":"10"}]})
}
async fn send(
    app: Router,
    uri: &str,
    etag: Option<&str>,
) -> Result<Response, Box<dyn std::error::Error>> {
    let mut request = Request::get(uri);
    if let Some(etag) = etag {
        request = request.header(header::IF_NONE_MATCH, etag);
    }
    Ok(app.oneshot(request.body(Body::empty())?).await?)
}
async fn body(response: Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 1_048_576).await?.to_vec(),
    )?)
}
async fn document(app: Router) -> Result<Value, Box<dyn std::error::Error>> {
    let response = send(app, "/api/v1/worlds", None).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(serde_json::from_str(&body(response).await?)?)
}
#[tokio::test]
async fn worlds_seeded_page_and_minimal_versioned_api() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.registry().await?;
    db.snapshot(0, roster()).await?;
    let response = send(db.app(true, 120), "/api/v1/worlds", None).await?;
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=30"
    );
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let raw = body(response).await?;
    assert!(!raw.contains("Private roster name"));
    let api: Value = serde_json::from_str(&raw)?;
    assert_eq!(api.as_object().map(|v| v.len()), Some(3));
    assert_eq!(api["schema"], 1);
    assert!(api["generated_at"].is_u64());
    assert_eq!(
        api["worlds"],
        json!([{"id":ID,"name":"Test <World>","status":"online","players_online":1}])
    );
    for (uri, lang, title) in [
        ("/en/worlds", "en", "World status"),
        ("/pl/worlds", "pl", "Status światów"),
        ("/worlds", "pl", "Status światów"),
    ] {
        let response = send(db.app(true, 120), uri, None).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
        let html = body(response).await?;
        for text in [
            title,
            "Test &#60;World&#62;",
            "Open PvP",
            "Europe",
            "UTC",
            "<dd>1</dd>",
            "op-badge--success",
            "name=\"viewport\"",
            "href=\"#main\"",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
        assert!(!html.contains("Private roster name"));
        assert!(!html.contains("<World>"));
    }
    db.finish().await
}
#[tokio::test]
async fn worlds_missing_stale_future_and_old_epoch_are_unknown() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.registry().await?;
    let api = document(db.app(true, 120)).await?;
    assert_eq!(api["worlds"][0]["players_online"], Value::Null);
    for age in [121, -60] {
        db.snapshot(age, roster()).await?;
        let api = document(db.app(true, 120)).await?;
        assert_eq!(api["worlds"][0]["status"], "unknown");
        assert!(api["worlds"][0]["players_online"].is_null());
        let html = body(send(db.app(true, 120), "/en/worlds", None).await?).await?;
        assert!(html.contains("Unknown"));
        assert!(html.contains("op-badge--warning"));
        assert!(!html.contains("<dd>0</dd>"));
    }
    db.snapshot(60, roster()).await?;
    assert_eq!(
        document(db.app(true, 120)).await?["worlds"][0]["status"],
        "online"
    );
    assert_eq!(
        document(db.app(true, 30)).await?["worlds"][0]["status"],
        "unknown"
    );
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2 WHERE id=1")
        .execute(&db.db)
        .await?;
    assert_eq!(
        document(db.app(true, 120)).await?["worlds"][0]["status"],
        "unknown"
    );
    db.finish().await
}
#[tokio::test]
async fn worlds_etag_revalidates_and_changes_with_public_data() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.registry().await?;
    db.snapshot(0, roster()).await?;
    let first = send(db.app(true, 120), "/api/v1/worlds", None).await?;
    let tag = first.headers()[header::ETAG].to_str()?.to_owned();
    for conditional in [
        tag.clone(),
        format!("\"other\", {}", &tag[2..]),
        "*".to_owned(),
    ] {
        let response = send(db.app(true, 120), "/api/v1/worlds", Some(&conditional)).await?;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=30"
        );
        assert_eq!(response.headers()[header::ETAG], tag);
        assert!(body(response).await?.is_empty());
    }
    for (age, payload) in [(121, roster()), (0, json!({"world_id":ID,"characters":[]}))] {
        db.snapshot(age, payload).await?;
        let response = send(db.app(true, 120), "/api/v1/worlds", Some(&tag)).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_ne!(response.headers()[header::ETAG], tag);
    }
    db.finish().await
}
#[tokio::test]
async fn worlds_flag_off_is_404_even_with_seeded_data() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.registry().await?;
    db.snapshot(0, roster()).await?;
    for uri in ["/worlds", "/en/worlds", "/pl/worlds", "/api/v1/worlds"] {
        assert_eq!(
            send(db.app(false, 120), uri, Some("*")).await?.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            send(db.app(true, 0), uri, None).await?.status(),
            StatusCode::NOT_FOUND
        );
    }
    assert!(!Config::default().enabled);
    db.finish().await
}
#[tokio::test]
async fn worlds_empty_registry_and_zero_players_are_distinct() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.snapshot(0, roster()).await?; // A projection cannot invent a registry entry.
    assert_eq!(document(db.app(true, 120)).await?["worlds"], json!([]));
    let html = body(send(db.app(true, 120), "/en/worlds", None).await?).await?;
    assert!(html.contains("No worlds available"));
    assert!(html.contains("op-empty"));
    db.registry().await?;
    db.snapshot(0, json!({"world_id":ID,"characters":[]}))
        .await?;
    let api = document(db.app(true, 120)).await?;
    assert_eq!(api["worlds"][0]["players_online"], 0);
    assert_eq!(api["worlds"][0]["status"], "online");
    db.finish().await
}
#[tokio::test]
async fn worlds_bad_registry_input_and_malformed_observations_fail_closed() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for id in ["not-a-uuid", "00000000-0000-0000-0000-000000000000"] {
        assert!(
            sqlx::query("INSERT INTO world_status(id,name) VALUES($1::uuid,'Bad')")
                .bind(id)
                .execute(&db.db)
                .await
                .is_err()
        );
    }
    db.registry().await?;
    for payload in [
        json!({"world_id":ID,"characters":{}}),
        json!({"world_id":"other","characters":[]}),
        json!({}),
    ] {
        db.snapshot(0, payload).await?;
        let api = document(db.app(true, 120)).await?;
        assert_eq!(api["worlds"][0]["status"], "unknown");
        assert!(api["worlds"][0]["players_online"].is_null());
    }
    db.finish().await
}
