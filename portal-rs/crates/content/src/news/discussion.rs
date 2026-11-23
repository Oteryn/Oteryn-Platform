//! Moderated, plain-text replies shared by the published translations of a news article.
mod web;
pub(super) use web::{public_routes, staff_routes};

use super::{language, slug_valid};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, MutationAuthorization, Permission, authorize_mutation_with_resources},
};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

const HOURLY_LIMIT: i64 = 5;
const MODERATE: &[Permission] = &[Permission::Access, Permission::ManageNews];

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("discussion or reply not found")]
    NotFound,
    #[error("discussion action forbidden")]
    Forbidden,
    #[error("invalid discussion input")]
    Invalid,
    #[error("discussion state changed")]
    Conflict,
    #[error("discussion posting limit reached")]
    Limit,
    #[error("discussion unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("staff authorization unavailable")]
    Staff(#[source] oteryn_admin::rbac::Error),
    #[error("discussion page unavailable")]
    Render(#[from] askama::Error),
}

impl From<oteryn_admin::rbac::Error> for Error {
    fn from(error: oteryn_admin::rbac::Error) -> Self {
        match error {
            oteryn_admin::rbac::Error::Denied => Self::Forbidden,
            other => Self::Staff(other),
        }
    }
}

fn plain_reply(body: &str) -> Result<&str, Error> {
    let body = body.trim();
    if (1..=2000).contains(&body.chars().count())
        && !body
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        && Parser::new(body).all(|event| {
            matches!(
                event,
                Event::Start(Tag::Paragraph)
                    | Event::End(TagEnd::Paragraph)
                    | Event::Text(_)
                    | Event::SoftBreak
                    | Event::HardBreak
            )
        })
    {
        Ok(body)
    } else {
        Err(Error::Invalid)
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Decision {
    Publish,
    Reject,
    Hide,
}

impl Decision {
    fn next(self, current: &str) -> Result<&'static str, Error> {
        match (self, current) {
            (Self::Publish, "pending") => Ok("published"),
            (Self::Reject, "pending") => Ok("rejected"),
            (Self::Hide, "published") => Ok("hidden"),
            _ => Err(Error::Conflict),
        }
    }
}

#[derive(sqlx::FromRow)]
struct Thread {
    id: Uuid,
    title: String,
}

async fn thread(conn: &mut PgConnection, lang: &str, slug: &str) -> Result<Thread, Error> {
    language(lang).map_err(|_| Error::NotFound)?;
    if !slug_valid(slug) {
        return Err(Error::NotFound);
    }
    sqlx::query_as(
        "SELECT id, CASE WHEN $1='pl' THEN title_pl ELSE title END AS title
         FROM news WHERE slug=$2 AND NOT is_changelog AND published_at<=clock_timestamp()
           AND ($1='en' OR (translation_published_at<=clock_timestamp()
                AND translation_source_updated_at>=updated_at
                AND length(trim(title_pl))>0 AND length(trim(body_pl))>0))
         FOR SHARE",
    )
    .bind(lang)
    .bind(slug)
    .fetch_optional(conn)
    .await?
    .ok_or(Error::NotFound)
}

async fn eligible(conn: &mut PgConnection, account: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active'
                       AND email_verified_at IS NOT NULL)
           AND NOT EXISTS(SELECT 1 FROM sanctions WHERE account_id=$1
               AND category='suspension' AND status='active'
               AND effective_at<=clock_timestamp()
               AND (expires_at IS NULL OR expires_at>clock_timestamp()))",
    )
    .bind(account)
    .fetch_one(conn)
    .await?)
}

#[derive(Deserialize)]
struct Edit {
    body: String,
    revision: i64,
}

enum Write<'a> {
    Reply(&'a str),
    Edit(Uuid, &'a Edit),
}

async fn write(
    pool: &PgPool,
    lang: &str,
    slug: &str,
    account: Uuid,
    session_hash: &[u8],
    change: Write<'_>,
) -> Result<(), Error> {
    let body = plain_reply(match change {
        Write::Reply(body) => body,
        Write::Edit(_, edit) => &edit.body,
    })?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    let thread = thread(&mut tx, lang, slug).await?;
    if !eligible(&mut tx, account).await? {
        return Err(Error::Forbidden);
    }

    match change {
        Write::Reply(_) => {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM news_discussion_replies
                 WHERE author_id=$1 AND created_at>clock_timestamp()-interval '1 hour'",
            )
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
            if count >= HOURLY_LIMIT {
                return Err(Error::Limit);
            }
            sqlx::query(
                "INSERT INTO news_discussion_replies(news_id,author_id,body) VALUES($1,$2,$3)",
            )
            .bind(thread.id)
            .bind(account)
            .bind(body)
            .execute(&mut *tx)
            .await?;
        }
        Write::Edit(id, edit) => {
            let current: Option<(String, i64)> = sqlx::query_as(
                "SELECT status,revision FROM news_discussion_replies
                 WHERE id=$1 AND news_id=$2 AND author_id=$3 FOR UPDATE",
            )
            .bind(id)
            .bind(thread.id)
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?;
            let (status, revision) = current.ok_or(Error::NotFound)?;
            if edit.revision <= 0 {
                return Err(Error::Invalid);
            }
            if status == "hidden" || revision != edit.revision {
                return Err(Error::Conflict);
            }
            sqlx::query(
                "UPDATE news_discussion_replies SET body=$2,status='pending',revision=revision+1
                 WHERE id=$1",
            )
            .bind(id)
            .bind(body)
            .execute(&mut *tx)
            .await?;
        }
    }

    let session: Option<Uuid> = sqlx::query_scalar(
        "SELECT session_id FROM web_sessions WHERE account_id=$1 AND id_hash=$2
           AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()
         FOR UPDATE",
    )
    .bind(account)
    .bind(session_hash)
    .fetch_optional(&mut *tx)
    .await?;
    if session.is_none() || !eligible(&mut tx, account).await? {
        return Err(Error::Forbidden);
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
struct Review {
    decision: Decision,
    revision: i64,
    reason: String,
}

async fn moderate(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Uuid,
    review: &Review,
) -> Result<(), Error> {
    let reason = review.reason.trim();
    if review.revision <= 0
        || !(1..=500).contains(&reason.chars().count())
        || reason.chars().any(char::is_control)
    {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "SELECT n.id FROM news n JOIN news_discussion_replies r ON r.news_id=n.id
                     WHERE r.id=$1 FOR SHARE OF n",
                )
                .bind(id)
                .execute(&mut *conn)
                .await?;
                sqlx::query("SELECT id FROM news_discussion_replies WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        },
        auth,
        context,
        MODERATE,
    )
    .await?;
    let (author, status, revision, open): (Uuid, String, i64, bool) = sqlx::query_as(
        "SELECT r.author_id,r.status,r.revision,
                coalesce(NOT n.is_changelog AND n.published_at<=clock_timestamp(),false)
         FROM news_discussion_replies r LEFT JOIN news n ON n.id=r.news_id WHERE r.id=$1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound)?;
    if author == context.account {
        return Err(Error::Forbidden);
    }
    if revision != review.revision {
        return Err(Error::Conflict);
    }
    let next = review.decision.next(&status)?;
    if next == "published" && !open {
        return Err(Error::Conflict);
    }
    sqlx::query("UPDATE news_discussion_replies SET status=$2,revision=revision+1 WHERE id=$1")
        .bind(id)
        .bind(next)
        .execute(&mut *tx)
        .await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "news.discussion.moderated",
            target: &id.to_string(),
            before: json!({ "status": status, "revision": revision }),
            after: json!({ "status": next, "revision": revision + 1, "reason": reason }),
        },
    )
    .await?;
    crate::revalidate(auth, &mut tx, context, MODERATE).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests;
