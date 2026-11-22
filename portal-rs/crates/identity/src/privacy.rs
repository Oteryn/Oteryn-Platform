//! Account privacy is a current, account-wide upper bound on public presentation.
//!
//! Implements the privacy overlay in the Platform public game data projection contract.
//! Public consumers must combine a fresh, unique ownership proof with `current` at
//! serving time. An absent result or a database error denies both visibility fields.
use crate::AccountId;
use sqlx::PgPool;

#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Preferences {
    pub public_account_association: bool,
    pub public_status_visible: bool,
    pub locale: String,
    pub privacy_revision: i64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Privacy {
    pub enabled: bool,
}

#[derive(Debug)]
pub struct SyntheticLocale<'a> {
    pub initial_locale: &'a str,
    pub association: bool,
    pub status: bool,
    pub token: uuid::Uuid,
    pub preview_locale: Option<&'a str>,
}

impl Privacy {
    /// Read at serving time. Missing, inactive or disabled state never grants visibility.
    /// Consumers must prove current ownership independently and must not cache an allow.
    pub async fn current(
        &self,
        pool: &PgPool,
        account: AccountId,
    ) -> Result<Option<Preferences>, sqlx::Error> {
        self.read(pool, account, false).await
    }

    pub async fn settings(
        &self,
        pool: &PgPool,
        account: AccountId,
    ) -> Result<Option<Preferences>, sqlx::Error> {
        self.read(pool, account, true).await
    }

    async fn read(
        &self,
        pool: &PgPool,
        account: AccountId,
        show_preview: bool,
    ) -> Result<Option<Preferences>, sqlx::Error> {
        if !self.enabled {
            return Ok(None);
        }
        sqlx::query_as(
            "SELECT public_account_association, public_status_visible, privacy_revision,
                    CASE WHEN $2 AND synthetic_privacy_locale_expires_at > statement_timestamp()
                         THEN synthetic_privacy_locale ELSE locale END AS locale
             FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL",
        )
        .bind(account)
        .bind(show_preview)
        .fetch_optional(pool)
        .await
    }

    /// The persisted preference is the fallback. Even a transaction that commits
    /// after its deadline cannot expose an expired preview to another request.
    pub async fn synthetic_locale(
        &self,
        pool: &PgPool,
        actor: crate::mutation_actor::MutationActor,
        change: SyntheticLocale<'_>,
    ) -> Result<bool, sqlx::Error> {
        if !self.enabled
            || !matches!(change.initial_locale, "pl" | "en")
            || change.token.is_nil()
            || change.preview_locale.is_some_and(|locale| {
                !matches!(locale, "pl" | "en") || locale == change.initial_locale
            })
        {
            return Ok(false);
        }

        let mut tx = pool.begin().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(actor.account)
            .execute(&mut *tx)
            .await?;
        if !actor.live(&mut tx).await? {
            return Ok(false);
        }
        let changed = sqlx::query(
            "UPDATE accounts SET
                 synthetic_privacy_locale=$6,
                 synthetic_privacy_locale_expires_at=CASE WHEN $6 IS NOT NULL
                     THEN transaction_timestamp() + interval '30 seconds' END,
                 synthetic_privacy_locale_token=CASE WHEN $6 IS NOT NULL THEN $5::uuid END
             WHERE id=$1 AND locale=$2 AND public_account_association=$3
               AND public_status_visible=$4 AND status='active' AND email_verified_at IS NOT NULL
               AND (($6 IS NULL AND synthetic_privacy_locale_token=$5)
                    OR ($6 IS NOT NULL AND
                        (synthetic_privacy_locale_expires_at IS NULL
                         OR synthetic_privacy_locale_expires_at <= statement_timestamp())
                        AND transaction_timestamp() + interval '30 seconds' > statement_timestamp()))",
        )
        .bind(actor.account)
        .bind(change.initial_locale)
        .bind(change.association)
        .bind(change.status)
        .bind(change.token)
        .bind(change.preview_locale)
        .execute(&mut *tx)
        .await?
        .rows_affected() == 1;
        tx.commit().await?;
        Ok(changed)
    }

    /// The caller supplies the authenticated session's account, never a form account id.
    /// The row update serializes concurrent changes and advances the privacy fence atomically.
    pub async fn update(
        &self,
        pool: &PgPool,
        account: impl Into<crate::mutation_actor::MutationActor>,
        association: bool,
        status: bool,
        locale: &str,
    ) -> Result<Option<Preferences>, sqlx::Error> {
        let actor = account.into();
        let account = actor.account;
        if !self.enabled || !matches!(locale, "pl" | "en") {
            return Ok(None);
        }
        let mut tx = pool.begin().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .execute(&mut *tx)
            .await?;
        if !actor.live(&mut tx).await? {
            return Ok(None);
        }

        let saved: Option<Preferences> = sqlx::query_as("UPDATE accounts SET public_account_association=$2,public_status_visible=$3,locale=$4,synthetic_privacy_locale=NULL,synthetic_privacy_locale_expires_at=NULL,synthetic_privacy_locale_token=NULL,privacy_revision=privacy_revision+1,updated_at=now() WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL RETURNING public_account_association,public_status_visible,locale,privacy_revision")
            .bind(account).bind(association).bind(status).bind(locale).fetch_optional(&mut *tx).await?;
        if let Some(ref prefs) = saved {
            sqlx::query("INSERT INTO account_privacy_events (account_id,revision) VALUES ($1,$2)")
                .bind(account)
                .bind(prefs.privacy_revision)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn synthetic_locale_preview_restores_and_late_commit_expires()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping privacy PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let (pool, admin, schema) = test_support::database(&url).await?;
        let id = uuid::Uuid::now_v7();
        let token = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO accounts (id,email,password_hash,locale,status,email_verified_at) VALUES ($1,$2,'test','en','active',now())")
            .bind(id).bind(format!("{id}@example.test")).execute(&pool).await?;
        let policy = Privacy { enabled: true };
        let change = |token, preview_locale| SyntheticLocale {
            initial_locale: "en",
            association: false,
            status: false,
            token,
            preview_locale,
        };
        let shown =
            || async { Ok::<_, sqlx::Error>(policy.settings(&pool, id).await?.map(|p| p.locale)) };
        assert!(
            policy
                .synthetic_locale(&pool, id.into(), change(token, Some("pl")))
                .await?
        );
        assert_eq!(shown().await?.as_deref(), Some("pl"));
        assert_eq!(
            policy
                .current(&pool, id)
                .await?
                .ok_or("missing account")?
                .locale,
            "en"
        );
        assert!(
            !policy
                .synthetic_locale(&pool, id.into(), change(uuid::Uuid::now_v7(), None))
                .await?
        );
        assert!(
            policy
                .synthetic_locale(&pool, id.into(), change(token, None))
                .await?
        );
        assert_eq!(shown().await?.as_deref(), Some("en"));

        // A read racing the write sees the original. The write commits after
        // its deadline, so subsequent reads also see the original.
        let mut pending = pool.begin().await?;
        sqlx::query("UPDATE accounts SET synthetic_privacy_locale='pl',
            synthetic_privacy_locale_token=$2,
            synthetic_privacy_locale_expires_at=clock_timestamp() - interval '1 second' WHERE id=$1")
            .bind(id).bind(token).execute(&mut *pending).await?;
        assert_eq!(shown().await?.as_deref(), Some("en"));
        pending.commit().await?;
        assert_eq!(shown().await?.as_deref(), Some("en"));
        test_support::close(pool, admin, schema).await?;
        Ok(())
    }

    #[tokio::test]
    async fn privacy_database_success_refusal_and_bad_input()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping privacy PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let (pool, admin, schema) = test_support::database(&url).await?;
        let id = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO accounts (id,email,password_hash,locale,status,email_verified_at) VALUES ($1,$2,'test','en','active',now())")
            .bind(id).bind(format!("{id}@example.test")).execute(&pool).await?;
        let service = Privacy { enabled: true };
        let initial = service.current(&pool, id).await?.ok_or("missing account")?;
        assert!(!initial.public_account_association && !initial.public_status_visible);
        assert!(service.update(&pool, id, true, true, "xx").await?.is_none());
        assert!(
            Privacy::default()
                .update(&pool, id, true, true, "pl")
                .await?
                .is_none()
        );
        let saved = service
            .update(&pool, id, true, true, "pl")
            .await?
            .ok_or("update refused")?;
        assert!(saved.public_account_association && saved.public_status_visible);
        assert_eq!(saved.locale, "pl");
        assert_eq!(saved.privacy_revision, initial.privacy_revision + 1);
        let (a, b) = tokio::join!(
            service.update(&pool, id, false, false, "en"),
            service.update(&pool, id, false, false, "pl")
        );
        assert!(a?.is_some() && b?.is_some());
        let current = service.current(&pool, id).await?.ok_or("missing current")?;
        assert_eq!(current.privacy_revision, saved.privacy_revision + 2);
        assert!(!current.public_account_association && !current.public_status_visible);
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await?;
        assert!(service.current(&pool, id).await?.is_none());
        assert!(service.update(&pool, id, true, true, "en").await?.is_none());
        assert!(
            service
                .update(&pool, uuid::Uuid::now_v7(), true, true, "en")
                .await?
                .is_none()
        );
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM account_privacy_events WHERE account_id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count, 3);
        sqlx::query("DELETE FROM account_privacy_events WHERE account_id=$1")
            .bind(id)
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM accounts WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await?;
        test_support::close(pool, admin, schema).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "privacy_test_support.rs"]
pub(crate) mod test_support;
