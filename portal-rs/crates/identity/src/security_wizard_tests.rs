#[path = "security_wizard_test_db.rs"]
mod database;
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
        let html = render(lang, &assessment, None)?;
        assert!(html.contains(title));
        assert!(html.contains(unknown));
        assert!(html.contains("0/4"));
        assert!(html.contains("op-shell"));
        assert!(html.contains("data-theme=\"dark\""));
        assert!(html.contains(&format!("href=\"/{lang}/account/recovery-key\"")));
        assert!(html.contains("width=device-width"));
        assert!(html.contains(&format!("href=\"/{lang}/account/security\"")));
    }
    assert_eq!(
        render("<script>", &assessment, None),
        Err(StatusCode::NOT_FOUND)
    );
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

#[tokio::test]
async fn audit02_production_facts_report_four_and_revocations()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('wizard-audit@example.test','unused','active','en',now()) RETURNING id").fetch_one(&db.pool).await?;
    sqlx::query(
        "INSERT INTO account_recovery_keys(account_id,key_hash,confirmed_at) VALUES($1,$2,now())",
    )
    .bind(id)
    .bind(crate::password::hash("wizard recovery key")?)
    .execute(&db.pool)
    .await?;
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('ab','hex'),true)").bind(id).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,'backup-wizard@example.test',now())").bind(id).execute(&db.pool).await?;
    let app = production(db.pool.clone(), true).layer(Extension(Viewer(id)));
    let response = app
        .clone()
        .oneshot(Request::get("/en/account/security/wizard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?.contains("4/4")
    );
    let facts = RecoveryFacts(db.pool.clone());
    assert_eq!(
        assess(
            Some(OffsetDateTime::now_utc()),
            facts.load(id).await?,
            OffsetDateTime::now_utc()
        )
        .map_err(|s| s.to_string())?
        .completed,
        4
    );
    for (index, statement) in [
        "UPDATE account_mfa_totp SET confirmed=false WHERE account_id=$1",
        "UPDATE account_recovery_keys SET revoked_at=now() WHERE account_id=$1",
        "DELETE FROM account_backup_emails WHERE account_id=$1",
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(id)
            .execute(&db.pool)
            .await?;
        let response = app
            .clone()
            .oneshot(Request::get("/en/account/security/wizard").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?
                .contains(&format!("{}/4", 3 - index))
        );
    }
    let loaded = facts.load(id).await?;
    assert_eq!(loaded.mfa, Evidence::Missing);
    assert_eq!(loaded.recovery_key, Evidence::Missing);
    assert_eq!(loaded.backup_email, Evidence::Missing);
    db.close().await
}

#[tokio::test]
async fn audit02_password_history_includes_reset_and_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('history@example.test',$1,'active','en',now()) RETURNING id")
        .bind(crate::password::hash("original test password")?).fetch_one(&db.pool).await?;
    let session_hash = vec![42u8; 32];
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,repeat('c',64),'test',now()+interval '1 hour',now()+interval '1 day')").bind(&session_hash).bind(id).execute(&db.pool).await?;
    let credentials = crate::credentials::Credentials {
        pool: db.pool.clone(),
        enabled: true,
        revoker: Arc::new(crate::credentials::TransactionalRevoker),
        notifications: Arc::new(
            crate::sessions::security_notifications(db.pool.clone()).map_err(|e| e.to_string())?,
        ),
    };
    assert_eq!(
        credentials
            .change(
                &session_hash,
                crate::credentials::PasswordChange {
                    current_password: "original test password".into(),
                    password: "changed test password".into(),
                    password_confirmation: "changed test password".into(),
                }
            )
            .await?,
        crate::credentials::Outcome::Changed
    );
    sqlx::query("UPDATE credential_security_events SET created_at=now()-interval '2 days' WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    let facts = RecoveryFacts(db.pool.clone());
    let old = facts
        .load(id)
        .await?
        .password_changed_at
        .ok_or("ordinary change date")?;
    let token = "d".repeat(64);
    sqlx::query("INSERT INTO account_password_resets(account_id,token_hash,expires_at) VALUES($1,$2,now()+interval '1 hour')")
        .bind(id).bind(crate::verify::token_hash(&token)).execute(&db.pool).await?;
    assert!(
        crate::reset::complete(
            &db.pool,
            crate::reset::Input {
                email: "history@example.test".into(),
                token,
                password: "reset test password".into(),
                password_confirmation: "reset test password".into(),
            }
        )
        .await?
    );
    let reset_date = facts
        .load(id)
        .await?
        .password_changed_at
        .ok_or("reset date")?;
    assert!(reset_date > old);
    let response = production(db.pool.clone(), true)
        .layer(Extension(Viewer(id)))
        .oneshot(Request::get("/en/account/security/wizard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains(&reset_date.date().to_string()));
    assert!(!html.contains(&old.date().to_string()));
    sqlx::query("UPDATE credential_security_events SET created_at=now()-interval '1 day' WHERE account_id=$1 AND created_at>$2").bind(id).bind(old).execute(&db.pool).await?;
    let before_recovery = facts
        .load(id)
        .await?
        .password_changed_at
        .ok_or("reset history")?;
    let mut keys = crate::recovery_key::RecoveryKeys::new(db.pool.clone());
    keys.enabled = crate::recovery_key::Enabled(true);
    let key = keys.generate(id, "reset test password").await?;
    keys.confirm(id, "reset test password", &key).await?;
    keys.recover(
        "history@example.test",
        &key,
        "recovered test password",
        "recovered test password",
    )
    .await?;
    let recovered = facts
        .load(id)
        .await?
        .password_changed_at
        .ok_or("recovery date")?;
    assert!(recovered > before_recovery);
    let response = production(db.pool.clone(), true)
        .layer(Extension(Viewer(id)))
        .oneshot(Request::get("/en/account/security/wizard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains(&recovered.date().to_string()));
    assert!(!html.contains(&before_recovery.date().to_string()));
    eprintln!(
        "audit02_password_history_includes_reset_and_recovery duration: {:?}",
        started.elapsed()
    );
    db.close().await
}
