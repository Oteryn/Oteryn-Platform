#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! OTERYN_PAYMENT_PROVIDER_WEBHOOK_V1 through the real route, the real coin
//! purchase service and PostgreSQL. Only the wallet host is a test double.
use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use oteryn_payments::{
    TestProvider,
    coin::{CoinOrderRequest, CoinSettings, OrderState, parse_packages},
    coin_store::{
        CoinError, CoinHost, CoinPurchases, Debit, HostFuture, Ownership, WalletSnapshot,
    },
    provider_webhook::{
        MAX_BODY_BYTES, PATH, ProviderWebhook, SIGNATURE_HEADER, TIMESTAMP_HEADER, WebhookSettings,
        router, sign,
    },
};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SECRET: &str = "provider-webhook-test-secret-0123456789abcdef";
const PREVIOUS: &str = "provider-webhook-previous-secret-0123456789ab";
const PACKAGE: &str = "01934f10-7c02-7001-805b-3b1122334402";

/// Coins kept as keyed deltas in a test table; reversal never goes below zero.
#[derive(Debug)]
struct Host {
    pool: PgPool,
}
impl CoinHost for Host {
    fn payer<'a>(&'a self, _: &'a str) -> HostFuture<'a, Result<Uuid, CoinError>> {
        Box::pin(async { Err(CoinError::Unauthorized) })
    }
    fn ownership(&self, _: Uuid, _: Uuid) -> HostFuture<'_, Ownership> {
        Box::pin(async { Ownership::Owned })
    }
    fn restriction_in<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: Uuid,
    ) -> HostFuture<'a, Result<Option<String>, CoinError>> {
        Box::pin(async { Ok(None) })
    }
    fn credit_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<(), CoinError>> {
        Box::pin(async move {
            sqlx::query("INSERT INTO test_coins (key,payer,delta) VALUES ($1,$2,$3) ON CONFLICT (key) DO NOTHING")
                .bind(key).bind(payer).bind(coins)
                .execute(&mut **tx).await?;
            Ok(())
        })
    }
    fn reverse_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<Debit, CoinError>> {
        Box::pin(async move {
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))")
                .bind(payer)
                .execute(&mut **tx)
                .await?;
            let balance: i64 = sqlx::query_scalar(
                "SELECT COALESCE(sum(delta),0)::bigint FROM test_coins WHERE payer=$1",
            )
            .bind(payer)
            .fetch_one(&mut **tx)
            .await?;
            if balance < coins {
                return Ok(Debit::Insufficient);
            }
            sqlx::query("INSERT INTO test_coins (key,payer,delta) VALUES ($1,$2,$3)")
                .bind(key)
                .bind(payer)
                .bind(-coins)
                .execute(&mut **tx)
                .await?;
            Ok(Debit::Debited)
        })
    }
    fn wallet(&self, payer: Uuid) -> HostFuture<'_, Result<WalletSnapshot, CoinError>> {
        Box::pin(async move {
            let total: i64 = sqlx::query_scalar(
                "SELECT COALESCE(sum(delta),0)::bigint FROM test_coins WHERE payer=$1",
            )
            .bind(payer)
            .fetch_one(&self.pool)
            .await?;
            Ok(WalletSnapshot {
                revision: 1,
                total,
                available: total,
            })
        })
    }
}

struct Harness {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    provider: TestProvider,
}
impl Harness {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!(
                "SKIPPED PostgreSQL provider webhook test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("provider_webhook_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |conn, _| {
                let statement = search_path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(statement))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        sqlx::query("CREATE TABLE test_coins (key uuid PRIMARY KEY, payer uuid NOT NULL, delta bigint NOT NULL)")
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            provider: TestProvider::new(&[9; 32])?,
        }))
    }
    fn settings(&self, enabled: bool) -> CoinSettings {
        let packages = format!(
            r#"[{{"id":"{PACKAGE}","title":"500 coins","coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"valid_for_seconds":120}}]"#
        );
        CoinSettings {
            enabled,
            packages: parse_packages(&packages).unwrap(),
            origin: "https://portal.example".to_owned(),
            environment: "testing".to_owned(),
        }
    }
    fn coins(&self, enabled: bool) -> Arc<CoinPurchases> {
        Arc::new(CoinPurchases::new(
            self.pool.clone(),
            self.settings(enabled),
            self.provider.clone(),
            Arc::new(Host {
                pool: self.pool.clone(),
            }),
        ))
    }
    fn app_with(&self, coins: Arc<CoinPurchases>) -> Router {
        let settings = WebhookSettings::from_lookup(|key| match key {
            "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED" => Some("true".to_owned()),
            "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET" => Some(SECRET.to_owned()),
            "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET_PREVIOUS" => Some(PREVIOUS.to_owned()),
            _ => None,
        })
        .unwrap();
        router(Some(Arc::new(ProviderWebhook::new(
            self.pool.clone(),
            coins,
            self.provider.clone(),
            settings,
        ))))
    }
    fn app(&self) -> Router {
        self.app_with(self.coins(true))
    }
    /// A credited sandbox order: (payer, request id, provider order reference).
    async fn paid_order(&self) -> (Uuid, Uuid, String) {
        let coins = self.coins(true);
        let payer = Uuid::now_v7();
        let catalogue = coins.catalogue(payer, None).await.unwrap();
        let package = &catalogue.packages[0];
        let request = Uuid::now_v7();
        let order = coins
            .create_order(
                payer,
                &CoinOrderRequest {
                    request_id: request,
                    package_id: package.id,
                    quote_id: package.quote_id,
                    character_id: None,
                },
            )
            .await
            .unwrap();
        let paid = coins
            .sandbox_complete(payer, order.order_id, true)
            .await
            .unwrap();
        assert_eq!(paid.state, OrderState::Paid);
        let reference: String = sqlx::query_scalar("SELECT i.provider_reference FROM payment_intents i JOIN coin_purchase_orders o ON o.payment_intent_id=i.id WHERE o.id=$1")
            .bind(order.order_id)
            .fetch_one(&self.pool)
            .await
            .unwrap();
        assert_eq!(self.balance(payer).await, 500);
        (payer, request, reference)
    }
    async fn balance(&self, payer: Uuid) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(sum(delta),0)::bigint FROM test_coins WHERE payer=$1")
            .bind(payer)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn state(&self, payer: Uuid, request: Uuid) -> OrderState {
        self.coins(true).order(payer, request).await.unwrap().state
    }
    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn outcome(&self, event_id: &str) -> Option<String> {
        sqlx::query_scalar("SELECT outcome FROM payment_provider_webhook_events WHERE event_id=$1")
            .bind(event_id)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
    }
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

fn now() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}

fn event(event_id: &str, kind: &str, reference: &str, minor_units: i64) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "event_id": event_id,
        "type": kind,
        "order_reference": reference,
        "amount": {"minor_units": minor_units, "currency": "EUR"},
    }))
    .unwrap()
}

fn signed(body: &[u8], secret: &str, timestamp: i64) -> Request<Body> {
    Request::post(PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .header(TIMESTAMP_HEADER, timestamp.to_string())
        .header(
            SIGNATURE_HEADER,
            sign(secret.as_bytes(), timestamp, body).unwrap(),
        )
        .body(Body::from(body.to_vec()))
        .unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, body.to_vec())
}

async fn deliver(app: &Router, body: &[u8]) -> (StatusCode, Value) {
    let (status, bytes) = send(app, signed(body, SECRET, now())).await;
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

#[tokio::test]
async fn refund_reverses_the_credit_once() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    let body = event("evt_refund_1", "refund", &reference, 499);
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        reply,
        json!({"version": 1, "event_id": "evt_refund_1", "outcome": "applied", "replay": false})
    );
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(h.balance(payer).await, 0);
    assert_eq!(h.outcome("evt_refund_1").await.as_deref(), Some("applied"));
    assert_eq!(
        h.count(
            "SELECT count(*) FROM coin_purchase_reversals WHERE kind='refund' AND outcome='debited'"
        )
        .await,
        1
    );
    h.finish().await
}

#[tokio::test]
async fn chargeback_reverses_the_credit_once() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    let (status, reply) = deliver(&app, &event("evt_cb_1", "chargeback", &reference, 499)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reply["outcome"], "applied");
    assert_eq!(h.state(payer, request).await, OrderState::Chargeback);
    assert_eq!(h.balance(payer).await, 0);
    assert_eq!(
        h.count("SELECT count(*) FROM coin_purchase_reversals WHERE kind='chargeback'")
            .await,
        1
    );
    h.finish().await
}

#[tokio::test]
async fn previous_secret_is_accepted_during_rotation() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, _, reference) = h.paid_order().await;
    let body = event("evt_rotated", "refund", &reference, 499);
    let (status, _) = send(&h.app(), signed(&body, PREVIOUS, now())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(h.balance(payer).await, 0);
    h.finish().await
}

#[tokio::test]
async fn replay_with_the_same_body_applies_nothing() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    let body = event("evt_replay", "refund", &reference, 499);
    assert_eq!(deliver(&app, &body).await.0, StatusCode::OK);
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        reply,
        json!({"version": 1, "event_id": "evt_replay", "outcome": "applied", "replay": true})
    );
    assert_eq!(h.balance(payer).await, 0);
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(
        h.count("SELECT count(*) FROM coin_purchase_reversals")
            .await,
        1
    );
    assert_eq!(
        h.count("SELECT count(*) FROM payment_provider_webhook_events")
            .await,
        1
    );
    h.finish().await
}

#[tokio::test]
async fn same_id_with_another_body_conflicts_and_is_audited() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    assert_eq!(
        deliver(&app, &event("evt_dup", "refund", &reference, 499))
            .await
            .0,
        StatusCode::OK
    );
    let (status, reply) = deliver(&app, &event("evt_dup", "chargeback", &reference, 499)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(reply, Value::Null);
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(
        h.count("SELECT count(*) FROM audit_log WHERE action='payment_provider_webhook.event_replay_refused' AND target='evt_dup'")
            .await,
        1
    );
    assert_eq!(h.outcome("evt_dup").await.as_deref(), Some("applied"));
    h.finish().await
}

#[tokio::test]
async fn signature_and_timestamp_are_required() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    let body = event("evt_auth", "refund", &reference, 499);
    let stamp = now();
    let good = sign(SECRET.as_bytes(), stamp, &body).unwrap();
    let mut flipped = good.clone().into_bytes();
    let last = flipped.len() - 1;
    flipped[last] = if flipped[last] == b'0' { b'1' } else { b'0' };
    let flipped = String::from_utf8(flipped).unwrap();
    let request_with = |headers: &[(&str, String)]| {
        let mut builder = Request::post(PATH);
        for (name, value) in headers {
            builder = builder.header(*name, value);
        }
        builder.body(Body::from(body.clone())).unwrap()
    };
    let cases = [
        // wrong secret, flipped bit, uppercase hex, missing prefix
        vec![
            (TIMESTAMP_HEADER, stamp.to_string()),
            (
                SIGNATURE_HEADER,
                sign(b"another-secret-of-sufficient-length-000", stamp, &body).unwrap(),
            ),
        ],
        vec![
            (TIMESTAMP_HEADER, stamp.to_string()),
            (SIGNATURE_HEADER, flipped),
        ],
        vec![
            (TIMESTAMP_HEADER, stamp.to_string()),
            (SIGNATURE_HEADER, good.to_uppercase()),
        ],
        vec![
            (TIMESTAMP_HEADER, stamp.to_string()),
            (SIGNATURE_HEADER, good.trim_start_matches("v1=").to_owned()),
        ],
        // signed for another timestamp
        vec![
            (TIMESTAMP_HEADER, (stamp + 1).to_string()),
            (SIGNATURE_HEADER, good.clone()),
        ],
        // missing headers
        vec![(TIMESTAMP_HEADER, stamp.to_string())],
        vec![(SIGNATURE_HEADER, good.clone())],
        vec![],
        // duplicated header
        vec![
            (TIMESTAMP_HEADER, stamp.to_string()),
            (TIMESTAMP_HEADER, stamp.to_string()),
            (SIGNATURE_HEADER, good.clone()),
        ],
        // malformed timestamp
        vec![
            (TIMESTAMP_HEADER, format!("+{stamp}")),
            (SIGNATURE_HEADER, good.clone()),
        ],
    ];
    for headers in cases {
        let (status, reply) = send(&app, request_with(&headers)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{headers:?}");
        assert!(reply.is_empty());
    }
    // Stale or future timestamps outside 300 s, correctly signed.
    for offset in [-301, 301] {
        let (status, _) = send(&app, signed(&body, SECRET, stamp + offset)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{offset}");
    }
    // At the edge of the tolerance it is accepted.
    let (status, _) = send(&app, signed(&body, SECRET, now() - 299)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(
        h.count("SELECT count(*) FROM payment_provider_webhook_events")
            .await,
        1
    );
    h.finish().await
}

#[tokio::test]
async fn flag_off_or_bad_configuration_answers_503_before_reading_the_body() {
    let off = router(None);
    let huge = vec![b'x'; MAX_BODY_BYTES * 4];
    let (status, body) = send(&off, signed(&huge, SECRET, now())).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.is_empty());
    let lookup = |pairs: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_owned())
        }
    };
    const ON: (&str, &str) = ("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED", "true");
    const KEY: (&str, &str) = ("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET", SECRET);
    assert!(WebhookSettings::from_lookup(lookup(&[KEY])).is_none());
    assert!(
        WebhookSettings::from_lookup(lookup(&[
            ("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED", "1"),
            KEY
        ]))
        .is_none()
    );
    assert!(WebhookSettings::from_lookup(lookup(&[ON])).is_none());
    assert!(
        WebhookSettings::from_lookup(lookup(&[
            ON,
            ("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET", "short")
        ]))
        .is_none()
    );
    assert!(
        WebhookSettings::from_lookup(lookup(&[
            ON,
            KEY,
            ("PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET_PREVIOUS", "short")
        ]))
        .is_none()
    );
    assert!(WebhookSettings::from_lookup(lookup(&[ON, KEY])).is_some());
    // The composition without coin purchases is off as well.
    let (status, _) = send(
        &oteryn_payments::provider_webhook::configured_router(
            PgPoolOptions::new()
                .connect_lazy("postgres://localhost/none")
                .unwrap(),
            None,
        ),
        signed(b"{}", SECRET, now()),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn oversized_body_is_refused() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let app = h.app();
    let mut body = event("evt_big", "refund", "test:none", 499);
    body.pop();
    let padding = MAX_BODY_BYTES + 1 - body.len() - r#","pad":""}"#.len();
    body.extend_from_slice(format!(r#","pad":"{}"}}"#, "x".repeat(padding)).as_bytes());
    assert_eq!(body.len(), MAX_BODY_BYTES + 1);
    // Declared length over the limit.
    let mut declared = signed(&body, SECRET, now());
    declared
        .headers_mut()
        .insert(header::CONTENT_LENGTH, body.len().to_string().parse()?);
    assert_eq!(send(&app, declared).await.0, StatusCode::PAYLOAD_TOO_LARGE);
    // No declared length: the read itself stops at the limit.
    assert_eq!(
        send(&app, signed(&body, SECRET, now())).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        h.count("SELECT count(*) FROM payment_provider_webhook_events")
            .await,
        0
    );
    h.finish().await
}

#[tokio::test]
async fn unknown_order_is_refused_and_stored() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let app = h.app();
    let body = event("evt_unknown", "refund", "test:not-an-order", 499);
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply,
        json!({"version": 1, "event_id": "evt_unknown", "code": "PROVIDER_WEBHOOK_UNKNOWN_ORDER"})
    );
    assert_eq!(
        h.outcome("evt_unknown").await.as_deref(),
        Some("unknown_order")
    );
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reply["outcome"], "unknown_order");
    assert_eq!(reply["replay"], true);
    h.finish().await
}

#[tokio::test]
async fn unknown_type_is_stored_and_ignored() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    let body = serde_json::to_vec(&json!({
        "version": 1,
        "event_id": "evt_other",
        "type": "payout.created",
        "order_reference": reference,
        "anything": {"nested": true},
    }))?;
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reply["outcome"], "ignored");
    assert_eq!(h.outcome("evt_other").await.as_deref(), Some("ignored"));
    assert_eq!(h.state(payer, request).await, OrderState::Paid);
    assert_eq!(h.balance(payer).await, 500);
    h.finish().await
}

#[tokio::test]
async fn mismatch_and_illegal_reversals_are_refused_and_stored() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let app = h.app();
    // A partial amount is not this order's payment.
    let (status, reply) = deliver(&app, &event("evt_partial", "refund", &reference, 100)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply["code"], "PROVIDER_WEBHOOK_EVENT_REJECTED");
    assert_eq!(h.outcome("evt_partial").await.as_deref(), Some("rejected"));
    assert_eq!(h.balance(payer).await, 500);
    assert_eq!(h.state(payer, request).await, OrderState::Paid);
    // Refund, then a second refund under another event ID.
    assert_eq!(
        deliver(&app, &event("evt_r1", "refund", &reference, 499))
            .await
            .0,
        StatusCode::OK
    );
    let (status, reply) = deliver(&app, &event("evt_r2", "refund", &reference, 499)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply["code"], "PROVIDER_WEBHOOK_ILLEGAL_TRANSITION");
    assert_eq!(
        h.outcome("evt_r2").await.as_deref(),
        Some("illegal_transition")
    );
    assert_eq!(h.balance(payer).await, 0);
    h.finish().await
}

#[tokio::test]
async fn malformed_verified_body_is_refused_unrecorded() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let app = h.app();
    let bodies = [
        b"not json".to_vec(),
        serde_json::to_vec(&json!({"version": 2, "event_id": "e", "type": "refund"}))?,
        serde_json::to_vec(&json!({"version": 1, "event_id": "has space", "type": "x"}))?,
        serde_json::to_vec(
            &json!({"version": 1, "event_id": "e", "type": "refund", "order_reference": "r"}),
        )?,
        serde_json::to_vec(
            &json!({"version": 1, "event_id": "e", "type": "refund", "order_reference": "r", "amount": {"minor_units": 0, "currency": "EUR"}}),
        )?,
        serde_json::to_vec(
            &json!({"version": 1, "event_id": "e", "type": "refund", "order_reference": "r", "amount": {"minor_units": 1, "currency": "EUR"}, "extra": 1}),
        )?,
    ];
    for body in bodies {
        assert_eq!(deliver(&app, &body).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        h.count("SELECT count(*) FROM payment_provider_webhook_events")
            .await,
        0
    );
    h.finish().await
}

#[tokio::test]
async fn unknown_effect_records_nothing_and_a_retry_applies() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    let body = event("evt_retry", "refund", &reference, 499);
    // The coin service cannot decide (here: switched off): 503, nothing recorded.
    let (status, reply) = deliver(&h.app_with(h.coins(false)), &body).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply, Value::Null);
    assert_eq!(h.outcome("evt_retry").await, None);
    assert_eq!(h.balance(payer).await, 500);
    let (status, reply) = deliver(&h.app(), &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reply["replay"], false);
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(h.balance(payer).await, 0);
    h.finish().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_deliveries_of_one_event_apply_once() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    for round in 0..5 {
        let (payer, request, reference) = h.paid_order().await;
        let app = h.app();
        let body = event(&format!("evt_parallel_{round}"), "refund", &reference, 499);
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let tasks: Vec<_> = (0..2)
            .map(|_| {
                let (app, body, barrier) = (app.clone(), body.clone(), barrier.clone());
                tokio::spawn(async move {
                    let request = signed(&body, SECRET, now());
                    barrier.wait().await;
                    let response = app.oneshot(request).await.unwrap();
                    let status = response.status();
                    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
                    (status, serde_json::from_slice::<Value>(&bytes).unwrap())
                })
            })
            .collect();
        let mut replays = Vec::new();
        for task in tasks {
            let (status, reply) = task.await?;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(reply["outcome"], "applied");
            replays.push(reply["replay"].as_bool().unwrap());
        }
        replays.sort_unstable();
        assert_eq!(replays, [false, true], "exactly one delivery applies");
        assert_eq!(h.balance(payer).await, 0);
        assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    }
    assert_eq!(
        h.count("SELECT count(*) FROM coin_purchase_reversals")
            .await,
        5
    );
    h.finish().await
}

#[tokio::test]
async fn failure_after_the_event_insert_rolls_back_the_reversal_and_a_retry_applies_once()
-> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let (payer, request, reference) = h.paid_order().await;
    // Fails the delivery's commit, after the event row and the reversal were written.
    sqlx::query("CREATE FUNCTION test_fail_event() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'forced failure after the event insert'; END$$")
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE CONSTRAINT TRIGGER test_fail_event AFTER INSERT ON payment_provider_webhook_events DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION test_fail_event()")
        .execute(&h.pool)
        .await?;
    let app = h.app();
    let body = event("evt_atomic", "refund", &reference, 499);
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply, Value::Null);
    assert_eq!(h.outcome("evt_atomic").await, None);
    assert_eq!(h.balance(payer).await, 500);
    assert_eq!(h.state(payer, request).await, OrderState::Paid);
    assert_eq!(
        h.count("SELECT count(*) FROM coin_purchase_reversals")
            .await,
        0
    );
    assert_eq!(
        h.count(
            "SELECT count(*) FROM payment_events WHERE provider_event_id LIKE 'provider-webhook:%'"
        )
        .await,
        0
    );
    sqlx::query("DROP TRIGGER test_fail_event ON payment_provider_webhook_events")
        .execute(&h.pool)
        .await?;
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        reply,
        json!({"version": 1, "event_id": "evt_atomic", "outcome": "applied", "replay": false})
    );
    assert_eq!(h.outcome("evt_atomic").await.as_deref(), Some("applied"));
    assert_eq!(h.balance(payer).await, 0);
    assert_eq!(h.state(payer, request).await, OrderState::Refunded);
    assert_eq!(
        h.count(
            "SELECT count(*) FROM coin_purchase_reversals WHERE kind='refund' AND outcome='debited'"
        )
        .await,
        1
    );
    // A later delivery of the same event is a replay and applies nothing.
    let (status, reply) = deliver(&app, &body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reply["replay"], true);
    assert_eq!(h.balance(payer).await, 0);
    assert_eq!(
        h.count("SELECT count(*) FROM coin_purchase_reversals")
            .await,
        1
    );
    h.finish().await
}
