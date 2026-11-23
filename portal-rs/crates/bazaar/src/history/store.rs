use crate::{Error, SearchQuery};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::{Date, Month};
use uuid::Uuid;

pub(super) const PAGE_SIZE: usize = 24;
pub(super) const MAX_PAGE: u16 = 999;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Filters {
    pub min_price: String,
    pub max_price: String,
    pub from: String,
    pub to: String,
    pub currency: String,
    pub form: String,
}

impl Filters {
    fn price(value: &str) -> Result<Option<i64>, Error> {
        if value.is_empty() {
            return Ok(None);
        }
        if value.len() > 12 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::InvalidInput);
        }
        value.parse().map(Some).map_err(|_| Error::InvalidInput)
    }

    fn day(value: &str) -> Result<Option<Date>, Error> {
        if value.is_empty() {
            return Ok(None);
        }
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes
                .split(|&b| b == b'-')
                .flatten()
                .all(u8::is_ascii_digit)
        {
            return Err(Error::InvalidInput);
        }
        let number = |start, end| {
            value[start..end]
                .parse::<u8>()
                .map_err(|_| Error::InvalidInput)
        };
        let year = value[..4].parse::<i32>().map_err(|_| Error::InvalidInput)?;
        let month = Month::try_from(number(5, 7)?).map_err(|_| Error::InvalidInput)?;
        if year == 0 {
            return Err(Error::InvalidInput);
        }
        Date::from_calendar_date(year, month, number(8, 10)?)
            .map(Some)
            .map_err(|_| Error::InvalidInput)
    }

    fn query(&self) -> Result<SearchQuery, Error> {
        let query = SearchQuery {
            price_min: Self::price(&self.min_price)?,
            price_max: Self::price(&self.max_price)?,
            currency: nonempty(&self.currency).map(str::to_owned),
            form: nonempty(&self.form).map(str::to_owned),
            ..Default::default()
        };
        query.validate()?;
        if Self::day(&self.from)?
            .zip(Self::day(&self.to)?)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(Error::InvalidInput);
        }
        Ok(query)
    }

    pub fn validate(&self) -> Result<(), Error> {
        self.query().map(|_| ())
    }
}

fn nonempty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct Sale {
    pub id: Uuid,
    pub kind: String,
    pub form: String,
    pub currency: String,
    pub amount: i64,
    pub day: Date,
    pub role: String,
    pub consent: bool,
    pub shareable: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct Price {
    pub form: String,
    pub currency: String,
    pub month: Option<String>,
    pub samples: i64,
    pub median: i64,
}

pub(super) async fn sales(
    db: &mut PgConnection,
    actor: Option<Uuid>,
    filters: &Filters,
    ids: Option<&[Uuid]>,
    page: u16,
) -> Result<Vec<Sale>, Error> {
    let query = filters.query()?;
    if page > MAX_PAGE {
        return Err(Error::InvalidInput);
    }
    Ok(sqlx::query_as(include_str!("sales.sql"))
        .bind(actor)
        .bind(query.price_min)
        .bind(query.price_max)
        .bind(Filters::day(&filters.from)?)
        .bind(Filters::day(&filters.to)?)
        .bind(query.currency)
        .bind(query.form)
        .bind(ids)
        .bind(i64::from(page) * PAGE_SIZE as i64)
        .fetch_all(db)
        .await?)
}

pub(super) async fn prices(db: &mut PgConnection) -> Result<Vec<Price>, Error> {
    Ok(sqlx::query_as(include_str!("prices.sql"))
        .fetch_all(db)
        .await?)
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct Saved {
    pub name: String,
    criteria: sqlx::types::Json<Filters>,
}

impl Saved {
    pub fn url_query(&self) -> String {
        serde_urlencoded::to_string(&self.criteria.0).unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub(super) struct Account {
    pub alerts_enabled: bool,
    pub saved: Vec<Saved>,
    pub watched: Vec<Sale>,
    pub alerts: Vec<Sale>,
}

pub(super) async fn account(db: &mut PgConnection, actor: Uuid) -> Result<Account, Error> {
    let alerts_enabled = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_history_preferences WHERE account=$1)",
    )
    .bind(actor)
    .fetch_one(&mut *db)
    .await?;
    let saved = sqlx::query_as(
        "SELECT name, criteria FROM bazaar_history_searches WHERE account=$1 ORDER BY name",
    )
    .bind(actor)
    .fetch_all(&mut *db)
    .await?;
    let watched = watched(db, actor).await?;
    let alerts = sqlx::query_as(include_str!("alerts.sql"))
        .bind(actor)
        .fetch_all(db)
        .await?;
    Ok(Account {
        alerts_enabled,
        saved,
        watched,
        alerts,
    })
}

pub(super) async fn watched(db: &mut PgConnection, actor: Uuid) -> Result<Vec<Sale>, Error> {
    Ok(sqlx::query_as(
        "SELECT m.id,m.kind,m.form,m.currency,m.amount,m.day,'shared' AS role,false AS consent,false AS shareable
         FROM bazaar_history_market m JOIN bazaar_history_watches w ON w.sale=m.id
         WHERE w.account=$1 ORDER BY m.day DESC,m.id DESC",
    ).bind(actor).fetch_all(&mut *db).await?)
}

#[derive(Debug)]
pub(super) enum Change {
    Save { name: String, filters: Filters },
    Delete { name: String },
    EnableAlerts,
    DisableAlerts,
    Watch { sale: Uuid },
    Unwatch { sale: Uuid },
    Dismiss { sale: Uuid },
    Share { sale: Uuid },
    Unshare { sale: Uuid },
}

pub(super) async fn authorize(db: &mut PgConnection, actor: Uuid, lock: bool) -> Result<(), Error> {
    let query = if lock {
        "SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE"
    } else {
        "SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL"
    };
    sqlx::query_scalar::<_, Uuid>(query)
        .bind(actor)
        .fetch_optional(db)
        .await?
        .ok_or(Error::Denied)?;
    Ok(())
}

fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.trim() != name
        || name.chars().count() > 80
        || name.chars().any(char::is_control)
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

pub(super) async fn change(pool: &PgPool, actor: Uuid, input: Change) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, actor, true).await?;
    match input {
        Change::Save { name, filters } => {
            validate_name(&name)?;
            filters.validate()?;
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM bazaar_history_searches WHERE account=$1 AND name<>$2",
            )
            .bind(actor)
            .bind(&name)
            .fetch_one(&mut *tx)
            .await?;
            if count >= 50 {
                return Err(Error::Denied);
            }
            sqlx::query(
                "INSERT INTO bazaar_history_searches(account,name,criteria) VALUES($1,$2,$3)
                 ON CONFLICT(account,name) DO UPDATE SET criteria=excluded.criteria,saved_at=clock_timestamp()
                 WHERE bazaar_history_searches.criteria IS DISTINCT FROM excluded.criteria",
            ).bind(actor).bind(name).bind(sqlx::types::Json(filters)).execute(&mut *tx).await?;
        }
        Change::Delete { name } => {
            validate_name(&name)?;
            sqlx::query("DELETE FROM bazaar_history_searches WHERE account=$1 AND name=$2")
                .bind(actor)
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        Change::EnableAlerts => {
            sqlx::query(
                "INSERT INTO bazaar_history_preferences(account) VALUES($1) ON CONFLICT DO NOTHING",
            )
            .bind(actor)
            .execute(&mut *tx)
            .await?;
        }
        Change::DisableAlerts => {
            sqlx::query("DELETE FROM bazaar_history_preferences WHERE account=$1")
                .bind(actor)
                .execute(&mut *tx)
                .await?;
        }
        Change::Watch { sale } => {
            // Expired shares are removed from the watch list when room is needed.
            sqlx::query("DELETE FROM bazaar_history_watches w USING bazaar_history_shares h,bazaar_history_settled s
                         WHERE w.account=$1 AND w.sale=h.public_id AND s.id=h.listing
                           AND s.completed_at < now()-interval '90 days'")
                .bind(actor).execute(&mut *tx).await?;
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM bazaar_history_watches WHERE account=$1 AND sale<>$2",
            )
            .bind(actor)
            .bind(sale)
            .fetch_one(&mut *tx)
            .await?;
            if count >= 100 {
                return Err(Error::Denied);
            }
            let visible =
                sqlx::query_scalar::<_, Uuid>("SELECT id FROM bazaar_history_market WHERE id=$1")
                    .bind(sale)
                    .fetch_optional(&mut *tx)
                    .await?;
            if visible.is_none() {
                return Err(Error::NotFound);
            }
            sqlx::query("INSERT INTO bazaar_history_watches(account,sale) VALUES($1,$2) ON CONFLICT DO NOTHING")
                .bind(actor).bind(sale).execute(&mut *tx).await?;
        }
        Change::Unwatch { sale } => {
            sqlx::query("DELETE FROM bazaar_history_watches WHERE account=$1 AND sale=$2")
                .bind(actor)
                .bind(sale)
                .execute(&mut *tx)
                .await?;
        }
        Change::Dismiss { sale } => {
            let alerts: Vec<Sale> = sqlx::query_as(include_str!("alerts.sql"))
                .bind(actor)
                .fetch_all(&mut *tx)
                .await?;
            if alerts.iter().any(|row| row.id == sale) {
                sqlx::query("INSERT INTO bazaar_history_dismissals(account,sale) VALUES($1,$2) ON CONFLICT DO NOTHING")
                    .bind(actor).bind(sale).execute(&mut *tx).await?;
            }
        }
        Change::Share { sale } | Change::Unshare { sale } => {
            share(&mut tx, actor, sale, matches!(input, Change::Share { .. })).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn share(db: &mut PgConnection, actor: Uuid, sale: Uuid, enabled: bool) -> Result<(), Error> {
    let seller: bool = sqlx::query_scalar(
        "SELECT s.seller=$1 FROM bazaar_history_settled s JOIN bazaar_listings l ON l.id=s.id
         WHERE s.id=$2 AND $1 IN (s.buyer,s.seller) AND s.kind='item'
           AND (NOT $3 OR s.completed_at >= now()-interval '90 days')
         FOR UPDATE OF l",
    )
    .bind(actor)
    .bind(sale)
    .bind(enabled)
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::NotFound)?;
    if enabled {
        sqlx::query(
            "INSERT INTO bazaar_history_shares(listing,seller_at,buyer_at)
             VALUES($1,CASE WHEN $2 THEN clock_timestamp() END,CASE WHEN NOT $2 THEN clock_timestamp() END)
             ON CONFLICT(listing) DO UPDATE SET
                seller_at=coalesce(bazaar_history_shares.seller_at,excluded.seller_at),
                buyer_at=coalesce(bazaar_history_shares.buyer_at,excluded.buyer_at)",
        ).bind(sale).bind(seller).execute(&mut *db).await?;
    } else {
        sqlx::query(
            "DELETE FROM bazaar_history_shares
             WHERE listing=$1 AND CASE WHEN $2 THEN buyer_at IS NULL ELSE seller_at IS NULL END",
        )
        .bind(sale)
        .bind(seller)
        .execute(&mut *db)
        .await?;
        sqlx::query(
            "UPDATE bazaar_history_shares SET
                seller_at=CASE WHEN $2 THEN NULL ELSE seller_at END,
                buyer_at=CASE WHEN $2 THEN buyer_at ELSE NULL END WHERE listing=$1",
        )
        .bind(sale)
        .bind(seller)
        .execute(db)
        .await?;
    }
    Ok(())
}
