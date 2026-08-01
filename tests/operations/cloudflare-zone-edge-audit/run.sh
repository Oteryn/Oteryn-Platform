#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/operations/cloudflare-zone-edge-audit.sh"
WORKFLOW="$ROOT/.github/workflows/cloudflare-zone-edge-audit.yml"
MOCK_CURL="$ROOT/tests/operations/cloudflare-zone-edge-audit/mock_curl.py"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TMP_DIR"' EXIT

fail_test() {
    printf 'TEST FAILURE: %s\n' "$*" >&2
    exit 1
}

bash -n "$SCRIPT"
python3 -m py_compile "$MOCK_CURL"

grep -F -- '--request GET' "$SCRIPT" >/dev/null || fail_test 'Cloudflare request is not fixed to GET'
if grep -Eq -- '--request[[:space:]]+(POST|PUT|PATCH|DELETE)' "$SCRIPT"; then
    fail_test 'mutating Cloudflare HTTP method exists in audit script'
fi
if grep -Eq 'api_(post|put|patch|delete)|curl .*(-X|--request) .*(POST|PUT|PATCH|DELETE)' "$SCRIPT"; then
    fail_test 'mutating Cloudflare helper exists in audit script'
fi
grep -F "if: github.event_name == 'push' || github.event_name == 'workflow_dispatch'" "$WORKFLOW" >/dev/null \
    || fail_test 'live audit event boundary is missing'
grep -F 'environment: production-cloudflare' "$WORKFLOW" >/dev/null \
    || fail_test 'protected Cloudflare environment is missing'
grep -F 'CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_EDGE_AUDIT_TOKEN }}' "$WORKFLOW" >/dev/null \
    || fail_test 'dedicated zone read token is not wired'
grep -F 'CLOUDFLARE_ACCESS_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}' "$WORKFLOW" >/dev/null \
    || fail_test 'separate Access token is not wired'

chmod +x "$MOCK_CURL"
export CLOUDFLARE_API_BASE_URL='https://mock.cloudflare.invalid/client/v4'
export CLOUDFLARE_API_TOKEN='user_mock-secret-value'
export CLOUDFLARE_ACCESS_API_TOKEN='cfat_mock-access-secret-value'
export CLOUDFLARE_ACCOUNT_ID='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
export CLOUDFLARE_ZONE_ID='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
export CLOUDFLARE_CURL_BIN="$MOCK_CURL"
export CLOUDFLARE_ZONE_EDGE_OUTPUT="$TMP_DIR/audit.json"
export MOCK_CURL_LOG="$TMP_DIR/calls.log"

output="$(bash "$SCRIPT")"
jq -e '
    .schema_version == 2
    and .audit_complete == true
    and .mutation == "none"
    and .secrets_emitted == false
    and .canonical_hosts == ["oteryn.molehill.cloud", "gateway.molehill.cloud"]
    and .retired_hosts == ["login.oteryn.molehill.cloud"]
    and .certificates.www_hostname_covered == "true"
    and .certificates.gateway_hostname_covered == "true"
    and .certificates.legacy_gateway_hostname_covered == "false"
    and .zone_settings.always_use_https == "off"
    and .zone_settings.minimum_tls_version == "1.3"
    and .zone_settings.security_header.strict_transport_security.max_age == 0
    and .rulesets.zone.rulesets[0].canonical_challenge_or_block_count == 1
    and .rulesets.zone.rulesets[0].retired_hostname_rule_count == 0
    and .bot_management.fight_mode == true
    and .access.matching_application_count == 0
    and .access.retired_application_count == 0
' "$TMP_DIR/audit.json" >/dev/null || fail_test 'sanitized audit classification is incorrect'

[[ "$(awk '$1 != "GET" {count++} END {print count+0}' "$TMP_DIR/calls.log")" == "0" ]] \
    || fail_test 'mock observed a non-GET Cloudflare request'
[[ "$(wc -l <"$TMP_DIR/calls.log")" -ge 9 ]] || fail_test 'expected audit endpoints were not queried'
for secret in user_mock-secret-value cfat_mock-access-secret-value; do
    if grep -F "$secret" "$TMP_DIR/audit.json" "$TMP_DIR/calls.log" || grep -F "$secret" <<<"$output"; then
        fail_test 'Cloudflare token leaked into output'
    fi
done

printf 'Cloudflare zone-edge audit validation passed.\n'
