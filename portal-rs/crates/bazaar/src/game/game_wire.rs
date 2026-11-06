use super::Error;
use oteryn_internal_api::canonical::{self, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Lock,
    Bind,
    Unlock,
    Escrow,
    Deliver,
    Return,
    Hold,
    Release,
    Payout,
}
impl Command {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Lock => "LockBazaarCharacter",
            Self::Bind => "BindBazaarCharacterTransfer",
            Self::Unlock => "UnlockBazaarCharacter",
            Self::Escrow => "EscrowBazaarItem",
            Self::Deliver => "DeliverBazaarItem",
            Self::Return => "ReturnBazaarItem",
            Self::Hold => "HoldBazaarGold",
            Self::Release => "ReleaseBazaarGold",
            Self::Payout => "PayoutBazaarGold",
        }
    }
    pub fn from_wire(value: &str) -> Option<Self> {
        [
            Self::Lock,
            Self::Bind,
            Self::Unlock,
            Self::Escrow,
            Self::Deliver,
            Self::Return,
            Self::Hold,
            Self::Release,
            Self::Payout,
        ]
        .into_iter()
        .find(|c| c.wire() == value)
    }
    fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Lock => &[
                "character_id",
                "expected_character_revision",
                "eligibility_revision",
                "snapshot_schema_version",
            ],
            Self::Bind => &[
                "character_id",
                "lock_id",
                "expected_lock_revision",
                "transfer_operation_id",
                "transfer_binding_digest",
                "purpose",
                "sale_id",
                "funding",
            ],
            Self::Unlock => &["character_id", "lock_id", "expected_lock_revision"],
            Self::Escrow => &[
                "seller_character_id",
                "item_instance_id",
                "expected_state_revision",
                "quantity",
                "transferability_revision",
            ],
            Self::Deliver => &[
                "escrow_id",
                "expected_escrow_revision",
                "buyer_account_id",
                "buyer_character_id",
                "escrow_receipt_digest",
                "sale_id",
                "funding",
            ],
            Self::Return => &[
                "escrow_id",
                "expected_escrow_revision",
                "seller_character_id",
                "escrow_receipt_digest",
            ],
            Self::Hold => &[
                "buyer_account_id",
                "buyer_character_id",
                "offer_id",
                "hold_purpose",
                "amount",
                "expected_hold_revision",
            ],
            Self::Release => &[
                "buyer_account_id",
                "hold_id",
                "expected_hold_revision",
                "release_reason",
                "amount",
            ],
            Self::Payout => &[
                "buyer_account_id",
                "hold_id",
                "expected_hold_revision",
                "sale_id",
                "gross_amount",
                "fee_bps",
                "fee_amount",
                "seller_amount",
                "delivery_operation_id",
                "delivery_receipt_digest",
            ],
        }
    }
}
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, Error> {
    match v.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::Invalid),
    }
}
pub(super) fn decimal(s: &str, zero: bool, max: u64) -> Result<u64, Error> {
    if s.is_empty()
        || s.len() > 20
        || !s.bytes().all(|b| b.is_ascii_digit())
        || (s.len() > 1 && s.starts_with('0'))
    {
        return Err(Error::Invalid);
    }
    let n = s.parse::<u64>().map_err(|_| Error::Invalid)?;
    if n > max || (!zero && n == 0) {
        return Err(Error::Invalid);
    }
    Ok(n)
}
pub(super) fn id(s: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(s).map_err(|_| Error::Invalid)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != s
    {
        return Err(Error::Invalid);
    }
    Ok(id)
}
pub(super) fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn digest(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
pub(super) fn ordered(v: &Value, fields: &[&str]) -> Result<(), Error> {
    match v {
        Value::Object(m)
            if m.len() == fields.len() && m.iter().zip(fields).all(|((k, _), f)| k == f) =>
        {
            Ok(())
        }
        _ => Err(Error::Invalid),
    }
}
/// Proposed gold fee is rounded upward, conserving gross = net + burned fee.
pub fn gold_payout(gross: u64) -> Result<(u64, u64), Error> {
    if !(1..=999_999_999_999).contains(&gross) {
        return Err(Error::Invalid);
    }
    let fee = (u128::from(gross) * 250).div_ceil(10000) as u64;
    let net = gross - fee;
    if net == 0 {
        return Err(Error::Invalid);
    }
    Ok((fee, net))
}
fn funding(v: &Value) -> Result<(), Error> {
    ordered(
        v,
        &[
            "currency",
            "reservation_id",
            "hold_revision",
            "gross_amount",
        ],
    )?;
    id(text(v, "reservation_id")?)?;
    let max = match text(v, "currency")? {
        "GOLD" => {
            decimal(text(v, "hold_revision")?, false, u64::MAX)?;
            999_999_999_999
        }
        "COINS" if v.get("hold_revision") == Some(&Value::Null) => i64::MAX as u64,
        _ => return Err(Error::Invalid),
    };
    let gross = decimal(text(v, "gross_amount")?, false, max)?;
    if text(v, "currency")? == "GOLD" {
        gold_payout(gross)?;
    }
    Ok(())
}
/// Validated canonical bytes; construction never infers owner, funding or custody.
#[derive(Debug, Clone)]
pub struct PreparedIntent {
    pub(super) raw: String,
    pub(super) value: Value,
    pub(super) command: Command,
}
impl PreparedIntent {
    pub fn parse(raw: &str) -> Result<Self, Error> {
        let value = canonical::decode(raw.as_bytes(), 4096, 12).map_err(|_| Error::Invalid)?;
        if raw.contains('\\')
            || canonical::encode(&value).map_err(|_| Error::Invalid)? != raw.as_bytes()
        {
            return Err(Error::Invalid);
        }
        let command = [
            Command::Lock,
            Command::Bind,
            Command::Unlock,
            Command::Escrow,
            Command::Deliver,
            Command::Return,
            Command::Hold,
            Command::Release,
            Command::Payout,
        ]
        .into_iter()
        .find(|c| Some(c.wire()) == text(&value, "command").ok())
        .ok_or(Error::Invalid)?;
        let mut fields = vec![
            "contract_version",
            "command",
            "variant",
            "issuer_authority",
            "issuer_decision_id",
            "source_revision",
            "operation_id",
            "listing_id",
            "seller_account_id",
            "world_id",
        ];
        fields.extend_from_slice(command.fields());
        fields.extend_from_slice(&["issued_at_source", "expires_at_source", "audience"]);
        ordered(&value, &fields)?;
        if value.get("contract_version") != Some(&Value::Unsigned(2))
            || text(&value, "variant")? != "PLATFORM_BAZAAR"
            || text(&value, "issuer_authority")? != "OTERYN_PLATFORM_BAZAAR_AUTHORITY"
            || text(&value, "audience")? != "OTERYN_GAME_BAZAAR_AUTHORITY"
        {
            return Err(Error::Invalid);
        }
        for key in &fields {
            let v = value.get(key).ok_or(Error::Invalid)?;
            if *key == "sale_id" && v == &Value::Null && command == Command::Bind {
                continue;
            }
            if key.ends_with("_id") {
                id(text(&value, key)?)?;
            }
            if key.ends_with("_revision")
                || matches!(*key, "issued_at_source" | "expires_at_source")
            {
                decimal(
                    text(&value, key)?,
                    *key == "expected_hold_revision" && command == Command::Hold,
                    u64::MAX,
                )?;
            }
            if key.ends_with("_digest") && !hash(text(&value, key)?) {
                return Err(Error::Invalid);
            }
        }
        let issued = decimal(text(&value, "issued_at_source")?, false, u64::MAX)?;
        let expires = decimal(text(&value, "expires_at_source")?, false, u64::MAX)?;
        if expires <= issued || expires - issued > 300 {
            return Err(Error::Invalid);
        }
        if value.get("buyer_account_id").is_some()
            && text(&value, "buyer_account_id")? == text(&value, "seller_account_id")?
        {
            return Err(Error::Invalid);
        }
        match command {
            Command::Lock if value.get("snapshot_schema_version") != Some(&Value::Unsigned(1)) => {
                return Err(Error::Invalid);
            }
            Command::Escrow => match value.get("quantity") {
                Some(Value::Unsigned(1..=4_294_967_295)) => {}
                _ => return Err(Error::Invalid),
            },
            Command::Bind => {
                let settlement = match text(&value, "purpose")? {
                    "BAZAAR_SETTLEMENT" => true,
                    "BAZAAR_LISTING_ESCROW" | "BAZAAR_ESCROW_RETURN" => false,
                    _ => return Err(Error::Invalid),
                };
                if settlement {
                    id(text(&value, "sale_id")?)?;
                    funding(value.get("funding").ok_or(Error::Invalid)?)?;
                } else if value.get("sale_id") != Some(&Value::Null)
                    || value.get("funding") != Some(&Value::Null)
                {
                    return Err(Error::Invalid);
                }
            }
            Command::Deliver => funding(value.get("funding").ok_or(Error::Invalid)?)?,
            Command::Hold | Command::Release => {
                let amount = decimal(text(&value, "amount")?, false, 999_999_999_999)?;
                if command == Command::Hold {
                    gold_payout(amount)?;
                }
                if command == Command::Hold
                    && !matches!(text(&value, "hold_purpose")?, "OFFER" | "RESERVE")
                {
                    return Err(Error::Invalid);
                }
                if command == Command::Release
                    && !matches!(
                        text(&value, "release_reason")?,
                        "OUTBID"
                            | "DECLINED"
                            | "CANCELLED"
                            | "UNSOLD"
                            | "EXCESS"
                            | "TERMINAL_REFUSAL"
                    )
                {
                    return Err(Error::Invalid);
                }
            }
            Command::Payout => {
                let (fee, net) = gold_payout(decimal(
                    text(&value, "gross_amount")?,
                    false,
                    999_999_999_999,
                )?)?;
                if value.get("fee_bps") != Some(&Value::Unsigned(250))
                    || decimal(text(&value, "fee_amount")?, true, 999_999_999_999)? != fee
                    || decimal(text(&value, "seller_amount")?, false, 999_999_999_999)? != net
                {
                    return Err(Error::Invalid);
                }
            }
            _ => {}
        }
        Ok(Self {
            raw: raw.into(),
            value,
            command,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        self.raw.as_bytes()
    }
    pub fn binding_digest(&self) -> String {
        digest(self.bytes())
    }
    pub fn command(&self) -> Command {
        self.command
    }
}
