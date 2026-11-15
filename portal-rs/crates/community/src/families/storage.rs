//! One current snapshot per (family, character); every write is one transaction under
//! T24's shared epoch fence, so a refused or failed write leaves the previous row intact.
use super::Family;
use sqlx::{PgConnection, PgPool, Row};

#[derive(Debug)]
pub(super) enum Error {
    Conflict,
    Database,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}
fn order(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
/// Locks the shared epoch row; `false` when the publication is below the fence.
async fn fence(tx: &mut PgConnection, epoch: &str) -> Result<bool, Error> {
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if highest.as_deref().is_some_and(|h| order(epoch, h).is_lt()) {
        return Ok(false);
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(epoch)
        .execute(&mut *tx)
        .await?;
    Ok(true)
}

pub(super) struct Publication<'a> {
    pub family: Family,
    pub character: uuid::Uuid,
    pub authority: &'a str,
    pub epoch: &'a str,
    pub revision: &'a str,
    pub observed: i64,
    pub body: serde_json::Value,
}

pub(super) async fn snapshot(db: &PgPool, p: &Publication<'_>) -> Result<&'static str, Error> {
    let mut tx = db.begin().await?;
    if !fence(&mut tx, p.epoch).await? {
        return Ok("superseded");
    }
    let old = sqlx::query("SELECT epoch::text AS epoch,revision::text AS revision,body FROM public_character_families_v2 WHERE family=$1 AND character_id=$2 FOR UPDATE")
        .bind(p.family.name())
        .bind(p.character)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(old) = old {
        let epoch: String = old.try_get("epoch")?;
        let revision: String = old.try_get("revision")?;
        let position = order(p.epoch, &epoch).then_with(|| order(p.revision, &revision));
        if position.is_lt() {
            return Ok("superseded");
        }
        if position.is_eq() {
            // Equal content accepts without refreshing the first observation.
            return if old.try_get::<serde_json::Value, _>("body")? == p.body {
                Ok("accepted")
            } else {
                Err(Error::Conflict)
            };
        }
    }
    sqlx::query("INSERT INTO public_character_families_v2 (family,character_id,source_authority,epoch,revision,observed,body) VALUES ($1,$2,$3,$4::numeric,$5::numeric,$6,$7) ON CONFLICT (family,character_id) DO UPDATE SET source_authority=EXCLUDED.source_authority,epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,observed=EXCLUDED.observed,body=EXCLUDED.body")
        .bind(p.family.name())
        .bind(p.character)
        .bind(p.authority)
        .bind(p.epoch)
        .bind(p.revision)
        .bind(p.observed)
        .bind(&p.body)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok("accepted")
}

pub(super) async fn watermark(
    db: &PgPool,
    family: Family,
    authority: &str,
    epoch: &str,
    complete_through: i64,
) -> Result<&'static str, Error> {
    let mut tx = db.begin().await?;
    if !fence(&mut tx, epoch).await? {
        return Ok("superseded");
    }
    let old = sqlx::query("SELECT epoch::text AS epoch,complete_through FROM public_character_families_v2_watermarks WHERE family=$1 FOR UPDATE")
        .bind(family.name())
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(old) = old {
        let old_epoch: String = old.try_get("epoch")?;
        let position = order(epoch, &old_epoch);
        if position.is_lt()
            || (position.is_eq() && old.try_get::<i64, _>("complete_through")? >= complete_through)
        {
            return Ok("superseded");
        }
    }
    sqlx::query("INSERT INTO public_character_families_v2_watermarks (family,source_authority,epoch,complete_through) VALUES ($1,$2,$3::numeric,$4) ON CONFLICT (family) DO UPDATE SET source_authority=EXCLUDED.source_authority,epoch=EXCLUDED.epoch,complete_through=EXCLUDED.complete_through")
        .bind(family.name())
        .bind(authority)
        .bind(epoch)
        .bind(complete_through)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok("accepted")
}

#[derive(Debug)]
pub(super) struct Unavailable;
impl From<sqlx::Error> for Unavailable {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}
/// The family's current epoch when its watermark is fresh (S=30 s, 1 s uncertainty)
/// and on the shared fence; anything else is unavailable, never empty.
pub(super) async fn fresh(
    db: &mut PgConnection,
    family: Family,
    authority: &str,
    now: u64,
) -> Result<(String, i64), Unavailable> {
    let row = sqlx::query("SELECT w.epoch::text AS epoch,w.complete_through FROM public_character_families_v2_watermarks w JOIN public_projection_state s ON s.id=1 AND s.highest_epoch=w.epoch WHERE w.family=$1 AND w.source_authority=$2")
        .bind(family.name())
        .bind(authority)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Unavailable)?;
    let through: i64 = row.try_get("complete_through")?;
    let through = u64::try_from(through).map_err(|_| Unavailable)?;
    if through > now.saturating_add(1) || now.saturating_sub(through).saturating_add(1) > 30 {
        return Err(Unavailable);
    }
    Ok((
        row.try_get("epoch")?,
        i64::try_from(through).map_err(|_| Unavailable)?,
    ))
}
/// The current-epoch body of one character, observed no later than completeness.
pub(super) async fn body(
    db: &mut PgConnection,
    family: Family,
    character: uuid::Uuid,
    authority: &str,
    epoch: &str,
    through: i64,
) -> Result<Option<(serde_json::Value, String)>, Unavailable> {
    let row = sqlx::query("SELECT body,revision::text AS revision,observed FROM public_character_families_v2 WHERE family=$1 AND character_id=$2 AND source_authority=$3 AND epoch=$4::numeric")
        .bind(family.name())
        .bind(character)
        .bind(authority)
        .bind(epoch)
        .fetch_optional(&mut *db)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<i64, _>("observed")? > through {
        return Err(Unavailable);
    }
    Ok(Some((row.try_get("body")?, row.try_get("revision")?)))
}
