#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
current_file="$state_dir/current-release.env"

[[ -f "$current_file" ]] || exit 0

if bash "$SCRIPT_DIR/release-state.sh" validate "$current_file" >/dev/null 2>&1; then
    exit 0
fi

# A state file that already contains either new provenance key but fails current
# validation is corrupt/partial, not legacy. Never guess through that boundary.
if grep -Eq '^(PLATFORM_SOURCE_SHA|GATEWAY_SOURCE_SHA)=' "$current_file"; then
    echo "Current release state contains partial/invalid component provenance; refusing automatic upgrade." >&2
    exit 1
fi

mapfile -t legacy < <(bash -c '
    set -euo pipefail
    source "$1"
    printf "%s\n" \
        "${RELEASE_SHA:-}" "${PLATFORM_IMAGE:-}" "${GATEWAY_IMAGE:-}" "${CANARY_IMAGE:-}" \
        "${SCHEMA_COMPATIBILITY_ID:-}" "${APP_ACCEPTS_SCHEMA_IDS:-}" "${ROLLBACK_ELIGIBLE:-}" \
        "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
        "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
' bash "$current_file")

release_sha="${legacy[0]:-}"
platform_image="${legacy[1]:-}"
gateway_image="${legacy[2]:-}"
canary_image="${legacy[3]:-}"
schema_id="${legacy[4]:-}"
accepts="${legacy[5]:-}"
eligible="${legacy[6]:-}"

[[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "Legacy release state has invalid RELEASE_SHA." >&2; exit 1; }
[[ "$platform_image" =~ @sha256:[0-9a-f]{64}$ ]] || { echo "Legacy Platform image is not immutable." >&2; exit 1; }
[[ "$gateway_image" =~ @sha256:[0-9a-f]{64}$ ]] || { echo "Legacy Gateway image is not immutable." >&2; exit 1; }
[[ "$canary_image" =~ @sha256:[0-9a-f]{64}$ ]] || { echo "Legacy Canary image is not immutable." >&2; exit 1; }

for image in "$platform_image" "$gateway_image"; do
    docker image inspect "$image" >/dev/null 2>&1 || docker pull "$image" >/dev/null
done
platform_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$platform_image" 2>/dev/null || true)"
gateway_revision="$(docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$gateway_image" 2>/dev/null || true)"
[[ "$platform_revision" == "$release_sha" ]] || {
    echo "Legacy Platform OCI revision does not match persisted RELEASE_SHA; refusing state upgrade." >&2
    exit 1
}
[[ "$gateway_revision" == "$release_sha" ]] || {
    echo "Legacy Gateway OCI revision does not match persisted RELEASE_SHA; refusing state upgrade." >&2
    exit 1
}

GAME_WORLD_ID="${legacy[7]:-}"
GAME_WORLD_SLUG="${legacy[8]:-}"
GAME_WORLD_NAME="${legacy[9]:-}"
GAME_WORLD_REGION="${legacy[10]:-}"
GAME_WORLD_HOST="${legacy[11]:-}"
GAME_WORLD_PORT="${legacy[12]:-}"
export GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT

backup="$current_file.legacy-v1"
[[ ! -e "$backup" ]] || { echo "Legacy release-state backup already exists; refusing ambiguous second upgrade." >&2; exit 1; }
cp "$current_file" "$backup"
chmod 600 "$backup"

if ! bash "$SCRIPT_DIR/release-state.sh" write "$current_file" \
    "$release_sha" "$release_sha" "$release_sha" \
    "$schema_id" "$accepts" "$platform_image" "$gateway_image" "$canary_image" "$eligible"; then
    cp "$backup" "$current_file"
    chmod 600 "$current_file"
    echo "Legacy release-state upgrade failed; original state restored." >&2
    exit 1
fi

bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
echo "Upgraded legacy current release state to explicit Platform/Gateway component provenance for $release_sha."
