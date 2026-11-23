//! Private editorial images. Multipart callers send the browser CSRF token in
//! X-CSRF-Token. Untrusted names and MIME headers never select stored paths.
use axum::{
    Extension, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::Multipart;
use image::{GenericImageView, ImageFormat, ImageReader};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, MutationAuthorization, Permission},
    web::{StaffPreflight, UploadPreflight},
};
use oteryn_web::Language;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{io::Cursor, time::Duration};
use tokio::time::{Instant, timeout_at};
use uuid::Uuid;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_DECODE_BYTES: usize = 160_000_000;
const JPEG_MAX_COMPONENTS: u8 = 4;
const THUMBNAIL_EDGE: u32 = 320;
// Header CSRF is verified before reading the body. Allow the full byte cap
// over a slower uplink while keeping the pre-authentication form read short.
const AUTHENTICATED_UPLOAD_TIMEOUT: Duration = Duration::from_secs(15);
const ANONYMOUS_FORM_UPLOAD_TIMEOUT: Duration = Duration::from_secs(3);
// At 2 Mbps, the full 8 MiB file takes about 34 seconds before overhead.
const SIGNED_IN_FORM_UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);
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
fn file_format(name: &str) -> Result<(ImageFormat, &'static str), StatusCode> {
    let extension = name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => Ok((ImageFormat::Jpeg, "image/jpeg")),
        "png" => Ok((ImageFormat::Png, "image/png")),
        "webp" => Ok((ImageFormat::WebP, "image/webp")),
        _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

/// Decode only verified containers with bounded dimensions and allocation.
pub fn process(name: &str, bytes: &[u8]) -> Result<ProcessedImage, StatusCode> {
    let (format, mime) = file_format(name)?;
    process_image(format, mime, bytes)
}

fn process_image(
    format: ImageFormat,
    mime: &'static str,
    bytes: &[u8],
) -> Result<ProcessedImage, StatusCode> {
    let bad = StatusCode::UNPROCESSABLE_ENTITY;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(bad);
    }
    if image::guess_format(bytes).map_err(invalid)? != format {
        return Err(bad);
    }
    let boundary = match format {
        ImageFormat::Jpeg => verify_jpeg_structure(bytes).is_some(),
        ImageFormat::Png => verify_png_end(bytes).is_some(),
        ImageFormat::WebP => verify_webp_chunks(bytes).is_some(),
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
    let (decoded, preview) = if format == ImageFormat::Jpeg {
        decode_jpeg(bytes, width, height)?
    } else {
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(6000);
        limits.max_image_height = Some(6000);
        limits.max_alloc = Some(MAX_DECODE_BYTES as u64);
        reader.limits(limits);
        (reader.decode().map_err(invalid)?, None)
    };
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
    let thumbnail = if width.max(height) > THUMBNAIL_EDGE {
        let thumb = if let Some(preview) = preview {
            let scale = f64::from(THUMBNAIL_EDGE) / f64::from(width.max(height));
            let edge = |length| ((f64::from(length) * scale).round() as u32).max(1);
            preview.thumbnail_exact(edge(width), edge(height))
        } else {
            decoded.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE)
        };
        Some(encode(&thumb)?)
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
/// Validate every chunk, including ancillary chunks the decoder can ignore.
fn verify_png_end(bytes: &[u8]) -> Option<()> {
    let mut rest = bytes.strip_prefix(b"\x89PNG\r\n\x1a\n")?;
    loop {
        let (length, chunks) = rest.split_first_chunk::<4>()?;
        let crc_offset = usize::try_from(u32::from_be_bytes(*length))
            .ok()?
            .checked_add(4)?;
        let (chunk, checksum) = chunks.split_at_checked(crc_offset)?;
        let (checksum, next) = checksum.split_first_chunk::<4>()?;
        if crc32fast::hash(chunk) != u32::from_be_bytes(*checksum) {
            return None;
        }
        if matches!(chunk.get(..4)?, b"acTL" | b"fcTL" | b"fdAT") {
            return None;
        }
        if chunk.starts_with(b"IEND") {
            return (chunk.len() == 4 && next.is_empty()).then_some(());
        }
        rest = next;
    }
}

fn verify_webp_chunks(bytes: &[u8]) -> Option<()> {
    let riff = bytes.strip_prefix(b"RIFF")?;
    let (size, data) = riff.split_first_chunk::<4>()?;
    if u64::from(u32::from_le_bytes(*size)) != data.len() as u64 {
        return None;
    }
    let mut rest = data.strip_prefix(b"WEBP")?;
    while !rest.is_empty() {
        let (kind, body) = rest.split_first_chunk::<4>()?;
        let (size, data) = body.split_first_chunk::<4>()?;
        let length = usize::try_from(u32::from_le_bytes(*size)).ok()?;
        let (payload, next) = data.split_at_checked(length)?;
        rest = next;
        if !length.is_multiple_of(2) {
            rest = rest.strip_prefix(&[0])?;
        }
        if matches!(kind, b"ANIM" | b"ANMF") || (kind == b"VP8X" && payload.first()? & 0x02 != 0) {
            return None;
        }
    }
    Some(())
}

/// Require the first parsed EOI to terminate the upload and every frame
/// component to have initial scan data before that marker.
fn verify_jpeg_structure(bytes: &[u8]) -> Option<()> {
    use oxiarc_jpeg::Marker;

    let mut rest = bytes.strip_prefix(b"\xff\xd8")?;
    let mut in_scan = false;
    let mut frame: Option<(&[u8], bool)> = None;
    let mut initial_components = 0u8;
    let mut approximation = [[None; 64]; JPEG_MAX_COMPONENTS as usize];
    loop {
        if in_scan {
            rest = &rest[memchr::memchr(0xff, rest)?..];
        }
        rest = rest.strip_prefix(&[0xff])?;
        let code_index = rest.iter().position(|&byte| byte != 0xff)?;
        let (&code, tail) = rest[code_index..].split_first()?;
        rest = tail;
        match Marker::from_code(code) {
            None | Some(Marker::Rst(_)) if in_scan => continue,
            Some(Marker::Eoi) => {
                let (components, _) = frame?;
                let all_components = (1 << (components.len() / 3)) - 1;
                return (rest.is_empty() && initial_components == all_components).then_some(());
            }
            None | Some(Marker::Soi | Marker::Rst(_)) => return None,
            Some(Marker::Tem) => continue,
            Some(marker) => {
                let length = usize::from(u16::from_be_bytes(rest.get(..2)?.try_into().ok()?));
                if length < 2 {
                    return None;
                }
                let payload = rest.get(2..length)?;
                match marker {
                    Marker::Sof(code) => {
                        let count = *payload.get(5)?;
                        if frame.is_some() || !(1..=JPEG_MAX_COMPONENTS).contains(&count) {
                            return None;
                        }
                        let components = payload.get(6..6 + usize::from(count) * 3)?;
                        frame = Some((components, matches!(code, 0xc2 | 0xca)));
                    }
                    Marker::Sos => {
                        let (components, progressive) = frame?;
                        let count = *payload.first()?;
                        if !(1..=JPEG_MAX_COMPONENTS).contains(&count) {
                            return None;
                        }
                        let selectors_end = 1 + usize::from(count) * 2;
                        let selectors = payload.get(1..selectors_end)?;
                        let spectral = payload
                            .get(selectors_end..selectors_end + 3)?
                            .try_into()
                            .ok()?;
                        let mut seen = 0u8;
                        for selector in selectors.as_chunks::<2>().0 {
                            let index = components
                                .as_chunks::<3>()
                                .0
                                .iter()
                                .position(|component| component[0] == selector[0])?;
                            let mask = 1 << index;
                            if seen & mask != 0 {
                                return None;
                            }
                            seen |= mask;
                            if progressive {
                                verify_progressive_scan(
                                    &mut approximation[index],
                                    spectral,
                                    count,
                                )?;
                            }
                            if !progressive || approximation[index][0].is_some() {
                                initial_components |= 1 << index;
                            }
                        }
                    }
                    _ => {}
                }
                rest = rest.get(length..)?;
                in_scan = marker == Marker::Sos;
            }
        }
    }
}

fn verify_progressive_scan(
    coefficients: &mut [Option<u8>; 64],
    [start, end, approximation]: [u8; 3],
    components: u8,
) -> Option<()> {
    let high = approximation >> 4;
    let low = approximation & 15;
    if start > end || high > 13 || low > 13 || (high != 0 && high != low + 1) {
        return None;
    }
    if (start == 0 && end != 0) || (start != 0 && (components != 1 || coefficients[0].is_none())) {
        return None;
    }
    let band = coefficients.get_mut(usize::from(start)..=usize::from(end))?;
    let previous = (high != 0).then_some(high);
    if band.iter().any(|&value| value != previous) {
        return None;
    }
    band.fill(Some(low));
    Some(())
}

fn decode_jpeg(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(image::DynamicImage, Option<image::DynamicImage>), StatusCode> {
    use zune_jpeg::{
        JpegDecoder,
        zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions},
    };

    // Zune can synthesize entropy bits after a premature EOI even in strict
    // mode. Require a complete scan decode through EOI before using its pixels.
    // OxiArc stores full-resolution i32 coefficients even at 1/8 output
    // scale. Each of the four supported components can occupy a full frame
    // with up to 31 padding samples per axis (8 samples × a factor of 4).
    const MAX_MCU_PADDING: u64 = 8 * 4 - 1;
    let max_coefficient_bytes = (u64::from(width) + MAX_MCU_PADDING)
        * (u64::from(height) + MAX_MCU_PADDING)
        * u64::from(JPEG_MAX_COMPONENTS)
        * size_of::<i32>() as u64;
    let verification = oxiarc_jpeg::DecodeOptions {
        limits: oxiarc_jpeg::DecodeLimits {
            max_width: width,
            max_height: height,
            max_pixels: u64::from(width) * u64::from(height),
            max_input_bytes: MAX_BYTES as u64,
            max_output_bytes: MAX_DECODE_BYTES as u64,
            max_components: JPEG_MAX_COMPONENTS,
            max_coefficient_bytes,
            ..oxiarc_jpeg::DecodeLimits::strict()
        },
        // At 1/8 scale the validator still entropy-decodes every MCU.
        scale: oxiarc_jpeg::Scale::ONE_EIGHTH,
        ..oxiarc_jpeg::DecodeOptions::strict()
    };
    let mut verifier = oxiarc_jpeg::Decoder::with_options(bytes, verification);
    let scaled_pixels = verifier.decode().map_err(invalid)?;
    // The strict scan verification already produces a downsampled image.
    // Reuse it when it has enough pixels for the thumbnail at the same aspect ratio.
    let preview =
        verifier.info().and_then(|info| {
            let (w, h) = (info.scaled_width, info.scaled_height);
            if w.max(h) < THUMBNAIL_EDGE || (w, h) == (width, height) {
                return None;
            }
            match info.pixel_format() {
                oxiarc_jpeg::PixelFormat::L8 => image::GrayImage::from_raw(w, h, scaled_pixels)
                    .map(image::DynamicImage::ImageLuma8),
                oxiarc_jpeg::PixelFormat::Rgb8 => image::RgbImage::from_raw(w, h, scaled_pixels)
                    .map(image::DynamicImage::ImageRgb8),
                _ => None,
            }
        });
    drop(verifier);

    let options = DecoderOptions::default()
        .set_strict_mode(true)
        .set_max_width(width as usize)
        .set_max_height(height as usize);
    let bad = StatusCode::UNPROCESSABLE_ENTITY;
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(invalid)?;
    if decoder.dimensions() != Some((width as usize, height as usize)) {
        return Err(bad);
    }
    let color = match decoder.input_colorspace().ok_or(bad)? {
        color @ (ColorSpace::Luma | ColorSpace::LumaA | ColorSpace::RGBA) => color,
        _ => ColorSpace::RGB,
    };
    decoder.set_options(decoder.options().jpeg_set_out_colorspace(color));
    let pixels = decoder.decode().map_err(invalid)?;
    let decoded = match color {
        ColorSpace::Luma => {
            image::GrayImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageLuma8)
        }
        ColorSpace::LumaA => image::GrayAlphaImage::from_raw(width, height, pixels)
            .map(image::DynamicImage::ImageLumaA8),
        ColorSpace::RGBA => {
            image::RgbaImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageRgba8)
        }
        _ => image::RgbImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageRgb8),
    }
    .ok_or(bad)?;
    Ok((decoded, preview))
}

/// Routes must be wrapped in the staff guard before mounting.
pub fn admin_router(pool: PgPool) -> Router {
    Router::new()
        .route(
            "/admin/media",
            axum::routing::post(upload)
                .layer(axum::middleware::from_fn(mutation_validation))
                .get(index),
        )
        .route("/admin/media/{id}/content", get(content))
        .route("/admin/media/{id}/thumbnail", get(thumbnail))
        .route(
            "/admin/media/{id}",
            axum::routing::delete(delete).layer(axum::middleware::from_fn(mutation_validation)),
        )
        .route(
            "/admin/media/{id}/delete",
            axum::routing::post(delete).layer(axum::middleware::from_fn(mutation_validation)),
        )
        .with_state(pool)
        .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
}
async fn mutation_validation(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let lang = request
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let language = if lang.starts_with("en") {
        oteryn_web::Language::En
    } else {
        oteryn_web::Language::Pl
    };
    let response = next.run(request).await;
    let key = match response.status() {
        StatusCode::BAD_REQUEST => "error-400-description",
        StatusCode::UNPROCESSABLE_ENTITY => "error-422-description",
        _ => return response,
    };
    (response.status(), language.text(key)).into_response()
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    configured_router_from(pool, &|key| std::env::var(key).ok())
}
fn configured_router_from(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let staff = oteryn_admin::web::configured_staff_preflight_from(
        pool.clone(),
        Permission::ManageMedia,
        lookup,
    )?;
    let routes = staff
        .wrap(admin_router(pool))
        .layer(axum::middleware::from_fn_with_state(staff, multipart_csrf))
        .layer(DefaultBodyLimit::max(MAX_BYTES + 16384))
        .layer(axum::middleware::from_fn(headers));
    oteryn_admin::web::configured_host_boundary_from(routes, lookup)
}
// Multer inserts headers into a HeaderMap, losing repeated declarations. Check
// the same multipart boundaries and raw header syntax before either parser runs.
fn validate_part_headers(body: &[u8], boundary: &str) -> Result<(), StatusCode> {
    use memchr::memmem::find;
    let bad = StatusCode::UNPROCESSABLE_ENTITY;
    let delimiter = format!("--{boundary}");
    let next_delimiter = format!("\r\n{delimiter}");
    let first = find(body, delimiter.as_bytes()).ok_or(bad)?;
    let mut rest = &body[first + delimiter.len()..];
    loop {
        if rest.starts_with(b"--") {
            return Ok(());
        }
        let padding = rest
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        let part = rest[padding..].strip_prefix(b"\r\n").ok_or(bad)?;
        let headers_end = find(part, b"\r\n\r\n").ok_or(bad)? + 4;
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let httparse::Status::Complete((_, headers)) =
            httparse::parse_headers(&part[..headers_end], &mut headers).map_err(invalid)?
        else {
            return Err(bad);
        };
        if headers
            .iter()
            .filter(|header| header.name.eq_ignore_ascii_case("content-type"))
            .nth(1)
            .is_some()
        {
            return Err(bad);
        }
        let data = &part[headers_end..];
        let next = find(data, next_delimiter.as_bytes()).ok_or(bad)?;
        rest = &data[next + next_delimiter.len()..];
    }
}

async fn checked_body(request: &mut Request, deadline: Instant) -> Result<Bytes, StatusCode> {
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let boundary = multer::parse_boundary(content_type).map_err(invalid)?;
    let body = std::mem::replace(request.body_mut(), axum::body::Body::empty());
    let bytes = timeout_at(deadline, axum::body::to_bytes(body, MAX_BYTES + 16384))
        .await
        .map_err(|_| StatusCode::REQUEST_TIMEOUT)?
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    validate_part_headers(&bytes, &boundary)?;
    Ok(bytes)
}

#[derive(Clone)]
struct CheckedPartHeaders;

struct CheckedMultipart(Multipart);

impl<S: Send + Sync> FromRequest<S> for CheckedMultipart {
    type Rejection = StatusCode;

    async fn from_request(mut request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if request.extensions().get::<CheckedPartHeaders>().is_none() {
            *request.body_mut() = axum::body::Body::from(
                checked_body(&mut request, Instant::now() + AUTHENTICATED_UPLOAD_TIMEOUT).await?,
            );
        }
        Multipart::from_request(request, state)
            .await
            .map(Self)
            .map_err(invalid)
    }
}

/// Adapt a native multipart form token to the shared synchronizer-token verifier.
/// The shared guard still validates the origin, session and constant-time token.
async fn multipart_csrf(
    State(staff): State<StaffPreflight>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    if request.method() == axum::http::Method::POST
        && request.uri().path() == "/admin/media"
        && !request.headers().contains_key("x-csrf-token")
    {
        let started = Instant::now();
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .cloned()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let short_deadline = started + ANONYMOUS_FORM_UPLOAD_TIMEOUT;
        let preflight = timeout_at(
            short_deadline,
            staff.upload_body_access(
                request.headers(),
                request.uri().path(),
                request.extensions().get::<Language>().copied(),
            ),
        )
        .await
        .map_err(|_| StatusCode::REQUEST_TIMEOUT)?;
        let access = match preflight {
            Ok(access) => access,
            Err(error) => return Ok(error.into_response()),
        };
        let deadline = match access {
            UploadPreflight::Long => started + SIGNED_IN_FORM_UPLOAD_TIMEOUT,
            UploadPreflight::Short | UploadPreflight::Denied => short_deadline,
        };
        let bytes = checked_body(&mut request, deadline)
            .await
            .map_err(|status| {
                if access == UploadPreflight::Denied && status == StatusCode::REQUEST_TIMEOUT {
                    StatusCode::FORBIDDEN
                } else {
                    status
                }
            })?;
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
        request.extensions_mut().insert(CheckedPartHeaders);
    }
    Ok(next.run(request).await)
}
#[derive(serde::Deserialize, Default)]
struct Options {
    #[serde(default)]
    page: u32,
    lang: Option<String>,
}

impl Options {
    fn language(&self) -> Language {
        if self.lang.as_deref() == Some("en") {
            Language::En
        } else {
            Language::Pl
        }
    }
}
#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\" class=\"op-theme\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"><title>{{ title }}</title><main class=\"op-stack\"><h1>{{ title }}</h1><form class=\"op-stack\" method=\"post\" action=\"/admin/media?lang={{ lang }}\" enctype=\"multipart/form-data\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"upload_key\" value=\"{{ upload_key }}\"><label for=\"image\">{{ image }}</label><input class=\"op-input\" id=\"image\" name=\"image\" type=\"file\" accept=\"image/jpeg,image/png,image/webp\" required><label for=\"alt\">{{ alt }}</label><textarea class=\"op-input\" id=\"alt\" name=\"alt_text\" maxlength=\"500\" required></textarea><button class=\"op-button\">{{ upload }}</button></form><ul>{% for item in items %}<li><img src=\"/admin/media/{{ item.0 }}/thumbnail\" width=\"160\" alt=\"{{ item.1 }}\"><p>{{ item.1 }}</p><form method=\"post\" action=\"/admin/media/{{ item.0 }}/delete?lang={{ lang }}\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><button class=\"op-button\">{{ delete }}</button></form></li>{% endfor %}</ul>{% if more %}<a href=\"?page={{ next }}&amp;lang={{ lang }}\">{{ next_label }}</a>{% endif %}</main></html>",
    ext = "html"
)]
struct MediaPage {
    next: u64,
    more: bool,
    next_label: String,
    lang: &'static str,
    title: String,
    image: String,
    alt: String,
    upload: String,
    delete: String,
    csrf: String,
    upload_key: Uuid,
    items: Vec<(Uuid, String)>,
}
fn label(lang: &str, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = if lang == "en" {
        concat!(
            "title = Media\nimage = Image (JPEG, PNG, WebP; max 8 MiB)\nalt = Alternative text\n",
            "upload = Upload\ndelete = Delete\nnext = Next page\n",
            "timeout-title = Upload timed out\ntimeout-message = The file did not upload before the deadline.\n",
            "conflict-title = This form was already used\nconflict-message = Open a new form to upload this file with the changed details.\n",
            "back = Back to the media form",
        )
    } else {
        concat!(
            "title = Media\nimage = Obraz (JPEG, PNG, WebP; maks. 8 MiB)\nalt = Tekst alternatywny\n",
            "upload = Prześlij\ndelete = Usuń\nnext = Następna strona\n",
            "timeout-title = Przesyłanie trwało zbyt długo\ntimeout-message = Plik nie został przesłany w wyznaczonym czasie.\n",
            "conflict-title = Formularz został już użyty\nconflict-message = Otwórz nowy formularz, aby przesłać plik ze zmienionymi danymi.\n",
            "back = Wróć do formularza multimediów",
        )
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
    language: Option<Extension<Language>>,
    Query(options): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    use askama::Template;
    let lang = language
        .map_or_else(|| options.language(), |Extension(lang)| lang)
        .code();
    let rows = sqlx::query(
        "SELECT id,alt_text FROM media ORDER BY created_at DESC,id DESC LIMIT 100 OFFSET $1",
    )
    .bind(i64::from(options.page) * 100)
    .fetch_all(&pool)
    .await
    .map_err(unavailable)?;
    let page = MediaPage {
        next: u64::from(options.page) + 1,
        more: rows.len() == 100,
        next_label: label(lang, "next"),
        lang,
        title: label(lang, "title"),
        image: label(lang, "image"),
        alt: label(lang, "alt"),
        upload: label(lang, "upload"),
        delete: label(lang, "delete"),
        csrf: session.csrf_token().await.map_err(unavailable)?,
        upload_key: Uuid::now_v7(),
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
    Extension(auth): Extension<MutationAuthorization>,
    Query(options): Query<Options>,
    headers: HeaderMap,
    CheckedMultipart(mut multipart): CheckedMultipart,
) -> Result<Response, StatusCode> {
    let bad = StatusCode::UNPROCESSABLE_ENTITY;
    let mut upload_key = None;
    for value in headers.get_all("idempotency-key") {
        set_upload_key(&mut upload_key, value.to_str().map_err(invalid)?)?;
    }
    let mut image = None;
    let mut alt = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid)? {
        match field.name() {
            Some("image") if image.is_none() => {
                let name = field
                    .file_name()
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .to_owned();
                let declared_type = field.headers().get(header::CONTENT_TYPE).cloned();
                let bytes = field.bytes().await.map_err(invalid)?;
                image = Some((name, declared_type, bytes));
            }
            Some("_csrf") => {
                let _ = field.text().await.map_err(invalid)?;
            }
            Some("upload_key") => {
                set_upload_key(&mut upload_key, &field.text().await.map_err(invalid)?)?;
            }
            Some("alt_text") if alt.is_none() => {
                alt = Some(field.text().await.map_err(invalid)?);
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    let alt = alt.ok_or(bad)?;
    if alt.trim().is_empty() || alt.chars().count() > 500 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let alt = alt.trim();
    let (name, declared_type, bytes) = image.ok_or(bad)?;
    let (format, mime) = file_format(&name)?;
    if bytes.is_empty()
        || bytes.len() > MAX_BYTES
        || declared_type.is_some_and(|value| {
            let value = value.as_bytes();
            !value.eq_ignore_ascii_case(b"application/octet-stream")
                && !value.eq_ignore_ascii_case(mime.as_bytes())
        })
    {
        return Err(bad);
    }
    let fingerprint = upload_key.map(|key| {
        let mut digest = Sha256::new();
        digest.update(mime.as_bytes());
        digest.update((alt.len() as u64).to_be_bytes());
        digest.update(alt.as_bytes());
        digest.update(&bytes);
        (key, <[u8; 32]>::from(digest.finalize()))
    });
    // The fast path avoids decoding a completed upload again. Its result is
    // checked below while holding the same locks as a new upload.
    let completed = match upload_key {
        Some(key) => upload_receipt(&pool, context.account, key).await?.is_some(),
        None => false,
    };
    let processed = if completed {
        None
    } else {
        Some(tokio_process(format, mime, bytes).await?)
    };
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
    if let Some((key, fingerprint)) = &fingerprint
        && let Some(previous) = upload_receipt(&mut *tx, context.account, *key).await?
    {
        if previous.as_slice() != fingerprint.as_slice() {
            return Err(StatusCode::CONFLICT);
        }
        tx.commit().await.map_err(unavailable)?;
        return Ok(media_redirect(&options));
    }
    let processed = processed.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE actor=$1 AND action='editorial_media.uploaded' AND created_at > now()-interval '1 minute'").bind(context.account).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if recent >= 10 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256,thumbnail,thumbnail_sha256) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(id).bind(alt).bind(processed.mime).bind(processed.width as i32).bind(processed.height as i32)
        .bind(&processed.content).bind(Sha256::digest(&processed.content).to_vec()).bind(&processed.thumbnail)
        .bind(processed.thumbnail.as_ref().map(|b| Sha256::digest(b).to_vec())).execute(&mut *tx).await.map_err(unavailable)?;
    if let Some((key, fingerprint)) = fingerprint {
        sqlx::query("INSERT INTO editorial_media_upload_receipts(account_id,upload_key,fingerprint) VALUES($1,$2,$3)")
            .bind(context.account)
            .bind(key)
            .bind(fingerprint.as_slice())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
    }
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
    Ok(media_redirect(&options))
}
fn set_upload_key(key: &mut Option<Uuid>, value: &str) -> Result<(), StatusCode> {
    let parsed = Uuid::parse_str(value).map_err(invalid)?;
    if parsed.is_nil() || key.is_some() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    *key = Some(parsed);
    Ok(())
}

async fn upload_receipt<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    actor: Uuid,
    key: Uuid,
) -> Result<Option<Vec<u8>>, StatusCode> {
    sqlx::query_scalar(
        "SELECT fingerprint FROM editorial_media_upload_receipts WHERE account_id=$1 AND upload_key=$2",
    )
    .bind(actor)
    .bind(key)
    .fetch_optional(executor)
    .await
    .map_err(unavailable)
}

async fn tokio_process(
    format: ImageFormat,
    mime: &'static str,
    bytes: Bytes,
) -> Result<ProcessedImage, StatusCode> {
    let permit = DECODERS
        .try_acquire()
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        process_image(format, mime, &bytes)
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
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Query(options): Query<Options>,
    method: axum::http::Method,
) -> Result<Response, StatusCode> {
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
        &[Permission::Access, Permission::ManageMedia],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let row = sqlx::query(
        "SELECT content,sha256,thumbnail,thumbnail_sha256 FROM media WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    auth.check_in(&mut tx, context, Permission::ManageMedia)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
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
    let status = if method == axum::http::Method::DELETE {
        StatusCode::SEE_OTHER
    } else {
        StatusCode::FOUND
    };
    Ok((status, [(header::LOCATION, media_url(options.language()))]).into_response())
}

fn media_redirect(options: &Options) -> Response {
    (
        StatusCode::FOUND,
        [(header::LOCATION, media_url(options.language()))],
    )
        .into_response()
}

fn media_url(lang: Language) -> &'static str {
    match lang {
        Language::En => "/admin/media?lang=en",
        Language::Pl => "/admin/media",
    }
}

#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><main><h1>{{ title }}</h1><p>{{ message }}</p><a href=\"{{ back_url }}\">{{ back }}</a></main></body></html>",
    ext = "html"
)]
struct UploadErrorPage {
    lang: &'static str,
    title: String,
    message: String,
    back_url: &'static str,
    back: String,
}

fn upload_error_page(status: StatusCode, language: Language) -> Option<String> {
    use askama::Template;
    let (title, message) = match status {
        StatusCode::REQUEST_TIMEOUT => ("timeout-title", "timeout-message"),
        StatusCode::CONFLICT => ("conflict-title", "conflict-message"),
        _ => return None,
    };
    let lang = language.code();
    UploadErrorPage {
        lang,
        title: label(lang, title),
        message: label(lang, message),
        back_url: media_url(language),
        back: label(lang, "back"),
    }
    .render()
    .ok()
}
#[cfg(test)]
#[path = "media_tests.rs"]
mod media_tests;

async fn headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let native_form = request.method() == axum::http::Method::POST
        && request.uri().path() == "/admin/media"
        && !request.headers().contains_key("x-csrf-token")
        && request
            .headers()
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|accept| accept.contains("text/html"));
    let language = native_form.then(|| {
        request
            .extensions()
            .get::<Language>()
            .copied()
            .unwrap_or_else(|| {
                Query::<Options>::try_from_uri(request.uri())
                    .unwrap_or(Query(Options::default()))
                    .language()
            })
    });
    let mut response = next.run(request).await;
    if let Some(language) = language
        && let Some(body) = upload_error_page(response.status(), language)
    {
        let (mut parts, _) = response.into_parts();
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html; charset=utf-8"),
        );
        parts.headers.insert(
            header::CONTENT_LANGUAGE,
            header::HeaderValue::from_static(language.code()),
        );
        response = Response::from_parts(parts, axum::body::Body::from(body));
    }
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
