#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

VALIDATOR="$ROOT/scripts/operations/oteryn-public-edge-validation.py"
SUMMARIZER="$ROOT/scripts/operations/oteryn-public-edge-result.py"
FIXTURE="$ROOT/tests/operations/oteryn-public-edge-validation/pass-fixture.json"

python3 -m py_compile "$VALIDATOR" "$SUMMARIZER"

OTERYN_PUBLIC_EDGE_FIXTURE="$FIXTURE" \
OTERYN_PUBLIC_EDGE_OUT="$TMP/pass" \
python3 "$VALIDATOR" >/dev/null

python3 - "$TMP/pass/evidence.json" <<'PY'
import json, sys
item=json.load(open(sys.argv[1]))
assert item["verdict"] == "PASS"
assert item["failed_required_checks"] == []
assert item["acceptance"]["tls_gateway"] is True
assert item["acceptance"]["gateway_invalid_login"] is True
assert item["mutation"] == "none"
PY

python3 - "$FIXTURE" "$TMP/fail.json" <<'PY'
import json, sys
item=json.load(open(sys.argv[1]))
item["requests"]["www_browser_root"]["http_code"] = 403
item["requests"]["www_browser_root"]["body"]["contains_cloudflare_interstitial"] = True
json.dump(item, open(sys.argv[2], "w"), indent=2)
PY

OTERYN_PUBLIC_EDGE_FIXTURE="$TMP/fail.json" \
OTERYN_PUBLIC_EDGE_OUT="$TMP/fail" \
python3 "$VALIDATOR" >/dev/null

python3 - "$TMP/fail/evidence.json" <<'PY'
import json, sys
item=json.load(open(sys.argv[1]))
assert item["verdict"] == "FAIL"
assert "www_browser_public" in item["failed_required_checks"]
PY

python3 - "$VALIDATOR" <<'PY'
import importlib.util, sys
spec=importlib.util.spec_from_file_location("validator", sys.argv[1])
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
headers=module.parse_headers("HTTP/2 200\r\nSet-Cookie: secret=x\r\nCache-Control: private, no-store\r\nServer: cloudflare\r\n\r\n")
assert "set-cookie" not in headers
assert headers["cache-control"] == "private, no-store"
assert module.body_signals(b"<title>Just a moment...</title>")["contains_cloudflare_interstitial"] is True
assert module.body_signals(b'{"service":"oteryn-game-gateway"}')["contains_gateway_identity"] is True
PY

cat >"$TMP/edge.json" <<'JSON'
{
  "observed_at_utc": "2026-08-01T00:00:00+00:00",
  "mutation": "none",
  "certificate_packs": {
    "active_gateway_coverage": true,
    "active_legacy_gateway_coverage": false
  },
  "ruleset_details": [
    {"oteryn_candidate_rules": [{"action": "block"}]}
  ],
  "bot_management": {"settings": {"fight_mode": true, "enable_js": true}},
  "access_applications": {"oteryn_applications": []},
  "zone_settings": {
    "browser_check": {"value": "on"},
    "security_level": {"value": "high"},
    "always_use_https": {"value": "on"},
    "security_header": {"value": {"strict_transport_security": {"max_age": 0}}}
  }
}
JSON

python3 "$SUMMARIZER" "$TMP/edge.json" "$TMP/pass/evidence.json" >"$TMP/result.txt"
grep -Fx 'gateway_certificate_active=true' "$TMP/result.txt" >/dev/null
grep -Fx 'retired_gateway_certificate_active=false' "$TMP/result.txt" >/dev/null
grep -Fx 'waf_candidate_count=1' "$TMP/result.txt" >/dev/null
grep -Fx 'public_verdict=PASS' "$TMP/result.txt" >/dev/null
grep -Fx 'public_mutation=none' "$TMP/result.txt" >/dev/null

for path in "$VALIDATOR" "$SUMMARIZER"; do
  if grep -Eq 'Authorization:|Bearer |CLOUDFLARE_API_TOKEN|Set-Cookie' "$path"; then
    echo "Unexpected credential or cookie collector in $path" >&2
    exit 1
  fi
done

echo "Oteryn bounded public edge validation tests passed."
