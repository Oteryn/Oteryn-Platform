//! Transactional registration with identical public results for existing addresses.
use crate::{
    accounts::{AccountId, Accounts, normalize_email},
    consents::{self, ConsentConfig},
    password, verify,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::IpAddr;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Registration {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub terms: Option<String>,
    pub privacy: Option<String>,
}
/// Field names and translation keys, never submitted credential values.
pub fn validate(input: &Registration, config: &ConsentConfig) -> Vec<(&'static str, &'static str)> {
    let mut errors = Vec::new();
    let email = normalize_email(&input.email);
    if email.len() > 254 || !email.is_ascii() || !email_address::EmailAddress::is_valid(&email) {
        errors.push(("email", "register-email-error"));
    }
    if !password::valid(&input.password) {
        errors.push(("password", "register-password-error"));
    }
    if input.password != input.password_confirmation {
        errors.push(("password_confirmation", "register-confirm-error"));
    }
    if input.terms.as_deref() != Some(config.terms_version.as_str()) {
        errors.push(("terms", "register-consent-error"));
    }
    if input.privacy.as_deref() != Some(config.privacy_version.as_str()) {
        errors.push(("privacy", "register-consent-error"));
    }
    errors
}
/// None means an existing account; callers must never authenticate that account.
pub async fn register(
    pool: &PgPool,
    input: Registration,
    config: &ConsentConfig,
    locale: &str,
    ip: Option<IpAddr>,
) -> Result<Option<AccountId>, sqlx::Error> {
    if !config.validate() || !validate(&input, config).is_empty() || !matches!(locale, "pl" | "en")
    {
        return Err(sqlx::Error::Protocol("invalid registration".into()));
    }
    crate::abuse::check_password(pool, &input.password)
        .await
        .map_err(|_| sqlx::Error::Protocol("registration protection refused".into()))?;
    // Hash on both paths, outside the asynchronous executor and before checking uniqueness.
    let hash = tokio::task::spawn_blocking(move || password::hash(&input.password))
        .await
        .map_err(|_| sqlx::Error::Protocol("password hashing unavailable".into()))?
        .map_err(sqlx::Error::Protocol)?;
    let mut tx = pool.begin().await?;
    let account = Accounts::create(&mut tx, &input.email, &hash, locale).await?;
    if let Some(account) = account {
        consents::record(&mut tx, account.id, config, ip).await?;
        verify::issue(&mut tx, account.id, locale).await?;
        tx.commit().await?;
        Ok(Some(account.id))
    } else {
        let id: AccountId = sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1")
            .bind(normalize_email(&input.email))
            .fetch_one(&mut *tx)
            .await?;
        let payload = serde_json::json!({"account_id":id,"locale":locale}).to_string();
        oteryn_db::outbox::enqueue(
            &mut tx,
            "email.account_exists",
            &payload,
            &uuid::Uuid::now_v7().to_string(),
            sqlx::types::time::OffsetDateTime::now_utc(),
            5,
        )
        .await?;
        tx.commit().await?;
        Ok(None)
    }
}

impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registration").finish_non_exhaustive()
    }
}
