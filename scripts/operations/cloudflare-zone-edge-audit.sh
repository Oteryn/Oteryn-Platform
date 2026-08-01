#!/usr/bin/env bash
set -euo pipefail

readonly CLOUDFLARE_API_BASE_URL="${CLOUDFLARE_API_BASE_URL:-https://api.cloudflare.com/client/v4}"
readonly CURL_BIN="${CLOUDFLARE_CURL_BIN:-curl}"
readonly WWW_HOST="oteryn.molehill.cloud"
readonly GATEWAY_HOST="gateway.molehill.cloud"
readonly LEGACY_GATEWAY_HOST="login.oteryn.molehill.cloud"
readonly OUTPUT_PATH="${CLOUDFLARE_ZONE_EDGE_OUTPUT:-cloudflare-zone-edge-audit.json}"
readonly EDGE_TOKEN="${CLOUDFLARE_API_TOKEN:-}"
readonly ACCESS_TOKEN="${CLOUDFLARE_ACCESS_API_TOKEN:-${CLOUDFLARE_API_TOKEN:-}}"

TEMP_FILES=()
declare -A API_FILE=()
declare -A API_STATE=()
declare -A API_HTTP=()

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

validate_identifiers() {
    [[ "$CLOUDFLARE_ACCOUNT_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ACCOUNT_ID must be a 32-character hexadecimal identifier"
    [[ "$CLOUDFLARE_ZONE_ID" =~ ^[0-9a-fA-F]{32}$ ]] || fail "CLOUDFLARE_ZONE_ID must be a 32-character hexadecimal identifier"
}

append_summary() {
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        printf '%s\n' "$1" >>"$GITHUB_STEP_SUMMARY"
    fi
}

api_get() {
    local key="$1"
    local path="$2"
    local token="${3:-$EDGE_TOKEN}"
    local response_file http_code

    response_file="$(mktemp)"
    TEMP_FILES+=("$response_file")
    API_FILE["$key"]="$response_file"
    API_STATE["$key"]="unknown"
    API_HTTP["$key"]="000"

    if [[ -z "$token" ]]; then
        API_STATE["$key"]="missing_token"
        return 0
    fi

    if ! http_code="$($CURL_BIN \
        --silent \
        --show-error \
        --connect-timeout 10 \
        --max-time 30 \
        --request GET \
        --header "Authorization: Bearer ${token}" \
        --header 'Content-Type: application/json' \
        --output "$response_file" \
        --write-out '%{http_code}' \
        "${CLOUDFLARE_API_BASE_URL}${path}")"; then
        API_STATE["$key"]="transport_error"
        return 0
    fi

    API_HTTP["$key"]="$http_code"
    if [[ ! "$http_code" =~ ^2[0-9][0-9]$ ]]; then
        API_STATE["$key"]="http_${http_code}"
        return 0
    fi
    if ! jq -e '.success == true' "$response_file" >/dev/null 2>&1; then
        API_STATE["$key"]="api_error"
        return 0
    fi
    API_STATE["$key"]="available"
}

state_object() {
    local key="$1"
    local required_permission="$2"
    jq -cn \
        --arg state "${API_STATE[$key]:-not_requested}" \
        --arg http "${API_HTTP[$key]:-000}" \
        --arg permission "$required_permission" \
        '{state: $state, http_status: $http, required_permission: $permission}'
}

require_complete_first_page() {
    local key="$1"
    local response_file
    [[ "${API_STATE[$key]}" == "available" ]] || return 0
    response_file="${API_FILE[$key]}"
    if [[ "$(jq -r '.result_info.total_pages // 1' "$response_file")" != "1" ]]; then
        API_STATE["$key"]="partial_pagination"
    fi
}

certificate_coverage() {
    local response_file="$1"
    local host="$2"
    jq -r --arg host "${host,,}" '
        def covers($pattern; $candidate):
            ($pattern | ascii_downcase | rtrimstr(".")) as $p
            | ($candidate | ascii_downcase | rtrimstr(".")) as $h
            | if $p == $h then true
              elif ($p | startswith("*.")) then
                ($p[2:]) as $suffix
                | ($h | endswith("." + $suffix))
                  and (($h | rtrimstr("." + $suffix)) as $label
                       | ($label != "") and (($label | contains(".")) | not))
              else false
              end;
        [
            .result[]?
            | select((.status // "") == "active")
            | ((.certificates[]? | select((.status // "") == "active") | .hosts[]?), .hosts[]?)
            | strings
        ]
        | unique
        | any(. as $pattern | covers($pattern; $host))
    ' "$response_file"
}

setting_value() {
    local response_file="$1"
    local setting="$2"
    jq -c --arg setting "$setting" '[.result[]? | select(.id == $setting)][0].value // null' "$response_file"
}

ruleset_summary() {
    local scope="$1"
    local list_key="$2"
    local list_file="${API_FILE[$list_key]}"
    local summaries_file ruleset_id phase detail_key detail_file item
    local detail_failures=0

    summaries_file="$(mktemp)"
    TEMP_FILES+=("$summaries_file")
    printf '[]\n' >"$summaries_file"

    if [[ "${API_STATE[$list_key]}" != "available" ]]; then
        jq -cn --arg state "${API_STATE[$list_key]}" '{state: $state, rulesets: []}'
        return 0
    fi

    while IFS=$'\t' read -r ruleset_id phase; do
        [[ -n "$ruleset_id" ]] || continue
        detail_key="${scope}_ruleset_${ruleset_id}"
        if [[ "$scope" == "zone" ]]; then
            api_get "$detail_key" "/zones/${CLOUDFLARE_ZONE_ID}/rulesets/${ruleset_id}"
        else
            api_get "$detail_key" "/accounts/${CLOUDFLARE_ACCOUNT_ID}/rulesets/${ruleset_id}"
        fi
        if [[ "${API_STATE[$detail_key]}" != "available" ]]; then
            detail_failures=$((detail_failures + 1))
            continue
        fi
        detail_file="${API_FILE[$detail_key]}"
        item="$(jq -c \
            --arg phase "$phase" \
            --arg www "$WWW_HOST" \
            --arg gateway "$GATEWAY_HOST" \
            --arg legacy "$LEGACY_GATEWAY_HOST" '
            def enabled_rules: [.result.rules[]? | select((.enabled // true) == true)];
            def quoted_host($host): ((.expression // "") | ascii_downcase | contains("\"" + ($host | ascii_downcase) + "\""));
            def canonical_ref: (quoted_host($www) or quoted_host($gateway));
            def retired_ref: quoted_host($legacy);
            {
              phase: $phase,
              enabled_rule_count: (enabled_rules | length),
              actions: (enabled_rules | group_by(.action // "unknown") | map({key: (.[0].action // "unknown"), value: length}) | from_entries),
              canonical_hostname_rule_count: (enabled_rules | map(select(canonical_ref)) | length),
              retired_hostname_rule_count: (enabled_rules | map(select(retired_ref)) | length),
              canonical_challenge_or_block_count: (enabled_rules | map(select(canonical_ref and ((.action // "") | IN("challenge", "managed_challenge", "block")))) | length),
              noncanonical_or_global_challenge_count: (enabled_rules | map(select((canonical_ref | not) and ((.action // "") | IN("challenge", "managed_challenge")))) | length)
            }
        ' "$detail_file")"
        jq --argjson item "$item" '. + [$item]' "$summaries_file" >"${summaries_file}.new"
        mv "${summaries_file}.new" "$summaries_file"
    done < <(jq -r '
        .result[]?
        | select((.phase // "") | IN(
            "http_request_firewall_custom",
            "http_request_firewall_managed",
            "http_request_sbfm",
            "http_ratelimit",
            "http_request_dynamic_redirect",
            "http_response_headers_transform"
          ))
        | [.id, .phase] | @tsv
    ' "$list_file")

    jq -cn \
        --arg state "$(if ((detail_failures == 0)); then printf available; else printf partial; fi)" \
        --argjson failures "$detail_failures" \
        --slurpfile rulesets "$summaries_file" \
        '{state: $state, detail_failures: $failures, rulesets: $rulesets[0]}'
}

main() {
    local token_path token_status observed_at
    local cert_www="unknown" cert_gateway="unknown" cert_legacy="unknown"
    local active_packs="null" pending_verifications="null" universal_enabled="null"
    local settings_available=false ssl_value="null" min_tls="null" tls13="null" always_https="null"
    local security_level="null" browser_check="null" challenge_ttl="null" hsts="null"
    local bot="null" access="null" pagerules="null" zone_rules account_rules
    local audit_complete output_json

    require_command jq
    require_command "$CURL_BIN"
    require_nonempty_env CLOUDFLARE_API_TOKEN
    require_nonempty_env CLOUDFLARE_ACCOUNT_ID
    require_nonempty_env CLOUDFLARE_ZONE_ID
    validate_identifiers

    if [[ "$EDGE_TOKEN" == cfat_* ]]; then
        token_path="/accounts/${CLOUDFLARE_ACCOUNT_ID}/tokens/verify"
    else
        token_path='/user/tokens/verify'
    fi
    api_get token "$token_path"
    token_status="unknown"
    if [[ "${API_STATE[token]}" == "available" ]]; then
        token_status="$(jq -r '.result.status // "unknown"' "${API_FILE[token]}")"
    fi
    [[ "$token_status" == "active" ]] || fail "Cloudflare API token is not proven active"

    api_get certificate_packs "/zones/${CLOUDFLARE_ZONE_ID}/ssl/certificate_packs?per_page=50"
    api_get universal_ssl "/zones/${CLOUDFLARE_ZONE_ID}/ssl/universal/settings"
    api_get ssl_verification "/zones/${CLOUDFLARE_ZONE_ID}/ssl/verification"
    api_get zone_settings "/zones/${CLOUDFLARE_ZONE_ID}/settings"
    api_get zone_rulesets "/zones/${CLOUDFLARE_ZONE_ID}/rulesets?per_page=50"
    api_get account_rulesets "/accounts/${CLOUDFLARE_ACCOUNT_ID}/rulesets?per_page=50"
    api_get bot_management "/zones/${CLOUDFLARE_ZONE_ID}/bot_management"
    api_get access_apps "/accounts/${CLOUDFLARE_ACCOUNT_ID}/access/apps?per_page=50" "$ACCESS_TOKEN"
    api_get page_rules "/zones/${CLOUDFLARE_ZONE_ID}/pagerules?status=active&per_page=50"

    for key in certificate_packs zone_rulesets account_rulesets access_apps page_rules; do
        require_complete_first_page "$key"
    done

    if [[ "${API_STATE[certificate_packs]}" == "available" ]]; then
        cert_www="$(certificate_coverage "${API_FILE[certificate_packs]}" "$WWW_HOST")"
        cert_gateway="$(certificate_coverage "${API_FILE[certificate_packs]}" "$GATEWAY_HOST")"
        cert_legacy="$(certificate_coverage "${API_FILE[certificate_packs]}" "$LEGACY_GATEWAY_HOST")"
        active_packs="$(jq '[.result[]? | select((.status // "") == "active")] | length' "${API_FILE[certificate_packs]}")"
    fi
    if [[ "${API_STATE[universal_ssl]}" == "available" ]]; then
        universal_enabled="$(jq -c '.result.enabled // null' "${API_FILE[universal_ssl]}")"
    fi
    if [[ "${API_STATE[ssl_verification]}" == "available" ]]; then
        pending_verifications="$(jq '[.result[]? | select((.certificate_status // "") != "active")] | length' "${API_FILE[ssl_verification]}")"
    fi
    if [[ "${API_STATE[zone_settings]}" == "available" ]]; then
        settings_available=true
        ssl_value="$(setting_value "${API_FILE[zone_settings]}" ssl)"
        min_tls="$(setting_value "${API_FILE[zone_settings]}" min_tls_version)"
        tls13="$(setting_value "${API_FILE[zone_settings]}" tls_1_3)"
        always_https="$(setting_value "${API_FILE[zone_settings]}" always_use_https)"
        security_level="$(setting_value "${API_FILE[zone_settings]}" security_level)"
        browser_check="$(setting_value "${API_FILE[zone_settings]}" browser_check)"
        challenge_ttl="$(setting_value "${API_FILE[zone_settings]}" challenge_ttl)"
        hsts="$(setting_value "${API_FILE[zone_settings]}" security_header)"
    fi

    zone_rules="$(ruleset_summary zone zone_rulesets)"
    account_rules="$(ruleset_summary account account_rulesets)"

    if [[ "${API_STATE[bot_management]}" == "available" ]]; then
        bot="$(jq -c '{fight_mode: (.result.fight_mode // null), enable_js: (.result.enable_js // null), ai_bots_protection: (.result.ai_bots_protection // null), content_bots_protection: (.result.content_bots_protection // null)}' "${API_FILE[bot_management]}")"
    fi
    if [[ "${API_STATE[access_apps]}" == "available" ]]; then
        access="$(jq -c --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" --arg legacy "$LEGACY_GATEWAY_HOST" '
            def domain: ((.domain // "") | ascii_downcase);
            {
              matching_application_count: ([.result[]? | select((domain == $www) or (domain == $gateway) or (domain | startswith($www + "/")) or (domain | startswith($gateway + "/")))] | length),
              retired_application_count: ([.result[]? | select((domain == $legacy) or (domain | startswith($legacy + "/")))] | length)
            }
        ' "${API_FILE[access_apps]}")"
    fi
    if [[ "${API_STATE[page_rules]}" == "available" ]]; then
        pagerules="$(jq -c --arg www "$WWW_HOST" --arg gateway "$GATEWAY_HOST" --arg legacy "$LEGACY_GATEWAY_HOST" '
            def canonical_ref: ([.targets[]?.constraint.value // ""] | any(contains($www) or contains($gateway)));
            def retired_ref: ([.targets[]?.constraint.value // ""] | any(contains($legacy)));
            [.result[]? | select((.status // "active") == "active")] as $active
            | [$active[] | select(canonical_ref)] as $matching
            | {
                matching_rule_count: ($matching | length),
                retired_rule_count: ([$active[] | select(retired_ref)] | length),
                always_https_action_count: ([$matching[]?.actions[]? | select(.id == "always_use_https")] | length),
                browser_check_action_count: ([$matching[]?.actions[]? | select(.id == "browser_check")] | length),
                security_level_action_count: ([$matching[]?.actions[]? | select(.id == "security_level")] | length),
                forwarding_url_action_count: ([$matching[]?.actions[]? | select(.id == "forwarding_url")] | length)
              }
        ' "${API_FILE[page_rules]}")"
    fi

    audit_complete=true
    for key in certificate_packs universal_ssl ssl_verification zone_settings zone_rulesets account_rulesets bot_management access_apps page_rules; do
        if [[ "${API_STATE[$key]}" != "available" ]]; then
            audit_complete=false
        fi
    done

    observed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    output_json="$(jq -cn \
        --arg observed_at "$observed_at" \
        --arg token_status "$token_status" \
        --argjson audit_complete "$audit_complete" \
        --arg cert_www "$cert_www" \
        --arg cert_gateway "$cert_gateway" \
        --arg cert_legacy "$cert_legacy" \
        --argjson active_packs "$active_packs" \
        --argjson pending_verifications "$pending_verifications" \
        --argjson universal_enabled "$universal_enabled" \
        --argjson settings_available "$settings_available" \
        --argjson ssl_value "$ssl_value" \
        --argjson min_tls "$min_tls" \
        --argjson tls13 "$tls13" \
        --argjson always_https "$always_https" \
        --argjson security_level "$security_level" \
        --argjson browser_check "$browser_check" \
        --argjson challenge_ttl "$challenge_ttl" \
        --argjson hsts "$hsts" \
        --argjson zone_rules "$zone_rules" \
        --argjson account_rules "$account_rules" \
        --argjson bot "$bot" \
        --argjson access "$access" \
        --argjson pagerules "$pagerules" \
        --argjson certificate_api "$(state_object certificate_packs 'SSL and Certificates Read')" \
        --argjson universal_api "$(state_object universal_ssl 'SSL and Certificates Read')" \
        --argjson verification_api "$(state_object ssl_verification 'SSL and Certificates Read')" \
        --argjson settings_api "$(state_object zone_settings 'Zone Settings Read')" \
        --argjson zone_rules_api "$(state_object zone_rulesets 'Zone WAF Read or Account Rulesets Read')" \
        --argjson account_rules_api "$(state_object account_rulesets 'Account WAF Read or Account Rulesets Read')" \
        --argjson bot_api "$(state_object bot_management 'Bot Management Read')" \
        --argjson access_api "$(state_object access_apps 'Access Apps and Policies Read')" \
        --argjson pagerules_api "$(state_object page_rules 'Page Rules Read')" '
        {
          schema_version: 2,
          observed_at: $observed_at,
          mutation: "none",
          secrets_emitted: false,
          token_status: $token_status,
          audit_complete: $audit_complete,
          canonical_hosts: ["oteryn.molehill.cloud", "gateway.molehill.cloud"],
          retired_hosts: ["login.oteryn.molehill.cloud"],
          api: {
            certificate_packs: $certificate_api,
            universal_ssl: $universal_api,
            ssl_verification: $verification_api,
            zone_settings: $settings_api,
            zone_rulesets: $zone_rules_api,
            account_rulesets: $account_rules_api,
            bot_management: $bot_api,
            access_apps: $access_api,
            page_rules: $pagerules_api
          },
          certificates: {
            active_pack_count: $active_packs,
            universal_ssl_enabled: $universal_enabled,
            nonactive_verification_count: $pending_verifications,
            www_hostname_covered: $cert_www,
            gateway_hostname_covered: $cert_gateway,
            legacy_gateway_hostname_covered: $cert_legacy
          },
          zone_settings: {
            available: $settings_available,
            ssl_mode: $ssl_value,
            minimum_tls_version: $min_tls,
            tls_1_3: $tls13,
            always_use_https: $always_https,
            security_level: $security_level,
            browser_integrity_check: $browser_check,
            challenge_ttl: $challenge_ttl,
            security_header: $hsts
          },
          rulesets: {zone: $zone_rules, account: $account_rules},
          bot_management: $bot,
          access: $access,
          page_rules: $pagerules
        }
    ')"

    printf '%s\n' "$output_json" | jq . >"$OUTPUT_PATH"

    append_summary '### Cloudflare Oteryn zone-edge audit'
    append_summary "- audit complete: \`${audit_complete}\`"
    append_summary "- certificate ${WWW_HOST}: \`${cert_www}\`"
    append_summary "- certificate ${GATEWAY_HOST}: \`${cert_gateway}\`"
    append_summary "- certificate retired ${LEGACY_GATEWAY_HOST}: \`${cert_legacy}\`"
    append_summary "- Always Use HTTPS: \`$(jq -r '.zone_settings.always_use_https // "unknown"' "$OUTPUT_PATH")\`"
    append_summary "- HSTS enabled: \`$(jq -r '.zone_settings.security_header.strict_transport_security.enabled // "unknown"' "$OUTPUT_PATH")\`"
    append_summary "- mutation: \`none\`"
    append_summary "- sanitized artifact: \`$(basename "$OUTPUT_PATH")\`"

    printf 'audit_complete=%s certificate_www=%s certificate_gateway=%s certificate_legacy_gateway=%s mutation=none\n' \
        "$audit_complete" "$cert_www" "$cert_gateway" "$cert_legacy"
}

main "$@"
