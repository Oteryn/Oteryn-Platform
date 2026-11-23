use super::*;
use crate::store::{Freshness, TrustedTime, stream_key};
use oteryn_internal_api::canonical;
use sqlx::PgPool;

pub(crate) struct Observation {
    pub occurred_ms: i64,
    pub payload: Value,
}

pub(crate) fn mapping_pending(family: &str) -> bool {
    UNPROVEN_PROGRESS.contains(&family)
}

fn freshness(family: &str, now: u64, observed: i64) -> Result<Freshness> {
    let age = u64::try_from(observed)
        .ok()
        .and_then(|time| now.checked_sub(time))
        .ok_or(Error::Unavailable)?;
    match age {
        0..=10_000 => Ok(Freshness::Fresh),
        10_001..=86_400_000 if matches!(family, "skill_history" | "pvp") => Ok(Freshness::Stale),
        _ => Err(Error::Unavailable),
    }
}

/// Read the published T72 prefix at the currently proven ownership and recovery cut.
pub(crate) async fn read_character_progress(
    pool: &PgPool,
    actor: Uuid,
    character: Uuid,
    source: &str,
    family: &str,
    time: TrustedTime,
) -> Result<(Freshness, Vec<Observation>)> {
    if !matches!(family, "skill_history" | "pvp" | "skull" | "stamina")
        || time.uncertainty_ms > 5_000
        || source.is_empty()
    {
        return Err(Error::Unavailable);
    }
    let mut tx = pool.begin().await?;
    let account_id = actor.to_string();
    let character_id = character.to_string();
    crate::store::lock_account(&mut tx, &account_id).await?;
    crate::store::shared_authority(&mut tx, source).await?;
    account(&mut tx, actor).await?;
    shared_fences(&mut tx).await?;
    let feed = live_feed(&mut tx).await?;
    let epoch = account_epoch_with_feed(&mut tx, actor, &feed).await?;
    let Ownership::Owner(cut) = ownership_with_feed(&mut tx, actor, character, &feed).await? else {
        return Err(Error::Denied);
    };
    let scope = Scope {
        authority: source.to_owned(),
        account: account_id,
        world: Some(cut.world.to_string()),
        character: Some(character_id),
        character_revision: Some(cut.character_revision.to_string()),
        from_projection_revision: None,
        projection_revision: Some(cut.to_projection_revision.to_string()),
        epoch: Some(epoch.to_string()),
        recovery: String::new(),
        family: family.to_owned(),
    };
    let key = stream_key(&scope);
    let row = sqlx::query(
        "SELECT published_through::text AS through, observed_ms, watermark_recovery::text AS recovery
         FROM pd_streams WHERE stream_key=$1 AND baseline_complete AND NOT erased
         AND watermark_recovery >= recovery_high_water FOR SHARE",
    )
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::Unavailable)?;
    let recovery: String = row.try_get("recovery")?;
    let observed: i64 = row.try_get("observed_ms")?;
    let floor = i64::try_from(cut.decided_at)
        .ok()
        .and_then(|s| s.checked_mul(1_000))
        .ok_or(Error::Unavailable)?;
    if observed < floor
        || !matches!(admit(&mut tx, &Scope { recovery: recovery.clone(), ..scope }).await?, Admission::CharacterOwner(revision) if revision == cut.character_revision)
    {
        return Err(Error::Unavailable);
    }
    let latest_now = time
        .unix_ms
        .checked_add(time.uncertainty_ms)
        .ok_or(Error::Unavailable)?;
    if u64::try_from(observed).map_err(|_| Error::Unavailable)? > time.unix_ms {
        return Err(Error::Unavailable);
    }
    let freshness = freshness(family, latest_now, observed)?;
    // Ownership_revision is bound by admission. For skull the envelope projection
    // revision is the Game table revision, so it cannot identify the ownership cut.
    let rows = sqlx::query(
        "SELECT payload,expires_ms,occurred_ms,kind FROM (
            SELECT DISTINCT ON (entity_key) payload,expires_ms,occurred_ms,kind
            FROM pd_records WHERE stream_key=$1 AND character_id=$2
            AND ownership_revision=$3 AND split_part(scope_key,'|',5)=$4
            AND source_revision<=$5::text::numeric
            ORDER BY entity_key,source_revision DESC
         ) AS current ORDER BY occurred_ms DESC LIMIT 201",
    )
    .bind(key)
    .bind(character.to_string())
    .bind(cut.character_revision.to_string())
    .bind(format!("{}:{recovery}", recovery.len()))
    .bind(row.try_get::<String, _>("through")?)
    .fetch_all(&mut *tx)
    .await?;
    if rows.len() > 200 {
        return Err(Error::Unavailable);
    }
    let mut observations = Vec::new();
    for row in rows {
        if row.try_get::<String, _>("kind")? == "REMOVE" {
            continue;
        }
        let occurred_ms: i64 = row.try_get("occurred_ms")?;
        if occurred_ms < floor {
            continue;
        }
        if occurred_ms > observed {
            return Err(Error::Unavailable);
        }
        let expires =
            u64::try_from(row.try_get::<i64, _>("expires_ms")?).map_err(|_| Error::Unavailable)?;
        if expires <= latest_now {
            if matches!(family, "skill_history" | "pvp") {
                continue;
            }
            return Err(Error::Unavailable);
        }
        let bytes: Vec<u8> = row.try_get("payload")?;
        observations.push(Observation {
            occurred_ms,
            payload: canonical::decode(&bytes, 262144, 9).map_err(|_| Error::Unavailable)?,
        });
    }
    if observations.is_empty()
        && (matches!(family, "stamina" | "skull")
            || cut.from_projection_revision != cut.to_projection_revision)
    {
        return Err(Error::Unavailable);
    }
    tx.commit().await?;
    Ok((freshness, observations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_historical_delay_but_refuses_old_or_future_data_and_stale_timers() {
        assert!(matches!(freshness("pvp", 10_000, 0), Ok(Freshness::Fresh)));
        assert!(matches!(freshness("pvp", 10_001, 0), Ok(Freshness::Stale)));
        for (family, now, observed) in [
            ("pvp", 86_400_001, 0),
            ("pvp", 2, 3),
            ("skull", 10_001, 0),
            ("stamina", 10_001, 0),
        ] {
            assert!(matches!(
                freshness(family, now, observed),
                Err(Error::Unavailable)
            ));
        }
    }
}
