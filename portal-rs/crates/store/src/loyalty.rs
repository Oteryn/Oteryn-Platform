//! Consumed Premium days, ordered by the shared Premium authority revision.
use crate::premium::{AccountState, Accounts, Error, MAX_REVISION};
use sqlx::PgPool;
use std::sync::Arc;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[path = "loyalty_http.rs"]
pub mod http;

#[path = "loyalty_stream.rs"]
pub mod stream;

#[derive(Debug, Clone)]
pub struct Ledger {
    pool: PgPool,
    accounts: Arc<dyn Accounts>,
    enabled: bool,
}
impl Ledger {
    pub fn new(pool: PgPool, accounts: Arc<dyn Accounts>, enabled: bool) -> Self {
        Self {
            pool,
            accounts,
            enabled,
        }
    }
    pub async fn snapshot(
        &self,
        account: Uuid,
        nonce: &str,
        revision: &str,
    ) -> Result<Vec<u8>, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if account.get_version_num() != 7 || account.get_variant() != uuid::Variant::RFC4122 {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        match self.accounts.state(account, &mut tx).await? {
            AccountState::Known => (),
            AccountState::Unknown => return Err(Error::UnknownAccount),
            AccountState::Ambiguous => return Err(Error::Unavailable),
        }
        sqlx::query("INSERT INTO premium_authority(account_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(account)
            .execute(&mut *tx)
            .await?;
        let (authority, last): (i64, Option<OffsetDateTime>) = sqlx::query_as("SELECT authority_revision,last_issued_at FROM premium_authority WHERE account_id=$1 FOR UPDATE")
            .bind(account).fetch_one(&mut *tx).await?;
        if !(0..MAX_REVISION).contains(&authority) {
            return Err(Error::Unavailable);
        }
        let now: OffsetDateTime =
            sqlx::query_scalar("SELECT date_trunc('second', clock_timestamp())")
                .fetch_one(&mut *tx)
                .await?;
        if last.is_some_and(|last| last > now) {
            return Err(Error::Unavailable);
        }
        // DISTINCT ON selects the final lifecycle event of each interval, including
        // revoked and restarted intervals. Sum seconds before flooring whole days.
        let points: i32 = sqlx::query_scalar("SELECT LEAST(2147483647, FLOOR(COALESCE(SUM(GREATEST(0, EXTRACT(EPOCH FROM LEAST(effective_until,$2)-effective_from))),0)/86400))::integer FROM (SELECT DISTINCT ON (entitlement_id,effective_from) effective_from,effective_until FROM premium_ledger WHERE account_id=$1 AND effective_from < $2 ORDER BY entitlement_id,effective_from,lifecycle_revision DESC) intervals")
            .bind(account).bind(now).fetch_one(&mut *tx).await?;
        let referrals: bool =
            sqlx::query_scalar("SELECT to_regclass('referral_loyalty_credits') IS NOT NULL")
                .fetch_one(&mut *tx)
                .await?;
        let points = if referrals {
            sqlx::query("SELECT referral_campaign_loyalty($1,$2)")
                .bind(account)
                .bind(points)
                .execute(&mut *tx)
                .await?;
            let credits: i64 = sqlx::query_scalar("SELECT LEAST(2147483647,COALESCE(sum(points),0))::bigint FROM referral_loyalty_credits WHERE account_id=$1")
                .bind(account).fetch_one(&mut *tx).await?;
            (i64::from(points) + credits).min(i64::from(i32::MAX)) as i32
        } else {
            points
        };
        let bytes = snapshot(
            points,
            authority + 1,
            now.unix_timestamp(),
            account,
            nonce,
            revision,
        )?;
        sqlx::query("UPDATE premium_authority SET authority_revision=$2,last_issued_at=$3,updated_at=clock_timestamp() WHERE account_id=$1")
            .bind(account).bind(authority+1).bind(now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO loyalty_state(account_id,loyalty_points,loyalty_revision,computed_at) VALUES ($1,$2,$3,$4) ON CONFLICT (account_id) DO UPDATE SET loyalty_points=EXCLUDED.loyalty_points,loyalty_revision=EXCLUDED.loyalty_revision,computed_at=EXCLUDED.computed_at,updated_at=clock_timestamp()")
            .bind(account).bind(points).bind(authority+1).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(bytes)
    }
}
pub const MAX_RESPONSE_BYTES: usize = 512;
pub fn snapshot(
    points: i32,
    authority: i64,
    computed: i64,
    account: Uuid,
    nonce: &str,
    revision: &str,
) -> Result<Vec<u8>, Error> {
    use oteryn_internal_api::canonical::{self, Value};
    if points < 0
        || !(1..=MAX_REVISION).contains(&authority)
        || !http::hex(nonce, 32)
        || !http::hex(revision, 40)
        || account.get_version_num() != 7
        || account.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    let date = OffsetDateTime::from_unix_timestamp(computed).map_err(|_| Error::Unavailable)?;
    if !(0..=9999).contains(&date.year()) {
        return Err(Error::Unavailable);
    }
    let timestamp = date.format(&Rfc3339).map_err(|_| Error::Unavailable)?;
    let text = |v: &str| Value::String(v.into());
    let bytes = canonical::encode(&Value::Object(vec![
        ("schema".into(), text("oteryn.loyalty_snapshot.v1")),
        ("producer_revision".into(), text(revision)),
        ("nonce".into(), text(nonce)),
        ("account_id".into(), text(&account.to_string())),
        ("loyalty_points".into(), Value::Signed(i64::from(points))),
        ("loyalty_revision".into(), Value::Signed(authority)),
        ("computed_at".into(), text(&timestamp)),
    ]))
    .map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
#[cfg(test)]
#[path = "loyalty_tests.rs"]
mod tests;
