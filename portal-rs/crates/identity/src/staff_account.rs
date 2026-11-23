//! Trusted staff account edits. The caller supplies target locks, authorization and audit.
use crate::{AccountId, accounts, email_change, reset, sessions, verify};
use sqlx::{PgConnection, Postgres, Transaction};

#[derive(Debug, Clone, Copy)]
pub enum Field {
    Email,
    DisplayName,
    Locale,
    PublicAccountAssociation,
    PublicStatusVisible,
    CharacterProfilesHidden,
}

impl Field {
    pub const ALL: [Self; 6] = [
        Self::Email,
        Self::DisplayName,
        Self::Locale,
        Self::PublicAccountAssociation,
        Self::PublicStatusVisible,
        Self::CharacterProfilesHidden,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::DisplayName => "display-name",
            Self::Locale => "locale",
            Self::PublicAccountAssociation => "public-account-association",
            Self::PublicStatusVisible => "public-status-visible",
            Self::CharacterProfilesHidden => "character-profiles-hidden",
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct Details {
    email: String,
    display_name: String,
    locale: String,
    public_account_association: bool,
    public_status_visible: bool,
    character_profiles_hidden: bool,
}

impl std::fmt::Debug for Details {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Details").finish_non_exhaustive()
    }
}

impl Details {
    pub fn value(&self, field: Field) -> &str {
        match field {
            Field::Email => &self.email,
            Field::DisplayName => &self.display_name,
            Field::Locale => &self.locale,
            Field::PublicAccountAssociation => flag_value(self.public_account_association),
            Field::PublicStatusVisible => flag_value(self.public_status_visible),
            Field::CharacterProfilesHidden => flag_value(self.character_profiles_hidden),
        }
    }
}

fn flag_value(flag: bool) -> &'static str {
    if flag { "true" } else { "false" }
}

/// Only open player accounts outside a pending closure are eligible for edits.
pub async fn details(
    conn: &mut PgConnection,
    id: AccountId,
) -> Result<Option<Details>, sqlx::Error> {
    sqlx::query_as(
        "SELECT email,display_name,locale,public_account_association,public_status_visible,character_profiles_hidden
         FROM accounts
         WHERE id=$1 AND status IN ('pending_verification','active')
           AND NOT EXISTS (SELECT FROM account_terminations WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL)"
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid account field value")]
    BadInput,
    #[error("account field edit conflicts with current state")]
    Conflict,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

/// A normalized, validated value for one account field.
pub struct Edit {
    field: Field,
    value: String,
}

impl std::fmt::Debug for Edit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Edit")
            .field("field", &self.field)
            .finish_non_exhaustive()
    }
}

impl Edit {
    pub fn parse(field: Field, raw: &str) -> Result<Self, Error> {
        let value = raw.trim();
        let valid = match field {
            Field::Email => reset::valid_email(value),
            Field::DisplayName => value.chars().count() <= 80 && !raw.chars().any(char::is_control),
            Field::Locale => matches!(value, "pl" | "en"),
            _ => matches!(value, "true" | "false"),
        };
        if !valid {
            return Err(Error::BadInput);
        }
        Ok(Self {
            field,
            value: match field {
                Field::Email => accounts::normalize_email(value),
                _ => value.to_owned(),
            },
        })
    }

    /// Address claims precede account locks, as in registration and confirmation.
    pub async fn lock_address(&self, conn: &mut PgConnection) -> Result<(), sqlx::Error> {
        if matches!(self.field, Field::Email) {
            accounts::claim_lock(conn, &self.value).await?;
        }
        Ok(())
    }

    /// Returns whether anything changed. The caller holds the account lock.
    pub async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: AccountId,
        current: &Details,
    ) -> Result<bool, Error> {
        let previous = current.value(self.field);
        if previous == self.value
            || (matches!(self.field, Field::Email) && previous.eq_ignore_ascii_case(&self.value))
        {
            return Ok(false);
        }
        let statement = match self.field {
            Field::Email => {
                self.replace_email(tx, id, &current.locale).await?;
                return Ok(true);
            }
            Field::DisplayName => {
                "UPDATE accounts SET display_name=$2,updated_at=now() WHERE id=$1"
            }
            Field::Locale => "UPDATE accounts SET locale=$2,updated_at=now() WHERE id=$1",
            Field::PublicAccountAssociation => {
                "UPDATE accounts SET public_account_association=$2::text::boolean,privacy_revision=privacy_revision+1,updated_at=now() WHERE id=$1"
            }
            Field::PublicStatusVisible => {
                "UPDATE accounts SET public_status_visible=$2::text::boolean,privacy_revision=privacy_revision+1,updated_at=now() WHERE id=$1"
            }
            Field::CharacterProfilesHidden => {
                "UPDATE accounts SET character_profiles_hidden=$2::text::boolean,privacy_revision=privacy_revision+1,updated_at=now() WHERE id=$1"
            }
        };
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(id)
            .bind(&self.value)
            .execute(&mut **tx)
            .await?;
        Ok(true)
    }

    async fn replace_email(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: AccountId,
        locale: &str,
    ) -> Result<(), Error> {
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM account_backup_emails WHERE account_id=$1 AND lower(email)=$2)
                OR EXISTS (SELECT FROM account_email_changes WHERE account_id=$1 AND kind='primary'
                  AND applied_at IS NOT NULL AND reverted_at IS NULL AND revert_until>clock_timestamp())"
        )
        .bind(id)
        .bind(&self.value)
        .fetch_one(&mut **tx)
        .await?;
        if blocked || !accounts::address_available(tx, Some(id), &self.value).await? {
            return Err(Error::Conflict);
        }

        email_change::cancel_pending(tx, id).await?;
        sqlx::query(
            "UPDATE accounts SET email=$2,email_verified_at=NULL,status='pending_verification',
                email_change_blocked_until=greatest(email_change_blocked_until,now()+interval '72 hours'),
                native_security_generation=native_security_generation+1,updated_at=now()
             WHERE id=$1"
        )
        .bind(id)
        .bind(&self.value)
        .execute(&mut **tx)
        .await?;
        sessions::revoke_staff_credentials(tx, id).await?;
        verify::issue(tx, id, locale).await?;
        Ok(())
    }
}
