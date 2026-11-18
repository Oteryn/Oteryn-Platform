//! Reconcile immutable staff approvals under the same listing and issuer locks as trading.
use super::*;
use crate::{Evidence, Reservations, State};
use sqlx::{Postgres, Transaction};

const APPROVED: &str = "FROM bazaar_moderation_outbox o
    JOIN bazaar_invalidation_requests r ON r.id=o.id AND r.listing_id=o.listing_id
    JOIN bazaar_moderation_holds h ON h.listing_id=o.listing_id AND h.revision=r.hold_revision
    WHERE o.kind='reconcile_invalidation' AND r.state='approved'";

pub(super) async fn approved(db: &mut PgConnection, listing: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT EXISTS(SELECT 1 {APPROVED} AND o.listing_id=$1)"
    )))
    .bind(listing)
    .fetch_one(db)
    .await?)
}

impl Runtime {
    pub(super) async fn tick_invalidations(&self) -> Result<(), Error> {
        let queue = "moderation-invalidations";
        let cursor = crate::scheduling::cursor(&self.pool, queue).await?;
        let batch: Vec<(Uuid, Uuid)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT o.id,o.listing_id {APPROVED}
             ORDER BY (o.id <= $1),o.id LIMIT 24"
        )))
        .bind(cursor)
        .fetch_all(&self.pool)
        .await?;
        for (request, listing) in batch {
            crate::scheduling::advance(&self.pool, queue, request).await?;
            match self.invalidate(request, listing).await {
                Ok(()) => {}
                Err(Error::Database(e)) => return Err(Error::Database(e)),
                Err(_) => {} // Await current-generation evidence, then retry on a later tick.
            }
        }
        Ok(())
    }

    async fn invalidate(&self, request: Uuid, id: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing: Listing =
            sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if !matches!(listing.state, State::Draft | State::Active) || !approved(&mut tx, id).await? {
            return Ok(());
        }
        let settling: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_settlements WHERE listing_id=$1)
                 OR EXISTS(SELECT 1 FROM bazaar_item_sales WHERE listing_id=$1)
                 OR EXISTS(SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1)
                 OR EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE listing_id=$1
                           AND command IN ('DeliverBazaarItem','PayoutBazaarGold'))
                 OR EXISTS(SELECT 1 FROM bazaar_all_transfers WHERE listing_id=$1
                           AND purpose='BAZAAR_SETTLEMENT')",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if settling {
            return Ok(());
        }

        let coin_holds: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT h.account_id,h.id FROM bazaar_coin_holds h
             WHERE h.amount>0 AND (h.id=$1 OR (h.id=$2 AND h.account_id=$3) OR EXISTS(
                SELECT 1 FROM bazaar_offers o WHERE o.listing_id=$1 AND o.id=h.id AND o.account=h.account_id))
             ORDER BY h.account_id,h.id",
        )
        .bind(id)
        .bind(listing.reservation)
        .bind(listing.leader)
        .fetch_all(&mut *tx)
        .await?;
        let gold_buyers: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT (intent::jsonb->>'buyer_account_id')::uuid FROM bazaar_game_outbox
             WHERE listing_id=$1 AND command='HoldBazaarGold'",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let mut coin_accounts: Vec<_> = coin_holds.iter().map(|&(account, _)| account).collect();
        coin_accounts.dedup();
        let mut parties = coin_accounts.clone();
        parties.extend(gold_buyers.iter().copied());
        parties.push(listing.owner);
        crate::locking::accounts(&mut tx, &parties).await?;
        self.authority().lock(&mut tx).await?;

        // Let the ordinary gold worker finish the existing hold/release chain.
        // The moderation hold prevents it from accepting new funding for trade.
        let funding_in_flight: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_bids WHERE listing_id=$1
             AND phase IN ('pending','releasing'))",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if funding_in_flight {
            return Ok(());
        }
        let mut gold_releases = Vec::new();
        for buyer in gold_buyers {
            if let Some(hold) =
                super::gold::current_hold(self.authority(), &mut tx, id, buyer, None).await?
                && hold.amount > 0
            {
                gold_releases.push((buyer, hold));
            }
        }
        if !gold_releases.is_empty() {
            for (buyer, hold) in gold_releases {
                // Each hold lifecycle has its own retry chain.
                self.gold_release(&mut tx, &listing, buyer, hold.amount, "CANCELLED", hold.id)
                    .await?;
            }
            tx.commit().await?;
            return Ok(());
        }
        if !self.invalidation_custody_ready(&mut tx, &listing).await? {
            tx.commit().await?;
            return Ok(());
        }

        let funds = CoinReservations {
            authority: self.authority().clone(),
        };
        if !coin_accounts.is_empty() {
            funds
                .lock_accounts(&mut tx, &coin_accounts)
                .await
                .map_err(core_error)?;
            for (account, reservation) in coin_holds {
                funds
                    .release(&mut tx, reservation, account)
                    .await
                    .map_err(core_error)?;
            }
        }
        // Retire the exact approved hold only in the terminal transaction. The
        // immutable request and outbox remain available for audit and replay.
        sqlx::query(
            "DELETE FROM bazaar_moderation_holds h USING bazaar_invalidation_requests r
             WHERE r.id=$1 AND h.listing_id=r.listing_id AND h.revision=r.hold_revision",
        )
        .bind(request)
        .execute(&mut *tx)
        .await?;
        crate::lifecycle::write_listing_with(
            &mut tx,
            id,
            crate::lifecycle::Event::Listing(State::Cancelled),
            crate::lifecycle::ListingChange::Invalidate,
        )
        .await
        .map_err(core_error)?;
        sqlx::query("UPDATE bazaar_offers SET state='expired',updated_at=now() WHERE listing_id=$1 AND state='pending'")
            .bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn invalidation_custody_ready(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        listing: &Listing,
    ) -> Result<bool, Error> {
        if listing.kind == Kind::Character
            && self
                .latest(tx, listing.id, Command::Unlock)
                .await?
                .is_some_and(|(_, receipt)| receipt.committed)
        {
            return Ok(true);
        }
        if self
            .authority()
            .cancellation_ready(tx, listing)
            .await
            .map_err(core_error)?
        {
            if listing.kind == Kind::Character {
                self.unlock(tx, listing).await?;
                return Ok(false);
            }
            return Ok(true);
        }

        // Local-only listings have no dispatched custody obligation. An
        // attempted deposit is harmless only with current terminal no-effect proof.
        let deposits: Vec<Uuid> = sqlx::query_scalar(
            "SELECT o.operation_id FROM bazaar_game_outbox o
             WHERE o.listing_id=$1 AND o.command IN ('EscrowBazaarItem','LockBazaarCharacter')
             AND (EXISTS(SELECT 1 FROM bazaar_authority_dispatch d WHERE d.operation_id=o.operation_id)
               OR EXISTS(SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=o.operation_id))",
        )
        .bind(listing.id)
        .fetch_all(&mut **tx)
        .await?;
        for operation in deposits {
            match self.authority().retained(tx, operation).await? {
                Some((_, receipt)) if receipt.committed => {
                    if receipt.command == Command::Lock
                        && super::characters::never_transferred(tx, listing.id).await?
                    {
                        self.unlock(tx, listing).await?;
                    } else if receipt.command == Command::Lock
                        && let Some((bind, decision)) =
                            self.latest(tx, listing.id, Command::Bind).await?
                        && decision.committed
                        && text(&bind.value, "purpose")? == "BAZAAR_LISTING_ESCROW"
                        && sqlx::query_scalar::<_, bool>(
                            "SELECT EXISTS(SELECT 1 FROM bazaar_current_transfers t
                             WHERE t.operation_id=$1 AND t.listing_id=$2
                               AND NOT EXISTS(SELECT 1 FROM character_commands c WHERE c.id=t.operation_id))",
                        )
                        .bind(id(text(&bind.value,"transfer_operation_id")?)?).bind(listing.id)
                        .fetch_one(&mut **tx).await?
                    {
                        self.transfer(tx, listing, "BAZAAR_LISTING_ESCROW", None)
                            .await?;
                    } else {
                        self.stage_return(tx, listing).await?;
                    }
                    return Ok(false);
                }
                Some(_) => {}
                None => return Ok(false),
            }
        }
        let other_custody: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_all_transfers WHERE listing_id=$1)
                 OR EXISTS(SELECT 1 FROM bazaar_item_details WHERE id=$1 AND escrow_id IS NOT NULL)",
        )
        .bind(listing.id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(!other_custody)
    }
}
