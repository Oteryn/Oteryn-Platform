//! Serialized aggregate commits; epoch/privacy fences survive generation rebuilds.
use super::wire::{self, Commit, Message, Page, Row};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row as SqlRow, Transaction};
use uuid::Uuid;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid snapshot")]
    Invalid,
    #[error("conflicting evidence")]
    Conflict,
    #[error("analytics unavailable")]
    Unavailable,
    #[error("rate limited")]
    Rate,
    #[error("storage unavailable")]
    Storage(#[from] sqlx::Error),
}
impl From<wire::Invalid> for Error {
    fn from(_: wire::Invalid) -> Self {
        Self::Invalid
    }
}
pub fn digest(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn family(f: wire::Family) -> Result<String, Error> {
    serde_json::to_value(f)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(Error::Invalid)
}
#[derive(Debug, Clone)]
pub struct Store {
    pub pool: PgPool,
    pub authority: String,
    pub privacy: String,
    pub rate: u32,
    pub max_staging_bytes: u64,
}
impl Store {
    pub async fn ingest(
        &self,
        message: Message,
        raw: &[u8],
        now: u64,
    ) -> Result<&'static str, Error> {
        let (authority, f, world, epoch, privacy) = match &message {
            Message::Page(p) => (
                &p.source_authority,
                p.family,
                &p.world_id,
                &p.analytics_epoch,
                &p.privacy_policy_revision,
            ),
            Message::Commit(c) => (
                &c.source_authority,
                c.family,
                &c.world_id,
                &c.analytics_epoch,
                &c.privacy_policy_revision,
            ),
            Message::Watermark(w) => (
                &w.source_authority,
                w.family,
                &w.world_id,
                &w.analytics_epoch,
                &w.privacy_policy_revision,
            ),
        };
        if authority != &self.authority || !(1..=6000).contains(&self.rate) {
            return Err(Error::Unavailable);
        }
        let policy_matches = privacy == &self.privacy;
        let authority = authority.clone();
        let epoch = epoch.clone();
        let privacy = privacy.clone();
        let family = family(f)?;
        let world = Uuid::parse_str(world).map_err(|_| Error::Invalid)?;
        let epoch_number = wire::uint(&epoch)?;
        let mut tx = self.pool.begin().await?;
        // Global serialization also enforces the four-exchange ceiling across processes.
        sqlx::query("SELECT pg_advisory_xact_lock(71140001)")
            .execute(&mut *tx)
            .await?;
        let fence=sqlx::query("SELECT epoch::text,privacy_floor,rate_window,rate_count FROM admin_analytics.fence WHERE authority=$1 FOR UPDATE")
            .bind(&authority).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
        let high = wire::uint(&fence.try_get::<String, _>("epoch")?)?;
        let floor_matches = fence.try_get::<String, _>("privacy_floor")? == self.privacy;
        let window = i64::try_from(now / 60).map_err(|_| Error::Invalid)?;
        let count = if fence.try_get::<i64, _>("rate_window")? == window {
            fence.try_get::<i32, _>("rate_count")?
        } else {
            0
        };
        if i64::from(count) >= i64::from(self.rate) {
            sqlx::query("UPDATE admin_analytics.fence SET epoch=GREATEST(epoch,$2::text::numeric),active_generation=CASE WHEN epoch<$2::text::numeric THEN NULL ELSE active_generation END WHERE authority=$1").bind(&authority).bind(&epoch).execute(&mut *tx).await?;
            tx.commit().await?;
            return Err(Error::Rate);
        }
        sqlx::query("UPDATE admin_analytics.fence SET rate_window=$2,rate_count=$3,epoch=GREATEST(epoch,$4::text::numeric),active_generation=CASE WHEN epoch<$4::text::numeric THEN NULL ELSE active_generation END WHERE authority=$1")
            .bind(&authority).bind(window).bind(count+1).bind(&epoch).execute(&mut *tx).await?;
        if !policy_matches || !floor_matches {
            tx.commit().await?;
            return Err(Error::Unavailable);
        }
        if epoch_number < high {
            let conflict = self
                .remember_fenced(&mut tx, &family, world, &message, raw)
                .await?;
            tx.commit().await?;
            if conflict {
                eprintln!("admin analytics: conflicting fenced evidence");
                return Err(Error::Conflict);
            }
            return Ok("superseded");
        }
        // Payload purge never deletes monotonic receipts/fences or acknowledged visibility.
        sqlx::query("DELETE FROM admin_analytics.pages WHERE staged_at<now()-interval '24 hours'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM admin_analytics.rows WHERE bucket_end <= $1 - CASE WHEN family='world_activity_hourly' THEN 2592000 ELSE 34560000 END")
            .bind(i64::try_from(now).map_err(|_|Error::Invalid)?).execute(&mut *tx).await?;
        let worlds:i64=sqlx::query_scalar("SELECT count(DISTINCT world) FROM (SELECT world FROM admin_analytics.pages WHERE authority=$1 UNION SELECT world FROM admin_analytics.subjects WHERE authority=$1 UNION SELECT world FROM admin_analytics.watermarks WHERE authority=$1) s WHERE world<>$2")
            .bind(&authority).bind(world).fetch_one(&mut *tx).await?;
        if worlds >= 256 {
            tx.commit().await?;
            return Err(Error::Unavailable);
        }
        let result = match message {
            Message::Page(p) => self.page(&mut tx, &family, world, &p, raw, now).await,
            Message::Commit(c) => self.commit(&mut tx, &family, world, &c, raw, now).await,
            Message::Watermark(w) => {
                let old=sqlx::query("SELECT epoch::text,cut::text,observed::text,digest,conflict FROM admin_analytics.watermarks WHERE authority=$1 AND family=$2 AND world=$3 FOR UPDATE")
                    .bind(&authority).bind(&family).bind(world).fetch_optional(&mut *tx).await?;
                let incoming = (
                    epoch_number,
                    wire::uint(&w.complete_through)?,
                    wire::uint(&w.observed_at)?,
                );
                if let Some(old) = old {
                    let old_order = (
                        wire::uint(&old.try_get::<String, _>("epoch")?)?,
                        wire::uint(&old.try_get::<String, _>("cut")?)?,
                        wire::uint(&old.try_get::<String, _>("observed")?)?,
                    );
                    if incoming < old_order {
                        tx.commit().await?;
                        return Ok("superseded");
                    }
                    if incoming == old_order && old.try_get::<String, _>("digest")? != digest(raw) {
                        sqlx::query("UPDATE admin_analytics.watermarks SET conflict=true WHERE authority=$1 AND family=$2 AND world=$3").bind(&authority).bind(&family).bind(world).execute(&mut *tx).await?;
                        tx.commit().await?;
                        return Err(Error::Conflict);
                    }
                }
                sqlx::query("INSERT INTO admin_analytics.watermarks(authority,family,world,epoch,privacy,cut,observed,digest) VALUES($1,$2,$3,$4::text::numeric,$5,$6::text::numeric,$7::text::numeric,$8) ON CONFLICT(authority,family,world) DO UPDATE SET epoch=excluded.epoch,privacy=excluded.privacy,cut=excluded.cut,observed=excluded.observed,digest=excluded.digest,conflict=false")
                    .bind(&authority).bind(&family).bind(world).bind(&epoch).bind(&privacy).bind(&w.complete_through).bind(&w.observed_at).bind(digest(raw)).execute(&mut *tx).await?;
                Ok("accepted")
            }
        };
        let result = match result {
            Ok(result) => result,
            Err(Error::Unavailable) => {
                tx.commit().await?;
                return Err(Error::Unavailable);
            }
            Err(error) => return Err(error),
        };
        tx.commit().await?;
        if result == "conflict" {
            eprintln!("admin analytics: conflicting evidence; presentation fenced");
            Err(Error::Conflict)
        } else {
            Ok(result)
        }
    }
    async fn remember_fenced(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        family: &str,
        world: Uuid,
        message: &Message,
        raw: &[u8],
    ) -> Result<bool, Error> {
        match message {
            Message::Page(p) => {
                sqlx::query("INSERT INTO admin_analytics.page_receipts(authority,family,world,bucket_start,epoch,revision,page_index,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7,$8) ON CONFLICT DO NOTHING")
                    .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).bind(digest(raw)).execute(&mut **tx).await?;
                let first:String=sqlx::query_scalar("SELECT digest FROM admin_analytics.page_receipts WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric AND page_index=$7")
                    .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).fetch_one(&mut **tx).await?;
                Ok(first != digest(raw))
            }
            Message::Commit(c) => {
                sqlx::query("INSERT INTO admin_analytics.receipts(authority,family,world,bucket_start,epoch,revision,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7) ON CONFLICT DO NOTHING")
                    .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(digest(raw)).execute(&mut **tx).await?;
                let first:String=sqlx::query_scalar("SELECT digest FROM admin_analytics.receipts WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric")
                    .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).fetch_one(&mut **tx).await?;
                Ok(first != digest(raw))
            }
            Message::Watermark(_) => Ok(false),
        }
    }
    async fn page(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        family: &str,
        world: Uuid,
        p: &Page,
        raw: &[u8],
        now: u64,
    ) -> Result<&'static str, Error> {
        let receipt:Option<String>=sqlx::query_scalar("SELECT digest FROM admin_analytics.page_receipts WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric AND page_index=$7")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).fetch_optional(&mut **tx).await?;
        if receipt.is_some_and(|d| d != digest(raw)) {
            self.conflict(tx, family, world, &p.bucket_start).await?;
            return Ok("conflict");
        }
        let old:Option<(String,String)>=sqlx::query_as("SELECT epoch::text,revision::text FROM admin_analytics.subjects WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).fetch_optional(&mut **tx).await?;
        if let Some((e, r)) = old {
            let existing = (wire::uint(&e)?, wire::uint(&r)?);
            let incoming = (
                wire::uint(&p.analytics_epoch)?,
                wire::uint(&p.snapshot_revision)?,
            );
            if incoming < existing {
                sqlx::query("INSERT INTO admin_analytics.page_receipts(authority,family,world,bucket_start,epoch,revision,page_index,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7,$8) ON CONFLICT DO NOTHING")
                    .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).bind(digest(raw)).execute(&mut **tx).await?;
                return Ok("superseded");
            }
            if incoming == existing {
                return Ok("accepted");
            }
        }
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_analytics.pages WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric AND page_index=$7)")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).fetch_one(&mut **tx).await?;
        if exists {
            return Ok("accepted");
        }
        let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_analytics.pages p WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start<>$4::text::numeric AND NOT EXISTS(SELECT 1 FROM admin_analytics.receipts c WHERE c.authority=p.authority AND c.family=p.family AND c.world=p.world AND c.bucket_start=p.bucket_start AND c.epoch=p.epoch AND c.revision=p.revision))")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).fetch_one(&mut **tx).await?;
        if pending {
            return Err(Error::Unavailable);
        }
        if wire::uint(&p.bucket_end)?
            .checked_add(p.family.retention())
            .ok_or(Error::Invalid)?
            <= now
        {
            return Ok("superseded");
        }
        let used: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(octet_length(body)),0)::bigint FROM admin_analytics.pages",
        )
        .fetch_one(&mut **tx)
        .await?;
        if u64::try_from(used)
            .ok()
            .and_then(|n| n.checked_add(raw.len() as u64))
            .is_none_or(|n| n > self.max_staging_bytes)
        {
            return Err(Error::Unavailable);
        }
        // At most two staged revisions per subject; no eviction under capacity pressure.
        let staged:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT DISTINCT epoch,revision FROM admin_analytics.pages p WHERE NOT EXISTS(SELECT 1 FROM admin_analytics.receipts c WHERE c.authority=p.authority AND c.family=p.family AND c.world=p.world AND c.bucket_start=p.bucket_start AND c.epoch=p.epoch AND c.revision=p.revision) AND authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND (epoch,revision)<>($5::text::numeric,$6::text::numeric)) s")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).fetch_one(&mut **tx).await?;
        if staged >= 2 {
            return Err(Error::Unavailable);
        }
        let worlds:i64=sqlx::query_scalar("SELECT count(DISTINCT world) FROM (SELECT world FROM admin_analytics.pages WHERE authority=$1 UNION SELECT world FROM admin_analytics.subjects WHERE authority=$1) s WHERE world<>$2")
            .bind(&self.authority).bind(world).fetch_one(&mut **tx).await?;
        if worlds >= 256 {
            return Err(Error::Unavailable);
        }
        sqlx::query("INSERT INTO admin_analytics.pages(authority,family,world,bucket_start,epoch,revision,page_index,digest,body) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7,$8,$9)")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).bind(digest(raw)).bind(raw).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO admin_analytics.page_receipts(authority,family,world,bucket_start,epoch,revision,page_index,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7,$8) ON CONFLICT DO NOTHING")
            .bind(&self.authority).bind(family).bind(world).bind(&p.bucket_start).bind(&p.analytics_epoch).bind(&p.snapshot_revision).bind(wire::uint(&p.page_index)? as i32).bind(digest(raw)).execute(&mut **tx).await?;
        Ok("accepted")
    }
    async fn conflict(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        family: &str,
        world: Uuid,
        start: &str,
    ) -> Result<(), Error> {
        sqlx::query("UPDATE admin_analytics.subjects SET conflict=true WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric")
            .bind(&self.authority).bind(family).bind(world).bind(start).execute(&mut **tx).await?;
        // A conflict before the first commit must also fence a future generation.
        sqlx::query("UPDATE admin_analytics.fence SET active_generation=NULL WHERE authority=$1")
            .bind(&self.authority)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
    async fn commit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        family: &str,
        world: Uuid,
        c: &Commit,
        raw: &[u8],
        now: u64,
    ) -> Result<&'static str, Error> {
        let receipt:Option<String>=sqlx::query_scalar("SELECT digest FROM admin_analytics.receipts WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric")
            .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).fetch_optional(&mut **tx).await?;
        if let Some(receipt) = receipt {
            if receipt == digest(raw) {
                return Ok("accepted");
            }
            self.conflict(tx, family, world, &c.bucket_start).await?;
            return Ok("conflict");
        }
        let old:Option<(String,String)>=sqlx::query_as("SELECT epoch::text,revision::text FROM admin_analytics.subjects WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric FOR UPDATE")
            .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).fetch_optional(&mut **tx).await?;
        let order = (
            wire::uint(&c.analytics_epoch)?,
            wire::uint(&c.snapshot_revision)?,
        );
        if old.is_some_and(|(e, r)| {
            wire::uint(&e)
                .ok()
                .zip(wire::uint(&r).ok())
                .is_some_and(|old| old > order)
        }) {
            sqlx::query("INSERT INTO admin_analytics.receipts(authority,family,world,bucket_start,epoch,revision,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7)")
                .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(digest(raw)).execute(&mut **tx).await?;
            return Ok("superseded");
        }
        let mut rows: Vec<Row> = Vec::new();
        let mut bytes = 0usize;
        for (index, expected) in c.page_digests.iter().enumerate() {
            let body:Option<Vec<u8>>=sqlx::query_scalar("SELECT body FROM admin_analytics.pages WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric AND epoch=$5::text::numeric AND revision=$6::text::numeric AND page_index=$7")
                .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(index as i32).fetch_optional(&mut **tx).await?;
            let body = body.ok_or(Error::Unavailable)?;
            bytes = bytes.checked_add(body.len()).ok_or(Error::Invalid)?;
            if bytes > 8 * 1024 * 1024 || digest(&body) != *expected {
                return Err(Error::Invalid);
            }
            let Message::Page(p) = wire::parse("page", &body, now)? else {
                return Err(Error::Invalid);
            };
            if p.source_authority != c.source_authority
                || p.family != c.family
                || p.world_id != c.world_id
                || p.bucket_start != c.bucket_start
                || p.bucket_end != c.bucket_end
                || p.analytics_epoch != c.analytics_epoch
                || p.snapshot_revision != c.snapshot_revision
                || p.source_observed_at != c.source_observed_at
                || p.privacy_policy_revision != c.privacy_policy_revision
                || wire::uint(&p.page_count)? != c.page_digests.len() as u64
            {
                return Err(Error::Invalid);
            }
            rows.extend(p.rows);
        }
        if rows.len() as u64 != wire::uint(&c.row_count)? || rows.len() > 8192 {
            return Err(Error::Invalid);
        }
        if c.state == "replace" {
            wire::rows(c.family, &rows, true)?
        }
        let end = wire::uint(&c.bucket_end)?;
        if end
            .checked_add(c.family.retention())
            .ok_or(Error::Invalid)?
            <= now
        {
            return Ok("superseded");
        }
        let generation:Option<Uuid>=sqlx::query_scalar("SELECT id FROM admin_analytics.generations WHERE authority=$1 AND epoch=$2::text::numeric ORDER BY created_at DESC LIMIT 1")
            .bind(&self.authority).bind(&c.analytics_epoch).fetch_optional(&mut **tx).await?;
        let generation = match generation {
            Some(id) => id,
            None => {
                let id = Uuid::now_v7();
                sqlx::query("INSERT INTO admin_analytics.generations(id,authority,epoch) VALUES($1,$2,$3::text::numeric)").bind(id).bind(&self.authority).bind(&c.analytics_epoch).execute(&mut **tx).await?;
                id
            }
        };
        sqlx::query("DELETE FROM admin_analytics.rows WHERE authority=$1 AND family=$2 AND world=$3 AND bucket_start=$4::text::numeric")
            .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).execute(&mut **tx).await?;
        for row in rows {
            sqlx::query("INSERT INTO admin_analytics.rows(generation,authority,family,world,bucket_start,bucket_end,key,state,values,epoch,revision,privacy,observed) VALUES($1,$2,$3,$4,$5::text::numeric,$6,$7,$8,$9,$10::text::numeric,$11::text::numeric,$12,$13::text::numeric)")
                .bind(generation).bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(i64::try_from(end).map_err(|_|Error::Invalid)?).bind(row.key).bind(row.state).bind(serde_json::json!(row.values)).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(&c.privacy_policy_revision).bind(&c.source_observed_at).execute(&mut **tx).await?;
        }
        sqlx::query("INSERT INTO admin_analytics.subjects(authority,family,world,bucket_start,bucket_end,epoch,revision,privacy,observed,commit_digest,erased,generation) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8,$9::text::numeric,$10,$11,$12) ON CONFLICT(authority,family,world,bucket_start) DO UPDATE SET bucket_end=excluded.bucket_end,epoch=excluded.epoch,revision=excluded.revision,privacy=excluded.privacy,observed=excluded.observed,commit_digest=excluded.commit_digest,erased=excluded.erased,generation=excluded.generation,conflict=false")
            .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.bucket_end).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(&c.privacy_policy_revision).bind(&c.source_observed_at).bind(digest(raw)).bind(c.state=="erase").bind(generation).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO admin_analytics.receipts(authority,family,world,bucket_start,epoch,revision,digest) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7)")
            .bind(&self.authority).bind(family).bind(world).bind(&c.bucket_start).bind(&c.analytics_epoch).bind(&c.snapshot_revision).bind(digest(raw)).execute(&mut **tx).await?;
        Ok("accepted")
    }
}

/// Hourly maintenance is independent of traffic and never deletes durable fences.
pub async fn purge(pool: &PgPool, now: u64) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71140001)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM admin_analytics.pages WHERE staged_at<now()-interval '24 hours'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM admin_analytics.rows r USING admin_analytics.fence f WHERE f.authority=r.authority AND (r.epoch<f.epoch OR r.privacy<>f.privacy_floor OR r.bucket_end::numeric+CASE WHEN r.family='world_activity_hourly' THEN 2592000 ELSE 34560000 END<=$1::text::numeric)").bind(now.to_string()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
