use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn flag_off_hides_route() -> TestResult {
    let world = WorldStore::new(
        oteryn_db::lazy_pool("postgres://localhost/unreachable")?,
        WorldOptions::default(),
    );
    let app = router(Viewer {
        enabled: false,
        directory: PathBuf::from("unused"),
        world,
    });
    let response = app
        .oneshot(Request::get("/en/atlas").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn postgres_viewer_routes() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL atlas test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::PgPool::connect(&url).await?;
    let mut random = [0; 8];
    getrandom::fill(&mut random)?;
    let schema = format!(
        "atlas_test_{}",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let version = "a".repeat(40);
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES ('01900000-0000-7000-8000-000000000001',$1,'ready')").bind(&version).execute(&pool).await?;
    sqlx::query(
        "UPDATE world_active_version SET version_id='01900000-0000-7000-8000-000000000001'",
    )
    .execute(&pool)
    .await?;
    for (kind, key, data) in [
        (
            "npc-placements",
            "visible",
            serde_json::json!({"declaration":{"npc":"public-npc","cell":{"x":100,"y":200,"floor":7}}}),
        ),
        (
            "npc-placements",
            "secret",
            serde_json::json!({"declaration":{"npc":"hidden-npc","cell":{"x":100,"y":200,"floor":7}}}),
        ),
        ("npcs", "public-npc", serde_json::json!({})),
        ("npcs", "hidden-npc", serde_json::json!({})),
    ] {
        sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ('01900000-0000-7000-8000-000000000001',$1,$2,$3,true)").bind(kind).bind(key).bind(data).execute(&pool).await?;
        if key != "hidden-npc" {
            sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES ($1,$2,true)")
                .bind(kind)
                .bind(key)
                .execute(&pool)
                .await?;
        }
    }
    let dir = tempfile::tempdir()?;
    let target = dir.path().join(&version).join("7");
    std::fs::create_dir_all(target.join("0/0"))?;
    let bytes = b"test-tile";
    std::fs::write(target.join("0/0/0.webp"), bytes)?;
    let m = Manifest {
        format_version: 1,
        world_version: version.clone(),
        floor: 7,
        tile_size: 256,
        bounds: crate::tiles::Bounds {
            min_x: 100,
            min_y: 200,
            max_x: 356,
            max_y: 456,
        },
        levels: vec![crate::tiles::Level {
            z: 0,
            width: 256,
            height: 256,
            columns: 1,
            rows: 1,
        }],
        tiles: vec![crate::tiles::Tile {
            path: "0/0/0.webp".into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }],
    };
    std::fs::write(target.join("manifest.json"), serde_json::to_vec(&m)?)?;
    let app = router(Viewer {
        enabled: true,
        directory: dir.path().into(),
        world: WorldStore::new(pool.clone(), WorldOptions { enabled: true }),
    });
    for lang in ["pl", "en"] {
        let r = app
            .clone()
            .oneshot(
                Request::get(format!("/{lang}/atlas?x=100&y=200&z=7&zoom=0")).body(Body::empty())?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::OK);
        assert!(
            r.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()?
                .contains("nonce-")
        );
        let body = String::from_utf8(to_bytes(r.into_body(), 1_000_000).await?.to_vec())?;
        assert!(body.contains(&format!("lang=\"{lang}\"")));
        assert!(body.contains("&#34;x&#34;:100"));
        assert!(body.contains("public-npc"));
        assert!(!body.contains("hidden-npc"));
    }
    for path in [
        "/en/atlas?x=-1&y=200",
        "/en/atlas?x=99&y=200",
        "/en/atlas?x=100",
        "/en/atlas?x=100&x=101&y=200",
        "/en/atlas?zoom=999",
        "/en/atlas?unknown=1",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(Request::get(path).body(Body::empty())?)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let tile = format!("/atlas/tiles/{version}/7/0/0/0.webp");
    assert_eq!(
        app.clone()
            .oneshot(Request::get(&tile).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::OK
    );
    std::fs::remove_file(target.join("0/0/0.webp"))?;
    assert_eq!(
        app.clone()
            .oneshot(Request::get(tile).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.oneshot(Request::get("/en/atlas?z=99").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
