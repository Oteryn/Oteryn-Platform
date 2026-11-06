//! Approved item views come only from current policy plus verified escrow evidence.
use super::game_wire::{id, text};
use super::{Authority, Error};
use crate::{
    Listing,
    items::{CreateItem, ItemAuthority, Publication},
};
use oteryn_internal_api::canonical::Value;
use sqlx::PgConnection;
use uuid::Uuid;
fn domain(e: Error) -> crate::Error {
    match e {
        Error::Database(e) => crate::Error::Database(e),
        _ => crate::Error::Denied,
    }
}
impl ItemAuthority for Authority {
    async fn cancellation_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<bool, crate::Error> {
        crate::Evidence::cancellation_ready(self, db, listing).await
    }

    async fn preflight(
        &self,
        db: &mut PgConnection,
        _seller: Uuid,
        input: &CreateItem,
    ) -> Result<bool, crate::Error> {
        if !self.fresh(db).await.map_err(domain)? {
            return Ok(false);
        }
        let Some(policy) = self
            .projection(db, "POLICY", input.world_id)
            .await
            .map_err(domain)?
        else {
            return Ok(false);
        };
        let Some(Value::Array(definitions)) = policy.get("definitions") else {
            return Ok(false);
        };
        // Advisory policy only; this cannot certify the supplied instance or custody.
        Ok(definitions.iter().any(|d|d.get("tradeable")==Some(&Value::Bool(true)) && d.get("definition").is_some_and(|v|text(v,"family").ok()==Some("item") && text(v,"production_key").ok()==Some(input.definition_key.as_str()) && text(v,"revision_ref").ok()==Some(input.definition_revision.as_str())) && matches!(d.get("max_quantity"),Some(Value::Unsigned(max)) if *max>=u64::from(input.quantity))))
    }
    async fn publication(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<Option<Publication>, crate::Error> {
        let Some((_, _, receipt)) = self
            .custody(db, listing, "ESCROWED")
            .await
            .map_err(domain)?
        else {
            return Ok(None);
        };
        let d = receipt
            .value
            .get("result_data")
            .ok_or(crate::Error::Denied)?;
        let item = d.get("item").ok_or(crate::Error::Denied)?;
        let def = item.get("definition").ok_or(crate::Error::Denied)?;
        let Some(Value::Array(attributes)) = item.get("attributes") else {
            return Ok(None);
        };
        let attributes = attributes
            .iter()
            .map(|a| {
                Ok((
                    text(a, "key").map_err(domain)?.to_owned(),
                    text(a, "value").map_err(domain)?.to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, crate::Error>>()?;
        Ok(Some(Publication {
            // Stable production key is a label, never an invented localized name.
            name: text(def, "production_key").map_err(domain)?.to_owned(),
            attributes,
            escrow_id: Some(id(text(d, "escrow_id").map_err(domain)?).map_err(domain)?),
            escrow_receipt_digest: Some(receipt.digest()),
            projection_epoch: self.fence.projection_epoch.to_string(),
            recovery_generation: self.fence.recovery_generation.to_string(),
        }))
    }
    async fn current(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<bool, crate::Error> {
        let returning: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_item_outbox WHERE listing_id=$1 AND step='return')",
        )
        .bind(listing.id)
        .fetch_one(&mut *db)
        .await?;
        if returning {
            return Ok(false);
        }
        Ok(self
            .custody(db, listing, "ESCROWED")
            .await
            .map_err(domain)?
            .is_some())
    }
    async fn offer_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        offer: Uuid,
        seller: Uuid,
    ) -> Result<bool, crate::Error> {
        if listing.form == crate::Form::WantToBuy || seller != listing.owner {
            return Ok(false);
        }
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE id=$1 AND listing_id=$2 AND state='pending')").bind(offer).bind(listing.id).fetch_one(&mut *db).await?;
        Ok(valid && self.current(db, listing).await?)
    }
}
// Keep protocol scalar helpers in scope only where required by future supported
// buy-order publication. Unknown subject bindings remain unavailable.
