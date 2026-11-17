use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use time::Date;
use uuid::Uuid;

type Result<T> = std::result::Result<T, sqlx::Error>;

/// Internal normalized input, also usable by future accepted projection adapters.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub character_ref: Uuid,
    pub day: Date,
    pub level: u64,
    pub experience: Option<String>,
    pub skills: Option<Value>,
    pub achievement_points: Option<i64>,
    pub minutes_online: Option<i32>,
    pub name: String,
    pub world: Uuid,
    pub guild: Option<Uuid>,
    pub observed_at: i64,
}

/// Every public read requires an explicit policy. Unknown characters must be denied.
pub type PermissionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Permissions> + Send + 'a>>;
pub trait Visibility: Send + Sync {
    fn permissions(&self, _character: Uuid) -> Permissions {
        Permissions::default()
    }
    fn current_permissions(&self, character: Uuid) -> PermissionFuture<'_> {
        Box::pin(async move { self.permissions(character) })
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Permissions {
    pub visible: bool,
    pub skills: bool,
    pub online: bool,
    pub guild: bool,
    pub deaths: bool,
}
#[derive(Debug)]
pub struct Daily {
    pub day: Date,
    pub level: u64,
    pub experience: Option<String>,
    pub skills: Option<Value>,
    pub achievement_points: Option<i64>,
    pub minutes_online: Option<i32>,
}
#[derive(Debug)]
pub struct Event {
    pub day: Date,
    pub kind: String,
    pub details: Value,
}
#[derive(Debug)]
pub struct History {
    pub daily: Vec<Daily>,
    pub events: Vec<Event>,
}
#[derive(Debug)]
pub struct Growth {
    pub character_ref: Uuid,
    pub experience_gain: String,
}
fn invalid() -> sqlx::Error {
    sqlx::Error::Protocol("invalid statistics input".into())
}
pub(crate) fn read_level(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64> {
    row.try_get::<&str, _>(column)?
        .parse()
        .map_err(|err| sqlx::Error::Decode(Box::new(err)))
}
fn range(start: Date, end: Date) -> Result<()> {
    if start > end { Err(invalid()) } else { Ok(()) }
}
/// Creates the containing month and two future months, under the worker transaction lock.
pub async fn partitions(tx: &mut Transaction<'_, Postgres>, day: Date) -> Result<()> {
    for offset in 0..3_i32 {
        let (start, end, suffix): (String, String, String) = sqlx::query_as(
            "SELECT d::date::text, (d + interval '1 month')::date::text, to_char(d,'YYYYMM') FROM (SELECT date_trunc('month',$1::date) + make_interval(months => $2) d) m",
        ).bind(day).bind(offset).fetch_one(&mut **tx).await?;
        // Identifiers and date literals are generated exclusively by PostgreSQL.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TABLE IF NOT EXISTS stats_character_daily_{suffix} PARTITION OF stats_character_daily FOR VALUES FROM ('{start}') TO ('{end}')")))
            .execute(&mut **tx).await?;
    }
    Ok(())
}
/// Callers serialize writes with the same advisory transaction lock as the worker.
pub async fn snapshot(tx: &mut Transaction<'_, Postgres>, s: &Snapshot) -> Result<()> {
    if s.level == 0
        || s.name.is_empty()
        || s.observed_at < 0
        || s.experience
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 20 || !v.bytes().all(|b| b.is_ascii_digit()))
        || s.skills.as_ref().is_some_and(|v| !v.is_object())
        || s.achievement_points.is_some_and(|v| v < 0)
        || s.minutes_online.is_some_and(|v| !(0..=1440).contains(&v))
    {
        return Err(invalid());
    }
    let updated = sqlx::query("INSERT INTO stats_character_daily (character_ref,day,level,experience,skills,achievement_points,minutes_online,name,world,guild,observed_at) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT (character_ref,day) DO UPDATE SET level=EXCLUDED.level,experience=EXCLUDED.experience,skills=EXCLUDED.skills,achievement_points=EXCLUDED.achievement_points,minutes_online=EXCLUDED.minutes_online,name=EXCLUDED.name,world=EXCLUDED.world,guild=EXCLUDED.guild,observed_at=EXCLUDED.observed_at WHERE stats_character_daily.observed_at <= EXCLUDED.observed_at AND stats_character_daily IS DISTINCT FROM EXCLUDED")
        .bind(s.character_ref).bind(s.day).bind(s.level.to_string()).bind(&s.experience).bind(&s.skills).bind(s.achievement_points).bind(s.minutes_online).bind(&s.name).bind(s.world).bind(s.guild).bind(s.observed_at).execute(&mut **tx).await?;
    if updated.rows_affected() == 0 {
        return Ok(());
    }
    // Rebuild both affected daily comparisons: revising a predecessor also
    // changes the observation attributed to the next retained snapshot.
    let days: Vec<Date> = sqlx::query_scalar("SELECT day FROM stats_character_daily WHERE character_ref=$1 AND day >= $2 ORDER BY day LIMIT 2")
        .bind(s.character_ref).bind(s.day).fetch_all(&mut **tx).await?;
    for day in days {
        sqlx::query("DELETE FROM stats_character_events WHERE character_ref=$1 AND day=$2 AND kind IN ('name_change','world_change','guild_leave','guild_join','level_up')")
            .bind(s.character_ref).bind(day).execute(&mut **tx).await?;
        let pair = sqlx::query("SELECT c.name,c.world,c.guild,c.level::text,p.name AS old_name,p.world AS old_world,p.guild AS old_guild,p.level::text AS old_level FROM stats_character_daily c JOIN LATERAL (SELECT * FROM stats_character_daily WHERE character_ref=c.character_ref AND day<c.day ORDER BY day DESC LIMIT 1) p ON true WHERE c.character_ref=$1 AND c.day=$2")
            .bind(s.character_ref).bind(day).fetch_optional(&mut **tx).await?;
        if let Some(p) = pair {
            let name: String = p.try_get("name")?;
            let world: Uuid = p.try_get("world")?;
            let guild: Option<Uuid> = p.try_get("guild")?;
            let level = read_level(&p, "level")?;
            let old_name: String = p.try_get("old_name")?;
            let old_world: Uuid = p.try_get("old_world")?;
            let old_guild: Option<Uuid> = p.try_get("old_guild")?;
            let old_level = read_level(&p, "old_level")?;
            for (kind, changed, details) in [
                (
                    "name_change",
                    old_name != name,
                    serde_json::json!({"from":old_name,"to":name}),
                ),
                (
                    "world_change",
                    old_world != world,
                    serde_json::json!({"from":old_world.to_string(),"to":world.to_string()}),
                ),
                (
                    "guild_leave",
                    old_guild != guild && old_guild.is_some(),
                    serde_json::json!({"guild":old_guild.map(|v|v.to_string())}),
                ),
                (
                    "guild_join",
                    old_guild != guild && guild.is_some(),
                    serde_json::json!({"guild":guild.map(|v|v.to_string())}),
                ),
                (
                    "level_up",
                    level > old_level,
                    serde_json::json!({"from":old_level,"to":level}),
                ),
            ] {
                if changed {
                    event(
                        tx,
                        s.character_ref,
                        day,
                        kind,
                        &format!("{day}:{details}"),
                        &details,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}
/// Stable source keys identify deaths and settled bazaar sales; no inferred sales.
pub async fn source_event(
    tx: &mut Transaction<'_, Postgres>,
    character: Uuid,
    day: Date,
    kind: &str,
    source_key: &str,
    details: &Value,
) -> Result<()> {
    if !["death", "bazaar_sale"].contains(&kind) || source_key.is_empty() || source_key.len() > 256
    {
        return Err(invalid());
    }
    event(tx, character, day, kind, source_key, details).await
}
async fn event(
    tx: &mut Transaction<'_, Postgres>,
    character: Uuid,
    day: Date,
    kind: &str,
    key: &str,
    details: &Value,
) -> Result<()> {
    sqlx::query("INSERT INTO stats_character_events (character_ref,day,kind,event_key,details) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
        .bind(character).bind(day).bind(kind).bind(key).bind(details).execute(&mut **tx).await?;
    Ok(())
}
pub async fn history(
    db: &PgPool,
    character: Uuid,
    start: Date,
    end: Date,
    visibility: &impl Visibility,
) -> Result<History> {
    range(start, end)?;
    if (end - start).whole_days() > 365 {
        return Err(invalid());
    }
    let p = visibility.current_permissions(character).await;
    let mut result = History {
        daily: Vec::new(),
        events: Vec::new(),
    };
    if !p.visible {
        return Ok(result);
    }
    for row in sqlx::query("SELECT day,level::text,experience::text,skills,achievement_points,minutes_online FROM stats_character_daily WHERE character_ref=$1 AND day BETWEEN $2 AND $3 ORDER BY day LIMIT 366")
        .bind(character).bind(start).bind(end).fetch_all(db).await? {
        result.daily.push(Daily { day: row.try_get("day")?, level: read_level(&row, "level")?, experience: row.try_get("experience")?, skills: if p.skills {row.try_get("skills")?} else {None}, achievement_points: row.try_get("achievement_points")?, minutes_online: if p.online {row.try_get("minutes_online")?} else {None} });
    }
    for row in sqlx::query("SELECT day,kind,details FROM stats_character_events WHERE character_ref=$1 AND day BETWEEN $2 AND $3 AND ($4 OR kind NOT LIKE 'guild_%') AND ($5 OR kind<>'death') AND (kind<>'death' OR NOT public_projection_conflicted('character_deaths')) ORDER BY day,kind,event_key LIMIT 1001")
        .bind(character).bind(start).bind(end).bind(p.guild).bind(p.deaths).fetch_all(db).await? {
        if result.events.len() == 1000 { return Err(invalid()); }
        let kind: String = row.try_get("kind")?;
        if (kind.starts_with("guild_") && !p.guild) || (kind == "death" && !p.deaths) {continue;}
        result.events.push(Event {day:row.try_get("day")?,kind,details:row.try_get("details")?});
    }
    Ok(result)
}
/// Growth requires both range endpoints. Privacy filtering precedes the top-N limit.
pub async fn top_growth(
    db: &PgPool,
    start: Date,
    end: Date,
    limit: usize,
    visibility: &impl Visibility,
) -> Result<Vec<Growth>> {
    range(start, end)?;
    if !(1..=1000).contains(&limit) {
        return Err(invalid());
    }
    let mut result = Vec::new();
    let mut cursor: Option<(String, Uuid)> = None;
    loop {
        let rows = sqlx::query("SELECT a.character_ref,(b.experience-a.experience)::text AS gain FROM stats_character_daily a JOIN stats_character_daily b USING(character_ref) WHERE a.day=$1 AND b.day=$2 AND a.experience IS NOT NULL AND b.experience IS NOT NULL AND ($3::text IS NULL OR b.experience-a.experience < $3::text::numeric OR (b.experience-a.experience = $3::text::numeric AND a.character_ref>$4)) ORDER BY b.experience-a.experience DESC,a.character_ref LIMIT 100")
            .bind(start).bind(end).bind(cursor.as_ref().map(|v|v.0.as_str())).bind(cursor.as_ref().map(|v|v.1)).fetch_all(db).await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let character_ref: Uuid = row.try_get("character_ref")?;
            let gain: String = row.try_get("gain")?;
            cursor = Some((gain.clone(), character_ref));
            if visibility.current_permissions(character_ref).await.visible {
                result.push(Growth {
                    character_ref,
                    experience_gain: gain,
                });
                if result.len() == limit {
                    return Ok(result);
                }
            }
        }
    }
    Ok(result)
}
