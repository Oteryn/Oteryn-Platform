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
        let mut tx = self.pool.begin().await?;
        let mut head: i64 = sqlx::query_scalar(
            "SELECT position FROM loyalty_stream_head WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        if cursor.position > head || cursor.through > head {
            return Err(Error::Invalid);
        }
        // A continuation reads only its immutable window, without recomputation.
        // A tail poll recomputes even when no Premium mutation has occurred: time
        // alone can advance a consumed-day total. One SQL snapshot covers all totals.
        if cursor.through == 0 {
            let totals: Vec<(Uuid, i32)> = sqlx::query_as(
                "WITH instant AS MATERIALIZED (SELECT date_trunc('second', clock_timestamp()) AS now), intervals AS (SELECT DISTINCT ON (account_id,entitlement_id,effective_from) account_id,effective_from,effective_until FROM premium_ledger ORDER BY account_id,entitlement_id,effective_from,lifecycle_revision DESC) SELECT a.account_id, LEAST(2147483647, FLOOR(COALESCE(SUM(GREATEST(0, EXTRACT(EPOCH FROM LEAST(i.effective_until,t.now)-i.effective_from))),0)/86400))::integer FROM premium_authority a CROSS JOIN instant t LEFT JOIN intervals i ON i.account_id=a.account_id AND i.effective_from<t.now GROUP BY a.account_id ORDER BY a.account_id"
            ).fetch_all(&mut *tx).await?;
            for (account, days) in totals {
                if account.get_version_num() != 7 || account.get_variant() != uuid::Variant::RFC4122
                {
                    return Err(Error::Unavailable);
                }
                match self.accounts.state(account, &mut tx).await? {
                    AccountState::Known => (),
                    AccountState::Unknown => continue,
                    AccountState::Ambiguous => return Err(Error::Unavailable),
                }
                let old: Option<(i32, i64)> = sqlx::query_as("SELECT spent_premium_days,platform_revision FROM loyalty_stream_state WHERE account_id=$1")
                    .bind(account).fetch_optional(&mut *tx).await?;
                if old.is_some_and(|(previous, _)| previous == days) {
                    continue;
                }
                let revision = old
                    .map_or(Some(1), |(_, revision)| revision.checked_add(1))
                    .ok_or(Error::Unavailable)?;
                head = head.checked_add(1).ok_or(Error::Unavailable)?;
                sqlx::query("INSERT INTO loyalty_stream_state(account_id,spent_premium_days,platform_revision) VALUES ($1,$2,$3) ON CONFLICT (account_id) DO UPDATE SET spent_premium_days=$2,platform_revision=$3,updated_at=clock_timestamp()")
                    .bind(account).bind(days).bind(revision).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO loyalty_stream_events(position,account_id,spent_premium_days,platform_revision) VALUES ($1,$2,$3,$4)")
                    .bind(head).bind(account).bind(days).bind(revision).execute(&mut *tx).await?;
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
        let rows: Vec<(i64, Uuid, i32, i64)> = sqlx::query_as("SELECT position,account_id,spent_premium_days,platform_revision FROM loyalty_stream_events WHERE position>$1 AND position<=$2 ORDER BY position LIMIT $3")
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
            .map(|(_, account_id, days, platform_revision)| Item {
                account_id,
                spent_premium_days: days,
                loyalty_points: days,
                platform_revision,
            })
            .collect();
        tx.commit().await?;
        Ok(Page { items, next })
    }
}

#[cfg(test)]
#[path = "loyalty_stream_tests.rs"]
mod tests;
