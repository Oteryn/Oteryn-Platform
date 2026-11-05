//! HTTP adapter: mount inside the existing browser session/CSRF middleware.
use super::{Error, check_password, consume, enabled, issue};
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
    (status, [(header::CACHE_CONTROL, "no-store")], Html(body)).into_response()
}
async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("registration.js"),
    )
}
async fn challenge(State(pool): State<PgPool>, request: Request) -> Response {
    let csrf = request
        .headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    match issue(&pool, csrf).await {
        Ok(token) => (
            [(header::CACHE_CONTROL, "no-store")],
            axum::Json(serde_json::json!({"token":token,"bits":super::DIFFICULTY})),
        )
            .into_response(),
        Err(error) => refusal(error, "pl"),
    }
}

/// Registration POSTs retain T03's validation and success response. Failures do not
/// reflect submitted credentials; a retry loads a new form challenge.
pub async fn protect(State(pool): State<PgPool>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if !matches!(path, "/register" | "/pl/register" | "/en/register") {
        return next.run(request).await;
    }
    let language = if path.starts_with("/en/") { "en" } else { "pl" };
    if let Err(error) = enabled(&pool).await {
        return refusal(error, language);
    }
    if request.method() != Method::POST {
        return next.run(request).await;
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
