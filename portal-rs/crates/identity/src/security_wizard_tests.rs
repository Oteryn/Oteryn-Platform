use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;

#[test]
fn positive_missing_unknown_and_bad_dates() -> Result<(), StatusCode> {
    let now = OffsetDateTime::now_utc();
    let facts = Facts {
        mfa: Evidence::Complete,
        recovery_key: Evidence::Complete,
        backup_email: Evidence::Complete,
        password_changed_at: Some(now),
    };
    assert_eq!(assess(Some(now), facts.clone(), now)?.completed, 4);
    assert_eq!(assess(None, Facts::default(), now)?.completed, 0);
    assert!(assess(Some(now + std::time::Duration::from_secs(1)), facts, now).is_err());
    assert!(
        assess(
            None,
            Facts {
                password_changed_at: Some(now + std::time::Duration::from_secs(1)),
                ..Facts::default()
            },
            now
        )
        .is_err()
    );
    let mixed = assess(
        None,
        Facts {
            mfa: Evidence::Missing,
            recovery_key: Evidence::Complete,
            ..Facts::default()
        },
        now,
    )?;
    assert_eq!(mixed.completed, 1);
    assert_eq!(
        mixed.steps,
        [
            Evidence::Missing,
            Evidence::Missing,
            Evidence::Complete,
            Evidence::Unknown
        ]
    );
    Ok(())
}

#[test]
fn localized_accessible_document_and_invalid_language() -> Result<(), StatusCode> {
    let assessment = assess(None, Facts::default(), OffsetDateTime::now_utc())?;
    for (lang, title, unknown) in [
        ("pl", "Bezpieczeństwo konta", "Niedostępne"),
        ("en", "Account security", "Unavailable"),
    ] {
        let html = render(lang, &assessment)?;
        assert!(html.contains(title));
        assert!(html.contains(unknown));
        assert!(html.contains("0/4"));
        assert!(html.contains("<main><h1>"));
        assert!(html.contains("width=device-width"));
        assert!(html.contains(&format!("href=\"/{lang}/account/security\"")));
    }
    assert_eq!(render("<script>", &assessment), Err(StatusCode::NOT_FOUND));
    assert_eq!(escape("<>&\"'"), "&lt;&gt;&amp;&quot;&#39;");
    Ok(())
}

fn app(pool: PgPool, enabled: bool) -> Router {
    router(Wizard {
        pool,
        enabled,
        facts: Arc::new(UnavailableFacts),
    })
}

#[tokio::test]
async fn route_refusal_and_bad_locale() -> Result<(), Box<dyn std::error::Error>> {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    for (enabled, path, status) in [
        (false, "/en/account/security/wizard", StatusCode::NOT_FOUND),
        (
            true,
            "/en/account/security/wizard",
            StatusCode::UNAUTHORIZED,
        ),
        (
            true,
            "/invalid/account/security/wizard",
            StatusCode::NOT_FOUND,
        ),
    ] {
        let response = app(pool.clone(), enabled)
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    Ok(())
}

#[tokio::test]
async fn postgres_owner_assessment_and_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL security wizard test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("security_wizard_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let id = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,locale,status,email_verified_at) VALUES ($1,$2,'unused','en','active',now())")
        .bind(id).bind(format!("wizard-{id}@example.test")).execute(&pool).await?;
    let request = || Request::get("/en/account/security/wizard").body(Body::empty());
    let response = app(pool.clone(), true)
        .layer(Extension(Viewer(id)))
        .oneshot(request()?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(body.contains("1/4"));
    assert!(body.contains("Unavailable"));
    assert!(!body.contains(&id.to_string()));
    assert!(!body.contains("@example.test"));
    #[derive(Debug)]
    struct VerifiedFacts;
    impl FactsSource for VerifiedFacts {
        fn load(&self, _: AccountId) -> FactsFuture<'_> {
            Box::pin(async {
                Ok(Facts {
                    mfa: Evidence::Complete,
                    recovery_key: Evidence::Complete,
                    backup_email: Evidence::Complete,
                    password_changed_at: Some(OffsetDateTime::now_utc()),
                })
            })
        }
    }
    let complete = router(Wizard {
        pool: pool.clone(),
        enabled: true,
        facts: Arc::new(VerifiedFacts),
    })
    .layer(Extension(Viewer(id)))
    .oneshot(request()?)
    .await?;
    assert_eq!(complete.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(complete.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("4/4"));
    #[derive(Debug)]
    struct FailedFacts;
    impl FactsSource for FailedFacts {
        fn load(&self, _: AccountId) -> FactsFuture<'_> {
            Box::pin(async { Err(sqlx::Error::RowNotFound) })
        }
    }
    let failure = router(Wizard {
        pool: pool.clone(),
        enabled: true,
        facts: Arc::new(FailedFacts),
    })
    .layer(Extension(Viewer(id)))
    .oneshot(request()?)
    .await?;
    assert_eq!(failure.status(), StatusCode::SERVICE_UNAVAILABLE);
    // Exercise a real browser cookie through the public session API. The
    // test-only login endpoint is never part of the production router.
    let browser = app(pool.clone(), true)
        .route("/test-login", get(move |Extension(session): Extension<oteryn_web::session::BrowserSession>| async move {
            oteryn_web::session::rotate(&session, oteryn_web::session::PrivilegeChange::Login(id)).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }))
        .layer(axum::middleware::from_fn(|mut request: axum::extract::Request, next: axum::middleware::Next| async move {
            if let Some(session) = request.extensions().get::<oteryn_web::session::BrowserSession>()
                && let Some(id) = session.account_id().await
            {
                request.extensions_mut().insert(Viewer(id));
            }
            next.run(request).await
        }))
        .layer(axum::middleware::from_fn_with_state(oteryn_web::session::SessionLayer::new(pool.clone()), oteryn_web::session::middleware));
    let login = browser
        .clone()
        .oneshot(Request::get("/test-login").body(Body::empty())?)
        .await?;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing session cookie")?
        .to_owned();
    let response = browser
        .clone()
        .oneshot(
            Request::get("/pl/account/security/wizard?account_id=ignored")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Bezpieczeństwo konta"));
    let response = browser
        .oneshot(
            Request::get("/en/account/security/wizard")
                .header(header::COOKIE, "__Host-oteryn_session=invalid")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        app(pool.clone(), true)
            .layer(Extension(Viewer(id)))
            .oneshot(request()?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app(pool.clone(), true)
            .layer(Extension(Viewer(uuid::Uuid::now_v7())))
            .oneshot(request()?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
