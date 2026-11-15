//! Financial cleanup commits a bounded competitor set at a time. A selected
//! offer is fenced by Matched before cleanup can be resumed by another request.
use crate::{Error, Offer};
use sqlx::PgConnection;
use uuid::Uuid;
pub(crate) const OFFER_BATCH: usize = 64;
pub(crate) async fn admit_offer(db: &mut PgConnection, listing: Uuid) -> Result<(), Error> {
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND state='pending' LIMIT 128) bounded").bind(listing).fetch_one(db).await?;
    if count >= 128 {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn offers(
    db: &mut PgConnection,
    listing: Uuid,
    exclude: Option<Uuid>,
) -> Result<Vec<Offer>, Error> {
    Ok(sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND state='pending' AND ($2::uuid IS NULL OR id<>$2) ORDER BY id LIMIT 64 FOR UPDATE")
        .bind(listing).bind(exclude).fetch_all(db).await?)
}
pub(crate) async fn finish(
    db: &mut PgConnection,
    batch: &[Offer],
    state: &str,
) -> Result<(), Error> {
    let ids: Vec<_> = batch.iter().map(|offer| offer.id).collect();
    sqlx::query("UPDATE bazaar_offers SET state=$2,updated_at=clock_timestamp() WHERE id=ANY($1) AND state='pending'").bind(ids).bind(state).execute(db).await?;
    Ok(())
}
