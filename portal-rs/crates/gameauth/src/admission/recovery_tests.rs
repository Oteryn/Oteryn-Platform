#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! §19.4, §19.7 and §19.8 wire bytes against the protocol 3 fixtures.
use super::*;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/contracts/fixtures/native-gateway-protocol3-v1/"
);
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";

fn fixture(name: &str) -> Vec<u8> {
    let raw = std::fs::read(format!("{FIXTURES}{name}")).unwrap();
    raw.strip_suffix(b"\n").unwrap_or(&raw).to_vec()
}

#[test]
fn request_fixture_decodes_and_binds_character_and_offer() {
    let raw = fixture("recover-request.json");
    let request = RecoveryRequest::decode(&raw).unwrap();
    assert_eq!(request.attempt_ref.to_string(), ATTEMPT);
    // The digest binds the offer: reordering members keeps it, a change does not.
    let text = String::from_utf8(raw).unwrap();
    let changed = text.replace(
        "\"client_build\":\"0.1.0+abc123\"",
        "\"client_build\":\"0.1.1+abc123\"",
    );
    assert_ne!(changed, text);
    match RecoveryRequest::decode(changed.as_bytes()) {
        Ok(other) => assert_ne!(other.digest, request.digest),
        Err(error) => assert_eq!(error, RecoveryError::OfferUnsupported),
    }
}

#[test]
fn bad_requests_are_refused() {
    let text = String::from_utf8(fixture("recover-request.json")).unwrap();
    let cases: Vec<(String, RecoveryError)> = vec![
        (
            text.replace("\"protocol_version\":3", "\"protocol_version\":2"),
            RecoveryError::UnsupportedVersion,
        ),
        (text.replace("otr1.", "otr2."), RecoveryError::Malformed),
        (
            text.replacen('{', "{\"device_proof\":\"x\",", 1),
            RecoveryError::Malformed,
        ),
        (
            text.replacen('{', "{\"channel_id\":null,", 1),
            RecoveryError::Malformed,
        ),
        (
            text.replace(ATTEMPT, "0192B3C4-5D6E-7F80-9A1B-2C3D4E5F6A7B"),
            RecoveryError::Malformed,
        ),
        ("[]".into(), RecoveryError::Malformed),
        ("{".into(), RecoveryError::Malformed),
    ];
    for (raw, expected) in cases {
        assert_eq!(
            RecoveryRequest::decode(raw.as_bytes()).unwrap_err(),
            expected,
            "{raw}"
        );
    }
}

#[test]
fn success_matches_fixture() {
    let expected = fixture("recover-success.json");
    let value: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    let endpoint: Endpoint = serde_json::from_value(value["endpoint"].clone()).unwrap();
    let body = success(
        value["attempt_ref"].as_str().unwrap().parse().unwrap(),
        value["world_id"].as_str().unwrap().parse().unwrap(),
        value["channel_id"].as_str().unwrap().parse().unwrap(),
        &endpoint,
        value["grant"]["token"].as_str().unwrap(),
        value["grant"]["valid_for_seconds"].as_i64().unwrap(),
    )
    .unwrap();
    assert_eq!(body, expected);
    assert!(body.len() <= MAX_RESPONSE);
}

#[test]
fn errors_match_fixtures() {
    let attempt = Some(ATTEMPT);
    for (name, error, attempt) in [
        (
            "recover-request-malformed.json",
            RecoveryError::Malformed,
            attempt,
        ),
        (
            "recover-authentication-required.json",
            RecoveryError::HandleRejected,
            attempt,
        ),
        (
            "recover-attempt-conflict.json",
            RecoveryError::AttemptConflict,
            attempt,
        ),
        (
            "recover-attempt-pending.json",
            RecoveryError::AttemptPending,
            attempt,
        ),
        (
            "recover-character-conflict.json",
            RecoveryError::CharacterConflict,
            attempt,
        ),
        (
            "recover-world-stale.json",
            RecoveryError::WorldStale,
            attempt,
        ),
        (
            "recover-route-unavailable.json",
            RecoveryError::RouteUnavailable,
            attempt,
        ),
        (
            "recover-grant-expired.json",
            RecoveryError::GrantExpired,
            attempt,
        ),
        (
            "recover-reconciliation-required.json",
            RecoveryError::ReconciliationRequired,
            attempt,
        ),
        (
            "recover-unavailable.json",
            RecoveryError::Unavailable,
            attempt,
        ),
        (
            "recover-unsupported-version.json",
            RecoveryError::UnsupportedVersion,
            attempt,
        ),
        (
            "recover-offer-unsupported.json",
            RecoveryError::OfferUnsupported,
            attempt,
        ),
        (
            "recover-rate-limited.json",
            RecoveryError::RateLimited,
            attempt,
        ),
        (
            "recover-revoke-malformed.json",
            RecoveryError::Malformed,
            None,
        ),
    ] {
        assert_eq!(error.body(attempt, 3), fixture(name), "{name}");
    }
    // Every SECURITY_TERMINAL reason has the same public bytes (§19.8).
    for error in [
        RecoveryError::DeviceProofRejected,
        RecoveryError::SecurityDenied,
    ] {
        assert_eq!(
            error.body(attempt, 3),
            fixture("recover-authentication-required.json")
        );
    }
    let statuses = [
        (RecoveryError::Malformed, 400),
        (RecoveryError::UnsupportedVersion, 400),
        (RecoveryError::HandleRejected, 401),
        (RecoveryError::DeviceProofRejected, 401),
        (RecoveryError::SecurityDenied, 401),
        (RecoveryError::OfferUnsupported, 409),
        (RecoveryError::AttemptConflict, 409),
        (RecoveryError::AttemptPending, 409),
        (RecoveryError::CharacterConflict, 409),
        (RecoveryError::WorldStale, 409),
        (RecoveryError::GrantExpired, 409),
        (RecoveryError::RateLimited, 429),
        (RecoveryError::RouteUnavailable, 503),
        (RecoveryError::ReconciliationRequired, 503),
        (RecoveryError::Unavailable, 503),
    ];
    for (error, status) in statuses {
        assert_eq!(error.status().as_u16(), status, "{error:?}");
    }
}

#[test]
fn revoke_fixtures() {
    let hash = decode_revoke(&fixture("recover-revoke-request.json")).unwrap();
    let request = RecoveryRequest::decode(&fixture("recover-request.json")).unwrap();
    assert_eq!(hash, request.handle_hash);
    assert_eq!(REVOKED, fixture("recover-revoke-success.json").as_slice());
    assert_eq!(
        decode_revoke(br#"{"protocol_version":3,"recovery_handle":"otr1.short"}"#).unwrap_err(),
        RecoveryError::Malformed
    );
    assert_eq!(
        decode_revoke(br#"{"protocol_version":3}"#).unwrap_err(),
        RecoveryError::Malformed
    );
}

#[test]
fn claims_are_in_contract_order() {
    let report = canonical::decode(
        br#"{"content_revision":"c1","map_revision":"m1","ruleset_revision":"r1","world_policy_revision":"p1"}"#,
        1024,
        5,
    )
    .unwrap();
    let attempt: Uuid = ATTEMPT.parse().unwrap();
    let bytes = payload(&Claims {
        iat: 1_790_000_000,
        ttl: 30,
        jti: "A".repeat(43),
        attempt,
        account: attempt,
        character: attempt,
        world: attempt,
        security_generation: 7,
        report: &report,
    })
    .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let a = ATTEMPT;
    assert_eq!(
        text,
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":1790000000,"nbf":1790000000,"exp":1790000030,"jti":"{jti}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"{a}","account_id":"{a}","character_id":"{a}","world_id":"{a}","account_security_generation":"7","protocol_major":1,"transport_profile":1,"ruleset_revision":"r1","content_revision":"c1","map_revision":"m1","world_policy_revision":"p1"}}"#,
            jti = "A".repeat(43)
        )
    );
}
