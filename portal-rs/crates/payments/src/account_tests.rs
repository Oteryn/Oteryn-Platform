use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn rejects_bad_paging() {
    for query in [
        "page=0",
        "page=-1",
        "page=10001",
        "page=9999999999999999999999",
        "page=1&page=2",
        "account_id=1",
        "page=a",
        "page=",
    ] {
        assert_eq!(paging(Some(query)), Err(StatusCode::BAD_REQUEST));
    }
    assert_eq!(paging(None), Ok(1));
    assert_eq!(paging(Some("page=2")), Ok(2));
}
#[test]
fn localized_responsive_semantics() -> TestResult {
    for lang in ["pl", "en"] {
        let locale = Locale::parse(lang).map_err(|_| "locale")?;
        for key in [
            "premium", "payments", "ACTIVE", "NONE", "status", "amount", "print",
        ] {
            assert!(!locale.text(key).is_empty());
        }
        let html = Page::new(locale, "payments").render()?;
        assert!(html.contains("width=device-width"));
        assert!(html.contains("<main"));
        assert!(html.contains("aria-label="));
        assert!(!html.contains("<table"));
    }
    Ok(())
}
#[tokio::test]
async fn anonymous_redirects_and_default_flag_hides_routes() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    for path in [
        "/en/panel/premium",
        "/en/panel/payments?page=bad",
        "/en/panel/payments/receipts/bad",
    ] {
        let response = router(pool.clone(), Flags::default())
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = router(
            pool.clone(),
            Flags {
                panel_premium_payments: true,
            },
        )
        .oneshot(Request::get(path).body(Body::empty())?)
        .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/en/login");
    }
    Ok(())
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl std::ops::Deref for Database {
    type Target = PgPool;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
impl Database {
    async fn finish(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
async fn database() -> Result<Option<Database>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping account panel DB test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("account_panel_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search_path = format!("SET search_path TO {schema}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _| {
            let statement = search_path.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(statement))
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    // Exercise the storage contracts without the unrelated duplicate migration versions.
    for migration in [
        include_str!("../../../migrations/20261105144852_t02_web_sessions.sql"),
        include_str!("../../../migrations/20261105153000_t31_payments.sql"),
        include_str!("../../../migrations/20261105160000_t27_premium_ledger.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await?;
    }
    Ok(Some(Database {
        pool,
        admin,
        schema,
    }))
}
async fn login(
    Extension(session): Extension<BrowserSession>,
    State(account): State<Uuid>,
) -> StatusCode {
    match oteryn_web::session::rotate(
        &session,
        oteryn_web::session::PrivilegeChange::Login(account),
    )
    .await
    {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
async fn cookie(pool: &PgPool, account: Uuid) -> Result<String, Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/login", get(login))
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .with_state(account);
    let response = app
        .oneshot(Request::get("/login").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned())
}
async fn seed(pool: &PgPool, account: Uuid) -> Result<(Uuid, Uuid), sqlx::Error> {
    let intent = Uuid::now_v7();
    let invoice = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$1::text,1234,'PLN','succeeded')").bind(intent).bind(account).execute(pool).await?;
    sqlx::query("INSERT INTO invoices(id,intent_id,account_id,amount,currency) VALUES ($1,$2,$3,1234,'PLN')").bind(invoice).bind(intent).bind(account).execute(pool).await?;
    Ok((intent, invoice))
}
async fn request(
    pool: &PgPool,
    cookie: &str,
    path: &str,
) -> Result<Response, Box<dyn std::error::Error>> {
    Ok(router(
        pool.clone(),
        Flags {
            panel_premium_payments: true,
        },
    )
    .oneshot(
        Request::get(path)
            .header(header::COOKIE, cookie)
            .body(Body::empty())?,
    )
    .await?)
}
async fn cleanup(pool: &PgPool, account: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM invoices WHERE account_id=$1")
        .bind(account)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM payment_intents WHERE account_id=$1")
        .bind(account)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .execute(pool)
        .await?;
    Ok(())
}
#[tokio::test]
async fn db_own_history_and_receipt_with_session() -> TestResult {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let (intent, invoice) = seed(&pool, account).await?;
    let cookie = cookie(&pool, account).await?;
    for path in [
        "/en/panel/payments".to_owned(),
        format!("/en/panel/payments/receipts/{invoice}"),
    ] {
        let response = request(&pool, &cookie, &path).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(html.contains("1234 PLN"));
        assert!(html.contains(&if path.ends_with(&invoice.to_string()) {
            invoice.to_string()
        } else {
            intent.to_string()
        }));
    }
    let response = request(&pool, &cookie, "/en/panel/premium").await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("No premium"));
    cleanup(&pool, account).await?;
    pool.finish().await?;
    Ok(())
}
#[tokio::test]
async fn db_other_accounts_receipt_is_404_and_history_is_private() -> TestResult {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    let (intent, invoice) = seed(&pool, owner).await?;
    let cookie = cookie(&pool, other).await?;
    let response = request(
        &pool,
        &cookie,
        &format!("/en/panel/payments/receipts/{invoice}"),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = request(&pool, &cookie, "/en/panel/payments").await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(!html.contains(&intent.to_string()));
    assert!(!html.contains(&invoice.to_string()));
    cleanup(&pool, owner).await?;
    cleanup(&pool, other).await?;
    pool.finish().await?;
    Ok(())
}
#[tokio::test]
async fn db_paging_and_bad_input() -> TestResult {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    for _ in 0..21 {
        seed(&pool, account).await?;
    }
    let cookie = cookie(&pool, account).await?;
    let response = request(&pool, &cookie, "/en/panel/payments").await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert_eq!(html.matches("<article").count(), 20);
    assert!(html.contains("rel=\"next\""));
    let response = request(&pool, &cookie, "/en/panel/payments?page=2").await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert_eq!(html.matches("<article").count(), 1);
    assert!(html.contains("rel=\"prev\""));
    for path in [
        "/en/panel/payments?page=0",
        "/en/panel/payments?page=1&page=2",
    ] {
        assert_eq!(
            request(&pool, &cookie, path).await?.status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(&pool, &cookie, "/en/panel/payments/receipts/bad")
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    cleanup(&pool, account).await?;
    pool.finish().await?;
    Ok(())
}

#[derive(Debug)]
struct KnownAccounts;
impl oteryn_store::premium::Accounts for KnownAccounts {
    fn state<'a>(
        &'a self,
        _: Uuid,
        _: &'a mut sqlx::PgConnection,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        oteryn_store::premium::AccountState,
                        oteryn_store::premium::Error,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(oteryn_store::premium::AccountState::Known) })
    }
}
#[tokio::test]
async fn db_premium_status_end_date_and_account_isolation() -> TestResult {
    use oteryn_store::premium::{Change, Mutation, Operator, repository::Ledger};
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    let ledger = Ledger::new(pool.clone(), std::sync::Arc::new(KnownAccounts), true);
    let operator = Operator {
        actor: Uuid::now_v7(),
        confirmed_mfa: true,
        permission: "products.premium.manage".into(),
    };
    let value = ledger
        .mutate(
            &operator,
            &Mutation {
                account: owner,
                request: Uuid::now_v7(),
                change: Change::Grant(30),
                reason: "Account panel test grant".into(),
            },
        )
        .await?;
    let owner_cookie = cookie(&pool, owner).await?;
    let response = request(&pool, &owner_cookie, "/en/panel/premium").await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Active"));
    assert!(
        html.contains(
            &date(OffsetDateTime::from_unix_timestamp(value.until)?).map_err(|_| "date")?
        )
    );
    let other_cookie = cookie(&pool, other).await?;
    let response = request(&pool, &other_cookie, "/en/panel/premium").await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("No premium"));
    ledger
        .mutate(
            &operator,
            &Mutation {
                account: owner,
                request: Uuid::now_v7(),
                change: Change::Revoke,
                reason: "Account panel test revocation".into(),
            },
        )
        .await?;
    let response = request(&pool, &owner_cookie, "/pl/panel/premium").await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Odebrane"));
    cleanup(&pool, owner).await?;
    cleanup(&pool, other).await?;
    pool.finish().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Chromium and PORTAL_BROWSER_MODULES with playwright-core and @axe-core/playwright"]
async fn db_browser_layout_labels_and_keyboard_at_360px() -> TestResult {
    let Some(database) = database().await? else {
        return Ok(());
    };
    let pool = database.pool.clone();
    let account = Uuid::now_v7();
    let (_, invoice) = seed(&pool, account).await?;
    let cookie = cookie(&pool, account).await?;
    let app = router(
        pool.clone(),
        Flags {
            panel_premium_payments: true,
        },
    )
    .route(
        "/static/css/tokens.css",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/css")],
                include_str!("../../../static/css/tokens.css"),
            )
        }),
    )
    .route(
        "/static/css/components.css",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/css")],
                include_str!("../../../static/css/components.css"),
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(
            std::env::var("PORTAL_BROWSER_NODE").unwrap_or_else(|_| "node".into()),
        )
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/account_browser.cjs"
        ))
        .env("PORTAL_BROWSER_URL", format!("http://{address}"))
        .env("PORTAL_BROWSER_COOKIE", cookie)
        .env("PORTAL_BROWSER_RECEIPT", invoice.to_string())
        .status()
    })
    .await?;
    server.abort();
    cleanup(&pool, account).await?;
    database.finish().await?;
    assert!(result?.success());
    Ok(())
}
