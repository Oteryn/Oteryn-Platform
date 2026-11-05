//! Independent client golden oracle; provenance and constants are in fixtures/SOURCE.md.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use oteryn_gateway::{
    issuer::{ISSUER_PATH, IssuerClient},
    router,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;
const REQUEST: &str = include_str!("fixtures/game_client_native_login/request.json");
const SORTED: &str = include_str!("fixtures/game_client_native_login/request_sorted.json");
const SUCCESS: &str = include_str!("fixtures/game_client_native_login/success.json");
const ERRORS: &str = include_str!("fixtures/game_client_native_login/errors.json");
const ATTEMPT: &str = "01890a5d-ac96-774b-bcce-b302099a8057";
#[derive(Clone, Deserialize)]
struct Reply {
    status: u16,
    body: String,
    #[serde(default)]
    retry_after: Option<String>,
}
fn replies(s: &str) -> Vec<Reply> {
    serde_json::from_str(s).unwrap()
}
fn value(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}
// These fixture-scoped strict types reject unknown, missing and duplicate success fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Success {
    protocol_version: u8,
    attempt_ref: String,
    world_id: String,
    channel_id: String,
    endpoint: Endpoint,
    grant: Grant,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    host: String,
    port: u16,
    tls_server_name: String,
    alpn: String,
    protocol_major: u16,
    transport_profile: u16,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    profile: String,
    token: String,
    valid_for_seconds: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientError {
    protocol_version: u8,
    error: ClientErrorDetail,
    attempt_ref: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientErrorDetail {
    code: String,
    public_class: String,
}
fn client_accepts(status: u16, body: &str) -> bool {
    if body.len() > 6144 {
        return false;
    }
    if status != 200 {
        let Ok(e) = serde_json::from_str::<ClientError>(body) else {
            return false;
        };
        return e.protocol_version == 2
            && (e.attempt_ref.is_null() || e.attempt_ref == ATTEMPT)
            && replies(ERRORS).iter().any(|r| {
                let v = value(&r.body);
                r.status == status
                    && v["error"]["code"] == e.error.code
                    && v["error"]["public_class"] == e.error.public_class
            });
    }
    let Ok(s) = serde_json::from_str::<Success>(body) else {
        return false;
    };
    let e = s.endpoint;
    let g = s.grant;
    let dns = |s: &str| {
        !s.is_empty()
            && s.len() <= 253
            && s.split('.').all(|l| {
                !l.is_empty()
                    && l.len() <= 63
                    && l.starts_with(|c: char| c.is_ascii_alphanumeric())
                    && l.ends_with(|c: char| c.is_ascii_alphanumeric())
                    && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
    };
    s.protocol_version == 2
        && s.attempt_ref == ATTEMPT
        && s.world_id == "01890a5d-ac96-774b-bcce-b302099a8059"
        && s.channel_id == "01890a5d-ac96-774b-bcce-b302099a805a"
        && dns(&e.host)
        && dns(&e.tls_server_name)
        && e.port > 0
        && e.alpn == "oteryn-game/1"
        && e.protocol_major == 1
        && e.transport_profile == 1
        && g.profile == "oteryn-pre-admission-v1"
        && (1..=30).contains(&g.valid_for_seconds)
        && g.token.len() <= 4096
        && g.token.split('.').count() == 3
        && g.token.split('.').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}
struct Harness {
    app: Router,
    seen: Arc<Mutex<Vec<Vec<u8>>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Harness {
    async fn new(responses: Vec<Reply>) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::from(responses)));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let captured = seen.clone();
        let upstream = Router::new().route(
            ISSUER_PATH,
            post(move |req: Request| {
                let queue = queue.clone();
                let captured = captured.clone();
                async move {
                    let body = to_bytes(req.into_body(), 2048).await.unwrap().to_vec();
                    captured.lock().unwrap().push(body);
                    let r = queue
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected upstream call");
                    let mut response =
                        (StatusCode::from_u16(r.status).unwrap(), r.body).into_response();
                    if let Some(retry) = r.retry_after {
                        response
                            .headers_mut()
                            .insert("retry-after", retry.parse().unwrap());
                    }
                    response
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let issuer = IssuerClient::new(
            &base,
            "synthetic".into(),
            Duration::from_secs(2),
            Duration::from_secs(2),
        )
        .unwrap();
        Self {
            app: router(issuer, true, "fixture".into()),
            seen,
            task,
        }
    }
    async fn send(&self, body: &str) -> (u16, String) {
        let response = self
            .app
            .clone()
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
        let status = response.status().as_u16();
        if status == 429 {
            let retry: u64 = response.headers()["retry-after"]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!(retry <= 30, "Retry-After {retry}");
        }
        let body = String::from_utf8(to_bytes(response.into_body(), 6144).await.unwrap().to_vec())
            .unwrap();
        assert!(
            client_accepts(status, &body),
            "client rejected {status}: {body}"
        );
        assert!(!body.contains("NATIVE_LOGIN_TICKET_REJECTED"));
        assert!(!body.contains("NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED"));
        (status, body)
    }
}
fn success() -> Reply {
    Reply {
        status: 200,
        body: SUCCESS.into(),
        retry_after: None,
    }
}
#[test]
fn oracle_accepts_goldens_and_rejects_client_negative_vectors() {
    assert!(client_accepts(200, SUCCESS));
    for r in replies(ERRORS) {
        assert!(client_accepts(r.status, &r.body));
    }
    let bad: Vec<String> = serde_json::from_str(include_str!(
        "fixtures/game_client_native_login/rejected_successes.json"
    ))
    .unwrap();
    assert_eq!(bad.len(), 11);
    for b in bad {
        assert!(!client_accepts(200, &b));
    }
    for r in replies(include_str!(
        "fixtures/game_client_native_login/rejected_errors.json"
    )) {
        assert!(!client_accepts(r.status, &r.body));
    }
}
#[tokio::test]
async fn requests_and_identical_retries_produce_exact_success() {
    for request in [
        REQUEST.to_owned(),
        SORTED.to_owned(),
        REQUEST.replace(
            "\"channel_id\":null",
            "\"channel_id\":\"01890a5d-ac96-774b-bcce-b302099a805a\"",
        ),
    ] {
        let h = Harness::new(vec![success(); 2]).await;
        let first = h.send(&request).await;
        assert_eq!(first.0, 200);
        assert_eq!(first.1, h.send(&request).await.1);
        assert_eq!(value(&first.1), value(SUCCESS));
        let seen = h.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        for body in seen.iter() {
            assert_eq!(
                serde_json::from_slice::<Value>(body).unwrap(),
                value(&request)
            );
        }
    }
}
#[tokio::test]
async fn every_error_and_security_normalization_are_client_compatible() {
    for r in replies(ERRORS) {
        let mut expected = value(&r.body);
        expected["attempt_ref"] = ATTEMPT.into();
        let h = Harness::new(vec![r.clone(); 2]).await;
        let first = h.send(REQUEST).await;
        assert_eq!(first.0, r.status);
        assert_eq!(value(&first.1), expected);
        assert_eq!(first.1, h.send(REQUEST).await.1);
    }
    for raw in [
        "NATIVE_LOGIN_TICKET_REJECTED",
        "NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED",
    ] {
        let body = replies(ERRORS)[7]
            .body
            .replace("NATIVE_LOGIN_AUTHENTICATION_REQUIRED", raw);
        let h = Harness::new(vec![Reply {
            status: 401,
            body,
            retry_after: None,
        }])
        .await;
        assert_eq!(h.send(REQUEST).await.0, 401);
    }
}
#[tokio::test]
async fn rejected_upstream_variants_never_escape_gateway() {
    let bad: Vec<String> = serde_json::from_str(include_str!(
        "fixtures/game_client_native_login/rejected_successes.json"
    ))
    .unwrap();
    let mut responses: Vec<Reply> = bad
        .into_iter()
        .map(|body| Reply {
            status: 200,
            body,
            retry_after: None,
        })
        .collect();
    responses.extend(replies(include_str!(
        "fixtures/game_client_native_login/rejected_errors.json"
    )));
    responses.push(Reply {
        status: 200,
        body: format!("{SUCCESS}{}", " ".repeat(6144)),
        retry_after: None,
    });
    for r in responses {
        let raw = r.body.contains("NATIVE_LOGIN_TICKET_REJECTED");
        let h = Harness::new(vec![r]).await;
        assert_eq!(h.send(REQUEST).await.0, if raw { 401 } else { 503 });
    }
}
#[tokio::test]
async fn request_boundary_and_sequences_server_side() {
    let mut boundary = REQUEST.to_owned();
    boundary.push_str(&" ".repeat(2048 - REQUEST.len()));
    let h = Harness::new(vec![success()]).await;
    assert_eq!(h.send(&boundary).await.0, 200);
    boundary.push(' ');
    assert_eq!(h.send(&boundary).await.0, 400);
    assert_eq!(h.seen.lock().unwrap().len(), 1);
    let sequences: Vec<Vec<Reply>> = serde_json::from_str(include_str!(
        "fixtures/game_client_native_login/sequences.json"
    ))
    .unwrap();
    for (i, sequence) in sequences.into_iter().enumerate() {
        let count = [3, 6, 1][i];
        let h = Harness::new(sequence.clone()).await;
        let mut previous = None;
        for r in sequence.iter().take(count) {
            let result = h.send(REQUEST).await;
            assert_eq!(result.0, r.status);
            assert_eq!(value(&result.1)["error"], value(&r.body)["error"]);
            if i == 1 {
                if let Some(body) = previous {
                    assert_eq!(result.1, body);
                }
                previous = Some(result.1);
            }
        }
        let seen = h.seen.lock().unwrap();
        assert_eq!(seen.len(), count);
        assert!(seen.windows(2).all(|w| w[0] == w[1]));
    }
}
#[tokio::test]
async fn retry_after_is_bounded_for_upstream_and_local_limits() {
    for retry in [
        None,
        Some("0"),
        Some("31"),
        Some("3600"),
        Some("garbage"),
        Some("17"),
    ] {
        let mut r = replies(ERRORS)[14].clone();
        r.retry_after = retry.map(str::to_owned);
        let h = Harness::new(vec![r]).await;
        assert_eq!(h.send(REQUEST).await.0, 429);
    }
    let h = Harness::new(vec![success(); 6]).await;
    for _ in 0..6 {
        assert_eq!(h.send(REQUEST).await.0, 200);
    }
    assert_eq!(h.send(REQUEST).await.0, 429);
    assert_eq!(h.seen.lock().unwrap().len(), 6);
}

#[tokio::test]
async fn local_validation_and_nullable_attempt_are_client_compatible() {
    let malformed = "NATIVE_LOGIN_REQUEST_MALFORMED";
    for (request, code) in [
        (REQUEST.replace(ATTEMPT, "invalid"), malformed),
        (REQUEST.replace("dev-1", &"a".repeat(65)), malformed),
        (REQUEST.replace("dev-1", "bad build"), malformed),
        (
            REQUEST.replace("oteryn-game/1", "h2"),
            "NATIVE_LOGIN_OFFER_UNSUPPORTED",
        ),
    ] {
        let h = Harness::new(vec![]).await;
        let result = h.send(&request).await;
        let body = value(&result.1);
        assert_eq!(body["error"]["code"], code);
        if request.contains("invalid") {
            assert!(body["attempt_ref"].is_null());
        }
        assert!(h.seen.lock().unwrap().is_empty());
    }
    let request = REQUEST.replace("dev-1", &"a".repeat(64));
    let h = Harness::new(vec![success()]).await;
    assert_eq!(h.send(&request).await.0, 200);
    // Handler echoes valid request attempts even when upstream sends null.
    for code in oteryn_gateway::errors::Code::ALL {
        for attempt in [Some(ATTEMPT), None] {
            let body = serde_json::to_string(&code.body(attempt)).unwrap();
            assert!(client_accepts(code.status(), &body));
        }
    }
}
