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
        sqlx::query("UPDATE pd_places_preferences p SET participating=false,privacy_revision=p.privacy_revision+1,updated_at=now() FROM accounts a WHERE a.id=p.account_id AND a.status='closed' AND p.participating").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO pd_places_replay_fences SELECT s.source_authority,s.session_id,s.ended_at+interval '120 days' FROM pd_places_sessions s JOIN pd_places_preferences p USING(account_id) WHERE NOT p.participating ON CONFLICT DO NOTHING").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_places_sessions s USING pd_places_preferences p WHERE s.account_id=p.account_id AND (NOT p.participating OR s.privacy_revision<>p.privacy_revision OR s.ended_at<=now()-interval '90 days')").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_places_replay_fences WHERE expires_at<=now()")
            .execute(&mut *tx)
            .await?;
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
