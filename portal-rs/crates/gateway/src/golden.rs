#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use serde_json::Value;
use std::collections::HashMap;

fn vectors() -> Vec<Value> {
    include_str!("../tests/fixtures/go_vectors.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap()
}
fn code(name: &str) -> Code {
    Code::ALL.into_iter().find(|c| c.name() == name).unwrap()
}
fn encoded(v: &impl Serialize) -> String {
    let mut b = wire::encode(v).unwrap();
    b.push(b'\n');
    String::from_utf8(b).unwrap()
}
#[test]
fn all_go_wire_and_error_vectors_are_byte_identical() {
    let mut count = 0;
    for v in vectors() {
        let kind = str_field(&v, "kind");
        if matches!(kind, "handler" | "limiter") {
            continue;
        }
        let input = str_field(&v, "input").as_bytes();
        match kind {
            "request" => match wire::parse_request(input) {
                Ok(r) => {
                    assert!(v.get("code").is_none(), "{v}");
                    assert_eq!(
                        wire::encode_request(&r).unwrap(),
                        str_field(&v, "output").as_bytes()
                    );
                    assert!(!format!("{r:?}").contains(&r.game_login_ticket));
                }
                Err(c) => {
                    assert_eq!(c.name(), str_field(&v, "code"), "{v}");
                    assert_eq!(u64::from(c.status()), v["status"].as_u64().unwrap());
                    assert_eq!(
                        encoded(&c.body(wire::attempt_echo(input).as_deref())),
                        str_field(&v, "output")
                    );
                }
            },
            "native" => assert_eq!(
                wire::is_native(input),
                v["result"].as_bool().unwrap(),
                "{v}"
            ),
            "echo" => assert_eq!(
                wire::attempt_echo(input).as_deref(),
                v["result"].as_str(),
                "{v}"
            ),
            "success" => {
                let r = serde_json::from_value(v["request"].clone()).unwrap();
                let result = wire::parse_success(input, &r);
                assert_eq!(result.is_some(), v["valid"].as_bool().unwrap(), "{v}");
                if let Some(s) = result {
                    assert_eq!(encoded(&s), str_field(&v, "output"));
                    assert!(!format!("{s:?} {:?}", s.grant).contains(&s.grant.token));
                }
            }
            "error" => {
                let r: wire::Request = serde_json::from_value(v["request"].clone()).unwrap();
                let result = errors::parse_issuer_error(
                    v["status"].as_u64().unwrap() as u16,
                    input,
                    &r.attempt_ref,
                );
                assert_eq!(result.is_some(), v["valid"].as_bool().unwrap(), "{v}");
                if let Some(rf) = result {
                    assert_eq!(rf.code.name(), str_field(&v, "code"));
                    assert_eq!(
                        encoded(&rf.code.body(Some(&r.attempt_ref))),
                        str_field(&v, "output")
                    );
                }
            }
            _ => panic!("unexpected kind {kind}"),
        }
        count += 1;
    }
    println!("{count} Go wire/error vectors byte-identical");
}
#[test]
fn all_go_limiter_vectors_match_in_order() {
    let mut instances = HashMap::new();
    let mut count = 0;
    for v in vectors().into_iter().filter(|v| v["kind"] == "limiter") {
        let l = instances
            .entry(str_field(&v, "instance").to_owned())
            .or_insert_with(|| {
                WindowLimiter::new(
                    v["limit"].as_u64().unwrap() as usize,
                    Duration::from_nanos(v["window"].as_u64().unwrap()),
                    v["max_keys"].as_u64().unwrap() as usize,
                )
            });
        let key = str_field(&v, "key");
        let bytes: Vec<_> = (0..key.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&key[i..i + 2], 16).unwrap())
            .collect();
        let result = l.allow(&bytes, Duration::from_nanos(v["now"].as_u64().unwrap()));
        assert_eq!(result.is_ok(), v["allowed"].as_bool().unwrap(), "{v}");
        assert_eq!(
            result.err().unwrap_or(0),
            v["retry"].as_u64().unwrap(),
            "{v}"
        );
        count += 1;
    }
    println!("{count} Go limiter operations identical");
}
#[tokio::test]
async fn all_go_handler_vectors_are_byte_identical() {
    let mut instances = HashMap::new();
    let mut count = 0;
    for v in vectors().into_iter().filter(|v| v["kind"] == "handler") {
        let (state, calls) = instances
            .entry(str_field(&v, "instance").to_owned())
            .or_insert_with(|| {
                (
                    StateData {
                        issuer: IssuerClient::new(
                            "http://127.0.0.1:1",
                            "synthetic".into(),
                            Duration::from_secs(1),
                            Duration::from_secs(1),
                        )
                        .unwrap(),
                        enabled: true,
                        version: "test".into(),
                        start: Instant::now(),
                        sources: Mutex::new(WindowLimiter::new(
                            30,
                            Duration::from_secs(60),
                            100_000,
                        )),
                        attempts: Mutex::new(WindowLimiter::new(
                            6,
                            Duration::from_secs(120),
                            100_000,
                        )),
                    },
                    0u64,
                )
            });
        let input = str_field(&v, "input").as_bytes();
        let echo = wire::attempt_echo(input);
        let remote = str_field(&v, "remote");
        let source = remote
            .parse::<SocketAddr>()
            .map(|a| a.ip().to_string())
            .unwrap_or(remote.to_owned());
        let response = match prepare(
            state,
            &source,
            input,
            Duration::from_nanos(v["now"].as_u64().unwrap()),
        ) {
            Err(r) => refuse(r, echo.as_deref()),
            Ok(_) => {
                *calls += 1;
                if v["refusal"].is_null() {
                    json(
                        200,
                        &serde_json::from_value::<wire::Success>(v["success"].clone()).unwrap(),
                    )
                } else {
                    refuse(
                        Refusal {
                            code: code(str_field(&v["refusal"], "Code")),
                            retry_after: v["refusal"]["RetryAfterSeconds"].as_u64().unwrap(),
                        },
                        echo.as_deref(),
                    )
                }
            }
        };
        assert_eq!(*calls, v["calls"].as_u64().unwrap(), "{v}");
        assert_eq!(
            u64::from(response.status().as_u16()),
            v["status"].as_u64().unwrap(),
            "{v}"
        );
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            // Client honours at most 30s; T52b intentionally tightens the Go vector.
            str_field(&v, "retry")
                .parse::<u64>()
                .map(|retry| retry.clamp(1, 30).to_string())
                .unwrap_or_default()
        );
        assert_eq!(
            to_bytes(response.into_body(), 8192).await.unwrap().as_ref(),
            str_field(&v, "output").as_bytes(),
            "{v}"
        );
        count += 1;
    }
    println!("{count} Go handler responses byte-identical");
}
#[test]
fn public_error_table_and_retry_bounds_match_go() {
    let expected = [
        (400, "RETRY_LOGIN"),
        (400, "CLIENT_UPDATE_REQUIRED"),
        (409, "CLIENT_UPDATE_REQUIRED"),
        (401, "AUTHENTICATION_REQUIRED"),
        (409, "RETRY_LOGIN"),
        (409, "SESSION_UNAVAILABLE"),
        (503, "TEMPORARILY_UNAVAILABLE"),
        (429, "TEMPORARILY_UNAVAILABLE"),
        (503, "TEMPORARILY_UNAVAILABLE"),
        (409, "RETRY_LOGIN"),
        (503, "TEMPORARILY_UNAVAILABLE"),
    ];
    for (c, (status, class)) in Code::ALL.into_iter().zip(expected) {
        assert_eq!((c.status(), c.class()), (status, class));
    }
    for (input, expected) in [
        ("", 60),
        ("0", 60),
        ("-1", 60),
        ("3601", 60),
        ("garbage", 60),
        (" 17", 60),
        ("+17", 17),
        ("17", 17),
        ("3600", 3600),
    ] {
        assert_eq!(errors::retry_after(input), expected);
    }
}
