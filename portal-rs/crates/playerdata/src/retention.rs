//! Bounded production retention and lifecycle erasure, preserving replay floors.
use crate::store::{Flags, Store, TrustedTime, UnavailableAuthority};
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub(crate) async fn expire_records(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    now: i64,
    limit: Option<i64>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "WITH expired AS (SELECT r.ctid, s.progress FROM pd_records r JOIN pd_streams s USING(stream_key)
            WHERE r.expires_ms <= $1 AND (r.payload IS NOT NULL OR
                (s.progress AND (r.occurred_ms<>0 OR r.coverage_ms<>0 OR r.entity_key<>'' OR r.character_id IS NOT NULL)))
            ORDER BY r.expires_ms LIMIT $2 FOR UPDATE OF r SKIP LOCKED)
         UPDATE pd_records r SET payload=NULL,
            occurred_ms=CASE WHEN e.progress THEN 0 ELSE r.occurred_ms END,
            coverage_ms=CASE WHEN e.progress THEN 0 ELSE r.coverage_ms END,
            entity_key=CASE WHEN e.progress THEN '' ELSE r.entity_key END,
            character_id=CASE WHEN e.progress THEN NULL ELSE r.character_id END
         FROM expired e WHERE r.ctid=e.ctid",
    )
    .bind(now)
    .bind(limit)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Debug)]
pub struct Retention(pub PgPool);
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move { self.expire_batch().await.map_err(|e| e.to_string()) })
    }
}
impl Retention {
    pub async fn expire_batch(&self) -> Result<(), sqlx::Error> {
        self.erase_lifecycle_batch().await?;
        let mut tx = self.0.begin().await?;
        // All cutoffs derive from the database clock, never a request timestamp.
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query("UPDATE pd_g9_records SET canonical_bytes=NULL,record=NULL,expires_at=0,purged_at=COALESCE(purged_at,$1) WHERE ctid IN (SELECT d.ctid FROM pd_g9_records d WHERE d.expires_at<=$1 AND (d.canonical_bytes IS NOT NULL OR (d.purged_at IS NOT NULL AND d.expires_at<>0))
            AND (d.record_kind<>'RARE_DROP' OR NOT EXISTS (SELECT 1 FROM pd_g9_records s
                WHERE s.source_authority=d.source_authority AND s.segment_id=d.segment_id
                AND s.account_id=d.account_id AND s.stream_generation=d.stream_generation
                AND s.record_kind='HUNT_SEGMENT' AND s.expires_at>$1))
            LIMIT 512 FOR UPDATE OF d SKIP LOCKED)").bind(now/1000).execute(&mut *tx).await?;
        expire_records(&mut tx, now, Some(512)).await?;
        sqlx::query("UPDATE pd_t76_streams SET snapshot=NULL,snapshot_at=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_streams WHERE snapshot IS NOT NULL AND snapshot_at<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now/1000-30*86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t76_records r WHERE canonical_bytes IS NOT NULL AND (occurred_at<=$1 OR EXISTS(SELECT 1 FROM pd_t76_streams s WHERE s.stream_key=r.stream_key AND s.staging_at<=$2)) LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now/1000-30*86400).bind(now/1000-86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_events SET canonical_bytes=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_events WHERE canonical_bytes IS NOT NULL AND occurred_at<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now/1000-30*86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t78_records SET payload=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t78_records r WHERE payload IS NOT NULL AND ((family IN ('death_recap','pvp') AND occurred_at_ms<=$1) OR (family='skull' AND occurred_at_ms<=$2 AND EXISTS(SELECT 1 FROM pd_t78_records n WHERE n.source_authority=r.source_authority AND n.account_id=r.account_id AND n.projection_epoch=r.projection_epoch AND n.recovery_generation=r.recovery_generation AND n.family=r.family AND n.character_id=r.character_id AND n.world_id=r.world_id AND n.entity_key=r.entity_key AND n.source_revision>r.source_revision AND EXISTS(SELECT 1 FROM pd_t78_streams published WHERE published.source_authority=n.source_authority AND published.account_id=n.account_id AND published.projection_epoch=n.projection_epoch AND published.recovery_generation=n.recovery_generation AND published.family=n.family AND published.baseline_complete AND n.source_revision<=published.complete_through)))) LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now-crate::deaths::HISTORY_MS as i64).bind(now-7*86400000_i64).execute(&mut *tx).await?;
        // Floors survive digest expiry. If expiry crosses retained evidence or
        // a missing prefix, close the generation instead of claiming completeness.
        sqlx::query("WITH candidates AS (SELECT source_authority,account_id,stream_generation FROM pd_g9_streams s WHERE EXISTS(SELECT 1 FROM pd_g9_records r WHERE r.source_authority=s.source_authority AND r.account_id=s.account_id AND r.stream_generation=s.stream_generation AND r.purged_at<=$1) ORDER BY source_authority,account_id,stream_generation LIMIT 128 FOR UPDATE SKIP LOCKED), expiry AS (SELECT c.source_authority,c.account_id,c.stream_generation,max(r.source_revision) AS through FROM candidates c JOIN pd_g9_records r USING(source_authority,account_id,stream_generation) WHERE r.purged_at<=$1 GROUP BY c.source_authority,c.account_id,c.stream_generation), cuts AS (SELECT e.*,e.through>s.applied_through OR EXISTS(SELECT 1 FROM pd_g9_records r WHERE r.source_authority=e.source_authority AND r.account_id=e.account_id AND r.stream_generation=e.stream_generation AND r.source_revision<=e.through AND (r.purged_at IS NULL OR r.purged_at>$1)) AS close_generation FROM expiry e JOIN pd_g9_streams s USING(source_authority,account_id,stream_generation)) UPDATE pd_g9_streams s SET retention_floor=GREATEST(s.retention_floor,c.through),blocked=s.blocked OR c.close_generation,admitted=s.admitted AND NOT c.close_generation FROM cuts c WHERE s.source_authority=c.source_authority AND s.account_id=c.account_id AND s.stream_generation=c.stream_generation")
            .bind(now/1000-120*86400).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_g9_records WHERE ctid IN (SELECT r.ctid FROM pd_g9_records r JOIN pd_g9_streams s USING(source_authority,account_id,stream_generation) WHERE r.purged_at<=$1 AND r.source_revision<=s.retention_floor LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now/1000-120*86400).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_t78_records WHERE ctid IN (SELECT ctid FROM pd_t78_records WHERE erased_at_ms<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now-120*86400000_i64).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx = self.0.begin().await?;
        // Lock a bounded set of streams before advancing floors and removing
        // digests. Ingest takes the same stream row locks.
        sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_streams s WHERE s.progress=false AND EXISTS(SELECT 1 FROM pd_records r WHERE r.stream_key=s.stream_key AND r.expires_ms<=$1 AND r.payload IS NULL AND r.source_revision=(SELECT min(source_revision) FROM pd_records p WHERE p.stream_key=s.stream_key AND p.source_revision>s.retention_floor)) AND (NOT s.erased OR s.erased_ms<=$2) ORDER BY stream_key LIMIT 128 FOR UPDATE SKIP LOCKED) UPDATE pd_streams s SET retention_floor=GREATEST(s.retention_floor,COALESCE((SELECT min(r.source_revision)-1 FROM pd_records r WHERE r.stream_key=s.stream_key AND (r.payload IS NOT NULL OR r.expires_ms>$1)),s.stored_through)) FROM candidates c WHERE s.stream_key=c.stream_key AND s.progress=false AND (NOT s.erased OR s.erased_ms<=$2)")
            .bind(now-31*86400000_i64).bind(now-120*86400000_i64).execute(&mut *tx).await?;
        sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_streams s WHERE s.progress=true AND s.erased=true AND s.erased_ms<=$1 AND s.retention_floor<s.stored_through ORDER BY stream_key LIMIT 128 FOR UPDATE SKIP LOCKED) UPDATE pd_streams s SET retention_floor=GREATEST(s.retention_floor,s.stored_through) FROM candidates c WHERE s.stream_key=c.stream_key AND s.progress=true AND s.erased=true AND s.erased_ms<=$1 AND s.retention_floor<s.stored_through")
            .bind(now-120*86400000_i64).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_records WHERE ctid IN (SELECT r.ctid FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE r.source_revision<=s.retention_floor LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .execute(&mut *tx).await?;
        // Character erasure clears scope_key and sets expires_ms to the purge
        // time plus 89 days. Keep the shared account cursor and its live rows.
        sqlx::query(
            "DELETE FROM pd_records WHERE ctid IN (
            SELECT r.ctid FROM pd_records r JOIN pd_streams s USING(stream_key)
            WHERE s.progress AND NOT s.erased AND r.scope_key='' AND r.payload IS NULL
            AND r.expires_ms<=$1 LIMIT 512 FOR UPDATE OF r SKIP LOCKED)",
        )
        .bind(now - 31 * 86400000_i64)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM pd_deleted_scopes WHERE ctid IN (SELECT ctid FROM pd_deleted_scopes WHERE expires_ms<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_conflicts WHERE ctid IN (SELECT ctid FROM pd_conflicts WHERE expires_ms<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now).execute(&mut *tx).await?;
        // T76 uses seconds. Advance only a contiguous expired prefix; an old
        // out-of-order occurrence must never move the floor past retained data.
        sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_t76_streams s WHERE EXISTS(SELECT 1 FROM pd_t76_records r WHERE r.stream_key=s.stream_key AND r.occurred_at<=$1 AND r.source_revision=(SELECT min(source_revision) FROM pd_t76_records p WHERE p.stream_key=s.stream_key AND p.source_revision>s.retention_floor)) AND (NOT s.fenced OR extract(epoch FROM s.updated_at)::bigint<=$2) ORDER BY stream_key LIMIT 128 FOR UPDATE SKIP LOCKED) UPDATE pd_t76_streams s SET retention_floor=GREATEST(s.retention_floor,COALESCE((SELECT min(source_revision)-1 FROM pd_t76_records r WHERE r.stream_key=s.stream_key AND r.occurred_at>$1),s.cursor)) FROM candidates c WHERE s.stream_key=c.stream_key AND (NOT s.fenced OR extract(epoch FROM s.updated_at)::bigint<=$2)")
            .bind(now/1000-61*86400).bind(now/1000-120*86400).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_t76_records WHERE ctid IN (SELECT r.ctid FROM pd_t76_records r JOIN pd_t76_streams s USING(stream_key) WHERE r.source_revision<=s.retention_floor LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_t76_events WHERE ctid IN (SELECT r.ctid FROM pd_t76_events r JOIN pd_t76_streams s USING(stream_key) WHERE r.occurred_at<=$1 AND (NOT s.fenced OR extract(epoch FROM s.updated_at)::bigint<=$2) LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now/1000-61*86400).bind(now/1000-120*86400).execute(&mut *tx).await?;
        tx.commit().await
    }
}

impl Retention {
    async fn erase_lifecycle_batch(&self) -> Result<(), sqlx::Error> {
        let store = Store::new(
            self.0.clone(),
            Arc::new(UnavailableAuthority),
            Flags::default(),
        );
        // Visit each candidate once. Commit a job's view cleanup before claiming
        // the next job, even when this pass leaves more records for a later run.
        let candidates: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM pd_erasure_jobs ORDER BY id LIMIT 32")
                .fetch_all(&self.0)
                .await?;
        for id in candidates {
            let mut tx = self.0.begin().await?;
            let Some(job) = sqlx::query(
                "SELECT account_id,character_id,closed_revision::text,
                    floor(extract(epoch FROM created_at)*1000)::bigint AS job_ms,
                    floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms
                FROM pd_erasure_jobs WHERE id=$1 FOR UPDATE SKIP LOCKED",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            else {
                continue;
            };
            let account: String = job.try_get("account_id")?;
            let character: Option<String> = job.try_get("character_id")?;
            let time = TrustedTime {
                unix_ms: job.try_get::<i64, _>("now_ms")? as u64,
                uncertainty_ms: 0,
            };
            let batch_size = 512_i64;
            let limit = Some(batch_size);
            let requested_ms = job.try_get("job_ms")?;
            let closed_revision: Option<String> = job.try_get("closed_revision")?;
            let journal_done = if let Some(character) = character.as_deref() {
                store
                    .erase_character_cut(
                        &account,
                        character,
                        time,
                        closed_revision.as_deref(),
                        limit,
                        requested_ms,
                    )
                    .await
            } else {
                store
                    .erase_account_batch(&account, time, limit, requested_ms)
                    .await
            }
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
            let account_id = uuid::Uuid::parse_str(&account)
                .map_err(|_| sqlx::Error::Protocol("invalid erasure account".into()))?;
            let character_id = character
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose()
                .map_err(|_| sqlx::Error::Protocol("invalid erasure character".into()))?;
            let items_done = crate::items::store::erase_scope(
                &mut tx,
                account_id,
                character_id,
                limit,
                Some(requested_ms),
            )
            .await?;
            let deaths_done = crate::deaths::storage::erase_scope(
                &mut tx,
                account_id,
                character_id,
                limit,
                requested_ms,
                closed_revision.as_deref(),
            )
            .await?;
            let removed = sqlx::query("DELETE FROM pd_game_logins WHERE ctid IN (SELECT ctid FROM pd_game_logins WHERE account_id=$1 AND ($2::uuid IS NULL OR character_id=$2) AND ($4::text IS NULL OR character_revision IS NULL OR character_revision <= $4::text::numeric) LIMIT $3 FOR UPDATE)")
                .bind(account_id).bind(character_id).bind(limit).bind(closed_revision.as_deref()).execute(&mut *tx).await?.rows_affected();
            let notes_done = if character_id.is_none() {
                sqlx::query(
                    "DELETE FROM pd_weekly_notes WHERE id IN (
                       SELECT id FROM pd_weekly_notes WHERE account_id=$1
                       ORDER BY week_start LIMIT $2 FOR UPDATE)",
                )
                .bind(account_id)
                .bind(batch_size)
                .execute(&mut *tx)
                .await?
                .rows_affected()
                    < batch_size as u64
            } else {
                true
            };
            if journal_done
                && items_done
                && deaths_done
                && notes_done
                && removed < batch_size as u64
            {
                sqlx::query("DELETE FROM pd_erasure_jobs WHERE id=$1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
        }
        Ok(())
    }
}
