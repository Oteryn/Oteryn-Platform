//! Production payload retention. Each dispatch is bounded independently per store;
//! replay digests, sequence floors and continuation evidence are never deleted here.
use sqlx::PgPool;

#[derive(Debug)]
pub struct Retention(pub PgPool);
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move { self.expire_batch().await.map_err(|e| e.to_string()) })
    }
}
impl Retention {
    pub async fn expire_batch(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.0.begin().await?;
        // All cutoffs derive from the database clock, never a request timestamp.
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *tx)
                .await?;
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
        tx.commit().await
    }
}
