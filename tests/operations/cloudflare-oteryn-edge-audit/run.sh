#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
tmp="$(mktemp -d)"
trap 'kill "${server_pid:-}" 2>/dev/null || true; rm -rf "$tmp"' EXIT
python3 "$root/tests/operations/cloudflare-oteryn-edge-audit/mock_server.py" &
server_pid=$!
for _ in $(seq 1 30); do
  if python3 - <<'PY'
import urllib.request
try:
    urllib.request.urlopen("http://127.0.0.1:18080/health", timeout=.2)
except Exception as e:
    if getattr(e, "code", None) == 404:
        raise SystemExit(0)
raise SystemExit(1)
PY
  then break; fi
  sleep .1
done
export CLOUDFLARE_API_BASE_URL="http://127.0.0.1:18080"
export CLOUDFLARE_API_TOKEN="cfat_test"
export CLOUDFLARE_ACCESS_API_TOKEN="cfat_access_test"
export CLOUDFLARE_ACCOUNT_ID="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
export CLOUDFLARE_ZONE_ID="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
export CLOUDFLARE_EDGE_AUDIT_OUT="$tmp/out"
python3 "$root/scripts/operations/cloudflare-oteryn-edge-audit.py"
python3 "$root/scripts/operations/cloudflare-token-capability-audit.py"
python3 - "$tmp/out/evidence.json" "$tmp/out/token-capability.json" <<'PY'
import json, sys
edge=json.load(open(sys.argv[1]))
token=json.load(open(sys.argv[2]))
assert edge["mutation"] == "none"
assert edge["token"]["access_token_separate"] is True
assert edge["certificate_packs"]["active_exact_login_coverage"] is True
assert edge["certificate_packs"]["pack_summaries"][0]["covers_www"] is True
assert edge["certificate_packs"]["pack_summaries"][0]["covers_login"] is True
assert edge["zone_settings"]["security_level"]["value"] == "under_attack"
redirect=edge["ruleset_details"][0]["oteryn_matching_rules"][0]
assert redirect["matches_www"] is True
assert "expression" not in redirect
waf=edge["ruleset_details"][1]
assert len(waf["sanitized_rules"]) == 3
assert waf["sanitized_rules"][0]["host_scope"] == "broad_no_host_predicate"
assert waf["sanitized_rules"][0]["expression_sha256"]
assert len(waf["oteryn_candidate_rules"]) == 1
assert waf["oteryn_candidate_rules"][0]["ref"] == "broad-bot-challenge"
assert edge["bot_management"]["settings"]["fight_mode"] is True
assert edge["access_applications"]["oteryn_applications"][0]["domain"] == "oteryn.molehill.cloud"
assert token["mutation"] == "none"
assert token["self_details"]["state"] == "readable"
assert token["self_details"]["has_account_api_tokens_read"] is True
assert token["self_details"]["has_account_api_tokens_write"] is False
assert token["permission_group_catalog"]["state"] == "readable"
PY
python3 - "$root/scripts/operations/cloudflare-oteryn-edge-audit.py" "$root/scripts/operations/cloudflare-token-capability-audit.py" <<'PY'
import pathlib, sys
for path in sys.argv[1:]:
    source=pathlib.Path(path).read_text()
    for method in ("POST","PUT","PATCH","DELETE"):
        assert f'method="{method}"' not in source
        assert f"method='{method}'" not in source
PY
python3 -m py_compile \
  "$root/scripts/operations/cloudflare-oteryn-edge-audit.py" \
  "$root/scripts/operations/cloudflare-token-capability-audit.py" \
  "$root/tests/operations/cloudflare-oteryn-edge-audit/mock_server.py"
echo "Cloudflare edge and token capability audit tests passed."
