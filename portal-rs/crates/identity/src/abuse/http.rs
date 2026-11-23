//! HTTP adapter: mount inside the existing browser session/CSRF middleware.
use super::{Error, check_password, consume, issue_with_policy, load_policy};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;

pub fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool) -> Router<S> {
    Router::new()
        .route("/register/abuse/challenge", post(challenge))
        .route("/pl/register/abuse/challenge", post(challenge))
        .route("/en/register/abuse/challenge", post(challenge))
        .route("/register/abuse.js", get(script))
        .with_state(pool)
}

fn text(language: &str, key: &str) -> String {
    let (locale, source) = if language == "en" {
        (unic_langid::langid!("en"), include_str!("en.ftl"))
    } else {
        (unic_langid::langid!("pl"), include_str!("pl.ftl"))
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.to_owned())
        && bundle.add_resource(resource).is_ok()
        && let Some(pattern) = bundle.get_message(key).and_then(|m| m.value())
    {
        return bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned();
    }
    String::new()
}
fn refusal(error: Error, language: &str) -> Response {
    refusal_with_reason(error, language, "")
}

fn refusal_with_reason(error: Error, language: &str, reason: &str) -> Response {
    let (status, key) = match error {
        Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "work-error"),
        Error::Breached => (StatusCode::UNPROCESSABLE_ENTITY, "breach-error"),
        Error::Disabled | Error::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    let language = if language == "en" { "en" } else { "pl" };
    let body = format!(
        "<!doctype html><html lang=\"{language}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><main><p role=\"alert\">{}</p><a href=\"/{language}/register\">{}</a></main></html>",
        text(language, "title"),
        text(language, key),
        text(language, "retry")
    );
    let mut response = (status, [(header::CACHE_CONTROL, "no-store")], Html(body)).into_response();
    let message = if reason.is_empty() {
        text(language, key)
    } else {
        reason.to_owned()
    };
    response
        .extensions_mut()
        .insert(oteryn_ui::RefusalMessage(message));
    response
}
async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("registration.js"),
    )
}
/// The browser client address resolved by the web layer through its
/// trusted-proxy policy. The abuse budgets prefer it over the socket peer, so
/// clients behind a trusted reverse proxy do not share one budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientIp(pub std::net::IpAddr);

fn source(request: &Request) -> Option<String> {
    if let Some(ClientIp(ip)) = request.extensions().get::<ClientIp>() {
        return Some(ip.to_string());
    }
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|peer| peer.ip().to_string())
}

fn language(request: &Request) -> &'static str {
    match request.uri().path().split('/').nth(1) {
        Some("en") => "en",
        Some("pl") => "pl",
        _ if request
            .extensions()
            .get::<oteryn_ui::Chrome>()
            .is_some_and(|chrome| chrome.lang == "en") =>
        {
            "en"
        }
        _ => "pl",
    }
}

async fn challenge(State(pool): State<PgPool>, request: Request) -> Response {
    let language = language(&request);
    let Some(source) = source(&request) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let policy = match load_policy(&pool).await {
        Ok(policy) if policy.enabled => policy,
        Ok(policy) => {
            return refusal_with_reason(Error::Disabled, language, policy.reason(language));
        }
        Err(error) => return refusal(error, language),
    };
    if let Err(response) = budget(
        &pool,
        "identity-registration-challenge",
        &source,
        i64::from(policy.challenge_limit),
        i64::from(policy.rate_window_seconds),
    )
    .await
    {
        return *response;
    }
    let csrf = request
        .headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    match issue_with_policy(&pool, csrf, &policy).await {
        Ok(token) => (
            [(header::CACHE_CONTROL, "no-store")],
            axum::Json(serde_json::json!({"token":token,"bits":policy.work_bits})),
        )
            .into_response(),
        Err(error) => refusal(error, language),
    }
}

/// Registration POSTs retain T03's validation and success response. Failures do not
/// reflect submitted credentials; a retry loads a new form challenge.
pub async fn protect(State(pool): State<PgPool>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if !matches!(path, "/register" | "/pl/register" | "/en/register") {
        return next.run(request).await;
    }
    let language = language(&request);
    let policy = match load_policy(&pool).await {
        Ok(policy) if policy.enabled => policy,
        Ok(policy) => {
            return refusal_with_reason(Error::Disabled, language, policy.reason(language));
        }
        Err(error) => return refusal(error, language),
    };
    if request.method() != Method::POST {
        return next.run(request).await;
    }
    let Some(source) = source(&request) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if let Err(response) = budget(
        &pool,
        "identity-registration-attempt",
        &source,
        i64::from(policy.attempt_limit),
        i64::from(policy.rate_window_seconds),
    )
    .await
    {
        return *response;
    }
    let body = std::mem::replace(request.body_mut(), Body::empty());
    let bytes = match to_bytes(body, 16 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return refusal(Error::Invalid, language),
    };
    // Preserve T03's field validation response before consuming work. Invalid
    // fields cannot reach account creation, even when proof is absent.
    if let Ok(input) = serde_urlencoded::from_bytes::<crate::register::Registration>(&bytes) {
        let config = request
            .extensions()
            .get::<crate::consents::ConsentConfig>()
            .cloned()
            .unwrap_or_default();
        if !crate::register::validate(&input, &config).is_empty() {
            *request.body_mut() = Body::from(bytes);
            return next.run(request).await;
        }
    }
    let fields: Vec<(String, String)> = match serde_urlencoded::from_bytes(&bytes) {
        Ok(fields) => fields,
        Err(_) => return refusal(Error::Invalid, language),
    };
    let field = |name: &str| -> Result<&str, Error> {
        let mut values = fields.iter().filter(|(key, _)| key == name);
        let value = values.next().ok_or(Error::Invalid)?.1.as_str();
        if values.next().is_some() {
            return Err(Error::Invalid);
        }
        Ok(value)
    };
    let proof = async {
        consume(
            &pool,
            field("_csrf")?,
            field("work_token")?,
            field("work_nonce")?,
        )
        .await?;
        check_password(&pool, field("password")?).await
    }
    .await;
    if let Err(error) = proof {
        return refusal(error, language);
    }
    *request.body_mut() = Body::from(bytes);
    next.run(request).await
}

/// Atomic, process-independent fixed-window budget for identity-owned adapters.
pub async fn budget(
    pool: &PgPool,
    policy: &str,
    key: &str,
    limit: i64,
    seconds: i64,
) -> Result<(), Box<Response>> {
    use sha2::{Digest, Sha256};
    let result: Result<(i64,i64),_> = sqlx::query_as("WITH clock AS MATERIALIZED (SELECT clock_timestamp() AS t), bucket AS (SELECT to_timestamp((floor(extract(epoch FROM t)/$3::bigint)*$3)::double precision) AS start,greatest(1,ceil($3::bigint-mod(extract(epoch FROM t),$3::bigint)))::bigint AS retry FROM clock), charged AS (INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count) SELECT $1,$2,start,1 FROM bucket ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE SET count=least(web_rate_limits.count+1,$4+1) RETURNING count) SELECT count,retry FROM charged CROSS JOIN bucket")
        .bind(policy).bind(Sha256::digest(key.as_bytes()).to_vec()).bind(seconds).bind(limit).fetch_one(pool).await;
    match result {
        Ok((count, _)) if count <= limit => Ok(()),
        Ok((_, retry)) => {
            let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
            if let Ok(value) = retry.to_string().parse() {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
            Err(Box::new(response))
        }
        Err(_) => Err(Box::new(StatusCode::SERVICE_UNAVAILABLE.into_response())),
    }
}
