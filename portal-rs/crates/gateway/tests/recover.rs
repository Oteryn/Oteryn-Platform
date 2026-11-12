//! Gateway login contract §19.4, §19.7, §19.9 and §19.10: `POST /v1/recover`
//! and `POST /v1/recover/revoke`, byte for byte against
//! `docs/contracts/fixtures/native-gateway-protocol3-v1/`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
};
use oteryn_gateway::{Options, issuer::IssuerClient, recover, router_with};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::ServiceExt;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/",
            $name
        ))
    };
}
const REQUEST: &str = fixture!("recover-request.json");
const SUCCESS: &str = fixture!("recover-success.json");
const MALFORMED: &str = fixture!("recover-request-malformed.json");
const UNSUPPORTED: &str = fixture!("recover-unsupported-version.json");
const OFFER: &str = fixture!("recover-offer-unsupported.json");
const RATE_LIMITED: &str = fixture!("recover-rate-limited.json");
const RECONCILIATION: &str = fixture!("recover-reconciliation-required.json");
const UNAVAILABLE: &str = fixture!("recover-unavailable.json");
const PENDING: &str = fixture!("recover-attempt-pending.json");
const AUTHENTICATION: &str = fixture!("recover-authentication-required.json");
const REVOKE_REQUEST: &str = fixture!("recover-revoke-request.json");
const REVOKE_SUCCESS: &str = fixture!("recover-revoke-success.json");
const REVOKE_MALFORMED: &str = fixture!("recover-revoke-malformed.json");
/// Every relayed §19.8 refusal and its status.
const REFUSALS: [(u16, &str); 11] = [
    (409, fixture!("recover-attempt-conflict.json")),
    (409, PENDING),
    (401, AUTHENTICATION),
    (409, fixture!("recover-character-conflict.json")),
    (409, fixture!("recover-grant-expired.json")),
    (409, OFFER),
    (429, RATE_LIMITED),
    (503, RECONCILIATION),
    (503, fixture!("recover-route-unavailable.json")),
    (503, UNAVAILABLE),
    (409, fixture!("recover-world-stale.json")),
];
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";
const HANDLE: &str = "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";

struct Server {
    url: String,
    calls: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Clone)]
struct Reply {
    path: &'static str,
    /// The exact private request, or `None` for any request.
    expected: Option<&'static str>,
    status: u16,
    body: String,
    retry_after: Option<&'static str>,
    hang: bool,
    calls: Arc<AtomicUsize>,
}
async fn reply(State(s): State<Reply>, r: Request<Body>) -> Response {
    s.calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(r.uri().path(), s.path);
    assert_eq!(
        r.headers()["authorization"],
        "Bearer synthetic-gateway-service-token"
    );
    // The private request is the public member set, compact, in contract order.
    let body = to_bytes(r.into_body(), 2048).await.unwrap();
    if let Some(expected) = s.expected {
        assert_eq!(body.as_ref(), expected.as_bytes());
    }
    if s.hang {
        std::future::pending::<()>().await;
    }
    let mut response = (StatusCode::from_u16(s.status).unwrap(), s.body).into_response();
    if let Some(value) = s.retry_after {
        response
            .headers_mut()
            .insert("retry-after", value.parse().unwrap());
    }
    response
}
async fn serve(state: Reply) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls = state.calls.clone();
    let app = Router::new()
        .route(state.path, any(reply))
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Server { url, calls, task }
}
async fn issuer(status: u16, body: &str) -> Server {
    issuer_with(status, body, None, false).await
}
async fn issuer_with(
    status: u16,
    body: &str,
    retry_after: Option<&'static str>,
    hang: bool,
) -> Server {
    serve(Reply {
        path: recover::RECOVER_PATH,
        expected: Some(REQUEST),
        status,
        body: body.to_owned(),
        retry_after,
        hang,
        calls: Arc::default(),
    })
    .await
}
async fn revoke_issuer(status: u16, body: &str) -> Server {
    serve(Reply {
        path: recover::REVOKE_PATH,
        expected: Some(REVOKE_REQUEST),
        status,
        body: body.to_owned(),
        retry_after: None,
        hang: false,
        calls: Arc::default(),
    })
    .await
}
/// An issuer that answers every recovery request the same way.
async fn any_issuer(status: u16, body: &str) -> Server {
    serve(Reply {
        path: recover::RECOVER_PATH,
        expected: None,
        status,
        body: body.to_owned(),
        retry_after: None,
        hang: false,
        calls: Arc::default(),
    })
    .await
}
fn client(url: &str) -> IssuerClient {
    IssuerClient::new(
        url,
        "synthetic-gateway-service-token".into(),
        Duration::from_millis(300),
        Duration::from_millis(300),
    )
    .unwrap()
}
fn app(url: &str) -> Router {
    router_with(client(url), options(true, true))
}
fn options(enabled: bool, protocol3: bool) -> Options {
    Options {
        enabled,
        protocol3,
        version: "test".into(),
    }
}
struct Answer {
    status: StatusCode,
    body: String,
    retry_after: Option<String>,
}
async fn post_from(app: Router, path: &str, body: &str, source: [u8; 4]) -> Answer {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((source, 40000))));
    let r = app.oneshot(request).await.unwrap();
    let status = r.status();
    if status != StatusCode::NOT_FOUND {
        assert_eq!(
            r.headers()["cache-control"],
            "no-store, no-cache, must-revalidate, private"
        );
    }
    let retry_after = r
        .headers()
        .get("retry-after")
        .map(|v| v.to_str().unwrap().to_owned());
    let raw = to_bytes(r.into_body(), 16384).await.unwrap();
    Answer {
        status,
        body: String::from_utf8(raw.to_vec()).unwrap(),
        retry_after,
    }
}
async fn post(app: Router, path: &str, body: &str) -> (StatusCode, String) {
    let a = post_from(app, path, body, [192, 0, 2, 1]).await;
    (a.status, a.body)
}
fn line(body: &str) -> String {
    format!("{body}\n")
}

#[tokio::test]
async fn recover_switches_keep_the_route_default_off() {
    // Native login off: the routes do not exist, as on the Go gateway.
    for protocol3 in [false, true] {
        let app = router_with(client("http://127.0.0.1:1"), options(false, protocol3));
        for (path, body) in [
            ("/v1/recover", REQUEST),
            ("/v1/recover/revoke", REVOKE_REQUEST),
        ] {
            assert_eq!(
                post(app.clone(), path, body).await,
                (StatusCode::NOT_FOUND, String::new())
            );
        }
    }
    // Protocol 3 off: every protocol 3 request is NATIVE_LOGIN_UNSUPPORTED_VERSION
    // in the protocol 2 body (§19.1), and the issuer is never called.
    let upstream = issuer(200, SUCCESS).await;
    let app = router_with(client(&upstream.url), options(true, false));
    assert_eq!(
        post(app.clone(), "/v1/recover", REQUEST).await,
        (
            StatusCode::BAD_REQUEST,
            line(&format!(
                "{{\"protocol_version\":2,\"error\":{{\"code\":\"NATIVE_LOGIN_UNSUPPORTED_VERSION\",\"public_class\":\"CLIENT_UPDATE_REQUIRED\"}},\"attempt_ref\":\"{ATTEMPT}\"}}"
            ))
        )
    );
    assert_eq!(
        post(app, "/v1/recover/revoke", REVOKE_REQUEST).await,
        (
            StatusCode::BAD_REQUEST,
            line(
                "{\"protocol_version\":2,\"error\":{\"code\":\"NATIVE_LOGIN_UNSUPPORTED_VERSION\",\"public_class\":\"CLIENT_UPDATE_REQUIRED\"},\"attempt_ref\":null}"
            )
        )
    );
    assert_eq!(upstream.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn recover_success_and_revoke_relay_byte_exact() {
    let upstream = issuer(200, SUCCESS).await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover", REQUEST).await,
        (StatusCode::OK, line(SUCCESS))
    );
    assert_eq!(upstream.calls.load(Ordering::SeqCst), 1);
    let upstream = revoke_issuer(200, REVOKE_SUCCESS).await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover/revoke", REVOKE_REQUEST).await,
        (StatusCode::OK, line(REVOKE_SUCCESS))
    );
    assert_eq!(upstream.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn recover_refusals_relay_byte_exact() {
    for (status, body) in REFUSALS {
        let retry = matches!(status, 409 | 429).then_some("7");
        let upstream = issuer_with(status, body, retry, false).await;
        let answer = post_from(app(&upstream.url), "/v1/recover", REQUEST, [192, 0, 2, 1]).await;
        assert_eq!(
            (answer.status.as_u16(), answer.body.as_str()),
            (status, line(body).as_str())
        );
        // Retry-After only where the code carries it (§10, §19.4 step 3).
        let expected = (body == PENDING || body == RATE_LIMITED).then(|| "7".to_owned());
        assert_eq!(answer.retry_after, expected, "{body}");
    }
    // Internal SECURITY_TERMINAL codes collapse to the public code (§19.8).
    for internal in [
        "NATIVE_RECOVERY_HANDLE_REJECTED",
        "NATIVE_RECOVERY_DEVICE_PROOF_REJECTED",
        "NATIVE_RECOVERY_ACCOUNT_SECURITY_DENIED",
    ] {
        let upstream = issuer(
            401,
            &AUTHENTICATION.replace("NATIVE_LOGIN_AUTHENTICATION_REQUIRED", internal),
        )
        .await;
        assert_eq!(
            post(app(&upstream.url), "/v1/recover", REQUEST).await,
            (StatusCode::UNAUTHORIZED, line(AUTHENTICATION))
        );
    }
    // A revoke refusal relays with `attempt_ref: null`.
    let upstream = revoke_issuer(400, REVOKE_MALFORMED).await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover/revoke", REVOKE_REQUEST).await,
        (StatusCode::BAD_REQUEST, line(REVOKE_MALFORMED))
    );
}

#[tokio::test]
async fn recover_rejects_bad_input_before_the_issuer() {
    let upstream = issuer(200, SUCCESS).await;
    let app = app(&upstream.url);
    let malformed = [
        // Unknown, missing, null and duplicate members.
        REQUEST.replace("\"offer\":", "\"channel_id\":null,\"offer\":"),
        REQUEST.replace(
            "\"character_id\":\"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c\",",
            "",
        ),
        REQUEST.replace("\"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c\"", "null"),
        REQUEST.replace(
            "\"protocol_version\":3,",
            "\"protocol_version\":3,\"protocol_version\":3,",
        ),
        // §19.15 binding is not built; `device_proof` is not in the member set yet.
        REQUEST.replace("]}}", "]},\"device_proof\":{\"kid\":\"k\",\"sig\":\"s\"}}"),
        // A handle that is not `otr1.` and 43 base64url characters of 32 bytes.
        REQUEST.replace(HANDLE, "otr1.short"),
        REQUEST.replace(HANDLE, &HANDLE.replace("Hh8", "Hh9")),
        REQUEST.replace(HANDLE, &HANDLE.replace("otr1.", "otr2.")),
        // Non-canonical identifiers and offer diagnostics.
        REQUEST.replace(ATTEMPT, &ATTEMPT.to_uppercase()),
        REQUEST.replace("\"windows\"", "\"linux\""),
        // Over 2048 bytes.
        REQUEST.replace("0.1.0+abc123", &"x".repeat(2048)),
        "not json".to_owned(),
        String::new(),
    ];
    for body in malformed {
        let echo = if body.contains(ATTEMPT) && body.len() <= 2048 {
            MALFORMED.to_owned()
        } else {
            MALFORMED.replace(&format!("\"{ATTEMPT}\""), "null")
        };
        assert_eq!(
            post(app.clone(), "/v1/recover", &body).await,
            (StatusCode::BAD_REQUEST, line(&echo)),
            "{body}"
        );
    }
    assert_eq!(
        post(
            app.clone(),
            "/v1/recover",
            &REQUEST.replace("\"protocol_version\":3", "\"protocol_version\":4")
        )
        .await,
        (StatusCode::BAD_REQUEST, line(UNSUPPORTED))
    );
    assert_eq!(
        post(
            app.clone(),
            "/v1/recover",
            &REQUEST.replace("\"alpn\":\"oteryn-game/1\"", "\"alpn\":\"oteryn-game/2\"")
        )
        .await,
        (StatusCode::CONFLICT, line(OFFER))
    );
    for body in [
        REVOKE_REQUEST.replace(HANDLE, "otr1.short"),
        REVOKE_REQUEST.replace("}", ",\"attempt_ref\":null}"),
        "{}".to_owned(),
    ] {
        assert_eq!(
            post(app.clone(), "/v1/recover/revoke", &body).await,
            (StatusCode::BAD_REQUEST, line(REVOKE_MALFORMED)),
            "{body}"
        );
    }
    assert_eq!(upstream.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn recover_rejects_malformed_issuer_material() {
    let bad = [
        // A fresh-admission grant is never a recovery grant.
        SUCCESS.replace("oteryn-reauth-recovery-v1", "oteryn-pre-admission-v1"),
        SUCCESS.replace("\"protocol_version\":3", "\"protocol_version\":2"),
        SUCCESS.replace(ATTEMPT, "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7c"),
        SUCCESS.replace("\"grant\":", "\"recovery\":null,\"grant\":"),
        SUCCESS.replace("\"valid_for_seconds\":20", "\"valid_for_seconds\":31"),
        SUCCESS.replace("\"port\":7172", "\"port\":0"),
        "{}".to_owned(),
    ];
    for body in bad {
        let upstream = issuer(200, &body).await;
        assert_eq!(
            post(app(&upstream.url), "/v1/recover", REQUEST).await,
            (StatusCode::SERVICE_UNAVAILABLE, line(RECONCILIATION)),
            "{body}"
        );
    }
    // A response over the 6144-byte recovery limit is not read as success.
    let upstream = issuer(200, &format!("{SUCCESS}{}", " ".repeat(6144))).await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover", REQUEST).await,
        (StatusCode::SERVICE_UNAVAILABLE, line(RECONCILIATION))
    );
    // An issuer that answers nothing after the request was written is unknown,
    // not failure: same attempt_ref and handle only.
    let upstream = issuer_with(200, SUCCESS, None, true).await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover", REQUEST).await,
        (StatusCode::SERVICE_UNAVAILABLE, line(RECONCILIATION))
    );
    // Unknown codes, protocol 2 bodies, a wrong status or echo are unavailable.
    for (status, body) in [
        (
            409,
            UNAVAILABLE.replace("NATIVE_LOGIN_UNAVAILABLE", "NATIVE_RECOVERY_SOMETHING"),
        ),
        (
            503,
            UNAVAILABLE.replace("\"protocol_version\":3", "\"protocol_version\":2"),
        ),
        (409, UNAVAILABLE.to_owned()),
        (
            503,
            UNAVAILABLE.replace(ATTEMPT, "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7c"),
        ),
        // The login reconciliation code is not the recovery one.
        (
            503,
            RECONCILIATION.replace(
                "RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED",
                "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
            ),
        ),
    ] {
        let upstream = issuer(status, &body).await;
        assert_eq!(
            post(app(&upstream.url), "/v1/recover", REQUEST).await,
            (StatusCode::SERVICE_UNAVAILABLE, line(UNAVAILABLE)),
            "{body}"
        );
    }
    // A revoke success must be exactly the §19.7 body.
    let upstream = revoke_issuer(200, "{\"protocol_version\":3,\"revoked\":false}").await;
    assert_eq!(
        post(app(&upstream.url), "/v1/recover/revoke", REVOKE_REQUEST).await,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            line(&UNAVAILABLE.replace(&format!("\"{ATTEMPT}\""), "null"))
        )
    );
}

#[tokio::test]
async fn recover_limits_follow_section_19_9() {
    let upstream = any_issuer(503, UNAVAILABLE).await;
    let app = app(&upstream.url);
    // (attempt_ref, handle hash): 6 requests in total.
    for _ in 0..6 {
        let a = post_from(app.clone(), "/v1/recover", REQUEST, [192, 0, 2, 10]).await;
        assert_eq!(a.status, StatusCode::SERVICE_UNAVAILABLE);
    }
    let a = post_from(app.clone(), "/v1/recover", REQUEST, [192, 0, 2, 11]).await;
    assert_eq!(
        (a.status, a.body),
        (StatusCode::TOO_MANY_REQUESTS, line(RATE_LIMITED))
    );
    assert!(
        a.retry_after
            .is_some_and(|s| (1..=30).contains(&s.parse::<u64>().unwrap()))
    );
    assert_eq!(upstream.calls.load(Ordering::SeqCst), 6);
    // Another handle, or another attempt, is a separate key.
    let other = REQUEST.replace("Hh8", "Hh4");
    let a = post_from(app.clone(), "/v1/recover", &other, [192, 0, 2, 11]).await;
    assert_eq!(a.status, StatusCode::SERVICE_UNAVAILABLE);

    // Source IP: 30 per minute on each route, counted separately.
    let source = [192, 0, 2, 20];
    for index in 0..30 {
        let body = REQUEST.replace(
            ATTEMPT,
            &format!("0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6{index:03x}"),
        );
        let a = post_from(app.clone(), "/v1/recover", &body, source).await;
        assert_eq!(a.status, StatusCode::SERVICE_UNAVAILABLE, "{index}");
    }
    let body = REQUEST.replace(ATTEMPT, "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6fff");
    let a = post_from(app.clone(), "/v1/recover", &body, source).await;
    assert_eq!(
        (a.status, a.body),
        (
            StatusCode::TOO_MANY_REQUESTS,
            line(&RATE_LIMITED.replace(ATTEMPT, "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6fff"))
        )
    );
    // The login and revoke budgets of that source are untouched.
    let a = post_from(app.clone(), "/v1/recover/revoke", "{}", source).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    for _ in 0..29 {
        post_from(app.clone(), "/v1/recover/revoke", "{}", source).await;
    }
    let a = post_from(app.clone(), "/v1/recover/revoke", "{}", source).await;
    assert_eq!(
        (a.status, a.body),
        (
            StatusCode::TOO_MANY_REQUESTS,
            line(&RATE_LIMITED.replace(&format!("\"{ATTEMPT}\""), "null"))
        )
    );
    let a = post_from(app, "/v1/login", "{\"protocol_version\":3}", source).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST, "{}", a.body);
}

#[test]
fn recover_wire_redacts_the_handle() {
    let request = recover::parse_request(REQUEST.as_bytes()).unwrap();
    let debug = format!("{request:?}");
    assert!(debug.contains(ATTEMPT));
    assert!(!debug.contains(HANDLE) && !debug.contains("AAECAwQF"));
    assert_eq!(
        recover::encode_request(&request).unwrap(),
        REQUEST.as_bytes()
    );
    assert_eq!(
        recover::parse_revoke(REVOKE_REQUEST.as_bytes())
            .map(|r| format!("{r:?}"))
            .unwrap(),
        "RevokeRequest{redacted}"
    );
}
