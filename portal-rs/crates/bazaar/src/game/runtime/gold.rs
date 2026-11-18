//! Game bank truth only. Platform stores receipt references and commercial intent,
//! never a second gold balance or a spendable pending seller credit.
use super::*;
use crate::{Evidence, Reservation, Settlement};
#[derive(Debug)]
pub(crate) struct Hold {
    pub id: Uuid,
    pub offer: Uuid,
    pub revision: u64,
    pub amount: u64,
}
pub(crate) async fn current_hold(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
    buyer: Uuid,
    cutoff: Option<&str>,
) -> Result<Option<Hold>, Error> {
    // A zero-revision Hold opens an immutable lifecycle. Later revisions are
    // comparable only within that lifecycle, including when an old hold is RELEASED.
    // During replay, reconstruct only the prefix ending at the reviewed intent.
    let lifecycle = sqlx::query("SELECT source_revision::text AS revision,intent::jsonb->>'offer_id' AS offer FROM bazaar_game_outbox WHERE listing_id=$1 AND intent::jsonb->>'buyer_account_id'=$2 AND command='HoldBazaarGold' AND intent::jsonb->>'expected_hold_revision'='0' AND ($3::text IS NULL OR source_revision <= $3::text::numeric) ORDER BY source_revision DESC LIMIT 1")
        .bind(listing).bind(buyer.to_string()).bind(cutoff).fetch_optional(&mut *db).await?;
    let Some(lifecycle) = lifecycle else {
        return Ok(None);
    };
    let start: String = lifecycle.try_get("revision")?;
    let offer = id(lifecycle.try_get("offer")?)?;
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_game_outbox o LEFT JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id AND r.recovery_generation=$4::text::numeric AND r.source_authority=$5 WHERE o.listing_id=$1 AND o.intent::jsonb->>'buyer_account_id'=$2 AND o.source_revision >= $7::text::numeric AND o.command IN ('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold') AND ($3::text IS NULL OR o.source_revision < $3::text::numeric) AND (r.operation_id IS NULL OR NOT EXISTS(SELECT 1 FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id AND d.recovery_generation=r.recovery_generation AND d.source_authority=$6) OR EXISTS(SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id=o.operation_id)))").bind(listing).bind(buyer.to_string()).bind(cutoff).bind(a.fence.recovery_generation.to_string()).bind(&a.config.receipt_authority).bind(&a.config.projection_authority).bind(&start).fetch_one(&mut *db).await?;
    if pending {
        return Err(Error::Unavailable);
    }
    // Bound reads independently of a listing's bid history. Previous effects were
    // admitted under the same issuer lock; re-verify the newest two exact records
    // and reject duplicate Game revisions rather than imposing a lifetime bid cap.
    let operations:Vec<Uuid>=sqlx::query_scalar("SELECT o.operation_id FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r ON r.operation_id=o.operation_id WHERE o.listing_id=$1 AND o.intent::jsonb->>'buyer_account_id'=$2 AND o.source_revision >= $6::text::numeric AND o.command IN ('HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold') AND ($3::text IS NULL OR o.source_revision < $3::text::numeric) AND r.recovery_generation=$4::text::numeric AND r.source_authority=$5 AND r.outcome='COMMITTED' ORDER BY (r.receipt::jsonb->'result_data'->>'hold_revision')::numeric DESC LIMIT 2").bind(listing).bind(buyer.to_string()).bind(cutoff).bind(a.fence.recovery_generation.to_string()).bind(&a.config.receipt_authority).bind(&start).fetch_all(&mut *db).await?;
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
            offer,
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
) -> Result<(), Error> {
    if !a.fresh(db).await? {
        return Err(Error::Unavailable);
    }
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
    let hold = current_hold(a, db, id(text(i, "listing_id")?)?, buyer, None)
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
        Some(text(&i.value, "source_revision")?),
    )
    .await?;
    let expected = decimal(text(&i.value, "expected_hold_revision")?, true, u64::MAX)?;
    if before.as_ref().map_or(0, |h| h.revision) != expected {
        return Err(Error::Denied);
    }
    let hold_id = match facts {
        Some(data) => Some(id(text(data, "hold_id")?)?),
        None => before.as_ref().map(|hold| hold.id),
    };
    let fenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2 AND ($3::uuid IS NULL OR hold_id=$3))",
    )
    .bind(listing)
    .bind(buyer)
    .bind(hold_id)
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
        Some(text(&i.value, "source_revision")?),
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
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2 AND hold_id=$3)",
    )
    .bind(listing)
    .bind(buyer)
    .bind(hold.id)
    .fetch_one(&mut *db)
    .await?;
    if fenced {
        let listing_row: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(listing)
            .fetch_one(&mut *db)
            .await?;
        let compensation = text(&i.value, "release_reason")? == "TERMINAL_REFUSAL"
            && (a.compensation_ready(db, &listing_row).await?
                || (facts.is_some() && compensated_return(a, db, &listing_row).await?));
        if !compensation {
            let Some((_, payout)) = record(a, db, listing, Command::Payout).await? else {
                return Err(Error::Unavailable);
            };
            if !payout.committed || text(&i.value, "release_reason")? != "EXCESS" {
                return Err(Error::Denied);
            }
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
async fn compensated_return(
    a: &Authority,
    db: &mut PgConnection,
    listing: &Listing,
) -> Result<bool, Error> {
    let operation: Option<Uuid> = sqlx::query_scalar(
        "SELECT return_operation_id FROM bazaar_authority_compensations WHERE listing_id=$1",
    )
    .bind(listing.id)
    .fetch_optional(&mut *db)
    .await?;
    let Some(operation) = operation else {
        return Ok(false);
    };
    if a.retained(db, operation)
        .await?
        .is_some_and(|(_, receipt)| receipt.committed)
    {
        return Ok(true);
    }
    match listing.kind {
        Kind::Item => Ok(record(a, db, listing.id, Command::Deliver)
            .await?
            .is_some_and(|(_, r)| !r.committed)),
        Kind::Character => Ok(a.settlement_bind_refused(db, listing.id).await?.is_some()),
    }
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
    let recovering = facts.is_some() && !a.resynced(db).await?;
    if !recovering && !a.delivered(db, &s).await.map_err(core_error)? {
        return Err(Error::Unavailable);
    }
    let hold = current_hold(
        a,
        db,
        listing,
        s.buyer,
        Some(text(&i.value, "source_revision")?),
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
    let expected_operation = id(text(&i.value, "delivery_operation_id")?)?;
    let expected_hash = text(&i.value, "delivery_receipt_digest")?;
    if recovering && l.kind == Kind::Item {
        let r = a.prerequisite(db, expected_hash).await?;
        let Some((intent, current)) = a.retained(db, expected_operation).await? else {
            return Err(Error::Unavailable);
        };
        if current.digest() != r.digest()
            || !current.committed
            || current.command != Command::Deliver
            || text(&intent.value, "sale_id")? != s.id.to_string()
            || text(&intent.value, "listing_id")? != listing.to_string()
        {
            return Err(Error::Denied);
        }
    } else if recovering {
        let Some((bind, r)) = record(a, db, listing, Command::Bind).await? else {
            return Err(Error::Unavailable);
        };
        if !r.committed
            || text(&bind.value, "purpose")? != "BAZAAR_SETTLEMENT"
            || text(&bind.value, "sale_id")? != s.id.to_string()
        {
            return Err(Error::Denied);
        }
        let transfer = id(text(&bind.value, "transfer_operation_id")?)?;
        let raw: String = sqlx::query_scalar(
            "SELECT receipt_json FROM character_commands WHERE id=$1 AND state='TERMINAL'",
        )
        .bind(transfer)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::Unavailable)?;
        if transfer != expected_operation || digest(raw.as_bytes()) != expected_hash {
            return Err(Error::Denied);
        }
    } else {
        let (operation, hash) = delivery(a, db, &l).await?;
        if operation != expected_operation || hash != expected_hash {
            return Err(Error::Denied);
        }
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

pub(crate) fn hold_binding(_listing: Uuid, hold: Option<&Hold>, request: Uuid) -> (Uuid, u64) {
    match hold {
        Some(hold) if hold.amount > 0 => (hold.offer, hold.revision),
        _ => (request, 0),
    }
}
