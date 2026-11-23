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
    pub native_security_generation: i64,
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

/// Validate a bare address for an account lookup or delivery.
pub fn valid_bare_email(email: &str) -> bool {
    email.len() <= 254
        && email.is_ascii()
        && email_address::EmailAddress::parse_with_options(
            email,
            email_address::Options::default().without_display_text(),
        )
        .is_ok()
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
    /// Resolve an account using the caller's transaction connection.
    pub async fn find_by_id_in(
        conn: &mut PgConnection,
        id: AccountId,
    ) -> Result<Option<Account>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_optional(conn)
            .await
    }
    pub async fn find_by_email(&self, email: &str) -> Result<Option<Account>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM accounts WHERE lower(email)=$1")
            .bind(normalize_email(email))
            .fetch_optional(&self.pool)
            .await
    }
    /// Call inside a transaction. An address reservation returns None without aborting it.
    pub async fn create(
        conn: &mut PgConnection,
        email: &str,
        hash: &str,
        locale: &str,
    ) -> Result<Option<Account>, sqlx::Error> {
        let email = normalize_email(email);
        claim_lock(conn, &email).await?;
        if !address_available(conn, None, &email).await? {
            return Ok(None);
        }
        sqlx::query_as("INSERT INTO accounts (email,password_hash,locale) VALUES ($1,$2,$3) ON CONFLICT (lower(email)) DO NOTHING RETURNING *")
            .bind(&email).bind(hash).bind(locale).fetch_optional(conn).await
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

/// Serialize primary, pending and backup address claims on the normalized address.
pub async fn claim_lock(conn: &mut PgConnection, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(normalize_email(email))
        .execute(conn)
        .await?;
    Ok(())
}
/// An address is taken while another account holds it as primary or backup,
/// claims it in an owner-authorized pending change, or may still revert to it as the old
/// primary address of an applied change.
pub async fn address_available(
    conn: &mut PgConnection,
    owner: Option<AccountId>,
    email: &str,
) -> Result<bool, sqlx::Error> {
    // Closed-account tombstones have no domain and cannot be claimed.
    if !email.contains('@') {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM accounts WHERE lower(email)=lower($2) AND id IS DISTINCT FROM $1) AND NOT EXISTS(SELECT 1 FROM account_backup_emails b JOIN accounts a ON a.id=b.account_id WHERE a.status<>'closed' AND lower(b.email)=lower($2) AND b.account_id IS DISTINCT FROM $1) AND NOT EXISTS(SELECT 1 FROM account_email_changes WHERE lower(new_email)=lower($2) AND account_id IS DISTINCT FROM $1 AND (NOT initiated_by_staff OR owner_approved_at IS NOT NULL) AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL AND expires_at>clock_timestamp()) AND NOT EXISTS(SELECT 1 FROM account_email_changes WHERE kind='primary' AND lower(old_email)=lower($2) AND account_id IS DISTINCT FROM $1 AND applied_at IS NOT NULL AND reverted_at IS NULL AND revert_until>clock_timestamp())")
        .bind(owner).bind(email).fetch_one(conn).await
}
