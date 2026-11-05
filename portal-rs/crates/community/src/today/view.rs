use super::{
    Config, Error,
    schema::{self, Daily},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Boost {
    pub creature: String,
    pub boss: String,
    pub from: i64,
    pub until: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Raid {
    pub name: String,
    pub scheduled_at: i64,
    pub state: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct DailyView {
    pub world: String,
    pub current: Boost,
    pub history_complete_from: i64,
    pub history: Vec<Boost>,
    pub raids: Vec<Raid>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Content {
    world_id: String,
    daily: Vec<Daily>,
}

/// One repeatable-read transaction fences the snapshot, watermark and current
/// catalogue visibility/names. No network provider participates in rendering.
pub async fn read(
    db: &PgPool,
    config: &Config,
    world: Uuid,
    lang: &str,
    now: i64,
) -> Result<DailyView, Error> {
    if !matches!(lang, "pl" | "en") {
        return Err(Error::BadInput);
    }
    if !config.public_enabled || !config.valid() || !config.worlds.contains(&world) || now < 1 {
        return Err(Error::Unavailable);
    }
    let mut tx = db.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query("SELECT s.content,s.observed,w.complete_through,w.observed AS watermark_observed,r.name FROM today_snapshots s JOIN today_watermarks w ON w.authority=s.authority AND w.epoch=s.epoch JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch JOIN world_status r ON r.id=s.world_id WHERE s.authority=$1 AND s.world_id=$2 AND NOT s.conflicted AND s.content IS NOT NULL AND s.received_at >= now()-interval '30 days'")
        .bind(&config.source_authority).bind(world).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
    let complete: i64 = row.try_get("complete_through")?;
    let observed: i64 = row.try_get("observed")?;
    let watermark_observed: i64 = row.try_get("watermark_observed")?;
    if now.saturating_sub(complete).saturating_add(1) > 30
        || complete > watermark_observed
        || watermark_observed > now.saturating_add(1)
        || observed > complete
        || observed > now.saturating_add(1)
    {
        return Err(Error::Unavailable);
    }
    // A family watermark covers every configured World. Missing baseline denies
    // the generation, rather than publishing whichever World arrived first.
    let worlds: Vec<Uuid> = config.worlds.iter().copied().collect();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM today_snapshots s JOIN world_status r ON r.id=s.world_id WHERE s.authority=$1 AND s.world_id=ANY($2) AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND s.observed <= $3 AND NOT s.conflicted AND s.content IS NOT NULL AND s.received_at >= now()-interval '30 days'")
        .bind(&config.source_authority).bind(&worlds).bind(complete).fetch_one(&mut *tx).await?;
    if count != worlds.len() as i64 {
        return Err(Error::Unavailable);
    }
    let content: String = row.try_get("content")?;
    let content: Content = serde_json::from_str(&content).map_err(|_| Error::Unavailable)?;
    if content.world_id != world.to_string() || content.daily.len() != 1 {
        return Err(Error::Unavailable);
    }
    let d = &content.daily[0];
    d.validate(observed).map_err(|_| Error::Unavailable)?;
    let (from, until) = (
        schema::seconds(&d.effective_from)?,
        schema::seconds(&d.effective_until)?,
    );
    // One second of clock uncertainty must fit entirely inside the half-open interval.
    if now - 1 < from || now.saturating_add(1) >= until {
        return Err(Error::Unavailable);
    }
    let mut creature_keys = vec![d.creature_key.as_str(), d.boss_key.as_str()];
    for h in &d.history {
        creature_keys.extend([h.creature_key.as_str(), h.boss_key.as_str()]);
    }
    let raid_keys: Vec<&str> = d.raids.iter().map(|r| r.raid_key.as_str()).collect();
    let rows: Vec<(String,String,String)> = sqlx::query_as("SELECT e.kind,e.key,CASE WHEN $3='pl' THEN n.pl ELSE n.en END FROM world_entities e JOIN world_active_version a ON a.version_id=e.version_id JOIN world_versions r ON r.id=e.version_id AND r.status='ready' JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed JOIN world_admin_names n ON n.kind=e.kind AND n.key=e.key WHERE e.publishable AND ((e.kind='Creature' AND e.key=ANY($1)) OR (e.kind='Raid' AND e.key=ANY($2)))")
        .bind(&creature_keys).bind(&raid_keys).bind(lang).fetch_all(&mut *tx).await?;
    let names: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(kind, key, name)| ((kind, key), name))
        .collect();
    let current = boost(&names, &d.creature_key, &d.boss_key, from, until)?;
    let mut history = Vec::new();
    for h in &d.history {
        history.push(boost(
            &names,
            &h.creature_key,
            &h.boss_key,
            schema::seconds(&h.effective_from)?,
            schema::seconds(&h.effective_until)?,
        )?);
    }
    let mut raids = Vec::new();
    for r in &d.raids {
        raids.push(Raid {
            name: name(&names, "Raid", &r.raid_key)?,
            scheduled_at: schema::seconds(&r.scheduled_at)?,
            state: r.state.clone(),
        });
    }
    let result = DailyView {
        world: row.try_get("name")?,
        current,
        history_complete_from: schema::seconds(&d.history_complete_from)?,
        history,
        raids,
    };
    tx.commit().await?;
    Ok(result)
}
fn boost(
    names: &BTreeMap<(String, String), String>,
    creature: &str,
    boss: &str,
    from: i64,
    until: i64,
) -> Result<Boost, Error> {
    Ok(Boost {
        creature: name(names, "Creature", creature)?,
        boss: name(names, "Creature", boss)?,
        from,
        until,
    })
}
fn name(
    names: &BTreeMap<(String, String), String>,
    kind: &str,
    key: &str,
) -> Result<String, Error> {
    let name = names
        .get(&(kind.to_owned(), key.to_owned()))
        .ok_or(Error::Unavailable)?;
    if name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
        return Err(Error::Unavailable);
    }
    Ok(name.clone())
}
