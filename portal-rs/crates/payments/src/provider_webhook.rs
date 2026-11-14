//! Generic signed provider webhook for refunds and chargebacks
//! (`docs/contracts/OTERYN_PAYMENT_PROVIDER_WEBHOOK_V1.md`). A verified event
//! is recorded once by its event ID and drives the existing coin purchase
//! reversal through `CoinPurchases::webhook`. No real provider is wired yet:
//! the route exists only when `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED=true`,
//! a signing secret is configured and the sandbox coin purchases run.
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::coin::{CoinEvent, EventState};
use crate::coin_store::CoinPurchases;
use crate::{Amount, Error, PaymentProvider, TestProvider};

pub const PATH: &str = "/api/v1/payments/provider-webhook";
pub const TIMESTAMP_HEADER: &str = "oteryn-webhook-timestamp";
pub const SIGNATURE_HEADER: &str = "oteryn-webhook-signature";
pub const VERSION: u64 = 1;
/// Strict limit on the raw body, checked before and while reading it.
pub const MAX_BODY_BYTES: usize = 16384;
/// A timestamp further than this from the portal clock is refused.
pub const TOLERANCE_SECONDS: i64 = 300;
const MIN_SECRET_BYTES: usize = 32;
const MAX_SECRET_BYTES: usize = 1024;

/// Signing secrets: the current one and, during rotation, the previous one.
/// Any missing or malformed value leaves the webhook off.
#[derive(Clone)]
pub struct WebhookSettings {
    secrets: Vec<Vec<u8>>,
}
impl std::fmt::Debug for WebhookSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookSettings").finish_non_exhaustive()
    }
}
impl WebhookSettings {
    /// `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED=true` (default off),
    /// `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET` and the optional
    /// `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET_PREVIOUS`, each 32..=1024 bytes.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        if lookup("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED").as_deref() != Some("true") {
            return None;
        }
        let valid = |secret: &String| (MIN_SECRET_BYTES..=MAX_SECRET_BYTES).contains(&secret.len());
        let current = lookup("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET").filter(valid)?;
        let mut secrets = vec![current.into_bytes()];
        if let Some(previous) = lookup("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET_PREVIOUS") {
            if !valid(&previous) {
                return None;
            }
            secrets.push(previous.into_bytes());
        }
        Some(Self { secrets })
    }
}

/// The service behind the route. `signer` must be the provider the
/// `CoinPurchases` service was built with: reversals reach it only through its
/// verified webhook.
pub struct ProviderWebhook {
    pool: PgPool,
    coins: Arc<CoinPurchases>,
    signer: TestProvider,
    settings: WebhookSettings,
}
impl std::fmt::Debug for ProviderWebhook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderWebhook").finish_non_exhaustive()
    }
}

/// Stored outcome of one event ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Applied,
    Ignored,
    UnknownOrder,
    Rejected,
    IllegalTransition,
}
impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Ignored => "ignored",
            Self::UnknownOrder => "unknown_order",
            Self::Rejected => "rejected",
            Self::IllegalTransition => "illegal_transition",
        }
    }
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "applied" => Self::Applied,
            "ignored" => Self::Ignored,
            "unknown_order" => Self::UnknownOrder,
            "rejected" => Self::Rejected,
            "illegal_transition" => Self::IllegalTransition,
            _ => return None,
        })
    }
    /// The stable refusal code of a first delivery, `None` for a 200.
    fn code(self) -> Option<&'static str> {
        match self {
            Self::Applied | Self::Ignored => None,
            Self::UnknownOrder => Some("PROVIDER_WEBHOOK_UNKNOWN_ORDER"),
            Self::Rejected => Some("PROVIDER_WEBHOOK_EVENT_REJECTED"),
            Self::IllegalTransition => Some("PROVIDER_WEBHOOK_ILLEGAL_TRANSITION"),
        }
    }
}

/// Result of one delivery, mapped one to one onto the contract's responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// 200: first delivery applied or ignored.
    Accepted { event_id: String, outcome: Outcome },
    /// 200: the same event ID with the same body; nothing applied again.
    Replay { event_id: String, outcome: Outcome },
    /// 422 with a stable code: refused and stored with that outcome.
    Refused { event_id: String, outcome: Outcome },
    /// 409: the same event ID with another body; audited, nothing applied.
    Conflict,
    /// 400: verified but malformed body.
    Invalid,
    /// 401: missing, malformed or wrong signature, or a stale timestamp.
    Unauthenticated,
    /// 413: body over `MAX_BODY_BYTES`.
    TooLarge,
    /// 503: off, or the outcome is unknown (nothing recorded; retry).
    Unavailable,
}

#[derive(Deserialize)]
struct Envelope {
    version: u64,
    event_id: String,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reversal {
    #[serde(rename = "version")]
    _version: u64,
    #[serde(rename = "event_id")]
    _event_id: String,
    #[serde(rename = "type")]
    _kind: String,
    order_reference: String,
    amount: Amount,
}

/// Printable ASCII without spaces, 1..=128 bytes.
fn event_id_valid(value: &str) -> bool {
    (1..=128).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_graphic())
}
fn type_valid(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}
fn unhex(value: &str) -> Option<Vec<u8>> {
    let digit = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| match pair {
            [high, low] => Some(digit(*high)? << 4 | digit(*low)?),
            _ => None,
        })
        .collect()
}

/// The signature base string: the decimal timestamp, `.`, the exact body.
pub fn sign(secret: &[u8], timestamp: i64, body: &[u8]) -> Option<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    Some(format!("v1={}", hex(&mac.finalize().into_bytes())))
}

fn now() -> Option<i64> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(seconds).ok()
}

/// Exactly one value of `name`, as ASCII.
fn single<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok()
}

impl ProviderWebhook {
    pub fn new(
        pool: PgPool,
        coins: Arc<CoinPurchases>,
        signer: TestProvider,
        settings: WebhookSettings,
    ) -> Self {
        Self {
            pool,
            coins,
            signer,
            settings,
        }
    }

    /// Timestamp within tolerance and one `v1=<64 hex>` signature matching a
    /// configured secret, compared in constant time.
    fn verify(&self, headers: &HeaderMap, body: &[u8]) -> bool {
        let Some(raw) = single(headers, TIMESTAMP_HEADER) else {
            return false;
        };
        if !(1..=12).contains(&raw.len()) || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        let (Ok(timestamp), Some(now)) = (raw.parse::<i64>(), now()) else {
            return false;
        };
        if (now - timestamp).abs() > TOLERANCE_SECONDS {
            return false;
        }
        let Some(signature) = single(headers, SIGNATURE_HEADER)
            .and_then(|value| value.strip_prefix("v1="))
            .filter(|value| value.len() == 64)
            .and_then(unhex)
        else {
            return false;
        };
        let mut matched = false;
        for secret in &self.settings.secrets {
            if let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) {
                mac.update(raw.as_bytes());
                mac.update(b".");
                mac.update(body);
                matched |= mac.verify_slice(&signature).is_ok();
            }
        }
        matched
    }

    /// One delivery whose body was read within `MAX_BODY_BYTES`.
    pub async fn deliver(&self, headers: &HeaderMap, body: &[u8]) -> Delivery {
        if body.len() > MAX_BODY_BYTES {
            return Delivery::TooLarge;
        }
        if !self.verify(headers, body) {
            return Delivery::Unauthenticated;
        }
        let Ok(envelope) = serde_json::from_slice::<Envelope>(body) else {
            return Delivery::Invalid;
        };
        if envelope.version != VERSION
            || !event_id_valid(&envelope.event_id)
            || !type_valid(&envelope.kind)
        {
            return Delivery::Invalid;
        }
        let reversal = match envelope.kind.as_str() {
            "refund" | "chargeback" => {
                let Ok(reversal) = serde_json::from_slice::<Reversal>(body) else {
                    return Delivery::Invalid;
                };
                if !event_id_valid(&reversal.order_reference) || reversal.amount.validate().is_err()
                {
                    return Delivery::Invalid;
                }
                Some(reversal)
            }
            _ => None,
        };
        let digest = Sha256::digest(body).to_vec();
        match self.record(&envelope, reversal.as_ref(), &digest).await {
            Ok(delivery) => delivery,
            Err(_) => Delivery::Unavailable,
        }
    }

    /// One transaction under the event ID's lock: a recorded ID is a replay or
    /// a conflict; a new one gets its effect and its row, committed together
    /// with the decision. A reversal runs through the coin purchase webhook,
    /// keyed by a sub-event ID derived from the event ID, so a retry after a
    /// lost commit is answered as that webhook's replay and applies nothing
    /// twice. An unknown effect rolls back and answers 503.
    async fn record(
        &self,
        envelope: &Envelope,
        reversal: Option<&Reversal>,
        digest: &[u8],
    ) -> Result<Delivery, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended('payment_provider_webhook:' || $1, 0))",
        )
        .bind(&envelope.event_id)
        .execute(&mut *tx)
        .await?;
        let existing: Option<(Vec<u8>, String)> = sqlx::query_as(
            "SELECT body_sha256,outcome FROM payment_provider_webhook_events WHERE event_id=$1",
        )
        .bind(&envelope.event_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((stored, outcome)) = existing {
            if stored == digest {
                tx.commit().await?;
                let outcome = Outcome::parse(&outcome).ok_or(Error::InvalidInput)?;
                return Ok(Delivery::Replay {
                    event_id: envelope.event_id.clone(),
                    outcome,
                });
            }
            sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'payment_provider_webhook.event_replay_refused',$1,$2::text::jsonb,$3::text::jsonb,$4)")
                .bind(&envelope.event_id)
                .bind(json!({"body_sha256": hex(&stored), "outcome": outcome}).to_string())
                .bind(json!({"body_sha256": hex(digest), "type": envelope.kind}).to_string())
                .bind(oteryn_core::new_id())
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(Delivery::Conflict);
        }
        let outcome = match reversal {
            None => Outcome::Ignored,
            Some(reversal) => self.reverse(&mut tx, envelope, reversal).await?,
        };
        sqlx::query("INSERT INTO payment_provider_webhook_events (event_id,body_sha256,event_type,order_reference,outcome) VALUES ($1,$2,$3,$4,$5)")
            .bind(&envelope.event_id)
            .bind(digest)
            .bind(&envelope.kind)
            .bind(reversal.map(|r| r.order_reference.as_str()))
            .bind(outcome.as_str())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let event_id = envelope.event_id.clone();
        Ok(match outcome {
            Outcome::Applied | Outcome::Ignored => Delivery::Accepted { event_id, outcome },
            _ => Delivery::Refused { event_id, outcome },
        })
    }

    /// Maps a refund or chargeback onto the coin order paid through this
    /// order reference. Only a definite answer is returned; anything else is
    /// an error, so nothing is recorded.
    async fn reverse(
        &self,
        tx: &mut sqlx::PgConnection,
        envelope: &Envelope,
        reversal: &Reversal,
    ) -> Result<Outcome, Error> {
        let intent: Option<Uuid> = sqlx::query_scalar("SELECT i.id FROM payment_intents i JOIN coin_purchase_orders o ON o.payment_intent_id=i.id WHERE i.provider=$1 AND i.provider_reference=$2")
            .bind(self.signer.name())
            .bind(&reversal.order_reference)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(intent_id) = intent else {
            return Ok(Outcome::UnknownOrder);
        };
        let event = CoinEvent {
            event_id: format!(
                "provider-webhook:{}",
                hex(&Sha256::digest(envelope.event_id.as_bytes()))
            ),
            intent_id,
            provider_reference: reversal.order_reference.clone(),
            amount: reversal.amount.clone(),
            state: if envelope.kind == "refund" {
                EventState::Refunded
            } else {
                EventState::Chargeback
            },
        };
        let body = serde_json::to_vec(&event).map_err(|_| Error::InvalidInput)?;
        let signature = self.signer.sign(&body)?;
        match self.coins.webhook(&body, &signature).await {
            Ok(_) => Ok(Outcome::Applied),
            // The amount, currency or reference differs from the order.
            Err(Error::InvalidInput) => Ok(Outcome::Rejected),
            Err(Error::IllegalTransition) => Ok(Outcome::IllegalTransition),
            Err(Error::NotFound) => Ok(Outcome::UnknownOrder),
            // Disabled, wallet or database failure, a signer mismatch: unknown.
            Err(error) => Err(error),
        }
    }
}

fn respond(delivery: Delivery) -> Response {
    let status = match &delivery {
        Delivery::Accepted { event_id, outcome } | Delivery::Replay { event_id, outcome } => {
            let replay = matches!(delivery, Delivery::Replay { .. });
            return Json(json!({
                "version": VERSION,
                "event_id": event_id,
                "outcome": outcome,
                "replay": replay,
            }))
            .into_response();
        }
        Delivery::Refused { event_id, outcome } => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "version": VERSION,
                    "event_id": event_id,
                    "code": outcome.code(),
                })),
            )
                .into_response();
        }
        Delivery::Conflict => StatusCode::CONFLICT,
        Delivery::Invalid => StatusCode::BAD_REQUEST,
        Delivery::Unauthenticated => StatusCode::UNAUTHORIZED,
        Delivery::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        Delivery::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    status.into_response()
}

async fn receive(
    State(service): State<Option<Arc<ProviderWebhook>>>,
    request: Request,
) -> Response {
    // Off or unconfigured: 503 before the body is read.
    let Some(service) = service else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let declared = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > MAX_BODY_BYTES) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let (parts, body) = request.into_parts();
    let Ok(body) = to_bytes(body, MAX_BODY_BYTES).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let mut response = respond(service.deliver(&parts.headers, &body).await);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

/// The route; without a service (the default) it answers 503.
pub fn router(service: Option<Arc<ProviderWebhook>>) -> Router {
    Router::new().route(PATH, post(receive)).with_state(service)
}

/// Composition from the environment: the coin purchase service and the
/// provider it was built with, or `None` when coin purchases are off.
pub fn configured_router(
    pool: PgPool,
    coins: Option<(Arc<CoinPurchases>, TestProvider)>,
) -> Router {
    let settings = WebhookSettings::from_lookup(|key| std::env::var(key).ok());
    let service = coins.zip(settings).map(|((coins, signer), settings)| {
        Arc::new(ProviderWebhook::new(pool, coins, signer, settings))
    });
    router(service)
}
