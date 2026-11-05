//! Complete current-epoch projections and effects commit together.
use crate::store::{self, Snapshot};
use serde_json::Value;
use sqlx::{PgPool, Row};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StatisticsJob {
    pub pool: PgPool,
    pub enabled: bool,
}
impl oteryn_db::outbox::Cleanup for StatisticsJob {
    fn clean(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            self.run().await.map_err(|e| e.to_string())?;
            Ok(())
        })
    }
}
fn invalid() -> sqlx::Error {
    sqlx::Error::Protocol("invalid statistics projection".into())
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, sqlx::Error> {
    v.get(key).and_then(Value::as_str).ok_or_else(invalid)
}
fn uuid(v: &Value, key: &str) -> Result<Uuid, sqlx::Error> {
    Uuid::parse_str(text(v, key)?).map_err(|_| invalid())
}
fn date(timestamp: i64) -> Result<Date, sqlx::Error> {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .map(|v| v.date())
        .map_err(|_| invalid())
}
impl StatisticsJob {
    /// Disabled jobs do not connect to the database.
    pub async fn run(&self) -> Result<usize, sqlx::Error> {
        if !self.enabled {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(65001)")
            .execute(&mut *tx)
            .await?;
        let today: Date = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'UTC')::date")
            .fetch_one(&mut *tx)
            .await?;
        store::partitions(&mut tx, today).await?;
        let rows = sqlx::query("SELECT s.family,s.subject,s.epoch::text,s.version::text,s.watermark,s.payload FROM public_snapshots s JOIN public_snapshots w ON w.family=s.family AND w.is_watermark AND w.epoch=s.epoch JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch LEFT JOIN stats_projection_cursors c ON c.family=s.family AND c.subject=s.subject WHERE NOT s.is_watermark AND s.family IN ('character_profile','character_deaths','world_online') AND s.watermark <= w.watermark AND (c.subject IS NULL OR (s.epoch,s.version,s.watermark) > (c.epoch,c.version,c.watermark)) ORDER BY s.watermark,s.family,s.subject").fetch_all(&mut *tx).await?;
        let count = rows.len();
        for row in rows {
            let family: String = row.try_get("family")?;
            let subject: String = row.try_get("subject")?;
            let character = Uuid::parse_str(&subject).map_err(|_| invalid())?;
            let observed: i64 = row.try_get("watermark")?;
            let day = date(observed)?;
            let payload: Value = row.try_get("payload")?;
            match family.as_str() {
                "character_profile" => {
                    let profiles = payload
                        .get("profile")
                        .and_then(Value::as_array)
                        .ok_or_else(invalid)?;
                    if let Some(p) = profiles.first() {
                        store::partitions(&mut tx, day).await?;
                        let guild = p
                            .get("guild")
                            .and_then(Value::as_array)
                            .and_then(|a| a.first())
                            .map(|g| uuid(g, "guild_id"))
                            .transpose()?;
                        let s = Snapshot {
                            character_ref: character,
                            day,
                            level: text(p, "level")?.parse().map_err(|_| invalid())?,
                            experience: None,
                            skills: None,
                            achievement_points: None,
                            minutes_online: None,
                            name: text(p, "name")?.into(),
                            world: uuid(p, "world_id")?,
                            guild,
                            observed_at: observed,
                        };
                        store::snapshot(&mut tx, &s).await?;
                    }
                }
                "character_deaths" => {
                    for death in payload
                        .get("deaths")
                        .and_then(Value::as_array)
                        .ok_or_else(invalid)?
                    {
                        let key = text(death, "occurred_at")?;
                        store::source_event(
                            &mut tx,
                            character,
                            date(key.parse().map_err(|_| invalid())?)?,
                            "death",
                            key,
                            death,
                        )
                        .await?;
                    }
                }
                "world_online" => {
                    let online = payload
                        .get("characters")
                        .and_then(Value::as_array)
                        .ok_or_else(invalid)?
                        .len();
                    // Observation time prevents manufacturing samples from stale projections.
                    sqlx::query("INSERT INTO stats_world_samples (world,sampled_at,players_online) VALUES ($1,to_timestamp(($2::bigint / 300) * 300),$3) ON CONFLICT DO NOTHING").bind(character).bind(observed).bind(i32::try_from(online).map_err(|_|invalid())?).execute(&mut *tx).await?;
                }
                _ => return Err(invalid()),
            }
            sqlx::query("INSERT INTO stats_projection_cursors (family,subject,epoch,version,watermark) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5) ON CONFLICT (family,subject) DO UPDATE SET epoch=EXCLUDED.epoch,version=EXCLUDED.version,watermark=EXCLUDED.watermark").bind(family).bind(subject).bind(row.try_get::<String,_>("epoch")?).bind(row.try_get::<String,_>("version")?).bind(observed).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        self.retention().await?;
        Ok(count)
    }
    /// One transaction per UTC day, including only samples older than 90 days.
    pub async fn retention(&self) -> Result<usize, sqlx::Error> {
        if !self.enabled {
            return Ok(0);
        }
        let mut days = 0;
        loop {
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(65001)")
                .execute(&mut *tx)
                .await?;
            let day: Option<Date> = sqlx::query_scalar("SELECT min((sampled_at AT TIME ZONE 'UTC')::date) FROM stats_world_samples WHERE sampled_at < now() - interval '90 days'").fetch_one(&mut *tx).await?;
            let Some(day) = day else {
                return Ok(days);
            };
            sqlx::query("INSERT INTO stats_world_daily (world,day,samples,players_sum,players_min,players_max) SELECT world,$1,count(*),sum(players_online),min(players_online),max(players_online) FROM stats_world_samples WHERE (sampled_at AT TIME ZONE 'UTC')::date=$1 AND sampled_at < now() - interval '90 days' GROUP BY world ON CONFLICT (world,day) DO UPDATE SET samples=stats_world_daily.samples+EXCLUDED.samples,players_sum=stats_world_daily.players_sum+EXCLUDED.players_sum,players_min=least(stats_world_daily.players_min,EXCLUDED.players_min),players_max=greatest(stats_world_daily.players_max,EXCLUDED.players_max)").bind(day).execute(&mut *tx).await?;
            sqlx::query(
                "DELETE FROM stats_world_samples WHERE (sampled_at AT TIME ZONE 'UTC')::date=$1 AND sampled_at < now() - interval '90 days'",
            )
            .bind(day)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            days += 1;
        }
    }
}
