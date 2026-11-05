//! Default-off Store purchase wire. SHA-256 catalog revisions follow the owner
//! decision in StorePurchaseInput.php and the pinned Game Store wire decoder.
use oteryn_internal_api::canonical::{self, Shape, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[path = "purchase_http.rs"]
pub mod http;
#[path = "purchase_repository.rs"]
mod repository;
pub use repository::{AccountState, Accounts, Purchases};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unauthorized,
    UnknownAccount,
    Conflict,
    Limited(u32),
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "store purchase: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl From<oteryn_wallet::Error> for Error {
    fn from(_: oteryn_wallet::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub purchase_enabled: bool,
    pub delivery_enabled: bool,
    pub environment: String,
    pub source_authority: String,
    pub peer_identity: String,
    pub enabled_unit_kinds: Vec<String>,
    pub requests_per_minute: u32,
    pub purchases_per_account_per_minute: u32,
    pub reads_per_account_per_minute: u32,
    pub max_pending_operations: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            purchase_enabled: false,
            delivery_enabled: false,
            environment: String::new(),
            source_authority: String::new(),
            peer_identity: String::new(),
            enabled_unit_kinds: Vec::new(),
            requests_per_minute: 600,
            purchases_per_account_per_minute: 10,
            reads_per_account_per_minute: 60,
            max_pending_operations: 1000,
        }
    }
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let mut settings = Self {
            purchase_enabled: get("PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED").as_deref()
                == Some("true"),
            delivery_enabled: get("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED").as_deref()
                == Some("true"),
            environment: get("OTERYN_ENV").unwrap_or_default(),
            source_authority: get("PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY")
                .unwrap_or_default(),
            peer_identity: get("PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY")
                .unwrap_or_default(),
            enabled_unit_kinds: get("PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS")
                .map(|value| serde_json::from_str(&value).unwrap_or_else(|_| vec![String::new()]))
                .unwrap_or_default(),
            ..Self::default()
        };
        for (suffix, bound) in [
            ("REQUESTS_PER_MINUTE", &mut settings.requests_per_minute),
            (
                "PURCHASES_PER_ACCOUNT_PER_MINUTE",
                &mut settings.purchases_per_account_per_minute,
            ),
            (
                "READS_PER_ACCOUNT_PER_MINUTE",
                &mut settings.reads_per_account_per_minute,
            ),
            (
                "MAX_PENDING_OPERATIONS",
                &mut settings.max_pending_operations,
            ),
        ] {
            if let Some(value) = get(&format!("PRODUCTS_ENTITLEMENTS_STORE_{suffix}")) {
                *bound = decimal(&value, false)
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(0);
            }
        }
        settings
    }
    pub fn valid(&self) -> bool {
        self.purchase_enabled
            && self.delivery_enabled
            && matches!(self.environment.as_str(), "testing" | "preproduction")
            && source(&self.source_authority)
            && oteryn_internal_api::guard::Identity::san(&self.peer_identity).is_ok()
            && (1..=6000).contains(&self.requests_per_minute)
            && (1..=60).contains(&self.purchases_per_account_per_minute)
            && (1..=600).contains(&self.reads_per_account_per_minute)
            && (1..=100000).contains(&self.max_pending_operations)
            && self.enabled_unit_kinds.iter().enumerate().all(|(i, kind)| {
                matches!(kind.as_str(), "PREMIUM_TIME" | "COSMETIC" | "ITEM")
                    && !self.enabled_unit_kinds[..i].contains(kind)
            })
    }
}
fn source(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn uuid(value: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(value).map_err(|_| Error::Invalid)?;
    if id.to_string() != value
        || id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    Ok(id)
}
fn decimal(value: &str, zero: bool) -> Option<u64> {
    if value == "0" && zero {
        return Some(0);
    }
    if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
fn hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn offer_key(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}
fn head(version: u8, operation: &str, expected: &str, authority: &str) -> Result<(), Error> {
    if version != 1 || operation != expected || !source(authority) {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub const BALANCE_SHAPE: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("operation", Shape::Scalar),
    ("source_authority", Shape::Scalar),
    ("account_id", Shape::Scalar),
]);
pub const HISTORY_SHAPE: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("operation", Shape::Scalar),
    ("source_authority", Shape::Scalar),
    ("account_id", Shape::Scalar),
    ("before_entry_id", Shape::Scalar),
    ("max_entries", Shape::Scalar),
]);
pub const PURCHASE_SHAPE: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("operation", Shape::Scalar),
    ("source_authority", Shape::Scalar),
    ("purchase_request_id", Shape::Scalar),
    ("account_id", Shape::Scalar),
    ("character_id", Shape::Scalar),
    ("world_id", Shape::Scalar),
    ("catalog_revision", Shape::Scalar),
    ("offer_key", Shape::Scalar),
    ("currency", Shape::Scalar),
    ("unit_price", Shape::Scalar),
    ("quantity", Shape::Scalar),
]);
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseRequest {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub purchase_request_id: String,
    pub account_id: String,
    pub character_id: String,
    pub world_id: String,
    pub catalog_revision: String,
    pub offer_key: String,
    pub currency: String,
    pub unit_price: String,
    pub quantity: u8,
}
impl PurchaseRequest {
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        canonical::decode_canonical(raw, 1024, 4, &PURCHASE_SHAPE).map_err(|_| Error::Invalid)?;
        let request: Self = serde_json::from_slice(raw).map_err(|_| Error::Invalid)?;
        head(
            request.contract_version,
            &request.operation,
            "PurchaseStoreOfferV1",
            &request.source_authority,
        )?;
        for id in [
            &request.purchase_request_id,
            &request.account_id,
            &request.character_id,
            &request.world_id,
        ] {
            uuid(id)?;
        }
        if !hex64(&request.catalog_revision)
            || !offer_key(&request.offer_key)
            || !matches!(request.currency.as_str(), "COINS" | "TRANSFERABLE_COINS")
            || decimal(&request.unit_price, false).is_none_or(|n| n > i64::MAX as u64)
            || !(1..=100).contains(&request.quantity)
        {
            return Err(Error::Invalid);
        }
        Ok(request)
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub before_entry_id: Option<String>,
    pub max_entries: Option<u8>,
}
impl ReadRequest {
    pub fn decode(raw: &[u8], history: bool) -> Result<Self, Error> {
        canonical::decode_canonical(
            raw,
            512,
            4,
            if history {
                &HISTORY_SHAPE
            } else {
                &BALANCE_SHAPE
            },
        )
        .map_err(|_| Error::Invalid)?;
        let request: Self = serde_json::from_slice(raw).map_err(|_| Error::Invalid)?;
        head(
            request.contract_version,
            &request.operation,
            if history {
                "ReadStoreHistoryPageV1"
            } else {
                "ReadStoreBalanceV1"
            },
            &request.source_authority,
        )?;
        uuid(&request.account_id)?;
        if history
            && (request
                .before_entry_id
                .as_deref()
                .and_then(|v| decimal(v, true))
                .is_none()
                || request.max_entries.is_none_or(|n| !(1..=32).contains(&n)))
        {
            return Err(Error::Invalid);
        }
        Ok(request)
    }
}
pub(super) fn object(members: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}
pub(super) fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
pub(super) fn number(value: u64) -> Value {
    Value::Unsigned(value)
}
pub(super) fn encode(value: &Value, max: usize) -> Result<Vec<u8>, Error> {
    let bytes = canonical::encode(value).map_err(|_| Error::Unavailable)?;
    if bytes.len() > max || bytes.contains(&b'\\') {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
#[cfg(test)]
#[path = "purchase_tests.rs"]
mod tests;
