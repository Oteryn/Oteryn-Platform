//! Private editorial images. Multipart callers send the browser CSRF token in
//! X-CSRF-Token. Untrusted names and MIME headers never select stored paths.
use axum::{
    Extension, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::Multipart;
use image::{GenericImageView, ImageFormat, ImageReader};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, Permission},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::io::Cursor;
use uuid::Uuid;
const MAX_BYTES: usize = 8 * 1024 * 1024;
fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}
/// Normalized image and optional same-format thumbnail for downstream consumers.
#[derive(Debug)]
pub struct ProcessedImage {
    pub content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
}
/// Decode only verified containers with bounded dimensions and allocation.
pub fn process(name: &str, bytes: &[u8]) -> Result<ProcessedImage, StatusCode> {
    let bad = StatusCode::UNPROCESSABLE_ENTITY;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(bad);
    }
    let extension = name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let (format, mime) = match extension.as_str() {
        "jpg" | "jpeg" => (ImageFormat::Jpeg, "image/jpeg"),
        "png" => (ImageFormat::Png, "image/png"),
        "webp" => (ImageFormat::WebP, "image/webp"),
        _ => return Err(bad),
    };
    if image::guess_format(bytes).map_err(invalid)? != format {
        return Err(bad);
    }
    let boundary = match format {
        ImageFormat::Jpeg => bytes.ends_with(&[255, 217]),
        ImageFormat::Png => bytes.ends_with(b"\0\0\0\0IEND\xaeB`\x82"),
        ImageFormat::WebP => bytes
            .get(4..8)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .is_some_and(|b| u32::from_le_bytes(b) as u64 + 8 == bytes.len() as u64),
        _ => false,
    };
    if !boundary {
        return Err(bad);
    }
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(invalid)?;
    if width == 0
        || height == 0
        || width > 6000
        || height > 6000
        || u64::from(width) * u64::from(height) > 20_000_000
    {
        return Err(bad);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(6000);
    limits.max_image_height = Some(6000);
    limits.max_alloc = Some(160_000_000);
    reader.limits(limits);
    let decoded = reader.decode().map_err(invalid)?;
    if decoded.dimensions() != (width, height) {
        return Err(bad);
    }
    let encode = |image: &image::DynamicImage| -> Result<Vec<u8>, StatusCode> {
        let mut output = Cursor::new(Vec::new());
        if format == ImageFormat::Jpeg {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 88)
                .encode_image(image)
                .map_err(invalid)?;
        } else {
            image.write_to(&mut output, format).map_err(invalid)?;
        }
        let bytes = output.into_inner();
        if bytes.len() > MAX_BYTES {
            return Err(bad);
        }
        Ok(bytes)
    };
    let content = encode(&decoded)?;
    let thumbnail = if width > 320 || height > 320 {
        Some(encode(&decoded.thumbnail(320, 320))?)
    } else {
        None
    };
    Ok(ProcessedImage {
        content,
        thumbnail,
        mime,
        width,
        height,
    })
}
/// Routes must be wrapped in the staff guard before mounting.
pub fn admin_router(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/media", get(index).post(upload))
        .route("/admin/media/{id}/content", get(content))
        .route("/admin/media/{id}/thumbnail", get(thumbnail))
        .route("/admin/media/{id}", axum::routing::delete(delete))
        .route("/admin/media/{id}/delete", axum::routing::post(delete))
        .with_state(pool)
        .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(oteryn_admin::web::configured_guard(
        admin_router(pool.clone()),
        pool,
        Permission::ManageMedia,
    )?
    .layer(axum::middleware::from_fn(multipart_csrf))
    .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
    .layer(axum::middleware::from_fn(headers)))
}
/// Adapt a native multipart form token to the shared synchronizer-token verifier.
/// The shared guard still validates the origin, session and constant-time token.
async fn multipart_csrf(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    use axum::extract::FromRequest;
    if request.method() == axum::http::Method::POST
        && request.uri().path() == "/admin/media"
        && !request.headers().contains_key("x-csrf-token")
    {
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .cloned()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let body = std::mem::replace(request.body_mut(), axum::body::Body::empty());
        let bytes = axum::body::to_bytes(body, MAX_BYTES + 16384)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let mut copy = axum::http::Request::new(axum::body::Body::from(bytes.clone()));
        copy.extensions_mut().extend(request.extensions().clone());
        copy.headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
        let mut multipart = Multipart::from_request(copy, &()).await.map_err(invalid)?;
        let mut token = None;
        while let Some(field) = multipart.next_field().await.map_err(invalid)? {
            if field.name() == Some("_csrf") {
                if token.is_some() {
                    return Err(StatusCode::FORBIDDEN);
                }
                token = Some(field.text().await.map_err(invalid)?);
            }
        }
        if let Some(token) = token {
            request.headers_mut().insert(
                "x-csrf-token",
                token.parse().map_err(|_| StatusCode::FORBIDDEN)?,
            );
        }
        *request.body_mut() = axum::body::Body::from(bytes);
    }
    Ok(next.run(request).await)
}
#[derive(serde::Deserialize, Default)]
struct Options {
    lang: Option<String>,
}
#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\" class=\"op-theme\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"><title>{{ title }}</title><main class=\"op-stack\"><h1>{{ title }}</h1><form class=\"op-stack\" method=\"post\" action=\"/admin/media\" enctype=\"multipart/form-data\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><label for=\"image\">{{ image }}</label><input class=\"op-input\" id=\"image\" name=\"image\" type=\"file\" accept=\"image/jpeg,image/png,image/webp\" required><label for=\"alt\">{{ alt }}</label><textarea class=\"op-input\" id=\"alt\" name=\"alt_text\" maxlength=\"500\" required></textarea><button class=\"op-button\">{{ upload }}</button></form><ul>{% for item in items %}<li><img src=\"/admin/media/{{ item.0 }}/thumbnail\" width=\"160\" alt=\"{{ item.1 }}\"><p>{{ item.1 }}</p><form method=\"post\" action=\"/admin/media/{{ item.0 }}/delete\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><button class=\"op-button\">{{ delete }}</button></form></li>{% endfor %}</ul></main></html>",
    ext = "html"
)]
struct MediaPage {
    lang: &'static str,
    title: String,
    image: String,
    alt: String,
    upload: String,
    delete: String,
    csrf: String,
    items: Vec<(Uuid, String)>,
}
fn label(lang: &str, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = if lang == "en" {
        "title = Media\nimage = Image (JPEG, PNG, WebP; max 8 MiB)\nalt = Alternative text\nupload = Upload\ndelete = Delete"
    } else {
        "title = Media\nimage = Obraz (JPEG, PNG, WebP; maks. 8 MiB)\nalt = Tekst alternatywny\nupload = Prześlij\ndelete = Usuń"
    };
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    let Ok(resource) = FluentResource::try_new(source.into()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
async fn index(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Query(options): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    use askama::Template;
    let lang = if options.lang.as_deref() == Some("en") {
        "en"
    } else {
        "pl"
    };
    let rows =
        sqlx::query("SELECT id,alt_text FROM media ORDER BY created_at DESC,id DESC LIMIT 100")
            .fetch_all(&pool)
            .await
            .map_err(unavailable)?;
    let page = MediaPage {
        lang,
        title: label(lang, "title"),
        image: label(lang, "image"),
        alt: label(lang, "alt"),
        upload: label(lang, "upload"),
        delete: label(lang, "delete"),
        csrf: session.csrf_token().await.map_err(unavailable)?,
        items: rows
            .iter()
            .map(|r| (r.get("id"), r.get("alt_text")))
            .collect(),
    };
    Ok(Html(page.render().map_err(unavailable)?))
}
async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: CheckContext,
    id: Uuid,
    action: &str,
) -> Result<(), StatusCode> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before: json!(null),
            after: json!({"id":id}),
        },
    )
    .await
    .map_err(unavailable)?;
    Ok(())
}
async fn upload(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    mut multipart: Multipart,
) -> Result<Response, StatusCode> {
    let mut image = None;
    let mut alt = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid)? {
        match field.name() {
            Some("image") if image.is_none() => {
                let name = field
                    .file_name()
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .to_owned();
                let bytes = field.bytes().await.map_err(invalid)?;
                image = Some((name, bytes));
            }
            Some("_csrf") => {
                let _ = field.text().await.map_err(invalid)?;
            }
            Some("alt_text") if alt.is_none() => {
                alt = Some(field.text().await.map_err(invalid)?);
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    let alt = alt.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    if alt.trim().is_empty() || alt.chars().count() > 500 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (name, bytes) = image.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    // CPU-heavy decoding does not block the async runtime.
    let mut tx = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))")
        .bind(context.account.to_string())
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE actor=$1 AND action='editorial_media.uploaded' AND created_at > now()-interval '1 minute'").bind(context.account).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if recent >= 10 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let processed = tokio_process(name, bytes.to_vec()).await?;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256,thumbnail,thumbnail_sha256) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(id).bind(alt.trim()).bind(processed.mime).bind(processed.width as i32).bind(processed.height as i32)
        .bind(&processed.content).bind(Sha256::digest(&processed.content).to_vec()).bind(&processed.thumbnail)
        .bind(processed.thumbnail.as_ref().map(|b| Sha256::digest(b).to_vec())).execute(&mut *tx).await.map_err(unavailable)?;
    record(&mut tx, context, id, "editorial_media.uploaded").await?;
    tx.commit().await.map_err(unavailable)?;
    Ok((StatusCode::FOUND, [(header::LOCATION, "/admin/media")]).into_response())
}
async fn tokio_process(name: String, bytes: Vec<u8>) -> Result<ProcessedImage, StatusCode> {
    static DECODERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let permit = DECODERS
        .try_acquire()
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        process(&name, &bytes)
    })
    .await
    .map_err(unavailable)?
}
async fn content(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    read(&pool, id, false).await
}
async fn thumbnail(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    read(&pool, id, true).await
}
async fn read(pool: &PgPool, id: Uuid, thumbnail: bool) -> Result<Response, StatusCode> {
    let row = sqlx::query(
        "SELECT mime_type,content,sha256,thumbnail,thumbnail_sha256 FROM media WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let variant: Option<Vec<u8>> = row.get("thumbnail");
    let (bytes, hash): (Vec<u8>, Vec<u8>) = if thumbnail && variant.is_some() {
        (
            variant.ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
            row.get::<Option<Vec<u8>>, _>("thumbnail_sha256")
                .ok_or(StatusCode::SERVICE_UNAVAILABLE)?,
        )
    } else {
        (row.get("content"), row.get("sha256"))
    };
    if Sha256::digest(&bytes).to_vec() != hash {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok((
        [
            (header::CONTENT_TYPE, row.get::<String, _>("mime_type")),
            (header::CACHE_CONTROL, "private, no-store".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            (header::CONTENT_DISPOSITION, "inline".into()),
        ],
        bytes,
    )
        .into_response())
}
async fn delete(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let row = sqlx::query(
        "SELECT content,sha256,thumbnail,thumbnail_sha256 FROM media WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let content: Vec<u8> = row.get("content");
    let hash: Vec<u8> = row.get("sha256");
    let thumbnail: Option<Vec<u8>> = row.get("thumbnail");
    let thumbnail_hash: Option<Vec<u8>> = row.get("thumbnail_sha256");
    if Sha256::digest(&content).to_vec() != hash
        || thumbnail.as_ref().map(|b| Sha256::digest(b).to_vec()) != thumbnail_hash
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let result = sqlx::query("DELETE FROM media WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error().is_some_and(|e| {
                e.is_foreign_key_violation() || e.code().as_deref() == Some("23001")
            }) {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                unavailable(e)
            }
        })?;
    if result.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }
    record(&mut tx, context, id, "editorial_media.deleted").await?;
    tx.commit().await.map_err(unavailable)?;
    Ok((StatusCode::FOUND, [(header::LOCATION, "/admin/media")]).into_response())
}
#[cfg(test)]
#[path = "media_tests.rs"]
mod media_tests;

async fn headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "private, no-store"),
        ("x-content-type-options", "nosniff"),
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("referrer-policy", "same-origin"),
        ("strict-transport-security", "max-age=31536000"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
