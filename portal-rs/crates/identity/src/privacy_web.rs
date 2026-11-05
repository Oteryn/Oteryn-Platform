//! Binary composition adapter using the public browser session API.
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_identity::privacy::{Preferences, Privacy};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
#[derive(Clone)]
struct PrivacyState {
    pool: PgPool,
    policy: Privacy,
    limiter: oteryn_web::ratelimit::RateLimiter,
}
pub fn configured_router(pool: PgPool) -> Router {
    router(
        pool,
        std::env::var("OTERYN_ACCOUNT_PRIVACY_ENABLED").is_ok_and(|v| v == "true"),
    )
}
fn router(pool: PgPool, enabled: bool) -> Router {
    if !enabled {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool.clone());
    if let Ok(origin) = std::env::var("OTERYN_PUBLIC_ORIGIN") {
        sessions.csrf.trusted_origins.push(origin);
    }
    Router::new()
        .route(
            "/{lang}/account/privacy",
            get(show).post(update).put(update),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .with_state(PrivacyState {
            limiter: oteryn_web::ratelimit::RateLimiter::new(pool.clone()),
            pool,
            policy: Privacy { enabled },
        })
        .layer(axum::middleware::from_fn(private_response))
}
async fn private_response(request: Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    for (name, value) in [
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
        (
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, header::HeaderValue::from_static(value));
    }
    response
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/app.css"><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><main class="op-stack"><h1>{{ title }}</h1>{% if invalid %}<p role="alert">{{ error }}</p>{% endif %}<form method="post" class="op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><label class="op-row"><input type="checkbox" name="public_account_association" value="1"{% if prefs.public_account_association %} checked{% endif %}>{{ association }}</label><label class="op-row"><input type="checkbox" name="public_status_visible" value="1"{% if prefs.public_status_visible %} checked{% endif %}>{{ status }}</label><label for="locale">{{ language }}</label><select class="op-input" id="locale" name="locale"><option value="pl"{% if prefs.locale == "pl" %} selected{% endif %}>Polski</option><option value="en"{% if prefs.locale == "en" %} selected{% endif %}>English</option></select><button class="op-button" type="submit">{{ save }}</button></form></main></body></html>"#,
    ext = "html"
)]
struct Page {
    lang: String,
    title: String,
    association: String,
    status: String,
    language: String,
    save: String,
    error: String,
    invalid: bool,
    prefs: Preferences,
    csrf: String,
}
fn text(lang: &str, key: &str) -> Result<String, StatusCode> {
    let source = match lang {
        "pl" => include_str!("privacy_pl.ftl"),
        "en" => include_str!("privacy_en.ftl"),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let resource = FluentResource::try_new(source.to_owned())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let locale = lang.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut errors = Vec::new();
    let value = bundle
        .format_pattern(pattern, None, &mut errors)
        .to_string();
    if !errors.is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(value)
}
async fn page(
    state: &PrivacyState,
    session: &BrowserSession,
    lang: &str,
    invalid: bool,
) -> Result<Response, StatusCode> {
    let title = text(lang, "title")?;
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let prefs = state
        .policy
        .current(&state.pool, id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::FORBIDDEN)?;
    let page = Page {
        lang: lang.to_owned(),
        title,
        association: text(lang, "association")?,
        status: text(lang, "status")?,
        language: text(lang, "language")?,
        save: text(lang, "save")?,
        error: text(lang, "invalid")?,
        invalid,
        prefs,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let mut response = oteryn_web::render(&page);
    if invalid {
        *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
    }
    response.headers_mut().insert(
        header::CONTENT_LANGUAGE,
        lang.parse().map_err(|_| StatusCode::NOT_FOUND)?,
    );
    Ok(response)
}
async fn show(
    State(state): State<PrivacyState>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(&state, &session, &lang, false).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
    public_account_association: Option<String>,
    public_status_visible: Option<String>,
    locale: String,
}
fn boolean(value: Option<&str>) -> Result<bool, ()> {
    match value {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        _ => Err(()),
    }
}
async fn update(
    State(state): State<PrivacyState>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    request: Request,
) -> Result<Response, StatusCode> {
    text(&lang, "title")?;
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .0;
    let source = state
        .limiter
        .trusted_proxies
        .client_ip(peer, request.headers())
        .map_err(|error| error.status())?;
    let mut keys = oteryn_web::ratelimit::Keys::source(source);
    keys.account = Some(id.to_string());
    if let Err(error) = state
        .limiter
        .check(
            oteryn_web::ratelimit::NamedPolicy::IdentitySecurityMutation,
            &[keys],
        )
        .await
    {
        let language = if lang == "en" {
            oteryn_web::Language::En
        } else {
            oteryn_web::Language::Pl
        };
        return Ok(error.response(language, false));
    }
    let bytes = axum::body::to_bytes(request.into_body(), 8192)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let parsed = serde_urlencoded::from_bytes::<Input>(&bytes)
        .ok()
        .and_then(|input| {
            let association = boolean(input.public_account_association.as_deref()).ok()?;
            let status = boolean(input.public_status_visible.as_deref()).ok()?;
            matches!(input.locale.as_str(), "pl" | "en").then_some((input, association, status))
        });
    let Some((input, association, status)) = parsed else {
        return page(&state, &session, &lang, true).await;
    };
    state
        .policy
        .update(&state.pool, id, association, status, &input.locale)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::FORBIDDEN)?;
    Ok(Redirect::to(&format!("/{}/account/privacy", input.locale)).into_response())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_and_translations() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(boolean(None), Ok(false));
        assert_eq!(boolean(Some("1")), Ok(true));
        assert_eq!(boolean(Some("yes")), Err(()));
        assert_eq!(boolean(Some("true")), Err(()));
        assert_eq!(boolean(Some("false")), Err(()));
        assert!(serde_urlencoded::from_str::<Input>("locale=en&locale=pl").is_err());
        assert!(serde_urlencoded::from_str::<Input>("locale=en&account_id=1").is_err());
        for lang in ["pl", "en"] {
            for key in [
                "title",
                "association",
                "status",
                "language",
                "save",
                "invalid",
            ] {
                assert!(
                    !text(lang, key)
                        .map_err(|_| "missing translation")?
                        .is_empty()
                );
            }
        }
        assert_eq!(text("xx", "title"), Err(StatusCode::NOT_FOUND));
        Ok(())
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request as HttpRequest,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn privacy_http_postgresql() -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping privacy HTTP PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let (pool, admin, schema) = test_support::database(&url).await?;
        let id: String = sqlx::query_scalar("SELECT uuidv7()::text")
            .fetch_one(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id,email,password_hash,locale,status,email_verified_at) VALUES ($1::text::uuid,$2,'test','en','active',now())").bind(&id).bind(format!("{id}@example.test")).execute(&pool).await?;
        let token: String =
            sqlx::query_scalar("SELECT encode(sha256(uuidv7()::text::bytea),'hex')")
                .fetch_one(&pool)
                .await?;
        let csrf = "a".repeat(64);
        sqlx::query("INSERT INTO web_sessions (id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES (sha256($1::text::bytea),$2::text::uuid,$3,'test',now()+interval '1 hour',now()+interval '1 hour')").bind(&token).bind(&id).bind(&csrf).execute(&pool).await?;
        let cookie = format!("__Host-oteryn_session={token}");
        let app = oteryn_web::router(oteryn_web::AppState { db: pool.clone() }, "static")
            .merge(router(pool.clone(), true));
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get("/en/account/privacy")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let html = String::from_utf8(to_bytes(response.into_body(), 16384).await?.to_vec())?;
        assert!(html.contains("Privacy and language"));
        for (body, status) in [
            ("locale=pl".to_string(), StatusCode::FORBIDDEN),
            (
                format!("_csrf={csrf}&locale=xx"),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                format!("_csrf={csrf}&locale=pl&public_status_visible=1"),
                StatusCode::SEE_OTHER,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    HttpRequest::post("/en/account/privacy")
                        .extension(axum::extract::ConnectInfo(
                            "127.0.0.1:12345".parse::<std::net::SocketAddr>()?,
                        ))
                        .header(header::COOKIE, &cookie)
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from(body))?,
                )
                .await?;
            assert_eq!(response.status(), status);
        }
        for attempt in 0..9 {
            let response = app
                .clone()
                .oneshot(
                    HttpRequest::post("/en/account/privacy")
                        .extension(axum::extract::ConnectInfo(
                            "127.0.0.1:12345".parse::<std::net::SocketAddr>()?,
                        ))
                        .header(header::COOKIE, &cookie)
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from(format!("_csrf={csrf}&locale=xx")))?,
                )
                .await?;
            if attempt == 8 {
                assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
                assert!(response.headers().contains_key(header::RETRY_AFTER));
            }
        }
        let locale: String =
            sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1::text::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(locale, "pl");
        let response = app
            .clone()
            .oneshot(HttpRequest::get("/en/account/privacy").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = router(pool.clone(), false)
            .oneshot(HttpRequest::get("/en/account/privacy").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1::text::uuid")
            .bind(&id)
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM account_privacy_events WHERE account_id=$1::text::uuid")
            .bind(&id)
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM accounts WHERE id=$1::text::uuid")
            .bind(&id)
            .execute(&pool)
            .await?;
        test_support::close(pool, admin, schema).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "privacy_test_support.rs"]
mod test_support;
