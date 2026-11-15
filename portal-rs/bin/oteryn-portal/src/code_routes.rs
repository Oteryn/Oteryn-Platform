//! Authenticated browser gift-code dispatch. Item codes stay unavailable.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_store::{codes::*, premium};
use oteryn_wallet as wallet;
use oteryn_web::{
    ratelimit::{Keys, NamedPolicy, RateLimiter},
    session::{BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;
pub const GENERATE: &str = "/{lang}/admin/store/codes";
pub const REDEEM: &str = "/{lang}/shop/codes/redeem";
#[derive(Clone, Debug)]
struct Policy;
impl wallet::AccountStateReader for Policy {
    async fn state(&self, conn: &mut PgConnection, account: Uuid) -> wallet::AccountState {
        crate::wallet_topup::Policy.state(conn, account).await
    }
}
impl wallet::TransferBlockCheck for Policy {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for Policy {
    async fn adjustment(&self, conn: &mut PgConnection, e: &wallet::AdjustmentAudit) -> bool {
        if e.amount <= 0 || e.reason != "Gift code redemption" {
            return false;
        }
        let permitted = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM code_redemptions r JOIN gift_codes c ON c.id=r.code_id JOIN code_batches b ON b.id=c.batch_id WHERE r.idempotency_key=$1 AND r.account_id=$2 AND b.kind='coins' AND b.amount=$3 AND b.created_by=$4)")
            .bind(e.operation_id).bind(e.account_id).bind(e.amount).bind(e.actor_id).fetch_one(&mut *conn).await;
        if !matches!(permitted, Ok(true)) {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'store.code.coins_redeemed',$2,$3,$4,$5)")
            .bind(e.account_id).bind(e.account_id.to_string()).bind(serde_json::json!({"available":e.before.available})).bind(serde_json::json!({"available":e.after.available,"amount":e.amount})).bind(e.operation_id).execute(conn).await.is_ok()
    }
}
#[derive(Clone, Debug)]
struct Attempts(RateLimiter);
impl AttemptHook for Attempts {
    async fn check(&self, account: Uuid) -> Result<(), Error> {
        charge(&self.0, account).await.map_err(|_| Error::Denied)
    }
}
async fn charge(limiter: &RateLimiter, account: Uuid) -> Result<(), oteryn_web::ratelimit::Denial> {
    let mut keys = Keys::source(std::net::Ipv4Addr::LOCALHOST.into());
    keys.pending_account = Some(account.to_string());
    limiter.check(NamedPolicy::StoreCodeRedeem, &[keys]).await
}
type Service = Codes<Policy, Policy, Policy, Attempts>;
#[derive(Clone)]
struct Context {
    codes: Arc<Service>,
    limiter: RateLimiter,
}
fn context(pool: PgPool, wallet_enabled: bool) -> Context {
    let limiter = RateLimiter::new(pool.clone());
    Context {
        codes: Arc::new(Codes::new(
            pool.clone(),
            CodeFlags { enabled: true },
            wallet::Ledger::new(
                pool.clone(),
                wallet::WalletFlags {
                    enabled: wallet_enabled,
                },
                Policy,
                Policy,
                Policy,
            ),
            premium::repository::Ledger::new(
                pool,
                Arc::new(oteryn_store::loyalty::stream::http::DatabaseAccounts),
                true,
            ),
            Attempts(limiter.clone()),
        )),
        limiter,
    }
}
pub fn configured(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    configured_from(pool, &|key| std::env::var(key).ok())
}
fn configured_from(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    if lookup("OTERYN_STORE_CODES_ENABLED").as_deref() != Some("true") {
        return Ok(Router::new());
    }
    let state = context(
        pool.clone(),
        lookup("OTERYN_WALLET_ENABLED").as_deref() == Some("true"),
    );
    let staff = Router::new()
        .route(GENERATE, post(generate))
        .with_state(state.clone());
    let staff = oteryn_admin::web::configured_guard_from(
        staff,
        pool.clone(),
        oteryn_admin::rbac::Permission::GenerateStoreCodes,
        lookup,
    )?;
    let mut sessions = SessionLayer::new(pool);
    sessions.csrf.trusted_origins = lookup("OTERYN_PUBLIC_ORIGIN").into_iter().collect();
    let player = Router::new()
        .route(REDEEM, post(redeem))
        .route_layer(middleware::from_fn_with_state(state.clone(), budget))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .with_state(state);
    Ok(staff
        .merge(player)
        .layer(DefaultBodyLimit::max(8192))
        .layer(middleware::from_fn(private)))
}
async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    response
}
async fn budget(
    State(state): State<Context>,
    Extension(session): Extension<BrowserSession>,
    request: Request,
    next: Next,
) -> Response {
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match charge(&state.limiter, account).await {
        Ok(()) => next.run(request).await,
        Err(oteryn_web::ratelimit::Denial::Limited {
            retry_after: seconds,
        }) => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, seconds.to_string())],
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Generate {
    kind: String,
    amount: i64,
    count: usize,
    max_per_code: i32,
    max_per_account: i32,
    valid_from: i64,
    valid_until: i64,
    note: String,
}
struct CodeAuthority {
    context: oteryn_admin::rbac::CheckContext,
    auth: oteryn_admin::rbac::MutationAuthorization,
}
impl StaffAuthorization for CodeAuthority {
    async fn authorize(&self, connection: &mut PgConnection, actor: Uuid) -> Result<(), Error> {
        if actor != self.context.account {
            return Err(Error::Denied);
        }
        self.auth
            .check_in(
                connection,
                self.context,
                oteryn_admin::rbac::Permission::GenerateStoreCodes,
            )
            .await
            .map_err(|e| match e {
                oteryn_admin::rbac::Error::Storage(_) => Error::Unavailable,
                _ => Error::Denied,
            })
    }
}
async fn generate(
    State(state): State<Context>,
    Extension(context): Extension<oteryn_admin::rbac::CheckContext>,
    Extension(auth): Extension<oteryn_admin::rbac::MutationAuthorization>,
    Json(input): Json<Generate>,
) -> Response {
    let actor = context.account;
    let authority = CodeAuthority { context, auth };
    let kind = match input.kind.as_str() {
        "coins" => Kind::Coins,
        "premium_days" => Kind::PremiumDays,
        _ => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let (Ok(valid_from), Ok(valid_until)) = (
        OffsetDateTime::from_unix_timestamp(input.valid_from),
        OffsetDateTime::from_unix_timestamp(input.valid_until),
    ) else {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    };
    let batch = Batch {
        kind,
        amount: input.amount,
        max_per_code: input.max_per_code,
        max_per_account: input.max_per_account,
        valid_from,
        valid_until,
        note: input.note,
    };
    match state
        .codes
        .generate_staff(actor, &authority, &batch, input.count)
        .await
    {
        Ok(result) => Json(serde_json::json!({"batch_id":result.batch_id,"codes":result.codes}))
            .into_response(),
        Err(e) => failure(e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Redeem {
    code: String,
    operation: Uuid,
}
async fn redeem(
    State(state): State<Context>,
    Extension(session): Extension<BrowserSession>,
    Json(input): Json<Redeem>,
) -> Response {
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state
        .codes
        .redeem_player(&Redemption {
            code: input.code,
            key: input.operation,
            account,
            character_ref: None,
        })
        .await
    {
        Ok(result) => {
            Json(serde_json::json!({"redemption_id":result.redemption_id,"amount":result.amount}))
                .into_response()
        }
        Err(e) => failure(e),
    }
}
fn failure(error: Error) -> Response {
    match error {
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied | Error::Refused => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Disabled | Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
    .into_response()
}

#[cfg(test)]
#[path = "code_routes_tests.rs"]
mod tests;
