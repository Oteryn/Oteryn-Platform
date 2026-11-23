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
#[test]
fn validates_upload_metadata_and_alternative_text() {
    assert!(validate_upload_metadata("Diagram.PNG", Some("image/png")).is_ok());
    assert!(validate_upload_metadata("diagram.webp", Some("application/octet-stream")).is_ok());
    for (name, mime) in [
        ("diagram.png", "image/svg+xml"),
        ("diagram.svg", "image/png"),
        ("../diagram.png", "image/png"),
        (".png", "image/png"),
    ] {
        assert_eq!(
            validate_upload_metadata(name, Some(mime)),
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
    assert_eq!(validated_alt("  A diagram  "), Ok("A diagram"));
    for alt in ["  ", "A\0B", &"é".repeat(501)] {
        assert_eq!(validated_alt(alt), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
}

#[tokio::test]
async fn postgres_media_atomic_storage_references_and_integrity() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    oteryn_db::migrate(&pool).await.unwrap();
    browser_lifecycle(&pool).await;
    let id = Uuid::now_v7();
    let processed = process("x.png", &fixture(ImageFormat::Png, 10, 10)).unwrap();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256) VALUES($1,'Image','image/png',10,10,$2,$3)")
        .bind(id).bind(&processed.content).bind(Sha256::digest(&processed.content).to_vec()).execute(&pool).await.unwrap();
    assert_eq!(
        read(&pool, id, false, false).await.unwrap().status(),
        StatusCode::OK
    );
    sqlx::query("INSERT INTO media_references(media_id,consumer) VALUES($1,'test')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mut context = CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: None,
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(context.account).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'editor-test-password','active','en',now())")
        .bind(context.account).bind(format!("{}@example.invalid",context.account)).execute(&pool).await.unwrap();
    context.session_proof = Some(
        crate::test_staff::proof(&pool, context.account)
            .await
            .unwrap(),
    );
    let auth =
        MutationAuthorization::new(std::sync::Arc::new(oteryn_admin::rbac::Authorizer::new(
            pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
        )));
    assert_eq!(
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(id),
            Method::POST,
            Ok(Form(DeleteForm::default()))
        )
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
        read(&pool, id, false, false).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(id),
            Method::POST,
            Ok(Form(DeleteForm::default()))
        )
        .await
        .err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    let content: Vec<u8> = sqlx::query_scalar("SELECT content FROM media WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE media SET sha256=$2 WHERE id=$1")
        .bind(id)
        .bind(Sha256::digest(content).to_vec())
        .execute(&pool)
        .await
        .unwrap();
    crate::test_staff::race_account_before_fence(
        &pool,
        context.account,
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(id),
            Method::POST,
            Ok(Form(DeleteForm::default())),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        read(&pool, id, false, false).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
    news_replacement(&pool, &auth, context, &processed).await;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
async fn news_replacement(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    image: &ProcessedImage,
) {
    let original = Uuid::now_v7();
    let replacement = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256)
         VALUES ($1,'Original diagram','image/png',10,10,$3,$4),
                ($2,'Replacement diagram','image/png',10,10,$3,$4)",
    )
    .bind(original)
    .bind(replacement)
    .bind(&image.content)
    .bind(Sha256::digest(&image.content).to_vec())
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO news(slug,title,body,media_id,published_at)
         VALUES ('media-draft','Draft','Draft article',$1,NULL),
                ('media-published','Published','Published article',$1,now())",
    )
    .bind(original)
    .execute(pool)
    .await
    .unwrap();
    let remove = |replacement_id| {
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(original),
            Method::POST,
            Ok(Form(DeleteForm {
                replacement_id,
                lang: None,
            })),
        )
    };
    assert_eq!(
        remove(None).await.err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(
        remove(Some(original)).await.err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(
        remove(Some(Uuid::now_v7())).await.err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    sqlx::query("INSERT INTO media_references(media_id,consumer) VALUES($1,'other')")
        .bind(original)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        remove(Some(replacement)).await.err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
    let attached = || {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT media_id FROM news WHERE slug IN ('media-draft','media-published')",
        )
        .fetch_all(pool)
    };
    assert_eq!(attached().await.unwrap(), vec![original; 2]);
    assert_eq!(
        read(pool, replacement, false, true).await.err(),
        Some(StatusCode::NOT_FOUND)
    );

    sqlx::query("DELETE FROM media_references WHERE media_id=$1")
        .bind(original)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE media SET sha256=$2 WHERE id=$1")
        .bind(replacement)
        .bind(vec![0u8; 32])
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        remove(Some(replacement)).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(attached().await.unwrap(), vec![original; 2]);
    assert!(read(pool, original, false, false).await.is_ok());
    sqlx::query("UPDATE media SET sha256=$2 WHERE id=$1")
        .bind(replacement)
        .bind(Sha256::digest(&image.content).to_vec())
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        remove(Some(replacement)).await.unwrap().status(),
        StatusCode::FOUND
    );
    assert_eq!(attached().await.unwrap(), vec![replacement; 2]);
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='cms.news_media_replaced'
         AND before=jsonb_build_object('media_id', $2::uuid)
         AND after=jsonb_build_object('media_id', $3::uuid)",
    )
    .bind(context.account)
    .bind(original)
    .bind(replacement)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(audited, 2);
    assert_eq!(
        read(pool, original, false, false).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
    assert_eq!(
        read(pool, replacement, false, true).await.unwrap().status(),
        StatusCode::OK
    );
    staff_library_view(pool, replacement).await;

    sqlx::query("UPDATE news SET published_at=NULL WHERE slug='media-published'")
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        read(pool, replacement, false, true).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
}

async fn staff_library_view(pool: &PgPool, asset: Uuid) {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    let actor = Uuid::now_v7();
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES('media_editor') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access'), ($1,'media.manage')")
        .bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
    let response = test_app(pool.clone(), actor)
        .oneshot(
            Request::get(format!("/admin/media?lang=en&q={asset}"))
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = axum::body::to_bytes(response.into_body(), 100_000)
        .await
        .unwrap();
    let html = std::str::from_utf8(&html).unwrap();
    assert!(html.contains("Replacement diagram"));
    assert!(html.contains("News posts (2)"));
    assert!(!html.contains("/admin/news/"));
}

#[tokio::test]
async fn missing_staff_context_is_refused() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

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
    assert_eq!(
        read(pool, id, true, false).await.unwrap().status(),
        StatusCode::OK
    );
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
        .clone()
        .oneshot(
            Request::post(format!("/admin/media/{id}/delete"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("x-csrf-token", csrf)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(read(pool, id, false, false).await.is_ok());

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
        read(pool, id, false, false).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
}

#[tokio::test]
async fn a3_media_host_gate_precedes_multipart_validation() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    for (enabled, host) in [(false, "staff.test"), (true, "public.test")] {
        let lookup = |key: &str| match key {
            "OTERYN_ADMIN_PANEL" => Some(enabled.to_string()),
            "OTERYN_ADMIN_HOST" => Some("staff.test".into()),
            _ => None,
        };
        let app = configured_router_from(pool.clone(), &lookup).unwrap();
        for content_type in [
            None,
            Some("text/plain"),
            Some("multipart/form-data; boundary=missing"),
        ] {
            let mut request = Request::post("/admin/media").header("host", host);
            if let Some(value) = content_type {
                request = request.header("content-type", value);
            }
            let response = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                app.clone()
                    .oneshot(request.body(Body::from("malformed upload")).unwrap()),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        socket.write_all(format!("POST /admin/media HTTP/1.1\r\nHost: {host}\r\nContent-Type: multipart/form-data; boundary=unfinished\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_millis(250),
            socket.read_to_end(&mut response),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(response.starts_with(b"HTTP/1.1 404"));
        server.abort();
        let _ = server.await;
    }
}

#[test]
fn postgres_a3_processing_releases_rbac_for_other_mutations() {
    const NAME: &str =
        "media::media_tests::postgres_a3_processing_releases_rbac_for_other_mutations";
    if std::env::var("OTERYN_A3_DECODER_CHILD").as_deref() != Ok("1") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("OTERYN_A3_DECODER_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).max_blocking_threads(1).enable_all().build().unwrap().block_on(async {
        let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
        use axum::{body::{Body,to_bytes},http::Request};
        use oteryn_admin::rbac::{Authorizer,Change};
        use tower::ServiceExt;
        let started=std::time::Instant::now();
        let url=std::env::var("PORTAL_TEST_DATABASE_URL").expect("PostgreSQL required");
        let root=PgPool::connect(&url).await.unwrap();
        let schema=format!("a3_decode_{}",Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}"))).execute(&root).await.unwrap();
        let search=schema.clone();
        let pool=sqlx::postgres::PgPoolOptions::new().max_connections(4).after_connect(move |connection,_| {
            let search=search.clone();
            Box::pin(async move { sqlx::query("SELECT set_config('search_path',$1,false)").bind(search).execute(connection).await?;Ok(()) })
        }).connect(&url).await.unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let uploader=Uuid::now_v7();
        let editor=Uuid::now_v7();
        for actor in [uploader,editor] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'image-staff-password','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await.unwrap();
        }
        let reader=oteryn_admin::web::PortalIdentity::configured(pool.clone());
        let app=oteryn_admin::web::guarded(admin_router(pool.clone()),pool.clone(),oteryn_admin::web::PanelConfig { admin_panel:true,host:"staff.test".into() },Staff(uploader),reader.clone(),oteryn_admin::rbac::Permission::ManageMedia);
        let response=app.clone().oneshot(Request::get("/admin/media").header("host","staff.test").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::OK);
        let cookie=response.headers()["set-cookie"].to_str().unwrap().split(';').next().unwrap().to_owned();
        let html=String::from_utf8(to_bytes(response.into_body(),1_000_000).await.unwrap().to_vec()).unwrap();
        let csrf=html.split("name=\"_csrf\" value=\"").nth(1).unwrap().split('"').next().unwrap().to_owned();
        crate::test_staff::browser(&pool,uploader,&cookie).await.unwrap();
        let context=CheckContext { account:editor,require_staff_mfa:true,mfa_verified:true,session_proof:Some(crate::test_staff::proof(&pool,editor).await.unwrap()) };
        let auth=Authorizer::new(pool.clone(),reader);
        let image=fixture(ImageFormat::Png,32,32);
        let mut body=b"--a3-image\r\nContent-Disposition: form-data; name=\"alt_text\"\r\n\r\nQueued image\r\n--a3-image\r\nContent-Disposition: form-data; name=\"image\"; filename=\"queued.png\"\r\nContent-Type: image/png\r\n\r\n".to_vec();
        body.extend(image);
        body.extend(b"\r\n--a3-image--\r\n");
        let held_decoder=DECODERS.acquire().await.unwrap();
        let (release,receiver)=tokio::sync::oneshot::channel::<()>();
        let entered=std::sync::Arc::new(tokio::sync::Notify::new());
        let signal=entered.clone();
        let blocker=tokio::task::spawn_blocking(move || { signal.notify_one();receiver.blocking_recv() });
        tokio::time::timeout(std::time::Duration::from_secs(1),entered.notified()).await.unwrap();
        let upload=tokio::spawn(app.oneshot(Request::post("/admin/media").header("host","staff.test").header("origin","https://staff.test").header("cookie",cookie).header("x-csrf-token",csrf).header("content-type","multipart/form-data; boundary=a3-image").body(Body::from(body)).unwrap()));
        tokio::time::timeout(std::time::Duration::from_secs(2),async { while DECODERS.available_permits()!=0 { tokio::task::yield_now().await; } }).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1),auth.change_from_panel(context,Uuid::now_v7(),Change::CreateRole { key:"decode_parallel",permissions:&[] })).await.unwrap().unwrap();
        let now=time::OffsetDateTime::now_utc();
        let draft=crate::events::Draft { reason: "Editorial test change".into(), starts_at:now+time::Duration::hours(1),ends_at:now+time::Duration::hours(2),featured:false,en:crate::events::Translation { title:"Parallel event".into(),slug:"parallel-event".into(),summary:"Editorial save during image processing".into(),body:"Editorial save during image processing".into() },pl:None };
        tokio::time::timeout(std::time::Duration::from_secs(1),crate::events::Events::new(pool.clone()).save(&auth,context,None,draft)).await.unwrap().unwrap();
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM media").fetch_one(&pool).await.unwrap(),0);
        release.send(()).unwrap();
        blocker.await.unwrap().unwrap();
        assert_eq!(tokio::time::timeout(std::time::Duration::from_secs(2),upload).await.unwrap().unwrap().unwrap().status(),StatusCode::FOUND);
        drop(held_decoder);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM media").fetch_one(&pool).await.unwrap(),1);
        pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE"))).execute(&root).await.unwrap();
        root.close().await;
        eprintln!("postgres_a3_processing_releases_rbac_for_other_mutations duration: {:?}",started.elapsed());
    });
}
