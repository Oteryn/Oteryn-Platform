use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_admin::{
    rbac::Permission,
    web::{PanelConfig, PortalIdentity, StaffIdentity, StaffSession},
};
use oteryn_web::session::BrowserSession;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct Staff(Uuid);
impl StaffSession for Staff {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
async fn html(app: Router, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::get(path)
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 2_000_000).await?.to_vec(),
    )?)
}
#[tokio::test]
async fn postgres_a3_staff_libraries_reach_records_beyond_each_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("a3_library_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |connection, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'library-password','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
    sqlx::query("WITH created AS (INSERT INTO pages(id,slug) SELECT uuidv7(),'archive-'||n FROM generate_series(1,501) n RETURNING id,slug) INSERT INTO editorial_pages(page_id,locale,title,body) SELECT id,'en',slug,'Editorial library content' FROM created").execute(&pool).await?;
    sqlx::query("INSERT INTO announcements(severity,starts_at,title,body,created_by,updated_by) SELECT 'info',now(),'Library announcement '||n,'Editorial announcement body',$1,$1 FROM generate_series(1,501) n").bind(actor).execute(&pool).await?;
    let image = image::DynamicImage::new_rgb8(1, 1);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    let bytes = bytes.into_inner();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256) SELECT uuidv7(),'Library image '||n,'image/png',1,1,$1,$2 FROM generate_series(1,101) n").bind(&bytes).bind(Sha256::digest(&bytes).to_vec()).execute(&pool).await?;
    let page:(String,String)=sqlx::query_as("SELECT p.slug,e.locale FROM pages p JOIN editorial_pages e ON e.page_id=p.id ORDER BY p.slug,e.locale OFFSET 500 LIMIT 1").fetch_one(&pool).await?;
    let announcement: Uuid = sqlx::query_scalar(
        "SELECT id FROM announcements ORDER BY updated_at DESC,id DESC OFFSET 500 LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    let media: Uuid = sqlx::query_scalar(
        "SELECT id FROM media ORDER BY created_at DESC,id DESC OFFSET 100 LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    let config = PanelConfig {
        admin_panel: true,
        host: "staff.test".into(),
    };
    let reader = PortalIdentity::configured(pool.clone());
    let routes = [
        (
            "/admin/pages",
            crate::pages::staff_router(pool.clone(), config.clone(), Staff(actor), reader.clone()),
            format!("/admin/pages/{}/{}", page.0, page.1),
        ),
        (
            "/admin/announcements",
            crate::announcements::staff_router(
                pool.clone(),
                config.clone(),
                Staff(actor),
                reader.clone(),
            ),
            format!("/admin/announcements/{announcement}/edit"),
        ),
        (
            "/admin/media",
            oteryn_admin::web::guarded(
                crate::media::admin_router(pool.clone()),
                pool.clone(),
                config,
                Staff(actor),
                reader,
                Permission::ManageMedia,
            ),
            format!("/admin/media/{media}/thumbnail"),
        ),
    ];
    for (path, app, expected) in routes {
        assert!(html(app.clone(), path).await?.contains("?page=1"));
        if path == "/admin/media" {
            for (lang, label) in [("en", "Next page"), ("pl", "Następna strona")] {
                let page = html(app.clone(), &format!("{path}?lang={lang}")).await?;
                assert!(page.contains(&format!(r#"href="?page=1&amp;lang={lang}">{label}</a>"#)));
            }
        }
        assert!(
            html(app, &format!("{path}?page=1"))
                .await?
                .contains(&expected),
            "continuation must reach {expected}"
        );
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!(
        "postgres_a3_staff_libraries_reach_records_beyond_each_cap duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
