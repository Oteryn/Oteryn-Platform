//! Parameterized PostgreSQL storage; ledger entries and audit are append-only.
use super::*;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct Ledger {
    pool: PgPool,
    accounts: Arc<dyn Accounts>,
    enabled: bool,
}
impl Ledger {
    /// Activation is explicit and defaults off in deployment configuration.
    pub fn new(pool: PgPool, accounts: Arc<dyn Accounts>, enabled: bool) -> Self {
        Self {
            pool,
            accounts,
            enabled,
        }
    }
    async fn lock(&self, connection: &mut PgConnection, account: Uuid) -> Result<i64, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if account.get_version_num() != 7 || account.get_variant() != uuid::Variant::RFC4122 {
            return Err(Error::Invalid);
        }
        match self.accounts.state(account, connection).await? {
            AccountState::Known => (),
            AccountState::Unknown => return Err(Error::UnknownAccount),
            AccountState::Ambiguous => return Err(Error::Unavailable),
        }
        sqlx::query("INSERT INTO premium_authority(account_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(account)
            .execute(&mut *connection)
            .await?;
        let revision: i64 = sqlx::query_scalar(
            "SELECT authority_revision FROM premium_authority WHERE account_id=$1 FOR UPDATE",
        )
        .bind(account)
        .fetch_one(&mut *connection)
        .await?;
        if !(0..=MAX_REVISION).contains(&revision) {
            return Err(Error::Unavailable);
        }
        Ok(revision)
    }
    pub async fn mutate(
        &self,
        operator: &Operator,
        mutation: &Mutation,
    ) -> Result<Entitlement, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !operator.confirmed_mfa || operator.permission != "products.premium.manage" {
            return Err(Error::Denied);
        }
        let reason = mutation.reason.trim();
        if !(10..=500).contains(&reason.chars().count())
            || mutation.request.is_nil()
            || operator.actor.get_version_num() != 7
            || operator.actor.get_variant() != uuid::Variant::RFC4122
            || matches!(mutation.change, Change::Grant(days) if !(1..=366).contains(&days))
        {
            return Err(Error::Invalid);
        }
        let hash = Sha256::digest(reason.as_bytes()).to_vec();
        let (kind, days) = match mutation.change {
            Change::Grant(days) => ("GRANT", Some(i32::from(days))),
            Change::Revoke => ("REVOKE", None),
        };
        let mut tx = self.pool.begin().await?;
        // A request lock precedes the account lock and also serializes conflicting
        // reuses across different accounts. Hash collisions only serialize work.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 27))")
            .bind(mutation.request.to_string())
            .execute(&mut *tx)
            .await?;
        self.lock(&mut tx, mutation.account).await?;
        if let Some(row) = sqlx::query("SELECT * FROM premium_ledger WHERE request_id=$1")
            .bind(mutation.request)
            .fetch_optional(&mut *tx)
            .await?
        {
            if row.try_get::<Uuid, _>("account_id")? != mutation.account
                || row.try_get::<Uuid, _>("actor_id")? != operator.actor
                || row.try_get::<String, _>("event_type")? != kind
                || row.try_get::<Option<i32>, _>("duration_days")? != days
                || row.try_get::<Vec<u8>, _>("reason_sha256")? != hash
            {
                return Err(Error::Conflict);
            }
            let result = recorded(&row)?;
            tx.commit().await?;
            return Ok(result);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let last: Option<OffsetDateTime> =
            sqlx::query_scalar("SELECT last_issued_at FROM premium_authority WHERE account_id=$1")
                .bind(mutation.account)
                .fetch_one(&mut *tx)
                .await?;
        if last.is_some_and(|last| last.unix_timestamp() > now.unix_timestamp()) {
            return Err(Error::Unavailable);
        }
        let existing = current(&mut tx, mutation.account).await?;
        if existing
            .as_ref()
            .is_some_and(|value| value.from > now.unix_timestamp())
        {
            return Err(Error::Unavailable);
        }
        let next = transition(existing.as_ref(), mutation.change, now.unix_timestamp())?;
        // The sum of deltas is the current interval's duration. Expired or revoked
        // time is cancelled when restarting; revocation removes the remainder.
        let previous_duration = existing.as_ref().map_or(0, |e| e.until - e.from);
        let delta = next.until - next.from - previous_duration;
        sqlx::query("INSERT INTO premium_ledger(request_id, account_id, actor_id, event_type, duration_days, reason_sha256, entitlement_id, lifecycle_revision, delta_seconds, effective_from, effective_until) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(mutation.request).bind(mutation.account).bind(operator.actor).bind(kind).bind(days).bind(hash)
            .bind(next.id).bind(next.revision).bind(delta).bind(timestamp(next.from)?).bind(timestamp(next.until)?)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO premium_audit(request_id,actor_id,account_id,event_name) VALUES ($1,$2,$3,$4)")
            .bind(mutation.request).bind(operator.actor).bind(mutation.account)
            .bind(if kind == "GRANT" { "products.premium_granted" } else { "products.premium_revoked" })
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(next)
    }
    /// Unknown ids are resolved before inserting an authority row. Issuance and
    /// mutations share the same lock, including NONE snapshots.
    pub async fn snapshot(
        &self,
        account: Uuid,
        nonce: &str,
        revision: &str,
    ) -> Result<Vec<u8>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        let authority = self.lock(&mut tx, account).await?;
        if authority == MAX_REVISION {
            return Err(Error::Unavailable);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let issued = now.unix_timestamp();
        let last: Option<OffsetDateTime> =
            sqlx::query_scalar("SELECT last_issued_at FROM premium_authority WHERE account_id=$1")
                .bind(account)
                .fetch_one(&mut *tx)
                .await?;
        if last.is_some_and(|last| last.unix_timestamp() > issued) {
            return Err(Error::Unavailable);
        }
        let entitlement = current(&mut tx, account).await?;
        let bytes = super::wire::snapshot(
            entitlement.as_ref(),
            issued,
            authority + 1,
            account,
            nonce,
            revision,
        )?;
        sqlx::query("UPDATE premium_authority SET authority_revision=$2,last_issued_at=$3,updated_at=clock_timestamp() WHERE account_id=$1")
            .bind(account).bind(authority+1).bind(timestamp(issued)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(bytes)
    }
}
fn timestamp(value: i64) -> Result<OffsetDateTime, Error> {
    OffsetDateTime::from_unix_timestamp(value).map_err(|_| Error::Unavailable)
}
fn recorded(row: &sqlx::postgres::PgRow) -> Result<Entitlement, Error> {
    let result = Entitlement {
        id: row.try_get("entitlement_id")?,
        revoked: row.try_get::<String, _>("event_type")? == "REVOKE",
        revision: row.try_get("lifecycle_revision")?,
        from: row
            .try_get::<OffsetDateTime, _>("effective_from")?
            .unix_timestamp(),
        until: row
            .try_get::<OffsetDateTime, _>("effective_until")?
            .unix_timestamp(),
    };
    result.validate()?;
    Ok(result)
}
async fn current(
    connection: &mut PgConnection,
    account: Uuid,
) -> Result<Option<Entitlement>, Error> {
    let row = sqlx::query(
        "SELECT * FROM premium_ledger WHERE account_id=$1 ORDER BY lifecycle_revision DESC LIMIT 1",
    )
    .bind(account)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut result = recorded(&row)?;
    let (sum, count, identities): (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(delta_seconds),0)::bigint, COUNT(*), COUNT(DISTINCT entitlement_id) FROM premium_ledger WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&mut *connection)
    .await?;
    let until = result.from.checked_add(sum).ok_or(Error::Unavailable)?;
    if result.until != until || count != result.revision || identities != 1 {
        return Err(Error::Unavailable);
    }
    result.until = until;
    Ok(Some(result))
}
