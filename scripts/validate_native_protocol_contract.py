from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
PROTO = ROOT / "docs/contracts/oteryn_native_gameplay_v1.proto"
CONTRACT = ROOT / "docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md"
FIXTURES = ROOT / "docs/contracts/fixtures/oteryn-native-v1"

def fail(message: str) -> None:
    raise SystemExit(message)

def walk(value, path="$"):
    if isinstance(value, dict):
        for key, item in value.items():
            if key in {"profile", "gameplay_profile", "transport_profile"}:
                fail(f"forbidden serialized profile key at {path}.{key}")
            walk(item, f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            walk(item, f"{path}[{index}]")

proto = PROTO.read_text(encoding="utf-8")
if "oteryn.native.v1" in proto:
    fail("legacy native profile value in IDL")
if re.search(r"^\s*(?:string|uint32)\s+(?:gameplay_profile|transport_profile)\s*=", proto, re.M):
    fail("active profile field in IDL")
for required in ["reserved 3;", "reserved \"gameplay_profile\";", "reserved 7;", "native_protocol_version = 17;", "native_protocol_version = 16;"]:
    if required not in proto:
        fail(f"missing IDL compatibility marker: {required}")
sha = hashlib.sha256(PROTO.read_bytes()).hexdigest()
contract = CONTRACT.read_text(encoding="utf-8")
if f"Canonical schema SHA-256: `{sha}`" not in contract:
    fail("canonical contract schema digest mismatch")
if re.search(r"\"(?:profile|gameplay_profile|transport_profile)\"\s*:", contract):
    fail("serialized profile key in canonical contract")
if "oteryn.native.v1" in contract:
    fail("legacy native profile value in canonical contract")
for filename in ["gateway-offer.json", "gateway-selection.json", "manifest.json"]:
    data = json.loads((FIXTURES / filename).read_text(encoding="utf-8"))
    walk(data)
manifest = json.loads((FIXTURES / "manifest.json").read_text(encoding="utf-8"))
identity = manifest["native_identity"]
if identity != {"family": "oteryn", "native_protocol_version": 1, "transport": "tcp.tls13.protobuf.be32.v1"}:
    fail("native identity is not the single authorized tuple")
if manifest["schema_revision"] != 2 or manifest["schema_sha256"] != sha:
    fail("fixture manifest schema provenance mismatch")
if manifest["production_enabled"] is not False:
    fail("contract fixtures must keep production disabled")
print(f"native contract valid: schema_sha256={sha}")
