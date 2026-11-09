use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{Binding, Identity, IdentityMap, VerifiedClient},
};
use oteryn_wallet::wire::{self, Flags};
use tower::ServiceExt;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/contracts/fixtures/coin-wire-v1")
            .join(name),
    )
    .unwrap()
}
fn parse(raw: &[u8]) -> Value {
    canonical::decode(raw, 65536, 8).unwrap()
}
fn member<'a>(v: &'a Value, key: &str) -> &'a str {
    match v.get(key).unwrap() {
        Value::String(s) => s,
        _ => panic!("string"),
    }
}
fn set(v: &mut Value, key: &str, value: Value) {
    let Value::Object(m) = v else {
        panic!("object")
    };
    m.iter_mut().find(|(k, _)| k == key).unwrap().1 = value;
}
fn map() -> IdentityMap {
    IdentityMap::new(vec![Binding {
        identity: Identity::san("coins.test.invalid").unwrap(),
        group: "game-coins".into(),
    }])
    .unwrap()
}
fn app(ledger: TestLedger, flags: Flags) -> Router {
    wire::router(ledger, flags, map())
}
fn enabled() -> Flags {
    Flags {
        transfers: true,
        market: true,
    }
}
async fn call(
    app: Router,
    path: &str,
    body: Option<Vec<u8>>,
    verified: bool,
) -> (StatusCode, Vec<u8>) {
    let mut req = if let Some(body) = body {
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    } else {
        Request::get(path).body(Body::empty()).unwrap()
    };
    if verified {
        req.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san("coins.test.invalid").unwrap(),
            ));
    }
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    if status != StatusCode::UNAUTHORIZED {
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["content-type"], "application/json");
    }
    (
        status,
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
}
fn ids() -> (Uuid, Uuid, Uuid, Uuid) {
    let t = parse(&fixture("transfers-request.json"));
    let o = parse(&fixture("offers-request.json"));
    (
        Uuid::parse_str(member(&t, "sender_account_id")).unwrap(),
        Uuid::parse_str(member(&t, "recipient_account_id")).unwrap(),
        Uuid::parse_str(member(&t, "operation_id")).unwrap(),
        Uuid::parse_str(member(&o, "operation_id")).unwrap(),
    )
}
fn normalize_receipt(expected: &mut Value, actual: &Value) {
    let actual = actual.get("receipt").unwrap();
    let Value::Object(m) = expected else {
        panic!("object")
    };
    let expected = &mut m.iter_mut().find(|(k, _)| k == "receipt").unwrap().1;
    set(
        expected,
        "occurred_at",
        actual.get("occurred_at").unwrap().clone(),
    );
}
#[tokio::test]
async fn fixtures_round_trip_real_wallet_handlers_for_every_route() {
    for route in [
        "transfers",
        "offers",
        "settlements",
        "cancellations",
        "operation",
        "balance",
        "history",
    ] {
        let Some(db) = Database::open().await else {
            return;
        };
        let (a, b, key, offer) = ids();
        let ledger = db.ledger(&[a, b]);
        fund(&ledger, a, 100).await;
        ledger
            .credit_non_transferable(id(), a, 5, id(), "Reward", GrantKind::Reward)
            .await
            .unwrap();
        if ["settlements", "cancellations"].contains(&route) {
            ledger.reserve_offer(offer, a, 30, 100).await.unwrap();
        }
        if route == "cancellations" {
            ledger.settle(id(), offer, b, 10).await.unwrap();
        }
        if ["operation", "balance", "history"].contains(&route) {
            ledger.transfer(key, a, b, 10).await.unwrap();
        }
        let (path, body) = match route {
            "operation" => (format!("/internal/v1/coins/operations/{key}"), None),
            "balance" => (format!("/internal/v1/coins/accounts/{a}/balance"), None),
            "history" => (
                format!("/internal/v1/coins/accounts/{a}/history?limit=1"),
                None,
            ),
            _ => (
                format!(
                    "/internal/v1/coins/{}{}",
                    if route == "transfers" { "" } else { "market/" },
                    route
                ),
                Some(fixture(&format!("{route}-request.json"))),
            ),
        };
        let router = app(ledger, enabled());
        let (status, bytes) = call(router.clone(), &path, body.clone(), true).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{route}: {}",
            String::from_utf8_lossy(&bytes)
        );
        let actual = parse(&bytes);
        let mut expected = parse(&fixture(&format!("{route}-ok.json")));
        if ["balance", "history"].contains(&route) {
            if route == "history" {
                let Value::Array(actual_entries) = actual.get("entries").unwrap() else {
                    panic!("array")
                };
                let Value::Object(m) = &mut expected else {
                    panic!("object")
                };
                let Value::Array(entries) =
                    &mut m.iter_mut().find(|(k, _)| k == "entries").unwrap().1
                else {
                    panic!("array")
                };
                set(
                    &mut entries[0],
                    "entry_id",
                    actual_entries[0].get("entry_id").unwrap().clone(),
                );
                set(
                    &mut entries[0],
                    "occurred_at",
                    actual_entries[0].get("occurred_at").unwrap().clone(),
                );
                set(
                    &mut expected,
                    "next_cursor",
                    actual.get("next_cursor").unwrap().clone(),
                );
                let cursor = member(&actual, "next_cursor");
                let (status, next) = call(
                    router.clone(),
                    &format!("/internal/v1/coins/accounts/{a}/history?cursor={cursor}&limit=1"),
                    None,
                    true,
                )
                .await;
                assert_eq!(status, StatusCode::OK);
                assert_ne!(member(&parse(&next), "next_cursor"), cursor);
            }
        } else {
            normalize_receipt(&mut expected, &actual);
        }
        assert_eq!(canonical::encode(&expected).unwrap(), bytes, "{route}");
        if route == "transfers" {
            let (status, replay) = call(router, &path, body, true).await;
            assert_eq!(status, StatusCode::OK);
            let mut expected = parse(&fixture("replay.json"));
            normalize_receipt(&mut expected, &parse(&replay));
            assert_eq!(canonical::encode(&expected).unwrap(), replay);
        }
        db.close().await;
    }
}
#[tokio::test]
async fn refusal_fixtures_and_invalid_inputs_use_exact_bodies() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b, _, _) = ids();
    let path = "/internal/v1/coins/transfers";
    let raw = fixture("transfers-request.json");
    let check = |code: &str, status: StatusCode, bytes: Vec<u8>| {
        assert_eq!(
            status,
            if ["invalid", "unknown_account"].contains(&code) {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::CONFLICT
            }
        );
        assert_eq!(bytes, fixture(&format!("{code}.json")));
    };
    let (s, r) = call(
        app(db.ledger(&[a, b]), enabled()),
        path,
        Some(raw.clone()),
        true,
    )
    .await;
    check("insufficient_transferable", s, r);
    let inactive = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Inactive,
        Blocks {
            blocked: None,
            unknown: false,
        },
        Audit(true),
    );
    let (s, r) = call(
        wire::router(inactive, enabled(), map()),
        path,
        Some(raw.clone()),
        true,
    )
    .await;
    check("account_inactive", s, r);
    let (s, r) = call(
        app(db.ledger(&[a]), enabled()),
        path,
        Some(raw.clone()),
        true,
    )
    .await;
    check("unknown_account", s, r);
    fund(&db.ledger(&[a, b]), a, 100).await;
    let blocked = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Accounts(vec![a, b]),
        Blocks {
            blocked: Some(a),
            unknown: false,
        },
        Audit(true),
    );
    let (s, r) = call(app(blocked, enabled()), path, Some(raw.clone()), true).await;
    check("transfer_blocked", s, r);
    let router = app(db.ledger(&[a, b]), enabled());
    assert_eq!(
        call(router.clone(), path, Some(raw), true).await.0,
        StatusCode::OK
    );
    let mut changed = parse(&fixture("transfers-request.json"));
    set(&mut changed, "amount", Value::String("11".into()));
    let (s, r) = call(
        router.clone(),
        path,
        Some(canonical::encode(&changed).unwrap()),
        true,
    )
    .await;
    check("idempotency_conflict", s, r);
    for name in [
        "zero",
        "negative",
        "overflow",
        "same-account",
        "bad-key",
        "extra",
    ] {
        let p = if name == "extra" {
            "/internal/v1/coins/market/offers"
        } else {
            path
        };
        let (s, r) = call(
            router.clone(),
            p,
            Some(fixture(&format!("invalid-{name}.json"))),
            true,
        )
        .await;
        check("invalid", s, r);
    }
    for raw in [
        b"bad".to_vec(),
        b"{\"x\":1,\"x\":2}".to_vec(),
        vec![b'x'; 1025],
        b"{\"a\":[[[[]]]]}".to_vec(),
        b"{}\n".to_vec(),
    ] {
        let (s, r) = call(router.clone(), path, Some(raw), true).await;
        check("invalid", s, r);
    }
    for query in [
        "limit=0",
        "limit=101",
        "limit=01",
        "cursor=bad",
        "limit=1&limit=2",
        "extra=1",
    ] {
        let (s, r) = call(
            router.clone(),
            &format!("/internal/v1/coins/accounts/{a}/history?{query}"),
            None,
            true,
        )
        .await;
        check("invalid", s, r);
    }
    let (s, r) = call(
        router,
        &format!("/internal/v1/coins/operations/{}", id()),
        None,
        true,
    )
    .await;
    check("unknown_account", s, r);
    db.close().await;
}
#[tokio::test]
async fn flags_transport_and_peer_provenance_fail_closed_without_database() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(20))
        .connect_lazy("postgresql://127.0.0.1:1/unused")
        .unwrap();
    let make = || {
        Ledger::new(
            pool.clone(),
            WalletFlags { enabled: true },
            Accounts(vec![]),
            Blocks {
                blocked: None,
                unknown: false,
            },
            Audit(false),
        )
    };
    for path in [
        "transfers",
        "market/offers",
        "market/settlements",
        "market/cancellations",
        "operations/019a65f0-0000-7000-8000-000000000003",
        "accounts/019a65f0-0000-7000-8000-000000000001/balance",
        "accounts/019a65f0-0000-7000-8000-000000000001/history",
    ] {
        let read = path.starts_with("accounts/") || path.starts_with("operations/");
        let body = if read {
            None
        } else {
            Some(b"invalid".to_vec())
        };
        let path = format!("/internal/v1/coins/{path}");
        let (s, r) = call(app(make(), Flags::default()), &path, body.clone(), false).await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(r, fixture("unavailable.json"));
        let (s, r) = call(app(make(), enabled()), &path, body, false).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert!(r.is_empty());
    }
    let (s, r) = call(
        app(make(), enabled()),
        "/internal/v1/coins/transfers",
        Some(fixture("transfers-request.json")),
        true,
    )
    .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r, fixture("unavailable.json"));
    for (flags, path) in [
        (
            Flags {
                transfers: true,
                market: false,
            },
            "/internal/v1/coins/market/offers",
        ),
        (
            Flags {
                transfers: false,
                market: true,
            },
            "/internal/v1/coins/transfers",
        ),
    ] {
        let (status, bytes) =
            call(app(make(), flags), path, Some(b"invalid".to_vec()), false).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(bytes, fixture("unavailable.json"));
    }
    let mut request =
        Request::get("/internal/v1/coins/accounts/019a65f0-0000-7000-8000-000000000001/balance")
            .body(Body::from("x"))
            .unwrap();
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(
            Identity::san("coins.test.invalid").unwrap(),
        ));
    let response = app(make(), enabled()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        to_bytes(response.into_body(), 65536).await.unwrap(),
        fixture("invalid.json")
    );
    // A peer cannot delay an off/unauthenticated refusal by withholding a body.
    for (flags, expected) in [
        (Flags::default(), StatusCode::SERVICE_UNAVAILABLE),
        (enabled(), StatusCode::UNAUTHORIZED),
    ] {
        let request = Request::get(
            "/internal/v1/coins/accounts/019a65f0-0000-7000-8000-000000000001/balance",
        )
        .body(Body::from_stream(futures_util::stream::pending::<
            Result<axum::body::Bytes, std::io::Error>,
        >()))
        .unwrap();
        let response =
            tokio::time::timeout(Duration::from_secs(1), app(make(), flags).oneshot(request))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(wire::identities(vec![]).is_err());
    assert!(
        wire::identities(vec![
            (
                "PORTAL_GAME_COINS_IDENTITIES".into(),
                "{\"coins.test.invalid\":[]}".into()
            ),
            (
                "OTHER_IDENTITIES".into(),
                "{\"coins.test.invalid\":[]}".into()
            )
        ])
        .is_err()
    );
    assert!(
        wire::identities(vec![(
            "PORTAL_GAME_COINS_IDENTITIES".into(),
            "{\"coins.test.invalid\":[]}".into()
        )])
        .is_ok()
    );
    pool.close().await;
}
