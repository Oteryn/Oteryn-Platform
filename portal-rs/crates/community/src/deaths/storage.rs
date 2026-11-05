use super::{
    now,
    wire::{Death, Snapshot, Watermark},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};

#[derive(Debug)]
pub(super) enum Error {
    Database,
    Conflict,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}
fn digest<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value)
        .map(|v| Sha256::digest(v).to_vec())
        .map_err(|_| Error::Database)
}
async fn fence(tx: &mut Transaction<'_, Postgres>, epoch: &str) -> Result<bool, Error> {
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut **tx)
    .await?;
    if highest.is_some_and(|h| compare(epoch, &h).is_lt()) {
        return Ok(false);
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(epoch)
        .execute(&mut **tx)
        .await?;
    Ok(true)
}
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
pub(super) async fn snapshot(db: &PgPool, s: &Snapshot) -> Result<&'static str, Error> {
    let mut tx = db.begin().await?;
    if !fence(&mut tx, &s.projection_epoch).await? {
        return Ok("superseded");
    }
    let hash = digest(s)?;
    let current = sqlx::query(
        "SELECT epoch::text, revision::text, digest FROM public_deaths_v2 WHERE subject=$1",
    )
    .bind(&s.character_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(row) = current {
        let epoch: String = row.try_get("epoch")?;
        let revision: String = row.try_get("revision")?;
        let order = compare(&s.projection_epoch, &epoch)
            .then_with(|| compare(&s.projection_revision, &revision));
        if order.is_lt() {
            return Ok("superseded");
        }
        if order.is_eq() {
            let old: Vec<u8> = row.try_get("digest")?;
            return if old == hash {
                Ok("accepted")
            } else {
                Err(Error::Conflict)
            };
        }
    }
    for death in &s.deaths {
        let hash = digest(death)?;
        let old: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT digest FROM public_deaths_v2_occurrences WHERE subject=$1 AND occurrence=$2",
        )
        .bind(&s.character_id)
        .bind(&death.death_occurrence_id)
        .fetch_optional(&mut *tx)
        .await?;
        if old.is_some_and(|old| old != hash) {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO public_deaths_v2_occurrences (subject,occurrence,digest) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING").bind(&s.character_id).bind(&death.death_occurrence_id).bind(hash).execute(&mut *tx).await?;
    }
    let cutoff = now().saturating_sub(30 * 86400);
    let deaths: Vec<&Death> = s
        .deaths
        .iter()
        .filter(|d| d.occurred_at.parse::<u64>().is_ok_and(|t| t >= cutoff))
        .collect();
    let payload = serde_json::to_value(deaths).map_err(|_| Error::Database)?;
    sqlx::query("INSERT INTO public_deaths_v2 (subject,authority,epoch,revision,digest,deaths) VALUES ($1,$2,$3::numeric,$4::numeric,$5,$6) ON CONFLICT (subject) DO UPDATE SET authority=EXCLUDED.authority, epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,digest=EXCLUDED.digest,deaths=EXCLUDED.deaths").bind(&s.character_id).bind(&s.source_authority).bind(&s.projection_epoch).bind(&s.projection_revision).bind(hash).bind(payload).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok("accepted")
}
pub(super) async fn watermark(db: &PgPool, w: &Watermark) -> Result<&'static str, Error> {
    let mut tx = db.begin().await?;
    if !fence(&mut tx, &w.projection_epoch).await? {
        return Ok("superseded");
    }
    let through: i64 = w.complete_through.parse().map_err(|_| Error::Database)?;
    let old = sqlx::query(
        "SELECT epoch::text, complete_through FROM public_deaths_v2_watermark WHERE id=1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(row) = old {
        let epoch: String = row.try_get("epoch")?;
        let previous: i64 = row.try_get("complete_through")?;
        if epoch == w.projection_epoch && through <= previous {
            return Ok("superseded");
        }
    }
    sqlx::query("INSERT INTO public_deaths_v2_watermark (id,authority,epoch,complete_through) VALUES (1,$1,$2::numeric,$3) ON CONFLICT (id) DO UPDATE SET authority=EXCLUDED.authority,epoch=EXCLUDED.epoch,complete_through=EXCLUDED.complete_through").bind(&w.source_authority).bind(&w.projection_epoch).bind(through).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok("accepted")
}
/// Purge expired bodies and fenced generations without expiring replay fences.
#[derive(Debug)]
pub struct Cleanup {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for Cleanup {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async {
            sqlx::query("UPDATE public_deaths_v2 SET deaths=CASE WHEN epoch < (SELECT highest_epoch FROM public_projection_state WHERE id=1) THEN '[]'::jsonb ELSE COALESCE((SELECT jsonb_agg(d ORDER BY ord) FROM jsonb_array_elements(deaths) WITH ORDINALITY AS e(d,ord) WHERE (d->>'occurred_at')::bigint >= $1),'[]'::jsonb) END").bind(i64::try_from(now().saturating_sub(30*86400)).map_err(|_| "invalid clock".to_owned())?).execute(&self.pool).await.map_err(|_| "deaths retention unavailable".to_owned())?;
            Ok(())
        })
    }
}
