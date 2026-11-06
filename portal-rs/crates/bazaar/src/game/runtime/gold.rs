//! Game bank truth only. Platform stores receipt references and commercial intent,
//! never a second gold balance or a spendable pending seller credit.
use super::*;
use crate::{Evidence, Reservation, Settlement};
#[derive(Debug)]
pub(crate) struct Hold {
    pub id: Uuid,
    pub revision: u64,
    pub amount: u64,
}
pub(crate) async fn current_hold(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
    buyer: Uuid,
    exclude: Option<Uuid>,
) -> Result<Option<Hold>, Error> {
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox o LEFT JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id AND r.recovery_generation=$4::text::numeric AND r.source_authority=$5 WHERE o.listing_id=$1 AND o.intent::jsonb->>'buyer_account_id'=$2 AND o.command IN ('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold') AND ($3::uuid IS NULL OR o.operation_id<>$3) AND (r.operation_id IS NULL OR NOT EXISTS(SELECT 1 FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id AND d.recovery_generation=r.recovery_generation AND d.source_authority=$6) OR EXISTS(SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id=o.operation_id)))").bind(listing).bind(buyer.to_string()).bind(exclude).bind(a.fence.recovery_generation.to_string()).bind(&a.config.receipt_authority).bind(&a.config.projection_authority).fetch_one(&mut *db).await?;
    if pending {
        return Err(Error::Unavailable);
    }
    // Bound reads independently of a listing's bid history. Previous effects were
    // admitted under the same issuer lock; re-verify the newest two exact records
    // and reject duplicate Game revisions rather than imposing a lifetime bid cap.
    let operations:Vec<Uuid>=sqlx::query_scalar("SELECT o.operation_id FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id WHERE o.listing_id=$1 AND o.intent::jsonb->>'buyer_account_id'=$2 AND o.command IN ('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold') AND ($3::uuid IS NULL OR o.operation_id<>$3) AND r.recovery_generation=$4::text::numeric AND r.source_authority=$5 AND r.outcome='COMMITTED' ORDER BY (r.receipt::jsonb->'result_data'->>'hold_revision')::numeric DESC LIMIT 2").bind(listing).bind(buyer.to_string()).bind(exclude).bind(a.fence.recovery_generation.to_string()).bind(&a.config.receipt_authority).fetch_all(&mut *db).await?;
    let mut current: Option<Hold> = None;
    for operation in operations {
        let Some((i, r)) = a.retained(db, operation).await? else {
            return Err(Error::Unavailable);
        };
        if !r.committed {
            continue;
        }
        if id(text(&i.value, "buyer_account_id")?)? != buyer {
            return Err(Error::Denied);
        }
        let d = r.value.get("result_data").ok_or(Error::Invalid)?;
        let hold = Hold {
            id: id(text(d, "hold_id")?)?,
            revision: number(d, "hold_revision")?,
            amount: decimal(
                text(
                    d,
                    if r.command == Command::Hold {
                        "held_amount"
                    } else {
                        "remaining_held"
                    },
                )?,
                true,
                999_999_999_999,
            )?,
        };
        if let Some(old) = &current {
            if old.id != hold.id || old.revision == hold.revision {
                return Err(Error::Conflict);
            }
            if old.revision > hold.revision {
                continue;
            }
        }
        current = Some(hold);
    }
    Ok(current)
}
pub(crate) async fn reserve(
    a: &Authority,
    db: &mut PgConnection,
    h: Reservation,
) -> Result<(), Error> {
    a.lock(db).await?;
    if !a.config.gold_fee_and_headroom_accepted || !a.fresh(db).await? {
        return Err(Error::Unavailable);
    }
    let hold = current_hold(a, db, h.id, h.account, None)
        .await?
        .ok_or(Error::Unavailable)?;
    if h.amount <= 0 || hold.amount != h.amount as u64 {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn funded(
    a: &Authority,
    db: &mut PgConnection,
    i: &Value,
    buyer: Uuid,
    exclude: Option<Uuid>,
) -> Result<(), Error> {
    let f = i.get("funding").ok_or(Error::Invalid)?;
    let row=sqlx::query("SELECT hold_id,hold_revision::text AS revision,gross_amount,seller,buyer FROM bazaar_authority_gold_sales WHERE id=$1 AND listing_id=$2 AND completed_at IS NULL").bind(id(text(i,"sale_id")?)?).bind(id(text(i,"listing_id")?)?).fetch_optional(&mut *db).await?.ok_or(Error::Unavailable)?;
    if row.try_get::<Uuid, _>("buyer")? != buyer
        || row.try_get::<Uuid, _>("seller")? != id(text(i, "seller_account_id")?)?
        || row.try_get::<Uuid, _>("hold_id")? != id(text(f, "reservation_id")?)?
        || row.try_get::<String, _>("revision")? != text(f, "hold_revision")?
        || row.try_get::<i64, _>("gross_amount")? as u64 != number(f, "gross_amount")?
    {
        return Err(Error::Denied);
    }
    let hold = current_hold(a, db, id(text(i, "listing_id")?)?, buyer, exclude)
        .await?
        .ok_or(Error::Unavailable)?;
    if hold.id != row.try_get::<Uuid, _>("hold_id")?
        || hold.revision != decimal(row.try_get("revision")?, false, u64::MAX)?
        || hold.amount < number(f, "gross_amount")?
    {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn sale(db: &mut PgConnection, listing: Uuid) -> Result<Settlement, Error> {
    sqlx::query_as("SELECT id,listing_id,buyer,seller,'gold'::bazaar_currency AS currency,gross_amount AS amount,fee_amount AS commission,seller_amount AS seller_net,hold_id AS reservation FROM bazaar_authority_gold_sales WHERE listing_id=$1").bind(listing).fetch_optional(db).await?.ok_or(Error::Unavailable)
}
pub(crate) async fn record(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
    command: Command,
) -> Result<Option<(PreparedIntent, super::super::Receipt)>, Error> {
    let op:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command=$2 ORDER BY source_revision DESC LIMIT 1").bind(listing).bind(command.wire()).fetch_optional(&mut *db).await?;
    match op {
        Some(op) => a.retained(db, op).await,
        None => Ok(None),
    }
}
pub(crate) async fn validate_hold(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
    facts: Option<&Value>,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let buyer = id(text(&i.value, "buyer_account_id")?)?;
    let before = current_hold(
        a,
        db,
        listing,
        buyer,
        Some(id(text(&i.value, "operation_id")?)?),
    )
    .await?;
    let expected = decimal(text(&i.value, "expected_hold_revision")?, true, u64::MAX)?;
    if before.as_ref().map_or(0, |h| h.revision) != expected {
        return Err(Error::Denied);
    }
    let fenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2)",
    )
    .bind(listing)
    .bind(buyer)
    .fetch_one(&mut *db)
    .await?;
    if fenced {
        return Err(Error::Denied);
    }
    if let (Some(before), Some(facts)) = (before, facts)
        && id(text(facts, "hold_id")?)? != before.id
    {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn validate_release(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
    facts: Option<&Value>,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let buyer = id(text(&i.value, "buyer_account_id")?)?;
    let hold = current_hold(
        a,
        db,
        listing,
        buyer,
        Some(id(text(&i.value, "operation_id")?)?),
    )
    .await?
    .ok_or(Error::Unavailable)?;
    if hold.id != id(text(&i.value, "hold_id")?)?
        || hold.revision != number(&i.value, "expected_hold_revision")?
        || hold.amount < number(&i.value, "amount")?
    {
        return Err(Error::Denied);
    }
    let fenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2)",
    )
    .bind(listing)
    .bind(buyer)
    .fetch_one(&mut *db)
    .await?;
    if fenced {
        let Some((_, payout)) = record(a, db, listing, Command::Payout).await? else {
            return Err(Error::Unavailable);
        };
        if !payout.committed || text(&i.value, "release_reason")? != "EXCESS" {
            return Err(Error::Denied);
        }
    }
    if let Some(facts) = facts
        && decimal(text(facts, "remaining_held")?, true, 999_999_999_999)?
            != hold.amount - number(&i.value, "amount")?
    {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn validate_payout(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
    facts: Option<&Value>,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let s = sale(db, listing).await?;
    if s.id != id(text(&i.value, "sale_id")?)?
        || s.buyer != id(text(&i.value, "buyer_account_id")?)?
        || s.seller != id(text(&i.value, "seller_account_id")?)?
        || s.reservation != id(text(&i.value, "hold_id")?)?
        || s.amount as u64 != number(&i.value, "gross_amount")?
        || s.commission as u64 != number(&i.value, "fee_amount")?
        || s.seller_net as u64 != number(&i.value, "seller_amount")?
    {
        return Err(Error::Denied);
    }
    if !a.delivered(db, &s).await.map_err(core_error)? {
        return Err(Error::Unavailable);
    }
    let hold = current_hold(
        a,
        db,
        listing,
        s.buyer,
        Some(id(text(&i.value, "operation_id")?)?),
    )
    .await?
    .ok_or(Error::Unavailable)?;
    if hold.id != s.reservation
        || hold.revision != number(&i.value, "expected_hold_revision")?
        || hold.amount < s.amount as u64
    {
        return Err(Error::Denied);
    }
    let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&mut *db)
        .await?;
    let (operation, hash) = delivery(a, db, &l).await?;
    if operation != id(text(&i.value, "delivery_operation_id")?)?
        || hash != text(&i.value, "delivery_receipt_digest")?
    {
        return Err(Error::Denied);
    }
    if let Some(facts) = facts
        && decimal(text(facts, "remaining_held")?, true, 999_999_999_999)?
            != hold.amount - s.amount as u64
    {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn delivery(
    a: &Authority,
    db: &mut PgConnection,
    l: &Listing,
) -> Result<(Uuid, String), Error> {
    if l.kind == Kind::Character {
        let Some((_, receipt, _, _)) =
            super::characters::proof(a, db, l.id, "BAZAAR_SETTLEMENT").await?
        else {
            return Err(Error::Unavailable);
        };
        return Ok((
            id(text(&receipt, "operation_id")?)?,
            digest(&canonical::encode(&receipt).map_err(|_| Error::Invalid)?),
        ));
    }
    let Some((_, receipt)) = record(a, db, l.id, Command::Deliver).await? else {
        return Err(Error::Unavailable);
    };
    if !receipt.committed {
        return Err(Error::Unavailable);
    }
    Ok((receipt.operation, receipt.digest()))
}
pub(crate) async fn settled(
    a: &Authority,
    db: &mut PgConnection,
    s: &Settlement,
    complete: bool,
) -> Result<(), Error> {
    a.lock(db).await?;
    let actual = sale(db, s.listing_id).await?;
    if actual != *s {
        return Err(Error::Denied);
    }
    let (fee, net) = super::super::gold_payout(s.amount as u64)?;
    if s.commission as u64 != fee || s.seller_net as u64 != net {
        return Err(Error::Denied);
    }
    if complete {
        if !a.delivered(db, s).await.map_err(core_error)? {
            return Err(Error::Unavailable);
        }
        let Some((i, r)) = record(a, db, s.listing_id, Command::Payout).await? else {
            return Err(Error::Unavailable);
        };
        if !r.committed || id(text(&i.value, "sale_id")?)? != s.id {
            return Err(Error::Denied);
        }
        let hold = current_hold(a, db, s.listing_id, s.buyer, None)
            .await?
            .ok_or(Error::Unavailable)?;
        if hold.id != s.reservation || hold.amount != 0 {
            return Err(Error::Unavailable);
        }
    } else {
        let hold = current_hold(a, db, s.listing_id, s.buyer, None)
            .await?
            .ok_or(Error::Unavailable)?;
        if hold.id != s.reservation || hold.amount < s.amount as u64 {
            return Err(Error::Denied);
        }
    }
    Ok(())
}
