#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
fn fixture(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
    let image = image::DynamicImage::new_rgb8(width, height);
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}
#[test]
fn normalizes_large_progressive_jpeg_with_full_chroma() {
    let bytes = include_bytes!("../../../e2e/fixtures/uploads/progressive-444.jpeg");
    assert!(bytes.len() <= MAX_BYTES);
    let info = oxiarc_jpeg::Decoder::new(bytes.as_slice())
        .read_info()
        .unwrap();
    assert_eq!(info.process, oxiarc_jpeg::CodingProcess::Progressive);
    assert_eq!(info.num_components, 3);
    assert!(
        info.components()
            .iter()
            .all(|component| (component.h, component.v) == (1, 1))
    );

    let image = process("progressive-444.jpeg", bytes).unwrap();
    assert_eq!(
        (image.width, image.height, image.mime),
        (4608, 3456, "image/jpeg")
    );
    let normalized = image::load_from_memory(&image.content).unwrap();
    assert_eq!(normalized.dimensions(), (4608, 3456));
    let thumbnail = image::load_from_memory(&image.thumbnail.unwrap()).unwrap();
    assert_eq!(thumbnail.dimensions(), (320, 240));
}

#[test]
fn jpeg_with_incomplete_scan_passes_header_checks_but_is_rejected() {
    let bytes = include_bytes!("../../../e2e/fixtures/uploads/truncated-scan.jpeg");
    assert_eq!(image::guess_format(bytes).unwrap(), ImageFormat::Jpeg);
    assert!(bytes.len() <= MAX_BYTES);
    assert!(bytes.ends_with(&[255, 217]));
    assert_eq!(
        ImageReader::with_format(Cursor::new(bytes), ImageFormat::Jpeg)
            .into_dimensions()
            .unwrap(),
        (640, 160)
    );
    assert_eq!(
        process("truncated-scan.jpeg", bytes).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn png_with_missing_scanline_reaches_pixel_decoder() {
    let bytes = include_bytes!("../../../e2e/fixtures/uploads/truncated-pixels.png");
    assert_eq!(image::guess_format(bytes).unwrap(), ImageFormat::Png);
    assert!(bytes.len() <= MAX_BYTES);
    assert!(bytes.ends_with(b"\0\0\0\0IEND\xaeB`\x82"));
    let reader = || ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    assert_eq!(reader().into_dimensions().unwrap(), (2, 2));
    assert!(reader().decode().is_err());
    assert_eq!(
        process("truncated-pixels.png", bytes).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
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
        read(&pool, id, false).await.unwrap().status(),
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
            Query(Options::default()),
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
        read(&pool, id, false).await.err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(
        delete(
            State(pool.clone()),
            Extension(context),
            Extension(auth.clone()),
            Path(id),
            Query(Options::default()),
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
            Query(Options::default()),
        ),
    )
    .await
    .unwrap()
    .unwrap();
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
    let sessions = SessionLayer::new(pool.clone());
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
    .layer(axum::middleware::from_fn_with_state(
        sessions,
        multipart_csrf,
    ))
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
    assert!(html.contains(r#"action="/admin/media?lang=en""#));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let make_with_image = |token: &str, alt: &str, mime: &str, image: &[u8]| {
        let mut body = format!("--boundary\r\nContent-Disposition: form-data; name=\"_csrf\"\r\n\r\n{token}\r\n--boundary\r\nContent-Disposition: form-data; name=\"alt_text\"\r\n\r\n{alt}\r\n--boundary\r\nContent-Disposition: form-data; name=\"image\"; filename=\"x.png\"\r\nContent-Type: {mime}\r\n\r\n").into_bytes();
        body.extend_from_slice(image);
        body.extend_from_slice(b"\r\n--boundary--\r\n");
        Request::post("/admin/media?lang=en")
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
    let png = fixture(ImageFormat::Png, 640, 160);
    let make = |token: &str, alt: &str, mime: &str| make_with_image(token, alt, mime, &png);
    assert_eq!(
        app.clone()
            .oneshot(make("wrong", "Image", "image/png"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(make(csrf, " ", "image/png"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.clone()
            .oneshot(make(csrf, "Mismatched MIME", "image/jpeg"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM media WHERE alt_text='Mismatched MIME')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );
    for header_csrf in [false, true] {
        let mut request = make(
            csrf,
            "Duplicate MIME",
            "image/png\r\ncontent-type: image/png",
        );
        if header_csrf {
            request
                .headers_mut()
                .insert("x-csrf-token", csrf.parse().unwrap());
        }
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert!(
            !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM media WHERE alt_text='Duplicate MIME')"
            )
            .fetch_one(pool)
            .await
            .unwrap()
        );
    }
    let mut stalled = make(csrf, "Stalled upload", "image/png");
    stalled
        .headers_mut()
        .insert("x-csrf-token", csrf.parse().unwrap());
    let body = to_bytes(
        std::mem::replace(stalled.body_mut(), Body::empty()),
        MAX_BYTES + 16384,
    )
    .await
    .unwrap();
    // Deliver a partial frame and keep the connection open without EOF.
    use futures_util::StreamExt;
    let first_frame = Ok::<_, std::convert::Infallible>(body.slice(..body.len() / 2));
    *stalled.body_mut() = Body::from_stream(
        futures_util::stream::iter([first_frame]).chain(futures_util::stream::pending()),
    );
    let start = tokio::time::Instant::now();
    let refusal = tokio::time::timeout(Duration::from_secs(20), app.clone().oneshot(stalled))
        .await
        .expect("the authenticated body deadline must respond within the test budget")
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(start.elapsed() >= Duration::from_secs(15));
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM media WHERE alt_text='Stalled upload')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );

    let mut anonymous = make(csrf, "Anonymous stalled upload", "image/png");
    anonymous.headers_mut().remove(header::COOKIE);
    *anonymous.body_mut() = Body::from_stream(futures_util::stream::pending::<
        Result<Bytes, std::convert::Infallible>,
    >());
    let refusal = tokio::time::timeout(Duration::from_secs(8), app.clone().oneshot(anonymous))
        .await
        .expect("anonymous forms must keep the short body deadline")
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::REQUEST_TIMEOUT);

    let uploaded = app
        .clone()
        .oneshot(make(csrf, "Image", "image/png"))
        .await
        .unwrap();
    assert_eq!(uploaded.status(), StatusCode::FOUND);
    assert_eq!(uploaded.headers()[header::LOCATION], "/admin/media?lang=en");
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
    // Incompressible pixels produce a valid PNG close to the upload cap.
    let mut state = 1_u32;
    let pixels = image::RgbImage::from_fn(1600, 1536, |_, _| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let [r, g, b, _] = state.to_le_bytes();
        image::Rgb([r, g, b])
    });
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(pixels)
        .write_to(&mut png, ImageFormat::Png)
        .unwrap();
    let png = png.into_inner();
    assert!((7 * 1024 * 1024..=MAX_BYTES).contains(&png.len()));
    let mut request = make_with_image(csrf, "Generic MIME", "application/octet-stream", &png);
    request
        .headers_mut()
        .insert("x-csrf-token", csrf.parse().unwrap());
    let body = to_bytes(
        std::mem::replace(request.body_mut(), Body::empty()),
        MAX_BYTES + 16384,
    )
    .await
    .unwrap();
    // Release each 10 Mbps chunk on a clock tick over roughly six seconds.
    let period = Duration::from_secs(1);
    let ticks = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
    *request.body_mut() = Body::from_stream(futures_util::stream::unfold(
        (body, ticks),
        |(mut remaining, mut ticks)| async move {
            if remaining.is_empty() {
                return None;
            }
            ticks.tick().await;
            let chunk = remaining.split_to(remaining.len().min(1_250_000));
            Some((Ok::<_, std::convert::Infallible>(chunk), (remaining, ticks)))
        },
    ));
    let generic = app.clone().oneshot(request).await.unwrap();
    assert_eq!(generic.status(), StatusCode::FOUND);
    let mime: String =
        sqlx::query_scalar("SELECT mime_type FROM media WHERE alt_text='Generic MIME'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(mime, "image/png");
    sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) SELECT $1,'editorial_media.uploaded','test','null'::jsonb,'null'::jsonb,uuidv7() FROM generate_series(1,8)").bind(actor).execute(pool).await.unwrap();
    assert_eq!(
        app.clone()
            .oneshot(make(csrf, "Image", "image/png"))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );

    let response = app
        .oneshot(
            Request::post(format!("/admin/media/{id}/delete?lang=en"))
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/admin/media?lang=en");
    assert_eq!(
        read(pool, id, false).await.err(),
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
        let draft=crate::events::Draft { starts_at:now+time::Duration::hours(1),ends_at:now+time::Duration::hours(2),featured:false,en:crate::events::Translation { title:"Parallel event".into(),slug:"parallel-event".into(),summary:"Editorial save during image processing".into(),body:"Editorial save during image processing".into() },pl:None };
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
