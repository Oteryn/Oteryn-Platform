#![allow(clippy::unwrap_used, clippy::panic)]
//! #1937: create intent v2 against `docs/contracts/fixtures/character-create-v2`,
//! byte for byte through the real serializer and validator.
use super::*;
use oteryn_internal_api::canonical::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};

/// Agreed with the Game server lane (2026-11-17): v1 members plus
/// `requested_sex`, `requested_vocation` right after `requested_name`.
const AGREED: [&str; 15] = [
    "contract_version",
    "command",
    "variant",
    "issuer_authority",
    "issuer_decision_id",
    "source_revision",
    "operation_id",
    "account_id",
    "target_world_id",
    "requested_name",
    "requested_sex",
    "requested_vocation",
    "issued_at_source",
    "expires_at_source",
    "audience",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/character-create-v2")
}
fn manifest() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()
}
fn keys(v: &Value) -> Vec<String> {
    let Value::Object(fields) = v else {
        panic!("object");
    };
    fields.iter().map(|(k, _)| k.clone()).collect()
}
/// Rebuild the intent from a vector's members and serialize it again.
fn reserialize(v: &Value) -> String {
    let id = |k| Uuid::parse_str(wire::get(v, k).unwrap()).unwrap();
    let number = |k| wire::get(v, k).unwrap().parse::<i64>().unwrap();
    let intent = Intent::Create {
        account_id: id("account_id"),
        world_id: id("target_world_id"),
        name: wire::get(v, "requested_name").unwrap().into(),
        sex: Sex::parse(wire::get(v, "requested_sex").unwrap()).unwrap(),
        vocation: Vocation::parse(wire::get(v, "requested_vocation").unwrap()).unwrap(),
    };
    intent.validate().unwrap();
    wire::intent(
        intent.command(),
        id("issuer_decision_id"),
        number("source_revision"),
        id("operation_id"),
        wire::binding(&intent),
        number("issued_at_source"),
        number("expires_at_source"),
    )
    .unwrap()
}

#[test]
fn create_v2_member_order_is_the_agreed_list() {
    assert_eq!(wire::CREATE_V2_MEMBERS, AGREED);
    let manifest = manifest();
    let listed: Vec<&str> = manifest["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert_eq!(listed, AGREED);
    for vocation in Vocation::ALL {
        for sex in Sex::ALL {
            let intent = Intent::Create {
                account_id: Uuid::now_v7(),
                world_id: Uuid::now_v7(),
                name: "Aldric Vale".into(),
                sex,
                vocation,
            };
            let raw = wire::intent(
                CREATE,
                Uuid::now_v7(),
                1,
                Uuid::now_v7(),
                wire::binding(&intent),
                1790000000,
                1790000300,
            )
            .unwrap();
            let v = wire::create_intent(raw.as_bytes()).unwrap();
            assert_eq!(keys(&v), AGREED);
            assert_eq!(v.get("contract_version"), Some(&Value::Unsigned(2)));
            assert_eq!(wire::get(&v, "requested_sex").unwrap(), sex.wire());
            assert_eq!(
                wire::get(&v, "requested_vocation").unwrap(),
                vocation.wire()
            );
        }
    }
}

#[test]
fn create_v2_fixtures_match_serializer_and_validator() {
    let manifest = manifest();
    let mut listed = BTreeSet::new();
    for entry in manifest["files"].as_array().unwrap() {
        let name = entry["file"].as_str().unwrap();
        let bytes = std::fs::read(root().join(name)).unwrap();
        assert_eq!(
            bytes.len() as u64,
            entry["bytes"].as_u64().unwrap(),
            "{name}"
        );
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, entry["sha256"].as_str().unwrap(), "{name}");
        assert!(!bytes.ends_with(b"\n"), "{name} has a trailing newline");
        listed.insert(name.to_owned());
    }
    let on_disk: BTreeSet<String> = std::fs::read_dir(root())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != "manifest.json" && n != "README.md")
        .collect();
    assert_eq!(listed, on_disk, "every vector is listed once");
    let mut accepted = BTreeSet::new();
    let mut rejected = 0;
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["file"].as_str().unwrap();
        assert!(listed.contains(name), "{name}");
        let raw = std::fs::read(root().join(name)).unwrap();
        match case["outcome"].as_str().unwrap() {
            "accept" => {
                let v = wire::create_intent(&raw).unwrap();
                assert_eq!(reserialize(&v).as_bytes(), raw.as_slice(), "{name}");
                for key in ["requested_sex", "requested_vocation"] {
                    assert_eq!(wire::get(&v, key).unwrap(), case[key].as_str().unwrap());
                }
                accepted.insert(wire::get(&v, "requested_vocation").unwrap().to_owned());
            }
            "reject" => {
                assert!(wire::create_intent(&raw).is_err(), "{name}");
                rejected += 1;
            }
            other => panic!("{name}: outcome {other}"),
        }
    }
    let every: BTreeSet<String> = Vocation::ALL.map(|v| v.wire().to_owned()).into();
    assert_eq!(accepted, every, "one accepted vector per vocation");
    assert_eq!(rejected, 4);
}

#[test]
fn create_v2_values_are_exact_and_v1_create_is_refused() {
    for raw in ["Female", "Male"] {
        assert_eq!(Sex::parse(raw).unwrap().wire(), raw);
    }
    for raw in ["Knight", "Paladin", "Sorcerer", "Druid", "Monk"] {
        assert_eq!(Vocation::parse(raw).unwrap().wire(), raw);
    }
    for raw in ["", "female", "MALE", " Male", "Male ", "0", "1", "Other"] {
        assert!(Sex::parse(raw).is_err(), "{raw:?}");
    }
    for raw in ["", "knight", "KNIGHT", "Knight ", "0", "1", "None", "Mage"] {
        assert!(Vocation::parse(raw).is_err(), "{raw:?}");
    }
    // The pinned Game v1 golden is no longer a current create intent.
    assert!(wire::create_intent(include_bytes!("fixtures/create-intent.json")).is_err());
    let knight = std::fs::read_to_string(root().join("create-intent-knight.json")).unwrap();
    for bad in [
        knight.replace("\"contract_version\":2", "\"contract_version\":1"),
        knight.replace("\"contract_version\":2", "\"contract_version\":3"),
        knight.replace(
            "\"requested_sex\":\"Male\",\"requested_vocation\":\"Knight\"",
            "\"requested_vocation\":\"Knight\",\"requested_sex\":\"Male\"",
        ),
        knight.replace(
            "\"requested_vocation\":\"Knight\"",
            "\"requested_vocation\":1",
        ),
        knight.replace(",\"audience\"", ",\"extra\":\"x\",\"audience\""),
        format!("{knight}\n"),
    ] {
        assert!(wire::create_intent(bad.as_bytes()).is_err(), "{bad}");
    }
}
