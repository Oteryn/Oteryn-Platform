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
    pub level: i64,
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
pub trait Visibility {
    fn permissions(&self, character: Uuid) -> Permissions;
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
    pub level: i64,
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
    if s.level <= 0
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
    let updated = sqlx::query("INSERT INTO stats_character_daily (character_ref,day,level,experience,skills,achievement_points,minutes_online,name,world,guild,observed_at) VALUES ($1,$2,$3,$4::text::numeric,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT (character_ref,day) DO UPDATE SET level=EXCLUDED.level,experience=EXCLUDED.experience,skills=EXCLUDED.skills,achievement_points=EXCLUDED.achievement_points,minutes_online=EXCLUDED.minutes_online,name=EXCLUDED.name,world=EXCLUDED.world,guild=EXCLUDED.guild,observed_at=EXCLUDED.observed_at WHERE stats_character_daily.observed_at <= EXCLUDED.observed_at AND stats_character_daily IS DISTINCT FROM EXCLUDED")
        .bind(s.character_ref).bind(s.day).bind(s.level).bind(&s.experience).bind(&s.skills).bind(s.achievement_points).bind(s.minutes_online).bind(&s.name).bind(s.world).bind(s.guild).bind(s.observed_at).execute(&mut **tx).await?;
    if updated.rows_affected() == 0 {
        return Ok(());
    }
    let previous = sqlx::query("SELECT name,world,guild,level FROM stats_character_daily WHERE character_ref=$1 AND day < $2 ORDER BY day DESC LIMIT 1")
        .bind(s.character_ref).bind(s.day).fetch_optional(&mut **tx).await?;
    if let Some(p) = previous {
        let old_name: String = p.try_get("name")?;
        let old_world: Uuid = p.try_get("world")?;
        let old_guild: Option<Uuid> = p.try_get("guild")?;
        let old_level: i64 = p.try_get("level")?;
        for (kind, changed, details) in [
            (
                "name_change",
                old_name != s.name,
                serde_json::json!({"from":old_name,"to":s.name}),
            ),
            (
                "world_change",
                old_world != s.world,
                serde_json::json!({"from":old_world.to_string(),"to":s.world.to_string()}),
            ),
            (
                "guild_leave",
                old_guild != s.guild && old_guild.is_some(),
                serde_json::json!({"guild":old_guild.map(|v|v.to_string())}),
            ),
            (
                "guild_join",
                old_guild != s.guild && s.guild.is_some(),
                serde_json::json!({"guild":s.guild.map(|v|v.to_string())}),
            ),
            (
                "level_up",
                s.level > old_level,
                serde_json::json!({"from":old_level,"to":s.level}),
            ),
        ] {
            if changed {
                event(
                    tx,
                    s.character_ref,
                    s.day,
                    kind,
                    &format!("{}:{details}", s.day),
                    &details,
                )
                .await?;
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
    let p = visibility.permissions(character);
    let mut result = History {
        daily: Vec::new(),
        events: Vec::new(),
    };
    if !p.visible {
        return Ok(result);
    }
    for row in sqlx::query("SELECT day,level,experience::text,skills,achievement_points,minutes_online FROM stats_character_daily WHERE character_ref=$1 AND day BETWEEN $2 AND $3 ORDER BY day")
        .bind(character).bind(start).bind(end).fetch_all(db).await? {
        result.daily.push(Daily { day: row.try_get("day")?, level: row.try_get("level")?, experience: row.try_get("experience")?, skills: if p.skills {row.try_get("skills")?} else {None}, achievement_points: row.try_get("achievement_points")?, minutes_online: if p.online {row.try_get("minutes_online")?} else {None} });
    }
    for row in sqlx::query("SELECT day,kind,details FROM stats_character_events WHERE character_ref=$1 AND day BETWEEN $2 AND $3 ORDER BY day,kind,event_key")
        .bind(character).bind(start).bind(end).fetch_all(db).await? {
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
    let rows = sqlx::query("SELECT a.character_ref,(b.experience-a.experience)::text AS gain FROM stats_character_daily a JOIN stats_character_daily b USING(character_ref) WHERE a.day=$1 AND b.day=$2 AND a.experience IS NOT NULL AND b.experience IS NOT NULL ORDER BY b.experience-a.experience DESC,a.character_ref")
        .bind(start).bind(end).fetch_all(db).await?;
    let mut result = Vec::new();
    for row in rows {
        let character_ref = row.try_get("character_ref")?;
        if visibility.permissions(character_ref).visible {
            result.push(Growth {
                character_ref,
                experience_gain: row.try_get("gain")?,
            });
            if result.len() == limit {
                break;
            }
        }
    }
    Ok(result)
}
