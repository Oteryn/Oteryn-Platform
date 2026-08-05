from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
PROTO = ROOT / "docs/contracts/oteryn_native_gameplay_v1.proto"
CONTRACT = ROOT / "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md"
FIXTURES = ROOT / "docs/contracts/fixtures/oteryn-native-v1"
CURRENT_DOCS = [
    CONTRACT,
    ROOT / "docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md",
    ROOT / "docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md",
    ROOT / "docs/architecture/adr/0010-native-gameplay-protocol-selection.md",
    ROOT / "docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md",
]

def fail(message: str) -> None:
    raise SystemExit(message)

def walk(value, path="$" ) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if "profile" in key.lower():
                fail(f"forbidden serialized profile-shaped key at {path}.{key}")
            walk(item, f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            walk(item, f"{path}[{index}]")

proto = PROTO.read_text(encoding="utf-8")
if "oteryn.native.v1" in proto:
    fail("legacy native profile value in IDL")
if re.search(r"^\s*(?:string|uint32)\s+(?:gameplay_profile|transport_profile)\s*=", proto, re.M):
    fail("active profile field in IDL")
for required in [
    "reserved 3;", 'reserved "gameplay_profile";', "reserved 7;",
    "native_protocol_version = 17;", "native_protocol_version = 16;",
    "string family = 2;", "string family = 6;", "string transport = 4;", "string transport = 8;",
]:
    if required not in proto:
        fail(f"missing IDL compatibility marker: {required}")
if proto.count("native_protocol_version =") != 2:
    fail("IDL must expose native_protocol_version exactly in ClientHello and ServerHello")

schema_sha = hashlib.sha256(PROTO.read_bytes()).hexdigest()
for document in CURRENT_DOCS:
    text = document.read_text(encoding="utf-8")
    if "oteryn.native.v1" in text:
        fail(f"legacy native profile value in current source: {document}")
contract = CONTRACT.read_text(encoding="utf-8")
if f"Canonical schema SHA-256: `{schema_sha}`" not in contract:
    fail("canonical contract schema digest mismatch")
if re.search(r'"(?:profile|gameplay_profile|transport_profile)"\s*:', contract):
    fail("serialized profile key in canonical contract")
for required in [
    "At most one candidate may have `family = oteryn`",
    "its `native_protocol_version` is exactly `1`",
    "It contains no `profile` key, alias or placeholder",
    "The current contract has no placeholder for that future work",
]:
    if required not in contract:
        fail(f"missing single-version contract invariant: {required}")

parsed = {}
for filename in ["gateway-offer.json", "gateway-selection.json", "manifest.json"]:
    data = json.loads((FIXTURES / filename).read_text(encoding="utf-8"))
    walk(data)
    parsed[filename] = data
offer = parsed["gateway-offer.json"]["gameplay_offer"]["candidates"]
native = [item for item in offer if item.get("family") == "oteryn"]
if len(native) != 1 or native[0].get("native_protocol_version") != 1:
    fail("fixture offer must contain exactly one authorized native descriptor")
selection = parsed["gateway-selection.json"]["gameplay_selection"]
if selection.get("family") != "oteryn" or selection.get("native_protocol_version") != 1:
    fail("fixture selection is not the authorized native tuple")
manifest = parsed["manifest.json"]
identity = manifest["native_identity"]
if identity != {"family": "oteryn", "native_protocol_version": 1, "transport": "tcp.tls13.protobuf.be32.v1"}:
    fail("native identity is not the single authorized tuple")
if manifest["schema_revision"] != 2 or manifest["schema_sha256"] != schema_sha:
    fail("fixture manifest schema provenance mismatch")
if manifest["production_enabled"] is not False:
    fail("contract fixtures must keep production disabled")
for provenance in [
    ROOT / "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md",
    ROOT / "docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md",
]:
    if schema_sha not in provenance.read_text(encoding="utf-8"):
        fail(f"schema digest not pinned in {provenance}")
print(f"native contract valid: schema_sha256={schema_sha}")
