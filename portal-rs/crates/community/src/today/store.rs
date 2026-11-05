use super::{
    Config, Error,
    schema::{self, Publication},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

pub(super) async fn apply(
    db: &PgPool,
    config: &Config,
    publication: &Publication,
) -> Result<&'static str, Error> {
    let (authority, epoch) = match publication {
        Publication::Snapshot(s) => (&s.source_authority, &s.projection_epoch),
        Publication::Watermark(w) => (&w.source_authority, &w.projection_epoch),
    };
    if authority != &config.source_authority {
        return Err(Error::Unauthorized);
    }
    let mut tx = db.begin().await?;
    // All public families share the existing recovery fence and lock ordering.
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if let Publication::Snapshot(s) = publication {
        let id = schema::world_id(&s.world_id)?;
        if !config.worlds.contains(&id)
            || !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM world_status WHERE id=$1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?
        {
            return Err(Error::Unauthorized);
        }
    }
    if highest.as_ref().is_some_and(|h| compare(epoch, h).is_lt()) {
        return Ok("superseded");
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(epoch)
        .execute(&mut *tx)
        .await?;
    let result = match publication {
        Publication::Snapshot(s) => {
            let id = schema::world_id(&s.world_id)?;
            let content = s.content()?;
            let fingerprint = Sha256::digest(content.as_bytes()).to_vec();
            let old = sqlx::query("SELECT epoch::text,revision::text,content,conflicted FROM today_snapshots WHERE authority=$1 AND world_id=$2")
                .bind(authority).bind(id).fetch_optional(&mut *tx).await?;
            if let Some(old) = old {
                let old_epoch: String = old.try_get("epoch")?;
                let old_revision: String = old.try_get("revision")?;
                let order = compare(epoch, &old_epoch)
                    .then_with(|| compare(&s.projection_revision, &old_revision));
                if order.is_lt() {
                    return Ok("superseded");
                }
                if order.is_eq() {
                    if old.try_get::<Option<String>, _>("content")?.as_deref() == Some(&content) {
                        tx.commit().await?;
                        return Ok("accepted");
                    }
                    sqlx::query("UPDATE today_snapshots SET conflicted=true WHERE authority=$1 AND world_id=$2")
                        .bind(authority).bind(id).execute(&mut *tx).await?;
                    tx.commit().await?;
                    return Err(Error::Conflict);
                }
            }
            // A later publication never silently reconciles a quarantined conflict.
            sqlx::query("INSERT INTO today_snapshots (authority,world_id,epoch,revision,observed,content,fingerprint) VALUES ($1,$2,$3::numeric,$4::numeric,$5,$6,$7) ON CONFLICT (authority,world_id) DO UPDATE SET epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,observed=EXCLUDED.observed,content=EXCLUDED.content,fingerprint=EXCLUDED.fingerprint,received_at=now()")
                .bind(authority).bind(id).bind(epoch).bind(&s.projection_revision)
                .bind(schema::seconds(&s.source_observed_at)?).bind(content).bind(fingerprint)
                .execute(&mut *tx).await?;
            "accepted"
        }
        Publication::Watermark(w) => {
            let complete = schema::seconds(&w.complete_through)?;
            let old: Option<(String, i64)> = sqlx::query_as(
                "SELECT epoch::text,complete_through FROM today_watermarks WHERE authority=$1",
            )
            .bind(authority)
            .fetch_optional(&mut *tx)
            .await?;
            if old.is_some_and(|(e, c)| e == *epoch && complete <= c) {
                tx.commit().await?;
                return Ok("superseded");
            }
            sqlx::query("INSERT INTO today_watermarks (authority,epoch,complete_through,observed) VALUES ($1,$2::numeric,$3,$4) ON CONFLICT (authority) DO UPDATE SET epoch=EXCLUDED.epoch,complete_through=EXCLUDED.complete_through,observed=EXCLUDED.observed")
                .bind(authority).bind(epoch).bind(complete).bind(schema::seconds(&w.observed_at)?)
                .execute(&mut *tx).await?;
            "accepted"
        }
    };
    tx.commit().await?;
    Ok(result)
}
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// Expire content while retaining minimal permanent replay/conflict fences.
/// No raw envelope or conflicting payload is retained.
#[derive(Debug)]
pub struct Retention(pub PgPool);
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async {
            sqlx::query("UPDATE today_snapshots SET content=NULL WHERE content IS NOT NULL AND received_at < now()-interval '30 days'")
                .execute(&self.0).await.map_err(|_| "daily retention unavailable".to_owned())?;
            Ok(())
        })
    }
}
