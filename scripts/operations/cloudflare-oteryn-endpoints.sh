#!/usr/bin/env bash
set -euo pipefail

readonly CLOUDFLARE_API_BASE_URL="${CLOUDFLARE_API_BASE_URL:-https://api.cloudflare.com/client/v4}"
readonly CURL_BIN="${CLOUDFLARE_CURL_BIN:-curl}"
readonly WWW_HOST="oteryn.molehill.cloud"
readonly WWW_SERVICE="http://127.0.0.1:8000"
readonly GATEWAY_HOST="gateway.molehill.cloud"
readonly GATEWAY_SERVICE="http://127.0.0.1:8080"
readonly LEGACY_GATEWAY_HOST="login.oteryn.molehill.cloud"
readonly APPLY_CONFIRMATION="APPLY-OTERYN-CLOUDFLARE"

TEMP_FILES=()

cleanup() {
    if ((${#TEMP_FILES[@]} > 0)); then
        rm -f -- "${TEMP_FILES[@]}"
    fi
}
trap cleanup EXIT

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    return 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command is unavailable: $1"
}

require_nonempty_env() {
    local name="$1"
    [[ -n "${!name:-}" ]] || fail "required environment variable is missing: $name"
}

canonical_json() {
    jq -cS .
}

sha256_json() {
    canonical_json | sha256sum | awk '{print $1}'
}

append_summary() {
    local line="$1"
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        printf '%s\n' "$line" >>"$GITHUB_STEP_SUMMARY"
    fi
}

api_error_summary() {
    local response_file="$1"
    if jq -e . "$response_file" >/dev/null 2>&1; then
        jq -c '{success: (.success // false), errors: (.errors // []), messages: (.messages // [])}' "$response_file" >&2
    else
        printf 'Cloudflare API returned a non-JSON response.\n' >&2
    fi
}

api_request() {
    local method="$1"
    local path="$2"
    local body="${3:-}"
    local response_file http_code
    local -a args

    response_file="$(mktemp)"
    TEMP_FILES+=("$response_file")

    args=(
        --silent
        --show-error
        --connect-timeout 10
        --max-time 30
        --request "$method"
        --header "Authorization: Bearer ${CLOUDFLARE_API_TOKEN}"
        --header 'Content-Type: application/json'
        --output "$response_file"
        --write-out '%{http_code}'
        "${CLOUDFLARE_API_BASE_URL}${path}"
    )

    if [[ -n "$body" ]]; then
        args+=(--data-binary "$body")
    fi

    if ! http_code="$($CURL_BIN "${args[@]}")"; then
        fail "Cloudflare API transport failed for ${method} ${path}"
        return 1
    fi

    if [[ ! "$http_code" =~ ^2[0-9][0-9]$ ]]; then
        printf 'Cloudflare API HTTP %s for %s %s.\n' "$http_code" "$method" "$path" >&2
        api_error_summary "$response_file"
        return 1
    fi

    if ! jq -e '.success == true' "$response_file" >/dev/null; then
        printf 'Cloudflare API reported failure for %s %s.\n' "$method" "$path" >&2
        api_error_summary "$response_file"
        return 1
    fi

    cat "$response_file"
}

validate_tunnel_config() {
    local config_json="$1"
    local catchall_count host host_count host_path_count

    if ! jq -e 'type == "object" and (.ingress | type == "array") and (.ingress | length > 0)' <<<"$config_json" >/dev/null; then
        fail "tunnel configuration must contain a non-empty ingress array"
        return 1
    fi

    catchall_count="$(jq '[.ingress[] | select(((.hostname // "") == "") and ((.path // "") == ""))] | length' <<<"$config_json")"
    if [[ "$catchall_count" != "1" ]]; then
        fail "tunnel configuration must contain exactly one pathless catch-all rule"
        return 1
    fi

    if ! jq -e '.ingress[-1] | ((.hostname // "") == "") and ((.path // "") == "")' <<<"$config_json" >/dev/null; then
        fail "the pathless catch-all rule must be the final ingress rule"
        return 1
    fi

    for host in "$WWW_HOST" "$GATEWAY_HOST" "$LEGACY_GATEWAY_HOST"; do
        host_count="$(jq --arg host "$host" '[.ingress[] | select((.hostname // "") == $host)] | length' <<<"$config_json")"
        if ((host_count > 1)); then
            fail "duplicate tunnel ingress rules exist for ${host}"
            return 1
        fi

        host_path_count="$(jq --arg host "$host" '[.ingress[] | select((.hostname // "") == $host and ((.path // "") != ""))] | length' <<<"$config_json")"
        if [[ "$host_path_count" != "0" ]]; then
            fail "path-scoped ingress rule exists for managed hostname ${host}"
            return 1
        fi
    done
}

build_rule() {
    local config_json="$1"
    local host="$2"
    local service="$3"
    local fallback_host="${4:-}"
    local existing

    existing="$(jq -c --arg host "$host" '[.ingress[] | select((.hostname // "") == $host)][0] // {}' <<<"$config_json")"
    if [[ "$existing" == "{}" && -n "$fallback_host" ]]; then
        existing="$(jq -c --arg host "$fallback_host" '[.ingress[] | select((.hostname // "") == $host)][0] // {}' <<<"$config_json")"
    fi

    if [[ "$existing" == "{}" ]]; then
        jq -cn --arg host "$host" --arg service "$service" '{hostname: $host, service: $service}'
    else
        jq -c --arg host "$host" --arg service "$service" 'del(.path) | .hostname = $host | .service = $service' <<<"$existing"
    fi
}

build_desired_config() {
    local config_json="$1"
    local www_rule gateway_rule

    validate_tunnel_config "$config_json" || return 1
    www_rule="$(build_rule "$config_json" "$WWW_HOST" "$WWW_SERVICE")"
    gateway_rule="$(build_rule "$config_json" "$GATEWAY_HOST" "$GATEWAY_SERVICE" "$LEGACY_GATEWAY_HOST")"

    jq -c \
        --arg www_host "$WWW_HOST" \
        --arg gateway_host "$GATEWAY_HOST" \
        --arg legacy_gateway_host "$LEGACY_GATEWAY_HOST" \
        --argjson www_rule "$www_rule" \
        --argjson gateway_rule "$gateway_rule" '
        . as $config
        | (.ingress
            | map(select(
                ((.hostname // "") != $www_host)
                and ((.hostname // "") != $gateway_host)
                and ((.hostname // "") != $legacy_gateway_host)
            ))) as $without_managed
        | ($without_managed[-1]) as $catchall
        | ($without_managed[0:-1]) as $other_rules
        | $config + {ingress: ([$www_rule, $gateway_rule] + $other_rules + [$catchall])}
    ' <<<"$config_json"
}

normalize_dns_name() {
    local value="$1"
    value="${value%.}"
    printf '%s' "${value,,}"
}

fetch_dns_records() {
    local host="$1"
    api_request GET "/zones/${CLOUDFLARE_ZONE_ID}/dns_records?name.exact=${host}&per_page=100"
}

dns_record_state() {
    local records_response="$1"
    local host="$2"
    local target="$3"
    local count record_type record_name record_content proxied

    if ! jq -e '.result | type == "array"' <<<"$records_response" >/dev/null; then
        fail "DNS response for ${host} has no result array"
        return 1
    fi
    count="$(jq '.result | length' <<<"$records_response")"

    if [[ "$count" == "0" ]]; then
        printf 'missing'
        return 0
    fi
    if [[ "$count" != "1" ]]; then
        fail "multiple DNS records exist for managed hostname ${host}"
        return 1
    fi

    record_type="$(jq -r '.result[0].type // ""' <<<"$records_response")"
    record_name="$(jq -r '.result[0].name // ""' <<<"$records_response")"
    record_content="$(jq -r '.result[0].content // ""' <<<"$records_response")"
    proxied="$(jq -r '.result[0].proxied // false' <<<"$records_response")"

    if [[ "$record_type" != "CNAME" ]]; then
        fail "conflicting ${record_type:-unknown} DNS record exists for ${host}"
        return 1
    fi
    if [[ "$(normalize_dns_name "$record_name")" != "$(normalize_dns_name "$host")" ]]; then
        fail "Cloudflare returned a non-exact DNS record for ${host}"
        return 1
    fi

    if [[ "$(normalize_dns_name "$record_content")" == "$(normalize_dns_name "$target")" && "$proxied" == "true" ]]; then
        printf 'current'
    else
        printf 'drift'
    fi
}

legacy_dns_state() {
    local records_response="$1"
    local target="$2"
    local count record_type record_name record_content

    if ! jq -e '.result | type == "array"' <<<"$records_response" >/dev/null; then
        fail "legacy DNS response has no result array"
        return 1
    fi
    count="$(jq '.result | length' <<<"$records_response")"

    if [[ "$count" == "0" ]]; then
        printf 'missing'
        return 0
    fi
    if [[ "$count" != "1" ]]; then
        fail "multiple DNS records exist for retired hostname ${LEGACY_GATEWAY_HOST}"
        return 1
    fi

    record_type="$(jq -r '.result[0].type // ""' <<<"$records_response")"
    record_name="$(jq -r '.result[0].name // ""' <<<"$records_response")"
    record_content="$(jq -r '.result[0].content // ""' <<<"$records_response")"

    if [[ "$record_type" != "CNAME" ]]; then
        fail "refusing to retire non-CNAME legacy DNS record for ${LEGACY_GATEWAY_HOST}"
        return 1
    fi
    if [[ "$(normalize_dns_name "$record_name")" != "$(normalize_dns_name "$LEGACY_GATEWAY_HOST")" ]]; then
        fail "Cloudflare returned a non-exact legacy DNS record"
        return 1
    fi
    if [[ "$(normalize_dns_name "$record_content")" != "$(normalize_dns_name "$target")" ]]; then
        fail "legacy DNS record points outside the managed Oteryn tunnel"
        return 1
    fi

    printf 'safe-to-retire'
}

upsert_dns_record() {
    local host="$1"
    local target="$2"
    local records_response="$3"
    local count record_id body

    count="$(jq '.result | length' <<<"$records_response")"
    body="$(jq -cn --arg host "$host" --arg target "$target" '{type: "CNAME", name: $host, content: $target, proxied: true}')"

    if [[ "$count" == "0" ]]; then
        api_request POST "/zones/${CLOUDFLARE_ZONE_ID}/dns_records" "$body" >/dev/null
        return 0
    fi

    [[ "$count" == "1" ]] || fail "refusing to update ambiguous DNS state for ${host}"
    record_id="$(jq -r '.result[0].id // ""' <<<"$records_response")"
    [[ -n "$record_id" ]] || fail "existing DNS record for ${host} has no record ID"
    api_request PATCH "/zones/${CLOUDFLARE_ZONE_ID}/dns_records/${record_id}" "$body" >/dev/null
}

retire_legacy_dns_record() {
    local records_response="$1"
    local target="$2"
    local state record_id

    state="$(legacy_dns_state "$records_response" "$target")"
    if [[ "$state" == "missing" ]]; then
        return 0
    fi
    [[ "$state" == "safe-to-retire" ]] || fail "legacy DNS record is not safe to retire"

    record_id="$(jq -r '.result[0].id // ""' <<<"$records_response")"
    [[ -n "$record_id" ]] || fail "legacy DNS record has no record ID"
    api_request DELETE "/zones/${CLOUDFLARE_ZONE_ID}/dns_records/${record_id}" >/dev/null
}

validate_identifiers() {
    [[ "$CLOUDFLARE_ACCOUNT_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ACCOUNT_ID must be a 32-character hexadecimal identifier"
    [[ "$CLOUDFLARE_ZONE_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ZONE_ID must be a 32-character hexadecimal identifier"
    [[ "$CLOUDFLARE_TUNNEL_ID" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$ ]] || fail "CLOUDFLARE_TUNNEL_ID must be a UUID"
}

record_mutation() {
    local current="$1"
    local item="$2"
    if [[ "$current" == "none" ]]; then
        printf '%s' "$item"
    else
        printf '%s+%s' "$current" "$item"
    fi
}

# ---------------------------------------------------------------------------
# Maintenance gate (P6b). See docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md.
# Pure functions first (no network, no Cloudflare access), then the flows.
# ---------------------------------------------------------------------------

readonly CLOSE_CONFIRMATION="CLOSE-OTERYN-EDGE"
readonly OPEN_CONFIRMATION="OPEN-OTERYN-EDGE"
readonly GATE_SERVICE="http_status:503"
readonly TASK_ID_REGEX='^OTERYN-[0-9]{8}-[a-z0-9-]+$'
readonly RUN_ID_REGEX='^[1-9][0-9]{0,19}$'
readonly ENDPOINTS_WORKFLOW_PATH=".github/workflows/cloudflare-oteryn-endpoints.yml"
readonly CUTOVER_WORKFLOW_PATH=".github/workflows/deploy-synology-staging.yml"
readonly CLOSURE_ARTIFACT="oteryn-edge-gate-closure"
readonly EVIDENCE_ARTIFACT="platform-cutover-gate-evidence"
readonly CLOSURE_SCHEMA="oteryn-edge-gate-closure/v1"
readonly AUDIT_RESULT_SCHEMA="oteryn-edge-audit-result/v1"
readonly FINGERPRINT_SCHEMA="oteryn-edge-fingerprint-candidate/v1"
readonly ORIGIN_MARKER_ELEMENT="header:x-oteryn-origin"
readonly GH_BIN="${OTERYN_GH_BIN:-gh}"

# Allowed-host constant: deliberately empty. An entry is a reviewed, owner-approved
# exception documented in CLOUDFLARE_ENDPOINT_MANAGEMENT.md. It is never an input
# (bash arrays cannot be imported from the environment).
ALLOWED_ORIGIN_HOSTS=()

EDGE_CONFIG=""
EDGE_VERSION=""
EDGE_HASH=""
EDGE_STATE="unknown"
TUNNEL_STATE="unknown"

gh_api() {
    "$GH_BIN" api "$@"
}

# --- classification -------------------------------------------------------

# Prints open|closed|split|unknown for a tunnel configuration JSON document.
classify_gate_state() {
    local config_json="$1" state
    if ! validate_tunnel_config "$config_json" >/dev/null 2>&1; then
        printf 'unknown'
        return 0
    fi
    state="$(jq -r \
        --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" \
        --arg www_origin "$WWW_SERVICE" --arg gateway_origin "$GATEWAY_SERVICE" \
        --arg closed "$GATE_SERVICE" '
        def norm: ((.hostname // "") | ascii_downcase | sub("\\.+$"; ""));
        [.ingress[] | select(type == "object")] as $rules
        | ([$rules[] | select(norm == $www)]) as $w
        | ([$rules[] | select(norm == $gateway)]) as $g
        | if ($w | length) != 1 or ($g | length) != 1 or (($w[0].path // "") != "") or (($g[0].path // "") != "") then "unknown"
          else ($w[0].service // "") as $ws | ($g[0].service // "") as $gs
            | if $ws == $www_origin and $gs == $gateway_origin then "open"
              elif $ws == $closed and $gs == $closed then "closed"
              elif ($ws == $closed and $gs == $gateway_origin) or ($ws == $www_origin and $gs == $closed) then "split"
              else "unknown" end
          end' <<<"$config_json" 2>/dev/null)" || state="unknown"
    case "$state" in
        open | closed | split | unknown) printf '%s' "$state" ;;
        *) printf 'unknown' ;;
    esac
}

# Sets the service of the two canonical rules; everything else is preserved byte for byte.
set_gate_services() {
    local config_json="$1" www_service="$2" gateway_service="$3"
    jq -c \
        --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" \
        --arg www_service "$www_service" --arg gateway_service "$gateway_service" '
        def norm: ((.hostname // "") | ascii_downcase | sub("\\.+$"; ""));
        .ingress |= map(
            if type != "object" then .
            elif norm == $www then .service = $www_service
            elif norm == $gateway then .service = $gateway_service
            else . end)' <<<"$config_json"
}

# Hash of the configuration with the two canonical `service` values blanked: proves that
# every unrelated rule and every other field of the canonical rules is unchanged.
config_shape_hash() {
    set_gate_services "$1" "" "" | sha256_json
}

# --- origin routes --------------------------------------------------------

# Prints "<host>\t<port>" (host lowercased, trailing dots stripped, brackets kept for
# IPv6, userinfo removed, default port applied). Returns 1 when unparseable.
parse_service_origin() {
    local service="$1" scheme rest authority hostport host port
    [[ "$service" =~ ^([A-Za-z][A-Za-z0-9+.-]*)://(.*)$ ]] || return 1
    scheme="${BASH_REMATCH[1],,}"
    rest="${BASH_REMATCH[2]}"
    authority="${rest%%[/?#]*}"
    [[ -n "$authority" ]] || return 1
    hostport="${authority##*@}"
    if [[ "$hostport" == \[* ]]; then
        [[ "$hostport" =~ ^\[([0-9A-Fa-f:.]+)\](:([0-9]*))?$ ]] || return 1
        host="[${BASH_REMATCH[1],,}]"
        port="${BASH_REMATCH[3]:-}"
    else
        [[ "$hostport" =~ ^([^:]+)(:([0-9]*))?$ ]] || return 1
        host="${BASH_REMATCH[1],,}"
        while [[ "$host" == *. ]]; do host="${host%.}"; done
        port="${BASH_REMATCH[3]:-}"
    fi
    [[ -n "$host" ]] || return 1
    if [[ -z "$port" ]]; then
        case "$scheme" in
            http | ws) port=80 ;;
            https | wss) port=443 ;;
            *) port="" ;;
        esac
    else
        ((${#port} <= 5)) || return 1
        port=$((10#$port))
        ((port <= 65535)) || return 1
    fi
    printf '%s\t%s' "$host" "$port"
}

# True for a bracketed IPv6 literal or any host whose last label is numeric or 0x-hex
# (decimal, octal, hex and dotted IPv4 spellings). Such a host is never judged by spelling.
is_ip_literal_host() {
    local host="$1" last
    [[ "$host" == \[* ]] && return 0
    last="${host##*.}"
    [[ "$last" =~ ^[0-9]+$ || "$last" =~ ^0[xX][0-9a-fA-F]*$ ]]
}

is_local_host_name() {
    local host="$1"
    [[ "$host" == localhost || "$host" == *.localhost ]]
}

# Canonical text form of an IP literal (IPv6 compression, IPv4-mapped/compatible, dotted,
# decimal, octal and hex IPv4). Prints nothing and returns 1 when it is not an IP literal.
canonical_ip_literal() {
    python3 -I - "$1" <<'PY'
import ipaddress
import socket
import sys

host = sys.argv[1]
try:
    if host.startswith("["):
        address = ipaddress.ip_address(host[1:-1])
    else:
        address = ipaddress.ip_address(socket.inet_ntoa(socket.inet_aton(host)))
    if address.version == 6:
        if address.ipv4_mapped is not None:
            address = address.ipv4_mapped
        elif address.packed[:12] == bytes(12) and int(address) > 1:
            address = ipaddress.ip_address(address.packed[12:])
    print(str(address))
except (ValueError, OSError):
    sys.exit(1)
PY
}

host_is_allowed() {
    local host="$1" entry host_canonical entry_canonical
    ((${#ALLOWED_ORIGIN_HOSTS[@]} > 0)) || return 1
    for entry in "${ALLOWED_ORIGIN_HOSTS[@]}"; do
        [[ "$host" == "${entry,,}" ]] && return 0
        if is_ip_literal_host "$host" && is_ip_literal_host "${entry,,}"; then
            host_canonical="$(canonical_ip_literal "$host" 2>/dev/null)" || continue
            entry_canonical="$(canonical_ip_literal "${entry,,}" 2>/dev/null)" || continue
            [[ "$host_canonical" == "$entry_canonical" ]] && return 0
        fi
    done
    return 1
}

# Returns 0 when the service counts as an "other origin route" (see the design note).
service_is_origin_route() {
    local service="$1" parsed host port
    case "$service" in
        http_status:* | hello_world | bastion | socks5 | socks-proxy | unix: | unix:* | unix+tls:*) return 1 ;;
    esac
    parsed="$(parse_service_origin "$service")" || return 0
    host="${parsed%%$'\t'*}"
    port="${parsed#*$'\t'}"
    host_is_allowed "$host" && return 1
    is_ip_literal_host "$host" && return 0
    is_local_host_name "$host" && return 0
    [[ "$port" == 8000 || "$port" == 8080 ]] && return 0
    return 1
}

# Prints only the number of rules (other than the two canonical ones) that route to a
# protected origin. Never prints a hostname, service or port.
other_origin_routes_count() {
    local config_json="$1" count=0 encoded service
    while IFS= read -r encoded; do
        [[ -n "$encoded" ]] || continue
        service="$(printf '%s' "$encoded" | base64 -d)"
        if service_is_origin_route "$service"; then
            count=$((count + 1))
        fi
    done < <(jq -r --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" '
        def norm: ((.hostname // "") | ascii_downcase | sub("\\.+$"; ""));
        .ingress[]
        | select(type != "object" or (norm != $www and norm != $gateway))
        | (if type == "object" then (.service // "") else "" end)
        | if type == "string" then . else "" end
        | @base64' <<<"$config_json")
    printf '%s' "$count"
}

# --- pure validators (JSON in, verdict out; no network) --------------------

# Prints ok | needs_jobs on success; prints a reason to stderr and returns 1 otherwise.
# policy: strict (completed+success), evidence (also in_progress), renewal (strict, or
# completed+failure when the jobs check passes).
validate_run_metadata() {
    local run_json="$1" run_id="$2" workflow_path="$3" policy="$4" verdict
    verdict="$(jq -r --arg id "$run_id" --arg path "$workflow_path" --arg policy "$policy" '
        (.path // "") as $p
        | if (.id | tostring) != $id then "run id mismatch"
          elif ($p != $path and $p != ($path + "@refs/heads/main")) then "unexpected workflow path"
          elif .event != "workflow_dispatch" then "unexpected event"
          elif .head_branch != "main" then "unexpected branch"
          elif (.run_attempt | tostring) != "1" then "run_attempt is not 1"
          elif ($policy == "evidence" and .status == "in_progress" and (.conclusion == null)) then "ok"
          elif (.status == "completed" and .conclusion == "success") then "ok"
          elif ($policy == "renewal" and .status == "completed" and .conclusion == "failure") then "needs_jobs"
          else "run state is not allowed" end' <<<"$run_json" 2>/dev/null)" || verdict="run metadata is not valid JSON"
    if [[ "$verdict" == ok || "$verdict" == needs_jobs ]]; then
        printf '%s' "$verdict"
        return 0
    fi
    fail "run ${run_id} refused: ${verdict}"
    return 1
}

# Prints the artifact id when exactly one unexpired artifact has the name.
validate_artifact_listing() {
    local artifacts_json="$1" name="$2" artifact_id
    artifact_id="$(jq -r --arg name "$name" '
        [.artifacts[]? | select(.name == $name and (.expired | not))] as $a
        | if ($a | length) == 1 and (($a[0].id | tostring) | test("^[0-9]+$")) then ($a[0].id | tostring) else "" end' <<<"$artifacts_json" 2>/dev/null)" || artifact_id=""
    if [[ -z "$artifact_id" ]]; then
        fail "expected exactly one unexpired ${name} artifact"
        return 1
    fi
    printf '%s' "$artifact_id"
}

# A failed close run is acceptable for a renewal only when the manage job succeeded,
# edge-probe failed and every other job succeeded or was skipped.
validate_renewal_jobs() {
    local jobs_json="$1"
    jq -e '
        [.jobs[]?] as $j
        | ([$j[] | select(.name == "manage")]) as $m
        | ([$j[] | select(.name == "edge-probe")]) as $p
        | ($m | length) == 1 and $m[0].conclusion == "success"
          and ($p | length) == 1 and $p[0].conclusion == "failure"
          and ([$j[] | select(.name != "manage" and .name != "edge-probe") | .conclusion]
               | all(. == "success" or . == "skipped"))' <<<"$jobs_json" >/dev/null 2>&1 || {
        fail "closure run failed in a way a renewal may not rely on"
        return 1
    }
}

validate_closure_json() {
    local closure_json="$1" task_id="$2" close_run_id="$3" verdict
    verdict="$(jq -r --arg schema "$CLOSURE_SCHEMA" --arg task "$task_id" --arg run "$close_run_id" '
        if .schema != $schema then "closure schema"
        elif .task_id != $task then "closure task_id mismatch"
        elif (.close_run_id | tostring) != $run then "closure close_run_id mismatch"
        elif (.run_attempt | tostring) != "1" then "closure run_attempt"
        elif (.edge_version | type) != "number" then "closure edge_version"
        elif ((.config_hash | type) != "string") or ((.config_hash | test("^[0-9a-f]{64}$")) | not) then "closure config_hash"
        elif (.fingerprint_status != "collected" and .fingerprint_status != "unavailable") then "closure fingerprint_status"
        elif (.fingerprint_status == "unavailable" and .origin_fingerprint != null) then "closure fingerprint"
        elif (.fingerprint_status == "collected" and ((.origin_fingerprint | type) != "object")) then "closure fingerprint"
        else "ok" end' <<<"$closure_json" 2>/dev/null)" || verdict="closure is not valid JSON"
    [[ "$verdict" == ok ]] && return 0
    fail "closure artifact refused: ${verdict}"
    return 1
}

validate_evidence_json() {
    local evidence_json="$1" task_id="$2" close_run_id="$3" verdict
    verdict="$(jq -r --arg task "$task_id" --arg run "$close_run_id" '
        if ((.schema | type) != "string") or (.schema == "") then "evidence schema"
        elif .task_id != $task then "evidence task_id mismatch"
        elif (.close_run_id | tostring) != $run then "evidence close_run_id mismatch"
        elif (.outcome != "cutover_complete" and .outcome != "rollback_verified") then "evidence outcome"
        else "ok" end' <<<"$evidence_json" 2>/dev/null)" || verdict="evidence is not valid JSON"
    [[ "$verdict" == ok ]] && return 0
    fail "evidence artifact refused: ${verdict}"
    return 1
}

# The closure must describe exactly the live closed edge (version and hash).
closure_matches_live() {
    local closure_json="$1" live_version="$2" live_hash="$3"
    jq -e --argjson version "${live_version:-null}" --arg hash "$live_hash" '
        .edge_version == $version and .config_hash == $hash' <<<"$closure_json" >/dev/null 2>&1 || {
        fail "closure no longer matches the live edge (edge_version or config_hash changed)"
        return 1
    }
}

# Prints {"status":"collected"|"unavailable","fingerprint":<object|null>} for a candidate file.
evaluate_fingerprint_candidate() {
    local candidate_file="$1" marker="$ORIGIN_MARKER_ELEMENT"
    if [[ ! -s "$candidate_file" ]] || (($(wc -c <"$candidate_file") > 8192)); then
        printf '{"status":"unavailable","fingerprint":null}'
        return 0
    fi
    jq -c --arg schema "$FINGERPRINT_SCHEMA" --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" --arg marker "$marker" '
        def usable($h):
            ($h | type) == "object"
            and (($h.status | type) == "number")
            and ($h.status >= 100 and $h.status <= 599)
            and ($h.status != 502 and $h.status != 503 and $h.status != 504 and ($h.status < 520 or $h.status > 530))
            and (($h.elements | type) == "array")
            and (($h.elements | length) <= 32)
            and ([$h.elements[] | (type == "string") and test("^(cookie|header):[a-z0-9_.-]{1,64}$")] | all)
            and ((($h.elements | unique) | length) >= 2 or ($h.elements | index($marker)) != null);
        try (
            if .schema == $schema
               and ((.hosts | type) == "object")
               and ((.hosts | keys) == ([$www, $gateway] | sort))
               and usable(.hosts[$www]) and usable(.hosts[$gateway])
            then {status: "collected", fingerprint: (.hosts | map_values(.elements | unique))}
            else {status: "unavailable", fingerprint: null} end
        ) catch {status: "unavailable", fingerprint: null}' "$candidate_file"
}

# --- artifact retrieval (gh api compatible) ---------------------------------

extract_zip_member() {
    python3 -I - "$1" "$2" <<'PY'
import sys
import zipfile

path, member = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(path) as archive:
    infos = [info for info in archive.infolist() if not info.is_dir()]
    if len(infos) != 1 or infos[0].filename != member or infos[0].file_size > 65536:
        sys.exit(1)
    sys.stdout.buffer.write(archive.read(member))
PY
}

# Prints the JSON content of the single artifact of a run, after the provenance checks.
fetch_run_artifact_json() {
    local run_id="$1" workflow_path="$2" artifact_name="$3" member="$4" policy="$5"
    local run_json verdict jobs_json artifacts_json artifact_id zip_file content
    [[ "$run_id" =~ $RUN_ID_REGEX ]] || { fail "run id must be numeric"; return 1; }
    [[ -n "${GITHUB_REPOSITORY:-}" ]] || { fail "GITHUB_REPOSITORY is required for provenance checks"; return 1; }
    run_json="$(gh_api "repos/${GITHUB_REPOSITORY}/actions/runs/${run_id}")" || { fail "cannot read run ${run_id}"; return 1; }
    verdict="$(validate_run_metadata "$run_json" "$run_id" "$workflow_path" "$policy")" || return 1
    if [[ "$verdict" == needs_jobs ]]; then
        jobs_json="$(gh_api "repos/${GITHUB_REPOSITORY}/actions/runs/${run_id}/jobs?per_page=100")" || { fail "cannot read jobs of run ${run_id}"; return 1; }
        validate_renewal_jobs "$jobs_json" || return 1
    fi
    artifacts_json="$(gh_api "repos/${GITHUB_REPOSITORY}/actions/runs/${run_id}/artifacts?per_page=100")" || { fail "cannot list artifacts of run ${run_id}"; return 1; }
    artifact_id="$(validate_artifact_listing "$artifacts_json" "$artifact_name")" || return 1
    zip_file="$(mktemp)"
    if ! gh_api "repos/${GITHUB_REPOSITORY}/actions/artifacts/${artifact_id}/zip" >"$zip_file"; then
        rm -f -- "$zip_file"
        fail "cannot download ${artifact_name}"
        return 1
    fi
    if ! content="$(extract_zip_member "$zip_file" "$member")"; then
        rm -f -- "$zip_file"
        fail "${artifact_name} does not contain exactly ${member}"
        return 1
    fi
    rm -f -- "$zip_file"
    jq -c . <<<"$content" || { fail "${artifact_name} is not valid JSON"; return 1; }
}

# --- guard (read-only, no Cloudflare secret) --------------------------------

gate_guard() {
    local workflow status response count
    [[ -n "${GITHUB_REPOSITORY:-}" ]] || { fail "GITHUB_REPOSITORY is required"; return 1; }
    for workflow in cloudflare-oteryn-endpoints.yml cloudflare-oteryn-endpoint-main-operation.yml; do
        for status in queued waiting pending in_progress; do
            response="$(gh_api "repos/${GITHUB_REPOSITORY}/actions/workflows/${workflow}/runs?status=${status}&per_page=100")" || {
                fail "cannot list ${status} runs; guard fails closed"
                return 1
            }
            jq -e '.workflow_runs | type == "array"' <<<"$response" >/dev/null 2>&1 || {
                fail "run list is unreadable; guard fails closed"
                return 1
            }
            count="$(jq --arg self "${GITHUB_RUN_ID:-0}" '
                [.workflow_runs[]
                 | select((.id | tostring) != $self)
                 | select((.display_title // "") | test("^Cloudflare Oteryn Endpoints \\[maintenance-(close|open)\\]"))] | length' <<<"$response")"
            if [[ "$count" != 0 ]]; then
                fail "a maintenance gate run is ${status}; apply must not start"
                return 1
            fi
        done
    done
    printf 'gate_guard=clear\n'
}

# --- edge reads ---------------------------------------------------------------

preflight() {
    local token_response token_verify_path tunnel_response tunnel_id account_tag config_src
    if [[ "$CLOUDFLARE_API_TOKEN" == cfat_* ]]; then
        token_verify_path="/accounts/${CLOUDFLARE_ACCOUNT_ID}/tokens/verify"
    else
        token_verify_path='/user/tokens/verify'
    fi
    token_response="$(api_request GET "$token_verify_path")"
    [[ "$(jq -r '.result.status // ""' <<<"$token_response")" == "active" ]] || fail "Cloudflare API token is not active"

    tunnel_response="$(api_request GET "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}")"
    tunnel_id="$(jq -r '.result.id // ""' <<<"$tunnel_response")"
    account_tag="$(jq -r '.result.account_tag // ""' <<<"$tunnel_response")"
    config_src="$(jq -r '.result.config_src // ""' <<<"$tunnel_response")"
    TUNNEL_STATE="$(jq -r '.result.status // "unknown"' <<<"$tunnel_response")"

    [[ "$tunnel_id" == "$CLOUDFLARE_TUNNEL_ID" ]] || fail "Cloudflare returned a different tunnel ID"
    [[ "${account_tag,,}" == "${CLOUDFLARE_ACCOUNT_ID,,}" ]] || fail "tunnel does not belong to CLOUDFLARE_ACCOUNT_ID"
    [[ "$config_src" == "cloudflare" ]] || fail "tunnel is not remotely managed (config_src=${config_src:-unknown})"
}

# Fresh read: sets EDGE_CONFIG, EDGE_VERSION, EDGE_HASH and EDGE_STATE. An unusable
# `version` makes the state unknown (no weakened closure binding).
read_edge() {
    local response
    response="$(api_request GET "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations")" || return 1
    EDGE_CONFIG="$(jq -c '.result.config // empty' <<<"$response")"
    [[ -n "$EDGE_CONFIG" ]] || { fail "Cloudflare returned no remote tunnel configuration"; return 1; }
    EDGE_VERSION="$(jq -r '.result.version | if type == "number" and . == floor and . >= 0 then tostring else "" end' <<<"$response")"
    EDGE_HASH="$(sha256_json <<<"$EDGE_CONFIG")"
    if [[ -z "$EDGE_VERSION" ]]; then
        EDGE_STATE="unknown"
    else
        EDGE_STATE="$(classify_gate_state "$EDGE_CONFIG")"
    fi
}

put_edge_config() {
    local config_json="$1"
    api_request PUT "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations" \
        "$(jq -cn --argjson config "$config_json" '{config: $config}')" >/dev/null
}

require_task_id() {
    [[ "${OTERYN_TASK_ID:-}" =~ $TASK_ID_REGEX ]] || { fail "task_id must match ${TASK_ID_REGEX}"; return 1; }
}

# Compares the closure named by OTERYN_CLOSURE_RUN_ID with the live edge (audit only).
compute_closure_match() {
    local closure_json
    [[ -n "${OTERYN_CLOSURE_RUN_ID:-}" ]] || { printf 'not_requested'; return 0; }
    if [[ "$EDGE_STATE" != closed ]] || [[ ! "${OTERYN_TASK_ID:-}" =~ $TASK_ID_REGEX ]]; then
        printf 'no'
        return 0
    fi
    if ! closure_json="$(fetch_run_artifact_json "$OTERYN_CLOSURE_RUN_ID" "$ENDPOINTS_WORKFLOW_PATH" "$CLOSURE_ARTIFACT" closure.json strict 2>/dev/null)"; then
        printf 'no'
        return 0
    fi
    if validate_closure_json "$closure_json" "$OTERYN_TASK_ID" "$OTERYN_CLOSURE_RUN_ID" >/dev/null 2>&1 \
        && closure_matches_live "$closure_json" "$EDGE_VERSION" "$EDGE_HASH" >/dev/null 2>&1; then
        printf 'yes'
    else
        printf 'no'
    fi
}

dns_state_or_fail() {
    local host="$1" target="$2"
    dns_record_state "$(fetch_dns_records "$host")" "$host" "$target"
}

# Fresh audit snapshot used after a gate write: sets AUDIT_* globals.
audit_snapshot() {
    local target="${CLOUDFLARE_TUNNEL_ID}.cfargotunnel.com" desired
    read_edge
    desired="$(build_desired_config "$EDGE_CONFIG")"
    if [[ "$(canonical_json <<<"$EDGE_CONFIG")" == "$(canonical_json <<<"$desired")" ]]; then
        AUDIT_DRIFT="current"
    else
        AUDIT_DRIFT="drift"
    fi
    AUDIT_WWW_DNS="$(dns_state_or_fail "$WWW_HOST" "$target")"
    AUDIT_GATEWAY_DNS="$(dns_state_or_fail "$GATEWAY_HOST" "$target")"
    AUDIT_LEGACY_DNS="$(legacy_dns_state "$(fetch_dns_records "$LEGACY_GATEWAY_HOST")" "$target")"
}

print_report() {
    local mode="$1" drift="$2" www="$3" gateway="$4" legacy="$5" mutation="$6" routes closure_match
    routes="$(other_origin_routes_count "$EDGE_CONFIG")"
    closure_match="not_requested"
    if [[ "$mode" == audit ]]; then
        closure_match="$(compute_closure_match)"
    fi
    printf 'mode=%s\n' "$mode"
    printf 'tunnel_status=%s\n' "$TUNNEL_STATE"
    printf 'tunnel_contract=%s\n' "$drift"
    printf 'www_dns=%s\n' "$www"
    printf 'gateway_dns=%s\n' "$gateway"
    printf 'legacy_gateway_dns=%s\n' "$legacy"
    printf 'mutation=%s\n' "$mutation"
    printf 'gate_state=%s\n' "$EDGE_STATE"
    printf 'edge_version=%s\n' "${EDGE_VERSION:-unknown}"
    printf 'other_origin_routes=%s\n' "$routes"
    if [[ "$mode" == audit ]]; then
        printf 'closure_match=%s\n' "$closure_match"
        write_audit_result "$drift" "$www" "$gateway" "$legacy" "$routes" "$closure_match"
    fi

    append_summary '## Cloudflare Oteryn endpoints'
    append_summary "- mode: \`${mode}\`"
    append_summary "- gate state: \`${EDGE_STATE}\` (edge version \`${EDGE_VERSION:-unknown}\`)"
    append_summary "- other origin routes: \`${routes}\`"
    append_summary "- tunnel status: \`${TUNNEL_STATE}\`"
    append_summary "- tunnel contract: \`${drift}\`"
    append_summary "- WWW DNS: \`${www}\`"
    append_summary "- Gateway DNS: \`${gateway}\`"
    append_summary "- retired legacy Gateway DNS: \`${legacy}\`"
    append_summary "- mutation: \`${mutation}\`"
}

write_audit_result() {
    local drift="$1" www="$2" gateway="$3" legacy="$4" routes="$5" closure_match="$6"
    [[ -n "${OTERYN_AUDIT_RESULT_FILE:-}" ]] || return 0
    jq -n \
        --arg schema "$AUDIT_RESULT_SCHEMA" \
        --arg run_id "${GITHUB_RUN_ID:-}" \
        --arg attempt "${GITHUB_RUN_ATTEMPT:-1}" \
        --arg task "${OTERYN_TASK_ID:-}" \
        --arg closure_run "${OTERYN_CLOSURE_RUN_ID:-}" \
        --arg gate_state "$EDGE_STATE" \
        --arg edge_version "${EDGE_VERSION:-}" \
        --arg hash "$EDGE_HASH" \
        --arg contract "$drift" \
        --arg www "$www" --arg gateway "$gateway" --arg legacy "$legacy" \
        --argjson routes "$routes" \
        --arg closure_match "$closure_match" '
        {
            schema: $schema,
            audit_run_id: $run_id,
            run_attempt: ($attempt | tonumber),
            task_id: (if $task == "" then null else $task end),
            closure_run_id: (if $closure_run == "" then null else $closure_run end),
            gate_state: $gate_state,
            edge_version: (if $edge_version == "" then null else ($edge_version | tonumber) end),
            config_hash: $hash,
            tunnel_contract: $contract,
            dns: {www: $www, gateway: $gateway, legacy_gateway: $legacy},
            other_origin_routes: $routes,
            closure_match: $closure_match
        }' >"$OTERYN_AUDIT_RESULT_FILE"
}

# --- maintenance-close ----------------------------------------------------------

run_maintenance_close() {
    local pre_state pre_version pre_hash pre_shape routes closure_run_id closure_json
    local fp_status="unavailable" fp_json="null" fp_eval planned mutation="gate-close"

    [[ "${CLOUDFLARE_GATE_CONFIRMATION:-}" == "$CLOSE_CONFIRMATION" ]] || fail "maintenance-close requires exact confirmation: ${CLOSE_CONFIRMATION}"
    require_task_id
    [[ "${GITHUB_RUN_ID:-}" =~ $RUN_ID_REGEX ]] || fail "GITHUB_RUN_ID is required"
    [[ "${GITHUB_RUN_ATTEMPT:-1}" == 1 ]] || fail "maintenance-close must be run_attempt 1; dispatch a new run"
    [[ -n "${OTERYN_CLOSURE_FILE:-}" ]] || fail "OTERYN_CLOSURE_FILE is required"
    rm -f -- "$OTERYN_CLOSURE_FILE"

    preflight
    read_edge
    pre_state="$EDGE_STATE"
    pre_version="$EDGE_VERSION"
    pre_hash="$EDGE_HASH"
    pre_shape="$(config_shape_hash "$EDGE_CONFIG")"
    case "$pre_state" in
        open | split | closed) ;;
        *) fail "maintenance-close refused: gate_state=${pre_state}" ;;
    esac

    routes="$(other_origin_routes_count "$EDGE_CONFIG")"
    printf 'other_origin_routes=%s\n' "$routes"
    [[ "$routes" == 0 ]] || fail "maintenance-close refused: other_origin_routes=${routes}"

    if [[ "$pre_state" == closed ]]; then
        closure_run_id="${OTERYN_CLOSURE_RUN_ID:-}"
        [[ "$closure_run_id" =~ $RUN_ID_REGEX ]] || fail "renewal from a closed gate requires closure_run_id"
        closure_json="$(fetch_run_artifact_json "$closure_run_id" "$ENDPOINTS_WORKFLOW_PATH" "$CLOSURE_ARTIFACT" closure.json renewal)"
        validate_closure_json "$closure_json" "$OTERYN_TASK_ID" "$closure_run_id" || fail "renewal refused"
        closure_matches_live "$closure_json" "$pre_version" "$pre_hash" || fail "renewal refused"
        fp_status="$(jq -r '.fingerprint_status' <<<"$closure_json")"
        fp_json="$(jq -c '.origin_fingerprint' <<<"$closure_json")"
        planned="$EDGE_CONFIG"
        mutation="gate-renew"
    else
        planned="$(set_gate_services "$EDGE_CONFIG" "$GATE_SERVICE" "$GATE_SERVICE")"
    fi

    read_edge
    if [[ "$EDGE_HASH" != "$pre_hash" || "$EDGE_VERSION" != "$pre_version" || "$EDGE_STATE" != "$pre_state" ]]; then
        fail "edge changed between the first read and the write; nothing was written (gate_state=${EDGE_STATE})"
    fi

    if [[ "$pre_state" == open ]] && [[ -n "${OTERYN_FINGERPRINT_FILE:-}" ]]; then
        fp_eval="$(evaluate_fingerprint_candidate "$OTERYN_FINGERPRINT_FILE")"
        fp_status="$(jq -r '.status' <<<"$fp_eval")"
        fp_json="$(jq -c '.fingerprint' <<<"$fp_eval")"
    fi

    put_edge_config "$planned"

    read_edge
    [[ "$EDGE_STATE" == closed ]] || fail "close verification failed: observed gate_state=${EDGE_STATE}"
    [[ "$(config_shape_hash "$EDGE_CONFIG")" == "$pre_shape" ]] || fail "close verification failed: unrelated configuration changed (gate_state=${EDGE_STATE})"
    [[ "$EDGE_VERSION" == "$((pre_version + 1))" ]] || fail "close verification failed: edge_version ${EDGE_VERSION} is not ${pre_version} plus one (gate_state=${EDGE_STATE})"

    jq -n \
        --arg schema "$CLOSURE_SCHEMA" --arg task "$OTERYN_TASK_ID" --arg run "$GITHUB_RUN_ID" \
        --argjson version "$EDGE_VERSION" --arg hash "$EDGE_HASH" \
        --arg fp_status "$fp_status" --argjson fp "$fp_json" '
        {schema: $schema, task_id: $task, close_run_id: $run, run_attempt: 1, edge_version: $version,
         config_hash: $hash, fingerprint_status: $fp_status, origin_fingerprint: $fp}' >"$OTERYN_CLOSURE_FILE"

    print_report maintenance-close not_checked not_checked not_checked not_checked "$mutation"
    printf 'fingerprint_status=%s\n' "$fp_status"
    append_summary "- close run id: \`${GITHUB_RUN_ID}\`, config hash: \`${EDGE_HASH}\`, fingerprint: \`${fp_status}\`"
}

# --- maintenance-open -----------------------------------------------------------

run_maintenance_open() {
    local pre_version pre_hash pre_shape closure_json evidence_json planned mutation="none"
    [[ "${CLOUDFLARE_GATE_CONFIRMATION:-}" == "$OPEN_CONFIRMATION" ]] || fail "maintenance-open requires exact confirmation: ${OPEN_CONFIRMATION}"
    require_task_id

    preflight
    read_edge
    case "$EDGE_STATE" in
        open) ;;
        closed)
            [[ "${OTERYN_CLOSURE_RUN_ID:-}" =~ $RUN_ID_REGEX ]] || fail "maintenance-open requires closure_run_id"
            [[ "${OTERYN_EVIDENCE_RUN_ID:-}" =~ $RUN_ID_REGEX ]] || fail "maintenance-open requires evidence_run_id"
            closure_json="$(fetch_run_artifact_json "$OTERYN_CLOSURE_RUN_ID" "$ENDPOINTS_WORKFLOW_PATH" "$CLOSURE_ARTIFACT" closure.json strict)"
            evidence_json="$(fetch_run_artifact_json "$OTERYN_EVIDENCE_RUN_ID" "$CUTOVER_WORKFLOW_PATH" "$EVIDENCE_ARTIFACT" gate-evidence.json evidence)"
            validate_closure_json "$closure_json" "$OTERYN_TASK_ID" "$OTERYN_CLOSURE_RUN_ID" || fail "open refused"
            validate_evidence_json "$evidence_json" "$OTERYN_TASK_ID" "$OTERYN_CLOSURE_RUN_ID" || fail "open refused"
            closure_matches_live "$closure_json" "$EDGE_VERSION" "$EDGE_HASH" || fail "open refused"
            pre_version="$EDGE_VERSION"
            pre_hash="$EDGE_HASH"
            pre_shape="$(config_shape_hash "$EDGE_CONFIG")"

            read_edge
            if [[ "$EDGE_STATE" != closed || "$EDGE_VERSION" != "$pre_version" || "$EDGE_HASH" != "$pre_hash" ]]; then
                fail "edge changed before the write; nothing was written (gate_state=${EDGE_STATE})"
            fi
            closure_matches_live "$closure_json" "$EDGE_VERSION" "$EDGE_HASH" || fail "open refused"

            planned="$(set_gate_services "$EDGE_CONFIG" "$WWW_SERVICE" "$GATEWAY_SERVICE")"
            put_edge_config "$planned"
            read_edge
            [[ "$EDGE_STATE" == open ]] || fail "open verification failed: observed gate_state=${EDGE_STATE}"
            [[ "$(config_shape_hash "$EDGE_CONFIG")" == "$pre_shape" ]] || fail "open verification failed: unrelated configuration changed"
            [[ "$EDGE_VERSION" == "$((pre_version + 1))" ]] || fail "open verification failed: edge_version ${EDGE_VERSION} is not ${pre_version} plus one"
            mutation="gate-open"
            ;;
        *) fail "maintenance-open refused: gate_state=${EDGE_STATE}" ;;
    esac

    audit_snapshot
    print_report maintenance-open "$AUDIT_DRIFT" "$AUDIT_WWW_DNS" "$AUDIT_GATEWAY_DNS" "$AUDIT_LEGACY_DNS" "$mutation"
    [[ "$EDGE_STATE" == open ]] || fail "post-open audit: gate_state=${EDGE_STATE}"
    [[ "$AUDIT_DRIFT" == current ]] || fail "post-open audit: tunnel_contract=${AUDIT_DRIFT}"
    [[ "$AUDIT_WWW_DNS" == current && "$AUDIT_GATEWAY_DNS" == current ]] || fail "post-open audit: canonical DNS is not current"
}

# --- secret-free edge fingerprint and probes (GitHub runner, no Cloudflare token) -

probe_origin_args() {
    PROBE_URL="https://$1/"
    PROBE_EXTRA=()
    if [[ -n "${OTERYN_PROBE_ORIGIN:-}" ]]; then
        PROBE_URL="${OTERYN_PROBE_ORIGIN%/}/"
        PROBE_EXTRA=(--header "Host: $1")
    fi
}

# Sets PROBE_STATUS, PROBE_SERVER and PROBE_ELEMENTS (names only, generic names removed).
probe_host() {
    local host="$1" header_file code
    header_file="$(mktemp)"
    probe_origin_args "$host"
    if code="$($CURL_BIN --silent --show-error --connect-timeout 10 --max-time 20 \
        --dump-header "$header_file" --output /dev/null --write-out '%{http_code}' \
        "${PROBE_EXTRA[@]}" "$PROBE_URL" 2>/dev/null)"; then
        PROBE_STATUS="$((10#${code:-0}))"
    else
        PROBE_STATUS=0
    fi
    PROBE_SERVER="$(tr -d '\r' <"$header_file" | awk -F': *' 'tolower($1) == "server" {print tolower($2); exit}')"
    PROBE_ELEMENTS="$(tr -d '\r' <"$header_file" | awk '
        /^[A-Za-z0-9-]+:/ {
            n = tolower($0); sub(/:.*/, "", n)
            if (n == "set-cookie") {
                v = $0; sub(/^[^:]*:[ \t]*/, "", v); split(v, a, "="); c = tolower(a[1])
                if (c !~ /^(session|sessionid|csrftoken|jsessionid|phpsessid)$/ && c !~ /^(__cf|cf_|_cf)/) print "cookie:" c
            } else if (n !~ /^(date|server|content-.*|cf-.*|strict-transport-security|x-frame-options|x-content-type-options|referrer-policy|content-security-policy|permissions-policy|x-powered-by|connection|transfer-encoding|vary|alt-svc|nel|report-to|cache-control|expires|pragma|location)$/) {
                print "header:" n
            }
        }' | sort -u)"
    rm -f -- "$header_file"
}

random_probe_host() {
    printf 'oteryn-probe-%s.molehill.cloud' "$(head -c 6 /dev/urandom | od -An -tx1 | tr -d ' \n')"
}

elements_json() {
    printf '%s\n' "$1" | jq -R -s -c 'split("\n") | map(select(length > 0))'
}

edge_fingerprint_collect() {
    local out="${OTERYN_FINGERPRINT_OUT:-fingerprint-candidate.json}" baseline_host baseline
    local www_status www_elements gateway_status gateway_elements
    require_command jq
    require_command "$CURL_BIN"
    baseline_host="$(random_probe_host)"
    probe_host "$baseline_host"
    baseline="$PROBE_ELEMENTS"
    probe_host "$WWW_HOST"
    www_status="$PROBE_STATUS"
    www_elements="$(comm -23 <(printf '%s\n' "$PROBE_ELEMENTS") <(printf '%s\n' "$baseline") | sed '/^$/d')"
    probe_host "$GATEWAY_HOST"
    gateway_status="$PROBE_STATUS"
    gateway_elements="$(comm -23 <(printf '%s\n' "$PROBE_ELEMENTS") <(printf '%s\n' "$baseline") | sed '/^$/d')"
    jq -n --arg schema "$FINGERPRINT_SCHEMA" --arg now "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" \
        --argjson www_status "$www_status" --argjson www_elements "$(elements_json "$www_elements")" \
        --argjson gateway_status "$gateway_status" --argjson gateway_elements "$(elements_json "$gateway_elements")" '
        {schema: $schema, collected_at: $now,
         hosts: {($www): {status: $www_status, elements: $www_elements},
                 ($gateway): {status: $gateway_status, elements: $gateway_elements}}}' >"$out"
    printf 'fingerprint_candidate=written\n'
}

# edge_probe closed|open. Closed: both canonical hosts answer 503 from cloudflare, and a
# synthetic label never carries a recorded application fingerprint. Open: neither is 503.
edge_probe() {
    local phase="$1" closure_file="${OTERYN_CLOSURE_FILE:-closure.json}" fp_status baseline_host
    local failed=0 host recorded
    require_command jq
    require_command "$CURL_BIN"
    case "$phase" in
        closed)
            for host in "$WWW_HOST" "$GATEWAY_HOST"; do
                probe_host "$host"
                if [[ "$PROBE_STATUS" != 503 || "$PROBE_SERVER" != cloudflare ]]; then
                    printf 'ERROR: canonical host did not answer 503 from the edge\n' >&2
                    failed=1
                fi
            done
            fp_status="$(jq -r '.fingerprint_status // "unavailable"' "$closure_file")"
            baseline_host="$(random_probe_host)"
            probe_host "$baseline_host"
            if [[ "$fp_status" == collected ]]; then
                for host in "$WWW_HOST" "$GATEWAY_HOST"; do
                    recorded="$(jq -c --arg host "$host" '.origin_fingerprint[$host] // []' "$closure_file")"
                    if jq -e -n --argjson recorded "$recorded" --argjson seen "$(elements_json "$PROBE_ELEMENTS")" \
                        '($recorded | length) > 0 and ([$recorded[] | . as $e | $seen | index($e)] | all(. != null))' >/dev/null; then
                        printf 'ERROR: a synthetic hostname returned a recorded application response\n' >&2
                        failed=1
                    fi
                done
                printf 'wildcard_probe=checked\n'
            else
                printf 'wildcard_probe=informational\n'
            fi
            printf 'fingerprint_status=%s\n' "$fp_status"
            ;;
        open)
            for host in "$WWW_HOST" "$GATEWAY_HOST"; do
                probe_host "$host"
                if [[ "$PROBE_STATUS" == 503 || "$PROBE_STATUS" == 0 ]]; then
                    printf 'ERROR: canonical host is still unavailable after open\n' >&2
                    failed=1
                fi
            done
            ;;
        *) fail "usage: edge-probe closed|open" ;;
    esac
    printf 'edge_probe_%s=%s\n' "$phase" "$([[ "$failed" == 0 ]] && echo pass || echo fail)"
    [[ "$failed" == 0 ]]
}

main() {
    local mode="${1:-}"
    local config_response current_config desired_config target
    local initial_config_hash initial_version current_config_hash tunnel_drift
    local www_dns gateway_dns legacy_dns www_state gateway_state legacy_state
    local mutation="none"

    case "$mode" in
        audit | apply | maintenance-close | maintenance-open) ;;
        gate-guard)
            require_command jq
            gate_guard
            return
            ;;
        edge-fingerprint)
            edge_fingerprint_collect
            return
            ;;
        edge-probe)
            edge_probe "${2:-}"
            return
            ;;
        *) fail "usage: $0 audit|apply|maintenance-close|maintenance-open|gate-guard|edge-fingerprint|edge-probe" ;;
    esac

    require_command jq
    require_command sha256sum
    require_command "$CURL_BIN"
    require_nonempty_env CLOUDFLARE_API_TOKEN
    require_nonempty_env CLOUDFLARE_ACCOUNT_ID
    require_nonempty_env CLOUDFLARE_ZONE_ID
    require_nonempty_env CLOUDFLARE_TUNNEL_ID
    validate_identifiers

    if [[ -n "${OTERYN_TASK_ID:-}" ]]; then
        require_task_id
    fi
    if [[ -n "${OTERYN_CLOSURE_RUN_ID:-}" ]]; then
        [[ "$OTERYN_CLOSURE_RUN_ID" =~ $RUN_ID_REGEX ]] || fail "closure_run_id must be numeric"
    fi

    if [[ "$mode" == "apply" ]]; then
        [[ "${CLOUDFLARE_APPLY_CONFIRMATION:-}" == "$APPLY_CONFIRMATION" ]] || fail "apply requires exact confirmation: ${APPLY_CONFIRMATION}"
    fi

    if [[ "$mode" == "maintenance-close" ]]; then
        run_maintenance_close
        return
    fi
    if [[ "$mode" == "maintenance-open" ]]; then
        run_maintenance_open
        return
    fi

    preflight

    read_edge
    current_config="$EDGE_CONFIG"
    initial_version="$EDGE_VERSION"
    desired_config="$(build_desired_config "$current_config")"

    if [[ "$mode" == "apply" && "$EDGE_STATE" != "open" ]]; then
        # Script-level refusal: covers every entry point. Nothing is written, not even DNS.
        fail "apply refused: gate_state=${EDGE_STATE}; apply runs only when both canonical rules are open"
    fi

    initial_config_hash="$EDGE_HASH"
    if [[ "$(canonical_json <<<"$current_config")" == "$(canonical_json <<<"$desired_config")" ]]; then
        tunnel_drift="current"
    else
        tunnel_drift="drift"
    fi

    target="${CLOUDFLARE_TUNNEL_ID}.cfargotunnel.com"
    www_dns="$(fetch_dns_records "$WWW_HOST")"
    gateway_dns="$(fetch_dns_records "$GATEWAY_HOST")"
    legacy_dns="$(fetch_dns_records "$LEGACY_GATEWAY_HOST")"
    www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
    gateway_state="$(dns_record_state "$gateway_dns" "$GATEWAY_HOST" "$target")"
    legacy_state="$(legacy_dns_state "$legacy_dns" "$target")"

    if [[ "$mode" == "apply" ]]; then
        if [[ "$tunnel_drift" == "drift" ]]; then
            read_edge
            current_config="$EDGE_CONFIG"
            current_config_hash="$EDGE_HASH"
            [[ "$current_config_hash" == "$initial_config_hash" && "$EDGE_VERSION" == "$initial_version" ]] || fail "tunnel configuration changed during planning; rerun audit before apply"
            [[ "$EDGE_STATE" == "open" ]] || fail "apply refused: gate_state=${EDGE_STATE} at write time"
            desired_config="$(build_desired_config "$current_config")"
            put_edge_config "$desired_config"
            mutation="$(record_mutation "$mutation" tunnel)"
        fi

        www_dns="$(fetch_dns_records "$WWW_HOST")"
        www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
        if [[ "$www_state" != "current" ]]; then
            upsert_dns_record "$WWW_HOST" "$target" "$www_dns"
            mutation="$(record_mutation "$mutation" dns-www)"
        fi

        gateway_dns="$(fetch_dns_records "$GATEWAY_HOST")"
        gateway_state="$(dns_record_state "$gateway_dns" "$GATEWAY_HOST" "$target")"
        if [[ "$gateway_state" != "current" ]]; then
            upsert_dns_record "$GATEWAY_HOST" "$target" "$gateway_dns"
            mutation="$(record_mutation "$mutation" dns-gateway)"
        fi

        read_edge
        current_config="$EDGE_CONFIG"
        desired_config="$(build_desired_config "$current_config")"
        [[ "$(canonical_json <<<"$current_config")" == "$(canonical_json <<<"$desired_config")" ]] || fail "post-write tunnel verification failed before legacy retirement"

        www_dns="$(fetch_dns_records "$WWW_HOST")"
        gateway_dns="$(fetch_dns_records "$GATEWAY_HOST")"
        [[ "$(dns_record_state "$www_dns" "$WWW_HOST" "$target")" == "current" ]] || fail "post-write WWW DNS verification failed"
        [[ "$(dns_record_state "$gateway_dns" "$GATEWAY_HOST" "$target")" == "current" ]] || fail "post-write Gateway DNS verification failed"

        legacy_dns="$(fetch_dns_records "$LEGACY_GATEWAY_HOST")"
        legacy_state="$(legacy_dns_state "$legacy_dns" "$target")"
        if [[ "$legacy_state" == "safe-to-retire" ]]; then
            retire_legacy_dns_record "$legacy_dns" "$target"
            mutation="$(record_mutation "$mutation" dns-legacy-delete)"
        fi

        read_edge
        current_config="$EDGE_CONFIG"
        desired_config="$(build_desired_config "$current_config")"
        [[ "$(canonical_json <<<"$current_config")" == "$(canonical_json <<<"$desired_config")" ]] || fail "final tunnel verification failed"
        [[ "$EDGE_STATE" == "open" ]] || fail "final tunnel verification failed: gate_state=${EDGE_STATE}"

        www_dns="$(fetch_dns_records "$WWW_HOST")"
        gateway_dns="$(fetch_dns_records "$GATEWAY_HOST")"
        legacy_dns="$(fetch_dns_records "$LEGACY_GATEWAY_HOST")"
        www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
        gateway_state="$(dns_record_state "$gateway_dns" "$GATEWAY_HOST" "$target")"
        legacy_state="$(legacy_dns_state "$legacy_dns" "$target")"

        [[ "$www_state" == "current" ]] || fail "final WWW DNS verification failed"
        [[ "$gateway_state" == "current" ]] || fail "final Gateway DNS verification failed"
        [[ "$legacy_state" == "missing" ]] || fail "legacy Gateway DNS record remains after apply"
        tunnel_drift="current"
    fi

    print_report "$mode" "$tunnel_drift" "$www_state" "$gateway_state" "$legacy_state" "$mutation"
}

if [[ "${CLOUDFLARE_LIBRARY_ONLY:-0}" != "1" ]]; then
    main "$@"
fi
