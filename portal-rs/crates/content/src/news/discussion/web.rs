use super::{Edit, Error, Review, Write, eligible, moderate, thread, write};
use crate::news::{Options, label};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_admin::rbac::{CheckContext, MutationAuthorization};
use oteryn_web::session::{self, BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

const PAGE_SIZE: i64 = 20;
const BODY_LIMIT: usize = 32 * 1024;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Limit => StatusCode::TOO_MANY_REQUESTS,
            Self::Storage(_) | Self::Staff(_) | Self::Render(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        status.into_response()
    }
}

async fn private(request: axum::extract::Request, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(in crate::news) fn public_routes(pool: PgPool, origin: &str) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = vec![origin.to_owned()];
    Router::new()
        .route("/{lang}/news/{slug}/discussion", get(show).post(submit))
        .route("/{lang}/news/{slug}/discussion/{id}/edit", post(edit))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(pool)
}

pub(in crate::news) fn staff_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/news/discussions", get(queue))
        .route("/admin/news/discussions/{id}", get(detail).post(review))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(middleware::from_fn(private))
        .with_state(pool)
}

#[derive(sqlx::FromRow)]
struct Reply {
    id: Uuid,
    body: String,
    status: String,
    revision: i64,
    mine: bool,
    time: String,
}

struct Pages {
    previous: Option<i64>,
    next: Option<i64>,
}

impl Pages {
    fn new(options: &Options, count: usize) -> Self {
        let page = options.page.unwrap_or(1);
        Self {
            previous: (page > 1).then_some(page - 1),
            next: (count > PAGE_SIZE as usize && page < super::super::MAX_PAGE).then_some(page + 1),
        }
    }
}

#[derive(Template)]
#[template(path = "content/discussion.html", config = "src/pages/askama.toml")]
struct Discussion<'a> {
    lang: &'a str,
    slug: &'a str,
    article: String,
    replies: Vec<Reply>,
    signed_in: bool,
    csrf: Option<String>,
    page: i64,
    pages: Pages,
}

impl Discussion<'_> {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
    fn status(&self, status: &str) -> String {
        self.text(&format!("discussion-status-{status}"))
    }
}

async fn show(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, slug)): Path<(String, String)>,
    Query(query): Query<Options>,
) -> Result<Html<String>, Error> {
    let offset = query.offset(PAGE_SIZE).map_err(|_| Error::Invalid)?;
    let account = session.account_id().await;
    let mut tx = pool.begin().await?;
    let article = thread(&mut tx, &lang, &slug).await?;
    let can_write = match account {
        Some(id) => eligible(&mut tx, id).await?,
        None => false,
    };
    let replies: Vec<Reply> = sqlx::query_as(
        "SELECT id,body,status,revision,coalesce(author_id=$2,false) AS mine,
                to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI') AS time
         FROM news_discussion_replies WHERE news_id=$1 AND (status='published' OR author_id=$2)
         ORDER BY id DESC LIMIT $3 OFFSET $4",
    )
    .bind(article.id)
    .bind(account)
    .bind(PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let pages = Pages::new(&query, replies.len());
    let csrf = if can_write {
        Some(session.csrf_token().await?)
    } else {
        None
    };
    render(Discussion {
        lang: &lang,
        slug: &slug,
        article: article.title,
        replies: replies.into_iter().take(PAGE_SIZE as usize).collect(),
        signed_in: account.is_some(),
        csrf,
        page: query.page.unwrap_or(1),
        pages,
    })
}

#[derive(Deserialize)]
struct ReplyForm {
    body: String,
}

async fn submit(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, slug)): Path<(String, String)>,
    Form(form): Form<ReplyForm>,
) -> Result<Redirect, LocalizedError> {
    let (account, hash) = credentials(&session)
        .await
        .map_err(|error| failure(&lang, error))?;
    write(
        &pool,
        &lang,
        &slug,
        account,
        &hash,
        Write::Reply(&form.body),
    )
    .await
    .map_err(|error| failure(&lang, error))?;
    Ok(Redirect::to(&format!(
        "/{lang}/news/{slug}/discussion#replies"
    )))
}

async fn edit(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, slug, id)): Path<(String, String, Uuid)>,
    Query(query): Query<Options>,
    Form(form): Form<Edit>,
) -> Result<Redirect, LocalizedError> {
    query
        .offset(PAGE_SIZE)
        .map_err(|_| failure(&lang, Error::Invalid))?;
    let (account, hash) = credentials(&session)
        .await
        .map_err(|error| failure(&lang, error))?;
    write(&pool, &lang, &slug, account, &hash, Write::Edit(id, &form))
        .await
        .map_err(|error| failure(&lang, error))?;
    Ok(Redirect::to(&format!(
        "/{lang}/news/{slug}/discussion?page={}#reply-{id}",
        query.page.unwrap_or(1)
    )))
}

async fn credentials(session: &BrowserSession) -> Result<(Uuid, Vec<u8>), Error> {
    let account = session.account_id().await.ok_or(Error::Forbidden)?;
    let hash = session.identity_hash().await.ok_or(Error::Forbidden)?;
    Ok((account, hash))
}

#[derive(sqlx::FromRow)]
struct QueueEntry {
    id: Uuid,
    article: Option<String>,
    excerpt: String,
    time: String,
}

#[derive(Template)]
#[template(
    path = "content/discussion_queue.html",
    config = "src/pages/askama.toml"
)]
struct Queue<'a> {
    lang: &'a str,
    status: &'a str,
    statuses: [&'static str; 4],
    entries: Vec<QueueEntry>,
    pages: Pages,
}

impl Queue<'_> {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
    fn selected(&self, status: &str) -> bool {
        self.status == status
    }
    fn status_label(&self, status: &str) -> String {
        self.text(&format!("discussion-status-{status}"))
    }
}

async fn queue(
    State(pool): State<PgPool>,
    Query(query): Query<Options>,
) -> Result<Html<String>, Error> {
    let status = query.status.as_deref().unwrap_or("pending");
    let statuses = ["pending", "published", "rejected", "hidden"];
    if !statuses.contains(&status) {
        return Err(Error::Invalid);
    }
    let offset = query.offset(PAGE_SIZE).map_err(|_| Error::Invalid)?;
    let entries: Vec<QueueEntry> = sqlx::query_as(
        "SELECT r.id,n.title AS article,left(r.body,120) AS excerpt,
                to_char(r.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI') AS time
         FROM news_discussion_replies r LEFT JOIN news n ON n.id=r.news_id
         WHERE r.status=$1 ORDER BY r.id LIMIT $2 OFFSET $3",
    )
    .bind(status)
    .bind(PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&pool)
    .await?;
    render(Queue {
        lang: query.lang(),
        status,
        statuses,
        pages: Pages::new(&query, entries.len()),
        entries: entries.into_iter().take(PAGE_SIZE as usize).collect(),
    })
}

#[derive(sqlx::FromRow)]
struct ReviewReply {
    body: String,
    status: String,
    revision: i64,
    article: Option<String>,
    slug: Option<String>,
    open: bool,
    own: bool,
}

#[derive(sqlx::FromRow)]
struct History {
    actor: Uuid,
    revision: i64,
    status: String,
    reason: String,
    time: String,
}

#[derive(Template)]
#[template(
    path = "content/discussion_review.html",
    config = "src/pages/askama.toml"
)]
struct ReviewPage<'a> {
    lang: &'a str,
    id: Uuid,
    reply: ReviewReply,
    decisions: Vec<&'static str>,
    csrf: String,
    history: Vec<History>,
    pages: Pages,
}

impl ReviewPage<'_> {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
    fn status(&self, status: &str) -> String {
        self.text(&format!("discussion-status-{status}"))
    }
    fn decision(&self, decision: &str) -> String {
        self.text(&format!("discussion-action-{decision}"))
    }
}

async fn detail(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Query(query): Query<Options>,
) -> Result<Html<String>, Error> {
    let offset = query.offset(PAGE_SIZE).map_err(|_| Error::Invalid)?;
    let reply: ReviewReply = sqlx::query_as(
        "SELECT r.body,r.status,r.revision,n.title AS article,n.slug,
                coalesce(NOT n.is_changelog AND n.published_at<=clock_timestamp()
                         AND (n.expires_at IS NULL OR n.expires_at>clock_timestamp()),false) AS open,
                r.author_id=$2 AS own
         FROM news_discussion_replies r LEFT JOIN news n ON n.id=r.news_id WHERE r.id=$1",
    )
    .bind(id)
    .bind(context.account)
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    let history: Vec<History> = sqlx::query_as(
        "SELECT actor,(after->>'revision')::bigint AS revision,after->>'status' AS status,
                after->>'reason' AS reason,to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI') AS time
         FROM audit_log WHERE action='news.discussion.moderated' AND target=$1
         ORDER BY id DESC LIMIT $2 OFFSET $3",
    )
    .bind(id.to_string())
    .bind(PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&pool)
    .await?;
    let decisions = match (reply.own, reply.status.as_str(), reply.open) {
        (false, "pending", true) => vec!["publish", "reject"],
        (false, "pending", false) => vec!["reject"],
        (false, "published", _) => vec!["hide"],
        _ => vec![],
    };
    render(ReviewPage {
        lang: query.lang(),
        id,
        reply,
        decisions,
        csrf: session.csrf_token().await?,
        pages: Pages::new(&query, history.len()),
        history: history.into_iter().take(PAGE_SIZE as usize).collect(),
    })
}

async fn review(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Query(query): Query<Options>,
    Form(form): Form<Review>,
) -> Result<Redirect, LocalizedError> {
    moderate(&pool, &auth, context, id, &form)
        .await
        .map_err(|error| failure(query.lang(), error))?;
    Ok(Redirect::to(&format!(
        "/admin/news/discussions/{id}?lang={}",
        query.lang()
    )))
}

struct LocalizedError {
    lang: &'static str,
    error: Error,
}

fn failure(lang: &str, error: Error) -> LocalizedError {
    let lang = if lang == "en" { "en" } else { "pl" };
    LocalizedError { lang, error }
}

impl IntoResponse for LocalizedError {
    fn into_response(self) -> Response {
        let key = match self.error {
            Error::Forbidden => "discussion-error-forbidden",
            Error::Invalid => "discussion-error-invalid",
            Error::Conflict => "discussion-error-conflict",
            Error::Limit => "discussion-error-limit",
            _ => return self.error.into_response(),
        };
        let title = label(self.lang, key);
        (
            self.error.into_response().status(),
            crate::news::page(self.lang, &title, String::new()),
        )
            .into_response()
    }
}

fn render(page: impl Template) -> Result<Html<String>, Error> {
    Ok(Html(page.render()?))
}
