//! Session-bound access credentials scoped to the statistics watch API.
use super::{Subscription, http::unavailable};
use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn valid(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

async fn session_id(
    db: &mut PgConnection,
    account: Uuid,
    session_hash: &[u8],
) -> Result<Uuid, StatusCode> {
    let active: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR SHARE",
    )
    .bind(account)
    .fetch_optional(&mut *db)
    .await
    .map_err(unavailable)?;
    if active.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }
    sqlx::query_scalar(
        "SELECT session_id FROM web_sessions WHERE id_hash=$1 AND account_id=$2
         AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp() FOR UPDATE",
    )
    .bind(session_hash)
    .bind(account)
    .fetch_optional(db)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::FORBIDDEN)
}

async fn read_status(
    db: &mut PgConnection,
    session: Uuid,
    session_hash: &[u8],
    account: Uuid,
) -> Result<(bool, bool), StatusCode> {
    sqlx::query_as("SELECT
        EXISTS(SELECT 1 FROM stats_watch_access WHERE session_id=$1 AND session_hash=$2 AND expires_at>clock_timestamp()),
        EXISTS(SELECT 1 FROM stats_watches WHERE account_id=$3)")
        .bind(session).bind(session_hash).bind(account).fetch_one(db).await.map_err(unavailable)
}

pub async fn status(
    pool: &PgPool,
    account: Uuid,
    session_hash: &[u8],
) -> Result<(bool, bool), StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_db::mutation_budget(&mut tx)
        .await
        .map_err(unavailable)?;
    let session = session_id(&mut tx, account, session_hash).await?;
    let status = read_status(&mut tx, session, session_hash, account).await?;
    tx.commit().await.map_err(unavailable)?;
    Ok(status)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Issue,
    Revoke,
    RemoveAll,
}

pub async fn update(
    pool: &PgPool,
    account: Uuid,
    session_hash: &[u8],
    action: Action,
) -> Result<Option<(String, bool)>, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_db::mutation_budget(&mut tx)
        .await
        .map_err(unavailable)?;
    if matches!(action, Action::RemoveAll) {
        super::lock_subscriptions(&mut tx, account)
            .await
            .map_err(unavailable)?;
    }
    let session = session_id(&mut tx, account, session_hash).await?;
    let issued = match action {
        Action::Issue => {
            super::http::budget(&mut *tx, account).await?;
            let mut entropy = [0_u8; 32];
            getrandom::fill(&mut entropy).map_err(unavailable)?;
            let token = format!("{:x}", Sha256::digest(entropy));
            sqlx::query(
                "INSERT INTO stats_watch_access(session_id,session_hash,token_hash,expires_at)
                        VALUES($1,$2,$3,clock_timestamp()+interval '1 hour')
                        ON CONFLICT(session_id) DO UPDATE SET session_hash=EXCLUDED.session_hash,
                        token_hash=EXCLUDED.token_hash,expires_at=EXCLUDED.expires_at",
            )
            .bind(session)
            .bind(session_hash)
            .bind(hash(&token))
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            let (_, has_watches) = read_status(&mut tx, session, session_hash, account).await?;
            Some((token, has_watches))
        }
        Action::Revoke => {
            sqlx::query("DELETE FROM stats_watch_access WHERE session_id=$1")
                .bind(session)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            None
        }
        Action::RemoveAll => {
            sqlx::query("DELETE FROM stats_watches WHERE account_id=$1")
                .bind(account)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            None
        }
    };
    tx.commit().await.map_err(unavailable)?;
    Ok(issued)
}

pub async fn owner(pool: &PgPool, token: &str) -> Result<Option<Uuid>, StatusCode> {
    if !valid(token) {
        return Ok(None);
    }
    sqlx::query_scalar(
        "SELECT a.id FROM stats_watch_access w JOIN web_sessions s USING(session_id)
         JOIN accounts a ON a.id=s.account_id WHERE w.token_hash=$1 AND s.id_hash=w.session_hash
         AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp()
         AND w.expires_at>clock_timestamp() AND a.status='active' AND a.email_verified_at IS NOT NULL",
    )
    .bind(hash(token)).fetch_optional(pool).await.map_err(unavailable)
}

pub async fn remove(
    pool: &PgPool,
    actor: Uuid,
    token: &str,
    subscription: Subscription,
) -> Result<(), StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_db::mutation_budget(&mut tx)
        .await
        .map_err(unavailable)?;
    let token_hash = hash(token);
    let session_hash: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT session_hash FROM stats_watch_access WHERE token_hash=$1")
            .bind(&token_hash)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let session_hash = session_hash.ok_or(StatusCode::UNAUTHORIZED)?;
    let session = session_id(&mut tx, actor, &session_hash)
        .await
        .map_err(|code| {
            if code == StatusCode::FORBIDDEN {
                StatusCode::UNAUTHORIZED
            } else {
                code
            }
        })?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stats_watch_access WHERE session_id=$1 AND token_hash=$2 AND expires_at>clock_timestamp())")
        .bind(session).bind(token_hash).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if !active {
        return Err(StatusCode::UNAUTHORIZED);
    }
    super::delete(&mut *tx, actor, subscription)
        .await
        .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)
}
