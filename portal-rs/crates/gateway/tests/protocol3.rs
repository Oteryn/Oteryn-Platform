//! Gateway login contract §19.1-§19.2: protocol 3 on `POST /v1/login`, byte
//! for byte against `docs/contracts/fixtures/native-gateway-protocol3-v1/`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::any,
};
use oteryn_gateway::{Options, issuer::IssuerClient, router_with, v3};
use std::time::Duration;
use tokio::task::JoinHandle;
use tower::ServiceExt;

const REQUEST: &str = include_str!(
    "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-request.json"
);
const SUCCESS: &str = include_str!(
    "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-success.json"
);
const CHANNEL_REFUSAL: &str = include_str!(
    "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-channel-refusal.json"
);
const ROUTE_UNAVAILABLE: &str = include_str!(
    "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-route-unavailable.json"
);
const UNSUPPORTED: &str = include_str!(
    "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-unsupported-version.json"
);
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";

struct Server {
    url: String,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Clone)]
struct Reply {
    status: u16,
    body: String,
}
async fn reply(State(s): State<Reply>, r: Request<Body>) -> impl IntoResponse {
    assert_eq!(r.uri().path(), oteryn_gateway::issuer::ISSUER_PATH);
    // The private request is the §3.1 member set with 3, in contract order.
    assert_eq!(
        to_bytes(r.into_body(), 2048).await.unwrap().as_ref(),
        REQUEST.as_bytes()
    );
    (StatusCode::from_u16(s.status).unwrap(), s.body)
}
async fn issuer(status: u16, body: &str) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route(oteryn_gateway::issuer::ISSUER_PATH, any(reply))
        .with_state(Reply {
            status,
            body: body.to_owned(),
        });
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Server { url, task }
}
fn client(url: &str) -> IssuerClient {
    IssuerClient::new(
        url,
        "synthetic-gateway-service-token".into(),
        Duration::from_millis(500),
        Duration::from_millis(500),
    )
    .unwrap()
}
async fn login(app: Router, body: &str) -> (StatusCode, String) {
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/login")
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    assert_eq!(
        r.headers()["cache-control"],
        "no-store, no-cache, must-revalidate, private"
    );
    let raw = to_bytes(r.into_body(), 16384).await.unwrap();
    (status, String::from_utf8(raw.to_vec()).unwrap())
}
fn options(enabled: bool, protocol3: bool) -> Options {
    Options {
        enabled,
        protocol3,
        version: "test".into(),
    }
}

#[tokio::test]
async fn protocol3_switch_off_answers_unsupported_version_in_protocol2() {
    let app = router_with(client("http://127.0.0.1:1"), options(true, false));
    assert_eq!(
        login(app, REQUEST).await,
        (StatusCode::BAD_REQUEST, format!("{UNSUPPORTED}\n"))
    );
    // Native login off keeps the Go gateway's generic refusal for every body,
    // and protocol 3 needs native login on.
    for protocol3 in [false, true] {
        let app = router_with(client("http://127.0.0.1:1"), options(false, protocol3));
        assert_eq!(
            login(app, REQUEST).await,
            (
                StatusCode::BAD_REQUEST,
                "{\"error\":\"invalid_request\"}\n".to_owned()
            )
        );
    }
}

#[tokio::test]
async fn protocol3_success_and_refusals_relay_byte_exact() {
    let upstream = issuer(200, SUCCESS).await;
    let app = router_with(client(&upstream.url), options(true, true));
    assert_eq!(
        login(app, REQUEST).await,
        (StatusCode::OK, format!("{SUCCESS}\n"))
    );
    for (status, body) in [(409, CHANNEL_REFUSAL), (503, ROUTE_UNAVAILABLE)] {
        let upstream = issuer(status, body).await;
        let app = router_with(client(&upstream.url), options(true, true));
        assert_eq!(
            login(app, REQUEST).await,
            (StatusCode::from_u16(status).unwrap(), format!("{body}\n"))
        );
    }
}

#[tokio::test]
async fn protocol3_rejects_malformed_issuer_material() {
    let reconciliation = format!(
        "{{\"protocol_version\":3,\"error\":{{\"code\":\"ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED\",\"public_class\":\"TEMPORARILY_UNAVAILABLE\"}},\"attempt_ref\":\"{ATTEMPT}\"}}\n"
    );
    let bad = [
        // A protocol 2 shaped success never satisfies protocol 3.
        SUCCESS.replace("\"protocol_version\":3", "\"protocol_version\":2"),
        // Channels out of canonical order.
        SUCCESS.replace(
            "{\"channel_id\":\"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71\",\"key\":\"antica\",\"label\":\"Antica\"},",
            "",
        )
        .replace(
            "\"label\":\"Pythera\"}]",
            "\"label\":\"Pythera\"},{\"channel_id\":\"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71\",\"key\":\"antica\",\"label\":\"Antica\"}]",
        ),
        // A label not derived from the key.
        SUCCESS.replace("\"label\":\"Obscura\"", "\"label\":\"obscura\""),
        // Selection that disagrees with the request.
        SUCCESS.replace("\"automatic\"", "\"requested\""),
        // A malformed handle.
        SUCCESS.replace("otr1.AAEC", "otr2.AAEC"),
        // A handle beyond the 12 h bound.
        SUCCESS.replace("43200", "43201"),
        // An extra member.
        SUCCESS.replace("\"recovery\":", "\"extra\":1,\"recovery\":"),
    ];
    for body in bad {
        let upstream = issuer(200, &body).await;
        let app = router_with(client(&upstream.url), options(true, true));
        assert_eq!(
            login(app, REQUEST).await,
            (StatusCode::SERVICE_UNAVAILABLE, reconciliation.clone()),
            "{body}"
        );
    }
    // `"recovery": null` is a valid success.
    let none = SUCCESS.replace(
        "{\"handle\":\"otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8\",\"expires_in\":43200}",
        "null",
    );
    let upstream = issuer(200, &none).await;
    let app = router_with(client(&upstream.url), options(true, true));
    assert_eq!(
        login(app, REQUEST).await,
        (StatusCode::OK, format!("{none}\n"))
    );
    // A channel refusal without channels, or a protocol 2 error body, is not
    // a protocol 3 refusal.
    let unavailable = format!(
        "{{\"protocol_version\":3,\"error\":{{\"code\":\"NATIVE_LOGIN_UNAVAILABLE\",\"public_class\":\"TEMPORARILY_UNAVAILABLE\"}},\"attempt_ref\":\"{ATTEMPT}\"}}\n"
    );
    for (status, body) in [
        (
            409,
            CHANNEL_REFUSAL.replace(
                ",\"channels\":[{\"channel_id\":\"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a72\",\"key\":\"obscura\",\"label\":\"Obscura\"}]",
                "",
            ),
        ),
        (
            503,
            ROUTE_UNAVAILABLE.replace("\"protocol_version\":3", "\"protocol_version\":2"),
        ),
    ] {
        let upstream = issuer(status, &body).await;
        let app = router_with(client(&upstream.url), options(true, true));
        assert_eq!(
            login(app, REQUEST).await,
            (StatusCode::SERVICE_UNAVAILABLE, unavailable.clone()),
            "{body}"
        );
    }
}

#[test]
fn protocol3_wire_rules() {
    assert!(v3::is_v3(REQUEST.as_bytes()));
    assert!(!oteryn_gateway::wire::is_native(REQUEST.as_bytes()));
    let request = v3::parse_request(REQUEST.as_bytes()).unwrap();
    assert_eq!(request.protocol_version, 3);
    // Protocol 2 parsing is unchanged for a protocol 3 body.
    assert_eq!(
        oteryn_gateway::wire::parse_request(REQUEST.as_bytes())
            .unwrap_err()
            .name(),
        "NATIVE_LOGIN_UNSUPPORTED_VERSION"
    );
    assert_eq!(v3::label("antica"), "Antica");
    assert_eq!(v3::label("1st-channel"), "1St-channel");
    assert!(v3::handle_valid(
        "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"
    ));
    assert!(!v3::handle_valid(
        "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh9"
    ));
}
