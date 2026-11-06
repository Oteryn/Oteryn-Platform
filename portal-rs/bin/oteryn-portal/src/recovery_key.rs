//! Bind recovery-key routes to the real browser session and existing abuse policies.
use axum::{
    Extension, Router,
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use oteryn_identity::{
    accounts::normalize_email,
    recovery_key::{Enabled, FEATURE, FeatureFlags},
    recovery_key_http::{BrowserContext, RateFuture},
};
use oteryn_web::{
    Language,
    ratelimit::{Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionKey, SessionLayer},
};
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};

const FLASH: SessionKey<String> = SessionKey::new("recovery_key_notice");

pub fn configured(app: Router, pool: PgPool) -> Router {
    if std::env::var("OTERYN_ACCOUNT_RECOVERY_KEYS").as_deref() != Ok("true") {
        return app;
    }
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let routes = oteryn_identity::recovery_key_http::routes(pool.clone())
        .layer(Extension(Enabled(true)))
        .route_layer(middleware::from_fn_with_state(
            RateLimiter::new(pool),
            context,
        ))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ));
    let mut features = FeatureFlags::new();
    features.insert(FEATURE.to_owned());
    // Keep the domain's conservative reset adapter: global recovery stays refused
    // until all credential families can be revoked in its transaction.
    app.merge(oteryn_web::ratelimit::finish_routes(routes))
        .layer(Extension(features))
}

async fn context(
    State(limiter): State<RateLimiter>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session = request
        .extensions()
        .get::<BrowserSession>()
        .cloned()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let language = *request
        .extensions()
        .get::<Language>()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let chrome = request
        .extensions()
        .get::<oteryn_ui::Chrome>()
        .cloned()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .0;
    let source = limiter
        .trusted_proxies
        .client_ip(peer, request.headers())
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let account = session.account_id().await;
    let managing = request.uri().path().contains("/account/");
    let flash_session = session.clone();
    let notice = session
        .get(FLASH)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .unwrap_or_default();
    session
        .set(FLASH, &String::new())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    request.extensions_mut().insert(BrowserContext {
        account,
        chrome,
        csrf,
        notice,
        language: language.code(),
        flash: Arc::new(move |message: &'static str| -> RateFuture {
            let session = flash_session.clone();
            Box::pin(async move {
                session
                    .set(FLASH, &message.to_owned())
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())
            })
        }),
        rate: Arc::new(move |email: String| -> RateFuture {
            let limiter = limiter.clone();
            Box::pin(async move {
                let mut keys = Keys::source(source);
                let policy = if managing {
                    keys.account = account.map(|id| id.to_string());
                    NamedPolicy::IdentityRecoveryKeyManage
                } else {
                    limiter
                        .check(NamedPolicy::IdentityPasswordRecoverySource, &[keys.clone()])
                        .await
                        .map_err(|denial| denial.response(language, false))?;
                    keys.email = Some(normalize_email(&email));
                    NamedPolicy::IdentityRecoveryKeyUse
                };
                limiter
                    .check(policy, &[keys])
                    .await
                    .map_err(|denial| denial.response(language, false))
            })
        }),
    });
    Ok(next.run(request).await)
}
