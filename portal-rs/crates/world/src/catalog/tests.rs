#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::catalog_fixture;
static FIXTURE: std::sync::LazyLock<catalog_fixture::Fixture> =
    std::sync::LazyLock::new(catalog_fixture::negative);
fn raw() -> &'static [u8] {
    &FIXTURE.bytes
}
fn sidecar() -> &'static [u8] {
    &FIXTURE.sidecar
}
fn base() -> Value {
    serde_json::from_slice(raw()).unwrap()
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
    let candidate = verify(raw(), sidecar(), &fence(&base())).unwrap();
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
            verify(raw(), sidecar(), &f).unwrap_err(),
            Error::ProvenanceConflict
        );
    }
    assert_eq!(
        verify(raw(), b"bad", &fence(&base())).unwrap_err(),
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
    let mut raw = raw().to_vec();
    raw.insert(1, b' ');
    let sidecar = format!("{:x}\n", Sha256::digest(&raw));
    assert!(verify(&raw, sidecar.as_bytes(), &fence(&base())).is_err());
}
#[test]
fn source_refuses_missing_file_or_mismatched_expected_sha256() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("catalog.json");
    let mut source = Source::local(file.clone(), fence(&base()), FIXTURE.sha256());
    assert_eq!(source.read().unwrap_err(), Error::Unavailable);
    std::fs::write(&file, raw()).unwrap();
    assert!(source.read().is_ok());
    source.expected_sha256 = "0".repeat(64);
    assert_eq!(source.read().unwrap_err(), Error::IntegrityMismatch);
}

#[test]
fn reference_producer_supported_vector_matches_consumer() {
    let fixture = catalog_fixture::supported();
    assert!(verify(&fixture.bytes, &fixture.sidecar, &fence(&fixture.value)).is_ok());
}

#[test]
fn no_configuration_is_cold_and_partial_configuration_refuses() {
    let absent = || Err(std::env::VarError::NotPresent);
    assert!(
        Source::from_values((0..8).map(|_| absent()).collect())
            .unwrap()
            .is_none()
    );
    let mut partial: Vec<_> = (0..8).map(|_| absent()).collect();
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
        Source::local(path, fence(&base()), FIXTURE.sha256())
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

#[test]
fn complete_supported_capabilities_are_required_and_never_inferred() {
    let unavailable = verify(raw(), sidecar(), &fence(&base())).unwrap();
    for required in [vec![], vec!["item"], vec!["creature"], vec!["future"]] {
        assert_eq!(
            unavailable.require_complete(&required),
            Err(Error::UnsupportedCapability)
        );
    }
    let (raw, sidecar, f) = artifact(supported());
    let candidate = verify(&raw, &sidecar, &f).unwrap();
    assert_eq!(candidate.require_complete(&["creature"]), Ok(()));
    assert_eq!(
        candidate.require_complete(&["creature", "creature"]),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        candidate.require_complete(&["creature", "item"]),
        Err(Error::UnsupportedCapability)
    );
    for state in ["partial", "unknown"] {
        let mut value = supported();
        value["completeness_manifest"][1]["state"] = state.into();
        let (raw, sidecar, f) = artifact(value);
        let candidate = verify(&raw, &sidecar, &f).unwrap();
        assert_eq!(
            candidate.require_complete(&["creature"]),
            Err(Error::UnsupportedCapability)
        );
    }
}

#[test]
fn artifact_urls_require_https_except_loopback_and_never_allow_credentials() {
    for url in [
        "http://example.test/catalog",
        "file:///tmp/catalog",
        "https://user:pass@example.test/catalog",
        "https://example.test/catalog#fragment",
    ] {
        assert!(transport::validate(&url::Url::parse(url).unwrap()).is_err());
    }
    for url in [
        "https://example.test/catalog",
        "http://127.0.0.1:1234/catalog",
    ] {
        assert!(transport::validate(&url::Url::parse(url).unwrap()).is_ok());
    }
}
#[tokio::test]
async fn configured_url_reads_are_bounded_pinned_and_do_not_follow_redirects() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn serve(response: Vec<u8>) -> (url::Url, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            let _ = socket.write_all(&response).await;
        });
        (
            url::Url::parse(&format!("http://{address}/catalog")).unwrap(),
            task,
        )
    }
    let mut success = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        raw().len()
    )
    .into_bytes();
    success.extend_from_slice(raw());
    let cases=[(success.clone(),None),(success.clone(),Some(Error::IntegrityMismatch)),(success,Some(Error::ProvenanceConflict)),(b"HTTP/1.1 302 Found\r\nLocation: /other\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),Some(Error::Unavailable)),(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",MAX_FILE_BYTES+1).into_bytes(),Some(Error::CapacityExceeded))];
    for (index, (response, expected)) in cases.into_iter().enumerate() {
        let (url, server) = serve(response).await;
        let mut source = Source::local(PathBuf::new(), fence(&base()), FIXTURE.sha256());
        source.url = Some(url);
        if index == 1 {
            source.expected_sha256 = "0".repeat(64);
        }
        if index == 2 {
            source.fence.source_revision = "b".repeat(40);
        }
        let result = source.fetch().await;
        assert_eq!(result.err(), expected);
        server.await.unwrap();
    }
}

#[test]
fn g2_visibility_rejects_the_whole_artifact_without_inventing_absence() {
    // The lane exports no hidden identities. Absence is its ordinary wire form.
    for field in [None, Some(serde_json::json!("public"))] {
        for path in ["record", "data", "nested"] {
            let mut value = supported();
            if let Some(field) = &field {
                match path {
                    "record" => value["entities"][0]["visibility"] = field.clone(),
                    "data" => value["entities"][0]["data"]["visibility"] = field.clone(),
                    _ => {
                        value["entities"][0]["data"]["nested"] =
                            serde_json::json!({"visibility":field})
                    }
                }
            }
            let (raw, sidecar, fence) = artifact(value);
            assert!(verify(&raw, &sidecar, &fence).is_ok(), "{path}");
        }
    }
    for bad in [
        serde_json::json!("hidden"),
        serde_json::json!("other"),
        serde_json::json!(true),
        serde_json::json!(12),
        serde_json::Value::Null,
        serde_json::json!([]),
    ] {
        for path in ["record", "data", "nested"] {
            let mut value = supported();
            match path {
                "record" => value["entities"][0]["visibility"] = bad.clone(),
                "data" => value["entities"][0]["data"]["visibility"] = bad.clone(),
                _ => value["entities"][0]["data"]["nested"] = serde_json::json!({"visibility":bad}),
            }
            let (raw, sidecar, fence) = artifact(value);
            assert_eq!(
                verify(&raw, &sidecar, &fence).err(),
                Some(Error::InvalidInput),
                "{path} {bad}"
            );
        }
    }
}

#[test]
fn g2_also_fences_relation_and_tombstone_record_visibility() {
    for collection in ["relations", "tombstones"] {
        for visibility in [
            None,
            Some(serde_json::json!("public")),
            Some(serde_json::json!("hidden")),
            Some(serde_json::json!("private")),
            Some(serde_json::json!(false)),
            Some(serde_json::Value::Null),
        ] {
            let mut value = supported();
            value["relations"] = serde_json::json!([{"type":"drop","relation_key":"oteryn:relation.synthetic","capability_id":"creature","source":"oteryn:creature.synthetic","target":null,"data":{}}]);
            value["tombstones"] = serde_json::json!([{"content_key":"oteryn:creature.removed","capability_id":"creature","reason":"removed"}]);
            if let Some(v) = &visibility {
                value[collection][0]["visibility"] = v.clone();
            }
            let (raw, sidecar, fence) = artifact(value);
            assert_eq!(
                verify(&raw, &sidecar, &fence).is_ok(),
                visibility
                    .as_ref()
                    .is_none_or(|v| v.as_str() == Some("public")),
                "{collection}: {visibility:?}"
            );
        }
    }
}

#[test]
fn loot_publication_key_collision_rejects_the_whole_artifact_before_persistence() {
    let mut value = base();
    value["capability_manifest"][2]["support"] = "supported".into();
    value["completeness_manifest"][2]["state"] = "complete".into();
    value["entities"] = serde_json::json!([{"type":"loot","content_key":"oteryn:loot.synthetic","capability_id":"creature_loot","data":{"name":"Synthetic loot"}}]);
    value["relations"] = serde_json::json!([{"type":"drop","relation_key":"oteryn:loot.synthetic","capability_id":"creature_loot","source":"oteryn:loot.synthetic","target":null,"data":{"name":"Synthetic drop"}}]);
    let (raw, sidecar, fence) = artifact(value.clone());
    assert_eq!(
        verify(&raw, &sidecar, &fence).err(),
        Some(Error::PublicationKeyCollision)
    );
    assert!(
        Error::PublicationKeyCollision
            .to_string()
            .contains("loot entity/relation publication key collision")
    );
    // Equal keys in different published kinds do not collide on (version,kind,key).
    value["entities"][0]["capability_id"] = "creature".into();
    value["capability_manifest"][1]["support"] = "supported".into();
    value["completeness_manifest"][1]["state"] = "complete".into();
    let (raw, sidecar, fence) = artifact(value);
    assert!(verify(&raw, &sidecar, &fence).is_ok());
}
