use sqlx::{PgConnection, PgPool};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::*;

/// Transactional API for trusted composing crates. Actor IDs must come from authentication.
/// Read models include private proxy maxima and must not be exposed directly to clients.
#[derive(Debug, Clone)]
pub struct Bazaar<A, R, E> {
    pool: PgPool,
    accounts: A,
    reservations: R,
    evidence: E,
    flags: Flags,
}

impl<A: Accounts, R: Reservations, E: Evidence> Bazaar<A, R, E> {
    pub fn new(pool: PgPool, accounts: A, reservations: R, evidence: E, flags: Flags) -> Self {
        Self {
            pool,
            accounts,
            reservations,
            evidence,
            flags,
        }
    }

    fn enabled(&self) -> Result<(), Error> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }

    fn currency(&self, currency: Currency) -> Result<(), Error> {
        self.enabled()?;
        if currency == Currency::Gold && !self.flags.gold_enabled {
            return Err(Error::Disabled);
        }
        Ok(())
    }

    async fn authorize(
        &self,
        db: &mut PgConnection,
        account: Uuid,
        listing: bool,
    ) -> Result<(), Error> {
        if account.is_nil() {
            return Err(Error::Denied);
        }
        let state = self
            .accounts
            .state(db, account)
            .await?
            .ok_or(Error::Denied)?;
        if if listing {
            state.allows_listing()
        } else {
            state.allows_trade()
        } {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }

    async fn authorize_many(
        &self,
        db: &mut PgConnection,
        mut actors: Vec<(Uuid, bool)>,
    ) -> Result<(), Error> {
        self.accounts
            .prepare(db, &actors.iter().map(|(id, _)| *id).collect::<Vec<_>>())
            .await?;
        actors.sort_unstable_by_key(|(id, _)| *id);
        for (id, listing) in actors {
            self.authorize(db, id, listing).await?;
        }
        Ok(())
    }

    async fn funds_lock(
        &self,
        db: &mut PgConnection,
        mut accounts: Vec<Uuid>,
    ) -> Result<(), Error> {
        accounts.sort_unstable();
        accounts.dedup();
        self.reservations.lock_accounts(db, &accounts).await
    }

    pub async fn get(&self, id: Uuid) -> Result<Listing, Error> {
        self.enabled()?;
        sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::NotFound)
    }

    /// Advertisements always run for fourteen days; their supplied end time is ignored.
    pub async fn create(&self, mut input: NewListing) -> Result<Listing, Error> {
        self.currency(input.currency)?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        self.authorize(&mut tx, input.owner, true).await?;
        let now = clock(&mut tx).await?;
        if input.form != Form::Auction {
            input.ends_at = now + Duration::days(14);
        }
        input.validate(now)?;
        let row = crate::lifecycle::create_listing(&mut tx, oteryn_core::new_id(), &input).await?;
        tx.commit().await?;
        Ok(row)
    }

    pub async fn publish(&self, id: Uuid, owner: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = locked(&mut tx, id).await?;
        self.currency(listing.currency)?;
        self.authorize(&mut tx, owner, true).await?;
        if listing.owner != owner || listing.ends_at <= clock(&mut tx).await? {
            return Err(Error::Denied);
        }
        listing.state.transition(State::Active)?;
        if !self.evidence.escrow_ready(&mut tx, &listing).await? {
            return Err(Error::Denied);
        }
        crate::lifecycle::write_listing(
            &mut tx,
            id,
            crate::lifecycle::Event::Listing(State::Active),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn bid(
        &self,
        id: Uuid,
        account: Uuid,
        maximum: i64,
        key: &str,
    ) -> Result<BidOutcome, Error> {
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let result = self.bid_in(&mut tx, id, account, maximum, key).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Caller-owned transaction: destination choice, maximum funding and bid
    /// idempotency must commit together. Roll back on any error.
    pub async fn bid_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        account: Uuid,
        maximum: i64,
        key: &str,
    ) -> Result<BidOutcome, Error> {
        self.enabled()?;
        validate_key(key)?;
        if maximum <= 0 {
            return Err(Error::InvalidInput);
        }
        let listing = locked(tx, id).await?;
        self.authorize_bid(tx, &listing, account).await?;
        if let Some(outcome) = Self::prior_bid(tx, id, account, maximum, key).await? {
            return Ok(outcome);
        }
        let now = clock(tx).await?;
        if listing.ends_at <= now {
            return Err(Error::IllegalTransition);
        }
        let (leader, price) = proxy_bid(&listing, account, maximum)?;
        let mut accounts = vec![account];
        if let Some(old) = listing.leader {
            accounts.push(old);
        }
        self.funds_lock(tx, accounts).await?;
        // Even an immediately outbid maximum must be fully funded.
        self.reservations
            .reserve(
                tx,
                Reservation {
                    id,
                    account,
                    currency: listing.currency,
                    amount: maximum,
                },
            )
            .await?;
        if leader != account {
            self.reservations.release(tx, id, account).await?;
        } else if let Some(old) = listing.leader.filter(|old| *old != account) {
            self.reservations.release(tx, id, old).await?;
        }
        let leader_max = if leader == account {
            maximum
        } else {
            listing.leader_max.ok_or(Error::IllegalTransition)?
        };
        let ends_at = if listing.ends_at - now < Duration::minutes(5) {
            listing
                .ends_at
                .checked_add(Duration::minutes(5))
                .ok_or(Error::Overflow)?
        } else {
            listing.ends_at
        };
        sqlx::query(
            "UPDATE bazaar_listings SET leader=$2,leader_max=$3,price=$4,ends_at=$5,
            reservation=id,updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(id)
        .bind(leader)
        .bind(leader_max)
        .bind(price)
        .bind(ends_at)
        .execute(&mut **tx)
        .await?;
        sqlx::query("INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(oteryn_core::new_id()).bind(id).bind(account).bind(maximum).bind(key)
            .bind(leader).bind(price).bind(ends_at).execute(&mut **tx).await?;
        Ok(BidOutcome {
            leader,
            price,
            ends_at,
        })
    }

    // The caller has already checked the immutable browser request binding.
    // Preserve current authorization without taking the listing's writer lock.
    pub(crate) async fn replay_coin_bid(
        &self,
        id: Uuid,
        account: Uuid,
        maximum: i64,
        key: &str,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing: Listing =
            sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR SHARE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        if listing.owner == account || listing.currency != Currency::Coins {
            return Err(Error::Denied);
        }
        self.authorize_bid(&mut tx, &listing, account).await?;
        Self::prior_bid(&mut tx, id, account, maximum, key)
            .await?
            .ok_or(Error::IllegalTransition)?;
        tx.commit().await?;
        Ok(())
    }

    async fn authorize_bid(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        account: Uuid,
    ) -> Result<(), Error> {
        self.currency(listing.currency)?;
        let mut parties = vec![account, listing.owner];
        parties.extend(listing.leader);
        self.accounts.prepare(db, &parties).await?;
        self.authorize_many(db, vec![(account, false), (listing.owner, false)])
            .await
    }

    async fn prior_bid(
        db: &mut PgConnection,
        id: Uuid,
        account: Uuid,
        maximum: i64,
        key: &str,
    ) -> Result<Option<BidOutcome>, Error> {
        let previous: Option<(Uuid, i64, Uuid, i64, OffsetDateTime)> = sqlx::query_as(
            "SELECT account,maximum,leader,price,ends_at FROM bazaar_bids WHERE listing_id=$1 AND idempotency_key=$2")
            .bind(id).bind(key).fetch_optional(db).await?;
        let Some((actor, amount, leader, price, ends_at)) = previous else {
            return Ok(None);
        };
        if actor != account || amount != maximum {
            return Err(Error::IdempotencyConflict);
        }
        Ok(Some(BidOutcome {
            leader,
            price,
            ends_at,
        }))
    }

    pub async fn offer(
        &self,
        id: Uuid,
        account: Uuid,
        amount: i64,
        key: &str,
    ) -> Result<Offer, Error> {
        self.enabled()?;
        validate_key(key)?;
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = locked(&mut tx, id).await?;
        self.currency(listing.currency)?;
        self.authorize_many(&mut tx, vec![(account, false), (listing.owner, false)])
            .await?;
        let existing: Option<Offer> = sqlx::query_as(
            "SELECT * FROM bazaar_offers WHERE listing_id=$1 AND idempotency_key=$2",
        )
        .bind(id)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(offer) = existing {
            if offer.account != account || offer.amount != amount {
                return Err(Error::IdempotencyConflict);
            }
            return Ok(offer);
        }
        if listing.state != State::Active
            || listing.form == Form::Auction
            || listing.owner == account
            || listing.ends_at <= clock(&mut tx).await?
            || (!listing.accept_offers && amount != listing.price)
        {
            return Err(Error::IllegalTransition);
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,57))")
            .bind(account.to_string())
            .execute(&mut *tx)
            .await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_offers WHERE account=$1 AND state='pending'",
        )
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND account=$2 AND state='pending')")
            .bind(id).bind(account).fetch_one(&mut *tx).await?;
        if count >= 20 || duplicate {
            return Err(Error::Denied);
        }
        crate::bounded::admit_offer(&mut tx, id).await?;
        let offer_id = oteryn_core::new_id();
        if listing.form == Form::FixedPrice {
            self.funds_lock(&mut tx, vec![account]).await?;
            self.reservations
                .reserve(
                    &mut tx,
                    Reservation {
                        id: offer_id,
                        account,
                        currency: listing.currency,
                        amount,
                    },
                )
                .await?;
        }
        let offer = sqlx::query_as(
            "INSERT INTO bazaar_offers(id,listing_id,account,amount,idempotency_key)
            VALUES ($1,$2,$3,$4,$5) RETURNING *",
        )
        .bind(offer_id)
        .bind(id)
        .bind(account)
        .bind(amount)
        .bind(key)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(offer)
    }

    /// Want-to-buy reverses buyer/seller roles. Integration evidence must cover the selected offer.
    pub async fn accept_offer(&self, id: Uuid, owner: Uuid, offer_id: Uuid) -> Result<(), Error> {
        self.enabled()?;
        loop {
            let mut tx = self.pool.begin().await?;
            crate::locking::listing_gate(&mut tx).await?;
            let listing = locked(&mut tx, id).await?;
            self.currency(listing.currency)?;
            let continuing =
                listing.state == State::Matched && listing.reservation == Some(offer_id);
            if listing.owner != owner
                || (!continuing
                    && (listing.state != State::Active
                        || listing.form == Form::Auction
                        || listing.ends_at <= clock(&mut tx).await?))
            {
                return Err(Error::IllegalTransition);
            }
            if continuing {
                let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND state='pending')").bind(id).fetch_one(&mut *tx).await?;
                if !pending {
                    return Err(Error::IllegalTransition);
                }
            }
            let selected: Offer = sqlx::query_as(
                "SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 FOR UPDATE",
            )
            .bind(id)
            .bind(offer_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
            let resumed = listing.state == State::Matched
                && listing.reservation == Some(offer_id)
                && selected.state == "accepted";
            if listing.owner != owner
                || (!resumed
                    && (listing.state != State::Active
                        || listing.form == Form::Auction
                        || listing.ends_at <= clock(&mut tx).await?
                        || selected.state != "pending"))
            {
                return Err(Error::IllegalTransition);
            }
            let offers = crate::bounded::offers(&mut tx, id, Some(offer_id)).await?;
            let mut parties: Vec<_> = offers.iter().map(|offer| offer.account).collect();
            parties.extend([owner, selected.account]);
            self.accounts.prepare(&mut tx, &parties).await?;
            self.authorize_many(&mut tx, vec![(owner, true), (selected.account, false)])
                .await?;
            if !resumed
                && !self
                    .evidence
                    .offer_ready(&mut tx, &listing, &selected)
                    .await?
            {
                return Err(Error::Denied);
            }

            let mut accounts: Vec<_> = offers.iter().map(|offer| offer.account).collect();
            accounts.extend([owner, selected.account]);
            self.funds_lock(&mut tx, accounts).await?;
            if !resumed {
                if listing.form == Form::WantToBuy {
                    self.reservations
                        .reserve(
                            &mut tx,
                            Reservation {
                                id: offer_id,
                                account: owner,
                                currency: listing.currency,
                                amount: selected.amount,
                            },
                        )
                        .await?;
                }
                sqlx::query("UPDATE bazaar_offers SET state='accepted',updated_at=clock_timestamp() WHERE id=$1").bind(offer_id).execute(&mut *tx).await?;
                crate::lifecycle::write_listing_with(
                    &mut tx,
                    id,
                    crate::lifecycle::Event::Listing(State::Matched),
                    crate::lifecycle::ListingChange::Match {
                        buyer: selected.account,
                        amount: selected.amount,
                        reservation: offer_id,
                    },
                )
                .await?;
            }
            if listing.form == Form::FixedPrice {
                for offer in &offers {
                    self.reservations
                        .release(&mut tx, offer.id, offer.account)
                        .await?;
                }
            }
            crate::bounded::finish(&mut tx, &offers, "rejected").await?;
            let remaining:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND state='pending')").bind(id).fetch_one(&mut *tx).await?;
            tx.commit().await?;
            if !remaining {
                return Ok(());
            }
        }
    }

    pub async fn reject_offer(&self, id: Uuid, owner: Uuid, offer_id: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = locked(&mut tx, id).await?;
        self.currency(listing.currency)?;
        if listing.owner != owner || listing.state != State::Active {
            return Err(Error::IllegalTransition);
        }
        let offer:Offer=sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 AND state='pending' FOR UPDATE").bind(id).bind(offer_id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        self.accounts
            .prepare(&mut tx, &[owner, offer.account])
            .await?;
        self.authorize(&mut tx, owner, false).await?;
        if listing.form == Form::FixedPrice {
            self.funds_lock(&mut tx, vec![offer.account]).await?;
            self.reservations
                .release(&mut tx, offer.id, offer.account)
                .await?;
        }
        sqlx::query(
            "UPDATE bazaar_offers SET state='rejected',updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(offer_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Cancellation stops at the first bid or offer, including rejected offers.
    pub async fn cancel(&self, id: Uuid, owner: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = locked(&mut tx, id).await?;
        self.currency(listing.currency)?;
        self.authorize(&mut tx, owner, true).await?;
        if owner != listing.owner {
            return Err(Error::Denied);
        }
        listing.state.transition(State::Cancelled)?;
        let has_activity: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_bids WHERE listing_id=$1)
            OR EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if has_activity {
            return Err(Error::IllegalTransition);
        }
        if !self.evidence.cancellation_ready(&mut tx, &listing).await? {
            return Err(Error::Denied);
        }
        crate::lifecycle::write_listing(
            &mut tx,
            id,
            crate::lifecycle::Event::Listing(State::Cancelled),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn expire(&self, id: Uuid) -> Result<(), Error> {
        self.enabled()?;
        loop {
            let mut tx = self.pool.begin().await?;
            crate::locking::listing_gate(&mut tx).await?;
            let listing = locked(&mut tx, id).await?;
            self.currency(listing.currency)?;
            if listing.state != State::Active
                || listing.leader.is_some()
                || listing.ends_at > clock(&mut tx).await?
            {
                return Err(Error::IllegalTransition);
            }
            let offers = crate::bounded::offers(&mut tx, id, None).await?;
            let mut parties: Vec<_> = offers.iter().map(|offer| offer.account).collect();
            parties.push(listing.owner);
            self.accounts.prepare(&mut tx, &parties).await?;
            self.funds_lock(&mut tx, offers.iter().map(|o| o.account).collect())
                .await?;
            if !self.evidence.cancellation_ready(&mut tx, &listing).await? {
                return Err(Error::Denied);
            }
            if listing.form == Form::FixedPrice {
                for offer in &offers {
                    self.reservations
                        .release(&mut tx, offer.id, offer.account)
                        .await?;
                }
            }
            crate::bounded::finish(&mut tx, &offers, "expired").await?;
            let complete = offers.len() < crate::bounded::OFFER_BATCH;
            if complete {
                crate::lifecycle::write_listing(
                    &mut tx,
                    id,
                    crate::lifecycle::Event::Listing(State::Expired),
                )
                .await?;
            }
            tx.commit().await?;
            if complete {
                return Ok(());
            }
        }
    }

    /// Proceeds remain pending until delivery. Retries return the original settlement.
    pub async fn settle(&self, id: Uuid, key: &str) -> Result<Settlement, Error> {
        self.enabled()?;
        validate_key(key)?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = lock_listing_row(&mut tx, id).await?;
        let result = self.settle_locked(&mut tx, &listing, key).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub(crate) async fn settle_locked(
        &self,
        tx: &mut PgConnection,
        listing: &Listing,
        key: &str,
    ) -> Result<Settlement, Error> {
        let id = listing.id;
        self.currency(listing.currency)?;
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT idempotency_key FROM bazaar_settlements WHERE listing_id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((old_key,)) = existing {
            if old_key != key {
                return Err(Error::IdempotencyConflict);
            }
            return settlement(tx, id).await;
        }
        let eligible = match listing.form {
            Form::Auction => listing.state == State::Active && listing.ends_at <= clock(tx).await?,
            _ => listing.state == State::Matched,
        };
        if !eligible {
            return Err(Error::IllegalTransition);
        }
        let pending: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND state='pending')",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if pending {
            return Err(Error::IllegalTransition);
        }
        let participant = listing.leader.ok_or(Error::IllegalTransition)?;
        let (buyer, seller) = if listing.form == Form::WantToBuy {
            (listing.owner, participant)
        } else {
            (participant, listing.owner)
        };
        let mut actors = vec![buyer, seller];
        actors.sort_unstable();
        for actor in &actors {
            self.authorize(tx, *actor, false).await?;
        }
        self.funds_lock(tx, actors).await?;
        let commission = listing.currency.commission(listing.price)?;
        let result = Settlement {
            id: oteryn_core::new_id(),
            listing_id: id,
            buyer,
            seller,
            currency: listing.currency,
            amount: listing.price,
            commission,
            seller_net: listing.price - commission,
            reservation: listing.reservation.ok_or(Error::IllegalTransition)?,
        };
        self.reservations.settle(tx, &result).await?;
        crate::lifecycle::write_coin_sale(tx, &result, key).await?;
        crate::lifecycle::write_listing(
            tx,
            id,
            crate::lifecycle::Event::Listing(State::AwaitingDelivery),
        )
        .await?;
        Ok(result)
    }

    pub async fn complete_delivery(&self, id: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing = locked(&mut tx, id).await?;
        self.complete_delivery_in(&mut tx, &listing).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn complete_delivery_in(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> Result<(), Error> {
        let id = listing.id;
        self.currency(listing.currency)?;
        if listing.state == State::Completed {
            return Ok(());
        }
        listing.state.transition(State::Completed)?;
        let result = settlement(db, id).await?;
        self.accounts
            .prepare(db, &[result.buyer, result.seller])
            .await?;
        for actor in [result.buyer, result.seller] {
            if !self.accounts.safe_return(db, actor).await? {
                return Err(Error::Denied);
            }
        }
        if !self.evidence.delivered(db, &result).await? {
            return Err(Error::Denied);
        }
        self.funds_lock(db, vec![result.buyer, result.seller])
            .await?;
        self.reservations.complete(db, &result).await?;
        crate::lifecycle::complete_coin_sale(db, id).await?;
        crate::lifecycle::write_listing(db, id, crate::lifecycle::Event::Listing(State::Completed))
            .await?;
        Ok(())
    }
}

async fn clock(db: &mut PgConnection) -> Result<OffsetDateTime, Error> {
    Ok(sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(db)
        .await?)
}
async fn locked(db: &mut PgConnection, id: Uuid) -> Result<Listing, Error> {
    crate::locking::listing_gate(db).await?;
    lock_listing_row(db, id).await
}

async fn lock_listing_row(db: &mut PgConnection, id: Uuid) -> Result<Listing, Error> {
    sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)
}
async fn settlement(db: &mut PgConnection, id: Uuid) -> Result<Settlement, Error> {
    sqlx::query_as("SELECT * FROM bazaar_settlements WHERE listing_id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)
}
