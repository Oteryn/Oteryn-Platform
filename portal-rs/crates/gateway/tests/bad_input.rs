//! Refusals at the public login router, without an admission issuer.
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_gateway::{Options, issuer::IssuerClient, router_with};
use std::time::Duration;
use tower::ServiceExt;

const LOGIN: &str = include_str!("fixtures/game_client_native_login/request.json");
const VECTORS: &str = include_str!("fixtures/go_vectors.jsonl");
const INVALID_REQUEST: &[u8] = b"{\"error\":\"invalid_request\"}\n";

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn assert_refusal(case: &str, input: Vec<u8>, expected: &[u8]) -> TestResult {
    let issuer = IssuerClient::new(
        "http://127.0.0.1:1",
        "synthetic-test-service-token".into(),
        Duration::from_secs(1),
        Duration::from_secs(1),
    )?;
    let app = router_with(
        issuer,
        Options {
            enabled: true,
            protocol3: true,
            version: "test".into(),
        },
    );
    let response = app
        .oneshot(
            Request::post("/v1/login")
                .header("content-type", "application/json")
                .body(Body::from(input))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{case}");
    assert_eq!(
        response.headers()["content-type"],
        "application/json",
        "{case}"
    );
    assert_eq!(
        response.headers()["cache-control"],
        "no-store, no-cache, must-revalidate, private",
        "{case}"
    );
    assert_eq!(
        to_bytes(response.into_body(), 512).await?.as_ref(),
        expected,
        "{case}"
    );
    Ok(())
}

#[tokio::test]
async fn login_refuses_unclassifiable_bodies() -> TestResult {
    for (case, body) in [
        ("malformed JSON", b"{]".to_vec()),
        ("truncated JSON", b"{\"protocol_version\":2,".to_vec()),
        (
            "invalid UTF-8",
            b"{\"protocol_version\":2,\"game_login_ticket\":\"\xff\"}".to_vec(),
        ),
    ] {
        assert_refusal(case, body, INVALID_REQUEST).await?;
    }
    Ok(())
}

#[tokio::test]
async fn recognizable_protocol2_requests_use_go_refusals() -> TestResult {
    let mut malformed = None;
    let mut malformed_with_echo = None;
    let mut unsupported = None;
    for line in VECTORS.lines().filter(|line| !line.is_empty()) {
        let vector: serde_json::Value = serde_json::from_str(line)?;
        match (vector["kind"].as_str(), vector["code"].as_str()) {
            (Some("request"), Some("NATIVE_LOGIN_UNSUPPORTED_VERSION")) => {
                unsupported = Some((
                    vector["input"]
                        .as_str()
                        .ok_or("missing Go vector input")?
                        .as_bytes()
                        .to_vec(),
                    vector["output"]
                        .as_str()
                        .ok_or("missing Go vector output")?
                        .to_owned(),
                ));
            }
            (Some("request"), Some("NATIVE_LOGIN_REQUEST_MALFORMED"))
                if vector["output"]
                    .as_str()
                    .is_some_and(|output| output.ends_with("\"attempt_ref\":null}\n")) =>
            {
                malformed = vector["output"].as_str().map(str::to_owned);
            }
            (Some("request"), Some("NATIVE_LOGIN_REQUEST_MALFORMED"))
                if malformed_with_echo.is_none() =>
            {
                malformed_with_echo = Some((
                    vector["input"]
                        .as_str()
                        .ok_or("missing Go vector input")?
                        .as_bytes()
                        .to_vec(),
                    vector["output"]
                        .as_str()
                        .ok_or("missing Go vector output")?
                        .to_owned(),
                ));
            }
            _ => {}
        }
    }

    let malformed = malformed.ok_or("missing malformed Go refusal")?;
    let mut oversized = LOGIN.as_bytes().to_vec();
    oversized.resize(2049, b' ');
    assert_refusal(
        "over the 2048-byte native request limit",
        oversized,
        malformed.as_bytes(),
    )
    .await?;

    // The request stays recognizable below the outer cap, even though it
    // exceeds the native protocol limit.
    let mut outer_limit = LOGIN.as_bytes().to_vec();
    outer_limit.resize(16_384, b' ');
    assert_refusal(
        "at the 16 KiB outer body cap",
        outer_limit.clone(),
        malformed.as_bytes(),
    )
    .await?;
    outer_limit.push(b' ');
    assert_refusal(
        "over the 16 KiB outer body cap",
        outer_limit,
        INVALID_REQUEST,
    )
    .await?;

    let (input, expected) = malformed_with_echo.ok_or("missing malformed Go refusal with echo")?;
    assert_refusal("malformed native request", input, expected.as_bytes()).await?;

    let (input, expected) = unsupported.ok_or("missing unsupported Go refusal")?;
    let input = String::from_utf8(input)?;
    let future = input.replace("\"protocol_version\":3", "\"protocol_version\":4");
    assert_ne!(future, input);
    assert_refusal(
        "unsupported protocol",
        future.into_bytes(),
        expected.as_bytes(),
    )
    .await
}
