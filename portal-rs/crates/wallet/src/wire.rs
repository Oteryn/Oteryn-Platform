//! Game-facing coin wire. Only the trusted mTLS listener supplies peer provenance.
use crate::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy},
};
use std::sync::Arc;

const PREFIX: &str = "/internal/v1/coins/";
const GROUP: &str = "game-coins";
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub transfers: bool,
    pub market: bool,
}
struct HttpState<A, B, H> {
    ledger: Arc<Ledger<A, B, H>>,
}
impl<A, B, H> Clone for HttpState<A, B, H> {
    fn clone(&self) -> Self {
        Self {
            ledger: self.ledger.clone(),
        }
    }
}
#[derive(Clone)]
struct GetBody(Arc<std::sync::Mutex<Option<Body>>>);

pub fn router<A, B, H>(ledger: Ledger<A, B, H>, flags: Flags, identities: IdentityMap) -> Router
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
{
    let state = HttpState {
        ledger: Arc::new(ledger),
    };
    let identities = Arc::new(identities);
    let mut app = Router::new();
    for (path, is_get, enabled) in [
        ("transfers", false, flags.transfers),
        ("market/offers", false, flags.market),
        ("market/settlements", false, flags.market),
        ("market/cancellations", false, flags.market),
        (
            "operations/{operation_id}",
            true,
            flags.transfers || flags.market,
        ),
        (
            "accounts/{account_id}/balance",
            true,
            flags.transfers || flags.market,
        ),
        (
            "accounts/{account_id}/history",
            true,
            flags.transfers || flags.market,
        ),
    ] {
        let handler = if is_get {
            get(handle::<A, B, H>)
        } else {
            post(handle::<A, B, H>)
        };
        app = app.merge(guard::protect(
            Router::new().route(&format!("{PREFIX}{path}"), handler),
            identities.clone(),
            RoutePolicy {
                enabled,
                group: GROUP.into(),
                max_body_bytes: 1024,
                max_json_depth: 4,
                json: JsonRule::Object,
            },
        ));
    }
    app.with_state(state).layer(middleware::from_fn(adapt))
}
async fn adapt(mut req: Request, next: Next) -> Response {
    if req.method() == Method::GET {
        let (parts, body) = req.into_parts();
        req = Request::from_parts(parts, Body::from("{}"));
        req.extensions_mut()
            .insert(GetBody(Arc::new(std::sync::Mutex::new(Some(body)))));
        req.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        req.headers_mut().remove(header::CONTENT_LENGTH);
    }
    let response = next.run(req).await;
    match response.status() {
        StatusCode::UNAUTHORIZED => response,
        StatusCode::SERVICE_UNAVAILABLE => failure("unavailable"),
        StatusCode::BAD_REQUEST
        | StatusCode::PAYLOAD_TOO_LARGE
        | StatusCode::UNSUPPORTED_MEDIA_TYPE => failure("invalid"),
        _ => response,
    }
}
fn text(s: impl ToString) -> Value {
    Value::String(s.to_string())
}
fn obj(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn timestamp(at: time::OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second(),
        at.nanosecond()
    )
}
fn response(status: StatusCode, value: Value) -> Response {
    match canonical::encode(&value) {
        Ok(bytes) if bytes.len() <= 65536 => (
            status,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            "{\"result\":\"unavailable\"}",
        )
            .into_response(),
    }
}
fn failure(code: &str) -> Response {
    let status = match code {
        "insufficient_transferable"
        | "account_inactive"
        | "transfer_blocked"
        | "idempotency_conflict" => StatusCode::CONFLICT,
        "unknown_account" | "invalid" => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    response(status, obj(vec![("result", text(code))]))
}
fn error(err: Error) -> Response {
    let code = match err {
        Error::Refused(Refusal::InsufficientFunds | Refusal::ReservationSettled) => {
            "insufficient_transferable"
        }
        Error::Refused(Refusal::AccountDenied) => "account_inactive",
        Error::Refused(Refusal::UnknownAccount) => "unknown_account",
        // coin-wire-v1 has no debt code: outstanding debt blocks the transfer.
        Error::Refused(Refusal::TransferBlocked | Refusal::DebtOutstanding) => "transfer_blocked",
        Error::Refused(Refusal::IdempotencyConflict) => "idempotency_conflict",
        Error::Refused(
            Refusal::Disabled | Refusal::TransferStateUnknown | Refusal::AuditDenied,
        )
        | Error::Storage(_) => "unavailable",
        _ => "invalid",
    };
    failure(code)
}
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str, Error> {
    match v.get(k) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Refusal::InvalidIdentifier.into()),
    }
}
fn uuid(s: &str, key: bool) -> Result<uuid::Uuid, Error> {
    let id = uuid::Uuid::parse_str(s).map_err(|_| Refusal::InvalidIdentifier)?;
    if id.is_nil() || id.to_string() != s || (key && id.get_version_num() != 7) {
        return Err(Refusal::InvalidIdentifier.into());
    }
    Ok(id)
}
fn id(v: &Value, k: &str, key: bool) -> Result<uuid::Uuid, Error> {
    uuid(string(v, k)?, key)
}
fn amount(v: &Value, k: &str) -> Result<i64, Error> {
    let s = string(v, k)?;
    let n = s.parse::<i64>().map_err(|_| Refusal::InvalidAmount)?;
    if n <= 0 || n.to_string() != s {
        return Err(Refusal::InvalidAmount.into());
    }
    Ok(n)
}
fn receipt(r: Receipt) -> Response {
    response(
        StatusCode::OK,
        obj(vec![
            ("result", text(if r.replayed { "replay" } else { "ok" })),
            (
                "receipt",
                obj(vec![
                    ("operation_id", text(r.operation_id)),
                    ("kind", text(r.kind)),
                    ("account_id", text(r.account_id)),
                    (
                        "counterparty_account_id",
                        r.counterparty_account_id.map_or(Value::Null, text),
                    ),
                    ("amount", text(r.amount)),
                    // One coin class (owner decision 2026-11-12): the coin-wire-v1
                    // members stay; transferable is the whole balance.
                    ("transferable_after", text(r.available_after)),
                    ("non_transferable_after", text(0)),
                    ("reserved_after", text(r.reserved_after)),
                    ("occurred_at", text(timestamp(r.occurred_at))),
                ]),
            ),
        ]),
    )
}
async fn handle<A, B, H>(State(s): State<HttpState<A, B, H>>, req: Request) -> Response
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
{
    match execute(&s.ledger, req).await {
        Ok(response) => response,
        Err(e) => error(e),
    }
}
async fn execute<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook>(
    ledger: &Ledger<A, B, H>,
    req: Request,
) -> Result<Response, Error> {
    let path = req
        .uri()
        .path()
        .strip_prefix(PREFIX)
        .ok_or(Refusal::InvalidIdentifier)?
        .to_owned();
    if req.method() == Method::GET {
        // The shared guard has already checked flags and verified provenance.
        // Never poll the original GET body on a disabled or unauthenticated route.
        let original = req
            .extensions()
            .get::<GetBody>()
            .ok_or(Refusal::InvalidIdentifier)?;
        let body = {
            let mut body = original.0.lock().map_err(|_| Refusal::InvalidIdentifier)?;
            body.take().ok_or(Refusal::InvalidIdentifier)?
        };
        if !to_bytes(body, 1).await.is_ok_and(|bytes| bytes.is_empty()) {
            return Err(Refusal::InvalidIdentifier.into());
        }
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() == 2 && parts[0] == "operations" {
            if req.uri().query().is_some() {
                return Err(Refusal::InvalidPage.into());
            }
            return ledger
                .operation(uuid(parts[1], true)?)
                .await?
                .filter(|r| {
                    matches!(
                        r.kind.as_str(),
                        "transfer" | "reserve" | "settlement" | "release"
                    )
                })
                .map(receipt)
                .ok_or(Refusal::UnknownAccount.into());
        }
        if parts.len() != 3 || parts[0] != "accounts" {
            return Err(Refusal::InvalidIdentifier.into());
        }
        let account = uuid(parts[1], false)?;
        ledger.coin_account(account).await?;
        if parts[2] == "balance" {
            if req.uri().query().is_some() {
                return Err(Refusal::InvalidPage.into());
            }
            let b = ledger.balance(account).await?;
            return Ok(response(
                StatusCode::OK,
                obj(vec![
                    ("result", text("ok")),
                    ("account_id", text(account)),
                    // One coin class: the whole balance is transferable.
                    ("transferable", text(b.available)),
                    ("non_transferable", text(0)),
                    ("reserved", text(b.reserved)),
                ]),
            ));
        }
        let mut page = HistoryPage::default();
        let mut seen = std::collections::HashSet::new();
        if let Some(q) = req.uri().query() {
            for pair in q.split('&') {
                let (key, value) = pair.split_once('=').ok_or(Refusal::InvalidPage)?;
                if !seen.insert(key) {
                    return Err(Refusal::InvalidPage.into());
                }
                match key {
                    "cursor" => {
                        page.before = if value.is_empty() {
                            None
                        } else {
                            Some(uuid(value, true)?)
                        }
                    }
                    "limit" => {
                        page.limit = value.parse().map_err(|_| Refusal::InvalidPage)?;
                        if page.limit.to_string() != value {
                            return Err(Refusal::InvalidPage.into());
                        }
                    }
                    _ => return Err(Refusal::InvalidPage.into()),
                }
            }
        }
        let entries = ledger.history(account, page).await?;
        let cursor = entries.last().map_or(Value::Null, |e| text(e.id));
        let values = entries
            .into_iter()
            .map(|e| {
                obj(vec![
                    ("entry_id", text(e.id)),
                    ("operation_id", text(e.operation_id)),
                    ("kind", text(e.kind)),
                    // One coin class: every entry, old or new, is transferable.
                    ("coin_class", text("transferable")),
                    (
                        "counterparty_account_id",
                        e.counterparty_account_id.map_or(Value::Null, text),
                    ),
                    ("available_delta", text(e.available_delta)),
                    ("reserved_delta", text(e.reserved_delta)),
                    ("occurred_at", text(timestamp(e.created_at))),
                ])
            })
            .collect();
        return Ok(response(
            StatusCode::OK,
            obj(vec![
                ("result", text("ok")),
                ("entries", Value::Array(values)),
                ("next_cursor", cursor),
            ]),
        ));
    }
    if req.uri().query().is_some() {
        return Err(Refusal::InvalidIdentifier.into());
    }
    let fields: &[&str] = match path.as_str() {
        "transfers" => &[
            "operation_id",
            "sender_account_id",
            "recipient_account_id",
            "amount",
        ],
        "market/offers" => &["operation_id", "seller_account_id", "amount", "gold_price"],
        "market/settlements" => &[
            "operation_id",
            "reservation_id",
            "buyer_account_id",
            "amount",
        ],
        "market/cancellations" => &["operation_id", "seller_account_id", "reservation_id"],
        _ => return Err(Refusal::InvalidIdentifier.into()),
    };
    let value = req
        .extensions()
        .get::<Value>()
        .ok_or(Refusal::InvalidIdentifier)?;
    let Value::Object(members) = value else {
        return Err(Refusal::InvalidIdentifier.into());
    };
    if members
        .iter()
        .map(|(k, _)| k.as_str())
        .ne(fields.iter().copied())
    {
        return Err(Refusal::InvalidIdentifier.into());
    }
    let raw = to_bytes(req.into_body(), 1024)
        .await
        .map_err(|_| Refusal::InvalidIdentifier)?;
    // Re-decode after consuming the request so no request borrow crosses a move.
    let value = canonical::decode(&raw, 1024, 4).map_err(|_| Refusal::InvalidIdentifier)?;
    if raw.contains(&b'\\')
        || canonical::encode(&value).map_err(|_| Refusal::InvalidIdentifier)? != raw
    {
        return Err(Refusal::InvalidIdentifier.into());
    }
    let key = id(&value, "operation_id", true)?;
    let used = binding(ledger, &path, key, &value).await?;
    if matches!(path.as_str(), "market/settlements" | "market/cancellations") {
        let reservation = id(&value, "reservation_id", true)?;
        // Operation bindings are immutable. A used key must reach the ledger's
        // replay/conflict check; a new key can consume only a SELL-offer hold.
        let admissible: bool = if used {
            true
        } else {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wallet_operations WHERE id=$1 AND kind='reserve' AND reason LIKE 'gold-price:%')")
                .bind(reservation).fetch_one(&ledger.pool).await?
        };
        if !admissible {
            return Err(Refusal::ReservationMissing.into());
        }
    }
    let r = match path.as_str() {
        "transfers" => {
            let a = id(&value, "sender_account_id", false)?;
            let b = id(&value, "recipient_account_id", false)?;
            ledger.coin_account(a).await?;
            ledger.coin_account(b).await?;
            ledger
                .transfer(key, a, b, amount(&value, "amount")?)
                .await?
        }
        "market/offers" => {
            let a = id(&value, "seller_account_id", false)?;
            ledger.coin_account(a).await?;
            ledger
                .reserve_offer(
                    key,
                    a,
                    amount(&value, "amount")?,
                    amount(&value, "gold_price")?,
                )
                .await?
        }
        "market/settlements" => {
            let b = id(&value, "buyer_account_id", false)?;
            ledger.coin_account(b).await?;
            ledger
                .settle(
                    key,
                    id(&value, "reservation_id", true)?,
                    b,
                    amount(&value, "amount")?,
                )
                .await?
        }
        _ => {
            let a = id(&value, "seller_account_id", false)?;
            ledger.coin_account(a).await?;
            ledger
                .release(key, a, id(&value, "reservation_id", true)?)
                .await?
        }
    };
    Ok(receipt(r))
}

// Validate used keys before account/reservation lookup so changing a valid
// identifier to an absent one still produces the required payload conflict.
async fn binding<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook>(
    ledger: &Ledger<A, B, H>,
    path: &str,
    key: uuid::Uuid,
    value: &Value,
) -> Result<bool, Error> {
    let kind = match path {
        "transfers" => "transfer",
        "market/offers" => "reserve",
        "market/settlements" => "settlement",
        _ => "release",
    };
    let account = match path {
        "transfers" => Some(id(value, "sender_account_id", false)?),
        "market/offers" | "market/cancellations" => Some(id(value, "seller_account_id", false)?),
        _ => None,
    };
    let counterparty = match path {
        "transfers" => Some(id(value, "recipient_account_id", false)?),
        "market/settlements" => Some(id(value, "buyer_account_id", false)?),
        _ => None,
    };
    if path == "transfers" && account == counterparty {
        return Err(Refusal::SameAccount.into());
    }
    let quantity = if path == "market/cancellations" {
        None
    } else {
        Some(amount(value, "amount")?)
    };
    let reference = if matches!(path, "market/settlements" | "market/cancellations") {
        Some(id(value, "reservation_id", true)?)
    } else {
        None
    };
    let reason = if path == "market/offers" {
        Some(format!("gold-price:{}", amount(value, "gold_price")?))
    } else {
        None
    };
    let same:Option<bool> = sqlx::query_scalar("SELECT kind=$2 AND ($3::uuid IS NULL OR account_id=$3) AND counterparty_id IS NOT DISTINCT FROM $4 AND ($5::bigint IS NULL OR amount=$5) AND reference_id IS NOT DISTINCT FROM $6 AND reason IS NOT DISTINCT FROM $7 FROM wallet_operations WHERE id=$1")
        .bind(key).bind(kind).bind(account).bind(counterparty).bind(quantity).bind(reference).bind(reason).fetch_optional(&ledger.pool).await?;
    match same {
        Some(false) => Err(Refusal::IdempotencyConflict.into()),
        Some(true) => Ok(true),
        None => Ok(false),
    }
}

/// Production adapters deny on missing or failed authoritative state. Grants are
/// deliberately unavailable on this consumer-only surface.
#[derive(Debug, Clone, Copy)]
pub struct DatabasePolicy;
impl AccountStateReader for DatabasePolicy {
    async fn state(&self, conn: &mut sqlx::PgConnection, account: uuid::Uuid) -> AccountState {
        match sqlx::query_scalar::<_,bool>("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE").bind(account).fetch_optional(conn).await {
            Ok(Some(true))=>AccountState::Active,Ok(Some(false))=>AccountState::Inactive,Ok(None)=>AccountState::Unknown,Err(_)=>AccountState::Unavailable,
        }
    }
}
impl TransferBlockCheck for DatabasePolicy {
    async fn permission(
        &self,
        conn: &mut sqlx::PgConnection,
        account: uuid::Uuid,
    ) -> TransferPermission {
        match sqlx::query_scalar::<_,bool>("SELECT status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=now() FROM accounts WHERE id=$1 FOR SHARE").bind(account).fetch_optional(conn).await {
            Ok(Some(true))=>TransferPermission::Allowed,Ok(Some(false))=>TransferPermission::Blocked,_=>TransferPermission::Unknown,
        }
    }
}
impl AuditHook for DatabasePolicy {
    async fn adjustment(&self, _: &mut sqlx::PgConnection, _: &AdjustmentAudit) -> bool {
        false
    }
}
/// Identity-map object format matches existing internal route identity variables.
/// All configured peer maps participate in overlap detection.
pub fn configured_router(pool: sqlx::PgPool) -> Router {
    let flags = Flags {
        transfers: std::env::var("PORTAL_COIN_TRANSFERS_ENABLED").as_deref() == Ok("true"),
        market: std::env::var("PORTAL_COIN_MARKET_ENABLED").as_deref() == Ok("true"),
    };
    let config = identities(std::env::vars().collect());
    let valid = config.is_ok();
    let ledger = Ledger::new(
        pool,
        WalletFlags {
            enabled: valid && (flags.transfers || flags.market),
        },
        DatabasePolicy,
        DatabasePolicy,
        DatabasePolicy,
    );
    router(
        ledger,
        if valid { flags } else { Flags::default() },
        config.unwrap_or_default(),
    )
}
pub fn identities(vars: Vec<(String, String)>) -> Result<IdentityMap, guard::ConfigError> {
    let mut bindings = Vec::new();
    let mut coins = false;
    for (key, raw) in vars {
        if key.ends_with("_IDENTITIES") {
            let Value::Object(members) =
                canonical::decode(raw.as_bytes(), 131072, 4).map_err(|_| guard::ConfigError)?
            else {
                return Err(guard::ConfigError);
            };
            let own = key == "PORTAL_GAME_COINS_IDENTITIES";
            if own {
                coins = !members.is_empty();
            }
            for (san, value) in members {
                if own && !matches!(value,Value::Array(ref a) if a.is_empty()) {
                    return Err(guard::ConfigError);
                }
                bindings.push(Binding {
                    identity: Identity::san(&san)?,
                    group: if own { GROUP.into() } else { key.clone() },
                });
            }
        } else if key.ends_with("_IDENTITY") || key.ends_with("_SAN") {
            bindings.push(Binding {
                identity: Identity::san(&raw)?,
                group: key,
            });
        }
    }
    if !coins {
        return Err(guard::ConfigError);
    }
    IdentityMap::new(bindings)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nwave_coins_refuse_sanction_identity_overlap() {
        for key in [
            "PORTAL_SANCTION_EXPORT_IDENTITY",
            "PORTAL_LOYALTY_STREAM_IDENTITY",
        ] {
            let config = |san: &str| {
                vec![
                    (
                        "PORTAL_GAME_COINS_IDENTITIES".into(),
                        r#"{"coins.example.invalid":[]}"#.into(),
                    ),
                    (key.into(), san.into()),
                ]
            };
            assert!(
                super::identities(config("coins.example.invalid")).is_err(),
                "{key}"
            );
            assert!(super::identities(config("reader.example.invalid")).is_ok());
        }
    }
}
