use super::*;
use crate::{BidOutcome, Offer, Reservation, proxy_bid};

#[derive(Debug, Clone)]
pub struct Items<A, R, G> {
    pool: PgPool,
    accounts: A,
    funds: R,
    authority: G,
    flags: ItemFlags,
}
impl<A: Accounts, R: Reservations, G: ItemAuthority> Items<A, R, G> {
    pub fn new(pool: PgPool, accounts: A, funds: R, authority: G, flags: ItemFlags) -> Self {
        Self {
            pool,
            accounts,
            funds,
            authority,
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
    async fn authorize(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        listing: bool,
    ) -> Result<(), Error> {
        let state = self.accounts.state(db, actor).await?.ok_or(Error::Denied)?;
        if !id(actor)
            || !(if listing {
                state.allows_listing()
            } else {
                state.allows_trade()
            })
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn current(&self, db: &mut PgConnection, row: &Listing) -> Result<(), Error> {
        if row.kind != Kind::Item
            || row.currency != Currency::Coins
            || !self.authority.current(db, row).await?
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    async fn parties(
        &self,
        db: &mut PgConnection,
        row: &Listing,
        actor: Uuid,
    ) -> Result<(), Error> {
        let mut actors = vec![row.owner, actor];
        actors.sort();
        actors.dedup();
        for account in actors {
            self.authorize(db, account, false).await?;
        }
        self.current(db, row).await
    }
    /// Coin prices are per unit on input; the ledger always reserves the full stack total.
    /// Gold is unavailable: T57's rounding/settlement adapter predates G-7 revision 2.
    pub async fn create(&self, actor: Uuid, input: CreateItem) -> Result<Uuid, Error> {
        self.enabled()?;
        input.validate()?;
        let binding = serde_json::to_string(&input).map_err(|_| Error::InvalidInput)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,60))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let prior: Option<(Uuid,String)> = sqlx::query_as("SELECT listing_id,binding::text FROM bazaar_item_requests WHERE actor=$1 AND request_id=$2")
            .bind(actor).bind(input.request_id).fetch_optional(&mut *tx).await?;
        if let Some((listing, old)) = prior {
            let old: serde_json::Value = serde_json::from_str(&old).map_err(|_| Error::Denied)?;
            if old != serde_json::to_value(&input).map_err(|_| Error::InvalidInput)? {
                return Err(Error::IdempotencyConflict);
            }
            return Ok(listing);
        }
        if !self.authority.preflight(&mut tx, actor, &input).await? {
            return Err(Error::Denied);
        }
        let listing = Uuid::now_v7();
        let now = clock(&mut tx).await?;
        let end = now + Duration::days(i64::from(input.auction_days.unwrap_or(14)));
        // T57 requires a positive internal floor. A negotiated notice has no public price.
        let total = input.total(input.unit_price.unwrap_or(1))?;
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES ($1,$2,'item',$3,'coins',$4,$5,$6)")
            .bind(listing).bind(actor).bind(input.mode.form()).bind(total).bind(matches!(input.mode,Mode::Offers|Mode::Buy)).bind(end).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_item_details(id,definition_key,definition_revision,world_id,seller_character_id,item_instance_id,quantity,requested_price,buy_now_price,message) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
            .bind(listing).bind(&input.definition_key).bind(&input.definition_revision).bind(input.world_id).bind(input.seller_character_id).bind(input.item_instance_id)
            .bind(i64::from(input.quantity)).bind(input.unit_price).bind(input.buy_now_unit_price).bind(&input.message).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_item_requests(actor,request_id,binding,listing_id) VALUES ($1,$2,$3::text::jsonb,$4)")
            .bind(actor).bind(input.request_id).bind(binding).bind(listing).execute(&mut *tx).await?;
        if input.mode != Mode::Buy {
            enqueue(&mut tx, listing, "escrow", input.request_id).await?;
        }
        tx.commit().await?;
        Ok(listing)
    }
    /// Publication and immutable approved facts are committed together, only after escrow.
    pub async fn publish(&self, listing: Uuid, actor: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, true).await?;
        if row.owner != actor || row.state != State::Draft || row.ends_at <= clock(&mut tx).await? {
            return Err(Error::Denied);
        }
        let projection = self
            .authority
            .publication(&mut tx, &row)
            .await?
            .ok_or(Error::Denied)?;
        projection.validate(row.form == Form::WantToBuy)?;
        let attributes =
            serde_json::to_string(&projection.attributes).map_err(|_| Error::Denied)?;
        sqlx::query("UPDATE bazaar_item_details SET approved_name=$2,approved_attributes=$3::text::jsonb,escrow_id=$4,escrow_receipt_digest=$5,projection_epoch=$6,recovery_generation=$7,updated_at=clock_timestamp() WHERE id=$1")
            .bind(listing).bind(projection.name).bind(attributes).bind(projection.escrow_id).bind(projection.escrow_receipt_digest).bind(projection.projection_epoch).bind(projection.recovery_generation).execute(&mut *tx).await?;
        state(&mut tx, listing, State::Active).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn bid(
        &self,
        listing: Uuid,
        actor: Uuid,
        unit_maximum: i64,
        request: Uuid,
    ) -> Result<BidOutcome, Error> {
        self.enabled()?;
        if !id(request) || unit_maximum <= 0 {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.parties(&mut tx, &row, actor).await?;
        let amount = total(&mut tx, listing, unit_maximum).await?;
        let key = request.to_string();
        let old: Option<(Uuid,i64,Uuid,i64,OffsetDateTime)> = sqlx::query_as("SELECT account,maximum,leader,price,ends_at FROM bazaar_bids WHERE listing_id=$1 AND idempotency_key=$2")
            .bind(listing).bind(&key).fetch_optional(&mut *tx).await?;
        if let Some((who, max, leader, price, ends_at)) = old {
            if who != actor || max != amount {
                return Err(Error::IdempotencyConflict);
            }
            return Ok(BidOutcome {
                leader,
                price,
                ends_at,
            });
        }
        let now = clock(&mut tx).await?;
        if row.ends_at <= now {
            return Err(Error::IllegalTransition);
        }
        let (leader, price) = proxy_bid(&row, actor, amount)?;
        let mut actors = vec![actor];
        if let Some(old) = row.leader {
            actors.push(old);
        }
        actors.sort();
        actors.dedup();
        self.funds.lock_accounts(&mut tx, &actors).await?;
        self.funds
            .reserve(
                &mut tx,
                Reservation {
                    id: listing,
                    account: actor,
                    currency: Currency::Coins,
                    amount,
                },
            )
            .await?;
        if leader != actor {
            self.funds.release(&mut tx, listing, actor).await?;
        } else if let Some(old) = row.leader.filter(|old| *old != actor) {
            self.funds.release(&mut tx, listing, old).await?;
        }
        let maximum = if leader == actor {
            amount
        } else {
            row.leader_max.ok_or(Error::Denied)?
        };
        let end = if row.ends_at - now < Duration::minutes(5) {
            row.ends_at + Duration::minutes(5)
        } else {
            row.ends_at
        };
        sqlx::query("UPDATE bazaar_listings SET leader=$2,leader_max=$3,price=$4,reservation=id,ends_at=$5,updated_at=clock_timestamp() WHERE id=$1")
            .bind(listing).bind(leader).bind(maximum).bind(price).bind(end).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(Uuid::now_v7()).bind(listing).bind(actor).bind(amount).bind(key).bind(leader).bind(price).bind(end).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(BidOutcome {
            leader,
            price,
            ends_at: end,
        })
    }
    /// For buy notices actor is the seller and T58 validates that response's deposit.
    pub async fn offer(
        &self,
        listing: Uuid,
        actor: Uuid,
        unit_amount: i64,
        request: Uuid,
    ) -> Result<Offer, Error> {
        self.enabled()?;
        if !id(request) || unit_amount <= 0 {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.parties(&mut tx, &row, actor).await?;
        let amount = total(&mut tx, listing, unit_amount).await?;
        let key = request.to_string();
        let old: Option<Offer> = sqlx::query_as(
            "SELECT * FROM bazaar_offers WHERE listing_id=$1 AND idempotency_key=$2",
        )
        .bind(listing)
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = old {
            if old.account != actor || old.amount != amount {
                return Err(Error::IdempotencyConflict);
            }
            return Ok(old);
        }
        if row.owner == actor
            || row.state != State::Active
            || row.form == Form::Auction
            || row.ends_at <= clock(&mut tx).await?
            || (!row.accept_offers && row.price != amount)
        {
            return Err(Error::IllegalTransition);
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,57))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_offers WHERE account=$1 AND state='pending'",
        )
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1 AND account=$2 AND state='pending')").bind(listing).bind(actor).fetch_one(&mut *tx).await?;
        if count >= 20 || duplicate {
            return Err(Error::Denied);
        }
        let offer = request;
        if row.form == Form::WantToBuy {
            if !self
                .authority
                .offer_ready(&mut tx, &row, offer, actor)
                .await?
            {
                return Err(Error::Denied);
            }
        } else {
            self.funds.lock_accounts(&mut tx, &[actor]).await?;
            self.funds
                .reserve(
                    &mut tx,
                    Reservation {
                        id: offer,
                        account: actor,
                        currency: Currency::Coins,
                        amount,
                    },
                )
                .await?;
        }
        let result=sqlx::query_as("INSERT INTO bazaar_offers(id,listing_id,account,amount,idempotency_key) VALUES ($1,$2,$3,$4,$5) RETURNING *")
            .bind(offer).bind(listing).bind(actor).bind(amount).bind(key).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn accept(&self, listing: Uuid, actor: Uuid, selected: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, true).await?;
        if row.owner != actor {
            return Err(Error::Denied);
        }
        let offers: Vec<Offer> = sqlx::query_as(
            "SELECT * FROM bazaar_offers WHERE listing_id=$1 ORDER BY id FOR UPDATE",
        )
        .bind(listing)
        .fetch_all(&mut *tx)
        .await?;
        let chosen = offers
            .iter()
            .find(|o| o.id == selected)
            .ok_or(Error::NotFound)?;
        if chosen.state == "accepted" && row.state == State::Matched {
            return Ok(());
        }
        if chosen.state != "pending"
            || row.form == Form::Auction
            || row.state != State::Active
            || row.ends_at <= clock(&mut tx).await?
        {
            return Err(Error::IllegalTransition);
        }
        self.parties(&mut tx, &row, chosen.account).await?;
        if !self
            .authority
            .offer_ready(
                &mut tx,
                &row,
                selected,
                if row.form == Form::WantToBuy {
                    chosen.account
                } else {
                    actor
                },
            )
            .await?
        {
            return Err(Error::Denied);
        }
        let mut actors: Vec<_> = offers
            .iter()
            .filter(|o| o.state == "pending")
            .map(|o| o.account)
            .collect();
        actors.push(actor);
        actors.sort();
        actors.dedup();
        self.funds.lock_accounts(&mut tx, &actors).await?;
        let buyer = if row.form == Form::WantToBuy {
            actor
        } else {
            chosen.account
        };
        // Reprove the full reservation before releasing competitors or fencing delivery.
        self.funds
            .reserve(
                &mut tx,
                Reservation {
                    id: selected,
                    account: buyer,
                    currency: Currency::Coins,
                    amount: chosen.amount,
                },
            )
            .await?;
        for offer in &offers {
            if offer.state == "pending" && offer.id != selected && row.form != Form::WantToBuy {
                self.funds.release(&mut tx, offer.id, offer.account).await?;
            }
        }
        sqlx::query("UPDATE bazaar_offers SET state=CASE WHEN id=$2 THEN 'accepted' ELSE 'rejected' END,updated_at=clock_timestamp() WHERE listing_id=$1 AND state='pending'").bind(listing).bind(selected).execute(&mut *tx).await?;
        sqlx::query("UPDATE bazaar_listings SET state='matched',leader=$2,leader_max=$3,price=$3,reservation=$4,updated_at=clock_timestamp() WHERE id=$1")
            .bind(listing).bind(chosen.account).bind(chosen.amount).bind(selected).execute(&mut *tx).await?;
        let seller = if row.form == Form::WantToBuy {
            chosen.account
        } else {
            actor
        };
        freeze_sale(&mut tx, listing, buyer, seller, chosen.amount).await?;
        enqueue(&mut tx, listing, "delivery", selected).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn reject(&self, listing: Uuid, actor: Uuid, offer: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, false).await?;
        if row.owner != actor || row.state != State::Active {
            return Err(Error::Denied);
        }
        let selected: Offer =
            sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 FOR UPDATE")
                .bind(listing)
                .bind(offer)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        if selected.state == "rejected" {
            return Ok(());
        }
        if selected.state != "pending" {
            return Err(Error::IllegalTransition);
        }
        if row.form != Form::WantToBuy {
            self.funds
                .lock_accounts(&mut tx, &[selected.account])
                .await?;
            self.funds.release(&mut tx, offer, selected.account).await?;
        }
        sqlx::query(
            "UPDATE bazaar_offers SET state='rejected',updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(offer)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Buy-now disappears after the first bid. Funding and the delivery fence are atomic.
    pub async fn buy_now(&self, listing: Uuid, actor: Uuid, request: Uuid) -> Result<(), Error> {
        self.enabled()?;
        if !id(request) {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.parties(&mut tx, &row, actor).await?;
        if row.owner == actor || row.form != Form::Auction {
            return Err(Error::Denied);
        }
        let previous:Option<(Uuid,String)>=sqlx::query_as("SELECT listing_id,binding::text FROM bazaar_item_requests WHERE actor=$1 AND request_id=$2").bind(actor).bind(request).fetch_optional(&mut *tx).await?;
        if let Some((who, binding)) = previous {
            if who == listing && binding == "\"buy_now\"" {
                return Ok(());
            }
            return Err(Error::IdempotencyConflict);
        }
        if row.state != State::Active
            || row.leader.is_some()
            || row.ends_at <= clock(&mut tx).await?
        {
            return Err(Error::IllegalTransition);
        }
        let unit: Option<i64> =
            sqlx::query_scalar("SELECT buy_now_price FROM bazaar_item_details WHERE id=$1")
                .bind(listing)
                .fetch_one(&mut *tx)
                .await?;
        let amount = total(&mut tx, listing, unit.ok_or(Error::Denied)?).await?;
        self.funds.lock_accounts(&mut tx, &[actor]).await?;
        self.funds
            .reserve(
                &mut tx,
                Reservation {
                    id: listing,
                    account: actor,
                    currency: Currency::Coins,
                    amount,
                },
            )
            .await?;
        sqlx::query("UPDATE bazaar_listings SET state='matched',leader=$2,leader_max=$3,price=$3,reservation=id,updated_at=clock_timestamp() WHERE id=$1").bind(listing).bind(actor).bind(amount).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_item_requests(actor,request_id,binding,listing_id) VALUES ($1,$2,'\"buy_now\"'::jsonb,$3)").bind(actor).bind(request).bind(listing).execute(&mut *tx).await?;
        freeze_sale(&mut tx, listing, actor, row.owner, amount).await?;
        enqueue(&mut tx, listing, "delivery", request).await?;
        tx.commit().await?;
        Ok(())
    }
    /// No debit or seller credit occurs here. T58 owns delivery-before-payout reconciliation.
    pub async fn close_auction(&self, listing: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        if row.form != Form::Auction
            || row.state != State::Active
            || row.ends_at > clock(&mut tx).await?
        {
            return Err(Error::IllegalTransition);
        }
        let winner = row.leader.ok_or(Error::Denied)?;
        self.parties(&mut tx, &row, winner).await?;
        self.funds.lock_accounts(&mut tx, &[winner]).await?;
        self.funds
            .reserve(
                &mut tx,
                Reservation {
                    id: listing,
                    account: winner,
                    currency: Currency::Coins,
                    amount: row.leader_max.ok_or(Error::Denied)?,
                },
            )
            .await?;
        state(&mut tx, listing, State::Matched).await?;
        freeze_sale(&mut tx, listing, winner, row.owner, row.price).await?;
        enqueue(&mut tx, listing, "delivery", listing).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn cancel(&self, listing: Uuid, actor: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, true).await?;
        if row.owner != actor {
            return Err(Error::Denied);
        }
        if row.state == State::Cancelled {
            return Ok(());
        }
        row.state.transition(State::Cancelled)?;
        let activity:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_bids WHERE listing_id=$1) OR EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1)").bind(listing).fetch_one(&mut *tx).await?;
        if activity {
            return Err(Error::Denied);
        }
        if !self.authority.cancellation_ready(&mut tx, &row).await? {
            if row.form != Form::WantToBuy {
                enqueue(&mut tx, listing, "return", listing).await?;
            }
            tx.commit().await?;
            return Ok(());
        }
        state(&mut tx, listing, State::Cancelled).await?;
        if row.form != Form::WantToBuy {
            enqueue(&mut tx, listing, "return", listing).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    /// Renewal is limited to still-active advertisements with no buyer activity. Expired
    /// deposits require T58's safe return/relisting evidence rather than reusing custody.
    pub async fn renew(&self, listing: Uuid, actor: Uuid, request: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, true).await?;
        self.current(&mut tx, &row).await?;
        if !id(request) {
            return Err(Error::InvalidInput);
        }
        let previous:Option<(Uuid,String)>=sqlx::query_as("SELECT listing_id,binding::text FROM bazaar_item_requests WHERE actor=$1 AND request_id=$2").bind(actor).bind(request).fetch_optional(&mut *tx).await?;
        if let Some((old, binding)) = previous {
            if old == listing && binding == "\"renew\"" {
                return Ok(());
            }
            return Err(Error::IdempotencyConflict);
        }
        let activity: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_offers WHERE listing_id=$1)")
                .bind(listing)
                .fetch_one(&mut *tx)
                .await?;
        if row.owner != actor
            || row.state != State::Active
            || row.form == Form::Auction
            || activity
            || row.ends_at <= clock(&mut tx).await?
        {
            return Err(Error::Denied);
        }
        sqlx::query("UPDATE bazaar_listings SET ends_at=clock_timestamp()+interval '14 days',updated_at=clock_timestamp() WHERE id=$1").bind(listing).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_item_requests(actor,request_id,binding,listing_id) VALUES ($1,$2,'\"renew\"'::jsonb,$3)").bind(actor).bind(request).bind(listing).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn expire(&self, listing: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        if row.state != State::Active
            || row.leader.is_some()
            || row.ends_at > clock(&mut tx).await?
        {
            return Err(Error::IllegalTransition);
        }
        let offers:Vec<Offer>=sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND state='pending' ORDER BY id FOR UPDATE").bind(listing).fetch_all(&mut *tx).await?;
        let mut accounts: Vec<_> = offers.iter().map(|o| o.account).collect();
        accounts.sort();
        accounts.dedup();
        if row.form != Form::WantToBuy {
            self.funds.lock_accounts(&mut tx, &accounts).await?;
            if !self.authority.cancellation_ready(&mut tx, &row).await? {
                enqueue(&mut tx, listing, "return", listing).await?;
                tx.commit().await?;
                return Ok(());
            }
            for offer in offers {
                self.funds.release(&mut tx, offer.id, offer.account).await?;
            }
            enqueue(&mut tx, listing, "return", listing).await?;
        }
        sqlx::query("UPDATE bazaar_offers SET state='expired',updated_at=clock_timestamp() WHERE listing_id=$1 AND state='pending'").bind(listing).execute(&mut *tx).await?;
        state(&mut tx, listing, State::Expired).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Owner-only drafts and offer inbox. No private proxy maxima cross this projection.
    pub async fn owner_view(
        &self,
        listing: Uuid,
        actor: Uuid,
    ) -> Result<(ItemView, Vec<Offer>), Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        self.authorize(&mut tx, actor, false).await?;
        if row.owner != actor {
            return Err(Error::Denied);
        }
        let view=sqlx::query_as("SELECT d.id,COALESCE(d.approved_name,d.definition_key) AS name,d.definition_key,d.quantity,d.requested_price AS unit_price,l.price AS total_price,CASE WHEN l.leader IS NULL THEN d.buy_now_price ELSE NULL END AS buy_now_unit_price,COALESCE(d.approved_attributes,'[]'::jsonb) AS attributes,d.message,l.ends_at,l.form,l.accept_offers FROM bazaar_item_details d JOIN bazaar_listings l ON l.id=d.id WHERE d.id=$1")
            .bind(listing).fetch_one(&mut *tx).await?;
        let offers=sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND state='pending' ORDER BY created_at LIMIT 20").bind(listing).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok((view, offers))
    }
    pub async fn recent(&self) -> Result<Vec<ItemView>, Error> {
        self.enabled()?;
        let ids:Vec<Uuid>=sqlx::query_scalar("SELECT l.id FROM bazaar_listings l JOIN bazaar_item_details d ON d.id=l.id WHERE l.state='active' AND l.ends_at>clock_timestamp() ORDER BY l.created_at DESC LIMIT 24").fetch_all(&self.pool).await?;
        let mut views = Vec::new();
        for id in ids {
            match self.view(id).await {
                Ok(view) => views.push(view),
                Err(Error::Denied | Error::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(views)
    }
    /// Public reads require fresh privacy/custody evidence in the same transaction.
    pub async fn view(&self, listing: Uuid) -> Result<ItemView, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let row = locked(&mut tx, listing).await?;
        if row.state != State::Active || row.ends_at <= clock(&mut tx).await? {
            return Err(Error::NotFound);
        }
        self.current(&mut tx, &row).await?;
        let view=sqlx::query_as("SELECT d.id,d.approved_name AS name,d.definition_key,d.quantity,d.requested_price AS unit_price,l.price AS total_price,CASE WHEN l.leader IS NULL THEN d.buy_now_price ELSE NULL END AS buy_now_unit_price,COALESCE(d.approved_attributes,'[]'::jsonb) AS attributes,d.message,l.ends_at,l.form,l.accept_offers FROM bazaar_item_details d JOIN bazaar_listings l ON l.id=d.id WHERE d.id=$1 AND d.approved_name IS NOT NULL")
            .bind(listing).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        tx.commit().await?;
        Ok(view)
    }
}
async fn clock(db: &mut PgConnection) -> Result<OffsetDateTime, Error> {
    Ok(sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(db)
        .await?)
}
async fn locked(db: &mut PgConnection, id: Uuid) -> Result<Listing, Error> {
    sqlx::query_as("SELECT l.* FROM bazaar_listings l JOIN bazaar_item_details d ON d.id=l.id WHERE l.id=$1 AND l.kind='item' FOR UPDATE OF l,d").bind(id).fetch_optional(db).await?.ok_or(Error::NotFound)
}
async fn state(db: &mut PgConnection, id: Uuid, next: State) -> Result<(), Error> {
    sqlx::query("UPDATE bazaar_listings SET state=$2,updated_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .bind(next)
        .execute(db)
        .await?;
    Ok(())
}
async fn total(db: &mut PgConnection, id: Uuid, unit: i64) -> Result<i64, Error> {
    let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM bazaar_item_details WHERE id=$1")
        .bind(id)
        .fetch_one(db)
        .await?;
    unit.checked_mul(quantity)
        .filter(|v| *v > 0)
        .ok_or(Error::Overflow)
}
async fn enqueue(
    db: &mut PgConnection,
    listing: Uuid,
    step: &str,
    step_id: Uuid,
) -> Result<(), Error> {
    let command = match step {
        "escrow" => "EscrowBazaarItem",
        "delivery" => "DeliverBazaarItem",
        "return" => "ReturnBazaarItem",
        _ => return Err(Error::InvalidInput),
    };
    let key = format!("bazaar:{listing}:{command}:{step_id}:1");
    // Local saga reference only. T58 must construct the accepted canonical intent once
    // the result mapping and capability advertisement are accepted. Never a wire body.
    sqlx::query("INSERT INTO bazaar_item_outbox(id,listing_id,step,issuance_key,local_payload) VALUES ($1,$2,$3,$4,jsonb_build_object('step_id',$5::text)) ON CONFLICT(issuance_key) DO NOTHING")
        .bind(Uuid::now_v7()).bind(listing).bind(step).bind(&key).bind(step_id.to_string()).execute(&mut *db).await?;
    let matches:bool=sqlx::query_scalar("SELECT listing_id=$2 AND step=$3 AND local_payload=jsonb_build_object('step_id',$4::text) FROM bazaar_item_outbox WHERE issuance_key=$1").bind(key).bind(listing).bind(step).bind(step_id.to_string()).fetch_one(&mut *db).await?;
    if !matches {
        return Err(Error::IdempotencyConflict);
    }
    Ok(())
}

async fn freeze_sale(
    db: &mut PgConnection,
    listing: Uuid,
    buyer: Uuid,
    seller: Uuid,
    gross: i64,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO bazaar_item_sales(id,listing_id,buyer,seller,gross_amount,fee_amount) VALUES ($1,$2,$3,$4,$5,$5/10)")
        .bind(Uuid::now_v7()).bind(listing).bind(buyer).bind(seller).bind(gross).execute(db).await?;
    Ok(())
}
