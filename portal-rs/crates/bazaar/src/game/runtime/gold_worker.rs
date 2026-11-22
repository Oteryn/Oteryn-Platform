use super::gold::{current_hold, delivery, record, sale};
use super::*;
use crate::lifecycle::{GoldBid, GoldPhase, create_gold_bid, write_gold_phase};
use crate::{Evidence, Reservation, Reservations, State, proxy_bid};
impl Runtime {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn request_gold_bid(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
        buyer: Uuid,
        maximum: i64,
        character: Uuid,
        key: Uuid,
    ) -> Result<(), Error> {
        if maximum <= 0 {
            return Err(Error::Invalid);
        }
        super::super::gold_payout(maximum as u64)?;
        let state = self
            .accounts(None)
            .state(tx, buyer)
            .await
            .map_err(core_error)?
            .ok_or(Error::Denied)?;
        if !state.allows_trade() {
            return Err(Error::Denied);
        }
        if check_bid_request(tx, key, l.id, buyer, character, maximum)
            .await?
            .is_some()
        {
            return Ok(());
        }
        if l.ends_at.unix_timestamp() <= now(tx).await? as i64
            || !self
                .authority()
                .escrow_ready(tx, l)
                .await
                .map_err(core_error)?
        {
            return Err(Error::Unavailable);
        }
        proxy_bid(l, buyer, maximum).map_err(core_error)?;
        let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_bids WHERE listing_id=$1 AND phase IN ('pending','releasing'))").bind(l.id).fetch_one(&mut **tx).await?;
        if pending {
            return Err(Error::Unavailable);
        }
        let hold = current_hold(self.authority(), tx, l.id, buyer, None).await?;
        let (offer, expected_revision) = super::gold::hold_binding(l.id, hold.as_ref(), key);
        let operation = self
            .issue(
                tx,
                l,
                buyer,
                None,
                Command::Hold,
                key,
                vec![
                    ("buyer_account_id", string(buyer)),
                    ("buyer_character_id", string(character)),
                    ("offer_id", string(offer)),
                    (
                        "hold_purpose",
                        string(if l.form == Form::Auction {
                            "OFFER"
                        } else {
                            "RESERVE"
                        }),
                    ),
                    ("amount", string(maximum)),
                    ("expected_hold_revision", string(expected_revision)),
                ],
            )
            .await?;
        create_gold_bid(
            tx,
            GoldBid {
                request: key,
                listing: l.id,
                buyer,
                character,
                maximum,
                operation,
            },
        )
        .await?;
        Ok(())
    }
    pub(in crate::game) async fn tick_gold_bids(&self) -> Result<(), Error> {
        let requests = gold_batch(&self.pool).await?;
        for request in requests {
            crate::scheduling::advance(&self.pool, "gold-bids", request).await?;
            match self.accept_gold_bid(request).await {
                Ok(()) => {}
                Err(Error::Database(e)) => return Err(Error::Database(e)),
                Err(_) => {}
            }
        }
        Ok(())
    }
    async fn accept_gold_bid(&self, request: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let listing: Uuid = sqlx::query_scalar(
            "SELECT listing_id FROM bazaar_authority_gold_bids WHERE request_id=$1",
        )
        .bind(request)
        .fetch_one(&mut *tx)
        .await?;
        crate::locking::listing_gate(&mut tx).await?;
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
            .bind(listing)
            .fetch_one(&mut *tx)
            .await?;
        let row =
            sqlx::query("SELECT * FROM bazaar_authority_gold_bids WHERE request_id=$1 FOR UPDATE")
                .bind(request)
                .fetch_one(&mut *tx)
                .await?;
        let mut parties = vec![l.owner, row.try_get::<Uuid, _>("buyer")?];
        parties.extend(l.leader);
        let released: Option<Uuid> = sqlx::query_scalar("SELECT (intent::jsonb->>'buyer_account_id')::uuid FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(row.try_get::<Option<Uuid>, _>("release_operation_id")?).fetch_optional(&mut *tx).await?;
        parties.extend(released);
        crate::locking::accounts(&mut tx, &parties).await?;
        self.authority().lock(&mut tx).await?;
        let phase: String = row.try_get("phase")?;
        if phase == "releasing" {
            let operation: Uuid = row
                .try_get::<Option<Uuid>, _>("release_operation_id")?
                .ok_or(Error::Unavailable)?;
            let Some((intent, receipt)) = self.authority().retained(&mut tx, operation).await?
            else {
                return Ok(());
            };
            if !receipt.committed {
                let replacement = self
                    .gold_release(
                        &mut tx,
                        &l,
                        id(text(&intent.value, "buyer_account_id")?)?,
                        number(&intent.value, "amount")?,
                        text(&intent.value, "release_reason")?,
                        request,
                    )
                    .await?;
                sqlx::query("UPDATE bazaar_authority_gold_bids SET release_operation_id=$2 WHERE request_id=$1")
                    .bind(request).bind(replacement).execute(&mut *tx).await?;
                tx.commit().await?;
                return Ok(());
            }
            let next = if row.try_get::<bool, _>("release_accepted")? {
                GoldPhase::Accepted
            } else {
                GoldPhase::Refused
            };
            write_gold_phase(&mut tx, request, next, None, None).await?;
            tx.commit().await?;
            return Ok(());
        }
        if phase != "pending" {
            return Ok(());
        }
        let operation: Uuid = row.try_get("hold_operation_id")?;
        let Some((_, receipt)) = self.authority().retained(&mut tx, operation).await? else {
            return Ok(());
        };
        if !receipt.committed {
            write_gold_phase(&mut tx, request, GoldPhase::Refused, None, None).await?;
            tx.commit().await?;
            return Ok(());
        }
        let buyer: Uuid = row.try_get("buyer")?;
        let maximum: i64 = row.try_get("maximum")?;
        let actor = self
            .accounts(None)
            .state(&mut tx, buyer)
            .await
            .map_err(core_error)?
            .ok_or(Error::Denied)?;
        let accepted_at: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let valid = l.ends_at > accepted_at
            && actor.allows_trade()
            && self
                .accounts(None)
                .state(&mut tx, l.owner)
                .await
                .map_err(core_error)?
                .is_some_and(|seller| seller.allows_trade())
            && self
                .authority()
                .escrow_ready(&mut tx, &l)
                .await
                .map_err(core_error)?;
        let outcome = if valid {
            proxy_bid(&l, buyer, maximum).ok()
        } else {
            None
        };
        let hold = current_hold(self.authority(), &mut tx, l.id, buyer, None)
            .await?
            .ok_or(Error::Unavailable)?;
        if let Some((leader, price)) = outcome {
            CoinReservations {
                authority: self.authority().clone(),
            }
            .reserve(
                &mut tx,
                Reservation {
                    id: l.id,
                    account: buyer,
                    currency: Currency::Gold,
                    amount: maximum,
                },
            )
            .await
            .map_err(core_error)?;
            let release = if leader != buyer {
                Some(
                    self.gold_release(&mut tx, &l, buyer, hold.amount, "OUTBID", request)
                        .await?,
                )
            } else if let Some(old) = l.leader.filter(|old| *old != buyer) {
                let previous = current_hold(self.authority(), &mut tx, l.id, old, None)
                    .await?
                    .ok_or(Error::Unavailable)?;
                Some(
                    self.gold_release(&mut tx, &l, old, previous.amount, "OUTBID", request)
                        .await?,
                )
            } else {
                None
            };
            let leader_max = if leader == buyer {
                maximum
            } else {
                l.leader_max.ok_or(Error::Invalid)?
            };
            let ends = if l.ends_at - accepted_at < Duration::minutes(5) {
                l.ends_at + Duration::minutes(5)
            } else {
                l.ends_at
            };
            sqlx::query("UPDATE bazaar_listings SET leader=$2,leader_max=$3,price=$4,ends_at=$5,reservation=id,updated_at=now() WHERE id=$1").bind(l.id).bind(leader).bind(leader_max).bind(price).bind(ends).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(oteryn_core::new_id()).bind(l.id).bind(buyer).bind(maximum).bind(request.to_string()).bind(leader).bind(price).bind(ends).execute(&mut *tx).await?;
            if leader == buyer {
                sqlx::query("INSERT INTO bazaar_authority_destinations(listing_id,buyer,character_id) VALUES($1,$2,$3) ON CONFLICT(listing_id,buyer) DO UPDATE SET character_id=EXCLUDED.character_id").bind(l.id).bind(buyer).bind(row.try_get::<Uuid,_>("character_id")?).execute(&mut *tx).await?;
            }
            let next = if release.is_none() {
                GoldPhase::Accepted
            } else {
                GoldPhase::Releasing
            };
            write_gold_phase(&mut tx, request, next, release, Some(true)).await?;
        } else {
            let keep = if l.leader == Some(buyer) {
                l.leader_max.ok_or(Error::Invalid)? as u64
            } else {
                0
            };
            if hold.amount < keep {
                return Err(Error::Denied);
            }
            if hold.amount == keep {
                write_gold_phase(&mut tx, request, GoldPhase::Refused, None, None).await?;
            } else {
                let operation = self
                    .gold_release(&mut tx, &l, buyer, hold.amount - keep, "EXCESS", request)
                    .await?;
                write_gold_phase(
                    &mut tx,
                    request,
                    GoldPhase::Releasing,
                    Some(operation),
                    None,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
    pub(super) async fn gold_release(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
        buyer: Uuid,
        amount: u64,
        reason: &str,
        step: Uuid,
    ) -> Result<Uuid, Error> {
        let hold = current_hold(self.authority(), tx, l.id, buyer, None)
            .await?
            .ok_or(Error::Unavailable)?;
        let key = if step == l.id { hold.id } else { step };
        self.issue(
            tx,
            l,
            l.owner,
            None,
            Command::Release,
            key,
            vec![
                ("buyer_account_id", string(buyer)),
                ("hold_id", string(hold.id)),
                ("expected_hold_revision", string(hold.revision)),
                ("release_reason", string(reason)),
                ("amount", string(amount)),
            ],
        )
        .await
    }
    pub(super) async fn advance_gold(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        l: &Listing,
    ) -> Result<(), Error> {
        match l.state {
            State::Active => {
                let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_bids WHERE listing_id=$1 AND phase IN ('pending','releasing'))").bind(l.id).fetch_one(&mut **tx).await?;
                if pending {
                    return Ok(());
                }
                // The worker selected this candidate before acquiring the listing row lock.
                let due: bool = sqlx::query_scalar("SELECT $2::timestamptz <= clock_timestamp() OR cancel_requested FROM bazaar_authority_assets WHERE listing_id=$1")
                    .bind(l.id).bind(l.ends_at).fetch_one(&mut **tx).await?;
                if !due {
                    return Ok(());
                }
                let Some(buyer) = l.leader else {
                    return self.return_unsold(tx, l).await;
                };
                let hold = current_hold(self.authority(), tx, l.id, buyer, None)
                    .await?
                    .ok_or(Error::Unavailable)?;
                if hold.amount < l.price as u64 {
                    return Err(Error::Denied);
                }
                for actor in [l.owner, buyer] {
                    if !self
                        .accounts(None)
                        .state(tx, actor)
                        .await
                        .map_err(core_error)?
                        .ok_or(Error::Denied)?
                        .allows_trade()
                    {
                        return Err(Error::Denied);
                    }
                }
                let (fee, net) = super::super::gold_payout(l.price as u64)?;
                let s = crate::Settlement {
                    id: oteryn_core::new_id(),
                    listing_id: l.id,
                    buyer,
                    seller: l.owner,
                    currency: crate::Currency::Gold,
                    amount: l.price,
                    commission: fee as i64,
                    seller_net: net as i64,
                    reservation: hold.id,
                };
                crate::lifecycle::write_gold_sale(tx, &s, hold.revision).await?;
                CoinReservations {
                    authority: self.authority().clone(),
                }
                .settle(tx, &s)
                .await
                .map_err(core_error)?;
                crate::lifecycle::write_listing(
                    tx,
                    l.id,
                    crate::lifecycle::Event::Listing(State::AwaitingDelivery),
                )
                .await
                .map_err(core_error)?;
            }
            State::AwaitingDelivery => {
                let s = sale(tx, l.id).await?;
                let revision: String = sqlx::query_scalar(
                    "SELECT hold_revision::text FROM bazaar_authority_gold_sales WHERE id=$1",
                )
                .bind(s.id)
                .fetch_one(&mut **tx)
                .await?;
                if l.kind == Kind::Character {
                    self.transfer(tx, l, "BAZAAR_SETTLEMENT", Some(&s)).await?;
                } else {
                    let Some((_, escrow)) = self.latest(tx, l.id, Command::Escrow).await? else {
                        return Err(Error::Unavailable);
                    };
                    let d = escrow.value.get("result_data").ok_or(Error::Invalid)?;
                    let destination:Uuid=sqlx::query_scalar("SELECT character_id FROM bazaar_authority_destinations WHERE listing_id=$1 AND buyer=$2").bind(l.id).bind(s.buyer).fetch_one(&mut **tx).await?;
                    let funding = Value::Object(vec![
                        ("currency".into(), string("GOLD")),
                        ("reservation_id".into(), string(s.reservation)),
                        ("hold_revision".into(), string(&revision)),
                        ("gross_amount".into(), string(s.amount)),
                    ]);
                    self.issue(
                        tx,
                        l,
                        l.owner,
                        None,
                        Command::Deliver,
                        s.id,
                        vec![
                            (
                                "escrow_id",
                                d.get("escrow_id").ok_or(Error::Invalid)?.clone(),
                            ),
                            (
                                "expected_escrow_revision",
                                d.get("escrow_revision").ok_or(Error::Invalid)?.clone(),
                            ),
                            ("buyer_account_id", string(s.buyer)),
                            ("buyer_character_id", string(destination)),
                            ("escrow_receipt_digest", string(escrow.digest())),
                            ("sale_id", string(s.id)),
                            ("funding", funding),
                        ],
                    )
                    .await?;
                }
                if !self
                    .authority()
                    .delivered(tx, &s)
                    .await
                    .map_err(core_error)?
                {
                    return Ok(());
                }
                let (operation, hash) = delivery(self.authority(), tx, l).await?;
                self.issue(
                    tx,
                    l,
                    l.owner,
                    None,
                    Command::Payout,
                    s.id,
                    vec![
                        ("buyer_account_id", string(s.buyer)),
                        ("hold_id", string(s.reservation)),
                        ("expected_hold_revision", string(&revision)),
                        ("sale_id", string(s.id)),
                        ("gross_amount", string(s.amount)),
                        ("fee_bps", Value::Unsigned(250)),
                        ("fee_amount", string(s.commission)),
                        ("seller_amount", string(s.seller_net)),
                        ("delivery_operation_id", string(operation)),
                        ("delivery_receipt_digest", string(hash)),
                    ],
                )
                .await?;
                let Some((_, payout)) = record(self.authority(), tx, l.id, Command::Payout).await?
                else {
                    return Ok(());
                };
                if !payout.committed {
                    return Err(Error::Denied);
                }
                let hold = current_hold(self.authority(), tx, l.id, s.buyer, None)
                    .await?
                    .ok_or(Error::Unavailable)?;
                if hold.amount > 0 {
                    self.gold_release(tx, l, s.buyer, hold.amount, "EXCESS", l.id)
                        .await?;
                    return Ok(());
                }
                CoinReservations {
                    authority: self.authority().clone(),
                }
                .complete(tx, &s)
                .await
                .map_err(core_error)?;
                crate::lifecycle::complete_gold_sale(tx, l.id, s.id)
                    .await
                    .map_err(core_error)?;
                crate::lifecycle::write_listing(
                    tx,
                    l.id,
                    crate::lifecycle::Event::Listing(State::Completed),
                )
                .await
                .map_err(core_error)?;
            }
            State::Completed | State::Cancelled if l.kind == Kind::Character => {
                self.unlock(tx, l).await?;
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) async fn gold_batch(pool: &PgPool) -> Result<Vec<Uuid>, Error> {
    let cursor = crate::scheduling::cursor(pool, "gold-bids").await?;
    let requests:Vec<Uuid>=sqlx::query_scalar("SELECT request_id FROM bazaar_authority_gold_bids WHERE phase IN ('pending','releasing') ORDER BY (request_id <= $1),request_id LIMIT 24").bind(cursor).fetch_all(pool).await?;
    if let Some(last) = requests.last() {
        crate::scheduling::advance(pool, "gold-bids", *last).await?;
    }
    Ok(requests)
}

#[cfg(test)]
mod tests {
    use crate::game::authority_tests::BrowserFixture;
    #[tokio::test]
    async fn r7_gold_worker_respects_extended_deadline() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = oteryn_db::test_support::global_lock_guard().await?;
        let db = BrowserFixture::new().await;
        sqlx::query(
            "UPDATE bazaar_listings SET currency='gold', ends_at=now()-interval '1 second'",
        )
        .execute(&db.pool)
        .await?;
        let selected = super::super::worker::listing_batch(&db.pool, 3).await?;
        assert_eq!(selected.len(), 1);
        sqlx::query("UPDATE bazaar_listings SET ends_at=now()+interval '5 minutes' WHERE id=$1")
            .bind(selected[0])
            .execute(&db.pool)
            .await?;
        let mut tx = db.pool.begin().await?;
        let listing = sqlx::query_as("SELECT * FROM bazaar_listings FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
        db.runtime.advance_gold(&mut tx, &listing).await?;
        let returns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bazaar_game_outbox WHERE command='ReturnBazaarItem'",
        )
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(returns, 0, "unexpired listing returned");
        tx.rollback().await?;
        db.end().await;
        Ok(())
    }
}
