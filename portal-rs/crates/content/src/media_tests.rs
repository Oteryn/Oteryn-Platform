#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
fn fixture(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
    let image = image::DynamicImage::new_rgb8(width, height);
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}
#[test]
fn normalizes_supported_images_and_variants() {
    for (format, name) in [
        (ImageFormat::Png, "x.png"),
        (ImageFormat::Jpeg, "x.jpeg"),
        (ImageFormat::WebP, "x.webp"),
    ] {
        let result = process(name, &fixture(format, 640, 160)).unwrap();
        assert_eq!((result.width, result.height), (640, 160));
        let thumb = image::load_from_memory(&result.thumbnail.unwrap()).unwrap();
        assert_eq!(thumb.dimensions(), (320, 80));
    }
    assert!(
        process("small.png", &fixture(ImageFormat::Png, 20, 20))
            .unwrap()
            .thumbnail
            .is_none()
    );
}
#[test]
fn refuses_bad_input_and_disguised_payloads() {
    let bytes = fixture(ImageFormat::Png, 1, 1);
    assert_eq!(
        process("x.jpg", &bytes).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    let mut appended = bytes;
    appended.extend_from_slice(b"payload");
    assert!(process("x.png", &appended).is_err());
    assert!(process("x.svg", b"<svg/>").is_err());
    assert!(process("x.png", &[]).is_err());
    assert!(process("x.png", &vec![0; MAX_BYTES + 1]).is_err());
    assert!(process("wide.png", &fixture(ImageFormat::Png, 6001, 1)).is_err());
}
#[tokio::test]
async fn postgres_media_atomic_storage_references_and_integrity() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL media test: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("t12_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let path = format!("SET search_path TO {schema}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(path)).execute(c).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    for migration in oteryn_db::MIGRATOR.iter() {
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration.sql.as_ref()))
            .execute(&pool)
            .await
            .unwrap();
    }
    browser_lifecycle(&pool).await;
    let id = Uuid::now_v7();
    let processed = process("x.png", &fixture(ImageFormat::Png, 10, 10)).unwrap();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256) VALUES($1,'Image','image/png',10,10,$2,$3)")
        .bind(id).bind(&processed.content).bind(Sha256::digest(&processed.content).to_vec()).execute(&pool).await.unwrap();
    assert_eq!(
        read(&pool, id, false).await.unwrap().status(),
        StatusCode::OK
    );
    sqlx::query("INSERT INTO media_references(media_id,consumer) VALUES($1,'test')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let context = CheckContext::new(Uuid::now_v7());
    assert_eq!(
        delete(State(pool.clone()), Extension(context), Path(id))
            .await
            .err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    sqlx::query("DELETE FROM media_references WHERE media_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE media SET sha256=$2 WHERE id=$1")
        .bind(id)
        .bind(vec![0u8; 32])
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        read(&pool, id, false).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(
        delete(State(pool.clone()), Extension(context), Path(id))
            .await
            .err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    sqlx::query("DELETE FROM media WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
#[tokio::test]
async fn missing_staff_context_is_refused() {
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let response = configured_router(pool)
        .unwrap()
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/media")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[derive(Clone)]
struct Staff(Uuid);
impl oteryn_admin::web::StaffSession for Staff {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
impl oteryn_admin::rbac::AccountStateReader for Staff {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        Ok(oteryn_admin::rbac::AccountState::Active)
    }
}
fn test_app(pool: PgPool, actor: Uuid) -> Router {
    oteryn_admin::web::guarded(
        admin_router(pool.clone()),
        pool,
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Staff(actor),
        Staff(actor),
        Permission::ManageMedia,
    )
    .layer(axum::middleware::from_fn(multipart_csrf))
    .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
}
async fn browser_lifecycle(pool: &PgPool) {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(pool).await.unwrap();
    let app = test_app(pool.clone(), actor);
    let page = app
        .clone()
        .oneshot(
            Request::get("/admin/media?lang=en")
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let cookie = page.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html =
        String::from_utf8(to_bytes(page.into_body(), 100000).await.unwrap().to_vec()).unwrap();
    assert!(html.contains("Alternative text"));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let make = |token: &str, alt: &str| {
        let mut body = format!("--boundary\r\nContent-Disposition: form-data; name=\"_csrf\"\r\n\r\n{token}\r\n--boundary\r\nContent-Disposition: form-data; name=\"alt_text\"\r\n\r\n{alt}\r\n--boundary\r\nContent-Disposition: form-data; name=\"image\"; filename=\"x.png\"\r\nContent-Type: image/png\r\n\r\n").into_bytes();
        body.extend(fixture(ImageFormat::Png, 640, 160));
        body.extend_from_slice(b"\r\n--boundary--\r\n");
        Request::post("/admin/media")
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", &cookie)
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=boundary",
            )
            .body(Body::from(body))
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(make("wrong", "Image"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone().oneshot(make(csrf, " ")).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.clone()
            .oneshot(make(csrf, "Image"))
            .await
            .unwrap()
            .status(),
        StatusCode::FOUND
    );
    let id: Uuid = sqlx::query_scalar(
        "SELECT id FROM media WHERE alt_text='Image' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(read(pool, id, true).await.unwrap().status(), StatusCode::OK);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='editorial_media.uploaded'",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
    sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) SELECT $1,'editorial_media.uploaded','test','null'::jsonb,'null'::jsonb,uuidv7() FROM generate_series(1,9)").bind(actor).execute(pool).await.unwrap();
    assert_eq!(
        app.clone()
            .oneshot(make(csrf, "Image"))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );

    let response = app
        .oneshot(
            Request::post(format!("/admin/media/{id}/delete"))
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        read(pool, id, false).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
}
