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
    let operation:Option<(Uuid,String)>=sqlx::query_as("SELECT operation_id,projection_epoch::text FROM bazaar_game_outbox WHERE listing_id=$1 AND command='LockBazaarCharacter' AND EXISTS(SELECT 1 FROM bazaar_authority_receipts r WHERE r.operation_id=bazaar_game_outbox.operation_id AND r.outcome='COMMITTED') ORDER BY source_revision DESC LIMIT 1").bind(listing).fetch_optional(&mut *db).await?;
    let Some((operation, epoch)) = operation else {
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
    let key = if let Some((_, key)) = a.config.snapshot_keys.iter().find(|(name, _)| name == kid) {
        *key
    } else {
        // Historical verification is limited to a retained, already admitted Lock.
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT public_key FROM bazaar_authority_snapshot_keys WHERE kid=$1 AND revoked",
        )
        .bind(kid)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::Unavailable)?;
        ed25519_dalek::VerifyingKey::from_bytes(&bytes.try_into().map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Invalid)?
    };
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
        projection_epoch: decimal(&epoch, false, u64::MAX)?,
        now: now(db).await?,
    };
    verify_snapshot(
        &canonical::encode(signed).map_err(|_| Error::Invalid)?,
        &e,
        kid,
        &key,
    )?;
    let signed = signed.clone();
    Ok(Some((i, r, signed)))
}
pub(crate) async fn staged(
    a: &Authority,
    db: &mut PgConnection,
    operation: Uuid,
) -> Result<Value, Error> {
    let row=sqlx::query("SELECT intent,intent_digest,projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_all_transfers WHERE operation_id=$1").bind(operation).fetch_optional(&mut *db).await?.ok_or(Error::Unavailable)?;
    let raw: String = row.try_get("intent")?;
    // Staging records when the immutable intent was created. Replayed receipts
    // and projections carry their own current recovery fences.
    if digest(raw.as_bytes()) != row.try_get::<String, _>("intent_digest")?
        || decimal(row.try_get("epoch")?, false, u64::MAX)? > a.fence.projection_epoch
        || decimal(row.try_get("generation")?, false, u64::MAX)? > a.fence.recovery_generation
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
pub(crate) async fn settlement_buyer(
    db: &mut PgConnection,
    i: &PreparedIntent,
) -> Result<Uuid, Error> {
    sqlx::query_scalar(
                "SELECT buyer FROM bazaar_settlements WHERE id=$1 AND listing_id=$2 AND seller=$3 UNION ALL SELECT buyer FROM bazaar_authority_gold_sales WHERE id=$1 AND listing_id=$2 AND seller=$3",
            )
            .bind(id(text(&i.value, "sale_id")?)?)
            .bind(id(text(&i.value, "listing_id")?)?)
            .bind(id(text(&i.value, "seller_account_id")?)?)
            .fetch_optional(db)
            .await?
            .ok_or(Error::Unavailable)
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
            let buyer = settlement_buyer(db, i).await?;
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
        return super::gold::funded(a, db, i, buyer).await;
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
    let row=sqlx::query("SELECT t.operation_id,t.intent,t.projection_epoch::text AS epoch,t.intent_digest,c.intent_json,c.receipt_json FROM bazaar_current_transfers t JOIN character_commands c ON c.id=t.operation_id WHERE t.listing_id=$1 AND t.purpose=$2 AND c.state='TERMINAL'").bind(listing).bind(purpose).fetch_optional(&mut *db).await?;
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
    if rejected_receipt(a, &r, &intent, now(db).await?)? {
        return Ok(None);
    }
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
        || !(decimal(row.try_get("epoch")?, false, u64::MAX)?..=a.fence.projection_epoch)
            .contains(&number(&r, "projection_epoch")?)
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
        if get("projection_epoch")? != self.fence.projection_epoch {
            return Ok(None);
        }
        let Some(Value::Array(blessings)) = s.get("blessings") else {
            return Err(crate::Error::Denied);
        };
        let blessings = blessings
            .iter()
            .map(|value| match value {
                Value::String(name) => Ok(name.clone()),
                _ => Err(crate::Error::Denied),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(Value::Array(skills)) = s.get("skills") else {
            return Err(crate::Error::Denied);
        };
        let skills = skills
            .iter()
            .map(|v| {
                Ok(Skill {
                    key: text(v, "key").map_err(to_core)?.into(),
                    level: decimal(text(v, "level").map_err(to_core)?, true, u64::MAX)
                        .map_err(to_core)?,
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
            blessings,
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

// Called under the issuer lock. A staged transfer or bind is already an
// obligation, even before a command becomes listable or a receipt arrives.
pub(super) async fn never_transferred(db: &mut PgConnection, listing: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM bazaar_all_transfers WHERE listing_id=$1)
            AND NOT EXISTS(SELECT 1 FROM bazaar_game_outbox WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer')"
    ).bind(listing).fetch_one(db).await?)
}

pub(crate) async fn verify_unlock(
    a: &Authority,
    db: &mut PgConnection,
    i: &PreparedIntent,
    current: Option<&Value>,
) -> Result<(), Error> {
    let listing = id(text(&i.value, "listing_id")?)?;
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_current_transfers t LEFT JOIN character_commands c ON c.id=t.operation_id WHERE t.listing_id=$1 AND (c.id IS NULL OR c.state='PENDING'))").bind(listing).fetch_one(&mut *db).await?;
    if pending {
        return Err(Error::Unavailable);
    }
    let owner = if let Some((_, receipt, _, _)) = proof(a, db, listing, "BAZAAR_SETTLEMENT").await?
    {
        id(text(&receipt, "to_account_id")?)?
    } else if let Some((_, receipt, _, _)) = proof(a, db, listing, "BAZAAR_ESCROW_RETURN").await? {
        id(text(&receipt, "to_account_id")?)?
    } else {
        let refused_escrow = a
            .bind_refused(db, listing, "BAZAAR_LISTING_ESCROW")
            .await?
            .is_some();
        let no_transfer = never_transferred(db, listing).await?;
        if !refused_escrow && !no_transfer {
            return Err(Error::Unavailable);
        }
        // Receipt admission follows the durable dispatch after closure.
        let mut refused_lock_recovery = false;
        let authorized = if current.is_some() {
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=$1)",
            )
            .bind(id(text(&i.value, "operation_id")?)?)
            .fetch_one(&mut *db)
            .await?
        } else {
            refused_lock_recovery = no_transfer
                && sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM bazaar_authority_assets a
                 JOIN bazaar_listings l ON l.id=a.listing_id
                 WHERE l.id=$1 AND l.state IN ('draft','cancelled') AND l.kind='character'
                   AND a.refusal_code IS NOT NULL)",
                )
                .bind(listing)
                .fetch_one(&mut *db)
                .await?;
            refused_escrow
                || refused_lock_recovery
                || super::invalidation::approved(db, listing).await?
        };
        if !authorized {
            return Err(Error::Denied);
        }
        let (lock, receipt, _) = lock_record(a, db, listing)
            .await?
            .ok_or(Error::Unavailable)?;
        for key in ["character_id", "world_id", "seller_account_id"] {
            same(&lock.value, key, &i.value, key)?;
        }
        let facts = receipt.value.get("result_data").ok_or(Error::Invalid)?;
        same(facts, "lock_id", &i.value, "lock_id")?;
        same(facts, "lock_revision", &i.value, "expected_lock_revision")?;
        if refused_lock_recovery {
            let status = a
                .projection(db, "STATUS", listing)
                .await?
                .ok_or(Error::Unavailable)?;
            if text(&status, "state")? != "locked" || text(&status, "asset_kind")? != "CHARACTER" {
                return Err(Error::Denied);
            }
            for key in ["world_id", "seller_account_id"] {
                same(&status, key, &lock.value, key)?;
            }
            same(&status, "asset_id", &lock.value, "character_id")?;
            same(&status, "custody_id", facts, "lock_id")?;
            same(&status, "custody_revision", facts, "lock_revision")?;
            same(&status, "last_operation_id", &lock.value, "operation_id")?;
        }
        id(text(&lock.value, "seller_account_id")?)?
    };
    if let Some(facts) = current
        && id(text(facts, "current_account_id")?)? != owner
    {
        return Err(Error::Denied);
    }
    Ok(())
}

pub(crate) fn rejected_receipt(
    a: &Authority,
    receipt: &Value,
    intent: &Value,
    now: u64,
) -> Result<bool, Error> {
    if text(receipt, "outcome")? != "REJECTED" {
        return Ok(false);
    }
    // Use the owning parser's closed code set; unknown or malformed refusals
    // cannot retire an operation. A terminal CC1 rejection is immutable no-effect
    // evidence, not a timeout or an ownership projection.
    let raw = canonical::encode(receipt).map_err(|_| Error::Invalid)?;
    oteryn_characters::commands::parse_receipt(&raw).map_err(|_| Error::Invalid)?;
    Ok(
        text(receipt, "source_authority")? == a.config.receipt_authority
            && text(receipt, "command")? == "TransferCharacterOwnership"
            && text(receipt, "operation_id")? == text(intent, "operation_id")?
            && number(receipt, "recovery_generation")? == a.fence.recovery_generation
            && number(receipt, "decided_at")?
                >= number(intent, "issued_at_source")?.saturating_sub(5)
            && number(receipt, "decided_at")? <= now.saturating_add(1),
    )
}

pub(crate) async fn reconcile_transfer(
    a: &Authority,
    db: &mut PgConnection,
    listing: Uuid,
    purpose: &str,
) -> Result<Option<Uuid>, Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
        .execute(&mut *db)
        .await?;
    let row=sqlx::query("SELECT t.operation_id,t.intent,c.intent_json,c.receipt_json,c.state FROM bazaar_current_transfers t LEFT JOIN character_commands c ON c.id=t.operation_id WHERE t.listing_id=$1 AND t.purpose=$2 ORDER BY t.source_revision DESC LIMIT 1").bind(listing).bind(purpose).fetch_optional(&mut *db).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let operation: Uuid = row.try_get("operation_id")?;
    let transfer = staged(a, db, operation).await?;
    let published: Option<String> = row.try_get("intent_json")?;
    let evidence = if let Some(published) = published {
        if published != row.try_get::<String, _>("intent")? {
            return Err(Error::Conflict);
        }
        if let Some(raw) = row.try_get::<Option<String>, _>("receipt_json")? {
            let receipt = decode(raw.as_bytes(), 1024)?;
            if row.try_get::<String, _>("state")? == "TERMINAL"
                && rejected_receipt(a, &receipt, &transfer, now(db).await?)?
            {
                Some(operation)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        let bind:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='BindBazaarCharacterTransfer' AND intent::jsonb->>'transfer_operation_id'=$2 ORDER BY source_revision DESC LIMIT 1").bind(listing).bind(operation.to_string()).fetch_optional(&mut *db).await?;
        if let Some(bind) = bind {
            if let Some((intent, receipt)) = a.retained(db, bind).await? {
                if !receipt.committed
                    && (text(&receipt.value, "result_code")? == "CHAR_CMD_INTENT_EXPIRED"
                        || (purpose == "BAZAAR_ESCROW_RETURN"
                            && text(&receipt.value, "result_code")?
                                == "CHAR_CMD_BAZAAR_STALE_REVISION"
                            && a.projection(db, "STATUS", listing)
                                .await?
                                .is_some_and(|status| {
                                    status.get("custody_revision")
                                        != intent.value.get("expected_lock_revision")
                                })))
                    && text(&intent.value, "transfer_binding_digest")?
                        == digest(&canonical::encode(&transfer).map_err(|_| Error::Invalid)?)
                {
                    Some(bind)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };
    if let Some(evidence) = evidence {
        crate::lifecycle::retire_transfer(db, operation, evidence, a.fence.recovery_generation)
            .await?;
        return Ok(None);
    }
    Ok(Some(operation))
}
