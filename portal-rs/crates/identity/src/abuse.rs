//! Registration abuse protection: a default-off policy gate and proof-of-work.
//!
//! Registration is disabled by default in `registration_abuse_policy`. The password
//! lookup is the shared breached-password check in [`crate::breach`] (HIBP range API
//! with a 30-day PostgreSQL cache, owner decision 2026-11-21): a breached password is
//! refused and an unavailable corpus denies registration; neither is ever accepted.
//! SHA-1 is used only for corpus lookup, never password storage (which uses Argon2id).
//!
//! Work: SHA-256 of `token:nonce` must begin with the configured number of zero bits.
//! Tokens contain 256 random bits, use the policy lifetime, and bind to the form's
//! CSRF token. Atomic DELETE consumes a valid solution once across all workers. Issuance
//! replaces any outstanding challenge for that form. Expired rows are pruned on issue.
//! HTTP adapters must enforce CSRF before calling issue/consume and must not log proofs.
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub mod policy;
use policy::Policy;

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

async fn load_policy(pool: &PgPool) -> Result<Policy, Error> {
    policy::read(pool)
        .await
        .map(|row| row.values)
        .map_err(|_| Error::Unavailable)
}

pub async fn enabled(pool: &PgPool) -> Result<(), Error> {
    if load_policy(pool).await?.enabled {
        Ok(())
    } else {
        Err(Error::Disabled)
    }
}

/// Issue at most one outstanding challenge per form, serialized across instances.
pub async fn issue(pool: &PgPool, csrf: &str) -> Result<String, Error> {
    let policy = load_policy(pool).await?;
    issue_with_policy(pool, csrf, &policy).await
}

async fn issue_with_policy(pool: &PgPool, csrf: &str, policy: &Policy) -> Result<String, Error> {
    if !policy.enabled {
        return Err(Error::Disabled);
    }
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
    sqlx::query("INSERT INTO registration_work(token_hash,form_hash,expires_at) VALUES ($1,$2,now()+make_interval(secs => $3))")
        .bind(Sha256::digest(token.as_bytes()).to_vec())
        .bind(binding)
        .bind(f64::from(policy.work_ttl_seconds))
        .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(token)
}

pub fn valid_solution(token: &str, nonce: &str) -> bool {
    solution_meets(token, nonce, u32::from(DIFFICULTY))
}

fn solution_meets(token: &str, nonce: &str, bits: u32) -> bool {
    if !valid_hex(token, 64)
        || nonce.is_empty()
        || nonce.len() > 16
        || !nonce.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let hash = Sha256::digest(format!("{token}:{nonce}").as_bytes());
    u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]).leading_zeros() >= bits
}

pub async fn consume(pool: &PgPool, csrf: &str, token: &str, nonce: &str) -> Result<(), Error> {
    let policy = load_policy(pool).await?;
    if !policy.enabled {
        return Err(Error::Disabled);
    }
    let binding = form_hash(csrf)?;
    if !solution_meets(token, nonce, policy.work_bits as u32) {
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
    check_password_with(
        pool,
        &crate::breach::Checker::from_env(pool.clone()),
        password,
    )
    .await
}

/// [`check_password`] with an explicit range source (tests use a local mock).
pub async fn check_password_with(
    pool: &PgPool,
    checker: &crate::breach::Checker,
    password: &str,
) -> Result<(), Error> {
    enabled(pool).await?;
    if !crate::password::valid(password) {
        return Err(Error::Invalid);
    }
    crate::breach::check(checker, password)
        .await
        .map_err(|error| match error {
            crate::breach::BreachError::Breached => Error::Breached,
            crate::breach::BreachError::Unavailable => Error::Unavailable,
        })
}

pub mod http;
#[cfg(test)]
mod tests;
