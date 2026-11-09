use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn package_configuration_fails_closed() {
    assert!(!valid_packages(&[]));
    let p = Package {
        key: "fixture".into(),
        coins: 100,
        minor_units: 250,
        currency: "PLN".into(),
    };
    assert!(valid_packages(std::slice::from_ref(&p)));
    assert!(!valid_packages(&[p.clone(), p.clone()]));
    assert!(!valid_packages(&[Package {
        coins: 0,
        ..p.clone()
    }]));
    assert!(!valid_packages(&[Package {
        currency: "pln".into(),
        ..p
    }]));
}
#[tokio::test]
async fn postgres_payment_intent_replay_conflict_and_interrupted_claim() -> TestResult {
    let Some((pool, admin, schema)) = database().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'fixture@example.invalid',now(),'unused-test-hash','active','en')").bind(account).execute(&pool).await?;
    let topups = PaymentTopUps {
        signer: TestProvider::new(&[5; 32])?,
        pool: pool.clone(),
        payments: Payments::new(
            pool.clone(),
            TestProvider::new(&[5; 32])?,
            Accounts(pool.clone()),
            PaymentFlags { enabled: true },
        ),
        packages: vec![
            Package {
                key: "fixture".into(),
                coins: 100,
                minor_units: 250,
                currency: "PLN".into(),
            },
            Package {
                key: "other".into(),
                coins: 200,
                minor_units: 500,
                currency: "PLN".into(),
            },
        ],
    };
    let topups = Arc::new(topups);
    let app = oteryn_store::web::router(
        pool.clone(),
        oteryn_store::web::Flags {
            shop: true,
            top_ups: true,
            ..Default::default()
        },
        oteryn_store::web::configured_backend(pool.clone(), Some(topups.clone())),
        vec!["https://portal.example".into()],
    );
    let (cookie, csrf) = browser_session(&pool, account).await?;
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'other@example.invalid',now(),'unused-test-hash','active','en')").bind(other).execute(&pool).await?;
    let (other_cookie, other_csrf) = browser_session(&pool, other).await?;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for action in ["pay", "reject", "cancel"] {
        // Each decision is an independent scenario with its own limiter window.
        sqlx::query("DELETE FROM store_wire_limits WHERE scope='purchase'")
            .execute(&pool)
            .await?;
        let operation = Uuid::now_v7();
        let form = format!("_csrf={csrf}&operation={operation}&package=fixture");
        let response = app
            .clone()
            .oneshot(browser_post("/en/shop/coins", &cookie, form)?)
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers()["location"].to_str()?.to_owned();
        let id = Uuid::parse_str(location.rsplit('/').next().ok_or("missing intent")?)?;
        let page = app
            .clone()
            .oneshot(
                Request::get(&location)
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(page.status(), StatusCode::OK);
        assert_eq!(page.headers()["cache-control"], "no-store");
        let html = String::from_utf8(
            axum::body::to_bytes(page.into_body(), 65536)
                .await?
                .to_vec(),
        )?;
        assert!(html.contains("TEST MODE – no real money is charged"));
        assert!(html.contains("Pay (test)"));
        assert!(html.contains("PLN 2.50"));
        let foreign = app
            .clone()
            .oneshot(
                Request::get(&location)
                    .header("cookie", &other_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
        let foreign = app
            .clone()
            .oneshot(browser_post(
                &location,
                &other_cookie,
                format!("_csrf={other_csrf}&action=pay"),
            )?)
            .await?;
        assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
        let foreign_post = app
            .clone()
            .oneshot(browser_post(
                &location,
                &other_cookie,
                "_csrf=invalid&action=pay".into(),
            )?)
            .await?;
        assert_eq!(foreign_post.status(), StatusCode::FORBIDDEN);
        let invalid = app
            .clone()
            .oneshot(browser_post(
                &location,
                &cookie,
                format!("_csrf={csrf}&action=unknown"),
            )?)
            .await?;
        assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
        for _ in 0..2 {
            let result = app
                .clone()
                .oneshot(browser_post(
                    &location,
                    &cookie,
                    format!("_csrf={csrf}&action={action}"),
                )?)
                .await?;
            assert_eq!(result.status(), StatusCode::SEE_OTHER);
        }
        let status = topups
            .checkout(account, id, None)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("missing checkout")?;
        assert!(!status.pending);
        assert_eq!(
            status.status,
            if action == "pay" {
                "checkout-succeeded"
            } else {
                "checkout-failed"
            }
        );
        let jobs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1",
        )
        .bind(id.to_string())
        .fetch_one(&pool)
        .await?;
        assert_eq!(jobs, i64::from(action == "pay"));
        let events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM payment_events WHERE intent_id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(events, 1);
        if action == "pay" {
            use crate::worker::Handler;
            let (job_id, payload): (Uuid, String) = sqlx::query_as("SELECT id,payload::text FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1").bind(id.to_string()).fetch_one(&pool).await?;
            let job = oteryn_db::outbox::Job {
                id: job_id,
                kind: "wallet.top_up".into(),
                payload,
                attempts: 1,
            };
            let delivery = crate::wallet_topup::Delivery {
                pool: pool.clone(),
                payments_enabled: true,
                wallet_enabled: true,
            };
            delivery.run(&job).await?;
            delivery.run(&job).await?;
            let balance: i64 = sqlx::query_scalar(
                "SELECT SUM(available_delta)::bigint FROM wallet_entries WHERE account_id=$1",
            )
            .bind(account)
            .fetch_one(&pool)
            .await?;
            assert_eq!(balance, 100);
            assert_eq!(
                topups
                    .checkout(account, id, None)
                    .await
                    .map_err(|e| format!("{e:?}"))?
                    .ok_or("missing checkout")?
                    .status,
                "checkout-credited"
            );
        }
    }
    sqlx::query(
        "TRUNCATE payment_events, invoices, outbox, store_web_topups, payment_intents CASCADE",
    )
    .execute(&pool)
    .await?;
    let operation = Uuid::now_v7();
    let first = topups.create(account, operation, "fixture".into()).await;
    assert!(first.is_ok());
    assert_eq!(
        first,
        topups.create(account, operation, "fixture".into()).await
    );
    assert_eq!(
        topups.create(account, operation, "other".into()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        topups.create(account, Uuid::nil(), "fixture".into()).await,
        Err(Error::Invalid)
    );
    assert_eq!(
        topups
            .create(account, Uuid::now_v7(), "unknown".into())
            .await,
        Err(Error::Invalid)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_entries")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let interrupted = Uuid::now_v7();
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency) VALUES($1,$2,'fixture',100,250,'PLN')").bind(interrupted).bind(account).execute(&pool).await?;
    assert_eq!(
        topups.create(account, interrupted, "fixture".into()).await,
        Err(Error::Unavailable)
    );
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert_eq!(
        topups
            .create(account, Uuid::now_v7(), "fixture".into())
            .await,
        Err(Error::Unavailable)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}

async fn browser_session(pool: &PgPool, account: Uuid) -> Result<(String, String), sqlx::Error> {
    use sha2::{Digest, Sha256};
    let token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    let csrf = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(&csrf).bind(account).execute(pool).await?;
    Ok((
        format!("{}={token}", oteryn_web::session::COOKIE_NAME),
        csrf,
    ))
}
fn browser_post(
    uri: &str,
    cookie: &str,
    body: String,
) -> Result<axum::http::Request<axum::body::Body>, axum::http::Error> {
    axum::http::Request::post(uri)
        .header("cookie", cookie)
        .header("origin", "https://portal.example")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(body))
}

#[tokio::test]
async fn checkout_and_webhook_routes_absent_outside_testing_gate() -> TestResult {
    use tower::ServiceExt;
    let pool = oteryn_db::lazy_pool("postgres://unused:unused@127.0.0.1:1/unused")?;
    for (testing, enabled) in [(false, false), (false, true), (true, false)] {
        let app = router_for(pool.clone(), testing, enabled);
        for method in [axum::http::Method::GET, axum::http::Method::POST] {
            let response = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method(method)
                        .uri(format!("/en/shop/test-checkout/{}", Uuid::now_v7()))
                        .body(axum::body::Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        }
        let response = app
            .oneshot(
                axum::http::Request::post("/__test/payments/webhook")
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
    Ok(())
}

async fn database() -> Result<Option<(PgPool, PgPool, String)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED shop payment PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("shop_payments_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    Ok(Some((pool, admin, schema)))
}

#[tokio::test]
#[ignore = "requires Chromium and browser modules"]
async fn postgres_dummy_checkout_browser_360px() -> TestResult {
    use crate::worker::Handler;
    let Some((pool, admin, schema)) = database().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let recipient = Uuid::now_v7();
    for (id, email) in [
        (account, "browser@example.invalid"),
        (recipient, "recipient@example.invalid"),
    ] {
        sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'unused-test-hash','active','en')").bind(id).bind(email).execute(&pool).await?;
    }
    let topups = Arc::new(PaymentTopUps {
        signer: TestProvider::new(&[5; 32])?,
        pool: pool.clone(),
        payments: Payments::new(
            pool.clone(),
            TestProvider::new(&[5; 32])?,
            Accounts(pool.clone()),
            PaymentFlags { enabled: true },
        ),
        packages: vec![Package {
            key: "fixture".into(),
            coins: 100,
            minor_units: 250,
            currency: "PLN".into(),
        }],
    });
    let (cookie, _) = browser_session(&pool, account).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = oteryn_store::web::router(
        pool.clone(),
        oteryn_store::web::Flags {
            shop: true,
            top_ups: true,
            transfers: true,
            purchases: false,
        },
        oteryn_store::web::configured_backend(pool.clone(), Some(topups)),
        vec![format!("http://{address}")],
    )
    .route(
        "/static/css/tokens.css",
        axum::routing::get(|| async {
            (
                [("content-type", "text/css")],
                include_str!("../../../static/css/tokens.css"),
            )
        }),
    )
    .route(
        "/static/css/components.css",
        axum::routing::get(|| async {
            (
                [("content-type", "text/css")],
                include_str!("../../../static/css/components.css"),
            )
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move {
        let delivery = crate::wallet_topup::Delivery {
            pool: worker_pool.clone(),
            payments_enabled: true,
            wallet_enabled: true,
        };
        loop {
            let jobs = match oteryn_db::outbox::claim(&worker_pool, 10, 30).await {
                Ok(jobs) => jobs,
                Err(error) => return Err::<(), sqlx::Error>(error),
            };
            for job in jobs {
                let error = delivery.run(&job).await.err();
                oteryn_db::outbox::finish(&worker_pool, &job, error.as_deref(), false).await?;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(
            std::env::var("PORTAL_BROWSER_NODE").unwrap_or_else(|_| "node".into()),
        )
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/store/src/web_checkout_browser.cjs"
        ))
        .env("PORTAL_BROWSER_URL", format!("http://{address}"))
        .env("PORTAL_BROWSER_COOKIE", cookie)
        .env("PORTAL_BROWSER_RECIPIENT", recipient.to_string())
        .status()
    })
    .await?;
    server.abort();
    worker.abort();
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    assert!(result?.success());
    Ok(())
}
