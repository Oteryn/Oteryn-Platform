//! TEST ONLY. Source input is pinned; supported data below is synthetic and has no
//! native publication authority. Expected Game-producer hashes prevent wire drift.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn canonical(value: &Value) -> Vec<u8> {
    fn write(value: &Value, out: &mut Vec<u8>) {
        match value {
            Value::Object(map) => {
                out.push(b'{');
                for (i, (key, value)) in map.iter().collect::<BTreeMap<_, _>>().iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    out.extend(serde_json::to_vec(key).unwrap());
                    out.push(b':');
                    write(value, out);
                }
                out.push(b'}');
            }
            Value::Array(items) => {
                out.push(b'[');
                for (i, value) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    write(value, out);
                }
                out.push(b']');
            }
            _ => out.extend(serde_json::to_vec(value).unwrap()),
        }
    }
    let mut out = Vec::new();
    write(value, &mut out);
    out
}
pub struct Fixture {
    pub bytes: Vec<u8>,
    pub sidecar: Vec<u8>,
    pub value: Value,
}
impl Fixture {
    pub fn text(&self, key: &str) -> String {
        self.value[key].as_str().unwrap().into()
    }
    pub fn sha256(&self) -> String {
        format!("{:x}", Sha256::digest(&self.bytes))
    }
}
pub fn emit(mut source: Value) -> Fixture {
    source["contract_id"] = "oteryn.game-platform-catalog".into();
    source["schema_version"] = "1.0.0".into();
    source["content_authority_id"] = "oteryn-native".into();
    let digest = format!("sha256:{:x}", Sha256::digest(canonical(&source)));
    source["snapshot_id"] = digest.clone().into();
    source["payload_digest"] = digest.into();
    let mut bytes = canonical(&source);
    bytes.push(b'\n');
    let sidecar = format!("{:x}\n", Sha256::digest(&bytes)).into_bytes();
    Fixture {
        bytes,
        sidecar,
        value: source,
    }
}
pub fn negative() -> Fixture {
    let fixture = emit(
        serde_json::from_str(include_str!(
            "../fixtures/catalog-d34807f5/unsupported-native-input.json"
        ))
        .unwrap(),
    );
    assert_eq!(
        fixture.sha256(),
        "4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d"
    );
    fixture
}
pub fn supported_source() -> Value {
    // Exact test_producer.py::valid_source() at the evidence pin, not a native adapter.
    json!({"authority_epoch":"native-epoch-1","source_revision":"a".repeat(40),"generated_at":"2026-08-22T17:55:00Z","ruleset_id":"oteryn:ruleset.reference_v1","content_profile_id":"oteryn:profile.reference_v1","required_capabilities":["item"],"capability_manifest":[{"capability_id":"item","support":"supported"},{"capability_id":"npc","support":"unsupported"}],"completeness_manifest":[{"capability_id":"item","state":"complete"},{"capability_id":"npc","state":"unknown"}],"entities":[{"type":"item","content_key":"oteryn:item.training_sword","capability_id":"item","data":{"name":"Training Sword","attack":7}}],"relations":[],"tombstones":[]})
}
pub fn supported() -> Fixture {
    let fixture = emit(supported_source());
    assert_eq!(
        fixture.sha256(),
        "f4780e98ecf0aa7b74cbd759d1a698751920273bc3ec9ccfbe9cffe51de878d7"
    );
    fixture
}
pub fn control_character() -> Fixture {
    let mut source = supported_source();
    source["entities"][0]["data"]["name"] = "Control\0Name".into();
    let fixture = emit(source);
    assert_eq!(
        fixture.sha256(),
        "e03a37dd0ca0cb7741d6924360b790f41910ff0d65eccf174debb5e598d99829"
    );
    fixture
}
