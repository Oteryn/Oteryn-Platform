#!/usr/bin/env bash
set -euo pipefail

is_sha() { [[ "${1:-}" =~ ^[0-9a-f]{40}$ ]]; }
is_schema_id() { [[ "${1:-}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]]; }
is_immutable_image() { [[ "${1:-}" =~ @sha256:[0-9a-f]{64}$ ]]; }

contains_schema_id() {
    local needle="$1" list="$2" item
    IFS=',' read -r -a items <<<"$list"
    for item in "${items[@]}"; do
        [[ "$item" == "$needle" ]] && return 0
    done
    return 1
}

validate_release_file() {
    local file="$1"
    [[ -f "$file" ]] || { echo "Missing release metadata: $file" >&2; return 1; }
    unset RELEASE_SHA PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE SCHEMA_COMPATIBILITY_ID APP_ACCEPTS_SCHEMA_IDS MIGRATION_POLICY ROLLBACK_ELIGIBLE
    unset GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT
    # State files are written only by write_release using printf %q for every
    # value, so sourcing preserves exact bytes without allowing whitespace or
    # shell metacharacters to become syntax.
    # shellcheck disable=SC1090
    source "$file"
    is_sha "${RELEASE_SHA:-}" || { echo "Invalid or missing RELEASE_SHA" >&2; return 1; }
    is_schema_id "${SCHEMA_COMPATIBILITY_ID:-}" || { echo "Invalid or missing SCHEMA_COMPATIBILITY_ID" >&2; return 1; }
    [[ "${MIGRATION_POLICY:-}" == expand-contract ]] || { echo "MIGRATION_POLICY must be expand-contract" >&2; return 1; }
    [[ -n "${APP_ACCEPTS_SCHEMA_IDS:-}" ]] || { echo "Missing APP_ACCEPTS_SCHEMA_IDS" >&2; return 1; }
    contains_schema_id "$SCHEMA_COMPATIBILITY_ID" "$APP_ACCEPTS_SCHEMA_IDS" || {
        echo "Release contract must accept its own primary schema identity '$SCHEMA_COMPATIBILITY_ID'." >&2
        return 1
    }
    is_immutable_image "${PLATFORM_IMAGE:-}" || { echo "PLATFORM_IMAGE is not immutable" >&2; return 1; }
    is_immutable_image "${GATEWAY_IMAGE:-}" || { echo "GATEWAY_IMAGE is not immutable" >&2; return 1; }
    is_immutable_image "${CANARY_IMAGE:-}" || { echo "CANARY_IMAGE is not immutable" >&2; return 1; }
    [[ "${ROLLBACK_ELIGIBLE:-}" =~ ^[01]$ ]] || { echo "Missing/invalid ROLLBACK_ELIGIBLE" >&2; return 1; }
}

assert_schema_compatible() {
    local schema_id="$1" last_good_file="$2" candidate_sha="$3"
    is_schema_id "$schema_id" || { echo "Current schema identity is invalid" >&2; return 1; }
    is_sha "$candidate_sha" || { echo "Candidate release identity is invalid" >&2; return 1; }
    validate_release_file "$last_good_file" || return 1
    local old_sha="$RELEASE_SHA" old_accepts="$APP_ACCEPTS_SCHEMA_IDS" old_eligible="$ROLLBACK_ELIGIBLE"
    [[ "$candidate_sha" != "$old_sha" ]] || { echo "Last-good identity is stale: it equals candidate release" >&2; return 1; }
    [[ "$old_eligible" == 1 ]] || { echo "Last-good release is explicitly ineligible for image rollback" >&2; return 1; }
    contains_schema_id "$schema_id" "$old_accepts" || {
        echo "Rollback rejected: last-good application does not declare compatibility with current schema '$schema_id'." >&2
        return 1
    }
}

resolve_image() {
    local ref="$1" digest
    if is_immutable_image "$ref"; then printf '%s\n' "$ref"; return 0; fi
    digest="$(docker image inspect --format '{{index .RepoDigests 0}}' "$ref" 2>/dev/null || true)"
    is_immutable_image "$digest" || { echo "Unable to resolve immutable digest for image: $ref" >&2; return 1; }
    printf '%s\n' "$digest"
}

write_state_value() {
    local key="$1" value="$2"
    printf '%s=%q\n' "$key" "$value"
}

write_release() {
    local out="$1" release_sha="$2" schema_id="$3" accepts="$4" platform="$5" gateway="$6" canary="$7" eligible="$8"
    is_sha "$release_sha" || { echo "Release SHA must be exact 40-char lowercase git SHA" >&2; return 1; }
    is_schema_id "$schema_id" || { echo "Invalid schema compatibility identity" >&2; return 1; }
    [[ -n "$accepts" ]] || { echo "Accepted schema identities are required" >&2; return 1; }
    contains_schema_id "$schema_id" "$accepts" || {
        echo "Release contract must accept its own primary schema identity '$schema_id'." >&2
        return 1
    }
    platform="$(resolve_image "$platform")"; gateway="$(resolve_image "$gateway")"; canary="$(resolve_image "$canary")"
    [[ "$eligible" =~ ^[01]$ ]] || return 1
    local tmp="${out}.tmp"
    umask 077
    {
        write_state_value RELEASE_SHA "$release_sha"
        write_state_value PLATFORM_IMAGE "$platform"
        write_state_value GATEWAY_IMAGE "$gateway"
        write_state_value CANARY_IMAGE "$canary"
        write_state_value SCHEMA_COMPATIBILITY_ID "$schema_id"
        write_state_value APP_ACCEPTS_SCHEMA_IDS "$accepts"
        write_state_value MIGRATION_POLICY expand-contract
        write_state_value ROLLBACK_ELIGIBLE "$eligible"
        write_state_value GAME_WORLD_ID "${GAME_WORLD_ID:-}"
        write_state_value GAME_WORLD_SLUG "${GAME_WORLD_SLUG:-}"
        write_state_value GAME_WORLD_NAME "${GAME_WORLD_NAME:-}"
        write_state_value GAME_WORLD_REGION "${GAME_WORLD_REGION:-}"
        write_state_value GAME_WORLD_HOST "${GAME_WORLD_HOST:-}"
        write_state_value GAME_WORLD_PORT "${GAME_WORLD_PORT:-}"
    } >"$tmp"
    validate_release_file "$tmp"
    mv "$tmp" "$out"
}

case "${1:-}" in
    validate) validate_release_file "$2" ;;
    compatible-schema) assert_schema_compatible "$2" "$3" "$4" ;;
    resolve-image) resolve_image "$2" ;;
    write) write_release "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9" ;;
    *) echo "usage: $0 {validate FILE|compatible-schema SCHEMA LAST_GOOD CANDIDATE_SHA|resolve-image IMAGE|write OUT SHA SCHEMA ACCEPTS PLATFORM GATEWAY CANARY ELIGIBLE}" >&2; exit 2 ;;
esac
