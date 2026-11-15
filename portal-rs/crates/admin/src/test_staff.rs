//! Durable staff fixtures shared by database tests.
use oteryn_admin::rbac::SessionProof;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn proof(pool: &PgPool, actor: Uuid) -> Result<SessionProof, sqlx::Error> {
    let password: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(actor)
        .fetch_one(pool)
        .await?;
    let grant = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let session = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) ON CONFLICT(account_id) DO UPDATE SET confirmed=true RETURNING generation")
        .bind(actor).bind(Sha256::digest(Uuid::now_v7().as_bytes()).to_vec()).fetch_one(pool).await?;
    let grant_hash: [u8; 32] = Sha256::digest(grant.as_bytes()).into();
    let session_hash: [u8; 32] = Sha256::digest(session.as_bytes()).into();
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '1 hour')")
        .bind(grant_hash.as_slice()).bind(actor).bind(generation).bind(Sha256::digest(password.as_bytes()).to_vec()).execute(pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation) SELECT $1,id,now()+interval '1 hour',now()+interval '1 day','staff-fixture',$2,jsonb_build_object('mfa_grant',$3::text),native_security_generation FROM accounts WHERE id=$4")
        .bind(session_hash.as_slice()).bind(format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()))).bind(grant).bind(actor).execute(pool).await?;
    Ok(SessionProof {
        session_hash,
        grant_hash,
    })
}

pub async fn browser(pool: &PgPool, actor: Uuid, cookie: &str) -> Result<(), sqlx::Error> {
    let proof = proof(pool, actor).await?;
    let bearer = cookie.split_once('=').ok_or(sqlx::Error::RowNotFound)?.1;
    sqlx::query("UPDATE web_sessions target SET account_id=source.account_id,authorization_generation=source.authorization_generation,data=source.data FROM web_sessions source WHERE source.id_hash=$1 AND target.id_hash=$2")
        .bind(proof.session_hash.as_slice()).bind(Sha256::digest(bearer.as_bytes()).to_vec()).execute(pool).await?;
    Ok(())
}

/// Interleave the real operation with the account/fence half of a role writer.
/// The barrier is bound to this transaction's backend, never query text alone.
pub async fn race_account_before_fence<F, T>(
    pool: &PgPool,
    account: Uuid,
    operation: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    race_account(pool, account, operation, false).await
}
/// Revoke the durable session while a mutation waits for its actor account.
pub async fn race_session_revocation<F, T>(
    pool: &PgPool,
    account: Uuid,
    operation: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    race_account(pool, account, operation, true).await
}
async fn race_account<F, T>(
    pool: &PgPool,
    account: Uuid,
    operation: F,
    revoke: bool,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let mut writer = pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut *writer)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await?;
    let task = tokio::spawn(operation);
    tokio::time::timeout(std::time::Duration::from_secs(2),async {
        loop {
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *writer).await?;
            let waiting: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND $1=ANY(pg_blocking_pids(pid)))").bind(pid).fetch_one(&mut *writer).await?;
            if waiting { break Ok::<(),sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(71010)")
        .fetch_one(&mut *writer)
        .await?;
    if !free {
        return Err("operation acquired RBAC before its account lock".into());
    }
    if revoke {
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
            .bind(account)
            .execute(&mut *writer)
            .await?;
    }
    writer.commit().await?;
    Ok(tokio::time::timeout(std::time::Duration::from_secs(3), task).await??)
}
