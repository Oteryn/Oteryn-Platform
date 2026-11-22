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
    sqlx::raw_sql(concat!(
        include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
        include_str!("../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql")
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql("CREATE TABLE community_privacy_revision(id integer PRIMARY KEY,revision bigint NOT NULL DEFAULT 0); INSERT INTO community_privacy_revision(id) VALUES(1)").execute(&db).await?;
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

#[tokio::test]
async fn audit06_home_scans_past_fifty_hidden_entries() -> TestResult {
    struct HiddenFifty;
    impl PublicVisibility for HiddenFifty {
        fn is_hidden<'a>(&'a self, id: &'a str) -> VisibilityFuture<'a, bool> {
            Box::pin(async move {
                Ok(id
                    .rsplit('-')
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .is_some_and(|n| n <= 50))
            })
        }
    }
    let (root, db, schema) = seeded().await?.ok_or("PostgreSQL required")?;
    let (rows, _) = rankings::home_top(&db, &HiddenFifty, oteryn_web::Language::En, 5, false)
        .await
        .map_err(|_| "ranking unavailable")?;
    assert_eq!(
        rows.iter().map(|r| r.rank.as_str()).collect::<Vec<_>>(),
        ["51", "52", "53", "54", "55"]
    );
    cleanup(root, db, schema).await
}

#[tokio::test]
async fn audit06_rankings_recheck_epoch_and_same_epoch_suppression() -> TestResult {
    struct Invalidating {
        db: PgPool,
        epoch: bool,
    }
    impl PublicVisibility for Invalidating {
        fn is_hidden<'a>(&'a self, _: &'a str) -> VisibilityFuture<'a, bool> {
            Box::pin(async { Ok(false) })
        }
        fn batch<'a>(&'a self, refs: &'a [String]) -> VisibilityFuture<'a, Vec<bool>> {
            Box::pin(async move {
                let query = if self.epoch {
                    "UPDATE public_projection_state SET highest_epoch=2"
                } else {
                    "UPDATE public_snapshots SET payload=jsonb_set(payload,'{rows}','[]') WHERE family='highscores'"
                };
                sqlx::query(query)
                    .execute(&self.db)
                    .await
                    .map_err(|_| rankings::VisibilityUnavailable)?;
                Ok(vec![false; refs.len()])
            })
        }
    }
    for epoch in [false, true] {
        for home in [false, true] {
            let (root, db, schema) = seeded().await?.ok_or("PostgreSQL required")?;
            let visibility = Arc::new(Invalidating {
                db: db.clone(),
                epoch,
            });
            if home {
                assert!(
                    rankings::home_top(
                        &db,
                        visibility.as_ref(),
                        oteryn_web::Language::En,
                        5,
                        false
                    )
                    .await
                    .is_err()
                );
            } else {
                assert_eq!(
                    send(
                        rankings::router(db.clone(), true, visibility),
                        "/en/rankings/level"
                    )
                    .await?
                    .0,
                    StatusCode::SERVICE_UNAVAILABLE
                );
            }
            cleanup(root, db, schema).await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn audit_w2_c06_targeted_ranking_ignores_unrelated_subject_count() -> TestResult {
    let started = std::time::Instant::now();
    let (pool, db, schema) = seeded().await?.ok_or("PostgreSQL required")?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) SELECT 'highscores',$1||':extra:'||n,false,1,1,1790000000,'{}'::jsonb FROM generate_series(1,1101) n")
        .bind(OTHER).execute(&db).await?;
    assert_eq!(
        send(
            rankings::router(db.clone(), true, Arc::new(Visible)),
            &format!("/en/rankings/level?world={WORLD}")
        )
        .await?
        .0,
        StatusCode::OK
    );
    cleanup(pool, db, schema).await?;
    eprintln!(
        "audit_w2_c06_targeted_ranking_ignores_unrelated_subject_count {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn cold_rankings_are_marked_but_database_failures_are_not() -> TestResult {
    let Some((pool, db, schema)) = seeded().await? else {
        return Ok(());
    };
    sqlx::query("DELETE FROM public_snapshots WHERE family='highscores'")
        .execute(&db)
        .await?;
    let app = rankings::router(db.clone(), true, Arc::new(Visible));
    let request = || Request::get("/en/rankings/experience").body(Body::empty());
    let cold = app.clone().oneshot(request()?).await?;
    assert_eq!(cold.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(cold.headers()["x-oteryn-game-data"], "unavailable");
    sqlx::query("ALTER TABLE public_snapshots RENAME TO broken_public_snapshots")
        .execute(&db)
        .await?;
    let failed = app.oneshot(request()?).await?;
    assert_eq!(failed.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!failed.headers().contains_key("x-oteryn-game-data"));
    assert_eq!(
        to_bytes(cold.into_body(), 64 * 1024).await?,
        to_bytes(failed.into_body(), 64 * 1024).await?,
    );
    cleanup(pool, db, schema).await
}
