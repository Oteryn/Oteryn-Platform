use super::*;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

/// Authenticated transport adapters call this only after purpose-bound mTLS.
/// Parsing, flags and configured source authority are checked again at this boundary.
pub async fn ingest(
    db: &PgPool,
    config: &Config,
    family: Option<Family>,
    raw: &[u8],
    now: i64,
) -> Result<Receipt, Error> {
    if !config.enabled {
        return Err(Error::Unavailable);
    }
    let p = wire::publication(family, raw, now)?;
    if !config.permits(p.family) {
        return Err(Error::Unavailable);
    }
    if p.authority != config.source_authority {
        return Err(Error::Unauthorized);
    }
    let mut tx = db.begin().await?;
    // Same lock as v1: an epoch raise fences every public-feed family atomically.
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if highest
        .as_ref()
        .is_some_and(|h| compare(&p.epoch, h).is_lt())
    {
        return Ok(Receipt::Superseded);
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(&p.epoch)
        .execute(&mut *tx)
        .await?;
    if let Some(complete) = p.watermark {
        let row = sqlx::query("SELECT epoch::text, complete_through, observed_at FROM stats_v2_watermarks WHERE source_authority=$1 AND family=$2").bind(&p.authority).bind(p.family.name()).fetch_optional(&mut *tx).await?;
        if let Some(row) = row {
            let epoch: String = row.try_get("epoch")?;
            let old: i64 = row.try_get("complete_through")?;
            let observed: i64 = row.try_get("observed_at")?;
            if epoch == p.epoch {
                if complete < old || p.observed < observed {
                    return Ok(Receipt::Superseded);
                }
                if complete == old && p.observed == observed {
                    return Ok(Receipt::Accepted);
                }
            }
        }
        sqlx::query("INSERT INTO stats_v2_watermarks VALUES ($1,$2,$3::numeric,$4,$5) ON CONFLICT (source_authority,family) DO UPDATE SET epoch=EXCLUDED.epoch,complete_through=EXCLUDED.complete_through,observed_at=EXCLUDED.observed_at")
            .bind(&p.authority).bind(p.family.name()).bind(&p.epoch).bind(complete).bind(p.observed).execute(&mut *tx).await?;
    } else {
        let mut content = p.content.clone();
        content.sort_all_objects();
        let digest =
            Sha256::digest(serde_json::to_vec(&content).map_err(|_| Error::Invalid)?).to_vec();
        let row = sqlx::query("SELECT epoch::text,revision::text,content_digest,payload,erased,expires_at,closed FROM stats_v2_subjects WHERE source_authority=$1 AND family=$2 AND subject=$3")
            .bind(&p.authority).bind(p.family.name()).bind(&p.subject).fetch_optional(&mut *tx).await?;
        let mut erased = p.expires <= now;
        let mut closed = p
            .interval
            .as_ref()
            .and_then(|w| w.sessions.first())
            .is_some_and(|s| !s.ended_at.is_empty());
        if let Some(row) = row {
            let epoch: String = row.try_get("epoch")?;
            let revision: String = row.try_get("revision")?;
            let order = compare(&p.epoch, &epoch).then_with(|| compare(&p.revision, &revision));
            if order.is_lt() {
                return Ok(Receipt::Superseded);
            }
            if order.is_eq() {
                let previous: Vec<u8> = row.try_get("content_digest")?;
                return if previous == digest {
                    Ok(Receipt::Accepted)
                } else {
                    Err(Error::Conflict)
                };
            }
            let expired: i64 = row.try_get("expires_at")?;
            // An expired interval cannot acquire a later end through a delayed correction.
            // Preserve the minimal subject/revision fence instead of resurrecting payloads.
            erased |= row.try_get::<bool, _>("erased")? || expired <= now;
            if row.try_get::<bool, _>("closed")? {
                closed = true;
                if p.interval
                    .as_ref()
                    .and_then(|w| w.sessions.first())
                    .is_some_and(|s| s.ended_at.is_empty())
                {
                    return Err(Error::Invalid);
                }
            }
        }
        if !erased
            && let Some(w) = &p.interval
            && let Some(s) = w.sessions.first()
        {
            let start = wire::decimal(&s.started_at, false)? as i64;
            let end = s
                .ended_at
                .first()
                .map(|s| wire::decimal(s, false))
                .transpose()?
                .map(|v| v as i64)
                .unwrap_or(i64::MAX);
            if start < end {
                let overlaps: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stats_v2_subjects WHERE source_authority=$1 AND family='character_online_sessions' AND subject<>$2 AND epoch=$3::numeric AND NOT erased AND expires_at>$4 AND payload->>'character_id'=$5 AND jsonb_array_length(payload->'sessions')=1 AND (payload->'sessions'->0->>'started_at')::bigint < $7 AND COALESCE((payload->'sessions'->0->'ended_at'->>0)::bigint,9223372036854775807)>$6 AND COALESCE((payload->'sessions'->0->'ended_at'->>0)::bigint,9223372036854775807)>(payload->'sessions'->0->>'started_at')::bigint)")
                    .bind(&p.authority).bind(&p.subject).bind(&p.epoch).bind(now).bind(&w.character_id).bind(start).bind(end).fetch_one(&mut *tx).await?;
                if overlaps {
                    return Err(Error::Invalid);
                }
            }
        }
        let payload = (!erased).then_some(&p.content);
        let open_unresolved = p.interval.as_ref().is_some_and(|w| {
            w.sessions
                .first()
                .map_or(!closed, |s| s.ended_at.is_empty())
        });
        sqlx::query("INSERT INTO stats_v2_subjects (source_authority,family,subject,epoch,revision,observed_at,content_digest,payload,expires_at,erased,open_unresolved,closed) VALUES ($1,$2,$3,$4::numeric,$5::numeric,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (source_authority,family,subject) DO UPDATE SET epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,observed_at=EXCLUDED.observed_at,content_digest=EXCLUDED.content_digest,payload=EXCLUDED.payload,expires_at=EXCLUDED.expires_at,erased=EXCLUDED.erased,open_unresolved=EXCLUDED.open_unresolved,closed=EXCLUDED.closed")
            .bind(&p.authority).bind(p.family.name()).bind(&p.subject).bind(&p.epoch).bind(&p.revision).bind(if erased { 1 } else { p.observed }).bind(digest).bind(payload).bind(if erased && p.expires <= now { 1 } else { p.expires }).bind(erased).bind(open_unresolved).bind(closed).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Receipt::Accepted)
}
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
/// Erase expired payloads while preserving non-payload replay and epoch fences.
/// Run at least daily. Serve-time expiry applies independently of this cleanup.
pub async fn purge(db: &PgPool, now: i64) -> Result<u64, Error> {
    if now <= 0 {
        return Err(Error::Invalid);
    }
    let mut tx = db.begin().await?;
    sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR UPDATE")
        .fetch_one(&mut *tx)
        .await?;
    let erased = sqlx::query("UPDATE stats_v2_subjects SET payload=NULL,erased=true,observed_at=1,expires_at=1 WHERE expires_at<=$1 AND (payload IS NOT NULL OR observed_at<>1 OR expires_at<>1)").bind(now).execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(erased)
}
pub(crate) async fn fresh(
    tx: &mut Transaction<'_, Postgres>,
    config: &Config,
    family: Family,
    now: i64,
    coverage: &impl CoveragePolicy,
) -> Result<Coverage, Error> {
    if now <= 0 {
        return Err(Error::Invalid);
    }
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR SHARE",
    )
    .fetch_one(&mut **tx)
    .await?;
    let cut = coverage.coverage(family)?;
    wire::decimal(&cut.epoch, false).map_err(|_| Error::Unavailable)?;
    if highest.as_ref() != Some(&cut.epoch)
        || cut.start < 0
        || cut.start > cut.through
        || cut.through > now.saturating_add(1)
    {
        return Err(Error::Unavailable);
    }
    let row = sqlx::query("SELECT complete_through,observed_at FROM stats_v2_watermarks WHERE source_authority=$1 AND family=$2 AND epoch=$3::numeric")
        .bind(&config.source_authority).bind(family.name()).bind(&cut.epoch).fetch_optional(&mut **tx).await?.ok_or(Error::Unavailable)?;
    let complete: i64 = row.try_get("complete_through")?;
    let observed: i64 = row.try_get("observed_at")?;
    if complete > now.saturating_add(1)
        || observed > now.saturating_add(1)
        || now.saturating_sub(complete).saturating_add(1) > 30
        || cut.through > complete
    {
        return Err(Error::Unavailable);
    }
    Ok(cut)
}
