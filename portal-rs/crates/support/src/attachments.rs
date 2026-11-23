//! Ticket-owned screenshots use the editorial image decoder and a separate store.
use super::tickets::{Error, Tickets, active};
use axum::{
    Extension,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, Path, Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Multipart;
use oteryn_admin::{
    audit,
    rbac::{self, CheckContext, MutationAuthorization, Permission},
};
use oteryn_content::media::{self, ProcessedImage};
use oteryn_web::{
    ratelimit::{ClientSource, NamedPolicy},
    session::{self, BrowserSession, SessionLayer, SignedInSession},
};
use sqlx::{PgConnection, PgPool};
use std::time::Duration;
use uuid::Uuid;

pub(super) const MAX_PER_TICKET: usize = 5;
const BODY_LIMIT: usize = media::MAX_BYTES + 16_384;
static UPLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);

fn ticket_authority_error(error: rbac::Error) -> Error {
    match error {
        rbac::Error::Storage(error) => Error::Database(error),
        _ => Error::Denied,
    }
}

pub(super) struct Upload {
    name: String,
    bytes: Bytes,
    request_key: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AttachmentError {
    #[error(transparent)]
    Ticket(#[from] Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("screenshot limit reached")]
    Full,
    #[error("screenshot audit unavailable")]
    Audit,
}

impl AttachmentError {
    fn response(self, lang: &str) -> Response {
        use super::tickets::web::{fail, screenshot_error};
        match self {
            Self::Ticket(error) => fail(error, lang),
            Self::Full => screenshot_error(StatusCode::CONFLICT, lang, "screenshot-full"),
            Self::Database(_) | Self::Audit => {
                screenshot_error(StatusCode::SERVICE_UNAVAILABLE, lang, "error")
            }
        }
    }
}

async fn parse(request: Request) -> Result<(header::HeaderValue, Upload), StatusCode> {
    let mut data = Multipart::from_request(request, &())
        .await
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut image = None;
    let mut csrf = None;
    let mut request_key = None;
    while let Some(field) = data.next_field().await.map_err(|e| e.status())? {
        match field.name() {
            Some("_csrf") if csrf.is_none() => {
                let token = field.bytes().await.map_err(|e| e.status())?;
                if token.len() != 64 {
                    return Err(StatusCode::FORBIDDEN);
                }
                csrf = Some(
                    header::HeaderValue::from_bytes(&token).map_err(|_| StatusCode::FORBIDDEN)?,
                );
            }
            Some("_csrf") => return Err(StatusCode::FORBIDDEN),
            Some("request_key") if request_key.is_none() => {
                let bytes = field.bytes().await.map_err(|e| e.status())?;
                let key =
                    Uuid::try_parse_ascii(&bytes).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                if key.is_nil() {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                request_key = Some(key);
            }
            Some("image") if image.is_none() => {
                let name = field
                    .file_name()
                    .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
                    .to_owned();
                let bytes = field.bytes().await.map_err(|e| e.status())?;
                if bytes.len() > media::MAX_BYTES {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
                image = Some((name, bytes));
            }
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
    let csrf = csrf.ok_or(StatusCode::FORBIDDEN)?;
    let request_key = request_key.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let (name, bytes) = image.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    Ok((
        csrf,
        Upload {
            name,
            bytes,
            request_key,
        },
    ))
}

fn is_upload(request: &Request) -> bool {
    request.method() == Method::POST && request.uri().path().ends_with("/attachments")
}

pub(super) async fn upload_session(
    state: State<SessionLayer>,
    request: Request,
    next: Next,
) -> Response {
    if is_upload(&request) {
        return session::read_only_middleware(state, request, next).await;
    }
    next.run(request).await
}

/// Lift the native form token into the shared synchronizer-token verifier.
/// No decoding or storage takes place in this request adapter.
pub(super) async fn form_csrf(request: Request, next: Next) -> Response {
    if !is_upload(&request) {
        return next.run(request).await;
    }
    let (mut parts, body) = request.into_parts();
    let language = parts.uri.path().split('/').nth(1).unwrap_or("pl");
    let error = |status| {
        let key = match status {
            StatusCode::PAYLOAD_TOO_LARGE => "screenshot-size",
            StatusCode::TOO_MANY_REQUESTS => "screenshot-busy",
            StatusCode::FORBIDDEN => "error",
            _ => "screenshot-invalid",
        };
        super::tickets::web::screenshot_error(status, language, key)
    };
    if parts.extensions.get::<SignedInSession>().is_none()
        || parts.headers.contains_key("x-csrf-token")
    {
        return error(StatusCode::FORBIDDEN);
    }
    let Ok(_permit) = UPLOADS.try_acquire() else {
        return error(StatusCode::TOO_MANY_REQUESTS);
    };
    // Retain the route's body limit while passing the remaining request to the guards.
    let mut request = Request::new(body);
    request.headers_mut().clone_from(&parts.headers);
    DefaultBodyLimit::max(BODY_LIMIT).apply(&mut request);
    let (csrf, image) = match tokio::time::timeout(Duration::from_secs(10), parse(request)).await {
        Ok(Ok(data)) => data,
        Ok(Err(status)) => return error(status),
        Err(_) => return error(StatusCode::REQUEST_TIMEOUT),
    };
    parts.headers.insert("x-csrf-token", csrf);
    parts.extensions.insert(std::sync::Arc::new(image));
    next.run(Request::from_parts(parts, axum::body::Body::empty()))
        .await
}

pub(super) async fn ids(
    pool: &PgPool,
    actor: Uuid,
    staff: bool,
    ticket: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT s.id FROM ticket_screenshots s JOIN tickets t ON t.id=s.ticket_id
         WHERE t.id=$1 AND ($2 OR t.account_id=$3) ORDER BY s.id",
    )
    .bind(ticket)
    .bind(staff)
    .bind(actor)
    .fetch_all(pool)
    .await
}

async fn scope(
    connection: &mut PgConnection,
    actor: Uuid,
    staff: bool,
    ticket: Uuid,
) -> Result<String, Error> {
    sqlx::query_scalar(
        "SELECT status FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE",
    )
    .bind(ticket)
    .bind(staff)
    .bind(actor)
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

fn check_capacity(status: &str, count: usize) -> Result<(), AttachmentError> {
    if !active(status) {
        return Err(Error::Conflict.into());
    }
    if count >= MAX_PER_TICKET {
        return Err(AttachmentError::Full);
    }
    Ok(())
}

pub(super) async fn store(
    tickets: &Tickets,
    actor: Uuid,
    staff: Option<(CheckContext, &MutationAuthorization)>,
    ticket: Uuid,
    request_key: Uuid,
    image: ProcessedImage,
) -> Result<Uuid, AttachmentError> {
    tickets.enabled()?;
    let mut tx = tickets.pool.begin().await?;
    let status = if let Some((context, auth)) = staff {
        if context.account != actor {
            return Err(Error::Denied.into());
        }
        rbac::authorize_mutation_with_resources(
            &mut tx,
            &[actor],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM tickets WHERE id=$1 FOR UPDATE")
                        .bind(ticket)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            auth,
            context,
            &[Permission::Access, Permission::ManageSupportTickets],
        )
        .await
        .map_err(ticket_authority_error)?;
        scope(&mut tx, actor, true, ticket).await?
    } else {
        tickets.lock_account(&mut tx, actor).await?;
        scope(&mut tx, actor, false, ticket).await?
    };
    let original: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, mime_type=$4 AND content=$5 FROM ticket_screenshots
         WHERE ticket_id=$1 AND uploader_id=$2 AND request_key=$3",
    )
    .bind(ticket)
    .bind(actor)
    .bind(request_key)
    .bind(image.mime)
    .bind(&image.content)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((id, matches)) = original {
        if !matches {
            return Err(Error::RequestKeyConflict.into());
        }
        tx.commit().await?;
        return Ok(id);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ticket_screenshots WHERE ticket_id=$1")
            .bind(ticket)
            .fetch_one(&mut *tx)
            .await?;
    check_capacity(
        &status,
        usize::try_from(count).map_err(|_| AttachmentError::Full)?,
    )?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO ticket_screenshots(ticket_id,uploader_id,request_key,mime_type,content)
         VALUES($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(ticket)
    .bind(actor)
    .bind(request_key)
    .bind(image.mime)
    .bind(image.content)
    .fetch_one(&mut *tx)
    .await?;
    if let Some((context, auth)) = staff {
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(actor),
                    request_id: Uuid::now_v7(),
                },
                action: "support.ticket_screenshot_added",
                target: &ticket.to_string(),
                before: serde_json::Value::Null,
                after: serde_json::json!({"screenshot_id": id}),
            },
        )
        .await
        .map_err(|_| AttachmentError::Audit)?;
        auth.check_in(&mut tx, context, Permission::ManageSupportTickets)
            .await
            .map_err(ticket_authority_error)?;
    }
    tx.commit().await?;
    Ok(id)
}

pub(super) async fn upload(
    State(tickets): State<Tickets>,
    Path((lang, ticket)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
    auth: Option<Extension<MutationAuthorization>>,
    peer: Option<ClientSource>,
    Extension(upload): Extension<std::sync::Arc<Upload>>,
) -> Response {
    use super::tickets::web::{
        account, fail, limited, limiter_response, redirect, screenshot_error,
    };
    if let Err(error) = super::tickets::web::lang(&lang) {
        return error.into_response();
    }
    if let Err(error) = tickets.enabled() {
        return fail(error, &lang);
    }
    let (actor, staff) = match account(&tickets, &session, staff).await {
        Ok(value) => value,
        Err(status) => return status.into_response(),
    };
    let staff_auth = match (staff, auth.as_ref()) {
        (Some(context), Some(auth)) => Some((context, &auth.0)),
        (Some(_), None) => return StatusCode::FORBIDDEN.into_response(),
        (None, _) => None,
    };
    if let Err(error) = limited(&tickets, NamedPolicy::SupportTicketReply, actor, peer).await {
        return limiter_response(error, &lang);
    }
    let visible = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tickets WHERE id=$1 AND ($2 OR account_id=$3))",
    )
    .bind(ticket)
    .bind(staff.is_some())
    .bind(actor)
    .fetch_one(&tickets.pool)
    .await;
    match visible {
        Ok(true) => {}
        Ok(false) => return fail(Error::NotFound, &lang),
        Err(error) => return fail(Error::Database(error), &lang),
    };
    let image = match media::process_upload(upload.name.clone(), upload.bytes.to_vec()).await {
        Ok(image) => image,
        Err(status) => {
            let key = match status {
                StatusCode::TOO_MANY_REQUESTS => "screenshot-busy",
                StatusCode::UNPROCESSABLE_ENTITY => "screenshot-invalid",
                _ => "error",
            };
            return screenshot_error(status, &lang, key);
        }
    };
    match store(
        &tickets,
        actor,
        staff_auth,
        ticket,
        upload.request_key,
        image,
    )
    .await
    {
        Ok(_) => redirect(&lang, staff.is_some(), ticket),
        Err(error) => error.response(&lang),
    }
}

pub(super) async fn read(
    State(tickets): State<Tickets>,
    Path((lang, ticket, id)): Path<(String, Uuid, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
) -> Result<Response, StatusCode> {
    super::tickets::web::lang(&lang)?;
    tickets.enabled().map_err(|_| StatusCode::FORBIDDEN)?;
    let (actor, staff) = super::tickets::web::account(&tickets, &session, staff).await?;
    let (mime, content): (String, Vec<u8>) = sqlx::query_as(
        "SELECT s.mime_type,s.content FROM ticket_screenshots s
         JOIN tickets t ON t.id=s.ticket_id JOIN accounts a ON a.id=$3
         WHERE t.id=$1 AND s.id=$2 AND a.status='active' AND ($4 OR t.account_id=$3)",
    )
    .bind(ticket)
    .bind(id)
    .bind(actor)
    .bind(staff.is_some())
    .fetch_optional(&tickets.pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::NOT_FOUND)?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CONTENT_DISPOSITION, "inline".into()),
        ],
        content,
    )
        .into_response())
}
