//! Owner-only Platform profile.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;

const DISPLAY_NAME_LIMIT: usize = 64;

pub fn configured_router(pool: PgPool) -> Router {
    if !oteryn_identity::navigation::profile_enabled() {
        return Router::new();
    }
    router(pool)
}

fn router(pool: PgPool) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    if let Ok(origin) = std::env::var("OTERYN_PUBLIC_ORIGIN") {
        sessions.csrf.trusted_origins.push(origin);
    }
    Router::new()
        .route("/{lang}/account/profile", get(show).post(save))
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
        .layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(request).await;
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
        .with_state(pool)
}

#[derive(sqlx::FromRow)]
struct Profile {
    display_name: String,
    created_at: String,
    verified_at: String,
    last_browser_login: Option<String>,
}

#[derive(Template)]
#[template(
    path = "account/profile.html",
    config = "../../templates/account/mfa/askama.toml"
)]
struct Page {
    lang: &'static str,
    language: Language,
    profile: Profile,
    csrf: String,
    invalid: bool,
    display_name_limit: usize,
}

impl Page {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }
}

fn language(lang: &str) -> Result<Language, StatusCode> {
    match lang {
        "pl" => Ok(Language::Pl),
        "en" => Ok(Language::En),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

fn unavailable(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

async fn page(
    pool: &PgPool,
    session: &BrowserSession,
    language: Language,
    invalid: bool,
) -> Result<Response, StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let profile = sqlx::query_as(
        "SELECT a.profile_display_name AS display_name,
                to_char(a.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS created_at,
                to_char(a.email_verified_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS verified_at,
                (SELECT to_char(e.created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS')
                 FROM security_events e WHERE e.account_id=a.id AND e.kind='login_succeeded'
                 ORDER BY e.created_at DESC LIMIT 1) AS last_browser_login
         FROM accounts a WHERE a.id=$1 AND a.status='active' AND a.email_verified_at IS NOT NULL",
    )
        .bind(id).fetch_optional(pool).await.map_err(unavailable)?.ok_or(StatusCode::FORBIDDEN)?;
    let csrf = session.csrf_token().await.map_err(unavailable)?;
    let page = Page {
        lang: language.code(),
        language,
        profile,
        csrf,
        invalid,
        display_name_limit: DISPLAY_NAME_LIMIT,
    };
    Ok(if invalid {
        oteryn_web::render_form_refusal(StatusCode::BAD_REQUEST, &page)
    } else {
        oteryn_web::render(&page)
    })
}

async fn show(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(&pool, &session, language(&lang)?, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileInput {
    display_name: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

fn display_name(value: &str) -> Option<&str> {
    let name = value.trim();
    (name.chars().count() <= DISPLAY_NAME_LIMIT && !value.chars().any(char::is_control))
        .then_some(name)
}

async fn save(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    ClientSource(source): ClientSource,
    limiter: Option<Extension<RateLimiter>>,
    Form(input): Form<ProfileInput>,
) -> Result<Response, StatusCode> {
    let language = language(&lang)?;
    let owner = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let mut keys = Keys::source(source);
    keys.account = Some(owner.to_string());
    let limiter = limiter
        .map(|limiter| limiter.0)
        .unwrap_or_else(|| RateLimiter::new(pool.clone()));
    if let Err(denial) = limiter
        .check(NamedPolicy::IdentitySecurityMutation, &[keys])
        .await
    {
        return Ok(denial.response(language, false));
    }
    let Some(name) = display_name(&input.display_name) else {
        return page(&pool, &session, language, true).await;
    };
    let session_hash = session.identity_hash().await.ok_or(StatusCode::FORBIDDEN)?;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let old: String = sqlx::query_scalar("SELECT profile_display_name FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE")
        .bind(owner).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(StatusCode::FORBIDDEN)?;
    // Keep the initiating browser session locked until the profile and audit commit.
    sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT id_hash FROM web_sessions WHERE id_hash=$1 AND account_id=$2
         AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()
         FOR UPDATE",
    )
    .bind(session_hash)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::FORBIDDEN)?;
    if old != name {
        sqlx::query(
            "UPDATE accounts SET profile_display_name=$2,updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(owner)
        .bind(name)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'account.profile.updated',$2,jsonb_build_object('has_display_name',$3::boolean),jsonb_build_object('has_display_name',$4::boolean),$5)")
            .bind(owner).bind(owner.to_string()).bind(!old.is_empty()).bind(!name.is_empty()).bind(uuid::Uuid::now_v7())
            .execute(&mut *tx).await.map_err(unavailable)?;
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(Redirect::to(&format!("/{lang}/account/profile")).into_response())
}

#[cfg(test)]
#[path = "mfa_test_db.rs"]
mod database;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_unicode_and_clearing_but_refuses_oversized_or_control_input() {
        assert_eq!(display_name(" Żółć "), Some("Żółć"));
        assert_eq!(display_name(""), Some(""));
        let emoji = "😀".repeat(DISPLAY_NAME_LIMIT);
        assert_eq!(display_name(&format!(" {emoji} ")), Some(emoji.as_str()));
        assert_eq!(display_name(&format!("{emoji}😀")), None);
        assert_eq!(display_name(&"ą".repeat(65)), None);
        assert_eq!(display_name("test\n"), None);
        assert!(
            serde_urlencoded::from_str::<ProfileInput>("display_name=abc&_csrf=x&account_id=other")
                .is_err()
        );
    }

    #[tokio::test]
    async fn panel_profile_updates_only_live_owner() -> Result<(), Box<dyn std::error::Error>> {
        use axum::{body::Body, extract::ConnectInfo, http::Request};
        use std::net::SocketAddr;
        use tower::ServiceExt;
        let db = database::Database::open()
            .await?
            .ok_or("PostgreSQL required")?;
        let owner: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('profile-owner@example.test','unused','active','en',now()) RETURNING id").fetch_one(&db.pool).await?;
        let cookie = "a".repeat(64);
        let csrf = "b".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
            .bind(cookie.as_bytes()).bind(owner).bind(&csrf).execute(&db.pool).await?;
        let app = router(db.pool.clone());
        let path = "/en/account/profile";
        let guest = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(guest.status(), StatusCode::FORBIDDEN);
        let cookie = format!("{}={cookie}", session::COOKIE_NAME);
        let read = app
            .clone()
            .oneshot(
                Request::get(path)
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(read.status(), StatusCode::OK);
        let send = |body: String| {
            let app = app.clone();
            let cookie = cookie.clone();
            async move {
                let request = Request::post(path)
                    .header(header::COOKIE, cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .extension(ConnectInfo("127.0.0.1:4324".parse::<SocketAddr>()?))
                    .body(Body::from(body))?;
                Ok::<_, Box<dyn std::error::Error>>(app.oneshot(request).await?)
            }
        };
        assert_eq!(
            send("display_name=missing-token".into()).await?.status(),
            StatusCode::FORBIDDEN
        );
        let invalid = send(format!("display_name={}&_csrf={csrf}", "x".repeat(65))).await?;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(invalid.into_body(), 32768).await?;
        let html = std::str::from_utf8(&body)?;
        assert!(html.contains("Use up to 64 characters without control characters."));
        assert!(html.contains("action=\"/en/account/profile\""));
        assert_eq!(
            send(format!(
                "display_name=foreign&_csrf={csrf}&account_id={}",
                uuid::Uuid::now_v7()
            ))
            .await?
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            send(format!("display_name=Saved&_csrf={csrf}"))
                .await?
                .status(),
            StatusCode::SEE_OTHER
        );
        let name: String =
            sqlx::query_scalar("SELECT profile_display_name FROM accounts WHERE id=$1")
                .bind(owner)
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(name, "Saved");
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn profile_save_cannot_overtake_logout() -> Result<(), Box<dyn std::error::Error>> {
        use axum::{body::Body, extract::ConnectInfo, http::Request};
        use sqlx::Connection;
        use std::{net::SocketAddr, time::Duration};
        use tower::ServiceExt;

        let db = database::Database::open()
            .await?
            .ok_or("PostgreSQL required")?;
        let owner: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('profile-logout@example.test','unused','active','en',now()) RETURNING id")
            .fetch_one(&db.pool).await?;
        let cookie = "a".repeat(64);
        let csrf = "b".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
            .bind(cookie.as_bytes()).bind(owner).bind(&csrf).execute(&db.pool).await?;

        let schema: String = sqlx::query_scalar("SELECT current_schema()")
            .fetch_one(&db.pool)
            .await?;
        let options = db
            .pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", schema)]);
        let mut account_connection = sqlx::PgConnection::connect_with(&options).await?;
        let mut logout_connection = sqlx::PgConnection::connect_with(&options).await?;
        let mut account_lock = account_connection.begin().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(owner)
            .fetch_one(&mut *account_lock)
            .await?;
        let request = Request::post("/en/account/profile")
            .header(header::COOKIE, format!("{}={cookie}", session::COOKIE_NAME))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .extension(ConnectInfo("127.0.0.1:4324".parse::<SocketAddr>()?))
            .body(Body::from(format!("display_name=Stale&_csrf={csrf}")))?;
        let save = tokio::spawn(router(db.pool.clone()).oneshot(request));

        let waiting_on_this_connection = "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
            WHERE pg_backend_pid()=ANY(pg_blocking_pids(pid)))";
        tokio::time::timeout(Duration::from_secs(10), async {
            while !sqlx::query_scalar::<_, bool>(waiting_on_this_connection)
                .fetch_one(&mut *account_lock)
                .await?
            {}
            Ok::<_, sqlx::Error>(())
        })
        .await??;
        // The browser middleware finished before save waited on the account lock.
        let mut logout = logout_connection.begin().await?;
        sqlx::query("DELETE FROM web_sessions WHERE id_hash=sha256($1::bytea)")
            .bind(cookie.as_bytes())
            .execute(&mut *logout)
            .await?;
        account_lock.commit().await?;
        tokio::time::timeout(Duration::from_secs(10), async {
            while !save.is_finished()
                && !sqlx::query_scalar::<_, bool>(waiting_on_this_connection)
                    .fetch_one(&mut *logout)
                    .await?
            {}
            Ok::<_, sqlx::Error>(())
        })
        .await??;
        logout.commit().await?;

        assert_eq!(save.await??.status(), StatusCode::FORBIDDEN);
        let state: (String, bool) = sqlx::query_as(
            "SELECT profile_display_name,
            EXISTS(SELECT 1 FROM audit_log WHERE actor=$1 AND action='account.profile.updated')
            FROM accounts WHERE id=$1",
        )
        .bind(owner)
        .fetch_one(&db.pool)
        .await?;
        assert_eq!(state, (String::new(), false));
        db.close().await?;
        Ok(())
    }
}
