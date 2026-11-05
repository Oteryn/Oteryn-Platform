use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn flags_deny_and_locales_are_complete() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://localhost/unreachable")?;
    for (enabled, world) in [(false, true), (true, false)] {
        let response = router(pool.clone(), enabled, world)
            .oneshot(Request::get("/atlas/spawns").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    for lang in ["pl", "en"] {
        let Json(text) = texts(Path(lang.into()))
            .await
            .map_err(|s| format!("translation status {s}"))?;
        assert_eq!(text.len(), 17);
        assert!(text.values().all(|value| !value.is_empty()));
    }
    assert!(texts(Path("unknown".into())).await.is_err());
    assert!(valid_text("Home", 1, 80));
    for text in ["", " ", "bad\nlabel", " trailing "] {
        assert!(!valid_text(text, 1, 80));
    }
    assert!(!valid_text(&"x".repeat(81), 1, 80));
    Ok(())
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    csrf: Option<&str>,
    body: &str,
) -> Result<Response, Box<dyn std::error::Error>> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header(
            "cookie",
            format!("{}={token}", oteryn_web::session::COOKIE_NAME),
        );
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    Ok(app
        .clone()
        .oneshot(request.body(Body::from(body.to_owned()))?)
        .await?)
}
async fn json(response: Response) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 1_000_000).await?,
    )?)
}

#[tokio::test]
async fn postgres_spawns_private_markers_and_reports() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL spawn test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("spawn_test_{}", Uuid::now_v7().simple());
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
    let app = router(pool.clone(), true, true);
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let token = "a".repeat(64);
    let other = "b".repeat(64);
    let csrf = "c".repeat(64);
    for (id, raw) in [(first, &token), (second, &other)] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,$2,'test-only','active','en')").bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES ($1,$2,'test',$3,now()+interval '1 hour',now()+interval '1 day')").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(&pool).await?;
    }
    assert_eq!(
        call(&app, "GET", "/atlas/private-markers", None, None, "")
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/atlas/private-markers",
            Some(&token),
            None,
            "{}"
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    let body = r#"{"x":100,"y":200,"floor":7,"text":"My home"}"#;
    let response = call(
        &app,
        "POST",
        "/atlas/private-markers",
        Some(&token),
        Some(&csrf),
        body,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    let id = json(response).await?["id"]
        .as_str()
        .ok_or("missing id")?
        .to_owned();
    let response = call(
        &app,
        "GET",
        "/atlas/private-markers",
        Some(&token),
        None,
        "",
    )
    .await?;
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        json(response).await?["markers"]
            .as_array()
            .ok_or("markers")?
            .len(),
        1
    );
    assert_eq!(
        json(
            call(
                &app,
                "GET",
                "/atlas/private-markers",
                Some(&other),
                None,
                ""
            )
            .await?
        )
        .await?["markers"],
        serde_json::json!([])
    );
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/atlas/private-markers/{id}"),
            Some(&other),
            Some(&csrf),
            ""
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    for body in [
        r#"{"x":-1,"y":200,"floor":7,"text":"a"}"#,
        r#"{"x":100,"y":200,"floor":7,"text":"a","account_id":"foreign"}"#,
    ] {
        assert_eq!(
            call(
                &app,
                "POST",
                "/atlas/private-markers",
                Some(&token),
                Some(&csrf),
                body
            )
            .await?
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        call(
            &app,
            "POST",
            "/atlas/map-reports",
            Some(&token),
            Some(&csrf),
            body
        )
        .await?
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let report = r#"{"x":100,"y":200,"floor":7,"text":"The map tile is incorrect."}"#;
    for _ in 0..5 {
        assert_eq!(
            call(
                &app,
                "POST",
                "/atlas/map-reports",
                Some(&token),
                Some(&csrf),
                report
            )
            .await?
            .status(),
            StatusCode::CREATED
        );
    }
    assert_eq!(
        call(
            &app,
            "POST",
            "/atlas/map-reports",
            Some(&token),
            Some(&csrf),
            report
        )
        .await?
        .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/atlas/private-markers/{id}"),
            Some(&token),
            Some(&csrf),
            ""
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(second)
        .execute(&pool)
        .await?;
    assert_eq!(
        call(
            &app,
            "GET",
            "/atlas/private-markers",
            Some(&other),
            None,
            ""
        )
        .await?
        .status(),
        // Suspension revokes the account's browser sessions (t06 credential-change trigger).
        StatusCode::UNAUTHORIZED
    );
    sqlx::query(
        "UPDATE web_sessions SET idle_expires_at=now()-interval '1 second' WHERE account_id=$1",
    )
    .bind(first)
    .execute(&pool)
    .await?;
    assert_eq!(
        call(
            &app,
            "GET",
            "/atlas/private-markers",
            Some(&token),
            None,
            ""
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );

    let version = Uuid::now_v7();
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES ($1,$2,'ready')")
        .bind(version)
        .bind("a".repeat(40))
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE world_active_version SET version_id=$1")
        .bind(version)
        .execute(&pool)
        .await?;
    // SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
    // content/world/spawns/spawns-00000-01999.json (small declaration shape, synthetic keys).
    let spawn = serde_json::json!({"declaration":{"points":[
        {"cell":{"x":100,"y":200,"floor":7},"creature":"visible","respawn_ms":90000},
        {"cell":{"x":101,"y":200,"floor":7},"creature":"visible","respawn_ms":90000},
        {"cell":{"x":100,"y":200,"floor":7},"creature":"hidden","respawn_ms":90000},
        {"cell":{"x":100,"y":200,"floor":8},"creature":"visible","respawn_ms":90000}]}});
    for (kind, key, data, allowed) in [
        ("spawns", "public", spawn.clone(), true),
        ("spawns", "secret", spawn, false),
        ("creatures", "visible", serde_json::json!({}), true),
        ("creatures", "hidden", serde_json::json!({}), false),
    ] {
        sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ($1,$2,$3,$4,true)").bind(version).bind(kind).bind(key).bind(data).execute(&pool).await?;
        if allowed {
            sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES ($1,$2,true)")
                .bind(kind)
                .bind(key)
                .execute(&pool)
                .await?;
        }
    }
    let path = format!(
        "/atlas/spawns?version={}&floor=7&min_x=100&min_y=200&max_x=101&max_y=200",
        "a".repeat(40)
    );
    let points = json(call(&app, "GET", &path, None, None, "").await?).await?;
    assert_eq!(points.as_array().ok_or("points")?.len(), 2);
    assert_eq!(points[0]["respawn_ms"], 90000);
    assert_eq!(points[0]["spawn"], "public");
    for path in [
        "/atlas/spawns?floor=7&min_x=101&min_y=200&max_x=100&max_y=200",
        "/atlas/spawns?floor=999",
        "/atlas/spawns?floor=7&unknown=1",
    ] {
        assert_eq!(
            call(&app, "GET", path, None, None, "").await?.status(),
            StatusCode::BAD_REQUEST
        );
    }
    sqlx::query(
        "UPDATE web_sessions SET idle_expires_at=now()+interval '1 hour' WHERE account_id=$1",
    )
    .bind(first)
    .execute(&pool)
    .await?;
    browser_check(&pool, &app, &token).await?;
    sqlx::query("UPDATE atlas_map_reports SET status='rejected' WHERE account_id=$1")
        .bind(first)
        .execute(&pool)
        .await?;
    assert_eq!(
        call(
            &app,
            "POST",
            "/atlas/map-reports",
            Some(&token),
            Some(&csrf),
            report
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO atlas_private_markers(account_id,x,y,floor,label) SELECT $1,100,200,7,'marker' FROM generate_series(1,100)").bind(first).execute(&pool).await?;
    assert_eq!(
        call(
            &app,
            "POST",
            "/atlas/private-markers",
            Some(&token),
            Some(&csrf),
            body
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    let stale = path.replace(&"a".repeat(40), &"b".repeat(40));
    assert_eq!(
        call(&app, "GET", &stale, None, None, "").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );

    sqlx::query("UPDATE world_entities SET data='{}' WHERE kind='spawns' AND key='public'")
        .execute(&pool)
        .await?;
    assert_eq!(
        call(&app, "GET", &path, None, None, "").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}

/// Optional real-browser qualification, using the same PostgreSQL fixture and HTTP handlers.
async fn browser_check(pool: &PgPool, app: &Router, token: &str) -> TestResult {
    let Ok(node) = std::env::var("PORTAL_ATLAS_BROWSER_NODE") else {
        return Ok(());
    };
    sqlx::query("UPDATE web_sessions SET idle_expires_at=now()+interval '1 hour'")
        .execute(pool)
        .await?;
    let directory = tempfile::tempdir()?;
    let version = "a".repeat(40);
    let target = directory.path().join(&version).join("7");
    std::fs::create_dir_all(&target)?;
    let manifest = crate::tiles::Manifest {
        format_version: 1,
        world_version: version,
        floor: 7,
        tile_size: 256,
        bounds: crate::tiles::Bounds {
            min_x: 0,
            min_y: 0,
            max_x: 512,
            max_y: 512,
        },
        levels: vec![
            crate::tiles::Level {
                z: 0,
                width: 256,
                height: 256,
                columns: 1,
                rows: 1,
            },
            crate::tiles::Level {
                z: 1,
                width: 512,
                height: 512,
                columns: 2,
                rows: 2,
            },
        ],
        tiles: vec![],
    };
    std::fs::write(target.join("manifest.json"), serde_json::to_vec(&manifest)?)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let app = app
        .clone()
        .merge(crate::web::router(crate::web::Viewer {
            enabled: true,
            directory: directory.path().into(),
            world: WorldStore::new(pool.clone(), WorldOptions { enabled: true }),
        }))
        .nest_service(
            "/static",
            tower_http::services::ServeDir::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../static"
            )),
        )
        .layer(Extension(oteryn_web::csrf::CsrfConfig {
            trusted_origins: vec![origin.clone()],
            ..Default::default()
        }))
        .layer(middleware::from_fn(viewer_policy));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let token = token.to_owned();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(node)
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/spawns_browser.cjs"
            ))
            .arg(origin)
            .arg(token)
            .output()
    })
    .await??;
    server.abort();
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "browser check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
