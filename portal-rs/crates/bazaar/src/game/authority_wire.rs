//! Exact CC v2 receipt grammar. Parsing alone never establishes authority.
use super::{
    Command, Error, PreparedIntent,
    game_wire::{decimal, digest, hash, id, ordered, text},
};
use oteryn_internal_api::canonical::{self, Value};
use uuid::Uuid;

pub(super) fn decode(raw: &[u8], max: usize) -> Result<Value, Error> {
    let v = canonical::decode(raw, max, 12).map_err(|_| Error::Invalid)?;
    if raw.contains(&b'\\') || canonical::encode(&v).map_err(|_| Error::Invalid)? != raw {
        return Err(Error::Invalid);
    }
    Ok(v)
}
pub(super) fn number(v: &Value, k: &str) -> Result<u64, Error> {
    decimal(text(v, k)?, false, u64::MAX)
}
pub(super) fn same(a: &Value, ak: &str, b: &Value, bk: &str) -> Result<(), Error> {
    if a.get(ak).is_none() || a.get(ak) != b.get(bk) {
        return Err(Error::Denied);
    }
    Ok(())
}
pub(super) fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b"._:/-".contains(&b))
        })
}
pub(super) fn item(v: &Value) -> Result<(), Error> {
    ordered(
        v,
        &[
            "item_instance_id",
            "world_id",
            "definition",
            "state_revision",
            "quantity",
            "attributes_hash",
            "attributes",
        ],
    )?;
    id(text(v, "item_instance_id")?)?;
    id(text(v, "world_id")?)?;
    number(v, "state_revision")?;
    if !matches!(v.get("quantity"), Some(Value::Unsigned(1..=4_294_967_295)))
        || !hash(text(v, "attributes_hash")?)
    {
        return Err(Error::Invalid);
    }
    let d = v.get("definition").ok_or(Error::Invalid)?;
    ordered(d, &["family", "production_key", "revision_ref"])?;
    for k in ["family", "production_key", "revision_ref"] {
        if !key(text(d, k)?) {
            return Err(Error::Invalid);
        }
    }
    let Some(Value::Array(attributes)) = v.get("attributes") else {
        return Err(Error::Invalid);
    };
    if attributes.len() > 6 {
        return Err(Error::Invalid);
    }
    let mut previous = "";
    for a in attributes {
        ordered(a, &["key", "value"])?;
        let k = text(a, "key")?;
        let max = match k {
            "tier" | "upgrade_level" => 255,
            "charges" => u32::MAX as u64,
            "attack" | "defense" | "armor" => 65535,
            _ => return Err(Error::Invalid),
        };
        if k <= previous {
            return Err(Error::Invalid);
        }
        previous = k;
        decimal(text(a, "value")?, true, max)?;
    }
    // attributes_hash covers Game's complete retained state, not this export.
    Ok(())
}
fn facts(v: &Value, command: Command) -> Result<(), Error> {
    let fields: &[&str] = match command {
        Command::Lock => &[
            "character_id",
            "lock_id",
            "lock_revision",
            "eligibility_revision",
            "signed_snapshot",
        ],
        Command::Bind => &[
            "character_id",
            "lock_id",
            "lock_revision",
            "transfer_operation_id",
            "transfer_binding_digest",
        ],
        Command::Unlock => &[
            "character_id",
            "lock_id",
            "lock_revision",
            "current_account_id",
        ],
        Command::Escrow => &[
            "escrow_id",
            "escrow_revision",
            "seller_character_id",
            "item",
        ],
        Command::Deliver | Command::Return => &[
            "escrow_id",
            "escrow_revision",
            "destination_account_id",
            "destination_character_id",
            "delivery_id",
            "item",
        ],
        Command::Hold => &[
            "hold_id",
            "hold_revision",
            "buyer_account_id",
            "offer_id",
            "hold_purpose",
            "held_amount",
        ],
        Command::Release => &[
            "hold_id",
            "hold_revision",
            "released_amount",
            "remaining_held",
            "hold_state",
        ],
        Command::Payout => &[
            "hold_id",
            "hold_revision",
            "sale_id",
            "gross_amount",
            "fee_amount",
            "seller_amount",
            "remaining_held",
            "buyer_ledger_entry_id",
            "seller_ledger_entry_id",
            "burn_entry_id",
            "delivery_operation_id",
            "delivery_receipt_digest",
        ],
    };
    ordered(v, fields)?;
    for k in fields {
        if k.ends_with("_id") {
            id(text(v, k)?)?;
        }
        if k.ends_with("_revision") {
            number(v, k)?;
        }
        if k.ends_with("_digest") && !hash(text(v, k)?) {
            return Err(Error::Invalid);
        }
        if k.ends_with("_amount") || *k == "remaining_held" {
            decimal(
                text(v, k)?,
                matches!(*k, "fee_amount" | "remaining_held"),
                999_999_999_999,
            )?;
        }
    }
    if let Some(i) = v.get("item") {
        item(i)?;
    }
    if command == Command::Hold && !matches!(text(v, "hold_purpose")?, "OFFER" | "RESERVE") {
        return Err(Error::Invalid);
    }
    if command == Command::Release {
        let remaining = decimal(text(v, "remaining_held")?, true, 999_999_999_999)?;
        if text(v, "hold_state")? != if remaining == 0 { "RELEASED" } else { "ACTIVE" } {
            return Err(Error::Invalid);
        }
    }
    if command == Command::Payout {
        let (fee, net) = super::gold_payout(number(v, "gross_amount")?)?;
        if number(v, "fee_amount")? != fee || number(v, "seller_amount")? != net {
            return Err(Error::Invalid);
        }
    }
    Ok(())
}
/// The retained bytes and digest are immutable; no ALREADY_APPLIED outcome exists.
#[derive(Debug, Clone)]
pub struct Receipt {
    pub(super) value: Value,
    pub(super) raw: String,
    pub operation: Uuid,
    pub generation: u64,
    pub command: Command,
    pub committed: bool,
}
impl Receipt {
    pub fn parse(raw: &[u8]) -> Result<Self, Error> {
        let v = decode(raw, 270336)?;
        ordered(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "command",
                "operation_id",
                "outcome",
                "result_code",
                "result_data",
                "recovery_generation",
                "decided_at",
            ],
        )?;
        if v.get("contract_version") != Some(&Value::Unsigned(2))
            || text(&v, "operation")? != "PublishCharacterCommandReceiptV1"
        {
            return Err(Error::Invalid);
        }
        let command = Command::from_wire(text(&v, "command")?).ok_or(Error::Invalid)?;
        let committed = match text(&v, "outcome")? {
            "COMMITTED" => true,
            "REJECTED" => false,
            _ => return Err(Error::Invalid),
        };
        let data = v.get("result_data").ok_or(Error::Invalid)?;
        if committed {
            if text(&v, "result_code")? != "CHAR_CMD_OK" {
                return Err(Error::Invalid);
            }
            facts(data, command)?;
        } else if raw.len() > 1024
            || data != &Value::Null
            || !matches!(
                text(&v, "result_code")?,
                "CHAR_CMD_UNSUPPORTED"
                    | "CHAR_CMD_INTENT_INVALID"
                    | "CHAR_CMD_INTENT_EXPIRED"
                    | "CHAR_CMD_OPERATION_CONFLICT"
                    | "CHAR_CMD_OWNER_MISMATCH"
                    | "CHAR_CMD_CHARACTER_NOT_FOUND"
                    | "CHAR_CMD_CHARACTER_IN_SESSION"
                    | "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"
                    | "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"
                    | "CHAR_CMD_HOUSING_NOT_PREPARED"
                    | "CHAR_CMD_BAZAAR_STALE_REVISION"
                    | "CHAR_CMD_BAZAAR_OBLIGATION"
                    | "CHAR_CMD_BAZAAR_ITEM_NOT_TRANSFERABLE"
                    | "CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE"
                    | "CHAR_CMD_BAZAAR_HOLD_CONFLICT"
                    | "CHAR_CMD_BAZAAR_PAYOUT_INVALID"
            )
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            operation: id(text(&v, "operation_id")?)?,
            generation: number(&v, "recovery_generation")?,
            command,
            committed,
            raw: std::str::from_utf8(raw).map_err(|_| Error::Invalid)?.into(),
            value: v,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        self.raw.as_bytes()
    }
    pub fn digest(&self) -> String {
        digest(self.bytes())
    }
    pub(super) fn bind(&self, intent: &PreparedIntent, now: u64) -> Result<(), Error> {
        let i = &intent.value;
        if self.command != intent.command || self.operation != id(text(i, "operation_id")?)? {
            return Err(Error::Conflict);
        }
        let decided = number(&self.value, "decided_at")?;
        if decided < number(i, "issued_at_source")? || decided > now.saturating_add(1) {
            return Err(Error::Denied);
        }
        if !self.committed {
            return Ok(());
        }
        if decided > number(i, "expires_at_source")? {
            return Err(Error::Denied);
        }
        let d = self.value.get("result_data").ok_or(Error::Invalid)?;
        match self.command {
            Command::Lock => {
                same(d, "character_id", i, "character_id")?;
                same(d, "eligibility_revision", i, "eligibility_revision")?;
            }
            Command::Bind => {
                for k in [
                    "character_id",
                    "lock_id",
                    "transfer_operation_id",
                    "transfer_binding_digest",
                ] {
                    same(d, k, i, k)?;
                }
                next(d, "lock_revision", i, "expected_lock_revision")?;
            }
            Command::Unlock => {
                for k in ["character_id", "lock_id"] {
                    same(d, k, i, k)?;
                }
                next(d, "lock_revision", i, "expected_lock_revision")?;
            }
            Command::Escrow => {
                same(d, "seller_character_id", i, "seller_character_id")?;
                let item = d.get("item").ok_or(Error::Invalid)?;
                for k in ["item_instance_id", "world_id", "quantity"] {
                    same(item, k, i, k)?;
                }
                next(item, "state_revision", i, "expected_state_revision")?;
            }
            Command::Deliver | Command::Return => {
                same(d, "escrow_id", i, "escrow_id")?;
                next(d, "escrow_revision", i, "expected_escrow_revision")?;
                let (account, character) = if self.command == Command::Deliver {
                    ("buyer_account_id", "buyer_character_id")
                } else {
                    ("seller_account_id", "seller_character_id")
                };
                same(d, "destination_account_id", i, account)?;
                same(d, "destination_character_id", i, character)?;
                same(
                    d.get("item").ok_or(Error::Invalid)?,
                    "world_id",
                    i,
                    "world_id",
                )?;
            }
            Command::Hold => {
                for k in ["buyer_account_id", "offer_id", "hold_purpose"] {
                    same(d, k, i, k)?;
                }
                same(d, "held_amount", i, "amount")?;
                next(d, "hold_revision", i, "expected_hold_revision")?;
            }
            Command::Release => {
                same(d, "hold_id", i, "hold_id")?;
                same(d, "released_amount", i, "amount")?;
                next(d, "hold_revision", i, "expected_hold_revision")?;
            }
            Command::Payout => {
                for k in [
                    "hold_id",
                    "sale_id",
                    "gross_amount",
                    "fee_amount",
                    "seller_amount",
                    "delivery_operation_id",
                    "delivery_receipt_digest",
                ] {
                    same(d, k, i, k)?;
                }
                next(d, "hold_revision", i, "expected_hold_revision")?;
            }
        }
        Ok(())
    }
}
fn next(d: &Value, dk: &str, i: &Value, ik: &str) -> Result<(), Error> {
    if decimal(text(i, ik)?, true, u64::MAX)?.checked_add(1) != Some(number(d, dk)?) {
        return Err(Error::Denied);
    }
    Ok(())
}
