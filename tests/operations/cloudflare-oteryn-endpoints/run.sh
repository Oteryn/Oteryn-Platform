#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/operations/cloudflare-oteryn-endpoints.sh"
WORKFLOW="$ROOT/.github/workflows/cloudflare-oteryn-endpoints.yml"

fail_test() {
    printf 'TEST FAILURE: %s\n' "$*" >&2
    exit 1
}

assert_jq() {
    local json="$1"
    local expression="$2"
    local message="$3"
    jq -e "$expression" <<<"$json" >/dev/null || fail_test "$message"
}

bash -n "$SCRIPT"
CLOUDFLARE_LIBRARY_ONLY=1 source "$SCRIPT"

fixture='{
  "originRequest": {"connectTimeout": "30s"},
  "ingress": [
    {"hostname": "other.molehill.cloud", "service": "http://127.0.0.1:9000"},
    {"hostname": "oteryn.molehill.cloud", "service": "http://old-www:8000", "originRequest": {"httpHostHeader": "old-www"}},
    {"hostname": "login.oteryn.molehill.cloud", "service": "http://old-login:8080"},
    {"service": "http_status:404"}
  ]
}'

desired="$(build_desired_config "$fixture")"
assert_jq "$desired" '.ingress[0].hostname == "oteryn.molehill.cloud" and .ingress[0].service == "http://127.0.0.1:8000"' 'WWW rule is not canonical'
assert_jq "$desired" '.ingress[0].originRequest.httpHostHeader == "old-www"' 'existing WWW originRequest was not preserved'
assert_jq "$desired" '.ingress[1].hostname == "login.oteryn.molehill.cloud" and .ingress[1].service == "http://127.0.0.1:8080"' 'login rule is not canonical'
assert_jq "$desired" '.ingress[2].hostname == "other.molehill.cloud" and .ingress[2].service == "http://127.0.0.1:9000"' 'unrelated ingress rule was not preserved'
assert_jq "$desired" '.ingress[-1].service == "http_status:404" and ((.ingress[-1].hostname // "") == "")' 'catch-all was not preserved at the end'
assert_jq "$desired" '.originRequest.connectTimeout == "30s"' 'top-level originRequest was not preserved'

idempotent="$(build_desired_config "$desired")"
[[ "$(canonical_json <<<"$desired")" == "$(canonical_json <<<"$idempotent")" ]] || fail_test 'desired tunnel configuration is not idempotent'

duplicate_fixture="$(jq '.ingress = ([.ingress[1]] + .ingress)' <<<"$fixture")"
if (build_desired_config "$duplicate_fixture" >/dev/null 2>&1); then
    fail_test 'duplicate canonical hostname was accepted'
fi

bad_catchall_fixture="$(jq '.ingress = ([.ingress[-1]] + .ingress[0:-1])' <<<"$fixture")"
if (build_desired_config "$bad_catchall_fixture" >/dev/null 2>&1); then
    fail_test 'non-final catch-all was accepted'
fi

path_fixture="$(jq '.ingress[1].path = "/partial"' <<<"$fixture")"
if (build_desired_config "$path_fixture" >/dev/null 2>&1); then
    fail_test 'path-scoped canonical hostname was accepted'
fi

target='123e4567-e89b-42d3-a456-426614174000.cfargotunnel.com'
missing='{"success":true,"result":[]}'
[[ "$(dns_record_state "$missing" "$WWW_HOST" "$target")" == 'missing' ]] || fail_test 'missing DNS state was misclassified'

current="$(jq -cn --arg host "${WWW_HOST^^}." --arg target "${target^^}." '{success:true,result:[{id:"record-1",type:"CNAME",name:$host,content:$target,proxied:true}]}')"
[[ "$(dns_record_state "$current" "$WWW_HOST" "$target")" == 'current' ]] || fail_test 'current DNS state was misclassified'

drift="$(jq '.result[0].proxied = false' <<<"$current")"
[[ "$(dns_record_state "$drift" "$WWW_HOST" "$target")" == 'drift' ]] || fail_test 'DNS proxy drift was not detected'

conflict="$(jq '.result[0].type = "A" | .result[0].content = "192.0.2.10"' <<<"$current")"
if (dns_record_state "$conflict" "$WWW_HOST" "$target" >/dev/null 2>&1); then
    fail_test 'conflicting DNS record type was accepted'
fi

multiple="$(jq '.result += [.result[0]]' <<<"$current")"
if (dns_record_state "$multiple" "$WWW_HOST" "$target" >/dev/null 2>&1); then
    fail_test 'multiple DNS records were accepted'
fi

PORT_FILE="$(mktemp)"
SUMMARY_FILE="$(mktemp)"
MOCK_PID=""
cleanup_test() {
    if [[ -n "$MOCK_PID" ]]; then
        kill "$MOCK_PID" >/dev/null 2>&1 || true
        wait "$MOCK_PID" 2>/dev/null || true
    fi
    rm -f "$PORT_FILE" "$SUMMARY_FILE"
}
trap cleanup_test EXIT

python3 "$ROOT/tests/operations/cloudflare-oteryn-endpoints/mock_cloudflare.py" --port-file "$PORT_FILE" &
MOCK_PID=$!
for _ in $(seq 1 100); do
    [[ -s "$PORT_FILE" ]] && break
    sleep 0.02
done
[[ -s "$PORT_FILE" ]] || fail_test 'mock Cloudflare server did not start'
MOCK_PORT="$(cat "$PORT_FILE")"
MOCK_BASE="http://127.0.0.1:${MOCK_PORT}/client/v4"
COMMON_ENV=(
    "CLOUDFLARE_API_BASE_URL=$MOCK_BASE"
    "CLOUDFLARE_API_TOKEN=test-token"
    "CLOUDFLARE_ACCOUNT_ID=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    "CLOUDFLARE_ZONE_ID=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    "CLOUDFLARE_TUNNEL_ID=123e4567-e89b-42d3-a456-426614174000"
    "GITHUB_STEP_SUMMARY=$SUMMARY_FILE"
)

audit_output="$(env "${COMMON_ENV[@]}" "$SCRIPT" audit)"
[[ "$audit_output" != *test-token* ]] || fail_test 'audit output exposed the API token'
state="$(curl --silent --show-error "http://127.0.0.1:${MOCK_PORT}/__state")"
assert_jq "$state" '.mutations | length == 0' 'audit mode mutated Cloudflare state'

if env "${COMMON_ENV[@]}" CLOUDFLARE_APPLY_CONFIRMATION=WRONG "$SCRIPT" apply >/dev/null 2>&1; then
    fail_test 'apply accepted an invalid confirmation phrase'
fi

apply_output="$(env "${COMMON_ENV[@]}" CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE "$SCRIPT" apply)"
[[ "$apply_output" != *test-token* ]] || fail_test 'apply output exposed the API token'
state="$(curl --silent --show-error "http://127.0.0.1:${MOCK_PORT}/__state")"
assert_jq "$state" '.mutations == ["tunnel-put", "dns-post:oteryn.molehill.cloud", "dns-patch:login.oteryn.molehill.cloud"]' 'apply did not perform the expected bounded mutations'
assert_jq "$state" '.config.ingress[0].hostname == "oteryn.molehill.cloud" and .config.ingress[0].service == "http://127.0.0.1:8000"' 'apply did not reconcile the WWW tunnel rule'
assert_jq "$state" '.config.ingress[1].hostname == "login.oteryn.molehill.cloud" and .config.ingress[1].service == "http://127.0.0.1:8080"' 'apply did not reconcile the login tunnel rule'
assert_jq "$state" '.config.ingress[2].hostname == "other.molehill.cloud"' 'apply did not preserve unrelated ingress ordering'
assert_jq "$state" '.config.ingress[-1].service == "http_status:404"' 'apply did not preserve the catch-all rule'
assert_jq "$state" '.dns["oteryn.molehill.cloud"].content == "123e4567-e89b-42d3-a456-426614174000.cfargotunnel.com" and .dns["oteryn.molehill.cloud"].proxied == true' 'apply did not create canonical WWW DNS'
assert_jq "$state" '.dns["login.oteryn.molehill.cloud"].content == "123e4567-e89b-42d3-a456-426614174000.cfargotunnel.com" and .dns["login.oteryn.molehill.cloud"].proxied == true' 'apply did not reconcile canonical login DNS'

env "${COMMON_ENV[@]}" CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE "$SCRIPT" apply >/dev/null
state="$(curl --silent --show-error "http://127.0.0.1:${MOCK_PORT}/__state")"
assert_jq "$state" '.mutations | length == 3' 'second apply was not idempotent'
[[ "$(cat "$SUMMARY_FILE")" != *test-token* ]] || fail_test 'step summary exposed the API token'

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
required = [
    "workflow_dispatch:",
    "pull_request:",
    "environment: production-cloudflare",
    "permissions:\n  contents: read",
    "scripts/operations/cloudflare-oteryn-endpoints.sh",
    "CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}",
    "CLOUDFLARE_ACCOUNT_ID: ${{ vars.CLOUDFLARE_ACCOUNT_ID }}",
    "CLOUDFLARE_ZONE_ID: ${{ vars.CLOUDFLARE_ZONE_ID }}",
    "CLOUDFLARE_TUNNEL_ID: ${{ vars.CLOUDFLARE_TUNNEL_ID }}",
]
for item in required:
    if item not in text:
        raise SystemExit(f"missing workflow invariant: {item}")

for forbidden_input in ("hostname:", "service:", "account_id:", "zone_id:", "tunnel_id:"):
    dispatch = text.split("workflow_dispatch:", 1)[1].split("permissions:", 1)[0]
    if forbidden_input in dispatch:
        raise SystemExit(f"arbitrary dispatch input is forbidden: {forbidden_input}")
PY

printf 'Cloudflare endpoint automation tests: PASS\n'
