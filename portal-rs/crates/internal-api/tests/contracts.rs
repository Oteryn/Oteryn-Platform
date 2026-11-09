use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Extension,
    http::{Request, StatusCode},
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Shape, Value},
    fixtures,
    guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use sha2::{Digest, Sha256};
use std::{error::Error, path::Path, sync::Arc};
use tower::ServiceExt;

static HEAD: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("operation", Shape::Scalar),
]);
fn san(value: &str) -> Result<Identity, Box<dyn Error>> {
    Ok(Identity::san(value)?)
}
fn bindings() -> Result<IdentityMap, Box<dyn Error>> {
    Ok(IdentityMap::new(vec![
        Binding {
            identity: san("store.test.invalid")?,
            group: "store".into(),
        },
        Binding {
            identity: Identity::spki_sha256([42; 32]),
            group: "premium".into(),
        },
    ])?)
}
fn policy() -> RoutePolicy {
    RoutePolicy {
        enabled: true,
        group: "store".into(),
        max_body_bytes: 64,
        max_json_depth: 4,
        json: JsonRule::Canonical(&HEAD),
    }
}
fn app(policy: RoutePolicy, map: IdentityMap) -> Router {
    guard::protect(
        Router::new().route(
            "/internal/v1/test",
            post(
                |Extension(value): Extension<Value>, bytes: axum::body::Bytes| async move {
                    assert!(matches!(value, Value::Object(_)));
                    bytes
                },
            ),
        ),
        Arc::new(map),
        policy,
    )
}
fn request(body: &str, identity: Option<Identity>) -> Result<Request<Body>, Box<dyn Error>> {
    let mut request = Request::post("/internal/v1/test")
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))?;
    if let Some(identity) = identity {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(identity));
    }
    Ok(request)
}
const BODY: &str = r#"{"contract_version":1,"operation":"read"}"#;
async fn assert_failure(
    app: Router,
    request: Request<Body>,
    status: StatusCode,
) -> Result<(), Box<dyn Error>> {
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), status);
    // Axum may synthesize Content-Length: 0; the guard adds no headers.
    for (name, value) in response.headers() {
        assert_eq!(name, "content-length");
        assert_eq!(value, "0");
    }
    assert!(to_bytes(response.into_body(), 1024).await?.is_empty());
    Ok(())
}
#[tokio::test]
async fn permits_san_and_spki_and_preserves_body() -> Result<(), Box<dyn Error>> {
    for (group, identity) in [
        ("store", san("store.test.invalid")?),
        ("premium", Identity::spki_sha256([42; 32])),
    ] {
        let mut p = policy();
        p.group = group.into();
        let response = app(p, bindings()?)
            .oneshot(request(BODY, Some(identity))?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(to_bytes(response.into_body(), 64).await?, BODY);
    }
    Ok(())
}
#[tokio::test]
async fn denies_unknown_missing_wrong_group_and_spoofed_headers() -> Result<(), Box<dyn Error>> {
    for identity in [
        None,
        Some(san("unknown.test.invalid")?),
        Some(san("store.test.invaliD")?),
        Some(san("store.test.invalid ")?),
        Some(Identity::spki_sha256([42; 32])),
    ] {
        let mut req = request(BODY, identity)?;
        req.headers_mut()
            .insert("ssl-client-verify", "SUCCESS".parse()?);
        req.headers_mut()
            .insert("ssl-client-san", "store.test.invalid".parse()?);
        assert_failure(app(policy(), bindings()?), req, StatusCode::UNAUTHORIZED).await?;
    }
    Ok(())
}
#[tokio::test]
async fn unavailable_precedes_authentication() -> Result<(), Box<dyn Error>> {
    for p in [
        RoutePolicy::default(),
        RoutePolicy {
            enabled: false,
            ..policy()
        },
        RoutePolicy {
            group: "missing".into(),
            ..policy()
        },
        RoutePolicy {
            max_body_bytes: 0,
            ..policy()
        },
    ] {
        assert_failure(
            app(p, bindings()?),
            request("bad", None)?,
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await?;
    }
    assert_failure(
        app(policy(), IdentityMap::default()),
        request(BODY, Some(san("store.test.invalid")?))?,
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await?;
    for status in [StatusCode::UNAUTHORIZED, StatusCode::SERVICE_UNAVAILABLE] {
        let response = guard::failure(status);
        assert!(response.headers().is_empty());
        assert!(to_bytes(response.into_body(), 1).await?.is_empty());
    }
    Ok(())
}
#[test]
fn identity_configuration_refuses_overlap_and_bad_input() -> Result<(), Box<dyn Error>> {
    for value in ["", "bad\nidentity", "żółć", &"x".repeat(129)] {
        assert!(Identity::san(value).is_err());
    }
    for group in ["store", "other"] {
        assert!(
            IdentityMap::new(vec![
                Binding {
                    identity: san("peer")?,
                    group: "store".into()
                },
                Binding {
                    identity: san("peer")?,
                    group: group.into()
                }
            ])
            .is_err()
        );
    }
    assert!(
        IdentityMap::new(vec![Binding {
            identity: san("peer")?,
            group: "".into()
        }])
        .is_err()
    );
    Ok(())
}
#[tokio::test]
async fn limits_actual_stream_and_declared_length() -> Result<(), Box<dyn Error>> {
    let oversized = format!(
        r#"{{"contract_version":1,"operation":"{}"}}"#,
        "x".repeat(80)
    );
    assert_failure(
        app(policy(), bindings()?),
        request(&oversized, Some(san("store.test.invalid")?))?,
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await?;
    for length in ["65", "01", "-1", "99999999999", "invalid"] {
        let mut req = request(BODY, Some(san("store.test.invalid")?))?;
        req.headers_mut().insert("content-length", length.parse()?);
        assert_failure(
            app(policy(), bindings()?),
            req,
            StatusCode::PAYLOAD_TOO_LARGE,
        )
        .await?;
    }
    let mut p = policy();
    p.max_body_bytes = BODY.len();
    assert_eq!(
        app(p, bindings()?)
            .oneshot(request(BODY, Some(san("store.test.invalid")?))?)
            .await?
            .status(),
        StatusCode::OK
    );
    Ok(())
}
#[tokio::test]
async fn refuses_bad_transport_and_noncanonical_json() -> Result<(), Box<dyn Error>> {
    for (name, value) in [
        ("content-type", "text/plain"),
        ("content-type", "application/json; charset=utf-8"),
        ("content-encoding", "gzip"),
        ("transfer-encoding", "chunked"),
    ] {
        let mut req = request(BODY, Some(san("store.test.invalid")?))?;
        req.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes())?,
            value.parse()?,
        );
        assert_failure(app(policy(), bindings()?), req, StatusCode::BAD_REQUEST).await?;
    }
    for body in [
        "",
        "[]",
        "{}",
        "bad",
        "\u{feff}{}",
        "{\"contract_version\":1, \"operation\":\"read\"}",
        "{\"operation\":\"read\",\"contract_version\":1}",
        "{\"contract_version\":1,\"operation\":\"r\\u0065ad\"}",
        "{\"contract_version\":1,\"operation\":\"read\"}\n",
        "{\"contract_version\":1,\"operation\":\"read\",\"extra\":1}",
        "{\"contract_version\":1,\"contract_version\":1,\"operation\":\"read\"}",
    ] {
        assert_failure(
            app(policy(), bindings()?),
            request(body, Some(san("store.test.invalid")?))?,
            StatusCode::BAD_REQUEST,
        )
        .await?;
    }
    Ok(())
}
#[tokio::test]
async fn noncanonical_permitted_only_by_explicit_policy() -> Result<(), Box<dyn Error>> {
    let mut p = policy();
    p.json = JsonRule::Object;
    let body = "{ \"operation\":\"read\", \"contract_version\":1 }\n";
    assert_eq!(
        app(p, bindings()?)
            .oneshot(request(body, Some(san("store.test.invalid")?))?)
            .await?
            .status(),
        StatusCode::OK
    );
    Ok(())
}
#[test]
fn ordered_codec_rules_and_bounds() -> Result<(), Box<dyn Error>> {
    let raw = "{\"z\":\"zażółć/世界\",\"a\":\"18446744073709551615\",\"b\":-9223372036854775808,\"c\":true,\"d\":null}";
    assert_eq!(
        canonical::encode(&canonical::decode(raw.as_bytes(), 1024, 4)?)?,
        raw.as_bytes()
    );
    for raw in [
        "{\"n\":1.0}",
        "{\"n\":1e1}",
        "{\"n\":18446744073709551616}",
        "{\"n\":9223372036854775808}",
        "{\"x\":1,\"x\":2}",
        "{\"x\":1,\"\\u0078\":2}",
        "{\"n\":01}",
        "{}{}",
    ] {
        assert!(canonical::decode(raw.as_bytes(), 1024, 4).is_err(), "{raw}");
    }
    assert!(canonical::decode(b"{\"x\":[{\"y\":1}]}", 1024, 4).is_ok());
    assert!(canonical::decode(b"{\"x\":[{\"y\":[]}]}", 1024, 4).is_err());
    assert!(canonical::decode(b"{}", 1, 4).is_err());
    let escaped = canonical::decode(b"{\"x\":\"\\n\\\"\\\\\\u2028\\u2029\"}", 1024, 4)?;
    assert_eq!(
        canonical::encode(&escaped)?,
        b"{\"x\":\"\\n\\\"\\\\\\u2028\\u2029\"}"
    );
    static NESTED: Shape = Shape::Object(&[(
        "items",
        Shape::Array(&Shape::Object(&[
            ("a", Shape::Scalar),
            ("b", Shape::Scalar),
        ])),
    )]);
    assert!(
        canonical::decode_canonical(b"{\"items\":[{\"b\":2,\"a\":1}]}", 1024, 4, &NESTED).is_err()
    );
    Ok(())
}
#[test]
fn every_golden_fixture_round_trips() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/contracts/fixtures");
    let all = fixtures::load(root)?;
    assert!(!all.is_empty());
    for set in [
        "store-wire-v1",
        "premium-snapshot-v1",
        "store-price-list-v1",
        "oteryn-native-v1",
    ] {
        assert!(
            all.iter()
                .any(|f| f.path.parent().is_some_and(|p| p.ends_with(set)))
        );
    }
    for fixture in &all {
        assert_eq!(
            canonical::encode(&canonical::decode(
                &fixture.canonical,
                16 * 1024 * 1024,
                128
            )?)?,
            fixture.canonical
        );
        if fixture.was_canonical {
            assert_eq!(fixture.canonical, fixture.original);
        }
        if fixture
            .path
            .parent()
            .is_some_and(|p| p.ends_with("store-price-list-v1"))
            && !fixture.path.ends_with("price-list.schema.json")
        {
            assert!(fixture.was_canonical);
        }
    }
    let manifest = all
        .iter()
        .find(|f| f.path.ends_with("store-wire-v1/manifest.json"))
        .ok_or("Store manifest missing")?;
    let Value::Object(entries) = manifest
        .value
        .get("valid")
        .ok_or("manifest valid entries missing")?
    else {
        return Err("missing golden manifest".into());
    };
    for (name, metadata) in entries {
        let fixture = all
            .iter()
            .find(|f| f.path.ends_with(format!("store-wire-v1/{name}")))
            .ok_or("golden file missing")?;
        assert_eq!(
            metadata.get("canonical_bytes"),
            Some(&Value::Unsigned(fixture.canonical.len() as u64)),
            "{name}"
        );
        let digest = format!("{:x}", Sha256::digest(&fixture.canonical));
        assert_eq!(
            metadata.get("sha256"),
            Some(&Value::String(digest)),
            "{name}"
        );
    }
    println!(
        "Golden fixtures: {} JSON files; {} already canonical; {} Store manifest byte/hash vectors",
        all.len(),
        all.iter().filter(|f| f.was_canonical).count(),
        entries.len()
    );
    Ok(())
}

#[test]
fn outbound_canonical_encoder_enforces_contract() -> Result<(), Box<dyn Error>> {
    let value = canonical::decode(BODY.as_bytes(), 64, 4)?;
    assert_eq!(
        canonical::encode_canonical(&value, 64, 4, &HEAD)?,
        BODY.as_bytes()
    );
    assert!(canonical::encode_canonical(&value, 1, 4, &HEAD).is_err());
    assert!(canonical::encode_canonical(&value, 64, 1, &HEAD).is_err());
    let escaped = canonical::decode(br#"{"contract_version":1,"operation":"line\n"}"#, 64, 4)?;
    assert!(canonical::encode_canonical(&escaped, 64, 4, &HEAD).is_err());
    assert!(canonical::encode(&Value::Unsigned(u64::MAX)).is_err());
    Ok(())
}

#[test]
fn coin_contract_vectors_have_stable_hashes_and_route_coverage() -> Result<(), Box<dyn Error>> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/contracts/fixtures/coin-wire-v1");
    let all = fixtures::load(root)?;
    let manifest = all
        .iter()
        .find(|f| f.path.ends_with("manifest.json"))
        .ok_or("coin manifest")?;
    let Value::Object(valid) = manifest.value.get("valid").ok_or("valid")? else {
        return Err("valid object".into());
    };
    for (name, meta) in valid {
        let fixture = all
            .iter()
            .find(|f| f.path.ends_with(name))
            .ok_or("vector")?;
        assert!(fixture.was_canonical, "{name}");
        assert_eq!(
            meta.get("canonical_bytes"),
            Some(&Value::Unsigned(fixture.canonical.len() as u64))
        );
        assert_eq!(
            meta.get("sha256"),
            Some(&Value::String(format!(
                "{:x}",
                Sha256::digest(&fixture.canonical)
            )))
        );
    }
    let Value::Array(routes) = manifest.value.get("routes").ok_or("routes")? else {
        return Err("routes array".into());
    };
    assert_eq!(routes.len(), 7);
    for route in routes {
        for key in ["request", "response"] {
            let Some(Value::String(name)) = route.get(key) else {
                return Err("route vector".into());
            };
            assert!(valid.iter().any(|(file, _)| file == name));
        }
    }
    for code in [
        "replay",
        "insufficient_transferable",
        "account_inactive",
        "transfer_blocked",
        "unknown_account",
        "idempotency_conflict",
        "invalid",
        "unavailable",
    ] {
        let file = format!("{code}.json");
        let fixture = all
            .iter()
            .find(|f| f.path.ends_with(&file))
            .ok_or("result vector")?;
        assert_eq!(
            fixture.value.get("result"),
            Some(&Value::String(code.into()))
        );
    }
    Ok(())
}
