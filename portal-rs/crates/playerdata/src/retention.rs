//! Bounded production retention and lifecycle erasure, preserving replay floors.
use crate::store::{Flags, Store, TrustedTime, UnavailableAuthority};
use sqlx::{PgPool, Row};
use std::sync::Arc;

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
        sqlx::query("UPDATE pd_g9_records SET canonical_bytes=NULL,record=NULL,purged_at=COALESCE(purged_at,$1) WHERE ctid IN (SELECT ctid FROM pd_g9_records WHERE expires_at<=$1 AND canonical_bytes IS NOT NULL LIMIT 512 FOR UPDATE SKIP LOCKED)").bind(now/1000).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_records SET payload=NULL WHERE ctid IN (SELECT ctid FROM pd_records WHERE payload IS NOT NULL AND expires_ms<=$1 ORDER BY expires_ms LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_streams SET snapshot=NULL,snapshot_at=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_streams WHERE snapshot IS NOT NULL AND snapshot_at<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now/1000-30*86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t76_records r WHERE canonical_bytes IS NOT NULL AND (occurred_at<=$1 OR EXISTS(SELECT 1 FROM pd_t76_streams s WHERE s.stream_key=r.stream_key AND s.staging_at<=$2)) LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now/1000-30*86400).bind(now/1000-86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_events SET canonical_bytes=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_events WHERE canonical_bytes IS NOT NULL AND occurred_at<=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .bind(now/1000-30*86400).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t78_records SET payload=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t78_records r WHERE payload IS NOT NULL AND ((family IN ('death_recap','pvp') AND occurred_at_ms<=$1) OR (family='skull' AND occurred_at_ms<=$2 AND EXISTS(SELECT 1 FROM pd_t78_records n WHERE n.source_authority=r.source_authority AND n.account_id=r.account_id AND n.projection_epoch=r.projection_epoch AND n.recovery_generation=r.recovery_generation AND n.family=r.family AND n.character_id=r.character_id AND n.world_id=r.world_id AND n.entity_key=r.entity_key AND n.source_revision>r.source_revision))) LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .bind(now-crate::deaths::HISTORY_MS as i64).bind(now-7*86400000_i64).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx = self.0.begin().await?;
        // Lock a bounded set of streams before advancing floors and removing
        // digests. Ingest takes the same stream row locks.
        sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_streams s WHERE EXISTS(SELECT 1 FROM pd_records r WHERE r.stream_key=s.stream_key AND r.expires_ms<=$1 AND r.payload IS NULL AND r.source_revision=(SELECT min(source_revision) FROM pd_records p WHERE p.stream_key=s.stream_key AND p.source_revision>s.retention_floor)) AND (NOT s.erased OR s.erased_ms<=$2) ORDER BY stream_key LIMIT 128 FOR UPDATE SKIP LOCKED) UPDATE pd_streams s SET retention_floor=GREATEST(s.retention_floor,COALESCE((SELECT min(r.source_revision)-1 FROM pd_records r WHERE r.stream_key=s.stream_key AND (r.payload IS NOT NULL OR r.expires_ms>$1)),s.stored_through)) FROM candidates c WHERE s.stream_key=c.stream_key AND (NOT s.erased OR s.erased_ms<=$2)")
            .bind(now-31*86400000_i64).bind(now-120*86400000_i64).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_records WHERE ctid IN (SELECT r.ctid FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE r.source_revision<=s.retention_floor LIMIT 512 FOR UPDATE OF r SKIP LOCKED)")
            .execute(&mut *tx).await?;
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
        // Erasure is idempotent. A crash before deleting the job repeats the purge;
        // new jobs remain visible even while ingestion is disabled.
        let mut tx = self.0.begin().await?;
        let jobs = sqlx::query(
            "SELECT id,account_id,character_id,closed_revision::text FROM pd_erasure_jobs ORDER BY id LIMIT 32 FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&self.0)
                .await?;
        for job in jobs {
            let account: String = job.try_get("account_id")?;
            let character: Option<String> = job.try_get("character_id")?;
            let time = TrustedTime {
                unix_ms: now as u64,
                uncertainty_ms: 0,
            };
            let result = if let Some(character) = character.as_deref() {
                store
                    .erase_character_cut(
                        &account,
                        character,
                        time,
                        job.try_get::<Option<String>, _>("closed_revision")?
                            .as_deref(),
                    )
                    .await
            } else {
                store.erase_account(&account, time).await
            };
            result.map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
            let account_id = uuid::Uuid::parse_str(&account)
                .map_err(|_| sqlx::Error::Protocol("invalid erasure account".into()))?;
            let character_id = character
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose()
                .map_err(|_| sqlx::Error::Protocol("invalid erasure character".into()))?;
            crate::items::store::erase_scope(&mut tx, account_id, character_id).await?;
            crate::deaths::storage::erase_scope(&mut tx, account_id, character_id).await?;
            sqlx::query("DELETE FROM pd_game_logins WHERE account_id=$1 AND ($2::uuid IS NULL OR character_id=$2)")
                .bind(account_id).bind(character_id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pd_erasure_jobs WHERE id=$1")
                .bind(job.try_get::<i64, _>("id")?)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
