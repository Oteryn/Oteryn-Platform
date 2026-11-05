use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_community::online::{self, CharacterRef, Config, PublicVisibility, VisibilityResult};
use sqlx::PgPool;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Visibility;
impl PublicVisibility for Visibility {
    fn hidden_batch<'a>(&'a self, refs: &'a [CharacterRef]) -> VisibilityResult<'a> {
        Box::pin(async { Ok(refs.iter().map(|r| r.name == "Hidden").collect()) })
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
async fn database() -> Result<Option<(PgPool, PgPool, String)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping online DB test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!(
        "online_test_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    // Identifier consists only of a fixed prefix, process ID and atomic integer.
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let options = root
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", schema.as_str())]);
    let db = PgPool::connect_with(options).await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    for world in [
        "11111111-1111-7111-8111-111111111111",
        "22222222-2222-7222-8222-222222222222",
    ] {
        sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('world_online',$1,1,1,1,$2)")
            .bind(world).bind(serde_json::json!({"world_id":world,"characters":[{"name":"Zed","level":"100","vocation":1},{"name":"Alice","level":"9","vocation":2},{"name":"Hidden","level":"999","vocation":3},{"name":"<script>","level":"2","vocation":0}]})).execute(&db).await?;
    }
    Ok(Some((root, db, schema)))
}
async fn request(
    app: Router,
    path: &str,
) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let response = app.oneshot(Request::get(path).body(Body::empty())?).await?;
    let status = response.status();
    let body = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    Ok((status, body))
}
async fn scenario(case: &str) -> TestResult {
    let Some((root, db, schema)) = database().await? else {
        return Ok(());
    };
    if case == "fresh" {
        sqlx::query("UPDATE public_snapshots SET watermark=extract(epoch FROM now())::bigint")
            .execute(&db)
            .await?;
    }
    if case == "epoch" {
        sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
            .execute(&db)
            .await?;
    }
    let config = Config {
        enabled: case != "disabled",
        stale_after: Duration::from_secs(600),
    };
    let visibility: Arc<dyn PublicVisibility> = if case == "unavailable" {
        Arc::new(online::FailClosed)
    } else {
        Arc::new(Visibility)
    };
    let app = online::router(db.clone(), visibility, config);
    let path = match case {
        "sort" => "/en/online?sort=level",
        "filter" => "/en/online?world=11111111-1111-7111-8111-111111111111",
        "bad" => "/en/online?sort=wrong",
        _ => "/en/online",
    };
    let (status, html) = request(app.clone(), path).await?;
    match case {
        "disabled" => assert_eq!(status, StatusCode::NOT_FOUND),
        "bad" => {
            assert_eq!(status, StatusCode::BAD_REQUEST);
            for q in [
                "world=bad!",
                "world=abc",
                "unknown=x",
                "sort=name&sort=level",
            ] {
                assert_eq!(
                    request(app.clone(), &format!("/pl/online?{q}")).await?.0,
                    StatusCode::BAD_REQUEST
                );
            }
        }
        "unavailable" => {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert!(html.contains("Temporarily unavailable"));
            assert!(!html.contains("Alice"));
        }
        "epoch" => {
            assert_eq!(status, StatusCode::OK);
            assert!(!html.contains("Alice"));
            assert_eq!(html.matches("<section").count(), 0);
        }
        _ => {
            assert_eq!(status, StatusCode::OK);
            assert!(!html.contains("Hidden"));
            assert!(html.contains("Visible characters online: 3"));
            assert!(!html.contains("<script>"));
            assert!(html.contains("script"));
            if case == "sort" {
                assert!(
                    html.find("Zed").ok_or("Zed missing")?
                        < html.find("Alice").ok_or("Alice missing")?
                );
            }
            if case == "filter" {
                assert_eq!(html.matches("<section").count(), 1);
            }
            if case == "fresh" {
                assert!(!html.contains("Data is stale"));
            }
            if case == "stale" {
                assert!(html.contains("Data is stale"));
                assert!(html.contains("Data as of: 1970-01-01 00:00:01"));
            }
            if case == "list" {
                assert_eq!(html.matches("<section").count(), 2);
                assert!(
                    html.find("Alice").ok_or("Alice missing")?
                        < html.find("Zed").ok_or("Zed missing")?
                );
                for path in ["/online", "/pl/online"] {
                    assert!(
                        request(app.clone(), path)
                            .await?
                            .1
                            .contains("Postacie online")
                    );
                }
            }
        }
    }
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
macro_rules! db_test {
    ($name:ident,$case:literal) => {
        #[tokio::test]
        async fn $name() -> TestResult {
            scenario($case).await
        }
    };
}
db_test!(list_per_world, "list");
db_test!(sort_by_level, "sort");
db_test!(world_filter, "filter");
db_test!(stale_notice, "stale");
db_test!(hidden_excluded, "hidden");
db_test!(fail_closed, "unavailable");
db_test!(flag_off_404, "disabled");
db_test!(bad_query_400, "bad");

db_test!(fresh_snapshot, "fresh");
db_test!(old_epoch_excluded, "epoch");
