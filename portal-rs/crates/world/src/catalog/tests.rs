#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
const RAW: &[u8] =
    include_bytes!("../../tests/fixtures/catalog-d34807f5/game-platform-catalog.json");
const SIDECAR: &[u8] =
    include_bytes!("../../tests/fixtures/catalog-d34807f5/game-platform-catalog.json.sha256");
fn base() -> Value {
    serde_json::from_slice(RAW).unwrap()
}
fn fence(v: &Value) -> Fence {
    Fence {
        source_revision: v["source_revision"].as_str().unwrap().into(),
        authority_epoch: v["authority_epoch"].as_str().unwrap().into(),
        ruleset_id: v["ruleset_id"].as_str().unwrap().into(),
        content_profile_id: v["content_profile_id"].as_str().unwrap().into(),
        snapshot_id: v["snapshot_id"].as_str().unwrap().into(),
    }
}
fn artifact(mut v: Value) -> (Vec<u8>, Vec<u8>, Fence) {
    let object = v.as_object_mut().unwrap();
    object.remove("snapshot_id");
    object.remove("payload_digest");
    let digest = format!("sha256:{:x}", Sha256::digest(json::canonical(&v).unwrap()));
    v["snapshot_id"] = digest.clone().into();
    v["payload_digest"] = digest.into();
    let f = fence(&v);
    let mut raw = json::canonical(&v).unwrap();
    raw.push(b'\n');
    let sidecar = format!("{:x}\n", Sha256::digest(&raw)).into_bytes();
    (raw, sidecar, f)
}
fn supported() -> Value {
    let mut v = base();
    v["capability_manifest"][1]["support"] = "supported".into();
    v["completeness_manifest"][1]["state"] = "complete".into();
    v["entities"] = serde_json::json!([{"type":"creature","content_key":"oteryn:creature.synthetic","capability_id":"creature","data":{"name":"Zażółć 🐉\u{2028}","health":42}}]);
    v
}
#[test]
fn exact_game_negative_fixture_and_synthetic_carriers_validate_as_inactive() {
    let candidate = verify(RAW, SIDECAR, &fence(&base())).unwrap();
    assert!(candidate.snapshot.entities.is_empty());
    assert!(
        candidate
            .snapshot
            .capability_manifest
            .iter()
            .all(|c| c.support == "unsupported")
    );
    assert_eq!(
        candidate.snapshot_id(),
        "sha256:6d599c15564e1ad1b0ec307a73fa52e517a452ae6b0eab4056a0cd67e2d27f8e"
    );
    let (raw, sidecar, f) = artifact(supported());
    assert!(verify(&raw, &sidecar, &f).is_ok());
}
#[test]
fn invalid_semantics_refuse_even_with_recomputed_valid_digests() {
    let cases: Vec<fn(&mut Value)> = vec![
        |v| v["unexpected"] = true.into(),
        |v| v["contract_id"] = "other".into(),
        |v| v["schema_version"] = "2.0.0".into(),
        |v| v["content_authority_id"] = "other".into(),
        |v| v["source_revision"] = "main".into(),
        |v| v["generated_at"] = "2026-02-30T12:00:00Z".into(),
        |v| v["generated_at"] = "2026-01-01T12:00:00+00:00".into(),
        |v| v["authority_epoch"] = "Bad epoch".into(),
        |v| v["ruleset_id"] = "unnamespaced".into(),
        |v| v["required_capabilities"] = serde_json::json!(["item"]),
        |v| v["required_capabilities"] = serde_json::json!(["creature", "creature"]),
        |v| v["capability_manifest"][0]["capability_id"] = "future".into(),
        |v| v["capability_manifest"][0] = v["capability_manifest"][1].clone(),
        |v| {
            v["completeness_manifest"].as_array_mut().unwrap().pop();
        },
        |v| v["completeness_manifest"][0]["state"] = "complete".into(),
        |v| v["entities"][0]["content_key"] = "123".into(),
        |v| v["entities"][0]["capability_id"] = "item".into(),
        |v| v["entities"][0]["data"] = serde_json::json!([]),
        |v| v["entities"][0]["data"]["health"] = serde_json::json!(1.25),
        |v| v["entities"][0]["data"]["health"] = serde_json::json!(9223372036854775808u64),
        |v| v["entities"][0]["data"]["name"] = "x".repeat(2049).into(),
        |v| {
            let e = v["entities"][0].clone();
            v["entities"].as_array_mut().unwrap().push(e);
        },
        |v| v["relations"] = serde_json::json!([{"type":"loot","relation_key":"oteryn:relation.synthetic","capability_id":"creature","source":"oteryn:creature.missing","target":null,"data":{}}]),
        |v| v["tombstones"] = serde_json::json!([{"content_key":"oteryn:creature.synthetic","capability_id":"creature","reason":"contradiction"}]),
        |v| {
            v["completeness_manifest"][1]["state"] = "partial".into();
            v["tombstones"] = serde_json::json!([{"content_key":"oteryn:creature.other","capability_id":"creature","reason":"incomplete"}]);
        },
        |v| {
            let mut deep = serde_json::json!({});
            for _ in 0..18 {
                deep = serde_json::json!({"nested":deep});
            }
            v["entities"][0]["data"] = deep;
        },
    ];
    for (index, mutate) in cases.into_iter().enumerate() {
        let mut v = supported();
        mutate(&mut v);
        let (raw, sidecar, f) = artifact(v);
        assert!(
            verify(&raw, &sidecar, &f).is_err(),
            "accepted invalid case {index}"
        );
    }
}
#[test]
fn provenance_and_integrity_have_separate_failure_categories() {
    for field in 0..5 {
        let mut f = fence(&base());
        match field {
            0 => f.source_revision = "a".repeat(40),
            1 => f.ruleset_id = "oteryn:ruleset.other".into(),
            2 => f.content_profile_id = "oteryn:profile.other".into(),
            3 => f.snapshot_id = format!("sha256:{}", "a".repeat(64)),
            _ => f.authority_epoch = "other".into(),
        };
        assert_eq!(
            verify(RAW, SIDECAR, &f).unwrap_err(),
            Error::ProvenanceConflict
        );
    }
    assert_eq!(
        verify(RAW, b"bad", &fence(&base())).unwrap_err(),
        Error::IntegrityMismatch
    );
    let mut v = base();
    v["generated_at"] = "2026-08-22T17:55:01Z".into();
    let mut raw = json::canonical(&v).unwrap();
    raw.push(b'\n');
    let sidecar = format!("{:x}\n", Sha256::digest(&raw));
    assert_eq!(
        verify(&raw, sidecar.as_bytes(), &fence(&v)).unwrap_err(),
        Error::IntegrityMismatch
    );
}
#[test]
fn duplicate_members_noncanonical_bytes_and_invalid_utf8_refuse() {
    for raw in [
        b"{\"x\":1,\"x\":2}\n".as_slice(),
        b"{\"data\":{\"x\":1,\"x\":2}}\n",
        b"\xff\n",
        b"{}",
        b"{}\n\n",
    ] {
        let sidecar = format!("{:x}\n", Sha256::digest(raw));
        assert!(verify(raw, sidecar.as_bytes(), &fence(&base())).is_err());
    }
    let mut raw = RAW.to_vec();
    raw.insert(1, b' ');
    let sidecar = format!("{:x}\n", Sha256::digest(&raw));
    assert!(verify(&raw, sidecar.as_bytes(), &fence(&base())).is_err());
}
#[test]
fn source_refuses_missing_or_mismatched_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("game-platform-catalog.json");
    let source = Source {
        file: file.clone(),
        fence: fence(&base()),
    };
    assert_eq!(source.read().unwrap_err(), Error::Unavailable);
    std::fs::write(&file, RAW).unwrap();
    std::fs::write(file.with_extension("json.sha256"), SIDECAR).unwrap();
    assert!(source.read().is_ok());
    std::fs::write(file.with_extension("json.sha256"), b"bad").unwrap();
    assert_eq!(source.read().unwrap_err(), Error::IntegrityMismatch);
}

#[test]
fn reference_producer_supported_vector_matches_consumer() {
    let raw = include_bytes!("../../tests/fixtures/catalog-d34807f5/synthetic-supported.json");
    let sidecar =
        include_bytes!("../../tests/fixtures/catalog-d34807f5/synthetic-supported.json.sha256");
    let value: Value = serde_json::from_slice(raw).unwrap();
    assert!(verify(raw, sidecar, &fence(&value)).is_ok());
}

#[test]
fn no_configuration_is_cold_and_partial_configuration_refuses() {
    let absent = || Err(std::env::VarError::NotPresent);
    assert!(
        Source::from_values((0..6).map(|_| absent()).collect())
            .unwrap()
            .is_none()
    );
    let mut partial: Vec<_> = (0..6).map(|_| absent()).collect();
    partial[0] = Ok("catalog.json".into());
    assert_eq!(
        Source::from_values(partial).unwrap_err(),
        Error::ProvenanceConflict
    );
}
#[test]
fn oversized_artifact_refuses_before_reading_payload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("too-large.json");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(
        Source {
            file: path,
            fence: fence(&base())
        }
        .read()
        .unwrap_err(),
        Error::CapacityExceeded
    );
}
#[test]
fn relation_target_is_nullable_but_must_be_present() {
    let mut v = supported();
    v["relations"] = serde_json::json!([{"type":"relation","relation_key":"oteryn:relation.synthetic","capability_id":"creature","source":"oteryn:creature.synthetic","target":null,"data":{}}]);
    let (raw, sidecar, f) = artifact(v.clone());
    assert!(verify(&raw, &sidecar, &f).is_ok());
    v["relations"][0].as_object_mut().unwrap().remove("target");
    let (raw, sidecar, f) = artifact(v);
    assert!(verify(&raw, &sidecar, &f).is_err());
}
