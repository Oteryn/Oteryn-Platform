#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/operations/cloudflare-oteryn-public-edge-repair.py"
MOCK="$ROOT/tests/operations/cloudflare-oteryn-public-edge-repair/mock_cloudflare.py"
TMP="$(mktemp -d)"
PIDS=()
trap 'for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done; rm -rf "$TMP"' EXIT

fail() {
  echo "TEST FAILURE: $*" >&2
  exit 1
}

python3 -m py_compile "$SCRIPT" "$MOCK"

start_server() {
  local scenario="$1" name="$2"
  local port_file="$TMP/$name.port" log_file="$TMP/$name.log"
  MOCK_SCENARIO="$scenario" MOCK_PORT_FILE="$port_file" MOCK_LOG_FILE="$log_file" \
    python3 "$MOCK" >"$TMP/$name.stdout" 2>"$TMP/$name.stderr" &
  local pid=$!
  PIDS+=("$pid")
  for _ in $(seq 1 50); do
    [[ -s "$port_file" ]] && break
    sleep 0.1
  done
  [[ -s "$port_file" ]] || fail "mock server $name did not start"
  printf '%s;%s;%s\n' "$pid" "$(cat "$port_file")" "$log_file"
}

run_repair() {
  local port="$1" out="$2" mode="$3" confirmation="${4:-}"
  CLOUDFLARE_API_BASE_URL="http://127.0.0.1:$port/client/v4" \
  CLOUDFLARE_API_TOKEN='mock-edge-token-secret' \
  CLOUDFLARE_ZONE_ID='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
  CLOUDFLARE_EXPECTED_COUNTRY_RULE_ID='e0f91939eb494d4490d975498a9a9724' \
  CLOUDFLARE_EXPECTED_COUNTRY_RULE_SHA256='21883a8c5097ed513d978b6886ade6b8f102d9801ed275ae65142777fbd9e6bc' \
  CLOUDFLARE_PUBLIC_EDGE_REPAIR_OUT="$out" \
  CLOUDFLARE_EDGE_REPAIR_CONFIRMATION="$confirmation" \
    python3 "$SCRIPT" "$mode"
}

IFS=';' read -r normal_pid normal_port normal_log < <(start_server normal normal)
run_repair "$normal_port" "$TMP/audit" audit >"$TMP/audit.out"
jq -e '
  .schema_version == 2
  and .operation_status == "success"
  and .mode == "audit"
  and .candidate_count == 1
  and .repair_state == "absent"
  and .repair_first == false
  and .bot_fight_mode == true
  and .mutation == "none"
  and .secrets_emitted == false
  and .expressions_emitted == false
' "$TMP/audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method != "GET") | .method' "$normal_log" | wc -l)" == "0" ]] \
  || fail "audit used a mutating method"

if run_repair "$normal_port" "$TMP/bad-confirm" apply BAD >"$TMP/bad-confirm.out" 2>"$TMP/bad-confirm.err"; then
  fail "invalid apply confirmation unexpectedly succeeded"
fi
[[ "$(jq -r 'select(.method != "GET") | .method' "$normal_log" | wc -l)" == "0" ]] \
  || fail "invalid confirmation mutated Cloudflare"

if CLOUDFLARE_API_BASE_URL="http://127.0.0.1:$normal_port/client/v4" \
  CLOUDFLARE_API_TOKEN='mock-edge-token-secret' \
  CLOUDFLARE_ZONE_ID='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
  CLOUDFLARE_EXPECTED_COUNTRY_RULE_ID='e0f91939eb494d4490d975498a9a9724' \
  CLOUDFLARE_EXPECTED_COUNTRY_RULE_SHA256='0000000000000000000000000000000000000000000000000000000000000000' \
  CLOUDFLARE_PUBLIC_EDGE_REPAIR_OUT="$TMP/hash-drift" \
  CLOUDFLARE_EDGE_REPAIR_CONFIRMATION='APPLY-OTERYN-PUBLIC-EDGE-REPAIR' \
  python3 "$SCRIPT" apply >"$TMP/hash-drift.out" 2>"$TMP/hash-drift.err"; then
  fail "audited candidate hash drift unexpectedly applied"
fi
grep -F 'expression hash does not match the audited rule' "$TMP/hash-drift.err" >/dev/null
[[ "$(jq -r 'select(.method != "GET") | .method' "$normal_log" | wc -l)" == "0" ]] \
  || fail "candidate hash drift mutated Cloudflare"

run_repair "$normal_port" "$TMP/apply" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/apply.out"
jq -e '
  .desired_state == true
  and .repair_state == "current"
  and .repair_first == true
  and .repair_index == 0
  and .repair_before_candidate == true
  and .bot_fight_mode == false
  and (.mutations | sort) == ["bot_fight_mode_disabled", "waf_skip_rule_created"]
' "$TMP/apply/evidence.json" >/dev/null
python3 - "$normal_log" <<'PY'
import json, sys
rows=[json.loads(line) for line in open(sys.argv[1])]
post=[r for r in rows if r["method"] == "POST"]
assert len(post) == 1
body=post[0]["body"]
assert body["action"] == "skip"
assert body["expression"] == 'http.host in {"oteryn.molehill.cloud" "gateway.molehill.cloud"}'
assert body["action_parameters"] == {"products": ["bic", "securityLevel"], "ruleset": "current"}
assert body["position"] == {"before": ""}
assert body["ref"] == "oteryn-public-edge-canonical-skip-v1"
assert body["description"].endswith("[bot-baseline:on]")
assert not [r for r in rows if r["method"] == "PATCH"]
puts=[r for r in rows if r["method"] == "PUT"]
assert puts[-1]["body"] == {"fight_mode": False}
PY

run_repair "$normal_port" "$TMP/idempotent" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/idempotent.out"
jq -e '.desired_state == true and .repair_first == true and .mutation == "none"' "$TMP/idempotent/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "POST") | .method' "$normal_log" | wc -l)" == "1" ]] \
  || fail "idempotent apply created another rule"

run_repair "$normal_port" "$TMP/rollback" rollback ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/rollback.out"
jq -e '
  .repair_state == "absent"
  and .repair_rule_count == 0
  and .bot_fight_mode == true
  and (.mutations | sort) == ["bot_fight_mode_restored", "waf_skip_rule_deleted"]
' "$TMP/rollback/evidence.json" >/dev/null

IFS=';' read -r partial_pid partial_port partial_log < <(start_server repair_present repair-present)
run_repair "$partial_port" "$TMP/repair-present-audit" audit >"$TMP/repair-present-audit.out"
jq -e '
  .repair_state == "shadowed_by_earlier_rules"
  and .repair_exact == true
  and .repair_first == false
  and .repair_before_candidate == true
  and .desired_state == false
' "$TMP/repair-present-audit/evidence.json" >/dev/null
run_repair "$partial_port" "$TMP/repair-present" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/repair-present.out"
jq -e '
  .desired_state == true
  and .repair_rule_count == 1
  and .repair_state == "current"
  and .repair_first == true
  and .repair_before_candidate == true
  and .bot_fight_mode == false
  and (.mutations | sort) == ["bot_fight_mode_disabled", "waf_skip_rule_moved_first"]
' "$TMP/repair-present/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "POST") | .method' "$partial_log" | wc -l)" == "0" ]] \
  || fail "existing exact repair rule was recreated"
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$partial_log" | wc -l)" == "1" ]] \
  || fail "existing repair rule was not moved exactly once"
[[ "$(jq -r 'select(.method == "PUT") | .method' "$partial_log" | wc -l)" == "1" ]] \
  || fail "existing partial state did not update Bot Fight Mode exactly once"

IFS=';' read -r partial_off_pid partial_off_port partial_off_log < <(start_server repair_present_bot_off repair-present-bot-off)
run_repair "$partial_off_port" "$TMP/repair-present-bot-off" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/repair-present-bot-off.out"
jq -e '
  .desired_state == true
  and .repair_state == "current"
  and .repair_first == true
  and .bot_fight_mode == false
  and .mutations == ["waf_skip_rule_moved_first"]
' "$TMP/repair-present-bot-off/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$partial_off_log" | wc -l)" == "1" ]] \
  || fail "reorder-only state did not issue exactly one PATCH"
[[ "$(jq -r 'select(.method == "PUT") | .method' "$partial_off_log" | wc -l)" == "0" ]] \
  || fail "reorder-only state changed Bot Fight Mode"

IFS=';' read -r partial_fail_pid partial_fail_port partial_fail_log < <(start_server repair_present_bot_fail repair-present-bot-fail)
if run_repair "$partial_fail_port" "$TMP/repair-present-bot-fail" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/repair-present-bot-fail.out" 2>"$TMP/repair-present-bot-fail.err"; then
  fail "Bot failure after reorder unexpectedly succeeded"
fi
grep -F 'waf_rule_position_restored' "$TMP/repair-present-bot-fail.err" >/dev/null
run_repair "$partial_fail_port" "$TMP/repair-present-bot-fail-audit" audit >"$TMP/repair-present-bot-fail-audit.out"
jq -e '
  .repair_state == "shadowed_by_earlier_rules"
  and .repair_first == false
  and .repair_before_candidate == true
  and .bot_fight_mode == true
' "$TMP/repair-present-bot-fail-audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "PATCH") | .method' "$partial_fail_log" | wc -l)" == "2" ]] \
  || fail "failed reordered state was not moved and restored exactly once"

IFS=';' read -r malformed_pid malformed_port malformed_log < <(start_server malformed_after_create malformed-after-create)
if run_repair "$malformed_port" "$TMP/malformed" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/malformed.out" 2>"$TMP/malformed.err"; then
  fail "malformed post-create response unexpectedly succeeded"
fi
grep -F 'partial changes were rolled back' "$TMP/malformed.err" >/dev/null
grep -F 'create response contained 0 matching repair rules' "$TMP/malformed.err" >/dev/null
run_repair "$malformed_port" "$TMP/malformed-audit" audit >"$TMP/malformed-audit.out"
jq -e '.repair_state == "absent" and .repair_rule_count == 0 and .bot_fight_mode == true' \
  "$TMP/malformed-audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "DELETE") | .method' "$malformed_log" | wc -l)" == "1" ]] \
  || fail "accepted create with malformed response was not rolled back"

IFS=';' read -r ambiguous_pid ambiguous_port ambiguous_log < <(start_server ambiguous ambiguous)
if run_repair "$ambiguous_port" "$TMP/ambiguous" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/ambiguous.out" 2>"$TMP/ambiguous.err"; then
  fail "ambiguous country rules unexpectedly applied"
fi
grep -F 'expected exactly one broad country block candidate; found 2' "$TMP/ambiguous.err" >/dev/null
[[ "$(jq -r 'select(.method != "GET") | .method' "$ambiguous_log" | wc -l)" == "0" ]] \
  || fail "ambiguous preflight mutated Cloudflare"

IFS=';' read -r bot_fail_pid bot_fail_port bot_fail_log < <(start_server bot_fail bot-fail)
if run_repair "$bot_fail_port" "$TMP/bot-fail" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/bot-fail.out" 2>"$TMP/bot-fail.err"; then
  fail "Bot permission failure unexpectedly succeeded"
fi
grep -F 'partial changes were rolled back' "$TMP/bot-fail.err" >/dev/null
run_repair "$bot_fail_port" "$TMP/bot-fail-audit" audit >"$TMP/bot-fail-audit.out"
jq -e '.repair_state == "absent" and .repair_rule_count == 0 and .bot_fight_mode == true' \
  "$TMP/bot-fail-audit/evidence.json" >/dev/null
[[ "$(jq -r 'select(.method == "DELETE") | .method' "$bot_fail_log" | wc -l)" == "1" ]] \
  || fail "partial WAF change was not rolled back"

for secret in mock-edge-token-secret; do
  if grep -R -F "$secret" "$TMP" --exclude='*.log' --exclude='*.stderr' --exclude='*.stdout'; then
    fail "token leaked into output"
  fi
done

if grep -R -E 'ip\.(geoip|src)\.country|country restriction|"PL"' "$TMP"/audit "$TMP"/apply "$TMP"/rollback "$TMP"/idempotent; then
  fail "country rule expression or literal leaked into sanitized evidence"
fi

IFS=';' read -r baseline_pid baseline_port baseline_log < <(start_server baseline_off baseline-off)
run_repair "$baseline_port" "$TMP/baseline-apply" apply APPLY-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/baseline-apply.out"
jq -e '
  .desired_state == true
  and .repair_first == true
  and .bot_fight_mode == false
  and .bot_baseline == false
  and .mutations == ["waf_skip_rule_created"]
' "$TMP/baseline-apply/evidence.json" >/dev/null
run_repair "$baseline_port" "$TMP/baseline-rollback" rollback ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR >"$TMP/baseline-rollback.out"
jq -e '
  .repair_state == "absent"
  and .bot_fight_mode == false
  and .mutations == ["waf_skip_rule_deleted"]
' "$TMP/baseline-rollback/evidence.json" >/dev/null

WORKFLOW="$ROOT/.github/workflows/cloudflare-oteryn-public-edge-repair.yml"
MARKER="$ROOT/ops/triggers/cloudflare-oteryn-public-edge-repair.md"
python3 - "$WORKFLOW" "$MARKER" <<'PY'
from pathlib import Path
import sys
workflow=Path(sys.argv[1]).read_text(encoding="utf-8")
marker=Path(sys.argv[2]).read_text(encoding="utf-8")
required=[
    "pull_request:",
    "push:",
    "branches:\n      - main",
    "ops/triggers/cloudflare-oteryn-public-edge-repair.md",
    "if: github.event_name == 'push'",
    "environment: production-cloudflare",
    "CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_EDGE_AUDIT_TOKEN }}",
    "CLOUDFLARE_ACCESS_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}",
    "ref: ${{ github.sha }}",
    "mode: apply",
    "mode: rollback",
    "APPLY-OTERYN-PUBLIC-EDGE-REPAIR",
    "ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR",
    "An operational repair PR may change only the repair marker.",
    "python3 scripts/operations/cloudflare-oteryn-public-edge-repair.py",
    "python3 scripts/operations/cloudflare-oteryn-edge-audit.py",
    "python3 scripts/operations/oteryn-public-edge-validation.py",
    "python3 scripts/operations/oteryn-public-edge-result.py",
    "issues: write",
    "ISSUE_NUMBER: '91'",
    "Only bounded repair and public-validation fields are published.",
    "Propagate operation failure",
    "Propagate collector execution failure",
]
for item in required:
    if item not in workflow:
        raise SystemExit(f"missing workflow invariant: {item}")
allowed_markers = {
    "# Cloudflare Oteryn public edge repair trigger\n\nmode: inert\nconfirmation:\n",
    "# Cloudflare Oteryn public edge repair trigger\n\nmode: audit\nconfirmation:\n",
    "# Cloudflare Oteryn public edge repair trigger\n\nmode: apply\nconfirmation: APPLY-OTERYN-PUBLIC-EDGE-REPAIR\n",
    "# Cloudflare Oteryn public edge repair trigger\n\nmode: rollback\nconfirmation: ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR\n",
}
if marker not in allowed_markers:
    raise SystemExit("committed repair marker is not an exact allowed state")
for forbidden in (
    'cat "$raw_output"',
    'body="$raw_output"',
    'body="$(cat "$raw_output")"',
    'cat cloudflare-public-edge-repair/evidence.json',
    'cat cloudflare-edge-audit/evidence.json',
    'cat public-edge-validation/evidence.json',
):
    if forbidden in workflow:
        raise SystemExit(f"raw output could reach issue comments: {forbidden}")
if workflow.count("APPLY-OTERYN-PUBLIC-EDGE-REPAIR") < 3:
    raise SystemExit("apply confirmation is not independently enforced")
if workflow.count("ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR") < 3:
    raise SystemExit("rollback confirmation is not independently enforced")
PY

echo "Cloudflare public-edge repair tests passed."
