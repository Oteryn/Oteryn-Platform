//! Password authentication. Call only after charging both login rate limits.
use crate::{
    AccountId,
    accounts::{Accounts, normalize_email},
    password,
};
use serde::Deserialize;
use sqlx::PgPool;

// SOURCE: app/Http/Requests/Identity/LoginIdentityRequest.php; non-credential timing fixture.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$M0Iyek83amFsS1J0cWNnLg$5wDTmQ1PJJorXDWlZfJnbjZkEATuPz4DMxaPpZ9azV0";

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials([redacted])")
    }
}
impl Credentials {
    pub fn valid(&self) -> bool {
        let email = normalize_email(&self.email);
        email.len() <= 254
            && email.is_ascii()
            && email_address::EmailAddress::is_valid(&email)
            && !self.password.is_empty()
            && self.password.len() <= 1024
    }
}
/// None is the same refusal for absent, disabled, unverified and MFA-protected accounts.
/// Until T06 supplies a challenge flow, MFA-protected accounts cannot create sessions here.
pub async fn authenticate(
    pool: &PgPool,
    input: Credentials,
) -> Result<Option<AccountId>, sqlx::Error> {
    if !input.valid() {
        return Ok(None);
    }
    let account = Accounts { pool: pool.clone() }
        .find_by_email(&input.email)
        .await?;
    let hash = account
        .as_ref()
        .map_or(DUMMY_HASH, |a| a.password_hash.as_str())
        .to_owned();
    let original = hash.clone();
    // The password verifier compares derived hashes in constant time. Missing accounts
    // perform the same policy-cost verification; CPU work never blocks the async executor.
    let (verified, replacement) = tokio::task::spawn_blocking(move || {
        let verified = password::verify(&input.password, &hash);
        let replacement = if verified && password::needs_rehash(&hash) {
            Some(password::hash(&input.password))
        } else {
            None
        };
        (verified, replacement)
    })
    .await
    .map_err(|_| sqlx::Error::Protocol("password verification unavailable".into()))?;
    let Some(account) = account.filter(|_| verified) else {
        return Ok(None);
    };
    let replacement = replacement.transpose().map_err(sqlx::Error::Protocol)?;
    // Compare-and-swap prevents a concurrent password change from being overwritten.
    // Recheck authorization after hashing, including the explicit MFA import guard.
    sqlx::query_scalar("UPDATE accounts SET password_hash=COALESCE($3,password_hash), updated_at=CASE WHEN $3::text IS NULL THEN updated_at ELSE now() END WHERE id=$1 AND password_hash=$2 AND status='active' AND email_verified_at IS NOT NULL AND login_mfa_required=false RETURNING id")
        .bind(account.id).bind(original).bind(replacement).fetch_optional(pool).await
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
