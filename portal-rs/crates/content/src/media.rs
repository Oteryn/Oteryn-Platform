//! Private editorial images. Multipart callers send the browser CSRF token in
//! X-CSRF-Token. Untrusted names and MIME headers never select stored paths.
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::FormRejection},
    http::{Method, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::Multipart;
use fluent_bundle::{FluentBundle, FluentResource};
use image::{GenericImageView, ImageFormat, ImageReader};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use oteryn_web::ratelimit::{PublicGameData, RateLimit};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::io::Cursor;
use uuid::Uuid;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const REFERENCE_PREVIEW: i64 = 5;
static DECODERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
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
        .route(
            "/admin/media/{id}",
            axum::routing::delete(delete).post(update_alt),
        )
        .route("/admin/media/{id}/delete", axum::routing::post(delete))
        .with_state(pool)
        .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    configured_router_from(pool, &|key| std::env::var(key).ok())
}
fn configured_router_from(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    oteryn_admin::web::configured_host_boundary_from(
        oteryn_admin::web::configured_guard_from(
            admin_router(pool.clone()),
            pool,
            Permission::ManageMedia,
            lookup,
        )?
        .layer(axum::middleware::from_fn(multipart_csrf))
        .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
        .layer(axum::middleware::from_fn(headers)),
        lookup,
    )
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
        let bytes = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            axum::body::to_bytes(body, MAX_BYTES + 16384),
        )
        .await
        .map_err(|_| StatusCode::REQUEST_TIMEOUT)?
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
    #[serde(default)]
    page: u32,
    lang: Option<String>,
    #[serde(default)]
    q: String,
}

struct Asset {
    id: Uuid,
    alt: String,
    name: Option<String>,
    references: Vec<Reference>,
    news_references: i64,
    other_references: i64,
    revision: String,
}

struct Reference {
    id: Uuid,
    title: String,
}

#[derive(askama::Template)]
#[template(path = "content/media.html")]
struct MediaPage {
    next: u64,
    previous: Option<u32>,
    more: bool,
    lang: &'static str,
    query: String,
    messages: FluentBundle<FluentResource>,
    csrf: String,
    items: Vec<Asset>,
    can_manage_news: bool,
}

impl MediaPage {
    fn text(&self, key: &str) -> String {
        label(&self.messages, key)
    }

    fn has_more_references(&self, asset: &Asset) -> bool {
        asset.news_references > REFERENCE_PREVIEW
    }
}

pub(crate) fn messages(lang: &str) -> Result<FluentBundle<FluentResource>, StatusCode> {
    let source = if lang == "en" {
        include_str!("../locales/en.ftl")
    } else {
        include_str!("../locales/pl.ftl")
    };
    let locale = lang.parse().map_err(unavailable)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    let resource =
        FluentResource::try_new(source.to_owned()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(bundle)
}

pub(crate) fn label(bundle: &FluentBundle<FluentResource>, key: &str) -> String {
    bundle
        .get_message(key)
        .and_then(|message| message.value())
        .map(|pattern| {
            bundle
                .format_pattern(pattern, None, &mut vec![])
                .into_owned()
        })
        .unwrap_or_default()
}

fn locale(lang: Option<&str>) -> &'static str {
    if lang == Some("en") { "en" } else { "pl" }
}

async fn index(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    Query(options): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    use askama::Template;
    let lang = locale(options.lang.as_deref());
    let pagination = crate::Pagination { page: options.page };
    let query = options.q.trim();
    if query.chars().count() > 255
        || query.chars().any(char::is_control)
        || pagination.page > 100_000
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let can_manage_news = can_manage(&pool, &auth, context, Permission::ManageNews).await?;
    let rows = sqlx::query(
        "SELECT m.id, m.alt_text, m.original_name, m.updated_at::text AS revision,
                (SELECT count(*) FROM news WHERE media_id=m.id) AS news_references,
                (SELECT count(*) FROM media_references WHERE media_id=m.id) AS other_references
         FROM media m
         WHERE $1='' OR strpos(lower(m.alt_text),lower($1))>0
            OR strpos(lower(m.original_name),lower($1))>0 OR m.id::text=lower($1)
         ORDER BY m.created_at DESC, m.id DESC LIMIT 101 OFFSET $2",
    )
    .bind(query)
    .bind(pagination.offset(100))
    .fetch_all(&pool)
    .await
    .map_err(unavailable)?;
    let mut items: Vec<_> = rows
        .iter()
        .take(100)
        .map(|row| Asset {
            id: row.get("id"),
            alt: row.get("alt_text"),
            name: row.get("original_name"),
            references: Vec::new(),
            news_references: row.get("news_references"),
            other_references: row.get("other_references"),
            revision: row.get("revision"),
        })
        .collect();
    if can_manage_news {
        let ids: Vec<_> = items.iter().map(|item| item.id).collect();
        let references = sqlx::query(
            "SELECT asset.id AS media_id, post.id, post.title
             FROM unnest($1::uuid[]) AS asset(id)
             CROSS JOIN LATERAL (
                 SELECT id, CASE WHEN $2='pl' THEN coalesce(title_pl,title) ELSE title END AS title
                 FROM news WHERE media_id=asset.id ORDER BY updated_at DESC,id DESC LIMIT $3
             ) AS post",
        )
        .bind(&ids)
        .bind(lang)
        .bind(REFERENCE_PREVIEW)
        .fetch_all(&pool)
        .await
        .map_err(unavailable)?;
        let mut by_asset = std::collections::HashMap::<Uuid, Vec<Reference>>::new();
        for row in references {
            by_asset
                .entry(row.get("media_id"))
                .or_default()
                .push(Reference {
                    id: row.get("id"),
                    title: row.get("title"),
                });
        }
        for item in &mut items {
            item.references = by_asset.remove(&item.id).unwrap_or_default();
        }
    }
    let csrf = session.csrf_token().await.map_err(unavailable)?;
    let page = MediaPage {
        next: pagination.next(),
        previous: pagination.page.checked_sub(1),
        more: rows.len() > 100 && pagination.page < 100_000,
        lang,
        query: query.to_owned(),
        messages: messages(lang)?,
        csrf,
        items,
        can_manage_news,
    };
    Ok(Html(page.render().map_err(unavailable)?))
}

pub(crate) async fn can_manage(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    permission: Permission,
) -> Result<bool, StatusCode> {
    let mut connection = pool.acquire().await.map_err(unavailable)?;
    match auth.check(&mut connection, context, permission).await {
        Ok(()) => Ok(true),
        Err(oteryn_admin::rbac::Error::Denied) => Ok(false),
        Err(error) => Err(authorization_status(error)),
    }
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
    Extension(auth): Extension<MutationAuthorization>,
    mut multipart: Multipart,
) -> Result<Response, StatusCode> {
    let mut image = None;
    let mut alt = None;
    let mut lang = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid)? {
        match field.name() {
            Some("image") if image.is_none() => {
                let name = field
                    .file_name()
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .to_owned();
                let mime = field.content_type().map(str::to_owned);
                validate_upload_metadata(&name, mime.as_deref())?;
                let bytes = field.bytes().await.map_err(invalid)?;
                image = Some((name, bytes));
            }
            Some("_csrf") => {
                let _ = field.text().await.map_err(invalid)?;
            }
            Some("lang") if lang.is_none() => {
                let value = field.text().await.map_err(invalid)?;
                if !matches!(value.as_str(), "en" | "pl") {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                lang = Some(value);
            }
            Some("alt_text") if alt.is_none() => {
                alt = Some(field.text().await.map_err(invalid)?);
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    let alt = validated_alt(alt.as_deref().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?)?;
    let (name, bytes) = image.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let processed = process_upload(name, bytes.to_vec()).await?;
    // Persist only after processing, with fresh transactional authorization.
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))")
                    .bind(context.account.to_string())
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &[Permission::Access, Permission::ManageMedia],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))")
        .bind(context.account.to_string())
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    auth.check_in(&mut tx, context, Permission::ManageMedia)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
    let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE actor=$1 AND action='editorial_media.uploaded' AND created_at > now()-interval '1 minute'").bind(context.account).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if recent >= 10 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256,thumbnail,thumbnail_sha256,original_name) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(id).bind(alt).bind(processed.mime).bind(processed.width as i32).bind(processed.height as i32)
        .bind(&processed.content).bind(Sha256::digest(&processed.content).to_vec()).bind(&processed.thumbnail)
        .bind(processed.thumbnail.as_ref().map(|b| Sha256::digest(b).to_vec())).bind(name).execute(&mut *tx).await.map_err(unavailable)?;
    record(&mut tx, context, id, "editorial_media.uploaded").await?;
    crate::revalidate(
        &auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManageMedia],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    tx.commit().await.map_err(unavailable)?;
    Ok(library_redirect(lang.as_deref(), None))
}
#[derive(serde::Deserialize)]
struct AltForm {
    alt_text: String,
    revision: String,
    lang: String,
}

async fn update_alt(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Form(form): Form<AltForm>,
) -> Result<Response, StatusCode> {
    let alt = validated_alt(&form.alt_text)?;
    if !matches!(form.lang.as_str(), "en" | "pl") || form.revision.len() > 64 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let permissions = [Permission::Access, Permission::ManageMedia];
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM media WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &permissions,
    )
    .await
    .map_err(authorization_status)?;
    let (before, revision): (String, String) =
        sqlx::query_as("SELECT alt_text, updated_at::text FROM media WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::NOT_FOUND)?;
    if revision != form.revision {
        return Err(StatusCode::CONFLICT);
    }
    sqlx::query(
        "UPDATE media SET alt_text=$2, updated_at=greatest(clock_timestamp(),updated_at+interval '1 microsecond')
         WHERE id=$1",
    )
    .bind(id)
    .bind(alt)
    .execute(&mut *tx)
    .await
    .map_err(unavailable)?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "editorial_media.alt_updated",
            target: &id.to_string(),
            before: json!({"alt_text": before}),
            after: json!({"alt_text": alt}),
        },
    )
    .await
    .map_err(unavailable)?;
    crate::revalidate(&auth, &mut tx, context, &permissions)
        .await
        .map_err(authorization_status)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(library_redirect(Some(&form.lang), Some(id)))
}

fn validated_alt(alt: &str) -> Result<&str, StatusCode> {
    if alt.trim().is_empty() || alt.chars().count() > 500 || alt.contains('\0') {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(alt.trim())
}

fn library_redirect(lang: Option<&str>, asset: Option<Uuid>) -> Response {
    let target = match lang {
        Some("en") => "/admin/media?lang=en",
        Some("pl") => "/admin/media?lang=pl",
        _ => "/admin/media",
    };
    let target = match asset {
        Some(id) => format!("/admin/media?lang={}&q={id}", locale(lang)),
        None => target.to_owned(),
    };
    (StatusCode::FOUND, [(header::LOCATION, target)]).into_response()
}

fn validate_upload_metadata(name: &str, mime: Option<&str>) -> Result<(), StatusCode> {
    let error = StatusCode::UNPROCESSABLE_ENTITY;
    if name.chars().count() > 255
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(error);
    }
    let (stem, extension) = name.rsplit_once('.').ok_or(error)?;
    let expected = match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => return Err(error),
    };
    if stem.trim().is_empty()
        || mime.is_some_and(|mime| mime != "application/octet-stream" && mime != expected)
    {
        return Err(error);
    }
    Ok(())
}
/// Run the shared image pipeline outside the async runtime with bounded concurrency.
pub async fn process_upload(name: String, bytes: Vec<u8>) -> Result<ProcessedImage, StatusCode> {
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
    read(&pool, id, false, false).await
}
async fn thumbnail(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    read(&pool, id, true, false).await
}
pub(crate) async fn published_content(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    _: RateLimit<PublicGameData>,
) -> Result<Response, StatusCode> {
    read(&pool, id, true, true).await
}
async fn read(
    pool: &PgPool,
    id: Uuid,
    thumbnail: bool,
    published_only: bool,
) -> Result<Response, StatusCode> {
    let row = sqlx::query(
        "SELECT mime_type,
                CASE WHEN $3 AND thumbnail IS NOT NULL THEN thumbnail ELSE content END AS bytes,
                CASE WHEN $3 AND thumbnail IS NOT NULL THEN thumbnail_sha256 ELSE sha256 END AS hash
         FROM media
         WHERE id=$1 AND (NOT $2 OR EXISTS (
             SELECT 1 FROM news WHERE media_id=$1 AND published_at<=now()
               AND (expires_at IS NULL OR now() < expires_at)
         ))",
    )
    .bind(id)
    .bind(published_only)
    .bind(thumbnail)
    .fetch_optional(pool)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let bytes: Vec<u8> = row.try_get("bytes").map_err(unavailable)?;
    let hash: Vec<u8> = row.try_get("hash").map_err(unavailable)?;
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
#[derive(Default, serde::Deserialize)]
struct DeleteForm {
    replacement_id: Option<Uuid>,
    lang: Option<String>,
}

async fn delete(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    method: Method,
    form: Result<Form<DeleteForm>, FormRejection>,
) -> Result<Response, StatusCode> {
    let form = match form {
        Ok(Form(form)) => form,
        Err(FormRejection::InvalidFormContentType(_)) if method == Method::DELETE => {
            DeleteForm::default()
        }
        Err(error) => return Err(error.into_response().status()),
    };
    let replacement = form.replacement_id;
    if replacement == Some(id)
        || form
            .lang
            .as_deref()
            .is_some_and(|lang| !matches!(lang, "pl" | "en"))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut permissions = vec![Permission::Access, Permission::ManageMedia];
    if replacement.is_some() {
        permissions.push(Permission::ManageNews);
    }
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                // News writes take the asset lock before the post lock.
                sqlx::query("SELECT id FROM media WHERE id=$1 OR id=$2 ORDER BY id FOR UPDATE")
                    .bind(id)
                    .bind(replacement)
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SELECT id FROM news WHERE media_id=$1 ORDER BY id FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &permissions,
    )
    .await
    .map_err(authorization_status)?;
    verify_asset(&mut tx, id).await?;
    if let Some(replacement) = replacement {
        verify_asset(&mut tx, replacement).await.map_err(|status| {
            if status == StatusCode::NOT_FOUND {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                status
            }
        })?;
        replace_in_news(&mut tx, context, id, replacement).await?;
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
    crate::revalidate(&auth, &mut tx, context, &permissions)
        .await
        .map_err(authorization_status)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(library_redirect(form.lang.as_deref(), None))
}

async fn verify_asset(connection: &mut sqlx::PgConnection, id: Uuid) -> Result<(), StatusCode> {
    let row =
        sqlx::query("SELECT content,sha256,thumbnail,thumbnail_sha256 FROM media WHERE id=$1")
            .bind(id)
            .fetch_optional(connection)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::NOT_FOUND)?;
    let content: Vec<u8> = row.get("content");
    let hash: Vec<u8> = row.get("sha256");
    let thumbnail: Option<Vec<u8>> = row.get("thumbnail");
    let thumbnail_hash: Option<Vec<u8>> = row.get("thumbnail_sha256");
    if Sha256::digest(&content).to_vec() != hash
        || thumbnail
            .as_ref()
            .map(|bytes| Sha256::digest(bytes).to_vec())
            != thumbnail_hash
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(())
}

pub(crate) fn authorization_status(error: oteryn_admin::rbac::Error) -> StatusCode {
    match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn replace_in_news(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: CheckContext,
    old: Uuid,
    replacement: Uuid,
) -> Result<(), StatusCode> {
    let changed = sqlx::query(
        "WITH revisions AS MATERIALIZED (
             SELECT id, greatest(clock_timestamp(),updated_at+interval '1 microsecond') AS stamp
             FROM news WHERE media_id=$1
         ), changed AS (
             UPDATE news SET media_id=$2, updated_at=revisions.stamp,
                 translation_source_updated_at=CASE WHEN translation_source_updated_at>=updated_at
                     THEN revisions.stamp ELSE translation_source_updated_at END
             FROM revisions WHERE news.id=revisions.id RETURNING news.id
         )
         INSERT INTO audit_log (actor, action, target, before, after, request_id)
         SELECT $3, 'cms.news_media_replaced', id::text,
             jsonb_build_object('media_id', $1::uuid), jsonb_build_object('media_id', $2::uuid), uuidv7()
         FROM changed",
    )
    .bind(old)
    .bind(replacement)
    .bind(context.account)
    .execute(&mut **tx)
    .await
    .map_err(unavailable)?;
    if changed.rows_affected() == 0 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
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
