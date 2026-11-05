//! Private player reads. The bearer authority must validate expiry, revocation and scope.
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_db::outbox::{PgPool, Uuid};
use oteryn_identity::accounts::Accounts;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};

pub type ReadFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StatusCode>> + Send + 'a>>;
/// Implementations return an owner only for a live, scoped player access token.
/// Browser cookies, service tokens and client-supplied account identifiers are not credentials.
pub trait BearerAuthority: std::fmt::Debug + Send + Sync {
    fn owner<'a>(&'a self, token: &'a str) -> ReadFuture<'a, Option<Uuid>>;
}
#[derive(Debug)]
pub struct DenyBearer;
impl BearerAuthority for DenyBearer {
    fn owner<'a>(&'a self, _: &'a str) -> ReadFuture<'a, Option<Uuid>> {
        Box::pin(async { Ok(None) })
    }
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub account_id: Uuid,
    pub locale: String,
    /// None means unavailable, never a fabricated zero balance.
    pub coins: Option<i64>,
}
#[derive(Debug, Serialize)]
pub struct FeedItem {
    pub id: Uuid,
    pub subject: String,
    pub body: String,
    pub read: bool,
}
/// Narrow read seam for wallet integration, without querying another crate's tables.
pub trait PlayerReads: std::fmt::Debug + Send + Sync {
    fn summary(&self, owner: Uuid) -> ReadFuture<'_, Summary>;
    fn feed(&self, owner: Uuid, limit: i64, unread: bool) -> ReadFuture<'_, Vec<FeedItem>>;
}
/// Read-only adapter to the wallet's public ledger; no cross-crate table access.
pub trait CoinBalance: std::fmt::Debug + Send + Sync {
    fn available(&self, owner: Uuid) -> ReadFuture<'_, i64>;
}
impl<A, B, H> CoinBalance for oteryn_wallet::Ledger<A, B, H>
where
    A: oteryn_wallet::AccountStateReader + std::fmt::Debug,
    B: oteryn_wallet::TransferBlockCheck + std::fmt::Debug,
    H: oteryn_wallet::AuditHook + std::fmt::Debug,
{
    fn available(&self, owner: Uuid) -> ReadFuture<'_, i64> {
        Box::pin(async move {
            self.balance(owner)
                .await
                .map(|balance| balance.available)
                .map_err(|error| match error {
                    oteryn_wallet::Error::Refused(oteryn_wallet::Refusal::AccountDenied) => {
                        StatusCode::FORBIDDEN
                    }
                    _ => StatusCode::SERVICE_UNAVAILABLE,
                })
        })
    }
}
#[derive(Debug)]
pub struct PrivateReads {
    pub pool: PgPool,
    /// Separate money flag, off unless explicitly enabled by composition.
    pub money_enabled: bool,
    pub wallet: Option<Arc<dyn CoinBalance>>,
    pub notifications: Arc<oteryn_notify::Notifications>,
}
impl PrivateReads {
    async fn active(&self, owner: Uuid) -> Result<String, StatusCode> {
        let account = Accounts {
            pool: self.pool.clone(),
        }
        .find_by_id(owner)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::FORBIDDEN)?;
        if account.status != "active"
            || account.email_verified_at.is_none()
            || !matches!(account.locale.as_str(), "pl" | "en")
        {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(account.locale)
    }
}
impl PlayerReads for PrivateReads {
    fn summary(&self, owner: Uuid) -> ReadFuture<'_, Summary> {
        Box::pin(async move {
            let locale = self.active(owner).await?;
            let coins = if self.money_enabled {
                let wallet = self
                    .wallet
                    .as_ref()
                    .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
                let amount = wallet.available(owner).await?;
                if amount < 0 {
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
                Some(amount)
            } else {
                None
            };
            Ok(Summary {
                account_id: owner,
                locale,
                coins,
            })
        })
    }
    fn feed(&self, owner: Uuid, limit: i64, unread: bool) -> ReadFuture<'_, Vec<FeedItem>> {
        Box::pin(async move {
            self.active(owner).await?;
            let items = self
                .notifications
                .list(owner, unread, limit)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(items
                .into_iter()
                .map(|n| FeedItem {
                    id: n.id,
                    subject: n.subject,
                    body: n.body,
                    read: n.read_at.is_some(),
                })
                .collect())
        })
    }
}
#[derive(Debug, Clone)]
pub struct Api {
    /// Must be explicitly enabled by the composition root; defaults to false.
    pub enabled: bool,
    pub authority: Arc<dyn BearerAuthority>,
    pub reads: Arc<dyn PlayerReads>,
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    limit: Option<i64>,
    unread_only: Option<bool>,
}
impl Api {
    async fn owner(&self, headers: &HeaderMap) -> Result<Uuid, StatusCode> {
        if !self.enabled {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let mut values = headers.get_all(header::AUTHORIZATION).iter();
        let value = values
            .next()
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if values.next().is_some() {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let (scheme, token) = value.split_once(' ').ok_or(StatusCode::UNAUTHORIZED)?;
        if !scheme.eq_ignore_ascii_case("bearer")
            || token.is_empty()
            || token.len() > 8192
            || !token.bytes().all(|c| c.is_ascii_graphic())
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        self.authority
            .owner(token)
            .await?
            .filter(|id| id.get_version_num() == 7)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
pub fn router(api: Api) -> Router {
    Router::new()
        .route("/api/v1/account/summary", get(summary))
        .route("/api/v1/feed", get(feed))
        .with_state(api)
        .layer(axum::middleware::from_fn(private_headers))
}
/// Until the OAuth authority is available, production wiring is deliberately closed.
pub fn configured_router() -> Router {
    #[derive(Debug)]
    struct Unavailable;
    impl PlayerReads for Unavailable {
        fn summary(&self, _: Uuid) -> ReadFuture<'_, Summary> {
            Box::pin(async { Err(StatusCode::SERVICE_UNAVAILABLE) })
        }
        fn feed(&self, _: Uuid, _: i64, _: bool) -> ReadFuture<'_, Vec<FeedItem>> {
            Box::pin(async { Err(StatusCode::SERVICE_UNAVAILABLE) })
        }
    }
    router(Api {
        enabled: false,
        authority: Arc::new(DenyBearer),
        reads: Arc::new(Unavailable),
    })
}
async fn summary(
    State(api): State<Api>,
    headers: HeaderMap,
    query: Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Summary>, StatusCode> {
    let owner = api.owner(&headers).await?;
    if !query.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(api.reads.summary(owner).await?))
}
async fn feed(
    State(api): State<Api>,
    headers: HeaderMap,
    query: Result<Query<Options>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let owner = api.owner(&headers).await?;
    let Query(options) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = options.limit.unwrap_or(20);
    if !(1..=100).contains(&limit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(
        serde_json::json!({"items": api.reads.feed(owner, limit, options.unread_only.unwrap_or(false)).await?}),
    ))
}
async fn private_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        header::VARY,
        header::HeaderValue::from_static("Authorization"),
    );
    response
}
