//! Small account repository shared by authentication flows.
use sqlx::{PgConnection, PgPool, types::time::OffsetDateTime};
pub type AccountId = uuid::Uuid;

/// Account credentials are deliberately excluded from Debug output.
#[derive(Clone, sqlx::FromRow)]
pub struct Account {
    pub id: AccountId,
    pub email: String,
    pub email_verified_at: Option<OffsetDateTime>,
    pub password_hash: String,
    pub status: String,
    pub locale: String,
    pub public_account_association: bool,
    pub public_status_visible: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}
#[derive(Debug, Clone)]
pub struct Accounts {
    pub pool: PgPool,
}
impl Accounts {
    pub async fn find_by_id(&self, id: AccountId) -> Result<Option<Account>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn find_by_email(&self, email: &str) -> Result<Option<Account>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM accounts WHERE lower(email)=$1")
            .bind(normalize_email(email))
            .fetch_optional(&self.pool)
            .await
    }
    /// A conflict returns None without aborting the caller's transaction.
    pub async fn create(
        conn: &mut PgConnection,
        email: &str,
        hash: &str,
        locale: &str,
    ) -> Result<Option<Account>, sqlx::Error> {
        sqlx::query_as("INSERT INTO accounts (email,password_hash,locale) VALUES ($1,$2,$3) ON CONFLICT (lower(email)) DO NOTHING RETURNING *")
            .bind(normalize_email(email)).bind(hash).bind(locale).fetch_optional(conn).await
    }
    /// Call with the account locked when consuming a verification token.
    pub async fn mark_verified(
        conn: &mut PgConnection,
        id: AccountId,
    ) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("UPDATE accounts SET email_verified_at=now(), status='active', updated_at=now() WHERE id=$1 AND status='pending_verification' AND email_verified_at IS NULL")
            .bind(id).execute(conn).await?.rows_affected()==1)
    }
}
