//! CC v1 ownership legs remain byte-identical and are published only after a
//! committed v2 digest binding. No general locked-state bypass is introduced.
use super::super::game_wire::ordered;
use super::super::{Receipt, SnapshotBinding, verify_snapshot};
use super::*;
use crate::characters::{CharacterChoice, CharacterEvidence, SaleSnapshot, Skill};
use crate::{Evidence, Settlement};

pub(crate) async fn lock_record(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
) -> Result<Option<(PreparedIntent, Receipt, Value)>, Error> {
    let operation:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='LockBazaarCharacter' ORDER BY source_revision DESC LIMIT 1").bind(listing).fetch_optional(&mut *db).await?;
    let Some(operation) = operation else {
        return Ok(None);
    };
    let Some((i, r)) = a.retained(db, operation).await? else {
        return Ok(None);
    };
    if !r.committed || r.command != Command::Lock {
        return Ok(None);
    }
    let d = r.value.get("result_data").ok_or(Error::Invalid)?;
    let signed = d.get("signed_snapshot").ok_or(Error::Invalid)?;
    let kid = text(signed, "kid")?;
    let key = &a
        .config
        .snapshot_keys
        .iter()
        .find(|(key, _)| key == kid)
        .ok_or(Error::Unavailable)?
        .1;
    let e = SnapshotBinding {
        source_authority: &a.config.projection_authority,
        listing,
        seller: id(text(&i.value, "seller_account_id")?)?,
        character: id(text(&i.value, "character_id")?)?,
        world: id(text(&i.value, "world_id")?)?,
        lock: id(text(d, "lock_id")?)?,
        lock_revision: number(d, "lock_revision")?,
        character_revision: number(&i.value, "expected_character_revision")?,
        eligibility_revision: number(&i.value, "eligibility_revision")?,
        projection_epoch: a.fence.projection_epoch,
        now: now(db).await?,
    };
    verify_snapshot(
        &canonical::encode(signed).map_err(|_| Error::Invalid)?,
        &e,
        kid,
        key,
    )?;
    let signed = signed.clone();
    Ok(Some((i, r, signed)))
}
pub(crate) async fn staged(
    a: &Authority,
    db: &mut PgConnection,
    operation: Uuid,
) -> Result<Value, Error> {
    let row=sqlx::query("SELECT intent,intent_digest,projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_authority_transfers WHERE operation_id=$1").bind(operation).fetch_optional(&mut *db).await?.ok_or(Error::Unavailable)?;
    let raw: String = row.try_get("intent")?;
    if digest(raw.as_bytes()) != row.try_get::<String, _>("intent_digest")?
        || decimal(row.try_get("epoch")?, false, u64::MAX)? != a.fence.projection_epoch
        || decimal(row.try_get("generation")?, false, u64::MAX)? != a.fence.recovery_generation
    {
        return Err(Error::Denied);
    }
    let v = decode(raw.as_bytes(), 4096)?;
    ordered(
        &v,
        &[
            "contract_version",
            "command",
            "variant",
            "issuer_authority",
            "issuer_decision_id",
            "source_revision",
            "operation_id",
            "character_id",
            "expected_from_account_id",
            "to_account_id",
            "purpose",
            "issued_at_source",
            "expires_at_source",
            "audience",
        ],
    )?;
    if v.get("contract_version") != Some(&Value::Unsigned(1))
        || text(&v, "command")? != "TransferCharacterOwnership"
        || text(&v, "variant")? != "PLATFORM_BAZAAR_TRANSFER"
        || text(&v, "issuer_authority")? != "OTERYN_PLATFORM_CHARACTER_AUTHORITY"
        || text(&v, "audience")? != "OTERYN_GAME_CHARACTER_AUTHORITY"
        || id(text(&v, "operation_id")?)? != operation
    {
        return Err(Error::Denied);
    }
    for key in [
        "issuer_decision_id",
        "character_id",
        "expected_from_account_id",
        "to_account_id",
    ] {
        id(text(&v, key)?)?;
    }
    for key in ["source_revision", "issued_at_source", "expires_at_source"] {
        number(&v, key)?;
    }
    if number(&v, "expires_at_source")? <= number(&v, "issued_at_source")?
        || number(&v, "expires_at_source")? - number(&v, "issued_at_source")? > 300
    {
        return Err(Error::Invalid);
    }
    Ok(v)
}
pub(crate) async fn validate_bind(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let transfer = id(text(&i.value, "transfer_operation_id")?)?;
    let v = staged(a, db, transfer).await?;
    let raw = canonical::encode(&v).map_err(|_| Error::Invalid)?;
    if digest(&raw) != text(&i.value, "transfer_binding_digest")? {
        return Err(Error::Denied);
    }
    for key in ["character_id", "purpose"] {
        same(&v, key, &i.value, key)?;
    }
    let Some((lock, receipt, _)) = lock_record(a, db, listing).await? else {
        return Err(Error::Unavailable);
    };
    for key in ["character_id", "seller_account_id", "world_id"] {
        same(&lock.value, key, &i.value, key)?;
    }
    same(
        receipt.value.get("result_data").ok_or(Error::Invalid)?,
        "lock_id",
        &i.value,
        "lock_id",
    )?;
    let escrow = a.config.escrow_account_id.ok_or(Error::Unavailable)?;
    let seller = id(text(&i.value, "seller_account_id")?)?;
    let (from, to) = match text(&v, "purpose")? {
        "BAZAAR_LISTING_ESCROW" => (seller, escrow),
        "BAZAAR_ESCROW_RETURN" => (escrow, seller),
        "BAZAAR_SETTLEMENT" => {
            let sale = id(text(&i.value, "sale_id")?)?;
            let buyer: Uuid = sqlx::query_scalar(
                "SELECT buyer FROM bazaar_settlements WHERE id=$1 AND listing_id=$2 AND seller=$3 UNION ALL SELECT buyer FROM bazaar_authority_gold_sales WHERE id=$1 AND listing_id=$2 AND seller=$3",
            )
            .bind(sale)
            .bind(listing)
            .bind(seller)
            .fetch_optional(&mut *db)
            .await?
            .ok_or(Error::Unavailable)?;
            funded(a, db, &i.value, buyer).await?;
            (escrow, buyer)
        }
        _ => return Err(Error::Invalid),
    };
    if id(text(&v, "expected_from_account_id")?)? != from || id(text(&v, "to_account_id")?)? != to {
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
    let funding = i.get("funding").ok_or(Error::Invalid)?;
    if text(funding, "currency")? == "GOLD" {
        return super::gold::funded(a, db, i, buyer, None).await;
    }
    if text(funding, "currency")? != "COINS" {
        return Err(Error::Invalid);
    }
    let fenced:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_coin_holds h JOIN bazaar_settlements s ON s.id=h.settlement_id JOIN bazaar_coin_dispositions d ON d.settlement_id=s.id WHERE h.id=$1 AND h.account_id=$2 AND s.id=$3 AND s.listing_id=$4 AND s.seller=$5 AND s.buyer=$2 AND s.amount=$6 AND h.amount>=$6 AND NOT h.completed AND NOT d.completed)")
        .bind(id(text(funding,"reservation_id")?)?).bind(buyer).bind(id(text(i,"sale_id")?)?).bind(id(text(i,"listing_id")?)?).bind(id(text(i,"seller_account_id")?)?).bind(i64::try_from(number(funding,"gross_amount")?).map_err(|_|Error::Invalid)?).fetch_one(db).await?;
    if !fenced {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(crate) async fn proof(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
    purpose: &str,
) -> Result<Option<(Value, Value, PreparedIntent, Receipt)>, Error> {
    let row=sqlx::query("SELECT t.operation_id,t.intent,t.intent_digest,c.intent_json,c.receipt_json FROM bazaar_authority_transfers t JOIN character_commands c ON c.id=t.operation_id WHERE t.listing_id=$1 AND t.purpose=$2 AND c.state='TERMINAL'").bind(listing).bind(purpose).fetch_optional(&mut *db).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let operation: Uuid = row.try_get("operation_id")?;
    let intent = staged(a, db, operation).await?;
    let raw: String = row.try_get("intent")?;
    if raw != row.try_get::<String, _>("intent_json")?
        || digest(raw.as_bytes()) != row.try_get::<String, _>("intent_digest")?
    {
        return Err(Error::Denied);
    }
    let receipt: String = row.try_get("receipt_json")?;
    let r = decode(receipt.as_bytes(), 1024)?;
    ordered(
        &r,
        &[
            "contract_version",
            "operation",
            "source_authority",
            "command",
            "operation_id",
            "outcome",
            "result_code",
            "character_id",
            "from_account_id",
            "to_account_id",
            "character_revision",
            "projection_epoch",
            "from_projection_revision",
            "to_projection_revision",
            "recovery_generation",
            "decided_at",
        ],
    )?;
    if r.get("contract_version") != Some(&Value::Unsigned(1))
        || text(&r, "operation")? != "PublishCharacterCommandReceiptV1"
        || text(&r, "source_authority")? != a.config.receipt_authority
        || text(&r, "command")? != "TransferCharacterOwnership"
        || text(&r, "outcome")? != "COMMITTED"
        || text(&r, "result_code")? != "CHAR_CMD_OK"
        || number(&r, "recovery_generation")? != a.fence.recovery_generation
        || number(&r, "projection_epoch")? != a.fence.projection_epoch
    {
        return Err(Error::Denied);
    }
    for (rk, ik) in [
        ("operation_id", "operation_id"),
        ("character_id", "character_id"),
        ("from_account_id", "expected_from_account_id"),
        ("to_account_id", "to_account_id"),
    ] {
        same(&r, rk, &intent, ik)?;
    }
    for key in [
        "character_revision",
        "from_projection_revision",
        "to_projection_revision",
    ] {
        number(&r, key)?;
    }
    if number(&r, "decided_at")? > now(db).await?.saturating_add(1) {
        return Err(Error::Denied);
    }
    let bind:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer' AND intent::jsonb->>'transfer_operation_id'=$2 ORDER BY source_revision DESC LIMIT 1").bind(listing).bind(operation.to_string()).fetch_optional(&mut *db).await?;
    let Some(bind) = bind else {
        return Ok(None);
    };
    let Some((i, b)) = a.retained(db, bind).await? else {
        return Ok(None);
    };
    if !b.committed
        || text(&i.value, "transfer_binding_digest")? != digest(raw.as_bytes())
        || text(&i.value, "purpose")? != purpose
    {
        return Err(Error::Denied);
    }
    // Current funding can already be captured on an exact completion replay;
    // immutable bind intent + its terminal Game receipt remain the sale proof.
    Ok(Some((intent, r, i, b)))
}
pub(crate) async fn custody(
    a: &Authority,
    db: &mut PgConnection,
    l: &Listing,
    s: &Value,
    state: &str,
) -> Result<Option<(PreparedIntent, Receipt, Value)>, Error> {
    let purpose = match state {
        "locked" => "BAZAAR_LISTING_ESCROW",
        "DELIVERED" => "BAZAAR_SETTLEMENT",
        "RETURNED" => "BAZAAR_ESCROW_RETURN",
        _ => return Ok(None),
    };
    let Some((_, transfer, bind, bind_receipt)) = proof(a, db, l.id, purpose).await? else {
        return Ok(None);
    };
    let Some((lock, lock_receipt, signed)) = lock_record(a, db, l.id).await? else {
        return Ok(None);
    };
    let d = lock_receipt
        .value
        .get("result_data")
        .ok_or(Error::Invalid)?;
    same(s, "world_id", &lock.value, "world_id")?;
    same(s, "seller_account_id", &lock.value, "seller_account_id")?;
    if text(s, "asset_kind")? != "CHARACTER" {
        return Err(Error::Denied);
    }
    same(s, "asset_id", &lock.value, "character_id")?;
    same(s, "custody_id", d, "lock_id")?;
    same(s, "snapshot_hash", &signed, "snapshot_hash")?;
    same(s, "last_operation_id", &transfer, "operation_id")?;
    if number(s, "custody_revision")?
        < number(
            bind_receipt
                .value
                .get("result_data")
                .ok_or(Error::Invalid)?,
            "lock_revision",
        )?
    {
        return Err(Error::Denied);
    }
    Ok(Some((bind, bind_receipt, signed)))
}
pub(crate) async fn delivered(
    a: &Authority,
    db: &mut PgConnection,
    l: &Listing,
    sale: &Settlement,
) -> Result<bool, Error> {
    if !a.fresh(db).await? {
        return Ok(false);
    }
    let Some(status) = a.projection(db, "STATUS", l.id).await? else {
        return Ok(false);
    };
    if text(&status, "state")? != "DELIVERED"
        || id(text(&status, "seller_account_id")?)? != sale.seller
    {
        return Ok(false);
    }
    let Some((bind, _, _)) = custody(a, db, l, &status, "DELIVERED").await? else {
        return Ok(false);
    };
    let Some((_, receipt, _, _)) = proof(a, db, l.id, "BAZAAR_SETTLEMENT").await? else {
        return Ok(false);
    };
    let funding = bind.value.get("funding").ok_or(Error::Invalid)?;
    Ok(id(text(&receipt, "to_account_id")?)? == sale.buyer
        && id(text(&bind.value, "sale_id")?)? == sale.id
        && id(text(funding, "reservation_id")?)? == sale.reservation
        && text(funding, "currency")?
            == if sale.currency == Currency::Gold {
                "GOLD"
            } else {
                "COINS"
            }
        && number(funding, "gross_amount")? == sale.amount as u64)
}
impl CharacterEvidence for Authority {
    async fn choices(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
    ) -> Result<Vec<CharacterChoice>, crate::Error> {
        if !self.fresh(db).await.map_err(to_core)? {
            return Ok(vec![]);
        }
        // Selection is advisory; lock admission still proves ownership, obligations
        // and cooldown. No portfolio row alone certifies sale eligibility.
        Ok(sqlx::query_as("SELECT c.character_id AS id,c.name FROM account_characters c JOIN account_character_snapshots s ON s.account_id=c.account_id WHERE c.account_id=$1 AND c.availability='AVAILABLE' AND NOT s.invalid AND s.projection_epoch=$2 ORDER BY c.character_id LIMIT 50").bind(actor).bind(self.fence.projection_epoch.to_string()).fetch_all(db).await?)
    }
    async fn approved(
        &self,
        db: &mut PgConnection,
        l: &Listing,
    ) -> Result<Option<SaleSnapshot>, crate::Error> {
        if !self.escrow_ready(db, l).await? {
            return Ok(None);
        }
        let Some((_, _, signed)) = lock_record(self, db, l.id).await.map_err(to_core)? else {
            return Ok(None);
        };
        let s = signed.get("snapshot").ok_or(crate::Error::Denied)?;
        let get = |key: &str| -> Result<u64, crate::Error> {
            decimal(text(s, key).map_err(to_core)?, true, u64::MAX).map_err(to_core)
        };
        let Some(Value::Array(skills)) = s.get("skills") else {
            return Err(crate::Error::Denied);
        };
        let skills = skills
            .iter()
            .map(|v| {
                Ok(Skill {
                    key: text(v, "key").map_err(to_core)?.into(),
                    level: number(v, "level").map_err(to_core)?,
                    progress_bps: match v.get("progress_bps") {
                        Some(Value::Unsigned(n)) => *n as u16,
                        _ => return Err(crate::Error::Denied),
                    },
                })
            })
            .collect::<Result<Vec<_>, crate::Error>>()?;
        let snapshot = SaleSnapshot {
            name: text(s, "name").map_err(to_core)?.into(),
            world: text(s, "world_id").map_err(to_core)?.into(),
            vocation: match s.get("vocation") {
                Some(Value::Unsigned(n)) => *n as u8,
                _ => return Err(crate::Error::Denied),
            },
            level: get("level")?,
            captured_at: get("captured_at")?,
            snapshot_hash: text(&signed, "snapshot_hash").map_err(to_core)?.into(),
            health: get("health")?,
            health_max: get("health_max")?,
            mana: get("mana")?,
            mana_max: get("mana_max")?,
            capacity_centi_oz: get("capacity_centi_oz")?,
            speed: get("speed")?,
            experience: get("experience")?,
            created_at: if s.get("created_at") == Some(&Value::Null) {
                None
            } else {
                Some(get("created_at")?)
            },
            blessings: vec![],
            world_transfer_eligible: match s.get("world_transfer_eligible") {
                Some(Value::Bool(v)) => Some(*v),
                _ => None,
            },
            skills,
            items: None,
            store_grants: None,
            outfits: None,
            mounts: None,
            achievements: None,
            quests: None,
        };
        snapshot.validate()?;
        Ok(Some(snapshot))
    }
}
fn to_core(e: Error) -> crate::Error {
    match e {
        Error::Database(e) => crate::Error::Database(e),
        _ => crate::Error::Denied,
    }
}

pub(crate) async fn verify_unlock(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
    current: Option<&Value>,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_transfers t LEFT JOIN character_commands c ON c.id=t.operation_id WHERE t.listing_id=$1 AND (c.id IS NULL OR c.state='PENDING'))").bind(listing).fetch_one(&mut *db).await?;
    if pending {
        return Err(Error::Unavailable);
    }
    let owner = if let Some((_, receipt, _, _)) = proof(a, db, listing, "BAZAAR_SETTLEMENT").await?
    {
        id(text(&receipt, "to_account_id")?)?
    } else if let Some((_, receipt, _, _)) = proof(a, db, listing, "BAZAAR_ESCROW_RETURN").await? {
        id(text(&receipt, "to_account_id")?)?
    } else {
        return Err(Error::Unavailable);
    };
    if let Some(facts) = current
        && id(text(facts, "current_account_id")?)? != owner
    {
        return Err(Error::Denied);
    }
    Ok(())
}
