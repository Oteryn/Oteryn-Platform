#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"

upgrade_one_release_state() {
    local state_file="$1" label="$2"
    local release_sha platform_image gateway_image canary_image schema_id accepts eligible
    local platform_revision gateway_revision backup

    [[ -f "$state_file" ]] || return 0

    if bash "$SCRIPT_DIR/release-state.sh" validate "$state_file" >/dev/null 2>&1; then
        return 0
    fi

    # A state file that already contains either new provenance key but fails
    # current validation is corrupt/partial, not legacy. Never guess through
    # that boundary.
    if grep -Eq '^(PLATFORM_SOURCE_SHA|GATEWAY_SOURCE_SHA)=' "$state_file"; then
        echo "$label release state contains partial/invalid component provenance; refusing automatic upgrade." >&2
        return 1
    fi

    mapfile -t legacy < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "${RELEASE_SHA:-}" "${PLATFORM_IMAGE:-}" "${GATEWAY_IMAGE:-}" "${CANARY_IMAGE:-}" \
            "${SCHEMA_COMPATIBILITY_ID:-}" "${APP_ACCEPTS_SCHEMA_IDS:-}" "${ROLLBACK_ELIGIBLE:-}" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
    ' bash "$state_file")

    release_sha="${legacy[0]:-}"
    platform_image="${legacy[1]:-}"
    gateway_image="${legacy[2]:-}"
    canary_image="${legacy[3]:-}"
    schema_id="${legacy[4]:-}"
    accepts="${legacy[5]:-}"
    eligible="${legacy[6]:-}"

    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "$label legacy release state has invalid RELEASE_SHA." >&2
        return 1
    }
    [[ "$platform_image" =~ @sha256:[0-9a-f]{64}$ ]] || {
        echo "$label legacy Platform image is not immutable." >&2
        return 1
    }
    [[ "$gateway_image" =~ @sha256:[0-9a-f]{64}$ ]] || {
        echo "$label legacy Gateway image is not immutable." >&2
        return 1
    }
    [[ "$canary_image" =~ @sha256:[0-9a-f]{64}$ ]] || {
        echo "$label legacy Canary image is not immutable." >&2
        return 1
    }

    for image in "$platform_image" "$gateway_image"; do
        docker image inspect "$image" >/dev/null 2>&1 || docker pull "$image" >/dev/null
    done
    platform_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$platform_image" 2>/dev/null || true)"
    gateway_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$gateway_image" 2>/dev/null || true)"
    [[ "$platform_revision" == "$release_sha" ]] || {
        echo "$label legacy Platform OCI revision does not match persisted RELEASE_SHA; refusing state upgrade." >&2
        return 1
    }
    [[ "$gateway_revision" == "$release_sha" ]] || {
        echo "$label legacy Gateway OCI revision does not match persisted RELEASE_SHA; refusing state upgrade." >&2
        return 1
    }

    GAME_WORLD_ID="${legacy[7]:-}"
    GAME_WORLD_SLUG="${legacy[8]:-}"
    GAME_WORLD_NAME="${legacy[9]:-}"
    GAME_WORLD_REGION="${legacy[10]:-}"
    GAME_WORLD_HOST="${legacy[11]:-}"
    GAME_WORLD_PORT="${legacy[12]:-}"
    export GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT

    backup="$state_file.legacy-v1"
    [[ ! -e "$backup" ]] || {
        echo "$label legacy release-state backup already exists; refusing ambiguous second upgrade." >&2
        return 1
    }
    cp "$state_file" "$backup"
    chmod 600 "$backup"

    if ! bash "$SCRIPT_DIR/release-state.sh" write "$state_file" \
        "$release_sha" "$release_sha" "$release_sha" \
        "$schema_id" "$accepts" "$platform_image" "$gateway_image" "$canary_image" "$eligible"; then
        cp "$backup" "$state_file"
        chmod 600 "$state_file"
        echo "$label legacy release-state upgrade failed; original state restored." >&2
        return 1
    fi

    if ! bash "$SCRIPT_DIR/release-state.sh" validate "$state_file"; then
        cp "$backup" "$state_file"
        chmod 600 "$state_file"
        echo "$label upgraded release state failed validation; original state restored." >&2
        return 1
    fi

    echo "Upgraded legacy $label release state to explicit Platform/Gateway component provenance for $release_sha."
}

mkdir -p "$state_dir"
upgrade_one_release_state "$state_dir/current-release.env" current
upgrade_one_release_state "$state_dir/last-good-release.env" last-good
upgrade_one_release_state "$state_dir/candidate-release.env" candidate
