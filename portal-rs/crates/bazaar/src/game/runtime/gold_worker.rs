use super::gold::{current_hold, delivery, record, sale};
use super::*;
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
        if check_bid_request(tx, key, l.id, buyer, character, maximum).await? {
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
                    ("hold_purpose", string("RESERVE")),
                    ("amount", string(maximum)),
                    ("expected_hold_revision", string(expected_revision)),
                ],
            )
            .await?;
        sqlx::query("INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$2,$3,$4,$5,$6)").bind(key).bind(l.id).bind(buyer).bind(character).bind(maximum).bind(operation).execute(&mut **tx).await?;
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
        self.authority().lock(&mut tx).await?;
        let row =
            sqlx::query("SELECT * FROM bazaar_authority_gold_bids WHERE request_id=$1 FOR UPDATE")
                .bind(request)
                .fetch_one(&mut *tx)
                .await?;
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
            sqlx::query(
                "UPDATE bazaar_authority_gold_bids SET phase=CASE WHEN release_accepted THEN 'accepted' ELSE 'refused' END WHERE request_id=$1",
            )
            .bind(request)
            .execute(&mut *tx)
            .await?;
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
            sqlx::query(
                "UPDATE bazaar_authority_gold_bids SET phase='refused' WHERE request_id=$1",
            )
            .bind(request)
            .execute(&mut *tx)
            .await?;
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
        let valid = l.ends_at.unix_timestamp() > now(&mut tx).await? as i64
            && actor.allows_trade()
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
            let ends = if l.ends_at.unix_timestamp() - (now(&mut tx).await? as i64) < 300 {
                l.ends_at + Duration::minutes(5)
            } else {
                l.ends_at
            };
            sqlx::query("UPDATE bazaar_listings SET leader=$2,leader_max=$3,price=$4,ends_at=$5,reservation=id,updated_at=now() WHERE id=$1").bind(l.id).bind(leader).bind(leader_max).bind(price).bind(ends).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(oteryn_core::new_id()).bind(l.id).bind(buyer).bind(maximum).bind(request.to_string()).bind(leader).bind(price).bind(ends).execute(&mut *tx).await?;
            if leader == buyer {
                sqlx::query("INSERT INTO bazaar_authority_destinations(listing_id,buyer,character_id) VALUES($1,$2,$3) ON CONFLICT(listing_id,buyer) DO UPDATE SET character_id=EXCLUDED.character_id").bind(l.id).bind(buyer).bind(row.try_get::<Uuid,_>("character_id")?).execute(&mut *tx).await?;
            }
            sqlx::query(
                "UPDATE bazaar_authority_gold_bids SET phase=CASE WHEN $2::uuid IS NULL THEN 'accepted' ELSE 'releasing' END,release_operation_id=$2,release_accepted=true WHERE request_id=$1",
            )
            .bind(request).bind(release)
            .execute(&mut *tx)
            .await?;
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
                sqlx::query(
                    "UPDATE bazaar_authority_gold_bids SET phase='refused' WHERE request_id=$1",
                )
                .bind(request)
                .execute(&mut *tx)
                .await?;
            } else {
                let operation = self
                    .gold_release(&mut tx, &l, buyer, hold.amount - keep, "EXCESS", request)
                    .await?;
                sqlx::query("UPDATE bazaar_authority_gold_bids SET phase='releasing',release_operation_id=$2 WHERE request_id=$1").bind(request).bind(operation).execute(&mut *tx).await?;
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
                let id = oteryn_core::new_id();
                sqlx::query("INSERT INTO bazaar_authority_gold_sales(id,listing_id,buyer,seller,gross_amount,fee_amount,seller_amount,hold_id,hold_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric)").bind(id).bind(l.id).bind(buyer).bind(l.owner).bind(l.price).bind(fee as i64).bind(net as i64).bind(hold.id).bind(hold.revision.to_string()).execute(&mut **tx).await?;
                let s = sale(tx, l.id).await?;
                CoinReservations {
                    authority: self.authority().clone(),
                }
                .settle(tx, &s)
                .await
                .map_err(core_error)?;
                sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery',updated_at=now() WHERE id=$1").bind(l.id).execute(&mut **tx).await?;
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
                sqlx::query(
                    "UPDATE bazaar_authority_gold_sales SET completed_at=now() WHERE id=$1",
                )
                .bind(s.id)
                .execute(&mut **tx)
                .await?;
                sqlx::query(
                    "UPDATE bazaar_listings SET state='completed',updated_at=now() WHERE id=$1",
                )
                .bind(l.id)
                .execute(&mut **tx)
                .await?;
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
