//! Registration abuse protection without external services.
//!
//! Registration is disabled by default in `registration_abuse_policy`. Operators must
//! atomically import COMPLETE SHA-1 password ranges (five uppercase hex prefix digits,
//! 35-digit uppercase suffixes with positive breach counts removed) from a trusted
//! breach corpus into `registration_password_ranges`, with a bounded freshness expiry.
//! An empty suffix array means a confirmed empty range, never a failed import.
//! Missing, stale, malformed or unavailable ranges deny registration. No password,
//! full hash, or prefix leaves the process; there is no external fallback or API flag.
//! SHA-1 is used only for corpus lookup, never password storage (which uses Argon2id).
//!
//! Work: SHA-256 of `token:nonce` must begin with two zero bytes (16 bits). Tokens
//! contain 256 random bits, expire after five minutes, and bind to the form's CSRF
//! token. Atomic DELETE consumes a valid solution once across all workers. Issuance
//! replaces any outstanding challenge for that form. Expired rows are pruned on issue.
//! HTTP adapters must enforce CSRF before calling issue/consume and must not log proofs.
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub const DIFFICULTY: u8 = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Invalid,
    Breached,
    Unavailable,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("registration protection refused")
    }
}
impl std::error::Error for Error {}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02X}");
        output
    })
}
fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}
fn form_hash(csrf: &str) -> Result<Vec<u8>, Error> {
    if csrf.len() != 64 || !csrf.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid);
    }
    Ok(Sha256::digest(csrf.as_bytes()).to_vec())
}

pub async fn enabled(pool: &PgPool) -> Result<(), Error> {
    match sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM registration_abuse_policy WHERE singleton=true",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(true)) => Ok(()),
        Ok(Some(false)) => Err(Error::Disabled),
        _ => Err(Error::Unavailable),
    }
}

/// Issue at most one outstanding challenge per form, serialized across instances.
pub async fn issue(pool: &PgPool, csrf: &str) -> Result<String, Error> {
    enabled(pool).await?;
    let binding = form_hash(csrf)?;
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random).map_err(|_| Error::Unavailable)?;
    let token = hex(&random);
    let mut tx = pool.begin().await.map_err(|_| Error::Unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(csrf)
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    sqlx::query("DELETE FROM registration_work WHERE form_hash=$1 OR expires_at<=now()")
        .bind(&binding)
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    sqlx::query("INSERT INTO registration_work(token_hash,form_hash,expires_at) VALUES ($1,$2,now()+interval '5 minutes')")
        .bind(Sha256::digest(token.as_bytes()).to_vec())
        .bind(binding)
        .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(token)
}

pub fn valid_solution(token: &str, nonce: &str) -> bool {
    if !valid_hex(token, 64)
        || nonce.is_empty()
        || nonce.len() > 16
        || !nonce.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let hash = Sha256::digest(format!("{token}:{nonce}").as_bytes());
    hash[0] == 0 && hash[1] == 0
}

pub async fn consume(pool: &PgPool, csrf: &str, token: &str, nonce: &str) -> Result<(), Error> {
    enabled(pool).await?;
    let binding = form_hash(csrf)?;
    if !valid_solution(token, nonce) {
        return Err(Error::Invalid);
    }
    let result = sqlx::query(
        "DELETE FROM registration_work WHERE token_hash=$1 AND form_hash=$2 AND expires_at>now()",
    )
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .bind(binding)
    .execute(pool)
    .await
    .map_err(|_| Error::Unavailable)?;
    if result.rows_affected() == 1 {
        Ok(())
    } else {
        Err(Error::Invalid)
    }
}

/// Fail closed before Argon2 work, account lookup or registration side effects.
pub async fn check_password(pool: &PgPool, password: &str) -> Result<(), Error> {
    enabled(pool).await?;
    if !crate::password::valid(password) {
        return Err(Error::Invalid);
    }
    let digest = hex(&sha1::Sha1::digest(password.as_bytes()));
    let suffixes: Option<Vec<String>> = sqlx::query_scalar(
        "SELECT suffixes FROM registration_password_ranges WHERE prefix=$1 AND expires_at>now()",
    )
    .bind(&digest[..5])
    .fetch_optional(pool)
    .await
    .map_err(|_| Error::Unavailable)?;
    let suffixes = suffixes.ok_or(Error::Unavailable)?;
    if suffixes.iter().any(|s| !valid_hex(s, 35)) {
        return Err(Error::Unavailable);
    }
    if suffixes.iter().any(|s| s == &digest[5..]) {
        Err(Error::Breached)
    } else {
        Ok(())
    }
}

pub mod http;
#[cfg(test)]
mod tests;
