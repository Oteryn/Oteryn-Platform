#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use rand_core::OsRng;
use serde_json::{Value, json};

fn window() -> Validity {
    Validity {
        not_before: 10,
        not_after: 100,
    }
}
fn key(n: u32) -> AdminKey {
    AdminKey::new(
        format!("platform-admin-signer-{n}"),
        SigningKey::generate(&mut OsRng),
        window(),
    )
    .unwrap()
}
fn request() -> RequestEnvelope {
    let request_id = uuid::Uuid::now_v7();
    serde_json::from_value(json!({
        "contract":"oteryn.admin-control.v1",
        "request_id":request_id, "admin_operation_id":request_id,
        "operation":"world.list", "operation_version":1, "kind":"query",
        "target":{"world_id":"w-test","node_id":null,"character_id":null,"account_ref":null},
        "params":{}, "dry_run":false,
        "actor":{"admin_id":"a-test","roles":["game_viewer"],"permission":"game.world.read",
        "mfa_at":"2026-11-05T18:00:00Z","step_up_at":null,"ip_hash":"hash","session_ref":"session"},
        "reason":null,"approvals":[],"risk_class":"R0",
        "issued_at":"2026-11-05T18:00:01Z","expires_at":"2026-11-05T18:01:01Z",
        "nonce":URL_SAFE_NO_PAD.encode([0u8;16]),"signer_key_id":"platform-admin-signer-1","signature":""
    })).unwrap()
}
#[test]
fn round_trip_and_rotation() {
    let mut ring = KeyRing::new(true);
    ring.add(key(1)).unwrap();
    ring.add(key(2)).unwrap();
    for id in ["platform-admin-signer-1", "platform-admin-signer-2"] {
        let mut req = request();
        ring.sign(&mut req, id, 10).unwrap();
        let decoded: RequestEnvelope =
            serde_json::from_slice(&serde_json::to_vec(&req).unwrap()).unwrap();
        assert_eq!(req, decoded);
        ring.verify(&decoded, 99).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(&req.signature).unwrap().len(), 64);
    }
}
#[test]
fn tamper_every_field() {
    let mut ring = KeyRing::new(true);
    ring.add(key(1)).unwrap();
    let mut req = request();
    ring.sign(&mut req, "platform-admin-signer-1", 50).unwrap();
    let Value::Object(fields) = serde_json::to_value(&req).unwrap() else {
        panic!()
    };
    for (field, value) in &fields {
        let changed = match value {
            Value::String(s) => Value::String(match field.as_str() {
                "request_id" | "admin_operation_id" => uuid::Uuid::now_v7().to_string(),
                "kind" => "command".into(),
                "risk_class" => "R1".into(),
                _ => format!("{s}x"),
            }),
            Value::Bool(b) => json!(!b),
            Value::Number(_) => json!(2),
            Value::Object(_) if field == "params" => json!({"x":1}),
            Value::Object(_) if field == "actor" => {
                let mut v = value.clone();
                v["admin_id"] = json!("other");
                v
            }
            Value::Object(_) => {
                let mut v = value.clone();
                v["world_id"] = json!("other");
                v
            }
            Value::Array(_) => {
                json!([{"admin_id":"a-other","approved_at":"later","approval_id":"approval"}])
            }
            Value::Null => json!({"code":"OTHER","text":"changed","ticket_ref":null}),
        };
        let mut tampered = fields.clone();
        tampered.insert(field.clone(), changed);
        let decoded = serde_json::from_value::<RequestEnvelope>(Value::Object(tampered));
        assert!(
            decoded.is_err() || ring.verify(&decoded.unwrap(), 50).is_err(),
            "{field}"
        );
    }
}
#[test]
fn refusals_and_bad_input() {
    let mut ring = KeyRing::new(true);
    ring.add(key(1)).unwrap();
    ring.add(key(2)).unwrap();
    assert!(ring.add(key(3)).is_err());
    let mut req = request();
    ring.sign(&mut req, "platform-admin-signer-1", 50).unwrap();
    for time in [9, 100, 101] {
        assert!(matches!(ring.verify(&req, time), Err(Error::Untrusted)));
        assert!(
            ring.sign(&mut req, "platform-admin-signer-1", time)
                .is_err()
        );
    }
    let mut wrong = KeyRing::new(true);
    wrong.add(key(1)).unwrap();
    assert!(matches!(wrong.verify(&req, 50), Err(Error::BadSignature)));
    assert!(matches!(
        KeyRing::default().sign(&mut req, "platform-admin-signer-1", 50),
        Err(Error::Disabled)
    ));
    req.admin_operation_id = uuid::Uuid::now_v7();
    assert!(ring.sign(&mut req, "platform-admin-signer-1", 50).is_err());
    req = request();
    req.nonce = "bad".into();
    assert!(ring.sign(&mut req, "platform-admin-signer-1", 50).is_err());
    assert!(AdminKey::new("other-1".into(), SigningKey::generate(&mut OsRng), window()).is_err());
    assert!(
        AdminKey::new(
            "platform-admin-signer-1".into(),
            SigningKey::generate(&mut OsRng),
            Validity {
                not_before: 1,
                not_after: 1
            }
        )
        .is_err()
    );
    let mut value = serde_json::to_value(request()).unwrap();
    value["unknown"] = json!(true);
    assert!(serde_json::from_value::<RequestEnvelope>(value).is_err());
    let mut value = serde_json::to_value(request()).unwrap();
    value["target"]["unknown"] = json!(true);
    assert!(serde_json::from_value::<RequestEnvelope>(value).is_err());
}
#[test]
fn result_signature_round_trip_and_tampering() {
    let key = SigningKey::generate(&mut OsRng);
    let mut result: ResultEnvelope = serde_json::from_value(json!({
        "request_id":uuid::Uuid::now_v7(),"admin_operation_id":uuid::Uuid::now_v7(),
        "status":"succeeded","error":null,"result":{"worlds":[]},"dry_run_preview":null,
        "game_ledger_id":null,"audit_event_ids":[],"effective_revision":null,"progress":null,
        "completed_at":"2026-11-05T18:00:01Z","agent_id":"agent-test","agent_build":"test","signature":""
    })).unwrap();
    result.signature =
        URL_SAFE_NO_PAD.encode(key.sign(&adm_x1_signing_bytes(&result).unwrap()).to_bytes());
    let copy: ResultEnvelope =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(copy, result);
    let value = serde_json::to_value(&result).unwrap();
    for member in value.as_object().unwrap().keys() {
        let mut changed = value.clone();
        changed.as_object_mut().unwrap().remove(member);
        assert!(
            serde_json::from_value::<ResultEnvelope>(changed).is_err(),
            "missing {member}"
        );
    }
    verify_result(&result, &key.verifying_key(), window(), 50).unwrap();
    assert!(
        verify_result(
            &result,
            &SigningKey::generate(&mut OsRng).verifying_key(),
            window(),
            50
        )
        .is_err()
    );
    assert!(verify_result(&result, &key.verifying_key(), window(), 100).is_err());
    let Value::Object(fields) = serde_json::to_value(&result).unwrap() else {
        panic!()
    };
    for (name, value) in &fields {
        let replacement = match value {
            Value::String(_) if name.ends_with("_id") && name != "agent_id" => {
                json!(uuid::Uuid::now_v7())
            }
            Value::String(_) if name == "status" => json!("failed"),
            Value::String(_) => json!("changed"),
            Value::Array(_) => json!(["changed"]),
            Value::Object(_) => json!({"changed":true}),
            _ => json!("changed"),
        };
        let mut v = fields.clone();
        v.insert(name.clone(), replacement);
        if let Ok(changed) = serde_json::from_value::<ResultEnvelope>(Value::Object(v)) {
            assert!(
                verify_result(&changed, &key.verifying_key(), window(), 50).is_err(),
                "{name}"
            );
        }
    }
}
#[test]
fn payload_validation_and_digest() {
    for raw in [
        r#"{"x":1,"x":2}"#,
        r#"{"x":{"y":1,"y":2}}"#,
        r#"{"x":9007199254740993}"#,
        r#"{"x":-9007199254740993}"#,
        r#"{"x":1e30}"#,
        r#"{"x":"\ud800"}"#,
        "NaN",
    ] {
        assert!(serde_json::from_str::<Json>(raw).is_err(), "{raw}");
    }
    let a: Json = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
    let b: Json = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
    assert_eq!(params_digest(&a).unwrap(), params_digest(&b).unwrap());
    assert_eq!(
        params_digest(&Json(json!({}))).unwrap().as_slice(),
        &Sha256::digest(b"{}")[..]
    );
    assert!(canonical::encode_wire(&json!([9007199254740993u64])).is_err());
    assert!(canonical::encode_wire(&json!({"n":"9007199254740993"})).is_ok());
}
#[test]
fn rfc8785_sample_and_utf16_sorting() {
    let input = r#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001],"string":"€$\u000f\nA'B\"\\\"/","literals":[null,true,false]}"#;
    let value: Value = serde_json::from_str(input).unwrap();
    let expected = r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\"/"}"#;
    assert_eq!(
        String::from_utf8(canonical::encode(&value).unwrap()).unwrap(),
        expected
    );
    let value = json!({"\u{20ac}":1,"\r":2,"\u{fb33}":3,"1":4,"\u{1f600}":5,"\u{80}":6,"ö":7});
    assert_eq!(
        String::from_utf8(canonical::encode(&value).unwrap()).unwrap(),
        "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"ö\":7,\"€\":1,\"😀\":5,\"דּ\":3}"
    );
}
#[test]
fn rfc8785_appendix_b_number_vectors() {
    let vectors = [
        (0x0000000000000000, "0"),
        (0x8000000000000000, "0"),
        (0x0000000000000001, "5e-324"),
        (0x8000000000000001, "-5e-324"),
        (0x7fefffffffffffff, "1.7976931348623157e+308"),
        (0xffefffffffffffff, "-1.7976931348623157e+308"),
        (0x4340000000000000, "9007199254740992"),
        (0xc340000000000000, "-9007199254740992"),
        (0x4430000000000000, "295147905179352830000"),
        (0x44b52d02c7e14af5, "9.999999999999997e+22"),
        (0x44b52d02c7e14af6, "1e+23"),
        (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
        (0x444b1ae4d6e2ef4e, "999999999999999700000"),
        (0x444b1ae4d6e2ef4f, "999999999999999900000"),
        (0x444b1ae4d6e2ef50, "1e+21"),
        (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
        (0x3eb0c6f7a0b5ed8d, "0.000001"),
        (0x41b3de4355555553, "333333333.3333332"),
        (0x41b3de4355555554, "333333333.33333325"),
        (0x41b3de4355555555, "333333333.3333333"),
        (0x41b3de4355555556, "333333333.3333334"),
        (0x41b3de4355555557, "333333333.33333343"),
        (0xbecbf647612f3696, "-0.0000033333333333333333"),
        (0x43143ff3c1cb0959, "1424953923781206.2"),
    ];
    for (bits, expected) in vectors {
        assert_eq!(
            String::from_utf8(canonical::encode(&f64::from_bits(bits)).unwrap()).unwrap(),
            expected,
            "{bits:x}"
        );
    }
    for bits in [0x7fffffffffffffff, 0x7ff0000000000000, 0xfff0000000000000] {
        assert!(canonical::encode(&f64::from_bits(bits)).is_err());
    }
}
#[test]
fn key_file_loading() {
    let path = std::env::temp_dir().join(format!("admin-key-test-{}", uuid::Uuid::now_v7()));
    let generated = SigningKey::generate(&mut OsRng);
    std::fs::write(&path, generated.to_bytes()).unwrap();
    let loaded = AdminKey::from_file(&path, "platform-admin-signer-1".into(), window()).unwrap();
    assert_eq!(loaded.verifying_key(), generated.verifying_key());
    assert!(!format!("{loaded:?}").contains(&format!("{:?}", generated.to_bytes())));
    std::fs::write(&path, b"invalid").unwrap();
    assert!(AdminKey::from_file(&path, "platform-admin-signer-1".into(), window()).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(AdminKey::from_file(&path, "platform-admin-signer-1".into(), window()).is_err());
    assert!(
        AdminKey::from_env(
            "ABSENT_ADMIN_SIGNING_KEY_TEST_PATH",
            "platform-admin-signer-1".into(),
            window()
        )
        .is_err()
    );
}

#[test]
fn deserialization_cannot_restore_tampered_signed_members() {
    let value = serde_json::to_value(request()).unwrap();
    for member in value.as_object().unwrap().keys() {
        let mut changed = value.clone();
        changed.as_object_mut().unwrap().remove(member);
        assert!(
            serde_json::from_value::<RequestEnvelope>(changed).is_err(),
            "missing {member}"
        );
    }
    for parent in ["target", "actor"] {
        for member in value[parent].as_object().unwrap().keys() {
            let mut changed = value.clone();
            changed[parent].as_object_mut().unwrap().remove(member);
            assert!(
                serde_json::from_value::<RequestEnvelope>(changed).is_err(),
                "missing {parent}.{member}"
            );
        }
    }
    let mut changed = value;
    changed["request_id"] = json!("0193f0c2-abcd-7000-8000-000000000000".to_uppercase());
    assert!(serde_json::from_value::<RequestEnvelope>(changed).is_err());
}

const GOLDEN_VECTORS: &[(&str, Option<&str>)] = &[
    ("busy.json", Some("ADM_BUSY")),
    ("closed-params.json", Some("ADM_INVALID_PARAMS")),
    ("disabled-operation.json", Some("ADM_FORBIDDEN")),
    ("disabled-world.json", Some("ADM_FORBIDDEN")),
    ("excessive-lifetime.json", Some("ADM_EXPIRED")),
    ("expired-key.json", Some("ADM_SIGNER_UNTRUSTED")),
    ("expired.json", Some("ADM_EXPIRED")),
    ("future-issued.json", Some("ADM_EXPIRED")),
    ("future-step-up.json", Some("ADM_CONTROL_MISSING")),
    (
        "idempotency-conflict.json",
        Some("ADM_IDEMPOTENCY_CONFLICT"),
    ),
    ("idempotency-recorded.json", None),
    ("internal.json", Some("ADM_INTERNAL")),
    ("invalid-nonce.json", Some("ADM_INVALID_PARAMS")),
    ("limit.json", Some("ADM_LIMIT_EXCEEDED")),
    ("missing-reason.json", Some("ADM_CONTROL_MISSING")),
    ("missing-step-up.json", Some("ADM_CONTROL_MISSING")),
    ("permission.json", Some("ADM_FORBIDDEN")),
    ("precondition.json", Some("ADM_PRECONDITION_FAILED")),
    ("preview-mismatch.json", Some("ADM_PREVIEW_STALE")),
    ("replay.json", Some("ADM_REPLAY")),
    ("risk-downgrade.json", Some("ADM_RISK_CLASS_MISMATCH")),
    ("rotated-out-key.json", Some("ADM_SIGNER_UNTRUSTED")),
    ("rotation-next-key.json", None),
    ("self-approval.json", Some("ADM_CONTROL_MISSING")),
    ("stale-approval.json", Some("ADM_CONTROL_MISSING")),
    ("stale-step-up.json", Some("ADM_CONTROL_MISSING")),
    ("superseded.json", Some("ADM_AGENT_SUPERSEDED")),
    ("tampered-field.json", Some("ADM_BAD_SIGNATURE")),
    ("target-not-found.json", Some("ADM_TARGET_NOT_FOUND")),
    ("unavailable.json", Some("ADM_TARGET_UNAVAILABLE")),
    ("unknown-contract.json", Some("ADM_UNKNOWN_OPERATION")),
    ("unknown-operation.json", Some("ADM_UNKNOWN_OPERATION")),
    ("unknown-version.json", Some("ADM_UNKNOWN_OPERATION")),
    ("valid.json", None),
    ("wrong-key.json", Some("ADM_BAD_SIGNATURE")),
];

// Deliberately limited to the pinned schema vocabulary, rejecting new keywords.
fn assert_schema(schema: &Value, root: &Value, value: &Value) {
    for keyword in schema.as_object().unwrap().keys() {
        assert!(
            [
                "$schema",
                "$defs",
                "$ref",
                "title",
                "description",
                "type",
                "properties",
                "additionalProperties",
                "required",
                "items",
                "enum",
                "anyOf",
                "format",
                "minimum"
            ]
            .contains(&keyword.as_str()),
            "unsupported schema keyword: {keyword}"
        );
    }
    if let Some(reference) = schema["$ref"].as_str() {
        assert_schema(
            root.pointer(reference.strip_prefix('#').unwrap()).unwrap(),
            root,
            value,
        );
    }
    if let Some(alternatives) = schema["anyOf"].as_array() {
        assert_eq!(alternatives.len(), 2);
        assert!(alternatives.iter().any(|s| s["type"] == "null"));
        assert!(alternatives.iter().any(|s| s.get("$ref").is_some()));
        let branch = alternatives
            .iter()
            .find(|s| (s["type"] == "null") == value.is_null())
            .unwrap();
        assert_schema(branch, root, value);
    }
    if let Some(kind) = schema.get("type") {
        let matches = |kind: &str| match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "null" => value.is_null(),
            "boolean" => value.is_boolean(),
            "integer" => value.as_u64().is_some() || value.as_i64().is_some(),
            _ => panic!("unsupported schema type: {kind}"),
        };
        assert!(
            if let Some(k) = kind.as_str() {
                matches(k)
            } else {
                kind.as_array()
                    .unwrap()
                    .iter()
                    .any(|k| matches(k.as_str().unwrap()))
            },
            "schema type mismatch: {value}"
        );
    }
    if let Some(options) = schema["enum"].as_array() {
        assert!(options.contains(value));
    }
    if let Some(minimum) = schema["minimum"].as_f64() {
        assert!(value.as_f64().unwrap() >= minimum);
    }
    if schema["format"] == "uint32" {
        assert!(u32::try_from(value.as_u64().unwrap()).is_ok());
    }
    if let Some(required) = schema["required"].as_array() {
        for key in required {
            assert!(value.get(key.as_str().unwrap()).is_some());
        }
    }
    if let Some(properties) = schema["properties"].as_object() {
        for (key, member) in value.as_object().unwrap() {
            if let Some(child) = properties.get(key) {
                assert_schema(child, root, member);
            } else {
                assert_ne!(schema["additionalProperties"], false);
            }
        }
    }
    if let Some(items) = schema.get("items") {
        for item in value.as_array().unwrap() {
            assert_schema(items, root, item);
        }
    }
}

fn golden_signature(vector: &Value, request: &RequestEnvelope) -> Result<(), Error> {
    let trusted = vector["trusted_signers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|key| key["key_id"].as_str() == Some(&request.signer_key_id))
        .ok_or(Error::Untrusted)?;
    let key: [u8; 32] = URL_SAFE_NO_PAD
        .decode(trusted["public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let parse_time = |text: &Value| {
        time::OffsetDateTime::parse(
            text.as_str().unwrap(),
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap()
        .unix_timestamp()
    };
    verify_signature(
        request,
        &request.signature,
        &VerifyingKey::from_bytes(&key).unwrap(),
        Validity {
            not_before: parse_time(&trusted["not_before"]),
            not_after: parse_time(&trusted["not_after"]),
        },
        vector["clock"].as_i64().unwrap(),
    )
}

#[test]
fn adm_x1_game_golden_envelopes() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/control/fixtures/admin-control-v1");
    let mut actual: Vec<_> = std::fs::read_dir(directory.join("vectors"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    actual.sort();
    assert_eq!(
        actual,
        GOLDEN_VECTORS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
    );
    let schema: Value =
        serde_json::from_slice(&std::fs::read(directory.join("request.schema.json")).unwrap())
            .unwrap();
    for &(name, expected_reason) in GOLDEN_VECTORS {
        let vector: Value =
            serde_json::from_slice(&std::fs::read(directory.join("vectors").join(name)).unwrap())
                .unwrap();
        assert_eq!(vector["expected_error"].as_str(), expected_reason, "{name}");
        assert_schema(&schema, &schema, &vector["request"]);
        let request: RequestEnvelope = serde_json::from_value(vector["request"].clone()).unwrap();
        assert_eq!(
            adm_x1_signing_bytes(&request).unwrap(),
            vector["canonical_signed_utf8"].as_str().unwrap().as_bytes(),
            "{name}"
        );
        let verified = golden_signature(&vector, &request);
        match expected_reason {
            Some("ADM_SIGNER_UNTRUSTED") => {
                assert!(matches!(verified, Err(Error::Untrusted)), "{name}")
            }
            Some("ADM_BAD_SIGNATURE") => {
                assert!(matches!(verified, Err(Error::BadSignature)), "{name}")
            }
            Some("ADM_INVALID_PARAMS") if name == "invalid-nonce.json" => {
                assert!(
                    matches!(validate_request(&request), Err(Error::Envelope)),
                    "{name}"
                );
                verified.unwrap();
            }
            None => {
                validate_request(&request).unwrap();
                verified.unwrap();
            }
            _ => verified.unwrap(),
        }
    }
}
