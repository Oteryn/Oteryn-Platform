#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/operations/cloudflare-oteryn-hsts-stage1.py"
MOCK="$ROOT/tests/operations/cloudflare-oteryn-hsts-stage1/mock_cloudflare.py"
WORKFLOW="$ROOT/.github/workflows/cloudflare-oteryn-hsts-stage1.yml"
MARKER="$ROOT/ops/triggers/cloudflare-oteryn-hsts-stage1.md"
TMP="$(mktemp -d)"
PIDS=()
trap 'for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done; rm -rf "$TMP"' EXIT

fail() { echo "TEST FAILURE: $*" >&2; exit 1; }
python3 -m py_compile "$SCRIPT" "$MOCK"

start_server() {
  local scenario="$1" name="$2"
  local port_file="$TMP/$name.port" log_file="$TMP/$name.log"
  MOCK_SCENARIO="$scenario" MOCK_PORT_FILE="$port_file" MOCK_LOG_FILE="$log_file" \
    python3 "$MOCK" >"$TMP/$name.stdout" 2>"$TMP/$name.stderr" &
  local pid=$!
  PIDS+=("$pid")
  for _ in $(seq 1 50); do [[ -s "$port_file" ]] && break; sleep 0.1; done
  [[ -s "$port_file" ]] || fail "mock server $name did not start"
  printf '%s;%s;%s\n' "$pid" "$(cat "$port_file")" "$log_file"
}

run_hsts() {
  local port="$1" out="$2" mode="$3" confirmation="${4:-}"
  CLOUDFLARE_API_BASE_URL="http://127.0.0.1:$port/client/v4" \
  CLOUDFLARE_API_TOKEN='mock-hsts-token-secret' \
  CLOUDFLARE_ZONE_ID='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
  CLOUDFLARE_HSTS_STAGE1_OUT="$out" \
  CLOUDFLARE_HSTS_CONFIRMATION="$confirmation" \
    python3 "$SCRIPT" "$mode"
}

IFS=';' read -r normal_pid normal_port normal_log < <(start_server normal normal)
run_hsts "$normal_port" "$TMP/audit" audit >"$TMP/audit.out"
jq -e '
  .operation_status == "success"
  and .mode == "audit"
  and .state == "baseline"
  and .max_age == 0
  and .include_subdomains == true
  and .preload == true
  and .mutation == "none"
  and .secrets_emitted == false
' "$TMP/audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method != "GET") | .method' "$normal_log" | wc -l)" == "0" ]] || fail "audit mutated Cloudflare"

if run_hsts "$normal_port" "$TMP/bad" apply BAD >"$TMP/bad.out" 2>"$TMP/bad.err"; then
  fail "invalid confirmation unexpectedly succeeded"
fi
[[ "$(jq -r 'select(.method != "GET") | .method' "$normal_log" | wc -l)" == "0" ]] || fail "invalid confirmation mutated Cloudflare"

run_hsts "$normal_port" "$TMP/apply" apply APPLY-OTERYN-HSTS-STAGE1 >"$TMP/apply.out"
jq -e '
  .state == "staged"
  and .desired_state == true
  and .enabled == true
  and .max_age == 2592000
  and .include_subdomains == false
  and .preload == false
  and .nosniff == true
  and .mutations == ["hsts_stage1_enabled"]
' "$TMP/apply/evidence.json" >/dev/null
python3 - "$normal_log" <<'PY'
import json, sys
rows=[json.loads(line) for line in open(sys.argv[1])]
patch=[row for row in rows if row["method"] == "PATCH"]
assert len(patch) == 1
assert patch[0]["body"] == {"value": {"strict_transport_security": {
    "enabled": True,
    "include_subdomains": False,
    "max_age": 2592000,
    "nosniff": True,
    "preload": False,
}}}
PY

run_hsts "$normal_port" "$TMP/idempotent" apply APPLY-OTERYN-HSTS-STAGE1 >"$TMP/idempotent.out"
jq -e '.state == "staged" and .mutation == "none"' "$TMP/idempotent/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$normal_log" | wc -l)" == "1" ]] || fail "idempotent apply patched again"

run_hsts "$normal_port" "$TMP/rollback" rollback ROLLBACK-OTERYN-HSTS-STAGE1 >"$TMP/rollback.out"
jq -e '
  .state == "baseline"
  and .max_age == 0
  and .include_subdomains == true
  and .preload == true
  and .mutations == ["hsts_stage1_rolled_back"]
' "$TMP/rollback/evidence.json" >/dev/null

IFS=';' read -r staged_pid staged_port staged_log < <(start_server staged staged)
run_hsts "$staged_port" "$TMP/already-staged" apply APPLY-OTERYN-HSTS-STAGE1 >"$TMP/already-staged.out"
jq -e '.state == "staged" and .mutation == "none"' "$TMP/already-staged/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$staged_log" | wc -l)" == "0" ]] || fail "already-staged state was patched"

IFS=';' read -r malformed_pid malformed_port malformed_log < <(start_server malformed_after_patch malformed)
if run_hsts "$malformed_port" "$TMP/malformed" apply APPLY-OTERYN-HSTS-STAGE1 >"$TMP/malformed.out" 2>"$TMP/malformed.err"; then
  fail "malformed accepted PATCH unexpectedly succeeded"
fi
grep -F 'rollback completed (baseline_restored)' "$TMP/malformed.err" >/dev/null
run_hsts "$malformed_port" "$TMP/malformed-audit" audit >"$TMP/malformed-audit.out"
jq -e '.state == "baseline" and .max_age == 0' "$TMP/malformed-audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$malformed_log" | wc -l)" == "2" ]] || fail "malformed accepted PATCH was not rolled back"

IFS=';' read -r deny_pid deny_port deny_log < <(start_server deny deny)
if run_hsts "$deny_port" "$TMP/deny" apply APPLY-OTERYN-HSTS-STAGE1 >"$TMP/deny.out" 2>"$TMP/deny.err"; then
  fail "permission denial unexpectedly succeeded"
fi
grep -F 'HTTP 403; codes=10000' "$TMP/deny.err" >/dev/null
run_hsts "$deny_port" "$TMP/deny-audit" audit >"$TMP/deny-audit.out"
jq -e '.state == "baseline" and .mutation == "none"' "$TMP/deny-audit/evidence.json" >/dev/null

for secret in mock-hsts-token-secret; do
  if grep -R -F "$secret" "$TMP" --exclude='*.log' --exclude='*.stderr' --exclude='*.stdout'; then
    fail "token leaked into output"
  fi
done

python3 - "$WORKFLOW" "$MARKER" <<'PY'
from pathlib import Path
import sys
workflow=Path(sys.argv[1]).read_text(encoding="utf-8")
marker=Path(sys.argv[2]).read_text(encoding="utf-8")
required=[
    "pull_request:", "push:", "branches:\n      - main",
    "ops/triggers/cloudflare-oteryn-hsts-stage1.md",
    "if: github.event_name == 'push'", "environment: production-cloudflare",
    "CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_EDGE_AUDIT_TOKEN }}",
    "ref: ${{ github.sha }}", "APPLY-OTERYN-HSTS-STAGE1", "ROLLBACK-OTERYN-HSTS-STAGE1",
    "An operational HSTS PR may change only the HSTS marker.",
    "python3 scripts/operations/cloudflare-oteryn-hsts-stage1.py",
    "python3 scripts/operations/oteryn-public-edge-validation.py",
    "positive_hsts_www", "ISSUE_NUMBER: '91'", "issues: write",
    "Only bounded HSTS and public-validation fields are published.",
]
for item in required:
    if item not in workflow:
        raise SystemExit(f"missing workflow invariant: {item}")
allowed={
    "# Cloudflare Oteryn HSTS stage 1 trigger\n\nmode: inert\nconfirmation:\n",
    "# Cloudflare Oteryn HSTS stage 1 trigger\n\nmode: audit\nconfirmation:\n",
    "# Cloudflare Oteryn HSTS stage 1 trigger\n\nmode: apply\nconfirmation: APPLY-OTERYN-HSTS-STAGE1\n",
    "# Cloudflare Oteryn HSTS stage 1 trigger\n\nmode: rollback\nconfirmation: ROLLBACK-OTERYN-HSTS-STAGE1\n",
}
if marker not in allowed:
    raise SystemExit("committed HSTS marker is not exact")
for forbidden in ('cat "$raw_output"', 'body="$raw_output"', 'cat cloudflare-hsts-stage1/evidence.json', 'cat public-edge-validation/evidence.json'):
    if forbidden in workflow:
        raise SystemExit(f"raw output could reach comments: {forbidden}")
PY

echo "Cloudflare HSTS stage-1 tests passed."
