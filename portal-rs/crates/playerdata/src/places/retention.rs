//! Privacy cleanup runs even when publication and ingest are disabled.
use sqlx::PgPool;
#[derive(Debug, Clone)]
pub struct PlacesRetention {
    pub pool: PgPool,
}
impl PlacesRetention {
    pub async fn purge(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(81005230117)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE pd_places_preferences SET participating=false,privacy_revision=privacy_revision+1,updated_at=now()
            WHERE account_id IN (SELECT p.account_id FROM pd_places_preferences p JOIN accounts a ON a.id=p.account_id
                WHERE a.status='closed' AND p.participating ORDER BY p.account_id LIMIT 512)")
            .execute(&mut *tx).await?;
        // Fence precisely the removed nonparticipating sessions in the same statement.
        sqlx::query("WITH removed AS (
            DELETE FROM pd_places_sessions WHERE ctid IN (
                SELECT s.ctid FROM pd_places_sessions s JOIN pd_places_preferences p USING(account_id)
                WHERE NOT p.participating OR s.privacy_revision<>p.privacy_revision OR s.ended_at<=now()-interval '90 days'
                ORDER BY s.ended_at LIMIT 512 FOR UPDATE OF s SKIP LOCKED)
            RETURNING source_authority,session_id,ended_at,account_id)
            INSERT INTO pd_places_replay_fences SELECT r.source_authority,r.session_id,r.ended_at+interval '120 days'
            FROM removed r JOIN pd_places_preferences p USING(account_id) WHERE NOT p.participating ON CONFLICT DO NOTHING")
            .execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_places_replay_fences WHERE ctid IN (SELECT ctid FROM pd_places_replay_fences WHERE expires_at<=now() ORDER BY expires_at LIMIT 512 FOR UPDATE SKIP LOCKED)")
            .execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_places_publication_days WHERE utc_day < (now() AT TIME ZONE 'UTC')::date - 90")
            .execute(&mut *tx).await?;
        tx.commit().await
    }
}
impl oteryn_db::outbox::Cleanup for PlacesRetention {
    fn clean(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async {
            self.purge()
                .await
                .map_err(|_| "hunting-place retention unavailable".to_owned())
        })
    }
}
