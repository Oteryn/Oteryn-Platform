//! PostgreSQL fixed-window limiter. Routes opt in with `RateLimit<Policy>` and
//! an `Extension<RateLimiter>`. Identity routes supply `Keys` from verified state.
//! Login checks both IdentityLogin (normalized email + IP) and IdentityLoginSource.
use crate::Language;
use axum::{
    extract::{ConnectInfo, FromRequestParts, Request},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug)]
enum Scope {
    Source,
    EmailSource,
    AccountSource,
    TokenSource,
    PendingSource,
    Pending,
    CredentialSource,
    Credential,
    LocaleSource,
}
/// Implemented by each named route policy below.
pub trait Policy: Send + Sync + 'static {
    const POLICY: NamedPolicy;
}
macro_rules! policies {
    ($( $variant:ident, $name:literal, $limit:literal, $seconds:literal, $scope:ident; )+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum NamedPolicy { $( $variant, )+ }
        impl NamedPolicy {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];
            pub fn name(self) -> &'static str { match self { $(Self::$variant => $name,)+ } }
            pub fn limit(self) -> i64 { match self { $(Self::$variant => $limit,)+ } }
            pub fn window_seconds(self) -> i64 { match self { $(Self::$variant => $seconds,)+ } }
            fn scope(self) -> Scope { match self { $(Self::$variant => Scope::$scope,)+ } }
        }
        $(#[derive(Debug)] pub struct $variant;
        impl Policy for $variant { const POLICY: NamedPolicy = NamedPolicy::$variant; })+
    };
}
policies! {
    PublicGameData, "public-game-data", 120, 60, Source;
    IdentityRegistration, "identity-registration", 5, 60, Source;
    IdentityLogin, "identity-login", 5, 60, EmailSource;
    IdentityLoginSource, "identity-login-source", 20, 60, Source;
    IdentityPasswordRecovery, "identity-password-recovery", 3, 60, EmailSource;
    IdentityPasswordRecoverySource, "identity-password-recovery-source", 10, 60, Source;
    IdentityPasswordReset, "identity-password-reset", 5, 60, EmailSource;
    IdentityPasswordChange, "identity-password-change", 5, 60, AccountSource;
    IdentityEmailChange, "identity-email-change", 3, 3600, AccountSource;
    IdentityAnonymousSource, "identity-anonymous-source", 10, 60, Source;
    IdentityVerificationSource, "identity-verification-source", 10, 60, Source;
    IdentityEmailToken, "identity-email-token", 10, 60, TokenSource;
    IdentitySessionRevoke, "identity-session-revoke", 10, 60, AccountSource;
    IdentitySecurityMutation, "identity-security-mutation", 10, 60, AccountSource;
    IdentityRecoveryKeyManage, "identity-recovery-key-manage", 5, 3600, AccountSource;
    IdentityRecoveryKeyUse, "identity-recovery-key-use", 5, 3600, EmailSource;
    IdentityTermination, "identity-termination", 3, 3600, AccountSource;
    IdentityMfaChallenge, "identity-mfa-challenge", 5, 60, PendingSource;
    IdentityMfaChallengeIdentity, "identity-mfa-challenge-identity", 10, 60, Pending;
    IdentityMfaChallengeSource, "identity-mfa-challenge-source", 20, 60, Source;
    IdentityMfaEnrollment, "identity-mfa-enrollment", 5, 60, AccountSource;
    IdentityMfaDisable, "identity-mfa-disable", 5, 60, AccountSource;
    CharacterCreate, "character-create", 5, 60, AccountSource;
    CharacterCreateStatus, "character-create-status", 60, 60, AccountSource;
    MarketplaceListing, "marketplace-listing", 3, 60, AccountSource;
    MarketplaceBid, "marketplace-bid", 20, 60, AccountSource;
    MarketplaceWatch, "marketplace-watch", 30, 60, AccountSource;
    SupportTicketCreate, "support-ticket-create", 6, 60, AccountSource;
    SupportTicketReply, "support-ticket-reply", 12, 60, AccountSource;
    SupportTicketStatus, "support-ticket-status", 12, 60, AccountSource;
    SupportReportSubmit, "support-report-submit", 3, 60, AccountSource;
    SupportEnforcementAcknowledge, "support-enforcement-acknowledge", 12, 60, AccountSource;
    SupportEnforcementAppeal, "support-enforcement-appeal", 3, 60, AccountSource;
    GameAuthTicketIssue, "game-auth-ticket-issue", 5, 60, CredentialSource;
    GameAuthNativeCharacters, "game-auth-native-characters", 10, 60, Credential;
    StoreCodeRedeem, "store-code-redeem", 5, 60, Pending;
    WikiSearch, "wiki-search", 30, 60, LocaleSource;
}
/// Sensitive material is deliberately omitted from Debug, logs and SQL.
/// Account and pending IDs must come from verified authentication/session state.
#[derive(Clone)]
pub struct Keys {
    pub source: IpAddr,
    pub email: Option<String>,
    pub account: Option<String>,
    pub pending_account: Option<String>,
    pub token: Option<String>,
    pub credential: Option<String>,
    pub locale: Option<Language>,
}
impl std::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Keys([redacted])")
    }
}
impl Keys {
    pub fn source(source: IpAddr) -> Self {
        Self {
            source,
            email: None,
            account: None,
            pending_account: None,
            token: None,
            credential: None,
            locale: None,
        }
    }
    fn hash(&self, policy: NamedPolicy) -> Result<Vec<u8>, Denial> {
        let mut digest = Sha256::new();
        // Length framing makes composite keys unambiguous.
        fn part(digest: &mut Sha256, value: &str) {
            digest.update((value.len() as u64).to_be_bytes());
            digest.update(value.as_bytes());
        }
        fn required(value: &Option<String>) -> Result<&str, Denial> {
            value
                .as_deref()
                .filter(|v| !v.trim().is_empty() && v.len() <= 8192)
                .ok_or(Denial::BadInput)
        }
        part(&mut digest, policy.name());
        match policy.scope() {
            Scope::EmailSource => part(&mut digest, &required(&self.email)?.trim().to_lowercase()),
            Scope::AccountSource => part(&mut digest, required(&self.account)?),
            Scope::TokenSource => part(&mut digest, required(&self.token)?),
            Scope::Pending | Scope::PendingSource => {
                part(&mut digest, required(&self.pending_account)?)
            }
            Scope::Credential | Scope::CredentialSource => {
                part(&mut digest, required(&self.credential)?)
            }
            Scope::LocaleSource => part(&mut digest, self.locale.ok_or(Denial::BadInput)?.code()),
            Scope::Source => {}
        }
        if !matches!(policy.scope(), Scope::Pending | Scope::Credential) {
            part(&mut digest, &self.source.to_string());
        }
        Ok(digest.finalize().to_vec())
    }
}
/// Exact deployment-configured proxy addresses shared with the gateway resolver.
#[derive(Clone, Debug, Default)]
pub struct TrustedProxies(pub Vec<IpAddr>);
impl TrustedProxies {
    pub fn parse(value: &str) -> Result<Self, Denial> {
        Ok(Self(
            oteryn_internal_api::proxy::TrustedProxies::parse(value)?.0,
        ))
    }
    pub fn configured() -> Result<Self, Denial> {
        Ok(Self(
            oteryn_internal_api::proxy::TrustedProxies::configured()?.0,
        ))
    }
    pub fn client_ip(&self, peer: SocketAddr, headers: &HeaderMap) -> Result<IpAddr, Denial> {
        Ok(oteryn_internal_api::proxy::TrustedProxies(self.0.clone()).client_ip(peer, headers)?)
    }
}
impl From<oteryn_internal_api::proxy::ProxyError> for Denial {
    fn from(_: oteryn_internal_api::proxy::ProxyError) -> Self {
        Self::BadInput
    }
}
/// Resolve using the same policy for every browser adapter, including merged routers.
pub fn client_source(peer: SocketAddr, headers: &HeaderMap) -> Result<IpAddr, Denial> {
    TrustedProxies::configured()?.client_ip(peer, headers)
}
/// Client source for account/source budgets; never trusts headers from arbitrary peers.
#[derive(Debug, Clone, Copy)]
pub struct ClientSource(pub IpAddr);
impl<S: Send + Sync> FromRequestParts<S> for ClientSource {
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        parts
            .extensions
            .get::<RateLimiter>()
            .map(|limiter| limiter.trusted_proxies.clone())
            .map(Ok)
            .unwrap_or_else(TrustedProxies::configured)
            .and_then(|proxies| proxies.client_ip(peer.0, &parts.headers))
            .map(Self)
            .map_err(|e| e.status())
    }
}
impl<S: Send + Sync> axum::extract::OptionalFromRequestParts<S> for ClientSource {
    type Rejection = StatusCode;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        if parts.extensions.get::<ConnectInfo<SocketAddr>>().is_none() {
            return Ok(None);
        }
        <Self as FromRequestParts<S>>::from_request_parts(parts, state)
            .await
            .map(Some)
    }
}
/// Share clones across route extensions. Database failures log once per policy window
/// per instance/process, without request details or keys.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    pool: PgPool,
    pub trusted_proxies: TrustedProxies,
    configuration_valid: bool,
    last_error_windows: Arc<Vec<AtomicU64>>,
}
impl RateLimiter {
    pub fn new(pool: PgPool) -> Self {
        let proxies = TrustedProxies::configured();
        Self {
            pool,
            configuration_valid: proxies.is_ok(),
            trusted_proxies: proxies.unwrap_or_default(),
            last_error_windows: Arc::new(
                NamedPolicy::ALL.iter().map(|_| AtomicU64::new(0)).collect(),
            ),
        }
    }
    pub async fn check(&self, policy: NamedPolicy, keys: &[Keys]) -> Result<(), Denial> {
        if !self.configuration_valid {
            return Err(Denial::Unavailable);
        }
        if matches!(
            policy,
            NamedPolicy::IdentityEmailToken | NamedPolicy::IdentityRecoveryKeyUse
        ) {
            check(self, NamedPolicy::IdentityAnonymousSource, keys).await?;
        }
        check(self, policy, keys).await
    }
    /// Charge a League browser request within its existing admission deadline.
    pub async fn check_public_game_data(
        &self,
        keys: &[Keys],
        deadline: crate::BoundedBrowserRead,
    ) -> Result<(), Denial> {
        if !self.configuration_valid {
            return Err(Denial::Unavailable);
        }
        check_with_deadline(self, NamedPolicy::PublicGameData, keys, Some(deadline)).await
    }
    pub(crate) async fn check_recovery_submission(&self, keys: Keys) -> Result<(), Denial> {
        let policy = NamedPolicy::IdentityRecoveryKeyUse;
        let mut aggregate = Keys::source(keys.source);
        aggregate.email = Some("aggregate-recovery-source".into());
        // The aggregate check also charges the shared anonymous-source budget.
        self.check(policy, &[aggregate]).await?;
        check(self, policy, &[keys]).await
    }
    fn unavailable(&self, policy: NamedPolicy) -> Denial {
        let window = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| d.as_secs() / policy.window_seconds() as u64);
        if self.last_error_windows[policy as usize].fetch_max(window, Ordering::Relaxed) < window {
            tracing::error!(
                policy = policy.name(),
                "rate limiter database unavailable; refusing traffic"
            );
        }
        Denial::Unavailable
    }
}
/// Charge each independent key; return the largest retry delay. Charges remain
/// on refusal. Partial failure never grants admission. Validate before any I/O.
pub async fn check(
    limiter: &RateLimiter,
    policy: NamedPolicy,
    keys: &[Keys],
) -> Result<(), Denial> {
    let deadline = (policy == NamedPolicy::PublicGameData).then(|| {
        crate::BoundedBrowserRead(tokio::time::Instant::now() + std::time::Duration::from_secs(1))
    });
    check_with_deadline(limiter, policy, keys, deadline).await
}

async fn check_with_deadline(
    limiter: &RateLimiter,
    policy: NamedPolicy,
    keys: &[Keys],
    deadline: Option<crate::BoundedBrowserRead>,
) -> Result<(), Denial> {
    if keys.is_empty() || keys.len() > 32 {
        return Err(Denial::BadInput);
    }
    let hashes = keys
        .iter()
        .map(|k| k.hash(policy))
        .collect::<Result<Vec<_>, _>>()?;
    let mut retry = 0_i64;
    for hash in hashes {
        // One atomic upsert using the database clock across all processes.
        // Saturate refusals to prevent overflow under sustained abuse.
        let query = sqlx::query_as::<_, (i64, i64)>(
            "WITH clock AS MATERIALIZED (SELECT statement_timestamp() AS now), \
             bucket AS (SELECT to_timestamp((floor(extract(epoch FROM now) / $3::bigint) * $3::bigint)::double precision) AS start, \
             greatest(1, ceil($3::bigint - mod(extract(epoch FROM now), $3::bigint)))::bigint AS retry FROM clock), \
             charged AS (INSERT INTO web_rate_limits (limiter_name, key_hash, window_start, count) \
             SELECT $1, $2, start, 1 FROM bucket \
             ON CONFLICT (limiter_name, key_hash, window_start) DO UPDATE \
             SET count = least(web_rate_limits.count + 1, $4 + 1) RETURNING count) \
             SELECT count, retry FROM charged CROSS JOIN bucket"
        ).bind(policy.name()).bind(hash).bind(policy.window_seconds()).bind(policy.limit());
        let (count, delay) = if let Some(deadline) = deadline {
            crate::bounded_browser_query(&limiter.pool, deadline, async |connection| {
                query.fetch_one(connection).await
            })
            .await
        } else {
            query.fetch_one(&limiter.pool).await
        }
        .map_err(|_| limiter.unavailable(policy))?;
        if count > policy.limit() {
            retry = retry.max(delay);
        }
    }
    if retry > 0 {
        Err(Denial::Limited { retry_after: retry })
    } else {
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Denial {
    Limited { retry_after: i64 },
    Unavailable,
    BadInput,
}
impl std::fmt::Display for Denial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rate limit refusal: {self:?}")
    }
}
impl std::error::Error for Denial {}
impl Denial {
    pub fn status(self) -> StatusCode {
        match self {
            Self::Limited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::BadInput => StatusCode::BAD_REQUEST,
        }
    }
    /// Reuse the shared Fluent catalogs and accessible page template.
    pub fn response(self, language: Language, api: bool) -> Response {
        let status = self.status();
        let title = language.text(&format!("error-{}-title", status.as_u16()));
        let mut response = if api {
            (
                status,
                axum::Json(serde_json::json!({"status": status.as_u16(), "error": title})),
            )
                .into_response()
        } else {
            let chrome = oteryn_ui::Registry::default().chrome(
                language.code(),
                "/",
                oteryn_ui::Account::Anonymous,
            );
            oteryn_ui::error(&chrome, status)
        };
        let delay = match self {
            Self::Limited { retry_after } => Some(retry_after.max(1)),
            Self::Unavailable => Some(60),
            Self::BadInput => None,
        };
        if let Some(delay) = delay
            && let Ok(value) = HeaderValue::from_str(&delay.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response.headers_mut().insert(
            header::CONTENT_LANGUAGE,
            HeaderValue::from_static(language.code()),
        );
        response
    }
}
/// Place before body extractors. Mount `Extension<RateLimiter>` on opted-in routes.
#[derive(Debug)]
pub struct RateLimit<P: Policy>(PhantomData<P>);
impl<S: Send + Sync, P: Policy> FromRequestParts<S> for RateLimit<P> {
    type Rejection = Response;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let lang = parts
            .extensions
            .get::<Language>()
            .copied()
            .unwrap_or(Language::Pl);
        let api = parts.uri.path() == "/api" || parts.uri.path().starts_with("/api/");
        let result = async {
            let limiter = parts
                .extensions
                .get::<RateLimiter>()
                .ok_or(Denial::Unavailable)?;
            let peer = parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .ok_or(Denial::Unavailable)?
                .0;
            let ip = limiter.trusted_proxies.client_ip(peer, &parts.headers)?;
            let mut keys = parts
                .extensions
                .get::<Keys>()
                .cloned()
                .unwrap_or_else(|| Keys::source(ip));
            // Identity extensions cannot override the transport source.
            keys.source = ip;
            keys.locale = Some(lang);
            limiter.check(P::POLICY, &[keys]).await
        }
        .await;
        result
            .map(|()| Self(PhantomData))
            .map_err(|e| e.response(lang, api))
    }
}
/// Mount outside the shared response policy to keep API limiter errors JSON
/// regardless of Accept, while preserving protocol and security headers.
pub async fn api_errors(request: Request, next: Next) -> Response {
    let api = request.uri().path() == "/api" || request.uri().path().starts_with("/api/");
    let head = request.method() == axum::http::Method::HEAD;
    let mut response = next.run(request).await;
    if api
        && matches!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
        )
    {
        let lang = if response
            .headers()
            .get(header::CONTENT_LANGUAGE)
            .is_some_and(|v| v == "en")
        {
            Language::En
        } else {
            Language::Pl
        };
        let replacement = if response.status() == StatusCode::TOO_MANY_REQUESTS {
            Denial::Limited { retry_after: 1 }
        } else {
            Denial::Unavailable
        }
        .response(lang, true);
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        response.headers_mut().remove(header::CONTENT_LENGTH);
        *response.body_mut() = if head {
            axum::body::Body::empty()
        } else {
            replacement.into_body()
        };
    }
    response
}
#[derive(Clone, Debug)]
pub struct RateLimitCleanup {
    pub pool: PgPool,
}
impl RateLimitCleanup {
    pub async fn clean(&self) -> Result<u64, sqlx::Error> {
        // All policies have windows <= one hour. Retaining an hour ensures
        // cleanup never deletes an active window, including concurrent checks.
        Ok(sqlx::query(
            "DELETE FROM web_rate_limits WHERE (limiter_name, key_hash, window_start) IN \
            (SELECT limiter_name, key_hash, window_start FROM web_rate_limits \
             WHERE window_start < statement_timestamp() - interval '1 hour' \
             LIMIT 1000 FOR UPDATE SKIP LOCKED)",
        )
        .execute(&self.pool)
        .await?
        .rows_affected())
    }
}
impl oteryn_db::outbox::Cleanup for RateLimitCleanup {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move { self.clean().await.map(|_| ()).map_err(|e| e.to_string()) })
    }
}

/// Apply the portal response policy to separately composed module routers.
/// The main portal router already applies this policy to its mounted modules.
pub fn finish_routes(router: axum::Router) -> axum::Router {
    crate::surface::finish(router).layer(axum::middleware::from_fn(api_errors))
}

/// Supply a standalone default without replacing the composition root's policy.
pub async fn supply(
    axum::extract::State(default): axum::extract::State<RateLimiter>,
    mut request: Request,
    next: Next,
) -> Response {
    if request.extensions().get::<RateLimiter>().is_none() {
        request.extensions_mut().insert(default);
    }
    next.run(request).await
}

#[cfg(test)]
mod proxy_configuration_tests {
    use super::*;
    #[test]
    fn proxy_configuration_rejects_malformed_allowlists() -> Result<(), Box<dyn std::error::Error>>
    {
        assert!(TrustedProxies::parse("")?.0.is_empty());
        assert_eq!(TrustedProxies::parse(" 127.0.0.1, ::1 ")?.0.len(), 2);
        for value in ["127.0.0.1,", "192.0.2.0/24", "proxy", "1.2.3.999"] {
            assert!(TrustedProxies::parse(value).is_err());
        }
        Ok(())
    }
}
