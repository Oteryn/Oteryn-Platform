use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{HeaderMap, HeaderValue, Request, StatusCode, header},
    routing::get,
};
use oteryn_web::ratelimit::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{net::SocketAddr, time::Duration};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED rate limiter database test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes)?;
        // Identifiers contain only a fixed prefix and decimal digits, never input.
        let schema = format!("ratelimit_{}", u64::from_be_bytes(bytes));
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        sqlx::QueryBuilder::new("CREATE SCHEMA ")
            .push(&schema)
            .build()
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(16)
            .after_connect(move |conn, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(sql)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::QueryBuilder::new("DROP SCHEMA ")
            .push(&self.schema)
            .push(" CASCADE")
            .build()
            .execute(&self.admin)
            .await?;
        self.admin.close().await;
        Ok(())
    }
}
fn keys(ip: &str) -> Result<Keys, Box<dyn std::error::Error>> {
    Ok(Keys::source(ip.parse()?))
}
fn app(limiter: RateLimiter) -> Router {
    let routes = Router::new()
        .route(
            "/en/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .route(
            "/pl/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .route(
            "/api/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .layer(Extension(limiter.clone()));
    finish_routes(routes)
}
async fn response(
    app: Router,
    path: &str,
    peer: &str,
    forwarded: Option<&str>,
) -> Result<axum::response::Response, Box<dyn std::error::Error>> {
    let mut request = Request::get(path).body(Body::empty())?;
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>()?));
    if let Some(ip) = forwarded {
        request
            .headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_str(ip)?);
    }
    Ok(app.oneshot(request).await?)
}

#[tokio::test]
async fn limit_returns_429_retry_after_and_localized_html_or_api_json() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    // Mount inside the shared policy, as production module routes are mounted.
    let limiter = RateLimiter::new(db.pool.clone());
    let routes = Router::new()
        .route(
            "/en/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .route(
            "/pl/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .route(
            "/api/limited",
            get(|_: RateLimit<IdentityRegistration>| async { "ok" }),
        )
        .layer(Extension(limiter.clone()));
    // A wrapper routes these requests through the production response policy.
    let app = finish_routes(routes);
    for _ in 0..5 {
        assert_eq!(
            response(app.clone(), "/en/limited", "192.0.2.1:80", None)
                .await?
                .status(),
            StatusCode::OK
        );
    }
    for (path, expected, json) in [
        ("/en/limited", "Too many requests", false),
        ("/pl/limited", "Zbyt wiele", false),
        ("/api/limited", "Too many requests", true),
    ] {
        let mut request = Request::get(path)
            .header(header::ACCEPT_LANGUAGE, "en")
            .body(Body::empty())?;
        request
            .extensions_mut()
            .insert(ConnectInfo("192.0.2.1:80".parse::<SocketAddr>()?));
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry: u64 = response.headers()[header::RETRY_AFTER].to_str()?.parse()?;
        assert!((1..=60).contains(&retry));
        let content = response.headers()[header::CONTENT_TYPE]
            .to_str()?
            .to_owned();
        let body = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(body.contains(expected), "{body}");
        if json {
            assert!(content.starts_with("application/json"));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&body)?["status"],
                429
            );
        } else {
            assert!(body.contains("<main"));
            assert!(body.contains("surface.css"));
        }
    }
    db.close().await
}
#[tokio::test]
async fn expired_window_rolls_over_and_keys_are_independent() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let limiter = RateLimiter::new(db.pool.clone());
    let a = keys("192.0.2.2")?;
    let b = keys("192.0.2.3")?;
    for _ in 0..5 {
        check(
            &limiter,
            NamedPolicy::IdentityRegistration,
            std::slice::from_ref(&a),
        )
        .await?;
    }
    assert!(matches!(
        limiter
            .check(NamedPolicy::IdentityRegistration, std::slice::from_ref(&a))
            .await,
        Err(Denial::Limited { .. })
    ));
    limiter
        .check(NamedPolicy::IdentityRegistration, &[b])
        .await?;
    // Cross a real PostgreSQL clock boundary, preserving the expired counters.
    let window: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()) / 60)::bigint")
            .fetch_one(&db.pool)
            .await?;
    // A host timer does not prove that the database clock crossed its boundary
    // (notably with PostgreSQL in another VM). Observe the actual bucket clock.
    tokio::time::timeout(Duration::from_secs(65), async {
        loop {
            let rolled: bool = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM statement_timestamp()) / 60)::bigint > $1",
            )
            .bind(window)
            .fetch_one(&db.pool)
            .await?;
            if rolled {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    limiter
        .check(NamedPolicy::IdentityRegistration, &[a])
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_rate_limits")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 3);
    db.close().await
}

#[tokio::test]
async fn fifty_concurrent_increments_across_instances_never_exceed_budget() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..50 {
        let limiter = RateLimiter::new(db.pool.clone());
        let mut k = keys("192.0.2.4")?;
        k.account = Some("account-a".into());
        tasks.spawn(async move { limiter.check(NamedPolicy::IdentityEmailChange, &[k]).await });
    }
    let mut allowed = 0;
    while let Some(task) = tasks.join_next().await {
        match task? {
            Ok(()) => allowed += 1,
            Err(Denial::Limited { .. }) => {}
            Err(e) => return Err(e.into()),
        }
    }
    assert_eq!(allowed, 3);
    let count: i64 = sqlx::query_scalar("SELECT count FROM web_rate_limits")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 4);
    db.close().await
}

#[tokio::test]
async fn proxy_trust_controls_the_actual_database_key() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut limiter = RateLimiter::new(db.pool.clone());
    let untrusted = app(limiter.clone());
    for _ in 0..5 {
        assert_eq!(
            response(
                untrusted.clone(),
                "/en/limited",
                "192.0.2.5:80",
                Some("203.0.113.1")
            )
            .await?
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        response(
            untrusted,
            "/en/limited",
            "192.0.2.5:80",
            Some("203.0.113.2")
        )
        .await?
        .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    limiter.trusted_proxies = TrustedProxies(vec!["192.0.2.5".parse()?]);
    let trusted = app(limiter);
    assert_eq!(
        response(
            trusted.clone(),
            "/en/limited",
            "192.0.2.5:80",
            Some("203.0.113.1")
        )
        .await?
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        response(
            trusted.clone(),
            "/en/limited",
            "192.0.2.5:80",
            Some("203.0.113.2")
        )
        .await?
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        response(trusted, "/en/limited", "192.0.2.5:80", Some("bad-ip"))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    db.close().await
}

#[tokio::test]
async fn unavailable_database_fails_closed_at_http_boundary() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let dead = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("postgres://test@127.0.0.1:1/unreachable")?;
    let limiter = RateLimiter::new(dead.clone());
    assert_eq!(
        limiter
            .check(NamedPolicy::IdentityRegistration, &[keys("192.0.2.6")?])
            .await,
        Err(Denial::Unavailable)
    );
    let response = response(app(limiter), "/api/limited", "192.0.2.6:80", None).await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    dead.close().await;
    db.close().await
}

#[tokio::test]
async fn cleanup_removes_old_windows_and_preserves_active_limits() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let limiter = RateLimiter::new(db.pool.clone());
    limiter
        .check(NamedPolicy::IdentityRegistration, &[keys("192.0.2.7")?])
        .await?;
    sqlx::query("INSERT INTO web_rate_limits SELECT limiter_name, key_hash, window_start - interval '2 hours', count FROM web_rate_limits").execute(&db.pool).await?;
    let cleanup = RateLimitCleanup {
        pool: db.pool.clone(),
    };
    oteryn_db::outbox::Cleanup::clean(&cleanup).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_rate_limits")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    assert_eq!(cleanup.clean().await?, 0);
    db.close().await
}

#[tokio::test]
async fn normalized_emails_share_budget_and_raw_keys_never_enter_table() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let limiter = RateLimiter::new(db.pool.clone());
    let mut a = keys("192.0.2.8")?;
    a.email = Some("  Person@Example.invalid ".into());
    for _ in 0..5 {
        limiter
            .check(NamedPolicy::IdentityLogin, std::slice::from_ref(&a))
            .await?;
    }
    a.email = Some("person@example.invalid".into());
    assert!(matches!(
        limiter.check(NamedPolicy::IdentityLogin, &[a]).await,
        Err(Denial::Limited { .. })
    ));
    let rows: Vec<(String, Vec<u8>)> =
        sqlx::query_as("SELECT limiter_name, key_hash FROM web_rate_limits")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "identity-login");
    assert_eq!(rows[0].1.len(), 32);
    let dump: String =
        sqlx::query_scalar("SELECT row_to_json(web_rate_limits)::text FROM web_rate_limits")
            .fetch_one(&db.pool)
            .await?;
    assert!(!dump.contains("192.0.2.8"));
    assert!(!dump.contains("example.invalid"));
    db.close().await
}

#[tokio::test]
async fn bad_key_sets_are_refused_without_charging() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let limiter = RateLimiter::new(db.pool.clone());
    assert_eq!(
        limiter.check(NamedPolicy::IdentityLogin, &[]).await,
        Err(Denial::BadInput)
    );
    assert_eq!(
        limiter
            .check(NamedPolicy::IdentityLogin, &[keys("192.0.2.9")?])
            .await,
        Err(Denial::BadInput)
    );
    let mut good = keys("192.0.2.9")?;
    good.email = Some("a@example.invalid".into());
    assert_eq!(
        limiter
            .check(
                NamedPolicy::IdentityLogin,
                &[good.clone(), keys("192.0.2.10")?]
            )
            .await,
        Err(Denial::BadInput)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_rate_limits")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    good.email = Some(" ".into());
    assert_eq!(
        limiter.check(NamedPolicy::IdentityLogin, &[good]).await,
        Err(Denial::BadInput)
    );
    db.close().await
}

#[test]
fn proxy_chain_stops_at_first_untrusted_hop_and_rejects_invalid_input() -> TestResult {
    let trust = TrustedProxies(vec!["192.0.2.1".parse()?, "192.0.2.2".parse()?]);
    let peer = "192.0.2.1:80".parse()?;
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("203.0.113.99, 203.0.113.1, 192.0.2.2"),
    );
    assert_eq!(
        trust.client_ip(peer, &headers)?,
        "203.0.113.1".parse::<std::net::IpAddr>()?
    );
    headers.insert("x-forwarded-for", HeaderValue::from_static("invalid"));
    assert_eq!(trust.client_ip(peer, &headers), Err(Denial::BadInput));
    assert_eq!(
        TrustedProxies::default().client_ip(peer, &headers)?,
        peer.ip()
    );
    Ok(())
}

#[tokio::test]
async fn multiple_keys_charge_independently_and_refusal_is_retained() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let limiter = RateLimiter::new(db.pool.clone());
    let a = keys("192.0.2.20")?;
    let b = keys("192.0.2.21")?;
    for _ in 0..5 {
        limiter
            .check(NamedPolicy::IdentityRegistration, std::slice::from_ref(&a))
            .await?;
    }
    assert!(matches!(
        check(&limiter, NamedPolicy::IdentityRegistration, &[a, b.clone()]).await,
        Err(Denial::Limited { .. })
    ));
    for _ in 0..4 {
        limiter
            .check(NamedPolicy::IdentityRegistration, std::slice::from_ref(&b))
            .await?;
    }
    assert!(matches!(
        limiter.check(NamedPolicy::IdentityRegistration, &[b]).await,
        Err(Denial::Limited { .. })
    ));
    db.close().await
}

#[test]
fn named_policies_match_the_existing_portal_limits() -> TestResult {
    let source = include_str!("../../../../app/Providers/AppServiceProvider.php");
    let mut found = 0;
    for block in source.split("RateLimiter::for('").skip(1) {
        let (name, rest) = block.split_once('\'').ok_or("missing limiter name")?;
        let policy = NamedPolicy::ALL
            .iter()
            .find(|p| p.name() == name)
            .ok_or("missing policy")?;
        let minute = rest.find("Limit::perMinute(");
        let hour = rest.find("Limit::perHour(");
        let (offset, period, prefix) = match (minute, hour) {
            (Some(a), Some(b)) if a < b => (a, 60, "Limit::perMinute("),
            (Some(a), None) => (a, 60, "Limit::perMinute("),
            (_, Some(b)) => (b, 3600, "Limit::perHour("),
            _ => return Err("missing limit".into()),
        };
        let limit = rest[offset + prefix.len()..]
            .split(')')
            .next()
            .ok_or("missing budget")?
            .parse::<i64>()?;
        assert_eq!(policy.limit(), limit, "{name}");
        assert_eq!(policy.window_seconds(), period, "{name}");
        found += 1;
    }
    assert_eq!(found, NamedPolicy::ALL.len());
    Ok(())
}
