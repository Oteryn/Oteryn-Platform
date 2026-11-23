use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub const PAGE_SIZE: i64 = 20;
const MAX_PAGE: u32 = 10_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sign in required")]
    SignIn,
    #[error("forum resource not found")]
    NotFound,
    #[error("invalid forum input")]
    Invalid,
    #[error("forum permission required")]
    Forbidden,
    #[error("verified account at least 24 hours old required")]
    Ineligible,
    #[error("public forum name required")]
    HandleRequired,
    #[error("public forum name is already in use")]
    HandleTaken,
    #[error("thread is locked")]
    Locked,
    #[error("forum state changed")]
    Conflict,
    #[error("forum limit reached")]
    Limit,
    #[error("forum storage unavailable")]
    Storage(#[from] sqlx::Error),
}

impl From<oteryn_admin::rbac::Error> for Error {
    fn from(value: oteryn_admin::rbac::Error) -> Self {
        match value {
            oteryn_admin::rbac::Error::Storage(error) => Self::Storage(error),
            _ => Self::Forbidden,
        }
    }
}

impl From<oteryn_support::reports::Error> for Error {
    fn from(value: oteryn_support::reports::Error) -> Self {
        use oteryn_support::reports::Error as Report;
        match value {
            Report::Disabled | Report::NotFound => Self::NotFound,
            Report::Invalid => Self::Invalid,
            Report::Forbidden | Report::UnjustifiedLimit => Self::Forbidden,
            Report::Conflict | Report::PendingLimit => Self::Conflict,
            Report::RateLimited => Self::Limit,
            Report::Storage(error) => Self::Storage(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Audience {
    Guest,
    Player(Uuid),
    Staff(Uuid),
}

impl Audience {
    pub fn account(self) -> Option<Uuid> {
        match self {
            Self::Guest => None,
            Self::Player(id) | Self::Staff(id) => Some(id),
        }
    }

    pub fn staff(self) -> bool {
        matches!(self, Self::Staff(_))
    }

    fn sees(self, visibility: &str) -> bool {
        match visibility {
            "public" => true,
            "signed-in" => self.account().is_some(),
            "staff" => self.staff(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Board {
    pub id: Uuid,
    pub name_pl: String,
    pub name_en: String,
    pub position: i32,
    pub visibility: String,
    pub version: i64,
}

impl Board {
    pub fn name(&self, lang: &str) -> &str {
        if lang == "pl" {
            &self.name_pl
        } else {
            &self.name_en
        }
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub locked: bool,
    pub pinned: bool,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Topic {
    pub id: Uuid,
    pub title: String,
    pub locked: bool,
    pub pinned: bool,
    pub removed: bool,
    pub unread: bool,
    pub replies: i64,
    pub activity_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Post {
    pub id: Uuid,
    pub author: Option<String>,
    pub body: String,
    pub quote_id: Option<Uuid>,
    pub quote_author: Option<String>,
    pub quote_body: Option<String>,
    pub created_at: OffsetDateTime,
    pub edited_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
    pub editable: bool,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct Paging {
    #[serde(default = "first_page")]
    pub page: u32,
    pub quote: Option<Uuid>,
    pub report: Option<Uuid>,
    pub lang: Option<String>,
}

fn first_page() -> u32 {
    1
}

impl Paging {
    pub fn offset(&self) -> Result<i64, Error> {
        Ok(i64::from(page_number(self.page)? - 1) * PAGE_SIZE)
    }

    pub fn has_next(&self, rows: usize) -> bool {
        self.page < MAX_PAGE && rows > PAGE_SIZE as usize
    }
}

pub fn page_number(page: u32) -> Result<u32, Error> {
    if (1..=MAX_PAGE).contains(&page) {
        Ok(page)
    } else {
        Err(Error::Invalid)
    }
}

pub fn text(value: &str, limit: usize, multiline: bool) -> Result<&str, Error> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > limit
        || value
            .chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        Err(Error::Invalid)
    } else {
        Ok(value)
    }
}

pub fn handle(value: &str) -> Result<&str, Error> {
    let value = value.trim();
    if (3..=24).contains(&value.len())
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        Ok(value)
    } else {
        Err(Error::Invalid)
    }
}

pub async fn live_account(pool: &PgPool, account: Uuid, session: &[u8]) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM accounts a JOIN web_sessions s ON s.account_id=a.id
          WHERE a.id=$1 AND s.id_hash=$2 AND a.status IN ('active','pending_verification')
            AND s.authorization_generation=a.native_security_generation
            AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp())",
    )
    .bind(account)
    .bind(session)
    .fetch_one(pool)
    .await?)
}

pub async fn charge_mutation(pool: &PgPool, account: Uuid) -> Result<(), Error> {
    let attempt: Option<i32> = sqlx::query_scalar(
        "INSERT INTO forum_mutation_limits(account_id, window_starts, attempts)
         VALUES($1, clock_timestamp(), 1)
         ON CONFLICT(account_id) DO UPDATE SET
           attempts=CASE WHEN forum_mutation_limits.window_starts <= excluded.window_starts-interval '1 minute'
                         THEN 1 ELSE forum_mutation_limits.attempts+1 END,
           window_starts=CASE WHEN forum_mutation_limits.window_starts <= excluded.window_starts-interval '1 minute'
                              THEN excluded.window_starts ELSE forum_mutation_limits.window_starts END
         WHERE forum_mutation_limits.window_starts <= excluded.window_starts-interval '1 minute'
            OR forum_mutation_limits.attempts < 30
         RETURNING attempts",
    ).bind(account).fetch_optional(pool).await?;
    attempt.map(|_| ()).ok_or(Error::Limit)
}

/// Revalidate the currently authenticated session while holding the account lock.
pub async fn player_account(
    db: &mut PgConnection,
    account: Uuid,
    session: &[u8],
    verified: bool,
) -> Result<(), Error> {
    if sqlx::query_scalar::<_, Uuid>("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .fetch_optional(&mut *db)
        .await?
        .is_none()
    {
        return Err(Error::SignIn);
    }
    let active: Option<(bool, bool)> = sqlx::query_as(
        "SELECT a.status='active' AND a.email_verified_at IS NOT NULL,
                a.created_at <= clock_timestamp()-interval '24 hours'
           FROM accounts a
          WHERE a.id=$1 AND a.status IN ('active','pending_verification')
            AND EXISTS (SELECT 1 FROM web_sessions s WHERE s.account_id=a.id AND s.id_hash=$2
                 AND s.authorization_generation=a.native_security_generation
                 AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp()
                 FOR SHARE)",
    )
    .bind(account)
    .bind(session)
    .fetch_optional(db)
    .await?;
    match active {
        None => Err(Error::SignIn),
        Some((false, _) | (_, false)) if verified => Err(Error::Ineligible),
        Some(_) => Ok(()),
    }
}

pub async fn posting_account(db: &mut PgConnection, account: Uuid) -> Result<(), Error> {
    let (eligible, named, limited): (bool, bool, bool) = sqlx::query_as(
        "SELECT status='active' AND email_verified_at IS NOT NULL
                    AND created_at <= clock_timestamp()-interval '24 hours',
                EXISTS(SELECT 1 FROM forum_profiles WHERE account_id=$1),
                (SELECT count(*) >= 5 FROM (
                    SELECT 1 FROM forum_posts WHERE author_id=$1
                    AND created_at > clock_timestamp()-interval '1 minute' LIMIT 5
                ) recent)
           FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(db)
    .await?;
    if !eligible {
        Err(Error::Ineligible)
    } else if !named {
        Err(Error::HandleRequired)
    } else if limited {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}

pub async fn boards(db: &PgPool, audience: Audience) -> Result<Vec<Board>, Error> {
    Ok(sqlx::query_as(
        "SELECT * FROM forum_boards
          WHERE visibility='public' OR ($1 AND visibility='signed-in') OR $2
          ORDER BY position,id",
    )
    .bind(audience.account().is_some())
    .bind(audience.staff())
    .fetch_all(db)
    .await?)
}

pub async fn board(db: &mut PgConnection, audience: Audience, id: Uuid) -> Result<Board, Error> {
    let board: Board = sqlx::query_as("SELECT * FROM forum_boards WHERE id=$1 FOR SHARE")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)?;
    if audience.sees(&board.visibility) {
        Ok(board)
    } else {
        Err(Error::NotFound)
    }
}

/// The thread lock serializes replies, moderation and post positions.
pub async fn thread(
    db: &mut PgConnection,
    audience: Audience,
    id: Uuid,
    write: bool,
) -> Result<(Thread, Board), Error> {
    let lookup = if write {
        "SELECT * FROM forum_threads WHERE id=$1 FOR UPDATE"
    } else {
        "SELECT * FROM forum_threads WHERE id=$1 FOR SHARE"
    };
    let thread: Thread = sqlx::query_as(sqlx::AssertSqlSafe(lookup))
        .bind(id)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::NotFound)?;
    if !audience.staff() && thread.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    let board = board(db, audience, thread.board_id).await?;
    Ok((thread, board))
}

pub async fn topics(
    db: &mut PgConnection,
    audience: Audience,
    board: Uuid,
    offset: i64,
) -> Result<Vec<Topic>, Error> {
    Ok(sqlx::query_as(
        "SELECT t.id,t.title,t.locked,t.pinned,t.deleted_at IS NOT NULL AS removed,
                coalesce(p.last_position > coalesce(r.position,0) AND $2 IS NOT NULL AND NOT $3,false) AS unread,
                p.replies,t.activity_at
           FROM (SELECT * FROM forum_threads WHERE board_id=$1 AND ($3 OR deleted_at IS NULL)
                 ORDER BY pinned DESC,activity_at DESC,id DESC LIMIT $4 OFFSET $5) t
           LEFT JOIN forum_read_markers r ON r.thread_id=t.id AND r.account_id=$2
           CROSS JOIN LATERAL (
             SELECT max(position) AS last_position,count(*) FILTER (WHERE position>1) AS replies
               FROM forum_posts WHERE thread_id=t.id AND deleted_at IS NULL
           ) p
          ORDER BY t.pinned DESC,t.activity_at DESC,t.id DESC",
    ).bind(board).bind(audience.account()).bind(audience.staff())
    .bind(PAGE_SIZE + 1).bind(offset).fetch_all(db).await?)
}

pub async fn posts(
    db: &mut PgConnection,
    audience: Audience,
    thread: Uuid,
    offset: i64,
) -> Result<Vec<Post>, Error> {
    Ok(sqlx::query_as(
        "SELECT p.*, n.public_handle AS author,
                qn.public_handle AS quote_author, left(q.body,400) AS quote_body,
                coalesce(p.author_id=$2 AND p.created_at > clock_timestamp()-interval '15 minutes'
                    AND p.deleted_at IS NULL,false) AS editable
           FROM forum_posts p
           JOIN accounts a ON a.id=p.author_id
           LEFT JOIN forum_profiles n ON n.account_id=a.id AND a.status<>'closed'
           LEFT JOIN forum_posts q ON q.thread_id=p.thread_id AND q.id=p.quote_id AND q.deleted_at IS NULL
           LEFT JOIN accounts qa ON qa.id=q.author_id
           LEFT JOIN forum_profiles qn ON qn.account_id=qa.id AND qa.status<>'closed'
          WHERE p.thread_id=$1 AND ($3 OR p.deleted_at IS NULL)
          ORDER BY p.position LIMIT $4 OFFSET $5",
    ).bind(thread).bind(audience.account()).bind(audience.staff()).bind(PAGE_SIZE + 1)
    .bind(offset).fetch_all(db).await?)
}

pub async fn mark_read(
    db: &mut PgConnection,
    account: Uuid,
    thread: Uuid,
    position: i64,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO forum_read_markers(account_id,thread_id,position) VALUES($1,$2,$3)
         ON CONFLICT(account_id,thread_id) DO UPDATE SET position=excluded.position
         WHERE forum_read_markers.position<excluded.position",
    )
    .bind(account)
    .bind(thread)
    .bind(position)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn permalink(
    db: &mut PgConnection,
    audience: Audience,
    thread: Uuid,
    post: Uuid,
) -> Result<i64, Error> {
    let page = sqlx::query_scalar(
        "SELECT 1 + (SELECT count(*) FROM forum_posts p WHERE p.thread_id=q.thread_id
                   AND p.position<q.position AND ($3 OR p.deleted_at IS NULL)) / $4
         FROM forum_posts q WHERE q.thread_id=$1 AND q.id=$2 AND ($3 OR q.deleted_at IS NULL)",
    )
    .bind(thread)
    .bind(post)
    .bind(audience.staff())
    .bind(PAGE_SIZE)
    .fetch_optional(db)
    .await?;
    page.ok_or(Error::NotFound)
}

pub async fn create_thread(
    db: &mut PgConnection,
    board: Uuid,
    account: Uuid,
    title: &str,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar(
        "INSERT INTO forum_threads(board_id, author_id, title) VALUES($1,$2,$3) RETURNING id",
    )
    .bind(board)
    .bind(account)
    .bind(text(title, 160, false)?)
    .fetch_one(db)
    .await?)
}

pub async fn add_post(
    db: &mut PgConnection,
    thread: Uuid,
    account: Uuid,
    body: &str,
    quote: Option<Uuid>,
) -> Result<(Uuid, OffsetDateTime), Error> {
    let body = text(body, 8000, true)?;
    if let Some(quote) = quote {
        permalink(db, Audience::Player(account), thread, quote).await?;
    }
    let position: i64 = sqlx::query_scalar(
        "UPDATE forum_threads SET last_position=last_position+1,activity_at=clock_timestamp()
         WHERE id=$1 RETURNING last_position",
    )
    .bind(thread)
    .fetch_one(&mut *db)
    .await?;
    Ok(sqlx::query_as(
        "INSERT INTO forum_posts(thread_id,author_id,position,body,quote_id)
         VALUES($1,$2,$3,$4,$5) RETURNING id,created_at",
    )
    .bind(thread)
    .bind(account)
    .bind(position)
    .bind(body)
    .bind(quote)
    .fetch_one(db)
    .await?)
}

pub async fn edit_own(
    db: &mut PgConnection,
    thread: Uuid,
    post: Uuid,
    account: Uuid,
    body: &str,
    version: i64,
) -> Result<(), Error> {
    let body = text(body, 8000, true)?;
    let current: Option<(Uuid, bool, i64)> = sqlx::query_as(
        "SELECT author_id,created_at > clock_timestamp()-interval '15 minutes', version
         FROM forum_posts WHERE id=$1 AND thread_id=$2 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(post)
    .bind(thread)
    .fetch_optional(&mut *db)
    .await?;
    let (author, within_window, current_version) = current.ok_or(Error::NotFound)?;
    if author != account || !within_window {
        return Err(Error::Forbidden);
    }
    if current_version != version {
        return Err(Error::Conflict);
    }
    sqlx::query(
        "UPDATE forum_posts SET body=$2,edited_at=clock_timestamp(),version=version+1 WHERE id=$1",
    )
    .bind(post)
    .bind(body)
    .execute(db)
    .await?;
    Ok(())
}
