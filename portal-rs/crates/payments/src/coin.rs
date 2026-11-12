//! Native coin purchases (sandbox) per Game contract OTERYN_COIN_PURCHASE_V1.
//!
//! Wire types, validation, the order state machine and deployment settings are
//! free of I/O here; `coin_store` holds the durable service and `coin_http` the
//! routes. Refunds and chargebacks are representable states only (phase A).
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Exactly the contract's `version`.
pub const VERSION: u8 = 1;
/// Largest coin value on the wire (0..999999999999).
pub const MAX_COINS: i64 = 999_999_999_999;
/// Largest price in minor units (1..9999999999).
pub const MAX_MINOR_UNITS: i64 = 9_999_999_999;
/// Request bodies above this many bytes are refused.
pub const MAX_REQUEST_BYTES: usize = 8192;
/// Bounded catalogue views.
pub const MAX_PACKAGES: usize = 20;
pub const MAX_RECENT_ORDERS: i64 = 20;
/// Path prefix of the hosted sandbox checkout page.
pub const CHECKOUT_PATH: &str = "/native-account/coin-checkout/";

/// Canonical lower-case UUIDv7 text only; anything else is refused.
pub fn canonical_id(raw: &str) -> Option<Uuid> {
    let id = Uuid::parse_str(raw).ok()?;
    (id.get_version_num() == 7
        && id.get_variant() == uuid::Variant::RFC4122
        && id.hyphenated().to_string() == raw)
        .then_some(id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Price {
    pub minor_units: i64,
    pub currency: String,
    pub exponent: u8,
}
impl Price {
    pub fn valid(&self) -> bool {
        (1..=MAX_MINOR_UNITS).contains(&self.minor_units)
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase())
            && self.exponent <= 3
    }
    /// Integer division with zero-padded fractional digits; no floating point.
    pub fn display(&self) -> String {
        let scale = 10_i64.pow(u32::from(self.exponent));
        let whole = self.minor_units / scale;
        if self.exponent == 0 {
            return format!("{whole} {}", self.currency);
        }
        let fraction = self.minor_units % scale;
        let width = usize::from(self.exponent);
        format!("{whole}.{fraction:0width$} {}", self.currency)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoinPackage {
    pub id: Uuid,
    pub quote_id: Uuid,
    pub title: String,
    pub coins: i64,
    pub price: Price,
    pub valid_for_seconds: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoinCatalogue {
    pub version: u8,
    pub sandbox: bool,
    pub character_id: Option<Uuid>,
    pub packages: Vec<CoinPackage>,
    pub recent_orders: Vec<CoinOrder>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CoinBalance {
    pub version: u8,
    pub revision: i64,
    pub total_coins: i64,
    pub available_coins: i64,
    pub transferable_coins: i64,
    pub review_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoinOrder {
    pub version: u8,
    pub order_id: Uuid,
    pub request_id: Uuid,
    pub revision: i64,
    pub package_id: Uuid,
    pub quote_id: Uuid,
    pub character_id: Option<Uuid>,
    pub coins: i64,
    pub price: Price,
    pub sandbox: bool,
    pub state: OrderState,
    pub checkout_url: Option<String>,
    pub receipt_id: Option<Uuid>,
    pub review_required: bool,
    pub code: String,
}

/// A validated `CoinOrderRequest`. The client never sends a price or coin count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinOrderRequest {
    pub request_id: Uuid,
    pub package_id: Uuid,
    pub quote_id: Uuid,
    pub character_id: Option<Uuid>,
}

/// `deserialize_with` makes the nullable member required: absent is refused.
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOrderRequest {
    version: u64,
    request_id: String,
    package_id: String,
    quote_id: String,
    #[serde(deserialize_with = "present")]
    character_id: Option<String>,
}
impl CoinOrderRequest {
    /// Strict decode: size, unknown and missing members, version and ID forms.
    pub fn decode(body: &[u8]) -> Option<Self> {
        if body.len() > MAX_REQUEST_BYTES {
            return None;
        }
        let raw: RawOrderRequest = serde_json::from_slice(body).ok()?;
        if raw.version != u64::from(VERSION) {
            return None;
        }
        let character_id = match raw.character_id {
            Some(id) => Some(canonical_id(&id)?),
            None => None,
        };
        Some(Self {
            request_id: canonical_id(&raw.request_id)?,
            package_id: canonical_id(&raw.package_id)?,
            quote_id: canonical_id(&raw.quote_id)?,
            character_id,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderState {
    AwaitingPayment,
    Processing,
    Paid,
    Failed,
    Refunded,
    Chargeback,
}
impl OrderState {
    /// Lifecycle never moves backwards; failed and chargeback are terminal.
    pub fn may_become(self, next: Self) -> bool {
        use OrderState::*;
        match self {
            AwaitingPayment => next != AwaitingPayment,
            Processing => matches!(next, Paid | Failed | Refunded | Chargeback),
            Paid => matches!(next, Refunded | Chargeback),
            Refunded => next == Chargeback,
            Failed | Chargeback => false,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingPayment => "awaiting_payment",
            Self::Processing => "processing",
            Self::Paid => "paid",
            Self::Failed => "failed",
            Self::Refunded => "refunded",
            Self::Chargeback => "chargeback",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "awaiting_payment" => Self::AwaitingPayment,
            "processing" => Self::Processing,
            "paid" => Self::Paid,
            "failed" => Self::Failed,
            "refunded" => Self::Refunded,
            "chargeback" => Self::Chargeback,
            _ => return None,
        })
    }
    /// Public support code (1..96 characters).
    pub fn code(self) -> &'static str {
        match self {
            Self::AwaitingPayment => "COIN_ORDER_AWAITING_PAYMENT",
            Self::Processing => "COIN_ORDER_PROCESSING",
            Self::Paid => "COIN_ORDER_PAID",
            Self::Failed => "COIN_ORDER_FAILED",
            Self::Refunded => "COIN_ORDER_REFUNDED",
            Self::Chargeback => "COIN_ORDER_CHARGEBACK",
        }
    }
}

/// One configured package. Its UUIDv7 is stable across deployments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDef {
    pub id: Uuid,
    pub title: String,
    pub coins: i64,
    pub price: Price,
    pub valid_for_seconds: u16,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackage {
    id: String,
    title: String,
    coins: i64,
    price: Price,
    valid_for_seconds: u16,
}

/// Deployment settings. Any missing or malformed value leaves the feature off.
#[derive(Debug, Clone, Default)]
pub struct CoinSettings {
    pub enabled: bool,
    pub packages: Vec<PackageDef>,
    /// Exact configured Platform origin, e.g. `https://oteryn.example`.
    pub origin: String,
    /// Environment a quote binds (`testing` or `preprod`).
    pub environment: String,
}
impl CoinSettings {
    /// `PORTAL_COIN_PURCHASES_ENABLED=true` (default off), `OTERYN_ENV` testing or
    /// preprod (the sandbox never runs in production), `OTERYN_PUBLIC_ORIGIN` and
    /// `PORTAL_COIN_PACKAGES` (JSON list; unset means no packages).
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let disabled = Self::default();
        if lookup("PORTAL_COIN_PURCHASES_ENABLED").as_deref() != Some("true") {
            return disabled;
        }
        let environment = match lookup("OTERYN_ENV").as_deref() {
            Some(value @ ("testing" | "preprod")) => value.to_owned(),
            _ => return disabled,
        };
        let Some(origin) = lookup("OTERYN_PUBLIC_ORIGIN").filter(|o| valid_origin(o)) else {
            return disabled;
        };
        let packages = match lookup("PORTAL_COIN_PACKAGES") {
            None => Vec::new(),
            Some(raw) => match parse_packages(&raw) {
                Some(packages) => packages,
                None => return disabled,
            },
        };
        Self {
            enabled: true,
            packages,
            origin,
            environment,
        }
    }
    pub fn checkout_url(&self, order: Uuid) -> String {
        format!("{}{CHECKOUT_PATH}{order}", self.origin)
    }
}

pub fn parse_packages(raw: &str) -> Option<Vec<PackageDef>> {
    let raw: Vec<RawPackage> = serde_json::from_str(raw).ok()?;
    if raw.len() > MAX_PACKAGES {
        return None;
    }
    let mut packages: Vec<PackageDef> = Vec::with_capacity(raw.len());
    for package in raw {
        let id = canonical_id(&package.id)?;
        let title_chars = package.title.chars().count();
        if !(1..=64).contains(&title_chars)
            || package.title.chars().any(char::is_control)
            || !(1..=MAX_COINS).contains(&package.coins)
            || !package.price.valid()
            || !(1..=300).contains(&package.valid_for_seconds)
            || packages.iter().any(|old| old.id == id)
        {
            return None;
        }
        packages.push(PackageDef {
            id,
            title: package.title,
            coins: package.coins,
            price: package.price,
            valid_for_seconds: package.valid_for_seconds,
        });
    }
    Some(packages)
}

/// `https://host[:port]`, or `http://` on the local development loopback only.
/// No credentials, path, query or fragment.
pub fn valid_origin(origin: &str) -> bool {
    let (secure, authority) = if let Some(rest) = origin.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = origin.strip_prefix("http://") {
        (false, rest)
    } else {
        return false;
    };
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let host_ok = !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-']);
    let port_ok = port.is_none_or(|port| {
        (1..=5).contains(&port.len())
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|p| p > 0)
    });
    host_ok && port_ok && (secure || matches!(host, "127.0.0.1" | "localhost"))
}

#[cfg(test)]
#[path = "coin_tests.rs"]
mod tests;
