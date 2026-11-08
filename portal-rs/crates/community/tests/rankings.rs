use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_community::rankings::{self, PublicVisibility, UnavailableVisibility, VisibilityFuture};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const WORLD: &str = "01900000-0000-7000-8000-000000000001";
const OTHER: &str = "01900000-0000-7000-8000-000000000002";
struct Visible;
impl PublicVisibility for Visible {
    fn is_hidden<'a>(&'a self, character: &'a str) -> VisibilityFuture<'a, bool> {
        Box::pin(async move { Ok(character.ends_with("0001")) })
    }
}
async fn send(app: Router, path: &str) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let response = app.oneshot(Request::get(path).body(Body::empty())?).await?;
    let status = response.status();
    let html = String::from_utf8(to_bytes(response.into_body(), 1_000_000).await?.to_vec())?;
    Ok((status, html))
}
#[tokio::test]
async fn disabled_unknown_and_bad_input() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        send(
            rankings::router(db.clone(), false, Arc::new(Visible)),
            "/en/rankings/level"
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    let app = rankings::router(db, true, Arc::new(Visible));
    for path in [
        "/en/rankings/unknown",
        "/en/rankings/level?world=no",
        "/en/rankings/level?vocation=6",
        "/fr/rankings/level",
    ] {
        assert_eq!(
            send(app.clone(), path).await?.0,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    for query in [
        "page=0",
        "page=-1",
        "page=abc",
        "page=9999999999999999999999999999",
        "page=1&page=2",
        "world=%ZZ",
        "world=%FF",
        "world=%",
        "vocation=1&vocation=2",
        "sort=name",
        "page=01",
    ] {
        let (status, html) = send(app.clone(), &format!("/pl/rankings/level?{query}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert!(html.contains("Nieprawidłowe zapytanie"));
    }
    Ok(())
}
async fn seeded() -> Result<Option<(PgPool, PgPool, String)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping rankings DB test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let pool = PgPool::connect(&url).await?;
    // The identifier contains only a fixed prefix and UUID hexadecimal digits.
    let schema = format!("rankings_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    let options = pool
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
    for world in [WORLD, OTHER] {
        for category in 1..=9 {
            for filter in [0, 1] {
                let rows: Vec<_> = (1..=102).map(|i| serde_json::json!({"rank":i.to_string(),"character_id":format!("01900000-0000-7000-8000-{i:012}"),"name":if i==1 { "HiddenSecret".into() } else if world==OTHER { format!("Other{i}") } else if filter==1 { format!("Druid{i}") } else if i==2 { "<script>alert(1)</script>".into() } else { format!("Player{i}") },"vocation":filter,"level":"200","value":(10000-i).to_string()})).collect();
                let payload = serde_json::json!({"world_id":world,"category":category,"filter":filter,"computed_at":1790000000,"rows":rows});
                sqlx::query("INSERT INTO public_snapshots (family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('highscores',$1,false,1,1,1790000000,$2)").bind(format!("{world}:{category}:{filter}")).bind(payload).execute(&db).await?;
            }
        }
    }
    Ok(Some((pool, db, schema)))
}
async fn cleanup(pool: PgPool, db: PgPool, schema: String) -> TestResult {
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
#[tokio::test]
async fn database_categories_locales_privacy_and_escaping() -> TestResult {
    let Some((pool, db, schema)) = seeded().await? else {
        return Ok(());
    };
    let app = rankings::router(db.clone(), true, Arc::new(Visible));
    for locale in ["pl", "en"] {
        for kind in [
            "level",
            "experience",
            "magic",
            "fist",
            "club",
            "sword",
            "axe",
            "distance",
            "shielding",
            "fishing",
        ] {
            let (status, html) = send(
                app.clone(),
                &format!("/{locale}/rankings/{kind}?world={WORLD}"),
            )
            .await?;
            assert_eq!(status, StatusCode::OK, "{kind}");
            assert!(html.contains(&format!("lang=\"{locale}\"")));
            assert!(html.contains("op-table-wrap"));
            assert!(html.contains("Player51"));
            assert!(!html.contains("Player52"));
            assert!(!html.contains("HiddenSecret"));
            assert!(html.contains("&#60;script&#62;"));
            assert!(!html.contains("<script>"));
            assert!(!html.contains("character_id"));
            assert!(html.contains("2026-09-21T14:13:20Z"));
        }
    }
    cleanup(pool, db, schema).await
}
#[tokio::test]
async fn database_filters_and_pagination() -> TestResult {
    let Some((pool, db, schema)) = seeded().await? else {
        return Ok(());
    };
    let app = rankings::router(db.clone(), true, Arc::new(Visible));
    let (status, html) = send(
        app.clone(),
        &format!("/en/rankings/level?world={OTHER}&vocation=1&page=2"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Other52"));
    assert!(html.contains("Other101"));
    assert!(!html.contains("Other102"));
    assert!(html.contains("vocation=1&#38;page=3"));
    let (_, html) = send(
        app.clone(),
        &format!("/en/rankings/magic?world={WORLD}&vocation=1"),
    )
    .await?;
    assert!(html.contains("Druid2"));
    assert!(!html.contains("Player"));
    let (status, html) = send(
        app.clone(),
        &format!("/en/rankings/level?world={WORLD}&page=3"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Player102"));
    assert!(html.contains("aria-disabled=\"true\""));
    assert_eq!(
        send(
            app.clone(),
            &format!("/en/rankings/level?world={WORLD}&page=4")
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            app.clone(),
            "/en/rankings/level?world=01900000-0000-7000-8000-000000000099"
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            app.clone(),
            &format!("/en/rankings/level?world={WORLD}&vocation=5")
        )
        .await?
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    cleanup(pool, db, schema).await
}
#[tokio::test]
async fn database_visibility_failure_empty_and_epoch_fence() -> TestResult {
    let Some((pool, db, schema)) = seeded().await? else {
        return Ok(());
    };
    let path = format!("/en/rankings/level?world={WORLD}");
    let (status, html) = send(
        rankings::router(db.clone(), true, Arc::new(UnavailableVisibility)),
        &path,
    )
    .await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(html.contains("Temporarily unavailable"));
    assert!(!html.contains("Player"));
    sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{rows}','[]')")
        .execute(&db)
        .await?;
    let app = rankings::router(db.clone(), true, Arc::new(Visible));
    let (status, html) = send(app.clone(), &path).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("No public characters"));
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
        .execute(&db)
        .await?;
    assert_eq!(
        send(app, "/en/rankings/level").await?.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    cleanup(pool, db, schema).await
}
#[tokio::test]
async fn home_top_skips_hidden_and_fails_closed() -> TestResult {
    let Some((pool, db, schema)) = seeded().await? else {
        return Ok(());
    };
    let (rows, as_of) = rankings::home_top(&db, &Visible, oteryn_web::Language::Pl, 5, true)
        .await
        .map_err(|_| "home ranking unavailable")?;
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "<script>alert(1)</script>",
            "Player3",
            "Player4",
            "Player5",
            "Player6"
        ]
    );
    assert_eq!(rows[1].rank, "3");
    assert_eq!(rows[1].url.as_deref(), Some("/pl/characters/Player3"));
    assert_eq!(as_of, "2026-09-21 14:13");
    let (rows, _) = rankings::home_top(&db, &Visible, oteryn_web::Language::En, 5, false)
        .await
        .map_err(|_| "home ranking unavailable")?;
    assert!(rows.iter().all(|row| row.url.is_none()));
    assert!(
        rankings::home_top(
            &db,
            &UnavailableVisibility,
            oteryn_web::Language::Pl,
            5,
            true
        )
        .await
        .is_err()
    );
    cleanup(pool, db, schema).await
}
