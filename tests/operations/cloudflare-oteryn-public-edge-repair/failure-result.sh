#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/operations/cloudflare-oteryn-public-edge-failure.py"
WORKFLOW="$ROOT/.github/workflows/cloudflare-oteryn-public-edge-repair.yml"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

python3 -m py_compile "$SCRIPT"

cat >"$TMP/waf.txt" <<'EOF'
ERROR: apply failed and partial changes were rolled back: Cloudflare POST /zones/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/rulesets/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/rules failed: HTTP 403; codes=9109
Bearer secret-value
ip.geoip.country ne "PL"
EOF
python3 "$SCRIPT" "$TMP/waf.txt" >"$TMP/waf.result"
grep -Fx 'failure_phase=create_waf_skip_rule' "$TMP/waf.result" >/dev/null
grep -Fx 'error_class=cloudflare_http_error' "$TMP/waf.result" >/dev/null
grep -Fx 'http_status=403' "$TMP/waf.result" >/dev/null
grep -Fx 'error_codes=9109' "$TMP/waf.result" >/dev/null
grep -Fx 'rollback_claim=reported_complete' "$TMP/waf.result" >/dev/null

cat >"$TMP/bot.txt" <<'EOF'
ERROR: apply failed and partial changes were rolled back: Cloudflare PUT /zones/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/bot_management failed: HTTP 403; codes=10000,9109
EOF
python3 "$SCRIPT" "$TMP/bot.txt" >"$TMP/bot.result"
grep -Fx 'failure_phase=update_bot_fight_mode' "$TMP/bot.result" >/dev/null
grep -Fx 'error_codes=10000,9109' "$TMP/bot.result" >/dev/null

printf '%s\n' 'ERROR: broad country block expression hash does not match the audited rule' >"$TMP/drift.txt"
python3 "$SCRIPT" "$TMP/drift.txt" >"$TMP/drift.result"
grep -Fx 'failure_phase=preflight' "$TMP/drift.result" >/dev/null
grep -Fx 'error_class=audited_rule_drift' "$TMP/drift.result" >/dev/null

printf '%s\n' 'completely unknown secret-value failure' >"$TMP/unknown.txt"
python3 "$SCRIPT" "$TMP/unknown.txt" >"$TMP/unknown.result"
grep -Fx 'error_class=unclassified_failure' "$TMP/unknown.result" >/dev/null

if grep -R -E 'secret-value|ip\.geoip\.country|"PL"|/zones/[0-9a-f]{32}' "$TMP"/*.result; then
  echo 'failure sanitizer leaked raw content' >&2
  exit 1
fi

grep -F 'python3 scripts/operations/cloudflare-oteryn-public-edge-failure.py "$raw_output"' "$WORKFLOW" >/dev/null
grep -F 'Audit state after operation failure' "$WORKFLOW" >/dev/null
grep -F 'post-failure-audit-result.txt' "$WORKFLOW" >/dev/null
if grep -F 'cat "$raw_output"' "$WORKFLOW"; then
  echo 'workflow publishes raw operation output' >&2
  exit 1
fi

echo 'Cloudflare repair failure sanitizer tests passed.'
