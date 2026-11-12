#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Native coin purchases through the real routes, OAuth grants, PostgreSQL
//! state and wallet ledger. Character ownership is the only stubbed adapter.
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_payments::{
    AccountReader, AccountState, Error as PaymentError, IntentState, PaymentEvent, PaymentFlags,
    Payments, WebhookOutcome,
    coin::{CoinBalance, CoinEvent, CoinOrderRequest, EventState, OrderState, parse_packages},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const REDIRECT: &str = "http://127.0.0.1:41000/callback";
const PACKAGE: &str = "01934f10-7c02-7001-805b-3b1122334402";
const ORIGIN: &str = "https://portal.example";
const KEY: &[u8] = b"coin-purchase-sandbox-test-key-0123456789";
const API: &str = "/api/v1/native-account/coin-purchases";

fn packages() -> String {
    format!(
        r#"[{{"id":"{PACKAGE}","title":"500 coins","coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"valid_for_seconds":120}}]"#
    )
}

/// The real host with a fixed set of owned characters.
#[derive(Debug)]
struct TestHost {
    host: Host,
    owned: Vec<(Uuid, Uuid)>,
}
impl CoinHost for TestHost {
    fn payer<'a>(&'a self, bearer: &'a str) -> HostFuture<'a, Result<Uuid, CoinError>> {
        self.host.payer(bearer)
    }
    fn ownership(&self, payer: Uuid, character: Uuid) -> HostFuture<'_, Ownership> {
        let owned = self.owned.contains(&(payer, character));
        Box::pin(async move {
            if owned {
                Ownership::Owned
            } else {
                Ownership::NotOwned
            }
        })
    }
    fn restriction_in<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        payer: Uuid,
    ) -> HostFuture<'a, Result<Option<String>, CoinError>> {
        self.host.restriction_in(connection, payer)
    }
    fn reverse_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<Debit, CoinError>> {
        self.host.reverse_in(tx, key, payer, coins)
    }
    fn credit_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<(), CoinError>> {
        self.host.credit_in(tx, key, payer, coins)
    }
    fn wallet(&self, payer: Uuid) -> HostFuture<'_, Result<WalletSnapshot, CoinError>> {
        self.host.wallet(payer)
    }
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    client: Uuid,
    config: oauth::Config,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED coin purchase PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        // Generated from a UUID only, never from input.
        let schema = format!("coin_native_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let client = Uuid::now_v7();
        sqlx::query("INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket) VALUES($1,true,true,true,true)")
            .bind(client).execute(&pool).await?;
        let config = oauth::Config {
            enabled: true,
            clients: vec![oauth::Client {
                client_id: client.to_string(),
                redirect_uris: vec![REDIRECT.into()],
            }],
        };
        Ok(Some(Self {
            pool,
            admin,
            schema,
            client,
            config,
        }))
    }
    async fn account(&self) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-hash','active','en',now()) RETURNING id")
            .bind(format!("{}@example.test", Uuid::now_v7())).fetch_one(&self.pool).await
    }
    /// Authorization code + PKCE exchange for one scope, as the native client does.
    async fn tokens(
        &self,
        account: Uuid,
        scope: &str,
    ) -> Result<oauth::Tokens, Box<dyn std::error::Error>> {
        let verifier = "v".repeat(43);
        let request = oauth::Authorization {
            response_type: "code".into(),
            client_id: self.client.to_string(),
            redirect_uri: REDIRECT.into(),
            scope: scope.into(),
            state: "test-state-for-browser".into(),
            code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            code_challenge_method: "S256".into(),
        };
        let code = oauth::authorize(&self.pool, &self.config, account, &request).await?;
        Ok(oauth::exchange(
            &self.pool,
            &self.config,
            &oauth::TokenRequest {
                grant_type: "authorization_code".into(),
                client_id: self.client.to_string(),
                redirect_uri: REDIRECT.into(),
                code,
                code_verifier: verifier,
                ..Default::default()
            },
        )
        .await?)
    }
    async fn token(&self, account: Uuid) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self
            .tokens(account, oauth::ACCOUNT_SERVICES_SCOPE)
            .await?
            .access_token)
    }
    fn service(&self, owned: Vec<(Uuid, Uuid)>) -> Arc<CoinPurchases> {
        let settings = CoinSettings {
            enabled: true,
            packages: parse_packages(&packages()).unwrap(),
            origin: ORIGIN.into(),
            environment: "testing".into(),
        };
        let host = TestHost {
            host: Host {
                pool: self.pool.clone(),
                oauth: self.config.clone(),
                projection: lcfa::Projection::new(
                    self.pool.clone(),
                    lcfa::Settings::default(),
                    Arc::new(lcfa::DenyAccounts),
                ),
            },
            owned,
        };
        Arc::new(CoinPurchases::new(
            self.pool.clone(),
            settings,
            provider(),
            Arc::new(host),
        ))
    }
    fn app(&self, service: &Arc<CoinPurchases>) -> Router {
        coin_http::router(self.pool.clone(), Some(service.clone()))
    }
    async fn count(&self, sql: &'static str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await
    }
    async fn close(self) -> TestResult {
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
fn provider() -> TestProvider {
    TestProvider::new(KEY).unwrap()
}

fn request(method: &str, path: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}
async fn send(app: Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    (
        status,
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
}
async fn get(app: Router, path: &str, token: &str) -> (StatusCode, Vec<u8>) {
    send(app, request("GET", path, Some(token), Body::empty())).await
}
async fn post(app: Router, token: &str, body: String) -> (StatusCode, Vec<u8>) {
    send(
        app,
        request(
            "POST",
            &format!("{API}/orders"),
            Some(token),
            Body::from(body),
        ),
    )
    .await
}
fn json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}
fn order_body(request: Uuid, quote: &str, character: Option<Uuid>) -> String {
    let character = character.map_or("null".to_owned(), |c| format!("\"{c}\""));
    format!(
        r#"{{"version":1,"request_id":"{request}","package_id":"{PACKAGE}","quote_id":"{quote}","character_id":{character}}}"#
    )
}
async fn quote(app: Router, token: &str) -> String {
    let (status, body) = get(app, &format!("{API}/catalogue"), token).await;
    assert_eq!(status, StatusCode::OK);
    json(&body)["packages"][0]["quote_id"]
        .as_str()
        .unwrap()
        .to_owned()
}
async fn top_ups(db: &Database, payer: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM wallet_operations WHERE account_id=$1 AND kind='top_up'",
    )
    .bind(payer)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn every_route_answers_503_when_off() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let order = Uuid::now_v7();
    for (method, path) in [
        ("GET", format!("{API}/catalogue")),
        ("GET", format!("{API}/balance")),
        ("POST", format!("{API}/orders")),
        ("GET", format!("{API}/requests/{order}")),
        ("GET", format!("/native-account/coin-checkout/{order}")),
        ("POST", format!("/native-account/coin-checkout/{order}")),
    ] {
        let (status, body) = send(
            coin_http::router(pool.clone(), None),
            request(method, &path, Some("token"), Body::empty()),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{method} {path}");
        assert!(body.is_empty());
    }
    let off = CoinSettings::from_lookup(|_| None);
    assert!(!off.enabled && off.packages.is_empty());
    Ok(())
}

#[tokio::test]
async fn catalogue_order_paid_credits_once() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);

    let (status, body) = get(db.app(&service), &format!("{API}/balance"), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        br#"{"version":1,"revision":1,"total_coins":0,"available_coins":0,"transferable_coins":0,"review_required":false}"#
    );

    let (status, body) = get(db.app(&service), &format!("{API}/catalogue"), &token).await;
    assert_eq!(status, StatusCode::OK);
    let catalogue = json(&body);
    let quote = catalogue["packages"][0]["quote_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        body,
        format!(
            r#"{{"version":1,"sandbox":true,"character_id":null,"packages":[{{"id":"{PACKAGE}","quote_id":"{quote}","title":"500 coins","coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"valid_for_seconds":120}}],"recent_orders":[]}}"#
        )
        .into_bytes()
    );

    let request_id = Uuid::now_v7();
    let (status, body) = post(
        db.app(&service),
        &token,
        order_body(request_id, &quote, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let order = json(&body)["order_id"].as_str().unwrap().to_owned();
    assert_eq!(
        body,
        format!(
            r#"{{"version":1,"order_id":"{order}","request_id":"{request_id}","revision":1,"package_id":"{PACKAGE}","quote_id":"{quote}","character_id":null,"coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"sandbox":true,"state":"awaiting_payment","checkout_url":"{ORIGIN}/native-account/coin-checkout/{order}","receipt_id":null,"review_required":false,"code":"COIN_ORDER_AWAITING_PAYMENT"}}"#
        )
        .into_bytes()
    );
    let order_id = Uuid::parse_str(&order)?;

    // Opening the checkout page never credits.
    let viewed = service.checkout(payer, order_id).await?;
    assert_eq!(viewed.state, OrderState::AwaitingPayment);
    assert_eq!(top_ups(&db, payer).await, 0);
    // Another payer cannot see or act on the order.
    let stranger = db.account().await?;
    assert_eq!(
        service.checkout(stranger, order_id).await,
        Err(CoinError::NotFound)
    );
    assert_eq!(
        service.sandbox_complete(stranger, order_id, true).await,
        Err(CoinError::NotFound)
    );

    let paid = service.sandbox_complete(payer, order_id, true).await?;
    assert_eq!(paid.state, OrderState::Paid);
    assert_eq!(top_ups(&db, payer).await, 1);
    // A duplicate verified delivery of the same event credits nothing.
    let intent: Uuid =
        sqlx::query_scalar("SELECT payment_intent_id FROM coin_purchase_orders WHERE id=$1")
            .bind(order_id)
            .fetch_one(&db.pool)
            .await?;
    let event = PaymentEvent {
        event_id: format!("coin-checkout:{intent}:paid"),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount {
            minor_units: 499,
            currency: "EUR".into(),
        },
        state: IntentState::Succeeded,
    };
    let raw = serde_json::to_vec(&event)?;
    let signature = provider().sign(&raw)?;
    assert_eq!(
        service.webhook(&raw, &signature).await?,
        WebhookOutcome::Replay
    );
    // A fresh event ID for the same payment cannot credit again either.
    let mut again = event.clone();
    again.event_id = format!("other:{intent}");
    let raw = serde_json::to_vec(&again)?;
    assert!(
        service
            .webhook(&raw, &provider().sign(&raw)?)
            .await
            .is_err()
    );
    // Unsigned or wrongly signed events are refused before parsing.
    assert!(matches!(
        service.webhook(&raw, &[0; 32]).await,
        Err(PaymentError::BadSignature)
    ));
    assert_eq!(top_ups(&db, payer).await, 1);

    let (status, body) = get(
        db.app(&service),
        &format!("{API}/requests/{request_id}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status_reply = json(&body);
    let receipt = status_reply["receipt_id"].as_str().unwrap().to_owned();
    assert_eq!(
        body,
        format!(
            r#"{{"version":1,"order_id":"{order}","request_id":"{request_id}","revision":2,"package_id":"{PACKAGE}","quote_id":"{quote}","character_id":null,"coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"sandbox":true,"state":"paid","checkout_url":null,"receipt_id":"{receipt}","review_required":false,"code":"COIN_ORDER_PAID"}}"#
        )
        .into_bytes()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM invoices WHERE id=$1::uuid AND account_id=$2"
        )
        .bind(&receipt)
        .bind(payer)
        .fetch_one(&db.pool)
        .await?,
        1
    );

    let (status, body) = get(db.app(&service), &format!("{API}/balance"), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        br#"{"version":1,"revision":2,"total_coins":500,"available_coins":500,"transferable_coins":500,"review_required":false}"#
    );
    // Payment again from the page is a no-op on a paid order.
    service.sandbox_complete(payer, order_id, true).await?;
    service.sandbox_complete(payer, order_id, false).await?;
    assert_eq!(
        service.order(payer, request_id).await?.state,
        OrderState::Paid
    );
    assert_eq!(top_ups(&db, payer).await, 1);
    // History is payer-scoped.
    let (_, body) = get(db.app(&service), &format!("{API}/catalogue"), &token).await;
    assert_eq!(json(&body)["recent_orders"][0]["order_id"], order.as_str());
    db.close().await
}

#[derive(Debug)]
struct Active;
impl AccountReader for Active {
    async fn state(&self, _: Uuid) -> Result<AccountState, PaymentError> {
        Ok(AccountState::Active)
    }
}

#[tokio::test]
async fn paid_never_follows_failed() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let quote = quote(db.app(&service), &token).await;
    let request_id = Uuid::now_v7();
    let order = service
        .create_order(
            payer,
            &CoinOrderRequest::decode(order_body(request_id, &quote, None).as_bytes()).unwrap(),
        )
        .await?;
    let failed = service
        .sandbox_complete(payer, order.order_id, false)
        .await?;
    assert_eq!(failed.state, OrderState::Failed);
    assert_eq!(
        (failed.revision, failed.checkout_url, failed.receipt_id),
        (2, None, None)
    );
    let intent: Uuid =
        sqlx::query_scalar("SELECT payment_intent_id FROM coin_purchase_orders WHERE id=$1")
            .bind(order.order_id)
            .fetch_one(&db.pool)
            .await?;
    let paid = PaymentEvent {
        event_id: format!("late-paid:{intent}"),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount {
            minor_units: 499,
            currency: "EUR".into(),
        },
        state: IntentState::Succeeded,
    };
    let raw = serde_json::to_vec(&paid)?;
    assert!(matches!(
        service.webhook(&raw, &provider().sign(&raw)?).await,
        Err(PaymentError::IllegalTransition)
    ));
    assert_eq!(
        service
            .sandbox_complete(payer, order.order_id, true)
            .await?
            .state,
        OrderState::Failed
    );
    // The generic payments webhook never applies events to a coin order.
    let generic = Payments::new(
        db.pool.clone(),
        provider(),
        Active,
        PaymentFlags { enabled: true },
    );
    assert!(matches!(
        generic.webhook(&raw, &provider().sign(&raw)?).await,
        Err(PaymentError::NotFound)
    ));
    // A failed order was never paid: a refund has nothing to reverse.
    let mut refund = paid.clone();
    refund.event_id = format!("refund:{intent}");
    refund.state = IntentState::Refunded;
    let raw = serde_json::to_vec(&refund)?;
    assert!(
        service
            .webhook(&raw, &provider().sign(&raw)?)
            .await
            .is_err()
    );
    let current = service.order(payer, request_id).await?;
    assert_eq!((current.state, current.revision), (OrderState::Failed, 2));
    assert_eq!(top_ups(&db, payer).await, 0);
    db.close().await
}

#[tokio::test]
async fn scopes_are_separate_tokens() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let service = db.service(vec![]);
    let coins = db.tokens(payer, oauth::ACCOUNT_SERVICES_SCOPE).await?;
    assert_eq!(coins.scope, "game:account-services");
    let ticket = db.tokens(payer, oauth::TICKET_SCOPE).await?;
    assert_eq!(ticket.scope, "game:ticket");

    // A game:ticket token never authorizes coin routes.
    for path in [
        "catalogue",
        "balance",
        "requests/01934f10-7c02-7001-805b-3b1122334401",
    ] {
        let (status, _) = get(
            db.app(&service),
            &format!("{API}/{path}"),
            &ticket.access_token,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let (status, _) = post(
        db.app(&service),
        &ticket.access_token,
        order_body(Uuid::now_v7(), PACKAGE, None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get(
        db.app(&service),
        &format!("{API}/balance"),
        &coins.access_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // An account-services token never verifies as a ticket token or mints a ticket.
    assert!(
        oauth::verify_access(&db.pool, &db.config, &coins.access_token)
            .await
            .is_err()
    );
    assert_eq!(
        oauth::verify_access(&db.pool, &db.config, &ticket.access_token).await?,
        payer
    );
    let player = oteryn_player_api::router(oteryn_player_api::Api {
        tickets: oteryn_gameauth::tickets::Tickets::new(
            db.pool.clone(),
            oteryn_gameauth::tickets::Settings {
                enabled: true,
                nonproduction: true,
            },
            crate::game_login::ticket_authority(db.config.clone()),
        ),
        projection: None,
    });
    let issue = |token: &str| {
        request(
            "POST",
            "/api/v1/game-auth/tickets",
            Some(token),
            Body::from(r#"{"protocol_version":1}"#),
        )
    };
    let response = player.clone().oneshot(issue(&coins.access_token)).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = player.oneshot(issue(&ticket.access_token)).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // A refresh keeps the family's single scope and cannot switch it.
    let refresh = |token: &str, scope: &str| oauth::TokenRequest {
        grant_type: "refresh_token".into(),
        client_id: db.client.to_string(),
        refresh_token: token.into(),
        scope: scope.into(),
        ..Default::default()
    };
    assert_eq!(
        oauth::exchange(
            &db.pool,
            &db.config,
            &refresh(&coins.refresh_token, oauth::TICKET_SCOPE)
        )
        .await
        .err(),
        Some(oauth::Error::InvalidGrant)
    );
    let fresh = db.tokens(payer, oauth::ACCOUNT_SERVICES_SCOPE).await?;
    let rotated = oauth::exchange(&db.pool, &db.config, &refresh(&fresh.refresh_token, "")).await?;
    assert_eq!(rotated.scope, "game:account-services");
    assert_eq!(
        oauth::verify_account_services(&db.pool, &db.config, &rotated.access_token).await?,
        payer
    );
    db.close().await
}

#[tokio::test]
async fn refusals_change_nothing() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let other = db.account().await?;
    let mine = Uuid::now_v7();
    let foreign = Uuid::now_v7();
    let token = db.token(payer).await?;
    let other_token = db.token(other).await?;
    let service = db.service(vec![(payer, mine), (other, foreign)]);

    // No or malformed credentials.
    let (status, _) = send(
        db.app(&service),
        request("GET", &format!("{API}/balance"), None, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get(db.app(&service), &format!("{API}/balance"), "not-a-token").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Character context: owned echoes it, foreign is 403 for catalogue and order.
    let (status, body) = get(
        db.app(&service),
        &format!("{API}/catalogue?character_id={mine}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let catalogue = json(&body);
    assert_eq!(catalogue["character_id"], mine.to_string());
    let bound = catalogue["packages"][0]["quote_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (status, _) = get(
        db.app(&service),
        &format!("{API}/catalogue?character_id={foreign}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &bound, Some(foreign)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // The quote binds its character: a context-free order cannot use it.
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &bound, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Another payer's quote cannot be used.
    let theirs = quote(db.app(&service), &other_token).await;
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &theirs, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Unknown quote.
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &Uuid::now_v7().to_string(), None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Expired quote.
    let expired = quote(db.app(&service), &token).await;
    sqlx::query(
        "UPDATE coin_purchase_quotes SET expires_at=now()-interval '1 second' WHERE id=$1::uuid",
    )
    .bind(&expired)
    .execute(&db.pool)
    .await?;
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &expired, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Consumed quote: one order, then another request ID for it conflicts.
    let consumed = quote(db.app(&service), &token).await;
    let first = Uuid::now_v7();
    let (status, created) =
        post(db.app(&service), &token, order_body(first, &consumed, None)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &consumed, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Changed replay: same request ID, different quote, package or character.
    let orders = db
        .count("SELECT count(*) FROM coin_purchase_orders")
        .await?;
    let unused = quote(db.app(&service), &token).await;
    for changed in [
        order_body(first, &unused, None),
        order_body(first, &consumed, Some(mine)),
        order_body(first, &consumed, None).replace(PACKAGE, &Uuid::now_v7().to_string()),
    ] {
        let (status, _) = post(db.app(&service), &token, changed).await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_orders")
            .await?,
        orders
    );
    let still: Option<bool> = sqlx::query_scalar(
        "SELECT consumed_at IS NULL FROM coin_purchase_quotes WHERE id=$1::uuid",
    )
    .bind(&unused)
    .fetch_optional(&db.pool)
    .await?;
    assert_eq!(still, Some(true));

    // Identical replay returns the same order, even after the quote expired.
    sqlx::query(
        "UPDATE coin_purchase_quotes SET expires_at=now()-interval '1 second' WHERE id=$1::uuid",
    )
    .bind(&consumed)
    .execute(&db.pool)
    .await?;
    let (status, replayed) =
        post(db.app(&service), &token, order_body(first, &consumed, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replayed, created);

    // Unknown requests and other payers' requests are 404.
    let (status, _) = get(
        db.app(&service),
        &format!("{API}/requests/{}", Uuid::now_v7()),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = get(
        db.app(&service),
        &format!("{API}/requests/{first}"),
        &other_token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Ineligible payer at order time (closed after the token was issued).
    let late = quote(db.app(&service), &other_token).await;
    sqlx::query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
        .bind(other)
        .execute(&db.pool)
        .await?;
    let request =
        CoinOrderRequest::decode(order_body(Uuid::now_v7(), &late, None).as_bytes()).unwrap();
    assert_eq!(
        service.create_order(other, &request).await,
        Err(CoinError::Conflict)
    );
    db.close().await
}

#[tokio::test]
async fn bad_input_is_400() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let quote = quote(db.app(&service), &token).await;
    let good = order_body(Uuid::now_v7(), &quote, None);
    for bad in [
        good.replace('}', r#","price":1}"#),
        good.replace(r#","character_id":null"#, ""),
        good.replace(&quote, &quote.to_uppercase()),
        good.replace(&quote, "6f1c1c1e-6b1a-4c1a-8f1a-1a1a1a1a1a1a"),
        good.replace(r#""version":1"#, r#""version":2"#),
        format!("{good}{}", " ".repeat(8192)),
        "not json".into(),
    ] {
        let (status, _) = post(db.app(&service), &token, bad.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad:.80}");
    }
    for path in [
        format!("{API}/requests/not-a-uuid"),
        format!(
            "{API}/requests/{}",
            Uuid::now_v7().to_string().to_uppercase()
        ),
        format!("{API}/requests/6f1c1c1e-6b1a-4c1a-8f1a-1a1a1a1a1a1a"),
        format!("{API}/catalogue?character_id=6f1c1c1e-6b1a-4c1a-8f1a-1a1a1a1a1a1a"),
        format!("{API}/catalogue?account_id={payer}"),
        format!(
            "{API}/catalogue?character_id={0}&character_id={0}",
            Uuid::now_v7()
        ),
        format!("{API}/balance?account_id={payer}"),
    ] {
        let (status, _) = get(db.app(&service), &path, &token).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
    }
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_orders")
            .await?,
        0
    );
    let (status, _) = post(db.app(&service), &token, good).await;
    assert_eq!(status, StatusCode::OK);
    db.close().await
}

#[tokio::test]
async fn concurrent_creates_have_one_winner() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let decode = |body: String| CoinOrderRequest::decode(body.as_bytes()).unwrap();

    // Different request IDs for one quote: exactly one order.
    let shared = quote(db.app(&service), &token).await;
    let (a, b) = (
        decode(order_body(Uuid::now_v7(), &shared, None)),
        decode(order_body(Uuid::now_v7(), &shared, None)),
    );
    let (first, second) = tokio::join!(
        service.create_order(payer, &a),
        service.create_order(payer, &b)
    );
    let outcomes = [first, second];
    assert_eq!(outcomes.iter().filter(|o| o.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == Err(CoinError::Conflict))
            .count(),
        1
    );

    // The identical request twice at once: both get the same order.
    let fresh = quote(db.app(&service), &token).await;
    let same = decode(order_body(Uuid::now_v7(), &fresh, None));
    let (first, second) = tokio::join!(
        service.create_order(payer, &same),
        service.create_order(payer, &same)
    );
    assert_eq!(first?.order_id, second?.order_id);
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_orders")
            .await?,
        2
    );

    // Concurrent payment completions credit once.
    let order = service.order(payer, same.request_id).await?;
    let (x, y) = tokio::join!(
        service.sandbox_complete(payer, order.order_id, true),
        service.sandbox_complete(payer, order.order_id, true)
    );
    assert_eq!(x?.state, OrderState::Paid);
    assert_eq!(y?.state, OrderState::Paid);
    assert_eq!(top_ups(&db, payer).await, 1);
    db.close().await
}

async fn awaiting_order(
    db: &Database,
    service: &Arc<CoinPurchases>,
    payer: Uuid,
    token: &str,
) -> Result<(Uuid, Uuid, Uuid), Box<dyn std::error::Error>> {
    let quote = quote(db.app(service), token).await;
    let request_id = Uuid::now_v7();
    let request = CoinOrderRequest::decode(order_body(request_id, &quote, None).as_bytes())
        .ok_or("order body")?;
    let order = service.create_order(payer, &request).await?;
    let intent: Uuid =
        sqlx::query_scalar("SELECT payment_intent_id FROM coin_purchase_orders WHERE id=$1")
            .bind(order.order_id)
            .fetch_one(&db.pool)
            .await?;
    Ok((order.order_id, intent, request_id))
}

#[tokio::test]
async fn ineligible_payer_at_payment_is_held_without_coins() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let (order, intent, request) = awaiting_order(&db, &service, payer, &token).await?;
    // The provider checkout exists while the payer is still eligible.
    service.checkout(payer, order).await?;
    // The account loses eligibility after ordering; then the provider's verified
    // payment event arrives.
    sqlx::query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
        .bind(payer)
        .execute(&db.pool)
        .await?;
    let event = PaymentEvent {
        event_id: format!("provider:{intent}:paid"),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount {
            minor_units: 499,
            currency: "EUR".into(),
        },
        state: IntentState::Succeeded,
    };
    let raw = serde_json::to_vec(&event)?;
    assert_eq!(
        service.webhook(&raw, &provider().sign(&raw)?).await?,
        WebhookOutcome::Applied
    );
    let held = service.order(payer, request).await?;
    assert_eq!(held.state, OrderState::Processing);
    assert!(held.review_required);
    assert_eq!(held.code, "COIN_ORDER_REVIEW_REQUIRED");
    assert_eq!((held.checkout_url, held.receipt_id), (None, None));
    assert_eq!(top_ups(&db, payer).await, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invoices WHERE account_id=$1")
            .bind(payer)
            .fetch_one(&db.pool)
            .await?,
        0
    );
    // A redelivery and the page stay held; nothing is ever credited.
    assert_eq!(
        service.webhook(&raw, &provider().sign(&raw)?).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Processing
    );
    assert_eq!(
        service.order(payer, request).await?.state,
        OrderState::Processing
    );
    assert_eq!(top_ups(&db, payer).await, 0);
    db.close().await
}

#[tokio::test]
async fn replayed_event_id_with_another_payload_is_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let (order, intent, _) = awaiting_order(&db, &service, payer, &token).await?;
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Paid
    );
    let exact = PaymentEvent {
        event_id: format!("coin-checkout:{intent}:paid"),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount {
            minor_units: 499,
            currency: "EUR".into(),
        },
        state: IntentState::Succeeded,
    };
    let mut amount = exact.clone();
    amount.amount.minor_units = 1;
    let mut currency = exact.clone();
    currency.amount.currency = "USD".into();
    let mut reference = exact.clone();
    reference.provider_reference = "test:other".into();
    for changed in [amount, currency, reference] {
        let raw = serde_json::to_vec(&changed)?;
        assert!(
            service
                .webhook(&raw, &provider().sign(&raw)?)
                .await
                .is_err(),
            "{changed:?}"
        );
    }
    // The exact replay is still recognised; one credit in total.
    let raw = serde_json::to_vec(&exact)?;
    assert_eq!(
        service.webhook(&raw, &provider().sign(&raw)?).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(top_ups(&db, payer).await, 1);
    assert_eq!(
        service.checkout(payer, order).await?.state,
        OrderState::Paid
    );
    db.close().await
}

// ---- Phase B: reversals, changed-payload replays, restricted payers. ----

fn coin_event(intent: Uuid, event_id: &str, state: EventState) -> CoinEvent {
    CoinEvent {
        event_id: event_id.to_owned(),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount {
            minor_units: 499,
            currency: "EUR".into(),
        },
        state,
    }
}
async fn deliver(
    service: &CoinPurchases,
    event: &CoinEvent,
) -> Result<WebhookOutcome, PaymentError> {
    let raw = serde_json::to_vec(event).unwrap();
    service.webhook(&raw, &provider().sign(&raw).unwrap()).await
}
async fn paid_order(
    db: &Database,
    service: &Arc<CoinPurchases>,
    payer: Uuid,
) -> Result<(Uuid, Uuid, Uuid), Box<dyn std::error::Error>> {
    let token = db.token(payer).await?;
    let (order, intent, request) = awaiting_order(db, service, payer, &token).await?;
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Paid
    );
    Ok((order, intent, request))
}
async fn wallet_of(service: &CoinPurchases, payer: Uuid) -> CoinBalance {
    service.balance(payer).await.unwrap()
}
async fn audits(db: &Database, action: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action=$1")
        .bind(action)
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
async fn reversal_debits(db: &Database, payer: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM wallet_operations WHERE account_id=$1 AND kind='adjustment' AND reason LIKE 'coin-purchase-reversal:%'")
        .bind(payer)
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn refund_then_chargeback_reverses_the_credit_once() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let (order, intent, request) = paid_order(&db, &service, payer).await?;
    assert_eq!(wallet_of(&service, payer).await.total_coins, 500);

    let refund = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    assert_eq!(deliver(&service, &refund).await?, WebhookOutcome::Applied);
    let (status, body) = get(
        db.app(&service),
        &format!("{API}/requests/{request}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let receipt = json(&body)["receipt_id"].as_str().unwrap().to_owned();
    assert_eq!(
        body,
        format!(
            r#"{{"version":1,"order_id":"{order}","request_id":"{request}","revision":3,"package_id":"{PACKAGE}","quote_id":"{}","character_id":null,"coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"sandbox":true,"state":"refunded","checkout_url":null,"receipt_id":"{receipt}","review_required":false,"code":"COIN_ORDER_REFUNDED"}}"#,
            json(&body)["quote_id"].as_str().unwrap()
        )
        .into_bytes()
    );
    let balance = wallet_of(&service, payer).await;
    assert_eq!(
        (
            balance.total_coins,
            balance.available_coins,
            balance.review_required
        ),
        (0, 0, false)
    );
    assert_eq!(reversal_debits(&db, payer).await, 1);
    assert_eq!(audits(&db, "coin_purchase.reversal_recorded").await, 1);
    assert_eq!(
        audits(&db, "wallet.coin_purchase_reversal.debited").await,
        1
    );

    // The same provider event again (a redelivery or a restart) is a replay.
    assert_eq!(deliver(&service, &refund).await?, WebhookOutcome::Replay);
    // A chargeback after the refund changes the state, never debits again.
    let chargeback = coin_event(
        intent,
        &format!("provider:{intent}:chargeback"),
        EventState::Chargeback,
    );
    assert_eq!(
        deliver(&service, &chargeback).await?,
        WebhookOutcome::Applied
    );
    let current = service.order(payer, request).await?;
    assert_eq!(
        (current.state, current.revision, current.code.as_str()),
        (OrderState::Chargeback, 4, "COIN_ORDER_CHARGEBACK")
    );
    assert_eq!(current.receipt_id.map(|r| r.to_string()), Some(receipt));
    // Chargeback is terminal; a late paid event cannot resurrect the order or
    // credit again, and another refund is refused.
    let late = coin_event(
        intent,
        &format!("provider:{intent}:late"),
        EventState::Succeeded,
    );
    assert_eq!(deliver(&service, &late).await?, WebhookOutcome::Applied);
    assert_eq!(audits(&db, "coin_purchase.settlement_fenced").await, 1);
    let again = coin_event(
        intent,
        &format!("provider:{intent}:again"),
        EventState::Refunded,
    );
    assert!(matches!(
        deliver(&service, &again).await,
        Err(PaymentError::IllegalTransition)
    ));
    assert_eq!(
        service.order(payer, request).await?.state,
        OrderState::Chargeback
    );
    assert_eq!(top_ups(&db, payer).await, 1);
    assert_eq!(reversal_debits(&db, payer).await, 1);
    assert_eq!(wallet_of(&service, payer).await.total_coins, 0);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM coin_purchase_reversals WHERE outcome='debited' AND debited=500"
        )
        .await?,
        1
    );
    let intent_state: String = sqlx::query_scalar("SELECT state FROM payment_intents WHERE id=$1")
        .bind(intent)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(intent_state, "chargeback");
    db.close().await
}

#[tokio::test]
async fn concurrent_reversals_debit_once_even_for_a_banned_payer() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let service = db.service(vec![]);
    let (_, intent, request) = paid_order(&db, &service, payer).await?;
    // A banned account still owes the reversed coins.
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(payer)
        .execute(&db.pool)
        .await?;
    let refund = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    let chargeback = coin_event(
        intent,
        &format!("provider:{intent}:cb"),
        EventState::Chargeback,
    );
    let (a, b, c, d) = tokio::join!(
        deliver(&service, &refund),
        deliver(&service, &refund),
        deliver(&service, &chargeback),
        deliver(&service, &chargeback),
    );
    let outcomes = [a, b, c, d];
    assert!(
        outcomes
            .iter()
            .any(|o| matches!(o, Ok(WebhookOutcome::Applied))),
        "{outcomes:?}"
    );
    assert_eq!(reversal_debits(&db, payer).await, 1);
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_reversals")
            .await?,
        1
    );
    let state = service.order(payer, request).await?.state;
    assert!(matches!(
        state,
        OrderState::Refunded | OrderState::Chargeback
    ));
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(payer)
        .execute(&db.pool)
        .await?;
    let balance = wallet_of(&service, payer).await;
    assert_eq!((balance.total_coins, balance.available_coins), (0, 0));
    db.close().await
}

#[tokio::test]
async fn reversal_of_unavailable_coins_holds_and_freezes_purchasing() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    // An order awaiting payment before the hold exists.
    let (pending, _, _) = awaiting_order(&db, &service, payer, &token).await?;
    let (order, intent, request) = paid_order(&db, &service, payer).await?;
    // 100 of the 500 purchased coins are reserved (e.g. a Bazaar bid).
    let host = Host {
        pool: db.pool.clone(),
        oauth: db.config.clone(),
        projection: lcfa::Projection::new(
            db.pool.clone(),
            lcfa::Settings::default(),
            Arc::new(lcfa::DenyAccounts),
        ),
    };
    host.ledger().reserve(Uuid::now_v7(), payer, 100).await?;
    let before = wallet_of(&service, payer).await;
    assert_eq!((before.total_coins, before.available_coins), (500, 400));

    let refund = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    assert_eq!(deliver(&service, &refund).await?, WebhookOutcome::Applied);
    let held = service.order(payer, request).await?;
    assert_eq!(
        (held.state, held.review_required, held.code.as_str()),
        (
            OrderState::Refunded,
            true,
            "COIN_ORDER_REVERSAL_REVIEW_REQUIRED"
        )
    );
    // Nothing was debited: no underflow, no partial erase, no mint.
    let after = wallet_of(&service, payer).await;
    assert_eq!(
        (
            after.total_coins,
            after.available_coins,
            after.review_required
        ),
        (500, 400, true)
    );
    assert_eq!(reversal_debits(&db, payer).await, 0);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM wallet_operations WHERE reason LIKE 'coin-purchase-reversal:%'"
        )
        .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT order_id FROM coin_purchase_holds WHERE account_id=$1 AND reason='reversal_insufficient_balance'")
            .bind(payer)
            .fetch_one(&db.pool)
            .await?,
        order
    );
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_reversals WHERE outcome='held' AND debited=0")
            .await?,
        1
    );

    // New purchasing is frozen, before any provider call, and audited.
    let quote = quote(db.app(&service), &token).await;
    let (status, _) = post(
        db.app(&service),
        &token,
        order_body(Uuid::now_v7(), &quote, None),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let intents_before = db.count("SELECT count(*) FROM payment_intents").await?;
    assert_eq!(
        service.checkout(payer, pending).await,
        Err(CoinError::Conflict)
    );
    assert_eq!(
        service.sandbox_complete(payer, pending, true).await,
        Err(CoinError::Conflict)
    );
    assert_eq!(
        db.count("SELECT count(*) FROM payment_intents").await?,
        intents_before
    );
    assert_eq!(
        db.count("SELECT count(*) FROM audit_log WHERE action='coin_purchase.payment_refused' AND after->>'reason'='coin_purchase_hold'")
            .await?,
        3
    );
    // A chargeback afterwards keeps the single held reversal and the hold.
    let chargeback = coin_event(
        intent,
        &format!("provider:{intent}:cb"),
        EventState::Chargeback,
    );
    assert_eq!(
        deliver(&service, &chargeback).await?,
        WebhookOutcome::Applied
    );
    let charged = service.order(payer, request).await?;
    assert_eq!(
        (charged.state, charged.code.as_str()),
        (
            OrderState::Chargeback,
            "COIN_ORDER_REVERSAL_REVIEW_REQUIRED"
        )
    );
    assert_eq!(wallet_of(&service, payer).await.total_coins, 500);
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_reversals")
            .await?,
        1
    );
    db.close().await
}

/// Test-only audit for a reward grant: accepts it and records the event.
#[derive(Debug, Clone, Copy)]
struct RewardAudit;
impl wallet::AuditHook for RewardAudit {
    async fn adjustment(&self, conn: &mut PgConnection, event: &wallet::AdjustmentAudit) -> bool {
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'test.wallet.reward',$2,'{}','{}',$3)")
            .bind(event.actor_id).bind(event.account_id.to_string()).bind(event.operation_id)
            .execute(conn).await.is_ok()
    }
}

/// One coin class (owner decision 2026-11-12): a reversal debits the one
/// balance, so coins of reward origin cover a purchase whose own coins are
/// reserved; the balance never goes negative and no hold is needed.
#[tokio::test]
async fn reversal_debits_reward_origin_coins() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let service = db.service(vec![]);
    let (_, intent, request) = paid_order(&db, &service, payer).await?;
    // 300 of the 500 purchased coins are reserved (e.g. a Bazaar bid), then a
    // 300-coin reward arrives: 500 unreserved coins, 200 of purchase origin.
    let ledger = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Policy,
        Policy,
        RewardAudit,
    );
    ledger.reserve(Uuid::now_v7(), payer, 300).await?;
    ledger
        .grant(
            Uuid::now_v7(),
            payer,
            300,
            Uuid::now_v7(),
            "Reward",
            wallet::GrantKind::Reward,
        )
        .await?;
    let before = wallet_of(&service, payer).await;
    assert_eq!(
        (
            before.total_coins,
            before.available_coins,
            before.transferable_coins
        ),
        (800, 500, 500)
    );

    let refund = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    assert_eq!(deliver(&service, &refund).await?, WebhookOutcome::Applied);
    let current = service.order(payer, request).await?;
    assert_eq!(
        (
            current.state,
            current.review_required,
            current.code.as_str()
        ),
        (OrderState::Refunded, false, "COIN_ORDER_REFUNDED")
    );
    let after = wallet_of(&service, payer).await;
    assert_eq!(
        (
            after.total_coins,
            after.available_coins,
            after.transferable_coins,
            after.review_required
        ),
        (300, 0, 0, false)
    );
    assert_eq!(reversal_debits(&db, payer).await, 1);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM coin_purchase_reversals WHERE outcome='debited' AND debited=500"
        )
        .await?,
        1
    );
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_holds").await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn reversal_before_credit_fences_the_payment() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let (order, intent, request) = awaiting_order(&db, &service, payer, &token).await?;
    service.checkout(payer, order).await?;
    // The provider reports a refund before the payment (out of order).
    let refund = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    assert_eq!(deliver(&service, &refund).await?, WebhookOutcome::Applied);
    let fenced = service.order(payer, request).await?;
    assert_eq!(
        (
            fenced.state,
            fenced.review_required,
            fenced.code.as_str(),
            fenced.checkout_url,
            fenced.receipt_id
        ),
        (
            OrderState::Processing,
            true,
            "COIN_ORDER_REVERSAL_REVIEW_REQUIRED",
            None,
            None
        )
    );
    // The later payment is retained and credits nothing.
    let paid = coin_event(
        intent,
        &format!("provider:{intent}:paid"),
        EventState::Succeeded,
    );
    assert_eq!(deliver(&service, &paid).await?, WebhookOutcome::Applied);
    assert_eq!(deliver(&service, &paid).await?, WebhookOutcome::Replay);
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Processing
    );
    assert_eq!(top_ups(&db, payer).await, 0);
    assert_eq!(wallet_of(&service, payer).await.total_coins, 0);
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_reversals WHERE outcome='fenced'")
            .await?,
        1
    );
    // No hold: no coins were ever at risk.
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_holds").await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn bad_reversal_events_are_refused() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let service = db.service(vec![]);
    let (_, intent, request) = paid_order(&db, &service, payer).await?;
    let good = coin_event(
        intent,
        &format!("provider:{intent}:refund"),
        EventState::Refunded,
    );
    // Wrong amount, currency, reference: refused and audited, nothing reversed.
    let mut amount = good.clone();
    amount.amount.minor_units = 498;
    let mut currency = good.clone();
    currency.amount.currency = "USD".into();
    let mut reference = good.clone();
    reference.provider_reference = "test:other".into();
    for bad in [&amount, &currency, &reference] {
        assert!(matches!(
            deliver(&service, bad).await,
            Err(PaymentError::InvalidInput)
        ));
    }
    assert_eq!(audits(&db, "coin_purchase.event_refused").await, 3);
    // Unsigned, unknown state, unknown member, oversized, unknown intent.
    let raw = serde_json::to_vec(&good)?;
    assert!(matches!(
        service.webhook(&raw, &[0; 32]).await,
        Err(PaymentError::BadSignature)
    ));
    for body in [
        String::from_utf8(raw.clone())?.replace("refunded", "partially_refunded"),
        String::from_utf8(raw.clone())?.replace('}', r#","coins":1}"#),
        format!("{}{}", String::from_utf8(raw.clone())?, " ".repeat(65536)),
    ] {
        let raw = body.into_bytes();
        assert!(matches!(
            service.webhook(&raw, &provider().sign(&raw)?).await,
            Err(PaymentError::InvalidInput)
        ));
    }
    let stranger = coin_event(Uuid::now_v7(), "provider:unknown", EventState::Refunded);
    assert!(matches!(
        deliver(&service, &stranger).await,
        Err(PaymentError::NotFound)
    ));
    let pending = coin_event(intent, "provider:pending", EventState::Pending);
    assert!(matches!(
        deliver(&service, &pending).await,
        Err(PaymentError::IllegalTransition)
    ));
    // Flag off: refused before verification.
    let off = CoinPurchases::new(
        db.pool.clone(),
        CoinSettings::default(),
        provider(),
        Arc::new(TestHost {
            host: Host {
                pool: db.pool.clone(),
                oauth: db.config.clone(),
                projection: lcfa::Projection::new(
                    db.pool.clone(),
                    lcfa::Settings::default(),
                    Arc::new(lcfa::DenyAccounts),
                ),
            },
            owned: vec![],
        }),
    );
    assert!(matches!(
        deliver(&off, &good).await,
        Err(PaymentError::Disabled)
    ));
    assert_eq!(service.order(payer, request).await?.state, OrderState::Paid);
    assert_eq!(reversal_debits(&db, payer).await, 0);
    assert_eq!(wallet_of(&service, payer).await.total_coins, 500);
    // The good event still applies afterwards.
    assert_eq!(deliver(&service, &good).await?, WebhookOutcome::Applied);
    assert_eq!(wallet_of(&service, payer).await.total_coins, 0);
    db.close().await
}

#[tokio::test]
async fn changed_payload_replays_are_refused_and_audited() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    let service = db.service(vec![]);
    let consumed = quote(db.app(&service), &token).await;
    let first = Uuid::now_v7();
    let (status, created) =
        post(db.app(&service), &token, order_body(first, &consumed, None)).await;
    assert_eq!(status, StatusCode::OK);
    // The identical replay is not audited.
    let (status, replayed) =
        post(db.app(&service), &token, order_body(first, &consumed, None)).await;
    assert_eq!((status, &replayed), (StatusCode::OK, &created));
    assert_eq!(audits(&db, "coin_purchase.request_replay_refused").await, 0);
    let unused = quote(db.app(&service), &token).await;
    let (status, _) = post(db.app(&service), &token, order_body(first, &unused, None)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let row: (Option<Uuid>, String, String, String) = sqlx::query_as("SELECT actor,target,before->>'quote_id',after->>'quote_id' FROM audit_log WHERE action='coin_purchase.request_replay_refused'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        row,
        (
            Some(payer),
            first.to_string(),
            consumed.clone(),
            unused.clone()
        )
    );
    assert_eq!(
        db.count("SELECT count(*) FROM coin_purchase_orders")
            .await?,
        1
    );

    // A provider event ID replayed with another state is refused, changes
    // nothing and is retained for audit with both payloads.
    let order: Uuid = json(&created)["order_id"].as_str().unwrap().parse()?;
    let intent: Uuid =
        sqlx::query_scalar("SELECT payment_intent_id FROM coin_purchase_orders WHERE id=$1")
            .bind(order)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Paid
    );
    let reused = coin_event(
        intent,
        &format!("coin-checkout:{intent}:paid"),
        EventState::Refunded,
    );
    assert!(matches!(
        deliver(&service, &reused).await,
        Err(PaymentError::InvalidInput)
    ));
    let audit: (String, String) = sqlx::query_as("SELECT before->>'state',after->>'state' FROM audit_log WHERE action='coin_purchase.event_replay_refused'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(audit, ("succeeded".to_owned(), "refunded".to_owned()));
    assert_eq!(
        service.checkout(payer, order).await?.state,
        OrderState::Paid
    );
    assert_eq!(wallet_of(&service, payer).await.total_coins, 500);
    assert_eq!(reversal_debits(&db, payer).await, 0);
    db.close().await
}

#[tokio::test]
async fn restricted_payers_are_refused_before_any_provider_call() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let staff = db.account().await?;
    let service = db.service(vec![]);
    let restrictions: [(&str, &str); 8] = [
        (
            "UPDATE accounts SET status='suspended' WHERE id=$1",
            "account_suspended",
        ),
        (
            "UPDATE accounts SET status='closed' WHERE id=$1",
            "account_closed",
        ),
        (
            "UPDATE accounts SET status='pending_verification' WHERE id=$1",
            "account_pending_verification",
        ),
        (
            "UPDATE accounts SET email_verified_at=NULL WHERE id=$1",
            "email_unverified",
        ),
        (
            "UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1",
            "email_change_lock",
        ),
        (
            "INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '30 days')",
            "termination_pending",
        ),
        (
            "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'suspension','active','test',now()-interval '1 minute',now()+interval '1 day',$2)",
            "sanction",
        ),
        (
            "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','test',now()-interval '1 minute',now()+interval '1 day',$2)",
            "sanction",
        ),
    ];
    for (restrict, reason) in restrictions {
        let payer = db.account().await?;
        let token = db.token(payer).await?;
        let quote = quote(db.app(&service), &token).await;
        let (pending, _, _) = awaiting_order(&db, &service, payer, &token).await?;
        sqlx::query(restrict)
            .bind(payer)
            .bind(staff)
            .execute(&db.pool)
            .await?;
        let request =
            CoinOrderRequest::decode(order_body(Uuid::now_v7(), &quote, None).as_bytes()).unwrap();
        let intents = db.count("SELECT count(*) FROM payment_intents").await?;
        assert_eq!(
            service.create_order(payer, &request).await,
            Err(CoinError::Conflict),
            "{reason}"
        );
        assert_eq!(
            service.checkout(payer, pending).await,
            Err(CoinError::Conflict),
            "{reason}"
        );
        assert_eq!(
            service.sandbox_complete(payer, pending, true).await,
            Err(CoinError::Conflict),
            "{reason}"
        );
        // No order, no consumed quote, no provider intent, no coins.
        assert_eq!(
            db.count("SELECT count(*) FROM payment_intents").await?,
            intents,
            "{reason}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM coin_purchase_orders WHERE payer_account_id=$1"
            )
            .bind(payer)
            .fetch_one(&db.pool)
            .await?,
            1
        );
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT consumed_at IS NULL FROM coin_purchase_quotes WHERE id=$1::uuid"
            )
            .bind(&quote)
            .fetch_one(&db.pool)
            .await?,
            "{reason}"
        );
        assert_eq!(top_ups(&db, payer).await, 0);
        let audited: Vec<(Option<Uuid>, String)> = sqlx::query_as("SELECT actor,after->>'stage' FROM audit_log WHERE action='coin_purchase.payment_refused' AND target=$1 AND after->>'reason'=$2 ORDER BY id")
            .bind(payer.to_string())
            .bind(reason)
            .fetch_all(&db.pool)
            .await?;
        assert_eq!(
            audited,
            vec![
                (Some(payer), "order".to_owned()),
                (Some(payer), "checkout".to_owned()),
                (Some(payer), "sandbox_payment".to_owned()),
            ],
            "{reason}"
        );
    }
    // A warning or an expired sanction does not restrict.
    let payer = db.account().await?;
    let token = db.token(payer).await?;
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'warning','active','test',now()-interval '1 minute',now()+interval '1 day',$2),($1,'suspension','expired','test',now()-interval '2 days',now()-interval '1 day',$2)")
        .bind(payer)
        .bind(staff)
        .execute(&db.pool)
        .await?;
    let (order, _, _) = awaiting_order(&db, &service, payer, &token).await?;
    assert_eq!(
        service.sandbox_complete(payer, order, true).await?.state,
        OrderState::Paid
    );
    db.close().await
}
