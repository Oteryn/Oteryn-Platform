use super::*;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::Request,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validates_queries_and_encodes_destinations() {
    for query in ["x", "bad\nquery", &"x".repeat(81)] {
        assert!(
            Filters {
                q: query.into(),
                ..Filters::default()
            }
            .validate()
            .is_err()
        );
    }
    for (kind, page) in [("unknown", 1), ("news", 0), ("news", 1001)] {
        assert!(
            Filters {
                q: "dragon".into(),
                kind: kind.into(),
                page: Some(page)
            }
            .validate()
            .is_err()
        );
    }
    let mut f = Filters {
        q: "  dragon   sword  ".into(),
        ..Filters::default()
    };
    assert!(f.validate().is_ok());
    assert_eq!(f.q, "dragon sword");
    assert_eq!(segment("a/b?<"), "a%2Fb%3F%3C");
    assert_eq!(
        destination("pl", "pages", "terms-of-service"),
        "/pl/legal/terms"
    );
    assert_eq!(
        destination("en", "wiki", "dragon"),
        "/en/wiki/articles/dragon"
    );
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED search PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| "random failed")?;
        let schema = format!(
            "search_{}",
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(3)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
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
fn filters(q: &str) -> Filters {
    Filters {
        q: q.into(),
        ..Filters::default()
    }
}
async fn get(app: Router, path: &str) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let mut request = Request::get(path).body(Body::empty())?;
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42148".parse::<std::net::SocketAddr>()?,
    ));
    let response = app.oneshot(request).await?;
    let status = response.status();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    Ok((
        status,
        String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?,
    ))
}
#[tokio::test]
async fn postgres_publication_translation_and_full_text() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::raw_sql("INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at) VALUES ('dragon-news','Dragon news','sword',now(),'Smocze wieści','miecz',now(),now()),('hidden-news','Dragon secret','hidden',NULL,NULL,NULL,NULL,NULL),('future-news','Dragon future','hidden',now()+interval '1 day',NULL,NULL,NULL,NULL)").execute(&db.pool).await?;
    let rows = editorial(&db.pool, "en", &filters("dragon")).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Dragon news");
    assert_eq!(
        editorial(&db.pool, "pl", &filters("smocze")).await?.len(),
        1
    );
    sqlx::raw_sql("UPDATE news SET updated_at=now()+interval '1 second' WHERE slug='dragon-news'")
        .execute(&db.pool)
        .await?;
    assert!(
        editorial(&db.pool, "pl", &filters("smocze"))
            .await
            .map_err(|_| "world search failed")?
            .is_empty()
    );
    assert!(
        editorial(&db.pool, "en", &filters("' OR true --"))
            .await
            .map_err(|_| "world search failed")?
            .is_empty()
    );
    sqlx::raw_sql("INSERT INTO pages(id,slug) VALUES (uuidv7(),'dragon-page'); INSERT INTO editorial_pages(page_id,locale,title,body,published_at) SELECT id,'en','Dragon page','page body',now() FROM pages").execute(&db.pool).await?;
    sqlx::raw_sql("INSERT INTO events(id,status,starts_at,ends_at,created_by,updated_by,published_by) VALUES ('00000000-0000-0000-0000-000000000048','published',now(),now()+interval '1 hour',uuidv7(),uuidv7(),uuidv7()); INSERT INTO event_translations(event_id,locale,title,slug,summary,body) VALUES ('00000000-0000-0000-0000-000000000048','en','Dragon event','dragon-event','summary','body')").execute(&db.pool).await?;
    sqlx::raw_sql("INSERT INTO wiki_guides(id,status,published_at) VALUES ('00000000-0000-0000-0000-000000000048','published',now()); INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note) VALUES (uuidv7(),'00000000-0000-0000-0000-000000000048',1,'en','Dragon guide','dragon-guide','summary','body',uuidv7(),'created')").execute(&db.pool).await?;
    assert_eq!(
        editorial(&db.pool, "en", &filters("dragon")).await?.len(),
        4
    );
    sqlx::raw_sql("UPDATE events SET status='archived'; UPDATE editorial_pages SET published_at=NULL; UPDATE wiki_guides SET status='archived',published_at=NULL").execute(&db.pool).await?;
    assert_eq!(
        editorial(&db.pool, "en", &filters("dragon")).await?.len(),
        1
    );
    db.close().await
}
#[tokio::test]
async fn postgres_http_success_refusal_bad_input_and_escaping() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::raw_sql("INSERT INTO news(slug,title,body,published_at) VALUES ('dragon','<script>dragon</script>','dragon body',now())").execute(&db.pool).await?;
    let app = routes(SearchState {
        pool: db.pool.clone(),
        world: None,
        characters: None,
    });
    let (status, html) = get(app.clone(), "/en/search?q=dragon").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
        "{html}"
    );
    assert!(!html.contains("<script>"));
    assert_eq!(
        get(app.clone(), "/en/search?q=x").await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        get(app.clone(), "/pl/search?q=dragon&kind=characters")
            .await?
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        get(app.clone(), "/en/search?q=dragon&kind=world").await?.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        get(app, "/en/search?q=dragon&page=bad").await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    db.close().await
}
#[tokio::test]
async fn postgres_world_candidates_require_current_public_api_allow() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::raw_sql("INSERT INTO world_versions(id,source_commit,status) VALUES ('00000000-0000-0000-0000-000000000048',repeat('a',40),'ready'); UPDATE world_active_version SET version_id='00000000-0000-0000-0000-000000000048'; INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ('00000000-0000-0000-0000-000000000048','Item','dragon-sword','{}',true); INSERT INTO world_visibility(kind,key,allowed) VALUES ('Item','dragon-sword',true)").execute(&db.pool).await?;
    let allow: WorldReader = Arc::new(|_, key| Box::pin(async move { Ok(Some(key)) }));
    assert_eq!(
        world_search(&db.pool, "en", "dragon", 1, allow.clone())
            .await
            .map_err(|_| "world search failed")?
            .len(),
        1
    );
    let deny: WorldReader = Arc::new(|_, _| Box::pin(async { Ok(None) }));
    assert!(
        world_search(&db.pool, "en", "dragon", 1, deny)
            .await
            .map_err(|_| "world search failed")?
            .is_empty()
    );
    sqlx::raw_sql("UPDATE world_visibility SET allowed=false")
        .execute(&db.pool)
        .await?;
    assert!(
        world_search(&db.pool, "en", "dragon", 1, allow)
            .await
            .map_err(|_| "world search failed")?
            .is_empty()
    );
    db.close().await
}

#[tokio::test]
async fn postgres_pagination_and_shared_search_abuse_boundary() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO news(slug,title,body,published_at) SELECT 'dragon-' || i,'Dragon ' || i,'body',now() FROM generate_series(1,25) i").execute(&db.pool).await?;
    let first = editorial(&db.pool, "en", &filters("dragon")).await?;
    assert_eq!(first.len(), 21);
    let second = editorial(
        &db.pool,
        "en",
        &Filters {
            page: Some(2),
            ..filters("dragon")
        },
    )
    .await?;
    assert_eq!(second.len(), 5);
    assert!(
        !first[..20]
            .iter()
            .any(|a| second.iter().any(|b| a.href == b.href))
    );
    let app = routes(SearchState {
        pool: db.pool.clone(),
        world: None,
        characters: None,
    });
    for _ in 0..30 {
        assert_eq!(
            get(app.clone(), "/en/search?q=dragon").await?.0,
            StatusCode::OK
        );
    }
    assert_eq!(
        get(app, "/en/search?q=dragon").await?.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    db.close().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Chromium and PORTAL_BROWSER_MODULES with browser dependencies"]
async fn postgres_browser_search_keyboard_and_accessibility() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at) VALUES ('dragon','Dragon <test>','dragon',now(),'Dragon <test>','dragon',now(),now())").execute(&db.pool).await?;
    let app = routes(SearchState {
        pool: db.pool.clone(),
        world: None,
        characters: None,
    })
    .nest_service(
        "/static",
        tower_http::services::ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../static")),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
    });
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(
            std::env::var("PORTAL_BROWSER_NODE").unwrap_or_else(|_| "node".into()),
        )
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/search/browser.cjs"
        ))
        .env("PORTAL_BROWSER_URL", format!("http://{address}"))
        .status()
    })
    .await?;
    server.abort();
    db.close().await?;
    assert!(result?.success());
    Ok(())
}
