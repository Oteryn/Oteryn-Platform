//! Durable consumed-Premium-day export. The game pulls; no game connection exists here.
use super::{AccountState, Accounts, Error};
use oteryn_internal_api::canonical::{self, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[path = "loyalty_stream_http.rs"]
pub mod http;

/// A position in immutable history, optionally bounded to a previously opened window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    position: i64,
    through: i64,
}
impl Cursor {
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut parts = text.split(':');
        if parts.next() != Some("v1") {
            return Err(Error::Invalid);
        }
        let mut number = || {
            let part = parts.next().ok_or(Error::Invalid)?;
            let value: i64 = part.parse().map_err(|_| Error::Invalid)?;
            if value < 0 || value.to_string() != part {
                return Err(Error::Invalid);
            }
            Ok(value)
        };
        let position = number()?;
        let through = number()?;
        if parts.next().is_some() || (through != 0 && position >= through) {
            return Err(Error::Invalid);
        }
        Ok(Self { position, through })
    }
}
impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v1:{}:{}", self.position, self.through)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub account_id: Uuid,
    pub spent_premium_days: i32,
    pub loyalty_points: i32,
    pub platform_revision: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub items: Vec<Item>,
    pub next: Cursor,
}
impl Page {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let items = self
            .items
            .iter()
            .map(|item| {
                Value::Object(vec![
                    (
                        "account_id".into(),
                        Value::String(item.account_id.to_string()),
                    ),
                    (
                        "spent_premium_days".into(),
                        Value::Signed(i64::from(item.spent_premium_days)),
                    ),
                    (
                        "loyalty_points".into(),
                        Value::Signed(i64::from(item.loyalty_points)),
                    ),
                    (
                        "platform_revision".into(),
                        Value::Signed(item.platform_revision),
                    ),
                ])
            })
            .collect();
        canonical::encode(&Value::Object(vec![
            (
                "schema".into(),
                Value::String("oteryn.loyalty_stream_page.v1".into()),
            ),
            ("items".into(), Value::Array(items)),
            (
                "next_cursor".into(),
                if self.items.is_empty() {
                    Value::Null
                } else {
                    Value::String(self.next.to_string())
                },
            ),
        ]))
        .map_err(|_| Error::Unavailable)
    }
}

#[derive(Debug, Clone)]
pub struct Stream {
    pool: PgPool,
    accounts: Arc<dyn Accounts>,
    enabled: bool,
}
impl Stream {
    pub fn new(pool: PgPool, accounts: Arc<dyn Accounts>, enabled: bool) -> Self {
        Self {
            pool,
            accounts,
            enabled,
        }
    }

    pub async fn read(&self, after: Option<Cursor>, limit: u16) -> Result<Page, Error> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if !(1..=500).contains(&limit) {
            return Err(Error::Invalid);
        }
        let cursor = after.unwrap_or(Cursor {
            position: 0,
            through: 0,
        });
        oteryn_db::retry::transaction(|| self.read_attempt(cursor, limit)).await
    }
    async fn read_attempt(
        &self,
        cursor: Cursor,
        limit: u16,
    ) -> Result<Page, oteryn_db::retry::TransactionError<Error>> {
        let mut tx = self.pool.begin().await?;
        let mut head: i64 = sqlx::query_scalar(
            "SELECT position FROM loyalty_stream_head WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        if cursor.position > head || cursor.through > head {
            return Err(Error::Invalid.into());
        }
        // A continuation reads only its immutable window, without recomputation.
        // A tail poll recomputes even when no Premium mutation has occurred: time
        // alone can advance a consumed-day total. One SQL snapshot covers all Premium consumption totals.
        if cursor.through == 0 {
            let totals: Vec<(Uuid, i32)> = sqlx::query_as(
                "WITH instant AS MATERIALIZED (SELECT date_trunc('second', clock_timestamp()) AS now), intervals AS (SELECT DISTINCT ON (account_id,entitlement_id,effective_from) account_id,effective_from,effective_until FROM premium_ledger ORDER BY account_id,entitlement_id,effective_from,lifecycle_revision DESC) SELECT days.account_id,days.spent FROM (SELECT a.account_id, LEAST(2147483647, FLOOR(COALESCE(SUM(GREATEST(0, EXTRACT(EPOCH FROM LEAST(i.effective_until,t.now)-i.effective_from))),0)/86400))::integer AS spent FROM (SELECT account_id FROM premium_authority UNION SELECT account_id FROM referral_loyalty_credits) a CROSS JOIN instant t LEFT JOIN intervals i ON i.account_id=a.account_id AND i.effective_from<t.now GROUP BY a.account_id) days ORDER BY days.account_id"
            ).fetch_all(&mut *tx).await?;
            // Capture and lock the full authority set before qualification takes
            // advisory lock 5600. Purchases lock account rows before that lock.
            // Include potential credit-only inviters so publication never needs
            // to acquire a new account lock while holding 5600.
            let candidates: Vec<Uuid> = totals.iter().map(|(account, _)| *account).collect();
            let inviters: Vec<Uuid> = sqlx::query_scalar("SELECT DISTINCT inviter_id FROM referral_campaign_invitations WHERE kind='new' AND invitee_id=ANY($1)")
                .bind(&candidates).fetch_all(&mut *tx).await?;
            let candidates: std::collections::BTreeSet<_> =
                candidates.into_iter().chain(inviters).collect();
            let locked_accounts: Vec<_> = candidates.iter().copied().collect();
            // Account adapters may take SHARE/UPDATE locks: prepare the full
            // ordered set first, so their reads never require lock upgrades.
            sqlx::query("SELECT referral_campaign_prepare($1)")
                .bind(&locked_accounts)
                .execute(&mut *tx)
                .await?;
            let mut known = std::collections::BTreeSet::new();
            for account in candidates {
                if account.get_version_num() != 7 || account.get_variant() != uuid::Variant::RFC4122
                {
                    return Err(Error::Unavailable.into());
                }
                match self.accounts.state(account, &mut tx).await? {
                    AccountState::Known => {
                        known.insert(account);
                    }
                    AccountState::Unknown => (),
                    AccountState::Ambiguous => return Err(Error::Unavailable.into()),
                }
                sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR KEY SHARE")
                    .bind(account)
                    .execute(&mut *tx)
                    .await?;
            }
            for &account in &known {
                // Snapshots insert/lock authority before taking 5600.
                // Prepare even credit-only accounts in that same order.
                sqlx::query(
                    "INSERT INTO premium_authority(account_id) VALUES($1) ON CONFLICT DO NOTHING",
                )
                .bind(account)
                .execute(&mut *tx)
                .await?;
                // ON CONFLICT does not lock an existing authority row.
                // Publication's foreign keys need this lock before 5600.
                sqlx::query(
                    "SELECT account_id FROM premium_authority WHERE account_id=$1 FOR KEY SHARE",
                )
                .bind(account)
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("SELECT referral_campaign_lock($1)")
                .bind(&locked_accounts)
                .execute(&mut *tx)
                .await?;
            // Award from Premium consumption only, before reading credit totals.
            for &(account, days) in &totals {
                if !known.contains(&account) {
                    continue;
                }
                sqlx::query("SELECT referral_campaign_loyalty($1,$2)")
                    .bind(account)
                    .bind(days)
                    .execute(&mut *tx)
                    .await?;
            }
            // Concurrently appearing accounts are observed on the next tail
            // poll; do not expand the authority set after taking 5600.
            let credits: Vec<(Uuid, i32)> = sqlx::query_as("SELECT account_id,LEAST(2147483647,sum(points))::integer FROM referral_loyalty_credits WHERE account_id=ANY($1) GROUP BY account_id")
                .bind(known.iter().copied().collect::<Vec<_>>()).fetch_all(&mut *tx).await?;
            let mut totals: std::collections::BTreeMap<_, _> = totals
                .into_iter()
                .map(|(account, days)| (account, (days, days)))
                .collect();
            for (account, credits) in credits {
                let (days, points) = totals.entry(account).or_insert((0, 0));
                *points = days.saturating_add(credits);
            }
            for (account, (days, points)) in totals {
                if !known.contains(&account) {
                    continue;
                }
                let old: Option<(i32, i32, i64)> = sqlx::query_as("SELECT spent_premium_days,loyalty_points,platform_revision FROM loyalty_stream_state WHERE account_id=$1")
                    .bind(account).fetch_optional(&mut *tx).await?;
                if old.is_some_and(|(previous_days, previous_points, _)| {
                    previous_days == days && previous_points == points
                }) {
                    continue;
                }
                let revision = old
                    .map_or(Some(1), |(_, _, revision)| revision.checked_add(1))
                    .ok_or(Error::Unavailable)?;
                head = head.checked_add(1).ok_or(Error::Unavailable)?;
                sqlx::query("INSERT INTO loyalty_stream_state(account_id,spent_premium_days,loyalty_points,platform_revision) VALUES ($1,$2,$3,$4) ON CONFLICT (account_id) DO UPDATE SET spent_premium_days=$2,loyalty_points=$3,platform_revision=$4,updated_at=clock_timestamp()")
                    .bind(account).bind(days).bind(points).bind(revision).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO loyalty_stream_events(position,account_id,spent_premium_days,loyalty_points,platform_revision) VALUES ($1,$2,$3,$4,$5)")
                    .bind(head).bind(account).bind(days).bind(points).bind(revision).execute(&mut *tx).await?;
            }
            sqlx::query("UPDATE loyalty_stream_head SET position=$1 WHERE singleton")
                .bind(head)
                .execute(&mut *tx)
                .await?;
        }
        let through = if cursor.through == 0 {
            head
        } else {
            cursor.through
        };
        let rows: Vec<(i64, Uuid, i32, i32, i64)> = sqlx::query_as("SELECT position,account_id,spent_premium_days,loyalty_points,platform_revision FROM loyalty_stream_events WHERE position>$1 AND position<=$2 ORDER BY position LIMIT $3")
            .bind(cursor.position).bind(through).bind(i64::from(limit)).fetch_all(&mut *tx).await?;
        let position = rows.last().map_or(cursor.position, |row| row.0);
        let next = if position == through {
            Cursor {
                position,
                through: 0,
            }
        } else {
            Cursor { position, through }
        };
        let items = rows
            .into_iter()
            .map(
                |(_, account_id, days, loyalty_points, platform_revision)| Item {
                    account_id,
                    spent_premium_days: days,
                    loyalty_points,
                    platform_revision,
                },
            )
            .collect();
        tx.commit().await?;
        Ok(Page { items, next })
    }
}

#[cfg(test)]
#[path = "loyalty_stream_tests.rs"]
mod tests;
