#!/usr/bin/env bash
set -euo pipefail

readonly CLOUDFLARE_API_BASE_URL="${CLOUDFLARE_API_BASE_URL:-https://api.cloudflare.com/client/v4}"
readonly CURL_BIN="${CLOUDFLARE_CURL_BIN:-curl}"
readonly WWW_HOST="oteryn.molehill.cloud"
readonly WWW_SERVICE="http://127.0.0.1:8000"
readonly LOGIN_HOST="login.oteryn.molehill.cloud"
readonly LOGIN_SERVICE="http://127.0.0.1:8080"
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

    if ! jq -e 'type == "object" and (.ingress | type == "array") and (.ingress | length > 0)' \
        <<<"$config_json" >/dev/null; then
        fail "tunnel configuration must contain a non-empty ingress array"
        return 1
    fi

    catchall_count="$(jq '[.ingress[] | select(((.hostname // "") == "") and ((.path // "") == ""))] | length' <<<"$config_json")"
    if [[ "$catchall_count" != "1" ]]; then
        fail "tunnel configuration must contain exactly one pathless catch-all rule"
        return 1
    fi

    if ! jq -e '.ingress[-1] | ((.hostname // "") == "") and ((.path // "") == "")' \
        <<<"$config_json" >/dev/null; then
        fail "the pathless catch-all rule must be the final ingress rule"
        return 1
    fi

    for host in "$WWW_HOST" "$LOGIN_HOST"; do
        host_count="$(jq --arg host "$host" '[.ingress[] | select((.hostname // "") == $host)] | length' <<<"$config_json")"
        if ((host_count > 1)); then
            fail "duplicate tunnel ingress rules exist for ${host}"
            return 1
        fi

        host_path_count="$(jq --arg host "$host" '[.ingress[] | select((.hostname // "") == $host and ((.path // "") != ""))] | length' <<<"$config_json")"
        if [[ "$host_path_count" != "0" ]]; then
            fail "path-scoped ingress rule exists for canonical hostname ${host}"
            return 1
        fi
    done
}

build_rule() {
    local config_json="$1"
    local host="$2"
    local service="$3"
    local existing

    existing="$(jq -c --arg host "$host" '[.ingress[] | select((.hostname // "") == $host)][0] // {}' <<<"$config_json")"
    if [[ "$existing" == "{}" ]]; then
        jq -cn --arg host "$host" --arg service "$service" '{hostname: $host, service: $service}'
    else
        jq -c --arg host "$host" --arg service "$service" '.hostname = $host | .service = $service' <<<"$existing"
    fi
}

build_desired_config() {
    local config_json="$1"
    local www_rule login_rule

    validate_tunnel_config "$config_json" || return 1
    www_rule="$(build_rule "$config_json" "$WWW_HOST" "$WWW_SERVICE")"
    login_rule="$(build_rule "$config_json" "$LOGIN_HOST" "$LOGIN_SERVICE")"

    jq -c \
        --arg www_host "$WWW_HOST" \
        --arg login_host "$LOGIN_HOST" \
        --argjson www_rule "$www_rule" \
        --argjson login_rule "$login_rule" '
        . as $config
        | (.ingress
            | map(select(
                ((.hostname // "") != $www_host)
                and ((.hostname // "") != $login_host)
            ))) as $without_oteryn
        | ($without_oteryn[-1]) as $catchall
        | ($without_oteryn[0:-1]) as $other_rules
        | $config + {ingress: ([$www_rule, $login_rule] + $other_rules + [$catchall])}
    ' <<<"$config_json"
}

normalize_dns_name() {
    local value="$1"
    value="${value%.}"
    printf '%s' "${value,,}"
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
        fail "multiple DNS records exist for canonical hostname ${host}"
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

fetch_dns_records() {
    local host="$1"
    api_request GET "/zones/${CLOUDFLARE_ZONE_ID}/dns_records?name.exact=${host}&per_page=100"
}

upsert_dns_record() {
    local host="$1"
    local target="$2"
    local records_response="$3"
    local count record_id body

    count="$(jq '.result | length' <<<"$records_response")"
    body="$(jq -cn \
        --arg host "$host" \
        --arg target "$target" \
        '{type: "CNAME", name: $host, content: $target, proxied: true}')"

    if [[ "$count" == "0" ]]; then
        api_request POST "/zones/${CLOUDFLARE_ZONE_ID}/dns_records" "$body" >/dev/null
        return 0
    fi

    [[ "$count" == "1" ]] || fail "refusing to update ambiguous DNS state for ${host}"
    record_id="$(jq -r '.result[0].id // ""' <<<"$records_response")"
    [[ -n "$record_id" ]] || fail "existing DNS record for ${host} has no record ID"
    api_request PATCH "/zones/${CLOUDFLARE_ZONE_ID}/dns_records/${record_id}" "$body" >/dev/null
}

validate_identifiers() {
    [[ "$CLOUDFLARE_ACCOUNT_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ACCOUNT_ID must be a 32-character hexadecimal identifier"
    [[ "$CLOUDFLARE_ZONE_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ZONE_ID must be a 32-character hexadecimal identifier"
    [[ "$CLOUDFLARE_TUNNEL_ID" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$ ]] || fail "CLOUDFLARE_TUNNEL_ID must be a UUID"
}

main() {
    local mode="${1:-}"
    local token_response token_verify_path tunnel_response config_response current_config desired_config
    local tunnel_id account_tag config_src tunnel_state target
    local initial_config_hash current_config_hash tunnel_drift
    local www_dns login_dns www_state login_state
    local mutation="none"

    [[ "$mode" == "audit" || "$mode" == "apply" ]] || fail "usage: $0 audit|apply"

    require_command jq
    require_command sha256sum
    require_command "$CURL_BIN"
    require_nonempty_env CLOUDFLARE_API_TOKEN
    require_nonempty_env CLOUDFLARE_ACCOUNT_ID
    require_nonempty_env CLOUDFLARE_ZONE_ID
    require_nonempty_env CLOUDFLARE_TUNNEL_ID
    validate_identifiers

    if [[ "$mode" == "apply" ]]; then
        [[ "${CLOUDFLARE_APPLY_CONFIRMATION:-}" == "$APPLY_CONFIRMATION" ]] || fail "apply requires exact confirmation: ${APPLY_CONFIRMATION}"
    fi

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
    tunnel_state="$(jq -r '.result.status // "unknown"' <<<"$tunnel_response")"

    [[ "$tunnel_id" == "$CLOUDFLARE_TUNNEL_ID" ]] || fail "Cloudflare returned a different tunnel ID"
    [[ "${account_tag,,}" == "${CLOUDFLARE_ACCOUNT_ID,,}" ]] || fail "tunnel does not belong to CLOUDFLARE_ACCOUNT_ID"
    [[ "$config_src" == "cloudflare" ]] || fail "tunnel is not remotely managed (config_src=${config_src:-unknown})"

    config_response="$(api_request GET "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations")"
    current_config="$(jq -c '.result.config // empty' <<<"$config_response")"
    [[ -n "$current_config" ]] || fail "Cloudflare returned no remote tunnel configuration"
    desired_config="$(build_desired_config "$current_config")"

    initial_config_hash="$(sha256_json <<<"$current_config")"
    if [[ "$(canonical_json <<<"$current_config")" == "$(canonical_json <<<"$desired_config")" ]]; then
        tunnel_drift="current"
    else
        tunnel_drift="drift"
    fi

    target="${CLOUDFLARE_TUNNEL_ID}.cfargotunnel.com"
    www_dns="$(fetch_dns_records "$WWW_HOST")"
    login_dns="$(fetch_dns_records "$LOGIN_HOST")"
    www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
    login_state="$(dns_record_state "$login_dns" "$LOGIN_HOST" "$target")"

    if [[ "$mode" == "apply" ]]; then
        if [[ "$tunnel_drift" == "drift" ]]; then
            config_response="$(api_request GET "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations")"
            current_config="$(jq -c '.result.config // empty' <<<"$config_response")"
            [[ -n "$current_config" ]] || fail "Cloudflare returned no remote tunnel configuration during apply precondition check"
            current_config_hash="$(sha256_json <<<"$current_config")"
            [[ "$current_config_hash" == "$initial_config_hash" ]] || fail "tunnel configuration changed during planning; rerun audit before apply"
            desired_config="$(build_desired_config "$current_config")"
            api_request PUT \
                "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations" \
                "$(jq -cn --argjson config "$desired_config" '{config: $config}')" >/dev/null
            mutation="tunnel"
        fi

        www_dns="$(fetch_dns_records "$WWW_HOST")"
        www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
        if [[ "$www_state" != "current" ]]; then
            upsert_dns_record "$WWW_HOST" "$target" "$www_dns"
            if [[ "$mutation" == "none" ]]; then
                mutation="dns-www"
            else
                mutation="${mutation}+dns-www"
            fi
        fi

        login_dns="$(fetch_dns_records "$LOGIN_HOST")"
        login_state="$(dns_record_state "$login_dns" "$LOGIN_HOST" "$target")"
        if [[ "$login_state" != "current" ]]; then
            upsert_dns_record "$LOGIN_HOST" "$target" "$login_dns"
            if [[ "$mutation" == "none" ]]; then
                mutation="dns-login"
            else
                mutation="${mutation}+dns-login"
            fi
        fi

        config_response="$(api_request GET "/accounts/${CLOUDFLARE_ACCOUNT_ID}/cfd_tunnel/${CLOUDFLARE_TUNNEL_ID}/configurations")"
        current_config="$(jq -c '.result.config // empty' <<<"$config_response")"
        desired_config="$(build_desired_config "$current_config")"
        [[ "$(canonical_json <<<"$current_config")" == "$(canonical_json <<<"$desired_config")" ]] || fail "post-apply tunnel verification detected remaining drift"

        www_dns="$(fetch_dns_records "$WWW_HOST")"
        login_dns="$(fetch_dns_records "$LOGIN_HOST")"
        www_state="$(dns_record_state "$www_dns" "$WWW_HOST" "$target")"
        login_state="$(dns_record_state "$login_dns" "$LOGIN_HOST" "$target")"
        [[ "$www_state" == "current" && "$login_state" == "current" ]] || fail "post-apply DNS verification detected remaining drift"
        tunnel_drift="current"
    fi

    append_summary '### Cloudflare Oteryn endpoints'
    append_summary "- mode: \`${mode}\`"
    append_summary '- token status: `active`'
    append_summary "- tunnel configuration source: \`${config_src}\`"
    append_summary "- tunnel runtime status: \`${tunnel_state}\`"
    append_summary "- tunnel endpoint contract: \`${tunnel_drift}\`"
    append_summary "- DNS ${WWW_HOST}: \`${www_state}\`"
    append_summary "- DNS ${LOGIN_HOST}: \`${login_state}\`"
    append_summary "- mutation: \`${mutation}\`"
    append_summary '- secrets emitted: `false`'

    printf 'mode=%s tunnel=%s dns_www=%s dns_login=%s mutation=%s\n' \
        "$mode" "$tunnel_drift" "$www_state" "$login_state" "$mutation"
}

if [[ "${CLOUDFLARE_LIBRARY_ONLY:-0}" != "1" ]]; then
    main "$@"
fi
