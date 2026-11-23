#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::http::Method;
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
fn jpeg_ends_at_first_parsed_eoi() {
    let jpeg = fixture(ImageFormat::Jpeg, 64, 64);
    let mut with_comment = jpeg[..2].to_vec();
    with_comment.extend_from_slice(b"\xff\xfe\x00\x04\xff\xd9");
    with_comment.extend_from_slice(&jpeg[2..]);
    assert!(process("comment.jpeg", &with_comment).is_ok());

    let two_images = [jpeg.as_slice(), jpeg.as_slice()].concat();
    assert_eq!(
        process("two-images.jpeg", &two_images).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn progressive_jpeg_requires_initial_dc_for_every_component() {
    use oxiarc_jpeg::{
        EncodeOptions, EncodeProcess, InputColor, ScanSpec, encode_to_vec_with_options,
    };

    let options = EncodeOptions {
        process: EncodeProcess::Progressive,
        progressive_script: Some(vec![
            ScanSpec::dc(vec![0, 1], 0, 0),
            ScanSpec::dc(vec![2], 0, 0),
        ]),
        ..EncodeOptions::default()
    };
    let mut jpeg =
        encode_to_vec_with_options(&[96; 16 * 16 * 3], 16, 16, InputColor::Rgb, &options).unwrap();
    assert!(process("complete.jpeg", &jpeg).is_ok());

    // Omit the final scan, which carries the third component's initial DC data.
    let last_scan = jpeg
        .windows(2)
        .rposition(|pair| pair == [0xff, 0xda])
        .unwrap();
    jpeg.truncate(last_scan);
    jpeg.extend_from_slice(&[0xff, 0xd9]);
    assert_eq!(
        process("missing-chroma.jpeg", &jpeg).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn progressive_jpeg_requires_ordered_scans_and_refinement() {
    use oxiarc_jpeg::{
        EncodeOptions, EncodeProcess, InputColor, ScanSpec, encode_to_vec_with_options,
    };
    let options = EncodeOptions {
        process: EncodeProcess::Progressive,
        progressive_script: Some(vec![
            ScanSpec::dc(vec![0], 0, 0),
            ScanSpec::ac(0, 1, 63, 0, 1),
            ScanSpec::ac(0, 1, 63, 1, 0),
        ]),
        ..EncodeOptions::default()
    };
    let jpeg =
        encode_to_vec_with_options(&[96; 16 * 16], 16, 16, InputColor::Luma, &options).unwrap();
    assert!(process("ordered.jpeg", &jpeg).is_ok());
    let spectral_offsets: Vec<_> = jpeg
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| (pair == [0xff, 0xda]).then_some(index + 7))
        .collect();
    let &[dc, ac, refinement] = spectral_offsets.as_slice() else {
        panic!("expected three scans")
    };

    let mut reordered = jpeg.clone();
    reordered[dc..dc + 3].copy_from_slice(&jpeg[ac..ac + 3]);
    reordered[ac..ac + 3].copy_from_slice(&jpeg[dc..dc + 3]);
    assert!(verify_jpeg_structure(&reordered).is_none());
    assert_eq!(
        process("ac-first.jpeg", &reordered).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );

    for invalid_approximation in [0x01, 0x21, 0x20] {
        let mut malformed = jpeg.clone();
        malformed[refinement + 2] = invalid_approximation;
        assert!(verify_jpeg_structure(&malformed).is_none());
    }
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
fn png_rejects_bytes_after_first_iend_even_with_terminal_iend() {
    let mut png = fixture(ImageFormat::Png, 32, 24);
    assert!(process("single.png", &png).is_ok());
    png.extend_from_within(png.len() - 12..);
    assert_eq!(
        process("repeat-end.png", &png).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn png_checks_the_crc_of_ancillary_chunks_after_pixel_data() {
    let mut png = fixture(ImageFormat::Png, 32, 24);
    let iend = png.split_off(png.len() - 12);
    let chunk = b"tEXtComment\0text";
    png.extend_from_slice(&(chunk.len() as u32 - 4).to_be_bytes());
    png.extend_from_slice(chunk);
    let crc_offset = png.len();
    png.extend_from_slice(&crc32fast::hash(chunk).to_be_bytes());
    png.extend_from_slice(&iend);
    assert!(process("ancillary.png", &png).is_ok());
    png[crc_offset] ^= 1;
    assert_eq!(
        process("ancillary.png", &png).err(),
        Some(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[test]
fn animated_uploads_are_rejected_even_when_the_first_frame_decodes() {
    for (name, format, bytes) in [
        (
            "animated-damaged.png",
            ImageFormat::Png,
            include_bytes!("../../../e2e/fixtures/uploads/animated-damaged.png").as_slice(),
        ),
        (
            "animated-damaged.webp",
            ImageFormat::WebP,
            include_bytes!("../../../e2e/fixtures/uploads/animated-damaged.webp").as_slice(),
        ),
    ] {
        let first_frame = ImageReader::with_format(Cursor::new(bytes), format)
            .decode()
            .unwrap();
        assert_eq!(first_frame.dimensions(), (16, 16));
        assert_eq!(
            process(name, bytes).err(),
            Some(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
}

#[test]
fn webp_requires_complete_trailing_chunks_and_padding() {
    fn append_chunk(mut webp: Vec<u8>, trailing: &[u8]) -> Vec<u8> {
        webp.extend_from_slice(trailing);
        let size = (webp.len() - 8) as u32;
        webp[4..8].copy_from_slice(&size.to_le_bytes());
        webp
    }
    let webp = fixture(ImageFormat::WebP, 32, 24);
    let complete = append_chunk(webp.clone(), b"TEST\x01\0\0\0a\0");
    assert!(process("complete.webp", &complete).is_ok());
    for trailing in [b"TEST\x08\0\0\0a".as_slice(), b"TEST\x01\0\0\0a"] {
        assert_eq!(
            process("truncated.webp", &append_chunk(webp.clone(), trailing)).err(),
            Some(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
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
        .max_connections(3)
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
            Method::POST,
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
            Method::POST,
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
            Method::POST,
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
    let staff = oteryn_admin::web::configured_staff_preflight_from(
        pool.clone(),
        Permission::ManageMedia,
        &|key| match key {
            "OTERYN_ADMIN_PANEL" => Some("true".into()),
            "OTERYN_ADMIN_HOST" => Some("staff.test".into()),
            _ => None,
        },
    )
    .unwrap();
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
    .layer(axum::middleware::from_fn_with_state(staff, multipart_csrf))
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
    for language in [oteryn_web::Language::Pl, oteryn_web::Language::En] {
        let mut upload = make(csrf, " ", "image/png");
        upload
            .headers_mut()
            .insert(header::ACCEPT_LANGUAGE, language.code().parse().unwrap());
        let refused = app.clone().oneshot(upload).await.unwrap();
        assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            to_bytes(refused.into_body(), 1024).await.unwrap().as_ref(),
            language.text("error-422-description").as_bytes()
        );

        for (method, path) in [
            (Method::POST, "/admin/media/invalid/delete"),
            (Method::DELETE, "/admin/media/invalid"),
        ] {
            let refused = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header(header::ACCEPT_LANGUAGE, language.code())
                        .header("host", "staff.test")
                        .header("origin", "https://staff.test")
                        .header("cookie", &cookie)
                        .header("x-csrf-token", csrf)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                to_bytes(refused.into_body(), 1024).await.unwrap().as_ref(),
                language.text("error-400-description").as_bytes()
            );
        }
    }
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
    async fn with_stalled_body(mut request: Request<Body>) -> Request<Body> {
        use futures_util::StreamExt;
        let body = to_bytes(
            std::mem::replace(request.body_mut(), Body::empty()),
            MAX_BYTES + 16384,
        )
        .await
        .unwrap();
        // Deliver a partial frame and keep the connection open without EOF.
        let first_frame = Ok::<_, std::convert::Infallible>(body.slice(..body.len() / 2));
        *request.body_mut() = Body::from_stream(
            futures_util::stream::iter([first_frame]).chain(futures_util::stream::pending()),
        );
        request
    }

    let mut stalled = with_stalled_body(make(csrf, "Stalled upload", "image/png")).await;
    stalled
        .headers_mut()
        .insert("x-csrf-token", csrf.parse().unwrap());
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

    // Use a durable signed-in session and the production router for the form
    // timeout. No CSRF header is supplied; its token is inside the body.
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'editor-test-password','active','en',now())")
        .bind(actor)
        .bind(format!("{actor}@example.invalid"))
        .execute(pool)
        .await
        .unwrap();
    crate::test_staff::browser(pool, actor, &cookie)
        .await
        .unwrap();
    let test_mfa_key = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let lookup = |key: &str| match key {
        "OTERYN_ADMIN_PANEL" => Some("true".into()),
        "OTERYN_ADMIN_HOST" => Some("staff.test".into()),
        "OTERYN_MFA_KEY" => Some(test_mfa_key.clone()),
        _ => None,
    };
    let app = configured_router_from(pool.clone(), &lookup)
        .unwrap()
        .layer(axum::middleware::from_fn_with_state(
            oteryn_web::chrome::Shell {
                registry: oteryn_ui::Registry::default(),
                sessions: oteryn_web::session::SessionLayer::new(pool.clone()),
            },
            oteryn_web::chrome::middleware,
        ));
    let form_page = app
        .clone()
        .oneshot(
            Request::get("/admin/media?lang=en")
                .header("host", "staff.test")
                .header(header::ACCEPT_LANGUAGE, "pl")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(form_page.status(), StatusCode::OK);
    let html = to_bytes(form_page.into_body(), 100_000).await.unwrap();
    let html = std::str::from_utf8(&html).unwrap();
    assert!(html.contains("Alternative text"));
    assert!(html.contains(r#"aria-label="Main navigation""#));
    assert!(html.contains(r#"aria-label="Footer navigation""#));
    let upload_key = html
        .split("name=\"upload_key\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    // A real lock prevents the preflight from reading the session row. The
    // short budget must include that lookup, and origin checks precede it.
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE web_sessions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let mut foreign_origin = with_stalled_body(make(csrf, "Foreign origin", "image/png")).await;
    foreign_origin
        .headers_mut()
        .insert(header::ORIGIN, "https://other.test".parse().unwrap());
    let refusal = tokio::time::timeout(Duration::from_secs(2), app.clone().oneshot(foreign_origin))
        .await
        .expect("origin must be checked before any session lookup, including in the shared shell")
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);

    let blocked_one = with_stalled_body(make(csrf, "Blocked lookup", "image/png")).await;
    let blocked_two = with_stalled_body(make(csrf, "Second blocked lookup", "image/png")).await;
    let (first, second) = tokio::time::timeout(Duration::from_secs(8), async {
        tokio::join!(
            app.clone().oneshot(blocked_one),
            app.clone().oneshot(blocked_two)
        )
    })
    .await
    .expect("preflight must bound both session lookups");
    for refusal in [first.unwrap(), second.unwrap()] {
        assert_eq!(refusal.status(), StatusCode::REQUEST_TIMEOUT);
    }
    let unrelated = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pool),
    )
    .await
    .expect("the pool must serve unrelated queries before the session lock is released")
    .unwrap();
    assert_eq!(unrelated, 1);
    lock.rollback().await.unwrap();

    let alt = "Signed-in native form stalled";
    let mut stalled = with_stalled_body(make(csrf, alt, "image/png")).await;
    assert!(!stalled.headers().contains_key("x-csrf-token"));
    stalled
        .headers_mut()
        .insert(header::ACCEPT, "text/html".parse().unwrap());
    let start = tokio::time::Instant::now();
    let refusal = tokio::time::timeout(Duration::from_secs(70), app.clone().oneshot(stalled))
        .await
        .expect("the signed-in native form must time out within the test budget")
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(start.elapsed() >= Duration::from_secs(60));
    assert_eq!(refusal.headers()[header::CONTENT_LANGUAGE], "en");
    let body = to_bytes(refusal.into_body(), 100_000).await.unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    assert!(body.contains("<h1>Upload timed out</h1>"));
    assert!(body.contains(r#"href="/admin/media?lang=en">Back to the media form</a>"#));
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM media WHERE alt_text=$1)")
            .bind(alt)
            .fetch_one(pool)
            .await
            .unwrap()
    );

    // The cutoff above also gives the earlier quota fixtures time to age out.
    let retry = |alt| {
        let mut request = make(csrf, alt, "image/png");
        request
            .headers_mut()
            .insert("idempotency-key", upload_key.parse().unwrap());
        request
    };
    let (first, second) = tokio::join!(
        app.clone().oneshot(retry("Concurrent replay")),
        app.clone().oneshot(retry("Concurrent replay")),
    );
    for response in [first.unwrap(), second.unwrap()] {
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()[header::LOCATION], "/admin/media?lang=en");
    }
    let uploads: i64 =
        sqlx::query_scalar("SELECT count(*) FROM media WHERE alt_text='Concurrent replay'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(uploads, 1);
    let audit_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log a JOIN media m ON m.id::text=a.target WHERE m.alt_text='Concurrent replay' AND a.action='editorial_media.uploaded'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(audit_rows, 1);
    let conflict = app
        .clone()
        .oneshot(retry("Changed replay payload"))
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM media WHERE alt_text='Changed replay payload')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );

    // A kick advances the account generation without removing the browser cookie or MFA grant.
    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(actor)
    .execute(pool)
    .await
    .unwrap();
    let stale = with_stalled_body(make(csrf, "Kicked editor stalled upload", "image/png")).await;
    let refusal = tokio::time::timeout(Duration::from_secs(8), app.clone().oneshot(stale))
        .await
        .expect("a stale generation must keep the short deadline")
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM media WHERE alt_text='Kicked editor stalled upload')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );
    let denied = app
        .clone()
        .oneshot(retry("Concurrent replay"))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE web_sessions SET authorization_generation=(SELECT native_security_generation FROM accounts WHERE id=$1) WHERE account_id=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("UPDATE web_sessions SET data=data - 'mfa_grant' WHERE account_id=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
    let missing_mfa = with_stalled_body(make(csrf, "MFA missing", "image/png")).await;
    let refusal = tokio::time::timeout(Duration::from_secs(8), app.clone().oneshot(missing_mfa))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);

    let support = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'support-test-password','active','en',now())")
        .bind(support)
        .bind(format!("{support}@example.invalid"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(support)
    .execute(pool)
    .await
    .unwrap();
    crate::test_staff::browser(pool, support, &cookie)
        .await
        .unwrap();
    let unauthorized = with_stalled_body(make(csrf, "Support stalled upload", "image/png")).await;
    let refusal = tokio::time::timeout(Duration::from_secs(8), app.clone().oneshot(unauthorized))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
    let refusal_audits = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='admin.access.denied' AND target='/admin/media'",
        )
        .bind(support)
        .fetch_one(pool)
    };
    let unsigned = app
        .clone()
        .oneshot(make("", "Unsigned support upload", "image/png"))
        .await
        .unwrap();
    assert_eq!(unsigned.status(), StatusCode::FORBIDDEN);
    assert_eq!(refusal_audits().await.unwrap(), 0);
    let signed = app
        .oneshot(make(csrf, "Signed support upload", "image/png"))
        .await
        .unwrap();
    assert_eq!(signed.status(), StatusCode::FORBIDDEN);
    assert_eq!(refusal_audits().await.unwrap(), 1);
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
