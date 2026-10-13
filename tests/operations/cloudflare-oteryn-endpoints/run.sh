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

TESTS="$ROOT/tests/operations/cloudflare-oteryn-endpoints"
GH_DIR="$(mktemp -d)"
WORK="$(mktemp -d)"
export FAKE_GH_DIR="$GH_DIR"
export OTERYN_GH_BIN="$TESTS/fake_gh.py"
export GITHUB_REPOSITORY="Oteryn/Oteryn-Platform"

bash -n "$SCRIPT"
CLOUDFLARE_LIBRARY_ONLY=1 source "$SCRIPT"

fixture='{
  "originRequest": {"connectTimeout": "30s"},
  "ingress": [
    {"hostname": "other.molehill.cloud", "service": "http://127.0.0.1:9000"},
    {"hostname": "oteryn.molehill.cloud", "service": "http://old-www:8000", "originRequest": {"httpHostHeader": "old-www"}},
    {"hostname": "login.oteryn.molehill.cloud", "service": "http://old-gateway:8080", "originRequest": {"connectTimeout": "15s"}},
    {"service": "http_status:404"}
  ]
}'

desired="$(build_desired_config "$fixture")"
assert_jq "$desired" '.ingress[0].hostname == "oteryn.molehill.cloud" and .ingress[0].service == "http://127.0.0.1:8000"' 'WWW rule is not canonical'
assert_jq "$desired" '.ingress[0].originRequest.httpHostHeader == "old-www"' 'existing WWW originRequest was not preserved'
assert_jq "$desired" '.ingress[1].hostname == "gateway.molehill.cloud" and .ingress[1].service == "http://127.0.0.1:8080"' 'Gateway rule is not canonical'
assert_jq "$desired" '.ingress[1].originRequest.connectTimeout == "15s"' 'legacy Gateway options were not preserved during rename'
assert_jq "$desired" '[.ingress[] | select((.hostname // "") == "login.oteryn.molehill.cloud")] | length == 0' 'legacy Gateway ingress was retained'
assert_jq "$desired" '.ingress[2].hostname == "other.molehill.cloud" and .ingress[2].service == "http://127.0.0.1:9000"' 'unrelated ingress rule was not preserved'
assert_jq "$desired" '.ingress[-1].service == "http_status:404" and ((.ingress[-1].hostname // "") == "")' 'catch-all was not preserved at the end'
assert_jq "$desired" '.originRequest.connectTimeout == "30s"' 'top-level originRequest was not preserved'

idempotent="$(build_desired_config "$desired")"
[[ "$(canonical_json <<<"$desired")" == "$(canonical_json <<<"$idempotent")" ]] || fail_test 'desired tunnel configuration is not idempotent'

duplicate_fixture="$(jq '.ingress = ([.ingress[1]] + .ingress)' <<<"$fixture")"
if (build_desired_config "$duplicate_fixture" >/dev/null 2>&1); then
    fail_test 'duplicate canonical hostname was accepted'
fi

legacy_duplicate_fixture="$(jq '.ingress = ([.ingress[2]] + .ingress)' <<<"$fixture")"
if (build_desired_config "$legacy_duplicate_fixture" >/dev/null 2>&1); then
    fail_test 'duplicate legacy Gateway hostname was accepted'
fi

bad_catchall_fixture="$(jq '.ingress = ([.ingress[-1]] + .ingress[0:-1])' <<<"$fixture")"
if (build_desired_config "$bad_catchall_fixture" >/dev/null 2>&1); then
    fail_test 'non-final catch-all was accepted'
fi

path_fixture="$(jq '.ingress[2].path = "/partial"' <<<"$fixture")"
if (build_desired_config "$path_fixture" >/dev/null 2>&1); then
    fail_test 'path-scoped legacy Gateway hostname was accepted'
fi

target='123e4567-e89b-42d3-a456-426614174000.cfargotunnel.com'
missing='{"success":true,"result":[]}'
[[ "$(dns_record_state "$missing" "$WWW_HOST" "$target")" == 'missing' ]] || fail_test 'missing DNS state was misclassified'
[[ "$(legacy_dns_state "$missing" "$target")" == 'missing' ]] || fail_test 'missing legacy DNS state was misclassified'

current="$(jq -cn --arg host "${GATEWAY_HOST^^}." --arg target "${target^^}." '{success:true,result:[{id:"record-1",type:"CNAME",name:$host,content:$target,proxied:true}]}')"
[[ "$(dns_record_state "$current" "$GATEWAY_HOST" "$target")" == 'current' ]] || fail_test 'current Gateway DNS state was misclassified'

drift="$(jq '.result[0].proxied = false' <<<"$current")"
[[ "$(dns_record_state "$drift" "$GATEWAY_HOST" "$target")" == 'drift' ]] || fail_test 'Gateway DNS proxy drift was not detected'

legacy_safe="$(jq -cn --arg host "$LEGACY_GATEWAY_HOST" --arg target "$target" '{success:true,result:[{id:"legacy-1",type:"CNAME",name:$host,content:$target,proxied:true}]}')"
[[ "$(legacy_dns_state "$legacy_safe" "$target")" == 'safe-to-retire' ]] || fail_test 'safe legacy DNS was not recognized'

legacy_conflict="$(jq '.result[0].content = "other.example.invalid"' <<<"$legacy_safe")"
if (legacy_dns_state "$legacy_conflict" "$target" >/dev/null 2>&1); then
    fail_test 'legacy DNS pointing outside the managed tunnel was accepted'
fi

conflict="$(jq '.result[0].type = "A" | .result[0].content = "192.0.2.10"' <<<"$current")"
if (dns_record_state "$conflict" "$GATEWAY_HOST" "$target" >/dev/null 2>&1); then
    fail_test 'conflicting DNS record type was accepted'
fi

multiple="$(jq '.result += [.result[0]]' <<<"$current")"
if (dns_record_state "$multiple" "$GATEWAY_HOST" "$target" >/dev/null 2>&1); then
    fail_test 'multiple DNS records were accepted'
fi


# ---------------------------------------------------------------------------
# Gate: pure-function tests (no network)
# ---------------------------------------------------------------------------
ok() { "$@" >/dev/null 2>&1 || fail_test "expected success: $*"; }
no() { if "$@" >/dev/null 2>&1; then fail_test "expected failure: $*"; fi; }
fixture() { python3 "$TESTS/gh_fixture.py" "$GH_DIR" "$@"; }

OPEN_CFG='{
  "originRequest": {"connectTimeout": "30s"},
  "ingress": [
    {"hostname": "oteryn.molehill.cloud", "service": "http://127.0.0.1:8000", "originRequest": {"httpHostHeader": "www"}},
    {"hostname": "gateway.molehill.cloud", "service": "http://127.0.0.1:8080"},
    {"hostname": "other.molehill.cloud", "service": "http://svc.example.test:9000", "originRequest": {"connectTimeout": "15s"}},
    {"service": "http_status:404"}
  ]
}'

[[ "$(classify_gate_state "$OPEN_CFG")" == open ]] || fail_test 'open configuration was not classified open'
CLOSED_CFG="$(set_gate_services "$OPEN_CFG" "$GATE_SERVICE" "$GATE_SERVICE")"
[[ "$(classify_gate_state "$CLOSED_CFG")" == closed ]] || fail_test 'closed configuration was not classified closed'
[[ "$(classify_gate_state "$(set_gate_services "$OPEN_CFG" "$GATE_SERVICE" "$GATEWAY_SERVICE")")" == split ]] || fail_test 'split (www closed) misclassified'
[[ "$(classify_gate_state "$(set_gate_services "$OPEN_CFG" "$WWW_SERVICE" "$GATE_SERVICE")")" == split ]] || fail_test 'split (gateway closed) misclassified'
[[ "$(classify_gate_state "$(set_gate_services "$OPEN_CFG" "$GATE_SERVICE" 'http://elsewhere:1')")" == unknown ]] || fail_test 'one 503 plus arbitrary service must be unknown'
[[ "$(classify_gate_state "$(jq 'del(.ingress[1])' <<<"$OPEN_CFG")")" == unknown ]] || fail_test 'missing canonical rule must be unknown'
[[ "$(classify_gate_state "$(jq '.ingress[0].service = "http://old-www:8000"' <<<"$OPEN_CFG")")" == unknown ]] || fail_test 'wrongly pointed canonical rule must be unknown'
[[ "$(classify_gate_state "$(jq '.ingress[0].path = "/x"' <<<"$OPEN_CFG")")" == unknown ]] || fail_test 'path-scoped canonical rule must be unknown'
[[ "$(classify_gate_state "$(jq '.ingress = ([.ingress[0]] + .ingress)' <<<"$OPEN_CFG")")" == unknown ]] || fail_test 'duplicate canonical rule must be unknown'
[[ "$(classify_gate_state '{"ingress":[]}')" == unknown ]] || fail_test 'invalid configuration must be unknown'
[[ "$(classify_gate_state "$(jq '.ingress[0].hostname = "OTERYN.Molehill.Cloud."' <<<"$OPEN_CFG")")" == open ]] || fail_test 'canonical hostname match must be case-insensitive and dot-insensitive'
[[ "$(classify_gate_state "$CLOSED_CFG")" != open ]] || fail_test 'closed classified open'

# Preservation of unrelated rules and canonical-rule fields.
[[ "$(jq -c '.ingress[2:]' <<<"$CLOSED_CFG")" == "$(jq -c '.ingress[2:]' <<<"$OPEN_CFG")" ]] || fail_test 'close changed unrelated rules'
[[ "$(jq -c '.ingress[0].originRequest' <<<"$CLOSED_CFG")" == '{"httpHostHeader":"www"}' ]] || fail_test 'close changed canonical rule fields'
[[ "$(config_shape_hash "$CLOSED_CFG")" == "$(config_shape_hash "$OPEN_CFG")" ]] || fail_test 'shape hash differs between open and closed'

# Origin-route precondition.
check_route() {
    local service="$1" expected="$2"
    if service_is_origin_route "$service"; then
        [[ "$expected" == match ]] || fail_test "service unexpectedly counted as origin route: $service"
    else
        [[ "$expected" == none ]] || fail_test "service unexpectedly not counted: $service"
    fi
}
for service in \
    'http://127.0.0.1:8000' 'http://127.0.0.1:8080' 'http://localhost:8000' 'http://[::1]:8080' 'http://0.0.0.0:8000' \
    'http://127.5.5.5:8080' 'http://platform.internal:8000' 'http://platform.internal:8080' 'http://192.168.1.5:8000' \
    'http://platform.internal:8000/with/path' 'https://user:pw@platform.internal:8080/x' \
    'http://[0:0:0:0:0:0:0:1]:80' 'http://[::ffff:127.0.0.1]:80' 'http://2130706433:80' 'http://0177.0.0.1:80' \
    'http://0x7f.1:80' 'http://127.1.2.3:80' 'http://0.0.0.0:80' 'http://[::]:80' 'http://203.0.113.9:80' \
    'http://127.0.0.1:80' 'http://localhost:3000' 'http://[::1]:9000' 'http://LOCALHOST:5' 'http://localhost.:5' \
    'http://127.0.0.1.:5' 'http://PLATFORM.internal.:8000' 'ws://platform.internal:8000' 'tcp://platform.internal:8080' \
    'garbage' 'http://' 'http://[::1' '' 'http://host:99999' 'http://host:80:90' 'http://a b:8000'; do
    check_route "$service" match
done
for service in 'http_status:404' 'http_status:503' 'unix:/var/run/x.sock' 'hello_world' 'bastion' \
    'http://svc.example.test:9000' 'https://svc.example.test' 'http://svc.example.test' 'http://svc.example.test.:9000'; do
    check_route "$service" none
done
[[ "$(canonical_ip_literal 2130706433)" == 127.0.0.1 && "$(canonical_ip_literal 0177.0.0.1)" == 127.0.0.1 \
    && "$(canonical_ip_literal 0x7f.1)" == 127.0.0.1 && "$(canonical_ip_literal '[0:0:0:0:0:0:0:1]')" == ::1 \
    && "$(canonical_ip_literal '[::ffff:127.0.0.1]')" == 127.0.0.1 && "$(canonical_ip_literal '[::]')" == :: ]] || fail_test 'IP literal canonicalization is wrong'
no canonical_ip_literal platform.internal

ALLOWED_ORIGIN_HOSTS=(platform.internal)
check_route 'http://platform.internal:8000' none
check_route 'http://other.internal:8000' match
ALLOWED_ORIGIN_HOSTS=(127.0.0.1)
check_route 'http://2130706433:80' none
check_route 'http://127.0.0.2:80' match
ALLOWED_ORIGIN_HOSTS=()

routes_for() { jq -c --argjson rule "$1" '.ingress = ([.ingress[0], .ingress[1], $rule] + .ingress[2:])' <<<"$OPEN_CFG"; }
[[ "$(other_origin_routes_count "$OPEN_CFG")" == 0 ]] || fail_test 'clean configuration has origin routes'
for rule in \
    '{"hostname":"*.molehill.cloud","service":"http://127.0.0.1:8000"}' \
    '{"hostname":"login.oteryn.molehill.cloud","service":"http://old-gateway:8080"}' \
    '{"path":"/private","service":"http://platform.internal:8000"}' \
    '{"hostname":"other.example","path":"/private","service":"http://127.0.0.1:80"}' \
    '{"hostname":"x.example","service":"http://[::1]:9000"}'; do
    [[ "$(other_origin_routes_count "$(routes_for "$rule")")" == 1 ]] || fail_test "origin route not counted for $rule"
done
[[ "$(other_origin_routes_count "$(jq '.ingress[-1].service = "http://127.0.0.1:8080"' <<<"$OPEN_CFG")")" == 1 ]] || fail_test 'catch-all on a protected origin not counted'
[[ "$(other_origin_routes_count "$(jq '.ingress[0].service = "http://127.0.0.1:8000"' <<<"$OPEN_CFG")")" == 0 ]] || fail_test 'canonical rules must not be counted'

# Scoped gate: other services on the tunnel are preserved; only bypass rules block a close.
# A tunnel shaped like the production one: canonical rules plus unrelated local and LAN services.
SHARED_CFG="$(jq -c '.ingress = ([.ingress[0], .ingress[1],
    {"hostname":"ha.example.test","service":"http://192.168.1.83:8123"},
    {"hostname":"nas.example.test","service":"https://localhost:5001","originRequest":{"noTLSVerify":true}},
    {"hostname":"site.example.test","service":"https://127.0.0.1:18574"},
    {"hostname":"site.example.test:8443","service":"https://127.0.0.1:18574"}] + .ingress[2:])' <<<"$OPEN_CFG")"
[[ "$(other_origin_routes_count "$SHARED_CFG")" == 4 ]] || fail_test 'shared tunnel origin routes miscounted'
[[ "$(oteryn_bypass_rules_count "$SHARED_CFG")" == 0 ]] || fail_test 'unrelated services were counted as bypass rules'
[[ "$(oteryn_bypass_rules_count "$OPEN_CFG")" == 0 ]] || fail_test 'clean configuration has bypass rules'
[[ "$(preserved_rules_count "$SHARED_CFG")" == 6 ]] || fail_test 'preserved rule count is wrong'
[[ "$(preserved_rules_count "$OPEN_CFG")" == 2 ]] || fail_test 'preserved rule count must include the catch-all'
SHARED_CLOSED="$(set_gate_services "$SHARED_CFG" "$GATE_SERVICE" "$GATE_SERVICE")"
[[ "$(classify_gate_state "$SHARED_CLOSED")" == closed ]] || fail_test 'shared tunnel did not close'
[[ "$(jq -c '.ingress[2:]' <<<"$SHARED_CLOSED")" == "$(jq -c '.ingress[2:]' <<<"$SHARED_CFG")" ]] || fail_test 'close changed preserved rules or their order'
[[ "$(preserved_rules_hash "$SHARED_CLOSED")" == "$(preserved_rules_hash "$SHARED_CFG")" ]] || fail_test 'preserved hash differs between open and closed'
[[ "$(preserved_rules_hash "$(jq '.ingress[3].service = "http://192.168.1.83:8124"' <<<"$SHARED_CFG")")" != "$(preserved_rules_hash "$SHARED_CFG")" ]] || fail_test 'preserved hash ignores a changed service'
[[ "$(preserved_rules_hash "$(jq '.ingress = (.ingress[0:2] + [.ingress[3], .ingress[2]] + .ingress[4:])' <<<"$SHARED_CFG")")" != "$(preserved_rules_hash "$SHARED_CFG")" ]] || fail_test 'preserved hash ignores rule order'
[[ "$(preserved_rules_hash "$(jq '.originRequest.connectTimeout = "31s"' <<<"$SHARED_CFG")")" != "$(preserved_rules_hash "$SHARED_CFG")" ]] || fail_test 'preserved hash ignores top-level fields'
[[ "$(preserved_rules_hash "$(jq '.ingress[0].originRequest.httpHostHeader = "x"' <<<"$SHARED_CFG")")" == "$(preserved_rules_hash "$SHARED_CFG")" ]] || fail_test 'preserved hash must exclude the canonical rules'
check_bypass() { # check_bypass <rule-json> <position: before|after> <expected-count>
    local cfg
    if [[ "$2" == before ]]; then
        cfg="$(jq -c --argjson rule "$1" '.ingress = ([$rule] + .ingress)' <<<"$SHARED_CFG")"
    else
        cfg="$(jq -c --argjson rule "$1" '.ingress = (.ingress[0:-1] + [$rule] + [.ingress[-1]])' <<<"$SHARED_CFG")"
    fi
    [[ "$(oteryn_bypass_rules_count "$cfg")" == "$3" ]] || fail_test "bypass count for $1 ($2) is not $3"
}
for rule in \
    '{"hostname":"x.example","service":"http://127.0.0.1:8000"}' \
    '{"hostname":"x.example","service":"http://platform.internal:8080"}' \
    '{"hostname":"x.example","service":"http://192.168.1.5:8000"}' \
    '{"hostname":"x.example","path":"/p","service":"http://[::1]:8080"}' \
    '{"hostname":"login.oteryn.molehill.cloud","service":"http://old-gateway:8080"}' \
    '{"hostname":"x.example","service":"garbage"}' \
    '{"hostname":"oteryn.molehill.cloud:8443","service":"https://127.0.0.1:18574"}' \
    '{"hostname":"GATEWAY.molehill.cloud.:443","service":"http_status:404"}'; do
    check_bypass "$rule" after 1
    check_bypass "$rule" before 1
done
for rule in '{"hostname":"*.molehill.cloud","service":"https://127.0.0.1:18574"}' '{"path":"/x","service":"http://svc.example.test:9000"}' \
    '{"hostname":"*","service":"http_status:404"}' '{"hostname":"*.cloud","service":"http_status:404"}' \
    '{"hostname":"*.oteryn.molehill.cloud","service":"http_status:404"}' '{"hostname":"a*.molehill.cloud","service":"http_status:404"}' \
    '{"hostname":"*.MOLEHILL.cloud.:8443","service":"http_status:404"}' '{"hostname":7,"service":"http_status:404"}'; do
    check_bypass "$rule" before 1
    check_bypass "$rule" after 0
done
# a wildcard that cannot match a canonical hostname does not shadow it
check_bypass '{"hostname":"*.example.test","service":"https://127.0.0.1:18574"}' before 0
check_bypass '{"hostname":"*.other.molehill.cloud","service":"http_status:404"}' before 0
check_bypass '{"hostname":"x.example","service":"https://127.0.0.1:18574"}' before 0
[[ "$(oteryn_bypass_rules_count "$(jq '.ingress[-1].service = "http://127.0.0.1:8080"' <<<"$SHARED_CFG")")" == 1 ]] || fail_test 'catch-all on an Oteryn origin port is not a bypass'
ALLOWED_ORIGIN_HOSTS=(platform.internal)
check_bypass '{"hostname":"x.example","service":"http://platform.internal:8080"}' after 0
ALLOWED_ORIGIN_HOSTS=()

# Evidence, closure, run provenance and fingerprint validators (offline).
TASK='OTERYN-20261013-p6b'
GOOD_CLOSURE="$(jq -cn --arg task "$TASK" '{schema:"oteryn-edge-gate-closure/v2",task_id:$task,close_run_id:"1001",run_attempt:1,edge_version:2,config_hash:("a"*1|"a"),preserved_rules_count:2,preserved_rules_hash:"",fingerprint_status:"unavailable",origin_fingerprint:null}' | jq -c ".config_hash = \"$(printf 'a%.0s' $(seq 64))\" | .preserved_rules_hash = \"$(printf 'c%.0s' $(seq 64))\"")"
ok validate_closure_json "$GOOD_CLOSURE" "$TASK" 1001
no validate_closure_json "$GOOD_CLOSURE" "$TASK" 1002
no validate_closure_json "$GOOD_CLOSURE" 'OTERYN-20261013-other' 1001
no validate_closure_json "$(jq '.run_attempt = 2' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.schema = "x"' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.fingerprint_status = "collected"' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.edge_version = "2"' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json 'not json' "$TASK" 1001
no validate_closure_json "$(jq '.schema = "oteryn-edge-gate-closure/v1"' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq 'del(.preserved_rules_hash)' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.preserved_rules_hash = "C"' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq 'del(.preserved_rules_count)' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.preserved_rules_count = 0' <<<"$GOOD_CLOSURE")" "$TASK" 1001
no validate_closure_json "$(jq '.preserved_rules_count = 1.5' <<<"$GOOD_CLOSURE")" "$TASK" 1001
HASH64="$(printf 'a%.0s' $(seq 64))"
PHASH64="$(printf 'c%.0s' $(seq 64))"
ok closure_matches_live "$GOOD_CLOSURE" 2 "$HASH64" "$PHASH64"
no closure_matches_live "$GOOD_CLOSURE" 3 "$HASH64" "$PHASH64"
no closure_matches_live "$GOOD_CLOSURE" 2 "$(printf 'b%.0s' $(seq 64))" "$PHASH64"
no closure_matches_live "$GOOD_CLOSURE" 2 "$HASH64" "$HASH64"
no closure_matches_live "$GOOD_CLOSURE" 2 "$HASH64" ""

GOOD_EVIDENCE="$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1001",outcome:"cutover_complete",head_sha:"0123456789012345678901234567890123456789"}')"
ok validate_evidence_json "$GOOD_EVIDENCE" "$TASK" 1001
ok validate_evidence_json "$(jq '.outcome = "rollback_verified"' <<<"$GOOD_EVIDENCE")" "$TASK" 1001
no validate_evidence_json "$(jq '.outcome = "stopped"' <<<"$GOOD_EVIDENCE")" "$TASK" 1001
no validate_evidence_json "$GOOD_EVIDENCE" "$TASK" 1002
no validate_evidence_json "$GOOD_EVIDENCE" 'OTERYN-20261013-other' 1001
no validate_evidence_json "$(jq '.task_id = "x"' <<<"$GOOD_EVIDENCE")" "$TASK" 1001

run_json() { jq -cn --arg id "$1" --arg path "$2" --arg event "$3" --arg branch "$4" --arg status "$5" --arg conclusion "$6" --argjson attempt "$7" \
    '{id:($id|tonumber),path:$path,event:$event,head_branch:$branch,status:$status,conclusion:(if $conclusion=="null" then null else $conclusion end),run_attempt:$attempt}'; }
EP='.github/workflows/cloudflare-oteryn-endpoints.yml'
CP='.github/workflows/deploy-synology-staging.yml'
[[ "$(validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed success 1)" 7 $EP strict)" == ok ]] || fail_test 'good closure run refused'
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed failure 1)" 7 $EP strict
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch main in_progress null 1)" 7 $EP strict
no validate_run_metadata "$(run_json 7 $EP push main completed success 1)" 7 $EP strict
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch feature completed success 1)" 7 $EP strict
no validate_run_metadata "$(run_json 7 $CP workflow_dispatch main completed success 1)" 7 $EP strict
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed success 2)" 7 $EP strict
no validate_run_metadata "$(run_json 8 $EP workflow_dispatch main completed success 1)" 7 $EP strict
[[ "$(validate_run_metadata "$(run_json 7 $CP workflow_dispatch main in_progress null 1)" 7 $CP evidence)" == ok ]] || fail_test 'in-progress evidence run refused'
[[ "$(validate_run_metadata "$(run_json 7 $CP workflow_dispatch main completed success 1)" 7 $CP evidence)" == ok ]] || fail_test 'successful evidence run refused'
for state in 'completed failure' 'completed cancelled' 'completed timed_out' 'completed neutral' 'completed skipped' 'completed stale' \
    'completed action_required' 'queued null' 'waiting null' 'pending null' 'completed null'; do
    set -- $state
    no validate_run_metadata "$(run_json 7 $CP workflow_dispatch main "$1" "$2" 1)" 7 $CP evidence
done
no validate_run_metadata "$(run_json 7 $CP workflow_dispatch main in_progress null 2)" 7 $CP evidence
[[ "$(validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed failure 1)" 7 $EP renewal)" == needs_jobs ]] || fail_test 'failed closure run not routed to the jobs check'
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed cancelled 1)" 7 $EP renewal
no validate_run_metadata "$(run_json 7 $EP workflow_dispatch main completed failure 2)" 7 $EP renewal

ok validate_renewal_jobs '{"jobs":[{"name":"manage","conclusion":"success"},{"name":"edge-probe","conclusion":"failure"},{"name":"validate","conclusion":"success"},{"name":"marker-audit","conclusion":"skipped"}]}'
no validate_renewal_jobs '{"jobs":[{"name":"manage","conclusion":"failure"},{"name":"edge-probe","conclusion":"failure"}]}'
no validate_renewal_jobs '{"jobs":[{"name":"manage","conclusion":"cancelled"},{"name":"edge-probe","conclusion":"failure"}]}'
no validate_renewal_jobs '{"jobs":[{"name":"manage","conclusion":"success"},{"name":"edge-probe","conclusion":"success"}]}'
no validate_renewal_jobs '{"jobs":[{"name":"manage","conclusion":"success"},{"name":"edge-probe","conclusion":"failure"},{"name":"edge-fingerprint","conclusion":"failure"}]}'

[[ "$(validate_artifact_listing '{"artifacts":[{"id":5,"name":"a","expired":false}]}' a)" == 5 ]] || fail_test 'single artifact refused'
no validate_artifact_listing '{"artifacts":[{"id":5,"name":"a","expired":true}]}' a
no validate_artifact_listing '{"artifacts":[{"id":5,"name":"a","expired":false},{"id":6,"name":"a","expired":false}]}' a
no validate_artifact_listing '{"artifacts":[]}' a
[[ "$(validate_artifact_listing '{"artifacts":[{"id":5,"name":"a","expired":true},{"id":6,"name":"a","expired":false}]}' a)" == 6 ]] || fail_test 'expired duplicate must be ignored'

candidate() { jq -cn --argjson www "$1" --argjson gw "$2" '{schema:"oteryn-edge-fingerprint-candidate/v1",collected_at:"2026-10-13T00:00:00Z",hosts:{"oteryn.molehill.cloud":$www,"gateway.molehill.cloud":$gw}}' >"$WORK/candidate.json"; evaluate_fingerprint_candidate "$WORK/candidate.json"; }
GOOD_HOST='{"status":200,"elements":["cookie:oteryn_sess","header:x-app"]}'
[[ "$(candidate "$GOOD_HOST" "$GOOD_HOST" | jq -r .status)" == collected ]] || fail_test 'good fingerprint candidate refused'
[[ "$(candidate "$GOOD_HOST" "$GOOD_HOST" | jq -c '.fingerprint["gateway.molehill.cloud"]')" == '["cookie:oteryn_sess","header:x-app"]' ]] || fail_test 'fingerprint not carried'
[[ "$(candidate '{"status":200,"elements":["header:x-oteryn-origin"]}' "$GOOD_HOST" | jq -r .status)" == collected ]] || fail_test 'marker alone must suffice'
for weak in '{"status":200,"elements":["header:x-app"]}' '{"status":503,"elements":["cookie:a","header:b"]}' '{"status":521,"elements":["cookie:a","header:b"]}' '{"status":200,"elements":[]}' '{"status":200,"elements":["Cookie:A","header:b"]}'; do
    [[ "$(candidate "$weak" "$GOOD_HOST" | jq -r .status)" == unavailable ]] || fail_test "weak fingerprint accepted: $weak"
done
[[ "$(evaluate_fingerprint_candidate "$WORK/missing.json" | jq -r .status)" == unavailable ]] || fail_test 'missing candidate must be unavailable'
echo '{"schema":"oteryn-edge-fingerprint-candidate/v1","hosts":{"oteryn.molehill.cloud":{"status":200,"elements":["a:b","c:d"]},"evil.example":{"status":200,"elements":[]}}}' >"$WORK/candidate.json"
[[ "$(evaluate_fingerprint_candidate "$WORK/candidate.json" | jq -r .status)" == unavailable ]] || fail_test 'non-canonical hostname in candidate accepted'

# Provenance through the gh-compatible function (stubbed by fake_gh.py).
fixture run 1001 "$EP" workflow_dispatch main completed success 1
fixture artifact 1001 oteryn-edge-gate-closure 91 false closure.json "$GOOD_CLOSURE"
[[ "$(fetch_run_artifact_json 1001 "$EP" oteryn-edge-gate-closure closure.json strict)" == "$GOOD_CLOSURE" ]] || fail_test 'closure artifact was not fetched'
fixture run 1002 "$EP" workflow_dispatch main completed failure 1
fixture artifact 1002 oteryn-edge-gate-closure 92 false closure.json "$GOOD_CLOSURE"
no fetch_run_artifact_json 1002 "$EP" oteryn-edge-gate-closure closure.json strict
no fetch_run_artifact_json 1002 "$EP" oteryn-edge-gate-closure closure.json renewal
fixture jobs 1002 manage=success edge-probe=failure validate=success
ok fetch_run_artifact_json 1002 "$EP" oteryn-edge-gate-closure closure.json renewal
fixture jobs 1002 manage=failure edge-probe=failure
no fetch_run_artifact_json 1002 "$EP" oteryn-edge-gate-closure closure.json renewal
no fetch_run_artifact_json 9999 "$EP" oteryn-edge-gate-closure closure.json strict
fixture run 1003 "$EP" workflow_dispatch main completed success 1
no fetch_run_artifact_json 1003 "$EP" oteryn-edge-gate-closure closure.json strict

# Serialization guard.
for wf in cloudflare-oteryn-endpoints.yml cloudflare-oteryn-endpoint-main-operation.yml; do
    for status in queued waiting pending in_progress; do
        fixture runs "$wf" "$status"
    done
done
ok gate_guard
fixture runs cloudflare-oteryn-endpoints.yml queued '5:Cloudflare Oteryn Endpoints [apply]'
ok gate_guard
for status in queued waiting pending in_progress; do
    fixture runs cloudflare-oteryn-endpoints.yml "$status" "5:Cloudflare Oteryn Endpoints [maintenance-close]"
    no gate_guard
    fixture runs cloudflare-oteryn-endpoints.yml "$status"
    fixture runs cloudflare-oteryn-endpoint-main-operation.yml "$status" "6:Cloudflare Oteryn Endpoints [maintenance-open]"
    no gate_guard
    fixture runs cloudflare-oteryn-endpoint-main-operation.yml "$status"
done
rm -f "$GH_DIR/runs_cloudflare-oteryn-endpoints.yml_waiting.json"
no gate_guard
fixture runs cloudflare-oteryn-endpoints.yml waiting

# ---------------------------------------------------------------------------
# Script tests against the mock
# ---------------------------------------------------------------------------
PORT_FILE="$(mktemp)"
SUMMARY_FILE="$(mktemp)"
ALL_OUT="$(mktemp)"
MOCK_PID=""
cleanup_test() {
    if [[ -n "$MOCK_PID" ]]; then
        kill "$MOCK_PID" >/dev/null 2>&1 || true
        wait "$MOCK_PID" 2>/dev/null || true
    fi
    rm -rf "$PORT_FILE" "$SUMMARY_FILE" "$ALL_OUT" "$GH_DIR" "$WORK"
}
trap cleanup_test EXIT

python3 "$TESTS/mock_cloudflare.py" --port-file "$PORT_FILE" &
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
TARGET_HOST='123e4567-e89b-42d3-a456-426614174000.cfargotunnel.com'

mock_post() { curl --silent --show-error --fail --request POST --header 'Content-Type: application/json' --data-binary "$2" "http://127.0.0.1:${MOCK_PORT}$1" >/dev/null; }
mock_state() { curl --silent --show-error "http://127.0.0.1:${MOCK_PORT}/__state"; }
run_script() {
    OUT="$(env "${COMMON_ENV[@]}" "$@" 2>&1)" && RC=0 || RC=$?
    printf '%s\n' "$OUT" >>"$ALL_OUT"
}
expect_rc() {
    [[ "$RC" == "$1" ]] || fail_test "$2 (rc=$RC): $OUT"
}
expect_no_write() {
    local before="$1" message="$2"
    assert_jq "$(mock_state)" ".mutations | length == $before" "$message"
}
reset_mock() { # reset_mock <config-json> [dns: current|default]
    local dns='{}'
    if [[ "${2:-current}" == default ]]; then
        mock_post /__reset "$(jq -cn --argjson c "$1" '{config:$c,version:1}')"
        return
    fi
    if [[ "${2:-current}" == current ]]; then
        dns="$(jq -cn --arg t "$TARGET_HOST" '{"oteryn.molehill.cloud":{id:"r1",type:"CNAME",name:"oteryn.molehill.cloud",content:$t,proxied:true},"gateway.molehill.cloud":{id:"r2",type:"CNAME",name:"gateway.molehill.cloud",content:$t,proxied:true}}')"
    fi
    mock_post /__reset "$(jq -cn --argjson c "$1" --argjson d "$dns" '{config:$c,dns:$d,version:1}')"
}

# --- existing behaviour: audit and apply ---------------------------------
run_script bash "$SCRIPT" audit
expect_rc 0 'audit failed on the default mock state'
audit_output="$OUT"
[[ "$audit_output" != *test-token* ]] || fail_test 'audit output exposed the API token'
[[ "$audit_output" == *'gateway_dns=missing'* ]] || fail_test 'audit did not report missing Gateway DNS'
[[ "$audit_output" == *'legacy_gateway_dns=safe-to-retire'* ]] || fail_test 'audit did not classify the exact legacy DNS record'
[[ "$audit_output" == *'gate_state=unknown'* && "$audit_output" == *'edge_version=1'* && "$audit_output" == *'other_origin_routes=2'* && "$audit_output" == *'oteryn_bypass_rules=1'* && "$audit_output" == *'preserved_rules=3'* ]] || fail_test 'audit did not report gate fields'
[[ "$audit_output" != *old-gateway* && "$audit_output" != *127.0.0.1* && "$audit_output" != *other.molehill* ]] || fail_test 'audit printed topology of unrelated rules'
expect_no_write 0 'audit mode mutated Cloudflare state'

run_script CLOUDFLARE_APPLY_CONFIRMATION=WRONG bash "$SCRIPT" apply
[[ "$RC" != 0 ]] || fail_test 'apply accepted an invalid confirmation phrase'

# apply from unknown (default state: canonical rules missing or wrongly pointed) is refused
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
[[ "$RC" != 0 && "$OUT" == *'apply refused: gate_state=unknown'* ]] || fail_test 'apply from unknown was not refused'
expect_no_write 0 'refused apply wrote to Cloudflare'

# apply migration from the open state (canonical rules on their origins, legacy rule present)
LEGACY_OPEN_CFG="$(jq -c '.ingress = ([.ingress[0], .ingress[1], {"hostname":"login.oteryn.molehill.cloud","service":"http://old-gateway:8080","originRequest":{"connectTimeout":"15s"}}, {"hostname":"other.molehill.cloud","service":"http://svc.example.test:9000"}, {"service":"http_status:404"}])
    | .ingress[1].originRequest = {"connectTimeout":"15s"}' <<<"$OPEN_CFG")"
reset_mock "$LEGACY_OPEN_CFG" default
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
expect_rc 0 'apply from open failed'
[[ "$OUT" != *test-token* && "$OUT" == *'legacy_gateway_dns=missing'* && "$OUT" == *'gate_state=open'* ]] || fail_test 'apply output is wrong'
state="$(mock_state)"
assert_jq "$state" '.mutations == ["tunnel-put", "dns-post:oteryn.molehill.cloud", "dns-post:gateway.molehill.cloud", "dns-delete:login.oteryn.molehill.cloud"]' 'apply did not perform the expected bounded migration'
assert_jq "$state" '.config.ingress[0].service == "http://127.0.0.1:8000" and .config.ingress[1].service == "http://127.0.0.1:8080"' 'apply did not keep canonical rules'
assert_jq "$state" '.config.ingress[1].originRequest.connectTimeout == "15s"' 'apply did not preserve Gateway options'
assert_jq "$state" '[.config.ingress[] | select((.hostname // "") == "login.oteryn.molehill.cloud")] | length == 0' 'apply retained the legacy Gateway ingress'
assert_jq "$state" '.config.ingress[2].hostname == "other.molehill.cloud" and .config.ingress[-1].service == "http_status:404"' 'apply did not preserve unrelated ingress'
assert_jq "$state" '.dns["login.oteryn.molehill.cloud"] == null and .dns["oteryn.molehill.cloud"].proxied == true' 'apply DNS result is wrong'
assert_jq "$state" '.version == 2' 'apply did not increment the version once'
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
expect_rc 0 'second apply failed'
expect_no_write 4 'second apply was not idempotent'

# apply aborts when the edge changes between its reads (an external edit, even identical rules)
reset_mock "$(jq -c '.ingress += [] | .ingress[2].service = "http://svc.example.test:9000"' <<<"$LEGACY_OPEN_CFG")" default
mock_post /__inject "$(jq -cn --argjson c "$LEGACY_OPEN_CFG" '{edit_after_get:1,edit_config:$c,edit_bumps_version:true}')"
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
[[ "$RC" != 0 && "$OUT" == *'changed during planning'* ]] || fail_test 'apply did not abort on an interleaved external edit'
assert_jq "$(mock_state)" '.mutations == ["external-edit"]' 'apply wrote after an interleaved edit'

# --- gate: close, audit, apply refusal, renewal, open ----------------------
TASK_ENV=(OTERYN_TASK_ID="$TASK" GITHUB_REPOSITORY=Oteryn/Oteryn-Platform)
CLOSURE_FILE="$WORK/closure.json"
AUDIT_FILE="$WORK/audit.json"
close_env() { printf '%s\n' CLOUDFLARE_GATE_CONFIRMATION=CLOSE-OTERYN-EDGE "OTERYN_TASK_ID=$TASK" "GITHUB_RUN_ID=$1" GITHUB_RUN_ATTEMPT=1 "OTERYN_CLOSURE_FILE=$CLOSURE_FILE"; }
run_close() { # run_close <run-id> [extra env...]
    local run_id="$1"; shift
    mapfile -t CLOSE_ENV < <(close_env "$run_id")
    run_script "${CLOSE_ENV[@]}" "$@" bash "$SCRIPT" maintenance-close
}
open_env() { printf '%s\n' CLOUDFLARE_GATE_CONFIRMATION=OPEN-OTERYN-EDGE "OTERYN_TASK_ID=$TASK"; }
run_open() { # run_open [extra env...]
    mapfile -t OPEN_ENV < <(open_env)
    run_script "${OPEN_ENV[@]}" "$@" bash "$SCRIPT" maintenance-open
}

reset_mock "$OPEN_CFG"
run_script "${TASK_ENV[@]}" GITHUB_RUN_ID=300 "OTERYN_AUDIT_RESULT_FILE=$AUDIT_FILE" bash "$SCRIPT" audit
expect_rc 0 'audit on open failed'
[[ "$OUT" == *'gate_state=open'* && "$OUT" == *'edge_version=1'* && "$OUT" == *'other_origin_routes=0'* && "$OUT" == *'oteryn_bypass_rules=0'* && "$OUT" == *'preserved_rules=2'* && "$OUT" == *'closure_match=not_requested'* ]] || fail_test "audit gate fields wrong: $OUT"
assert_jq "$(cat "$AUDIT_FILE")" '.schema == "oteryn-edge-audit-result/v2" and .audit_run_id == "300" and .gate_state == "open" and .edge_version == 1 and .other_origin_routes == 0 and .oteryn_bypass_rules == 0 and .preserved_rules.count == 2 and (.preserved_rules.hash | test("^[0-9a-f]{64}$")) and .closure_match == "not_requested" and .task_id == "OTERYN-20261013-p6b"' 'audit.json is wrong'
[[ "$(cat "$AUDIT_FILE")" != *other.molehill* && "$(cat "$AUDIT_FILE")" != *svc.example* ]] || fail_test 'audit.json contains unrelated hostnames'

# refusals before any write
run_close 1001 CLOUDFLARE_GATE_CONFIRMATION=WRONG
[[ "$RC" != 0 ]] || fail_test 'close accepted a wrong confirmation'
run_close 1001 OTERYN_TASK_ID=bad-task
[[ "$RC" != 0 ]] || fail_test 'close accepted a malformed task_id'
run_close 1001 GITHUB_RUN_ATTEMPT=2
[[ "$RC" != 0 ]] || fail_test 'close accepted run_attempt 2'
expect_no_write 0 'refused close wrote'
for rule in '{"hostname":"x.example","service":"http://127.0.0.1:8000"}' '{"hostname":"oteryn.molehill.cloud:8443","service":"https://127.0.0.1:18574"}'; do
    reset_mock "$(routes_for "$rule")"
    run_close 1001
    [[ "$RC" != 0 && "$OUT" == *'oteryn_bypass_rules=1'* && "$OUT" != *x.example* && "$OUT" != *18574* ]] || fail_test "close did not refuse on a bypass rule: $OUT"
    expect_no_write 0 'close with a bypass rule wrote'
    [[ ! -e "$CLOSURE_FILE" ]] || fail_test 'refused close produced a closure'
done
reset_mock "$(jq -c '.ingress = ([{"hostname":"*.molehill.cloud","service":"https://127.0.0.1:18574"}] + .ingress)' <<<"$OPEN_CFG")"
run_close 1001
[[ "$RC" != 0 && "$OUT" == *'oteryn_bypass_rules=1'* ]] || fail_test 'close did not refuse on a wildcard rule that shadows a canonical rule'
expect_no_write 0 'close with a shadowing rule wrote'

# close on a shared tunnel: other services' rules (local, LAN and IP-literal origins) are
# preserved verbatim, in order, with the catch-all; only the two canonical rules change.
reset_mock "$SHARED_CFG"
run_script "${TASK_ENV[@]}" GITHUB_RUN_ID=301 "OTERYN_AUDIT_RESULT_FILE=$AUDIT_FILE" bash "$SCRIPT" audit
expect_rc 0 'audit on a shared open tunnel failed'
SHARED_PRESERVED="$(preserved_rules_hash "$SHARED_CFG")"
assert_jq "$(cat "$AUDIT_FILE")" '.gate_state == "open" and .other_origin_routes == 4 and .oteryn_bypass_rules == 0 and .preserved_rules.count == 6 and .preserved_rules.hash == "'"$SHARED_PRESERVED"'"' 'shared audit.json is wrong'
for leak in ha.example nas.example site.example 192.168 18574 5001 8123 localhost; do
    [[ "$OUT" != *"$leak"* && "$(cat "$AUDIT_FILE")" != *"$leak"* && "$(cat "$SUMMARY_FILE")" != *"$leak"* ]] || fail_test "audit leaked unrelated topology: $leak"
done
run_close 1001
expect_rc 0 'close on a shared tunnel failed'
[[ "$OUT" == *'other_origin_routes=4'* && "$OUT" == *'oteryn_bypass_rules=0'* && "$OUT" == *'preserved_rules=6'* && "$OUT" == *'gate_state=closed'* ]] || fail_test "shared close output wrong: $OUT"
for leak in ha.example nas.example site.example 192.168 18574 5001 8123 localhost; do
    [[ "$OUT" != *"$leak"* ]] || fail_test "close leaked unrelated topology: $leak"
done
state="$(mock_state)"
assert_jq "$state" '.mutations == ["tunnel-put"] and .config.ingress[0].service == "http_status:503" and .config.ingress[1].service == "http_status:503"' 'shared close did not close exactly the canonical rules'
[[ "$(jq -c '.config.ingress[2:]' <<<"$state")" == "$(jq -c '.ingress[2:]' <<<"$SHARED_CFG")" ]] || fail_test 'shared close changed preserved rules'
[[ "$(jq -c '.config.originRequest' <<<"$state")" == "$(jq -c '.originRequest' <<<"$SHARED_CFG")" ]] || fail_test 'shared close changed top-level fields'
assert_jq "$(cat "$CLOSURE_FILE")" '.preserved_rules_count == 6 and .preserved_rules_hash == "'"$SHARED_PRESERVED"'"' 'shared closure does not record the preserved rules'
SHARED_CLOSURE="$(cat "$CLOSURE_FILE")"
fixture run 1001 "$EP" workflow_dispatch main completed success 1
rm -f "$GH_DIR"/*_runs_1001_artifacts.json "$GH_DIR"/*_artifacts_91_zip.zip
fixture artifact 1001 oteryn-edge-gate-closure 91 false closure.json "$SHARED_CLOSURE"
run_script "${TASK_ENV[@]}" OTERYN_CLOSURE_RUN_ID=1001 "OTERYN_AUDIT_RESULT_FILE=$AUDIT_FILE" bash "$SCRIPT" audit
assert_jq "$(cat "$AUDIT_FILE")" '.gate_state == "closed" and .closure_match == "yes" and .preserved_rules.hash == "'"$SHARED_PRESERVED"'"' 'shared closed audit is wrong'
fixture run 2090 "$CP" workflow_dispatch main in_progress null 1
fixture artifact 2090 platform-cutover-gate-evidence 190 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1001",outcome:"rollback_verified",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1001 OTERYN_EVIDENCE_RUN_ID=2090
expect_rc 0 'open on a shared tunnel failed'
[[ "$(jq -c .config <<<"$(mock_state)")" == "$(jq -c . <<<"$SHARED_CFG")" ]] || fail_test 'shared open did not restore the exact original configuration'
# a closure whose preserved hash disagrees with the live edge never opens
reset_mock "$SHARED_CFG"
run_close 1001
expect_rc 0 'second shared close failed'
TAMPERED="$(jq -c --arg h "$(printf 'd%.0s' $(seq 64))" '.preserved_rules_hash = $h' "$CLOSURE_FILE")"
rm -f "$GH_DIR"/*_runs_1001_artifacts.json "$GH_DIR"/*_artifacts_91_zip.zip
fixture artifact 1001 oteryn-edge-gate-closure 91 false closure.json "$TAMPERED"
run_open OTERYN_CLOSURE_RUN_ID=1001 OTERYN_EVIDENCE_RUN_ID=2090
[[ "$RC" != 0 && "$OUT" == *'preserved rules'* ]] || fail_test "open accepted a closure with a different preserved hash: $OUT"
expect_no_write 1 'open with a mismatching preserved hash wrote'
rm -f "$GH_DIR"/*_runs_1001_artifacts.json "$GH_DIR"/*_artifacts_91_zip.zip
reset_mock "$OPEN_CFG"

# local tunnel and split/unknown refusal
reset_mock "$OPEN_CFG"
mock_post /__reset "$(jq -cn --argjson c "$OPEN_CFG" '{config:$c,config_src:"local"}')"
run_close 1001
[[ "$RC" != 0 ]] || fail_test 'close accepted a locally managed tunnel'
reset_mock "$(jq '.ingress[0].service = "http://wrong:1"' <<<"$OPEN_CFG")"
run_close 1001
[[ "$RC" != 0 && "$OUT" == *'gate_state=unknown'* ]] || fail_test 'close accepted an unknown edge'
expect_no_write 0 'close from unknown wrote'

# close from open with a collected fingerprint
reset_mock "$OPEN_CFG"
candidate "$GOOD_HOST" "$GOOD_HOST" >/dev/null
run_close 1001 "OTERYN_FINGERPRINT_FILE=$WORK/candidate.json"
expect_rc 0 'close from open failed'
[[ "$OUT" == *'gate_state=closed'* && "$OUT" == *'edge_version=2'* && "$OUT" == *'mutation=gate-close'* && "$OUT" != *test-token* ]] || fail_test "close output wrong: $OUT"
state="$(mock_state)"
assert_jq "$state" '.mutations == ["tunnel-put"] and .version == 2' 'close did not write exactly once'
assert_jq "$state" '.config.ingress[0].service == "http_status:503" and .config.ingress[1].service == "http_status:503"' 'close did not close both canonical rules'
assert_jq "$state" '.config.ingress[2:] == '"$(jq -c '.ingress[2:]' <<<"$OPEN_CFG")"' and .config.originRequest.connectTimeout == "30s" and .config.ingress[0].originRequest.httpHostHeader == "www"' 'close changed unrelated configuration'
CLOSED_LIVE="$(jq -c .config <<<"$state")"
closure="$(cat "$CLOSURE_FILE")"
assert_jq "$closure" '.schema == "oteryn-edge-gate-closure/v2" and .task_id == "OTERYN-20261013-p6b" and .close_run_id == "1001" and .run_attempt == 1 and .edge_version == 2 and .preserved_rules_count == 2 and .preserved_rules_hash == "'"$(preserved_rules_hash "$OPEN_CFG")"'" and .fingerprint_status == "collected" and (.origin_fingerprint["oteryn.molehill.cloud"] | length) == 2' 'closure.json is wrong'
[[ "$(jq -r .config_hash <<<"$closure")" == "$(sha256_json <<<"$CLOSED_LIVE")" ]] || fail_test 'closure config_hash does not match the live closed edge'

# audit on a closed gate is bounded drift (exit 0); apply refuses; split cannot be applied either
run_script "${TASK_ENV[@]}" "OTERYN_AUDIT_RESULT_FILE=$AUDIT_FILE" bash "$SCRIPT" audit
expect_rc 0 'audit on closed failed'
[[ "$OUT" == *'gate_state=closed'* && "$OUT" == *'tunnel_contract=drift'* ]] || fail_test 'audit on closed is wrong'
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
[[ "$RC" != 0 && "$OUT" == *'apply refused: gate_state=closed'* ]] || fail_test 'apply from closed was not refused'
expect_no_write 1 'apply from closed wrote'

# audit closure_match
fixture run 1001 "$EP" workflow_dispatch main completed success 1
rm -f "$GH_DIR"/*_runs_1001_artifacts.json
fixture artifact 1001 oteryn-edge-gate-closure 91 false closure.json "$closure"
run_script "${TASK_ENV[@]}" OTERYN_CLOSURE_RUN_ID=1001 "OTERYN_AUDIT_RESULT_FILE=$AUDIT_FILE" bash "$SCRIPT" audit
[[ "$OUT" == *'closure_match=yes'* ]] || fail_test "closure_match should be yes: $OUT"
assert_jq "$(cat "$AUDIT_FILE")" '.closure_match == "yes" and .closure_run_id == "1001"' 'audit.json closure_match is wrong'
run_script OTERYN_TASK_ID=OTERYN-20261013-other GITHUB_REPOSITORY=Oteryn/Oteryn-Platform OTERYN_CLOSURE_RUN_ID=1001 bash "$SCRIPT" audit
[[ "$OUT" == *'closure_match=no'* ]] || fail_test 'closure_match should be no for another task'
run_script "${TASK_ENV[@]}" OTERYN_CLOSURE_RUN_ID=4040 bash "$SCRIPT" audit
[[ "$OUT" == *'closure_match=no'* ]] || fail_test 'closure_match should be no for an unknown run'

# split cannot be closed from apply and open refuses
mock_post /__external_edit "$(jq -cn --argjson c "$(set_gate_services "$OPEN_CFG" "$GATE_SERVICE" "$GATEWAY_SERVICE")" '{config:$c}')"
run_script CLOUDFLARE_APPLY_CONFIRMATION=APPLY-OTERYN-CLOUDFLARE bash "$SCRIPT" apply
[[ "$RC" != 0 && "$OUT" == *'gate_state=split'* ]] || fail_test 'apply from split was not refused'
run_open OTERYN_CLOSURE_RUN_ID=1001 OTERYN_EVIDENCE_RUN_ID=2001
[[ "$RC" != 0 && "$OUT" == *'gate_state=split'* ]] || fail_test 'open from split was not refused'
# a split close completes the close (fingerprint unavailable: application not fully reachable)
run_close 1005 "OTERYN_FINGERPRINT_FILE=$WORK/candidate.json"
expect_rc 0 'close from split failed'
assert_jq "$(cat "$CLOSURE_FILE")" '.fingerprint_status == "unavailable" and .origin_fingerprint == null and .edge_version == 4' 'split close closure is wrong'
# now closed again at version 4 (1001 closure is stale); restore the version-2 scenario
reset_mock "$OPEN_CFG"
run_close 1001 "OTERYN_FINGERPRINT_FILE=$WORK/candidate.json"
expect_rc 0 'close failed after reset'
closure="$(cat "$CLOSURE_FILE")"
rm -f "$GH_DIR"/*_runs_1001_artifacts.json "$GH_DIR"/*_artifacts_91_zip.zip
fixture artifact 1001 oteryn-edge-gate-closure 91 false closure.json "$closure"

# renewal: needs closure_run_id and a verifiable previous closure
run_close 1010
[[ "$RC" != 0 && "$OUT" == *'requires closure_run_id'* ]] || fail_test 'renewal without closure_run_id was not refused'
run_close 1010 OTERYN_CLOSURE_RUN_ID=4040
[[ "$RC" != 0 ]] || fail_test 'renewal with an unreadable closure was not refused'
run_close 1010 OTERYN_CLOSURE_RUN_ID=1001 OTERYN_TASK_ID=OTERYN-20261013-other
[[ "$RC" != 0 ]] || fail_test 'renewal for another task was not refused'
expect_no_write 1 'refused renewal wrote'
run_close 1010 OTERYN_CLOSURE_RUN_ID=1001
expect_rc 0 'renewal failed'
[[ "$OUT" == *'mutation=gate-renew'* && "$OUT" == *'edge_version=3'* ]] || fail_test 'renewal output wrong'
assert_jq "$(mock_state)" '.version == 3 and .mutations == ["tunnel-put","tunnel-put"]' 'renewal did not perform exactly one write'
[[ "$(jq -c .config <<<"$(mock_state)")" == "$CLOSED_LIVE" ]] || fail_test 'renewal changed the configuration'
RENEWED="$(cat "$CLOSURE_FILE")"
assert_jq "$RENEWED" '.close_run_id == "1010" and .edge_version == 3 and .fingerprint_status == "collected"' 'renewed closure is wrong'
[[ "$(jq -c .origin_fingerprint <<<"$RENEWED")" == "$(jq -c .origin_fingerprint <<<"$closure")" ]] || fail_test 'renewal did not carry the fingerprint forward'
# the superseded closure can no longer renew or open
run_close 1011 OTERYN_CLOSURE_RUN_ID=1001
[[ "$RC" != 0 && "$OUT" == *'no longer matches'* ]] || fail_test 'a superseded closure renewed the gate'
fixture run 1010 "$EP" workflow_dispatch main completed success 1
fixture artifact 1010 oteryn-edge-gate-closure 94 false closure.json "$RENEWED"
fixture run 2001 "$CP" workflow_dispatch main in_progress null 1
fixture artifact 2001 platform-cutover-gate-evidence 95 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1001",outcome:"cutover_complete",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1001 OTERYN_EVIDENCE_RUN_ID=2001
[[ "$RC" != 0 ]] || fail_test 'open accepted a superseded closure with its old evidence'
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2001
[[ "$RC" != 0 && "$OUT" == *'close_run_id mismatch'* ]] || fail_test 'open accepted evidence naming an earlier close'
run_open OTERYN_CLOSURE_RUN_ID=1010
[[ "$RC" != 0 ]] || fail_test 'open without evidence_run_id was not refused'
expect_no_write 2 'refused open wrote'
# evidence run states outside the allowlist, attempt above 1, other task, probe-failed closure
fixture run 2002 "$CP" workflow_dispatch main completed failure 1
fixture artifact 2002 platform-cutover-gate-evidence 96 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1010",outcome:"cutover_complete",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2002
[[ "$RC" != 0 ]] || fail_test 'open accepted a failed evidence run'
fixture run 2003 "$CP" workflow_dispatch main completed success 2
fixture artifact 2003 platform-cutover-gate-evidence 97 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1010",outcome:"cutover_complete",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2003
[[ "$RC" != 0 ]] || fail_test 'open accepted evidence from run_attempt 2'
fixture run 2004 "$CP" workflow_dispatch main in_progress null 1
fixture artifact 2004 platform-cutover-gate-evidence 98 false gate-evidence.json "$(jq -cn '{schema:"s/v1",task_id:"OTERYN-20261013-other",close_run_id:"1010",outcome:"cutover_complete",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2004
[[ "$RC" != 0 ]] || fail_test 'open accepted evidence from another task'
fixture run 2005 "$CP" workflow_dispatch main completed success 1
fixture artifact 2005 platform-cutover-gate-evidence 99 true gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1010",outcome:"cutover_complete",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2005
[[ "$RC" != 0 ]] || fail_test 'open accepted an expired evidence artifact'
fixture run 2006 "$CP" workflow_dispatch main completed success 1
fixture artifact 2006 platform-cutover-gate-evidence 100 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1010",outcome:"stopped",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2006
[[ "$RC" != 0 ]] || fail_test 'open accepted evidence with a stopped outcome'
expect_no_write 2 'refused open wrote'

# open aborts when the edge is edited between the classification read and the PUT
fixture run 2010 "$CP" workflow_dispatch main in_progress null 1
fixture artifact 2010 platform-cutover-gate-evidence 101 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1010",outcome:"rollback_verified",head_sha:"x"}')"
mock_post /__inject "$(jq -cn --argjson c "$CLOSED_LIVE" '{edit_after_get:1,edit_config:$c,edit_bumps_version:true}')"
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2010
[[ "$RC" != 0 && "$OUT" == *'edge changed before the write'* ]] || fail_test "open did not abort on a version-only external edit: $OUT"
assert_jq "$(mock_state)" '[.mutations[] | select(. == "tunnel-put")] | length == 2' 'open wrote after a version-only external edit'
# the external edit raised the version: the closure is permanently stale, even for the identical rules
run_open OTERYN_CLOSURE_RUN_ID=1010 OTERYN_EVIDENCE_RUN_ID=2010
[[ "$RC" != 0 && "$OUT" == *'no longer matches'* ]] || fail_test 'a stale closure opened the gate after an external edit'
# back to a clean closed gate at a fresh version to complete the open
reset_mock "$OPEN_CFG"
run_close 1020 "OTERYN_FINGERPRINT_FILE=$WORK/candidate.json"
expect_rc 0 'close 1020 failed'
fixture run 1020 "$EP" workflow_dispatch main completed success 1
fixture artifact 1020 oteryn-edge-gate-closure 110 false closure.json "$(cat "$CLOSURE_FILE")"
fixture run 2020 "$CP" workflow_dispatch main in_progress null 1
fixture artifact 2020 platform-cutover-gate-evidence 111 false gate-evidence.json "$(jq -cn --arg task "$TASK" '{schema:"s/v1",task_id:$task,close_run_id:"1020",outcome:"rollback_verified",head_sha:"x"}')"
run_open OTERYN_CLOSURE_RUN_ID=1020 OTERYN_EVIDENCE_RUN_ID=2020
expect_rc 0 'open with valid closure and evidence failed'
[[ "$OUT" == *'gate_state=open'* && "$OUT" == *'mutation=gate-open'* && "$OUT" == *'edge_version=3'* && "$OUT" == *'tunnel_contract=current'* ]] || fail_test "open output wrong: $OUT"
state="$(mock_state)"
assert_jq "$state" '.version == 3 and .config.ingress[0].service == "http://127.0.0.1:8000" and .config.ingress[1].service == "http://127.0.0.1:8080"' 'open did not restore the fixed origins'
[[ "$(jq -c .config <<<"$state")" == "$(jq -c . <<<"$OPEN_CFG")" ]] || fail_test 'open did not restore the exact original configuration'
# retry from open: no write, no ids needed
run_open
expect_rc 0 'open from open (retry) failed'
[[ "$OUT" == *'mutation=none'* ]] || fail_test 'idempotent open wrote'
assert_jq "$(mock_state)" '.version == 3' 'idempotent open changed the version'
# old closure and its evidence do not open a gate that was closed again after a successful open
run_close 1030 "OTERYN_FINGERPRINT_FILE=$WORK/candidate.json"
expect_rc 0 'second close after open failed'
run_open OTERYN_CLOSURE_RUN_ID=1020 OTERYN_EVIDENCE_RUN_ID=2020
[[ "$RC" != 0 ]] || fail_test 'old closure and evidence reopened a re-closed gate'

# close: stale-read aborts (unrelated edit, version-only edit), PUT failures, read-back failures
reset_mock "$OPEN_CFG"
EDITED="$(jq '.ingress[2].originRequest.connectTimeout = "99s"' <<<"$OPEN_CFG")"
mock_post /__inject "$(jq -cn --argjson c "$EDITED" '{edit_after_get:1,edit_config:$c}')"
run_close 1040
[[ "$RC" != 0 && "$OUT" == *'nothing was written'* ]] || fail_test 'close did not abort on an unrelated interleaved edit'
assert_jq "$(mock_state)" '.mutations == ["external-edit"] and .config.ingress[2].originRequest.connectTimeout == "99s"' 'close overwrote an unrelated interleaved edit'
reset_mock "$OPEN_CFG"
mock_post /__inject "$(jq -cn --argjson c "$OPEN_CFG" '{edit_after_get:1,edit_config:$c}')"
run_close 1041
[[ "$RC" != 0 && "$OUT" == *'nothing was written'* ]] || fail_test 'close did not abort on a version-only interleaved edit'
assert_jq "$(mock_state)" '.mutations == ["external-edit"]' 'close wrote after a version-only edit'
[[ ! -e "$CLOSURE_FILE" ]] || fail_test 'aborted close left a closure'

reset_mock "$OPEN_CFG"
mock_post /__inject '{"put_mode":"fail_500"}'
run_close 1042
[[ "$RC" != 0 && ! -e "$CLOSURE_FILE" ]] || fail_test 'a failed PUT produced success or a closure'
assert_jq "$(mock_state)" '.mutations == []' 'failed PUT changed state'
reset_mock "$OPEN_CFG"
mock_post /__inject '{"put_mode":"applied_then_500"}'
run_close 1043
[[ "$RC" != 0 && ! -e "$CLOSURE_FILE" ]] || fail_test 'an ambiguous PUT produced success or a closure'
mock_post /__inject '{}'
run_close 1044
[[ "$RC" != 0 && "$OUT" == *'requires closure_run_id'* ]] || fail_test 'a closed edge without closure was renewed'
reset_mock "$OPEN_CFG"
mock_post /__inject '{"put_version_bump":2}'
run_close 1045
[[ "$RC" != 0 && "$OUT" == *'plus one'* && ! -e "$CLOSURE_FILE" ]] || fail_test "a wrong version increment was reported as success: $OUT"
reset_mock "$OPEN_CFG"
mock_post /__inject '{"readback_fail":true}'
run_close 1046
[[ "$RC" != 0 && ! -e "$CLOSURE_FILE" ]] || fail_test 'a read-back failure was reported as success'
reset_mock "$OPEN_CFG"
mock_post /__inject '{"omit_version":true}'
run_close 1047
[[ "$RC" != 0 && "$OUT" == *'gate_state=unknown'* ]] || fail_test 'a missing version did not stop the close'
expect_no_write 0 'close without a usable version wrote'

# --- secret-free fingerprint and probes -------------------------------------
PROBE_ENV=("OTERYN_PROBE_ORIGIN=http://127.0.0.1:${MOCK_PORT}")
set_probe() { mock_post /__reset "$(jq -cn --argjson p "$1" '{probe:$p}')"; }
set_probe '{"oteryn.molehill.cloud":{"status":200,"headers":{"X-App":"1","Server":"nginx","Content-Type":"text/html"},"cookies":["oteryn_sess","session"]},"gateway.molehill.cloud":{"status":200,"headers":{"X-Gw":"1"},"cookies":["gw_token"]},"*":{"status":404,"headers":{"X-Generic":"1"},"cookies":[]}}'
env "${PROBE_ENV[@]}" "OTERYN_FINGERPRINT_OUT=$WORK/fp.json" bash "$SCRIPT" edge-fingerprint >/dev/null
assert_jq "$(cat "$WORK/fp.json")" '.hosts["oteryn.molehill.cloud"].elements == ["cookie:oteryn_sess","header:x-app"] and .hosts["gateway.molehill.cloud"].elements == ["cookie:gw_token","header:x-gw"]' 'fingerprint candidate collected the wrong elements'
assert_jq "$(cat "$WORK/fp.json")" 'tostring | (contains("nginx") or contains("text/html")) | not' 'fingerprint candidate recorded a value'
[[ "$(evaluate_fingerprint_candidate "$WORK/fp.json" | jq -r .status)" == collected ]] || fail_test 'collected candidate is not accepted'

printf '%s' "$(jq -cn --arg task "$TASK" --argjson fp "$(evaluate_fingerprint_candidate "$WORK/fp.json" | jq -c .fingerprint)" '{fingerprint_status:"collected",origin_fingerprint:$fp}')" >"$WORK/probe-closure.json"
printf '%s' '{"fingerprint_status":"unavailable","origin_fingerprint":null}' >"$WORK/probe-unavailable.json"
CLOSED_PROBE='"oteryn.molehill.cloud":{"status":503,"headers":{"Server":"cloudflare"},"cookies":[]},"gateway.molehill.cloud":{"status":503,"headers":{"Server":"cloudflare"},"cookies":[]}'
probe_closed() { env "${PROBE_ENV[@]}" "OTERYN_CLOSURE_FILE=$1" bash "$SCRIPT" edge-probe closed >"$WORK/probe.out" 2>&1; }
set_probe "{$CLOSED_PROBE,\"*\":{\"status\":200,\"headers\":{\"X-Other\":\"1\"},\"cookies\":[\"other\"]}}"
probe_closed "$WORK/probe-closure.json" || fail_test 'closed probe failed on a legitimate unrelated 200'
set_probe "{$CLOSED_PROBE,\"*\":{\"status\":200,\"headers\":{\"X-App\":\"1\"},\"cookies\":[\"session\"]}}"
probe_closed "$WORK/probe-closure.json" || fail_test 'a single shared header name must not fail the probe'
set_probe "{$CLOSED_PROBE,\"*\":{\"status\":200,\"headers\":{\"X-App\":\"1\"},\"cookies\":[\"oteryn_sess\"]}}"
if probe_closed "$WORK/probe-closure.json"; then fail_test 'a full recorded application response did not fail the probe'; fi
probe_closed "$WORK/probe-unavailable.json" || fail_test 'probe must be informational when the fingerprint is unavailable'
grep -q 'wildcard_probe=informational' "$WORK/probe.out" || fail_test 'informational probe was not logged'
set_probe '{"oteryn.molehill.cloud":{"status":200,"headers":{},"cookies":[]},"gateway.molehill.cloud":{"status":503,"headers":{"Server":"cloudflare"},"cookies":[]},"*":{"status":404,"headers":{},"cookies":[]}}'
if probe_closed "$WORK/probe-closure.json"; then fail_test 'a canonical host not answering 503 did not fail the closed probe'; fi
set_probe '{"oteryn.molehill.cloud":{"status":200,"headers":{},"cookies":[]},"gateway.molehill.cloud":{"status":302,"headers":{},"cookies":[]}}'
env "${PROBE_ENV[@]}" bash "$SCRIPT" edge-probe open >/dev/null || fail_test 'open probe failed on non-503 responses'
set_probe '{"oteryn.molehill.cloud":{"status":503,"headers":{},"cookies":[]},"gateway.molehill.cloud":{"status":200,"headers":{},"cookies":[]}}'
if env "${PROBE_ENV[@]}" bash "$SCRIPT" edge-probe open >/dev/null 2>&1; then fail_test 'open probe accepted a 503'; fi

[[ "$(cat "$ALL_OUT")" != *test-token* ]] || fail_test 'a script output exposed the API token'
[[ "$(cat "$SUMMARY_FILE")" != *test-token* ]] || fail_test 'step summary exposed the API token'

python3 "$TESTS/check-gate-workflow.py"

printf 'Cloudflare endpoint migration tests: PASS\n'
